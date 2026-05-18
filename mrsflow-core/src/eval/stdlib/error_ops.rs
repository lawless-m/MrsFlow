//! `Error.*` and `Action.*` stdlib bindings. Tiny — `Error.Record` is
//! the canonical constructor for the record used with the `error`
//! keyword. `Action.WithErrorContext` is a Power BI engine internal
//! that lands as NotImplemented until mrsflow grows an `Action` type.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Value};
use super::common::{expect_text, three, two, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Error.Record", three("reason", "message", "detail"), record),
        ("Action.WithErrorContext", two("action", "context"), action_with_error_context),
    ]
}

fn record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let reason = expect_text(&args[0])?.to_string();
    let message = expect_text(&args[1])?.to_string();
    Ok(Value::Record(Record {
        fields: vec![
            ("Reason".to_string(), Value::Text(reason)),
            ("Message".to_string(), Value::Text(message)),
            ("Detail".to_string(), args[2].clone()),
        ],
        env: EnvNode::empty(),
    }))
}

fn action_with_error_context(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // MS docs: "intended for internal use only". mrsflow has no `action`
    // type yet (the Action.* family is empty); raise the same coercion
    // error PQ raises when the inner `action` arg isn't an Action value.
    // Since no value can ever satisfy that in mrsflow, every call lands
    // here. The rendered value matches PQ's wording: `null`, `"text"`,
    // `42`, `true`, etc.
    let rendered = match &args[0] {
        Value::Null => "null".to_string(),
        Value::Text(s) => format!("\"{s}\""),
        Value::Number(n) => format!("{n}"),
        Value::Logical(b) => if *b { "true".into() } else { "false".into() },
        _ => "(value)".into(),
    };
    Err(MError::Other(format!(
        "We cannot convert the value {rendered} to type Action."
    )))
}
