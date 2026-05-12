//! `Lines.*` stdlib bindings.
//!
//! v1 assumes UTF-8 throughout: `quoteStyle` and `encoding` parameters are
//! accepted but ignored.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{expect_text, expect_text_list, one, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        (
            "Lines.FromText",
            vec![
                Param { name: "text".into(),       optional: false, type_annotation: None },
                Param { name: "quoteStyle".into(), optional: true,  type_annotation: None },
            ],
            from_text,
        ),
        (
            "Lines.ToText",
            vec![
                Param { name: "lines".into(),         optional: false, type_annotation: None },
                Param { name: "lineSeparator".into(), optional: true,  type_annotation: None },
            ],
            to_text,
        ),
        (
            "Lines.FromBinary",
            vec![
                Param { name: "binary".into(),                optional: false, type_annotation: None },
                Param { name: "quoteStyle".into(),            optional: true,  type_annotation: None },
                Param { name: "includeLineSeparators".into(), optional: true,  type_annotation: None },
                Param { name: "encoding".into(),              optional: true,  type_annotation: None },
            ],
            from_binary,
        ),
        (
            "Lines.ToBinary",
            vec![
                Param { name: "lines".into(),          optional: false, type_annotation: None },
                Param { name: "lineSeparator".into(),  optional: true,  type_annotation: None },
                Param { name: "lineTerminator".into(), optional: true,  type_annotation: None },
                Param { name: "encoding".into(),       optional: true,  type_annotation: None },
            ],
            to_binary,
        ),
    ]
}

fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::List(split_lines(text, false)))
}

fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lines = expect_text_list(&args[0], "Lines.ToText")?;
    let sep = match args.get(1) {
        Some(Value::Text(s)) => s.clone(),
        Some(Value::Null) | None => "\r\n".to_string(),
        Some(other) => return Err(type_mismatch("text", other)),
    };
    Ok(Value::Text(lines.join(&sep)))
}

fn from_binary(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bytes = match &args[0] {
        Value::Binary(b) => b,
        other => return Err(type_mismatch("binary", other)),
    };
    let text = std::str::from_utf8(bytes)
        .map_err(|e| MError::Other(format!("Lines.FromBinary: invalid UTF-8: {e}")))?;
    let include_seps = matches!(args.get(2), Some(Value::Logical(true)));
    Ok(Value::List(split_lines(text, include_seps)))
}

fn to_binary(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lines = expect_text_list(&args[0], "Lines.ToBinary")?;
    let sep = match args.get(1) {
        Some(Value::Text(s)) => s.clone(),
        Some(Value::Null) | None => "\r\n".to_string(),
        Some(other) => return Err(type_mismatch("text", other)),
    };
    // lineTerminator (arg 2) appends after the final line; default empty.
    let term = match args.get(2) {
        Some(Value::Text(s)) => s.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => return Err(type_mismatch("text", other)),
    };
    let mut joined = lines.join(&sep);
    joined.push_str(&term);
    Ok(Value::Binary(joined.into_bytes()))
}

/// Split `text` on \r\n / \r / \n. When `keep_seps` is true, each emitted
/// line includes the separator that terminated it.
fn split_lines(text: &str, keep_seps: bool) -> Vec<Value> {
    let mut out: Vec<Value> = Vec::new();
    let bytes = text.as_bytes();
    let mut start = 0;
    let mut i = 0;
    while i < bytes.len() {
        let (cut, sep_end) = match bytes[i] {
            b'\r' if i + 1 < bytes.len() && bytes[i + 1] == b'\n' => (i, i + 2),
            b'\r' | b'\n' => (i, i + 1),
            _ => {
                i += 1;
                continue;
            }
        };
        let end = if keep_seps { sep_end } else { cut };
        out.push(Value::Text(text[start..end].to_string()));
        start = sep_end;
        i = sep_end;
    }
    // Tail after last separator. Power Query emits this even when empty —
    // matching the "trailing newline → empty last element" behaviour.
    if start <= bytes.len() {
        out.push(Value::Text(text[start..].to_string()));
    }
    out
}
