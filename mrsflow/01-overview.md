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
- Reads Parquet as input (both shells) and live ODBC queries (CLI shell only); writes Parquet as output
- Exposes itself as a CLI binary (`mrsflow`) and as a WASM module sharing the same core, with the IO surface differing per shell

That's it. No CSV, no JSON, no Web.Contents, no native database connectors beyond ODBC. Those can be added later if needed; for v1 they aren't.

## Relationship to the existing `Serious-DBI-Sam` project

The user maintains a DuckDB extension + .NET gRPC bridge service that exposes legacy DBISAM databases (32-bit Windows ODBC) to DuckDB on Linux. That bridge stays relevant because **Linux has no DBISAM driver at any bit-width** — to query DBISAM data from a Linux mrsflow process, the bridge is the only path.

Two viable shapes for getting DBISAM data into mrsflow:

```
# Pre-staged (bridge produces Parquet, mrsflow consumes via --in):
DBISAM → bridge (Windows) → DuckDB → Parquet file → [mrsflow --in t=t.parquet] → Parquet

# Live (mrsflow's Odbc.DataSource against a DuckDB instance attached to the bridge):
M code: Source = Odbc.DataSource("DSN=DuckDB"), Orders = Source{[Name="orders"]}[Data]
```

For *non-DBISAM* databases (Postgres, SQL Server, MySQL, Sage X3, etc.), Linux 64-bit ODBC drivers exist; mrsflow's `Odbc.DataSource` reaches them directly with no bridge needed. (A potential 64-bit DBISAM driver on Windows would also bypass the bridge for the Windows-side workflow, but Linux mrsflow continues to need it.)

mrsflow itself stays cleanly 64-bit on Debian and trivially WASM-compilable. The 32-bit constraint remains fully contained inside the bridge service.
