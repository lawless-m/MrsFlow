# 06 — Resources

## Language specification

- **Microsoft Learn (current, maintained):** https://learn.microsoft.com/en-us/powerquery-m/power-query-m-language-specification
- **PDF snapshot (July 2019):** https://download.microsoft.com/download/8/1/A/81A62C9B-04D5-4B6D-B162-D28E4D848552/Power%20Query%20M%20Formula%20Language%20Specification%20(July%202019).pdf
- **Spec introduction / quick tour:** https://learn.microsoft.com/en-us/powerquery-m/m-spec-introduction
- **M function reference (Microsoft):** https://learn.microsoft.com/en-us/powerquery-m/

Grab both the PDF and use the Learn version as the live source of truth. The spec covers lexical structure, values, expressions, environments, identifiers, evaluation model, operators, functions, errors, let-expressions, if-expressions, sections, and a consolidated grammar.

## Reference parser (TypeScript, Microsoft-maintained, MIT)

- **Parser:** https://github.com/microsoft/powerquery-parser
- **Language services (built on parser):** https://github.com/microsoft/powerquery-language-services
- **Formatter:** https://github.com/microsoft/powerquery-formatter

Use the parser as a parser-level oracle — feed both the TS parser and the Rust parser the same M source, compare ASTs structurally to catch parsing divergences.

## Community function reference

- **powerquery.io:** https://powerquery.io/ — sometimes more navigable than Microsoft's own docs.

## Headless M execution (for the test oracle)

- **PowerQueryNet:** https://github.com/gsimardnet/PowerQueryNet — .NET wrapper around Microsoft's Mashup Engine. Requires Power BI Desktop or the Power Query SDK installed on Windows. Wraps closed-source Microsoft runtime; fine for testing, not redistributable.

## Rust crates likely needed

**Core:**
- `arrow` — columnar in-memory data (https://crates.io/crates/arrow)
- `parquet` — Parquet read/write, designed to pair with arrow (https://crates.io/crates/parquet)
- `chrono` — date/time types matching M's Date/DateTime semantics (https://crates.io/crates/chrono)

**Parsing (pick one or hand-roll):**
- `chumsky` — parser combinators with good error messages (https://crates.io/crates/chumsky)
- `nom` — older, more established combinators (https://crates.io/crates/nom)
- (Hand-written recursive descent is also reasonable — matches the TS parser's approach.)

**CLI:**
- `clap` — argument parsing (https://crates.io/crates/clap)
- `anyhow` / `thiserror` — error handling at the shell layer (the core has its own error model)
- `odbc-api` — Rust bindings to native ODBC drivers, used by the CLI shell's `IoHost` to back `Odbc.Query` and `Odbc.DataSource` (https://crates.io/crates/odbc-api)

**WASM:**
- `wasm-bindgen` — JS interop (https://crates.io/crates/wasm-bindgen)
- `wasm-pack` — build tooling (https://github.com/rustwasm/wasm-pack)

**Testing:**
- `insta` — snapshot testing for ASTs (https://crates.io/crates/insta)
- Custom Arrow-diff harness for the M-output equivalence checks

## Related project (same user, same context)

- **Serious-DBI-Sam:** https://github.com/lawless-m/Serious-DBI-Sam — DuckDB extension + .NET 8 gRPC bridge for querying legacy DBISAM databases. Solves the 32-bit Windows ODBC problem entirely. mrsflow consumes Parquet produced downstream of this bridge; it does not need its own ODBC story.

## Browser-side Parquet and Arrow (for the WASM build)

- **Arrow JS:** https://arrow.apache.org/docs/js/ — `apache-arrow` on npm, native JS implementation with efficient buffer sharing with WASM.
- **DuckDB-Wasm:** https://duckdb.org/docs/api/wasm/overview.html — already used by the user's existing browser projects. Can register Arrow tables as queryable views, so mrsflow output can become SQL input without leaving the browser.
- **parquet-wasm:** https://github.com/kylebarron/parquet-wasm — Parquet read/write in WASM if needed independently of DuckDB.

## Background reading (optional but useful)

- **Power Query M intro on Microsoft Learn:** https://learn.microsoft.com/en-us/powerquery-m/m-spec-introduction — describes M's evaluation model as "modeled after the evaluation model commonly found in spreadsheets, where the order of calculation can be determined based on dependencies between the formulas in the cells." This dependency-driven evaluation model is what mrsflow's evaluator must implement.
