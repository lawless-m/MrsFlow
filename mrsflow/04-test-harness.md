# 04 — Test Harness

## The oracle

Microsoft's M implementation is the reference. Any divergence is, by default, a bug in mrsflow. The user's existing query corpus is both the test suite and the spec for what v1 must support.

## How to produce oracle outputs

**v1 approach: manual runs on Windows, committed goldens.**

For each test query, the user runs it once through Excel or Power BI Desktop on Windows, exports the result to Parquet, and commits the file to the repo as the golden output. CI then diffs `mrsflow` output against the committed golden — no Microsoft runtime needed at test time, no Windows needed in CI, no fragile automation in the loop.

This trades automation for reliability. Goldens only need regenerating when a query changes or a new one is added, which is infrequent and a manual step is fine.

If manual regeneration becomes a chore, the well-trodden automation path is **PowerShell driving Excel via COM** — open workbook, refresh queries, export. It works reliably when run interactively. The pain point is *not* the automation itself but unattended execution: Scheduled Tasks / service accounts running Excel without a desktop session hit awkward COM permission and DCOM-identity issues. Solvable, but the manual approach sidesteps it entirely until the corpus is large enough to justify the work.

[PowerQueryNet](https://github.com/gsimardnet/PowerQueryNet) is a second option that avoids Excel entirely, but it's a less-maintained third-party wrapper around the Mashup Engine — riskier dependency than well-known PowerShell+Excel patterns.

## The diff

Output comparison happens at the Arrow level, not the text level. The `arrow` crate has comparison kernels that handle:
- Schema equality (column names, types, nullability)
- Per-column value equality with proper null semantics
- Approximate float comparison if needed

This sidesteps the entire text-format ambiguity around dates, floats, locale-formatted numbers, etc.

## Test cycle

```
for query in corpus:
    expected = read_parquet(goldens/query.parquet)  # committed, Windows-generated
    actual   = run_mrsflow(query)                   # produces Parquet
    assert arrow_equals(expected, actual)
```

Goldens live under `tests/goldens/` (or similar) and are checked in alongside the `.pq` source.

Failed tests show:
- Schema diff (which columns differ in type/name/nullability)
- First N differing rows
- Optionally: which step in the M expression introduced the divergence (harder, deferred)

## Regression tracking

Every fixed bug becomes a new test case. Every query the user adds to their real workload gets dropped into the corpus. The corpus grows monotonically; v1 ships when 100% of it passes.

## What this catches and what it doesn't

**Catches:**
- Wrong arithmetic, wrong comparison semantics, wrong type coercions
- Sort stability differences
- Null handling divergences
- Group-by aggregation differences
- Join semantics (inner vs left vs outer, key matching)
- Off-by-one errors in any list/table operation

**Doesn't catch:**
- Performance regressions (separate benchmarking needed; not v1 priority)
- Memory leaks under sustained load (separate concern)
- WASM-specific behaviour differences (the WASM build needs its own smoke tests)
- Anything where mrsflow produces "the right answer" but in a different way than Microsoft's (spec-correct but oracle-different — judgment call when this happens)

## Refining the parser specifically

Microsoft's TypeScript parser ([github.com/microsoft/powerquery-parser](https://github.com/microsoft/powerquery-parser)) can be used as a parser-level oracle independently. Run a corpus of M source files through both parsers, compare ASTs structurally. This catches parsing bugs without needing to evaluate anything. Worth doing for the parser layer specifically because parser bugs cascade into evaluation bugs that look mysterious.

### Fuzz input generation via Prolog DCG (sidecar)

Hand-written test cases catch what you think to test. To find what you didn't, write the M grammar as a Definite Clause Grammar in Prolog (SWI or Scryer) and use it as a *generator*: synthesise random valid M source — varied identifier shapes, deep nesting, edge-case literals (`#date`, `#duration`, escaped text), unusual operator chains. Feed the generated source to both the TypeScript reference parser and the Rust parser, compare ASTs structurally, fail on any divergence.

This is a development tool, not part of the shipped product — it lives in its own subdirectory (e.g. `tools/grammar-fuzz/`) with its own dependencies and is invoked manually or in CI. The DCG itself doubles as an executable second reading of the spec, which has independent value when grammar questions arise.

**Build this from day one, in parallel with the parser, not after.** Provable correctness is more valuable early than late: every Rust commit gets validated against an independent encoding of the grammar from the moment there's anything to validate. Even before the Rust parser exists, the DCG can be exercised against the TypeScript reference parser to confirm it generates source Microsoft accepts — that work isn't wasted, it sharpens the DCG. Once the Rust lexer exists, lexer-level differential testing (DCG-generated source → both lexers → compare token streams) starts immediately. Parser-level differential follows as soon as the parser does.
