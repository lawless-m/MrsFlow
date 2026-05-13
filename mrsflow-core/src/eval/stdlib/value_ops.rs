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
        Value::Decimal { .. } => TypeRep::Number,
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
        (
            Value::Decimal { mantissa: ma, scale: sa, precision: pa },
            Value::Decimal { mantissa: mb, scale: sb, precision: pb },
        ) => decimal_add_sub(*ma, *sa, *pa, *mb, *sb, *pb, false),
        (Value::Decimal { .. }, Value::Number(_)) | (Value::Number(_), Value::Decimal { .. }) => {
            Ok(Value::Number(
                a.as_f64_lossy().unwrap() + b.as_f64_lossy().unwrap(),
            ))
        }
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
        (
            Value::Decimal { mantissa: ma, scale: sa, precision: pa },
            Value::Decimal { mantissa: mb, scale: sb, precision: pb },
        ) => decimal_add_sub(*ma, *sa, *pa, *mb, *sb, *pb, true),
        (Value::Decimal { .. }, Value::Number(_)) | (Value::Number(_), Value::Decimal { .. }) => {
            Ok(Value::Number(
                a.as_f64_lossy().unwrap() - b.as_f64_lossy().unwrap(),
            ))
        }
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
        (
            Value::Decimal { mantissa: ma, scale: sa, precision: pa },
            Value::Decimal { mantissa: mb, scale: sb, precision: pb },
        ) => decimal_multiply(*ma, *sa, *pa, *mb, *sb, *pb),
        (Value::Decimal { .. }, Value::Number(_)) | (Value::Number(_), Value::Decimal { .. }) => {
            Ok(Value::Number(
                args[0].as_f64_lossy().unwrap() * args[1].as_f64_lossy().unwrap(),
            ))
        }
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
        (
            Value::Decimal { mantissa: ma, scale: sa, precision: pa },
            Value::Decimal { mantissa: mb, scale: sb, precision: pb },
        ) => decimal_divide(*ma, *sa, *pa, *mb, *sb, *pb),
        (Value::Decimal { .. }, Value::Number(_)) | (Value::Number(_), Value::Decimal { .. }) => {
            Ok(Value::Number(
                args[0].as_f64_lossy().unwrap() / args[1].as_f64_lossy().unwrap(),
            ))
        }
        (a, b) => Err(MError::Other(format!(
            "Value.Divide: cannot divide {} by {}",
            super::super::type_name(a),
            super::super::type_name(b),
        ))),
    }
}

/// Shared add/subtract path for Decimal × Decimal. Aligns to
/// `max(scale_a, scale_b)` by scaling the lower-scale mantissa up,
/// then adds or subtracts. Result precision is `max(pa, pb) + 1` to
/// reserve a digit for overflow into the next column (capped at 76 —
/// the Arrow Decimal256 maximum).
fn decimal_add_sub(
    ma: arrow::datatypes::i256,
    sa: i8,
    pa: u8,
    mb: arrow::datatypes::i256,
    sb: i8,
    pb: u8,
    subtract: bool,
) -> Result<Value, MError> {
    let (ma, mb, scale) = align_scales(ma, sa, mb, sb)?;
    let mantissa = if subtract {
        ma.wrapping_sub(mb)
    } else {
        ma.wrapping_add(mb)
    };
    let precision = pa.max(pb).saturating_add(1).min(76);
    Ok(Value::Decimal {
        mantissa,
        scale,
        precision,
    })
}

fn decimal_multiply(
    ma: arrow::datatypes::i256,
    sa: i8,
    pa: u8,
    mb: arrow::datatypes::i256,
    sb: i8,
    pb: u8,
) -> Result<Value, MError> {
    let mantissa = ma.wrapping_mul(mb);
    let scale = sa.checked_add(sb).ok_or_else(|| {
        MError::Other(format!(
            "Value.Multiply: Decimal scale overflow ({sa} + {sb})"
        ))
    })?;
    let precision = pa.saturating_add(pb).min(76);
    Ok(Value::Decimal {
        mantissa,
        scale,
        precision,
    })
}

/// Integer Decimal divide: pick a target scale `s_target = max(sa, sb, 6)`
/// and compute `ma * 10^(s_target + sb - sa) / mb`. Truncates toward zero
/// (matches Rust's i256::wrapping_div semantics). Errors on division by
/// zero — M's `/` on Number returns ±Infinity for divide-by-zero, but
/// Decimal can't represent infinity so we error.
fn decimal_divide(
    ma: arrow::datatypes::i256,
    sa: i8,
    pa: u8,
    mb: arrow::datatypes::i256,
    sb: i8,
    pb: u8,
) -> Result<Value, MError> {
    if mb == arrow::datatypes::i256::ZERO {
        return Err(MError::Other(
            "Value.Divide: Decimal division by zero".into(),
        ));
    }
    let target_scale: i8 = sa.max(sb).max(6);
    let scale_up: i8 = target_scale
        .checked_add(sb)
        .and_then(|x| x.checked_sub(sa))
        .ok_or_else(|| MError::Other("Value.Divide: Decimal scale overflow".into()))?;
    let scaled_ma = if scale_up >= 0 {
        ma.wrapping_mul(pow10_i256(scale_up as u32))
    } else {
        ma.wrapping_div(pow10_i256((-scale_up) as u32))
    };
    let mantissa = scaled_ma.wrapping_div(mb);
    let precision = pa.saturating_add(pb).saturating_add(6).min(76);
    Ok(Value::Decimal {
        mantissa,
        scale: target_scale,
        precision,
    })
}

/// Bring two Decimal mantissas to a common scale = max(sa, sb).
pub(crate) fn align_scales(
    ma: arrow::datatypes::i256,
    sa: i8,
    mb: arrow::datatypes::i256,
    sb: i8,
) -> Result<(arrow::datatypes::i256, arrow::datatypes::i256, i8), MError> {
    use std::cmp::Ordering::*;
    match sa.cmp(&sb) {
        Equal => Ok((ma, mb, sa)),
        Less => {
            let diff = (sb - sa) as u32;
            Ok((ma.wrapping_mul(pow10_i256(diff)), mb, sb))
        }
        Greater => {
            let diff = (sa - sb) as u32;
            Ok((ma, mb.wrapping_mul(pow10_i256(diff)), sa))
        }
    }
}

/// 10^n as an i256. Caches small powers (n ≤ 38, the Decimal128 range)
/// in a stack-local fast path; larger n falls through to a loop. n > 76
/// will overflow i256 — caller's responsibility to cap.
pub(crate) fn pow10_i256(n: u32) -> arrow::datatypes::i256 {
    let mut acc = arrow::datatypes::i256::ONE;
    let ten = arrow::datatypes::i256::from_i128(10);
    for _ in 0..n {
        acc = acc.wrapping_mul(ten);
    }
    acc
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
                    let av = super::table::cell_to_value(a, c, r)?;
                    let bv = super::table::cell_to_value(b, c, r)?;
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
        (
            Value::Decimal { mantissa: ma, scale: sa, .. },
            Value::Decimal { mantissa: mb, scale: sb, .. },
        ) => {
            let (ma, mb, _) = align_scales(*ma, *sa, *mb, *sb)?;
            ma.cmp(&mb)
        }
        (Value::Decimal { .. }, Value::Number(_)) | (Value::Number(_), Value::Decimal { .. }) => {
            a.as_f64_lossy()
                .unwrap()
                .partial_cmp(&b.as_f64_lossy().unwrap())
                .ok_or_else(|| MError::Other("Value.Compare: NaN".into()))?
        }
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
