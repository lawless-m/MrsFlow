//! CLI shell for mrsflow. Implements the `IoHost` trait against the real
//! filesystem and the `parquet` crate. ODBC plumbing lands in eval-8.

use std::fs::File;
use std::path::Path;

use arrow::record_batch::RecordBatch;
use mrsflow_core::eval::{IoError, IoHost, Table, Value};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;

pub struct CliIoHost;

impl CliIoHost {
    pub fn new() -> Self {
        Self
    }
}

impl Default for CliIoHost {
    fn default() -> Self {
        Self::new()
    }
}

impl IoHost for CliIoHost {
    fn parquet_read(&self, path: &str) -> Result<Value, IoError> {
        let file = File::open(path)
            .map_err(|e| IoError::Other(format!("open {}: {}", path, e)))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| IoError::Other(format!("parquet read {}: {}", path, e)))?;
        let mut reader = builder
            .build()
            .map_err(|e| IoError::Other(format!("parquet read {}: {}", path, e)))?;
        let mut batches: Vec<RecordBatch> = Vec::new();
        for batch in reader.by_ref() {
            let b =
                batch.map_err(|e| IoError::Other(format!("parquet read {}: {}", path, e)))?;
            batches.push(b);
        }
        let combined = concatenate_batches(&batches)
            .map_err(|e| IoError::Other(format!("parquet read {}: {}", path, e)))?;
        Ok(Value::Table(Table { batch: combined }))
    }

    fn parquet_write(&self, path: &str, value: &Value) -> Result<(), IoError> {
        let batch = match value {
            Value::Table(t) => &t.batch,
            _ => {
                return Err(IoError::Other(
                    "parquet_write: value must be a table".into(),
                ));
            }
        };
        let parent = Path::new(path).parent();
        if let Some(p) = parent {
            if !p.as_os_str().is_empty() {
                std::fs::create_dir_all(p).map_err(|e| {
                    IoError::Other(format!("mkdir {}: {}", p.display(), e))
                })?;
            }
        }
        let file = File::create(path)
            .map_err(|e| IoError::Other(format!("create {}: {}", path, e)))?;
        let mut writer = ArrowWriter::try_new(file, batch.schema(), None)
            .map_err(|e| IoError::Other(format!("parquet write {}: {}", path, e)))?;
        writer
            .write(batch)
            .map_err(|e| IoError::Other(format!("parquet write {}: {}", path, e)))?;
        writer
            .close()
            .map_err(|e| IoError::Other(format!("parquet close {}: {}", path, e)))?;
        Ok(())
    }

    fn odbc_query(
        &self,
        _connection_string: &str,
        _sql: &str,
        _options: Option<&Value>,
    ) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
    }

    fn odbc_data_source(
        &self,
        _connection_string: &str,
        _options: Option<&Value>,
    ) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
    }
}

/// Concatenate multiple `RecordBatch`es with the same schema into one.
/// Single-batch input is returned cloned. Empty input produces an empty
/// batch — though in practice parquet always emits at least one batch.
fn concatenate_batches(batches: &[RecordBatch]) -> Result<RecordBatch, arrow::error::ArrowError> {
    match batches.len() {
        0 => {
            // No-batch case: build a default empty batch. Parquet readers
            // typically don't produce this, but cover it defensively.
            Ok(RecordBatch::new_empty(std::sync::Arc::new(
                arrow::datatypes::Schema::empty(),
            )))
        }
        1 => Ok(batches[0].clone()),
        _ => arrow::compute::concat_batches(&batches[0].schema(), batches),
    }
}
