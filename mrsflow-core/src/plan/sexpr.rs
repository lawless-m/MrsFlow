//! S-expression rendering for the Plan IR. One uniform textual form for both
//! the relational and scalar layers, so every lowering / rewrite pass can be
//! dumped and diffed — which is what the differential harness consumes.
//!
//! String quoting matches `parser::ast`'s printer so the two dumps are
//! byte-compatible. Names (columns, sources, descriptors) are always quoted —
//! M identifiers can contain spaces and punctuation, so the doc's illustrative
//! unquoted `(col Country)` becomes `(col "Country")` here.

use super::ir::*;

impl Rel {
    /// Canonical S-expression form of a relational plan.
    pub fn to_sexpr(&self) -> String {
        let mut out = String::new();
        write_rel(&mut out, self);
        out
    }
}

impl Scalar {
    /// Canonical S-expression form of a scalar expression.
    pub fn to_sexpr(&self) -> String {
        let mut out = String::new();
        write_scalar(&mut out, self);
        out
    }
}

fn write_rel(out: &mut String, r: &Rel) {
    match r {
        Rel::Scan(src) => {
            out.push_str("(scan ");
            write_source(out, src);
            out.push(')');
        }
        Rel::Filter { predicate, input } => {
            out.push_str("(filter ");
            write_scalar(out, predicate);
            out.push(' ');
            write_rel(out, input);
            out.push(')');
        }
        Rel::Project { star, items, input } => {
            out.push_str("(project ");
            out.push_str(if *star { "extend" } else { "replace" });
            out.push_str(" (");
            for (i, it) in items.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, &it.name);
                out.push(' ');
                write_scalar(out, &it.expr);
                out.push(')');
            }
            out.push_str(") ");
            write_rel(out, input);
            out.push(')');
        }
        Rel::Sort { keys, input } => {
            out.push_str("(sort (");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                out.push_str(if k.descending { "desc " } else { "asc " });
                write_quoted(out, &k.column);
                out.push(')');
            }
            out.push_str(") ");
            write_rel(out, input);
            out.push(')');
        }
        Rel::Limit { n, offset, input } => {
            out.push_str("(limit ");
            match n {
                Some(n) => out.push_str(&n.to_string()),
                None => out.push('*'),
            }
            out.push(' ');
            out.push_str(&offset.to_string());
            out.push(' ');
            write_rel(out, input);
            out.push(')');
        }
        Rel::Aggregate { keys, aggs, input } => {
            out.push_str("(aggregate (");
            for (i, k) in keys.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write_scalar(out, k);
            }
            out.push_str(") (");
            for (i, a) in aggs.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, &a.name);
                out.push(' ');
                out.push_str(agg_func_name(a.func));
                if let Some(c) = &a.column {
                    out.push(' ');
                    write_scalar(out, c);
                }
                out.push(')');
            }
            out.push_str(") ");
            write_rel(out, input);
            out.push(')');
        }
        Rel::Join {
            kind,
            left_keys,
            right_keys,
            left,
            right,
        } => {
            out.push_str("(join ");
            out.push_str(join_kind_name(*kind));
            out.push_str(" (");
            write_name_list(out, left_keys);
            out.push_str(") (");
            write_name_list(out, right_keys);
            out.push_str(") ");
            write_rel(out, left);
            out.push(' ');
            write_rel(out, right);
            out.push(')');
        }
        Rel::Distinct { on, input } => {
            out.push_str("(distinct (");
            write_name_list(out, on);
            out.push_str(") ");
            write_rel(out, input);
            out.push(')');
        }
        Rel::EvalM { descr, inputs } => {
            out.push_str("(eval-m ");
            write_quoted(out, descr);
            for inp in inputs {
                out.push(' ');
                write_rel(out, inp);
            }
            out.push(')');
        }
    }
}

fn write_source(out: &mut String, s: &Source) {
    match s {
        Source::Document { func, args } => {
            out.push_str("(document ");
            write_quoted(out, func);
            for a in args {
                out.push(' ');
                write_scalar(out, a);
            }
            out.push(')');
        }
        Source::Ref(name) => {
            out.push_str("(ref ");
            write_quoted(out, name);
            out.push(')');
        }
    }
}

fn write_scalar(out: &mut String, s: &Scalar) {
    match s {
        Scalar::Col(name) => {
            out.push_str("(col ");
            write_quoted(out, name);
            out.push(')');
        }
        Scalar::QualifiedCol { table, name } => {
            out.push_str("(col ");
            write_quoted(out, table);
            out.push(' ');
            write_quoted(out, name);
            out.push(')');
        }
        Scalar::Lit(lit) => write_lit(out, lit),
        Scalar::Cmp { op, lhs, rhs } => {
            out.push('(');
            out.push_str(cmp_op_name(*op));
            out.push(' ');
            write_scalar(out, lhs);
            out.push(' ');
            write_scalar(out, rhs);
            out.push(')');
        }
        Scalar::Bool { op, args } => {
            out.push('(');
            out.push_str(bool_op_name(*op));
            for a in args {
                out.push(' ');
                write_scalar(out, a);
            }
            out.push(')');
        }
        Scalar::Arith { op, lhs, rhs } => {
            out.push('(');
            out.push_str(arith_op_name(*op));
            out.push(' ');
            write_scalar(out, lhs);
            out.push(' ');
            write_scalar(out, rhs);
            out.push(')');
        }
        Scalar::Call { func, args } => {
            out.push_str("(call ");
            write_quoted(out, func);
            for a in args {
                out.push(' ');
                write_scalar(out, a);
            }
            out.push(')');
        }
        Scalar::Opaque => out.push_str("(opaque)"),
    }
}

fn write_lit(out: &mut String, lit: &Lit) {
    match lit {
        Lit::Number(s) => {
            out.push_str("(lit number ");
            write_quoted(out, s);
            out.push(')');
        }
        Lit::Text(s) => {
            out.push_str("(lit text ");
            write_quoted(out, s);
            out.push(')');
        }
        Lit::Logical(b) => {
            out.push_str("(lit bool ");
            out.push_str(if *b { "true" } else { "false" });
            out.push(')');
        }
        Lit::Date(d) => {
            out.push_str("(lit date ");
            write_quoted(out, &d.to_string());
            out.push(')');
        }
        Lit::Datetime(dt) => {
            out.push_str("(lit datetime ");
            write_quoted(out, &dt.to_string());
            out.push(')');
        }
        Lit::Null => out.push_str("(lit null)"),
    }
}

fn write_name_list(out: &mut String, names: &[String]) {
    for (i, n) in names.iter().enumerate() {
        if i > 0 {
            out.push(' ');
        }
        write_quoted(out, n);
    }
}

fn cmp_op_name(op: CmpOp) -> &'static str {
    match op {
        CmpOp::Eq => "=",
        CmpOp::Ne => "<>",
        CmpOp::Lt => "<",
        CmpOp::Le => "<=",
        CmpOp::Gt => ">",
        CmpOp::Ge => ">=",
    }
}

fn bool_op_name(op: BoolOp) -> &'static str {
    match op {
        BoolOp::And => "and",
        BoolOp::Or => "or",
        BoolOp::Not => "not",
    }
}

fn arith_op_name(op: ArithOp) -> &'static str {
    match op {
        ArithOp::Add => "+",
        ArithOp::Sub => "-",
        ArithOp::Mul => "*",
        ArithOp::Div => "/",
    }
}

fn agg_func_name(f: AggFunc) -> &'static str {
    match f {
        AggFunc::Sum => "sum",
        AggFunc::Count => "count",
        AggFunc::Average => "avg",
        AggFunc::Min => "min",
        AggFunc::Max => "max",
        AggFunc::CountDistinct => "count-distinct",
        AggFunc::Opaque => "opaque",
    }
}

fn join_kind_name(k: JoinKind) -> &'static str {
    match k {
        JoinKind::Inner => "inner",
        JoinKind::LeftOuter => "left",
        JoinKind::RightOuter => "right",
        JoinKind::FullOuter => "full",
        JoinKind::LeftAnti => "left-anti",
        JoinKind::RightAnti => "right-anti",
    }
}

/// Quote a string for the S-expression format. Mirrors the parser-level
/// `write_quoted` so the M-AST dump and the Plan-IR dump escape identically.
fn write_quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
}
