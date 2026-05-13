//! `Excel.Workbook` — parses XLSX/XLS bytes into a Table of sheets,
//! ListObject tables, and defined names.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::type_mismatch;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        (
            "Excel.Workbook",
            vec![
                Param { name: "workbook".into(),   optional: false, type_annotation: None },
                Param { name: "useHeaders".into(), optional: true,  type_annotation: None },
                Param { name: "delayTypes".into(), optional: true,  type_annotation: None },
            ],
            workbook,
        ),
        ("Excel.CurrentWorkbook", vec![], current_workbook),
        (
            "Excel.ShapeTable",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            shape_table,
        ),
    ]
}

fn current_workbook(_args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    host.current_workbook()
        .map_err(|e| MError::Other(format!("Excel.CurrentWorkbook: {e:?}")))
}

/// MS docs flag this as "intended for internal use only" — a Power BI
/// connector-synthesis helper that applies a transform spec record to
/// a base table. The options record's shape is undocumented; without
/// a contract we can't translate it to Table.* primitives correctly.
/// When `options` is null or absent, the function is a no-op — return
/// the input table unchanged so trivial pass-through uses don't break.
/// Any non-null options record errors with NotImplemented.
fn shape_table(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match args.get(1) {
        None | Some(Value::Null) => Ok(args[0].clone()),
        Some(_) => Err(MError::NotImplemented(
            "Excel.ShapeTable: options record handling not implemented \
             (MS-documented as internal use only; spec is opaque)",
        )),
    }
}

fn workbook(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let bytes: &[u8] = match &args[0] {
        Value::Binary(b) => b.as_slice(),
        other => return Err(type_mismatch("binary", other)),
    };

    // M's spec: useHeaders defaults to false, delayTypes defaults to false.
    // Both accept null as "use default".
    let use_headers = match args.get(1) {
        None | Some(Value::Null) => false,
        Some(Value::Logical(b)) => *b,
        Some(other) => return Err(type_mismatch("logical or null", other)),
    };
    let delay_types = match args.get(2) {
        None | Some(Value::Null) => false,
        Some(Value::Logical(b)) => *b,
        Some(other) => return Err(type_mismatch("logical or null", other)),
    };

    host.excel_workbook(bytes, use_headers, delay_types)
        .map_err(|e| MError::Other(format!("Excel.Workbook: {e:?}")))
}
