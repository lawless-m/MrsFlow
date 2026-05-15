//! `DateTimeZone.*` stdlib bindings.

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
            "#datetimezone",
            vec![
                Param { name: "year".into(),         optional: false, type_annotation: None },
                Param { name: "month".into(),        optional: false, type_annotation: None },
                Param { name: "day".into(),          optional: false, type_annotation: None },
                Param { name: "hour".into(),         optional: false, type_annotation: None },
                Param { name: "minute".into(),       optional: false, type_annotation: None },
                Param { name: "second".into(),       optional: false, type_annotation: None },
                Param { name: "offsetHours".into(),  optional: false, type_annotation: None },
                Param { name: "offsetMinutes".into(), optional: false, type_annotation: None },
            ],
            constructor,
        ),
        ("DateTimeZone.FixedLocalNow", vec![], local_now),
        ("DateTimeZone.FixedUtcNow", vec![], utc_now),
        ("DateTimeZone.LocalNow", vec![], local_now),
        ("DateTimeZone.UtcNow", vec![], utc_now),
        ("DateTimeZone.From", one("value"), from),
        (
            "DateTimeZone.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            from_text,
        ),
        ("DateTimeZone.FromFileTime", one("fileTime"), from_file_time),
        ("DateTimeZone.RemoveZone", one("dtz"), remove_zone),
        (
            "DateTimeZone.SwitchZone",
            vec![
                Param { name: "dtz".into(),     optional: false, type_annotation: None },
                Param { name: "hours".into(),   optional: false, type_annotation: None },
                Param { name: "minutes".into(), optional: true,  type_annotation: None },
            ],
            switch_zone,
        ),
        ("DateTimeZone.ToLocal", one("dtz"), to_local),
        ("DateTimeZone.ToUtc", one("dtz"), to_utc),
        ("DateTimeZone.ToRecord", one("dtz"), to_record),
        (
            "DateTimeZone.ToText",
            vec![
                Param { name: "dtz".into(),     optional: false, type_annotation: None },
                Param { name: "format".into(),  optional: true,  type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            to_text,
        ),
        ("DateTimeZone.ZoneHours", one("dtz"), zone_hours),
        ("DateTimeZone.ZoneMinutes", one("dtz"), zone_minutes),
    ]
}

fn constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use super::common::expect_int;
    let y = expect_int(&args[0], "#datetimezone: year")?;
    let mo = expect_int(&args[1], "#datetimezone: month")?;
    let d = expect_int(&args[2], "#datetimezone: day")?;
    let h = expect_int(&args[3], "#datetimezone: hour")?;
    let mn = expect_int(&args[4], "#datetimezone: minute")?;
    // Seconds may be fractional.
    let (sec, nano) = match &args[5] {
        Value::Number(n) => {
            let whole = n.trunc() as u32;
            let frac = ((n - n.trunc()) * 1_000_000_000.0).round() as u32;
            (whole, frac)
        }
        other => return Err(type_mismatch("number (second)", other)),
    };
    let oh = expect_int(&args[6], "#datetimezone: offsetHours")?;
    let om = expect_int(&args[7], "#datetimezone: offsetMinutes")?;
    let date = chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .ok_or_else(|| MError::Other(format!("#datetimezone: invalid date {y}-{mo}-{d}")))?;
    let time = chrono::NaiveTime::from_hms_nano_opt(h as u32, mn as u32, sec, nano)
        .ok_or_else(|| MError::Other(format!("#datetimezone: invalid time {h}:{mn}:{sec}")))?;
    let naive = date.and_time(time);
    let total_offset_secs = (oh * 3600 + om.signum() * (om.abs() * 60)) as i32;
    let offset = chrono::FixedOffset::east_opt(total_offset_secs)
        .ok_or_else(|| MError::Other(format!("#datetimezone: invalid offset {oh}:{om}")))?;
    Ok(Value::Datetimezone(naive.and_local_timezone(offset).unwrap()))
}

fn local_now(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Datetimezone(chrono::Local::now().fixed_offset()))
}


fn utc_now(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Datetimezone(
        chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
    ))
}


fn from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => Ok(Value::Datetimezone(*dt)),
        Value::Datetime(naive) => {
            // Assume local offset.
            let local_off = *chrono::Local::now().offset();
            let dt = naive.and_local_timezone(local_off).single()
                .ok_or_else(|| MError::Other("DateTimeZone.From: ambiguous/invalid local time".into()))?;
            Ok(Value::Datetimezone(dt))
        }
        Value::Date(d) => {
            let naive = d.and_time(chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap());
            let local_off = *chrono::Local::now().offset();
            let dt = naive.and_local_timezone(local_off).single()
                .ok_or_else(|| MError::Other("DateTimeZone.From: ambiguous/invalid local time".into()))?;
            Ok(Value::Datetimezone(dt))
        }
        Value::Text(_) => from_text(args, host),
        other => Err(type_mismatch("text/date/datetime/datetimezone/null", other)),
    }
}


fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // Try a few common formats with timezone info.
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(text) {
        return Ok(Value::Datetimezone(dt));
    }
    for fmt in &[
        "%Y-%m-%dT%H:%M:%S%z",
        "%Y-%m-%d %H:%M:%S%z",
        "%Y-%m-%dT%H:%M:%S%.f%z",
    ] {
        if let Ok(dt) = chrono::DateTime::parse_from_str(text, fmt) {
            return Ok(Value::Datetimezone(dt));
        }
    }
    Err(MError::Other(format!("DateTimeZone.FromText: cannot parse {text:?}")))
}


fn from_file_time(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let ticks = match &args[0] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    // Windows FILETIME → UTC.
    let base = chrono::DateTime::<chrono::FixedOffset>::parse_from_rfc3339("1601-01-01T00:00:00+00:00").unwrap();
    let nanos = (ticks * 100.0) as i64;
    Ok(Value::Datetimezone(base + chrono::Duration::nanoseconds(nanos)))
}


fn remove_zone(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => Ok(Value::Datetime(dt.naive_local())),
        other => Err(type_mismatch("datetimezone", other)),
    }
}


fn switch_zone(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Datetimezone(dt) => *dt,
        other => return Err(type_mismatch("datetimezone", other)),
    };
    let hours = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 => *n as i32,
        other => return Err(type_mismatch("integer (hours)", other)),
    };
    let minutes = match args.get(2) {
        Some(Value::Number(n)) if n.fract() == 0.0 => *n as i32,
        Some(Value::Null) | None => 0,
        Some(other) => return Err(type_mismatch("integer (minutes)", other)),
    };
    let secs = hours * 3600 + minutes.signum() * minutes.abs() * 60 * hours.signum().max(1);
    // Simpler signed-offset calculation: combine hours+minutes with shared sign.
    let total_minutes = hours * 60 + if hours < 0 { -minutes.abs() } else { minutes };
    let _ = secs;
    let off = chrono::FixedOffset::east_opt(total_minutes * 60)
        .ok_or_else(|| MError::Other("DateTimeZone.SwitchZone: offset out of range".into()))?;
    Ok(Value::Datetimezone(dt.with_timezone(&off)))
}


fn to_local(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => {
            let local_off = *chrono::Local::now().offset();
            Ok(Value::Datetimezone(dt.with_timezone(&local_off)))
        }
        other => Err(type_mismatch("datetimezone", other)),
    }
}


fn to_utc(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => Ok(Value::Datetimezone(
            dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        )),
        other => Err(type_mismatch("datetimezone", other)),
    }
}


fn to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::{Datelike, Timelike};
    let dt = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Datetimezone(dt) => *dt,
        other => return Err(type_mismatch("datetimezone", other)),
    };
    let off_total_secs = dt.offset().local_minus_utc();
    let off_hours = off_total_secs / 3600;
    let off_minutes = (off_total_secs.abs() % 3600) / 60 * off_total_secs.signum();
    Ok(Value::Record(Record {
        fields: vec![
            ("Year".into(),        Value::Number(dt.year() as f64)),
            ("Month".into(),       Value::Number(dt.month() as f64)),
            ("Day".into(),         Value::Number(dt.day() as f64)),
            ("Hour".into(),        Value::Number(dt.hour() as f64)),
            ("Minute".into(),      Value::Number(dt.minute() as f64)),
            ("Second".into(),      Value::Number(dt.second() as f64)),
            ("ZoneHours".into(),   Value::Number(off_hours as f64)),
            ("ZoneMinutes".into(), Value::Number(off_minutes as f64)),
        ],
        env: super::super::env::EnvNode::empty(),
    }))
}


fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Datetimezone(dt) => *dt,
        other => return Err(type_mismatch("datetimezone", other)),
    };
    // PQ accepts either a plain text Format string or an options record
    // `[Format = "...", Culture = "..."]`. The record form is what
    // Power Query docs document; the plain-text form is a shorthand.
    let (fmt, culture_from_rec) = match args.get(1) {
        None | Some(Value::Null) => (None, None),
        Some(Value::Text(s)) => {
            let t = s.trim();
            (if t.is_empty() { None } else { Some(s.clone()) }, None)
        }
        Some(Value::Record(rec)) => {
            // Record fields can be thunks (lazy bindings) — force before
            // typecheck. NoIoHost is fine since these are pure literals.
            let force = |v: Value| -> Result<Value, MError> {
                super::super::force(v, &mut |e, env| {
                    super::super::evaluate(e, env, &super::super::NoIoHost)
                })
            };
            let f = rec.fields.iter()
                .find(|(k, _)| k == "Format")
                .map(|(_, v)| v.clone());
            let c = rec.fields.iter()
                .find(|(k, _)| k == "Culture")
                .map(|(_, v)| v.clone());
            let f = match f { Some(v) => Some(force(v)?), None => None };
            let c = match c { Some(v) => Some(force(v)?), None => None };
            let fmt_text = match f {
                None | Some(Value::Null) => None,
                Some(Value::Text(s)) => {
                    let t = s.trim();
                    if t.is_empty() { None } else { Some(s) }
                }
                Some(other) => return Err(type_mismatch("text (Format)", &other)),
            };
            let cul_text = match c {
                Some(Value::Text(s)) => Some(s),
                _ => None,
            };
            (fmt_text, cul_text)
        }
        Some(other) => return Err(type_mismatch("text or record (format)", other)),
    };
    let culture = match args.get(2) {
        Some(Value::Text(c)) => Some(c.clone()),
        _ => culture_from_rec,
    };
    // .NET's "u" and "R" standard formats convert to UTC before rendering.
    // Detect those before pattern translation so the offset is normalised.
    let (effective_dt, pattern) = match fmt {
        None => {
            // No format: default emits short culture-specific date-time-offset.
            // PQ's en-GB default is "dd/MM/yyyy HH:mm:ss zzz" (offset spelled out).
            let s = dt.format("%d/%m/%Y %H:%M:%S %:z").to_string();
            return Ok(Value::Text(s));
        }
        Some(f) => {
            let trimmed = f.trim();
            // PQ rejects standalone "K" / "z" / "zz" / "zzz" — these are
            // offset-only specifiers and PQ's documented behaviour is to
            // refuse when used alone. (q185 covers K; q1159/q1160/q1164 cover
            // zzz/z/zz/half-offset variants — all surfaced via Excel-empty.)
            if matches!(trimmed, "K" | "z" | "zz" | "zzz") {
                return Err(MError::Other(
                    "The output DateTimeZone format specified isn't supported.".into()
                ));
            }
            if matches!(trimmed, "G" | "g") {
                return Ok(Value::Text(dt.to_rfc3339()));
            }
            // u/R convert to UTC; everything else keeps the local offset.
            let dt2 = if matches!(trimmed, "u" | "R" | "r") {
                use chrono::TimeZone;
                let utc = chrono::Utc.from_utc_datetime(&dt.naive_utc());
                utc.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
            } else {
                dt
            };
            let mut expanded = super::date::expand_standard_date_format(&f, culture.as_deref());
            // .NET's "O"/"o" round-trip format on DateTimeZone appends K
            // (offset). expand_standard_date_format omits K so the plain-date
            // DateTime.ToText path stays safe; we add it here for tz values.
            if matches!(trimmed, "O" | "o") {
                expanded.push('K');
            }
            (dt2, super::date::dotnet_to_strftime(&expanded))
        }
    };
    Ok(Value::Text(effective_dt.format(&pattern).to_string()))
}


fn zone_hours(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => Ok(Value::Number((dt.offset().local_minus_utc() / 3600) as f64)),
        other => Err(type_mismatch("datetimezone", other)),
    }
}


fn zone_minutes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => {
            let total = dt.offset().local_minus_utc();
            let m = (total.abs() % 3600) / 60 * total.signum();
            Ok(Value::Number(m as f64))
        }
        other => Err(type_mismatch("datetimezone", other)),
    }
}

