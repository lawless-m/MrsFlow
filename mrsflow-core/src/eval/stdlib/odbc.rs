//! `Odbc.*` stdlib bindings.

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
        ("Odbc.Query", two("connection", "sql"), query),
        (
            "Odbc.DataSource",
            vec![
                Param { name: "connection".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),    optional: true,  type_annotation: None },
            ],
            data_source,
        ),
        ("Odbc.InferOptions", one("connection"), infer_options),
    ]
}

fn query(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let conn = expect_text(&args[0])?;
    let sql = expect_text(&args[1])?;
    host.odbc_query(conn, sql, None)
        .map_err(|e| MError::Other(format!("Odbc.Query: {e:?}")))
}

fn data_source(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let conn = expect_text(&args[0])?;
    // Deep-force the options record so the host can read its fields
    // without wrestling with lazy thunks. Record-literal field values
    // (`[HierarchicalNavigation=true]`) are stored as thunks; without
    // forcing, the host's option-parser sees Thunk and falls through to
    // the default.
    let forced_opt = match args.get(1) {
        Some(v) => Some(super::super::deep_force(v.clone(), host)?),
        None => None,
    };
    host.odbc_data_source(conn, forced_opt.as_ref())
        .map_err(|e| MError::Other(format!("Odbc.DataSource: {e:?}")))
}

/// Stub returning an empty SqlCapabilities record. The real
/// Odbc.InferOptions introspects the driver via `SQLGetInfo` /
/// `SQLGetTypeInfo` and produces a record describing aggregates,
/// supported literals, identifier quoting, etc. — used by Power BI's
/// query-folding engine. mrsflow doesn't fold, so the returned record
/// is empty; downstream code that reads `SqlCapabilities.X` will get
/// null (M's missing-field default), which folding logic treats as
/// "feature unsupported" — the conservative answer.
fn infer_options(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_text(&args[0])?;
    Ok(Value::Record(Record {
        fields: vec![
            (
                "SqlCapabilities".to_string(),
                Value::Record(Record { fields: vec![], env: EnvNode::empty() }),
            ),
            (
                "SQLGetInfo".to_string(),
                Value::Record(Record { fields: vec![], env: EnvNode::empty() }),
            ),
            (
                "SQLGetTypeInfo".to_string(),
                Value::Record(Record { fields: vec![], env: EnvNode::empty() }),
            ),
        ],
        env: EnvNode::empty(),
    }))
}
