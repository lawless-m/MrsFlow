//! `DateTime.*` stdlib bindings.

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
            "#datetime",
            vec![
                Param { name: "year".into(),   optional: false, type_annotation: None },
                Param { name: "month".into(),  optional: false, type_annotation: None },
                Param { name: "day".into(),    optional: false, type_annotation: None },
                Param { name: "hour".into(),   optional: false, type_annotation: None },
                Param { name: "minute".into(), optional: false, type_annotation: None },
                Param { name: "second".into(), optional: false, type_annotation: None },
            ],
            constructor,
        ),
        ("DateTime.Date", one("datetime"), date),
        ("DateTime.Time", one("datetime"), time),
        ("DateTime.From", one("value"), from),
        (
            "DateTime.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            from_text,
        ),
        (
            "DateTime.ToText",
            vec![
                Param { name: "datetime".into(), optional: false, type_annotation: None },
                Param { name: "format".into(),   optional: true,  type_annotation: None },
                Param { name: "culture".into(),  optional: true,  type_annotation: None },
            ],
            to_text,
        ),
        ("DateTime.ToRecord", one("datetime"), to_record),
        (
            "DateTime.AddZone",
            vec![
                Param { name: "datetime".into(), optional: false, type_annotation: None },
                Param { name: "hours".into(),    optional: false, type_annotation: None },
                Param { name: "minutes".into(),  optional: true,  type_annotation: None },
            ],
            add_zone,
        ),
        ("DateTime.FromFileTime", one("fileTime"), from_file_time),
        ("DateTime.LocalNow", vec![], local_now),
        ("DateTime.FixedLocalNow", vec![], local_now),
        ("DateTime.IsInCurrentHour", one("datetime"), is_in_current_hour),
        ("DateTime.IsInCurrentMinute", one("datetime"), is_in_current_minute),
        ("DateTime.IsInCurrentSecond", one("datetime"), is_in_current_second),
        ("DateTime.IsInNextHour", one("datetime"), is_in_next_hour),
        ("DateTime.IsInNextMinute", one("datetime"), is_in_next_minute),
        ("DateTime.IsInNextSecond", one("datetime"), is_in_next_second),
        ("DateTime.IsInPreviousHour", one("datetime"), is_in_previous_hour),
        ("DateTime.IsInPreviousMinute", one("datetime"), is_in_previous_minute),
        ("DateTime.IsInPreviousSecond", one("datetime"), is_in_previous_second),
        ("DateTime.IsInNextNHours", two("datetime", "n"), is_in_next_n_hours),
        ("DateTime.IsInNextNMinutes", two("datetime", "n"), is_in_next_n_minutes),
        ("DateTime.IsInNextNSeconds", two("datetime", "n"), is_in_next_n_seconds),
        ("DateTime.IsInPreviousNHours", two("datetime", "n"), is_in_previous_n_hours),
        ("DateTime.IsInPreviousNMinutes", two("datetime", "n"), is_in_previous_n_minutes),
        ("DateTime.IsInPreviousNSeconds", two("datetime", "n"), is_in_previous_n_seconds),
    ]
}

fn constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#datetime: year")?;
    let mo = expect_int(&args[1], "#datetime: month")?;
    let d = expect_int(&args[2], "#datetime: day")?;
    let h = expect_int(&args[3], "#datetime: hour")?;
    let mn = expect_int(&args[4], "#datetime: minute")?;
    let s = expect_int(&args[5], "#datetime: second")?;
    let date = chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .ok_or_else(|| MError::Other(format!("#datetime: invalid date {y}-{mo:02}-{d:02}")))?;
    let time = chrono::NaiveTime::from_hms_opt(h as u32, mn as u32, s as u32)
        .ok_or_else(|| MError::Other(format!("#datetime: invalid time {h:02}:{mn:02}:{s:02}")))?;
    Ok(Value::Datetime(chrono::NaiveDateTime::new(date, time)))
}


fn date(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetime(dt) => Ok(Value::Date(dt.date())),
        other => Err(type_mismatch("datetime", other)),
    }
}


fn time(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetime(dt) => Ok(Value::Time(dt.time())),
        other => Err(type_mismatch("datetime", other)),
    }
}


fn from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetime(dt) => Ok(Value::Datetime(*dt)),
        Value::Date(d) => Ok(Value::Datetime(
            d.and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
        )),
        Value::Text(_) => from_text(args, host),
        Value::Number(n) => {
            // PQ treats Number as OLE Automation date (days since 1899-12-30).
            let base = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap()
                .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            let secs = (n * 86400.0) as i64;
            Ok(Value::Datetime(base + chrono::Duration::seconds(secs)))
        }
        other => Err(type_mismatch("text/date/datetime/number/null", other)),
    }
}


fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%d/%m/%Y %H:%M:%S",
        "%d/%m/%Y %H:%M",
        "%m/%d/%Y %H:%M:%S",
    ] {
        if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(text, fmt) {
            return Ok(Value::Datetime(dt));
        }
    }
    // Try parsing as a Date and promoting.
    for fmt in &["%Y-%m-%d", "%d/%m/%Y", "%m/%d/%Y"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(text, fmt) {
            return Ok(Value::Datetime(
                d.and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap())
            ));
        }
    }
    Err(MError::Other(format!("DateTime.FromText: cannot parse {text:?}")))
}


fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let dt = match &args[0] {
        Value::Datetime(dt) => *dt,
        other => return Err(type_mismatch("datetime", other)),
    };
    // Format string (arg 1): "G"/"g" (general, .NET conv) → default ISO
    // shape; no-arg / null behaves the same. Other .NET format specs are
    // not implemented; surface the actual format in the error.
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
            "DateTime.ToText: format string {fmt:?} not yet supported (only G / no-format)"
        )));
    }
    // Culture arg (2) accepted but ignored — en-US shape.
    let _ = args.get(2);
    Ok(Value::Text(dt.format("%Y-%m-%dT%H:%M:%S").to_string()))
}


fn to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::{Datelike, Timelike};
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let dt = match &args[0] {
        Value::Datetime(dt) => *dt,
        other => return Err(type_mismatch("datetime", other)),
    };
    Ok(Value::Record(Record {
        fields: vec![
            ("Year".into(),   Value::Number(dt.year() as f64)),
            ("Month".into(),  Value::Number(dt.month() as f64)),
            ("Day".into(),    Value::Number(dt.day() as f64)),
            ("Hour".into(),   Value::Number(dt.hour() as f64)),
            ("Minute".into(), Value::Number(dt.minute() as f64)),
            ("Second".into(), Value::Number(dt.second() as f64)),
        ],
        env: super::super::env::EnvNode::empty(),
    }))
}


fn add_zone(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Err(MError::NotImplemented(
        "DateTime.AddZone: produces a Datetimezone; pending Datetimezone Value variant",
    ))
}


fn from_file_time(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let ticks = match &args[0] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    // Windows FILETIME: 100ns ticks since 1601-01-01 UTC.
    let base = chrono::NaiveDate::from_ymd_opt(1601, 1, 1).unwrap()
        .and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
    let nanos = (ticks * 100.0) as i64;
    Ok(Value::Datetime(base + chrono::Duration::nanoseconds(nanos)))
}


fn local_now(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Datetime(chrono::Local::now().naive_local()))
}


fn now_naive() -> chrono::NaiveDateTime {
    chrono::Local::now().naive_local()
}


fn extract_naive_datetime(v: &Value, ctx: &str) -> Result<Option<chrono::NaiveDateTime>, MError> {
    match v {
        Value::Null => Ok(None),
        Value::Datetime(dt) => Ok(Some(*dt)),
        Value::Date(d) => Ok(Some(d.and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()))),
        other => Err(MError::Other(format!(
            "{}: argument must be a datetime (got {})",
            ctx, super::super::type_name(other)
        ))),
    }
}


fn dt_in_range(target: chrono::NaiveDateTime, start: chrono::NaiveDateTime, end_exclusive: chrono::NaiveDateTime) -> bool {
    target >= start && target < end_exclusive
}


fn start_of_unit(dt: chrono::NaiveDateTime, unit_secs: i64) -> chrono::NaiveDateTime {
    // Floor to the most recent multiple of `unit_secs` since Unix epoch.
    let total = dt.and_utc().timestamp();
    let floored = total - total.rem_euclid(unit_secs);
    chrono::DateTime::<chrono::Utc>::from_timestamp(floored, 0).unwrap().naive_utc()
}

// ----- IsInCurrent* (single-unit) -----


fn is_in_current_hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInCurrentHour")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 3600);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::hours(1))))
}

fn is_in_current_minute(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInCurrentMinute")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 60);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::minutes(1))))
}

fn is_in_current_second(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInCurrentSecond")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::seconds(1))))
}

// ----- IsInNext* -----


fn is_in_next_hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInNextHour")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 3600) + chrono::Duration::hours(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::hours(1))))
}

fn is_in_next_minute(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInNextMinute")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 60) + chrono::Duration::minutes(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::minutes(1))))
}

fn is_in_next_second(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInNextSecond")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 1) + chrono::Duration::seconds(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::seconds(1))))
}

// ----- IsInPrevious* -----


fn is_in_previous_hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInPreviousHour")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 3600) - chrono::Duration::hours(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::hours(1))))
}

fn is_in_previous_minute(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInPreviousMinute")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 60) - chrono::Duration::minutes(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::minutes(1))))
}

fn is_in_previous_second(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInPreviousSecond")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_unit(now_naive(), 1) - chrono::Duration::seconds(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::seconds(1))))
}

// ----- IsInNextN* / IsInPreviousN* -----


fn is_in_next_n_hours(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInNextNHours")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "DateTime.IsInNextNHours")?;
    let s = start_of_unit(now_naive(), 3600) + chrono::Duration::hours(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::hours(n))))
}

fn is_in_next_n_minutes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInNextNMinutes")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "DateTime.IsInNextNMinutes")?;
    let s = start_of_unit(now_naive(), 60) + chrono::Duration::minutes(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::minutes(n))))
}

fn is_in_next_n_seconds(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInNextNSeconds")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "DateTime.IsInNextNSeconds")?;
    let s = start_of_unit(now_naive(), 1) + chrono::Duration::seconds(1);
    Ok(Value::Logical(dt_in_range(dt, s, s + chrono::Duration::seconds(n))))
}

fn is_in_previous_n_hours(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInPreviousNHours")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "DateTime.IsInPreviousNHours")?;
    let e = start_of_unit(now_naive(), 3600);
    Ok(Value::Logical(dt_in_range(dt, e - chrono::Duration::hours(n), e)))
}

fn is_in_previous_n_minutes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInPreviousNMinutes")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "DateTime.IsInPreviousNMinutes")?;
    let e = start_of_unit(now_naive(), 60);
    Ok(Value::Logical(dt_in_range(dt, e - chrono::Duration::minutes(n), e)))
}

fn is_in_previous_n_seconds(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match extract_naive_datetime(&args[0], "DateTime.IsInPreviousNSeconds")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "DateTime.IsInPreviousNSeconds")?;
    let e = start_of_unit(now_naive(), 1);
    Ok(Value::Logical(dt_in_range(dt, e - chrono::Duration::seconds(n), e)))
}

// ----- DateTimeZone.* -----

