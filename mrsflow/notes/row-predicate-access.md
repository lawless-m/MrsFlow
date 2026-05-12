# Row-by-record-predicate access — next eval blocker

`Excel.Workbook` and ODBC sources both return a Table-of-sources whose canonical access pattern is `Source{[Item="...",Kind="Sheet"]}[Data]` (or `{[Name=...,Kind="Database"]}`). The full corpus across `examples/cots/` and `examples/JBP/` opens with this.

`ItemAccess` in `mrsflow-core/src/eval/mod.rs` currently supports only numeric indexing on Tables/Lists. When given a Record predicate it errors with `TypeMismatch { expected: "number", found: "record" }`. M's spec defines this as "select the unique row whose listed fields all equal the predicate's; error if zero or multiple match".

**Why this matters:** `Excel.Workbook`, `Json.Document`, and (eventually) `Odbc.DataSource` all landed in stdlib but the corpus can't actually run a sheet query end-to-end without this access form.

**How to fix:** Extend the `Value::Table` arm in `Expr::ItemAccess` to accept `Value::Record(r)` and filter rows by field equality. Must work over both `TableRepr::Arrow` and `TableRepr::Rows` variants (see `02-architecture.md` on the het-cell hybrid).
