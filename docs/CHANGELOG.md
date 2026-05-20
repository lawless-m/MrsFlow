# Changelog

Pre-v1, no released versions. This is a narrative of notable work rather
than a SemVer log — renames and removals are free until v1.

## The oracle-coverage grind (q1393–q1537)

A sustained push to close the gap between "mrsflow has a binding for
this" and "mrsflow matches Excel for this," driven entirely by the
`Oracle/` differential harness. Roughly 140 new test cases and ~50
behavioural fixes. The loop was always the same: probe a function
against Excel, find the divergence, fix mrsflow to match the *engine*
(not the docs), commit the fix plus its un-parked q-case.

Outcome:

| Metric                       | Before | After  |
| ---------------------------- | ------:| ------:|
| Baseline Oracle DIFFs        |     12 |      3 |
| Oracle-tested families       |      0 |     88 |
| Implemented-but-untested     |    ~30 |      6 |

### New stdlib modules

- **`geo.rs`** — `Geography.*` / `Geometry.*` / `GeographyPoint.From` /
  `GeometryPoint.From`. WKT `POINT` parse + construct + serialise.
  Field-order quirk: WKT geography is `(longitude, latitude)`.
- **`row_expression.rs`** — `RowExpression.From` / `ItemExpression.From`
  walk a 1-arg lambda's body into an AST record
  (`{Kind, Operator, Left, Right}`, etc.). `RowExpression.Row` /
  `.Column` / `ItemExpression.Item` are the sentinel/constructor pieces.
  Useful for introspecting a filter predicate without re-parsing source —
  the same path connector authors use for native-query folding.

### Behavioural fixes (mrsflow now matches the engine)

Catalogued in [`COMPATIBILITY.md`](COMPATIBILITY.md). The big ones:

- **`BinaryFormat` numerics default to big-endian** — a single fix that
  cleared 7 of the long-standing baseline DIFFs at once.
- **`BinaryFormat.7BitEncodedSignedInteger` zigzag decoding.**
- **`Single.From` rounds through f32** (`3.14 → 3.140000104904175`).
- **`Binary.From(text)` base64-decodes** instead of taking raw UTF-8.
- **Time/Datetime JSON fractional seconds at .NET-tick precision**
  (7 digits, not 9).
- **~15 enum-ordinal families corrected** (Compression, GroupKind,
  RankKind, Precision, Occurrence, RoundingMode, ExtraValues, …).
- **`WebMethod` → HTTP verb text**, **`TimeZone.Current` → tz name text**,
  **`Culture.Current` → BCP-47 locale**.
- **Enum `.Type` constants rebound to `TypeRep::Type`** (type-of-types),
  with `Type.Is` raising the matching coercion error.
- **`WebAction.Request` rebound from a number to a function value.**
- **Return-shape corrections**: `Value.Lineage` record vs `Value.Traits`
  list, `Binary.InferContentType` null-record, partition-key sentinels,
  `Type.Facets` 10-slot record, `Table.CombineColumnsToRecord` insert
  position.
- **Error-wording matches**: `Variable.Value`, `Value.Versions`,
  `Value.Expression`, `Value.Alternates`, `Excel.Workbook`
  (`DataFormat.Error`), several `*.ViewFunction` type-checks.
- **`List.Alternate` and `List.Times` semantics/signatures** brought in
  line with the engine.
- **`time` / `datetimezone` / `none` / `password` primitive type
  keywords** recognised by the type-position parser.

### Test-harness improvements

- **`gen_status.ps1` scanner widened** to attribute a function to a
  q-case across more delimiter positions (trailing `)`, `}`, `[`,
  newline, end-of-file) — was previously blind to names in terminal
  position.
- **`coverage.m` strips harness leakage** — the `EvalFile` workbook
  wrapper and a malformed "Invoked FunctionEvalFile" row no longer
  pollute the dashboard.
- **`docs/` reader-facing set added** — this changelog plus
  CONTRIBUTING, COVERAGE, COMPATIBILITY.

### Deliberate non-fixes

A few divergences were investigated and left alone with a documented
reason rather than a workaround: the `#shared` catalogue dump (q1165),
`File.Contents` relative-path wording (q1167, the dashboard loader needs
it), and `BinaryFormat.Group`'s argument shape (q1179). See
[`COMPATIBILITY.md`](COMPATIBILITY.md).

## Earlier foundations

Predating the coverage grind:

- Synchronous, pure evaluator with an `IoHost` trait boundary; CLI and
  WASM shells over the same core.
- `TableRepr` lazy variants (`LazyParquet`, `LazyOdbc`, `JoinView`,
  `ExpandView`) with predicate folding into Parquet row-group statistics
  and ODBC `WHERE`/projection.
- Vendored `odbc-api` fork patching `Indicator::from_isize` for the
  DBISAM driver's SQLLEN quirk.
- Native MySQL / PostgreSQL drivers with rustls TLS.
- The `Oracle/` differential harness itself, and the Prolog companion
  evaluator (`mrsflow/08-prolog-differential.md`).
