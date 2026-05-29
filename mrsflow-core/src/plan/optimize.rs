//! Logical optimisation — RA→RA rewrites applied to the plan before any fold
//! pass, per `mrsflow/10-plan-ir.md` §"Logical optimisation, before folding".
//!
//! Each rewrite has an explicit equivalence precondition. A generic bottom-up
//! driver applies the *schema-free* rules to a fixpoint; the *schema-aware*
//! rules (join pushdown, projection pruning) additionally need a [`Catalog`]
//! and run via [`optimize_with_catalog`].
//!
//! Rules:
//!   * **Conjunction splitting** — `Filter(and a b …)` → a stack of
//!     single-predicate filters, so each conjunct pushes independently.
//!   * **Filter pushdown** — push a `Filter` below the operator beneath it
//!     when the row-set is provably unchanged. Targets: `Project` (passthrough
//!     columns), `Aggregate` (group-key predicates), `Sort` (always),
//!     `Distinct` (whole-row, or key-only predicates), and — with a catalog —
//!     `Join` (to the side that owns all the predicate's columns, respecting
//!     outer-join row preservation).
//!   * **Project composition** — collapse an identity `SelectColumns` sitting
//!     directly on another replacing `Project`. Cleans up the redundant
//!     projection that pruning can introduce just above a scan.
//!   * **Projection pruning** — top-down, drop columns nothing above consumes:
//!     drop dead `Project`/`Aggregate` outputs and narrow scans to the columns
//!     actually needed. Needs a catalog to know each scan's full column set.

use super::ir::*;
use super::schema::{schema_of, Catalog};

/// Apply the schema-free optimisation passes to a fixpoint. Use this when no
/// catalog is available; join pushdown and projection pruning are skipped.
pub fn optimize(plan: Rel) -> Rel {
    optimize_inner(plan, None)
}

/// Apply all optimisation passes, including the schema-aware ones (join
/// pushdown and projection pruning). Idempotent.
pub fn optimize_with_catalog(plan: Rel, catalog: &dyn Catalog) -> Rel {
    optimize_inner(plan, Some(catalog))
}

fn optimize_inner(plan: Rel, catalog: Option<&dyn Catalog>) -> Rel {
    let ctx = Ctx { catalog };
    // 1. Conjunction split, filter pushdown, and project composition to fixpoint.
    let mut cur = fixpoint(plan, |r| rewrite(r, &ctx));
    // 2. Projection pruning — a single top-down pass (it propagates the needed
    //    column set in one traversal). Then re-run the rewrite fixpoint to
    //    compose the projections pruning inserted and push filters through them.
    if let Some(cat) = catalog {
        cur = prune(cur, Need::All, cat);
        cur = fixpoint(cur, |r| rewrite(r, &ctx));
    }
    cur
}

struct Ctx<'a> {
    catalog: Option<&'a dyn Catalog>,
}

fn fixpoint(mut cur: Rel, step: impl Fn(Rel) -> Rel) -> Rel {
    loop {
        let next = step(cur.clone());
        if next == cur {
            return next;
        }
        cur = next;
    }
}

// --- Bottom-up rewrite driver (schema-free rules + catalog-gated join push) --

fn rewrite(rel: Rel, ctx: &Ctx) -> Rel {
    // Children first.
    let rel = match rel {
        Rel::Scan(_) => rel,
        Rel::Filter { predicate, input } => Rel::Filter {
            predicate,
            input: Box::new(rewrite(*input, ctx)),
        },
        Rel::Project { star, items, input } => Rel::Project {
            star,
            items,
            input: Box::new(rewrite(*input, ctx)),
        },
        Rel::Sort { keys, input } => Rel::Sort {
            keys,
            input: Box::new(rewrite(*input, ctx)),
        },
        Rel::Limit { n, offset, input } => Rel::Limit {
            n,
            offset,
            input: Box::new(rewrite(*input, ctx)),
        },
        Rel::Aggregate { keys, aggs, input } => Rel::Aggregate {
            keys,
            aggs,
            input: Box::new(rewrite(*input, ctx)),
        },
        Rel::Join {
            kind,
            left_keys,
            right_keys,
            left,
            right,
        } => Rel::Join {
            kind,
            left_keys,
            right_keys,
            left: Box::new(rewrite(*left, ctx)),
            right: Box::new(rewrite(*right, ctx)),
        },
        Rel::Distinct { on, input } => Rel::Distinct {
            on,
            input: Box::new(rewrite(*input, ctx)),
        },
        Rel::EvalM { descr, inputs } => Rel::EvalM {
            descr,
            inputs: inputs.into_iter().map(|i| rewrite(i, ctx)).collect(),
        },
    };
    // Local rule at this node.
    match rel {
        Rel::Filter { predicate, input } => rewrite_filter(predicate, *input, ctx),
        Rel::Project {
            star: false,
            items,
            input,
        } => compose_replace_project(items, *input),
        other => other,
    }
}

fn rewrite_filter(predicate: Scalar, input: Rel, ctx: &Ctx) -> Rel {
    match predicate {
        // Conjunction splitting, innermost-first to keep left-to-right order.
        Scalar::Bool {
            op: BoolOp::And,
            args,
        } if args.len() >= 2 => {
            let mut node = input;
            for conj in args.into_iter().rev() {
                node = Rel::Filter {
                    predicate: conj,
                    input: Box::new(node),
                };
            }
            node
        }
        other => push_filter(other, input, ctx),
    }
}

/// Push a single-predicate `Filter` below the operator beneath it when sound.
fn push_filter(predicate: Scalar, input: Rel, ctx: &Ctx) -> Rel {
    let cols = predicate_cols(&predicate);
    match input {
        Rel::Project {
            star,
            items,
            input: inner,
        } if project_passthrough(&cols, star, &items) => Rel::Project {
            star,
            items,
            input: Box::new(Rel::Filter {
                predicate,
                input: inner,
            }),
        },
        Rel::Aggregate {
            keys,
            aggs,
            input: inner,
        } if cols.as_ref().is_some_and(|c| c.iter().all(|x| keys.contains(x))) => Rel::Aggregate {
            keys,
            aggs,
            input: Box::new(Rel::Filter {
                predicate,
                input: inner,
            }),
        },
        Rel::Sort { keys, input: inner } => Rel::Sort {
            keys,
            input: Box::new(Rel::Filter {
                predicate,
                input: inner,
            }),
        },
        Rel::Distinct { on, input: inner } if distinct_commutes(&cols, &on) => Rel::Distinct {
            on,
            input: Box::new(Rel::Filter {
                predicate,
                input: inner,
            }),
        },
        Rel::Join {
            kind,
            left_keys,
            right_keys,
            left,
            right,
        } => {
            let side = ctx
                .catalog
                .zip(cols.as_ref())
                .and_then(|(cat, c)| join_push_side(c, kind, &left, &right, cat));
            match side {
                Some(JoinSide::Left) => Rel::Join {
                    kind,
                    left_keys,
                    right_keys,
                    left: Box::new(Rel::Filter {
                        predicate,
                        input: left,
                    }),
                    right,
                },
                Some(JoinSide::Right) => Rel::Join {
                    kind,
                    left_keys,
                    right_keys,
                    left,
                    right: Box::new(Rel::Filter {
                        predicate,
                        input: right,
                    }),
                },
                None => Rel::Filter {
                    predicate,
                    input: Box::new(Rel::Join {
                        kind,
                        left_keys,
                        right_keys,
                        left,
                        right,
                    }),
                },
            }
        }
        // Scan, Filter, Limit, EvalM, and guarded-out cases: leave in place.
        other => Rel::Filter {
            predicate,
            input: Box::new(other),
        },
    }
}

enum JoinSide {
    Left,
    Right,
}

/// Which side of a join may a filter push to, given column provenance and the
/// join kind? `None` if it cannot push: the predicate is opaque, spans both
/// sides, references an unknown/ambiguous column, or the kind does not preserve
/// rows on the candidate side (a filter on the null-supplying side of an outer
/// join is not equivalent before vs after the join).
fn join_push_side(cols: &[String], kind: JoinKind, left: &Rel, right: &Rel, cat: &dyn Catalog) -> Option<JoinSide> {
    if cols.is_empty() {
        return None;
    }
    let ls = schema_of(left, cat)?;
    let rs = schema_of(right, cat)?;
    let touches_left = cols.iter().any(|c| ls.contains(c));
    let touches_right = cols.iter().any(|c| rs.contains(c));
    let unknown = cols.iter().any(|c| !ls.contains(c) && !rs.contains(c));
    if unknown {
        return None;
    }
    if touches_left && !touches_right && matches!(kind, JoinKind::Inner | JoinKind::LeftOuter) {
        return Some(JoinSide::Left);
    }
    if touches_right && !touches_left && matches!(kind, JoinKind::Inner | JoinKind::RightOuter) {
        return Some(JoinSide::Right);
    }
    None
}

/// Collapse `Project(replace, outer, Project(replace, inner, X))` when every
/// `outer` item is an identity passthrough — the outer just re-selects from the
/// inner, so compose into one projection.
fn compose_replace_project(outer: Vec<ProjectItem>, input: Rel) -> Rel {
    if let Rel::Project {
        star: false,
        items: inner,
        input: inner_in,
    } = input
    {
        let all_identity = outer
            .iter()
            .all(|it| it.expr == Scalar::Col(it.name.clone()));
        if all_identity {
            let mut composed = Vec::with_capacity(outer.len());
            let mut ok = true;
            for o in &outer {
                match inner.iter().find(|i| i.name == o.name) {
                    Some(i) => composed.push(i.clone()),
                    None => {
                        ok = false;
                        break;
                    }
                }
            }
            if ok {
                return Rel::Project {
                    star: false,
                    items: composed,
                    input: inner_in,
                };
            }
        }
        return Rel::Project {
            star: false,
            items: outer,
            input: Box::new(Rel::Project {
                star: false,
                items: inner,
                input: inner_in,
            }),
        };
    }
    Rel::Project {
        star: false,
        items: outer,
        input: Box::new(input),
    }
}

// --- Projection pruning (top-down, catalog-aware) -------------------------

/// The columns a parent requires from a node's output. `All` means "every
/// output column" (the default at the root, and the conservative answer
/// wherever the footprint cannot be bounded).
#[derive(Clone)]
enum Need {
    All,
    Cols(Vec<String>),
}

impl Need {
    fn from_opt(cols: Option<Vec<String>>) -> Need {
        match cols {
            Some(c) => Need::Cols(c),
            None => Need::All,
        }
    }

    fn union(self, other: Need) -> Need {
        match (self, other) {
            (Need::All, _) | (_, Need::All) => Need::All,
            (Need::Cols(mut a), Need::Cols(b)) => {
                for c in b {
                    if !a.contains(&c) {
                        a.push(c);
                    }
                }
                Need::Cols(a)
            }
        }
    }

    fn union_cols(self, extra: &[String]) -> Need {
        match self {
            Need::All => Need::All,
            Need::Cols(mut v) => {
                for c in extra {
                    if !v.contains(c) {
                        v.push(c.clone());
                    }
                }
                Need::Cols(v)
            }
        }
    }
}

fn prune(rel: Rel, need: Need, cat: &dyn Catalog) -> Rel {
    match rel {
        Rel::Scan(src) => {
            // Narrow the scan when the parent needs a strict, non-empty subset
            // of its columns. The inserted projection is what a fold pass turns
            // into column pushdown; a redundant one (when a SelectColumns sits
            // directly above) is collapsed by project composition afterwards.
            if let Need::Cols(wanted) = &need {
                if let Some(schema) = cat.schema_of_source(&src) {
                    let kept: Vec<String> = schema
                        .columns
                        .iter()
                        .filter(|c| wanted.contains(c))
                        .cloned()
                        .collect();
                    if !kept.is_empty() && kept.len() < schema.columns.len() {
                        return Rel::Project {
                            star: false,
                            items: kept
                                .into_iter()
                                .map(|n| ProjectItem {
                                    expr: Scalar::Col(n.clone()),
                                    name: n,
                                })
                                .collect(),
                            input: Box::new(Rel::Scan(src)),
                        };
                    }
                }
            }
            Rel::Scan(src)
        }

        Rel::Filter { predicate, input } => {
            let child = need.union(Need::from_opt(predicate_cols(&predicate)));
            Rel::Filter {
                predicate,
                input: Box::new(prune(*input, child, cat)),
            }
        }

        Rel::Sort { keys, input } => {
            let key_cols: Vec<String> = keys.iter().map(|k| k.column.clone()).collect();
            let child = need.union_cols(&key_cols);
            Rel::Sort {
                keys,
                input: Box::new(prune(*input, child, cat)),
            }
        }

        Rel::Limit { n, offset, input } => Rel::Limit {
            n,
            offset,
            input: Box::new(prune(*input, need, cat)),
        },

        Rel::Distinct { on, input } => {
            // Whole-row dedup compares every column, so all are needed; a keyed
            // dedup only needs its keys beyond what the parent wants.
            let child = if on.is_empty() {
                Need::All
            } else {
                need.union_cols(&on)
            };
            Rel::Distinct {
                on,
                input: Box::new(prune(*input, child, cat)),
            }
        }

        Rel::Project {
            star: false,
            items,
            input,
        } => {
            let kept = drop_unneeded(items, &need, |it| &it.name);
            // The child must supply exactly the columns the kept items read.
            let mut child = Need::Cols(Vec::new());
            for it in &kept {
                child = child.union(Need::from_opt(predicate_cols(&it.expr)));
            }
            Rel::Project {
                star: false,
                items: kept,
                input: Box::new(prune(*input, child, cat)),
            }
        }

        Rel::Project {
            star: true,
            items,
            input,
        } => {
            let all_names: Vec<String> = items.iter().map(|i| i.name.clone()).collect();
            let kept = drop_unneeded(items, &need, |it| &it.name);
            // Columns the parent needs that this project did not produce must
            // come from the input; plus whatever the kept items' exprs read.
            let child = match &need {
                Need::All => Need::All,
                Need::Cols(w) => {
                    let from_input: Vec<String> = w
                        .iter()
                        .filter(|c| !all_names.contains(c))
                        .cloned()
                        .collect();
                    let mut ch = Need::Cols(from_input);
                    for it in &kept {
                        ch = ch.union(Need::from_opt(predicate_cols(&it.expr)));
                    }
                    ch
                }
            };
            Rel::Project {
                star: true,
                items: kept,
                input: Box::new(prune(*input, child, cat)),
            }
        }

        Rel::Aggregate { keys, aggs, input } => {
            let kept = drop_unneeded(aggs, &need, |a| &a.name);
            // Group keys are always needed (they define the grouping); add the
            // column each surviving aggregate ranges over. An opaque aggregate
            // has an unknown footprint, so fall back to needing everything.
            let mut child = Need::Cols(keys.clone());
            for a in &kept {
                if a.func == AggFunc::Opaque {
                    child = Need::All;
                } else if let Some(c) = &a.column {
                    child = child.union_cols(std::slice::from_ref(c));
                }
            }
            Rel::Aggregate {
                keys,
                aggs: kept,
                input: Box::new(prune(*input, child, cat)),
            }
        }

        Rel::Join {
            kind,
            left_keys,
            right_keys,
            left,
            right,
        } => match (schema_of(&left, cat), schema_of(&right, cat)) {
            (Some(ls), Some(rs)) => {
                let (lneed, rneed) = match &need {
                    Need::All => (Need::All, Need::All),
                    Need::Cols(w) => {
                        let l: Vec<String> =
                            w.iter().filter(|c| ls.contains(c)).cloned().collect();
                        let r: Vec<String> =
                            w.iter().filter(|c| rs.contains(c)).cloned().collect();
                        (
                            Need::Cols(l).union_cols(&left_keys),
                            Need::Cols(r).union_cols(&right_keys),
                        )
                    }
                };
                Rel::Join {
                    kind,
                    left_keys,
                    right_keys,
                    left: Box::new(prune(*left, lneed, cat)),
                    right: Box::new(prune(*right, rneed, cat)),
                }
            }
            // Unknown side schema: cannot split the requirement — keep all.
            _ => Rel::Join {
                kind,
                left_keys,
                right_keys,
                left: Box::new(prune(*left, Need::All, cat)),
                right: Box::new(prune(*right, Need::All, cat)),
            },
        },

        // Opaque: every input column may be consumed inside the thunk.
        Rel::EvalM { descr, inputs } => Rel::EvalM {
            descr,
            inputs: inputs.into_iter().map(|i| prune(i, Need::All, cat)).collect(),
        },
    }
}

/// Keep only the elements whose name the parent needs. `Need::All` keeps all.
fn drop_unneeded<T>(items: Vec<T>, need: &Need, name: impl Fn(&T) -> &String) -> Vec<T> {
    match need {
        Need::All => items,
        Need::Cols(w) => items.into_iter().filter(|it| w.contains(name(it))).collect(),
    }
}

// --- Shared scalar-column analysis ----------------------------------------

/// The set of columns a scalar references, or `None` if it contains an opaque
/// sub-expression (whose column footprint cannot be enumerated).
fn predicate_cols(s: &Scalar) -> Option<Vec<String>> {
    let mut cols = Vec::new();
    if collect_cols(s, &mut cols) {
        Some(cols)
    } else {
        None
    }
}

fn collect_cols(s: &Scalar, out: &mut Vec<String>) -> bool {
    match s {
        Scalar::Col(n) => {
            if !out.contains(n) {
                out.push(n.clone());
            }
            true
        }
        Scalar::QualifiedCol { name, .. } => {
            if !out.contains(name) {
                out.push(name.clone());
            }
            true
        }
        Scalar::Lit(_) => true,
        Scalar::Cmp { lhs, rhs, .. } | Scalar::Arith { lhs, rhs, .. } => {
            collect_cols(lhs, out) && collect_cols(rhs, out)
        }
        Scalar::Bool { args, .. } | Scalar::Call { args, .. } => {
            args.iter().all(|a| collect_cols(a, out))
        }
        Scalar::Opaque => false,
    }
}

/// Are all the predicate's referenced columns safe to push below this Project?
/// `None` columns (an opaque predicate) are conservatively unsafe.
fn project_passthrough(cols: &Option<Vec<String>>, star: bool, items: &[ProjectItem]) -> bool {
    let cols = match cols {
        Some(c) => c,
        None => return false,
    };
    cols.iter().all(|c| match items.iter().find(|it| &it.name == c) {
        // extend (AddColumn): a named item is freshly computed/renamed → unsafe;
        // a column not in the item list is an input passthrough → safe.
        Some(_) if star => false,
        None if star => true,
        // replace (SelectColumns): safe only when the item is an identity
        // passthrough (output name == referenced input column).
        Some(it) => it.expr == Scalar::Col(c.clone()),
        // replace: referenced column is not in the output — can't push.
        None => false,
    })
}

/// May a filter be pushed below this `Distinct`?
fn distinct_commutes(cols: &Option<Vec<String>>, on: &[String]) -> bool {
    on.is_empty() || cols.as_ref().is_some_and(|c| c.iter().all(|x| on.contains(x)))
}

#[cfg(test)]
mod tests {
    use super::super::lower::lower;
    use super::super::schema::{Catalog, Schema};
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;
    use std::collections::HashMap;

    fn parse_lower(src: &str) -> Rel {
        let toks = tokenize(src).expect("lex");
        let ast = parse(&toks).expect("parse");
        lower(&ast)
    }

    /// Schema-free optimisation, dumped.
    fn opt(src: &str) -> String {
        optimize(parse_lower(src)).to_sexpr()
    }

    /// A stub catalog keyed by ref-name or by a document's first text argument.
    struct Cat(HashMap<&'static str, Vec<&'static str>>);

    impl Catalog for Cat {
        fn schema_of_source(&self, source: &Source) -> Option<Schema> {
            let key = match source {
                Source::Ref(n) => n.as_str(),
                Source::Document { args, .. } => match args.first() {
                    Some(Scalar::Lit(Lit::Text(s))) => s.as_str(),
                    _ => return None,
                },
            };
            self.0
                .get(key)
                .map(|cols| Schema::new(cols.iter().map(|s| s.to_string()).collect()))
        }
    }

    fn cat(entries: &[(&'static str, &[&'static str])]) -> Cat {
        Cat(entries.iter().map(|(k, v)| (*k, v.to_vec())).collect())
    }

    /// Catalog-aware optimisation, dumped.
    fn opt_cat(src: &str, catalog: &Cat) -> String {
        optimize_with_catalog(parse_lower(src), catalog).to_sexpr()
    }

    // --- schema-free passes (unchanged from the previous increment) -------

    #[test]
    fn conjunction_splits_into_stacked_filters() {
        assert_eq!(
            opt(r#"Table.SelectRows(t, each [a] = 1 and [b] = 2)"#),
            r#"(filter (= (col "a") (lit number "1")) (filter (= (col "b") (lit number "2")) (scan (ref "t"))))"#
        );
    }

    #[test]
    fn filter_pushes_below_sort() {
        assert_eq!(
            opt(r#"Table.SelectRows(Table.Sort(t, "x"), each [a] = 1)"#),
            r#"(sort ((asc "x")) (filter (= (col "a") (lit number "1")) (scan (ref "t"))))"#
        );
    }

    #[test]
    fn filter_on_group_key_pushes_below_aggregate() {
        assert_eq!(
            opt(r#"Table.SelectRows(Table.Group(t, {"Region"}, {{"Total", each List.Sum([Amount])}}), each [Region] = "GB")"#),
            r#"(aggregate ("Region") (("Total" sum (col "Amount"))) (filter (= (col "Region") (lit text "GB")) (scan (ref "t"))))"#
        );
    }

    #[test]
    fn filter_on_aggregate_output_stays_above() {
        assert_eq!(
            opt(r#"Table.SelectRows(Table.Group(t, {"Region"}, {{"Total", each List.Sum([Amount])}}), each [Total] > 100)"#),
            r#"(filter (> (col "Total") (lit number "100")) (aggregate ("Region") (("Total" sum (col "Amount"))) (scan (ref "t"))))"#
        );
    }

    #[test]
    fn filter_pushes_below_passthrough_project() {
        assert_eq!(
            opt(r#"Table.SelectRows(Table.SelectColumns(t, {"a", "b"}), each [a] = 1)"#),
            r#"(project replace (("a" (col "a")) ("b" (col "b"))) (filter (= (col "a") (lit number "1")) (scan (ref "t"))))"#
        );
    }

    #[test]
    fn filter_on_added_column_stays_above_project() {
        assert_eq!(
            opt(r#"Table.SelectRows(Table.AddColumn(t, "x", each [a] * 2), each [x] > 10)"#),
            r#"(filter (> (col "x") (lit number "10")) (project extend (("x" (* (col "a") (lit number "2")))) (scan (ref "t"))))"#
        );
    }

    #[test]
    fn filter_does_not_push_below_limit() {
        assert_eq!(
            opt(r#"Table.SelectRows(Table.FirstN(t, 10), each [a] = 1)"#),
            r#"(filter (= (col "a") (lit number "1")) (limit 10 0 (scan (ref "t"))))"#
        );
    }

    #[test]
    fn split_conjuncts_push_independently() {
        assert_eq!(
            opt(r#"Table.SelectRows(Table.Sort(t, "x"), each [a] = 1 and [b] = 2)"#),
            r#"(sort ((asc "x")) (filter (= (col "a") (lit number "1")) (filter (= (col "b") (lit number "2")) (scan (ref "t")))))"#
        );
    }

    #[test]
    fn no_catalog_means_no_pruning() {
        // Without a schema, the scan cannot be narrowed; plan is unchanged.
        assert_eq!(
            opt(r#"Table.SelectColumns(Parquet.Document("p"), {"a"})"#),
            r#"(project replace (("a" (col "a"))) (scan (document "Parquet.Document" (lit text "p"))))"#
        );
    }

    // --- schema analysis --------------------------------------------------

    #[test]
    fn schema_of_aggregate_is_keys_plus_aggs() {
        use super::super::schema::schema_of;
        let plan = parse_lower(
            r#"Table.Group(t, {"Region"}, {{"Total", each List.Sum([Amount])}, {"N", each Table.RowCount(_)}})"#,
        );
        let c = cat(&[("t", &["Region", "Amount", "Junk"])]);
        let s = schema_of(&plan, &c).expect("schema");
        assert_eq!(s.columns, vec!["Region", "Total", "N"]);
    }

    #[test]
    fn schema_of_evalm_is_unknown() {
        use super::super::schema::schema_of;
        let plan = parse_lower(r#"Table.Pivot(t, {"a"}, "b", "c")"#);
        let c = cat(&[("t", &["a", "b"])]);
        assert!(schema_of(&plan, &c).is_none());
    }

    // --- join pushdown (provenance) --------------------------------------

    #[test]
    fn inner_join_pushes_filter_to_owning_side() {
        let c = cat(&[("a", &["k", "x"]), ("b", &["k", "z"])]);
        // [x] belongs to the left only → pushes left.
        assert_eq!(
            opt_cat(
                r#"Table.SelectRows(Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.Inner), each [x] = 1)"#,
                &c
            ),
            r#"(join inner ("k") ("k") (filter (= (col "x") (lit number "1")) (scan (ref "a"))) (scan (ref "b")))"#
        );
        // [z] belongs to the right only → pushes right.
        assert_eq!(
            opt_cat(
                r#"Table.SelectRows(Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.Inner), each [z] = 1)"#,
                &c
            ),
            r#"(join inner ("k") ("k") (scan (ref "a")) (filter (= (col "z") (lit number "1")) (scan (ref "b"))))"#
        );
    }

    #[test]
    fn left_outer_does_not_push_to_null_supplying_side() {
        let c = cat(&[("a", &["k", "x"]), ("b", &["k", "z"])]);
        // Filter on the right (null-supplying) side of a LEFT OUTER stays above.
        assert_eq!(
            opt_cat(
                r#"Table.SelectRows(Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.LeftOuter), each [z] = 1)"#,
                &c
            ),
            r#"(filter (= (col "z") (lit number "1")) (join left ("k") ("k") (scan (ref "a")) (scan (ref "b"))))"#
        );
        // But a filter on the preserved (left) side does push.
        assert_eq!(
            opt_cat(
                r#"Table.SelectRows(Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.LeftOuter), each [x] = 1)"#,
                &c
            ),
            r#"(join left ("k") ("k") (filter (= (col "x") (lit number "1")) (scan (ref "a"))) (scan (ref "b")))"#
        );
    }

    #[test]
    fn ambiguous_column_does_not_push_into_join() {
        // `k` exists on both sides → ambiguous provenance → stays above.
        let c = cat(&[("a", &["k", "x"]), ("b", &["k", "z"])]);
        assert_eq!(
            opt_cat(
                r#"Table.SelectRows(Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.Inner), each [k] = 1)"#,
                &c
            ),
            r#"(filter (= (col "k") (lit number "1")) (join inner ("k") ("k") (scan (ref "a")) (scan (ref "b"))))"#
        );
    }

    // --- projection pruning ----------------------------------------------

    #[test]
    fn prune_narrows_scan_below_select_columns() {
        // SelectColumns is already minimal — pruning inserts then composition
        // collapses, leaving a single narrowing projection over the scan.
        let c = cat(&[("p", &["a", "b", "c"])]);
        assert_eq!(
            opt_cat(r#"Table.SelectColumns(Parquet.Document("p"), {"a"})"#, &c),
            r#"(project replace (("a" (col "a"))) (scan (document "Parquet.Document" (lit text "p"))))"#
        );
    }

    #[test]
    fn prune_narrows_scan_below_aggregate() {
        let c = cat(&[("p", &["Region", "Amount", "Junk"])]);
        assert_eq!(
            opt_cat(
                r#"Table.Group(Parquet.Document("p"), {"Region"}, {{"Total", each List.Sum([Amount])}})"#,
                &c
            ),
            r#"(aggregate ("Region") (("Total" sum (col "Amount"))) (project replace (("Region" (col "Region")) ("Amount" (col "Amount"))) (scan (document "Parquet.Document" (lit text "p")))))"#
        );
    }

    #[test]
    fn prune_narrows_both_join_inputs() {
        // The corpus pattern: join wide tables, keep two columns. Pruning pushes
        // "only these columns (plus the join keys)" down to each scan.
        let c = cat(&[("a", &["k", "x", "extra"]), ("b", &["k", "z", "more"])]);
        assert_eq!(
            opt_cat(
                r#"Table.SelectColumns(Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.Inner), {"x", "z"})"#,
                &c
            ),
            r#"(project replace (("x" (col "x")) ("z" (col "z"))) (join inner ("k") ("k") (project replace (("k" (col "k")) ("x" (col "x"))) (scan (ref "a"))) (project replace (("k" (col "k")) ("z" (col "z"))) (scan (ref "b")))))"#
        );
    }

    #[test]
    fn eval_m_blocks_pruning_below_it() {
        // Pivot is opaque — its input may consume any column, so the scan below
        // is not narrowed.
        let c = cat(&[("p", &["a", "b", "c"])]);
        assert_eq!(
            opt_cat(
                r#"Table.SelectColumns(Table.Pivot(Parquet.Document("p"), {"a"}, "b", "c"), {"a"})"#,
                &c
            ),
            r#"(project replace (("a" (col "a"))) (eval-m "Table.Pivot" (scan (document "Parquet.Document" (lit text "p")))))"#
        );
    }

    #[test]
    fn whole_row_distinct_blocks_pruning() {
        // Whole-row dedup compares every column, so none below can be dropped.
        let c = cat(&[("p", &["a", "b"])]);
        assert_eq!(
            opt_cat(r#"Table.SelectColumns(Table.Distinct(Parquet.Document("p")), {"a"})"#, &c),
            r#"(project replace (("a" (col "a"))) (distinct () (scan (document "Parquet.Document" (lit text "p")))))"#
        );
    }

    #[test]
    fn optimise_with_catalog_is_idempotent() {
        let c = cat(&[("a", &["k", "x", "extra"]), ("b", &["k", "z", "more"])]);
        let src = r#"Table.SelectColumns(Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.Inner), {"x", "z"})"#;
        let once = optimize_with_catalog(parse_lower(src), &c);
        let twice = optimize_with_catalog(once.clone(), &c);
        assert_eq!(once, twice);
    }
}
