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
    /// `NULL = NULL` is TRUE — two-valued logic. DBISAM does this (see the
    /// `dbisam-null-semantics` reference); standard SQL / mrsflow use
    /// three-valued logic where it is unknown. Affects equi-join key matching
    /// and `= NULL` comparisons over nullable columns.
    pub null_equals_null: bool,
}

impl Semantics {
    /// mrsflow's own semantics: case-sensitive, NULLs first, true division,
    /// three-valued null logic.
    pub fn mrsflow() -> Self {
        Semantics {
            case_insensitive_text: false,
            nulls_first: true,
            integer_division: false,
            null_equals_null: false,
        }
    }

    /// DBISAM's documented semantics. Only `null_equals_null` is known to
    /// diverge (two-valued logic, per `dbisam-null-semantics`); the rest match
    /// mrsflow until proven otherwise, so the harness flags only the divergence
    /// we can substantiate rather than inventing exclusions.
    pub fn dbisam() -> Self {
        Semantics {
            null_equals_null: true,
            ..Semantics::mrsflow()
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
        Rel::Join { kind, left_keys, right_keys, left, right } => {
            if !matches!(kind, JoinKind::Inner | JoinKind::LeftOuter) {
                return Err(format!("join kind {kind:?} not interpreted by the harness"));
            }
            let lt = interpret(left, db, folded, sem)?;
            let rt = interpret(right, db, folded, sem)?;
            let ltab = base_table(left).ok_or_else(|| "join: left has no base table".to_string())?;
            let rtab = base_table(right).ok_or_else(|| "join: right has no base table".to_string())?;
            // Qualify output columns `table.col` so an Aggregate/Project above
            // can disambiguate a name present on both sides.
            let mut columns: Vec<String> = lt.columns.iter().map(|c| format!("{ltab}.{c}")).collect();
            columns.extend(rt.columns.iter().map(|c| format!("{rtab}.{c}")));
            let lk: Vec<usize> = left_keys.iter().map(|k| col_index(&lt.columns, k)).collect::<Result<_, _>>()?;
            let rk: Vec<usize> = right_keys.iter().map(|k| col_index(&rt.columns, k)).collect::<Result<_, _>>()?;
            let null_right = vec![Cell::Null; rt.columns.len()];
            let mut rows: Vec<Vec<Cell>> = Vec::new();
            for lrow in &lt.rows {
                let mut matched = false;
                for rrow in &rt.rows {
                    // Equi-join. Standard SQL drops NULL keys; DBISAM's
                    // two-valued logic matches NULL = NULL (see `Semantics`).
                    let eq = lk.iter().zip(&rk).all(|(&li, &ri)| {
                        match (&lrow[li], &rrow[ri]) {
                            (Cell::Null, Cell::Null) => sem.null_equals_null,
                            (Cell::Null, _) | (_, Cell::Null) => false,
                            (a, b) => order_cells(a, b, sem).map_or(false, |o| o.is_eq()),
                        }
                    });
                    if eq {
                        matched = true;
                        let mut out = lrow.clone();
                        out.extend(rrow.iter().cloned());
                        rows.push(out);
                    }
                }
                if !matched && matches!(kind, JoinKind::LeftOuter) {
                    let mut out = lrow.clone();
                    out.extend(null_right.iter().cloned());
                    rows.push(out);
                }
            }
            Ok(Table { columns, rows })
        }

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
                .map(|c| col_index(&t.columns, &scalar_col_name(c)?))
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
            let mut columns: Vec<String> = keys
                .iter()
                .map(scalar_col_name)
                .collect::<Result<_, _>>()?;
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

/// The base table name at the bottom of a (filtered/projected) scan subtree,
/// used to qualify a join side's columns. `None` if the leaf isn't a `Scan`.
fn base_table(rel: &Rel) -> Option<String> {
    match rel {
        Rel::Scan(Source::Ref(n)) => Some(n.clone()),
        Rel::Filter { input, .. }
        | Rel::Project { input, .. }
        | Rel::Sort { input, .. }
        | Rel::Limit { input, .. }
        | Rel::Distinct { input, .. } => base_table(input),
        _ => None,
    }
}

/// The column name a group key or aggregate ranges over (`table.col` for a
/// qualified reference over a join, bare otherwise).
fn scalar_col_name(s: &Scalar) -> Result<String, String> {
    match s {
        Scalar::Col(n) => Ok(n.clone()),
        // Over a join the interpreter names columns `table.col`, so a qualified
        // reference resolves to that form; a bare `Col` stays single-table.
        Scalar::QualifiedCol { table, name } => Ok(format!("{table}.{name}")),
        other => Err(format!("expected a column reference, got {other:?}")),
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
    let col_name = a.column.as_ref().map(scalar_col_name).transpose()?;
    match (a.func, col_name.as_deref()) {
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

/// SQL `LIKE`: `%` matches any run (incl. empty), `_` any single char. Honours
/// `case_insensitive_text`. (DBISAM treats `*` as a literal, not a wildcard,
/// which this respects — only `%`/`_` are special.)
fn like_match(text: &str, pattern: &str, sem: &Semantics) -> bool {
    fn rec(t: &[char], p: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('%') => rec(t, &p[1..]) || (!t.is_empty() && rec(&t[1..], p)),
            Some('_') => !t.is_empty() && rec(&t[1..], &p[1..]),
            Some(&c) => !t.is_empty() && t[0] == c && rec(&t[1..], &p[1..]),
        }
    }
    let prep = |s: &str| -> Vec<char> {
        if sem.case_insensitive_text {
            s.to_lowercase().chars().collect()
        } else {
            s.chars().collect()
        }
    };
    rec(&prep(text), &prep(pattern))
}

fn eval_scalar(s: &Scalar, columns: &[String], row: &[Cell], sem: &Semantics) -> Result<Cell, String> {
    match s {
        Scalar::Col(n) => Ok(row[col_index(columns, n)?].clone()),
        // Over a join, columns are named `table.col`; resolve to that form.
        Scalar::QualifiedCol { table, name } => {
            Ok(row[col_index(columns, &format!("{table}.{name}"))?].clone())
        }
        Scalar::Lit(lit) => Ok(lit_cell(lit)),
        Scalar::Cmp { op, lhs, rhs } => {
            let a = eval_scalar(lhs, columns, row, sem)?;
            let b = eval_scalar(rhs, columns, row, sem)?;
            if let CmpOp::Like = op {
                // LIKE is a text pattern match, not an ordering.
                return Ok(match (&a, &b) {
                    (Cell::Null, _) | (_, Cell::Null) => Cell::Null,
                    (Cell::Text(s), Cell::Text(pat)) => Cell::Bool(like_match(s, pat, sem)),
                    _ => Cell::Bool(false),
                });
            }
            Ok(match order_cells_opt(&a, &b, sem) {
                None => Cell::Null, // comparison with NULL is unknown
                Some(ord) => Cell::Bool(match op {
                    CmpOp::Eq => ord.is_eq(),
                    CmpOp::Ne => ord.is_ne(),
                    CmpOp::Lt => ord.is_lt(),
                    CmpOp::Le => ord.is_le(),
                    CmpOp::Gt => ord.is_gt(),
                    CmpOp::Ge => ord.is_ge(),
                    CmpOp::Like => unreachable!("handled above"),
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

    #[test]
    fn instrument_validates_on_join_group() {
        // Gate 2 for the headline fold: SUM(orderi.quantity) grouped by orderh
        // keys over orderh LEFT JOIN orderi — the qualified Aggregate(Join)
        // shape the connector builds (not producible via `lower`). Order 3 has
        // no lines, so it exercises the unmatched-left → SUM-over-NULL case.
        let qcol = |t: &str, n: &str| Scalar::QualifiedCol { table: t.into(), name: n.into() };
        let db = Db::new()
            .with(
                "orderh",
                &["ref", "custcode"],
                vec![
                    vec![Cell::Int(1), Cell::Text("C1".into())],
                    vec![Cell::Int(2), Cell::Text("C2".into())],
                    vec![Cell::Int(3), Cell::Text("C3".into())], // no orderi lines
                ],
            )
            .with(
                "orderi",
                &["ref", "quantity"],
                vec![
                    vec![Cell::Int(1), Cell::Int(10)],
                    vec![Cell::Int(1), Cell::Int(5)],
                    vec![Cell::Int(2), Cell::Int(7)],
                ],
            );
        let plan = Rel::Aggregate {
            keys: vec![qcol("orderh", "ref"), qcol("orderh", "custcode")],
            aggs: vec![Aggregation {
                name: "qty".into(),
                func: AggFunc::Sum,
                column: Some(qcol("orderi", "quantity")),
            }],
            input: Box::new(Rel::Join {
                kind: JoinKind::LeftOuter,
                left_keys: vec!["ref".into()],
                right_keys: vec!["ref".into()],
                left: Box::new(Rel::Scan(Source::Ref("orderh".into()))),
                right: Box::new(Rel::Scan(Source::Ref("orderi".into()))),
            }),
        };
        // Sanity: the reference computes the groups we expect (15, 7, 0).
        let r = interpret(&plan, &db, None, &Semantics::mrsflow()).expect("interpret");
        assert_eq!(r.rows.len(), 3, "one group per order");
        assert_eq!(r.rows[2][2], Cell::Num(0.0), "unmatched order sums to 0");
        // And the fold route agrees under matching semantics.
        let m = Semantics::mrsflow();
        differential(&plan, &db, &m, &m).expect("join+group fold agrees");
    }

    #[test]
    fn join_key_collation_diverges() {
        // The harness is a real instrument for joins, not a no-op: a backend
        // that matches join keys case-insensitively ("K" = "k") produces a row
        // mrsflow's case-sensitive join drops, and differential() flags it.
        let qcol = |t: &str, n: &str| Scalar::QualifiedCol { table: t.into(), name: n.into() };
        let db = Db::new()
            .with("a", &["k", "x"], vec![vec![Cell::Text("K".into()), Cell::Int(1)]])
            .with("b", &["k", "y"], vec![vec![Cell::Text("k".into()), Cell::Int(2)]]);
        let plan = Rel::Project {
            star: false,
            items: vec![
                ProjectItem { name: "x".into(), expr: qcol("a", "x") },
                ProjectItem { name: "y".into(), expr: qcol("b", "y") },
            ],
            input: Box::new(Rel::Join {
                kind: JoinKind::Inner,
                left_keys: vec!["k".into()],
                right_keys: vec!["k".into()],
                left: Box::new(Rel::Scan(Source::Ref("a".into()))),
                right: Box::new(Rel::Scan(Source::Ref("b".into()))),
            }),
        };
        let dbisam = Semantics { case_insensitive_text: true, ..Semantics::mrsflow() };
        let m = Semantics::mrsflow();
        assert!(
            differential(&plan, &db, &dbisam, &m).is_err(),
            "case-folded join key should diverge"
        );
    }

    #[test]
    fn null_join_key_diverges_but_non_null_is_safe() {
        // DBISAM's two-valued logic (`dbisam-null-semantics`) matches NULL =
        // NULL on a join key; mrsflow's three-valued logic drops it. Folding a
        // join over a key that *is* NULL on both sides changes the answer, and
        // the harness flags it under `Semantics::dbisam()`.
        let qcol = |t: &str, n: &str| Scalar::QualifiedCol { table: t.into(), name: n.into() };
        let plan = Rel::Project {
            star: false,
            items: vec![
                ProjectItem { name: "x".into(), expr: qcol("a", "x") },
                ProjectItem { name: "y".into(), expr: qcol("b", "y") },
            ],
            input: Box::new(Rel::Join {
                kind: JoinKind::Inner,
                left_keys: vec!["k".into()],
                right_keys: vec!["k".into()],
                left: Box::new(Rel::Scan(Source::Ref("a".into()))),
                right: Box::new(Rel::Scan(Source::Ref("b".into()))),
            }),
        };
        let null_keys = Db::new()
            .with("a", &["k", "x"], vec![vec![Cell::Null, Cell::Int(1)]])
            .with("b", &["k", "y"], vec![vec![Cell::Null, Cell::Int(2)]]);
        assert!(
            differential(&plan, &null_keys, &Semantics::dbisam(), &Semantics::mrsflow()).is_err(),
            "NULL = NULL join-key divergence should be flagged"
        );
        // Same plan, no NULL key values — the routes agree, the fold is safe.
        let real_keys = Db::new()
            .with("a", &["k", "x"], vec![vec![Cell::Int(7), Cell::Int(1)]])
            .with("b", &["k", "y"], vec![vec![Cell::Int(7), Cell::Int(2)]]);
        differential(&plan, &real_keys, &Semantics::dbisam(), &Semantics::mrsflow())
            .expect("non-null keys: fold is safe");
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
