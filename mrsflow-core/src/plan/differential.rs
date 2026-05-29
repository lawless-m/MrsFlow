//! The differential fold harness — Gate 2 of the fold pass, per
//! `mrsflow/10-plan-ir.md` §"Differential gate — no Excel in the loop".
//!
//! A fold class is only safe to enable once this proves it: the same query,
//! over the same source rows, run two ways and diffed —
//!   * **reference route** — the whole plan evaluated by mrsflow's own
//!     operator semantics;
//!   * **folded route** — the maximal foldable subtree executed by the
//!     connector, the residual run by mrsflow over the rows it returns.
//! Divergences become Gate-2 fold-exclusion rules.
//!
//! Both routes live inside mrsflow, so no Excel is needed. To stay
//! self-contained (no live DBISAM), the folded route *models* the connector by
//! interpreting the folded subtree under an explicit [`Semantics`] — collation,
//! NULL ordering, integer division: the usual suspects in an engine of that
//! vintage. When the two semantics agree, the routes must agree, which is how
//! the doc says to validate the instrument against trusted (filter-and-project)
//! behaviour first. When a modeled quirk changes the result, [`differential`]
//! reports it — that finding is a Gate-2 exclusion. When wiring to a real
//! connector later, the folded route's interpreter is replaced by actual SQL
//! execution; the diff machinery is unchanged.

use std::collections::HashMap;

use super::fold::{fold, Dbisam, FOLDED};
use super::ir::*;
use super::schema::{Catalog, Schema};

/// A scalar value in the harness's in-memory tables. Deliberately small — this
/// is a semantics reference for the foldable IR classes, not the production
/// value type.
#[derive(Debug, Clone, PartialEq)]
pub enum Cell {
    Null,
    Int(i64),
    Num(f64),
    Text(String),
    Bool(bool),
    Date(chrono::NaiveDate),
    Datetime(chrono::NaiveDateTime),
}

/// An in-memory table: named columns and row-major cells.
#[derive(Debug, Clone, PartialEq)]
pub struct Table {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<Cell>>,
}

/// The in-memory source: tables keyed by the name a `Scan(Ref …)` carries.
#[derive(Debug, Default)]
pub struct Db {
    tables: HashMap<String, Table>,
}

impl Db {
    pub fn new() -> Self {
        Db::default()
    }

    pub fn with(mut self, name: &str, columns: &[&str], rows: Vec<Vec<Cell>>) -> Self {
        self.tables.insert(
            name.to_string(),
            Table {
                columns: columns.iter().map(|s| s.to_string()).collect(),
                rows,
            },
        );
        self
    }

    fn get(&self, name: &str) -> Option<&Table> {
        self.tables.get(name)
    }
}

impl Catalog for Db {
    fn schema_of_source(&self, source: &Source) -> Option<Schema> {
        let name = match source {
            Source::Ref(n) => n.as_str(),
            Source::Document { args, .. } => match args.first() {
                Some(Scalar::Lit(Lit::Text(s))) => s.as_str(),
                _ => return None,
            },
        };
        self.get(name).map(|t| Schema::new(t.columns.clone()))
    }
}

/// The semantic knobs a backend can differ on. The reference (mrsflow) values
/// are [`Semantics::mrsflow`]; a backend that diverges on any of these forces
/// the affected fold class onto the exclusion list.
#[derive(Debug, Clone, Copy)]
pub struct Semantics {
    /// Text comparison ignores case (a stand-in for a collation difference).
    pub case_insensitive_text: bool,
    /// NULLs sort before non-NULLs in `ORDER BY` (otherwise after).
    pub nulls_first: bool,
    /// `/` between two integers truncates toward zero (otherwise true division).
    pub integer_division: bool,
}

impl Semantics {
    /// mrsflow's own semantics: case-sensitive, NULLs first, true division.
    pub fn mrsflow() -> Self {
        Semantics {
            case_insensitive_text: false,
            nulls_first: true,
            integer_division: false,
        }
    }
}

/// A detected disagreement between the two routes.
#[derive(Debug, Clone, PartialEq)]
pub struct Divergence(pub String);

/// Run `plan` both ways over `db` and diff. `Ok(())` means the folded route
/// (under `fold_sem`) matches the reference route (under `ref_sem`) — the fold
/// is safe for that semantics. `Err` carries the first disagreement.
pub fn differential(
    plan: &Rel,
    db: &Db,
    fold_sem: &Semantics,
    ref_sem: &Semantics,
) -> Result<(), Divergence> {
    let reference = interpret(plan, db, None, ref_sem).map_err(Divergence)?;

    let f = fold(plan, &Dbisam);
    let folded_route = match &f.folded {
        // The folded subtree runs under the connector's semantics; the residual
        // runs over its rows under mrsflow's.
        Some(sub) => {
            let pushed = interpret(sub, db, None, fold_sem).map_err(Divergence)?;
            interpret(&f.residual, db, Some(&pushed), ref_sem).map_err(Divergence)?
        }
        // Nothing folded — the routes are identical by construction.
        None => reference.clone(),
    };

    diff(&reference, &folded_route)
}

fn diff(reference: &Table, folded: &Table) -> Result<(), Divergence> {
    if reference.columns != folded.columns {
        return Err(Divergence(format!(
            "column mismatch: reference {:?} vs folded {:?}",
            reference.columns, folded.columns
        )));
    }
    if reference.rows.len() != folded.rows.len() {
        return Err(Divergence(format!(
            "row count mismatch: reference {} vs folded {}",
            reference.rows.len(),
            folded.rows.len()
        )));
    }
    for (i, (a, b)) in reference.rows.iter().zip(&folded.rows).enumerate() {
        if a != b {
            return Err(Divergence(format!(
                "row {i} differs: reference {a:?} vs folded {b:?}"
            )));
        }
    }
    Ok(())
}

// --- reference interpreter for the foldable IR classes --------------------

/// Evaluate a plan to a table. `folded` supplies the rows that stand in for the
/// [`FOLDED`] sentinel leaf (the connector's result) when present.
fn interpret(
    rel: &Rel,
    db: &Db,
    folded: Option<&Table>,
    sem: &Semantics,
) -> Result<Table, String> {
    match rel {
        Rel::Scan(Source::Ref(name)) => db
            .get(name)
            .cloned()
            .ok_or_else(|| format!("unknown table {name}")),
        Rel::Scan(Source::Document { .. }) => Err("cannot interpret a document scan".to_string()),

        Rel::EvalM { descr, inputs } if descr == FOLDED && inputs.is_empty() => folded
            .cloned()
            .ok_or_else(|| "folded sentinel without supplied rows".to_string()),
        Rel::EvalM { descr, .. } => Err(format!("cannot interpret opaque step {descr}")),
        Rel::Join { .. } => Err("join interpretation out of scope for the harness".to_string()),

        Rel::Filter { predicate, input } => {
            let t = interpret(input, db, folded, sem)?;
            let mut rows = Vec::new();
            for row in &t.rows {
                if truthy(&eval_scalar(predicate, &t.columns, row, sem)?) {
                    rows.push(row.clone());
                }
            }
            Ok(Table { columns: t.columns, rows })
        }

        Rel::Project { star, items, input } => {
            let t = interpret(input, db, folded, sem)?;
            let mut columns = if *star { t.columns.clone() } else { Vec::new() };
            columns.extend(items.iter().map(|it| it.name.clone()));
            let mut rows = Vec::with_capacity(t.rows.len());
            for row in &t.rows {
                let mut out = if *star { row.clone() } else { Vec::new() };
                for it in items {
                    out.push(eval_scalar(&it.expr, &t.columns, row, sem)?);
                }
                rows.push(out);
            }
            Ok(Table { columns, rows })
        }

        Rel::Sort { keys, input } => {
            let mut t = interpret(input, db, folded, sem)?;
            let idx: Vec<usize> = keys
                .iter()
                .map(|k| col_index(&t.columns, &k.column))
                .collect::<Result<_, _>>()?;
            // Stable sort so equal keys keep input order.
            let mut err = None;
            t.rows.sort_by(|a, b| {
                for (k, &ci) in keys.iter().zip(&idx) {
                    let ord = match order_cells(&a[ci], &b[ci], sem) {
                        Ok(o) => o,
                        Err(e) => {
                            err = Some(e);
                            std::cmp::Ordering::Equal
                        }
                    };
                    let ord = if k.descending { ord.reverse() } else { ord };
                    if ord != std::cmp::Ordering::Equal {
                        return ord;
                    }
                }
                std::cmp::Ordering::Equal
            });
            match err {
                Some(e) => Err(e),
                None => Ok(t),
            }
        }

        Rel::Limit { n, offset, input } => {
            let t = interpret(input, db, folded, sem)?;
            let start = (*offset as usize).min(t.rows.len());
            let end = match n {
                Some(k) => (start + *k as usize).min(t.rows.len()),
                None => t.rows.len(),
            };
            Ok(Table {
                columns: t.columns,
                rows: t.rows[start..end].to_vec(),
            })
        }

        Rel::Distinct { on, input } => {
            let t = interpret(input, db, folded, sem)?;
            let key_idx: Vec<usize> = if on.is_empty() {
                (0..t.columns.len()).collect()
            } else {
                on.iter()
                    .map(|c| col_index(&t.columns, c))
                    .collect::<Result<_, _>>()?
            };
            let mut seen: Vec<Vec<Cell>> = Vec::new();
            let mut rows = Vec::new();
            for row in &t.rows {
                let key: Vec<Cell> = key_idx.iter().map(|&i| row[i].clone()).collect();
                if !seen.contains(&key) {
                    seen.push(key);
                    rows.push(row.clone());
                }
            }
            Ok(Table { columns: t.columns, rows })
        }

        Rel::Aggregate { keys, aggs, input } => {
            let t = interpret(input, db, folded, sem)?;
            let key_idx: Vec<usize> = keys
                .iter()
                .map(|c| col_index(&t.columns, c))
                .collect::<Result<_, _>>()?;
            // Preserve first-seen group order for determinism.
            let mut order: Vec<Vec<Cell>> = Vec::new();
            let mut groups: Vec<(Vec<Cell>, Vec<&Vec<Cell>>)> = Vec::new();
            for row in &t.rows {
                let key: Vec<Cell> = key_idx.iter().map(|&i| row[i].clone()).collect();
                match order.iter().position(|k| k == &key) {
                    Some(pos) => groups[pos].1.push(row),
                    None => {
                        order.push(key.clone());
                        groups.push((key, vec![row]));
                    }
                }
            }
            let mut columns = keys.clone();
            columns.extend(aggs.iter().map(|a| a.name.clone()));
            let mut rows = Vec::with_capacity(groups.len());
            for (key, members) in &groups {
                let mut out = key.clone();
                for a in aggs {
                    out.push(aggregate(a, &t.columns, members)?);
                }
                rows.push(out);
            }
            Ok(Table { columns, rows })
        }
    }
}

fn aggregate(a: &Aggregation, columns: &[String], rows: &[&Vec<Cell>]) -> Result<Cell, String> {
    let col_cells = |name: &str| -> Result<Vec<Cell>, String> {
        let i = col_index(columns, name)?;
        Ok(rows.iter().map(|r| r[i].clone()).collect())
    };
    let nums = |name: &str| -> Result<Vec<f64>, String> {
        Ok(col_cells(name)?
            .iter()
            .filter_map(as_f64)
            .collect())
    };
    match (a.func, &a.column) {
        (AggFunc::Count, None) => Ok(Cell::Int(rows.len() as i64)),
        (AggFunc::Count, Some(c)) => {
            let n = col_cells(c)?.iter().filter(|v| **v != Cell::Null).count();
            Ok(Cell::Int(n as i64))
        }
        (AggFunc::CountDistinct, Some(c)) => {
            let mut seen: Vec<Cell> = Vec::new();
            for v in col_cells(c)? {
                if v != Cell::Null && !seen.contains(&v) {
                    seen.push(v);
                }
            }
            Ok(Cell::Int(seen.len() as i64))
        }
        (AggFunc::Sum, Some(c)) => Ok(Cell::Num(nums(c)?.iter().sum())),
        (AggFunc::Average, Some(c)) => {
            let v = nums(c)?;
            if v.is_empty() {
                Ok(Cell::Null)
            } else {
                Ok(Cell::Num(v.iter().sum::<f64>() / v.len() as f64))
            }
        }
        (AggFunc::Min, Some(c)) => Ok(nums(c)?
            .into_iter()
            .fold(None, |acc, x| Some(acc.map_or(x, |a: f64| a.min(x))))
            .map_or(Cell::Null, Cell::Num)),
        (AggFunc::Max, Some(c)) => Ok(nums(c)?
            .into_iter()
            .fold(None, |acc, x| Some(acc.map_or(x, |a: f64| a.max(x))))
            .map_or(Cell::Null, Cell::Num)),
        _ => Err(format!("unsupported aggregate {:?}", a.func)),
    }
}

fn eval_scalar(s: &Scalar, columns: &[String], row: &[Cell], sem: &Semantics) -> Result<Cell, String> {
    match s {
        Scalar::Col(n) => Ok(row[col_index(columns, n)?].clone()),
        // The reference interpreter works over a single flat row with unique
        // column names, so a qualifier is informational — resolve by name.
        Scalar::QualifiedCol { name, .. } => Ok(row[col_index(columns, name)?].clone()),
        Scalar::Lit(lit) => Ok(lit_cell(lit)),
        Scalar::Cmp { op, lhs, rhs } => {
            let a = eval_scalar(lhs, columns, row, sem)?;
            let b = eval_scalar(rhs, columns, row, sem)?;
            Ok(match order_cells_opt(&a, &b, sem) {
                None => Cell::Null, // comparison with NULL is unknown
                Some(ord) => Cell::Bool(match op {
                    CmpOp::Eq => ord.is_eq(),
                    CmpOp::Ne => ord.is_ne(),
                    CmpOp::Lt => ord.is_lt(),
                    CmpOp::Le => ord.is_le(),
                    CmpOp::Gt => ord.is_gt(),
                    CmpOp::Ge => ord.is_ge(),
                }),
            })
        }
        Scalar::Bool { op, args } => {
            let vals: Vec<bool> = args
                .iter()
                .map(|a| Ok(truthy(&eval_scalar(a, columns, row, sem)?)))
                .collect::<Result<_, String>>()?;
            Ok(Cell::Bool(match op {
                BoolOp::And => vals.iter().all(|b| *b),
                BoolOp::Or => vals.iter().any(|b| *b),
                BoolOp::Not => !vals.first().copied().unwrap_or(false),
            }))
        }
        Scalar::Arith { op, lhs, rhs } => {
            let a = eval_scalar(lhs, columns, row, sem)?;
            let b = eval_scalar(rhs, columns, row, sem)?;
            arith(*op, &a, &b, sem)
        }
        Scalar::Call { func, args } => {
            let vals: Vec<Cell> = args
                .iter()
                .map(|a| eval_scalar(a, columns, row, sem))
                .collect::<Result<_, _>>()?;
            call(func, &vals)
        }
        Scalar::Opaque => Err("cannot interpret opaque scalar".to_string()),
    }
}

fn arith(op: ArithOp, a: &Cell, b: &Cell, sem: &Semantics) -> Result<Cell, String> {
    if *a == Cell::Null || *b == Cell::Null {
        return Ok(Cell::Null);
    }
    // Integer division is the only operation that differs by semantics.
    if let (ArithOp::Div, Cell::Int(x), Cell::Int(y)) = (op, a, b) {
        if sem.integer_division {
            return if *y == 0 {
                Ok(Cell::Null)
            } else {
                Ok(Cell::Int(x / y))
            };
        }
    }
    let (x, y) = (
        as_f64(a).ok_or("non-numeric arithmetic")?,
        as_f64(b).ok_or("non-numeric arithmetic")?,
    );
    let r = match op {
        ArithOp::Add => x + y,
        ArithOp::Sub => x - y,
        ArithOp::Mul => x * y,
        ArithOp::Div => x / y,
    };
    // Keep integer-typed results integral so equality with the SQL route is exact.
    if let (Cell::Int(_), Cell::Int(_)) = (a, b) {
        if op != ArithOp::Div && r.fract() == 0.0 {
            return Ok(Cell::Int(r as i64));
        }
    }
    Ok(Cell::Num(r))
}

fn call(func: &str, args: &[Cell]) -> Result<Cell, String> {
    let text = |c: &Cell| match c {
        Cell::Text(s) => Some(s.clone()),
        _ => None,
    };
    match (func, args) {
        ("Text.Upper", [c]) => Ok(text(c).map_or(Cell::Null, |s| Cell::Text(s.to_uppercase()))),
        ("Text.Lower", [c]) => Ok(text(c).map_or(Cell::Null, |s| Cell::Text(s.to_lowercase()))),
        ("Text.Trim", [c]) => Ok(text(c).map_or(Cell::Null, |s| Cell::Text(s.trim().to_string()))),
        ("Number.Abs", [c]) => Ok(as_f64(c).map_or(Cell::Null, |x| Cell::Num(x.abs()))),
        ("Number.Round", [c]) => Ok(as_f64(c).map_or(Cell::Null, |x| Cell::Num(x.round()))),
        _ => Err(format!("unsupported call {func}")),
    }
}

fn lit_cell(lit: &Lit) -> Cell {
    match lit {
        Lit::Null => Cell::Null,
        Lit::Text(s) => Cell::Text(s.clone()),
        Lit::Logical(b) => Cell::Bool(*b),
        Lit::Date(d) => Cell::Date(*d),
        Lit::Datetime(dt) => Cell::Datetime(*dt),
        Lit::Number(s) => {
            if let Ok(i) = s.parse::<i64>() {
                Cell::Int(i)
            } else {
                s.parse::<f64>().map(Cell::Num).unwrap_or(Cell::Null)
            }
        }
    }
}

fn as_f64(c: &Cell) -> Option<f64> {
    match c {
        Cell::Int(i) => Some(*i as f64),
        Cell::Num(n) => Some(*n),
        _ => None,
    }
}

fn truthy(c: &Cell) -> bool {
    matches!(c, Cell::Bool(true))
}

fn col_index(columns: &[String], name: &str) -> Result<usize, String> {
    columns
        .iter()
        .position(|c| c == name)
        .ok_or_else(|| format!("unknown column {name}"))
}

/// Total order over two non-NULL cells under the given semantics. NULLs make
/// the order undefined ([`order_cells_opt`] returns `None`); within a sort,
/// NULL placement is decided by `sem.nulls_first`.
fn order_cells(a: &Cell, b: &Cell, sem: &Semantics) -> Result<std::cmp::Ordering, String> {
    use std::cmp::Ordering;
    match (a, b) {
        (Cell::Null, Cell::Null) => Ok(Ordering::Equal),
        (Cell::Null, _) => Ok(if sem.nulls_first { Ordering::Less } else { Ordering::Greater }),
        (_, Cell::Null) => Ok(if sem.nulls_first { Ordering::Greater } else { Ordering::Less }),
        _ => order_cells_opt(a, b, sem).ok_or_else(|| "incomparable cells".to_string()),
    }
}

fn order_cells_opt(a: &Cell, b: &Cell, sem: &Semantics) -> Option<std::cmp::Ordering> {
    match (a, b) {
        (Cell::Null, _) | (_, Cell::Null) => None,
        (Cell::Text(x), Cell::Text(y)) => {
            if sem.case_insensitive_text {
                Some(x.to_lowercase().cmp(&y.to_lowercase()))
            } else {
                Some(x.cmp(y))
            }
        }
        (Cell::Bool(x), Cell::Bool(y)) => Some(x.cmp(y)),
        (Cell::Date(x), Cell::Date(y)) => Some(x.cmp(y)),
        (Cell::Datetime(x), Cell::Datetime(y)) => Some(x.cmp(y)),
        _ => {
            let (x, y) = (as_f64(a)?, as_f64(b)?);
            x.partial_cmp(&y)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::lower::lower;
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn plan(src: &str) -> Rel {
        let toks = tokenize(src).expect("lex");
        parse(&toks).map(|ast| lower(&ast)).expect("parse")
    }

    fn sample_db() -> Db {
        Db::new().with(
            "t",
            &["Region", "Amount", "name"],
            vec![
                vec![Cell::Text("GB".into()), Cell::Int(10), Cell::Text("abc".into())],
                vec![Cell::Text("GB".into()), Cell::Int(5), Cell::Text("ABC".into())],
                vec![Cell::Text("US".into()), Cell::Int(7), Cell::Text("xyz".into())],
                vec![Cell::Text("US".into()), Cell::Null, Cell::Text("def".into())],
            ],
        )
    }

    /// With matching semantics, the folded and reference routes must agree —
    /// this validates the harness against trusted behaviour.
    fn agrees(src: &str) {
        let m = Semantics::mrsflow();
        differential(&plan(src), &sample_db(), &m, &m)
            .unwrap_or_else(|e| panic!("expected agreement for `{src}`: {e:?}"));
    }

    #[test]
    fn instrument_validates_on_filter() {
        agrees(r#"Table.SelectRows(t, each [Region] = "GB")"#);
    }

    #[test]
    fn instrument_validates_on_projection() {
        agrees(r#"Table.SelectColumns(t, {"Region", "Amount"})"#);
        agrees(r#"Table.AddColumn(t, "more", each [Amount] + 1)"#);
    }

    #[test]
    fn instrument_validates_on_group() {
        agrees(r#"Table.Group(t, {"Region"}, {{"Total", each List.Sum([Amount])}, {"N", each Table.RowCount(_)}})"#);
    }

    #[test]
    fn instrument_validates_on_sort_and_limit() {
        agrees(r#"Table.FirstN(Table.Sort(t, "Amount"), 2)"#);
    }

    #[test]
    fn instrument_validates_on_distinct() {
        agrees(r#"Table.Distinct(Table.SelectColumns(t, {"Region"}))"#);
    }

    #[test]
    fn instrument_validates_on_partial_fold() {
        // Sort-over-limit splits: the limit folds, the sort runs over its rows.
        agrees(r#"Table.Sort(Table.FirstN(t, 3), "Amount")"#);
    }

    // --- divergences the harness should catch -----------------------------

    #[test]
    fn case_insensitive_collation_diverges() {
        // A backend that compares text case-insensitively keeps "ABC" too.
        let folded = Semantics {
            case_insensitive_text: true,
            ..Semantics::mrsflow()
        };
        let r = differential(
            &plan(r#"Table.SelectRows(t, each [name] = "abc")"#),
            &sample_db(),
            &folded,
            &Semantics::mrsflow(),
        );
        assert!(r.is_err(), "case-insensitive collation should diverge");
    }

    #[test]
    fn integer_division_diverges() {
        // 7 / 2 is 3.5 in mrsflow but 3 under truncating integer division.
        let folded = Semantics {
            integer_division: true,
            ..Semantics::mrsflow()
        };
        let r = differential(
            &plan(r#"Table.AddColumn(t, "half", each [Amount] / 2)"#),
            &sample_db(),
            &folded,
            &Semantics::mrsflow(),
        );
        assert!(r.is_err(), "integer division should diverge");
    }

    #[test]
    fn null_ordering_diverges() {
        // The NULL Amount sorts first under mrsflow but last under the backend.
        let folded = Semantics {
            nulls_first: false,
            ..Semantics::mrsflow()
        };
        let r = differential(
            &plan(r#"Table.Sort(t, "Amount")"#),
            &sample_db(),
            &folded,
            &Semantics::mrsflow(),
        );
        assert!(r.is_err(), "NULL ordering should diverge");
    }
}
