//! `Value.*` arithmetic + equality stdlib bindings.

#![allow(unused_imports)]

use chrono::{Datelike, Duration as ChronoDuration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Timelike};

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, TypeRep, Value};
use super::common::{
    expect_function, expect_text, invoke_callback_with_host, one, three, two, type_mismatch,
    values_equal_primitive,
};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        // --- Slice #170: type / inspect ---
        ("Value.As", two("value", "type"), as_),
        ("Value.Is", two("value", "type"), is),
        ("Value.Type", one("value"), type_),
        ("Value.ReplaceType", two("value", "type"), replace_type),
        (
            "Value.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            from_text,
        ),
        ("Value.Expression", one("value"), expression),
        ("Value.Lineage", one("value"), lineage),
        ("Value.Traits", one("value"), traits),
        ("Value.VersionIdentity", one("value"), version_identity),
        ("Value.Versions", one("value"), versions),
        (
            "Value.NativeQuery",
            vec![
                Param { name: "target".into(),     optional: false, type_annotation: None },
                Param { name: "query".into(),      optional: false, type_annotation: None },
                Param { name: "parameters".into(), optional: true,  type_annotation: None },
                Param { name: "options".into(),    optional: true,  type_annotation: None },
            ],
            native_query,
        ),
        ("Value.Optimize", one("value"), passthrough),
        (
            "Value.Firewall",
            vec![
                Param { name: "value".into(),       optional: false, type_annotation: None },
                Param { name: "trustLevel".into(),  optional: true,  type_annotation: None },
            ],
            firewall,
        ),
        ("Value.Alternates", one("value"), alternates),
        ("Value.ViewError", one("value"), passthrough),
        ("Value.ViewFunction", one("value"), passthrough),
        // --- Slice #171: metadata ---
        ("Value.Metadata", one("value"), metadata),
        ("Value.ReplaceMetadata", two("value", "metadata"), replace_metadata),
        ("Value.RemoveMetadata", one("value"), remove_metadata),
        // --- Slice #169: arithmetic + equality ---
        (
            "Value.Add",
            vec![
                Param { name: "x".into(),         optional: false, type_annotation: None },
                Param { name: "y".into(),         optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            add,
        ),
        (
            "Value.Subtract",
            vec![
                Param { name: "x".into(),         optional: false, type_annotation: None },
                Param { name: "y".into(),         optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            subtract,
        ),
        (
            "Value.Multiply",
            vec![
                Param { name: "x".into(),         optional: false, type_annotation: None },
                Param { name: "y".into(),         optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            multiply,
        ),
        (
            "Value.Divide",
            vec![
                Param { name: "x".into(),         optional: false, type_annotation: None },
                Param { name: "y".into(),         optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            divide,
        ),
        (
            "Value.Equals",
            vec![
                Param { name: "x".into(),                optional: false, type_annotation: None },
                Param { name: "y".into(),                optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            equals,
        ),
        (
            "Value.Compare",
            vec![
                Param { name: "x".into(),        optional: false, type_annotation: None },
                Param { name: "y".into(),        optional: false, type_annotation: None },
                Param { name: "comparer".into(), optional: true,  type_annotation: None },
            ],
            compare,
        ),
        (
            "Value.NullableEquals",
            vec![
                Param { name: "x".into(),        optional: false, type_annotation: None },
                Param { name: "y".into(),        optional: false, type_annotation: None },
                Param { name: "comparer".into(), optional: true,  type_annotation: None },
            ],
            nullable_equals,
        ),
    ]
}

// --- Slice #170 impls ---

fn expect_typerep(v: &Value) -> Result<&TypeRep, MError> {
    match v {
        Value::Type(t) => Ok(t),
        other => Err(type_mismatch("type", other)),
    }
}

fn as_(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_typerep(&args[1])?;
    if super::super::type_conforms(&args[0], t) {
        Ok(args[0].clone())
    } else {
        Err(MError::Other(format!(
            "Value.As: value does not conform to type {}",
            super::super::type_rep_name(t)
        )))
    }
}

fn is(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_typerep(&args[1])?;
    Ok(Value::Logical(super::super::type_conforms(&args[0], t)))
}

fn type_(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Type(typerep_of(&args[0])))
}

fn typerep_of(v: &Value) -> TypeRep {
    match v {
        Value::Null => TypeRep::Null,
        Value::Logical(_) => TypeRep::Logical,
        Value::Number(_) => TypeRep::Number,
        Value::Text(_) => TypeRep::Text,
        Value::Date(_) => TypeRep::Date,
        Value::Datetime(_) => TypeRep::Datetime,
        Value::Datetimezone(_) => TypeRep::Datetimezone,
        Value::Time(_) => TypeRep::Time,
        Value::Duration(_) => TypeRep::Duration,
        Value::Binary(_) => TypeRep::Binary,
        Value::List(_) => TypeRep::List,
        Value::Record(_) => TypeRep::Record,
        Value::Table(_) => TypeRep::Table,
        Value::Function(_) => TypeRep::Function,
        Value::Type(_) => TypeRep::Type,
        Value::Thunk(_) => TypeRep::Any,
        Value::WithMetadata { inner, .. } => typerep_of(inner),
    }
}

fn replace_type(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: we don't track ascribed type metadata — return value unchanged.
    let _ = expect_typerep(&args[1])?;
    Ok(args[0].clone())
}

fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let s = expect_text(&args[0])?.trim();
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Value.FromText: culture argument not yet supported",
        ));
    }
    if s.eq_ignore_ascii_case("null") {
        return Ok(Value::Null);
    }
    if s.eq_ignore_ascii_case("true") {
        return Ok(Value::Logical(true));
    }
    if s.eq_ignore_ascii_case("false") {
        return Ok(Value::Logical(false));
    }
    if let Ok(n) = s.parse::<f64>() {
        return Ok(Value::Number(n));
    }
    // Quoted text: strip leading/trailing double-quote pair.
    if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
        return Ok(Value::Text(s[1..s.len() - 1].to_string()));
    }
    Err(MError::Other(format!(
        "Value.FromText: could not parse: {s}"
    )))
}

fn expression(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Err(MError::NotImplemented(
        "Value.Expression: source expression introspection requires cloud-PQ infra",
    ))
}

fn lineage(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no fold lineage tracking.
    Ok(Value::List(Vec::new()))
}

fn traits(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Record(Record {
        fields: Vec::new(),
        env: EnvNode::empty(),
    }))
}

fn version_identity(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Err(MError::NotImplemented(
        "Value.VersionIdentity: requires versioning machinery not built in v1",
    ))
}

fn versions(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Err(MError::NotImplemented(
        "Value.Versions: requires versioning machinery not built in v1",
    ))
}

fn native_query(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Err(MError::NotImplemented(
        "Value.NativeQuery: query folding not available without a fold-aware data source",
    ))
}

fn passthrough(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(args[0].clone())
}

fn firewall(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no privacy levels — values pass through unchanged.
    Ok(args[0].clone())
}

fn alternates(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::List(vec![args[0].clone()]))
}

// --- Slice #171: metadata ---

fn metadata(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::WithMetadata { meta, .. } => Ok(Value::Record(meta.clone())),
        _ => Ok(Value::Record(Record {
            fields: Vec::new(),
            env: EnvNode::empty(),
        })),
    }
}

fn replace_metadata(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let meta = match &args[1] {
        Value::Record(r) => r.clone(),
        Value::WithMetadata { inner, .. } => match inner.as_ref() {
            Value::Record(r) => r.clone(),
            other => return Err(type_mismatch("record (metadata)", other)),
        },
        other => return Err(type_mismatch("record (metadata)", other)),
    };
    let inner = match &args[0] {
        Value::WithMetadata { inner, .. } => (**inner).clone(),
        v => v.clone(),
    };
    Ok(Value::WithMetadata {
        inner: Box::new(inner),
        meta,
    })
}

fn remove_metadata(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(match &args[0] {
        Value::WithMetadata { inner, .. } => (**inner).clone(),
        v => v.clone(),
    })
}

// --- Slice #169 impls ---

pub(crate) fn value_add(a: &Value, b: &Value) -> Result<Value, MError> {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a + b)),
        (Value::Text(a), Value::Text(b)) => Ok(Value::Text(format!("{a}{b}"))),
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

pub(crate) fn value_subtract(a: &Value, b: &Value) -> Result<Value, MError> {
    match (a, b) {
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

fn add(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    value_add(&args[0], &args[1])
}

fn subtract(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    value_subtract(&args[0], &args[1])
}

fn multiply(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match (&args[0], &args[1]) {
        (Value::Number(a), Value::Number(b)) => Ok(Value::Number(a * b)),
        (a, b) => Err(MError::Other(format!(
            "Value.Multiply: cannot multiply {} by {}",
            super::super::type_name(a),
            super::super::type_name(b),
        ))),
    }
}

fn divide(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn equals(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Value.Equals: equationCriteria not yet supported",
        ));
    }
    Ok(Value::Logical(values_equal_deep(&args[0], &args[1])?))
}

fn compare(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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

fn nullable_equals(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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
