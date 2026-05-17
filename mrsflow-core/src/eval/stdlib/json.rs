//! `Json.Document` / `Json.FromValue` — JSON read/write.

use chrono::Timelike;

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
    // PQ post-processing of serde_json's output (see pq_jsonify).
    let text = pq_jsonify(&text);
    // Unwrap the __RAWNUM__ sentinel emitted for whole-valued floats that
    // exceed i64 range — strip the quotes and the prefix.
    let text = unwrap_raw_num_markers(&text);
    // Returns Binary per the M spec — UTF-8 bytes of the JSON text.
    Ok(Value::Binary(text.into_bytes()))
}

/// Post-process serde_json output so it matches Power Query's JSON encoder.
/// PQ escapes non-ASCII chars and short escape pairs (`\t`, `\n`, `\r`, `\b`,
/// `\f`) into their full `\uXXXX` form, and escapes `/` as `\/`.
fn pq_jsonify(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut in_string = false;
    let mut prev_backslash = false;
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if !in_string {
            if c == '"' { in_string = true; }
            out.push(c);
            i += 1;
            continue;
        }
        // We're inside a JSON string literal.
        if prev_backslash {
            // Expand the short escapes serde_json emits to the 6-char form.
            let replaced = match c {
                't'  => Some("u0009"),
                'n'  => Some("u000a"),
                'r'  => Some("u000d"),
                'b'  => Some("u0008"),
                'f'  => Some("u000c"),
                _    => None,
            };
            if let Some(r) = replaced {
                out.push_str(r);
            } else {
                out.push(c);
            }
            prev_backslash = false;
            i += 1;
            continue;
        }
        if c == '\\' {
            out.push(c);
            prev_backslash = true;
            i += 1;
            continue;
        }
        if c == '"' {
            in_string = false;
            out.push(c);
            i += 1;
            continue;
        }
        if c == '/' {
            out.push_str("\\/");
            i += 1;
            continue;
        }
        if (c as u32) > 0x7f {
            // Escape non-ASCII as \uXXXX. Supplementary planes (> U+FFFF)
            // need a UTF-16 surrogate pair.
            let cp = c as u32;
            if cp <= 0xFFFF {
                out.push_str(&format!("\\u{cp:04x}"));
            } else {
                let v = cp - 0x10000;
                let hi = 0xD800 + (v >> 10);
                let lo = 0xDC00 + (v & 0x3FF);
                out.push_str(&format!("\\u{hi:04x}\\u{lo:04x}"));
            }
            i += 1;
            continue;
        }
        out.push(c);
        i += 1;
    }
    out
}

/// Replace `"__RAWNUM__<digits>"` sentinel strings with bare number literals
/// in JSON output. Used for whole-valued floats that don't fit in i64.
fn unwrap_raw_num_markers(s: &str) -> String {
    const MARKER: &str = "\"__RAWNUM__";
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    let bytes = s.as_bytes();
    while i < bytes.len() {
        if s[i..].starts_with(MARKER) {
            // Find the closing quote.
            let start = i + MARKER.len();
            let end_quote = s[start..].find('"');
            if let Some(rel_end) = end_quote {
                let digits = &s[start..start + rel_end];
                // Validate: optional `-` then digits.
                let is_num = !digits.is_empty()
                    && digits.chars().enumerate().all(|(j, c)| {
                        c.is_ascii_digit() || (j == 0 && c == '-')
                    });
                if is_num {
                    out.push_str(digits);
                    i = start + rel_end + 1;
                    continue;
                }
            }
        }
        let c = s[i..].chars().next().unwrap();
        out.push(c);
        i += c.len_utf8();
    }
    out
}

/// PQ ISO-8601 form: "P[<days>D][T[<h>H][<m>M][<s>S]]". Zero is "PT0S".
fn duration_to_iso(d: chrono::Duration) -> String {
    let total = d.num_seconds();
    if total == 0 { return "PT0S".to_string(); }
    let neg = total < 0;
    let abs = total.unsigned_abs() as i64;
    let days = abs / 86400;
    let rem = abs % 86400;
    let hours = rem / 3600;
    let minutes = (rem / 60) % 60;
    let seconds = rem % 60;
    let mut out = String::new();
    if neg { out.push('-'); }
    out.push('P');
    if days > 0 { out.push_str(&format!("{days}D")); }
    if hours > 0 || minutes > 0 || seconds > 0 {
        out.push('T');
        if hours > 0 { out.push_str(&format!("{hours}H")); }
        if minutes > 0 { out.push_str(&format!("{minutes}M")); }
        if seconds > 0 { out.push_str(&format!("{seconds}S")); }
    }
    if days == 0 && hours == 0 && minutes == 0 && seconds == 0 {
        out.push_str("T0S");
    }
    out
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
            if n.fract() == 0.0 && n.abs() < (i64::MAX as f64) {
                Ok(serde_json::Value::Number((*n as i64).into()))
            } else if n.fract() == 0.0 && n.abs() < 1e21 {
                // Whole-valued but exceeds i64 range — encode as a string
                // sentinel that pq_jsonify will unwrap back to a bare number.
                Ok(serde_json::Value::String(format!("__RAWNUM__{:.0}", n)))
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
        Value::Datetime(dt) => {
            // PQ emits fractional seconds only when non-zero, trailing zeros
            // trimmed (e.g. "10:30:00.5", not "10:30:00.500000").
            let s = if dt.nanosecond() == 0 {
                dt.format("%Y-%m-%dT%H:%M:%S").to_string()
            } else {
                let mut s = dt.format("%Y-%m-%dT%H:%M:%S%.f").to_string();
                while s.ends_with('0') { s.pop(); }
                if s.ends_with('.') { s.pop(); }
                s
            };
            Ok(serde_json::Value::String(s))
        }
        Value::Datetimezone(dt) => {
            // PQ uses "Z" for zero offset (UTC), not "+00:00". chrono's
            // to_rfc3339 emits "+00:00" — patch the trailing offset.
            let s = dt.to_rfc3339();
            let s = if let Some(stripped) = s.strip_suffix("+00:00") {
                format!("{stripped}Z")
            } else {
                s
            };
            Ok(serde_json::Value::String(s))
        }
        Value::Time(t) => Ok(serde_json::Value::String(t.to_string())),
        Value::Duration(d) => {
            // ISO-8601 form. PQ emits "P[nD][T[nH][nM][nS]]". Negative durations
            // are prefixed with '-'.
            Ok(serde_json::Value::String(duration_to_iso(*d)))
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
        Value::Function(_) | Value::Type(_) => Err(MError::Other(
            "We can't convert values of this type to JSON.".into(),
        )),
        Value::Thunk(_) => Err(MError::Other(
            "Json.FromValue: thunk should have been forced before serialisation".into(),
        )),
        Value::WithMetadata { inner, meta } => {
            // A cell-error marker that wasn't caught by Table.ReplaceErrorValues
            // should propagate as a PQ error on access.
            if meta.fields.iter().any(|(k, _)| k == "__cell_error") {
                let msg = match inner.as_ref() {
                    Value::Record(r) => r.fields.iter()
                        .find(|(k, _)| k == "Message")
                        .and_then(|(_, v)| if let Value::Text(s) = v { Some(s.clone()) } else { None })
                        .unwrap_or_else(|| "cell error".into()),
                    _ => "cell error".into(),
                };
                return Err(MError::Other(msg));
            }
            value_to_json(inner)
        }
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
            Value::list_of(xs.into_iter().map(json_to_value).collect())
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
