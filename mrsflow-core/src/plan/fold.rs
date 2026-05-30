//! Connector SQL emission and the fold split — Gate 1 of the fold pass, per
//! `mrsflow/10-plan-ir.md` §"The fold planner — the grammar *is* the decision
//! procedure".
//!
//! The emitter's success **is** the syntactic fold predicate: if [`emit`]
//! renders valid SQL for a subtree, it folds; if it returns [`Unfoldable`], it
//! does not. There is no separate hand-written capability table to drift out of
//! sync — the dialect ceiling and the codegen are the same operation.
//!
//! [`fold`] performs the execution split: it walks a plan from the top, finds
//! the maximal subtree the dialect can emit, and returns that subtree's SQL
//! plus a *residual* plan to run in the evaluator over the rows the SQL
//! returns. The folded subtree appears in the residual as the [`FOLDED`]
//! sentinel leaf.
//!
//! Scope of the v1 DBISAM dialect: a single flat `SELECT` over one base table —
//! `WHERE`, `GROUP BY`, `ORDER BY`, `TOP n`, `SELECT DISTINCT`, column
//! projection, and a small allow-list of scalar functions. No subqueries, no
//! `OFFSET` (DBISAM has none), no `HAVING`, no joins yet; each of those is a
//! fold boundary, not a wrong answer. This is intentionally only Gate 1 —
//! semantic equivalence (Gate 2, the differential harness) is a later increment
//! and must pass before any of this is enabled against a live source.

use super::ir::*;

/// Marker descriptor on the `EvalM` sentinel that stands in for a folded
/// subtree's rows inside a residual plan.
pub const FOLDED: &str = "$folded";

/// Why a subtree could not be emitted as connector SQL. Carries a short reason
/// for debugging and dump-and-diff; the planner only cares that it is an error.
#[derive(Debug, Clone, PartialEq)]
pub struct Unfoldable(pub String);

fn unfoldable(reason: impl Into<String>) -> Unfoldable {
    Unfoldable(reason.into())
}

/// The rendered clauses of a single `SELECT`, handed to [`Dialect::render_select`]
/// for final assembly (so dialects can differ on e.g. `TOP n` vs `LIMIT n`).
#[derive(Debug, Clone, Default)]
pub struct SelectParts {
    pub distinct: bool,
    pub top: Option<u64>,
    /// Rendered SELECT-list items; `["*"]` when no projection was applied.
    pub projection: Vec<String>,
    /// Rendered FROM target (already quoted).
    pub from: String,
    /// Rendered, AND-combined WHERE fragments.
    pub where_: Vec<String>,
    /// Rendered (quoted) GROUP BY columns.
    pub group_by: Vec<String>,
    /// Rendered ORDER BY items (e.g. `"col" ASC`).
    pub order_by: Vec<String>,
}

/// A SQL dialect: the parts of emission that vary per backend. v1 ships
/// [`Dbisam`]; the trait is the seam where PostgreSQL etc. slot in later.
pub trait Dialect {
    fn quote_ident(&self, name: &str) -> String;
    fn text_literal(&self, s: &str) -> String;
    fn bool_literal(&self, b: bool) -> String;
    fn date_literal(&self, d: &chrono::NaiveDate) -> String;
    fn datetime_literal(&self, dt: &chrono::NaiveDateTime) -> String;
    /// Whether the dialect can express a row offset. DBISAM cannot.
    fn supports_offset(&self) -> bool;
    /// SQL for a scalar function call, or `None` if the dialect has no proven
    /// analogue (which makes the enclosing expression unfoldable).
    fn scalar_call(&self, func: &str, args: &[String]) -> Option<String>;
    /// Assemble the final `SELECT` text from its clauses.
    fn render_select(&self, parts: &SelectParts) -> String;
}

/// The DBISAM dialect: double-quoted identifiers, `TOP n`, no `OFFSET`, and a
/// deliberately small scalar-function allow-list.
pub struct Dbisam;

impl Dialect for Dbisam {
    fn quote_ident(&self, name: &str) -> String {
        format!("\"{}\"", name.replace('"', "\"\""))
    }

    fn text_literal(&self, s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    fn bool_literal(&self, b: bool) -> String {
        if b { "TRUE" } else { "FALSE" }.to_string()
    }

    fn date_literal(&self, d: &chrono::NaiveDate) -> String {
        // DBISAM date-literal syntax: #YYYY-MM-DD#.
        format!("#{}#", d.format("%Y-%m-%d"))
    }

    fn datetime_literal(&self, dt: &chrono::NaiveDateTime) -> String {
        format!("#{}#", dt.format("%Y-%m-%d %H:%M:%S"))
    }

    fn supports_offset(&self) -> bool {
        false
    }

    fn scalar_call(&self, func: &str, args: &[String]) -> Option<String> {
        match (func, args.len()) {
            ("Text.Upper", 1) => Some(format!("UPPER({})", args[0])),
            ("Text.Lower", 1) => Some(format!("LOWER({})", args[0])),
            ("Text.Trim", 1) => Some(format!("TRIM({})", args[0])),
            ("Number.Abs", 1) => Some(format!("ABS({})", args[0])),
            ("Number.Round", 1) => Some(format!("ROUND({})", args[0])),
            ("Number.Round", 2) => Some(format!("ROUND({}, {})", args[0], args[1])),
            // Everything else is off the proven allow-list → not foldable.
            _ => None,
        }
    }

    fn render_select(&self, p: &SelectParts) -> String {
        let mut s = String::from("SELECT ");
        if p.distinct {
            s.push_str("DISTINCT ");
        }
        s.push_str(&p.projection.join(", "));
        s.push_str(" FROM ");
        s.push_str(&p.from);
        if !p.where_.is_empty() {
            s.push_str(" WHERE ");
            s.push_str(&p.where_.join(" AND "));
        }
        if !p.group_by.is_empty() {
            s.push_str(" GROUP BY ");
            s.push_str(&p.group_by.join(", "));
        }
        if !p.order_by.is_empty() {
            s.push_str(" ORDER BY ");
            s.push_str(&p.order_by.join(", "));
        }
        // DBISAM's `TOP n` is a trailing clause, like `LIMIT` — it goes at the
        // very end, after ORDER BY, NOT after SELECT.
        if let Some(n) = p.top {
            s.push_str(&format!(" TOP {n}"));
        }
        s
    }
}

/// A portable-ish generic-ODBC dialect: plain double-quoted identifiers,
/// `0`/`1` booleans, ANSI `DATE '…'` / `TIMESTAMP '…'` literals, and no `TOP`
/// (drivers disagree on row-limit syntax). This reproduces the long-standing
/// `LazyOdbc` SQL so routing that path through the emitter changes nothing
/// observable — it just unifies the two SQL generators.
pub struct GenericOdbc;

impl Dialect for GenericOdbc {
    fn quote_ident(&self, name: &str) -> String {
        format!("\"{name}\"")
    }

    fn text_literal(&self, s: &str) -> String {
        format!("'{}'", s.replace('\'', "''"))
    }

    fn bool_literal(&self, b: bool) -> String {
        if b { "1" } else { "0" }.to_string()
    }

    fn date_literal(&self, d: &chrono::NaiveDate) -> String {
        format!("DATE '{d}'")
    }

    fn datetime_literal(&self, dt: &chrono::NaiveDateTime) -> String {
        format!("TIMESTAMP '{dt}'")
    }

    fn supports_offset(&self) -> bool {
        false
    }

    fn scalar_call(&self, func: &str, args: &[String]) -> Option<String> {
        // Same proven allow-list as DBISAM; the accumulator never emits calls.
        Dbisam.scalar_call(func, args)
    }

    fn render_select(&self, p: &SelectParts) -> String {
        // Like DBISAM but without `TOP` (the legacy renderer never emitted a
        // row limit for this path).
        let mut s = String::from("SELECT ");
        if p.distinct {
            s.push_str("DISTINCT ");
        }
        s.push_str(&p.projection.join(", "));
        s.push_str(" FROM ");
        s.push_str(&p.from);
        if !p.where_.is_empty() {
            s.push_str(" WHERE ");
            s.push_str(&p.where_.join(" AND "));
        }
        if !p.group_by.is_empty() {
            s.push_str(" GROUP BY ");
            s.push_str(&p.group_by.join(", "));
        }
        if !p.order_by.is_empty() {
            s.push_str(" ORDER BY ");
            s.push_str(&p.order_by.join(", "));
        }
        s
    }
}

/// Emit a full single-`SELECT` for `rel`, or [`Unfoldable`] at the first
/// construct the dialect cannot express. Success is the syntactic fold predicate.
pub fn emit(rel: &Rel, d: &dyn Dialect) -> Result<String, Unfoldable> {
    let sel = build(rel, d)?;
    Ok(d.render_select(&sel.into_parts()))
}

/// A connector's choice of SQL dialect, stored on a deferred plan so
/// `render_sql` emits the right flavour per transport. The dialect trait
/// objects are zero-sized, so this enum is just a tag that picks one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlDialect {
    /// Portable ANSI-ish SQL — the long-standing `Odbc.DataSource` output.
    GenericOdbc,
    /// DBISAM dialect (double-quoted identifiers, `TOP n`, `#…#` dates) —
    /// the native `Exportmaster` connector.
    Dbisam,
}

impl SqlDialect {
    /// Emit `rel` under this dialect. See [`emit`].
    pub fn emit(self, rel: &Rel) -> Result<String, Unfoldable> {
        match self {
            SqlDialect::GenericOdbc => emit(rel, &GenericOdbc),
            SqlDialect::Dbisam => emit(rel, &Dbisam),
        }
    }
}

/// Split `rel` into the maximal foldable subtree and a residual plan.
///
/// Walks from the top: as soon as a node (with everything below it) emits, that
/// is the fold and the residual is the [`FOLDED`] sentinel. Otherwise the node
/// is peeled off as residual and its single relational input is folded. The
/// result's `sql` is the pushed query (if any) and `residual` is what the
/// evaluator runs over the returned rows.
pub fn fold(rel: &Rel, d: &dyn Dialect) -> FoldResult {
    if let Ok(sql) = emit(rel, d) {
        return FoldResult {
            sql: Some(sql),
            residual: folded_sentinel(),
            folded: Some(rel.clone()),
        };
    }
    // The whole subtree does not fold; peel this node and fold what is below.
    let peel = |rebuild: &dyn Fn(Rel) -> Rel, input: &Rel| {
        let sub = fold(input, d);
        FoldResult {
            sql: sub.sql,
            residual: rebuild(sub.residual),
            folded: sub.folded,
        }
    };
    match rel {
        Rel::Filter { predicate, input } => peel(
            &|inp| Rel::Filter {
                predicate: predicate.clone(),
                input: Box::new(inp),
            },
            input,
        ),
        Rel::Project { star, items, input } => peel(
            &|inp| Rel::Project {
                star: *star,
                items: items.clone(),
                input: Box::new(inp),
            },
            input,
        ),
        Rel::Sort { keys, input } => peel(
            &|inp| Rel::Sort {
                keys: keys.clone(),
                input: Box::new(inp),
            },
            input,
        ),
        Rel::Limit { n, offset, input } => peel(
            &|inp| Rel::Limit {
                n: *n,
                offset: *offset,
                input: Box::new(inp),
            },
            input,
        ),
        Rel::Aggregate { keys, aggs, input } => peel(
            &|inp| Rel::Aggregate {
                keys: keys.clone(),
                aggs: aggs.clone(),
                input: Box::new(inp),
            },
            input,
        ),
        // A single-input opaque step usually sits *above* a foldable spine —
        // fold the spine and keep the opaque step as residual.
        Rel::EvalM { descr, inputs } if inputs.len() == 1 => peel(
            &|inp| Rel::EvalM {
                descr: descr.clone(),
                inputs: vec![inp],
            },
            &inputs[0],
        ),
        // Scan that didn't emit, a Join, or a multi-input/zero-input EvalM:
        // nothing below this folds in the single-query v1 model.
        other => FoldResult {
            sql: None,
            residual: other.clone(),
            folded: None,
        },
    }
}

/// The result of [`fold`].
#[derive(Debug, Clone, PartialEq)]
pub struct FoldResult {
    /// SQL pushed to the connector, or `None` if nothing folded.
    pub sql: Option<String>,
    /// The plan to evaluate in-memory over the pushed rows. Equal to the
    /// [`FOLDED`] sentinel exactly when the whole plan folded.
    pub residual: Rel,
    /// The logical subtree that became `sql` — what the connector executes.
    /// `None` when nothing folded. Used by the differential harness to run the
    /// folded route, and available to wiring for the result schema.
    pub folded: Option<Rel>,
}

impl FoldResult {
    /// Did the entire plan fold to SQL?
    pub fn is_full(&self) -> bool {
        self.sql.is_some()
            && matches!(&self.residual, Rel::EvalM { descr, inputs } if descr == FOLDED && inputs.is_empty())
    }
}

fn folded_sentinel() -> Rel {
    Rel::EvalM {
        descr: FOLDED.to_string(),
        inputs: Vec::new(),
    }
}

// --- single-SELECT builder ------------------------------------------------

/// In-progress `SELECT`, accumulated bottom-up. The two extra flags beyond
/// [`SelectParts`] track clause-ordering state.
struct Sel {
    distinct: bool,
    top: Option<u64>,
    projection: Vec<String>,
    projection_set: bool,
    from: String,
    where_: Vec<String>,
    group_by: Vec<String>,
    has_aggregate: bool,
    order_by: Vec<String>,
}

impl Sel {
    fn into_parts(self) -> SelectParts {
        SelectParts {
            distinct: self.distinct,
            top: self.top,
            projection: self.projection,
            from: self.from,
            where_: self.where_,
            group_by: self.group_by,
            order_by: self.order_by,
        }
    }
}

fn build(rel: &Rel, d: &dyn Dialect) -> Result<Sel, Unfoldable> {
    match rel {
        Rel::Scan(Source::Ref(table)) => Ok(Sel {
            distinct: false,
            top: None,
            projection: vec!["*".to_string()],
            projection_set: false,
            from: d.quote_ident(table),
            where_: Vec::new(),
            group_by: Vec::new(),
            has_aggregate: false,
            order_by: Vec::new(),
        }),
        Rel::Scan(Source::Document { .. }) => {
            Err(unfoldable("document/connector leaf is not a SQL table"))
        }

        Rel::Filter { predicate, input } => {
            let mut sel = build(input, d)?;
            // WHERE must precede grouping/projection/ordering/limiting and is
            // unsound after DISTINCT.
            if sel.has_aggregate
                || sel.projection_set
                || !sel.group_by.is_empty()
                || !sel.order_by.is_empty()
                || sel.top.is_some()
                || sel.distinct
            {
                return Err(unfoldable("filter above a closed SELECT (would need HAVING/subquery)"));
            }
            sel.where_.push(emit_scalar(predicate, d)?);
            Ok(sel)
        }

        Rel::Project { star, items, input } => {
            let mut sel = build(input, d)?;
            if sel.has_aggregate
                || !sel.group_by.is_empty()
                || !sel.order_by.is_empty()
                || sel.top.is_some()
                || sel.distinct
            {
                return Err(unfoldable("projection above a closed SELECT"));
            }
            let rendered = render_projection(items, d)?;
            if *star {
                // AddColumn: append computed columns to whatever is selected.
                if !sel.projection_set {
                    sel.projection = vec!["*".to_string()];
                }
                sel.projection.extend(rendered);
            } else {
                // SelectColumns / rename: replaces the select list, once.
                if sel.projection_set {
                    return Err(unfoldable("stacked replacing projections"));
                }
                sel.projection = rendered;
            }
            sel.projection_set = true;
            Ok(sel)
        }

        Rel::Aggregate { keys, aggs, input } => {
            let mut sel = build(input, d)?;
            if sel.has_aggregate
                || sel.projection_set
                || !sel.group_by.is_empty()
                || !sel.order_by.is_empty()
                || sel.top.is_some()
                || sel.distinct
            {
                return Err(unfoldable("aggregate above a closed SELECT"));
            }
            let mut projection = Vec::with_capacity(keys.len() + aggs.len());
            for k in keys {
                projection.push(emit_scalar(k, d)?);
            }
            for a in aggs {
                projection.push(render_aggregate(a, d)?);
            }
            sel.group_by = keys
                .iter()
                .map(|k| emit_scalar(k, d))
                .collect::<Result<_, _>>()?;
            sel.projection = projection;
            sel.projection_set = true;
            sel.has_aggregate = true;
            Ok(sel)
        }

        Rel::Sort { keys, input } => {
            let mut sel = build(input, d)?;
            // ORDER BY once, and before TOP (TOP applies after ordering).
            if !sel.order_by.is_empty() || sel.top.is_some() {
                return Err(unfoldable("sort above an existing sort/limit"));
            }
            sel.order_by = keys
                .iter()
                .map(|k| {
                    format!(
                        "{} {}",
                        d.quote_ident(&k.column),
                        if k.descending { "DESC" } else { "ASC" }
                    )
                })
                .collect();
            Ok(sel)
        }

        Rel::Limit { n, offset, input } => {
            let mut sel = build(input, d)?;
            if *offset > 0 && !d.supports_offset() {
                return Err(unfoldable("row offset not supported by dialect"));
            }
            match n {
                None => Ok(sel), // open upper bound + zero offset → no-op
                Some(k) => {
                    if sel.top.is_some() {
                        return Err(unfoldable("stacked limits"));
                    }
                    sel.top = Some(*k);
                    Ok(sel)
                }
            }
        }

        Rel::Distinct { on, input } => {
            if !on.is_empty() {
                return Err(unfoldable("keyed DISTINCT (no DISTINCT ON in dialect)"));
            }
            let mut sel = build(input, d)?;
            if sel.has_aggregate
                || sel.projection_set
                || !sel.group_by.is_empty()
                || !sel.order_by.is_empty()
                || sel.top.is_some()
                || sel.distinct
            {
                return Err(unfoldable("distinct above a closed SELECT"));
            }
            sel.distinct = true;
            Ok(sel)
        }

        Rel::Join {
            kind,
            left_keys,
            right_keys,
            left,
            right,
        } => {
            let l = build(left, d)?;
            let r = build(right, d)?;
            // v1: each side must be a plain (optionally filtered) table source.
            // A projection/aggregate/sort/limit/distinct on either side would
            // need a derived-table subquery, which this flat-SELECT model can't
            // express — refuse rather than emit wrong SQL.
            let plain = |s: &Sel| {
                !s.projection_set
                    && !s.has_aggregate
                    && s.group_by.is_empty()
                    && s.order_by.is_empty()
                    && s.top.is_none()
                    && !s.distinct
            };
            if !plain(&l) || !plain(&r) {
                return Err(unfoldable("join over a non-trivial subquery"));
            }
            if left_keys.is_empty() || left_keys.len() != right_keys.len() {
                return Err(unfoldable("join needs equal-length, non-empty key lists"));
            }
            let kw = match kind {
                JoinKind::Inner => "JOIN",
                JoinKind::LeftOuter => "LEFT JOIN",
                // RIGHT/FULL OUTER and the anti-joins aren't in the v1 dialect
                // surface; leave them to the in-memory evaluator.
                _ => return Err(unfoldable("join kind not foldable in v1")),
            };
            // Keys are qualified by each side's table so the ON clause is
            // unambiguous (the key column exists on both sides).
            let on = left_keys
                .iter()
                .zip(right_keys)
                .map(|(lk, rk)| {
                    format!(
                        "{}.{} = {}.{}",
                        l.from,
                        d.quote_ident(lk),
                        r.from,
                        d.quote_ident(rk)
                    )
                })
                .collect::<Vec<_>>()
                .join(" AND ");
            let mut where_ = l.where_;
            where_.extend(r.where_);
            Ok(Sel {
                distinct: false,
                top: None,
                projection: vec!["*".to_string()],
                projection_set: false,
                from: format!("{} {} {} ON {}", l.from, kw, r.from, on),
                where_,
                group_by: Vec::new(),
                has_aggregate: false,
                order_by: Vec::new(),
            })
        }
        Rel::EvalM { descr, .. } => Err(unfoldable(format!("opaque step: {descr}"))),
    }
}

fn render_projection(items: &[ProjectItem], d: &dyn Dialect) -> Result<Vec<String>, Unfoldable> {
    items
        .iter()
        .map(|it| {
            if it.expr == Scalar::Col(it.name.clone()) {
                Ok(d.quote_ident(&it.name))
            } else {
                Ok(format!("{} AS {}", emit_scalar(&it.expr, d)?, d.quote_ident(&it.name)))
            }
        })
        .collect()
}

fn render_aggregate(a: &Aggregation, d: &dyn Dialect) -> Result<String, Unfoldable> {
    let col = |c: &Option<Scalar>| -> Result<String, Unfoldable> {
        match c {
            Some(s) => emit_scalar(s, d),
            None => Err(unfoldable("aggregate requires a column")),
        }
    };
    let body = match a.func {
        AggFunc::Sum => format!("SUM({})", col(&a.column)?),
        AggFunc::Average => format!("AVG({})", col(&a.column)?),
        AggFunc::Min => format!("MIN({})", col(&a.column)?),
        AggFunc::Max => format!("MAX({})", col(&a.column)?),
        AggFunc::CountDistinct => format!("COUNT(DISTINCT {})", col(&a.column)?),
        AggFunc::Count => match &a.column {
            Some(s) => format!("COUNT({})", emit_scalar(s, d)?),
            None => "COUNT(*)".to_string(),
        },
        AggFunc::Opaque => return Err(unfoldable("opaque aggregate")),
    };
    Ok(format!("{} AS {}", body, d.quote_ident(&a.name)))
}

fn emit_scalar(s: &Scalar, d: &dyn Dialect) -> Result<String, Unfoldable> {
    match s {
        Scalar::Col(n) => Ok(d.quote_ident(n)),
        Scalar::QualifiedCol { table, name } => {
            Ok(format!("{}.{}", d.quote_ident(table), d.quote_ident(name)))
        }
        Scalar::Lit(lit) => emit_lit(lit, d),
        Scalar::Cmp { op, lhs, rhs } => emit_cmp(*op, lhs, rhs, d),
        Scalar::Bool { op, args } => emit_bool(*op, args, d),
        Scalar::Arith { op, lhs, rhs } => {
            let l = emit_scalar(lhs, d)?;
            let r = emit_scalar(rhs, d)?;
            let sym = match op {
                ArithOp::Add => "+",
                ArithOp::Sub => "-",
                ArithOp::Mul => "*",
                ArithOp::Div => "/",
            };
            Ok(format!("({l} {sym} {r})"))
        }
        Scalar::Call { func, args } => {
            let rendered: Result<Vec<String>, _> =
                args.iter().map(|a| emit_scalar(a, d)).collect();
            d.scalar_call(func, &rendered?)
                .ok_or_else(|| unfoldable(format!("no dialect analogue for {func}")))
        }
        Scalar::Opaque => Err(unfoldable("opaque scalar")),
    }
}

fn emit_lit(lit: &Lit, d: &dyn Dialect) -> Result<String, Unfoldable> {
    match lit {
        // Render the verbatim lexeme, but only for plain decimal forms — hex
        // and the like have no portable SQL spelling.
        Lit::Number(s) if s.parse::<f64>().is_ok() => Ok(s.clone()),
        Lit::Number(s) => Err(unfoldable(format!("non-decimal numeric literal {s}"))),
        Lit::Text(s) => Ok(d.text_literal(s)),
        Lit::Logical(b) => Ok(d.bool_literal(*b)),
        Lit::Date(dt) => Ok(d.date_literal(dt)),
        Lit::Datetime(dt) => Ok(d.datetime_literal(dt)),
        Lit::Null => Ok("NULL".to_string()),
    }
}

fn emit_cmp(op: CmpOp, lhs: &Scalar, rhs: &Scalar, d: &dyn Dialect) -> Result<String, Unfoldable> {
    let is_null = |s: &Scalar| matches!(s, Scalar::Lit(Lit::Null));
    if is_null(lhs) || is_null(rhs) {
        let other = if is_null(lhs) { rhs } else { lhs };
        let o = emit_scalar(other, d)?;
        return match op {
            CmpOp::Eq => Ok(format!("{o} IS NULL")),
            CmpOp::Ne => Ok(format!("{o} IS NOT NULL")),
            _ => Err(unfoldable("ordered comparison against null")),
        };
    }
    let l = emit_scalar(lhs, d)?;
    let r = emit_scalar(rhs, d)?;
    let sym = match op {
        CmpOp::Eq => "=",
        CmpOp::Ne => "<>",
        CmpOp::Lt => "<",
        CmpOp::Le => "<=",
        CmpOp::Gt => ">",
        CmpOp::Ge => ">=",
    };
    Ok(format!("{l} {sym} {r}"))
}

fn emit_bool(op: BoolOp, args: &[Scalar], d: &dyn Dialect) -> Result<String, Unfoldable> {
    let parts: Result<Vec<String>, _> = args.iter().map(|a| emit_scalar(a, d)).collect();
    let parts = parts?;
    match op {
        BoolOp::Not => match parts.as_slice() {
            [one] => Ok(format!("(NOT {one})")),
            _ => Err(unfoldable("NOT with non-unary operands")),
        },
        BoolOp::And => Ok(format!("({})", parts.join(" AND "))),
        BoolOp::Or => Ok(format!("({})", parts.join(" OR "))),
    }
}

#[cfg(test)]
mod tests {
    use super::super::lower::lower;
    use super::*;
    use crate::lexer::tokenize;
    use crate::parser::parse;

    fn rel(src: &str) -> Rel {
        let toks = tokenize(src).expect("lex");
        let ast = parse(&toks).expect("parse");
        lower(&ast)
    }

    /// Emit DBISAM SQL, expecting success.
    fn sql(src: &str) -> String {
        emit(&rel(src), &Dbisam).expect("foldable")
    }

    /// Emit, expecting an Unfoldable boundary.
    fn boundary(src: &str) -> Unfoldable {
        emit(&rel(src), &Dbisam).expect_err("should not fold")
    }

    #[test]
    fn plain_scan() {
        assert_eq!(sql("t"), r#"SELECT * FROM "t""#);
    }

    #[test]
    fn filter_to_where() {
        assert_eq!(
            sql(r#"Table.SelectRows(t, each [Country] = "GB")"#),
            r#"SELECT * FROM "t" WHERE "Country" = 'GB'"#
        );
    }

    #[test]
    fn conjunction_to_where() {
        assert_eq!(
            sql(r#"Table.SelectRows(t, each [a] = 1 and [b] > 2)"#),
            r#"SELECT * FROM "t" WHERE ("a" = 1 AND "b" > 2)"#
        );
    }

    #[test]
    fn null_comparison_becomes_is_null() {
        assert_eq!(
            sql(r#"Table.SelectRows(t, each [x] = null)"#),
            r#"SELECT * FROM "t" WHERE "x" IS NULL"#
        );
        assert_eq!(
            sql(r#"Table.SelectRows(t, each [x] <> null)"#),
            r#"SELECT * FROM "t" WHERE "x" IS NOT NULL"#
        );
    }

    #[test]
    fn group_by_with_aggregates() {
        assert_eq!(
            sql(r#"Table.Group(t, {"Region"}, {{"Total", each List.Sum([Amount])}, {"N", each Table.RowCount(_)}})"#),
            r#"SELECT "Region", SUM("Amount") AS "Total", COUNT(*) AS "N" FROM "t" GROUP BY "Region""#
        );
    }

    #[test]
    fn select_columns_projection() {
        assert_eq!(
            sql(r#"Table.SelectColumns(t, {"a", "b"})"#),
            r#"SELECT "a", "b" FROM "t""#
        );
    }

    #[test]
    fn add_column_projection() {
        assert_eq!(
            sql(r#"Table.AddColumn(t, "double", each [a] * 2)"#),
            r#"SELECT *, ("a" * 2) AS "double" FROM "t""#
        );
    }

    #[test]
    fn sort_and_top() {
        assert_eq!(
            sql(r#"Table.FirstN(Table.Sort(t, {{"a", Order.Descending}}), 3)"#),
            r#"SELECT * FROM "t" ORDER BY "a" DESC TOP 3"#
        );
    }

    #[test]
    fn distinct_whole_row() {
        assert_eq!(sql("Table.Distinct(t)"), r#"SELECT DISTINCT * FROM "t""#);
    }

    #[test]
    fn scalar_function_allow_list() {
        assert_eq!(
            sql(r#"Table.SelectRows(t, each Text.Upper([name]) = "X")"#),
            r#"SELECT * FROM "t" WHERE UPPER("name") = 'X'"#
        );
    }

    #[test]
    fn full_spine() {
        // WHERE + GROUP BY would normally not combine with a top-level filter,
        // but filter-then-group-then-sort-then-top is the canonical foldable
        // shape.
        assert_eq!(
            sql(r#"Table.FirstN(Table.Sort(Table.SelectRows(t, each [a] > 5), "a"), 10)"#),
            r#"SELECT * FROM "t" WHERE "a" > 5 ORDER BY "a" ASC TOP 10"#
        );
    }

    // --- fold boundaries --------------------------------------------------

    #[test]
    fn opaque_predicate_is_boundary() {
        boundary(r#"Table.SelectRows(t, each MyFunc([a]))"#);
    }

    #[test]
    fn offset_is_boundary() {
        boundary("Table.Range(t, 5, 10)");
    }

    #[test]
    fn keyed_distinct_is_boundary() {
        boundary(r#"Table.Distinct(t, {"a"})"#);
    }

    #[test]
    fn document_scan_is_boundary() {
        boundary(r#"Parquet.Document("p")"#);
    }

    #[test]
    fn inner_join_emits() {
        assert_eq!(
            sql(r#"Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.Inner)"#),
            r#"SELECT * FROM "a" JOIN "b" ON "a"."k" = "b"."k""#
        );
    }

    #[test]
    fn left_outer_join_emits() {
        assert_eq!(
            sql(r#"Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.LeftOuter)"#),
            r#"SELECT * FROM "a" LEFT JOIN "b" ON "a"."k" = "b"."k""#
        );
    }

    #[test]
    fn join_with_qualified_projection_emits() {
        // What the connector's join folder builds: explicit table-qualified
        // output columns over a LEFT JOIN, so nothing is ambiguous.
        let plan = Rel::Project {
            star: false,
            items: vec![
                ProjectItem {
                    name: "SAPRODUCT".into(),
                    expr: Scalar::QualifiedCol {
                        table: "Analysis".into(),
                        name: "SAPRODUCT".into(),
                    },
                },
                ProjectItem {
                    name: "Desc".into(),
                    expr: Scalar::QualifiedCol {
                        table: "PRODGRP".into(),
                        name: "Desc".into(),
                    },
                },
            ],
            input: Box::new(Rel::Join {
                kind: JoinKind::LeftOuter,
                left_keys: vec!["key".into()],
                right_keys: vec!["Sub Sub Category".into()],
                left: Box::new(Rel::Scan(Source::Ref("Analysis".into()))),
                right: Box::new(Rel::Scan(Source::Ref("PRODGRP".into()))),
            }),
        };
        assert_eq!(
            emit(&plan, &Dbisam).expect("foldable"),
            r#"SELECT "Analysis"."SAPRODUCT" AS "SAPRODUCT", "PRODGRP"."Desc" AS "Desc" FROM "Analysis" LEFT JOIN "PRODGRP" ON "Analysis"."key" = "PRODGRP"."Sub Sub Category""#
        );
    }

    #[test]
    fn anti_join_is_boundary() {
        boundary(r#"Table.NestedJoin(a, {"k"}, b, {"k"}, "n", JoinKind.LeftAnti)"#);
    }

    #[test]
    fn aggregate_over_join_emits() {
        // The canonical payoff: SUM(orderi.quantity) grouped by orderh keys
        // over orderh LEFT JOIN orderi — the shape the connector's join+group
        // folder builds, with every column qualified across the two sides.
        let plan = Rel::Aggregate {
            keys: vec![
                Scalar::QualifiedCol { table: "orderh".into(), name: "ref".into() },
                Scalar::QualifiedCol { table: "orderh".into(), name: "custcode".into() },
            ],
            aggs: vec![Aggregation {
                name: "qty".into(),
                func: AggFunc::Sum,
                column: Some(Scalar::QualifiedCol {
                    table: "orderi".into(),
                    name: "quantity".into(),
                }),
            }],
            input: Box::new(Rel::Join {
                kind: JoinKind::LeftOuter,
                left_keys: vec!["ref".into()],
                right_keys: vec!["ref".into()],
                left: Box::new(Rel::Scan(Source::Ref("orderh".into()))),
                right: Box::new(Rel::Scan(Source::Ref("orderi".into()))),
            }),
        };
        assert_eq!(
            emit(&plan, &Dbisam).expect("foldable"),
            r#"SELECT "orderh"."ref", "orderh"."custcode", SUM("orderi"."quantity") AS "qty" FROM "orderh" LEFT JOIN "orderi" ON "orderh"."ref" = "orderi"."ref" GROUP BY "orderh"."ref", "orderh"."custcode""#
        );
    }

    #[test]
    fn filter_above_group_is_boundary() {
        // A HAVING — not emitted in v1.
        boundary(r#"Table.SelectRows(Table.Group(t, {"r"}, {{"s", each List.Sum([x])}}), each [s] > 1)"#);
    }

    // --- fold split -------------------------------------------------------

    #[test]
    fn fold_full_plan() {
        let f = fold(&rel(r#"Table.SelectRows(t, each [a] = 1)"#), &Dbisam);
        assert!(f.is_full());
        assert_eq!(f.sql.as_deref(), Some(r#"SELECT * FROM "t" WHERE "a" = 1"#));
        assert_eq!(f.residual.to_sexpr(), r#"(eval-m "$folded")"#);
    }

    #[test]
    fn fold_splits_sort_over_limit() {
        // Sort above Limit cannot share one SELECT (TOP applies after ORDER BY),
        // so the limit folds and the sort runs over its rows.
        let f = fold(&rel(r#"Table.Sort(Table.FirstN(t, 5), "x")"#), &Dbisam);
        assert_eq!(f.sql.as_deref(), Some(r#"SELECT * FROM "t" TOP 5"#));
        assert_eq!(f.residual.to_sexpr(), r#"(sort ((asc "x")) (eval-m "$folded"))"#);
    }

    #[test]
    fn fold_under_opaque_step() {
        // Pivot is opaque but sits above a foldable filter — fold the spine,
        // keep the pivot as residual.
        let f = fold(
            &rel(r#"Table.Pivot(Table.SelectRows(t, each [a] = 1), {"x"}, "b", "c")"#),
            &Dbisam,
        );
        assert_eq!(f.sql.as_deref(), Some(r#"SELECT * FROM "t" WHERE "a" = 1"#));
        assert_eq!(
            f.residual.to_sexpr(),
            r#"(eval-m "Table.Pivot" (eval-m "$folded"))"#
        );
    }

    #[test]
    fn fold_nothing_when_no_sql_source() {
        let f = fold(&rel(r#"Parquet.Document("p")"#), &Dbisam);
        assert_eq!(f.sql, None);
        assert!(!f.is_full());
    }
}
