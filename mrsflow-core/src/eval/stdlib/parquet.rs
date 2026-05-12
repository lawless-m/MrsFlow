//! `Parquet.*` stdlib bindings.

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
        ("Parquet.Document", one("path"), document),
    ]
}

fn document(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let path = expect_text(&args[0])?;
    host.parquet_read(path).map_err(|e| {
        MError::Other(format!("Parquet.Document({:?}): {:?}", path, e))
    })
}

// --- Table.* expansion (eval-7d) ---
//
// Five more Table.* ops by corpus frequency. SelectRows and AddColumn
// invoke an M closure with a row-as-record value, matching the
// `each [ColumnName]` access pattern users write.

