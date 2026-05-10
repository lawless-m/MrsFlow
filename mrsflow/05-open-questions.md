# 05 — Open Questions for Claude Code

Things to resolve once Claude Code has access to the user's actual Power Query repo on the intranet.

## Corpus analysis (highest priority)

Scan the existing `.pq` / Power Query files and produce:

1. **Function frequency table.** Which `Table.*`, `List.*`, `Record.*`, `Text.*`, `Number.*`, `Date.*` etc. functions are actually used, and how often. This is the real v1 stdlib scope. The estimate in `03-scope-v1.md` is provisional — the corpus is the truth.

2. **Language feature usage.** Which constructs appear: `let/in` (assumed always), `if/then/else`, lambdas with `each`, lambdas with explicit `(x) =>`, type ascriptions, record literals, list literals, complex field access patterns, error handling (`try/otherwise`), function definitions assigned to identifiers.

3. **Source connector usage.** Which `Source = ...` patterns appear. Confirm the assumption that ingestion can be migrated to "read Parquet" upstream. Identify queries that do non-trivial work in their source step (filters pushed to the source, etc.) versus queries that just read and transform.

4. **Long tail.** Functions used only once or twice — these can be deferred from v1 if they're awkward to implement.

## Feasibility checks

- **Does the corpus rely on M behaviours that aren't in the published spec?** Microsoft's runtime has undocumented behaviour. If real queries depend on it, that's important to know.
- **Are there queries with circular references, intentional errors, or other "clever" patterns** that would stress an implementation in non-obvious ways?
- **What's the typical query size?** A handful of steps versus dozens versus hundreds. Affects what "useful" means in v1.

## Implementation kickoff questions

Once the corpus analysis is in:

1. **Project structure.** Cargo workspace with `mrsflow-core`, `mrsflow-cli`, `mrsflow-wasm` as separate crates is the recommended starting structure. CLI binary is named `mrsflow`.

2. **Parser approach.** Hand-written recursive descent (matches Microsoft's TS parser approach, gives best error messages) versus parser combinators (`nom`, `chumsky`) versus parser generator (`pest`, `lalrpop`). Recommend hand-written for production quality and to keep dependencies minimal; chumsky is a reasonable second choice if hand-rolling feels too heavy.

3. **Value representation.** How to represent M values in Rust. Tagged enum is the obvious starting point:
   ```rust
   enum Value {
       Null,
       Logical(bool),
       Number(f64),
       Text(String),
       Date(NaiveDate),
       List(Vec<Value>),
       Record(BTreeMap<String, Value>),
       Table(arrow::RecordBatch),
       Function(/* closure representation */),
   }
   ```
   Open question: do we use `Arc` for sharing, given M's immutable semantics? Probably yes for tables, maybe not for primitives.

4. **Error model.** M errors are values, not exceptions. Need to decide on a representation that supports `try ... otherwise ...` cleanly. Probably an `Error` variant in the value enum.

5. **Lazy evaluation.** How lazy do we go? Microsoft's M is fairly lazy. v1 can probably afford to be eager-ish for tables (load fully into Arrow) and lazy only for the `let` binding chain. Worth confirming what the corpus needs.

## Build and CI

- Set up the workspace with CI from day one (GitHub Actions, building Linux + Windows + WASM targets).
- Establish the Microsoft-M oracle harness early — it's the project's quality backbone and retrofitting it later is painful.
- Document the build process for a fresh Debian machine and a fresh Windows machine in a README.

## Things explicitly NOT to decide yet

- Performance optimisation strategies — wait until something is too slow.
- Push-down to DuckDB or DataFusion — not v1, possibly never.
- Custom dialect extensions to M — resist the temptation, stay compatible with Microsoft's M for as long as possible.
- Language services / LSP integration — interesting future project, not v1.
- Publishing to crates.io / public release strategy — internal tool first, public later if at all.
