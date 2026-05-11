//! Starter stdlib for eval-6: pure functions bound in the root env.
//!
//! Each function lives in this module as a `BuiltinFn`. `root_env()` builds
//! the initial env containing every binding, used by callers that want a
//! stdlib-aware environment instead of an empty one (`EnvNode::empty()`).
//!
//! Function scope is corpus-driven: the top non-Arrow stdlib calls in the
//! user's actual queries (`Text.Replace`, `Text.Contains`, `List.Transform`,
//! `Number.From`, …). Arrow-backed Table.* and date/datetime/duration land
//! in eval-7+.

use std::sync::Arc;

use arrow::array::{Array, ArrayRef, BooleanArray, Float64Array, NullArray, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::parser::Param;

use super::env::{Env, EnvNode, EnvOps};
use super::value::{BuiltinFn, Closure, FnBody, MError, Table, Value};

/// Build the initial environment containing every stdlib intrinsic plus
/// the two literal constants `#nan` and `#infinity`. Tests and shells pass
/// this as the starting env instead of `EnvNode::empty()`.
pub fn root_env() -> Env {
    let mut env = EnvNode::empty();
    for (name, params, body) in builtin_bindings() {
        let closure = Closure {
            params,
            body: FnBody::Builtin(body),
            env: EnvNode::empty(),
        };
        env = env.extend(name.to_string(), Value::Function(closure));
    }
    env = env.extend("#nan".into(), Value::Number(f64::NAN));
    env = env.extend("#infinity".into(), Value::Number(f64::INFINITY));
    env
}

fn one(name: &str) -> Vec<Param> {
    vec![Param {
        name: name.into(),
        optional: false,
        type_annotation: None,
    }]
}

fn two(a: &str, b: &str) -> Vec<Param> {
    vec![
        Param {
            name: a.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: b.into(),
            optional: false,
            type_annotation: None,
        },
    ]
}

fn three(a: &str, b: &str, c: &str) -> Vec<Param> {
    vec![
        Param {
            name: a.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: b.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: c.into(),
            optional: false,
            type_annotation: None,
        },
    ]
}

fn builtin_bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Number.From", one("value"), number_from),
        ("Text.From", one("value"), text_from),
        ("Text.Contains", two("text", "substring"), text_contains),
        ("Text.Replace", three("text", "old", "new"), text_replace),
        ("Text.Trim", one("text"), text_trim),
        ("Text.Length", one("text"), text_length),
        ("Text.PositionOf", two("text", "substring"), text_position_of),
        ("Text.EndsWith", two("text", "suffix"), text_ends_with),
        ("List.Transform", two("list", "transform"), list_transform),
        ("List.Select", two("list", "selection"), list_select),
        ("List.Sum", one("list"), list_sum),
        ("List.Count", one("list"), list_count),
        ("List.Min", one("list"), list_min),
        ("List.Max", one("list"), list_max),
        ("Record.Field", two("record", "field"), record_field),
        ("Record.FieldNames", one("record"), record_field_names),
        ("Logical.From", one("value"), logical_from),
        ("Logical.FromText", one("text"), logical_from_text),
        ("#table", two("columns", "rows"), table_constructor),
        ("Table.ColumnNames", one("table"), table_column_names),
        ("Table.RenameColumns", two("table", "renames"), table_rename_columns),
        ("Table.RemoveColumns", two("table", "names"), table_remove_columns),
        (
            "#date",
            three("year", "month", "day"),
            date_constructor,
        ),
        (
            "#datetime",
            vec![
                Param { name: "year".into(),   optional: false, type_annotation: None },
                Param { name: "month".into(),  optional: false, type_annotation: None },
                Param { name: "day".into(),    optional: false, type_annotation: None },
                Param { name: "hour".into(),   optional: false, type_annotation: None },
                Param { name: "minute".into(), optional: false, type_annotation: None },
                Param { name: "second".into(), optional: false, type_annotation: None },
            ],
            datetime_constructor,
        ),
        (
            "#duration",
            vec![
                Param { name: "days".into(),    optional: false, type_annotation: None },
                Param { name: "hours".into(),   optional: false, type_annotation: None },
                Param { name: "minutes".into(), optional: false, type_annotation: None },
                Param { name: "seconds".into(), optional: false, type_annotation: None },
            ],
            duration_constructor,
        ),
    ]
}

fn type_mismatch(expected: &'static str, found: &Value) -> MError {
    MError::TypeMismatch {
        expected,
        found: super::type_name(found),
    }
}

// --- Number.* ---

fn number_from(args: &[Value]) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(*n)),
        Value::Logical(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
        Value::Text(s) => s
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| MError::Other(format!("Number.From: cannot parse {:?}", s))),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

// --- Text.* ---

fn text_from(args: &[Value]) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => Ok(Value::Text(s.clone())),
        // {:?} for f64 matches `value_dump`'s canonical num format
        // (always-trailing fractional digit). Keeping parity here so
        // Text.From(42) prints the same as the differential's `(num 42.0)`.
        Value::Number(n) => Ok(Value::Text(format!("{:?}", n))),
        Value::Logical(b) => Ok(Value::Text(
            if *b { "true" } else { "false" }.to_string(),
        )),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

fn text_contains(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    Ok(Value::Logical(text.contains(sub)))
}

fn text_replace(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    Ok(Value::Text(text.replace(old, new)))
}

fn text_trim(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.trim().to_string()))
}

fn text_length(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // M counts characters, not bytes — use char count.
    Ok(Value::Number(text.chars().count() as f64))
}

fn text_position_of(args: &[Value]) -> Result<Value, MError> {
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

fn text_ends_with(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let suffix = expect_text(&args[1])?;
    Ok(Value::Logical(text.ends_with(suffix)))
}

fn expect_text(v: &Value) -> Result<&str, MError> {
    match v {
        Value::Text(s) => Ok(s.as_str()),
        other => Err(type_mismatch("text", other)),
    }
}

// --- List.* ---

fn list_transform(args: &[Value]) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let f = expect_function(&args[1])?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let v = invoke_builtin_callback(f, vec![item.clone()])?;
        out.push(v);
    }
    Ok(Value::List(out))
}

fn list_select(args: &[Value]) -> Result<Value, MError> {
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

fn list_sum(args: &[Value]) -> Result<Value, MError> {
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

fn list_count(args: &[Value]) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Number(list.len() as f64))
}

fn list_min(args: &[Value]) -> Result<Value, MError> {
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

fn list_max(args: &[Value]) -> Result<Value, MError> {
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

fn expect_list(v: &Value) -> Result<&Vec<Value>, MError> {
    match v {
        Value::List(xs) => Ok(xs),
        other => Err(type_mismatch("list", other)),
    }
}

fn expect_function(v: &Value) -> Result<&Closure, MError> {
    match v {
        Value::Function(c) => Ok(c),
        other => Err(type_mismatch("function", other)),
    }
}

/// Call a closure from within a builtin. Builtins receive already-forced
/// values, so we need only mirror the body-dispatch part of the evaluator's
/// Invoke handling. For M-bodied closures we run the body in the captured
/// env; for nested builtins we recurse directly.
///
/// Slice-6 stdlib needs IoHost-free recursion only — List.Transform's
/// callback fn cannot itself perform IO since builtins don't carry a host.
/// Pass a no-op host to be safe.
fn invoke_builtin_callback(closure: &Closure, args: Vec<Value>) -> Result<Value, MError> {
    if args.len() != closure.params.len() {
        return Err(MError::Other(format!(
            "callback: arity mismatch: expected {}, got {}",
            closure.params.len(),
            args.len()
        )));
    }
    match &closure.body {
        FnBody::Builtin(f) => f(&args),
        FnBody::M(body) => {
            let mut call_env = closure.env.clone();
            for (param, value) in closure.params.iter().zip(args.into_iter()) {
                call_env = call_env.extend(param.name.clone(), value);
            }
            super::evaluate(body, &call_env, &super::NoIoHost)
        }
    }
}

// --- Record.* ---

fn record_field(args: &[Value]) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let name = expect_text(&args[1])?;
    match record.fields.iter().find(|(n, _)| n == name) {
        Some((_, v)) => super::force(v.clone(), &mut |e, env| {
            super::evaluate(e, env, &super::NoIoHost)
        }),
        None => Err(MError::Other(format!("Record.Field: field not found: {}", name))),
    }
}

fn record_field_names(args: &[Value]) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let names: Vec<Value> = record
        .fields
        .iter()
        .map(|(n, _)| Value::Text(n.clone()))
        .collect();
    Ok(Value::List(names))
}

// --- Logical.* ---

fn logical_from(args: &[Value]) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Logical(b) => Ok(Value::Logical(*b)),
        Value::Number(n) => Ok(Value::Logical(*n != 0.0)),
        Value::Text(_) => logical_from_text(args),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

fn logical_from_text(args: &[Value]) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    match text.to_ascii_lowercase().as_str() {
        "true" => Ok(Value::Logical(true)),
        "false" => Ok(Value::Logical(false)),
        _ => Err(MError::Other(format!(
            "Logical.FromText: not a boolean: {:?}",
            text
        ))),
    }
}


// --- Table.* (eval-7a) ---
//
// #table(columns, rows) and the three top-corpus Table.* operations.
// Compound type expressions in the columns position aren't supported in
// this slice — only a list of text column names. Date/Datetime/Duration/
// Binary cells land in eval-7b alongside chrono.

fn table_constructor(args: &[Value]) -> Result<Value, MError> {
    let names = expect_text_list(&args[0], "#table: columns")?;
    let rows = expect_list_of_lists(&args[1], "#table: rows")?;
    for (i, row) in rows.iter().enumerate() {
        if row.len() != names.len() {
            return Err(MError::Other(format!(
                "#table: row {} has {} cells, expected {}",
                i,
                row.len(),
                names.len()
            )));
        }
    }
    let batch = values_to_record_batch(&names, &rows)?;
    Ok(Value::Table(Table { batch }))
}

fn table_column_names(args: &[Value]) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names: Vec<Value> = table
        .batch
        .schema()
        .fields()
        .iter()
        .map(|f| Value::Text(f.name().clone()))
        .collect();
    Ok(Value::List(names))
}

fn table_rename_columns(args: &[Value]) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let renames = expect_list(&args[1])?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    for r in renames {
        let inner = match r {
            Value::List(xs) => xs,
            other => {
                return Err(type_mismatch(
                    "list (each rename must be {old, new})",
                    other,
                ));
            }
        };
        if inner.len() != 2 {
            return Err(MError::Other(format!(
                "Table.RenameColumns: each rename must be a 2-element list, got {}",
                inner.len()
            )));
        }
        let old = expect_text(&inner[0])?.to_string();
        let new = expect_text(&inner[1])?.to_string();
        pairs.push((old, new));
    }
    let schema = table.batch.schema();
    let existing: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();
    for (old, _new) in &pairs {
        if !existing.contains(old) {
            return Err(MError::Other(format!(
                "Table.RenameColumns: column not found: {}",
                old
            )));
        }
    }
    let new_fields: Vec<Field> = schema
        .fields()
        .iter()
        .map(|f| {
            let mut name = f.name().clone();
            for (old, new) in &pairs {
                if &name == old {
                    name = new.clone();
                    break;
                }
            }
            Field::new(name, f.data_type().clone(), f.is_nullable())
        })
        .collect();
    let new_schema = Arc::new(Schema::new(new_fields));
    let columns: Vec<ArrayRef> = table.batch.columns().to_vec();
    let new_batch = RecordBatch::try_new(new_schema, columns)
        .map_err(|e| MError::Other(format!("Table.RenameColumns: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

fn table_remove_columns(args: &[Value]) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = expect_text_list(&args[1], "Table.RemoveColumns: names")?;
    let schema = table.batch.schema();
    let existing: Vec<String> = schema.fields().iter().map(|f| f.name().clone()).collect();
    for n in &names {
        if !existing.contains(n) {
            return Err(MError::Other(format!(
                "Table.RemoveColumns: column not found: {}",
                n
            )));
        }
    }
    let keep_indices: Vec<usize> = (0..existing.len())
        .filter(|&i| !names.contains(&existing[i]))
        .collect();
    let new_fields: Vec<Field> = keep_indices
        .iter()
        .map(|&i| schema.field(i).clone())
        .collect();
    let new_schema = Arc::new(Schema::new(new_fields));
    let new_columns: Vec<ArrayRef> = keep_indices
        .iter()
        .map(|&i| table.batch.column(i).clone())
        .collect();
    let new_batch = RecordBatch::try_new(new_schema, new_columns)
        .map_err(|e| MError::Other(format!("Table.RemoveColumns: rebuild failed: {}", e)))?;
    Ok(Value::Table(Table { batch: new_batch }))
}

// --- Table helpers ---

fn expect_table(v: &Value) -> Result<&Table, MError> {
    match v {
        Value::Table(t) => Ok(t),
        other => Err(type_mismatch("table", other)),
    }
}

fn expect_text_list(v: &Value, ctx: &str) -> Result<Vec<String>, MError> {
    let xs = expect_list(v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match x {
            Value::Text(s) => out.push(s.clone()),
            other => {
                return Err(MError::Other(format!(
                    "{}: expected list of text, got {}",
                    ctx,
                    super::type_name(other)
                )));
            }
        }
    }
    Ok(out)
}

fn expect_list_of_lists<'a>(v: &'a Value, ctx: &str) -> Result<Vec<Vec<Value>>, MError> {
    let xs = expect_list(v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match x {
            Value::List(inner) => out.push(inner.clone()),
            other => {
                return Err(MError::Other(format!(
                    "{}: expected list of lists, got {}",
                    ctx,
                    super::type_name(other)
                )));
            }
        }
    }
    Ok(out)
}

/// Infer a column type by scanning all cells. Supported variants in this
/// slice: Number → Float64, Text → Utf8, Logical → Boolean, Null. A column
/// of all-null produces an Arrow NullArray. Mixed primitive kinds within
/// one column → MError::Other.
fn values_to_record_batch(
    column_names: &[String],
    rows: &[Vec<Value>],
) -> Result<RecordBatch, MError> {
    let n_rows = rows.len();
    let n_cols = column_names.len();

    // Special case: schema with zero columns isn't constructible via the
    // standard RecordBatch path. Caller still wants a real Table value
    // back, so build an empty-schema batch with the correct row count.
    if n_cols == 0 {
        let schema = Arc::new(Schema::empty());
        let options =
            arrow::record_batch::RecordBatchOptions::new().with_row_count(Some(n_rows));
        return RecordBatch::try_new_with_options(schema, vec![], &options)
            .map_err(|e| MError::Other(format!("#table: empty-cols rebuild failed: {}", e)));
    }

    let mut fields: Vec<Field> = Vec::with_capacity(n_cols);
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(n_cols);
    for col_idx in 0..n_cols {
        let (dtype, array) = infer_column(rows, col_idx, n_rows)?;
        let is_nullable = matches!(dtype, DataType::Null) || column_has_null(rows, col_idx);
        fields.push(Field::new(column_names[col_idx].clone(), dtype, is_nullable));
        columns.push(array);
    }
    let schema = Arc::new(Schema::new(fields));
    RecordBatch::try_new(schema, columns)
        .map_err(|e| MError::Other(format!("#table: build failed: {}", e)))
}

fn column_has_null(rows: &[Vec<Value>], col: usize) -> bool {
    rows.iter().any(|r| matches!(r[col], Value::Null))
}

fn infer_column(
    rows: &[Vec<Value>],
    col: usize,
    n_rows: usize,
) -> Result<(DataType, ArrayRef), MError> {
    // Find first non-null cell to determine column kind.
    let mut kind: Option<&'static str> = None;
    for r in rows {
        match &r[col] {
            Value::Null => {}
            Value::Number(_) => {
                kind = Some("number");
                break;
            }
            Value::Text(_) => {
                kind = Some("text");
                break;
            }
            Value::Logical(_) => {
                kind = Some("logical");
                break;
            }
            other => {
                return Err(MError::NotImplemented(match other {
                    Value::Date(_) | Value::Datetime(_) | Value::Duration(_) => {
                        "date/datetime/duration cells (deferred to eval-7b)"
                    }
                    Value::Binary(_) => "binary cells (deferred)",
                    _ => "non-primitive cell type (deferred)",
                }));
            }
        }
    }
    match kind {
        None => Ok((
            DataType::Null,
            Arc::new(NullArray::new(n_rows)) as ArrayRef,
        )),
        Some("number") => {
            let values: Vec<Option<f64>> = rows
                .iter()
                .map(|r| match &r[col] {
                    Value::Null => Ok(None),
                    Value::Number(n) => Ok(Some(*n)),
                    other => Err(MError::Other(format!(
                        "#table: column {} mixed types: number + {}",
                        col,
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((DataType::Float64, Arc::new(Float64Array::from(values))))
        }
        Some("text") => {
            let values: Vec<Option<String>> = rows
                .iter()
                .map(|r| match &r[col] {
                    Value::Null => Ok(None),
                    Value::Text(s) => Ok(Some(s.clone())),
                    other => Err(MError::Other(format!(
                        "#table: column {} mixed types: text + {}",
                        col,
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((DataType::Utf8, Arc::new(StringArray::from(values))))
        }
        Some("logical") => {
            let values: Vec<Option<bool>> = rows
                .iter()
                .map(|r| match &r[col] {
                    Value::Null => Ok(None),
                    Value::Logical(b) => Ok(Some(*b)),
                    other => Err(MError::Other(format!(
                        "#table: column {} mixed types: logical + {}",
                        col,
                        super::type_name(other)
                    ))),
                })
                .collect::<Result<_, _>>()?;
            Ok((DataType::Boolean, Arc::new(BooleanArray::from(values))))
        }
        _ => unreachable!(),
    }
}

/// Convert a single cell of a RecordBatch back to a Value. Used by
/// value_dump's table printer.
pub fn cell_to_value(batch: &RecordBatch, col: usize, row: usize) -> Result<Value, MError> {
    let array = batch.column(col);
    if array.is_null(row) {
        return Ok(Value::Null);
    }
    match array.data_type() {
        DataType::Float64 => {
            let a = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .expect("Float64");
            Ok(Value::Number(a.value(row)))
        }
        DataType::Utf8 => {
            let a = array.as_any().downcast_ref::<StringArray>().expect("Utf8");
            Ok(Value::Text(a.value(row).to_string()))
        }
        DataType::Boolean => {
            let a = array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .expect("Boolean");
            Ok(Value::Logical(a.value(row)))
        }
        DataType::Null => Ok(Value::Null),
        other => Err(MError::NotImplemented(match other {
            DataType::Date32 | DataType::Date64 | DataType::Timestamp(_, _) => {
                "date/datetime cell decode (deferred to eval-7b)"
            }
            _ => "unsupported cell type",
        })),
    }
}

// --- chrono constructors (eval-7b) ---
//
// #date(y,m,d), #datetime(y,m,d,h,m,s), #duration(d,h,m,s). All operands
// must be whole-numbered f64s; non-integer or out-of-range values error.

fn date_constructor(args: &[Value]) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#date: year")?;
    let mo = expect_int(&args[1], "#date: month")?;
    let d = expect_int(&args[2], "#date: day")?;
    chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .map(Value::Date)
        .ok_or_else(|| MError::Other(format!("#date: invalid date {}-{:02}-{:02}", y, mo, d)))
}

fn datetime_constructor(args: &[Value]) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#datetime: year")?;
    let mo = expect_int(&args[1], "#datetime: month")?;
    let d = expect_int(&args[2], "#datetime: day")?;
    let h = expect_int(&args[3], "#datetime: hour")?;
    let mn = expect_int(&args[4], "#datetime: minute")?;
    let s = expect_int(&args[5], "#datetime: second")?;
    let date = chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .ok_or_else(|| MError::Other(format!("#datetime: invalid date {}-{:02}-{:02}", y, mo, d)))?;
    let time = chrono::NaiveTime::from_hms_opt(h as u32, mn as u32, s as u32)
        .ok_or_else(|| MError::Other(format!("#datetime: invalid time {:02}:{:02}:{:02}", h, mn, s)))?;
    Ok(Value::Datetime(chrono::NaiveDateTime::new(date, time)))
}

fn duration_constructor(args: &[Value]) -> Result<Value, MError> {
    let d = expect_int(&args[0], "#duration: days")?;
    let h = expect_int(&args[1], "#duration: hours")?;
    let mn = expect_int(&args[2], "#duration: minutes")?;
    let s = expect_int(&args[3], "#duration: seconds")?;
    let total = d
        .checked_mul(86400)
        .and_then(|x| x.checked_add(h.checked_mul(3600)?))
        .and_then(|x| x.checked_add(mn.checked_mul(60)?))
        .and_then(|x| x.checked_add(s))
        .ok_or_else(|| MError::Other("#duration: overflow".into()))?;
    Ok(Value::Duration(chrono::Duration::seconds(total)))
}

fn expect_int(v: &Value, ctx: &str) -> Result<i64, MError> {
    match v {
        Value::Number(n) => {
            if n.fract() != 0.0 {
                return Err(MError::Other(format!("{}: not an integer: {}", ctx, n)));
            }
            Ok(*n as i64)
        }
        other => Err(type_mismatch("number", other)),
    }
}
