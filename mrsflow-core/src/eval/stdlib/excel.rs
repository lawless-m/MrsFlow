//! `Excel.Workbook` — parses XLSX/XLS bytes into a Table of sheets,
//! ListObject tables, and defined names.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::type_mismatch;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![(
        "Excel.Workbook",
        vec![
            Param { name: "workbook".into(),   optional: false, type_annotation: None },
            Param { name: "useHeaders".into(), optional: true,  type_annotation: None },
            Param { name: "delayTypes".into(), optional: true,  type_annotation: None },
        ],
        workbook,
    )]
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
