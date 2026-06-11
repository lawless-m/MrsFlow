//! `RowExpression.*` and `ItemExpression.*` — AST reflection for connector
//! query-folding handlers.
//!
//! `RowExpression.From(f)` lifts a 1-arg lambda's body into a row-expression
//! record. The body must be restricted to: Constant, Invocation, Unary,
//! Binary, If, FieldAccess. Parameter references become `RowExpression.Row`;
//! `param[colname]` shortcuts become `RowExpression.Column(colname)`.
//!
//! `ItemExpression.From` is documented as identical — same handler.
//!
//! `RowExpression.Row` / `ItemExpression.Item` are sentinel records — the
//! "current row" / "current item" placeholder.
//!
//! `RowExpression.Column(name)` constructs a column-reference record.

#![allow(unused_imports)]

use crate::parser::{BinaryOp, Expr, Param, UnaryOp};

use super::super::env::{Env, EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, FnBody, MError, Record, Value};
use super::common::{expect_text, one, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("RowExpression.From",   one("function"), from_handler),
        ("ItemExpression.From",  one("function"), from_handler),
        ("RowExpression.Column", one("name"),     column),
    ]
}

/// Bind the sentinel `RowExpression.Row` / `ItemExpression.Item` values into
/// the root env. They're plain marker records — callers walking the AST
/// compare against them by shape.
pub(super) fn extend_env(env: Env) -> Env {
    env.extend("RowExpression.Row".into(), sentinel_invocation())
       .extend("ItemExpression.Item".into(), sentinel_invocation())
}

/// Sentinel record for the "current row"/"current item" placeholder. Excel
/// renders both as the same shape: an Invocation AST node with no
/// Function/Arguments fields — it's the placeholder the AST walker
/// substitutes for the lambda parameter.
fn sentinel_invocation() -> Value {
    Value::Record(Record {
        fields: vec![("Kind".into(), Value::Text("Invocation".into()))],
        env: EnvNode::empty(),
    })
}

fn from_handler(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let closure = match &args[0] {
        Value::Function(c) => c.clone(),
        other => return Err(type_mismatch("function", other)),
    };
    if closure.params.len() != 1 {
        return Err(MError::Other(
            "RowExpression.From: function must take exactly one argument.".into(),
        ));
    }
    let body = match &closure.body {
        FnBody::M(expr) => expr.as_ref(),
        FnBody::Builtin(_) => {
            return Err(MError::Other(
                "RowExpression.From: cannot reflect a builtin function.".into(),
            ));
        }
    };
    let param_name = closure.params[0].name.clone();
    walk(body, &param_name)
}

fn walk(e: &Expr, param: &str) -> Result<Value, MError> {
    match e {
        Expr::Identifier(name) if name == param => Ok(sentinel_invocation()),
        Expr::NumberLit(s) => {
            let n: f64 = s.parse()
                .map_err(|_| MError::Other(format!("RowExpression: bad number literal {s:?}")))?;
            Ok(constant(Value::Number(n)))
        }
        Expr::TextLit(s) => Ok(constant(Value::Text(s.clone()))),
        Expr::LogicalLit(b) => Ok(constant(Value::Logical(*b))),
        Expr::NullLit => Ok(constant(Value::Null)),
        Expr::Unary(op, inner) => Ok(record(vec![
            ("Kind", Value::Text("Unary".into())),
            ("Operator", Value::Text(unary_name(op).into())),
            ("Expression", walk(inner, param)?),
        ])),
        Expr::Binary(op, l, r) => Ok(record(vec![
            ("Kind", Value::Text("Binary".into())),
            ("Operator", Value::Text(binary_name(op).into())),
            ("Left", walk(l, param)?),
            ("Right", walk(r, param)?),
        ])),
        Expr::If { cond, then_branch, else_branch } => Ok(record(vec![
            ("Kind", Value::Text("If".into())),
            ("Condition", walk(cond, param)?),
            ("True", walk(then_branch, param)?),
            ("False", walk(else_branch, param)?),
        ])),
        // FieldAccess stays as FieldAccess regardless of whether the
        // target is the param. The docs imply a RowExpression.Column
        // substitution, but Excel's actual output uses FieldAccess
        // throughout — RowExpression.Column is just a constructor for
        // callers who want to BUILD a column-ref by hand.
        Expr::FieldAccess { target, field, optional: _ } => Ok(record(vec![
            ("Kind", Value::Text("FieldAccess".into())),
            ("Expression", walk(target, param)?),
            ("MemberName", Value::Text(field.clone())),
        ])),
        Expr::Invoke { target, args } => {
            let target_v = walk(target, param)?;
            let arg_vs: Result<Vec<Value>, _> = args.iter().map(|a| walk(a, param)).collect();
            Ok(record(vec![
                ("Kind", Value::Text("Invocation".into())),
                ("Function", target_v),
                ("Arguments", Value::list_of(arg_vs?)),
            ]))
        }
        other => Err(MError::Other(format!(
            "RowExpression.From: expression node not supported in a row expression: {other:?}"
        ))),
    }
}

fn unary_name(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Plus => "Positive",
        UnaryOp::Minus => "Negative",
        UnaryOp::Not => "Not",
        UnaryOp::Type => "Type",
        UnaryOp::Nullable => "Nullable",
    }
}

fn binary_name(op: &BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "Add",
        BinaryOp::Subtract => "Subtract",
        BinaryOp::Multiply => "Multiply",
        BinaryOp::Divide => "Divide",
        // `&` (concat) must be distinct from logical `and`; reflecting both
        // as "And" made them indistinguishable. No RowExpression oracle case
        // exercises `&`, so this is unobserved today.
        BinaryOp::Concat => "Concatenate",
        BinaryOp::LessThan => "LessThan",
        BinaryOp::LessEquals => "LessThanOrEqualTo",
        BinaryOp::GreaterThan => "GreaterThan",
        BinaryOp::GreaterEquals => "GreaterThanOrEqualTo",
        BinaryOp::Equal => "Equals",
        BinaryOp::NotEqual => "NotEquals",
        BinaryOp::And => "And",
        BinaryOp::Or => "Or",
        BinaryOp::As => "As",
        BinaryOp::Is => "Is",
        BinaryOp::Meta => "Meta",
    }
}

fn constant(v: Value) -> Value {
    record(vec![
        ("Kind", Value::Text("Constant".into())),
        ("Value", v),
    ])
}

/// `RowExpression.Column(name)` returns a FieldAccess AST record whose
/// target is the row-sentinel — identical to what `RowExpression.From`
/// emits for `each [name]` (which desugars to `each _[name]`).
fn column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let name = expect_text(&args[0])?;
    Ok(record(vec![
        ("Kind", Value::Text("FieldAccess".into())),
        ("Expression", sentinel_invocation()),
        ("MemberName", Value::Text(name.into())),
    ]))
}

fn record(fields: Vec<(&'static str, Value)>) -> Value {
    Value::Record(Record {
        fields: fields.into_iter().map(|(k, v)| (k.into(), v)).collect(),
        env: EnvNode::empty(),
    })
}
