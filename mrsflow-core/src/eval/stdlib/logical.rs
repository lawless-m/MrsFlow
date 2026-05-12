//! `Logical.*` stdlib bindings.

#![allow(unused_imports)]

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Date32Array, DurationMicrosecondArray, Float64Array,
    NullArray, StringArray, TimestampMicrosecondArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::parser::Param;

use super::super::env::{Env, EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Record, Table, Value};
use super::common::{
    expect_function, expect_int, expect_list, expect_list_of_lists, expect_table,
    expect_text, expect_text_list, int_n_arg, invoke_builtin_callback,
    invoke_callback_with_host, one, three, two, type_mismatch, values_equal_primitive,
};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Logical.ToText", one("logical"), to_text),
        ("Logical.From", one("value"), from),
        ("Logical.FromText", one("text"), from_text),
    ]
}

fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Logical(b) => Ok(Value::Text(if *b { "true".into() } else { "false".into() })),
        other => Err(type_mismatch("logical", other)),
    }
}


fn from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Logical(b) => Ok(Value::Logical(*b)),
        Value::Number(n) => Ok(Value::Logical(*n != 0.0)),
        Value::Text(_) => from_text(args, host),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}


fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    match text.to_ascii_lowercase().as_str() {
        "true" => Ok(Value::Logical(true)),
        "false" => Ok(Value::Logical(false)),
        _ => Err(MError::Other(format!(
            "Logical.FromText: not a boolean: {text:?}"
        ))),
    }
}


// --- Table.* (eval-7a) ---
//
// #table(columns, rows) and the three top-corpus Table.* operations.
// Compound type expressions in the columns position aren't supported in
// this slice — only a list of text column names. Date/Datetime/Duration/
// Binary cells land in eval-7b alongside chrono.

