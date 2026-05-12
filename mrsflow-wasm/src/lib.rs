//! Browser shell for mrsflow. Compiles to `wasm32-unknown-unknown`.
//!
//! Wraps `mrsflow-core`'s evaluator with an in-memory `IoHost`. M source
//! is evaluated against a JS-supplied map of name → Uint8Array; the only
//! IO call routed through is `Parquet.Document(name)`, which looks up
//! bytes in the map and parses them via the `parquet` crate. All other
//! IoHost methods return `NotSupported`.

use std::collections::HashMap;

use arrow::record_batch::RecordBatch;
use bytes::Bytes;
use js_sys::{Array, Object, Uint8Array};
use mrsflow_core::eval::{
    deep_force, evaluate, root_env, value_to_sexpr, IoError, IoHost, Table, Value,
};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// IoHost backed by an in-memory map of name → parquet bytes. The keys
/// are the same names M source references via `Parquet.Document(name)`.
struct WasmIoHost {
    inputs: HashMap<String, Bytes>,
}

impl IoHost for WasmIoHost {
    fn parquet_read(&self, path: &str) -> Result<Value, IoError> {
        let buf = self
            .inputs
            .get(path)
            .ok_or_else(|| IoError::Other(format!("no input named {path:?}")))?
            .clone();
        let builder = ParquetRecordBatchReaderBuilder::try_new(buf)
            .map_err(|e| IoError::Other(format!("parquet read {path}: {e}")))?;
        let reader = builder
            .build()
            .map_err(|e| IoError::Other(format!("parquet read {path}: {e}")))?;
        let mut batches: Vec<RecordBatch> = Vec::new();
        for batch in reader {
            let b = batch.map_err(|e| IoError::Other(format!("parquet read {path}: {e}")))?;
            batches.push(b);
        }
        let combined = match batches.len() {
            0 => RecordBatch::new_empty(std::sync::Arc::new(
                arrow::datatypes::Schema::empty(),
            )),
            1 => batches.into_iter().next().expect("len == 1"),
            _ => arrow::compute::concat_batches(&batches[0].schema(), &batches)
                .map_err(|e| IoError::Other(format!("parquet read {path}: {e}")))?,
        };
        Ok(Value::Table(Table::from_arrow(combined)))
    }

    fn parquet_write(&self, _: &str, _: &Value) -> Result<(), IoError> {
        Err(IoError::NotSupported)
    }
    fn odbc_query(&self, _: &str, _: &str, _: Option<&Value>) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
    }
    fn odbc_data_source(&self, _: &str, _: Option<&Value>) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
    }
    fn file_read(&self, _: &str) -> Result<Vec<u8>, IoError> {
        Err(IoError::NotSupported)
    }
    fn file_modified(
        &self,
        _: &str,
    ) -> Result<chrono::DateTime<chrono::FixedOffset>, IoError> {
        Err(IoError::NotSupported)
    }
    fn excel_workbook(&self, _: &[u8], _: bool, _: bool) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
    }
    fn web_contents(
        &self,
        _: &str,
        _: &[(String, String)],
        _: &[u16],
        _: Option<&[u8]>,
    ) -> Result<Vec<u8>, IoError> {
        Err(IoError::NotSupported)
    }
    fn folder_contents(&self, _: &str) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
    }
    fn folder_files(&self, _: &str) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
    }
    fn current_workbook(&self) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
    }
}

fn parse_inputs(js: &JsValue) -> Result<HashMap<String, Bytes>, JsValue> {
    let obj = js
        .dyn_ref::<Object>()
        .ok_or_else(|| JsValue::from_str("inputs must be an object"))?;
    let entries = Object::entries(obj);
    let mut map: HashMap<String, Bytes> = HashMap::with_capacity(entries.length() as usize);
    for entry in entries.iter() {
        let pair: Array = entry
            .dyn_into()
            .map_err(|_| JsValue::from_str("malformed entry in inputs object"))?;
        let key = pair
            .get(0)
            .as_string()
            .ok_or_else(|| JsValue::from_str("input key is not a string"))?;
        let u8: Uint8Array = pair.get(1).dyn_into().map_err(|_| {
            JsValue::from_str(&format!("input {key:?}: value is not a Uint8Array"))
        })?;
        map.insert(key, Bytes::from(u8.to_vec()));
    }
    Ok(map)
}

/// Evaluate `source` against the supplied parquet bytes.
///
/// `inputs` is a JS object of `{ name: Uint8Array }` — each name becomes
/// available to M source via `Parquet.Document(name)`. Returns the result
/// as a canonical S-expression string (the same format the CLI's
/// differential harness emits). For a Table this can be large; callers
/// who want a preview should narrow the M source itself (e.g. wrap in
/// `Table.FirstN(_, 100)`).
#[wasm_bindgen]
pub fn run(source: &str, inputs: JsValue) -> Result<String, JsValue> {
    let host = WasmIoHost {
        inputs: parse_inputs(&inputs)?,
    };
    let toks =
        tokenize(source).map_err(|e| JsValue::from_str(&format!("lex error: {e:?}")))?;
    let expr =
        parse(&toks).map_err(|e| JsValue::from_str(&format!("parse error: {e:?}")))?;
    let env = root_env();
    let raw = evaluate(&expr, &env, &host)
        .map_err(|e| JsValue::from_str(&format!("eval error: {e:?}")))?;
    let value = deep_force(raw, &host)
        .map_err(|e| JsValue::from_str(&format!("eval error: {e:?}")))?;
    Ok(value_to_sexpr(&value))
}
