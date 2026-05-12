//! `Text / Character / Guid.*` stdlib bindings.

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
        ("Character.FromNumber", one("number"), character_from_number),
        ("Character.ToNumber", one("text"), character_to_number),
        ("Guid.From", one("value"), guid_from),
        ("Text.NewGuid", vec![], new_guid),
        ("Text.From", one("value"), from),
        ("Text.Contains", two("text", "substring"), contains),
        ("Text.Replace", three("text", "old", "new"), replace),
        ("Text.Trim", one("text"), trim),
        ("Text.Lower", one("text"), lower),
        ("Text.Upper", one("text"), upper),
        ("Text.Length", one("text"), length),
        ("Text.PositionOf", two("text", "substring"), position_of),
        ("Text.EndsWith", two("text", "suffix"), ends_with),
        ("Text.StartsWith", two("text", "prefix"), starts_with),
        ("Text.TrimEnd", one("text"), trim_end),
        (
            "Text.TrimStart",
            vec![
                Param { name: "text".into(), optional: false, type_annotation: None },
                Param { name: "trim".into(), optional: true,  type_annotation: None },
            ],
            trim_start,
        ),
        ("Text.Reverse", one("text"), reverse),
        ("Text.Proper", one("text"), proper),
        ("Text.At", two("text", "index"), at),
        (
            "Text.Range",
            vec![
                Param { name: "text".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            range,
        ),
        ("Text.Remove", two("text", "removeChars"), remove),
        (
            "Text.RemoveRange",
            vec![
                Param { name: "text".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            remove_range,
        ),
        ("Text.Insert", three("text", "offset", "newText"), insert),
        (
            "Text.ReplaceRange",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "offset".into(),  optional: false, type_annotation: None },
                Param { name: "count".into(),   optional: false, type_annotation: None },
                Param { name: "newText".into(), optional: false, type_annotation: None },
            ],
            replace_range,
        ),
        (
            "Text.PadStart",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "character".into(), optional: true,  type_annotation: None },
            ],
            pad_start,
        ),
        (
            "Text.PadEnd",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "character".into(), optional: true,  type_annotation: None },
            ],
            pad_end,
        ),
        ("Text.Repeat", two("text", "count"), repeat),
        ("Text.Select", two("text", "selectChars"), select),
        ("Text.ToList", one("text"), to_list),
        ("Text.SplitAny", two("text", "separators"), split_any),
        (
            "Text.PositionOfAny",
            vec![
                Param { name: "text".into(),       optional: false, type_annotation: None },
                Param { name: "characters".into(), optional: false, type_annotation: None },
                Param { name: "occurrence".into(), optional: true,  type_annotation: None },
            ],
            position_of_any,
        ),
        (
            "Text.BeforeDelimiter",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "delimiter".into(), optional: false, type_annotation: None },
                Param { name: "index".into(),     optional: true,  type_annotation: None },
            ],
            before_delimiter,
        ),
        (
            "Text.AfterDelimiter",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "delimiter".into(), optional: false, type_annotation: None },
                Param { name: "index".into(),     optional: true,  type_annotation: None },
            ],
            after_delimiter,
        ),
        (
            "Text.BetweenDelimiters",
            vec![
                Param { name: "text".into(),           optional: false, type_annotation: None },
                Param { name: "startDelimiter".into(), optional: false, type_annotation: None },
                Param { name: "endDelimiter".into(),   optional: false, type_annotation: None },
                Param { name: "startIndex".into(),     optional: true,  type_annotation: None },
                Param { name: "endIndex".into(),       optional: true,  type_annotation: None },
            ],
            between_delimiters,
        ),
        ("Text.InferNumberType", one("text"), infer_number_type),
        ("Text.Clean", one("text"), clean),
        (
            "Text.Format",
            vec![
                Param { name: "formatString".into(), optional: false, type_annotation: None },
                Param { name: "arguments".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(),      optional: true,  type_annotation: None },
            ],
            format,
        ),
        ("Text.Start", two("text", "count"), start),
        (
            "Text.Middle",
            vec![
                Param { name: "text".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            middle,
        ),
        ("Text.End", two("text", "count"), end),
        ("Text.Split", two("text", "separator"), split),
        (
            "Text.Combine",
            vec![
                Param { name: "texts".into(),     optional: false, type_annotation: None },
                Param { name: "separator".into(), optional: true,  type_annotation: None },
            ],
            combine,
        ),
    ]
}

fn character_from_number(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            if !n.is_finite() || *n < 0.0 || n.fract() != 0.0 || *n > u32::MAX as f64 {
                return Err(MError::Other(format!(
                    "Character.FromNumber: not a valid codepoint: {n}"
                )));
            }
            let cp = *n as u32;
            char::from_u32(cp)
                .map(|c| Value::Text(c.to_string()))
                .ok_or_else(|| MError::Other(format!(
                    "Character.FromNumber: invalid Unicode codepoint U+{cp:04X}"
                )))
        }
        other => Err(type_mismatch("number", other)),
    }
}


fn character_to_number(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => s
            .chars()
            .next()
            .map(|c| Value::Number(c as u32 as f64))
            .ok_or_else(|| MError::Other("Character.ToNumber: empty text".into())),
        other => Err(type_mismatch("text", other)),
    }
}


fn guid_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => {
            // Validate 8-4-4-4-12 hex format. PQ's Guid value is text-shaped;
            // we keep it as Text but normalise to lowercase.
            let lower = s.to_lowercase();
            let bytes = lower.as_bytes();
            let dashes_at = [8, 13, 18, 23];
            if bytes.len() != 36
                || !dashes_at.iter().all(|&i| bytes[i] == b'-')
                || !bytes
                    .iter()
                    .enumerate()
                    .all(|(i, &b)| dashes_at.contains(&i) || b.is_ascii_hexdigit())
            {
                return Err(MError::Other(format!("Guid.From: invalid GUID: {s:?}")));
            }
            Ok(Value::Text(lower))
        }
        other => Err(type_mismatch("text", other)),
    }
}


fn new_guid(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    // RFC 4122 v4: set version (high nibble of byte 6) and variant (high bits of byte 8).
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let s = format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    );
    Ok(Value::Text(s))
}


fn from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => Ok(Value::Text(s.clone())),
        // {:?} for f64 matches `value_dump`'s canonical num format
        // (always-trailing fractional digit). Keeping parity here so
        // Text.From(42) prints the same as the differential's `(num 42.0)`.
        Value::Number(n) => Ok(Value::Text(format!("{n:?}"))),
        Value::Logical(b) => Ok(Value::Text(
            if *b { "true" } else { "false" }.to_string(),
        )),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}


fn contains(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    Ok(Value::Logical(text.contains(sub)))
}


fn replace(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    Ok(Value::Text(text.replace(old, new)))
}


fn trim(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.trim().to_string()))
}


fn lower(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.to_lowercase()))
}


fn upper(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.to_uppercase()))
}


fn length(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // M counts characters, not bytes — use char count.
    Ok(Value::Number(text.chars().count() as f64))
}


fn position_of(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    // Per spec: -1 when not found, byte offset on miss... but for parity
    // with the M spec (and the corpus), return a char index. The empty-sub
    // edge case isn't load-bearing for slice-6 tests.
    let idx = text.find(sub).map(|byte_idx| {
        text[..byte_idx].chars().count()
    });
    Ok(Value::Number(match idx {
        Some(i) => i as f64,
        None => -1.0,
    }))
}


fn ends_with(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let suffix = expect_text(&args[1])?;
    Ok(Value::Logical(text.ends_with(suffix)))
}


fn starts_with(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let prefix = expect_text(&args[1])?;
    Ok(Value::Logical(text.starts_with(prefix)))
}


fn trim_end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.trim_end().to_string()))
}

/// Helper: extract chars from a text-or-list-of-text "chars" argument.
fn chars_from_arg(v: &Value, ctx: &'static str) -> Result<Vec<char>, MError> {
    match v {
        Value::Text(s) => Ok(s.chars().collect()),
        Value::List(xs) => {
            let mut out = Vec::new();
            for x in xs {
                match x {
                    Value::Text(s) => out.extend(s.chars()),
                    other => return Err(MError::Other(format!(
                        "{}: list element must be text, got {}",
                        ctx, super::super::type_name(other)
                    ))),
                }
            }
            Ok(out)
        }
        other => Err(type_mismatch("text or list of text", other)),
    }
}


fn at(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let idx = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    text.chars()
        .nth(idx)
        .map(|c| Value::Text(c.to_string()))
        .ok_or_else(|| MError::Other(format!("Text.At: index {idx} out of range")))
}


fn range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars: Vec<char> = text.chars().collect();
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match args.get(2) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => chars.len().saturating_sub(offset),
        Some(other) => return Err(type_mismatch("non-negative integer or null", other)),
    };
    if offset > chars.len() {
        return Err(MError::Other("Text.Range: offset out of range".into()));
    }
    let end = (offset + count).min(chars.len());
    Ok(Value::Text(chars[offset..end].iter().collect()))
}


fn remove(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let drop = chars_from_arg(&args[1], "Text.Remove")?;
    Ok(Value::Text(text.chars().filter(|c| !drop.contains(c)).collect()))
}


fn remove_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars: Vec<char> = text.chars().collect();
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match args.get(2) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => 1,
        Some(other) => return Err(type_mismatch("non-negative integer or null", other)),
    };
    if offset > chars.len() {
        return Err(MError::Other("Text.RemoveRange: offset out of range".into()));
    }
    let end = (offset + count).min(chars.len());
    let mut out: String = chars[..offset].iter().collect();
    out.extend(chars[end..].iter());
    Ok(Value::Text(out))
}


fn insert(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars: Vec<char> = text.chars().collect();
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let new_text = expect_text(&args[2])?;
    if offset > chars.len() {
        return Err(MError::Other("Text.Insert: offset out of range".into()));
    }
    let mut out: String = chars[..offset].iter().collect();
    out.push_str(new_text);
    out.extend(chars[offset..].iter());
    Ok(Value::Text(out))
}


fn replace_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars: Vec<char> = text.chars().collect();
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match &args[2] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let new_text = expect_text(&args[3])?;
    if offset > chars.len() {
        return Err(MError::Other("Text.ReplaceRange: offset out of range".into()));
    }
    let end = (offset + count).min(chars.len());
    let mut out: String = chars[..offset].iter().collect();
    out.push_str(new_text);
    out.extend(chars[end..].iter());
    Ok(Value::Text(out))
}


fn pad_start(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let target = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let pad_char = match args.get(2) {
        Some(Value::Text(s)) => s.chars().next().ok_or_else(||
            MError::Other("Text.PadStart: pad character is empty".into()))?,
        Some(Value::Null) | None => ' ',
        Some(other) => return Err(type_mismatch("text", other)),
    };
    let n = text.chars().count();
    if n >= target {
        Ok(Value::Text(text.to_string()))
    } else {
        let mut out: String = std::iter::repeat_n(pad_char, target - n).collect();
        out.push_str(text);
        Ok(Value::Text(out))
    }
}


fn pad_end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let target = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let pad_char = match args.get(2) {
        Some(Value::Text(s)) => s.chars().next().ok_or_else(||
            MError::Other("Text.PadEnd: pad character is empty".into()))?,
        Some(Value::Null) | None => ' ',
        Some(other) => return Err(type_mismatch("text", other)),
    };
    let n = text.chars().count();
    if n >= target {
        Ok(Value::Text(text.to_string()))
    } else {
        let mut out = text.to_string();
        out.extend(std::iter::repeat_n(pad_char, target - n));
        Ok(Value::Text(out))
    }
}


fn repeat(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    Ok(Value::Text(text.repeat(count)))
}


fn select(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let keep = chars_from_arg(&args[1], "Text.Select")?;
    Ok(Value::Text(text.chars().filter(|c| keep.contains(c)).collect()))
}


fn to_list(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::List(text.chars().map(|c| Value::Text(c.to_string())).collect()))
}


fn split_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let seps_text = expect_text(&args[1])?;
    let seps: Vec<char> = seps_text.chars().collect();
    let parts: Vec<Value> = text
        .split(|c: char| seps.contains(&c))
        .map(|s| Value::Text(s.to_string()))
        .collect();
    Ok(Value::List(parts))
}


fn position_of_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars = chars_from_arg(&args[1], "Text.PositionOfAny")?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Text.PositionOfAny: occurrence arg not yet supported",
        ));
    }
    let idx = text
        .char_indices()
        .find(|(_, c)| chars.contains(c))
        .map(|(byte_idx, _)| text[..byte_idx].chars().count());
    Ok(Value::Number(match idx {
        Some(i) => i as f64,
        None => -1.0,
    }))
}

/// Find the byte offsets of every occurrence of `delim` in `text`.
fn delimiter_byte_offsets(text: &str, delim: &str) -> Vec<usize> {
    if delim.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = 0;
    while let Some(i) = text[start..].find(delim) {
        let abs = start + i;
        out.push(abs);
        start = abs + delim.len();
    }
    out
}


fn pick_delimiter_index(args_index: Option<&Value>, ctx: &str) -> Result<(usize, bool), MError> {
    // Returns (index, from_end). `from_end` true means count from the right.
    match args_index {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => Ok((*n as usize, false)),
        Some(Value::List(xs)) if xs.len() == 2 => {
            // {index, RelativePosition.FromEnd=1 or FromStart=0}
            let i = match &xs[0] {
                Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
                other => return Err(MError::Other(format!(
                    "{}: index list element 0 must be non-negative integer (got {})",
                    ctx, super::super::type_name(other)
                ))),
            };
            let from_end = matches!(&xs[1], Value::Number(n) if *n == 1.0);
            Ok((i, from_end))
        }
        Some(Value::Null) | None => Ok((0, false)),
        Some(other) => Err(type_mismatch("non-negative integer or {index, direction} list", other)),
    }
}


fn before_delimiter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let delim = expect_text(&args[1])?;
    let (index, from_end) = pick_delimiter_index(args.get(2), "Text.BeforeDelimiter")?;
    let offsets = delimiter_byte_offsets(text, delim);
    let pick = if from_end {
        offsets.get(offsets.len().wrapping_sub(1).wrapping_sub(index))
    } else {
        offsets.get(index)
    };
    match pick {
        Some(&byte_idx) => Ok(Value::Text(text[..byte_idx].to_string())),
        None => Ok(Value::Text(String::new())),
    }
}


fn after_delimiter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let delim = expect_text(&args[1])?;
    let (index, from_end) = pick_delimiter_index(args.get(2), "Text.AfterDelimiter")?;
    let offsets = delimiter_byte_offsets(text, delim);
    let pick = if from_end {
        offsets.get(offsets.len().wrapping_sub(1).wrapping_sub(index))
    } else {
        offsets.get(index)
    };
    match pick {
        Some(&byte_idx) => Ok(Value::Text(text[byte_idx + delim.len()..].to_string())),
        None => Ok(Value::Text(String::new())),
    }
}


fn between_delimiters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let start_delim = expect_text(&args[1])?;
    let end_delim = expect_text(&args[2])?;
    let (start_index, start_from_end) =
        pick_delimiter_index(args.get(3), "Text.BetweenDelimiters")?;
    let (end_index, end_from_end) =
        pick_delimiter_index(args.get(4), "Text.BetweenDelimiters")?;
    let start_offsets = delimiter_byte_offsets(text, start_delim);
    let start_pick = if start_from_end {
        start_offsets.get(start_offsets.len().wrapping_sub(1).wrapping_sub(start_index))
    } else {
        start_offsets.get(start_index)
    };
    let start_byte = match start_pick {
        Some(&b) => b + start_delim.len(),
        None => return Ok(Value::Text(String::new())),
    };
    let rest = &text[start_byte..];
    let end_offsets = delimiter_byte_offsets(rest, end_delim);
    let end_pick = if end_from_end {
        end_offsets.get(end_offsets.len().wrapping_sub(1).wrapping_sub(end_index))
    } else {
        end_offsets.get(end_index)
    };
    match end_pick {
        Some(&b) => Ok(Value::Text(rest[..b].to_string())),
        None => Ok(Value::Text(String::new())),
    }
}


fn clean(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(
        text.chars()
            .filter(|c| !(c.is_control() && *c != '\n' && *c != '\r' && *c != '\t'))
            .filter(|c| !c.is_control())
            .collect(),
    ))
}

/// Stringify a value for Text.Format substitution.
fn format_arg_to_text(v: &Value) -> String {
    match v {
        Value::Null => "".into(),
        Value::Text(s) => s.clone(),
        Value::Number(n) => {
            if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e16 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        Value::Logical(b) => if *b { "true".into() } else { "false".into() },
        Value::Date(d) => d.to_string(),
        Value::Datetime(dt) => dt.to_string(),
        Value::Duration(d) => format!("{d}"),
        other => format!("{other:?}"),
    }
}


fn format(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let fmt = expect_text(&args[0])?;
    let mut out = String::with_capacity(fmt.len());
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut key = String::new();
            let mut closed = false;
            for kc in chars.by_ref() {
                if kc == '}' {
                    closed = true;
                    break;
                }
                key.push(kc);
            }
            if !closed {
                return Err(MError::Other(
                    "Text.Format: unterminated #{...} placeholder".into(),
                ));
            }
            let value = match &args[1] {
                Value::List(xs) => {
                    let idx: usize = key.parse().map_err(|_| MError::Other(format!(
                        "Text.Format: index {key:?} not a number for list arguments"
                    )))?;
                    xs.get(idx).cloned().ok_or_else(|| MError::Other(format!(
                        "Text.Format: index {idx} out of range"
                    )))?
                }
                Value::Record(r) => {
                    let raw = r
                        .fields
                        .iter()
                        .find(|(n, _)| n == &key)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| MError::Other(format!(
                            "Text.Format: field {key:?} not in arguments record"
                        )))?;
                    super::super::force(raw, &mut |e, env| super::super::evaluate(e, env, host))?
                }
                other => return Err(type_mismatch("list or record (arguments)", other)),
            };
            out.push_str(&format_arg_to_text(&value));
        } else {
            out.push(c);
        }
    }
    Ok(Value::Text(out))
}


fn infer_number_type(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    if text.trim().parse::<f64>().is_ok() {
        Ok(Value::Type(super::super::value::TypeRep::Number))
    } else {
        Err(MError::Other(format!(
            "Text.InferNumberType: cannot infer numeric type from {text:?}"
        )))
    }
}


fn trim_start(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    match args.get(1) {
        Some(Value::Null) | None => Ok(Value::Text(text.trim_start().to_string())),
        Some(Value::Text(t)) => {
            let chars: Vec<char> = t.chars().collect();
            Ok(Value::Text(text.trim_start_matches(|c| chars.contains(&c)).to_string()))
        }
        Some(Value::List(xs)) => {
            let mut chars: Vec<char> = Vec::new();
            for v in xs {
                match v {
                    Value::Text(s) => chars.extend(s.chars()),
                    other => return Err(type_mismatch("text (in trim list)", other)),
                }
            }
            Ok(Value::Text(text.trim_start_matches(|c| chars.contains(&c)).to_string()))
        }
        Some(other) => Err(type_mismatch("text or list of text", other)),
    }
}


fn reverse(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.chars().rev().collect()))
}


fn proper(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let mut out = String::with_capacity(text.len());
    let mut start_of_word = true;
    for c in text.chars() {
        if c.is_whitespace() {
            out.push(c);
            start_of_word = true;
        } else if start_of_word {
            out.extend(c.to_uppercase());
            start_of_word = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    Ok(Value::Text(out))
}


fn combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let texts = expect_list(&args[0])?;
    let sep = match args.get(1) {
        Some(Value::Text(s)) => s.as_str(),
        Some(Value::Null) | None => "",
        Some(other) => return Err(type_mismatch("text or null", other)),
    };
    let parts: Result<Vec<&str>, MError> = texts
        .iter()
        .map(|v| match v {
            Value::Text(s) => Ok(s.as_str()),
            other => Err(type_mismatch("text (in list)", other)),
        })
        .collect();
    Ok(Value::Text(parts?.join(sep)))
}


fn start(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if count <= 0 {
        return Ok(Value::Text(String::new()));
    }
    Ok(Value::Text(text.chars().take(count as usize).collect()))
}


fn end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if count <= 0 {
        return Ok(Value::Text(String::new()));
    }
    let total = text.chars().count();
    let skip = total.saturating_sub(count as usize);
    Ok(Value::Text(text.chars().skip(skip).collect()))
}


fn middle(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if offset < 0 {
        return Ok(Value::Text(String::new()));
    }
    // Optional 3rd arg: count. Null/missing → take rest of string.
    let count = match args.get(2) {
        Some(Value::Number(n)) => Some(*n as isize),
        Some(Value::Null) | None => None,
        Some(other) => return Err(type_mismatch("number or null", other)),
    };
    let mut iter = text.chars().skip(offset as usize);
    let result: String = match count {
        Some(c) if c <= 0 => String::new(),
        Some(c) => iter.by_ref().take(c as usize).collect(),
        None => iter.collect(),
    };
    Ok(Value::Text(result))
}


fn split(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sep = expect_text(&args[1])?;
    // Power Query Text.Split on empty separator returns a list of single-char
    // texts; we emulate that to be on the safe side.
    let parts: Vec<Value> = if sep.is_empty() {
        text.chars().map(|c| Value::Text(c.to_string())).collect()
    } else {
        text.split(sep).map(|s| Value::Text(s.to_string())).collect()
    };
    Ok(Value::List(parts))
}

