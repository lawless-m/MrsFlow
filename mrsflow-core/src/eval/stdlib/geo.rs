//! `Geography.*` and `Geometry.*` — WKT (Well-Known Text) parsing and
//! constructors. Only the POINT shape is supported (round-trip matches
//! Excel byte-for-byte). LINESTRING, POLYGON, MULTIPOINT etc. would be
//! larger features.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Value};
use super::common::{expect_text, one, two};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Geography.FromWellKnownText", one("wkt"), geography_from_wkt),
        ("Geography.ToWellKnownText", one("value"), geography_to_wkt),
        (
            "GeographyPoint.From",
            vec![
                Param { name: "longitude".into(), optional: false, type_annotation: None },
                Param { name: "latitude".into(),  optional: false, type_annotation: None },
                Param { name: "z".into(),         optional: true,  type_annotation: None },
                Param { name: "srid".into(),      optional: true,  type_annotation: None },
            ],
            geography_point_from,
        ),
        ("Geometry.FromWellKnownText", one("wkt"), geometry_from_wkt),
        ("Geometry.ToWellKnownText", one("value"), geometry_to_wkt),
        (
            "GeometryPoint.From",
            vec![
                Param { name: "x".into(),    optional: false, type_annotation: None },
                Param { name: "y".into(),    optional: false, type_annotation: None },
                Param { name: "z".into(),    optional: true,  type_annotation: None },
                Param { name: "srid".into(), optional: true,  type_annotation: None },
            ],
            geometry_point_from,
        ),
    ]
}

fn record_kind_xy(kind: &str, a_name: &str, a: f64, b_name: &str, b: f64) -> Value {
    Value::Record(Record {
        fields: vec![
            ("Kind".into(), Value::Text(kind.into())),
            (a_name.into(), Value::Number(a)),
            (b_name.into(), Value::Number(b)),
        ],
        env: EnvNode::empty(),
    })
}

fn parse_point_wkt(wkt: &str) -> Result<(f64, f64), MError> {
    // "POINT(x y)" / "POINT (x y)" / case-insensitive prefix.
    let trimmed = wkt.trim();
    let upper = trimmed.to_ascii_uppercase();
    let rest = upper.strip_prefix("POINT")
        .ok_or_else(|| MError::Other(format!("WKT: only POINT supported, got {wkt:?}")))?
        .trim_start();
    let inner = rest.strip_prefix('(')
        .and_then(|s| s.strip_suffix(')'))
        .ok_or_else(|| MError::Other(format!("WKT: missing parens in {wkt:?}")))?
        .trim();
    let mut parts = inner.split_ascii_whitespace();
    let x: f64 = parts.next().ok_or_else(|| MError::Other("WKT: missing x".into()))?
        .parse().map_err(|e| MError::Other(format!("WKT: bad x: {e}")))?;
    let y: f64 = parts.next().ok_or_else(|| MError::Other("WKT: missing y".into()))?
        .parse().map_err(|e| MError::Other(format!("WKT: bad y: {e}")))?;
    Ok((x, y))
}

fn geography_from_wkt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let wkt = expect_text(&args[0])?;
    let (lon, lat) = parse_point_wkt(wkt)?;
    // WKT for geography is "POINT(longitude latitude)" — note the order.
    Ok(record_kind_xy("POINT", "Longitude", lon, "Latitude", lat))
}

fn geometry_from_wkt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let wkt = expect_text(&args[0])?;
    let (x, y) = parse_point_wkt(wkt)?;
    Ok(record_kind_xy("POINT", "X", x, "Y", y))
}

fn geography_point_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lon = expect_number(&args[0], "GeographyPoint.From: longitude")?;
    let lat = expect_number(&args[1], "GeographyPoint.From: latitude")?;
    Ok(record_kind_xy("POINT", "Longitude", lon, "Latitude", lat))
}

fn geometry_point_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let x = expect_number(&args[0], "GeometryPoint.From: x")?;
    let y = expect_number(&args[1], "GeometryPoint.From: y")?;
    Ok(record_kind_xy("POINT", "X", x, "Y", y))
}

fn geography_to_wkt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let (lon, lat) = expect_point_fields(&args[0], "Longitude", "Latitude")?;
    Ok(Value::Text(format_point_wkt(lon, lat)))
}

fn geometry_to_wkt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let (x, y) = expect_point_fields(&args[0], "X", "Y")?;
    Ok(Value::Text(format_point_wkt(x, y)))
}

fn expect_number(v: &Value, ctx: &str) -> Result<f64, MError> {
    match v {
        Value::Number(n) => Ok(*n),
        other => Err(MError::Other(format!("{ctx}: expected number, got {other:?}"))),
    }
}

fn expect_point_fields(v: &Value, a: &str, b: &str) -> Result<(f64, f64), MError> {
    let r = match v {
        Value::Record(r) => r,
        other => return Err(MError::Other(format!("expected record, got {other:?}"))),
    };
    let mut av: Option<f64> = None;
    let mut bv: Option<f64> = None;
    for (name, val) in &r.fields {
        if name == a {
            if let Value::Number(n) = val { av = Some(*n); }
        } else if name == b {
            if let Value::Number(n) = val { bv = Some(*n); }
        }
    }
    match (av, bv) {
        (Some(x), Some(y)) => Ok((x, y)),
        _ => Err(MError::Other(format!("missing {a}/{b} fields"))),
    }
}

fn format_point_wkt(a: f64, b: f64) -> String {
    format!("POINT({} {})", trim_trailing_zero(a), trim_trailing_zero(b))
}

fn trim_trailing_zero(n: f64) -> String {
    let s = format!("{n}");
    // Rust prints whole-number floats as "10" already (no ".0"); match.
    s
}
