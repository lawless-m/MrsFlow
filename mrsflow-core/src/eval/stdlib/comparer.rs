//! `Comparer.*` factory + helper stdlib bindings.
//!
//! Factories return a 2-arg closure `(a, b) => -1 | 0 | 1`. `Comparer.Equals`
//! is a 3-arg helper that wraps an existing comparer.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Value};
use super::common::{
    expect_function, invoke_callback_with_host, one, two, type_mismatch,
};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Comparer.Ordinal", vec![], comparer_ordinal),
        ("Comparer.OrdinalIgnoreCase", vec![], comparer_ordinal_ignore_case),
        (
            "Comparer.FromCulture",
            vec![
                Param { name: "culture".into(),    optional: false, type_annotation: None },
                Param { name: "ignoreCase".into(), optional: true,  type_annotation: None },
            ],
            comparer_from_culture,
        ),
        (
            "Comparer.Equals",
            vec![
                Param { name: "comparer".into(), optional: false, type_annotation: None },
                Param { name: "x".into(),        optional: false, type_annotation: None },
                Param { name: "y".into(),        optional: false, type_annotation: None },
            ],
            comparer_equals,
        ),
    ]
}

// --- Factories ---

fn comparer_ordinal(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_comparer(ordinal_impl))
}

fn comparer_ordinal_ignore_case(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_comparer(ordinal_ignore_case_impl))
}

fn comparer_from_culture(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Culture arg is accepted (must be text) but ignored in v1.
    if !matches!(&args[0], Value::Text(_) | Value::Null) {
        return Err(type_mismatch("text (culture name)", &args[0]));
    }
    let ignore = matches!(args.get(1), Some(Value::Logical(true)));
    if ignore {
        Ok(make_comparer(ordinal_ignore_case_impl))
    } else {
        Ok(make_comparer(ordinal_impl))
    }
}

fn make_comparer(impl_fn: BuiltinFn) -> Value {
    Value::Function(Closure {
        params: vec![
            Param { name: "a".into(), optional: false, type_annotation: None },
            Param { name: "b".into(), optional: false, type_annotation: None },
        ],
        body: FnBody::Builtin(impl_fn),
        env: EnvNode::empty(),
    })
}

// --- Comparer impls ---

fn ordinal_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let cmp = compare_ordinal(&args[0], &args[1])?;
    Ok(Value::Number(cmp as f64))
}

fn ordinal_ignore_case_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let cmp = compare_ordinal_ignore_case(&args[0], &args[1])?;
    Ok(Value::Number(cmp as f64))
}

fn compare_ordinal(a: &Value, b: &Value) -> Result<i32, MError> {
    match (a, b) {
        (Value::Text(x), Value::Text(y)) => Ok(sign(x.as_bytes().cmp(y.as_bytes()))),
        (Value::Number(x), Value::Number(y)) => Ok(sign(
            x.partial_cmp(y)
                .ok_or_else(|| MError::Other("Comparer.Ordinal: NaN".into()))?,
        )),
        (Value::Logical(x), Value::Logical(y)) => Ok(sign(x.cmp(y))),
        (Value::Null, Value::Null) => Ok(0),
        // Null sorts before non-null per Power Query.
        (Value::Null, _) => Ok(-1),
        (_, Value::Null) => Ok(1),
        (other, _) => Err(type_mismatch("comparable value", other)),
    }
}

fn compare_ordinal_ignore_case(a: &Value, b: &Value) -> Result<i32, MError> {
    match (a, b) {
        (Value::Text(x), Value::Text(y)) => {
            // ASCII case-fold, then byte-compare.
            let lx = x.to_ascii_lowercase();
            let ly = y.to_ascii_lowercase();
            Ok(sign(lx.as_bytes().cmp(ly.as_bytes())))
        }
        _ => compare_ordinal(a, b),
    }
}

fn sign(o: std::cmp::Ordering) -> i32 {
    match o {
        std::cmp::Ordering::Less => -1,
        std::cmp::Ordering::Equal => 0,
        std::cmp::Ordering::Greater => 1,
    }
}

// --- Helper: Comparer.Equals(comparer, x, y) ---

fn comparer_equals(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let cmp = expect_function(&args[0])?;
    let result = invoke_callback_with_host(cmp, vec![args[1].clone(), args[2].clone()], host)?;
    match result {
        Value::Number(n) => Ok(Value::Logical(n == 0.0)),
        other => Err(type_mismatch("number (comparer result)", &other)),
    }
}
