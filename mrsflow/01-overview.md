# 01 — Project Overview

## The name

**mrsflow** — M + rs (Rust) + flow. Pronounceable as "M-RS-flow" or "Mrs Flow."

## The thesis

**M without Microsoft.** Power Query's M is a genuinely good language for tabular data transformation, but it's trapped inside Excel and Power BI. There is no open-source M evaluator in any language. mrsflow builds one in Rust, deployable as a standalone CLI on Debian and as a WASM module in browsers.

## Why this is worth doing

- **Manager-happiness in CI/CD.** M files are declarative artefacts that look like configuration: version-controllable, code-reviewable, diffable in PRs. "Here's the .pq file, here's the test, here's the CI job" is a cleaner governance story than bash scripts piping SQL into duckdb. The team already trusts M because Power Query is enterprise-blessed Microsoft tech.
- **Decoupling from Excel as a refresh runtime.** Currently queries refresh by opening Excel/Power BI. A headless CLI lets the same queries run in pipelines without a desktop in the loop.
- **WASM unlocks browser deployment.** The user already builds browser tools using DuckDB-Wasm + SQL. Adding mrsflow-Wasm gives those tools a transformation layer without server roundtrips. M-anywhere-code-runs is a stronger pitch than M-without-Excel-on-Windows.
- **Reuses the user's existing skills and query corpus.** Years of M code can run in the new pipeline.

## What it is NOT

- Not a 4GL revival or a "better Excel" pitch — those framings don't hold up.
- Not a graph-spreadsheet UI experiment (that's where the conversation started, but Power Query already implements that model and the gap is the implementation/openness, not the design).
- Not aiming for 100% Microsoft M compatibility. Aiming for "the queries the user actually writes, run correctly."
- Not an ingest tool. Parquet is the input format. Other tools (DuckDB, the existing DBISAM bridge) handle ingestion upstream.

## The shape of v1

A Rust crate that:
- Parses M source per the language spec
- Evaluates M expressions against in-memory Arrow tables
- Reads Parquet as input, writes Parquet as output
- Exposes itself as a CLI binary (`mrsflow`) and as a WASM module sharing the same core

That's it. No CSV, no JSON, no ODBC, no Web.Contents. Those can be added later if needed; for v1 they aren't.

## Relationship to the existing `Serious-DBI-Sam` project

The user already maintains a DuckDB extension + .NET gRPC bridge service that exposes legacy DBISAM databases (32-bit Windows ODBC) to modern DuckDB on Linux. That project solves the legacy ingestion problem completely.

mrsflow does NOT need an ODBC connector for v1. The pipeline is:

```
DBISAM → bridge (32-bit, Windows) → DuckDB → Parquet → [mrsflow] → Parquet → downstream
```

The 32-bit constraint is fully contained inside the existing bridge service. mrsflow stays cleanly 64-bit on Debian and trivially WASM-compilable.
