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
            constructor,
        ),
        ("Duration.Days", one("duration"), days),
        ("Duration.Hours", one("duration"), hours),
        ("Duration.Minutes", one("duration"), minutes),
        ("Duration.Seconds", one("duration"), seconds),
        ("Duration.TotalDays", one("duration"), total_days),
        ("Duration.TotalHours", one("duration"), total_hours),
        ("Duration.TotalMinutes", one("duration"), total_minutes),
        ("Duration.TotalSeconds", one("duration"), total_seconds),
        ("Duration.From", one("value"), from),
        ("Duration.FromText", one("text"), from_text),
        ("Duration.ToRecord", one("duration"), to_record),
        (
            "Duration.ToText",
            vec![
                Param { name: "duration".into(), optional: false, type_annotation: None },
                Param { name: "format".into(),   optional: true,  type_annotation: None },
            ],
            to_text,
        ),
    ]
}

fn constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.Days")?;
    Ok(Value::Number(d.num_days() as f64))
}


fn hours(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.Hours")?;
    let total_secs = d.num_seconds();
    let sign = if total_secs < 0 { -1 } else { 1 };
    let abs = total_secs.abs();
    let h = (abs % 86400) / 3600;
    Ok(Value::Number((sign * h) as f64))
}


fn minutes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.Minutes")?;
    let total_secs = d.num_seconds();
    let sign = if total_secs < 0 { -1 } else { 1 };
    let abs = total_secs.abs();
    let m = (abs % 3600) / 60;
    Ok(Value::Number((sign * m) as f64))
}


fn seconds(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.Seconds")?;
    let total_secs = d.num_seconds();
    let sign = if total_secs < 0 { -1 } else { 1 };
    let abs = total_secs.abs();
    let s = abs % 60;
    Ok(Value::Number((sign * s) as f64))
}


fn total_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.TotalDays")?;
    Ok(Value::Number(d.num_seconds() as f64 / 86400.0))
}


fn total_hours(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.TotalHours")?;
    Ok(Value::Number(d.num_seconds() as f64 / 3600.0))
}


fn total_minutes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.TotalMinutes")?;
    Ok(Value::Number(d.num_seconds() as f64 / 60.0))
}


fn total_seconds(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.TotalSeconds")?;
    Ok(Value::Number(d.num_seconds() as f64))
}


fn from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Duration(d) => Ok(Value::Duration(*d)),
        Value::Number(n) => {
            // PQ treats Number as number of days.
            Ok(Value::Duration(chrono::Duration::seconds((n * 86400.0) as i64)))
        }
        Value::Text(_) => from_text(args, host),
        other => Err(type_mismatch("text/number/duration/null", other)),
    }
}

/// Parse PQ duration text: "[d.]hh:mm:ss[.fff]" or "hh:mm:ss" or
/// ISO-8601 "P[nD][T[nH][nM][nS]]".
fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?.trim();
    let (negative, body) = if let Some(rest) = text.strip_prefix('-') {
        (true, rest)
    } else {
        (false, text)
    };
    // ISO-8601 form. PQ accepts "P1D", "P1DT2H30M", "PT5M", etc.
    if let Some(iso_body) = body.strip_prefix('P') {
        return parse_iso8601_duration(iso_body, negative);
    }
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
        return Err(MError::Other(format!("Duration.FromText: expected hh:mm:ss[.fff], got {text:?}")));
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
                "Duration.FromText: bad seconds {a:?}")))?, b)
        }
        None => (s_full.parse::<i64>().map_err(|_| MError::Other(format!(
            "Duration.FromText: bad seconds {s_full:?}")))?, ""),
    };
    let mut total = days * 86400 + h * 3600 + m * 60 + s;
    if negative { total = -total; }
    Ok(Value::Duration(chrono::Duration::seconds(total)))
}

/// Parse ISO-8601 duration body (after the leading `P`).
/// Supports nD, T-section with nH/nM/nS. Fractional seconds permitted.
fn parse_iso8601_duration(body: &str, negative: bool) -> Result<Value, MError> {
    let mut total_secs: i64 = 0;
    let (date_part, time_part) = match body.find('T') {
        Some(i) => (&body[..i], &body[i + 1..]),
        None => (body, ""),
    };
    // Date part: only D is supported (M/Y vary in length so PQ doesn't
    // round-trip them through Duration anyway).
    if !date_part.is_empty() {
        let n_end = date_part.len() - 1;
        let last = date_part.as_bytes()[n_end];
        if last != b'D' {
            return Err(MError::Other(format!(
                "Duration.FromText: unsupported ISO-8601 date unit in {body:?}"
            )));
        }
        let days: i64 = date_part[..n_end].parse().map_err(|_| MError::Other(
            format!("Duration.FromText: bad days in {body:?}"),
        ))?;
        total_secs += days * 86400;
    }
    // Time part: H, M, S terminated tokens.
    if !time_part.is_empty() {
        let mut num_start = 0usize;
        let bytes = time_part.as_bytes();
        for i in 0..bytes.len() {
            let c = bytes[i];
            if matches!(c, b'H' | b'M' | b'S') {
                let tok = &time_part[num_start..i];
                let secs = match c {
                    b'H' => tok.parse::<i64>().map_err(|_| MError::Other(
                        format!("Duration.FromText: bad hours in {body:?}"),
                    ))? * 3600,
                    b'M' => tok.parse::<i64>().map_err(|_| MError::Other(
                        format!("Duration.FromText: bad minutes in {body:?}"),
                    ))? * 60,
                    b'S' => {
                        // Allow fractional seconds — truncate to whole seconds.
                        if let Some(dot) = tok.find('.') {
                            tok[..dot].parse::<i64>().map_err(|_| MError::Other(
                                format!("Duration.FromText: bad seconds in {body:?}"),
                            ))?
                        } else {
                            tok.parse::<i64>().map_err(|_| MError::Other(
                                format!("Duration.FromText: bad seconds in {body:?}"),
                            ))?
                        }
                    }
                    _ => unreachable!(),
                };
                total_secs += secs;
                num_start = i + 1;
            }
        }
        if num_start != bytes.len() {
            return Err(MError::Other(format!(
                "Duration.FromText: trailing unterminated token in {body:?}"
            )));
        }
    }
    if negative { total_secs = -total_secs; }
    Ok(Value::Duration(chrono::Duration::seconds(total_secs)))
}


fn to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let d = extract_duration(&args[0], "Duration.ToText")?;
    let general = match args.get(1) {
        None | Some(Value::Null) => true,
        Some(Value::Text(fmt)) => {
            let trimmed = fmt.trim();
            matches!(trimmed, "G" | "g") || trimmed.is_empty()
        }
        Some(other) => return Err(type_mismatch("text (format)", other)),
    };
    if !general {
        let Some(Value::Text(fmt)) = args.get(1) else { unreachable!() };
        return Err(MError::Other(format!(
            "Duration.ToText: format string {fmt:?} not yet supported (only G / no-format)"
        )));
    }
    let _ = args.get(2);
    let total_secs = d.num_seconds();
    let sign_str = if total_secs < 0 { "-" } else { "" };
    let abs = total_secs.abs();
    let days = abs / 86400;
    let h = (abs % 86400) / 3600;
    let m = (abs % 3600) / 60;
    let s = abs % 60;
    let body = if days != 0 {
        format!("{sign_str}{days}.{h:02}:{m:02}:{s:02}")
    } else {
        format!("{sign_str}{h:02}:{m:02}:{s:02}")
    };
    Ok(Value::Text(body))
}

