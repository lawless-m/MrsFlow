//! Lowering: M AST → Plan IR.
//!
//! Walks the M AST and recognises the relational spine — the chain of
//! `Table.*` transforms over a source leaf — producing a [`Rel`] tree. The
//! canonical Power-Query shape is a `let` whose steps each reference the
//! previous one; [`Lowerer`] threads a binding map so that chain collapses
//! into a single nested plan rather than a forest of opaque references.
//!
//! Anything that does not map to a relational operator (or a scalar form) is
//! represented honestly: [`Rel::EvalM`] at the relational level, [`Scalar::Opaque`]
//! at the scalar level. Lowering never guesses — an unrecognised operation is
//! a fold boundary, not a wrong answer.

use std::collections::HashMap;

use crate::parser::{BinaryOp, Expr, ListItem, UnaryOp};

use super::ir::*;

/// Lower a full M expression to its logical relational plan.
pub fn lower(expr: &Expr) -> Rel {
    Lowerer::default().lower_rel(expr)
}

#[derive(Default)]
struct Lowerer {
    /// let-binding name → its lowered plan. Earlier bindings are visible to
    /// later ones, matching M's sequential `let` scoping for the step chain.
    bindings: HashMap<String, Rel>,
}

impl Lowerer {
    fn lower_rel(&mut self, expr: &Expr) -> Rel {
        match expr {
            Expr::Let { bindings, body } => {
                for (name, value) in bindings {
                    let r = self.lower_rel(value);
                    self.bindings.insert(name.clone(), r);
                }
                self.lower_rel(body)
            }
            Expr::Identifier(name) => match self.bindings.get(name) {
                Some(r) => r.clone(),
                None => Rel::Scan(Source::Ref(name.clone())),
            },
            Expr::Invoke { target, args } => match target.as_ref() {
                Expr::Identifier(func) => self.lower_invoke(func, args, expr),
                _ => self.eval_m(expr),
            },
            other => self.eval_m(other),
        }
    }

    fn lower_invoke(&mut self, func: &str, args: &[Expr], full: &Expr) -> Rel {
        match func {
            "Table.SelectRows" if args.len() >= 2 => {
                let input = self.lower_rel(&args[0]);
                let predicate = lower_each(&args[1]);
                Rel::Filter {
                    predicate,
                    input: Box::new(input),
                }
            }
            "Table.SelectColumns" if args.len() >= 2 => {
                let input = self.lower_rel(&args[0]);
                match column_name_list(&args[1]) {
                    Some(names) => Rel::Project {
                        star: false,
                        items: names
                            .into_iter()
                            .map(|n| ProjectItem {
                                expr: Scalar::Col(n.clone()),
                                name: n,
                            })
                            .collect(),
                        input: Box::new(input),
                    },
                    None => eval_m_over("Table.SelectColumns", input),
                }
            }
            "Table.AddColumn" if args.len() >= 3 => {
                let input = self.lower_rel(&args[0]);
                match text_lit(&args[1]) {
                    Some(name) => Rel::Project {
                        star: true,
                        items: vec![ProjectItem {
                            name,
                            expr: lower_each(&args[2]),
                        }],
                        input: Box::new(input),
                    },
                    None => eval_m_over("Table.AddColumn", input),
                }
            }
            "Table.Sort" if args.len() >= 2 => {
                let input = self.lower_rel(&args[0]);
                match sort_keys(&args[1]) {
                    Some(keys) => Rel::Sort {
                        keys,
                        input: Box::new(input),
                    },
                    None => eval_m_over("Table.Sort", input),
                }
            }
            "Table.FirstN" if args.len() >= 2 => {
                let input = self.lower_rel(&args[0]);
                match number_lit(&args[1]).and_then(parse_count) {
                    // A function in the count slot is a "take while" condition,
                    // not a row limit — that does not fold to LIMIT.
                    Some(n) => Rel::Limit {
                        n: Some(n),
                        offset: 0,
                        input: Box::new(input),
                    },
                    None => eval_m_over("Table.FirstN", input),
                }
            }
            "Table.Range" if args.len() >= 2 => {
                let input = self.lower_rel(&args[0]);
                let offset = number_lit(&args[1]).and_then(parse_count);
                // A count argument that is present but not a plain number means
                // we cannot represent the slice — bail to EvalM rather than
                // silently drop it.
                let count = match args.get(2) {
                    None => Ok(None),
                    Some(a) => match number_lit(a).and_then(parse_count) {
                        Some(n) => Ok(Some(n)),
                        None => Err(()),
                    },
                };
                match (offset, count) {
                    (Some(offset), Ok(n)) => Rel::Limit {
                        n,
                        offset,
                        input: Box::new(input),
                    },
                    _ => eval_m_over("Table.Range", input),
                }
            }
            "Table.Group" if args.len() >= 3 => {
                let input = self.lower_rel(&args[0]);
                let keys = match column_name_list(&args[1]) {
                    Some(k) => k,
                    None => return eval_m_over("Table.Group", input),
                };
                match aggregations(&args[2]) {
                    Some(aggs) => Rel::Aggregate {
                        keys: keys.into_iter().map(Scalar::Col).collect(),
                        aggs,
                        input: Box::new(input),
                    },
                    None => eval_m_over("Table.Group", input),
                }
            }
            "Table.NestedJoin" if args.len() >= 5 => {
                let left = self.lower_rel(&args[0]);
                let left_keys = column_name_list(&args[1]);
                let right = self.lower_rel(&args[2]);
                let right_keys = column_name_list(&args[3]);
                // args[4] is the nested column name; args[5] (optional) the kind.
                let kind = args.get(5).map(join_kind).unwrap_or(JoinKind::LeftOuter);
                match (left_keys, right_keys) {
                    (Some(lk), Some(rk)) => Rel::Join {
                        kind,
                        left_keys: lk,
                        right_keys: rk,
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                    _ => Rel::EvalM {
                        descr: "Table.NestedJoin".to_string(),
                        inputs: vec![left, right],
                    },
                }
            }
            "Table.Distinct" if !args.is_empty() => {
                let input = self.lower_rel(&args[0]);
                match args.get(1) {
                    None => Rel::Distinct {
                        on: Vec::new(),
                        input: Box::new(input),
                    },
                    Some(a) => match column_name_list(a) {
                        Some(on) => Rel::Distinct {
                            on,
                            input: Box::new(input),
                        },
                        // A comparer or other criteria we cannot model as keys.
                        None => eval_m_over("Table.Distinct", input),
                    },
                }
            }
            _ if is_source(func) => Rel::Scan(Source::Document {
                func: func.to_string(),
                args: args.iter().map(|a| lower_scalar(a, "")).collect(),
            }),
            _ => self.eval_m(full),
        }
    }

    /// Build an `EvalM` for an unrecognised expression, lowering any arguments
    /// that look like table pipelines so the foldable spine below stays visible.
    fn eval_m(&mut self, expr: &Expr) -> Rel {
        let descr = descr_of(expr);
        let mut inputs = Vec::new();
        if let Expr::Invoke { args, .. } = expr {
            for a in args {
                if self.looks_like_table(a) {
                    inputs.push(self.lower_rel(a));
                }
            }
        }
        Rel::EvalM { descr, inputs }
    }

    fn looks_like_table(&self, e: &Expr) -> bool {
        match e {
            Expr::Let { .. } => true,
            Expr::Identifier(n) => self.bindings.contains_key(n),
            Expr::Invoke { target, .. } => {
                matches!(target.as_ref(), Expr::Identifier(f) if is_table_func(f))
            }
            _ => false,
        }
    }
}

/// An `EvalM` wrapping a single already-lowered relational input.
fn eval_m_over(descr: &str, input: Rel) -> Rel {
    Rel::EvalM {
        descr: descr.to_string(),
        inputs: vec![input],
    }
}

// --- Scalar lowering ------------------------------------------------------

/// Lower an `each`/function argument to a scalar, against the body's parameter
/// name. Non-unary functions and anything else are opaque.
fn lower_each(expr: &Expr) -> Scalar {
    match expr {
        Expr::Each(body) => lower_scalar(body, "_"),
        Expr::Function { params, body, .. } if params.len() == 1 => {
            lower_scalar(body, &params[0].name)
        }
        _ => Scalar::Opaque,
    }
}

/// Lower a scalar expression. `param` is the row parameter so `[col]`
/// (desugared to `FieldAccess` on the parameter) becomes a column reference.
/// Structure is preserved even when a sub-expression is opaque — that lets a
/// later conjunction-split push the foldable part of an `and`.
fn lower_scalar(expr: &Expr, param: &str) -> Scalar {
    match expr {
        Expr::FieldAccess {
            target,
            field,
            optional: false,
        } => match target.as_ref() {
            Expr::Identifier(n) if n == param => Scalar::Col(field.clone()),
            _ => Scalar::Opaque,
        },
        Expr::NumberLit(s) => Scalar::Lit(Lit::Number(s.clone())),
        Expr::TextLit(s) => Scalar::Lit(Lit::Text(s.clone())),
        Expr::LogicalLit(b) => Scalar::Lit(Lit::Logical(*b)),
        Expr::NullLit => Scalar::Lit(Lit::Null),
        Expr::Unary(UnaryOp::Not, inner) => Scalar::Bool {
            op: BoolOp::Not,
            args: vec![lower_scalar(inner, param)],
        },
        // Signed numeric literals: fold `-5` / `+5` into the literal so they
        // reach the emitter, rather than stranding the comparison as opaque.
        Expr::Unary(UnaryOp::Minus, inner) => match inner.as_ref() {
            Expr::NumberLit(s) => Scalar::Lit(Lit::Number(format!("-{s}"))),
            _ => Scalar::Opaque,
        },
        Expr::Unary(UnaryOp::Plus, inner) => lower_scalar(inner, param),
        Expr::Binary(op, l, r) => lower_binary(*op, l, r, param),
        Expr::Invoke { target, args } => match target.as_ref() {
            Expr::Identifier(func) if is_scalar_call(func) => Scalar::Call {
                func: func.clone(),
                args: args.iter().map(|a| lower_scalar(a, param)).collect(),
            },
            _ => Scalar::Opaque,
        },
        _ => Scalar::Opaque,
    }
}

fn lower_binary(op: BinaryOp, l: &Expr, r: &Expr, param: &str) -> Scalar {
    let cmp = |o| Scalar::Cmp {
        op: o,
        lhs: Box::new(lower_scalar(l, param)),
        rhs: Box::new(lower_scalar(r, param)),
    };
    let arith = |o| Scalar::Arith {
        op: o,
        lhs: Box::new(lower_scalar(l, param)),
        rhs: Box::new(lower_scalar(r, param)),
    };
    let boolean = |o| Scalar::Bool {
        op: o,
        args: vec![lower_scalar(l, param), lower_scalar(r, param)],
    };
    match op {
        BinaryOp::Equal => cmp(CmpOp::Eq),
        BinaryOp::NotEqual => cmp(CmpOp::Ne),
        BinaryOp::LessThan => cmp(CmpOp::Lt),
        BinaryOp::LessEquals => cmp(CmpOp::Le),
        BinaryOp::GreaterThan => cmp(CmpOp::Gt),
        BinaryOp::GreaterEquals => cmp(CmpOp::Ge),
        BinaryOp::And => boolean(BoolOp::And),
        BinaryOp::Or => boolean(BoolOp::Or),
        BinaryOp::Add => arith(ArithOp::Add),
        BinaryOp::Subtract => arith(ArithOp::Sub),
        BinaryOp::Multiply => arith(ArithOp::Mul),
        BinaryOp::Divide => arith(ArithOp::Div),
        // Concatenation, type relations, and metadata have no scalar form yet.
        BinaryOp::Concat | BinaryOp::As | BinaryOp::Is | BinaryOp::Meta => Scalar::Opaque,
    }
}

// --- Argument parsing helpers --------------------------------------------

/// A bare text literal or a list of text literals → a list of column names.
fn column_name_list(expr: &Expr) -> Option<Vec<String>> {
    match expr {
        Expr::TextLit(s) => Some(vec![s.clone()]),
        Expr::List(items) => {
            let mut out = Vec::with_capacity(items.len());
            for it in items {
                match it {
                    ListItem::Single(Expr::TextLit(s)) => out.push(s.clone()),
                    _ => return None,
                }
            }
            Some(out)
        }
        _ => None,
    }
}

fn text_lit(expr: &Expr) -> Option<String> {
    match expr {
        Expr::TextLit(s) => Some(s.clone()),
        _ => None,
    }
}

fn number_lit(expr: &Expr) -> Option<&str> {
    match expr {
        Expr::NumberLit(s) => Some(s.as_str()),
        _ => None,
    }
}

/// Parse a numeric lexeme as a non-negative row count.
fn parse_count(lexeme: &str) -> Option<u64> {
    let t = lexeme.trim();
    if let Ok(n) = t.parse::<u64>() {
        return Some(n);
    }
    let f = t.parse::<f64>().ok()?;
    // Only an exact non-negative integer in u64 range folds into a pushed-down
    // LIMIT. A fractional count (FirstN(t, 2.7)) or out-of-range one
    // (Range(t, 0, 1e30) saturating to u64::MAX) must return None so the
    // caller falls back to the evaluator, which truncates/errors per M
    // semantics — folding it would diverge silently.
    if f.is_finite() && f >= 0.0 && f.fract() == 0.0 && f < 18_446_744_073_709_551_616.0 {
        Some(f as u64)
    } else {
        None
    }
}

/// `Table.Sort` keys: `"Col"`, or a list of `"Col"` / `{"Col", Order.X}`.
fn sort_keys(expr: &Expr) -> Option<Vec<SortKey>> {
    match expr {
        Expr::TextLit(s) => Some(vec![SortKey {
            column: s.clone(),
            descending: false,
        }]),
        Expr::List(items) => {
            let mut keys = Vec::with_capacity(items.len());
            for it in items {
                let inner = match it {
                    ListItem::Single(e) => e,
                    _ => return None,
                };
                let key = match inner {
                    Expr::TextLit(s) => SortKey {
                        column: s.clone(),
                        descending: false,
                    },
                    Expr::List(pair) => {
                        let column = match pair.first() {
                            Some(ListItem::Single(Expr::TextLit(s))) => s.clone(),
                            _ => return None,
                        };
                        let descending = match pair.get(1) {
                            None => false,
                            Some(ListItem::Single(e)) => sort_descending(e)?,
                            _ => return None,
                        };
                        SortKey { column, descending }
                    }
                    _ => return None,
                };
                keys.push(key);
            }
            Some(keys)
        }
        _ => None,
    }
}

fn sort_descending(order: &Expr) -> Option<bool> {
    match order {
        Expr::Identifier(n) if n == "Order.Ascending" => Some(false),
        Expr::Identifier(n) if n == "Order.Descending" => Some(true),
        Expr::NumberLit(s) => match parse_count(s)? {
            0 => Some(false),
            1 => Some(true),
            _ => None,
        },
        _ => None,
    }
}

/// `Table.Group` aggregations: a list of `{"Name", each f([Col]), type}`.
fn aggregations(expr: &Expr) -> Option<Vec<Aggregation>> {
    let items = match expr {
        Expr::List(items) => items,
        _ => return None,
    };
    let mut out = Vec::with_capacity(items.len());
    for it in items {
        let spec = match it {
            ListItem::Single(Expr::List(spec)) => spec,
            _ => return None,
        };
        let name = match spec.first() {
            Some(ListItem::Single(Expr::TextLit(s))) => s.clone(),
            _ => return None,
        };
        let body = match spec.get(1) {
            Some(ListItem::Single(e)) => e,
            _ => return None,
        };
        let (func, column) = agg_body(body);
        out.push(Aggregation {
            name,
            func,
            column: column.map(Scalar::Col),
        });
    }
    Some(out)
}

/// Classify a single aggregation's `each` body. Recognises `List.<Agg>([Col])`
/// and `Table.RowCount(_)`; anything else is `Opaque` so the `Aggregate` node
/// is still produced but cannot fold.
fn agg_body(each_expr: &Expr) -> (AggFunc, Option<String>) {
    let body = match each_expr {
        Expr::Each(b) => b.as_ref(),
        Expr::Function { params, body, .. } if params.len() == 1 => body.as_ref(),
        _ => return (AggFunc::Opaque, None),
    };
    let (func_name, args) = match body {
        Expr::Invoke { target, args } => match target.as_ref() {
            Expr::Identifier(f) => (f.as_str(), args),
            _ => return (AggFunc::Opaque, None),
        },
        _ => return (AggFunc::Opaque, None),
    };
    let func = match func_name {
        "List.Sum" => AggFunc::Sum,
        "List.Average" => AggFunc::Average,
        "List.Min" => AggFunc::Min,
        "List.Max" => AggFunc::Max,
        "List.Count" => AggFunc::Count,
        "Table.RowCount" => return (AggFunc::Count, None),
        _ => return (AggFunc::Opaque, None),
    };
    let column = args.first().and_then(|a| match a {
        Expr::FieldAccess {
            target,
            field,
            optional: false,
        } => match target.as_ref() {
            Expr::Identifier(n) if n == "_" => Some(field.clone()),
            _ => None,
        },
        _ => None,
    });
    (func, column)
}

/// `JoinKind.X` identifier or the integer code; defaults to `LeftOuter`
/// (the `Table.NestedJoin` default) when absent or unrecognised.
fn join_kind(expr: &Expr) -> JoinKind {
    match expr {
        Expr::Identifier(n) => match n.as_str() {
            "JoinKind.Inner" => JoinKind::Inner,
            "JoinKind.LeftOuter" => JoinKind::LeftOuter,
            "JoinKind.RightOuter" => JoinKind::RightOuter,
            "JoinKind.FullOuter" => JoinKind::FullOuter,
            "JoinKind.LeftAnti" => JoinKind::LeftAnti,
            "JoinKind.RightAnti" => JoinKind::RightAnti,
            _ => JoinKind::LeftOuter,
        },
        Expr::NumberLit(s) => match parse_count(s) {
            Some(0) => JoinKind::Inner,
            Some(1) => JoinKind::LeftOuter,
            Some(2) => JoinKind::RightOuter,
            Some(3) => JoinKind::FullOuter,
            Some(4) => JoinKind::LeftAnti,
            Some(5) => JoinKind::RightAnti,
            _ => JoinKind::LeftOuter,
        },
        _ => JoinKind::LeftOuter,
    }
}

fn descr_of(expr: &Expr) -> String {
    match expr {
        Expr::Invoke { target, .. } => match target.as_ref() {
            Expr::Identifier(f) => f.clone(),
            _ => "invoke".to_string(),
        },
        Expr::Record(_) => "record".to_string(),
        Expr::List(_) => "list".to_string(),
        Expr::If { .. } => "if".to_string(),
        Expr::FieldAccess { .. } => "field-access".to_string(),
        Expr::ItemAccess { .. } => "item-access".to_string(),
        _ => "expr".to_string(),
    }
}

/// Leaf constructors that begin a scan. The `.Document` family is matched by
/// suffix; the rest are named explicitly.
fn is_source(func: &str) -> bool {
    func.ends_with(".Document")
        || matches!(
            func,
            "Odbc.DataSource"
                | "Odbc.Query"
                | "Sql.Database"
                | "Sql.Databases"
                | "PostgreSQL.Database"
                | "MySQL.Database"
                | "Value.NativeQuery"
                | "Excel.Workbook"
                | "Web.Contents"
                | "Folder.Files"
                | "Exportmaster.Database"
                | "Exportmaster.Contents"
        )
}

/// Does this function name produce a table? Used to decide which arguments of
/// an unrecognised call to keep lowering as relational inputs.
fn is_table_func(func: &str) -> bool {
    func.starts_with("Table.") || is_source(func)
}

/// The bounded allow-list of scalar M functions with SQL analogues. Kept
/// intentionally small — everything off the list lowers to `Opaque`.
fn is_scalar_call(func: &str) -> bool {
    matches!(
        func,
        "Text.Upper"
            | "Text.Lower"
            | "Text.Length"
            | "Text.Trim"
            | "Text.Start"
            | "Text.End"
            | "Text.Range"
            | "Text.Contains"
            | "Text.StartsWith"
            | "Text.EndsWith"
            | "Text.Replace"
            | "Number.Round"
            | "Number.RoundDown"
            | "Number.RoundUp"
            | "Number.Abs"
            | "Date.Year"
            | "Date.Month"
            | "Date.Day"
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    /// Parse M source and lower it to the Plan-IR S-expression.
    fn plan(src: &str) -> String {
        let toks = tokenize(src).expect("lex");
        let ast = parse(&toks).expect("parse");
        lower(&ast).to_sexpr()
    }

    #[test]
    fn scan_leaf() {
        assert_eq!(
            plan(r#"Parquet.Document("/data/sales.parquet")"#),
            r#"(scan (document "Parquet.Document" (lit text "/data/sales.parquet")))"#
        );
    }

    #[test]
    fn bare_identifier_is_ref_scan() {
        assert_eq!(plan("Source"), r#"(scan (ref "Source"))"#);
    }

    #[test]
    fn filter_over_scan() {
        // The worked example from the design doc.
        assert_eq!(
            plan(r#"Table.SelectRows(Parquet.Document("/data/sales.parquet"), each [Country] = "GB")"#),
            r#"(filter (= (col "Country") (lit text "GB")) (scan (document "Parquet.Document" (lit text "/data/sales.parquet"))))"#
        );
    }

    #[test]
    fn select_columns_replaces() {
        assert_eq!(
            plan(r#"Table.SelectColumns(t, {"a", "b"})"#),
            r#"(project replace (("a" (col "a")) ("b" (col "b"))) (scan (ref "t")))"#
        );
    }

    #[test]
    fn add_column_extends() {
        assert_eq!(
            plan(r#"Table.AddColumn(t, "double", each [a] * 2)"#),
            r#"(project extend (("double" (* (col "a") (lit number "2")))) (scan (ref "t")))"#
        );
    }

    #[test]
    fn sort_keys_with_direction() {
        assert_eq!(
            plan(r#"Table.Sort(t, {{"a", Order.Ascending}, {"b", Order.Descending}})"#),
            r#"(sort ((asc "a") (desc "b")) (scan (ref "t")))"#
        );
    }

    #[test]
    fn first_n_is_limit() {
        assert_eq!(
            plan("Table.FirstN(t, 10)"),
            r#"(limit 10 0 (scan (ref "t")))"#
        );
    }

    #[test]
    fn range_is_limit_with_offset() {
        assert_eq!(
            plan("Table.Range(t, 5, 20)"),
            r#"(limit 20 5 (scan (ref "t")))"#
        );
    }

    #[test]
    fn group_with_sum() {
        assert_eq!(
            plan(r#"Table.Group(t, {"Region"}, {{"Total", each List.Sum([Amount]), type number}})"#),
            r#"(aggregate ((col "Region")) (("Total" sum (col "Amount"))) (scan (ref "t")))"#
        );
    }

    #[test]
    fn group_row_count_has_no_column() {
        assert_eq!(
            plan(r#"Table.Group(t, {"Region"}, {{"N", each Table.RowCount(_)}})"#),
            r#"(aggregate ((col "Region")) (("N" count)) (scan (ref "t")))"#
        );
    }

    #[test]
    fn nested_join_is_join() {
        assert_eq!(
            plan(r#"Table.NestedJoin(a, {"k"}, b, {"k"}, "nested", JoinKind.Inner)"#),
            r#"(join inner ("k") ("k") (scan (ref "a")) (scan (ref "b")))"#
        );
    }

    #[test]
    fn distinct_passthrough() {
        assert_eq!(plan("Table.Distinct(t)"), r#"(distinct () (scan (ref "t")))"#);
    }

    #[test]
    fn let_chain_collapses_to_nested_plan() {
        let src = r#"
            let
                Source = Parquet.Document("/data/sales.parquet"),
                GB = Table.SelectRows(Source, each [Country] = "GB"),
                Cols = Table.SelectColumns(GB, {"Region", "Amount"})
            in
                Cols
        "#;
        assert_eq!(
            plan(src),
            r#"(project replace (("Region" (col "Region")) ("Amount" (col "Amount"))) (filter (= (col "Country") (lit text "GB")) (scan (document "Parquet.Document" (lit text "/data/sales.parquet")))))"#
        );
    }

    #[test]
    fn unrecognised_op_is_eval_m_over_spine() {
        // Table.Pivot is not in the node set; the scan below it stays visible.
        assert_eq!(
            plan(r#"Table.Pivot(Parquet.Document("p.parquet"), {"a"}, "b", "c")"#),
            r#"(eval-m "Table.Pivot" (scan (document "Parquet.Document" (lit text "p.parquet"))))"#
        );
    }

    #[test]
    fn opaque_predicate_stays_opaque() {
        // A cross-column comparison has no foldable scalar form, but the
        // Filter node and its structure are still produced.
        assert_eq!(
            plan(r#"Table.SelectRows(t, each [a] > [b])"#),
            r#"(filter (> (col "a") (col "b")) (scan (ref "t")))"#
        );
    }

    #[test]
    fn conjunction_keeps_structure() {
        assert_eq!(
            plan(r#"Table.SelectRows(t, each [a] = 1 and [b] = 2)"#),
            r#"(filter (and (= (col "a") (lit number "1")) (= (col "b") (lit number "2"))) (scan (ref "t")))"#
        );
    }

    #[test]
    fn scalar_call_allow_list() {
        assert_eq!(
            plan(r#"Table.SelectRows(t, each Text.Upper([name]) = "X")"#),
            r#"(filter (= (call "Text.Upper" (col "name")) (lit text "X")) (scan (ref "t")))"#
        );
        // Off-list functions are opaque.
        assert_eq!(
            plan(r#"Table.SelectRows(t, each MyFunc([name]) = "X")"#),
            r#"(filter (= (opaque) (lit text "X")) (scan (ref "t")))"#
        );
    }

    #[test]
    fn signed_number_literals_fold() {
        assert_eq!(
            plan(r#"Table.SelectRows(t, each [a] = -5)"#),
            r#"(filter (= (col "a") (lit number "-5")) (scan (ref "t")))"#
        );
        assert_eq!(
            plan(r#"Table.SelectRows(t, each [a] = +5)"#),
            r#"(filter (= (col "a") (lit number "5")) (scan (ref "t")))"#
        );
    }
}
