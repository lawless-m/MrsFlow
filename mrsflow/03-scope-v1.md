# 03 — Scope of v1

## The scoping principle

**v1 reads Parquet and ODBC, writes Parquet.** Inputs to M are either (a) Parquet files bound to named identifiers via CLI args (works in both CLI and WASM shells) or (b) live ODBC queries via `Odbc.DataSource` / `Odbc.Query` calls inside M code (CLI shell only — ODBC has no browser story). Output is a single Parquet file via CLI arg.

The two channels are complementary: file-based input gives `--in name=path.parquet` ergonomics that don't require an ODBC stack; M-code-driven ODBC matches the shape of Excel-generated M (which calls `Odbc.DataSource(...)` heavily) and unlocks live database access without a Parquet pre-stage.

This still radically reduces scope versus "implement all of M":
- No CSV parsing at the M level (locale, headers, type inference are all painful — go through DuckDB ODBC if you need CSV)
- No JSON, no Excel reading, no Web.Contents
- No native database connectors beyond ODBC (`Sql.Database`, `PostgreSQL.Database`, etc. — use `Odbc.DataSource` against the appropriate DSN instead)
- No type coercion swamp at the Parquet boundary (Parquet has typed schemas; M operates on already-typed data). ODBC type mapping is a smaller swamp the evaluator handles directly.

## CLI contract

```
mrsflow run query.pq --in customers=customers.parquet --in sales=sales.parquet --out result.parquet
```

- `query.pq` is an M expression.
- `--in name=path.parquet` makes the Parquet file available as an M identifier (zero or more occurrences).
- `--out path.parquet` is where the result is written.
- Exit code 0 on success, non-zero with structured error on failure.

ODBC-driven inputs need no CLI flag — they're expressed in the M source itself:

```
let
    Source = Odbc.DataSource("DSN=warehouse"),
    Orders = Source{[Name="orders"]}[Data]
in
    Table.SelectRows(Orders, each [Total] > 100)
```

Composable: chain invocations to build pipelines. Each step is a code-reviewable .pq file plus a CLI command. ODBC connections are opened per-invocation by default.

## Language features needed (provisional)

The user's actual query corpus determines this — see `05-open-questions.md`. As a starting estimate for what's likely required:

**Core language:**
- `let ... in ...` expressions
- `if ... then ... else ...`
- Lambda functions (`(x) => expr`)
- All primitive literals: number, text, logical, date, datetime, null
- Records and record literals (`[Name = "x", Age = 30]`)
- Lists and list literals
- Field access (`record[field]`)
- Arithmetic, comparison, logical operators
- `each` and `_` shorthand for single-arg lambdas

**Standard library (the likely 80/20):**
- `Table.*`: `SelectRows`, `SelectColumns`, `RemoveColumns`, `AddColumn`, `RenameColumns`, `Sort`, `Group`, `Join`, `NestedJoin`, `Distinct`, `RowCount`, `FromRecords`, `ToRecords`
- `List.*`: `Sum`, `Average`, `Count`, `Min`, `Max`, `Distinct`, `Contains`, `Select`, `Transform`
- `Record.*`: `Field`, `FieldNames`, `AddField`, `RemoveFields`, `RenameFields`
- `Text.*`: `From`, `Upper`, `Lower`, `Trim`, `Replace`, `Contains`, `StartsWith`, `EndsWith`, `Length`, `Split`
- `Number.*`: `From`, `Round`, `Abs`
- `Date.*`: `From`, `Year`, `Month`, `Day`, `AddDays`, `AddMonths`
- Type conversion: `Number.From`, `Text.From`, `Date.From`, `Logical.From`

Plus `Parquet.Document(path)` (or equivalent) as the only data source. Naming convention TBD — likely mirror Microsoft's `X.Document(File.Contents(path))` pattern even though Microsoft has no native Parquet equivalent, for query portability.

This is roughly 40-50 functions. Real corpus will trim or extend it.

## Out of scope for v1

- All non-Parquet sources
- Type ascription syntax (`as table`, `as number` in declarations)
- Custom types and type expressions beyond basic primitives
- Sections (the modularity feature; spec mentions M doesn't really use them anyway)
- The full long tail of stdlib functions (~600+ in Microsoft's implementation)
- Performance optimisation beyond "doesn't fall over on reasonable inputs"
- Pretty-printing / formatting / language services
- Error message quality beyond "tells you which line and roughly what went wrong"

These are all reasonable v2+ work. Don't let them creep into v1.

## Definition of "v1 done"

The user's existing query corpus, modified only to read Parquet inputs instead of original sources, runs through `mrsflow` and produces output that diffs cleanly against Microsoft M's output for the same queries. The CLI is usable in a CI pipeline. WASM build runs in a browser against a fetched Parquet file.

That's the bar. Not feature-completeness against the spec.
