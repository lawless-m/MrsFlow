//! `Combiner.*` factory stdlib bindings.
//!
//! Each factory returns a `Value::Function` closure that, when applied to a
//! list of texts, produces a combined text. Same synthetic-closure pattern
//! as `Splitter.*` — the inner builtin receives `[list, ...captured]`.

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
        (
            "Combiner.CombineTextByDelimiter",
            vec![
                Param { name: "delimiter".into(), optional: false, type_annotation: None },
                Param { name: "quoteStyle".into(), optional: true, type_annotation: None },
            ],
            combine_text_by_delimiter,
        ),
        (
            "Combiner.CombineTextByEachDelimiter",
            one("delimiters"),
            combine_text_by_each_delimiter,
        ),
        (
            "Combiner.CombineTextByLengths",
            vec![
                Param { name: "lengths".into(), optional: false, type_annotation: None },
                Param { name: "template".into(), optional: true, type_annotation: None },
            ],
            combine_text_by_lengths,
        ),
        (
            "Combiner.CombineTextByPositions",
            vec![
                Param { name: "positions".into(), optional: false, type_annotation: None },
                Param { name: "template".into(), optional: true, type_annotation: None },
            ],
            combine_text_by_positions,
        ),
        (
            "Combiner.CombineTextByRanges",
            vec![
                Param { name: "ranges".into(), optional: false, type_annotation: None },
                Param { name: "template".into(), optional: true, type_annotation: None },
            ],
            combine_text_by_ranges,
        ),
    ]
}

/// Build the synthetic M-bodied closure whose `list` argument is forwarded to
/// the inner impl along with the captured factory params.
fn make_combiner(captures: Vec<(String, Value)>, impl_fn: BuiltinFn) -> Value {
    let mut env = EnvNode::empty();
    let mut impl_params: Vec<Param> = vec![Param {
        name: "list".into(),
        optional: false,
        type_annotation: None,
    }];
    let mut call_args: Vec<Expr> = vec![Expr::Identifier("list".into())];
    for (k, v) in &captures {
        env = env.extend(k.clone(), v.clone());
        impl_params.push(Param {
            name: k.clone(),
            optional: false,
            type_annotation: None,
        });
        call_args.push(Expr::Identifier(k.clone()));
    }
    let impl_name = "__combiner_impl__".to_string();
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
            name: "list".into(),
            optional: false,
            type_annotation: None,
        }],
        body: FnBody::M(Box::new(body)),
        env,
    })
}

// --- Factories ---

fn combine_text_by_delimiter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if !matches!(&args[0], Value::Text(_)) {
        return Err(type_mismatch("text", &args[0]));
    }
    let qs_n: i64 = match args.get(1) {
        None | Some(Value::Null) => 0,
        Some(Value::Number(n)) => {
            let k = *n as i64;
            if k != 0 && k != 1 {
                return Err(MError::Other(format!(
                    "Combiner.CombineTextByDelimiter: quoteStyle must be QuoteStyle.None (0) or QuoteStyle.Csv (1), got {k}"
                )));
            }
            k
        }
        Some(other) => return Err(type_mismatch("number (QuoteStyle.*)", other)),
    };
    Ok(make_combiner(
        vec![
            ("__delim".into(), args[0].clone()),
            ("__qs".into(), Value::Number(qs_n as f64)),
        ],
        combine_text_by_delimiter_impl,
    ))
}

fn combine_text_by_each_delimiter(
    args: &[Value],
    _host: &dyn IoHost,
) -> Result<Value, MError> {
    let _ = expect_text_list(&args[0], "Combiner.CombineTextByEachDelimiter")?;
    Ok(make_combiner(
        vec![("__delims".into(), args[0].clone())],
        combine_text_by_each_delimiter_impl,
    ))
}

/// Validate the optional `template` arg used by CombineTextBy{Lengths,
/// Positions,Ranges}. Null or an empty/whitespace-only text trivially
/// degenerates to the default layout — pass through. Any text that
/// looks like it contains placeholders ($1, $2, …) requires a real
/// templating engine and is rejected as not yet supported.
fn accept_trivial_template(arg: Option<&Value>, fn_name: &str) -> Result<(), MError> {
    match arg {
        None | Some(Value::Null) => Ok(()),
        Some(Value::Text(s)) => {
            if s.trim().is_empty() {
                Ok(())
            } else {
                Err(MError::Other(format!(
                    "{fn_name}: non-trivial template ({s:?}) not yet supported"
                )))
            }
        }
        Some(other) => Err(type_mismatch("text (template) or null", other)),
    }
}

fn combine_text_by_lengths(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    for v in xs {
        let _ = expect_int(v, "Combiner.CombineTextByLengths")?;
    }
    accept_trivial_template(args.get(1), "Combiner.CombineTextByLengths")?;
    Ok(make_combiner(
        vec![("__lengths".into(), args[0].clone())],
        combine_text_by_lengths_impl,
    ))
}

fn combine_text_by_positions(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    for v in xs {
        let _ = expect_int(v, "Combiner.CombineTextByPositions")?;
    }
    accept_trivial_template(args.get(1), "Combiner.CombineTextByPositions")?;
    Ok(make_combiner(
        vec![("__positions".into(), args[0].clone())],
        combine_text_by_positions_impl,
    ))
}

fn combine_text_by_ranges(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    for r in xs {
        let pair = match r {
            Value::List(p) => p,
            other => return Err(type_mismatch("list (range pair)", other)),
        };
        if pair.len() != 2 {
            return Err(MError::Other(format!(
                "Combiner.CombineTextByRanges: range must have 2 elements, got {}",
                pair.len()
            )));
        }
        let _ = expect_int(&pair[0], "Combiner.CombineTextByRanges (offset)")?;
        let _ = expect_int(&pair[1], "Combiner.CombineTextByRanges (count)")?;
    }
    accept_trivial_template(args.get(1), "Combiner.CombineTextByRanges")?;
    Ok(make_combiner(
        vec![("__ranges".into(), args[0].clone())],
        combine_text_by_ranges_impl,
    ))
}

// --- Inner impls ---

fn combine_text_by_delimiter_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let items = expect_text_list(&args[0], "Combiner.CombineTextByDelimiter")?;
    let delim = expect_text(&args[1])?;
    let qs_n = match &args[2] {
        Value::Number(n) => *n as i64,
        _ => 0,
    };
    if qs_n != 1 {
        return Ok(Value::Text(items.join(delim)));
    }
    // Csv quoting: wrap a field in "..." if it contains the delimiter,
    // a `"`, CR, or LF; double any embedded `"`.
    let mut quoted: Vec<String> = Vec::with_capacity(items.len());
    for item in &items {
        let needs_quoting = (!delim.is_empty() && item.contains(delim))
            || item.contains('"')
            || item.contains('\n')
            || item.contains('\r');
        if needs_quoting {
            let mut s = String::with_capacity(item.len() + 2);
            s.push('"');
            for c in item.chars() {
                if c == '"' {
                    s.push('"');
                    s.push('"');
                } else {
                    s.push(c);
                }
            }
            s.push('"');
            quoted.push(s);
        } else {
            quoted.push(item.clone());
        }
    }
    Ok(Value::Text(quoted.join(delim)))
}

fn combine_text_by_each_delimiter_impl(
    args: &[Value],
    _host: &dyn IoHost,
) -> Result<Value, MError> {
    let items = expect_text_list(&args[0], "Combiner.CombineTextByEachDelimiter")?;
    let delims = expect_text_list(&args[1], "Combiner.CombineTextByEachDelimiter")?;
    if items.is_empty() {
        return Ok(Value::Text(String::new()));
    }
    if delims.is_empty() && items.len() > 1 {
        return Err(MError::Other(
            "Combiner.CombineTextByEachDelimiter: empty delimiters list with multi-item input".into(),
        ));
    }
    let mut out = String::new();
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            // Cycle delimiters when shorter than gaps.
            let d = &delims[(i - 1) % delims.len()];
            out.push_str(d);
        }
        out.push_str(item);
    }
    Ok(Value::Text(out))
}

fn combine_text_by_lengths_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let items = expect_text_list(&args[0], "Combiner.CombineTextByLengths")?;
    let lengths_v = expect_list(&args[1])?;
    let mut lengths: Vec<usize> = Vec::with_capacity(lengths_v.len());
    for v in lengths_v {
        let n = expect_int(v, "Combiner.CombineTextByLengths")?;
        if n < 0 {
            return Err(MError::Other(
                "Combiner.CombineTextByLengths: length must be non-negative".into(),
            ));
        }
        lengths.push(n as usize);
    }
    if items.len() != lengths.len() {
        return Err(MError::Other(format!(
            "Combiner.CombineTextByLengths: items ({}) and lengths ({}) must have same count",
            items.len(),
            lengths.len()
        )));
    }
    let mut out = String::new();
    for (item, &n) in items.iter().zip(lengths.iter()) {
        let chars: Vec<char> = item.chars().collect();
        if chars.len() >= n {
            out.extend(chars.iter().take(n));
        } else {
            out.extend(chars.iter());
            for _ in chars.len()..n {
                out.push(' ');
            }
        }
    }
    Ok(Value::Text(out))
}

fn combine_text_by_positions_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let items = expect_text_list(&args[0], "Combiner.CombineTextByPositions")?;
    let positions_v = expect_list(&args[1])?;
    let mut positions: Vec<usize> = Vec::with_capacity(positions_v.len());
    for v in positions_v {
        let n = expect_int(v, "Combiner.CombineTextByPositions")?;
        if n < 0 {
            return Err(MError::Other(
                "Combiner.CombineTextByPositions: position must be non-negative".into(),
            ));
        }
        positions.push(n as usize);
    }
    if items.len() != positions.len() {
        return Err(MError::Other(format!(
            "Combiner.CombineTextByPositions: items ({}) and positions ({}) must have same count",
            items.len(),
            positions.len()
        )));
    }
    // Build a char buffer big enough to hold the last item, pad with spaces.
    let total: usize = positions
        .iter()
        .zip(items.iter())
        .map(|(p, s)| p + s.chars().count())
        .max()
        .unwrap_or(0);
    let mut buf: Vec<char> = vec![' '; total];
    for (item, &pos) in items.iter().zip(positions.iter()) {
        for (i, c) in item.chars().enumerate() {
            if pos + i < buf.len() {
                buf[pos + i] = c;
            }
        }
    }
    Ok(Value::Text(buf.into_iter().collect()))
}

fn combine_text_by_ranges_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let items = expect_text_list(&args[0], "Combiner.CombineTextByRanges")?;
    let ranges_v = expect_list(&args[1])?;
    let mut ranges: Vec<(usize, usize)> = Vec::with_capacity(ranges_v.len());
    for r in ranges_v {
        let pair = match r {
            Value::List(p) => p,
            other => return Err(type_mismatch("list (range pair)", other)),
        };
        let offset = expect_int(&pair[0], "Combiner.CombineTextByRanges (offset)")?;
        let count = expect_int(&pair[1], "Combiner.CombineTextByRanges (count)")?;
        if offset < 0 || count < 0 {
            return Err(MError::Other(
                "Combiner.CombineTextByRanges: offset/count must be non-negative".into(),
            ));
        }
        ranges.push((offset as usize, count as usize));
    }
    if items.len() != ranges.len() {
        return Err(MError::Other(format!(
            "Combiner.CombineTextByRanges: items ({}) and ranges ({}) must have same count",
            items.len(),
            ranges.len()
        )));
    }
    // Each item is written into its {offset, count} slot, padded/truncated.
    let total: usize = ranges.iter().map(|(o, c)| o + c).max().unwrap_or(0);
    let mut buf: Vec<char> = vec![' '; total];
    for (item, &(offset, count)) in items.iter().zip(ranges.iter()) {
        let chars: Vec<char> = item.chars().collect();
        for i in 0..count {
            if offset + i < buf.len() {
                buf[offset + i] = if i < chars.len() { chars[i] } else { ' ' };
            }
        }
    }
    Ok(Value::Text(buf.into_iter().collect()))
}
