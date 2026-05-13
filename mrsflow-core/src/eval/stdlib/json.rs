//! `Json.Document` / `Json.FromValue` — JSON read/write.

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Value};
use super::common::type_mismatch;
use super::table::table_to_rows;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        (
            "Json.Document",
            vec![
                Param { name: "jsonText".into(), optional: false, type_annotation: None },
                Param { name: "encoding".into(), optional: true,  type_annotation: None },
            ],
            document,
        ),
        (
            "Json.FromValue",
            vec![
                Param { name: "value".into(),    optional: false, type_annotation: None },
                Param { name: "encoding".into(), optional: true,  type_annotation: None },
            ],
            from_value,
        ),
    ]
}

fn document(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bytes: Vec<u8> = match &args[0] {
        Value::Binary(b) => b.clone(),
        Value::Text(t) => t.as_bytes().to_vec(),
        other => return Err(type_mismatch("binary or text", other)),
    };
    // `encoding` (args.get(1)) is accepted but ignored — M's Json.Document
    // takes a TextEncoding.* enum; we always decode as UTF-8 since serde_json
    // requires that. If/when the corpus calls Json.Document with non-UTF-8
    // encoding, decode here before parsing.
    let parsed: serde_json::Value = serde_json::from_slice(&bytes)
        .map_err(|e| MError::Other(format!("Json.Document: {e}")))?;
    Ok(json_to_value(parsed))
}

fn from_value(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    // `encoding` (args.get(1)) is accepted but ignored — we always emit UTF-8.
    // Deep-force first so record field thunks resolve before serialisation;
    // serde_json can't observe lazy values.
    let forced = super::super::deep_force(args[0].clone(), host)?;
    let json = value_to_json(&forced)?;
    let text = serde_json::to_string(&json)
        .map_err(|e| MError::Other(format!("Json.FromValue: {e}")))?;
    // Returns Binary per the M spec — UTF-8 bytes of the JSON text.
    Ok(Value::Binary(text.into_bytes()))
}

fn value_to_json(v: &Value) -> Result<serde_json::Value, MError> {
    use serde_json::Number;
    match v {
        Value::Null => Ok(serde_json::Value::Null),
        Value::Logical(b) => Ok(serde_json::Value::Bool(*b)),
        Value::Number(n) => {
            // JSON has no NaN / Infinity; emit null per the convention used
            // by many JSON serialisers (and by Json.Document's inverse,
            // which would round-trip these through f64::NAN).
            if !n.is_finite() {
                return Ok(serde_json::Value::Null);
            }
            // Prefer the integer encoding when the value fits exactly —
            // matches what `serde_json::to_string(&42i64)` would produce
            // and avoids gratuitous ".0" suffixes for whole numbers.
            if n.fract() == 0.0 && n.abs() < 1e15 {
                Ok(serde_json::Value::Number((*n as i64).into()))
            } else {
                Ok(Number::from_f64(*n)
                    .map(serde_json::Value::Number)
                    .unwrap_or(serde_json::Value::Null))
            }
        }
        Value::Decimal { mantissa, scale, .. } => {
            // Render via the lossy f64 path. JSON has no decimal type,
            // so any encoding loses precision — f64 is the obvious
            // choice and matches what most JSON consumers expect.
            let f = super::super::value::decimal_to_f64(*mantissa, *scale);
            value_to_json(&Value::Number(f))
        }
        Value::Text(s) => Ok(serde_json::Value::String(s.clone())),
        Value::Date(d) => Ok(serde_json::Value::String(d.to_string())),
        Value::Datetime(dt) => Ok(serde_json::Value::String(dt.to_string())),
        Value::Datetimezone(dt) => Ok(serde_json::Value::String(dt.to_rfc3339())),
        Value::Time(t) => Ok(serde_json::Value::String(t.to_string())),
        Value::Duration(d) => {
            // ISO-8601 PT_S form; M's Duration is a chrono::Duration internally.
            Ok(serde_json::Value::String(format!("PT{}S", d.num_seconds())))
        }
        Value::Binary(b) => {
            // Mirror Binary.ToText(_, BinaryEncoding.Base64): JSON has no
            // binary type, base64 is the canonical text wrapper.
            Ok(serde_json::Value::String(super::binary::base64_encode(b)))
        }
        Value::List(xs) => {
            let arr: Result<Vec<_>, _> = xs.iter().map(value_to_json).collect();
            Ok(serde_json::Value::Array(arr?))
        }
        Value::Record(r) => {
            let mut map = serde_json::Map::new();
            for (name, v) in &r.fields {
                map.insert(name.clone(), value_to_json(v)?);
            }
            Ok(serde_json::Value::Object(map))
        }
        Value::Table(t) => {
            // Tables serialise as a list of row-records — same shape
            // Json.Document would round-trip back to a list of records.
            let forced = t.force()?;
            let (names, rows) = table_to_rows(&forced)?;
            let mut arr: Vec<serde_json::Value> = Vec::with_capacity(rows.len());
            for row in rows {
                let mut map = serde_json::Map::new();
                for (name, cell) in names.iter().zip(row.iter()) {
                    map.insert(name.clone(), value_to_json(cell)?);
                }
                arr.push(serde_json::Value::Object(map));
            }
            Ok(serde_json::Value::Array(arr))
        }
        Value::Function(_) => Err(MError::Other(
            "Json.FromValue: cannot serialise a function".into(),
        )),
        Value::Type(_) => Err(MError::Other(
            "Json.FromValue: cannot serialise a type value".into(),
        )),
        Value::Thunk(_) => Err(MError::Other(
            "Json.FromValue: thunk should have been forced before serialisation".into(),
        )),
        Value::WithMetadata { inner, .. } => value_to_json(inner),
    }
}

fn json_to_value(j: serde_json::Value) -> Value {
    match j {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Logical(b),
        serde_json::Value::Number(n) => {
            // M numbers are all f64. JSON ints that don't round-trip as f64
            // are rare in practice (corpus has none yet); when they appear,
            // they'll lose precision here — track if it becomes an issue.
            let f = n.as_f64().unwrap_or(f64::NAN);
            Value::Number(f)
        }
        serde_json::Value::String(s) => Value::Text(s),
        serde_json::Value::Array(xs) => {
            Value::List(xs.into_iter().map(json_to_value).collect())
        }
        serde_json::Value::Object(map) => {
            // serde_json's `preserve_order` feature keeps insertion order,
            // matching M's record semantics.
            let fields: Vec<(String, Value)> = map
                .into_iter()
                .map(|(k, v)| (k, json_to_value(v)))
                .collect();
            Value::Record(Record { fields, env: EnvNode::empty() })
        }
    }
}
