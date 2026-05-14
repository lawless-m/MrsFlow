# MrsFlow

A Rust implementation of Power Query M. Parses M source, evaluates it
against in-memory Arrow tables, reads/writes Parquet, and talks to
databases over ODBC and native protocols (MySQL, PostgreSQL). Ships as
a CLI binary and a WebAssembly module that share the same evaluator
core.

The thesis, in one line: **M is a good language for tabular data; it
shouldn't be trapped inside Excel.**

## Status

Pre-v1. Single-author internal tool — no API stability, no release
cadence, no external contributors. The shape of v1 is documented in
[`mrsflow/03-scope-v1.md`](mrsflow/03-scope-v1.md): Parquet → M →
Parquet via the CLI, with ODBC for live database reads. Everything
beyond that is opportunistic.

## What works today

**Language**: `let ... in`, `if`/`then`/`else`, lambdas (`(x) => …` and
`each`), records, lists, field access, nested record/list literals,
all primitive types (number/text/logical/date/datetime/duration/null/
binary), arithmetic and comparison operators, the cycle detector, and
identifier-named functions imported from input bindings.

**Standard library** — ~36 namespaces, ~600 callable functions.
Numbers are bindings actually registered in `mrsflow-core/src/eval/stdlib/`.

*Tabular data:*

| Namespace      | Fns | Examples                                                       |
| -------------- | ---:| -------------------------------------------------------------- |
| `Table.*`      | 114 | `SelectRows`, `SelectColumns`, `Sort`, `Group`, `NestedJoin`, `ExpandTableColumn`, `Pivot`, `RowCount`, `AddColumn`, `PromoteHeaders`, `FromRecords`, `ToRecords` |
| `List.*`       |  72 | `Sum`, `Average`, `Count`, `Distinct`, `Transform`, `Select`, `Accumulate`, `Contains` |
| `Record.*`     |  18 | `Field`, `FieldNames`, `AddField`, `RemoveFields`, `RenameFields`, `Combine` |

*Scalars:*

| Namespace      | Fns | Examples                                                       |
| -------------- | ---:| -------------------------------------------------------------- |
| `Text.*`       |  42 | `From`, `Upper`/`Lower`, `Trim`, `Replace`, `Contains`, `Length`, `Split`, `BeforeDelimiter`, `BetweenDelimiters` |
| `Number.*`     |  41 | `From`, `Round`, `Abs`, `ToText`, full trig (`Sin`/`Cos`/`Atan`/…) |
| `Binary.*`     |  18 | `ToText`/`FromText` with `BinaryEncoding`, `Compress`, `Buffer`, `Combine` |
| `Logical.*`    |   4 | `From`, `FromText`, `ToText`, `Type`                           |

*Temporal:*

| Namespace          | Fns | Examples                                              |
| ------------------ | ---:| ----------------------------------------------------- |
| `Date.*`           |  58 | `AddDays`, `AddMonths`, `Year`, `ToText`, custom-format |
| `DateTime.*`       |  26 | `From`, `FixedLocalNow`, `AddZone`, `ToText`           |
| `DateTimeZone.*`   |  16 | `From`, `FixedUtcNow`, `FromFileTime`                  |
| `Duration.*`       |  13 | `Days`, `Hours`, `FromText`, `ToText`                  |
| `Time.*`           |  10 | `From`, `Hour`, `EndOfHour`, `ToText`                  |

*Type system:*

| Namespace      | Fns | Examples                                                       |
| -------------- | ---:| -------------------------------------------------------------- |
| `Value.*`      |  26 | `Compare`, `Equals`, `As`, `Type`, `FromText`                  |
| `Type.*`       |  25 | `ForFunction`, `ClosedRecord`, `IsNullable`, `Facets`          |

*Document parsing & serialisation:*

| Namespace      | Fns | Notes                                                                  |
| -------------- | ---:| ---------------------------------------------------------------------- |
| `Json.*`       |   2 | `Document`, `FromValue`                                                |
| `Csv.*`        |   1 | `Document` with `Delimiter`/`QuoteStyle`                               |
| `Xml.*`        |   2 | `Document`, `Tables`                                                   |
| `Html.*`       |   1 | `Table` — CSS-selector extraction                                      |
| `Lines.*`      |   4 | `FromBinary`/`ToBinary`, `FromText`/`ToText`                           |

*IO and database connectors:*

| Namespace      | Fns | Notes                                                                  |
| -------------- | ---:| ---------------------------------------------------------------------- |
| `Parquet.*`    |   1 | `Document` — lazy, predicate pushdown                                  |
| `Odbc.*`       |   3 | `DataSource` (flat + nested nav), `Query`, `InferOptions` — fold-aware |
| `MySQL.*`      |   2 | `Database`, `Query` — native protocol, rustls TLS                      |
| `PostgreSQL.*` |   2 | `Database`, `Query` — native protocol, rustls TLS, lossless NUMERIC    |
| `Excel.*`      |   3 | `Workbook` (.xlsx parse), `CurrentWorkbook`, `ShapeTable`              |
| `Web.*`        |   4 | `Contents`, `Headers`, `Page`, `BrowserContents`                       |
| `File.*`       |   1 | `Contents`                                                             |
| `Folder.*`     |   — | `Contents`, `Files`                                                    |

*Combinators and meta:*

| Namespace      | Fns | Examples                                                       |
| -------------- | ---:| -------------------------------------------------------------- |
| `Splitter.*`   |  10 | `SplitTextByDelimiter`, `SplitTextByCharacterTransition`       |
| `Function.*`   |   7 | `Invoke`, `InvokeAfter`, `From`                                |
| `Combiner.*`   |   5 | `CombineTextByDelimiter`, `CombineTextByLengths`               |
| `Comparer.*`   |   4 | `Equals`, `Ordinal`, `OrdinalIgnoreCase`, `FromCulture`        |
| `Uri.*`        |   4 | `Parts`, `BuildQueryString`, `EscapeDataString`, `Combine`     |
| `Expression.*` |   3 | `Evaluate`, `Constant`, `Identifier`                           |
| `Diagnostics.*`|   3 | `Trace`, `ActivityId`, `CorrelationId`                         |
| `Replacer.*`   |   2 | `ReplaceText`, `ReplaceValue`                                  |
| `Variable.*`   |   2 | `Value`, `ValueOrDefault`                                      |
| `Error.*`      |   — | error-record helpers                                           |

Enum-style tokens are also registered: `BinaryEncoding`, `TextEncoding`,
`JoinKind`, `Order`, `QuoteStyle`, `TraceLevel`, `ExtraValues`,
`Compression`, `Day`, plus type tokens (`Int64.Type`, `Decimal.Type`,
`Currency.Type`, …).

For the full surface, see
[`mrsflow/stdlib-reference/`](mrsflow/stdlib-reference/) or run
`tools/stdlib_coverage.py`.

**Predicate folding**: `Table.SelectRows` and `Table.SelectColumns`
push down into Parquet (row-group elimination via statistics) and into
ODBC (SQL `WHERE` clause + projection). The foldable subset is
literal-RHS comparisons AND'd together; non-foldable predicates fall
back transparently to in-memory filtering.

## A short M query

```m
let
    Source = Parquet.Document("/data/sales.parquet"),
    GB = Table.SelectRows(Source, each [Country] = "GB"),
    ByRegion = Table.Group(
        GB,
        {"Region"},
        {{"Total", each List.Sum([Amount]), type number}})
in
    ByRegion
```

Save as `gb.m`, then:

```bash
mrsflow gb.m -o by-region.parquet
```

The `SelectRows` predicate folds into Parquet — only row groups whose
`Country` statistics include `"GB"` get read.

## Repo layout

| Path             | What it is                                                                 |
| ---------------- | -------------------------------------------------------------------------- |
| `mrsflow-core/`  | Pure evaluator. Lexer, parser, AST, eval, stdlib. No IO.                   |
| `mrsflow-cli/`   | CLI shell. Filesystem + Parquet + (optional) ODBC + MySQL + PostgreSQL.    |
| `mrsflow-wasm/`  | Browser shell. Same evaluator, different IO host.                          |
| `mrsflow/`       | Design docs (`01-overview` … `09-lazy-tables`) and a stdlib reference.     |
| `Oracle/`        | Differential testing against real Power Query in Excel — see below.        |
| `examples/`      | Real M queries pulled from production work (untracked, machine-local).     |
| `parqs/`         | Sample Parquet inputs (untracked, machine-local).                          |
| `tools/`         | Coverage scripts, grammar fuzzer, MS-docs mirror.                          |
| `vendor/`        | Local fork of `odbc-api` with a patched `Indicator::from_isize`.           |

## Architecture

```
                ┌────────────────────────────────────┐
                │           mrsflow-core             │
                │                                    │
                │  lexer → parser → AST → evaluator  │
                │                          │         │
                │                          ▼         │
                │              stdlib (~40 modules)  │
                │                          │         │
                │                          ▼         │
                │        TableRepr: Arrow | Rows |   │
                │          LazyParquet | LazyOdbc |  │
                │           JoinView | ExpandView    │
                │                                    │
                │             IoHost trait           │
                └──────────────┬─────────────────────┘
                               │ (no IO above this line)
            ┌──────────────────┼──────────────────┐
            ▼                                     ▼
   ┌──────────────────┐               ┌──────────────────────┐
   │   mrsflow-cli    │               │    mrsflow-wasm      │
   │   CliIoHost      │               │     WasmIoHost       │
   │                  │               │                      │
   │ - Parquet IO     │               │ - browser fetch      │
   │ - ODBC           │               │ - parquet-wasm       │
   │ - MySQL native   │               │ - IndexedDB          │
   │ - PostgreSQL     │               │                      │
   └──────────────────┘               └──────────────────────┘
```

The evaluator is synchronous and pure — no `tokio`, no filesystem, no
clock. All side effects live in the shell's `IoHost` implementation.
This makes the WASM build trivial and the test harness deterministic.

`TableRepr` is the variant type behind every M table value. `Arrow`
and `Rows` are eager; `LazyParquet` and `LazyOdbc` carry deferred
plans that fold predicates and projections before any data flows;
`JoinView` and `ExpandView` defer join/expand materialisation until
forced. See [`mrsflow/09-lazy-tables.md`](mrsflow/09-lazy-tables.md).

## Quick start

Build the workspace:

```bash
cargo build --release
```

Run a query:

```bash
mrsflow query.m                     # print result to stdout
mrsflow query.m -o output.parquet   # write Table result to Parquet
```

Bind Parquet inputs as M identifiers:

```bash
mrsflow query.m --in customers=customers.parquet --in sales=sales.parquet
```

Database connectors are feature-gated:

```bash
cargo build --release --features "odbc mysql postgresql"
```

- **ODBC** needs an ODBC driver manager. `apt install unixodbc-dev` on
  Debian, built in on Windows. Then install whatever driver you need
  (DBISAM, DuckDB ODBC, Postgres ODBC, etc.).
- **MySQL** and **PostgreSQL** use pure-Rust drivers (`mysql` and
  `tokio-postgres`) with `rustls` for TLS. No system OpenSSL.

## Testing

Three tiers:

- **Unit tests** — `cargo test`. Covers the evaluator, stdlib
  functions, predicate folding, SQL emission. ~630 tests, runs
  without external services.
- **Predicate-fold engagement tests** — inject a dummy `LazyOdbc` /
  `LazyParquet` whose force function panics. If folding fails and
  forces unexpectedly, the test explodes; if it folds correctly, the
  panic never fires.
- **Oracle differential tests** — `Oracle/` runs the same M expressions
  through real Power Query in Excel (Windows) and through mrsflow,
  then diffs the results. `Oracle/Oracle.m` is a single Catalog query
  of `{Q, Result}` rows; `Oracle/QueryOracle.ps1` refreshes the
  workbook and dumps per-case `cases/qN.excel.out` files for diffing
  against mrsflow's `cases/qN.mrsflow.out`.

Unit tests catch internal regressions; the Oracle catches semantic
divergence from Microsoft's M. Both are needed — driver quirks
(DBISAM SQLLEN, DuckDB column-name truncation) and option-record
handling (`HierarchicalNavigation` flat vs nested) only surface
against the real Excel oracle.

## What's deliberately out of scope

- **No async in the evaluator.** All concurrency lives in the shell.
  The evaluator is synchronous and pure; this keeps the WASM build
  trivial and the test harness deterministic.
- **No SQL injection guards on `*.Query` connectors.** `Odbc.Query`,
  `MySQL.Query`, `PostgreSQL.Query` take raw SQL strings — the M
  layer trusts its caller. Escaping is the user's responsibility.
- **No 100% M compatibility.** The goal is "the queries we actually
  write, run correctly." Microsoft-specific connectors
  (`Salesforce.*`, `SharePoint.*`, etc.) and the corners of the M
  spec that nobody uses in practice are skipped.
- **No backwards-compatibility shims.** Pre-v1; renames and removals
  are free. Don't depend on internal APIs.

## Design docs

[`mrsflow/CONTENTS.md`](mrsflow/CONTENTS.md) indexes the full series:

- `01-overview.md` — thesis, what mrsflow is and isn't.
- `02-architecture.md` — workspace shape, IoHost trait, shells.
- `03-scope-v1.md` — what v1 covers and what it deliberately doesn't.
- `04-test-harness.md` — testing strategy.
- `05-open-questions.md` — unresolved decisions.
- `06-resources.md` — references (M spec, Arrow, Parquet, etc.).
- `07-evaluator-design.md` — AST, environments, lazy values.
- `08-prolog-differential.md` — the Prolog companion evaluator used as
  a second-opinion oracle.
- `09-lazy-tables.md` — `TableRepr` variants, predicate folding.

## License

MIT. See [`LICENSE`](LICENSE).
