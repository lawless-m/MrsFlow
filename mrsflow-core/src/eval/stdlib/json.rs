//! `Json.Document` — parse JSON text or binary into M values.

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Value};
use super::common::type_mismatch;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![(
        "Json.Document",
        vec![
            Param { name: "jsonText".into(), optional: false, type_annotation: None },
            Param { name: "encoding".into(), optional: true,  type_annotation: None },
        ],
        document,
    )]
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
