//! Shared helpers used by multiple stdlib sub-modules.

#![allow(unused_imports)]

use std::sync::Arc;

use crate::parser::{Expr, Param};

use super::super::env::{Env, EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Record, Table, Value};

// --- Param builders ---

pub(super) fn one(name: &str) -> Vec<Param> {
    vec![Param {
        name: name.into(),
        optional: false,
        type_annotation: None,
    }]
}


pub(super) fn two(a: &str, b: &str) -> Vec<Param> {
    vec![
        Param {
            name: a.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: b.into(),
            optional: false,
            type_annotation: None,
        },
    ]
}


pub(super) fn three(a: &str, b: &str, c: &str) -> Vec<Param> {
    vec![
        Param {
            name: a.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: b.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: c.into(),
            optional: false,
            type_annotation: None,
        },
    ]
}


// --- Error / arg validators ---


pub(super) fn type_mismatch(expected: &'static str, found: &Value) -> MError {
    MError::TypeMismatch {
        expected,
        found: super::super::type_name(found),
    }
}

// --- Number.* ---


pub(super) fn expect_text(v: &Value) -> Result<&str, MError> {
    match v {
        Value::Text(s) => Ok(s.as_str()),
        other => Err(type_mismatch("text", other)),
    }
}

// --- List.* ---


pub(super) fn expect_list(v: &Value) -> Result<&Vec<Value>, MError> {
    match v {
        Value::List(xs) => Ok(xs),
        other => Err(type_mismatch("list", other)),
    }
}


pub(super) fn expect_function(v: &Value) -> Result<&Closure, MError> {
    match v {
        Value::Function(c) => Ok(c),
        other => Err(type_mismatch("function", other)),
    }
}

/// Call a closure from within a builtin. Builtins receive already-forced
/// values, so we need only mirror the body-dispatch part of the evaluator's
/// Invoke handling. For M-bodied closures we run the body in the captured
/// env; for nested builtins we recurse directly.
///
/// Slice-6 stdlib needs IoHost-free recursion only — List.Transform's
/// callback fn cannot itself perform IO since builtins don't carry a host.
/// Pass a no-op host to be safe.

pub(super) fn expect_table(v: &Value) -> Result<&Table, MError> {
    match v {
        Value::Table(t) => Ok(t),
        other => Err(type_mismatch("table", other)),
    }
}


pub(super) fn expect_text_list(v: &Value, ctx: &str) -> Result<Vec<String>, MError> {
    let xs = expect_list(v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match x {
            Value::Text(s) => out.push(s.clone()),
            other => {
                return Err(MError::Other(format!(
                    "{}: expected list of text, got {}",
                    ctx,
                    super::super::type_name(other)
                )));
            }
        }
    }
    Ok(out)
}


pub(super) fn expect_list_of_lists<'a>(v: &'a Value, ctx: &str) -> Result<Vec<Vec<Value>>, MError> {
    let xs = expect_list(v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match x {
            Value::List(inner) => out.push(inner.clone()),
            other => {
                return Err(MError::Other(format!(
                    "{}: expected list of lists, got {}",
                    ctx,
                    super::super::type_name(other)
                )));
            }
        }
    }
    Ok(out)
}

/// Build a Table from column names + row-major cells. Picks the Arrow-backed
/// representation when every column fits the uniform-column rule; falls back
/// to a Rows-backed Table when any column is heterogeneous (compound values,
/// mixed primitives, Binary).

pub(super) fn expect_int(v: &Value, ctx: &str) -> Result<i64, MError> {
    match v {
        Value::Number(n) => {
            if n.fract() != 0.0 {
                return Err(MError::Other(format!("{}: not an integer: {}", ctx, n)));
            }
            Ok(*n as i64)
        }
        other => Err(type_mismatch("number", other)),
    }
}

// --- Parquet IO (eval-7c) ---
//
// The pure evaluator core can't open files; Parquet.Document just delegates
// to the shell's IoHost. CliIoHost in mrsflow-cli decodes the file via the
// `parquet` crate; NoIoHost (default in unit tests) errors. WASM shell will
// similarly error or proxy through DuckDB-Wasm later.


pub(crate) fn values_equal_primitive(a: &Value, b: &Value) -> Result<bool, MError> {
    match (a, b) {
        (Value::Null, Value::Null) => Ok(true),
        (Value::Logical(x), Value::Logical(y)) => Ok(x == y),
        (Value::Number(x), Value::Number(y)) => Ok(x == y),
        (Value::Text(x), Value::Text(y)) => Ok(x == y),
        (Value::Date(x), Value::Date(y)) => Ok(x == y),
        (Value::Datetime(x), Value::Datetime(y)) => Ok(x == y),
        (Value::Duration(x), Value::Duration(y)) => Ok(x == y),
        // Different primitive variants are not equal — null vs non-null included.
        (
            Value::Null
            | Value::Logical(_)
            | Value::Number(_)
            | Value::Text(_)
            | Value::Date(_)
            | Value::Datetime(_)
            | Value::Duration(_),
            Value::Null
            | Value::Logical(_)
            | Value::Number(_)
            | Value::Text(_)
            | Value::Date(_)
            | Value::Datetime(_)
            | Value::Duration(_),
        ) => Ok(false),
        _ => Err(MError::NotImplemented(
            "equality on compound values (list/record/table/etc.) deferred",
        )),
    }
}


pub(super) fn int_n_arg(v: &Value, ctx: &str) -> Result<i64, MError> {
    match v {
        Value::Number(n) if n.is_finite() && n.fract() == 0.0 => Ok(*n as i64),
        other => Err(MError::Other(format!(
            "{}: count must be an integer (got {})", ctx, super::super::type_name(other)
        ))),
    }
}


pub(super) fn invoke_builtin_callback(closure: &Closure, args: Vec<Value>) -> Result<Value, MError> {
    if args.len() != closure.params.len() {
        return Err(MError::Other(format!(
            "callback: arity mismatch: expected {}, got {}",
            closure.params.len(),
            args.len()
        )));
    }
    // Callbacks from List.Transform/Select can't reach the original host
    // — pass NoIoHost so IO-using callbacks fail loudly rather than picking
    // up some unrelated environment. If a future stdlib function needs to
    // thread the real host through callbacks, refactor this signature.
    let host = super::super::NoIoHost;
    match &closure.body {
        FnBody::Builtin(f) => f(&args, &host),
        FnBody::M(body) => {
            let mut call_env = closure.env.clone();
            for (param, value) in closure.params.iter().zip(args.into_iter()) {
                call_env = call_env.extend(param.name.clone(), value);
            }
            super::super::evaluate(body, &call_env, &host)
        }
    }
}

// --- Record.* ---


pub(super) fn invoke_callback_with_host(
    closure: &Closure,
    args: Vec<Value>,
    host: &dyn IoHost,
) -> Result<Value, MError> {
    if args.len() != closure.params.len() {
        return Err(MError::Other(format!(
            "callback: arity mismatch: expected {}, got {}",
            closure.params.len(),
            args.len()
        )));
    }
    match &closure.body {
        FnBody::Builtin(f) => f(&args, host),
        FnBody::M(body) => {
            let mut call_env = closure.env.clone();
            for (param, value) in closure.params.iter().zip(args.into_iter()) {
                call_env = call_env.extend(param.name.clone(), value);
            }
            super::super::evaluate(body, &call_env, host)
        }
    }
}

// --- Table.* eval-7e: type-aware ops + concat ---

