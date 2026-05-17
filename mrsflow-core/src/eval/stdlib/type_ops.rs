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
        ("Type.Is", two("value", "type"), is),
        ("Type.IsNullable", one("type"), is_nullable),
        ("Type.NonNullable", one("type"), non_nullable),
        ("Type.OpenRecord", one("type"), open_record),
        ("Type.ClosedRecord", one("type"), closed_record),
        ("Type.IsOpenRecord", one("type"), is_open_record),
        ("Type.RecordFields", one("type"), record_fields),
        ("Type.ListItem", one("type"), list_item),
        ("Type.TableColumn", two("type", "columnName"), table_column),
        ("Type.TableKeys", one("type"), table_keys),
        (
            "Type.AddTableKey",
            three("type", "columns", "isPrimary"),
            identity_passthrough,
        ),
        (
            "Type.ReplaceTableKeys",
            two("type", "keys"),
            identity_passthrough,
        ),
        ("Type.TableRow", one("type"), table_row),
        ("Type.TableSchema", one("type"), table_schema),
        ("Type.FunctionParameters", one("type"), function_parameters),
        (
            "Type.FunctionRequiredParameters",
            one("type"),
            function_required_parameters,
        ),
        ("Type.FunctionReturn", one("type"), function_return),
        (
            "Type.ForFunction",
            two("signature", "requiredParameters"),
            for_function,
        ),
        ("Type.ForRecord", two("fields", "open"), for_record),
        ("Type.Facets", one("type"), facets),
        ("Type.ReplaceFacets", two("type", "facets"), identity_passthrough),
        ("Type.TablePartitionKey", one("type"), table_partition_key),
        (
            "Type.ReplaceTablePartitionKey",
            two("type", "key"),
            identity_passthrough,
        ),
        ("Type.Union", one("types"), union_),
    ]
}

fn expect_type(v: &Value) -> Result<&TypeRep, MError> {
    match v {
        Value::Type(t) => Ok(t),
        other => Err(type_mismatch("type", other)),
    }
}

fn is(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_type(&args[1])?;
    // PQ idiom: when the value is itself a type-value, Type.Is performs
    // a type-subtype check (e.g. `each Type.Is(_, type number)` where `_`
    // is a column's type). Otherwise it's a value-vs-type conformance test.
    let result = match &args[0] {
        Value::Type(a) => super::super::type_is_subtype(a, t),
        v => super::super::type_conforms(v, t),
    };
    Ok(Value::Logical(result))
}

fn is_nullable(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_type(&args[0])?;
    Ok(Value::Logical(matches!(t, TypeRep::Nullable(_))))
}

fn non_nullable(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_type(&args[0])?;
    let stripped = match t {
        TypeRep::Nullable(inner) => (**inner).clone(),
        other => other.clone(),
    };
    Ok(Value::Type(stripped))
}

fn open_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::RecordOf { fields, .. } => Ok(Value::Type(TypeRep::RecordOf {
            fields: fields.clone(),
            open: true,
        })),
        other => Err(MError::Other(format!(
            "Type.OpenRecord: expected record-type, got {other:?}"
        ))),
    }
}

fn closed_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::RecordOf { fields, .. } => Ok(Value::Type(TypeRep::RecordOf {
            fields: fields.clone(),
            open: false,
        })),
        other => Err(MError::Other(format!(
            "Type.ClosedRecord: expected record-type, got {other:?}"
        ))),
    }
}

fn is_open_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::RecordOf { open, .. } => Ok(Value::Logical(*open)),
        _ => Ok(Value::Logical(false)),
    }
}

fn record_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
            "Type.RecordFields: expected record-type, got {other:?}"
        ))),
    }
}

fn list_item(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::ListOf(item) => Ok(Value::Type((**item).clone())),
        other => Err(MError::Other(format!(
            "Type.ListItem: expected list-type, got {other:?}"
        ))),
    }
}

fn table_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let t = expect_type(&args[0])?;
    let name = expect_text(&args[1])?;
    match t {
        TypeRep::TableOf { columns } => columns
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, ty)| Value::Type(ty.clone()))
            .ok_or_else(|| MError::Other(format!("Type.TableColumn: column not found: {name}"))),
        other => Err(MError::Other(format!(
            "Type.TableColumn: expected table-type, got {other:?}"
        ))),
    }
}

fn table_keys(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_type(&args[0])?;
    Ok(Value::list_of(Vec::new()))
}

fn table_row(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::TableOf { columns } => {
            let fields = columns
                .iter()
                .map(|(n, t)| (n.clone(), t.clone(), false))
                .collect();
            Ok(Value::Type(TypeRep::RecordOf { fields, open: false }))
        }
        other => Err(MError::Other(format!(
            "Type.TableRow: expected table-type, got {other:?}"
        ))),
    }
}

fn table_schema(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
            "Type.TableSchema: expected table-type, got {other:?}"
        ))),
    }
}

fn function_parameters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::FunctionOf { params, .. } => {
            // Preserve declared param names; fall back to ParameterN only when
            // the slot was constructed without a name (e.g. via Type.ForFunction).
            let fields: Vec<(String, Value)> = params
                .iter()
                .enumerate()
                .map(|(i, (n, t, _opt))| {
                    let name = if n.is_empty() { format!("Parameter{}", i + 1) } else { n.clone() };
                    (name, Value::Type(t.clone()))
                })
                .collect();
            Ok(Value::Record(Record {
                fields,
                env: EnvNode::empty(),
            }))
        }
        other => Err(MError::Other(format!(
            "Type.FunctionParameters: expected function-type, got {other:?}"
        ))),
    }
}

fn function_required_parameters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::FunctionOf { params, .. } => {
            let n = params.iter().filter(|(_, _, opt)| !*opt).count();
            Ok(Value::Number(n as f64))
        }
        other => Err(MError::Other(format!(
            "Type.FunctionRequiredParameters: expected function-type, got {other:?}"
        ))),
    }
}

fn function_return(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match expect_type(&args[0])? {
        TypeRep::FunctionOf { return_type, .. } => Ok(Value::Type((**return_type).clone())),
        other => Err(MError::Other(format!(
            "Type.FunctionReturn: expected function-type, got {other:?}"
        ))),
    }
}

fn for_function(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
    let mut named_params: Vec<(String, TypeRep)> = Vec::new();
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
                for (pn, pv) in &inner.fields {
                    let pv = super::super::force(pv.clone(), &mut |e, env| {
                        super::super::evaluate(e, env, &super::super::NoIoHost)
                    })?;
                    match pv {
                        Value::Type(t) => named_params.push((pn.clone(), t)),
                        other => return Err(type_mismatch("type (parameter)", &other)),
                    }
                }
            }
            _ => {}
        }
    }
    let params = named_params
        .into_iter()
        .enumerate()
        .map(|(i, (n, t))| (n, t, i >= req_count))
        .collect();
    Ok(Value::Type(TypeRep::FunctionOf {
        params,
        return_type: Box::new(return_type),
    }))
}

fn for_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

fn facets(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_type(&args[0])?;
    Ok(Value::Record(Record {
        fields: Vec::new(),
        env: EnvNode::empty(),
    }))
}

fn table_partition_key(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_type(&args[0])?;
    Ok(Value::list_of(Vec::new()))
}

fn identity_passthrough(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_type(&args[0])?;
    Ok(args[0].clone())
}

fn union_(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
