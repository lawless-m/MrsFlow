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

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Date32Array, DurationMicrosecondArray, Float64Array,
    NullArray, StringArray, TimestampMicrosecondArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::parser::Param;

use super::env::{Env, EnvNode, EnvOps};
use super::iohost::IoHost;
use super::value::{BuiltinFn, Closure, FnBody, MError, Record, Table, Value};

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
        (
            "Number.Round",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round,
        ),
        ("Number.Abs", one("number"), number_abs),
        ("Number.Sign", one("number"), number_sign),
        ("Number.Power", two("base", "exponent"), number_power),
        ("Number.Sqrt", one("number"), number_sqrt),
        ("Text.From", one("value"), text_from),
        ("Text.Contains", two("text", "substring"), text_contains),
        ("Text.Replace", three("text", "old", "new"), text_replace),
        ("Text.Trim", one("text"), text_trim),
        ("Text.Lower", one("text"), text_lower),
        ("Text.Upper", one("text"), text_upper),
        ("Text.Length", one("text"), text_length),
        ("Text.PositionOf", two("text", "substring"), text_position_of),
        ("Text.EndsWith", two("text", "suffix"), text_ends_with),
        ("Text.StartsWith", two("text", "prefix"), text_starts_with),
        ("Text.TrimEnd", one("text"), text_trim_end),
        ("Text.Start", two("text", "count"), text_start),
        (
            "Text.Middle",
            vec![
                Param { name: "text".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            text_middle,
        ),
        ("Text.End", two("text", "count"), text_end),
        ("Text.Split", two("text", "separator"), text_split),
        (
            "Text.Combine",
            vec![
                Param { name: "texts".into(),     optional: false, type_annotation: None },
                Param { name: "separator".into(), optional: true,  type_annotation: None },
            ],
            text_combine,
        ),
        ("List.Transform", two("list", "transform"), list_transform),
        ("List.Select", two("list", "selection"), list_select),
        ("List.Sum", one("list"), list_sum),
        ("List.Count", one("list"), list_count),
        ("List.Min", one("list"), list_min),
        ("List.Max", one("list"), list_max),
        ("List.Combine", one("lists"), list_combine),
        ("Record.Field", two("record", "field"), record_field),
        ("Record.FieldNames", one("record"), record_field_names),
        ("Logical.From", one("value"), logical_from),
        ("Logical.FromText", one("text"), logical_from_text),
        ("#table", two("columns", "rows"), table_constructor),
        ("Table.ColumnNames", one("table"), table_column_names),
        ("Table.RenameColumns", two("table", "renames"), table_rename_columns),
        ("Table.RemoveColumns", two("table", "names"), table_remove_columns),
        (
            "#date",
            three("year", "month", "day"),
            date_constructor,
        ),
        (
            "#datetime",
            vec![
                Param { name: "year".into(),   optional: false, type_annotation: None },
                Param { name: "month".into(),  optional: false, type_annotation: None },
                Param { name: "day".into(),    optional: false, type_annotation: None },
                Param { name: "hour".into(),   optional: false, type_annotation: None },
                Param { name: "minute".into(), optional: false, type_annotation: None },
                Param { name: "second".into(), optional: false, type_annotation: None },
            ],
            datetime_constructor,
        ),
        (
            "#duration",
            vec![
                Param { name: "days".into(),    optional: false, type_annotation: None },
                Param { name: "hours".into(),   optional: false, type_annotation: None },
                Param { name: "minutes".into(), optional: false, type_annotation: None },
                Param { name: "seconds".into(), optional: false, type_annotation: None },
            ],
            duration_constructor,
        ),
        ("Parquet.Document", one("path"), parquet_document),
        ("Table.SelectColumns", two("table", "names"), table_select_columns),
        ("Table.SelectRows", two("table", "predicate"), table_select_rows),
        (
            "Table.AddColumn",
            vec![
                Param { name: "table".into(),     optional: false, type_annotation: None },
                Param { name: "name".into(),      optional: false, type_annotation: None },
                Param { name: "transform".into(), optional: false, type_annotation: None },
                Param { name: "type".into(),      optional: true,  type_annotation: None },
            ],
            table_add_column,
        ),
        (
            "List.Accumulate",
            three("list", "seed", "accumulator"),
            list_accumulate,
        ),
        ("Table.FromRows", two("rows", "columns"), table_from_rows),
        ("Table.PromoteHeaders", one("table"), table_promote_headers),
        (
            "Table.TransformColumnTypes",
            two("table", "transforms"),
            table_transform_column_types,
        ),
        (
            "Table.TransformColumns",
            two("table", "transforms"),
            table_transform_columns,
        ),
        ("Table.Combine", one("tables"), table_combine),
        ("Table.TransformRows", two("table", "transform"), table_transform_rows),
        ("Table.InsertRows", three("table", "offset", "rows"), table_insert_rows),
        ("Date.FromText", one("text"), date_from_text),
        ("Date.From", one("value"), date_from),
        ("Date.Year", one("date"), date_year),
        ("Date.Month", one("date"), date_month),
        ("Date.Day", one("date"), date_day),
        (
            "Date.ToText",
            vec![
                Param { name: "date".into(),   optional: false, type_annotation: None },
                Param { name: "format".into(), optional: true,  type_annotation: None },
            ],
            date_to_text,
        ),
        ("Odbc.Query", two("connection", "sql"), odbc_query),
    ]
}

fn type_mismatch(expected: &'static str, found: &Value) -> MError {
    MError::TypeMismatch {
        expected,
        found: super::type_name(found),
    }
}

// --- Number.* ---

fn number_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn number_abs(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(n.abs())),
        other => Err(type_mismatch("number", other)),
    }
}

fn number_sign(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(if *n > 0.0 {
            1.0
        } else if *n < 0.0 {
            -1.0
        } else {
            0.0
        })),
        other => Err(type_mismatch("number", other)),
    }
}

fn number_power(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let base = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let exp = match &args[1] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    Ok(Value::Number(base.powf(exp)))
}

fn number_sqrt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(n.sqrt())),
        other => Err(type_mismatch("number", other)),
    }
}

fn number_round(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let digits = match args.get(1) {
        Some(Value::Number(d)) => *d as i32,
        Some(Value::Null) | None => 0,
        Some(other) => return Err(type_mismatch("number or null", other)),
    };
    // Simple half-away-from-zero. M's default is banker's, but the corpus
    // only relies on basic rounding for display.
    let factor = 10f64.powi(digits);
    Ok(Value::Number((n * factor).round() / factor))
}

// --- Text.* ---

fn text_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn text_contains(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    Ok(Value::Logical(text.contains(sub)))
}

fn text_replace(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    Ok(Value::Text(text.replace(old, new)))
}

fn text_trim(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.trim().to_string()))
}

fn text_lower(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.to_lowercase()))
}

fn text_upper(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.to_uppercase()))
}

fn text_length(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // M counts characters, not bytes — use char count.
    Ok(Value::Number(text.chars().count() as f64))
}

fn text_position_of(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn text_ends_with(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let suffix = expect_text(&args[1])?;
    Ok(Value::Logical(text.ends_with(suffix)))
}

fn text_starts_with(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let prefix = expect_text(&args[1])?;
    Ok(Value::Logical(text.starts_with(prefix)))
}

fn text_trim_end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.trim_end().to_string()))
}

fn text_combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let texts = expect_list(&args[0])?;
    let sep = match args.get(1) {
        Some(Value::Text(s)) => s.as_str(),
        Some(Value::Null) | None => "",
        Some(other) => return Err(type_mismatch("text or null", other)),
    };
    let parts: Result<Vec<&str>, MError> = texts
        .iter()
        .map(|v| match v {
            Value::Text(s) => Ok(s.as_str()),
            other => Err(type_mismatch("text (in list)", other)),
        })
        .collect();
    Ok(Value::Text(parts?.join(sep)))
}

fn text_start(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if count <= 0 {
        return Ok(Value::Text(String::new()));
    }
    Ok(Value::Text(text.chars().take(count as usize).collect()))
}

fn text_end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if count <= 0 {
        return Ok(Value::Text(String::new()));
    }
    let total = text.chars().count();
    let skip = total.saturating_sub(count as usize);
    Ok(Value::Text(text.chars().skip(skip).collect()))
}

fn text_middle(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if offset < 0 {
        return Ok(Value::Text(String::new()));
    }
    // Optional 3rd arg: count. Null/missing → take rest of string.
    let count = match args.get(2) {
        Some(Value::Number(n)) => Some(*n as isize),
        Some(Value::Null) | None => None,
        Some(other) => return Err(type_mismatch("number or null", other)),
    };
    let mut iter = text.chars().skip(offset as usize);
    let result: String = match count {
        Some(c) if c <= 0 => String::new(),
        Some(c) => iter.by_ref().take(c as usize).collect(),
        None => iter.collect(),
    };
    Ok(Value::Text(result))
}

fn text_split(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sep = expect_text(&args[1])?;
    // Power Query Text.Split on empty separator returns a list of single-char
    // texts; we emulate that to be on the safe side.
    let parts: Vec<Value> = if sep.is_empty() {
        text.chars().map(|c| Value::Text(c.to_string())).collect()
    } else {
        text.split(sep).map(|s| Value::Text(s.to_string())).collect()
    };
    Ok(Value::List(parts))
}

fn expect_text(v: &Value) -> Result<&str, MError> {
    match v {
        Value::Text(s) => Ok(s.as_str()),
        other => Err(type_mismatch("text", other)),
    }
}

// --- List.* ---

fn list_transform(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let f = expect_function(&args[1])?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let v = invoke_builtin_callback(f, vec![item.clone()])?;
        out.push(v);
    }
    Ok(Value::List(out))
}

fn list_select(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn list_sum(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn list_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Number(list.len() as f64))
}

fn list_min(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn list_max(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn list_combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lists = expect_list(&args[0])?;
    let mut out: Vec<Value> = Vec::new();
    for v in lists {
        match v {
            Value::List(xs) => out.extend(xs.iter().cloned()),
            other => return Err(type_mismatch("list (in list)", other)),
        }
    }
    Ok(Value::List(out))
}

fn list_accumulate(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut acc = args[1].clone();
    let f = expect_function(&args[2])?;
    for item in list {
        acc = invoke_callback_with_host(f, vec![acc, item.clone()], host)?;
    }
    Ok(acc)
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
    // Callbacks from List.Transform/Select can't reach the original host
    // — pass NoIoHost so IO-using callbacks fail loudly rather than picking
    // up some unrelated environment. If a future stdlib function needs to
    // thread the real host through callbacks, refactor this signature.
    let host = super::NoIoHost;
    match &closure.body {
        FnBody::Builtin(f) => f(&args, &host),
        FnBody::M(body) => {
            let mut call_env = closure.env.clone();
            for (param, value) in closure.params.iter().zip(args.into_iter()) {
                call_env = call_env.extend(param.name.clone(), value);
            }
            super::evaluate(body, &call_env, &host)
        }
    }
}

// --- Record.* ---

fn record_field(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn record_field_names(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn logical_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Logical(b) => Ok(Value::Logical(*b)),
        Value::Number(n) => Ok(Value::Logical(*n != 0.0)),
        Value::Text(_) => logical_from_text(args, host),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

fn logical_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


// --- Table.* (eval-7a) ---
//
// #table(columns, rows) and the three top-corpus Table.* operations.
// Compound type expressions in the columns position aren't supported in
// this slice — only a list of text column names. Date/Datetime/Duration/
// Binary cells land in eval-7b alongside chrono.

fn table_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let names = expect_text_list(&args[0], "#table: columns")?;
    let rows = expect_list_of_lists(&args[1], "#table: rows")?;
    for (i, row) in rows.iter().enumerate() {
        if row.len() != names.len() {
            return Err(MError::Other(format!(
                "#table: row {} has {} cells, expected {}",
                i,
                row.len(),
                names.len()
            )));
        }
    }
    let batch = values_to_record_batch(&names, &rows)?;
    Ok(Value::Table(Table { batch }))
}

fn table_column_names(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names: Vec<Value> = table
        .batch
        .schema()
        .fields()
        .iter()
        .map(|f| Value::Text(f.name().clone()))
        .collect();
    Ok(Value::List(names))
}

fn table_rename_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let renames = expect_list(&args[1])?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    for r in renames {
        let inner = match r {
            Value::List(xs) => xs,
            other => {
                return Err(type_mismatch(
                    "list (each rename must be {old, new})",
                    other,
                ));
            }
        };
        if inner.len() != 2 {
            return Err(MError::Other(format!(
                "Table.RenameColumns: each rename must be a 2-element list, got {}",
                inner.len()
            )));
        }
        let old = expect_text(&inner[0])?.to_string();
        let new = expect_text(&inner[1])?.to_string();
        pairs.push((old, new));
    }
    let schema = table.batch.schema();
    let existing: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();
    for (old, _new) in &pairs {
        if !existing.contains(old) {
            return Err(MError::Other(format!(
                "Table.RenameColumns: column not found: {}",
                old
            )));
        }
    }
    let new_fields: Vec<Field> = schema
        .fields()
        .iter()
        .map(|f| {
            let mut name = f.name().clone();
            for (old, new) in &pairs {
                if &name == old {
                    name = new.clone();
                    break;
                }
            }
            Field::new(name, f.data_type().clone(), f.is_nullable())
        })
        .collect();
    let new_schema = Arc::new(Schema::new(new_fields));
    let columns: Vec<ArrayRef> = table.batch.columns().to_vec();
    let new_batch = RecordBatch::try_new(new_schema, columns)
        .map_err(|e| MError::Other(format!("Table.RenameColumns: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

fn table_remove_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = expect_text_list(&args[1], "Table.RemoveColumns: names")?;
    let schema = table.batch.schema();
    let existing: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();
    for n in &names {
        if !existing.contains(n) {
            return Err(MError::Other(format!(
                "Table.RemoveColumns: column not found: {}",
                n
            )));
        }
    }
    let keep_indices: Vec<usize> = (0..existing.len())
        .filter(|&i| !names.contains(&existing[i]))
        .collect();
    let new_fields: Vec<Field> = keep_indices
        .iter()
        .map(|&i| schema.field(i).clone())
        .collect();
    let new_schema = Arc::new(Schema::new(new_fields));
    let new_columns: Vec<ArrayRef> = keep_indices
        .iter()
        .map(|&i| table.batch.column(i).clone())
        .collect();
    let new_batch = RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| MError::Other(format!("Table.RemoveColumns: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

// --- Table helpers ---

fn expect_table(v: &Value) -> Result<&Table, MError> {
    match v {
        Value::Table(t) => Ok(t),
        other => Err(type_mismatch("table", other)),
    }
}

fn expect_text_list(v: &Value, ctx: &str) -> Result<Vec<String>, MError> {
    let xs = expect_list(v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match x {
            Value::Text(s) => out.push(s.clone()),
            other => {
                return Err(MError::Other(format!(
                    "{}: expected list of text, got {}",
                    ctx,
                    super::type_name(other)
                )));
            }
        }
    }
    Ok(out)
}

fn expect_list_of_lists<'a>(v: &'a Value, ctx: &str) -> Result<Vec<Vec<Value>>, MError> {
    let xs = expect_list(v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match x {
            Value::List(inner) => out.push(inner.clone()),
            other => {
                return Err(MError::Other(format!(
                    "{}: expected list of lists, got {}",
                    ctx,
                    super::type_name(other)
                )));
            }
        }
    }
    Ok(out)
}

/// Infer a column type by scanning all cells. Supported variants in this
/// slice: Number → Float64, Text → Utf8, Logical → Boolean, Null. A column
/// of all-null produces an Arrow NullArray. Mixed primitive kinds within
/// one column → MError::Other.
fn values_to_record_batch(
    column_names: &[String],
    rows: &[Vec<Value>],
) -> Result<RecordBatch, MError> {
    let n_rows = rows.len();
    let n_cols = column_names.len();

    // Special case: schema with zero columns isn't constructible via the
    // standard RecordBatch path. Caller still wants a real Table value
    // back, so build an empty-schema batch with the correct row count.
    if n_cols == 0 {
        let schema = Arc::new(Schema::empty());
        let options =
            arrow::record_batch::RecordBatchOptions::new().with_row_count(Some(n_rows));
        return RecordBatch::try_new_with_options(schema, vec![], &options)
            .map_err(|e| MError::Other(format!("#table: empty-cols rebuild failed: {}", e)));
    }

    let mut fields: Vec<Field> = Vec::with_capacity(n_cols);
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(n_cols);
    for col_idx in 0..n_cols {
        let cells: Vec<&Value> = rows.iter().map(|r| &r[col_idx]).collect();
        let (dtype, array) = infer_cells(&cells)?;
        let is_nullable =
            matches!(dtype, DataType::Null) || cells.iter().any(|v| matches!(v, Value::Null));
        fields.push(Field::new(column_names[col_idx].clone(), dtype, is_nullable));
        columns.push(array);
    }
    let schema = Arc::new(Schema::new(fields));
    RecordBatch::try_new(schema, columns)
        .map_err(|e| MError::Other(format!("#table: build failed: {}", e)))
}

/// Infer the Arrow type for one column's cells and build the matching
/// array. Used by both `#table` (per-column scan of rows) and by
/// `Table.AddColumn` (the freshly-computed cells).
pub(crate) fn infer_cells(cells: &[&Value]) -> Result<(DataType, ArrayRef), MError> {
    let n_rows = cells.len();
    // Find first non-null cell to determine column kind.
    let mut kind: Option<&'static str> = None;
    for v in cells {
        match v {
            Value::Null => {}
            Value::Number(_) => {
                kind = Some("number");
                break;
            }
            Value::Text(_) => {
                kind = Some("text");
                break;
            }
            Value::Logical(_) => {
                kind = Some("logical");
                break;
            }
            Value::Date(_) => {
                kind = Some("date");
                break;
            }
            Value::Datetime(_) => {
                kind = Some("datetime");
                break;
            }
            Value::Duration(_) => {
                kind = Some("duration");
                break;
            }
            other => {
                return Err(MError::NotImplemented(match other {
                    Value::Binary(_) => "binary cells (deferred)",
                    _ => "non-primitive cell type (deferred)",
                }));
            }
        }
    }
    match kind {
        None => Ok((
            DataType::Null,
            Arc::new(NullArray::new(n_rows)) as ArrayRef,
        )),
        Some("number") => {
            let values: Vec<Option<f64>> = cells
                .iter()
                .map(|v| match v {
                    Value::Null => Ok(None),
                    Value::Number(n) => Ok(Some(*n)),
                    other => Err(MError::Other(format!(
                        "column: mixed types: number + {}",
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((DataType::Float64, Arc::new(Float64Array::from(values))))
        }
        Some("text") => {
            let values: Vec<Option<String>> = cells
                .iter()
                .map(|v| match v {
                    Value::Null => Ok(None),
                    Value::Text(s) => Ok(Some(s.clone())),
                    other => Err(MError::Other(format!(
                        "column: mixed types: text + {}",
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((DataType::Utf8, Arc::new(StringArray::from(values))))
        }
        Some("logical") => {
            let values: Vec<Option<bool>> = cells
                .iter()
                .map(|v| match v {
                    Value::Null => Ok(None),
                    Value::Logical(b) => Ok(Some(*b)),
                    other => Err(MError::Other(format!(
                        "column: mixed types: logical + {}",
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((DataType::Boolean, Arc::new(BooleanArray::from(values))))
        }
        Some("date") => {
            // Date32 stores days since 1970-01-01.
            let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let values: Vec<Option<i32>> = cells
                .iter()
                .map(|v| match v {
                    Value::Null => Ok(None),
                    Value::Date(d) => Ok(Some(
                        d.signed_duration_since(epoch).num_days() as i32,
                    )),
                    other => Err(MError::Other(format!(
                        "column: mixed types: date + {}",
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((DataType::Date32, Arc::new(Date32Array::from(values))))
        }
        Some("datetime") => {
            // Timestamp(Microsecond, None): i64 microseconds since unix epoch.
            let values: Vec<Option<i64>> = cells
                .iter()
                .map(|v| match v {
                    Value::Null => Ok(None),
                    Value::Datetime(dt) => Ok(Some(dt.and_utc().timestamp_micros())),
                    other => Err(MError::Other(format!(
                        "column: mixed types: datetime + {}",
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((
                DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
                Arc::new(TimestampMicrosecondArray::from(values)),
            ))
        }
        Some("duration") => {
            // Duration(Microsecond): i64 microseconds.
            let values: Vec<Option<i64>> = cells
                .iter()
                .map(|v| match v {
                    Value::Null => Ok(None),
                    Value::Duration(d) => d.num_microseconds().map(Some).ok_or_else(|| {
                        MError::Other(format!("duration overflows i64 microseconds: {:?}", d))
                    }),
                    other => Err(MError::Other(format!(
                        "column: mixed types: duration + {}",
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((
                DataType::Duration(arrow::datatypes::TimeUnit::Microsecond),
                Arc::new(DurationMicrosecondArray::from(values)),
            ))
        }
        _ => unreachable!(),
    }
}

/// Convert a single cell of a RecordBatch back to a Value. Used by
/// value_dump's table printer.
pub fn cell_to_value(batch: &RecordBatch, col: usize, row: usize) -> Result<Value, MError> {
    let array = batch.column(col);
    if array.is_null(row) {
        return Ok(Value::Null);
    }
    match array.data_type() {
        DataType::Float64 => {
            let a = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .expect("Float64");
            Ok(Value::Number(a.value(row)))
        }
        DataType::Utf8 => {
            let a = array.as_any().downcast_ref::<StringArray>().expect("Utf8");
            Ok(Value::Text(a.value(row).to_string()))
        }
        DataType::Boolean => {
            let a = array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .expect("Boolean");
            Ok(Value::Logical(a.value(row)))
        }
        DataType::Null => Ok(Value::Null),
        DataType::Date32 => {
            let a = array
                .as_any()
                .downcast_ref::<Date32Array>()
                .expect("Date32");
            let days = a.value(row);
            let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let d = epoch
                .checked_add_signed(chrono::Duration::days(days as i64))
                .ok_or_else(|| MError::Other(format!("Date32 out of range: {} days", days)))?;
            Ok(Value::Date(d))
        }
        DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None) => {
            let a = array
                .as_any()
                .downcast_ref::<TimestampMicrosecondArray>()
                .expect("TimestampMicrosecond");
            let micros = a.value(row);
            let dt = chrono::DateTime::from_timestamp_micros(micros)
                .ok_or_else(|| MError::Other(format!("Timestamp out of range: {} us", micros)))?
                .naive_utc();
            Ok(Value::Datetime(dt))
        }
        DataType::Duration(arrow::datatypes::TimeUnit::Microsecond) => {
            let a = array
                .as_any()
                .downcast_ref::<DurationMicrosecondArray>()
                .expect("DurationMicrosecond");
            let micros = a.value(row);
            Ok(Value::Duration(chrono::Duration::microseconds(micros)))
        }
        other => Err(MError::NotImplemented(match other {
            DataType::Date64 | DataType::Timestamp(_, _) => {
                "non-microsecond timestamp decode (deferred)"
            }
            _ => "unsupported cell type",
        })),
    }
}

// --- chrono constructors (eval-7b) ---
//
// #date(y,m,d), #datetime(y,m,d,h,m,s), #duration(d,h,m,s). All operands
// must be whole-numbered f64s; non-integer or out-of-range values error.

fn date_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#date: year")?;
    let mo = expect_int(&args[1], "#date: month")?;
    let d = expect_int(&args[2], "#date: day")?;
    chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .map(Value::Date)
        .ok_or_else(|| MError::Other(format!("#date: invalid date {}-{:02}-{:02}", y, mo, d)))
}

fn datetime_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#datetime: year")?;
    let mo = expect_int(&args[1], "#datetime: month")?;
    let d = expect_int(&args[2], "#datetime: day")?;
    let h = expect_int(&args[3], "#datetime: hour")?;
    let mn = expect_int(&args[4], "#datetime: minute")?;
    let s = expect_int(&args[5], "#datetime: second")?;
    let date = chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .ok_or_else(|| MError::Other(format!("#datetime: invalid date {}-{:02}-{:02}", y, mo, d)))?;
    let time = chrono::NaiveTime::from_hms_opt(h as u32, mn as u32, s as u32)
        .ok_or_else(|| MError::Other(format!("#datetime: invalid time {:02}:{:02}:{:02}", h, mn, s)))?;
    Ok(Value::Datetime(chrono::NaiveDateTime::new(date, time)))
}

fn duration_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = expect_int(&args[0], "#duration: days")?;
    let h = expect_int(&args[1], "#duration: hours")?;
    let mn = expect_int(&args[2], "#duration: minutes")?;
    let s = expect_int(&args[3], "#duration: seconds")?;
    let total = d
        .checked_mul(86400)
        .and_then(|x| x.checked_add(h.checked_mul(3600)?))
        .and_then(|x| x.checked_add(mn.checked_mul(60)?))
        .and_then(|x| x.checked_add(s))
        .ok_or_else(|| MError::Other("#duration: overflow".into()))?;
    Ok(Value::Duration(chrono::Duration::seconds(total)))
}

fn expect_int(v: &Value, ctx: &str) -> Result<i64, MError> {
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

fn parquet_document(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let path = expect_text(&args[0])?;
    host.parquet_read(path).map_err(|e| {
        MError::Other(format!("Parquet.Document({:?}): {:?}", path, e))
    })
}

// --- Table.* expansion (eval-7d) ---
//
// Five more Table.* ops by corpus frequency. SelectRows and AddColumn
// invoke an M closure with a row-as-record value, matching the
// `each [ColumnName]` access pattern users write.

fn table_select_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = expect_text_list(&args[1], "Table.SelectColumns: names")?;
    let schema = table.batch.schema();
    let existing: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();
    // Look up each requested name; preserve the requested order.
    let mut indices: Vec<usize> = Vec::with_capacity(names.len());
    for n in &names {
        match existing.iter().position(|e| e == n) {
            Some(i) => indices.push(i),
            None => {
                return Err(MError::Other(format!(
                    "Table.SelectColumns: column not found: {}",
                    n
                )));
            }
        }
    }
    let new_fields: Vec<Field> = indices.iter().map(|&i| schema.field(i).clone()).collect();
    let new_schema = Arc::new(Schema::new(new_fields));
    let new_columns: Vec<ArrayRef> = indices
        .iter()
        .map(|&i| table.batch.column(i).clone())
        .collect();
    let new_batch = RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| MError::Other(format!("Table.SelectColumns: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

fn table_select_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let predicate = expect_function(&args[1])?;
    let n_rows = table.batch.num_rows();
    let mut keep: Vec<u32> = Vec::new();
    for row in 0..n_rows {
        let record = row_to_record(&table.batch, row)?;
        let result = invoke_callback_with_host(predicate, vec![record], host)?;
        match result {
            Value::Logical(true) => keep.push(row as u32),
            Value::Logical(false) => {}
            other => {
                return Err(MError::TypeMismatch {
                    expected: "logical (from row predicate)",
                    found: super::type_name(&other),
                });
            }
        }
    }
    let indices = arrow::array::UInt32Array::from(keep);
    let new_columns: Vec<ArrayRef> = table
        .batch
        .columns()
        .iter()
        .map(|c| {
            arrow::compute::take(c.as_ref(), &indices, None)
                .map_err(|e| MError::Other(format!("Table.SelectRows: take failed: {}", e)))
        })
        .collect::<Result<_, _>>()?;
    let new_batch = RecordBatch::try_new(table.batch.schema(), new_columns)
        .map_err(|e| MError::Other(format!("Table.SelectRows: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

fn table_add_column(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let new_name = expect_text(&args[1])?.to_string();
    let transform = expect_function(&args[2])?;
    let n_rows = table.batch.num_rows();
    let mut new_cells: Vec<Value> = Vec::with_capacity(n_rows);
    for row in 0..n_rows {
        let record = row_to_record(&table.batch, row)?;
        let v = invoke_callback_with_host(transform, vec![record], host)?;
        new_cells.push(v);
    }
    let cell_refs: Vec<&Value> = new_cells.iter().collect();
    let (inferred_dtype, inferred_array) = infer_cells(&cell_refs)?;

    // Optional 4th arg: target column type. If supplied (and not `type any`),
    // cast and use its dtype/nullability for the new field; otherwise use the
    // inferred shape.
    let (dtype, new_array, nullable) = match args.get(3) {
        Some(Value::Type(t)) if !matches!(t, super::value::TypeRep::Any) => {
            let (target_dtype, target_nullable) = type_rep_to_datatype(t)?;
            let cast = arrow::compute::cast(&inferred_array, &target_dtype).map_err(|e| {
                MError::Other(format!(
                    "Table.AddColumn: cast {} to {:?} failed: {}",
                    new_name, target_dtype, e
                ))
            })?;
            (target_dtype, cast, target_nullable)
        }
        Some(Value::Type(_)) | Some(Value::Null) | None => {
            let nullable = matches!(inferred_dtype, DataType::Null)
                || new_cells.iter().any(|v| matches!(v, Value::Null));
            (inferred_dtype, inferred_array, nullable)
        }
        Some(other) => return Err(type_mismatch("type or null", other)),
    };

    let schema = table.batch.schema();
    let mut fields: Vec<Field> = schema.fields().iter().map(|f| (**f).clone()).collect();
    fields.push(Field::new(new_name, dtype, nullable));
    let new_schema = Arc::new(Schema::new(fields));
    let mut new_columns: Vec<ArrayRef> = table.batch.columns().to_vec();
    new_columns.push(new_array);
    let new_batch = RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| MError::Other(format!("Table.AddColumn: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

fn table_from_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Same as #table but with arg order (rows, columns).
    let rows = expect_list_of_lists(&args[0], "Table.FromRows: rows")?;
    let names = expect_text_list(&args[1], "Table.FromRows: columns")?;
    for (i, row) in rows.iter().enumerate() {
        if row.len() != names.len() {
            return Err(MError::Other(format!(
                "Table.FromRows: row {} has {} cells, expected {}",
                i,
                row.len(),
                names.len()
            )));
        }
    }
    let batch = values_to_record_batch(&names, &rows)?;
    Ok(Value::Table(Table { batch }))
}

fn table_promote_headers(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.batch.num_rows() == 0 {
        return Err(MError::Other(
            "Table.PromoteHeaders: table has no header row".into(),
        ));
    }
    // Read row 0 as the new names; every cell must be text.
    let mut new_names: Vec<String> = Vec::with_capacity(table.batch.num_columns());
    for col in 0..table.batch.num_columns() {
        match cell_to_value(&table.batch, col, 0)? {
            Value::Text(s) => new_names.push(s),
            other => {
                return Err(MError::Other(format!(
                    "Table.PromoteHeaders: header cell in column {} is not text: {}",
                    col,
                    super::type_name(&other)
                )));
            }
        }
    }
    // Slice every column from row 1 to end. The column types are preserved
    // as-is from the existing schema — we are NOT re-inferring from data
    // rows. If users want different types after promotion, they call
    // Table.TransformColumnTypes (eval-7e).
    let n_remaining = table.batch.num_rows() - 1;
    let new_columns: Vec<ArrayRef> = table
        .batch
        .columns()
        .iter()
        .map(|c| c.slice(1, n_remaining))
        .collect();
    let schema = table.batch.schema();
    let new_fields: Vec<Field> = schema
        .fields()
        .iter()
        .zip(new_names.iter())
        .map(|(f, n)| Field::new(n.clone(), f.data_type().clone(), f.is_nullable()))
        .collect();
    let new_schema = Arc::new(Schema::new(new_fields));
    let new_batch = RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| MError::Other(format!("Table.PromoteHeaders: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

/// Build a record Value from one row of a RecordBatch — column name → cell.
pub(super) fn row_to_record(batch: &RecordBatch, row: usize) -> Result<Value, MError> {
    let schema = batch.schema();
    let mut fields: Vec<(String, Value)> = Vec::with_capacity(batch.num_columns());
    for col in 0..batch.num_columns() {
        let name = schema.field(col).name().clone();
        let value = cell_to_value(batch, col, row)?;
        fields.push((name, value));
    }
    Ok(Value::Record(Record {
        fields,
        env: EnvNode::empty(),
    }))
}

/// Like `invoke_builtin_callback` but threads the real host through. Used
/// when a Table.* op invokes its callback in a context where the original
/// host should propagate (so an Odbc-using row predicate could in theory
/// work — though none of slice 7d's tests exercise that).
fn invoke_callback_with_host(
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
            super::evaluate(body, &call_env, host)
        }
    }
}

// --- Table.* eval-7e: type-aware ops + concat ---

fn table_transform_column_types(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let transforms = expect_list(&args[1])?;
    // Auto-wrap single `{name, type}` pair to match Power Query leniency.
    let owned: Vec<Value>;
    let transforms: &[Value] = if is_single_col_type_pair(transforms) {
        owned = vec![Value::List(transforms.to_vec())];
        &owned
    } else {
        transforms
    };
    // Parse the {col_name, type_value} pairs first; error early on bad shapes.
    let pairs = parse_col_type_pairs(transforms)?;

    let schema = table.batch.schema();
    let mut new_fields: Vec<Field> = schema.fields().iter().map(|f| (**f).clone()).collect();
    let mut new_columns: Vec<ArrayRef> = table.batch.columns().to_vec();

    for (name, target) in &pairs {
        let idx = schema
            .index_of(name)
            .map_err(|_| MError::Other(format!(
                "Table.TransformColumnTypes: column not found: {}",
                name
            )))?;
        // `type any` → keep current shape (no cast). Power Query's `any` is
        // the no-constraint type; the corpus uses it for mixed-shape columns.
        let Some((target_dtype, target_nullable)) = target else {
            continue;
        };
        let cast = arrow::compute::cast(&new_columns[idx], target_dtype).map_err(|e| {
            MError::Other(format!(
                "Table.TransformColumnTypes: cast {} to {:?} failed: {}",
                name, target_dtype, e
            ))
        })?;
        new_columns[idx] = cast;
        new_fields[idx] = Field::new(name, target_dtype.clone(), *target_nullable);
    }

    let new_schema = Arc::new(Schema::new(new_fields));
    let new_batch = RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| MError::Other(format!("Table.TransformColumnTypes: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

fn parse_col_type_pairs(
    transforms: &[Value],
) -> Result<Vec<(String, Option<(DataType, bool)>)>, MError> {
    let mut out = Vec::with_capacity(transforms.len());
    for t in transforms {
        let inner = match t {
            Value::List(xs) => xs,
            other => {
                return Err(type_mismatch(
                    "list (each transform must be {name, type})",
                    other,
                ));
            }
        };
        if inner.len() != 2 {
            return Err(MError::Other(format!(
                "Table.TransformColumnTypes: each transform must be a 2-element list, got {}",
                inner.len()
            )));
        }
        let name = expect_text(&inner[0])?.to_string();
        let type_value = match &inner[1] {
            Value::Type(t) => t.clone(),
            other => return Err(type_mismatch("type", other)),
        };
        // `type any` → None (no-cast). Anything else must be castable.
        let mapped = if matches!(type_value, super::value::TypeRep::Any) {
            None
        } else {
            Some(type_rep_to_datatype(&type_value)?)
        };
        out.push((name, mapped));
    }
    Ok(out)
}

/// Map a TypeRep to (DataType, nullable). Compound and non-primitive types
/// error — eval-7e supports the primitive set only.
fn type_rep_to_datatype(t: &super::value::TypeRep) -> Result<(DataType, bool), MError> {
    use super::value::TypeRep;
    match t {
        TypeRep::Null => Ok((DataType::Null, true)),
        TypeRep::Logical => Ok((DataType::Boolean, false)),
        TypeRep::Number => Ok((DataType::Float64, false)),
        TypeRep::Text => Ok((DataType::Utf8, false)),
        TypeRep::Date => Ok((DataType::Date32, false)),
        TypeRep::Datetime => Ok((
            DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
            false,
        )),
        TypeRep::Duration => Ok((
            DataType::Duration(arrow::datatypes::TimeUnit::Microsecond),
            false,
        )),
        TypeRep::Nullable(inner) => {
            let (dt, _) = type_rep_to_datatype(inner)?;
            Ok((dt, true))
        }
        TypeRep::Any | TypeRep::AnyNonNull | TypeRep::List | TypeRep::Record
        | TypeRep::Table | TypeRep::Function | TypeRep::Type | TypeRep::Binary => {
            Err(MError::Other(format!(
                "Table.TransformColumnTypes: type {:?} is not a castable primitive",
                t
            )))
        }
    }
}

fn table_transform_columns(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let transforms = expect_list(&args[1])?;
    // Real Power Query accepts both `{name, fn}` (single pair) and
    // `{{name, fn}, ...}` (list of pairs). Auto-wrap the single-pair form.
    let owned: Vec<Value>;
    let transforms: &[Value] = if is_single_col_fn_pair(transforms) {
        owned = vec![Value::List(transforms.to_vec())];
        &owned
    } else {
        transforms
    };
    let pairs = parse_col_fn_pairs(transforms)?;

    let schema = table.batch.schema();
    let mut new_fields: Vec<Field> = schema.fields().iter().map(|f| (**f).clone()).collect();
    let mut new_columns: Vec<ArrayRef> = table.batch.columns().to_vec();
    let n_rows = table.batch.num_rows();

    for (name, closure, type_opt) in &pairs {
        let idx = schema
            .index_of(name)
            .map_err(|_| MError::Other(format!(
                "Table.TransformColumns: column not found: {}",
                name
            )))?;
        let mut new_cells: Vec<Value> = Vec::with_capacity(n_rows);
        for row in 0..n_rows {
            let cell = cell_to_value(&table.batch, idx, row)?;
            let v = invoke_callback_with_host(closure, vec![cell], host)?;
            new_cells.push(v);
        }
        let cell_refs: Vec<&Value> = new_cells.iter().collect();
        let (inferred_dtype, inferred_array) = infer_cells(&cell_refs)?;
        let (dtype, new_array, nullable) = match type_opt {
            Some(t) if !matches!(t, super::value::TypeRep::Any) => {
                let (target_dtype, target_nullable) = type_rep_to_datatype(t)?;
                let cast = arrow::compute::cast(&inferred_array, &target_dtype).map_err(|e| {
                    MError::Other(format!(
                        "Table.TransformColumns: cast {} to {:?} failed: {}",
                        name, target_dtype, e
                    ))
                })?;
                (target_dtype, cast, target_nullable)
            }
            _ => {
                let nullable = matches!(inferred_dtype, DataType::Null)
                    || new_cells.iter().any(|v| matches!(v, Value::Null));
                (inferred_dtype, inferred_array, nullable)
            }
        };
        new_columns[idx] = new_array;
        new_fields[idx] = Field::new(name, dtype, nullable);
    }

    let new_schema = Arc::new(Schema::new(new_fields));
    let new_batch = RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| MError::Other(format!("Table.TransformColumns: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

fn is_single_col_fn_pair(xs: &[Value]) -> bool {
    // Either `{name, fn}` or `{name, fn, type}` as a single transform.
    let head_ok = !xs.is_empty()
        && matches!(xs.first(), Some(Value::Text(_)))
        && matches!(xs.get(1), Some(Value::Function(_)));
    match xs.len() {
        2 => head_ok,
        3 => head_ok && matches!(xs[2], Value::Type(_) | Value::Null),
        _ => false,
    }
}

fn is_single_col_type_pair(xs: &[Value]) -> bool {
    xs.len() == 2 && matches!(xs[0], Value::Text(_)) && matches!(xs[1], Value::Type(_))
}

fn parse_col_fn_pairs<'a>(
    transforms: &'a [Value],
) -> Result<Vec<(String, &'a Closure, Option<super::value::TypeRep>)>, MError> {
    let mut out = Vec::with_capacity(transforms.len());
    for t in transforms {
        let inner = match t {
            Value::List(xs) => xs,
            other => {
                return Err(type_mismatch(
                    "list (each transform must be {name, function} or {name, function, type})",
                    other,
                ));
            }
        };
        if inner.len() != 2 && inner.len() != 3 {
            return Err(MError::Other(format!(
                "Table.TransformColumns: each transform must be 2 or 3 elements, got {}",
                inner.len()
            )));
        }
        let name = expect_text(&inner[0])?.to_string();
        let closure = match &inner[1] {
            Value::Function(c) => c,
            other => return Err(type_mismatch("function", other)),
        };
        let type_opt = if inner.len() == 3 {
            match &inner[2] {
                Value::Type(t) => Some(t.clone()),
                Value::Null => None,
                other => return Err(type_mismatch("type or null", other)),
            }
        } else {
            None
        };
        out.push((name, closure, type_opt));
    }
    Ok(out)
}

fn table_combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let tables = expect_list(&args[0])?;
    if tables.is_empty() {
        return Err(MError::Other("Table.Combine: empty table list".into()));
    }
    let batches: Result<Vec<RecordBatch>, MError> = tables
        .iter()
        .map(|t| match t {
            Value::Table(table) => Ok(table.batch.clone()),
            other => Err(type_mismatch("table (in list)", other)),
        })
        .collect();
    let batches = batches?;
    if batches.len() == 1 {
        return Ok(Value::Table(Table {
            batch: batches.into_iter().next().unwrap(),
        }));
    }
    // First-pass: require identical schemas. arrow::compute::concat_batches
    // enforces this for us, but the error message is generic — wrap it.
    let schema = batches[0].schema();
    for (i, b) in batches.iter().enumerate().skip(1) {
        if b.schema() != schema {
            return Err(MError::Other(format!(
                "Table.Combine: schema of table {} does not match table 0",
                i
            )));
        }
    }
    let combined = arrow::compute::concat_batches(&schema, &batches)
        .map_err(|e| MError::Other(format!("Table.Combine: concat failed: {}", e)))?;
    Ok(Value::Table(Table { batch: combined }))
}

fn table_transform_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let transform = expect_function(&args[1])?;
    let n_rows = table.batch.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n_rows);
    for row in 0..n_rows {
        let record = row_to_record(&table.batch, row)?;
        out.push(invoke_callback_with_host(transform, vec![record], host)?);
    }
    Ok(Value::List(out))
}

fn table_insert_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let new_records = expect_list(&args[2])?;
    let n_existing = table.batch.num_rows();
    if offset > n_existing {
        return Err(MError::Other(format!(
            "Table.InsertRows: offset {} exceeds row count {}",
            offset, n_existing
        )));
    }

    // Column names come from the original schema.
    let names: Vec<String> = table
        .batch
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect();

    // Build the merged row list: existing[..offset], new, existing[offset..].
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(n_existing + new_records.len());
    for row in 0..offset {
        let mut cells = Vec::with_capacity(names.len());
        for col in 0..names.len() {
            cells.push(cell_to_value(&table.batch, col, row)?);
        }
        rows.push(cells);
    }
    for r in new_records {
        let record = match r {
            Value::Record(rec) => rec,
            other => return Err(type_mismatch("record (in rows)", other)),
        };
        let mut cells = Vec::with_capacity(names.len());
        for name in &names {
            let v = record
                .fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .unwrap_or(Value::Null);
            // Record literal fields are thunks per the spec — force before
            // pushing to the Arrow batch builder.
            let v = super::force(v, &mut |e, env| super::evaluate(e, env, host))?;
            cells.push(v);
        }
        rows.push(cells);
    }
    for row in offset..n_existing {
        let mut cells = Vec::with_capacity(names.len());
        for col in 0..names.len() {
            cells.push(cell_to_value(&table.batch, col, row)?);
        }
        rows.push(cells);
    }

    let batch = values_to_record_batch(&names, &rows)?;
    Ok(Value::Table(Table { batch }))
}

fn date_to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Date(d) => *d,
        other => return Err(type_mismatch("date", other)),
    };
    let chrono_fmt = match args.get(1) {
        Some(Value::Null) | None => "%Y-%m-%d",
        Some(Value::Text(s)) => match s.as_str() {
            "yyyy-MM-dd" => "%Y-%m-%d",
            "dd/MM/yyyy" => "%d/%m/%Y",
            "dd-MM-yyyy" => "%d-%m-%Y",
            "MM/dd/yyyy" => "%m/%d/%Y",
            "yyyy/MM/dd" => "%Y/%m/%d",
            other => {
                return Err(MError::Other(format!(
                    "Date.ToText: unsupported format {:?}; supported: yyyy-MM-dd, dd/MM/yyyy, dd-MM-yyyy, MM/dd/yyyy, yyyy/MM/dd",
                    other
                )));
            }
        },
        Some(other) => return Err(type_mismatch("text or null", other)),
    };
    Ok(Value::Text(d.format(chrono_fmt).to_string()))
}

fn date_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Date(*d)),
        Value::Datetime(dt) => Ok(Value::Date(dt.date())),
        Value::Text(_) => date_from_text(args, host),
        other => Err(type_mismatch("date/datetime/text/null", other)),
    }
}

fn date_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.year() as f64)),
        other => Err(type_mismatch("date", other)),
    }
}

fn date_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.month() as f64)),
        other => Err(type_mismatch("date", other)),
    }
}

fn date_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.day() as f64)),
        other => Err(type_mismatch("date", other)),
    }
}

fn date_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // Power Query's Date.FromText is locale-aware. Try ISO first, then a
    // couple of common UK/US forms. Not the full spec — just enough for the
    // corpus.
    for fmt in &["%Y-%m-%d", "%d-%m-%Y", "%m-%d-%Y", "%Y/%m/%d", "%d/%m/%Y"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(text, fmt) {
            return Ok(Value::Date(d));
        }
    }
    Err(MError::Other(format!(
        "Date.FromText: cannot parse {:?}",
        text
    )))
}

// --- ODBC (eval-8) ---
//
// Delegates to the shell's IoHost. CliIoHost (built with `--features odbc`)
// uses odbc-api against an installed driver; NoIoHost and CliIoHost built
// without the feature return a NotSupported-style error. WASM shell will
// likewise return NotSupported when it lands.

fn odbc_query(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let conn = expect_text(&args[0])?;
    let sql = expect_text(&args[1])?;
    host.odbc_query(conn, sql, None)
        .map_err(|e| MError::Other(format!("Odbc.Query: {:?}", e)))
}
