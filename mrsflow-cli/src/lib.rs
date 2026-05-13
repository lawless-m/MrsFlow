//! CLI shell for mrsflow. Implements the `IoHost` trait against the real
//! filesystem and the `parquet` crate. ODBC plumbing lands in eval-8.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};

use mrsflow_core::eval::{deep_force, root_env, EnvOps, IoError, IoHost, Table, Value};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::{parse, Expr};
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

pub struct CliIoHost {
    /// Named parameters injected via `--param NAME=VALUE`. Surface to M
    /// queries via `Excel.CurrentWorkbook()` (the only PQ idiom for
    /// query-time parameters). Values are always text — the corpus calls
    /// `Text.From` on numeric uses so this is fine.
    params: Vec<(String, String)>,
}

impl CliIoHost {
    pub fn new() -> Self {
        Self { params: Vec::new() }
    }

    pub fn with_params(params: Vec<(String, String)>) -> Self {
        Self { params }
    }
}

impl Default for CliIoHost {
    fn default() -> Self {
        Self::new()
    }
}

impl IoHost for CliIoHost {
    fn parquet_read(&self, path: &str) -> Result<Value, IoError> {
        // Read the bytes into a Bytes buffer and hand to Table::lazy_parquet —
        // mrsflow-core decides when (and which columns) to actually decode.
        // For Parquet files of ~200MB this is a single allocation + read; the
        // big win is that the per-column decode happens later, only for the
        // columns the M source actually references.
        let bytes = std::fs::read(path)
            .map_err(|e| IoError::Other(format!("open {path}: {e}")))?;
        let table = Table::lazy_parquet(bytes::Bytes::from(bytes))
            .map_err(|e| IoError::Other(format!("parquet read {path}: {e:?}")))?;
        Ok(Value::Table(table))
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

    fn excel_workbook(
        &self,
        bytes: &[u8],
        use_headers: bool,
        delay_types: bool,
    ) -> Result<Value, IoError> {
        excel_workbook_impl(bytes, use_headers, delay_types)
    }

    fn web_contents(
        &self,
        url: &str,
        headers: &[(String, String)],
        manual_status: &[u16],
        content: Option<&[u8]>,
    ) -> Result<Vec<u8>, IoError> {
        web_contents_impl(url, headers, manual_status, content)
    }

    fn folder_contents(&self, path: &str) -> Result<Value, IoError> {
        folder_impl(path, /* recursive */ false)
    }

    fn folder_files(&self, path: &str) -> Result<Value, IoError> {
        folder_impl(path, /* recursive */ true)
    }

    fn current_workbook(&self) -> Result<Value, IoError> {
        // Build a Table with columns Name, Content. Each Content cell is a
        // 1-row Table with a single "Value" column. Matches PQ's
        // Excel.CurrentWorkbook(){[Name="..."]}[Content]{0}[Value] chain.
        let value_cols: Vec<String> = vec!["Value".into()];
        let rows: Vec<Vec<Value>> = self
            .params
            .iter()
            .map(|(k, v)| {
                let inner = Table::from_rows(
                    value_cols.clone(),
                    vec![vec![Value::Text(v.clone())]],
                );
                vec![Value::Text(k.clone()), Value::Table(inner)]
            })
            .collect();
        Ok(Value::Table(Table::from_rows(
            vec!["Name".into(), "Content".into()],
            rows,
        )))
    }
}

/// Build a `Folder.*` result table. When `recursive`, walks subdirectories
/// and emits file rows only; otherwise emits one row per immediate child
/// (including subdirectories, whose Content is null).
fn folder_impl(path: &str, recursive: bool) -> Result<Value, IoError> {
    use mrsflow_core::eval::Record;

    let root = Path::new(path);
    if !root.is_dir() {
        return Err(IoError::Other(format!(
            "Folder.* expects a directory, got {path:?}"
        )));
    }

    let mut rows: Vec<Vec<Value>> = Vec::new();

    let mut emit = |entry_path: &Path| -> Result<(), IoError> {
        let md = std::fs::metadata(entry_path)
            .map_err(|e| IoError::Other(format!("metadata {}: {e}", entry_path.display())))?;
        let is_dir = md.is_dir();

        let name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| IoError::Other(format!("non-utf8 name: {}", entry_path.display())))?
            .to_string();

        let extension = if is_dir {
            String::new()
        } else {
            entry_path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| format!(".{e}"))
                .unwrap_or_default()
        };

        let parent = entry_path
            .parent()
            .and_then(|p| p.to_str())
            .unwrap_or("")
            .to_string();
        // PQ convention: trailing separator on folder paths.
        let folder_path = if parent.is_empty() {
            String::new()
        } else if parent.ends_with('/') || parent.ends_with('\\') {
            parent
        } else {
            format!("{parent}/")
        };

        let content = if is_dir {
            Value::Null
        } else {
            // Eager read. Big-dir cost noted in stdlib::folder.
            let bytes = std::fs::read(entry_path)
                .map_err(|e| IoError::Other(format!("read {}: {e}", entry_path.display())))?;
            Value::Binary(bytes)
        };

        let modified = systime_to_datetime(md.modified().ok());
        let accessed = systime_to_datetime(md.accessed().ok());
        let created = systime_to_datetime(md.created().ok());

        // Hidden: Linux convention is dotfile prefix. Windows attribute
        // bits aren't reachable through std::fs::metadata; if/when we
        // need them, switch to a platform-cfg'd path.
        let hidden = name.starts_with('.');
        let kind = if is_dir { "Folder" } else { "File" };
        let attrs = Value::Record(Record {
            fields: vec![
                ("Kind".into(), Value::Text(kind.into())),
                ("Size".into(), Value::Number(md.len() as f64)),
                ("Hidden".into(), Value::Logical(hidden)),
                ("Directory".into(), Value::Logical(is_dir)),
            ],
            env: mrsflow_core::eval::EnvNode::empty(),
        });

        rows.push(vec![
            content,
            Value::Text(name),
            Value::Text(extension),
            accessed,
            modified,
            created,
            attrs,
            Value::Text(folder_path),
        ]);
        Ok(())
    };

    if recursive {
        for entry in walkdir::WalkDir::new(root).follow_links(false) {
            let entry = entry
                .map_err(|e| IoError::Other(format!("walk {path}: {e}")))?;
            if entry.file_type().is_file() {
                emit(entry.path())?;
            }
        }
    } else {
        let read_dir = std::fs::read_dir(root)
            .map_err(|e| IoError::Other(format!("read_dir {path}: {e}")))?;
        for entry in read_dir {
            let entry = entry
                .map_err(|e| IoError::Other(format!("read_dir {path}: {e}")))?;
            emit(&entry.path())?;
        }
    }

    let columns = vec![
        "Content".into(),
        "Name".into(),
        "Extension".into(),
        "Date accessed".into(),
        "Date modified".into(),
        "Date created".into(),
        "Attributes".into(),
        "Folder Path".into(),
    ];
    Ok(Value::Table(Table::from_rows(columns, rows)))
}

fn systime_to_datetime(t: Option<std::time::SystemTime>) -> Value {
    match t {
        Some(st) => {
            let dt: chrono::DateTime<chrono::Utc> = st.into();
            Value::Datetime(dt.naive_utc())
        }
        None => Value::Null,
    }
}

fn web_contents_impl(
    url: &str,
    headers: &[(String, String)],
    manual_status: &[u16],
    content: Option<&[u8]>,
) -> Result<Vec<u8>, IoError> {
    use reqwest::blocking::Client;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    let mut hm = HeaderMap::with_capacity(headers.len());
    for (k, v) in headers {
        // Empty value: PQ uses this as "use ambient credentials". reqwest
        // can't supply those; skip the header so the request still goes
        // out, just unauthenticated.
        if v.is_empty() {
            continue;
        }
        let name = HeaderName::try_from(k.as_str())
            .map_err(|e| IoError::Other(format!("invalid header name {k:?}: {e}")))?;
        let value = HeaderValue::from_str(v)
            .map_err(|e| IoError::Other(format!("invalid header value for {k:?}: {e}")))?;
        hm.insert(name, value);
    }

    let client = Client::builder()
        .default_headers(hm)
        .build()
        .map_err(|e| IoError::Other(format!("http client init: {e}")))?;

    // M's contract: presence of `Content` switches the verb to POST.
    let (req, method_label) = match content {
        None => (client.get(url), "GET"),
        Some(body) => (client.post(url).body(body.to_vec()), "POST"),
    };
    let resp = req
        .send()
        .map_err(|e| IoError::Other(format!("{method_label} {url}: {e}")))?;

    let status = resp.status().as_u16();
    let body = resp
        .bytes()
        .map_err(|e| IoError::Other(format!("read body {url}: {e}")))?
        .to_vec();

    let ok = (200..300).contains(&status) || manual_status.contains(&status);
    if !ok {
        return Err(IoError::Other(format!(
            "{method_label} {url}: HTTP {status} (not in ManualStatusHandling)"
        )));
    }
    Ok(body)
}

fn excel_workbook_impl(
    bytes: &[u8],
    use_headers: bool,
    delay_types: bool,
) -> Result<Value, IoError> {
    use std::io::Cursor;
    use calamine::{open_workbook_auto_from_rs, Reader, Sheets, SheetVisible};

    // `open_workbook_auto_from_rs` requires `RS: Clone` — it tries each
    // format in turn. `Cursor<Vec<u8>>` is Clone; the bytes get duplicated
    // up to 4× during sniffing but the cost is dwarfed by actual parsing.
    let cursor = Cursor::new(bytes.to_vec());
    let mut wb = open_workbook_auto_from_rs(cursor)
        .map_err(|e| IoError::Other(format!("open workbook: {e}")))?;

    // Sheet metadata (name + visibility) — snapshot before mutating wb.
    let sheet_meta: Vec<(String, bool)> = wb
        .sheets_metadata()
        .iter()
        .map(|s| (s.name.clone(), !matches!(s.visible, SheetVisible::Visible)))
        .collect();

    let mut all_rows: Vec<Vec<Value>> = Vec::new();

    // --- Sheet rows ---
    for (name, hidden) in &sheet_meta {
        let range = wb
            .worksheet_range(name)
            .map_err(|e| IoError::Other(format!("read sheet {name:?}: {e}")))?;
        let data_table = range_to_table(&range, use_headers, delay_types);
        all_rows.push(vec![
            Value::Text(name.clone()),
            Value::Table(data_table),
            Value::Text(name.clone()),
            Value::Text("Sheet".into()),
            Value::Logical(*hidden),
        ]);
    }

    // --- Table rows (ListObjects, xlsx only) ---
    if let Sheets::Xlsx(xlsx) = &mut wb {
        xlsx.load_tables()
            .map_err(|e| IoError::Other(format!("load_tables: {e}")))?;
        let names: Vec<String> = xlsx.table_names().into_iter().cloned().collect();
        for tname in &names {
            let tbl = xlsx
                .table_by_name(tname)
                .map_err(|e| IoError::Other(format!("table {tname:?}: {e}")))?;
            // ListObjects always have headers — they're authored as named
            // columns. We use those directly regardless of use_headers.
            let columns: Vec<String> = tbl.columns().to_vec();
            let data_rows: Vec<Vec<Value>> = tbl
                .data()
                .rows()
                .map(|row| row.iter().map(|c| cell_to_value(c, delay_types)).collect())
                .collect();
            let data_table = Table::from_rows(columns, data_rows);
            all_rows.push(vec![
                Value::Text(tname.clone()),
                Value::Table(data_table),
                Value::Text(tname.clone()),
                Value::Text("Table".into()),
                Value::Logical(false),
            ]);
        }
    }

    // --- DefinedName rows ---
    //
    // Power Query's DefinedName rows expose the *value* the name resolves
    // to. Evaluating XLSX formulas (`Sheet1!$A$1:$B$10` etc.) is a parser
    // job we don't have. For now, surface the row so `Source{[Name=…,
    // Kind="DefinedName"]}` finds it, with Data=Null. The formula text is
    // stored as Item so a curious caller can read it.
    for (n, formula) in wb.defined_names() {
        all_rows.push(vec![
            Value::Text(n.clone()),
            Value::Null,
            Value::Text(formula.clone()),
            Value::Text("DefinedName".into()),
            Value::Logical(false),
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
        all_rows,
    )))
}

/// Build a Rows-backed Table from a calamine Range. With `use_headers`,
/// the first row's text representations become column names and rows
/// 1..N become data. Without `use_headers`, columns are `Column1..N`.
/// With `delay_types=false`, attempts per-column type promotion.
fn range_to_table(
    range: &calamine::Range<calamine::Data>,
    use_headers: bool,
    delay_types: bool,
) -> Table {
    let (_, width) = range.get_size();
    let mut row_iter = range.rows();

    let (columns, data_rows): (Vec<String>, Vec<Vec<Value>>) = if use_headers {
        let header = row_iter.next();
        let columns: Vec<String> = match header {
            Some(h) => h
                .iter()
                .enumerate()
                .map(|(i, c)| cell_header_text(c, i))
                .collect(),
            None => (1..=width).map(|i| format!("Column{i}")).collect(),
        };
        let rows: Vec<Vec<Value>> = row_iter
            .map(|r| r.iter().map(|c| cell_to_value(c, delay_types)).collect())
            .collect();
        (columns, rows)
    } else {
        let columns: Vec<String> = (1..=width).map(|i| format!("Column{i}")).collect();
        let rows: Vec<Vec<Value>> = row_iter
            .map(|r| r.iter().map(|c| cell_to_value(c, delay_types)).collect())
            .collect();
        (columns, rows)
    };

    Table::from_rows(columns, data_rows)
}

/// Header-row cell → column name. Mirrors Power Query's PromoteHeaders:
/// empty/null cells fall back to `Column<n+1>`. Duplicate detection
/// (suffixing `_1, _2…`) isn't implemented yet — corpus queries that hit
/// dupe headers will need it.
fn cell_header_text(cell: &calamine::Data, idx: usize) -> String {
    use calamine::Data;
    match cell {
        Data::Empty | Data::Error(_) => format!("Column{}", idx + 1),
        Data::String(s) if s.is_empty() => format!("Column{}", idx + 1),
        Data::String(s) => s.clone(),
        Data::Float(f) => format!("{f}"),
        Data::Int(i) => format!("{i}"),
        Data::Bool(b) => format!("{b}"),
        Data::DateTime(d) => format!("{}", d.as_f64()),
        Data::DateTimeIso(s) | Data::DurationIso(s) => s.clone(),
    }
}

fn cell_to_value(cell: &calamine::Data, delay_types: bool) -> Value {
    use calamine::Data;
    match cell {
        Data::Empty => Value::Null,
        Data::String(s) => Value::Text(s.clone()),
        Data::Float(f) => Value::Number(*f),
        Data::Int(i) => Value::Number(*i as f64),
        Data::Bool(b) => Value::Logical(*b),
        Data::DateTime(d) => {
            if delay_types {
                // PQ's delayTypes=true contract: keep the serial float.
                Value::Number(d.as_f64())
            } else {
                // Decode via calamine. `as_datetime()` handles Excel's
                // 1900-leap-year quirk and returns NaiveDateTime.
                match d.as_datetime() {
                    Some(ndt) => {
                        let has_time = ndt.time() != chrono::NaiveTime::MIN;
                        if has_time {
                            Value::Datetime(ndt)
                        } else {
                            Value::Date(ndt.date())
                        }
                    }
                    None => Value::Number(d.as_f64()),
                }
            }
        }
        Data::DateTimeIso(s) | Data::DurationIso(s) => Value::Text(s.clone()),
        Data::Error(_) => Value::Null,
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
            Err(panic_payload) => {
                if let Ok(mut set) = columnar_blocklist().lock() {
                    set.insert(connection_string.to_string());
                }
                let panic_msg = panic_payload
                    .downcast_ref::<&'static str>()
                    .map(|s| s.to_string())
                    .or_else(|| panic_payload.downcast_ref::<String>().cloned())
                    .unwrap_or_else(|| "<non-string panic payload>".to_string());
                eprintln!(
                    "Odbc.Query: columnar fetch panicked for `{connection_string}`; \
                     falling back to row-at-a-time. panic: {panic_msg}"
                );
            }
        }
    }

    odbc_query_row_at_a_time(connection_string, sql)
}

#[cfg(feature = "odbc")]
fn odbc_query_columnar(connection_string: &str, sql: &str) -> Result<Value, IoError> {
    use arrow::array::{Date32Array, Float64Array, StringArray, TimestampMicrosecondArray};
    use arrow::datatypes::{DataType, Schema, TimeUnit};
    use chrono::NaiveDate;
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

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let mut acc_f64: Vec<Vec<Option<f64>>> = vec![Vec::new(); n_cols];
    let mut acc_str: Vec<Vec<Option<String>>> = vec![Vec::new(); n_cols];
    let mut acc_date: Vec<Vec<Option<i32>>> = vec![Vec::new(); n_cols];
    let mut acc_ts: Vec<Vec<Option<i64>>> = vec![Vec::new(); n_cols];

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
                        acc_f64[col_idx].push(opt.copied());
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
                            .map(|b| String::from_utf8_lossy(b).trim_end().to_string());
                        acc_str[col_idx].push(s);
                    }
                }
                DataType::Date32 => {
                    let view = batch
                        .column(col_idx)
                        .as_nullable_slice::<odbc_api::sys::Date>()
                        .expect("Date buffer");
                    for opt in view {
                        let cell = opt.and_then(|d| {
                            NaiveDate::from_ymd_opt(
                                d.year as i32,
                                d.month as u32,
                                d.day as u32,
                            )
                            .map(|nd| (nd - epoch).num_days() as i32)
                        });
                        acc_date[col_idx].push(cell);
                    }
                }
                DataType::Timestamp(TimeUnit::Microsecond, _) => {
                    let view = batch
                        .column(col_idx)
                        .as_nullable_slice::<odbc_api::sys::Timestamp>()
                        .expect("Timestamp buffer");
                    for opt in view {
                        let cell = opt.and_then(|t| {
                            let date = NaiveDate::from_ymd_opt(
                                t.year as i32,
                                t.month as u32,
                                t.day as u32,
                            )?;
                            // fraction is in nanoseconds (per ODBC spec).
                            let nanos = t.fraction;
                            let dt = date.and_hms_nano_opt(
                                t.hour as u32,
                                t.minute as u32,
                                t.second as u32,
                                nanos,
                            )?;
                            Some(dt.and_utc().timestamp_micros())
                        });
                        acc_ts[col_idx].push(cell);
                    }
                }
                _ => unreachable!("describe_columns only emits Float64/Utf8/Date32/Timestamp(us)"),
            }
        }
    }

    let columns: Vec<arrow::array::ArrayRef> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| -> arrow::array::ArrayRef {
            match f.data_type() {
                DataType::Float64 => std::sync::Arc::new(Float64Array::from(std::mem::take(
                    &mut acc_f64[i],
                ))),
                DataType::Utf8 => std::sync::Arc::new(StringArray::from(std::mem::take(
                    &mut acc_str[i],
                ))),
                DataType::Date32 => std::sync::Arc::new(Date32Array::from(std::mem::take(
                    &mut acc_date[i],
                ))),
                DataType::Timestamp(TimeUnit::Microsecond, _) => std::sync::Arc::new(
                    TimestampMicrosecondArray::from(std::mem::take(&mut acc_ts[i])),
                ),
                _ => unreachable!(),
            }
        })
        .collect();

    let schema = std::sync::Arc::new(Schema::new(fields));
    let batch = arrow::record_batch::RecordBatch::try_new(schema, columns)
        .map_err(|e| IoError::Other(format!("Odbc.Query batch: {}", e)))?;
    Ok(Value::Table(Table::from_arrow(batch)))
}

#[cfg(feature = "odbc")]
fn odbc_query_row_at_a_time(connection_string: &str, sql: &str) -> Result<Value, IoError> {
    use arrow::array::{Date32Array, Float64Array, StringArray, TimestampMicrosecondArray};
    use arrow::datatypes::{DataType, Schema, TimeUnit};
    use chrono::NaiveDate;
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

    let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    let mut acc_f64: Vec<Vec<Option<f64>>> = vec![Vec::new(); n_cols];
    let mut acc_str: Vec<Vec<Option<String>>> = vec![Vec::new(); n_cols];
    let mut acc_date: Vec<Vec<Option<i32>>> = vec![Vec::new(); n_cols];
    let mut acc_ts: Vec<Vec<Option<i64>>> = vec![Vec::new(); n_cols];

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
                    acc_f64[col_idx].push(cell);
                }
                DataType::Utf8 => {
                    let cell = if !has_data {
                        None
                    } else {
                        Some(String::from_utf8_lossy(&buf).into_owned())
                    };
                    acc_str[col_idx].push(cell);
                }
                DataType::Date32 => {
                    let cell = if !has_data {
                        None
                    } else {
                        let text = String::from_utf8_lossy(&buf);
                        let t = text.trim();
                        let d = NaiveDate::parse_from_str(t, "%Y-%m-%d")
                            .or_else(|_| NaiveDate::parse_from_str(t, "%d/%m/%Y"))
                            .or_else(|_| NaiveDate::parse_from_str(t, "%m/%d/%Y"))
                            .ok();
                        d.map(|d| (d - epoch).num_days() as i32)
                    };
                    acc_date[col_idx].push(cell);
                }
                DataType::Timestamp(TimeUnit::Microsecond, _) => {
                    let cell = if !has_data {
                        None
                    } else {
                        let text = String::from_utf8_lossy(&buf);
                        let t = text.trim();
                        // Common ODBC timestamp shapes; prefer ISO.
                        let dt = chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S%.f")
                            .or_else(|_| {
                                chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%d %H:%M:%S")
                            })
                            .or_else(|_| {
                                chrono::NaiveDateTime::parse_from_str(t, "%Y-%m-%dT%H:%M:%S%.f")
                            })
                            .ok();
                        dt.map(|d| d.and_utc().timestamp_micros())
                    };
                    acc_ts[col_idx].push(cell);
                }
                _ => unreachable!("describe_columns only emits Float64/Utf8/Date32/Timestamp(us)"),
            }
        }
    }

    let columns: Vec<arrow::array::ArrayRef> = fields
        .iter()
        .enumerate()
        .map(|(i, f)| -> arrow::array::ArrayRef {
            match f.data_type() {
                DataType::Float64 => std::sync::Arc::new(Float64Array::from(std::mem::take(
                    &mut acc_f64[i],
                ))),
                DataType::Utf8 => std::sync::Arc::new(StringArray::from(std::mem::take(
                    &mut acc_str[i],
                ))),
                DataType::Date32 => std::sync::Arc::new(Date32Array::from(std::mem::take(
                    &mut acc_date[i],
                ))),
                DataType::Timestamp(TimeUnit::Microsecond, _) => std::sync::Arc::new(
                    TimestampMicrosecondArray::from(std::mem::take(&mut acc_ts[i])),
                ),
                _ => unreachable!(),
            }
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
            // Temporal types: bind natively (SQL_C_TYPE_DATE/TIMESTAMP).
            // DBISAM silently returns null for date columns when bound as
            // text, so going through native struct buffers is required.
            // The columnar path decodes the resulting odbc_api::sys::Date /
            // Timestamp into Arrow Date32 / Timestamp(us). Time stays as
            // text since we don't have a typed Arrow Time array in this
            // pipeline.
            odbc_api::DataType::Date => {
                (DataType::Date32, BufferDesc::Date { nullable: true })
            }
            odbc_api::DataType::Time { .. } => {
                (DataType::Utf8, BufferDesc::Text { max_str_len: 32 })
            }
            odbc_api::DataType::Timestamp { .. } => {
                (
                    DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
                    BufferDesc::Timestamp { nullable: true },
                )
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
