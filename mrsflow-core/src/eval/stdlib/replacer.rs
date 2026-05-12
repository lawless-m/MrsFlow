//! `Replacer.*` factory stdlib bindings.
//!
//! Each factory takes no args and returns a `Value::Function` that accepts
//! `(input, oldValue, newValue)` and produces the replaced value. Used as
//! the `replacer` argument to `Text.Replace`, `Table.ReplaceValue`, etc.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Value};
use super::common::{expect_text, type_mismatch, values_equal_primitive};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Replacer.ReplaceText", vec![], replacer_replace_text),
        ("Replacer.ReplaceValue", vec![], replacer_replace_value),
    ]
}

fn replacer_replace_text(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_replacer(replace_text_impl))
}

fn replacer_replace_value(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_replacer(replace_value_impl))
}

/// Build a 3-arg closure `(input, oldValue, newValue) => impl(...)`.
fn make_replacer(impl_fn: BuiltinFn) -> Value {
    Value::Function(Closure {
        params: vec![
            Param { name: "input".into(),    optional: false, type_annotation: None },
            Param { name: "oldValue".into(), optional: false, type_annotation: None },
            Param { name: "newValue".into(), optional: false, type_annotation: None },
        ],
        body: FnBody::Builtin(impl_fn),
        env: EnvNode::empty(),
    })
}

fn replace_text_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let input = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    if old.is_empty() {
        // Power Query: empty oldValue is a no-op (replacing nothing).
        return Ok(Value::Text(input.to_string()));
    }
    Ok(Value::Text(input.replace(old, new)))
}

fn replace_value_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if values_equal_primitive(&args[0], &args[1])? {
        Ok(args[2].clone())
    } else {
        Ok(args[0].clone())
    }
}
