# NEXT_STEPS.md

Working handoff for the native **Exportmaster (DBISAM) memo/blob fetch**. Written
2026-05-30. Lives at repo root so it can be cross-referenced alongside the C#
work once this repo is checked out on the same box (sem04).

> Honesty rule for this file: nothing below is described as "working" unless it
> has been verified. Known-incomplete pieces are listed as such, with file:line.
> The whole reason this doc exists is that the connector was previously presented
> as complete while memo/blob columns silently returned wrong data.

---

## 1. The task

Implement **memo/blob content fetch** for the native Exportmaster connector
(reqcode `0x0280` OpenBlob / `0x028A` FreeBlob — `DBISAM-PROTOCOL.md` §6a).
Today a memo/blob column resolves to its 8-byte server-side **handle**, not its
content. The fetch is a separate per-cell round-trip against the live cursor.

## 2. What is NOT real yet (silent placeholders — fix or fail loudly)

- **`mrsflow-cli/src/exportmaster/blob.rs`** — empty stub (`// TODO: implement`).
- **`mrsflow-cli/src/exportmaster/row.rs:198-207`** — decodes Blob/Memo/Graphic to
  `CellValue::BlobHandle([u8;8])`.
- **`row.rs:303`** — pushes those **8 handle bytes** into the column as if they
  were the value. ⇒ selecting a memo column returns 8 meaningless bytes, **no error**.
- **`row.rs:209-216`** — `VarBytes` decode is an unverified guess ("refine when a
  real VarBytes column surfaces"). Will mis-decode silently.
- **ODBC route `mrsflow-cli/src/lib.rs:1501`** — DBISAM memos arrive as
  `LongVarbinary` and are force-decoded to UTF-8 **text capped at 64 KiB**; true
  fixed/var `Binary` SQL types are unsupported (hit the `unsupported SQL type`
  error). Different route, same "not really fetching blobs."

### Proposed interim (pending matthew's go-ahead)
Make the blob/memo column path **hard-error**
(`"Exportmaster: memo/blob content fetch (0x0280) not implemented — column <name>"`)
instead of returning handle bytes. Turns a hidden stub into an honest failure.
When the real fetch lands, this guard flips to the working path.

## 3. What is blocking — and the safe way through

The **exact `0x0280` request byte layout is unknown.** §6a describes it in prose;
the inner-len arithmetic does not reconcile (`0x59`=89 vs ~86 by hand), some fields
are shown with their Pack length-prefix and others without, and **there is no
capture in `.em_tmp/cap/` to diff against.**

- **DO NOT fuzz candidate requests at a live server.** rivsem04 was crashed in
  another session by poking it. Sending uncertain/malformed requests at a
  production ERP box is how that happens.
- **DO get ground truth from a capture.** matthew reports the **C# version has
  solved `0x0280`** (and that real captures will be needed). The official
  Exportmaster ODBC driver (pyodbc / DBSYS) sends *well-formed* requests — capture
  one reading a memo column and read the bytes straight off it. Non-destructive,
  and gives an offline fixture to unit-test against.

### Cross-reference targets (on sem04)
- C# project that solved `0x0280` — **confirm path** (likely `CS-EM2Parquet`;
  Exportmaster memo→Parquet would need exactly this). Port the request layout +
  any row-identity details verbatim from there.
- `DBISAM-PROTOCOL.md` §6a (fetch), §6b (field-type `sub` codes), §6c (framing).
- Capture rig: `.em_tmp/cap/*.pcapng`, `Oracle/capture_mrsflow.ps1`.

## 4. Port plan (once the captured layout is in hand)

1. `msg::build_open_blob(cursor_handle, col_ordinal, row_hash, primary_key)` +
   `build_free_blob(...)` in `msg.rs`, matching the **captured** bytes exactly.
   → verify: unit test round-trips the captured request bytes.
2. CP1252 decode for `ftMemo` text (`+0xA8 == 0x16`); raw bytes for `ftBlob`
   (`0x00`) / `ftGraphic` (`0x1A`). Small hand-rolled CP1252 table avoids a new
   dep (CLAUDE.md: minimum deps).
3. Wire into the fetch path — fetch blobs **after** the row scan but **before**
   `CloseCursor` (handles are only valid against the live cursor;
   `client.rs:205` is the close). Resolve `BlobHandle` → content in the column
   builder. → verify: `SELECT … TOP 3` over NIINGRED shows real ingredient text,
   not 8 bytes.
4. → verify non-ASCII: a memo with the Polish `ł` ("Spółka") decodes correctly.

### Key facts already established (so the C# cross-ref is faster)
- Row identity for the fetch IS available per wire row: **MD5 hash at
  `row[9..25]`**, **PK = first field**. The RecordBlock wire rows include the
  25-byte on-disk header (`read_record_block_batch` in `response.rs`; the decode
  in `client.rs:189` slices `row[first_off..]` with `first_off=25`, which only
  works because the header is present). NB: `cursor.rs:221`'s "header absent"
  note is about the legacy `find_row_starts` heuristic, **not** this path.
- Per-row **bookmarks** are also on the wire but currently discarded
  (`response.rs:200-201`) — check the C# to see whether `0x0280` keys on the
  hash+PK (per §6a) or on the bookmark.
- **Cost:** one `0x0280`+`0x028A` per blob cell. A memo column over N rows = N
  extra round-trips — only sane for small/TOP-N result sets. The ODBC route
  sidesteps this (memo inline in the batch); the native route cannot.

## 5. Target for the live read (when layout is known-good)

- Table **`NISAINT_CS\NIINGRED`** — `NIEAN` is the barcode key; a memo column
  holds ingredient text. §6a's verified example: `NIEAN="00715677478441"` →
  "Sugar, glucose…". Connection (native): `Exportmaster.Database("<host>", …)`
  user `e3user`. **Host TBD by matthew** — not rivsem01, not a fuzz target.

---

## Done this session (for continuity)
- **#3 dialect-from-backend** (`1aed2c2`): `Odbc.DataSource` detects
  `SQL_DBMS_NAME` and routes DBISAM through the DCG-validated `Dbisam` dialect
  instead of hardcoded `GenericOdbc`. Build green; ODBC path latent (not yet
  reachable end-to-end), so *not* claimed working live.
- **Workstation-name rename** (`c5b5f2b`): the Connect handshake's cosmetic
  field-3 label was the inline literal `"RIVSEM048692"` — now a named
  `WORKSTATION_NAME` const documented as **not a target host** (it caused a real
  "are we attaching to rivsem04?" scare). Compiles.
