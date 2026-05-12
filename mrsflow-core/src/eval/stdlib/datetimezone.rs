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
        ("DateTimeZone.FixedLocalNow", vec![], dtz_local_now),
        ("DateTimeZone.FixedUtcNow", vec![], dtz_utc_now),
        ("DateTimeZone.LocalNow", vec![], dtz_local_now),
        ("DateTimeZone.UtcNow", vec![], dtz_utc_now),
        ("DateTimeZone.From", one("value"), dtz_from),
        (
            "DateTimeZone.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            dtz_from_text,
        ),
        ("DateTimeZone.FromFileTime", one("fileTime"), dtz_from_file_time),
        ("DateTimeZone.RemoveZone", one("dtz"), dtz_remove_zone),
        (
            "DateTimeZone.SwitchZone",
            vec![
                Param { name: "dtz".into(),     optional: false, type_annotation: None },
                Param { name: "hours".into(),   optional: false, type_annotation: None },
                Param { name: "minutes".into(), optional: true,  type_annotation: None },
            ],
            dtz_switch_zone,
        ),
        ("DateTimeZone.ToLocal", one("dtz"), dtz_to_local),
        ("DateTimeZone.ToUtc", one("dtz"), dtz_to_utc),
        ("DateTimeZone.ToRecord", one("dtz"), dtz_to_record),
        (
            "DateTimeZone.ToText",
            vec![
                Param { name: "dtz".into(),     optional: false, type_annotation: None },
                Param { name: "format".into(),  optional: true,  type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            dtz_to_text,
        ),
        ("DateTimeZone.ZoneHours", one("dtz"), dtz_zone_hours),
        ("DateTimeZone.ZoneMinutes", one("dtz"), dtz_zone_minutes),
    ]
}

fn dtz_local_now(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Datetimezone(chrono::Local::now().fixed_offset()))
}


fn dtz_utc_now(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Datetimezone(
        chrono::Utc::now().with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
    ))
}


fn dtz_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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
        Value::Text(_) => dtz_from_text(args, host),
        other => Err(type_mismatch("text/date/datetime/datetimezone/null", other)),
    }
}


fn dtz_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
    Err(MError::Other(format!("DateTimeZone.FromText: cannot parse {:?}", text)))
}


fn dtz_from_file_time(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn dtz_remove_zone(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => Ok(Value::Datetime(dt.naive_local())),
        other => Err(type_mismatch("datetimezone", other)),
    }
}


fn dtz_switch_zone(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn dtz_to_local(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => {
            let local_off = *chrono::Local::now().offset();
            Ok(Value::Datetimezone(dt.with_timezone(&local_off)))
        }
        other => Err(type_mismatch("datetimezone", other)),
    }
}


fn dtz_to_utc(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => Ok(Value::Datetimezone(
            dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap())
        )),
        other => Err(type_mismatch("datetimezone", other)),
    }
}


fn dtz_to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn dtz_to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let dt = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Datetimezone(dt) => *dt,
        other => return Err(type_mismatch("datetimezone", other)),
    };
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("DateTimeZone.ToText: format string not yet supported"));
    }
    Ok(Value::Text(dt.to_rfc3339()))
}


fn dtz_zone_hours(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Datetimezone(dt) => Ok(Value::Number((dt.offset().local_minus_utc() / 3600) as f64)),
        other => Err(type_mismatch("datetimezone", other)),
    }
}


fn dtz_zone_minutes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

