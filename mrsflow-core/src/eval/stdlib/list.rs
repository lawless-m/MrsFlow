//! `List.*` stdlib bindings.

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
        ("List.Transform", two("list", "transform"), list_transform),
        ("List.Select", two("list", "selection"), list_select),
        ("List.Sum", one("list"), list_sum),
        (
            "List.Average",
            vec![
                Param { name: "list".into(),      optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            list_average,
        ),
        ("List.Count", one("list"), list_count),
        ("List.Min", one("list"), list_min),
        ("List.Max", one("list"), list_max),
        ("List.Combine", one("lists"), list_combine),
        ("List.IsEmpty", one("list"), list_is_empty),
        (
            "List.First",
            vec![
                Param { name: "list".into(),    optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            list_first,
        ),
        (
            "List.Last",
            vec![
                Param { name: "list".into(),    optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            list_last,
        ),
        ("List.Reverse", one("list"), list_reverse),
        (
            "List.Numbers",
            vec![
                Param { name: "start".into(),     optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "increment".into(), optional: true,  type_annotation: None },
            ],
            list_numbers,
        ),
        (
            "List.PositionOf",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "value".into(),            optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_position_of,
        ),
        (
            "List.RemoveFirstN",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            list_remove_first_n,
        ),
        ("List.RemoveItems", two("list", "list2"), list_remove_items),
        ("List.Zip", one("lists"), list_zip),
        ("List.FirstN", two("list", "countOrCondition"), list_first_n),
        (
            "List.LastN",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            list_last_n,
        ),
        ("List.Skip", two("list", "countOrCondition"), list_skip),
        ("List.Distinct", one("list"), list_distinct),
        (
            "List.Sort",
            vec![
                Param { name: "list".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_sort,
        ),
        (
            "List.RemoveMatchingItems",
            two("list", "items"),
            list_remove_matching_items,
        ),
        ("List.AnyTrue", one("list"), list_any_true),
        ("List.AllTrue", one("list"), list_all_true),
        (
            "List.Accumulate",
            three("list", "seed", "accumulator"),
            list_accumulate,
        ),
        (
            "List.Contains",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "value".into(),            optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_contains,
        ),
        (
            "List.ContainsAll",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "values".into(),           optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_contains_all,
        ),
        (
            "List.ContainsAny",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "values".into(),           optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_contains_any,
        ),
        (
            "List.IsDistinct",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_is_distinct,
        ),
        ("List.MatchesAll", two("list", "condition"), list_matches_all),
        ("List.MatchesAny", two("list", "condition"), list_matches_any),
        ("List.FindText", two("list", "text"), list_find_text),
        (
            "List.PositionOfAny",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "values".into(),           optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_position_of_any,
        ),
        ("List.Positions", one("list"), list_positions),
        (
            "List.Range",
            vec![
                Param { name: "list".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            list_range,
        ),
        (
            "List.RemoveLastN",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            list_remove_last_n,
        ),
        ("List.RemoveNulls", one("list"), list_remove_nulls),
        (
            "List.RemoveRange",
            vec![
                Param { name: "list".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            list_remove_range,
        ),
        ("List.InsertRange", three("list", "offset", "values"), list_insert_range),
        ("List.ReplaceMatchingItems", two("list", "replacements"), list_replace_matching_items),
        (
            "List.ReplaceRange",
            vec![
                Param { name: "list".into(),      optional: false, type_annotation: None },
                Param { name: "offset".into(),    optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "newValues".into(), optional: false, type_annotation: None },
            ],
            list_replace_range,
        ),
        (
            "List.ReplaceValue",
            vec![
                Param { name: "list".into(),     optional: false, type_annotation: None },
                Param { name: "oldValue".into(), optional: false, type_annotation: None },
                Param { name: "newValue".into(), optional: false, type_annotation: None },
                Param { name: "replacer".into(), optional: false, type_annotation: None },
            ],
            list_replace_value,
        ),
        ("List.Repeat", two("list", "count"), list_repeat),
        ("List.Alternate", four_with_opts("list", "count", "repeatInterval", "offset"), list_alternate),
        ("List.Split", two("list", "pageSize"), list_split),
        ("List.Buffer", one("list"), list_buffer),
    ]
}

fn four_with_opts(a: &str, b: &str, c: &str, d: &str) -> Vec<Param> {
    vec![
        Param { name: a.into(), optional: false, type_annotation: None },
        Param { name: b.into(), optional: false, type_annotation: None },
        Param { name: c.into(), optional: true,  type_annotation: None },
        Param { name: d.into(), optional: true,  type_annotation: None },
    ]
}

fn list_transform(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let f = expect_function(&args[1])?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let v = invoke_builtin_callback(f, vec![item.clone()])?;
        out.push(v);
    }
    Ok(Value::List(out))
}


fn list_select(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let pred = expect_function(&args[1])?;
    let mut out = Vec::new();
    for item in list {
        let v = invoke_builtin_callback(pred, vec![item.clone()])?;
        match v {
            Value::Logical(true) => out.push(item.clone()),
            Value::Logical(false) => {}
            other => return Err(type_mismatch("logical (from predicate)", &other)),
        }
    }
    Ok(Value::List(out))
}


fn list_sum(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut total = 0.0;
    for v in list {
        total += match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
    }
    Ok(Value::Number(total))
}


fn list_average(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut total = 0.0f64;
    let mut n = 0usize;
    for v in list {
        match v {
            Value::Null => continue,
            Value::Number(x) => {
                total += x;
                n += 1;
            }
            other => return Err(type_mismatch("number (in list)", other)),
        }
    }
    if n == 0 {
        Ok(Value::Null)
    } else {
        Ok(Value::Number(total / n as f64))
    }
}


fn list_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Number(list.len() as f64))
}


fn list_zip(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lists = expect_list(&args[0])?;
    let inner: Vec<&Vec<Value>> = lists
        .iter()
        .map(|v| expect_list(v))
        .collect::<Result<_, _>>()?;
    let max_len = inner.iter().map(|l| l.len()).max().unwrap_or(0);
    let mut out: Vec<Value> = Vec::with_capacity(max_len);
    for i in 0..max_len {
        let row: Vec<Value> = inner
            .iter()
            .map(|l| l.get(i).cloned().unwrap_or(Value::Null))
            .collect();
        out.push(Value::List(row));
    }
    Ok(Value::List(out))
}


fn list_remove_first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let n = match args.get(1) {
        Some(Value::Number(n)) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(MError::Other(
                    "List.RemoveFirstN: count must be a non-negative integer".into(),
                ));
            }
            *n as usize
        }
        Some(Value::Function(_)) => {
            return Err(MError::NotImplemented(
                "List.RemoveFirstN: predicate (skip-while) form not yet supported",
            ));
        }
        Some(Value::Null) | None => 1,
        Some(other) => return Err(type_mismatch("number or function", other)),
    };
    Ok(Value::List(list.iter().skip(n).cloned().collect()))
}


fn list_remove_items(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let drop = expect_list(&args[1])?;
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for v in list {
        let mut keep = true;
        for d in drop {
            if values_equal_primitive(v, d)? {
                keep = false;
                break;
            }
        }
        if keep {
            out.push(v.clone());
        }
    }
    Ok(Value::List(out))
}


fn list_position_of(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let target = &args[1];
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "List.PositionOf: occurrence arg not yet supported",
        ));
    }
    if !matches!(args.get(3), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "List.PositionOf: equationCriteria not yet supported",
        ));
    }
    for (i, v) in list.iter().enumerate() {
        if values_equal_primitive(v, target)? {
            return Ok(Value::Number(i as f64));
        }
    }
    Ok(Value::Number(-1.0))
}


fn list_numbers(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let start = match &args[0] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let count = match &args[1] {
        Value::Number(n) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(MError::Other("List.Numbers: count must be a non-negative integer".into()));
            }
            *n as usize
        }
        other => return Err(type_mismatch("number", other)),
    };
    let increment = match args.get(2) {
        Some(Value::Number(n)) => *n,
        Some(Value::Null) | None => 1.0,
        Some(other) => return Err(type_mismatch("number", other)),
    };
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        out.push(Value::Number(start + (i as f64) * increment));
    }
    Ok(Value::List(out))
}


fn list_min(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut best: Option<f64> = None;
    for v in list {
        let n = match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
        best = Some(match best {
            None => n,
            Some(curr) => if n < curr { n } else { curr },
        });
    }
    Ok(Value::Number(best.unwrap()))
}


fn list_max(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut best: Option<f64> = None;
    for v in list {
        let n = match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
        best = Some(match best {
            None => n,
            Some(curr) => if n > curr { n } else { curr },
        });
    }
    Ok(Value::Number(best.unwrap()))
}


fn list_is_empty(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Logical(list.is_empty()))
}


fn list_first(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if let Some(first) = list.first() {
        Ok(first.clone())
    } else {
        Ok(args.get(1).cloned().unwrap_or(Value::Null))
    }
}


fn list_last(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if let Some(last) = list.last() {
        Ok(last.clone())
    } else {
        Ok(args.get(1).cloned().unwrap_or(Value::Null))
    }
}


fn list_sort(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "List.Sort: comparisonCriteria not yet supported",
        ));
    }
    enum Kind { Empty, Num, Text }
    let mut kind = Kind::Empty;
    for v in list {
        let k = match v {
            Value::Number(_) => Kind::Num,
            Value::Text(_) => Kind::Text,
            other => return Err(type_mismatch("number or text (in list)", other)),
        };
        match (&kind, &k) {
            (Kind::Empty, _) => kind = k,
            (Kind::Num, Kind::Num) | (Kind::Text, Kind::Text) => {}
            _ => return Err(MError::Other(
                "List.Sort: mixed-type lists not supported (numbers and text together)".into(),
            )),
        }
    }
    let mut out: Vec<Value> = list.clone();
    match kind {
        Kind::Empty => {}
        Kind::Num => out.sort_by(|a, b| {
            let (Value::Number(x), Value::Number(y)) = (a, b) else { unreachable!() };
            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }),
        Kind::Text => out.sort_by(|a, b| {
            let (Value::Text(x), Value::Text(y)) = (a, b) else { unreachable!() };
            x.cmp(y)
        }),
    }
    Ok(Value::List(out))
}


fn list_reverse(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut out = list.clone();
    out.reverse();
    Ok(Value::List(out))
}


fn list_first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    // Power Query also accepts a predicate (take-while) form; not yet supported.
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Value::Function(_) => {
            return Err(MError::NotImplemented(
                "List.FirstN: predicate (take-while) form not yet supported",
            ));
        }
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    Ok(Value::List(list.iter().take(count).cloned().collect()))
}


fn list_last_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let count = match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Function(_)) => {
            return Err(MError::NotImplemented(
                "List.LastN: predicate form not yet supported",
            ));
        }
        Some(Value::Null) | None => 1,
        Some(other) => return Err(type_mismatch("non-negative integer", other)),
    };
    let n = list.len();
    let start = n.saturating_sub(count);
    Ok(Value::List(list[start..].to_vec()))
}

/// Structural equality for primitive cell types only — number, text, logical,
/// null, date, datetime, duration. Compound values (list/record/table/function/
/// type/thunk/binary) error out; the caller wraps the error.

fn list_any_true(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    for v in list {
        match v {
            Value::Logical(b) => {
                if *b {
                    return Ok(Value::Logical(true));
                }
            }
            other => return Err(type_mismatch("logical (in list)", other)),
        }
    }
    Ok(Value::Logical(false))
}


fn list_all_true(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    for v in list {
        match v {
            Value::Logical(b) => {
                if !*b {
                    return Ok(Value::Logical(false));
                }
            }
            other => return Err(type_mismatch("logical (in list)", other)),
        }
    }
    Ok(Value::Logical(true))
}


fn list_remove_matching_items(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let items = expect_list(&args[1])?;
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for v in list {
        let mut drop = false;
        for x in items {
            if values_equal_primitive(v, x)? {
                drop = true;
                break;
            }
        }
        if !drop {
            out.push(v.clone());
        }
    }
    Ok(Value::List(out))
}


fn list_distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for v in list {
        let mut seen = false;
        for kept in &out {
            if values_equal_primitive(kept, v)? {
                seen = true;
                break;
            }
        }
        if !seen {
            out.push(v.clone());
        }
    }
    Ok(Value::List(out))
}


fn list_skip(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Value::Function(_) => {
            return Err(MError::NotImplemented(
                "List.Skip: predicate (skip-while) form not yet supported",
            ));
        }
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    Ok(Value::List(list.iter().skip(count).cloned().collect()))
}


fn list_combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lists = expect_list(&args[0])?;
    let mut out: Vec<Value> = Vec::new();
    for v in lists {
        match v {
            Value::List(xs) => out.extend(xs.iter().cloned()),
            other => return Err(type_mismatch("list (in list)", other)),
        }
    }
    Ok(Value::List(out))
}


fn list_accumulate(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut acc = args[1].clone();
    let f = expect_function(&args[2])?;
    for item in list {
        acc = invoke_callback_with_host(f, vec![acc, item.clone()], host)?;
    }
    Ok(acc)
}

fn list_contains(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let target = &args[1];
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("List.Contains: equationCriteria not yet supported"));
    }
    for v in list {
        if values_equal_primitive(v, target)? {
            return Ok(Value::Logical(true));
        }
    }
    Ok(Value::Logical(false))
}

fn list_contains_all(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let values = expect_list(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("List.ContainsAll: equationCriteria not yet supported"));
    }
    for v in values {
        let mut found = false;
        for x in list {
            if values_equal_primitive(x, v)? { found = true; break; }
        }
        if !found { return Ok(Value::Logical(false)); }
    }
    Ok(Value::Logical(true))
}

fn list_contains_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let values = expect_list(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("List.ContainsAny: equationCriteria not yet supported"));
    }
    for v in values {
        for x in list {
            if values_equal_primitive(x, v)? { return Ok(Value::Logical(true)); }
        }
    }
    Ok(Value::Logical(false))
}

fn list_is_distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("List.IsDistinct: equationCriteria not yet supported"));
    }
    for (i, a) in list.iter().enumerate() {
        for b in &list[i + 1..] {
            if values_equal_primitive(a, b)? {
                return Ok(Value::Logical(false));
            }
        }
    }
    Ok(Value::Logical(true))
}

fn list_matches_all(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let predicate = expect_function(&args[1])?;
    for v in list {
        let r = invoke_callback_with_host(predicate, vec![v.clone()], host)?;
        match r {
            Value::Logical(true) => continue,
            Value::Logical(false) => return Ok(Value::Logical(false)),
            other => return Err(type_mismatch("logical", &other)),
        }
    }
    Ok(Value::Logical(true))
}

fn list_matches_any(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let predicate = expect_function(&args[1])?;
    for v in list {
        let r = invoke_callback_with_host(predicate, vec![v.clone()], host)?;
        match r {
            Value::Logical(true) => return Ok(Value::Logical(true)),
            Value::Logical(false) => continue,
            other => return Err(type_mismatch("logical", &other)),
        }
    }
    Ok(Value::Logical(false))
}

fn list_find_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let needle = expect_text(&args[1])?;
    let mut out: Vec<Value> = Vec::new();
    for v in list {
        if let Value::Text(s) = v {
            if s.contains(needle) {
                out.push(v.clone());
            }
        }
    }
    Ok(Value::List(out))
}

fn list_position_of_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let values = expect_list(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("List.PositionOfAny: occurrence not yet supported"));
    }
    if !matches!(args.get(3), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("List.PositionOfAny: equationCriteria not yet supported"));
    }
    for (i, v) in list.iter().enumerate() {
        for t in values {
            if values_equal_primitive(v, t)? {
                return Ok(Value::Number(i as f64));
            }
        }
    }
    Ok(Value::Number(-1.0))
}

fn list_positions(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::List((0..list.len()).map(|i| Value::Number(i as f64)).collect()))
}

fn list_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match args.get(2) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => list.len().saturating_sub(offset),
        Some(other) => return Err(type_mismatch("non-negative integer or null", other)),
    };
    let end = (offset + count).min(list.len());
    let start = offset.min(list.len());
    Ok(Value::List(list[start..end].to_vec()))
}

fn list_remove_last_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let n = match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Function(_)) => {
            return Err(MError::NotImplemented("List.RemoveLastN: predicate form not yet supported"));
        }
        Some(Value::Null) | None => 1,
        Some(other) => return Err(type_mismatch("non-negative integer", other)),
    };
    let keep = list.len().saturating_sub(n);
    Ok(Value::List(list[..keep].to_vec()))
}

fn list_remove_nulls(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::List(list.iter().filter(|v| !matches!(v, Value::Null)).cloned().collect()))
}

fn list_remove_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match args.get(2) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => 1,
        Some(other) => return Err(type_mismatch("non-negative integer", other)),
    };
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    out.extend_from_slice(&list[..offset.min(list.len())]);
    let end = (offset + count).min(list.len());
    if end < list.len() {
        out.extend_from_slice(&list[end..]);
    }
    Ok(Value::List(out))
}

fn list_insert_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let values = expect_list(&args[2])?;
    if offset > list.len() {
        return Err(MError::Other("List.InsertRange: offset out of range".into()));
    }
    let mut out: Vec<Value> = Vec::with_capacity(list.len() + values.len());
    out.extend_from_slice(&list[..offset]);
    out.extend_from_slice(values);
    out.extend_from_slice(&list[offset..]);
    Ok(Value::List(out))
}

fn list_replace_matching_items(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let replacements = expect_list(&args[1])?;
    // Build a (old, new) lookup. Each replacement is a 2-elem list.
    let pairs: Vec<(Value, Value)> = replacements.iter().map(|r| match r {
        Value::List(xs) if xs.len() == 2 => Ok((xs[0].clone(), xs[1].clone())),
        other => Err(MError::Other(format!(
            "List.ReplaceMatchingItems: each replacement must be a 2-elem list (got {})",
            super::super::type_name(other)
        ))),
    }).collect::<Result<_, _>>()?;
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for v in list {
        let mut replaced = false;
        for (old, new) in &pairs {
            if values_equal_primitive(v, old)? {
                out.push(new.clone());
                replaced = true;
                break;
            }
        }
        if !replaced {
            out.push(v.clone());
        }
    }
    Ok(Value::List(out))
}

fn list_replace_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match &args[2] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let new_values = expect_list(&args[3])?;
    if offset > list.len() {
        return Err(MError::Other("List.ReplaceRange: offset out of range".into()));
    }
    let mut out: Vec<Value> = Vec::with_capacity(list.len() - count + new_values.len());
    out.extend_from_slice(&list[..offset]);
    out.extend_from_slice(new_values);
    let end = (offset + count).min(list.len());
    if end < list.len() {
        out.extend_from_slice(&list[end..]);
    }
    Ok(Value::List(out))
}

fn list_replace_value(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let old_value = &args[1];
    let new_value = &args[2];
    let replacer = expect_function(&args[3])?;
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for v in list {
        let r = invoke_callback_with_host(
            replacer,
            vec![v.clone(), old_value.clone(), new_value.clone()],
            host,
        )?;
        out.push(r);
    }
    Ok(Value::List(out))
}

fn list_repeat(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let mut out: Vec<Value> = Vec::with_capacity(list.len() * count);
    for _ in 0..count {
        out.extend_from_slice(list);
    }
    Ok(Value::List(out))
}

fn list_alternate(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer (count)", other)),
    };
    let repeat_interval = match args.get(2) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => count + 1,
        Some(other) => return Err(type_mismatch("non-negative integer (repeatInterval)", other)),
    };
    let offset = match args.get(3) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => 0,
        Some(other) => return Err(type_mismatch("non-negative integer (offset)", other)),
    };
    if repeat_interval == 0 {
        return Err(MError::Other("List.Alternate: repeatInterval must be > 0".into()));
    }
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for (i, v) in list.iter().enumerate() {
        // Position within the cycle starting at offset; values within `count`
        // window are dropped, rest kept.
        if i < offset {
            out.push(v.clone());
            continue;
        }
        let cycle_pos = (i - offset) % repeat_interval;
        if cycle_pos >= count {
            out.push(v.clone());
        }
    }
    Ok(Value::List(out))
}

fn list_split(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let page = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n > 0.0 => *n as usize,
        other => return Err(type_mismatch("positive integer (pageSize)", other)),
    };
    let chunks: Vec<Value> = list
        .chunks(page)
        .map(|c| Value::List(c.to_vec()))
        .collect();
    Ok(Value::List(chunks))
}

fn list_buffer(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::List(list.clone()))
}

