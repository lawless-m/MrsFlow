# 03 — Scope of v1

## The scoping principle

**v1 is "M over Parquet."** Inputs are one or more Parquet files bound to named identifiers. Output is a Parquet file. Nothing else.

This radically reduces scope versus "implement M":
- No CSV parsing (locale, headers, type inference are all painful)
- No JSON, no Excel reading, no SQL connectors, no Web.Contents
- No ODBC (the existing DBISAM bridge handles legacy data upstream)
- No type coercion swamp (Parquet has typed schemas; M operates on already-typed data)

## CLI contract

```
mrsflow run query.pq --in customers=customers.parquet --in sales=sales.parquet --out result.parquet
```

- `query.pq` is an M expression.
- `--in name=path.parquet` makes the Parquet file available as an M identifier.
- `--out path.parquet` is where the result is written.
- Exit code 0 on success, non-zero with structured error on failure.

Composable: chain invocations to build pipelines. Each step is a code-reviewable .pq file plus a CLI command.

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
