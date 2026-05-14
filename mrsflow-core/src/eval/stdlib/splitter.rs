//! `Splitter.*` factory stdlib bindings.
//!
//! Each factory returns a `Value::Function` closure that, when applied to a
//! text, produces the split. The closure is M-bodied — its body is a
//! synthetic AST node that invokes an internal impl builtin with the user's
//! text plus the factory's captured parameters (closed-over via the
//! closure's environment).

#![allow(unused_imports)]

use crate::parser::{Expr, Param};

use super::super::env::{EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Value};
use super::common::{
    expect_int, expect_list, expect_text, expect_text_list, one, two, type_mismatch,
};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Splitter.SplitByNothing", vec![], split_by_nothing),
        (
            "Splitter.SplitTextByDelimiter",
            vec![
                Param { name: "delimiter".into(), optional: false, type_annotation: None },
                Param { name: "quoteStyle".into(), optional: true, type_annotation: None },
            ],
            split_text_by_delimiter,
        ),
        (
            "Splitter.SplitTextByAnyDelimiter",
            vec![
                Param { name: "delimiters".into(), optional: false, type_annotation: None },
                Param { name: "quoteStyle".into(), optional: true, type_annotation: None },
            ],
            split_text_by_any_delimiter,
        ),
        (
            "Splitter.SplitTextByEachDelimiter",
            vec![
                Param { name: "delimiters".into(), optional: false, type_annotation: None },
                Param { name: "quoteStyle".into(), optional: true, type_annotation: None },
                Param { name: "startAtEnd".into(), optional: true, type_annotation: None },
            ],
            split_text_by_each_delimiter,
        ),
        (
            "Splitter.SplitTextByLengths",
            vec![
                Param { name: "lengths".into(), optional: false, type_annotation: None },
                Param { name: "startAtEnd".into(), optional: true, type_annotation: None },
            ],
            split_text_by_lengths,
        ),
        (
            "Splitter.SplitTextByPositions",
            vec![
                Param { name: "positions".into(), optional: false, type_annotation: None },
                Param { name: "startAtEnd".into(), optional: true, type_annotation: None },
            ],
            split_text_by_positions,
        ),
        (
            "Splitter.SplitTextByRanges",
            vec![
                Param { name: "ranges".into(), optional: false, type_annotation: None },
                Param { name: "startAtEnd".into(), optional: true, type_annotation: None },
            ],
            split_text_by_ranges,
        ),
        (
            "Splitter.SplitTextByCharacterTransition",
            two("before", "after"),
            split_text_by_character_transition,
        ),
        (
            "Splitter.SplitTextByRepeatedLengths",
            one("length"),
            split_text_by_repeated_lengths,
        ),
        (
            "Splitter.SplitTextByWhitespace",
            vec![Param {
                name: "quoteStyle".into(),
                optional: true,
                type_annotation: None,
            }],
            split_text_by_whitespace,
        ),
    ]
}

/// Build the synthetic M-bodied closure that the user later applies to a
/// text. `captures` are bound by name in the closure's env; the body invokes
/// `impl_fn` with `text` followed by each capture value in declaration order.
fn make_splitter(captures: Vec<(String, Value)>, impl_fn: BuiltinFn) -> Value {
    let mut env = EnvNode::empty();
    let mut impl_params: Vec<Param> = vec![Param {
        name: "text".into(),
        optional: false,
        type_annotation: None,
    }];
    let mut call_args: Vec<Expr> = vec![Expr::Identifier("text".into())];
    for (k, v) in &captures {
        env = env.extend(k.clone(), v.clone());
        impl_params.push(Param {
            name: k.clone(),
            optional: false,
            type_annotation: None,
        });
        call_args.push(Expr::Identifier(k.clone()));
    }
    // Inner impl closure — synthetic name avoids collision with user idents.
    let impl_name = "__splitter_impl__".to_string();
    let impl_closure = Value::Function(Closure {
        params: impl_params,
        body: FnBody::Builtin(impl_fn),
        env: EnvNode::empty(),
    });
    env = env.extend(impl_name.clone(), impl_closure);
    let body = Expr::Invoke {
        target: Box::new(Expr::Identifier(impl_name)),
        args: call_args,
    };
    Value::Function(Closure {
        params: vec![Param {
            name: "text".into(),
            optional: false,
            type_annotation: None,
        }],
        body: FnBody::M(Box::new(body)),
        env,
    })
}

// --- Factories ---

fn split_by_nothing(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_splitter(vec![], split_by_nothing_impl))
}

fn split_text_by_delimiter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if !matches!(&args[0], Value::Text(_)) {
        return Err(type_mismatch("text", &args[0]));
    }
    let qs = parse_quote_style(args.get(1), "Splitter.SplitTextByDelimiter")?;
    Ok(make_splitter(
        vec![
            ("__delim".into(), args[0].clone()),
            ("__qs".into(), Value::Number(qs as i64 as f64)),
        ],
        split_text_by_delimiter_impl,
    ))
}

/// QuoteStyle.None = 0 (no quoting); QuoteStyle.Csv = 1 (RFC4180-style).
#[derive(Copy, Clone, PartialEq)]
enum QuoteStyle {
    None,
    Csv,
}

fn parse_quote_style(arg: Option<&Value>, fn_name: &str) -> Result<QuoteStyle, MError> {
    match arg {
        None | Some(Value::Null) => Ok(QuoteStyle::None),
        Some(Value::Number(n)) => {
            let k = *n as i64;
            match k {
                0 => Ok(QuoteStyle::None),
                1 => Ok(QuoteStyle::Csv),
                _ => Err(MError::Other(format!(
                    "{fn_name}: quoteStyle must be QuoteStyle.None (0) or QuoteStyle.Csv (1), got {k}"
                ))),
            }
        }
        Some(other) => Err(type_mismatch("number (QuoteStyle.*)", other)),
    }
}

/// CSV-aware split: outside double-quoted regions, ask `match_delim_len`
/// at each position whether a delimiter starts here (and its byte length).
/// Inside quotes, `""` is an escaped quote; surrounding quotes are stripped.
/// Quote opens only at the start of a field.
fn csv_aware_split<F>(text: &str, mut match_delim_len: F) -> Vec<String>
where
    F: FnMut(&str) -> Option<usize>,
{
    let mut parts: Vec<String> = Vec::new();
    let mut buf = String::new();
    let mut in_quote = false;
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if in_quote {
            let c = text[i..].chars().next().unwrap();
            let cl = c.len_utf8();
            if c == '"' {
                if text[i + cl..].starts_with('"') {
                    buf.push('"');
                    i += cl + 1;
                } else {
                    in_quote = false;
                    i += cl;
                }
            } else {
                buf.push(c);
                i += cl;
            }
        } else if buf.is_empty() && bytes[i] == b'"' {
            in_quote = true;
            i += 1;
        } else if let Some(dlen) = match_delim_len(&text[i..]) {
            parts.push(std::mem::take(&mut buf));
            i += dlen;
        } else {
            let c = text[i..].chars().next().unwrap();
            buf.push(c);
            i += c.len_utf8();
        }
    }
    parts.push(buf);
    parts
}

fn csv_split(text: &str, delim: &str) -> Vec<String> {
    if delim.is_empty() {
        return vec![text.to_string()];
    }
    csv_aware_split(text, |rest| {
        if rest.starts_with(delim) { Some(delim.len()) } else { None }
    })
}

fn csv_split_any(text: &str, delims: &[String]) -> Vec<String> {
    if delims.iter().all(|d| d.is_empty()) {
        return vec![text.to_string()];
    }
    csv_aware_split(text, |rest| {
        for d in delims {
            if !d.is_empty() && rest.starts_with(d.as_str()) {
                return Some(d.len());
            }
        }
        None
    })
}

fn split_text_by_any_delimiter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Validate at factory time so the error surfaces immediately.
    let _ = expect_text_list(&args[0], "Splitter.SplitTextByAnyDelimiter")?;
    let qs = parse_quote_style(args.get(1), "Splitter.SplitTextByAnyDelimiter")?;
    Ok(make_splitter(
        vec![
            ("__delims".into(), args[0].clone()),
            ("__qs".into(), Value::Number(qs as i64 as f64)),
        ],
        split_text_by_any_delimiter_impl,
    ))
}

fn split_text_by_each_delimiter(
    args: &[Value],
    _host: &dyn IoHost,
) -> Result<Value, MError> {
    let _ = expect_text_list(&args[0], "Splitter.SplitTextByEachDelimiter")?;
    let qs = parse_quote_style(args.get(1), "Splitter.SplitTextByEachDelimiter")?;
    let start_at_end = match args.get(2) {
        None | Some(Value::Null) => false,
        Some(Value::Logical(b)) => *b,
        Some(other) => return Err(type_mismatch("logical (startAtEnd)", other)),
    };
    Ok(make_splitter(
        vec![
            ("__delims".into(), args[0].clone()),
            ("__qs".into(), Value::Number(qs as i64 as f64)),
            ("__rev".into(), Value::Logical(start_at_end)),
        ],
        split_text_by_each_delimiter_impl,
    ))
}

fn split_text_by_lengths(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    for v in xs {
        let _ = expect_int(v, "Splitter.SplitTextByLengths")?;
    }
    let start_at_end = match args.get(1) {
        None | Some(Value::Null) => false,
        Some(Value::Logical(b)) => *b,
        Some(other) => return Err(type_mismatch("logical (startAtEnd)", other)),
    };
    Ok(make_splitter(
        vec![
            ("__lengths".into(), args[0].clone()),
            ("__rev".into(), Value::Logical(start_at_end)),
        ],
        split_text_by_lengths_impl,
    ))
}

fn split_text_by_positions(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    for v in xs {
        let _ = expect_int(v, "Splitter.SplitTextByPositions")?;
    }
    if matches!(args.get(1), Some(Value::Logical(true))) {
        return Err(MError::NotImplemented(
            "Splitter.SplitTextByPositions: startAtEnd=true not yet supported",
        ));
    }
    Ok(make_splitter(
        vec![("__positions".into(), args[0].clone())],
        split_text_by_positions_impl,
    ))
}

fn split_text_by_ranges(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    for r in xs {
        let pair = match r {
            Value::List(p) => p,
            other => return Err(type_mismatch("list (range pair)", other)),
        };
        if pair.len() != 2 {
            return Err(MError::Other(format!(
                "Splitter.SplitTextByRanges: range must have 2 elements, got {}",
                pair.len()
            )));
        }
        let _ = expect_int(&pair[0], "Splitter.SplitTextByRanges (offset)")?;
        let _ = expect_int(&pair[1], "Splitter.SplitTextByRanges (count)")?;
    }
    if matches!(args.get(1), Some(Value::Logical(true))) {
        return Err(MError::NotImplemented(
            "Splitter.SplitTextByRanges: startAtEnd=true not yet supported",
        ));
    }
    Ok(make_splitter(
        vec![("__ranges".into(), args[0].clone())],
        split_text_by_ranges_impl,
    ))
}

fn split_text_by_character_transition(
    args: &[Value],
    _host: &dyn IoHost,
) -> Result<Value, MError> {
    let _ = expect_text_list(&args[0], "Splitter.SplitTextByCharacterTransition (before)")?;
    let _ = expect_text_list(&args[1], "Splitter.SplitTextByCharacterTransition (after)")?;
    Ok(make_splitter(
        vec![
            ("__before".into(), args[0].clone()),
            ("__after".into(), args[1].clone()),
        ],
        split_text_by_character_transition_impl,
    ))
}

fn split_text_by_repeated_lengths(
    args: &[Value],
    _host: &dyn IoHost,
) -> Result<Value, MError> {
    let _ = expect_int(&args[0], "Splitter.SplitTextByRepeatedLengths")?;
    Ok(make_splitter(
        vec![("__length".into(), args[0].clone())],
        split_text_by_repeated_lengths_impl,
    ))
}

fn split_text_by_whitespace(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if !matches!(args.first(), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Splitter.SplitTextByWhitespace: quoteStyle not yet supported",
        ));
    }
    Ok(make_splitter(vec![], split_text_by_whitespace_impl))
}

// --- Inner impls ---
// Each receives [text, ...captured] and returns Value::List.

fn split_by_nothing_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Identity-wrap: returns a singleton list containing the input unchanged.
    // Unlike the other Splitter.* functions, this one accepts any value type
    // (it's how Table.FromList puts a list of arbitrary values into a table).
    Ok(Value::List(vec![args[0].clone()]))
}

fn split_text_by_delimiter_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let delim = expect_text(&args[1])?;
    let qs_n = match &args[2] {
        Value::Number(n) => *n as i64,
        _ => 0,
    };
    let parts: Vec<Value> = if delim.is_empty() {
        vec![Value::Text(text.to_string())]
    } else if qs_n == 1 {
        csv_split(text, delim).into_iter().map(Value::Text).collect()
    } else {
        text.split(delim).map(|s| Value::Text(s.to_string())).collect()
    };
    Ok(Value::List(parts))
}

fn split_text_by_any_delimiter_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let delims = expect_text_list(&args[1], "Splitter.SplitTextByAnyDelimiter")?;
    let qs_n = match &args[2] {
        Value::Number(n) => *n as i64,
        _ => 0,
    };
    if delims.is_empty() {
        return Ok(Value::List(vec![Value::Text(text.to_string())]));
    }
    let parts: Vec<String> = if qs_n == 1 {
        csv_split_any(text, &delims)
    } else {
        let mut parts: Vec<String> = Vec::new();
        let mut buf = String::new();
        let mut i = 0;
        let bytes = text.as_bytes();
        while i < bytes.len() {
            let mut matched: Option<usize> = None;
            for d in &delims {
                if !d.is_empty() && text[i..].starts_with(d.as_str()) {
                    matched = Some(d.len());
                    break;
                }
            }
            match matched {
                Some(skip) => {
                    parts.push(std::mem::take(&mut buf));
                    i += skip;
                }
                None => {
                    let c = text[i..].chars().next().unwrap();
                    buf.push(c);
                    i += c.len_utf8();
                }
            }
        }
        parts.push(buf);
        parts
    };
    Ok(Value::List(parts.into_iter().map(Value::Text).collect()))
}

/// Sweep `text` left-to-right tracking double-quote state; collect byte
/// offsets of every position that lies *outside* a quoted region.
/// (Inside `""`-escaped quote pairs we stay in-quote.)
fn out_of_quote_positions(text: &str) -> Vec<usize> {
    let mut out: Vec<usize> = Vec::with_capacity(text.len() + 1);
    let mut in_quote = false;
    let bytes = text.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        if !in_quote {
            out.push(i);
        }
        let c = text[i..].chars().next().unwrap();
        let cl = c.len_utf8();
        if c == '"' {
            if in_quote && text[i + cl..].starts_with('"') {
                // escaped quote — stay in quote, skip both
                i += cl + 1;
                continue;
            }
            in_quote = !in_quote;
        }
        i += cl;
    }
    if !in_quote {
        out.push(bytes.len());
    }
    out
}

/// Find the first byte offset of `delim` in `text`. If `csv` is true,
/// only matches starting at a position outside any double-quoted region
/// count.
fn find_delim(text: &str, delim: &str, csv: bool) -> Option<usize> {
    if delim.is_empty() {
        return None;
    }
    if !csv {
        return text.find(delim);
    }
    let valid = out_of_quote_positions(text);
    let mut start = 0usize;
    while let Some(pos) = text[start..].find(delim) {
        let abs = start + pos;
        if valid.binary_search(&abs).is_ok() {
            return Some(abs);
        }
        // Advance by one char beyond the match attempt.
        let c = text[abs..].chars().next().unwrap();
        start = abs + c.len_utf8();
    }
    None
}

/// Find the last byte offset of `delim` in `text`. If `csv`, only
/// matches outside double-quoted regions count.
fn rfind_delim(text: &str, delim: &str, csv: bool) -> Option<usize> {
    if delim.is_empty() {
        return None;
    }
    if !csv {
        return text.rfind(delim);
    }
    let valid = out_of_quote_positions(text);
    // Scan all matches and keep the last one that's out-of-quote.
    let mut last: Option<usize> = None;
    let mut start = 0usize;
    while let Some(pos) = text[start..].find(delim) {
        let abs = start + pos;
        if valid.binary_search(&abs).is_ok() {
            last = Some(abs);
        }
        let c = text[abs..].chars().next().unwrap();
        start = abs + c.len_utf8();
    }
    last
}

fn split_text_by_each_delimiter_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let delims = expect_text_list(&args[1], "Splitter.SplitTextByEachDelimiter")?;
    let csv = matches!(&args[2], Value::Number(n) if *n as i64 == 1);
    let reverse = matches!(&args[3], Value::Logical(true));

    if !reverse {
        let mut rest = text.to_string();
        let mut parts: Vec<String> = Vec::new();
        for d in &delims {
            match find_delim(&rest, d, csv) {
                Some(pos) => {
                    parts.push(rest[..pos].to_string());
                    rest = rest[pos + d.len()..].to_string();
                }
                None => {
                    return Err(MError::Other(format!(
                        "Splitter.SplitTextByEachDelimiter: delimiter not found: {d:?}"
                    )));
                }
            }
        }
        parts.push(rest);
        return Ok(Value::List(parts.into_iter().map(Value::Text).collect()));
    }

    // Reverse mode: walk delims right-to-left, cut on each delimiter's
    // last occurrence from the right; assemble parts from the right.
    let mut rest = text.to_string();
    let mut tail_parts: Vec<String> = Vec::new();
    for d in delims.iter().rev() {
        match rfind_delim(&rest, d, csv) {
            Some(pos) => {
                tail_parts.push(rest[pos + d.len()..].to_string());
                rest.truncate(pos);
            }
            None => {
                return Err(MError::Other(format!(
                    "Splitter.SplitTextByEachDelimiter: delimiter not found: {d:?}"
                )));
            }
        }
    }
    // `rest` is now the leftmost field; `tail_parts` is right-to-left order.
    let mut parts: Vec<String> = Vec::with_capacity(tail_parts.len() + 1);
    parts.push(rest);
    parts.extend(tail_parts.into_iter().rev());
    Ok(Value::List(parts.into_iter().map(Value::Text).collect()))
}

fn split_text_by_lengths_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let lengths_v = expect_list(&args[1])?;
    let reverse = matches!(&args[2], Value::Logical(true));
    let mut lengths: Vec<usize> = Vec::with_capacity(lengths_v.len());
    for v in lengths_v {
        let n = expect_int(v, "Splitter.SplitTextByLengths")?;
        if n < 0 {
            return Err(MError::Other(
                "Splitter.SplitTextByLengths: length must be non-negative".into(),
            ));
        }
        lengths.push(n as usize);
    }
    let chars: Vec<char> = text.chars().collect();
    if !reverse {
        let mut parts: Vec<Value> = Vec::with_capacity(lengths.len());
        let mut idx = 0usize;
        for n in lengths {
            if idx + n > chars.len() {
                return Err(MError::Other(format!(
                    "Splitter.SplitTextByLengths: text too short for length sequence (need {}, have {} remaining)",
                    n,
                    chars.len() - idx
                )));
            }
            let chunk: String = chars[idx..idx + n].iter().collect();
            parts.push(Value::Text(chunk));
            idx += n;
        }
        return Ok(Value::List(parts));
    }
    // Reverse: walk lengths right-to-left, taking chunks from the end of text.
    let mut tail_parts: Vec<String> = Vec::with_capacity(lengths.len());
    let mut end = chars.len();
    for n in lengths.iter().rev() {
        if *n > end {
            return Err(MError::Other(format!(
                "Splitter.SplitTextByLengths: text too short for length sequence (need {}, have {} remaining)",
                n, end
            )));
        }
        let start = end - n;
        let chunk: String = chars[start..end].iter().collect();
        tail_parts.push(chunk);
        end = start;
    }
    // tail_parts is in right-to-left order; reverse to match original
    // lengths-array order in the output list.
    let parts: Vec<Value> = tail_parts.into_iter().rev().map(Value::Text).collect();
    Ok(Value::List(parts))
}

fn split_text_by_positions_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let positions_v = expect_list(&args[1])?;
    let mut positions: Vec<usize> = Vec::with_capacity(positions_v.len());
    for v in positions_v {
        let n = expect_int(v, "Splitter.SplitTextByPositions")?;
        if n < 0 {
            return Err(MError::Other(
                "Splitter.SplitTextByPositions: position must be non-negative".into(),
            ));
        }
        positions.push(n as usize);
    }
    let chars: Vec<char> = text.chars().collect();
    let mut parts: Vec<Value> = Vec::with_capacity(positions.len());
    for i in 0..positions.len() {
        let start = positions[i];
        let end = positions.get(i + 1).copied().unwrap_or(chars.len());
        if start > chars.len() || end > chars.len() || start > end {
            return Err(MError::Other(format!(
                "Splitter.SplitTextByPositions: position {} out of range (text length {})",
                start,
                chars.len()
            )));
        }
        let chunk: String = chars[start..end].iter().collect();
        parts.push(Value::Text(chunk));
    }
    Ok(Value::List(parts))
}

fn split_text_by_ranges_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let ranges_v = expect_list(&args[1])?;
    let chars: Vec<char> = text.chars().collect();
    let mut parts: Vec<Value> = Vec::with_capacity(ranges_v.len());
    for r in ranges_v {
        let pair = match r {
            Value::List(p) => p,
            other => return Err(type_mismatch("list (range pair)", other)),
        };
        let offset = expect_int(&pair[0], "Splitter.SplitTextByRanges (offset)")?;
        let count = expect_int(&pair[1], "Splitter.SplitTextByRanges (count)")?;
        if offset < 0 || count < 0 {
            return Err(MError::Other(
                "Splitter.SplitTextByRanges: offset/count must be non-negative".into(),
            ));
        }
        let start = offset as usize;
        let end = start + count as usize;
        if end > chars.len() {
            return Err(MError::Other(format!(
                "Splitter.SplitTextByRanges: range {}..{} out of bounds (length {})",
                start,
                end,
                chars.len()
            )));
        }
        let chunk: String = chars[start..end].iter().collect();
        parts.push(Value::Text(chunk));
    }
    Ok(Value::List(parts))
}

fn split_text_by_character_transition_impl(
    args: &[Value],
    _host: &dyn IoHost,
) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let before = expect_text_list(&args[1], "Splitter.SplitTextByCharacterTransition (before)")?;
    let after = expect_text_list(&args[2], "Splitter.SplitTextByCharacterTransition (after)")?;
    // `before` and `after` are lists of single-character texts. A transition
    // is a position where the previous char is in `before` and the next is
    // in `after`. Split immediately before the `after` char.
    let chars: Vec<char> = text.chars().collect();
    let in_set = |s: &[String], ch: char| -> bool {
        s.iter().any(|t| t.starts_with(ch) && t.chars().count() == 1)
    };
    let mut parts: Vec<String> = Vec::new();
    let mut buf = String::new();
    for i in 0..chars.len() {
        if i > 0 && in_set(&before, chars[i - 1]) && in_set(&after, chars[i]) {
            parts.push(std::mem::take(&mut buf));
        }
        buf.push(chars[i]);
    }
    parts.push(buf);
    Ok(Value::List(parts.into_iter().map(Value::Text).collect()))
}

fn split_text_by_repeated_lengths_impl(
    args: &[Value],
    _host: &dyn IoHost,
) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let n = expect_int(&args[1], "Splitter.SplitTextByRepeatedLengths")?;
    if n <= 0 {
        return Err(MError::Other(
            "Splitter.SplitTextByRepeatedLengths: length must be positive".into(),
        ));
    }
    let len = n as usize;
    let chars: Vec<char> = text.chars().collect();
    let mut parts: Vec<Value> = Vec::new();
    let mut i = 0usize;
    while i < chars.len() {
        let end = std::cmp::min(i + len, chars.len());
        let chunk: String = chars[i..end].iter().collect();
        parts.push(Value::Text(chunk));
        i = end;
    }
    Ok(Value::List(parts))
}

fn split_text_by_whitespace_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let parts: Vec<Value> = text
        .split_whitespace()
        .map(|s| Value::Text(s.to_string()))
        .collect();
    Ok(Value::List(parts))
}
