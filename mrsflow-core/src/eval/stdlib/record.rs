//! `Record.*` stdlib bindings.

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
        ("Record.Field", two("record", "field"), field),
        ("Record.FieldNames", one("record"), field_names),
        ("Record.FieldValues", one("record"), field_values),
        ("Record.HasFields", two("record", "fields"), has_fields),
        ("Record.Combine", one("records"), combine),
        (
            "Record.FieldOrDefault",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "field".into(),        optional: false, type_annotation: None },
                Param { name: "defaultValue".into(), optional: true,  type_annotation: None },
            ],
            field_or_default,
        ),
        (
            "Record.RemoveFields",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "fields".into(),       optional: false, type_annotation: None },
                Param { name: "missingField".into(), optional: true,  type_annotation: None },
            ],
            remove_fields,
        ),
        (
            "Record.AddField",
            vec![
                Param { name: "record".into(),    optional: false, type_annotation: None },
                Param { name: "fieldName".into(), optional: false, type_annotation: None },
                Param { name: "value".into(),     optional: false, type_annotation: None },
                Param { name: "delayed".into(),   optional: true,  type_annotation: None },
            ],
            add_field,
        ),
        ("Record.FieldCount", one("record"), field_count),
        ("Record.FromList", two("values", "fields"), from_list),
        ("Record.FromTable", one("table"), from_table),
        (
            "Record.RenameFields",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "renames".into(),      optional: false, type_annotation: None },
                Param { name: "missingField".into(), optional: true,  type_annotation: None },
            ],
            rename_fields,
        ),
        (
            "Record.ReorderFields",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "fieldOrder".into(),   optional: false, type_annotation: None },
                Param { name: "missingField".into(), optional: true,  type_annotation: None },
            ],
            reorder_fields,
        ),
        (
            "Record.SelectFields",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "fields".into(),       optional: false, type_annotation: None },
                Param { name: "missingField".into(), optional: true,  type_annotation: None },
            ],
            select_fields,
        ),
        ("Record.ToList", one("record"), to_list),
        ("Record.ToTable", one("record"), to_table),
        (
            "Record.TransformFields",
            vec![
                Param { name: "record".into(),              optional: false, type_annotation: None },
                Param { name: "transformOperations".into(), optional: false, type_annotation: None },
                Param { name: "missingField".into(),        optional: true,  type_annotation: None },
            ],
            transform_fields,
        ),
    ]
}

fn field(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let name = expect_text(&args[1])?;
    match record.fields.iter().find(|(n, _)| n == name) {
        Some((_, v)) => super::super::force(v.clone(), &mut |e, env| {
            super::super::evaluate(e, env, &super::super::NoIoHost)
        }),
        None => Err(MError::Other(format!("Record.Field: field not found: {name}"))),
    }
}


fn field_names(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn field_values(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let values: Result<Vec<Value>, MError> = record
        .fields
        .iter()
        .map(|(_, v)| super::super::force(v.clone(), &mut |e, env| super::super::evaluate(e, env, host)))
        .collect();
    Ok(Value::List(values?))
}


fn has_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let names: Vec<String> = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(xs) => xs
            .iter()
            .map(|v| match v {
                Value::Text(s) => Ok(s.clone()),
                other => Err(type_mismatch("text (in list)", other)),
            })
            .collect::<Result<_, _>>()?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let has_all = names
        .iter()
        .all(|n| record.fields.iter().any(|(fname, _)| fname == n));
    Ok(Value::Logical(has_all))
}


fn combine(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let records = expect_list(&args[0])?;
    let mut fields: Vec<(String, Value)> = Vec::new();
    for rv in records {
        let rec = match rv {
            Value::Record(r) => r,
            other => return Err(type_mismatch("record (in list)", other)),
        };
        for (name, v) in &rec.fields {
            let forced = super::super::force(v.clone(), &mut |e, env| super::super::evaluate(e, env, host))?;
            if let Some(slot) = fields.iter_mut().find(|(n, _)| n == name) {
                slot.1 = forced;
            } else {
                fields.push((name.clone(), forced));
            }
        }
    }
    Ok(Value::Record(Record {
        fields,
        env: super::super::env::EnvNode::empty(),
    }))
}


fn field_or_default(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let name = expect_text(&args[1])?;
    let default = args.get(2).cloned().unwrap_or(Value::Null);
    match record.fields.iter().find(|(n, _)| n == name) {
        Some((_, v)) => super::super::force(v.clone(), &mut |e, env| super::super::evaluate(e, env, host)),
        None => Ok(default),
    }
}


fn remove_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let drop_names: Vec<String> = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(xs) => xs
            .iter()
            .map(|v| match v {
                Value::Text(s) => Ok(s.clone()),
                other => Err(type_mismatch("text (in list)", other)),
            })
            .collect::<Result<_, _>>()?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Record.RemoveFields: missingField option not yet supported",
        ));
    }
    // Default behaviour: any name not present in the record is an error.
    for n in &drop_names {
        if !record.fields.iter().any(|(fname, _)| fname == n) {
            return Err(MError::Other(format!(
                "Record.RemoveFields: field not found: {n}"
            )));
        }
    }
    let kept: Vec<(String, Value)> = record
        .fields
        .iter()
        .filter(|(n, _)| !drop_names.contains(n))
        .cloned()
        .collect();
    Ok(Value::Record(Record {
        fields: kept,
        env: record.env.clone(),
    }))
}

fn add_field(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let name = expect_text(&args[1])?;
    if record.fields.iter().any(|(n, _)| n == name) {
        return Err(MError::Other(format!(
            "Record.AddField: field already exists: {name}"
        )));
    }
    // v1: ignore the optional `delayed` flag — we evaluate args eagerly.
    let mut fields = record.fields.clone();
    fields.push((name.to_string(), args[2].clone()));
    Ok(Value::Record(Record {
        fields,
        env: record.env.clone(),
    }))
}


fn field_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    Ok(Value::Number(record.fields.len() as f64))
}


fn from_list(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let values = expect_list(&args[0])?;
    let names = expect_text_list(&args[1], "Record.FromList")?;
    if values.len() != names.len() {
        return Err(MError::Other(format!(
            "Record.FromList: values ({}) and fields ({}) must have same length",
            values.len(),
            names.len()
        )));
    }
    let fields: Vec<(String, Value)> = names
        .into_iter()
        .zip(values.iter().cloned())
        .collect();
    Ok(Value::Record(Record {
        fields,
        env: super::super::env::EnvNode::empty(),
    }))
}


fn from_table(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    let name_idx = names.iter().position(|n| n == "Name").ok_or_else(|| {
        MError::Other("Record.FromTable: table must have a 'Name' column".into())
    })?;
    let value_idx = names.iter().position(|n| n == "Value").ok_or_else(|| {
        MError::Other("Record.FromTable: table must have a 'Value' column".into())
    })?;
    let mut fields: Vec<(String, Value)> = Vec::with_capacity(table.num_rows());
    for row in 0..table.num_rows() {
        let n_cell = super::cell_to_value(table, name_idx, row)?;
        let name = match n_cell {
            Value::Text(s) => s,
            other => return Err(type_mismatch("text (in Name column)", &other)),
        };
        let v_cell = super::cell_to_value(table, value_idx, row)?;
        let forced = super::super::force(v_cell, &mut |e, env| {
            super::super::evaluate(e, env, host)
        })?;
        fields.push((name, forced));
    }
    Ok(Value::Record(Record {
        fields,
        env: super::super::env::EnvNode::empty(),
    }))
}


fn rename_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Record.RenameFields: missingField option not yet supported",
        ));
    }
    // renames may be a single {old, new} pair or a list of such pairs.
    let raw = expect_list(&args[1])?;
    let pairs: Vec<(String, String)> = if raw.iter().all(|v| matches!(v, Value::List(_))) {
        raw.iter()
            .map(parse_rename_pair)
            .collect::<Result<_, _>>()?
    } else {
        vec![parse_rename_pair(&Value::List(raw.clone()))?]
    };
    let mut fields = record.fields.clone();
    for (old, new) in pairs {
        let pos = fields.iter().position(|(n, _)| n == &old).ok_or_else(|| {
            MError::Other(format!("Record.RenameFields: field not found: {old}"))
        })?;
        if old != new && fields.iter().any(|(n, _)| n == &new) {
            return Err(MError::Other(format!(
                "Record.RenameFields: target field already exists: {new}"
            )));
        }
        fields[pos].0 = new;
    }
    Ok(Value::Record(Record {
        fields,
        env: record.env.clone(),
    }))
}


fn parse_rename_pair(v: &Value) -> Result<(String, String), MError> {
    let xs = expect_list(v)?;
    if xs.len() != 2 {
        return Err(MError::Other(format!(
            "Record.RenameFields: pair must have 2 elements, got {}",
            xs.len()
        )));
    }
    let old = expect_text(&xs[0])?.to_string();
    let new = expect_text(&xs[1])?.to_string();
    Ok((old, new))
}


fn reorder_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Record.ReorderFields: missingField option not yet supported",
        ));
    }
    let order = expect_text_list(&args[1], "Record.ReorderFields")?;
    // Power Query allows partial reorder — listed fields move to the front
    // in the given order, remaining fields keep their original order behind.
    for n in &order {
        if !record.fields.iter().any(|(fname, _)| fname == n) {
            return Err(MError::Other(format!(
                "Record.ReorderFields: field not found: {n}"
            )));
        }
    }
    let mut fields: Vec<(String, Value)> = Vec::with_capacity(record.fields.len());
    for n in &order {
        let (fname, fv) = record
            .fields
            .iter()
            .find(|(fname, _)| fname == n)
            .unwrap();
        fields.push((fname.clone(), fv.clone()));
    }
    for (fname, fv) in &record.fields {
        if !order.iter().any(|n| n == fname) {
            fields.push((fname.clone(), fv.clone()));
        }
    }
    Ok(Value::Record(Record {
        fields,
        env: record.env.clone(),
    }))
}


fn select_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Record.SelectFields: missingField option not yet supported",
        ));
    }
    let keep: Vec<String> = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(_) => expect_text_list(&args[1], "Record.SelectFields")?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let mut fields: Vec<(String, Value)> = Vec::with_capacity(keep.len());
    for n in &keep {
        let (fname, fv) = record
            .fields
            .iter()
            .find(|(fname, _)| fname == n)
            .ok_or_else(|| {
                MError::Other(format!("Record.SelectFields: field not found: {n}"))
            })?;
        fields.push((fname.clone(), fv.clone()));
    }
    Ok(Value::Record(Record {
        fields,
        env: record.env.clone(),
    }))
}


fn to_list(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let values: Result<Vec<Value>, MError> = record
        .fields
        .iter()
        .map(|(_, v)| {
            super::super::force(v.clone(), &mut |e, env| {
                super::super::evaluate(e, env, host)
            })
        })
        .collect();
    Ok(Value::List(values?))
}


fn to_table(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let names = vec!["Name".to_string(), "Value".to_string()];
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(record.fields.len());
    for (n, v) in &record.fields {
        let forced = super::super::force(v.clone(), &mut |e, env| {
            super::super::evaluate(e, env, host)
        })?;
        rows.push(vec![Value::Text(n.clone()), forced]);
    }
    Ok(Value::Table(super::table::values_to_table(&names, &rows)?))
}


fn transform_fields(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Record.TransformFields: missingField option not yet supported",
        ));
    }
    let raw = expect_list(&args[1])?;
    // Accept either a single {name, fn} pair or a list of such pairs.
    let pair_values: Vec<&Vec<Value>> = if raw.iter().all(|v| matches!(v, Value::List(_))) {
        raw.iter()
            .map(|v| match v {
                Value::List(xs) => Ok(xs),
                other => Err(type_mismatch("list (pair)", other)),
            })
            .collect::<Result<_, _>>()?
    } else {
        vec![raw]
    };
    let mut fields = record.fields.clone();
    for xs in pair_values {
        if xs.len() != 2 {
            return Err(MError::Other(format!(
                "Record.TransformFields: pair must have 2 elements, got {}",
                xs.len()
            )));
        }
        let name = expect_text(&xs[0])?.to_string();
        let closure = expect_function(&xs[1])?;
        let pos = fields.iter().position(|(n, _)| n == &name).ok_or_else(|| {
            MError::Other(format!("Record.TransformFields: field not found: {name}"))
        })?;
        let forced = super::super::force(fields[pos].1.clone(), &mut |e, env| {
            super::super::evaluate(e, env, host)
        })?;
        let new_val = invoke_callback_with_host(closure, vec![forced], host)?;
        fields[pos].1 = new_val;
    }
    Ok(Value::Record(Record {
        fields,
        env: record.env.clone(),
    }))
}
