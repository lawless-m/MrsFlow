# 08 — Prolog as a Differential Oracle

How `mrsflow` uses a parallel implementation in Prolog to validate its Rust evaluator, and how the pattern transfers to other projects.

## The setup

`mrsflow` is a Rust implementation of Microsoft's Power Query M language. To validate it, we built a *parallel* implementation of the same evaluator in Prolog (scryer-prolog) at `tools/grammar-fuzz/evaluator.pl`. A differential harness (`tools/grammar-fuzz/diff_eval.sh`) runs both implementations against a shared corpus of test cases and diffs their output.

The corpus is ~165 small M expressions, each exercising one or two language features. Both implementations are required to produce **byte-identical** output in a canonical S-expression format. Any divergence is by definition a bug in one (or both) implementations — the harness refuses to be green until both agree on every case. Through ~50 evaluator slices we've held this at `165 / 165 passed`.

## Why Prolog

Three reasons it works as the oracle:

1. **Different paradigm forces independent thought.** Rust's evaluator manipulates Arrow RecordBatches, `Rc<RefCell<…>>`, and enum dispatch. Prolog manipulates terms via pattern matching and unification. The two implementations are unlikely to make the *same* mistake, so disagreement is informative — exactly the property a differential test needs.

2. **Prolog's term language is a natural fit for M's value model.** M values (lists, records, tables-as-list-of-records) map cleanly onto Prolog terms: `list([num(1), text("a")])`, `record([("k", num(1))])`, `table([Cols, Rows])`. The pattern-match clauses read like the spec.

3. **It's cheap to write.** The Prolog mirror is ~1,000 lines for what takes ~10,000 lines of Rust. The Prolog doesn't have to be fast — it only needs to be a faithful semantic reference.

## How the harness works concretely

1. **Shared canonical format.** Both implementations emit values as S-expressions: `(num 1.0)`, `(text "hello")`, `(list ((item (num 1)) (item (num 2))))`, `(table ((cols ("a" "b")) (rows (((num 1) (num 2)) ((num 3) (num 4))))))`. The Rust side uses `mrsflow-core/src/eval/sexpr.rs`; the Prolog side has matching writer predicates.

2. **Shared corpus.** A list of `.m` source snippets, each accompanied by an expected category (literal, function-result, error, etc.). The same input goes into both implementations.

3. **Run both, diff outputs.** For each case: `mrsflow eval-string '<expr>'` produces output A; `scryer evaluator.pl '<expr>'` produces output B. The harness reports `passed: N    failed: M`.

4. **The diff is the test.** No hand-written expected output — both implementations are written, both must agree. This catches:
   - Rust regression (Rust changed, Prolog unchanged → diverge)
   - Prolog spec misunderstanding (Prolog changed → diverge)
   - Genuinely ambiguous spec corners (both implementers had to think about it independently)

## Real bugs it has caught

A few examples from earlier slices:

- **Integer vs float division.** Rust used `as i64` truncation in one place; Prolog kept floats. Diff caught it on a corpus case involving `Number.From("3.5") / 2`.
- **`each` desugaring.** `each [x]` desugars to `(_) => [x]` where `_` is the parameter. Rust and Prolog disagreed on whether `_` was bound in the body's scope or whether `[x]` resolved to the outer scope. The diff forced us to read the spec carefully.
- **String concat with `&` on nullable values.** Null-propagation rules. The differential found three inconsistencies before we got it right.

In every case, neither side was *obviously* wrong on inspection — the disagreement was what made us go check the spec.

## Using this pattern in a different context

The pattern generalises well. Recipe:

### 1. You need a language or protocol where output is well-defined

The thing under test must produce deterministic output for given input — no clocks, randomness, or environment dependencies. For non-determinism: capture and replay, or normalise the output.

### 2. Pick an oracle implementation in a *different paradigm*

Not just a different library — a different *kind* of language. The whole point is to avoid sharing bugs through shared abstractions. Examples that work well:

- **Lisp or Scheme** for tree-shaped IRs
- **Prolog** for anything with structural matching, logic, or rule application (type checkers, query planners, grammars)
- **Haskell** for pure-functional reference of an effectful system
- **Python** for "obvious correct" baselines where speed doesn't matter
- **A different team's implementation** — the original Spec/Ref/Code competition pattern

Cost-benefit favours expressive languages here. You're writing throwaway-ish code that has to be *obviously correct* on inspection.

### 3. Define a canonical normalised output format

This is the key step most people skip. The format must be:

- Trivially parseable by both implementations (we use S-expressions; JSON works; protobufs work)
- Lossless for everything that matters semantically
- Byte-identical when semantics match — no whitespace ambiguity, consistent number formatting (`{:?}` for f64 in Rust gives `1.0`, not `1`), stable map/record key ordering, normalised strings

We had a slice early on (`eval-7c`) just for extracting the S-expression formatter into a single source of truth used by every printer.

### 4. Build a corpus that exercises features, not coverage

The corpus is where the value lives. Add cases:

- **As you implement a feature** — for each new builtin or syntax form, add 2–3 tests
- **When a bug is found** — regression cases
- **Edge cases from the spec** — null propagation, empty lists, zero-arity functions
- **Combinations** — features × features, since most bugs live at boundaries

Don't aim for code coverage. Aim for "if any of these regress, we want to know."

### 5. Make the diff loud and the harness fast

`bash tools/grammar-fuzz/diff_eval.sh` runs in under a second for 165 cases. That means we run it on every commit, every slice, every refactor. If it took 30 seconds, we'd skip it.

Fast diff = constant feedback = the oracle pays for itself.

### 6. Treat oracle bugs as findings, not problems

When the diff goes red and you trace it to a bug in *your oracle*, that's still valuable — it means a corner of the spec is genuinely ambiguous or under-specified. Fix the oracle, write up the spec ambiguity, move on. The point isn't that the oracle is right; it's that two independent reads must agree.

## When this doesn't work

- **Non-deterministic systems.** Database query planners, ML models, concurrent runtimes — too much state.
- **Performance-critical correctness.** The oracle is slow; you can't use it to validate a JIT.
- **When the two implementations *share* a flawed mental model.** A common failure mode: both implementers read the same wrong blog post. Mitigation: pick implementers (or paradigms) likely to come at the problem differently.
- **Tiny domains.** For 5-line algorithms, a unit test is fine; the oracle overhead doesn't pay back.

## How this relates to the Excel-based oracle

The Prolog differential gives us cheap continuous feedback. It catches Rust-vs-Prolog disagreement — which is most bugs, since one implementer typically has the right read and the other doesn't.

For the rare case where Rust and Prolog *agree* but might both be wrong (a shared misreading of the spec), the `Oracle/` directory holds a Windows-only Excel-COM harness: edit `Oracle/now.m` with the expression in question, run `Oracle/QueryOracle.ps1`, read cell A2. That's Microsoft's actual implementation as ground truth.

Together: cheap continuous feedback from Prolog, plus a rare expensive ground truth from Excel when we need it. The Prolog catches 99% of regressions in milliseconds; Excel resolves the remaining 1% when both Rust and Prolog are confused.

That's the pattern worth lifting.
