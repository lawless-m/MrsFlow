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
        Ok(Value::Table(Table::from_arrow(combined)))
    }

    fn parquet_write(&self, path: &str, value: &Value) -> Result<(), IoError> {
        let batch_owned = match value {
            Value::Table(t) => t
                .try_to_arrow()
                .map_err(|e| IoError::Other(format!("parquet_write: {:?}", e)))?,
            _ => {
                return Err(IoError::Other(
                    "parquet_write: value must be a table".into(),
                ));
            }
        };
        let batch = &batch_owned;
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

    #[cfg(not(feature = "odbc"))]
    fn odbc_query(
        &self,
        _connection_string: &str,
        _sql: &str,
        _options: Option<&Value>,
    ) -> Result<Value, IoError> {
        Err(IoError::Other(
            "Odbc.Query: built without ODBC support — recompile mrsflow-cli with --features odbc".into(),
        ))
    }

    #[cfg(feature = "odbc")]
    fn odbc_query(
        &self,
        connection_string: &str,
        sql: &str,
        _options: Option<&Value>,
    ) -> Result<Value, IoError> {
        odbc_query_impl(connection_string, sql)
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

// --- ODBC implementation (feature-gated) ---
//
// odbc-api links to libodbc at build time. Behind a feature flag so the
// workspace builds without it. The driver manager (unixODBC on Linux,
// MS Driver Manager on Windows) must be installed for compilation when
// the feature is on; an actual ODBC driver must be installed at runtime
// for any given DSN to resolve.

#[cfg(feature = "odbc")]
fn odbc_query_impl(connection_string: &str, sql: &str) -> Result<Value, IoError> {
    use std::sync::OnceLock;

    use arrow::array::{Float64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use odbc_api::{
        buffers::{BufferDesc, ColumnarAnyBuffer},
        ConnectionOptions, Cursor, Environment,
    };

    // Lazy-init env: an Environment is the entry point for all ODBC ops.
    // Sharing one per process is the common pattern; odbc-api supports
    // multi-threading by way of Environment::shared() etc., but for the
    // CLI shell a single-threaded OnceLock is enough.
    static ENV: OnceLock<Environment> = OnceLock::new();
    let env = ENV.get_or_init(|| {
        Environment::new()
            .expect("ODBC environment allocation failed — check libodbc install")
    });

    let conn = env
        .connect_with_connection_string(connection_string, ConnectionOptions::default())
        .map_err(|e| IoError::Other(format!("Odbc.Query connect: {}", e)))?;

    let cursor = conn
        .execute(sql, ())
        .map_err(|e| IoError::Other(format!("Odbc.Query execute: {}", e)))?;

    let Some(mut cursor) = cursor else {
        // Statement didn't produce a result set (DDL, etc.). Return an
        // empty-table value rather than an error, so callers can chain.
        let schema = std::sync::Arc::new(Schema::empty());
        let options =
            arrow::record_batch::RecordBatchOptions::new().with_row_count(Some(0));
        return Ok(Value::Table(Table {
            batch: arrow::record_batch::RecordBatch::try_new_with_options(
                schema, vec![], &options,
            )
            .map_err(|e| IoError::Other(format!("Odbc.Query empty: {}", e)))?,
        }));
    };

    // Build the Arrow schema from the cursor's column descriptions. SQL
    // type → Arrow DataType mapping (first pass, expand as corpus needs):
    let n_cols = cursor
        .num_result_cols()
        .map_err(|e| IoError::Other(format!("Odbc.Query cols: {}", e)))?;
    let mut fields: Vec<Field> = Vec::with_capacity(n_cols as usize);
    let mut buf_descs: Vec<BufferDesc> = Vec::with_capacity(n_cols as usize);
    for col_idx in 1..=n_cols {
        let desc = cursor
            .describe_col(col_idx as u16)
            .map_err(|e| IoError::Other(format!("Odbc.Query describe col {}: {}", col_idx, e)))?;
        let name = String::from_utf16_lossy(&desc.name);
        let (arrow_dtype, buf_desc) = match desc.data_type {
            odbc_api::DataType::Integer | odbc_api::DataType::SmallInt
            | odbc_api::DataType::TinyInt | odbc_api::DataType::BigInt
            | odbc_api::DataType::Real | odbc_api::DataType::Float { .. }
            | odbc_api::DataType::Double
            | odbc_api::DataType::Decimal { .. } | odbc_api::DataType::Numeric { .. } => {
                (DataType::Float64, BufferDesc::F64 { nullable: true })
            }
            odbc_api::DataType::Char { length }
            | odbc_api::DataType::WChar { length }
            | odbc_api::DataType::Varchar { length }
            | odbc_api::DataType::WVarchar { length }
            | odbc_api::DataType::LongVarchar { length } => {
                let max = length.map(|n| n.get()).unwrap_or(255);
                (DataType::Utf8, BufferDesc::Text { max_str_len: max })
            }
            other => {
                return Err(IoError::Other(format!(
                    "Odbc.Query: column {} ({}) has unsupported SQL type {:?}",
                    col_idx, name, other
                )));
            }
        };
        fields.push(Field::new(name, arrow_dtype, true));
        buf_descs.push(buf_desc);
    }

    // Fetch rows in batches. odbc-api requires a row-set buffer sized at
    // construction; pick 1024 as a reasonable default.
    const BATCH_SIZE: usize = 1024;
    let buffer = ColumnarAnyBuffer::from_descs(BATCH_SIZE, buf_descs);
    let mut row_set_cursor = cursor
        .bind_buffer(buffer)
        .map_err(|e| IoError::Other(format!("Odbc.Query bind: {}", e)))?;

    let mut accumulated_columns: Vec<Vec<Option<f64>>> = vec![Vec::new(); n_cols as usize];
    let mut accumulated_strings: Vec<Vec<Option<String>>> = vec![Vec::new(); n_cols as usize];

    while let Some(batch) = row_set_cursor
        .fetch()
        .map_err(|e| IoError::Other(format!("Odbc.Query fetch: {}", e)))?
    {
        let n_rows = batch.num_rows();
        for col_idx in 0..n_cols as usize {
            match fields[col_idx].data_type() {
                DataType::Float64 => {
                    let view = batch
                        .column(col_idx)
                        .as_nullable_slice::<f64>()
                        .expect("F64 buffer");
                    let (values, indicators) = view;
                    for row in 0..n_rows {
                        let v = if indicators[row] < 0 {
                            None
                        } else {
                            Some(values[row])
                        };
                        accumulated_columns[col_idx].push(v);
                    }
                }
                DataType::Utf8 => {
                    let view = batch
                        .column(col_idx)
                        .as_text_view()
                        .expect("Text buffer");
                    for row in 0..n_rows {
                        let s = view
                            .get(row)
                            .map(|b| String::from_utf8_lossy(b).into_owned());
                        accumulated_strings[col_idx].push(s);
                    }
                }
                _ => unreachable!("schema only contains Float64/Utf8 in this slice"),
            }
        }
    }

    // Build Arrow columns from accumulated buffers.
    let columns: Vec<arrow::array::ArrayRef> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| match f.data_type() {
            DataType::Float64 => std::sync::Arc::new(Float64Array::from(
                std::mem::take(&mut accumulated_columns[i]),
            )) as arrow::array::ArrayRef,
            DataType::Utf8 => std::sync::Arc::new(StringArray::from(
                std::mem::take(&mut accumulated_strings[i]),
            )) as arrow::array::ArrayRef,
            _ => unreachable!(),
        })
        .collect();

    let schema = std::sync::Arc::new(Schema::new(fields));
    let batch = arrow::record_batch::RecordBatch::try_new(schema, columns)
        .map_err(|e| IoError::Other(format!("Odbc.Query batch: {}", e)))?;
    Ok(Value::Table(Table { batch }))
}
