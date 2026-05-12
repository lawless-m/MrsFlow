//! `Excel.Workbook` — parses XLSX bytes into a Table of sheets.
//!
//! Only the corpus calling convention is supported:
//! `Excel.Workbook(binary, null, true)` — `useHeaders=null` (no header
//! promotion; downstream `Table.PromoteHeaders` handles that) and
//! `delayTypes=true` (no per-column type inference; cells keep XLSX's
//! native types). Calling with `useHeaders=true` or `delayTypes=false`
//! errors — when a query needs them, expand here.

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

    // useHeaders: only null/missing accepted for now.
    match args.get(1) {
        None | Some(Value::Null) => {}
        Some(Value::Logical(false)) => {} // null and false both mean "don't promote"
        Some(Value::Logical(true)) => {
            return Err(MError::NotImplemented(
                "Excel.Workbook: useHeaders=true is not supported \
                 (use Table.PromoteHeaders on the sheet's Data column)",
            ));
        }
        Some(other) => return Err(type_mismatch("logical or null", other)),
    }

    // delayTypes: only true accepted for now. The corpus always passes true.
    match args.get(2) {
        Some(Value::Logical(true)) => {}
        None | Some(Value::Null) | Some(Value::Logical(false)) => {
            return Err(MError::NotImplemented(
                "Excel.Workbook: delayTypes=false is not supported \
                 (pass true; cells keep their XLSX-native types)",
            ));
        }
        Some(other) => return Err(type_mismatch("logical", other)),
    }

    host.excel_workbook(bytes)
        .map_err(|e| MError::Other(format!("Excel.Workbook: {e:?}")))
}
