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
use super::comparer::upper_invariant_non_expanding;
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
        (
            "Text.FromBinary",
            vec![
                Param { name: "binary".into(),   optional: false, type_annotation: None },
                Param { name: "encoding".into(), optional: true,  type_annotation: None },
            ],
            from_binary,
        ),
        (
            "Text.ToBinary",
            vec![
                Param { name: "text".into(),                 optional: false, type_annotation: None },
                Param { name: "encoding".into(),             optional: true,  type_annotation: None },
                Param { name: "includeByteOrderMark".into(), optional: true,  type_annotation: None },
            ],
            to_binary,
        ),
        (
            "Text.Contains",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "substring".into(), optional: false, type_annotation: None },
                Param { name: "comparer".into(),  optional: true,  type_annotation: None },
            ],
            contains,
        ),
        ("Text.Replace", three("text", "old", "new"), replace),
        (
            "Text.Trim",
            vec![
                Param { name: "text".into(), optional: false, type_annotation: None },
                Param { name: "trim".into(), optional: true,  type_annotation: None },
            ],
            trim,
        ),
        (
            "Text.Lower",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            lower,
        ),
        (
            "Text.Upper",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            upper,
        ),
        ("Text.Length", one("text"), length),
        (
            "Text.PositionOf",
            vec![
                Param { name: "text".into(),       optional: false, type_annotation: None },
                Param { name: "substring".into(),  optional: false, type_annotation: None },
                Param { name: "occurrence".into(), optional: true,  type_annotation: None },
                Param { name: "comparer".into(),   optional: true,  type_annotation: None },
            ],
            position_of,
        ),
        (
            "Text.EndsWith",
            vec![
                Param { name: "text".into(),     optional: false, type_annotation: None },
                Param { name: "suffix".into(),   optional: false, type_annotation: None },
                Param { name: "comparer".into(), optional: true,  type_annotation: None },
            ],
            ends_with,
        ),
        (
            "Text.StartsWith",
            vec![
                Param { name: "text".into(),     optional: false, type_annotation: None },
                Param { name: "prefix".into(),   optional: false, type_annotation: None },
                Param { name: "comparer".into(), optional: true,  type_annotation: None },
            ],
            starts_with,
        ),
        (
            "Text.TrimEnd",
            vec![
                Param { name: "text".into(), optional: false, type_annotation: None },
                Param { name: "trim".into(), optional: true,  type_annotation: None },
            ],
            trim_end,
        ),
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


fn from_binary(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bytes = match &args[0] {
        Value::Binary(b) => b,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("binary", other)),
    };
    // encoding: only UTF-8 (65001) or null/missing accepted. Other code
    // pages would silently mis-decode without a fallback library, so we
    // refuse rather than guess. PQ's `TextEncoding.Utf8` is 65001.
    check_utf8_encoding(args.get(1), "Text.FromBinary")?;
    let s = std::str::from_utf8(bytes).map_err(|e| {
        MError::Other(format!("Text.FromBinary: invalid UTF-8: {e}"))
    })?;
    Ok(Value::Text(s.to_string()))
}

fn to_binary(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let s = match &args[0] {
        Value::Text(s) => s,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("text", other)),
    };
    // PQ Text.ToBinary tolerates any encoding number (treats unknowns as a
    // pass-through to UTF-8 bytes). Only reject if a non-number/non-null
    // is supplied — matches PQ's softer enforcement.
    match args.get(1) {
        None | Some(Value::Null) | Some(Value::Number(_)) => {}
        Some(other) => return Err(type_mismatch("number or null", other)),
    }
    Ok(Value::Binary(s.as_bytes().to_vec()))
}

fn check_utf8_encoding(arg: Option<&Value>, fname: &str) -> Result<(), MError> {
    match arg {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Number(n)) if *n == 65001.0 => Ok(()),
        Some(Value::Number(n)) => Err(MError::Other(format!(
            "{fname}: Encoding={n} not supported (only 65001/UTF-8)"
        ))),
        Some(other) => Err(type_mismatch("number or null", other)),
    }
}

fn from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::{Datelike, Timelike};
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => Ok(Value::Text(s.clone())),
        // Render integer-valued f64 without the `.0` to match PQ:
        // `Text.From(42)` is "42", not "42.0". Same approach
        // Json.FromValue uses for the same reason. (Oracle case q27
        // was the catalog row that flagged this.) Non-integer floats
        // keep default Rust formatting.
        Value::Number(n) => {
            // PQ collapses negative zero to "0" (matches .NET's
            // Double.ToString convention). Detect -0 before downstream
            // formatting which would otherwise emit "-0".
            let n_normalised = if *n == 0.0 { 0.0 } else { *n };
            let text = if n_normalised.is_nan() {
                "NaN".to_string()
            } else if n_normalised.is_infinite() {
                if n_normalised > 0.0 { "∞".to_string() } else { "-∞".to_string() }
            } else if n_normalised.fract() == 0.0 && n_normalised.abs() < 1e21 {
                // Whole-valued — render as integer text without `.0`, even for
                // values above 2^53 where f64 loses 1-digit precision.
                format!("{:.0}", n_normalised)
            } else {
                format!("{n_normalised}")
            };
            Ok(Value::Text(text))
        }
        Value::Logical(b) => Ok(Value::Text(
            if *b { "true" } else { "false" }.to_string(),
        )),
        // PQ Text.From for date/datetime/time uses the current culture's short
        // form. Corpus runs en-GB → dd/MM/yyyy for dates.
        Value::Date(d) => Ok(Value::Text(format!("{:02}/{:02}/{:04}", d.day(), d.month(), d.year()))),
        Value::Datetime(dt) => Ok(Value::Text(format!(
            "{:02}/{:02}/{:04} {:02}:{:02}:{:02}",
            dt.day(), dt.month(), dt.year(), dt.hour(), dt.minute(), dt.second()
        ))),
        Value::Time(t) => Ok(Value::Text(format!("{:02}:{:02}:{:02}", t.hour(), t.minute(), t.second()))),
        Value::Duration(d) => {
            let total = d.num_seconds();
            let days = total / 86400;
            let rem = total.rem_euclid(86400);
            let h = rem / 3600;
            let m = (rem / 60) % 60;
            let s = rem % 60;
            Ok(Value::Text(if days != 0 {
                format!("{days}.{h:02}:{m:02}:{s:02}")
            } else {
                format!("{h:02}:{m:02}:{s:02}")
            }))
        }
        // PQ rejects List/Record/Table/Function/Type in Text.From with a clear
        // "cannot convert" error.
        Value::List(_) | Value::Record(_) | Value::Table(_)
        | Value::Function(_) | Value::Type(_) => {
            // PQ message wording, capitalised type name.
            let t = match v {
                Value::List(_) => "List",
                Value::Record(_) => "Record",
                Value::Table(_) => "Table",
                Value::Function(_) => "Function",
                Value::Type(_) => "Type",
                _ => unreachable!(),
            };
            Err(MError::Other(format!(
                "We cannot convert a value of type {t} to type Text."
            )))
        }
        other => Err(type_mismatch("text/number/logical/date/null", other)),
    }
}


fn contains(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    if comparer_is_ordinal_ignore_case(args.get(2), host)? {
        Ok(Value::Logical(
            upper_invariant_non_expanding(text).contains(&upper_invariant_non_expanding(sub))
        ))
    } else {
        Ok(Value::Logical(text.contains(sub)))
    }
}


fn replace(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    // PQ: empty old → no-op (avoid the str::replace behaviour of inserting
    // `new` between every char and at both ends).
    if old.is_empty() {
        return Ok(Value::Text(text.to_string()));
    }
    Ok(Value::Text(text.replace(old, new)))
}


fn trim(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let text = expect_text(&args[0])?;
    match args.get(1) {
        Some(Value::Null) | None => Ok(Value::Text(text.trim().to_string())),
        Some(v) => {
            let chars = chars_from_arg(v, "Text.Trim")?;
            Ok(Value::Text(text.trim_matches(|c| chars.contains(&c)).to_string()))
        }
    }
}


fn lower(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let text = expect_text(&args[0])?;
    let culture = culture_arg(args.get(1), "Text.Lower")?;
    Ok(Value::Text(case_map(text, culture, false)))
}


fn upper(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let text = expect_text(&args[0])?;
    let culture = culture_arg(args.get(1), "Text.Upper")?;
    Ok(Value::Text(case_map(text, culture, true)))
}

fn culture_arg(v: Option<&Value>, ctx: &str) -> Result<Option<String>, MError> {
    match v {
        None | Some(Value::Null) => Ok(None),
        Some(Value::Text(s)) => Ok(Some(s.clone())),
        Some(other) => Err(MError::Other(format!(
            "{ctx}: culture must be text (got {})", super::super::type_name(other)
        ))),
    }
}

fn case_map(text: &str, culture: Option<String>, upper: bool) -> String {
    let turkish = matches!(culture.as_deref(), Some(c) if {
        let c = c.to_ascii_lowercase();
        c.starts_with("tr") || c.starts_with("az")
    });
    if !turkish {
        // PQ uses legacy .NET case mapping where ß stays ß under ToUpper
        // (not the Unicode 5.1+ behaviour that maps to "SS"), and
        // ToLower("İ") drops the combining dot to plain "i" (Rust's
        // default Unicode mapping yields "i\u{307}" which doesn't
        // match .NET non-Turkic ToLower).
        return text.chars().map(|c| -> String {
            if upper {
                match c {
                    'ß' => "ß".into(),
                    _ => c.to_uppercase().collect(),
                }
            } else {
                match c {
                    'İ' => "i".into(),
                    _ => c.to_lowercase().collect(),
                }
            }
        }).collect();
    }
    // Turkish: I↔ı (dotless), İ↔i (dotted). Char-by-char to keep the special
    // pairs intact; everything else uses default Unicode casing.
    text.chars()
        .map(|c| match (upper, c) {
            (true,  'i') => "İ".to_string(),
            (true,  'ı') => "I".to_string(),
            (false, 'I') => "ı".to_string(),
            (false, 'İ') => "i".to_string(),
            _ => if upper { c.to_uppercase().collect() } else { c.to_lowercase().collect() },
        })
        .collect()
}


fn length(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let text = expect_text(&args[0])?;
    // PQ counts UTF-16 code units (.NET String.Length), not codepoints.
    // Surrogate pairs count as 2; combining marks count as 1 each.
    Ok(Value::Number(text.encode_utf16().count() as f64))
}


fn position_of(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    let mode = parse_occurrence(args.get(2), "Text.PositionOf")?;
    let case_insensitive = comparer_is_ordinal_ignore_case(args.get(3), host)?;
    let (hay, needle): (String, String) = if case_insensitive {
        // Match .NET OrdinalIgnoreCase: per-codepoint ToUpper without
        // multi-char expansion (Rust folds ß → SS; .NET keeps ß).
        // ToLower would also normalise "İ" → "i̇" which falsely matches "i".
        (
            upper_invariant_non_expanding(text),
            upper_invariant_non_expanding(sub),
        )
    } else {
        (text.to_string(), sub.to_string())
    };
    // PQ: empty needle matches at every char position in text.
    // Non-empty text → positions 0..length. Empty text → [0] (single
    // match at position 0, observable via First/Last; All would be
    // empty but PQ's documented behaviour for "" + Occurrence.All is
    // unverified — we match the First-of-[0] interpretation).
    if needle.is_empty() {
        let n = text.chars().count();
        let positions: Vec<usize> = if n == 0 { vec![0] } else { (0..n).collect() };
        return Ok(occurrence_result(mode, &positions));
    }
    let byte_offsets = delimiter_byte_offsets(&hay, &needle);
    let char_indices: Vec<usize> = byte_offsets
        .iter()
        .map(|&b| hay[..b].chars().count())
        .collect();
    Ok(occurrence_result(mode, &char_indices))
}

/// Detect a Comparer argument that should produce case-insensitive matching.
/// Returns true only for `Comparer.OrdinalIgnoreCase` (which we recognise by
/// invoking the comparer on the pair "a"/"A" and seeing if it returns 0).
fn comparer_is_ordinal_ignore_case(arg: Option<&Value>, host: &dyn IoHost) -> Result<bool, MError> {
    use super::common::invoke_callback_with_host;
    match arg {
        None | Some(Value::Null) => Ok(false),
        Some(Value::Function(c)) => {
            let r = invoke_callback_with_host(
                c,
                vec![Value::Text("a".into()), Value::Text("A".into())],
                host,
            )?;
            Ok(matches!(r, Value::Number(n) if n == 0.0))
        }
        Some(other) => Err(type_mismatch("function (Comparer.*)", other)),
    }
}


fn ends_with(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let suffix = expect_text(&args[1])?;
    if comparer_is_ordinal_ignore_case(args.get(2), host)? {
        Ok(Value::Logical(
            upper_invariant_non_expanding(text).ends_with(&upper_invariant_non_expanding(suffix))
        ))
    } else {
        Ok(Value::Logical(text.ends_with(suffix)))
    }
}


fn starts_with(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let prefix = expect_text(&args[1])?;
    if comparer_is_ordinal_ignore_case(args.get(2), host)? {
        Ok(Value::Logical(
            upper_invariant_non_expanding(text).starts_with(&upper_invariant_non_expanding(prefix))
        ))
    } else {
        Ok(Value::Logical(text.starts_with(prefix)))
    }
}


fn trim_end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let text = expect_text(&args[0])?;
    match args.get(1) {
        Some(Value::Null) | None => Ok(Value::Text(text.trim_end().to_string())),
        Some(v) => {
            let chars = chars_from_arg(v, "Text.TrimEnd")?;
            Ok(Value::Text(text.trim_end_matches(|c| chars.contains(&c)).to_string()))
        }
    }
}

/// Helper: extract chars from a text-or-list-of-text "chars" argument.
/// List elements must each be a single-character text — PQ rejects
/// multi-char list elements (matching .NET's "must be single character").
fn chars_from_arg(v: &Value, ctx: &'static str) -> Result<Vec<char>, MError> {
    match v {
        Value::Text(s) => Ok(s.chars().collect()),
        Value::List(xs) => {
            let mut out = Vec::new();
            for x in xs.iter() {
                match x {
                    Value::Text(s) => {
                        let mut it = s.chars();
                        let c = it.next().ok_or_else(|| MError::Other(format!(
                            "{}: list element is empty (must be single character)", ctx
                        )))?;
                        if it.next().is_some() {
                            return Err(MError::Other(format!(
                                "{}: list element must be a single character, got {:?}",
                                ctx, s
                            )));
                        }
                        out.push(c);
                    }
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
    if offset + count > chars.len() {
        return Err(MError::Other("Text.Range: offset+count out of range".into()));
    }
    Ok(Value::Text(chars[offset..offset + count].iter().collect()))
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
        Some(Value::Text(s)) => {
            let mut it = s.chars();
            let c = it.next().ok_or_else(||
                MError::Other("Text.PadStart: pad character is empty".into()))?;
            if it.next().is_some() {
                return Err(MError::Other(
                    "The value isn't a single-character string.".into()));
            }
            c
        }
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
        Some(Value::Text(s)) => {
            let mut it = s.chars();
            let c = it.next().ok_or_else(||
                MError::Other("Text.PadEnd: pad character is empty".into()))?;
            if it.next().is_some() {
                return Err(MError::Other(
                    "The value isn't a single-character string.".into()));
            }
            c
        }
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
        Value::Number(n) if *n < 0.0 => return Err(MError::Other(
            "The 'count' argument is out of range.".into(),
        )),
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
    Ok(Value::list_of(text.chars().map(|c| Value::Text(c.to_string())).collect()))
}


fn split_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let seps_text = expect_text(&args[1])?;
    let seps: Vec<char> = seps_text.chars().collect();
    let parts: Vec<Value> = text
        .split(|c: char| seps.contains(&c))
        .map(|s| Value::Text(s.to_string()))
        .collect();
    Ok(Value::list_of(parts))
}


/// Occurrence mode: First (default), Last, All. Local to text.rs;
/// list.rs and table.rs each keep their own copy for the same reason.
#[derive(Copy, Clone, PartialEq)]
enum Occurrence {
    First,
    Last,
    All,
}

fn parse_occurrence(arg: Option<&Value>, fn_name: &str) -> Result<Occurrence, MError> {
    match arg {
        None | Some(Value::Null) => Ok(Occurrence::First),
        Some(Value::Number(n)) => match *n as i64 {
            0 => Ok(Occurrence::First),
            1 => Ok(Occurrence::Last),
            2 => Ok(Occurrence::All),
            k => Err(MError::Other(format!(
                "{fn_name}: occurrence must be Occurrence.First/Last/All (0/1/2), got {k}"
            ))),
        },
        Some(other) => Err(type_mismatch("number (Occurrence.*)", other)),
    }
}

fn occurrence_result(mode: Occurrence, matches: &[usize]) -> Value {
    match mode {
        Occurrence::First => Value::Number(matches.first().copied().map(|i| i as f64).unwrap_or(-1.0)),
        Occurrence::Last => Value::Number(matches.last().copied().map(|i| i as f64).unwrap_or(-1.0)),
        Occurrence::All => Value::list_of(matches.iter().map(|&i| Value::Number(i as f64)).collect()),
    }
}

fn position_of_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars = chars_from_arg(&args[1], "Text.PositionOfAny")?;
    let mode = parse_occurrence(args.get(2), "Text.PositionOfAny")?;
    let mut matches: Vec<usize> = Vec::new();
    for (char_pos, (_, c)) in text.char_indices().enumerate() {
        if chars.contains(&c) {
            matches.push(char_pos);
            if mode == Occurrence::First {
                break;
            }
        }
    }
    Ok(occurrence_result(mode, &matches))
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
        Value::Null => "null".into(),
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
            // PQ Text.Format placeholders must be numeric indices; named
            // placeholders error with PQ's standard message.
            if key.parse::<usize>().is_err() {
                return Err(MError::Other(
                    "Invalid placeholder or value in format string. Underlying error message: We expected an integer value.".into(),
                ));
            }
            let value = match &args[1] {
                Value::List(xs) => {
                    let idx: usize = key.parse().unwrap();
                    xs.get(idx).cloned().ok_or_else(|| MError::Other(
                        "Invalid placeholder or value in format string. Underlying error message: There weren't enough elements in the enumeration to complete the operation.".into()
                    ))?
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
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let text = expect_text(&args[0])?;
    match args.get(1) {
        Some(Value::Null) | None => Ok(Value::Text(text.trim_start().to_string())),
        Some(v) => {
            let chars = chars_from_arg(v, "Text.TrimStart")?;
            Ok(Value::Text(text.trim_start_matches(|c| chars.contains(&c)).to_string()))
        }
    }
}


fn reverse(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let text = expect_text(&args[0])?;
    // PQ reverses by UTF-16 code unit (.NET's string indexing model),
    // not by codepoint. Surrogate pairs get split — this matches
    // .NET's `new string(str.Reverse().ToArray())` and PQ's observed
    // behaviour on supplementary-plane chars.
    let utf16: Vec<u16> = text.encode_utf16().collect();
    let reversed: Vec<u16> = utf16.into_iter().rev().collect();
    let result = String::from_utf16_lossy(&reversed);
    Ok(Value::Text(result))
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
    let (sep, sep_present) = match args.get(1) {
        Some(Value::Text(s)) => (s.as_str(), true),
        Some(Value::Null) | None => ("", false),
        Some(other) => return Err(type_mismatch("text or null", other)),
    };
    // PQ:
    // - With separator: null elements are SKIPPED entirely (no duplicated
    //   separator from a null slot).
    // - Without separator: null elements stringify to "" (the join is
    //   indistinguishable from skipping when sep is empty, but the
    //   skip-vs-empty branching keeps the intent explicit).
    let parts: Result<Vec<&str>, MError> = texts
        .iter()
        .filter_map(|v| match v {
            Value::Text(s) => Some(Ok(s.as_str())),
            Value::Null if sep_present => None,
            Value::Null => Some(Ok("")),
            other => Some(Err(type_mismatch("text (in list)", other))),
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
    // PQ: empty separator → no split, returns the whole text as a single
    // element (or [""] for empty text).
    let parts: Vec<Value> = if sep.is_empty() {
        vec![Value::Text(text.to_string())]
    } else {
        text.split(sep).map(|s| Value::Text(s.to_string())).collect()
    };
    Ok(Value::list_of(parts))
}

