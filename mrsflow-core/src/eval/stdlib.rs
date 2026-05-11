//! Starter stdlib for eval-6: pure functions bound in the root env.
//!
//! Each function lives in this module as a `BuiltinFn`. `root_env()` builds
//! the initial env containing every binding, used by callers that want a
//! stdlib-aware environment instead of an empty one (`EnvNode::empty()`).
//!
//! Function scope is corpus-driven: the top non-Arrow stdlib calls in the
//! user's actual queries (`Text.Replace`, `Text.Contains`, `List.Transform`,
//! `Number.From`, …). Arrow-backed Table.* and date/datetime/duration land
//! in eval-7+.

use crate::parser::Param;

use super::env::{Env, EnvNode, EnvOps};
use super::value::{BuiltinFn, Closure, FnBody, MError, Value};

/// Build the initial environment containing every stdlib intrinsic plus
/// the two literal constants `#nan` and `#infinity`. Tests and shells pass
/// this as the starting env instead of `EnvNode::empty()`.
pub fn root_env() -> Env {
    let mut env = EnvNode::empty();
    for (name, params, body) in builtin_bindings() {
        let closure = Closure {
            params,
            body: FnBody::Builtin(body),
            env: EnvNode::empty(),
        };
        env = env.extend(name.to_string(), Value::Function(closure));
    }
    env = env.extend("#nan".into(), Value::Number(f64::NAN));
    env = env.extend("#infinity".into(), Value::Number(f64::INFINITY));
    env
}

fn one(name: &str) -> Vec<Param> {
    vec![Param {
        name: name.into(),
        optional: false,
        type_annotation: None,
    }]
}

fn two(a: &str, b: &str) -> Vec<Param> {
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

fn three(a: &str, b: &str, c: &str) -> Vec<Param> {
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

fn builtin_bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Number.From", one("value"), number_from),
        ("Text.From", one("value"), text_from),
        ("Text.Contains", two("text", "substring"), text_contains),
        ("Text.Replace", three("text", "old", "new"), text_replace),
        ("Text.Trim", one("text"), text_trim),
        ("Text.Length", one("text"), text_length),
        ("Text.PositionOf", two("text", "substring"), text_position_of),
        ("Text.EndsWith", two("text", "suffix"), text_ends_with),
        ("List.Transform", two("list", "transform"), list_transform),
        ("List.Select", two("list", "selection"), list_select),
        ("List.Sum", one("list"), list_sum),
        ("List.Count", one("list"), list_count),
        ("List.Min", one("list"), list_min),
        ("List.Max", one("list"), list_max),
        ("Record.Field", two("record", "field"), record_field),
        ("Record.FieldNames", one("record"), record_field_names),
        ("Logical.From", one("value"), logical_from),
        ("Logical.FromText", one("text"), logical_from_text),
    ]
}

fn type_mismatch(expected: &'static str, found: &Value) -> MError {
    MError::TypeMismatch {
        expected,
        found: super::type_name(found),
    }
}

// --- Number.* ---

fn number_from(args: &[Value]) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(*n)),
        Value::Logical(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
        Value::Text(s) => s
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| MError::Other(format!("Number.From: cannot parse {:?}", s))),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

// --- Text.* ---

fn text_from(args: &[Value]) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => Ok(Value::Text(s.clone())),
        // {:?} for f64 matches `value_dump`'s canonical num format
        // (always-trailing fractional digit). Keeping parity here so
        // Text.From(42) prints the same as the differential's `(num 42.0)`.
        Value::Number(n) => Ok(Value::Text(format!("{:?}", n))),
        Value::Logical(b) => Ok(Value::Text(
            if *b { "true" } else { "false" }.to_string(),
        )),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

fn text_contains(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    Ok(Value::Logical(text.contains(sub)))
}

fn text_replace(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    Ok(Value::Text(text.replace(old, new)))
}

fn text_trim(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.trim().to_string()))
}

fn text_length(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // M counts characters, not bytes — use char count.
    Ok(Value::Number(text.chars().count() as f64))
}

fn text_position_of(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    // Per spec: -1 when not found, byte offset on miss... but for parity
    // with the M spec (and the corpus), return a char index. The empty-sub
    // edge case isn't load-bearing for slice-6 tests.
    let idx = text.find(sub).map(|byte_idx| {
        text[..byte_idx].chars().count()
    });
    Ok(Value::Number(match idx {
        Some(i) => i as f64,
        None => -1.0,
    }))
}

fn text_ends_with(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let suffix = expect_text(&args[1])?;
    Ok(Value::Logical(text.ends_with(suffix)))
}

fn expect_text(v: &Value) -> Result<&str, MError> {
    match v {
        Value::Text(s) => Ok(s.as_str()),
        other => Err(type_mismatch("text", other)),
    }
}

// --- List.* ---

fn list_transform(args: &[Value]) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let f = expect_function(&args[1])?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let v = invoke_builtin_callback(f, vec![item.clone()])?;
        out.push(v);
    }
    Ok(Value::List(out))
}

fn list_select(args: &[Value]) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let pred = expect_function(&args[1])?;
    let mut out = Vec::new();
    for item in list {
        let v = invoke_builtin_callback(pred, vec![item.clone()])?;
        match v {
            Value::Logical(true) => out.push(item.clone()),
            Value::Logical(false) => {}
            other => return Err(type_mismatch("logical (from predicate)", &other)),
        }
    }
    Ok(Value::List(out))
}

fn list_sum(args: &[Value]) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut total = 0.0;
    for v in list {
        total += match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
    }
    Ok(Value::Number(total))
}

fn list_count(args: &[Value]) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Number(list.len() as f64))
}

fn list_min(args: &[Value]) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut best: Option<f64> = None;
    for v in list {
        let n = match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
        best = Some(match best {
            None => n,
            Some(curr) => if n < curr { n } else { curr },
        });
    }
    Ok(Value::Number(best.unwrap()))
}

fn list_max(args: &[Value]) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut best: Option<f64> = None;
    for v in list {
        let n = match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
        best = Some(match best {
            None => n,
            Some(curr) => if n > curr { n } else { curr },
        });
    }
    Ok(Value::Number(best.unwrap()))
}

fn expect_list(v: &Value) -> Result<&Vec<Value>, MError> {
    match v {
        Value::List(xs) => Ok(xs),
        other => Err(type_mismatch("list", other)),
    }
}

fn expect_function(v: &Value) -> Result<&Closure, MError> {
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
fn invoke_builtin_callback(closure: &Closure, args: Vec<Value>) -> Result<Value, MError> {
    if args.len() != closure.params.len() {
        return Err(MError::Other(format!(
            "callback: arity mismatch: expected {}, got {}",
            closure.params.len(),
            args.len()
        )));
    }
    match &closure.body {
        FnBody::Builtin(f) => f(&args),
        FnBody::M(body) => {
            let mut call_env = closure.env.clone();
            for (param, value) in closure.params.iter().zip(args.into_iter()) {
                call_env = call_env.extend(param.name.clone(), value);
            }
            super::evaluate(body, &call_env, &super::NoIoHost)
        }
    }
}

// --- Record.* ---

fn record_field(args: &[Value]) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let name = expect_text(&args[1])?;
    match record.fields.iter().find(|(n, _)| n == name) {
        Some((_, v)) => super::force(v.clone(), &mut |e, env| {
            super::evaluate(e, env, &super::NoIoHost)
        }),
        None => Err(MError::Other(format!("Record.Field: field not found: {}", name))),
    }
}

fn record_field_names(args: &[Value]) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let names: Vec<Value> = record
        .fields
        .iter()
        .map(|(n, _)| Value::Text(n.clone()))
        .collect();
    Ok(Value::List(names))
}

// --- Logical.* ---

fn logical_from(args: &[Value]) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Logical(b) => Ok(Value::Logical(*b)),
        Value::Number(n) => Ok(Value::Logical(*n != 0.0)),
        Value::Text(_) => logical_from_text(args),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

fn logical_from_text(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    match text.to_ascii_lowercase().as_str() {
        "true" => Ok(Value::Logical(true)),
        "false" => Ok(Value::Logical(false)),
        _ => Err(MError::Other(format!(
            "Logical.FromText: not a boolean: {:?}",
            text
        ))),
    }
}
