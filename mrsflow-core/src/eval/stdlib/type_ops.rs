//! `Type.*` stdlib bindings. Operate on `TypeRep` values constructed via
//! `type X` expressions. v1 implements the corpus-relevant accessors;
//! key/partition/facet machinery is bound but returns stubs.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, TypeRep, Value};
use super::common::{expect_function, expect_list, expect_text, one, two, three, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Type.Is", two("value", "type"), type_is),
        ("Type.IsNullable", one("type"), type_is_nullable),
        ("Type.NonNullable", one("type"), type_non_nullable),
        ("Type.OpenRecord", one("type"), type_open_record),
        ("Type.ClosedRecord", one("type"), type_closed_record),
        ("Type.IsOpenRecord", one("type"), type_is_open_record),
        ("Type.RecordFields", one("type"), type_record_fields),
        ("Type.ListItem", one("type"), type_list_item),
        ("Type.TableColumn", two("type", "columnName"), type_table_column),
        ("Type.TableKeys", one("type"), type_table_keys),
        (
            "Type.AddTableKey",
            three("type", "columns", "isPrimary"),
            type_identity_passthrough,
        ),
        (
            "Type.ReplaceTableKeys",
            two("type", "keys"),
            type_identity_passthrough,
        ),
        ("Type.TableRow", one("type"), type_table_row),
        ("Type.TableSchema", one("type"), type_table_schema),
        ("Type.FunctionParameters", one("type"), type_function_parameters),
        (
            "Type.FunctionRequiredParameters",
            one("type"),
            type_function_required_parameters,
        ),
        ("Type.FunctionReturn", one("type"), type_function_return),
        (
            "Type.ForFunction",
            two("signature", "requiredParameters"),
            type_for_function,
        ),
        ("Type.ForRecord", two("fields", "open"), type_for_record),
        ("Type.Facets", one("type"), type_facets),
        ("Type.ReplaceFacets", two("type", "facets"), type_identity_passthrough),
        ("Type.TablePartitionKey", one("type"), type_table_partition_key),
        (
            "Type.ReplaceTablePartitionKey",
            two("type", "key"),
            type_identity_passthrough,
        ),
        ("Type.Union", one("types"), type_union),
    ]
}

fn expect_type(v: &Value) -> Result<&TypeRep, MError> {
    match v {
        Value::Type(t) => Ok(t),
        other => Err(type_mismatch("type", other)),
    }
}

fn type_is(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_type(&args[1])?;
    Ok(Value::Logical(super::super::type_conforms(&args[0], t)))
}

fn type_is_nullable(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_type(&args[0])?;
    Ok(Value::Logical(matches!(t, TypeRep::Nullable(_))))
}

fn type_non_nullable(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_type(&args[0])?;
    let stripped = match t {
        TypeRep::Nullable(inner) => (**inner).clone(),
        other => other.clone(),
    };
    Ok(Value::Type(stripped))
}

fn type_open_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::RecordOf { fields, .. } => Ok(Value::Type(TypeRep::RecordOf {
            fields: fields.clone(),
            open: true,
        })),
        other => Err(MError::Other(format!(
            "Type.OpenRecord: expected record-type, got {:?}",
            other
        ))),
    }
}

fn type_closed_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::RecordOf { fields, .. } => Ok(Value::Type(TypeRep::RecordOf {
            fields: fields.clone(),
            open: false,
        })),
        other => Err(MError::Other(format!(
            "Type.ClosedRecord: expected record-type, got {:?}",
            other
        ))),
    }
}

fn type_is_open_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::RecordOf { open, .. } => Ok(Value::Logical(*open)),
        _ => Ok(Value::Logical(false)),
    }
}

fn type_record_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::RecordOf { fields, .. } => {
            let out_fields: Vec<(String, Value)> = fields
                .iter()
                .map(|(name, t, optional)| {
                    let info = Record {
                        fields: vec![
                            ("Type".to_string(), Value::Type(t.clone())),
                            ("Optional".to_string(), Value::Logical(*optional)),
                        ],
                        env: EnvNode::empty(),
                    };
                    (name.clone(), Value::Record(info))
                })
                .collect();
            Ok(Value::Record(Record {
                fields: out_fields,
                env: EnvNode::empty(),
            }))
        }
        other => Err(MError::Other(format!(
            "Type.RecordFields: expected record-type, got {:?}",
            other
        ))),
    }
}

fn type_list_item(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::ListOf(item) => Ok(Value::Type((**item).clone())),
        other => Err(MError::Other(format!(
            "Type.ListItem: expected list-type, got {:?}",
            other
        ))),
    }
}

fn type_table_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_type(&args[0])?;
    let name = expect_text(&args[1])?;
    match t {
        TypeRep::TableOf { columns } => columns
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, ty)| Value::Type(ty.clone()))
            .ok_or_else(|| MError::Other(format!("Type.TableColumn: column not found: {}", name))),
        other => Err(MError::Other(format!(
            "Type.TableColumn: expected table-type, got {:?}",
            other
        ))),
    }
}

fn type_table_keys(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_type(&args[0])?;
    Ok(Value::List(Vec::new()))
}

fn type_table_row(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::TableOf { columns } => {
            let fields = columns
                .iter()
                .map(|(n, t)| (n.clone(), t.clone(), false))
                .collect();
            Ok(Value::Type(TypeRep::RecordOf { fields, open: false }))
        }
        other => Err(MError::Other(format!(
            "Type.TableRow: expected table-type, got {:?}",
            other
        ))),
    }
}

fn type_table_schema(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::TableOf { columns } => {
            let names = vec!["Name".to_string(), "Type".to_string()];
            let rows: Vec<Vec<Value>> = columns
                .iter()
                .map(|(n, t)| vec![Value::Text(n.clone()), Value::Type(t.clone())])
                .collect();
            Ok(Value::Table(super::table::values_to_table(&names, &rows)?))
        }
        other => Err(MError::Other(format!(
            "Type.TableSchema: expected table-type, got {:?}",
            other
        ))),
    }
}

fn type_function_parameters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::FunctionOf { params, .. } => {
            // Power Query convention: anonymous param positions named Parameter1, ...
            let fields: Vec<(String, Value)> = params
                .iter()
                .enumerate()
                .map(|(i, (t, _opt))| (format!("Parameter{}", i + 1), Value::Type(t.clone())))
                .collect();
            Ok(Value::Record(Record {
                fields,
                env: EnvNode::empty(),
            }))
        }
        other => Err(MError::Other(format!(
            "Type.FunctionParameters: expected function-type, got {:?}",
            other
        ))),
    }
}

fn type_function_required_parameters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::FunctionOf { params, .. } => {
            let n = params.iter().filter(|(_, opt)| !*opt).count();
            Ok(Value::Number(n as f64))
        }
        other => Err(MError::Other(format!(
            "Type.FunctionRequiredParameters: expected function-type, got {:?}",
            other
        ))),
    }
}

fn type_function_return(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::FunctionOf { return_type, .. } => Ok(Value::Type((**return_type).clone())),
        other => Err(MError::Other(format!(
            "Type.FunctionReturn: expected function-type, got {:?}",
            other
        ))),
    }
}

fn type_for_function(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // signature: a record with fields ReturnType (type) and Parameters (record).
    let sig = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let req_count = match &args[1] {
        Value::Number(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => *n as usize,
        other => return Err(type_mismatch("number", other)),
    };
    let mut return_type = TypeRep::Any;
    let mut param_types: Vec<TypeRep> = Vec::new();
    for (name, raw) in &sig.fields {
        let v = super::super::force(raw.clone(), &mut |e, env| {
            super::super::evaluate(e, env, &super::super::NoIoHost)
        })?;
        match name.as_str() {
            "ReturnType" => {
                return_type = match v {
                    Value::Type(t) => t,
                    other => return Err(type_mismatch("type (ReturnType)", &other)),
                };
            }
            "Parameters" => {
                let inner = match v {
                    Value::Record(r) => r,
                    other => return Err(type_mismatch("record (Parameters)", &other)),
                };
                for (_, pv) in &inner.fields {
                    let pv = super::super::force(pv.clone(), &mut |e, env| {
                        super::super::evaluate(e, env, &super::super::NoIoHost)
                    })?;
                    match pv {
                        Value::Type(t) => param_types.push(t),
                        other => return Err(type_mismatch("type (parameter)", &other)),
                    }
                }
            }
            _ => {}
        }
    }
    let params = param_types
        .into_iter()
        .enumerate()
        .map(|(i, t)| (t, i >= req_count))
        .collect();
    Ok(Value::Type(TypeRep::FunctionOf {
        params,
        return_type: Box::new(return_type),
    }))
}

fn type_for_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let rec = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record (fields spec)", other)),
    };
    let open = match &args[1] {
        Value::Logical(b) => *b,
        other => return Err(type_mismatch("logical (open flag)", other)),
    };
    let mut fields: Vec<(String, TypeRep, bool)> = Vec::with_capacity(rec.fields.len());
    for (name, raw) in &rec.fields {
        let v = super::super::force(raw.clone(), &mut |e, env| {
            super::super::evaluate(e, env, &super::super::NoIoHost)
        })?;
        // Each value can be a Type (required field) or a record {Type=..., Optional=...}.
        match v {
            Value::Type(t) => fields.push((name.clone(), t, false)),
            Value::Record(r) => {
                let mut t: Option<TypeRep> = None;
                let mut opt = false;
                for (fn_, fv) in &r.fields {
                    let fv = super::super::force(fv.clone(), &mut |e, env| {
                        super::super::evaluate(e, env, &super::super::NoIoHost)
                    })?;
                    match (fn_.as_str(), fv) {
                        ("Type", Value::Type(rt)) => t = Some(rt),
                        ("Optional", Value::Logical(b)) => opt = b,
                        _ => {}
                    }
                }
                fields.push((name.clone(), t.unwrap_or(TypeRep::Any), opt));
            }
            other => return Err(type_mismatch("type or info record", &other)),
        }
    }
    Ok(Value::Type(TypeRep::RecordOf { fields, open }))
}

fn type_facets(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_type(&args[0])?;
    Ok(Value::Record(Record {
        fields: Vec::new(),
        env: EnvNode::empty(),
    }))
}

fn type_table_partition_key(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_type(&args[0])?;
    Ok(Value::List(Vec::new()))
}

fn type_identity_passthrough(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_type(&args[0])?;
    Ok(args[0].clone())
}

fn type_union(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    let mut iter = xs.iter();
    let mut common: TypeRep = match iter.next() {
        Some(v) => expect_type(v)?.clone(),
        None => return Ok(Value::Type(TypeRep::Any)),
    };
    for v in iter {
        let t = expect_type(v)?;
        if &common != t {
            common = TypeRep::Any;
        }
    }
    Ok(Value::Type(common))
}
