//! `Time.*` stdlib bindings.

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
            "#time",
            vec![
                Param { name: "hour".into(),   optional: false, type_annotation: None },
                Param { name: "minute".into(), optional: false, type_annotation: None },
                Param { name: "second".into(), optional: false, type_annotation: None },
            ],
            constructor,
        ),
        ("Time.Hour", one("time"), hour),
        ("Time.Minute", one("time"), minute),
        ("Time.Second", one("time"), second),
        ("Time.From", one("value"), from),
        (
            "Time.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            from_text,
        ),
        (
            "Time.ToText",
            vec![
                Param { name: "time".into(),   optional: false, type_annotation: None },
                Param { name: "format".into(), optional: true,  type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            to_text,
        ),
        ("Time.ToRecord", one("time"), to_record),
        ("Time.StartOfHour", one("time"), start_of_hour),
        ("Time.EndOfHour", one("time"), end_of_hour),
    ]
}

fn constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let h = expect_int(&args[0], "#time: hour")?;
    let m = expect_int(&args[1], "#time: minute")?;
    let s_raw = &args[2];
    let (sec, nano) = match s_raw {
        Value::Number(n) => {
            if !(0.0..60.0).contains(n) {
                return Err(MError::Other(format!("#time: second out of range: {n}")));
            }
            let whole = n.trunc() as u32;
            let frac_nano = ((n - n.trunc()) * 1_000_000_000.0).round() as u32;
            (whole, frac_nano)
        }
        other => return Err(type_mismatch("number (second)", other)),
    };
    if !(0..24).contains(&h) || !(0..60).contains(&m) {
        return Err(MError::Other(format!("#time: out of range h={h} m={m}")));
    }
    let t = chrono::NaiveTime::from_hms_nano_opt(h as u32, m as u32, sec, nano)
        .ok_or_else(|| MError::Other("#time: invalid time".into()))?;
    Ok(Value::Time(t))
}

fn extract_time(v: &Value, ctx: &str) -> Result<chrono::NaiveTime, MError> {
    match v {
        Value::Time(t) => Ok(*t),
        Value::Datetime(dt) => Ok(dt.time()),
        other => Err(MError::Other(format!(
            "{}: argument must be a time or datetime (got {})",
            ctx, super::super::type_name(other)
        ))),
    }
}


fn hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    Ok(Value::Number(extract_time(&args[0], "Time.Hour")?.hour() as f64))
}


fn minute(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    Ok(Value::Number(extract_time(&args[0], "Time.Minute")?.minute() as f64))
}


fn second(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    Ok(Value::Number(extract_time(&args[0], "Time.Second")?.second() as f64))
}


fn from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Time(t) => Ok(Value::Time(*t)),
        Value::Datetime(dt) => Ok(Value::Time(dt.time())),
        Value::Text(_) => from_text(args, host),
        Value::Number(n) => {
            // PQ: fractional day → time (0.5 = 12:00:00, 0.75 = 18:00:00).
            let frac = n.rem_euclid(1.0);
            let total_nanos = (frac * 86_400.0 * 1_000_000_000.0).round() as u64;
            let secs = (total_nanos / 1_000_000_000) as u32;
            let nanos = (total_nanos % 1_000_000_000) as u32;
            let t = chrono::NaiveTime::from_num_seconds_from_midnight_opt(secs, nanos)
                .ok_or_else(|| MError::Other("Time.From: invalid fraction".into()))?;
            Ok(Value::Time(t))
        }
        other => Err(type_mismatch("text/time/datetime/number/null", other)),
    }
}


fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    for fmt in &["%H:%M:%S%.f", "%H:%M:%S", "%H:%M"] {
        if let Ok(t) = chrono::NaiveTime::parse_from_str(text, fmt) {
            return Ok(Value::Time(t));
        }
    }
    Err(MError::Other(format!("Time.FromText: cannot parse {text:?}")))
}


fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let t = extract_time(&args[0], "Time.ToText")?;
    let fmt = match args.get(1) {
        None | Some(Value::Null) => None,
        Some(Value::Text(s)) => {
            let trimmed = s.trim();
            if trimmed.is_empty() { None } else { Some(s.clone()) }
        }
        Some(other) => return Err(type_mismatch("text (format)", other)),
    };
    let culture = match args.get(2) {
        Some(Value::Text(c)) => Some(c.clone()),
        _ => None,
    };
    let pattern = match fmt {
        // PQ Time.ToText default omits seconds — "14:30:45" → "14:30".
        None => return Ok(Value::Text(t.format("%H:%M").to_string())),
        Some(f) => {
            let expanded = super::date::expand_standard_date_format(&f, culture.as_deref());
            super::date::dotnet_to_strftime(&expanded)
        }
    };
    Ok(Value::Text(t.format(&pattern).to_string()))
}


fn to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let t = extract_time(&args[0], "Time.ToRecord")?;
    Ok(Value::Record(Record {
        fields: vec![
            ("Hour".into(),   Value::Number(t.hour() as f64)),
            ("Minute".into(), Value::Number(t.minute() as f64)),
            ("Second".into(), Value::Number(t.second() as f64)),
        ],
        env: super::super::env::EnvNode::empty(),
    }))
}


fn start_of_hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Time(t) => Ok(Value::Time(
            chrono::NaiveTime::from_hms_opt(t.hour(), 0, 0).unwrap()
        )),
        Value::Datetime(dt) => {
            let new_time = chrono::NaiveTime::from_hms_opt(dt.time().hour(), 0, 0).unwrap();
            Ok(Value::Datetime(dt.date().and_time(new_time)))
        }
        other => Err(type_mismatch("time or datetime", other)),
    }
}


fn end_of_hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Time(t) => Ok(Value::Time(
            chrono::NaiveTime::from_hms_nano_opt(t.hour(), 59, 59, 999_999_999).unwrap()
        )),
        Value::Datetime(dt) => {
            let new_time = chrono::NaiveTime::from_hms_nano_opt(dt.time().hour(), 59, 59, 999_999_999).unwrap();
            Ok(Value::Datetime(dt.date().and_time(new_time)))
        }
        other => Err(type_mismatch("time or datetime", other)),
    }
}

