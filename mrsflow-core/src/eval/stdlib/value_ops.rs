//! `Value.*` arithmetic + equality stdlib bindings.

#![allow(unused_imports)]

use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{
    expect_function, invoke_callback_with_host, one, three, two, type_mismatch,
    values_equal_primitive,
};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        (
            "Value.Add",
            vec![
                Param { name: "x".into(),         optional: false, type_annotation: None },
                Param { name: "y".into(),         optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            value_add,
        ),
        (
            "Value.Subtract",
            vec![
                Param { name: "x".into(),         optional: false, type_annotation: None },
                Param { name: "y".into(),         optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            value_subtract,
        ),
        (
            "Value.Multiply",
            vec![
                Param { name: "x".into(),         optional: false, type_annotation: None },
                Param { name: "y".into(),         optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            value_multiply,
        ),
        (
            "Value.Divide",
            vec![
                Param { name: "x".into(),         optional: false, type_annotation: None },
                Param { name: "y".into(),         optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            value_divide,
        ),
        (
            "Value.Equals",
            vec![
                Param { name: "x".into(),                optional: false, type_annotation: None },
                Param { name: "y".into(),                optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            value_equals,
        ),
        (
            "Value.Compare",
            vec![
                Param { name: "x".into(),        optional: false, type_annotation: None },
                Param { name: "y".into(),        optional: false, type_annotation: None },
                Param { name: "comparer".into(), optional: true,  type_annotation: None },
            ],
            value_compare,
        ),
        (
            "Value.NullableEquals",
            vec![
                Param { name: "x".into(),        optional: false, type_annotation: None },
                Param { name: "y".into(),        optional: false, type_annotation: None },
                Param { name: "comparer".into(), optional: true,  type_annotation: None },
            ],
            value_nullable_equals,
        ),
    ]
}

fn value_add(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
        (Value::Text(a), Value::Text(b)) => Ok(Value::Text(format!("{}{}", a, b))),
        (Value::Date(d), Value::Duration(dur)) | (Value::Duration(dur), Value::Date(d)) => {
            // Date + Duration: Power Query promotes to Datetime when the
            // duration has a non-day component.
            let dt = d.and_hms_opt(0, 0, 0).unwrap() + *dur;
            if dur.num_seconds() % 86400 == 0 {
                Ok(Value::Date(dt.date()))
            } else {
                Ok(Value::Datetime(dt))
            }
        }
        (Value::Datetime(dt), Value::Duration(dur))
        | (Value::Duration(dur), Value::Datetime(dt)) => Ok(Value::Datetime(*dt + *dur)),
        (Value::Time(t), Value::Duration(dur)) | (Value::Duration(dur), Value::Time(t)) => {
            let secs = dur.num_seconds().rem_euclid(86400);
            let added = NaiveTime::from_num_seconds_from_midnight_opt(secs as u32, 0)
                .unwrap_or(*t);
            // Carry the seconds offset onto t.
            let total = t.num_seconds_from_midnight() as i64 + secs;
            let total = total.rem_euclid(86400) as u32;
            let _ = added;
            Ok(Value::Time(
                NaiveTime::from_num_seconds_from_midnight_opt(total, 0)
                    .unwrap_or(*t),
            ))
        }
        (Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(*a + *b)),
        (Value::Datetimezone(a), Value::Duration(b))
        | (Value::Duration(b), Value::Datetimezone(a)) => Ok(Value::Datetimezone(*a + *b)),
        (a, b) => Err(MError::Other(format!(
            "Value.Add: cannot add {} and {}",
            super::super::type_name(a),
            super::super::type_name(b),
        ))),
    }
}

fn value_subtract(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a - b)),
        (Value::Date(a), Value::Date(b)) => {
            let dur = a.and_hms_opt(0, 0, 0).unwrap() - b.and_hms_opt(0, 0, 0).unwrap();
            Ok(Value::Duration(dur))
        }
        (Value::Datetime(a), Value::Datetime(b)) => Ok(Value::Duration(*a - *b)),
        (Value::Datetimezone(a), Value::Datetimezone(b)) => Ok(Value::Duration(*a - *b)),
        (Value::Time(a), Value::Time(b)) => Ok(Value::Duration(*a - *b)),
        (Value::Duration(a), Value::Duration(b)) => Ok(Value::Duration(*a - *b)),
        (Value::Date(d), Value::Duration(dur)) => {
            let dt = d.and_hms_opt(0, 0, 0).unwrap() - *dur;
            if dur.num_seconds() % 86400 == 0 {
                Ok(Value::Date(dt.date()))
            } else {
                Ok(Value::Datetime(dt))
            }
        }
        (Value::Datetime(dt), Value::Duration(dur)) => Ok(Value::Datetime(*dt - *dur)),
        (Value::Datetimezone(dt), Value::Duration(dur)) => Ok(Value::Datetimezone(*dt - *dur)),
        (a, b) => Err(MError::Other(format!(
            "Value.Subtract: cannot subtract {} from {}",
            super::super::type_name(b),
            super::super::type_name(a),
        ))),
    }
}

fn value_multiply(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
        (a, b) => Err(MError::Other(format!(
            "Value.Multiply: cannot multiply {} by {}",
            super::super::type_name(a),
            super::super::type_name(b),
        ))),
    }
}

fn value_divide(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a / b)),
        (a, b) => Err(MError::Other(format!(
            "Value.Divide: cannot divide {} by {}",
            super::super::type_name(a),
            super::super::type_name(b),
        ))),
    }
}

/// Deep structural equality. Lists / records compare element-wise (forcing
/// thunks first); tables compare via their backing rows.
pub(super) fn values_equal_deep(a: &Value, b: &Value) -> Result<bool, MError> {
    match (a, b) {
        (Value::List(xs), Value::List(ys)) => {
            if xs.len() != ys.len() {
                return Ok(false);
            }
            for (x, y) in xs.iter().zip(ys.iter()) {
                if !values_equal_deep(x, y)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (Value::Record(a), Value::Record(b)) => {
            if a.fields.len() != b.fields.len() {
                return Ok(false);
            }
            for ((an, av), (bn, bv)) in a.fields.iter().zip(b.fields.iter()) {
                if an != bn {
                    return Ok(false);
                }
                let av = super::super::force(av.clone(), &mut |e, env| {
                    super::super::evaluate(e, env, &super::super::NoIoHost)
                })?;
                let bv = super::super::force(bv.clone(), &mut |e, env| {
                    super::super::evaluate(e, env, &super::super::NoIoHost)
                })?;
                if !values_equal_deep(&av, &bv)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        (Value::Table(a), Value::Table(b)) => {
            if a.column_names() != b.column_names() {
                return Ok(false);
            }
            if a.num_rows() != b.num_rows() {
                return Ok(false);
            }
            let n_cols = a.num_columns();
            for r in 0..a.num_rows() {
                for c in 0..n_cols {
                    let av = super::cell_to_value(a, c, r)?;
                    let bv = super::cell_to_value(b, c, r)?;
                    if !values_equal_deep(&av, &bv)? {
                        return Ok(false);
                    }
                }
            }
            Ok(true)
        }
        _ => values_equal_primitive(a, b),
    }
}

fn value_equals(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Value.Equals: equationCriteria not yet supported",
        ));
    }
    Ok(Value::Logical(values_equal_deep(&args[0], &args[1])?))
}

fn value_compare(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    if let Some(Value::Function(c)) = args.get(2) {
        // User-supplied comparer takes precedence.
        let result = invoke_callback_with_host(c, vec![args[0].clone(), args[1].clone()], host)?;
        match result {
            Value::Number(_) => return Ok(result),
            other => return Err(type_mismatch("number (comparer result)", &other)),
        }
    }
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(type_mismatch("function or null (comparer)", &args[2]));
    }
    let cmp = compare_values(&args[0], &args[1])?;
    Ok(Value::Number(cmp as f64))
}

fn compare_values(a: &Value, b: &Value) -> Result<i32, MError> {
    use std::cmp::Ordering::*;
    let ord = match (a, b) {
        (Value::Null, Value::Null) => Equal,
        (Value::Null, _) => Less,
        (_, Value::Null) => Greater,
        (Value::Number(x), Value::Number(y)) => x
            .partial_cmp(y)
            .ok_or_else(|| MError::Other("Value.Compare: NaN".into()))?,
        (Value::Text(x), Value::Text(y)) => x.cmp(y),
        (Value::Logical(x), Value::Logical(y)) => x.cmp(y),
        (Value::Date(x), Value::Date(y)) => x.cmp(y),
        (Value::Datetime(x), Value::Datetime(y)) => x.cmp(y),
        (Value::Datetimezone(x), Value::Datetimezone(y)) => x.cmp(y),
        (Value::Time(x), Value::Time(y)) => x.cmp(y),
        (Value::Duration(x), Value::Duration(y)) => x.cmp(y),
        (a, b) => {
            return Err(MError::Other(format!(
                "Value.Compare: cannot compare {} and {}",
                super::super::type_name(a),
                super::super::type_name(b),
            )));
        }
    };
    Ok(match ord {
        Less => -1,
        Equal => 0,
        Greater => 1,
    })
}

fn value_nullable_equals(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    if let Some(Value::Function(c)) = args.get(2) {
        let result = invoke_callback_with_host(c, vec![args[0].clone(), args[1].clone()], host)?;
        match result {
            Value::Number(n) => return Ok(Value::Logical(n == 0.0)),
            other => return Err(type_mismatch("number (comparer result)", &other)),
        }
    }
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(type_mismatch("function or null (comparer)", &args[2]));
    }
    // null only equals null.
    Ok(Value::Logical(values_equal_deep(&args[0], &args[1])?))
}
