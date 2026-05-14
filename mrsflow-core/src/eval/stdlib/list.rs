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
        ("List.Transform", two("list", "transform"), transform),
        ("List.Select", two("list", "selection"), select),
        ("List.Sum", one("list"), sum),
        (
            "List.Average",
            vec![
                Param { name: "list".into(),      optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            average,
        ),
        ("List.Count", one("list"), count),
        ("List.Min", one("list"), min),
        ("List.Max", one("list"), max),
        ("List.Combine", one("lists"), combine),
        ("List.IsEmpty", one("list"), is_empty),
        (
            "List.First",
            vec![
                Param { name: "list".into(),    optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            first,
        ),
        (
            "List.Last",
            vec![
                Param { name: "list".into(),    optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            last,
        ),
        ("List.Reverse", one("list"), reverse),
        (
            "List.Numbers",
            vec![
                Param { name: "start".into(),     optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "increment".into(), optional: true,  type_annotation: None },
            ],
            numbers,
        ),
        (
            "List.PositionOf",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "value".into(),            optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            position_of,
        ),
        (
            "List.RemoveFirstN",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            remove_first_n,
        ),
        ("List.RemoveItems", two("list", "list2"), remove_items),
        ("List.Zip", one("lists"), zip),
        ("List.FirstN", two("list", "countOrCondition"), first_n),
        (
            "List.LastN",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            last_n,
        ),
        ("List.Skip", two("list", "countOrCondition"), skip),
        ("List.Distinct", one("list"), distinct),
        (
            "List.Sort",
            vec![
                Param { name: "list".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: true,  type_annotation: None },
            ],
            sort,
        ),
        (
            "List.RemoveMatchingItems",
            two("list", "items"),
            remove_matching_items,
        ),
        ("List.AnyTrue", one("list"), any_true),
        ("List.AllTrue", one("list"), all_true),
        (
            "List.Accumulate",
            three("list", "seed", "accumulator"),
            accumulate,
        ),
        (
            "List.Contains",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "value".into(),            optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            contains,
        ),
        (
            "List.ContainsAll",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "values".into(),           optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            contains_all,
        ),
        (
            "List.ContainsAny",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "values".into(),           optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            contains_any,
        ),
        (
            "List.IsDistinct",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            is_distinct,
        ),
        ("List.MatchesAll", two("list", "condition"), matches_all),
        ("List.MatchesAny", two("list", "condition"), matches_any),
        ("List.FindText", two("list", "text"), find_text),
        (
            "List.PositionOfAny",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "values".into(),           optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            position_of_any,
        ),
        ("List.Positions", one("list"), positions),
        (
            "List.Range",
            vec![
                Param { name: "list".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            range,
        ),
        (
            "List.RemoveLastN",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            remove_last_n,
        ),
        ("List.RemoveNulls", one("list"), remove_nulls),
        (
            "List.RemoveRange",
            vec![
                Param { name: "list".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            remove_range,
        ),
        ("List.InsertRange", three("list", "offset", "values"), insert_range),
        ("List.ReplaceMatchingItems", two("list", "replacements"), replace_matching_items),
        (
            "List.ReplaceRange",
            vec![
                Param { name: "list".into(),      optional: false, type_annotation: None },
                Param { name: "offset".into(),    optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "newValues".into(), optional: false, type_annotation: None },
            ],
            replace_range,
        ),
        (
            "List.ReplaceValue",
            vec![
                Param { name: "list".into(),     optional: false, type_annotation: None },
                Param { name: "oldValue".into(), optional: false, type_annotation: None },
                Param { name: "newValue".into(), optional: false, type_annotation: None },
                Param { name: "replacer".into(), optional: false, type_annotation: None },
            ],
            replace_value,
        ),
        ("List.Repeat", two("list", "count"), repeat),
        ("List.Times", two("value", "count"), times),
        (
            "List.ConformToPageReader",
            vec![
                Param { name: "list".into(),    optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            conform_to_page_reader,
        ),
        ("List.Alternate", four_with_opts("list", "count", "repeatInterval", "offset"), alternate),
        ("List.Split", two("list", "pageSize"), split),
        ("List.Buffer", one("list"), buffer),
        (
            "List.Difference",
            vec![
                Param { name: "list1".into(),            optional: false, type_annotation: None },
                Param { name: "list2".into(),            optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            difference,
        ),
        (
            "List.Intersect",
            vec![
                Param { name: "lists".into(),            optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            intersect,
        ),
        (
            "List.Union",
            vec![
                Param { name: "lists".into(),            optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            union_,
        ),
        ("List.Single", one("list"), single),
        (
            "List.SingleOrDefault",
            vec![
                Param { name: "list".into(),    optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            single_or_default,
        ),
        ("List.Median", one("list"), median),
        (
            "List.Mode",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            mode,
        ),
        (
            "List.Modes",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            modes,
        ),
        (
            "List.Product",
            vec![
                Param { name: "list".into(),      optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            product,
        ),
        (
            "List.MaxN",
            vec![
                Param { name: "list".into(),                optional: false, type_annotation: None },
                Param { name: "count".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(),  optional: true,  type_annotation: None },
                Param { name: "includeNulls".into(),        optional: true,  type_annotation: None },
            ],
            max_n,
        ),
        (
            "List.MinN",
            vec![
                Param { name: "list".into(),                optional: false, type_annotation: None },
                Param { name: "count".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(),  optional: true,  type_annotation: None },
                Param { name: "includeNulls".into(),        optional: true,  type_annotation: None },
            ],
            min_n,
        ),
        ("List.NonNullCount", one("list"), non_null_count),
        ("List.StandardDeviation", one("list"), standard_deviation),
        ("List.Covariance", two("list1", "list2"), covariance),
        (
            "List.Percentile",
            vec![
                Param { name: "list".into(),       optional: false, type_annotation: None },
                Param { name: "percentile".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),    optional: true,  type_annotation: None },
            ],
            percentile,
        ),
        (
            "List.Random",
            vec![
                Param { name: "count".into(), optional: false, type_annotation: None },
                Param { name: "seed".into(),  optional: true,  type_annotation: None },
            ],
            random,
        ),
        (
            "List.TransformMany",
            three("list", "collectionTransform", "resultTransform"),
            transform_many,
        ),
        (
            "List.Generate",
            vec![
                Param { name: "initial".into(),   optional: false, type_annotation: None },
                Param { name: "condition".into(), optional: false, type_annotation: None },
                Param { name: "next".into(),      optional: false, type_annotation: None },
                Param { name: "selector".into(),  optional: true,  type_annotation: None },
            ],
            generate,
        ),
        ("List.Dates", three("start", "count", "step"), dates),
        ("List.DateTimes", three("start", "count", "step"), datetimes),
        ("List.DateTimeZones", three("start", "count", "step"), datetimezones),
        ("List.Durations", three("start", "count", "step"), durations),
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

fn transform(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let f = expect_function(&args[1])?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let v = invoke_builtin_callback(f, vec![item.clone()])?;
        out.push(v);
    }
    Ok(Value::List(out))
}


fn select(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn sum(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn average(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Number(list.len() as f64))
}


fn zip(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn remove_first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    match args.get(1) {
        Some(Value::Number(n)) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(MError::Other(
                    "List.RemoveFirstN: count must be a non-negative integer".into(),
                ));
            }
            Ok(Value::List(list.iter().skip(*n as usize).cloned().collect()))
        }
        Some(Value::Function(f)) => {
            // skip-while: drop while predicate true, then keep rest
            let mut start = 0usize;
            for v in list {
                if predicate_holds(f, v, "List.RemoveFirstN")? {
                    start += 1;
                } else {
                    break;
                }
            }
            Ok(Value::List(list[start..].to_vec()))
        }
        Some(Value::Null) | None => Ok(Value::List(list.iter().skip(1).cloned().collect())),
        Some(other) => Err(type_mismatch("number or function", other)),
    }
}


fn remove_items(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn position_of(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let target = &args[1];
    let mode = parse_occurrence(args.get(2), "List.PositionOf")?;
    let criteria = equation_criteria_fn(args, 3, "List.PositionOf")?;
    let mut matches: Vec<usize> = Vec::new();
    for (i, v) in list.iter().enumerate() {
        if eq_via_criteria(v, target, criteria)? {
            matches.push(i);
            if mode == Occurrence::First {
                break;
            }
        }
    }
    Ok(occurrence_result(mode, &matches))
}


fn numbers(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn min(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn max(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn is_empty(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Logical(list.is_empty()))
}


fn first(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if let Some(first) = list.first() {
        Ok(first.clone())
    } else {
        Ok(args.get(1).cloned().unwrap_or(Value::Null))
    }
}


fn last(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if let Some(last) = list.last() {
        Ok(last.clone())
    } else {
        Ok(args.get(1).cloned().unwrap_or(Value::Null))
    }
}


fn sort(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    // comparisonCriteria (arg 1): function `(x, y) => number` where
    // negative=less, 0=equal, positive=greater. PQ also supports a
    // field-selector form for records and a numeric direction shorthand;
    // those land separately when the corpus needs them.
    if let Some(v) = args.get(1) {
        match v {
            Value::Null => {}
            Value::Function(f) => {
                // Use an error slot so the fallible callback can propagate
                // out of the infallible sort_by closure.
                let mut out: Vec<Value> = list.clone();
                let mut sort_err: Option<MError> = None;
                out.sort_by(|a, b| {
                    if sort_err.is_some() {
                        return std::cmp::Ordering::Equal;
                    }
                    match invoke_builtin_callback(f, vec![a.clone(), b.clone()]) {
                        Ok(Value::Number(n)) => {
                            if n < 0.0 {
                                std::cmp::Ordering::Less
                            } else if n > 0.0 {
                                std::cmp::Ordering::Greater
                            } else {
                                std::cmp::Ordering::Equal
                            }
                        }
                        Ok(other) => {
                            sort_err = Some(MError::Other(format!(
                                "List.Sort: comparisonCriteria must return a number, got {}",
                                super::super::type_name(&other),
                            )));
                            std::cmp::Ordering::Equal
                        }
                        Err(e) => {
                            sort_err = Some(e);
                            std::cmp::Ordering::Equal
                        }
                    }
                });
                if let Some(e) = sort_err {
                    return Err(e);
                }
                return Ok(Value::List(out));
            }
            other => {
                return Err(MError::Other(format!(
                    "List.Sort: comparisonCriteria as {} not yet supported (function only)",
                    super::super::type_name(other),
                )));
            }
        }
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


fn reverse(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut out = list.clone();
    out.reverse();
    Ok(Value::List(out))
}


fn first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => {
            let count = *n as usize;
            Ok(Value::List(list.iter().take(count).cloned().collect()))
        }
        Value::Function(f) => {
            // take-while: stop on first false
            let mut out: Vec<Value> = Vec::new();
            for v in list {
                if predicate_holds(f, v, "List.FirstN")? {
                    out.push(v.clone());
                } else {
                    break;
                }
            }
            Ok(Value::List(out))
        }
        other => Err(type_mismatch("non-negative integer or function", other)),
    }
}

fn predicate_holds(f: &Closure, v: &Value, fn_name: &str) -> Result<bool, MError> {
    let r = invoke_builtin_callback(f, vec![v.clone()])?;
    match r {
        Value::Logical(b) => Ok(b),
        other => Err(MError::Other(format!(
            "{fn_name}: predicate must return logical, got {}",
            super::super::type_name(&other),
        ))),
    }
}


fn last_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => {
            let count = *n as usize;
            let len = list.len();
            let start = len.saturating_sub(count);
            Ok(Value::List(list[start..].to_vec()))
        }
        Some(Value::Function(f)) => {
            // take-from-end-while: scan from end, keep while predicate true
            let mut start = list.len();
            for v in list.iter().rev() {
                if predicate_holds(f, v, "List.LastN")? {
                    start -= 1;
                } else {
                    break;
                }
            }
            Ok(Value::List(list[start..].to_vec()))
        }
        Some(Value::Null) | None => {
            let len = list.len();
            let start = len.saturating_sub(1);
            Ok(Value::List(list[start..].to_vec()))
        }
        Some(other) => Err(type_mismatch("non-negative integer or function", other)),
    }
}

/// Structural equality for primitive cell types only — number, text, logical,
/// null, date, datetime, duration. Compound values (list/record/table/function/
/// type/thunk/binary) error out; the caller wraps the error.
fn any_true(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn all_true(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn remove_matching_items(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn skip(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => {
            let count = *n as usize;
            Ok(Value::List(list.iter().skip(count).cloned().collect()))
        }
        Value::Function(f) => {
            // skip-while: drop while predicate true, then keep rest
            let mut start = 0usize;
            for v in list {
                if predicate_holds(f, v, "List.Skip")? {
                    start += 1;
                } else {
                    break;
                }
            }
            Ok(Value::List(list[start..].to_vec()))
        }
        other => Err(type_mismatch("non-negative integer or function", other)),
    }
}


fn combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn accumulate(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut acc = args[1].clone();
    let f = expect_function(&args[2])?;
    for item in list {
        acc = invoke_callback_with_host(f, vec![acc, item.clone()], host)?;
    }
    Ok(acc)
}

fn contains(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let target = &args[1];
    // equationCriteria (arg 2): null/missing → default primitive equality.
    // Function `(x, y) => logical` → call per pair and use its boolean.
    // Other shapes (records, numbers for case-folding text comparison)
    // aren't supported yet.
    let criteria = equation_criteria_fn(args, 2, "List.Contains")?;
    for v in list {
        if eq_via_criteria(v, target, criteria)? {
            return Ok(Value::Logical(true));
        }
    }
    Ok(Value::Logical(false))
}

/// Extract an optional equationCriteria function arg. `Null`/missing →
/// `None` (caller falls back to primitive equality). `Function` → `Some(_)`.
/// Other shapes are not yet supported.
fn equation_criteria_fn<'a>(
    args: &'a [Value],
    idx: usize,
    fn_name: &str,
) -> Result<Option<&'a Closure>, MError> {
    match args.get(idx) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::Function(c)) => Ok(Some(c)),
        Some(other) => Err(MError::Other(format!(
            "{fn_name}: equationCriteria as {} not yet supported (function only)",
            super::super::type_name(other),
        ))),
    }
}

/// Occurrence mode: First (default), Last, All. Used by PositionOf-family.
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

/// Build the return value of a PositionOf-family scan according to the
/// occurrence mode and the matches list (in encounter order).
fn occurrence_result(mode: Occurrence, matches: &[usize]) -> Value {
    match mode {
        Occurrence::First => Value::Number(matches.first().copied().map(|i| i as f64).unwrap_or(-1.0)),
        Occurrence::Last => Value::Number(matches.last().copied().map(|i| i as f64).unwrap_or(-1.0)),
        Occurrence::All => Value::List(matches.iter().map(|&i| Value::Number(i as f64)).collect()),
    }
}

/// Compare two values via the optional equationCriteria function — fall
/// back to primitive equality when `criteria` is `None`.
fn eq_via_criteria(
    a: &Value,
    b: &Value,
    criteria: Option<&Closure>,
) -> Result<bool, MError> {
    match criteria {
        None => values_equal_primitive(a, b),
        Some(f) => {
            let r = invoke_builtin_callback(f, vec![a.clone(), b.clone()])?;
            match r {
                Value::Logical(b) => Ok(b),
                other => Err(type_mismatch("logical (from equationCriteria)", &other)),
            }
        }
    }
}

fn contains_all(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let values = expect_list(&args[1])?;
    let criteria = equation_criteria_fn(args, 2, "List.ContainsAll")?;
    for v in values {
        let mut found = false;
        for x in list {
            if eq_via_criteria(x, v, criteria)? { found = true; break; }
        }
        if !found { return Ok(Value::Logical(false)); }
    }
    Ok(Value::Logical(true))
}

fn contains_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let values = expect_list(&args[1])?;
    let criteria = equation_criteria_fn(args, 2, "List.ContainsAny")?;
    for v in values {
        for x in list {
            if eq_via_criteria(x, v, criteria)? { return Ok(Value::Logical(true)); }
        }
    }
    Ok(Value::Logical(false))
}

fn is_distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let criteria = equation_criteria_fn(args, 1, "List.IsDistinct")?;
    for (i, a) in list.iter().enumerate() {
        for b in &list[i + 1..] {
            if eq_via_criteria(a, b, criteria)? {
                return Ok(Value::Logical(false));
            }
        }
    }
    Ok(Value::Logical(true))
}

fn matches_all(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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

fn matches_any(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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

fn find_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let needle = expect_text(&args[1])?;
    let mut out: Vec<Value> = Vec::new();
    for v in list {
        if let Value::Text(s) = v
            && s.contains(needle) {
                out.push(v.clone());
            }
    }
    Ok(Value::List(out))
}

fn position_of_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let values = expect_list(&args[1])?;
    let mode = parse_occurrence(args.get(2), "List.PositionOfAny")?;
    let criteria = equation_criteria_fn(args, 3, "List.PositionOfAny")?;
    let mut matches: Vec<usize> = Vec::new();
    'outer: for (i, v) in list.iter().enumerate() {
        for t in values {
            if eq_via_criteria(v, t, criteria)? {
                matches.push(i);
                if mode == Occurrence::First {
                    break 'outer;
                }
                break;
            }
        }
    }
    Ok(occurrence_result(mode, &matches))
}

fn positions(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::List((0..list.len()).map(|i| Value::Number(i as f64)).collect()))
}

fn range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn remove_last_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => {
            let keep = list.len().saturating_sub(*n as usize);
            Ok(Value::List(list[..keep].to_vec()))
        }
        Some(Value::Function(f)) => {
            // drop tail items while predicate true, scan from end
            let mut keep = list.len();
            for v in list.iter().rev() {
                if predicate_holds(f, v, "List.RemoveLastN")? {
                    keep -= 1;
                } else {
                    break;
                }
            }
            Ok(Value::List(list[..keep].to_vec()))
        }
        Some(Value::Null) | None => {
            let keep = list.len().saturating_sub(1);
            Ok(Value::List(list[..keep].to_vec()))
        }
        Some(other) => Err(type_mismatch("non-negative integer or function", other)),
    }
}

fn remove_nulls(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::List(list.iter().filter(|v| !matches!(v, Value::Null)).cloned().collect()))
}

fn remove_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn insert_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn replace_matching_items(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn replace_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn replace_value(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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

fn repeat(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn times(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let value = &args[0];
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer (count)", other)),
    };
    Ok(Value::List(vec![value.clone(); count]))
}

/// Power BI engine hook for paged data sources — "intended for internal
/// use only" per MS docs. MS's signature returns a table; mrsflow has
/// no paging engine, so we return the list unchanged. Real user code
/// won't call this; the binding exists so name lookup doesn't fail if
/// someone copy-pastes a Power BI export.
fn conform_to_page_reader(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_list(&args[0])?;
    Ok(args[0].clone())
}

fn alternate(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn split(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn buffer(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::List(list.clone()))
}

fn difference(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list1 = expect_list(&args[0])?;
    let list2 = expect_list(&args[1])?;
    let criteria = equation_criteria_fn(args, 2, "List.Difference")?;
    // PQ semantics: multiplicities are subtracted (each list2 occurrence removes
    // one matching list1 occurrence, in order).
    let mut remaining: Vec<bool> = vec![true; list1.len()];
    for v in list2 {
        for (i, x) in list1.iter().enumerate() {
            if remaining[i] && eq_via_criteria(x, v, criteria)? {
                remaining[i] = false;
                break;
            }
        }
    }
    let out: Vec<Value> = list1.iter().enumerate()
        .filter(|(i, _)| remaining[*i])
        .map(|(_, v)| v.clone())
        .collect();
    Ok(Value::List(out))
}

fn intersect(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lists = expect_list(&args[0])?;
    let criteria = equation_criteria_fn(args, 1, "List.Intersect")?;
    if lists.is_empty() {
        return Ok(Value::List(Vec::new()));
    }
    let sublists: Vec<&Vec<Value>> = lists.iter().map(|v| expect_list(v)).collect::<Result<_, _>>()?;
    let first = sublists[0];
    let mut out: Vec<Value> = Vec::new();
    'outer: for v in first {
        // Skip if already added (dedupe in result).
        for existing in &out {
            if eq_via_criteria(existing, v, criteria)? { continue 'outer; }
        }
        // Must appear in every other sublist.
        for other in &sublists[1..] {
            let mut found = false;
            for x in *other {
                if eq_via_criteria(v, x, criteria)? { found = true; break; }
            }
            if !found { continue 'outer; }
        }
        out.push(v.clone());
    }
    Ok(Value::List(out))
}

fn dates(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let start = match &args[0] {
        Value::Date(d) => *d,
        other => return Err(type_mismatch("date (start)", other)),
    };
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer (count)", other)),
    };
    let step = match &args[2] {
        Value::Duration(d) => *d,
        other => return Err(type_mismatch("duration (step)", other)),
    };
    let mut out = Vec::with_capacity(count);
    let mut cur = start;
    for _ in 0..count {
        out.push(Value::Date(cur));
        cur = cur.checked_add_signed(step)
            .ok_or_else(|| MError::Other("List.Dates: result out of range".into()))?;
    }
    Ok(Value::List(out))
}

fn datetimes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let start = match &args[0] {
        Value::Datetime(dt) => *dt,
        other => return Err(type_mismatch("datetime (start)", other)),
    };
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer (count)", other)),
    };
    let step = match &args[2] {
        Value::Duration(d) => *d,
        other => return Err(type_mismatch("duration (step)", other)),
    };
    let mut out = Vec::with_capacity(count);
    let mut cur = start;
    for _ in 0..count {
        out.push(Value::Datetime(cur));
        cur = cur.checked_add_signed(step)
            .ok_or_else(|| MError::Other("List.DateTimes: result out of range".into()))?;
    }
    Ok(Value::List(out))
}

fn datetimezones(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let start = match &args[0] {
        Value::Datetimezone(dt) => *dt,
        other => return Err(type_mismatch("datetimezone (start)", other)),
    };
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer (count)", other)),
    };
    let step = match &args[2] {
        Value::Duration(d) => *d,
        other => return Err(type_mismatch("duration (step)", other)),
    };
    let mut out = Vec::with_capacity(count);
    let mut cur = start;
    for _ in 0..count {
        out.push(Value::Datetimezone(cur));
        cur = cur.checked_add_signed(step)
            .ok_or_else(|| MError::Other("List.DateTimeZones: result out of range".into()))?;
    }
    Ok(Value::List(out))
}

fn durations(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let start = match &args[0] {
        Value::Duration(d) => *d,
        other => return Err(type_mismatch("duration (start)", other)),
    };
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer (count)", other)),
    };
    let step = match &args[2] {
        Value::Duration(d) => *d,
        other => return Err(type_mismatch("duration (step)", other)),
    };
    let mut out = Vec::with_capacity(count);
    let mut cur = start;
    for _ in 0..count {
        out.push(Value::Duration(cur));
        cur = cur.checked_add(&step)
            .ok_or_else(|| MError::Other("List.Durations: result out of range".into()))?;
    }
    Ok(Value::List(out))
}

fn transform_many(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let collection_fn = expect_function(&args[1])?;
    let result_fn = expect_function(&args[2])?;
    let mut out: Vec<Value> = Vec::new();
    for item in list {
        let inner = invoke_callback_with_host(collection_fn, vec![item.clone()], host)?;
        let inner_list = match &inner {
            Value::List(xs) => xs.clone(),
            other => return Err(MError::Other(format!(
                "List.TransformMany: collectionTransform must return a list (got {})",
                super::super::type_name(other)
            ))),
        };
        for inner_item in inner_list {
            let mapped = invoke_callback_with_host(
                result_fn,
                vec![item.clone(), inner_item],
                host,
            )?;
            out.push(mapped);
        }
    }
    Ok(Value::List(out))
}

fn generate(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let initial_fn = expect_function(&args[0])?;
    let condition_fn = expect_function(&args[1])?;
    let next_fn = expect_function(&args[2])?;
    let selector_fn: Option<&Closure> = match args.get(3) {
        Some(Value::Function(c)) => Some(c),
        Some(Value::Null) | None => None,
        Some(other) => return Err(type_mismatch("function (selector)", other)),
    };
    let mut state = invoke_callback_with_host(initial_fn, vec![], host)?;
    let mut out: Vec<Value> = Vec::new();
    let mut iters = 0;
    while iters < 100_000 {
        let cond = invoke_callback_with_host(condition_fn, vec![state.clone()], host)?;
        match cond {
            Value::Logical(false) => break,
            Value::Logical(true) => {}
            other => return Err(type_mismatch("logical", &other)),
        }
        let item = match selector_fn {
            Some(s) => invoke_callback_with_host(s, vec![state.clone()], host)?,
            None => state.clone(),
        };
        out.push(item);
        state = invoke_callback_with_host(next_fn, vec![state], host)?;
        iters += 1;
    }
    if iters >= 100_000 {
        return Err(MError::Other("List.Generate: exceeded 100000 iteration cap".into()));
    }
    Ok(Value::List(out))
}

fn numbers_only(list: &[Value], ctx: &str) -> Result<Vec<f64>, MError> {
    let mut out = Vec::with_capacity(list.len());
    for v in list {
        match v {
            Value::Number(n) => out.push(*n),
            Value::Null => continue,
            other => return Err(MError::Other(format!(
                "{}: expected number, got {}", ctx, super::super::type_name(other)
            ))),
        }
    }
    Ok(out)
}

fn single(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    match list.len() {
        0 => Err(MError::Other("List.Single: list is empty".into())),
        1 => Ok(list[0].clone()),
        n => Err(MError::Other(format!("List.Single: expected exactly one element, got {n}"))),
    }
}

fn single_or_default(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    match list.len() {
        0 => Ok(args.get(1).cloned().unwrap_or(Value::Null)),
        1 => Ok(list[0].clone()),
        n => Err(MError::Other(format!("List.SingleOrDefault: expected at most one element, got {n}"))),
    }
}

fn median(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut nums = numbers_only(list, "List.Median")?;
    if nums.is_empty() {
        return Ok(Value::Null);
    }
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = nums.len();
    let median = if n % 2 == 1 {
        nums[n / 2]
    } else {
        (nums[n / 2 - 1] + nums[n / 2]) / 2.0
    };
    Ok(Value::Number(median))
}

fn mode(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let criteria = equation_criteria_fn(args, 1, "List.Mode")?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    // Tally each distinct value's count, preserving first-seen order.
    let mut tally: Vec<(Value, usize)> = Vec::new();
    for v in list {
        let mut matched = false;
        for (k, c) in tally.iter_mut() {
            if eq_via_criteria(k, v, criteria)? {
                *c += 1;
                matched = true;
                break;
            }
        }
        if !matched {
            tally.push((v.clone(), 1));
        }
    }
    let max = tally.iter().map(|(_, c)| *c).max().unwrap();
    let (v, _) = tally.into_iter().find(|(_, c)| *c == max).unwrap();
    Ok(v)
}

fn modes(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let criteria = equation_criteria_fn(args, 1, "List.Modes")?;
    if list.is_empty() {
        return Ok(Value::List(Vec::new()));
    }
    let mut tally: Vec<(Value, usize)> = Vec::new();
    for v in list {
        let mut matched = false;
        for (k, c) in tally.iter_mut() {
            if eq_via_criteria(k, v, criteria)? {
                *c += 1;
                matched = true;
                break;
            }
        }
        if !matched {
            tally.push((v.clone(), 1));
        }
    }
    let max = tally.iter().map(|(_, c)| *c).max().unwrap();
    let out: Vec<Value> = tally.into_iter()
        .filter(|(_, c)| *c == max)
        .map(|(v, _)| v)
        .collect();
    Ok(Value::List(out))
}

fn product(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let nums = numbers_only(list, "List.Product")?;
    if nums.is_empty() {
        return Ok(Value::Null);
    }
    Ok(Value::Number(nums.iter().product()))
}

fn sort_numeric_or_text(list: &[Value], ctx: &str, descending: bool) -> Result<Vec<Value>, MError> {
    enum Kind { Empty, Num, Text }
    let mut kind = Kind::Empty;
    for v in list {
        let k = match v {
            Value::Number(_) => Kind::Num,
            Value::Text(_) => Kind::Text,
            Value::Null => continue,
            other => return Err(MError::Other(format!(
                "{}: expected number or text, got {}", ctx, super::super::type_name(other)
            ))),
        };
        match (&kind, &k) {
            (Kind::Empty, _) => kind = k,
            (Kind::Num, Kind::Num) | (Kind::Text, Kind::Text) => {}
            _ => return Err(MError::Other(format!(
                "{ctx}: mixed-type list not supported"
            ))),
        }
    }
    let mut out: Vec<Value> = list.iter().filter(|v| !matches!(v, Value::Null)).cloned().collect();
    match kind {
        Kind::Empty => {}
        Kind::Num => out.sort_by(|a, b| {
            let (Value::Number(x), Value::Number(y)) = (a, b) else { unreachable!() };
            let c = x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal);
            if descending { c.reverse() } else { c }
        }),
        Kind::Text => out.sort_by(|a, b| {
            let (Value::Text(x), Value::Text(y)) = (a, b) else { unreachable!() };
            let c = x.cmp(y);
            if descending { c.reverse() } else { c }
        }),
    }
    Ok(out)
}

fn max_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let n = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let sorted = sort_numeric_or_text(list, "List.MaxN", true)?;
    Ok(Value::List(sorted.into_iter().take(n).collect()))
}

fn min_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let n = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let sorted = sort_numeric_or_text(list, "List.MinN", false)?;
    Ok(Value::List(sorted.into_iter().take(n).collect()))
}

fn non_null_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let n = list.iter().filter(|v| !matches!(v, Value::Null)).count();
    Ok(Value::Number(n as f64))
}

fn standard_deviation(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let nums = numbers_only(list, "List.StandardDeviation")?;
    if nums.len() < 2 {
        return Ok(Value::Null);
    }
    let mean: f64 = nums.iter().sum::<f64>() / nums.len() as f64;
    let var: f64 = nums.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (nums.len() - 1) as f64;
    Ok(Value::Number(var.sqrt()))
}

fn covariance(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = numbers_only(expect_list(&args[0])?, "List.Covariance: list1")?;
    let ys = numbers_only(expect_list(&args[1])?, "List.Covariance: list2")?;
    if xs.len() != ys.len() {
        return Err(MError::Other("List.Covariance: lists must have equal length".into()));
    }
    if xs.len() < 2 {
        return Ok(Value::Null);
    }
    let mx: f64 = xs.iter().sum::<f64>() / xs.len() as f64;
    let my: f64 = ys.iter().sum::<f64>() / ys.len() as f64;
    let cov: f64 = xs.iter().zip(ys.iter())
        .map(|(x, y)| (x - mx) * (y - my))
        .sum::<f64>() / (xs.len() - 1) as f64;
    Ok(Value::Number(cov))
}

fn percentile(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut nums = numbers_only(list, "List.Percentile")?;
    let p = match &args[1] {
        Value::Number(n) if *n >= 0.0 && *n <= 1.0 => *n,
        other => return Err(MError::Other(format!(
            "List.Percentile: percentile must be in [0,1] (got {other:?})"
        ))),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("List.Percentile: options not yet supported"));
    }
    if nums.is_empty() {
        return Ok(Value::Null);
    }
    nums.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let n = nums.len();
    // Linear interpolation between adjacent ranks.
    let rank = p * (n - 1) as f64;
    let lo = rank.floor() as usize;
    let hi = rank.ceil() as usize;
    let frac = rank - lo as f64;
    Ok(Value::Number(nums[lo] + frac * (nums[hi] - nums[lo])))
}

fn random(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let count = match &args[0] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented("List.Random: seed not yet supported"));
    }
    let out: Vec<Value> = (0..count).map(|_| Value::Number(rand::random::<f64>())).collect();
    Ok(Value::List(out))
}

fn union_(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lists = expect_list(&args[0])?;
    let criteria = equation_criteria_fn(args, 1, "List.Union")?;
    let mut out: Vec<Value> = Vec::new();
    for v in lists {
        let sub = expect_list(v)?;
        'inner: for x in sub {
            for existing in &out {
                if eq_via_criteria(existing, x, criteria)? { continue 'inner; }
            }
            out.push(x.clone());
        }
    }
    Ok(Value::List(out))
}

