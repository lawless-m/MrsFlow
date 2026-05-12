# mrsflow — Project Handoff

`mrsflow` is a planned Rust implementation of the Power Query M formula language. The name is **M** + **rs** (Rust convention) + **flow** (what the language does — data flow transformations).

This zip contains the design notes from a conversation scoping out the project.

## Where to start

1. **`01-overview.md`** — The thesis, motivation, and what we're actually building. Read first.
2. **`02-architecture.md`** — Stack decisions, why Rust, how the pieces fit together.
3. **`03-scope-v1.md`** — What's in and out of v1. The Parquet-in-Parquet-out scoping.
4. **`04-test-harness.md`** — How to validate against Microsoft's M as oracle.
5. **`05-open-questions.md`** — Things to resolve in Claude Code with access to the real query corpus.
6. **`06-resources.md`** — Links to the language spec, reference parser, relevant crates, prior art.
7. **`07-evaluator-design.md`** — Load-bearing decisions for the evaluator (laziness, error model, environment, value/number representation), the Prolog evaluator companion as first-class differential, and the slicing plan.
8. **`08-prolog-differential.md`** — How the parallel Prolog evaluator works as a differential oracle for the Rust implementation, and how the pattern transfers to other projects.

## Status

Design phase complete. Next step is for Claude Code (with intranet access to the existing Power Query repo) to scan the real query corpus and produce an evidence-based list of which M language features and library functions v1 actually needs to implement.

## Key context for whoever picks this up

- The user has 45 years of programming experience across many languages but doesn't program in Rust — Claude Code does the actual coding work.
- The user runs Debian at home (RTX 4070, 8GB VRAM, 16GB RAM) and Windows 11 at work (dual Xeon, RTX 3090 24GB VRAM, 64GB RAM).
- Avoid Python. Rust is the chosen language for the core.
- A separate but related project (`Serious-DBI-Sam`) already solves the 32-bit ODBC legacy database problem via a DuckDB extension + .NET gRPC bridge. The M tool does NOT need to solve this — it consumes Parquet that's produced upstream.

## The name

`mrsflow` — pronounceable as "M-RS-flow" or "Mrs Flow." Captures M-the-language, rs-the-implementation, and flow-the-data-shape in one short identifier. Namespace appears clear on GitHub, crates.io, and general search.
