# 02 — Architecture

## Language: Rust

Considered alternatives: C# (would inherit Microsoft ecosystem familiarity but loses WASM, loses Debian-native, no existing C# parser to start from). C++ (would suit a DuckDB extension specifically but loses the standalone CLI / WASM story). The Microsoft-maintained M parser is in TypeScript, not C# — useful as a reference oracle but not a usable starter codebase for this project.

Rust wins on:
- WASM-native via `wasm-bindgen` / `wasm-pack`
- Debian-native single-binary deployment
- First-class Arrow ecosystem (`arrow` crate)
- First-class Parquet (`parquet` crate, designed to pair with arrow)
- Cross-compilation to 32-bit Windows if ever needed (it shouldn't be — see overview)
- Matches user preference

## Data substrate: Apache Arrow

Every value flowing through the system is Arrow-typed. M tables map naturally to Arrow record batches. Columns are typed Arrow arrays. Records become Arrow structs. Lists become Arrow lists. This is the same substrate DuckDB and Parquet already use, so the entire pipeline shares one columnar representation with no conversion costs at boundaries.

## Critical architectural discipline: pure core, IO at the edges

The mrsflow evaluator is **synchronous and pure**. It takes M source plus input Arrow tables and produces output Arrow tables. It does not touch the filesystem, network, clock, or environment. It does not use `tokio` or any async runtime.

IO lives in shells around the core:
- The CLI shell handles file reading/writing, argument parsing, exit codes.
- The WASM shell handles JavaScript interop, browser fetch, IndexedDB.
- A future server shell could handle HTTP requests, etc.

This discipline is non-negotiable. If async or IO leaks into the evaluator, the WASM build becomes ugly and the test harness becomes harder. Pay the structural cost upfront.

## Components

```
mrsflow-core (lib)
├── lexer
├── parser  → AST
├── evaluator → Arrow tables in/out, sync, pure
└── stdlib (Table.*, List.*, Record.*, etc., as plain Rust functions over Arrow)

mrsflow-cli (bin: mrsflow)
├── argument parsing (clap or similar)
├── Parquet IO (read inputs, write outputs)
└── invokes mrsflow-core

mrsflow-wasm (cdylib)
├── wasm-bindgen interface
├── browser-side Parquet (parquet-wasm or arrow-js)
└── invokes mrsflow-core
```

Recommend a Cargo workspace for clean separation between the three crates, with `mrsflow-core` having no IO dependencies and the shells layering them on.

## Evaluation strategy: tree-walking interpreter (v1)

Walk the AST, evaluate lazily, materialise Arrow tables when forced. M's lazy semantics map naturally to deferred evaluation in the walker. This is simpler than compiling to a query plan and matches Microsoft's M semantics more faithfully.

A future v2 could compile common patterns down to DataFusion logical plans for performance. Don't do this in v1 — it's premature optimisation and complicates record-typed values, which don't map cleanly to SQL.

## Why not a DuckDB extension instead

Considered and rejected. Records are fundamental to M and don't map cleanly to DuckDB's STRUCT type (which is nominally typed and fixed at plan time). The queries that genuinely need M (record surgery, nested JSON, per-row record logic) are exactly the ones that don't transpile cleanly to SQL. A hybrid "transpile what you can, interpret the rest" architecture in C++ against DuckDB internals is more complex than just building a clean Rust evaluator.

The standalone Rust approach also keeps the WASM door open, which a DuckDB-extension approach does not.

## Test harness uses Microsoft's M as oracle

See `04-test-harness.md`. The implementation is validated by running real queries through both Microsoft's M (via Excel or PowerQueryNet) and through mrsflow, then diffing Arrow output. The user's existing query corpus is the test suite.
