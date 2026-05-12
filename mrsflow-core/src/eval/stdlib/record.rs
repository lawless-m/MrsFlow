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
        ("Record.Field", two("record", "field"), record_field),
        ("Record.FieldNames", one("record"), record_field_names),
        ("Record.FieldValues", one("record"), record_field_values),
        ("Record.HasFields", two("record", "fields"), record_has_fields),
        ("Record.Combine", one("records"), record_combine),
        (
            "Record.FieldOrDefault",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "field".into(),        optional: false, type_annotation: None },
                Param { name: "defaultValue".into(), optional: true,  type_annotation: None },
            ],
            record_field_or_default,
        ),
        (
            "Record.RemoveFields",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "fields".into(),       optional: false, type_annotation: None },
                Param { name: "missingField".into(), optional: true,  type_annotation: None },
            ],
            record_remove_fields,
        ),
    ]
}

fn record_field(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let name = expect_text(&args[1])?;
    match record.fields.iter().find(|(n, _)| n == name) {
        Some((_, v)) => super::super::force(v.clone(), &mut |e, env| {
            super::super::evaluate(e, env, &super::super::NoIoHost)
        }),
        None => Err(MError::Other(format!("Record.Field: field not found: {}", name))),
    }
}


fn record_field_names(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn record_field_values(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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


fn record_has_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn record_combine(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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


fn record_field_or_default(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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


fn record_remove_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
                "Record.RemoveFields: field not found: {}",
                n
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

// --- Logical.* ---

