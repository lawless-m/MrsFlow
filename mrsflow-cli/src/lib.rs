//! CLI shell for mrsflow. Implements the `IoHost` trait against the real
//! filesystem and the `parquet` crate. ODBC plumbing lands in eval-8.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use arrow::record_batch::RecordBatch;
use mrsflow_core::eval::{deep_force, root_env, EnvOps, IoError, IoHost, Table, Value};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::{parse, Expr};
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;

/// Errors that can happen while running the multi-query CLI mode. The CLI
/// shell maps these to exit codes; the library exposes them so callers
/// (tests, future shells) can inspect them.
#[derive(Debug)]
pub enum MultiQueryError {
    Io(String),
    Lex(String),
    Parse(String),
    Eval(String),
    /// Two input paths share the same filename stem — they would collide
    /// as binding names in the shared env.
    DuplicateStem {
        name: String,
        first: PathBuf,
        second: PathBuf,
    },
    /// `--out NAME` references a stem not present in the input list.
    UnknownOutName(String),
    /// `--out NAME` query evaluated to a non-Table value.
    NotATable { name: String, kind: &'static str },
    Write(String),
}

impl std::fmt::Display for MultiQueryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            MultiQueryError::Io(s) => write!(f, "IO ERROR: {s}"),
            MultiQueryError::Lex(s) => write!(f, "LEX ERROR: {s}"),
            MultiQueryError::Parse(s) => write!(f, "PARSE ERROR: {s}"),
            MultiQueryError::Eval(s) => write!(f, "EVAL ERROR: {s}"),
            MultiQueryError::DuplicateStem { name, first, second } => write!(
                f,
                "duplicate binding name '{name}' from two input paths: {} and {}",
                first.display(),
                second.display()
            ),
            MultiQueryError::UnknownOutName(name) => write!(
                f,
                "--out names '{name}' which is not in the input set",
            ),
            MultiQueryError::NotATable { name, kind } => write!(
                f,
                "--out {name}: expected Table, got {kind}",
            ),
            MultiQueryError::Write(s) => write!(f, "WRITE ERROR: {s}"),
        }
    }
}

/// Multi-query CLI mode. Each input file's stem becomes a binding in a
/// shared env so the queries can reference each other (`query2.m` can say
/// `Table.SelectRows(query1, …)`). M's laziness means non-`--out` inputs
/// only evaluate if a `--out` query transitively references them.
///
/// For each name in `outs`, look up the binding, force it to a Value, and
/// write `<out_dir>/<name>.parquet`. Returns the paths actually written.
pub fn run_multi_query(
    inputs: &[PathBuf],
    outs: &[String],
    out_dir: &Path,
    host: &dyn IoHost,
) -> Result<Vec<PathBuf>, MultiQueryError> {
    let mut bindings: Vec<(String, Expr)> = Vec::with_capacity(inputs.len());
    let mut seen: HashMap<String, PathBuf> = HashMap::with_capacity(inputs.len());
    for path in inputs {
        let stem = path
            .file_stem()
            .and_then(|s| s.to_str())
            .ok_or_else(|| MultiQueryError::Io(format!("no valid stem for {}", path.display())))?
            .to_string();
        if let Some(prev) = seen.get(&stem) {
            return Err(MultiQueryError::DuplicateStem {
                name: stem,
                first: prev.clone(),
                second: path.clone(),
            });
        }
        let src = std::fs::read_to_string(path)
            .map_err(|e| MultiQueryError::Io(format!("read {}: {e}", path.display())))?;
        let toks = tokenize(&src)
            .map_err(|e| MultiQueryError::Lex(format!("{}: {e:?}", path.display())))?;
        let expr = parse(&toks)
            .map_err(|e| MultiQueryError::Parse(format!("{}: {e:?}", path.display())))?;
        seen.insert(stem.clone(), path.clone());
        bindings.push((stem, expr));
    }

    for name in outs {
        if !seen.contains_key(name) {
            return Err(MultiQueryError::UnknownOutName(name.clone()));
        }
    }

    let env = root_env().extend_lazy(bindings);

    std::fs::create_dir_all(out_dir)
        .map_err(|e| MultiQueryError::Io(format!("mkdir {}: {e}", out_dir.display())))?;

    let mut written: Vec<PathBuf> = Vec::with_capacity(outs.len());
    for name in outs {
        let raw = env
            .lookup(name)
            .expect("presence validated against `seen` above");
        let value = deep_force(raw, host)
            .map_err(|e| MultiQueryError::Eval(format!("{name}: {e:?}")))?;
        match &value {
            Value::Table(_) => {
                let path = out_dir.join(format!("{name}.parquet"));
                let path_str = path.to_str().ok_or_else(|| {
                    MultiQueryError::Io(format!("non-utf8 path: {}", path.display()))
                })?;
                host.parquet_write(path_str, &value).map_err(|e| {
                    MultiQueryError::Write(format!("{}: {e:?}", path.display()))
                })?;
                written.push(path);
            }
            other => {
                return Err(MultiQueryError::NotATable {
                    name: name.clone(),
                    kind: value_kind(other),
                });
            }
        }
    }
    Ok(written)
}

/// Single-character classification for diagnostic messages. Mirrors the
/// `kind()` in main.rs and `type_name()` in core.
fn value_kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Logical(_) => "logical",
        Value::Number(_) => "number",
        Value::Text(_) => "text",
        Value::Date(_) => "date",
        Value::Datetime(_) => "datetime",
        Value::Datetimezone(_) => "datetimezone",
        Value::Time(_) => "time",
        Value::Duration(_) => "duration",
        Value::Binary(_) => "binary",
        Value::List(_) => "list",
        Value::Record(_) => "record",
        Value::Table(_) => "table",
        Value::Function(_) => "function",
        Value::Type(_) => "type",
        Value::Thunk(_) => "thunk",
        Value::WithMetadata { inner, .. } => value_kind(inner),
    }
}

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
            .map_err(|e| IoError::Other(format!("open {path}: {e}")))?;
        let builder = ParquetRecordBatchReaderBuilder::try_new(file)
            .map_err(|e| IoError::Other(format!("parquet read {path}: {e}")))?;
        let mut reader = builder
            .build()
            .map_err(|e| IoError::Other(format!("parquet read {path}: {e}")))?;
        let mut batches: Vec<RecordBatch> = Vec::new();
        for batch in reader.by_ref() {
            let b =
                batch.map_err(|e| IoError::Other(format!("parquet read {path}: {e}")))?;
            batches.push(b);
        }
        let combined = concatenate_batches(&batches)
            .map_err(|e| IoError::Other(format!("parquet read {path}: {e}")))?;
        Ok(Value::Table(Table::from_arrow(combined)))
    }

    fn parquet_write(&self, path: &str, value: &Value) -> Result<(), IoError> {
        let batch_owned = match value {
            Value::Table(t) => t
                .try_to_arrow()
                .map_err(|e| IoError::Other(format!("parquet_write: {e:?}")))?,
            _ => {
                return Err(IoError::Other(
                    "parquet_write: value must be a table".into(),
                ));
            }
        };
        let batch = &batch_owned;
        let parent = Path::new(path).parent();
        if let Some(p) = parent
            && !p.as_os_str().is_empty() {
                std::fs::create_dir_all(p).map_err(|e| {
                    IoError::Other(format!("mkdir {}: {}", p.display(), e))
                })?;
            }
        let file = File::create(path)
            .map_err(|e| IoError::Other(format!("create {path}: {e}")))?;
        let mut writer = ArrowWriter::try_new(file, batch.schema(), None)
            .map_err(|e| IoError::Other(format!("parquet write {path}: {e}")))?;
        writer
            .write(batch)
            .map_err(|e| IoError::Other(format!("parquet write {path}: {e}")))?;
        writer
            .close()
            .map_err(|e| IoError::Other(format!("parquet close {path}: {e}")))?;
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

    fn file_read(&self, path: &str) -> Result<Vec<u8>, IoError> {
        std::fs::read(path).map_err(|e| IoError::Other(format!("read {path}: {e}")))
    }

    fn file_modified(
        &self,
        path: &str,
    ) -> Result<chrono::DateTime<chrono::FixedOffset>, IoError> {
        let modified = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .map_err(|e| IoError::Other(format!("metadata {path}: {e}")))?;
        // SystemTime → DateTime<Utc> → FixedOffset(0). Power Query's
        // File.Modified returns datetimezone; we report UTC since std::fs
        // doesn't expose the filesystem's local-time interpretation.
        let utc: chrono::DateTime<chrono::Utc> = modified.into();
        Ok(utc.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
    }

    fn excel_workbook(&self, bytes: &[u8]) -> Result<Value, IoError> {
        excel_workbook_impl(bytes)
    }
}

fn excel_workbook_impl(bytes: &[u8]) -> Result<Value, IoError> {
    use std::io::Cursor;
    use calamine::{open_workbook_from_rs, Reader, Xlsx};

    // calamine wants an owned `Read + Seek`. Cloning the bytes is cheap
    // compared to parsing the XLSX — fine for v1.
    let cursor = Cursor::new(bytes.to_vec());
    let mut wb: Xlsx<_> = open_workbook_from_rs(cursor)
        .map_err(|e| IoError::Other(format!("open xlsx: {e}")))?;

    let sheet_names: Vec<String> = wb.sheet_names();
    let mut sheet_rows: Vec<Vec<Value>> = Vec::with_capacity(sheet_names.len());
    for name in &sheet_names {
        let range = wb
            .worksheet_range(name)
            .map_err(|e| IoError::Other(format!("read sheet {name:?}: {e}")))?;
        let (_, width) = range.get_size();
        let columns: Vec<String> = (1..=width).map(|i| format!("Column{i}")).collect();
        let data_rows: Vec<Vec<Value>> = range
            .rows()
            .map(|row| row.iter().map(cell_to_value).collect())
            .collect();
        let data_table = Table::from_rows(columns, data_rows);
        sheet_rows.push(vec![
            Value::Text(name.clone()),       // Name
            Value::Table(data_table),         // Data
            Value::Text(name.clone()),       // Item — same as Name for sheets
            Value::Text("Sheet".into()),      // Kind
            Value::Logical(false),            // Hidden — calamine doesn't expose this in 0.35
        ]);
    }

    Ok(Value::Table(Table::from_rows(
        vec![
            "Name".into(),
            "Data".into(),
            "Item".into(),
            "Kind".into(),
            "Hidden".into(),
        ],
        sheet_rows,
    )))
}

fn cell_to_value(cell: &calamine::Data) -> Value {
    use calamine::Data;
    match cell {
        Data::Empty => Value::Null,
        Data::String(s) => Value::Text(s.clone()),
        Data::Float(f) => Value::Number(*f),
        Data::Int(i) => Value::Number(*i as f64),
        Data::Bool(b) => Value::Logical(*b),
        // delayTypes=true: dates stay as their Excel-serial float; downstream
        // M code can decode via Date.From or similar if needed.
        Data::DateTime(d) => Value::Number(d.as_f64()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => Value::Text(s.clone()),
        Data::Error(_) => Value::Null,
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
