//! `Diagnostics.*` stdlib bindings.
//!
//! mrsflow has no host-managed trace sink — `Diagnostics.Trace` is a
//! pass-through that returns `value` unchanged after eprinting the
//! message. ActivityId / CorrelationId return a null-GUID string since
//! mrsflow has no per-evaluation identifier to surface.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::type_mismatch;

const NULL_GUID: &str = "00000000-0000-0000-0000-000000000000";

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Diagnostics.ActivityId", vec![], activity_id),
        ("Diagnostics.CorrelationId", vec![], correlation_id),
        (
            "Diagnostics.Trace",
            vec![
                Param { name: "traceLevel".into(), optional: false, type_annotation: None },
                Param { name: "message".into(),    optional: false, type_annotation: None },
                Param { name: "value".into(),      optional: false, type_annotation: None },
                Param { name: "delayed".into(),    optional: true,  type_annotation: None },
            ],
            trace,
        ),
    ]
}

fn activity_id(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Text(NULL_GUID.into()))
}

fn correlation_id(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Text(NULL_GUID.into()))
}

fn trace(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let level = match &args[0] {
        Value::Number(n) => *n as i64,
        other => return Err(type_mismatch("number (traceLevel)", other)),
    };
    let message = match &args[1] {
        Value::Text(s) => s.clone(),
        Value::Null => String::new(),
        // anynonnull per the spec, but fall back to a debug rendering
        // rather than erroring — diagnostics shouldn't change observable
        // result on bad input.
        other => format!("{other:?}"),
    };
    eprintln!("[Diagnostics.Trace {level}] {message}");
    // `delayed = true` means the host should defer forcing `value` until
    // the trace is emitted. Since we always emit eagerly above, the
    // distinction doesn't matter — return value as-is either way.
    Ok(args[2].clone())
}
