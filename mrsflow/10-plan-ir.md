# Plan IR and the Fold Planner

Thesis in one line: **the accumulator in `LazyOdbc` tops out at filter-and-project because filter and project are the only two operators that compose without a plan. To fold a `Table.Group` or a join into DBISAM we need a logical relational plan between the M AST and the connector, and a planner that decides — per connector — how much of that plan can be pushed down.**

This doc specifies that plan IR, the scalar sub-IR underneath it, how M lowers into both, and how the DBISAM dialect grammar becomes the fold decision procedure rather than a separately-maintained capability table. It follows `09-lazy-tables.md` (which it largely subsumes for the connector path) and leans on `08-prolog-differential.md` and `04-test-harness.md` for the safety discipline.

## Why now, and why this is bimodal

For queries M already folds fully — the filter-project-sort spine against a source it reaches well — this buys nothing. The whole query is already down at the source and there is nothing left to win. If the real workload in `examples/` is mostly that shape, the accumulator is sufficient and this layer should not be built.

The win is concentrated and large where M folds *badly*: aggregation and joins against a source M reaches through a weak ODBC driver. DBISAM is exactly that source — no native Power Query connector, reached over ODBC, driver reports little capability, so M pulls the filtered table over the wire and groups it in the engine. Where a `Table.Group` collapses millions of rows to hundreds, pushing the `GROUP BY` down is not a few percent, it is moving hundreds of rows instead of millions. We reach DBISAM over a reverse-engineered native protocol and own the dialect emitter, so the ceiling is what the DBISAM engine executes, not what a driver admits.

**Gate before building any of this:** triage `examples/`. For each query that hits a group or join against DBISAM, find where M's fold breaks and how many rows cross the wire at that break versus at the source. If the tail is empty, stop at the accumulator. If three queries pull a million rows to summarise to a few hundred, those three are the justification by name.

A second, smaller justification survives even if the fold tail disappoints: pushing a filter below a join or group shrinks what the in-memory evaluator chews on, which helps pure-Parquet queries with no source folding at all. The logical-optimisation passes pay off independent of the fold percentage.

## Three layers, all S-expressions

| Layer | Faithful to | Status | Example |
| --- | --- | --- | --- |
| M AST | source text | exists | `(invoke Table.SelectRows tbl (each (= [Country] "GB")))` |
| Plan IR | meaning (relational algebra) | new | `(filter (= (col Country) (lit "GB")) (scan sales))` |
| Scalar IR | the expressions inside `each` | new | `(= (col Country) (lit text "GB"))` |

Keeping all three as S-expressions is deliberate: one uniform tree representation, a generic bottom-up rewrite engine over all of them, and trivial dump-and-diff at every pass — which the differential harness consumes directly. The M AST is what the user wrote; the Plan IR is what it computes; the Scalar IR is the leaf-level expression language that fold-safety reasoning operates on.

## The relational node set

Closed and minimal. Logical only — no node encodes a backend choice or a fold decision; that is the planner's job, downstream.

| Node | Shape | Lowers from |
| --- | --- | --- |
| `Scan` | source binding | a `*.Document` / connector leaf |
| `Filter` | predicate (scalar), input | `Table.SelectRows` |
| `Project` | named expressions (scalar), input | `Table.SelectColumns`, `Table.AddColumn`, `RenameColumns` |
| `Sort` | keys + direction, input | `Table.Sort` |
| `Limit` | n, optional offset, input | `Table.FirstN`, `Table.Range` |
| `Aggregate` | group keys, named aggregations, input | `Table.Group` |
| `Join` | kind, left, right, condition (scalar), key sets | `Table.NestedJoin` (+ following `ExpandTableColumn`) |
| `Distinct` | input | `Table.Distinct` |
| `EvalM` | opaque M thunk, declared output schema | anything not in the above set |

`EvalM` is the escape hatch and the most important node for honesty. Any step that doesn't map to a relational operator — a call into a user function, a list/record manipulation that isn't a table transform, arbitrary table-returning M — lowers to `EvalM`. The planner cannot see through it; folding stops at it. Most of the time `EvalM` sits *above* the foldable spine, so it costs nothing but a materialisation boundary.

The `NestedJoin` + `ExpandTableColumn` pair is recognised during lowering and collapsed into a single `Join` (optionally followed by a `Project` to flatten), rather than carried as the nested-column shape. The `JoinView`/`ExpandView` `TableRepr` variants already model that deferral; this just names it in the plan.

## The scalar IR

Distinct from the relational layer, and typed. This is the layer where "can this fold to SQL, and is it safe to" gets answered, so it must not be raw M-AST blobs.

| Form | Notes |
| --- | --- |
| `(col Name)` | column reference |
| `(lit type value)` | typed literal — type carried for decimal/date/null reasoning |
| `(cmp op a b)` | `= <> < <= > >=` |
| `(bool op …)` | `and` / `or` / `not` |
| `(arith op a b)` | `+ - * /` |
| `(call fn args…)` | bounded allow-list of M functions with SQL analogues |
| `(opaque)` | an `each` body that does not reduce to the above |

Lowering an `each` body produces *either* a scalar IR expression *or* `(opaque)`. An opaque scalar is itself a fold boundary: a `Filter` whose predicate is opaque cannot fold and falls to in-memory evaluation, where the full M evaluator runs the original `each` over the rows. The `call` allow-list is the only place library functions enter — `Text.Upper → UPPER`, `Text.Contains → LIKE`, `Number.Round → ROUND`, and so on — and it is intentionally small; everything off the list is `opaque`, not wrong.

## The fold planner — the grammar *is* the decision procedure

This is the payoff from grinding out the DBISAM DCG. Given a logical plan and a target connector, walk the plan bottom-up and attempt to emit the connector's SQL dialect through the grammar. The maximal subtree that emits valid SQL is the fold; the first node that won't emit is the boundary; everything above the boundary runs in the evaluator over the rows the fold returns.

The emitter's success **is** the syntactic fold predicate. There is no separate hand-written "can DBISAM take a `GROUP BY` here" table to drift out of sync with the emitter — if the grammar generates valid DBISAM SQL for the subtree, it folds; if it doesn't, it doesn't. The dialect ceiling and the codegen are the same operation.

But syntactic emittability is only the first of two gates:

- **Gate 1 — dialect (the grammar).** Can the DCG emit valid DBISAM SQL for this subtree? Encodes the dialect ceiling: `TOP n` not `LIMIT/OFFSET`, no window functions, limited subqueries, DBISAM's own string/date function names and date-literal syntax.
- **Gate 2 — semantics (proven, not assumed).** Is folding this class equivalent to in-memory evaluation for DBISAM? Collation on text comparison, `NULL` ordering in `ORDER BY`, integer-vs-decimal division, date-boundary handling — the usual suspects in an engine this vintage. These are *proven* by the differential harness and recorded as fold-exclusion rules. They live in the connector, not the planner, because they are dialect-specific.

Both gates must pass for a node to fold.

## Capability model

The plan IR is backend-agnostic; the fold pass is backend-specific. Each connector is effectively a row in a capability table, but for SQL backends that row is *computed*: "what the dialect grammar emits" minus "what the differential harness proved unsafe." Do not hand-write it.

| Backend | Capability | Realised by |
| --- | --- | --- |
| DBISAM (native) | filter, project, sort, `TOP`, equi-join, aggregate — pending Gate 2 | dialect grammar + proven exclusions |
| PostgreSQL (native) | broad — the rich end of the table | (its own emitter) |
| ODBC (generic) | portable lowest-common-denominator | driver-reported |
| Parquet | row-group elimination on `Filter` over column stats; column selection on `Project`; **no** aggregate/join/sort | statistics, not SQL |

Note the asymmetry: Parquet can take *less* than the current accumulator's LCD, while native DBISAM and PostgreSQL can take *more*. The accumulator folded everything at the intersection of all backends, which is why the native paths were folding at the level of the poorest. The planner pushes the maximal subtree *each backend* supports.

## Logical optimisation, before folding

RA→RA rewrites with explicit equivalence preconditions, applied to the logical plan before the fold pass:

- **Filter pushdown** — push `Filter` below `Join`/`Aggregate`/`Project` where the predicate's columns permit. Helps both the fold (more selective scan at the source) and the in-memory path (less for the evaluator to chew).
- **Projection pruning** — drop columns nothing above consumes.
- **Conjunction splitting** — break `and`-predicates so each conjunct can be pushed independently; the part that folds folds, the part that doesn't stays.

These are the justification that survives a disappointing fold percentage, because they help even when nothing reaches a source.

## Execution split

```
M AST
  │ lower
  ▼
Plan IR ──► logical optimise ──► fold pass (per connector, via DCG)
                                      │
                          ┌───────────┴───────────┐
                          ▼                       ▼
                  foldable subtree           residual plan
                  → DBISAM SQL               → evaluator runs over
                    over native socket          the returned rows
```

The `LazyOdbc` predicate/projection accumulation is subsumed by this for the connector path: filter-and-project become the trivial bottom of the same fold walk. The lazy `TableRepr` variants remain the *carrier* of a deferred plan; the planner is what fills them above filter-and-project.

## Differential gate — no Excel in the loop

Every fold class is enabled on DBISAM only after the harness proves it. Same query, same source rows, two routes — folded into DBISAM SQL versus pulled raw and run through mrsflow's own operators — diffed. Divergences become Gate 2 exclusions. This is the same discipline as the Prolog and Excel oracles (`08`, `04`), pointed at the fold path itself, and it needs no Excel because both routes are inside mrsflow. Build this harness *first*, against the filter-and-project folding that already exists, so the instrument is validated against trusted behaviour before it measures anything new.

## What's deliberately out of scope

- **Cost-based planning.** The fold pass is rule-based — push the maximal safe subtree, full stop. No statistics-driven join reordering. DBISAM isn't worth a cost model.
- **Cross-source folding.** A join whose two inputs are different connectors does not fold; the join runs in the evaluator. Only same-source subtrees fold.
- **Writing back.** Read-only. The connector pulls and folds; it does not emit DML.
- **Speculative scalar translation.** The `call` allow-list grows only when a function has a proven-equivalent DBISAM analogue. Unknown functions are `opaque`, never guessed.

## Open questions

1. **Where does `EvalM` force?** Eagerly at the boundary, or lazily threaded so a downstream fold can still see the schema? Affects whether an `EvalM` between two foldable regions kills the lower fold.
2. **Aggregate over a non-foldable filter.** If the `Filter` beneath an `Aggregate` is opaque, do we fold neither, or fold the `Aggregate` over locally-filtered rows? The second needs the planner to split the pipeline at the opaque node and fold the group separately — more complex, possibly not worth it for v1.
3. **Does the scalar `call` allow-list live in the planner or the connector?** Function *names* are dialect-specific (DBISAM's string functions differ), but the *recognition* of `Text.Upper` is generic. Likely: generic recognition in the planner, dialect spelling in the connector emitter.
4. **Probe-derived vs declared ceiling.** Do we map the DBISAM ceiling by constructed probes against a live instance (empirical, accurate, needs the instance) or declare it in the grammar (portable, may lie)? Probably both — grammar declares, harness verifies.

## Execution plan: give the native DBISAM connector the fold

The IR landed inert, and a triage of the real `RIVSTS*` corpus (2026-05-29) found the gap is sharper than "wire the fold pass in." The connector that *folds* today is the generic-ODBC path (`Odbc.DataSource` builds a `TableRepr::LazyOdbc`, so `SelectRows`/`SelectColumns`/`FirstN` narrow the plan and `render_sql` emits `GenericOdbc`). The connector that is *fast and owned* — the native `Exportmaster.*` client — does **not** fold: `list_tables_as_navigation` hands each table's `[Data]` an eager `SELECT * FROM <t>` thunk, so navigating to `Analysis` pulls the whole table (~2m36s, ~4.79M rows on a fast LAN) and any subsequent filter runs in memory. The `Dbisam` dialect emitter exists in `fold.rs` with **no live caller**. So the workload that justifies the IR (e.g. the Kingsbury `Cost_and_Sell` query: `Analysis` filtered `SAPRODUCT LIKE '4K0%'` → 681 of 4.79M rows, ~7000×, then a same-source `Analysis ⋈ PRODGRP`) is exactly the workload the native connector can't currently fold.

The design follows from one observation: **`LazyOdbcState` is already transport-agnostic.** Of its fields only `force_fn` is connector-specific (and it is already an injected closure); `to_plan` emits dialect-free Plan IR; the fold accumulation in the `Table.*` stdlib operates on `projection`/`where_filters`/`limit` without caring who built the value. The only ODBC-specific coupling is `render_sql` hardcoding the `GenericOdbc` dialect. So we **generalise the one repr** rather than add a parallel `LazyExportmaster` (which would duplicate all the accumulation for nothing).

### Step 1 — native filter / projection / limit fold (mostly reuse)

- **Carry the dialect in the state.** Add a `dialect` discriminant to `LazyOdbcState` and make `render_sql` dispatch through it (`emit(&self.to_plan(), dialect)`). This is required, not cosmetic: DBISAM and generic-ODBC genuinely diverge (`TOP n` vs no-`LIMIT`; `#…#` date literals), so a native-backed state with a `limit` set must render `TOP n`. The dialect therefore cannot live only in the force closure — `render_sql` itself must know it.
- **Make the native navigation build `LazyOdbc`.** Replace the eager `SELECT *` thunk in `list_tables_as_navigation` with a builder mirroring the ODBC bridge (`build_lazy_odbc_table`): a cheap schema probe (reuse the existing capped-cursor fetch with a zero-row target, avoiding a `WHERE 1=0` the dialect may dislike) to populate `schema`, and a `force_fn` that renders the narrowed state under the `Dbisam` dialect and runs it via `query_to_table`, reconnecting per force as the ODBC and MySQL thunks already do. `connection_string` becomes an opaque identity (the host); only the force closure reads it.

Once the navigation yields `LazyOdbc`, the existing `Table.SelectRows`/`SelectColumns`/`FirstN` folding applies to the native connector for free, and the `Dbisam` emitter gets its first live caller. This is the headline win and the bulk of the value.

### Step 2 — native same-source join fold (new lowering)

Join is not in the accumulator (which is `Scan → Filter* → Project`); `Table.NestedJoin` builds a separate `JoinView`. Folding `Analysis ⋈ PRODGRP` means: in the `NestedJoin` path, detect when **both inputs are `LazyOdbc` over the same source** (matching connection identity + dialect), build a combined `Join(a.to_plan(), b.to_plan())`, and emit via `fold(Dbisam)`. The same-source guard is mandatory — cross-source merges (DBISAM × Excel, which is the whole shape of the minimal corpus) must remain a `JoinView` run in the evaluator. Sequence this after Step 1 is differential-verified; it is genuinely new code, not reuse.

### Out of scope for this increment

Aggregate fold: the corpus has **zero** `Table.Group` over a navigated source (every `Table.Group` sits over `Odbc.Query` raw-SQL passthrough or Excel), so `Aggregate` folding has no workload to justify it yet and is deferred.

### Verification

Unit tests mirroring the existing `odbc_*_folds_*` cases but asserting DBISAM render output (`TOP n`, `#…#` dates); the `differential.rs` Gate-2 harness, which already models DBISAM `Semantics`, run over the new native fold; and a live integration test of the Kingsbury query behind the `exportmaster` feature, asserting both the 681-row result and that only 681 rows cross the wire.

The `LazyOdbcState` → `LazySqlState` rename (and the `TableRepr::LazyOdbc` variant) is cosmetic and deferred — land the wiring under the existing name to keep the diff reviewable, rename in a separate pass.
