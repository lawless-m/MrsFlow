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

    #[cfg(not(feature = "odbc"))]
    fn odbc_data_source(
        &self,
        _connection_string: &str,
        _options: Option<&Value>,
    ) -> Result<Value, IoError> {
        Err(IoError::Other(
            "Odbc.DataSource: built without ODBC support — recompile mrsflow-cli with --features odbc".into(),
        ))
    }

    #[cfg(feature = "odbc")]
    fn odbc_data_source(
        &self,
        connection_string: &str,
        _options: Option<&Value>,
    ) -> Result<Value, IoError> {
        odbc_data_source_impl(connection_string)
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

// Shared ODBC environment — one per process, lazily initialised. Used by
// both the columnar and row-at-a-time query paths plus DataSource catalog
// enumeration.
#[cfg(feature = "odbc")]
fn odbc_env() -> &'static odbc_api::Environment {
    use std::sync::OnceLock;
    static ENV: OnceLock<odbc_api::Environment> = OnceLock::new();
    ENV.get_or_init(|| {
        odbc_api::Environment::new()
            .expect("ODBC environment allocation failed — check libodbc install")
    })
}

// Cache of connection strings whose driver panics through odbc-api's
// columnar text/binary buffers (DBISAM is the corpus case: it returns a
// non-standard negative SQLLEN indicator, odbc-api 13 hard-panics in
// `Indicator::from_isize`). Once we've caught the panic for a given
// connection string we skip the columnar fast path for it.
#[cfg(feature = "odbc")]
fn columnar_blocklist() -> &'static std::sync::Mutex<std::collections::HashSet<String>> {
    use std::sync::OnceLock;
    static BL: OnceLock<std::sync::Mutex<std::collections::HashSet<String>>> = OnceLock::new();
    BL.get_or_init(|| std::sync::Mutex::new(std::collections::HashSet::new()))
}

#[cfg(feature = "odbc")]
fn odbc_query_impl(connection_string: &str, sql: &str) -> Result<Value, IoError> {
    // Try columnar fast path unless this connection has previously panicked.
    // The columnar path bulks rows via bind_buffer / ColumnarAnyBuffer and
    // is roughly 100× faster than row-at-a-time for wide tables.
    let known_broken = columnar_blocklist()
        .lock()
        .map(|set| set.contains(connection_string))
        .unwrap_or(false);

    if !known_broken {
        // Silence the default panic hook for the duration of the columnar
        // attempt — odbc-api's `Indicator::from_isize` panic on a
        // misbehaving driver (DBISAM) is expected and recoverable here.
        // Restore the original hook after the catch_unwind returns so
        // genuine panics elsewhere still get the normal treatment.
        let original_hook = std::panic::take_hook();
        std::panic::set_hook(Box::new(|_| {}));
        let conn_owned = connection_string.to_string();
        let sql_owned = sql.to_string();
        let attempt = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            odbc_query_columnar(&conn_owned, &sql_owned)
        }));
        std::panic::set_hook(original_hook);
        match attempt {
            Ok(result) => return result,
            Err(_panic) => {
                if let Ok(mut set) = columnar_blocklist().lock() {
                    set.insert(connection_string.to_string());
                }
                eprintln!(
                    "Odbc.Query: columnar fetch panicked for `{connection_string}`; \
                     falling back to row-at-a-time (driver indicator quirk)"
                );
            }
        }
    }

    odbc_query_row_at_a_time(connection_string, sql)
}

#[cfg(feature = "odbc")]
fn odbc_query_columnar(connection_string: &str, sql: &str) -> Result<Value, IoError> {
    use arrow::array::{Float64Array, StringArray};
    use arrow::datatypes::{DataType, Schema};
    use odbc_api::{
        buffers::ColumnarAnyBuffer,
        ConnectionOptions, Cursor,
    };

    let env = odbc_env();
    let conn = env
        .connect_with_connection_string(connection_string, ConnectionOptions::default())
        .map_err(|e| IoError::Other(format!("Odbc.Query connect: {}", e)))?;
    let cursor = conn
        .execute(sql, (), None)
        .map_err(|e| IoError::Other(format!("Odbc.Query execute: {}", e)))?;
    let Some(mut cursor) = cursor else {
        return Ok(empty_table());
    };

    let (fields, buf_descs) = describe_columns(&mut cursor)?;
    let n_cols = fields.len();

    const BATCH_SIZE: usize = 1024;
    let buffer = ColumnarAnyBuffer::from_descs(BATCH_SIZE, buf_descs);
    let mut row_set_cursor = cursor
        .bind_buffer(buffer)
        .map_err(|e| IoError::Other(format!("Odbc.Query bind: {}", e)))?;

    let mut accumulated_columns: Vec<Vec<Option<f64>>> = vec![Vec::new(); n_cols];
    let mut accumulated_strings: Vec<Vec<Option<String>>> = vec![Vec::new(); n_cols];

    while let Some(batch) = row_set_cursor
        .fetch()
        .map_err(|e| IoError::Other(format!("Odbc.Query fetch: {}", e)))?
    {
        let n_rows = batch.num_rows();
        for col_idx in 0..n_cols {
            match fields[col_idx].data_type() {
                DataType::Float64 => {
                    let view = batch
                        .column(col_idx)
                        .as_nullable_slice::<f64>()
                        .expect("F64 buffer");
                    for opt in view {
                        accumulated_columns[col_idx].push(opt.copied());
                    }
                    let _ = n_rows;
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

    let columns: Vec<arrow::array::ArrayRef> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| match f.data_type() {
            DataType::Float64 => std::sync::Arc::new(Float64Array::from(std::mem::take(
                &mut accumulated_columns[i],
            ))) as arrow::array::ArrayRef,
            DataType::Utf8 => std::sync::Arc::new(StringArray::from(std::mem::take(
                &mut accumulated_strings[i],
            ))) as arrow::array::ArrayRef,
            _ => unreachable!(),
        })
        .collect();

    let schema = std::sync::Arc::new(Schema::new(fields));
    let batch = arrow::record_batch::RecordBatch::try_new(schema, columns)
        .map_err(|e| IoError::Other(format!("Odbc.Query batch: {}", e)))?;
    Ok(Value::Table(Table::from_arrow(batch)))
}

#[cfg(feature = "odbc")]
fn odbc_query_row_at_a_time(connection_string: &str, sql: &str) -> Result<Value, IoError> {
    use arrow::array::{Float64Array, StringArray};
    use arrow::datatypes::{DataType, Schema};
    use odbc_api::{ConnectionOptions, Cursor};

    let env = odbc_env();
    let conn = env
        .connect_with_connection_string(connection_string, ConnectionOptions::default())
        .map_err(|e| IoError::Other(format!("Odbc.Query connect: {}", e)))?;
    let cursor = conn
        .execute(sql, (), None)
        .map_err(|e| IoError::Other(format!("Odbc.Query execute: {}", e)))?;
    let Some(mut cursor) = cursor else {
        return Ok(empty_table());
    };

    let (fields, _buf_descs) = describe_columns(&mut cursor)?;
    let n_cols = fields.len();

    let mut accumulated_columns: Vec<Vec<Option<f64>>> = vec![Vec::new(); n_cols];
    let mut accumulated_strings: Vec<Vec<Option<String>>> = vec![Vec::new(); n_cols];

    let mut buf = Vec::<u8>::new();
    while let Some(mut row) = cursor
        .next_row()
        .map_err(|e| IoError::Other(format!("Odbc.Query fetch: {}", e)))?
    {
        for col_idx in 0..n_cols {
            buf.clear();
            let has_data = row
                .get_text((col_idx + 1) as u16, &mut buf)
                .map_err(|e| {
                    IoError::Other(format!(
                        "Odbc.Query read col {}: {}",
                        col_idx + 1,
                        e
                    ))
                })?;
            match fields[col_idx].data_type() {
                DataType::Float64 => {
                    let cell = if !has_data {
                        None
                    } else {
                        String::from_utf8_lossy(&buf).trim().parse::<f64>().ok()
                    };
                    accumulated_columns[col_idx].push(cell);
                }
                DataType::Utf8 => {
                    let cell = if !has_data {
                        None
                    } else {
                        Some(String::from_utf8_lossy(&buf).into_owned())
                    };
                    accumulated_strings[col_idx].push(cell);
                }
                _ => unreachable!("schema only contains Float64/Utf8 in this slice"),
            }
        }
    }

    let columns: Vec<arrow::array::ArrayRef> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| match f.data_type() {
            DataType::Float64 => std::sync::Arc::new(Float64Array::from(std::mem::take(
                &mut accumulated_columns[i],
            ))) as arrow::array::ArrayRef,
            DataType::Utf8 => std::sync::Arc::new(StringArray::from(std::mem::take(
                &mut accumulated_strings[i],
            ))) as arrow::array::ArrayRef,
            _ => unreachable!(),
        })
        .collect();

    let schema = std::sync::Arc::new(Schema::new(fields));
    let batch = arrow::record_batch::RecordBatch::try_new(schema, columns)
        .map_err(|e| IoError::Other(format!("Odbc.Query batch: {}", e)))?;
    Ok(Value::Table(Table::from_arrow(batch)))
}

#[cfg(feature = "odbc")]
fn describe_columns<C: odbc_api::Cursor + odbc_api::ResultSetMetadata>(
    cursor: &mut C,
) -> Result<(Vec<arrow::datatypes::Field>, Vec<odbc_api::buffers::BufferDesc>), IoError> {
    use arrow::datatypes::{DataType, Field};
    use odbc_api::buffers::BufferDesc;

    let n_cols = cursor
        .num_result_cols()
        .map_err(|e| IoError::Other(format!("Odbc.Query cols: {}", e)))?;
    let mut fields = Vec::with_capacity(n_cols as usize);
    let mut buf_descs = Vec::with_capacity(n_cols as usize);
    let mut desc = odbc_api::ColumnDescription::default();
    for col_idx in 1..=n_cols {
        cursor
            .describe_col(col_idx as u16, &mut desc)
            .map_err(|e| IoError::Other(format!("Odbc.Query describe col {}: {}", col_idx, e)))?;
        let name = desc.name_to_string().unwrap_or_else(|_| format!("col{col_idx}"));
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
    Ok((fields, buf_descs))
}

#[cfg(feature = "odbc")]
fn empty_table() -> Value {
    let schema = std::sync::Arc::new(arrow::datatypes::Schema::empty());
    let options = arrow::record_batch::RecordBatchOptions::new().with_row_count(Some(0));
    let batch = arrow::record_batch::RecordBatch::try_new_with_options(schema, vec![], &options)
        .expect("empty batch always valid");
    Value::Table(Table::from_arrow(batch))
}

// --- ODBC navigation table (Odbc.DataSource, feature-gated) ---
//
// Builds the two-level navigation table Power Query's "Get Data from ODBC"
// generates: top level rows are catalogs/databases; each catalog's `[Data]`
// resolves on demand to a sub-table of tables; each table's `[Data]`
// resolves on demand to the actual SELECT * result. The catalog/table
// metadata is fetched eagerly via SQLTables (cheap); only the row data is
// lazy (the costly part).

#[cfg(feature = "odbc")]
fn odbc_data_source_impl(connection_string: &str) -> Result<Value, IoError> {
    use std::cell::RefCell;
    use std::collections::BTreeMap;
    use std::rc::Rc;

    use mrsflow_core::eval::{MError, ThunkState};
    use odbc_api::{ConnectionOptions, Cursor, ResultSetMetadata};

    let env = odbc_env();

    let conn = env
        .connect_with_connection_string(connection_string, ConnectionOptions::default())
        .map_err(|e| IoError::Other(format!("Odbc.DataSource connect: {}", e)))?;

    // SQLTables with empty filters returns all tables across all catalogs.
    // Column order: TABLE_CAT, TABLE_SCHEM, TABLE_NAME, TABLE_TYPE, REMARKS.
    let mut tables_cursor = conn
        .tables("", "", "", "")
        .map_err(|e| IoError::Other(format!("Odbc.DataSource tables: {}", e)))?;

    let n_cols = tables_cursor
        .num_result_cols()
        .map_err(|e| IoError::Other(format!("Odbc.DataSource n_cols: {}", e)))?;
    if n_cols < 4 {
        return Err(IoError::Other(format!(
            "Odbc.DataSource: SQLTables returned only {n_cols} columns, expected >= 4"
        )));
    }

    let mut by_catalog: BTreeMap<String, Vec<(String, String)>> = BTreeMap::new();
    let mut buf = Vec::<u8>::new();
    while let Some(mut row) = tables_cursor
        .next_row()
        .map_err(|e| IoError::Other(format!("Odbc.DataSource fetch: {}", e)))?
    {
        let mut read = |col: u16| -> Result<String, IoError> {
            buf.clear();
            row.get_text(col, &mut buf)
                .map_err(|e| IoError::Other(format!("Odbc.DataSource col {col}: {e}")))?;
            Ok(String::from_utf8_lossy(&buf).into_owned())
        };
        let catalog = read(1)?;
        let _schema = read(2)?;
        let table_name = read(3)?;
        let table_type = read(4)?;
        // Skip system catalogs / internal driver bookkeeping.
        if table_name.is_empty() {
            continue;
        }
        by_catalog
            .entry(catalog)
            .or_default()
            .push((table_name, table_type));
    }

    // Drop the cursor + connection now so the lazy thunks open fresh
    // connections when they fire (the cached `Environment` is per-process).
    drop(tables_cursor);
    drop(conn);

    let cols = vec!["Name".to_string(), "Data".to_string(), "Kind".to_string()];
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(by_catalog.len());
    for (catalog, tables) in by_catalog {
        let conn_string = connection_string.to_string();
        let catalog_for_thunk = catalog.clone();
        let tables_for_thunk = tables.clone();
        let inner: Rc<dyn Fn() -> Result<Value, MError>> = Rc::new(move || {
            Ok(build_table_nav(
                &conn_string,
                &catalog_for_thunk,
                &tables_for_thunk,
            ))
        });
        let data = Value::Thunk(Rc::new(RefCell::new(ThunkState::Native(inner))));
        rows.push(vec![
            Value::Text(catalog),
            data,
            Value::Text("Database".to_string()),
        ]);
    }
    Ok(Value::Table(Table::from_rows(cols, rows)))
}

#[cfg(feature = "odbc")]
fn build_table_nav(connection: &str, catalog: &str, tables: &[(String, String)]) -> Value {
    use std::cell::RefCell;
    use std::rc::Rc;

    use mrsflow_core::eval::{MError, ThunkState};

    let cols = vec!["Name".to_string(), "Data".to_string(), "Kind".to_string()];
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(tables.len());
    for (name, table_type) in tables {
        let conn_string = connection.to_string();
        let catalog_name = catalog.to_string();
        let table_name = name.clone();
        let _ = &catalog_name; // captured for future multi-catalog support
        let fetcher: Rc<dyn Fn() -> Result<Value, MError>> = Rc::new(move || {
            // Bare table name; DBISAM (the corpus driver) rejects
            // "catalog"."table" qualification, and the connection string
            // already pins the database for single-catalog DSNs. For
            // multi-catalog drivers (MSSQL, etc.) we'd need to issue `USE`
            // or append `DATABASE=<catalog>` to the connection string;
            // expand here when the corpus hits that case.
            let sql = format!("SELECT * FROM \"{}\"", table_name);
            odbc_query_impl(&conn_string, &sql)
                .map_err(|e| MError::Other(format!("Odbc.DataSource fetch: {e:?}")))
        });
        let data = Value::Thunk(Rc::new(RefCell::new(ThunkState::Native(fetcher))));
        // PQ uses title-case Kind values: "Table", "View", etc.
        let kind = match table_type.as_str() {
            "TABLE" => "Table",
            "VIEW" => "View",
            "SYSTEM TABLE" => "SystemTable",
            "ALIAS" => "Alias",
            "SYNONYM" => "Synonym",
            other => other,
        };
        rows.push(vec![
            Value::Text(name.clone()),
            data,
            Value::Text(kind.to_string()),
        ]);
    }
    Value::Table(Table::from_rows(cols, rows))
}
