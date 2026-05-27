# Known bugs and gaps

Tracked failures surfaced by corpus sweeps over `examples/{category,cots,JBP,nisa,xmas}/*.pq`. Each entry: symptom, where it surfaces, what we know, what we don't.

This file is the source of truth — if you fix one, delete the entry. If you find a new one, add it here rather than leaving it in a session log.

## ODBC / DBISAM

### B1. DBISAM columnar fetch fundamentally broken; row-at-a-time fallback only safe path
- **Symptom:** `Odbc.Query: columnar fetch panicked for 'dsn=Exportmaster'; falling back to row-at-a-time. panic: ... TryFromIntError(())`. Query completes via fallback but slowly — wide tables take minutes (`Customer` 58s, `Product Table` 356s, `Ingredients Table` 53 min).
- **Hits:** `category/{Customer, Product Table, Ingredients Table, Main Product Category, Sub Product Category, Sub Sub Category}`, `cots/PRODGRP`, `JBP/PRODGRP`, `nisa/PRODGRP`.
- **Diagnosis (deep-dive 2026-05-24):** DBISAM's ODBC driver has FOUR compounding bugs that together make bulk fetch unusable. Each was verified by direct instrumentation:
  1. **Per-cell indicator SQLLEN bug:** each indicator is written to the low 32 bits of a 64-bit slot, leaving the high 32 as register garbage. Direct probe captured values like `0xffffffff_00000019`, `0x600000004`, `0x100000001`. Worse: the low 32 bits don't even reliably encode the real length — row `1-10-1` (6 chars actual) came back with low32=4. No simple mask recovers the truth.
  2. **`SQL_ATTR_ROWS_FETCHED_PTR` bug:** the driver always reports `n_rows = BATCH_SIZE` (the bound rowset size), never the actual filled count.
  3. **`SQL_ATTR_ROW_STATUS_PTR` bug:** probed directly — DBISAM writes `SQL_ROW_SUCCESS` to *every* slot in the bound rowset including the ones it didn't actually fill. So the standard ODBC "how many rows did I really fill?" mechanism also lies.
  4. **Partial-fill bug:** DBISAM only fills approximately the first half of the bound rowset (bound=1024 → fills 513, bound=100 → fills 51). The unfilled tail has indicators set to `NULL_DATA`. **And rows that should appear in the NEXT batch are silently lost** — the driver's internal cursor jumps past them. Verified empirically: with raw_bytes_at + null-trim + all-null-row filter, Sub Sub Category produces 26,163 rows where the row-at-a-time fallback produces 51,823 — and the first 513 match exactly while the next batch starts ~25 rows further along in the data, never returning to fetch what was skipped.
- **Why CS-EM2Parquet works:** it uses System.Data.Odbc with single-row fetch (`SQL_ATTR_ROW_ARRAY_SIZE = 1` implicitly). That's structurally what our row-at-a-time fallback does. There's no way to make bulk fetch work without DBISAM driver changes — the data loss is **inside the driver**, not in our buffer interpretation.
- **What we have:** the cap-fix from commit `eca8963` prevents the 2 TiB allocator abort at bind time, so the existing panic + row-at-a-time fallback is recoverable. The patch comment in `indicator.rs` is accurate — keep the panic, accept the slow fallback. Architectural workaround for production: use a separate process (e.g. CS-EM2Parquet, which already runs nightly) to bulk-export DBISAM tables to parquet, then MrsFlow queries the parquet directly.

### B2. `unsupported SQL type LongVarbinary` on memo columns
- **Status:** fixed for `describe_columns` (no longer fast-fails). Query now reaches data fetch via columnar bind (capped at `CELL_CAP = 64 KiB`) — but most memo-bearing tables then hit B1 at fetch time and fall back. Cells exceeding 64 KiB are truncated; for DBISAM memo columns in this corpus that hasn't surfaced.
- **Hits:** `category/Ingredients Table` (column `NIINGREDSUNI`). `cots/PRICES`.
- **Open question:** if truncation does matter, the cap can be raised — at `CELL_CAP × BATCH_SIZE = 64 MiB` per column, there's headroom before allocator pressure becomes real.

## Stdlib semantic gaps

(none currently outstanding — S1 and S2 closed; see Recently fixed.)

## Notes on what is intentionally NOT a bug

### N1. `JBP/PowerBI tab` Int64 cast on a string column — won't fix
- **Symptom:** `Table.TransformColumnTypes: cast SAPRODUCT to Int64 failed: column has heterogeneous cells`.
- **Hits:** `JBP/PowerBI tab` (only).
- **Diagnosis:** SAPRODUCT is an ASCII product-code string that happens to contain digits. PowerQuery's "Detect Data Type" feature auto-classified it as `Int64.Type` when the M was first generated — wrong call, but invisible because PQ silently parses text-that-looks-numeric back to int. The user-correct version of this query (in `cots/PowerBI tab` and `nisa/PowerBI tab`) casts SAPRODUCT to `type text`, matching the data.
- **Why not fix:** MrsFlow correctly surfaces that the M is wrong. Making the Int64 cast silently coerce text would replicate PQ's misfeature and lose a useful signal. The fix belongs in the M (change `Int64.Type` → `type text`), not the engine.

## Notes on what is NOT a bug

- **`Analysis` timeouts (300s+)** — these are queries that load a 4.8M-row table then filter in M without folding. Excel also takes 3-7 minutes on these. Pattern is wrong, not engine.
- **`IM002` "data source not found"** — DSN isn't installed on this machine (`OCS1`, possibly others). Environmental.
- **Transitive failures from dependent queries** — e.g. `JBP/Analysis incl Rebate` joins `Analysis` (timeout), `Customer Rebate` (OK), and `PowerBI tab` (N1). It can only succeed when its upstream queries do; its FAIL row mirrors whichever upstream failed first (typically the `Analysis` timeout). The error message often shows incidental B1 driver panic chatter that resolved via row-at-a-time — the real cause is upstream, not this query.

## Recently fixed (left here briefly for reference; trim periodically)

- ~~B3: DROP / ALTER after a SELECT failed with server `0x2B05 ExecuteError`~~ — root cause pinned by Derek's BPL disassembly of `DeleteDataTable` (RVA 0x07F72C): server checks `word ptr [table+0x8]` = `TDataTable.UseCount`; nonzero → raise `0x2B05`. Materialised SELECTs create a server-side temp table whose cursor refs the source, keeping UseCount > 0. Fix: send `0x0029 TDataSession.RemoveAllRemoteMemoryTables` (bodyless) after the SELECT's CloseCursor + ResetStatement cleanup. Verified end-to-end: CREATE → INSERT → SELECT → DROP → SELECT-fails-because-table-gone. See `Derek/DBISAM-PROTOCOL.md` §7f for the exact lock model.
- ~~`unsupported SQL type Bit`~~ — DBISAM logical columns. Fixed: now maps to `DataType::Boolean` via `BufferDesc::Bit`. Hit `category/Customer`, `cots/{CUSTOMER, ORDERH, ORDERI, PRODUCT}`, etc. Some still fall back to slow row-at-a-time via B1.
- ~~Process abort: 2 TiB columnar buffer allocation~~ — DBISAM reports `LongVarchar { length: i32::MAX }` for memo columns; multiplied by `BATCH_SIZE = 1024` the allocator asked for 2 TiB and the process died before `catch_unwind` could save it. Fixed by capping declared text lengths at `CELL_CAP = 64 KiB` in `describe_columns`. Columnar bind now always succeeds; if fetch then panics, the existing fallback path handles it.
- ~~`#table(n, rows)` numeric-first-arg overload~~ — auto-names `Column1..ColumnN`.
- ~~Currency symbols in text→number cast~~ — `£ $ € ¥` stripped.
- ~~Empty text → null on numeric cast~~ — `""` and whitespace-only cells become null.
- ~~`Text.Proper(null) = null`~~ — was rejecting nulls; now passes through.
- ~~Heterogeneous-cell `type text` cast errors~~ — fixed in `b58382c`. `Table.TransformColumnTypes(_, {{"col", type text}})` now coerces per cell via `Text.From` rather than rejecting mixed columns. Also: the parquet writer (`rows_to_arrow`) coerces heterogeneous primitive columns to text on write instead of failing. Closed 5 of the 6 cluster cases; the remaining one (`JBP/PowerBI tab`) is filed under N1 — intentionally not fixed because the user's M is wrong.
- ~~`Float64 → Date32` cast not supported (S2)~~ — fixed in `afb6893`. `cultural_cast` now interprets Float64 source as Excel/PQ serial days (since 1899-12-30) when target is Date32. Closed all 6 cluster cases (cots/JBP/nisa × GP PowerBI + Revenue PowerBI).
- ~~`#table(type table [...], rows)` overload~~ — fixed in `c622d12`. The third constructor form (type-table first arg) now extracts column names from the TableOf TypeRep; declared per-column types are not enforced at construction time (PQ leniency). Closed 3 cluster cases (cots/JBP/nisa LastRefreshed).
- ~~`Percentage.Type` cast doesn't divide by 100 (S1)~~ — fixed in `ff593e0`. Threaded an `is_percentage` bool through the cast pipeline; `parse_text_to_number` now strips `%` and divides by 100 only when the user-declared type is `Percentage.Type`. Unmarked `type number` casts on `"3.10%"` still error (no silent inflate-by-100). Closed all 5 cluster cases.
