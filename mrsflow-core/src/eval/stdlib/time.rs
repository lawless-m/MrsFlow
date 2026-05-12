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
        ("Time.Hour", one("time"), time_hour),
        ("Time.Minute", one("time"), time_minute),
        ("Time.Second", one("time"), time_second),
        ("Time.From", one("value"), time_from),
        (
            "Time.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            time_from_text,
        ),
        (
            "Time.ToText",
            vec![
                Param { name: "time".into(),   optional: false, type_annotation: None },
                Param { name: "format".into(), optional: true,  type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            time_to_text,
        ),
        ("Time.ToRecord", one("time"), time_to_record),
        ("Time.StartOfHour", one("time"), time_start_of_hour),
        ("Time.EndOfHour", one("time"), time_end_of_hour),
    ]
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


fn time_hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    Ok(Value::Number(extract_time(&args[0], "Time.Hour")?.hour() as f64))
}


fn time_minute(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    Ok(Value::Number(extract_time(&args[0], "Time.Minute")?.minute() as f64))
}


fn time_second(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Timelike;
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    Ok(Value::Number(extract_time(&args[0], "Time.Second")?.second() as f64))
}


fn time_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Time(t) => Ok(Value::Time(*t)),
        Value::Datetime(dt) => Ok(Value::Time(dt.time())),
        Value::Text(_) => time_from_text(args, host),
        other => Err(type_mismatch("text/time/datetime/null", other)),
    }
}


fn time_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    for fmt in &["%H:%M:%S%.f", "%H:%M:%S", "%H:%M"] {
        if let Ok(t) = chrono::NaiveTime::parse_from_str(text, fmt) {
            return Ok(Value::Time(t));
        }
    }
    Err(MError::Other(format!("Time.FromText: cannot parse {:?}", text)))
}


fn time_to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) { return Ok(Value::Null); }
    let t = extract_time(&args[0], "Time.ToText")?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("Time.ToText: format string not yet supported"));
    }
    Ok(Value::Text(t.format("%H:%M:%S").to_string()))
}


fn time_to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn time_start_of_hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn time_end_of_hour(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

