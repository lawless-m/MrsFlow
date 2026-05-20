# MrsFlow

A Rust implementation of the Power Query **M** language. Parses M source,
evaluates it against in-memory Arrow tables, reads and writes Parquet,
and talks to databases over ODBC and native protocols (MySQL,
PostgreSQL). Ships as a CLI binary and a WebAssembly module that share
one evaluator core.

The thesis, in one line: **M is a good language for tabular data; it
shouldn't be trapped inside Excel.**

```m
let
    Source   = Parquet.Document("/data/sales.parquet"),
    GB       = Table.SelectRows(Source, each [Country] = "GB"),
    ByRegion = Table.Group(GB, {"Region"},
                 {{"Total", each List.Sum([Amount]), type number}})
in
    ByRegion
```

```bash
mrsflow gb.m -o by-region.parquet
```

The `SelectRows` predicate folds into the Parquet read — only row groups
whose `Country` statistics include `"GB"` are decoded.

## Verified against real Power Query

The thing that makes this more than a weekend M interpreter: **every
stdlib function is differentially tested against Microsoft's own
implementation.** `Oracle/` runs the same M expressions through the real
Power Query engine inside Excel and through mrsflow, then diffs the
results byte-for-byte.

|                                              |        |
| -------------------------------------------- | ------:|
| Oracle test cases (one M expression each)    | 1,525  |
| Matching Excel byte-for-byte                 | 1,522  |
| Known divergences (documented, see below)    |     3  |
| PQ `#shared` names implemented                |   774  |
| …with at least one oracle test                |   745  |
| …untested (clock-tight / connector-only)      |    29  |
| PQ names deliberately unimplemented           |    80  |

The three remaining divergences are catalogued in
[`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) — none is a silent
wrong answer; each is a structural difference (the `#shared` catalogue
listing itself, a relative-path error message, and one connector-only
function-argument shape).

This matters because M has corners no documentation describes
accurately. The oracle caught dozens of them: enum constants whose
ordinals don't match the docs, `BinaryFormat` numerics defaulting to
big-endian, `Single.From` losing precision through a 32-bit round-trip,
`Value.Lineage` returning a record where `Value.Traits` returns a list.
You only find these by asking Excel. See
[`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) for the catalogue.

## Status

**v1.** The goal set out in
[`mrsflow/03-scope-v1.md`](mrsflow/03-scope-v1.md) — Parquet → M →
Parquet via the CLI, with ODBC for live database reads — is met, and the
745-of-774 oracle-verified function surface backs it up. Single-author
tool: still no formal release cadence, and the API may shift between
versions, but the data-shaping core is done and proven against real
Power Query.

## What works today

**Language**: `let … in`, `if`/`then`/`else`, lambdas (`(x) => …` and
`each`), records, lists, field and item access, nested literals, all
primitive types (number / text / logical / date / datetime /
datetimezone / time / duration / null / binary), the full operator set,
the cycle detector, and identifier-named functions imported from input
bindings.

**Standard library** — 99 namespaces, 778 callable names registered in
`mrsflow-core/src/eval/stdlib/`. Highlights:

| Area        | Namespaces                                                      |
| ----------- | -------------------------------------------------------------- |
| Tabular     | `Table.*` (114), `List.*` (72), `Record.*` (18)                |
| Scalar      | `Text.*` (42), `Number.*` (41), `Binary.*` (18), `Logical.*`   |
| Temporal    | `Date.*` (58), `DateTime.*`, `DateTimeZone.*`, `Duration.*`, `Time.*` |
| Types       | `Value.*` (26), `Type.*` (25), and the full `*.Type` token set |
| Documents   | `Json.*`, `Csv.*`, `Xml.*`, `Html.*`, `Lines.*`                |
| Connectors  | `Parquet.*`, `Odbc.*`, `MySQL.*`, `PostgreSQL.*`, `Excel.*`, `Web.*`, `File.*`, `Folder.*` |
| Geo         | `Geography.*`, `Geometry.*`, `GeographyPoint.*`, `GeometryPoint.*` (WKT POINT round-trips) |
| Combinators | `Splitter.*`, `Combiner.*`, `Comparer.*`, `Replacer.*`         |
| Meta        | `Function.*`, `Expression.*`, `RowExpression.*` (AST reflection), `Diagnostics.*`, `Uri.*`, `Variable.*` |

For the exact surface and per-function oracle status, see
[`Oracle/coverage/COVERAGE.md`](Oracle/coverage/COVERAGE.md) (auto-generated)
or [`docs/COVERAGE.md`](docs/COVERAGE.md) (the human summary).

**Predicate folding**: `Table.SelectRows` and `Table.SelectColumns`
push down into Parquet (row-group elimination via column statistics)
and into ODBC (SQL `WHERE` + projection). The foldable subset is
literal-RHS comparisons AND'd together; anything else falls back
transparently to in-memory filtering.

## Quick start

```bash
cargo build --release

mrsflow query.m                                    # print result to stdout
mrsflow query.m -o output.parquet                  # write a Table result to Parquet
mrsflow query.m --in customers=customers.parquet   # bind a Parquet file as an M identifier
```

Database connectors are feature-gated:

```bash
cargo build --release --features "odbc mysql postgresql"
```

- **ODBC** needs a driver manager (`apt install unixodbc-dev` on Debian;
  built in on Windows) plus whatever driver you target (DBISAM, DuckDB
  ODBC, Postgres ODBC, …).
- **MySQL** / **PostgreSQL** use pure-Rust drivers (`mysql`,
  `tokio-postgres`) with `rustls` for TLS — no system OpenSSL.

## Architecture

```
                ┌────────────────────────────────────┐
                │           mrsflow-core              │
                │                                     │
                │  lexer → parser → AST → evaluator   │
                │                          │          │
                │                          ▼          │
                │              stdlib (~45 modules)   │
                │                          │          │
                │                          ▼          │
                │        TableRepr: Arrow | Rows |    │
                │          LazyParquet | LazyOdbc |   │
                │           JoinView | ExpandView     │
                │                                     │
                │             IoHost trait            │
                └──────────────┬──────────────────────┘
                               │ (no IO above this line)
            ┌──────────────────┼──────────────────┐
            ▼                                      ▼
   ┌──────────────────┐               ┌──────────────────────┐
   │   mrsflow-cli    │               │     mrsflow-wasm     │
   │   CliIoHost      │               │      WasmIoHost      │
   │                  │               │                      │
   │ - Parquet IO     │               │ - browser fetch      │
   │ - ODBC           │               │ - parquet-wasm       │
   │ - MySQL native   │               │ - IndexedDB          │
   │ - PostgreSQL     │               │                      │
   └──────────────────┘               └──────────────────────┘
```

The evaluator is **synchronous and pure** — no `tokio`, no filesystem,
no clock. Every side effect lives in the shell's `IoHost`
implementation. That keeps the WASM build trivial and the test harness
deterministic.

`TableRepr` is the variant type behind every M table value. `Arrow` and
`Rows` are eager; `LazyParquet` and `LazyOdbc` carry deferred plans that
fold predicates and projections before any data flows; `JoinView` and
`ExpandView` defer materialisation until forced. See
[`mrsflow/09-lazy-tables.md`](mrsflow/09-lazy-tables.md).

## Repo layout

| Path             | What it is                                                              |
| ---------------- | ----------------------------------------------------------------------- |
| `mrsflow-core/`  | Pure evaluator. Lexer, parser, AST, eval, stdlib. No IO.                |
| `mrsflow-cli/`   | CLI shell. Filesystem + Parquet + (optional) ODBC / MySQL / PostgreSQL. |
| `mrsflow-wasm/`  | Browser shell. Same evaluator, different IO host.                       |
| `mrsflow/`       | Design-doc series (`01-overview` … `09-lazy-tables`) + stdlib reference.|
| `docs/`          | Reader-facing docs: contributing, coverage, compatibility, changelog.   |
| `Oracle/`        | Differential testing against real Power Query in Excel.                 |
| `examples/`      | Real M queries from production work (machine-local, untracked).         |
| `tools/`         | Coverage scripts, grammar fuzzer, MS-docs mirror.                       |
| `vendor/`        | Local fork of `odbc-api` with a patched `Indicator::from_isize`.        |

## Documentation

**Reader-facing** (`docs/`):
- [`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) — how to add a stdlib
  function and verify it against the oracle.
- [`docs/COVERAGE.md`](docs/COVERAGE.md) — what's implemented, what's
  oracle-tested, and what's deliberately left out (with reasons).
- [`docs/COMPATIBILITY.md`](docs/COMPATIBILITY.md) — where mrsflow
  intentionally diverges from Excel, and the misleading-docs findings
  the oracle surfaced.
- [`docs/CHANGELOG.md`](docs/CHANGELOG.md) — narrative of the work.

**Design series** ([`mrsflow/CONTENTS.md`](mrsflow/CONTENTS.md)):
`01-overview`, `02-architecture`, `03-scope-v1`, `04-test-harness`,
`05-open-questions`, `06-resources`, `07-evaluator-design`,
`08-prolog-differential`, `09-lazy-tables`.

## Testing

Three tiers:

- **Unit tests** — `cargo test`. Evaluator, stdlib, predicate folding,
  SQL emission. ~630 tests, no external services.
- **Predicate-fold engagement tests** — inject a dummy `LazyOdbc` /
  `LazyParquet` whose force function panics. If folding fails and forces
  unexpectedly, the test explodes; if it folds correctly, the panic
  never fires.
- **Oracle differential tests** — `Oracle/` (see above). Unit tests
  catch internal regressions; the oracle catches semantic divergence
  from Microsoft's M. Both are needed.

## What's deliberately out of scope

- **No async in the evaluator.** All concurrency lives in the shell.
- **No SQL-injection guards on `*.Query` connectors.** They take raw SQL;
  the M layer trusts its caller.
- **No 100% M compatibility.** The 80 unimplemented PQ functions are
  almost entirely cloud connectors (Salesforce, SharePoint, Azure, SAP,
  …) that need a backend mrsflow doesn't have. See
  [`docs/COVERAGE.md`](docs/COVERAGE.md).
- **No backwards-compatibility promises yet.** v1 is functional, not
  frozen — internal APIs may still be renamed between versions. Don't
  pin to them.

## Contributing & conduct

[`docs/CONTRIBUTING.md`](docs/CONTRIBUTING.md) walks the add-a-function
path. [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) sets out the house rules
— in short: bring your work, let it be judged on whether it makes the
software better, expect plain speech in return.

## License

MIT. See [`LICENSE`](LICENSE).
