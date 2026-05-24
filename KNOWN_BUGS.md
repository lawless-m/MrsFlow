# Known bugs and gaps

Tracked failures surfaced by corpus sweeps over `examples/{category,cots,JBP,nisa,xmas}/*.pq`. Each entry: symptom, where it surfaces, what we know, what we don't.

This file is the source of truth — if you fix one, delete the entry. If you find a new one, add it here rather than leaving it in a session log.

## ODBC / DBISAM

### B1. DBISAM columnar fetch panics on certain tables → slow row-at-a-time fallback
- **Symptom:** `Odbc.Query: columnar fetch panicked for 'dsn=Exportmaster'; falling back to row-at-a-time. panic: ... TryFromIntError(())`. Query completes via fallback but slowly — wide tables with many rows take minutes (`Customer` 58s, `Product Table` 356s, `Ingredients Table` 53 min).
- **Hits:** `category/{Customer, Product Table, Ingredients Table, Main Product Category, Sub Product Category, Sub Sub Category}`, `cots/PRODGRP`, `JBP/PRODGRP`, `nisa/PRODGRP`.
- **What we know:** the vendored odbc-api `Indicator::from_isize` panics on any negative SQLLEN that isn't `NULL_DATA` or `NO_TOTAL`. The vendor patch comment (`vendor/odbc-api/src/buffers/indicator.rs`) explicitly says swallowing the panic was tried and rejected — it silently lost rows. So the panic is the *correct* surface, and row-at-a-time fallback is the safe path.
- **What we don't know:** which exact column or cell triggers the negative SQLLEN on each table. CS-EM2Parquet reads the same tables fine via System.Data.Odbc (which uses cell-by-cell `SQLGetData` — structurally similar to our row-at-a-time but with a managed driver layer that may massage the indicators).
- **Next step:** the fast fix would be patching `Indicator::from_isize` to interpret driver-specific negative SQLLEN values correctly (matching whatever System.Data.Odbc does). Without that, the fallback works — just slowly.

### B2. `unsupported SQL type LongVarbinary` on memo columns
- **Status:** fixed for `describe_columns` (no longer fast-fails). Query now reaches data fetch via columnar bind (capped at `CELL_CAP = 64 KiB`) — but most memo-bearing tables then hit B1 at fetch time and fall back. Cells exceeding 64 KiB are truncated; for DBISAM memo columns in this corpus that hasn't surfaced.
- **Hits:** `category/Ingredients Table` (column `NIINGREDSUNI`). `cots/PRICES`.
- **Open question:** if truncation does matter, the cap can be raised — at `CELL_CAP × BATCH_SIZE = 64 MiB` per column, there's headroom before allocator pressure becomes real.

## Stdlib semantic gaps

### S1. `Percentage.Type` cast doesn't divide by 100
- **Symptom:** `Table.TransformColumnTypes: cast Growth Scale 1 to number failed on '3.10%': invalid float literal`.
- **Hits:** `JBP/{Trigger Points 2023, 2024, WIth Profit 2023}`, `cots/Growth Rebates NISA`, `nisa/Growth Rebates NISA`.
- **What we know:** PQ's `Percentage.Type` cast means "strip `%` and divide by 100". By the time we reach `parse_text_to_number`, the target type has already collapsed to `Float64` and we've lost the distinction between `Percentage.Type` and `type number` / `Currency.Type`. We can't just strip `%` because that silently inflates by 100×.
- **What we don't know:** whether the original `TypeRep` is reachable at the cast site or whether we'd need to thread it through.
- **Next step:** plumb the original `TypeRep` (or at minimum a "is_percentage" bit) into `cultural_cast` / `parse_text_to_number` so the parser knows to divide.

### S2. `Float64 → Date32` cast not supported
- **Symptom:** `Table.TransformColumnTypes: cast Date to Date32 failed: Cast error: Casting from Float64 to Date32 not supported`.
- **Hits:** `cots/{GP PowerBI, Revenue PowerBI}`, `JBP/{GP PowerBI, Revenue PowerBI}`, `nisa/{GP PowerBI, Revenue PowerBI}`.
- **What we know:** the source column is numeric (Excel/PQ's serial date — days since 1899-12-30). Arrow's built-in cast doesn't know that convention.
- **Next step:** add a Float64→Date32 path in `cultural_cast` interpreting the f64 as Excel serial days.

### S3. Heterogeneous-cell text cast (nulls mixed with text)
- **Symptom:** `Table.TransformColumnTypes: cast CODE to Utf8 failed: column has heterogeneous cells`.
- **Hits:** `cots/Customer Rebate`, `JBP/Customer Rebate`, `nisa/Customer Rebate`, `cots/PowerBI tab`, `JBP/PowerBI tab`, `nisa/PowerBI tab`.
- **What we know:** the column contains text + nulls (legitimate PQ shape — nullable text). `infer_cells` declines because it sees mixed types.
- **Next step:** treat null cells as compatible with any inferred type in `infer_cells` (or equivalently in the Utf8 path here), matching what we already do for numeric.

## Notes on what is NOT a bug

- **`Analysis` timeouts (300s+)** — these are queries that load a 4.8M-row table then filter in M without folding. Excel also takes 3-7 minutes on these. Pattern is wrong, not engine.
- **`IM002` "data source not found"** — DSN isn't installed on this machine (`OCS1`, possibly others). Environmental.

## Recently fixed (left here briefly for reference; trim periodically)

- ~~`unsupported SQL type Bit`~~ — DBISAM logical columns. Fixed: now maps to `DataType::Boolean` via `BufferDesc::Bit`. Hit `category/Customer`, `cots/{CUSTOMER, ORDERH, ORDERI, PRODUCT}`, etc. Some still fall back to slow row-at-a-time via B1.
- ~~Process abort: 2 TiB columnar buffer allocation~~ — DBISAM reports `LongVarchar { length: i32::MAX }` for memo columns; multiplied by `BATCH_SIZE = 1024` the allocator asked for 2 TiB and the process died before `catch_unwind` could save it. Fixed by capping declared text lengths at `CELL_CAP = 64 KiB` in `describe_columns`. Columnar bind now always succeeds; if fetch then panics, the existing fallback path handles it.
- ~~`#table(n, rows)` numeric-first-arg overload~~ — auto-names `Column1..ColumnN`.
- ~~Currency symbols in text→number cast~~ — `£ $ € ¥` stripped.
- ~~Empty text → null on numeric cast~~ — `""` and whitespace-only cells become null.
- ~~`Text.Proper(null) = null`~~ — was rejecting nulls; now passes through.
