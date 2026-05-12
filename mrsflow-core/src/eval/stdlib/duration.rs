//! `Duration.*` stdlib bindings.

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
        ("Duration.Days", one("duration"), duration_days),
        ("Duration.Hours", one("duration"), duration_hours),
        ("Duration.Minutes", one("duration"), duration_minutes),
        ("Duration.Seconds", one("duration"), duration_seconds),
        ("Duration.TotalDays", one("duration"), duration_total_days),
        ("Duration.TotalHours", one("duration"), duration_total_hours),
        ("Duration.TotalMinutes", one("duration"), duration_total_minutes),
        ("Duration.TotalSeconds", one("duration"), duration_total_seconds),
        ("Duration.From", one("value"), duration_from),
        ("Duration.FromText", one("text"), duration_from_text),
        ("Duration.ToRecord", one("duration"), duration_to_record),
        (
            "Duration.ToText",
            vec![
                Param { name: "duration".into(), optional: false, type_annotation: None },
                Param { name: "format".into(),   optional: true,  type_annotation: None },
            ],
            duration_to_text,
        ),
    ]
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


fn extract_duration(v: &Value, ctx: &str) -> Result<chrono::Duration, MError> {
    match v {
        Value::Duration(d) => Ok(*d),
        other => Err(MError::Other(format!(
            "{}: argument must be a duration (got {})",
            ctx, super::super::type_name(other)
        ))),
    }
}


fn duration_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.Days")?;
    Ok(Value::Number(d.num_days() as f64))
}


fn duration_hours(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.Hours")?;
    let total_secs = d.num_seconds();
    let sign = if total_secs < 0 { -1 } else { 1 };
    let abs = total_secs.abs();
    let h = (abs % 86400) / 3600;
    Ok(Value::Number((sign * h) as f64))
}


fn duration_minutes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.Minutes")?;
    let total_secs = d.num_seconds();
    let sign = if total_secs < 0 { -1 } else { 1 };
    let abs = total_secs.abs();
    let m = (abs % 3600) / 60;
    Ok(Value::Number((sign * m) as f64))
}


fn duration_seconds(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.Seconds")?;
    let total_secs = d.num_seconds();
    let sign = if total_secs < 0 { -1 } else { 1 };
    let abs = total_secs.abs();
    let s = abs % 60;
    Ok(Value::Number((sign * s) as f64))
}


fn duration_total_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.TotalDays")?;
    Ok(Value::Number(d.num_seconds() as f64 / 86400.0))
}


fn duration_total_hours(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.TotalHours")?;
    Ok(Value::Number(d.num_seconds() as f64 / 3600.0))
}


fn duration_total_minutes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.TotalMinutes")?;
    Ok(Value::Number(d.num_seconds() as f64 / 60.0))
}


fn duration_total_seconds(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.TotalSeconds")?;
    Ok(Value::Number(d.num_seconds() as f64))
}


fn duration_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Duration(d) => Ok(Value::Duration(*d)),
        Value::Number(n) => {
            // PQ treats Number as number of days.
            Ok(Value::Duration(chrono::Duration::seconds((n * 86400.0) as i64)))
        }
        Value::Text(_) => duration_from_text(args, host),
        other => Err(type_mismatch("text/number/duration/null", other)),
    }
}

/// Parse PQ duration text: "[d.]hh:mm:ss[.fff]" or just "hh:mm:ss".

fn duration_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?.trim();
    let (negative, body) = if let Some(rest) = text.strip_prefix('-') {
        (true, rest)
    } else {
        (false, text)
    };
    // Split off optional days prefix.
    let (days, time_part) = if let Some(dot) = body.find('.') {
        // Could be "d.HH:MM:SS" or "HH:MM:SS.fff".
        let head = &body[..dot];
        let tail = &body[dot + 1..];
        if head.chars().all(|c| c.is_ascii_digit()) && tail.contains(':') {
            (head.parse::<i64>().unwrap_or(0), tail)
        } else {
            (0, body)
        }
    } else {
        (0, body)
    };
    let parts: Vec<&str> = time_part.split(':').collect();
    if parts.len() != 3 {
        return Err(MError::Other(format!("Duration.FromText: expected hh:mm:ss[.fff], got {:?}", text)));
    }
    let h: i64 = parts[0].parse().map_err(|_| MError::Other(format!(
        "Duration.FromText: bad hours {:?}", parts[0])))?;
    let m: i64 = parts[1].parse().map_err(|_| MError::Other(format!(
        "Duration.FromText: bad minutes {:?}", parts[1])))?;
    let s_full = parts[2];
    let (s, _frac) = match s_full.find('.') {
        Some(i) => {
            let (a, b) = s_full.split_at(i);
            (a.parse::<i64>().map_err(|_| MError::Other(format!(
                "Duration.FromText: bad seconds {:?}", a)))?, b)
        }
        None => (s_full.parse::<i64>().map_err(|_| MError::Other(format!(
            "Duration.FromText: bad seconds {:?}", s_full)))?, ""),
    };
    let mut total = days * 86400 + h * 3600 + m * 60 + s;
    if negative { total = -total; }
    Ok(Value::Duration(chrono::Duration::seconds(total)))
}


fn duration_to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.ToRecord")?;
    let total_secs = d.num_seconds();
    let sign = if total_secs < 0 { -1 } else { 1 };
    let abs = total_secs.abs();
    let days = abs / 86400;
    let h = (abs % 86400) / 3600;
    let m = (abs % 3600) / 60;
    let s = abs % 60;
    Ok(Value::Record(Record {
        fields: vec![
            ("Days".into(),    Value::Number((sign * days) as f64)),
            ("Hours".into(),   Value::Number((sign * h) as f64)),
            ("Minutes".into(), Value::Number((sign * m) as f64)),
            ("Seconds".into(), Value::Number((sign * s) as f64)),
        ],
        env: super::super::env::EnvNode::empty(),
    }))
}


fn duration_to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.ToText")?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("Duration.ToText: format string not yet supported"));
    }
    let total_secs = d.num_seconds();
    let sign_str = if total_secs < 0 { "-" } else { "" };
    let abs = total_secs.abs();
    let days = abs / 86400;
    let h = (abs % 86400) / 3600;
    let m = (abs % 3600) / 60;
    let s = abs % 60;
    let body = if days != 0 {
        format!("{}{}.{:02}:{:02}:{:02}", sign_str, days, h, m, s)
    } else {
        format!("{}{:02}:{:02}:{:02}", sign_str, h, m, s)
    };
    Ok(Value::Text(body))
}

