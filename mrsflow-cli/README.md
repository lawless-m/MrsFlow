# mrsflow-cli

Command-line shell for the `mrsflow` M-language evaluator. Reads a `.m`
source file, evaluates it against `CliIoHost` (real filesystem + Parquet
IO + optional ODBC), prints the result as a canonical S-expression or
writes a table result to Parquet.

## Usage

```
mrsflow input.m                 # print result to stdout
mrsflow input.m -o output.pq    # write Value::Table result to Parquet
```

The CLI exits with a non-zero status on parse, evaluation, or IO errors.
Error messages go to stderr.

## ODBC support

`Odbc.Query(connection_string, sql)` is bound in the root environment but
needs an ODBC driver manager at build time. By default the workspace
builds **without** ODBC support, and calls to `Odbc.Query` return a clear
"built without ODBC support" error.

To enable real ODBC:

1. Install the ODBC driver manager + dev headers:
   - **Debian/Ubuntu**: `apt install unixodbc-dev`
   - **macOS**: `brew install unixodbc`
   - **Windows**: built-in (MS ODBC Driver Manager)
2. Install the actual ODBC driver for your database (DuckDB, SQLite,
   DBISAM, etc).
3. Rebuild: `cargo build --features odbc`

Once built with `--features odbc`, calls to `Odbc.Query` will route
through `odbc-api` against the installed driver. Example:

```
let
  source = Odbc.Query("DSN=DuckDB", "SELECT * FROM read_parquet('foo.parquet')"),
  filtered = Table.SelectRows(source, each [year] > 2020)
in
  filtered
```

`Odbc.DataSource` (navigation table form) is not yet implemented — it
needs nested-column support in the table representation, which is a
separate slice.

## Architecture

The core evaluator (`mrsflow-core`) is pure and shells delegate IO via
the `IoHost` trait. `CliIoHost` in this crate implements `parquet_read`,
`parquet_write`, and (feature-gated) `odbc_query`. A future WASM shell
will return `NotSupported` for filesystem-bound methods.

See `mrsflow/07-evaluator-design.md` for the full architecture.
