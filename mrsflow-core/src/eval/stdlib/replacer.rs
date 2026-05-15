//! `Replacer.*` stdlib bindings.
//!
//! In Power Query, `Replacer.ReplaceValue` and `Replacer.ReplaceText` are
//! themselves 3-arg functions `(input, oldValue, newValue) => …`, passed
//! bare (no parens) as the `replacer` argument to `Text.Replace`,
//! `Table.ReplaceValue`, etc. They are not factories.

#![allow(unused_imports)]

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{expect_text, three, type_mismatch, values_equal_primitive};
use crate::parser::Param;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Replacer.ReplaceText",  three("input", "oldValue", "newValue"), replace_text),
        ("Replacer.ReplaceValue", three("input", "oldValue", "newValue"), replace_value),
    ]
}

fn replace_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let input = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    if old.is_empty() {
        return Ok(Value::Text(input.to_string()));
    }
    Ok(Value::Text(input.replace(old, new)))
}

fn replace_value(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if values_equal_primitive(&args[0], &args[1])? {
        Ok(args[2].clone())
    } else {
        Ok(args[0].clone())
    }
}
