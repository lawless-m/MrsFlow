//! `Function.*` stdlib bindings.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{expect_function, expect_list, invoke_callback_with_host, one, two, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Function.From", two("functionType", "function"), function_from),
        ("Function.Invoke", two("function", "args"), function_invoke),
        ("Function.InvokeAfter", two("function", "delay"), function_invoke_after),
        (
            "Function.InvokeWithErrorContext",
            two("function", "errorContext"),
            function_invoke_with_error_context,
        ),
        ("Function.IsDataSource", one("function"), function_is_data_source),
        (
            "Function.ScalarVector",
            two("scalarFunction", "vectorFunction"),
            function_scalar_vector,
        ),
    ]
}

fn function_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: signature/type info isn't tracked on functions — ignore arg 0.
    let _ = &args[0];
    let _ = expect_function(&args[1])?;
    Ok(args[1].clone())
}

fn function_invoke(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let f = expect_function(&args[0])?;
    let xs = expect_list(&args[1])?;
    invoke_callback_with_host(f, xs.clone(), host)
}

fn function_invoke_after(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    // v1: we have no async runtime — fire immediately. The `delay` value
    // (a Duration) is accepted but ignored.
    let f = expect_function(&args[0])?;
    if !matches!(&args[1], Value::Duration(_) | Value::Null) {
        return Err(type_mismatch("duration", &args[1]));
    }
    invoke_callback_with_host(f, Vec::new(), host)
}

fn function_invoke_with_error_context(
    args: &[Value],
    host: &dyn IoHost,
) -> Result<Value, MError> {
    let f = expect_function(&args[0])?;
    // errorContext is a record describing the call site. v1 doesn't tag the
    // error path with it — on success the result passes through unchanged.
    let _ = &args[1];
    invoke_callback_with_host(f, Vec::new(), host)
}

fn function_is_data_source(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no data-source tagging on functions.
    let _ = expect_function(&args[0])?;
    Ok(Value::Logical(false))
}

fn function_scalar_vector(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no query folding — always pick the vector (runtime) function.
    let _ = expect_function(&args[0])?;
    let _ = expect_function(&args[1])?;
    Ok(args[1].clone())
}
