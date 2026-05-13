//! `Variable.*` — query-parameter lookup.
//!
//! Backed by the same `IoHost::current_workbook()` plumbing as
//! `Excel.CurrentWorkbook()`. Each `--param NAME=VALUE` flag on the
//! CLI surfaces as a row in the workbook's parameter table; these two
//! helpers do the row-lookup so users don't have to chain
//! `Excel.CurrentWorkbook(){[Name="..."]}[Content]{0}[Value]` by hand.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{expect_text, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Variable.Value", vec![
            Param { name: "identifier".into(), optional: false, type_annotation: None },
        ], value),
        ("Variable.ValueOrDefault", vec![
            Param { name: "identifier".into(),   optional: false, type_annotation: None },
            Param { name: "defaultValue".into(), optional: true,  type_annotation: None },
        ], value_or_default),
    ]
}

fn value(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let name = expect_text(&args[0])?.to_string();
    match lookup(&name, host)? {
        Some(v) => Ok(v),
        None => Err(MError::Other(format!(
            "Variable.Value: variable '{name}' is not defined"
        ))),
    }
}

fn value_or_default(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let name = expect_text(&args[0])?.to_string();
    let default = args.get(1).cloned().unwrap_or(Value::Null);
    match lookup(&name, host)? {
        Some(v) => Ok(v),
        None => Ok(default),
    }
}

/// Look up the parameter in the current workbook's parameter table.
/// Returns `Some(inner_value)` if found, `None` if no row has Name=name.
/// Workbook shape per `IoHost::current_workbook`: columns `Name, Content`,
/// where each `Content` is a 1-row table with a single `Value` column.
fn lookup(name: &str, host: &dyn IoHost) -> Result<Option<Value>, MError> {
    let wb = host
        .current_workbook()
        .map_err(|e| MError::Other(format!("Variable: {e:?}")))?;
    let table = match wb {
        Value::Table(t) => t,
        other => {
            return Err(MError::Other(format!(
                "Variable: workbook expected to be a table, got {}",
                super::super::type_name(&other)
            )));
        }
    };
    let forced = table.force()?;
    let names = forced.column_names();
    let name_col = names.iter().position(|c| c == "Name").ok_or_else(|| {
        MError::Other("Variable: workbook missing 'Name' column".into())
    })?;
    let content_col = names.iter().position(|c| c == "Content").ok_or_else(|| {
        MError::Other("Variable: workbook missing 'Content' column".into())
    })?;
    for row in 0..forced.num_rows() {
        let cell_name = super::table::cell_to_value(&forced, name_col, row)?;
        if let Value::Text(s) = cell_name {
            if s == name {
                let content = super::table::cell_to_value(&forced, content_col, row)?;
                return extract_value_cell(&content).map(Some);
            }
        }
    }
    Ok(None)
}

/// Unwrap a `[Value=…]` 1-row table to its inner value. Tolerant of
/// the shape so callers with slightly different workbook conventions
/// (just a value, a record `[Value=…]`) still work.
fn extract_value_cell(content: &Value) -> Result<Value, MError> {
    match content {
        Value::Table(t) => {
            let forced = t.force()?;
            let names = forced.column_names();
            let v_col = names
                .iter()
                .position(|c| c == "Value")
                .unwrap_or(0);
            if forced.num_rows() == 0 {
                return Err(MError::Other(
                    "Variable: Content table is empty".into(),
                ));
            }
            super::table::cell_to_value(&forced, v_col, 0)
        }
        Value::Record(r) => {
            for (n, v) in &r.fields {
                if n == "Value" {
                    return Ok(v.clone());
                }
            }
            Err(MError::Other(
                "Variable: Content record missing 'Value' field".into(),
            ))
        }
        other => Ok(other.clone()),
    }
}
