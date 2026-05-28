//! Logical optimisation — RA→RA rewrites applied to the plan before any fold
//! pass, per `mrsflow/10-plan-ir.md` §"Logical optimisation, before folding".
//!
//! Each rewrite has an explicit equivalence precondition and is expressed as a
//! node-local rule; a generic bottom-up driver ([`optimize`]) applies them to
//! a fixpoint. These passes are the justification that survives a disappointing
//! fold percentage — they shrink what the in-memory evaluator chews on even
//! when nothing reaches a source.
//!
//! Implemented here:
//!   * **Conjunction splitting** — `Filter(and a b …)` becomes a stack of
//!     single-predicate filters, so each conjunct can then be pushed
//!     independently. Always sound.
//!   * **Filter pushdown** — push a `Filter` below the operator beneath it
//!     when the row-set is provably unchanged. Pushdown targets and their
//!     preconditions are spelled out in [`push_filter`].
//!
//! Deliberately deferred:
//!   * **Filter pushdown below `Join`** — sound only with column provenance
//!     (which side a column belongs to). The pure-AST lowering has no schema,
//!     so a bare column name could ambiguously belong to either input. Needs
//!     the schema-carrying plan a later increment will build.
//!   * **Projection pruning** — dropping unconsumed columns requires knowing
//!     the full input column set (to enumerate what a `*`/passthrough actually
//!     carries) and column provenance. Same schema dependency; deferred.

use super::ir::*;

/// Apply the logical-optimisation passes to a fixpoint and return the rewritten
/// plan. Idempotent: optimising an already-optimised plan returns it unchanged.
pub fn optimize(plan: Rel) -> Rel {
    let mut current = plan;
    loop {
        let next = rewrite(current.clone());
        if next == current {
            return next;
        }
        current = next;
    }
}

/// One bottom-up sweep: rewrite children first, then apply the local rules at
/// this node. The fixpoint loop in [`optimize`] re-runs this until stable, so a
/// rule that produces new rewritable structure (e.g. a split conjunction) is
/// picked up on the next sweep.
fn rewrite(rel: Rel) -> Rel {
    let rel = map_children(rel, rewrite);
    match rel {
        Rel::Filter { predicate, input } => rewrite_filter(predicate, *input),
        other => other,
    }
}

/// Rebuild `rel` with `f` applied to each relational child.
fn map_children(rel: Rel, f: fn(Rel) -> Rel) -> Rel {
    match rel {
        Rel::Scan(_) => rel,
        Rel::Filter { predicate, input } => Rel::Filter {
            predicate,
            input: Box::new(f(*input)),
        },
        Rel::Project { star, items, input } => Rel::Project {
            star,
            items,
            input: Box::new(f(*input)),
        },
        Rel::Sort { keys, input } => Rel::Sort {
            keys,
            input: Box::new(f(*input)),
        },
        Rel::Limit { n, offset, input } => Rel::Limit {
            n,
            offset,
            input: Box::new(f(*input)),
        },
        Rel::Aggregate { keys, aggs, input } => Rel::Aggregate {
            keys,
            aggs,
            input: Box::new(f(*input)),
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
            left: Box::new(f(*left)),
            right: Box::new(f(*right)),
        },
        Rel::Distinct { on, input } => Rel::Distinct {
            on,
            input: Box::new(f(*input)),
        },
        Rel::EvalM { descr, inputs } => Rel::EvalM {
            descr,
            inputs: inputs.into_iter().map(f).collect(),
        },
    }
}

fn rewrite_filter(predicate: Scalar, input: Rel) -> Rel {
    match predicate {
        // Conjunction splitting: Filter(and a b …) → Filter(a, Filter(b, …)).
        // Built innermost-first so the original left-to-right order is kept.
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
        other => push_filter(other, input),
    }
}

/// Push a single-predicate `Filter` below the operator beneath it when the
/// row-set is provably preserved; otherwise leave the filter in place.
///
/// Targets and preconditions:
///   * **Project** — push when every referenced column passes through the
///     project unchanged (not introduced, renamed, or computed by it).
///   * **Aggregate** — push when the predicate references only group keys
///     (rows in a group share those values, so filtering commutes with the
///     grouping). A predicate on an aggregated output is a HAVING and stays.
///   * **Sort** — always: sorting then filtering yields the same rows in the
///     same order as filtering then sorting.
///   * **Distinct** — push when the dedup is whole-row, or the predicate
///     references only the distinct keys (otherwise which row survives a
///     keyed dedup can differ from filtering first).
fn push_filter(predicate: Scalar, input: Rel) -> Rel {
    let cols = predicate_cols(&predicate);
    match input {
        Rel::Project { star, items, input: inner } if project_passthrough(&cols, star, &items) => {
            Rel::Project {
                star,
                items,
                input: Box::new(Rel::Filter {
                    predicate,
                    input: inner,
                }),
            }
        }
        Rel::Aggregate { keys, aggs, input: inner }
            if cols.as_ref().is_some_and(|c| c.iter().all(|x| keys.contains(x))) =>
        {
            Rel::Aggregate {
                keys,
                aggs,
                input: Box::new(Rel::Filter {
                    predicate,
                    input: inner,
                }),
            }
        }
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
        // Scan, Filter, Limit, Join, EvalM, and the guarded-out cases above:
        // not a safe (or not yet supported) pushdown — leave the filter here.
        other => Rel::Filter {
            predicate,
            input: Box::new(other),
        },
    }
}

/// Are all the predicate's referenced columns safe to push below this Project?
/// `None` columns (an opaque predicate) are conservatively unsafe.
fn project_passthrough(cols: &Option<Vec<String>>, star: bool, items: &[ProjectItem]) -> bool {
    let cols = match cols {
        Some(c) => c,
        None => return false,
    };
    cols.iter().all(|c| {
        match items.iter().find(|it| &it.name == c) {
            // extend (AddColumn): a named item is freshly computed/renamed → unsafe;
            // a column not in the item list is an input passthrough → safe.
            Some(_) if star => false,
            None if star => true,
            // replace (SelectColumns): safe only when the item is an identity
            // passthrough (output name == referenced input column).
            Some(it) => it.expr == Scalar::Col(c.clone()),
            // replace: referenced column is not even in the output — can't push.
            None => false,
        }
    })
}

/// May a filter be pushed below this `Distinct`?
fn distinct_commutes(cols: &Option<Vec<String>>, on: &[String]) -> bool {
    on.is_empty() || cols.as_ref().is_some_and(|c| c.iter().all(|x| on.contains(x)))
}

/// The set of columns a scalar references, or `None` if it contains an opaque
/// sub-expression (whose column footprint we cannot enumerate).
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

#[cfg(test)]
mod tests {
    use super::super::lower::lower;
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Parse, lower, optimise, and dump.
    fn opt(src: &str) -> String {
        let toks = tokenize(src).expect("lex");
        let ast = parse(&toks).expect("parse");
        optimize(lower(&ast)).to_sexpr()
    }

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
        // [Total] is an aggregated output, not a group key — this is a HAVING.
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
        // [x] is computed by the AddColumn — cannot push below it.
        assert_eq!(
            opt(r#"Table.SelectRows(Table.AddColumn(t, "x", each [a] * 2), each [x] > 10)"#),
            r#"(filter (> (col "x") (lit number "10")) (project extend (("x" (* (col "a") (lit number "2")))) (scan (ref "t"))))"#
        );
    }

    #[test]
    fn filter_on_other_column_pushes_below_added_column() {
        // [a] passes through the AddColumn untouched — safe to push.
        assert_eq!(
            opt(r#"Table.SelectRows(Table.AddColumn(t, "x", each [a] * 2), each [a] = 1)"#),
            r#"(project extend (("x" (* (col "a") (lit number "2")))) (filter (= (col "a") (lit number "1")) (scan (ref "t"))))"#
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
        // Both conjuncts are sort-transparent, so both end up below the sort.
        assert_eq!(
            opt(r#"Table.SelectRows(Table.Sort(t, "x"), each [a] = 1 and [b] = 2)"#),
            r#"(sort ((asc "x")) (filter (= (col "a") (lit number "1")) (filter (= (col "b") (lit number "2")) (scan (ref "t")))))"#
        );
    }

    #[test]
    fn opaque_predicate_does_not_push_below_project() {
        // No column footprint → conservatively kept above the project.
        assert_eq!(
            opt(r#"Table.SelectRows(Table.SelectColumns(t, {"a"}), each MyFunc([a]))"#),
            r#"(filter (opaque) (project replace (("a" (col "a"))) (scan (ref "t"))))"#
        );
    }

    #[test]
    fn optimise_is_idempotent() {
        let src = r#"Table.SelectRows(Table.Sort(t, "x"), each [a] = 1 and [b] = 2)"#;
        let toks = tokenize(src).unwrap();
        let ast = parse(&toks).unwrap();
        let once = optimize(lower(&ast));
        let twice = optimize(once.clone());
        assert_eq!(once, twice);
    }
}
