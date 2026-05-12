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
    let opts = args.get(1);
    host.odbc_data_source(conn, opts)
        .map_err(|e| MError::Other(format!("Odbc.DataSource: {e:?}")))
}
