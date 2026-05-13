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
        Value::Decimal { .. } => "number",
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

    #[cfg(not(feature = "mysql"))]
    fn mysql_database(
        &self,
        _server: &str,
        _database: &str,
        _options: Option<&Value>,
    ) -> Result<Value, IoError> {
        Err(IoError::Other(
            "MySQL.Database: built without MySQL support — recompile mrsflow-cli with --features mysql".into(),
        ))
    }

    #[cfg(feature = "mysql")]
    fn mysql_database(
        &self,
        server: &str,
        database: &str,
        options: Option<&Value>,
    ) -> Result<Value, IoError> {
        mysql_database_impl(server, database, options)
    }

    #[cfg(not(feature = "postgresql"))]
    fn postgres_database(
        &self,
        _server: &str,
        _database: &str,
        _options: Option<&Value>,
    ) -> Result<Value, IoError> {
        Err(IoError::Other(
            "PostgreSQL.Database: built without PostgreSQL support — recompile mrsflow-cli with --features postgresql".into(),
        ))
    }

    #[cfg(feature = "postgresql")]
    fn postgres_database(
        &self,
        server: &str,
        database: &str,
        options: Option<&Value>,
    ) -> Result<Value, IoError> {
        postgres_database_impl(server, database, options)
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

    fn web_headers(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Result<Vec<(String, String)>, IoError> {
        web_headers_impl(url, headers)
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

fn web_headers_impl(
    url: &str,
    headers: &[(String, String)],
) -> Result<Vec<(String, String)>, IoError> {
    use reqwest::blocking::Client;
    use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

    let mut hm = HeaderMap::with_capacity(headers.len());
    for (k, v) in headers {
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

    let resp = client
        .head(url)
        .send()
        .map_err(|e| IoError::Other(format!("HEAD {url}: {e}")))?;

    let mut out: Vec<(String, String)> = Vec::with_capacity(resp.headers().len());
    for (k, v) in resp.headers() {
        if let Ok(s) = v.to_str() {
            out.push((k.as_str().to_string(), s.to_string()));
        }
    }
    Ok(out)
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

// ============================================================================
// MySQL.Database — native MySQL protocol client with rustls TLS.
// Connection options come in via the M record passed to MySQL.Database
// (already deep-forced in the stdlib binding, so we just pattern-match
// here without forcing). The shape of the returned navigation table
// mirrors what Odbc.DataSource produces: a table with columns
// (Name, Data, ItemKind, ItemName, IsLeaf) where each Data is a lazy
// thunk that runs `SELECT * FROM \`db\`.\`table\`` on force.
// ============================================================================

#[cfg(feature = "mysql")]
struct MySqlConn {
    host: String,
    port: u16,
    user: Option<String>,
    password: Option<String>,
    database: String,
    ssl_mode: SslMode,
    connection_timeout_s: Option<u64>,
}

#[cfg(feature = "mysql")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum SslMode {
    None,
    Preferred,
    Required,
    VerifyCa,
    VerifyFull,
}

#[cfg(feature = "mysql")]
fn mysql_database_impl(
    server: &str,
    database: &str,
    options: Option<&Value>,
) -> Result<Value, IoError> {
    use std::cell::RefCell;
    use std::rc::Rc;

    use mrsflow_core::eval::{MError, ThunkState};

    let conn = parse_mysql_conn(server, database, options)?;

    // Connect once to enumerate tables. The Data thunks reopen on force —
    // simpler than sharing a connection across thunk firings, and the
    // navigation table itself can outlive any single connection.
    let table_names = mysql_list_tables(&conn)?;

    let cols = vec![
        "Name".to_string(),
        "Data".to_string(),
        "ItemKind".to_string(),
        "ItemName".to_string(),
        "IsLeaf".to_string(),
    ];
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(table_names.len());
    for name in table_names {
        let conn_for_thunk = conn.clone();
        let table_for_thunk = name.clone();
        let fetcher: Rc<dyn Fn() -> Result<Value, MError>> = Rc::new(move || {
            let sql = format!(
                "SELECT * FROM `{}`.`{}`",
                conn_for_thunk.database.replace('`', "``"),
                table_for_thunk.replace('`', "``"),
            );
            mysql_query_value(&conn_for_thunk, &sql)
                .map_err(|e| MError::Other(format!("MySQL fetch: {e:?}")))
        });
        let data = Value::Thunk(Rc::new(RefCell::new(ThunkState::Native(fetcher))));
        rows.push(vec![
            Value::Text(name.clone()),
            data,
            Value::Text("Table".to_string()),
            Value::Text(name),
            Value::Logical(true),
        ]);
    }
    Ok(Value::Table(Table::from_rows(cols, rows)))
}

#[cfg(feature = "mysql")]
impl Clone for MySqlConn {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            port: self.port,
            user: self.user.clone(),
            password: self.password.clone(),
            database: self.database.clone(),
            ssl_mode: self.ssl_mode,
            connection_timeout_s: self.connection_timeout_s,
        }
    }
}

#[cfg(feature = "mysql")]
fn parse_mysql_conn(
    server: &str,
    database: &str,
    options: Option<&Value>,
) -> Result<MySqlConn, IoError> {
    // server is "host" or "host:port". Options may override Port.
    let (host_part, port_from_server) = match server.rsplit_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(n) => (h.to_string(), Some(n)),
            Err(_) => (server.to_string(), None),
        },
        None => (server.to_string(), None),
    };

    let mut user: Option<String> = None;
    let mut password: Option<String> = None;
    let mut port_from_opts: Option<u16> = None;
    let mut ssl_mode = SslMode::Preferred;
    let mut connection_timeout_s: Option<u64> = None;

    if let Some(Value::Record(r)) = options {
        for (k, v) in &r.fields {
            match (k.as_str(), v) {
                ("UserName", Value::Text(s)) => user = Some(s.clone()),
                ("UserName", Value::Null) | ("UserName", _) if matches!(v, Value::Null) => {}
                ("Password", Value::Text(s)) => password = Some(s.clone()),
                ("Password", Value::Null) => {}
                ("Port", Value::Number(n)) if n.is_finite() && *n >= 0.0 && *n <= 65535.0 => {
                    port_from_opts = Some(*n as u16);
                }
                ("Port", Value::Null) => {}
                ("SslMode", Value::Text(s)) => {
                    ssl_mode = match s.as_str() {
                        "None" => SslMode::None,
                        "Preferred" => SslMode::Preferred,
                        "Required" => SslMode::Required,
                        "VerifyCA" => SslMode::VerifyCa,
                        "VerifyFull" => SslMode::VerifyFull,
                        other => {
                            return Err(IoError::Other(format!(
                                "MySQL.Database: unknown SslMode {other:?}"
                            )));
                        }
                    };
                }
                ("SslMode", Value::Null) => {}
                ("ConnectionTimeout", Value::Duration(d)) => {
                    connection_timeout_s = Some(d.num_seconds().max(0) as u64);
                }
                ("ConnectionTimeout", Value::Number(n)) => {
                    connection_timeout_s = Some(n.max(0.0) as u64);
                }
                ("ConnectionTimeout", Value::Null) => {}
                ("CommandTimeout", _) => {} // accepted, not forwarded
                ("Encoding", _) => {} // accepted, ignored (mysql crate is UTF-8)
                ("CreateNavigationProperties", _) => {} // ignored
                ("HierarchicalNavigation", _) => {} // ignored
                ("ReturnSingleDatabase", _) => {} // ignored
                _ => {} // unknown fields silently ignored, matching PQ tolerance
            }
        }
    }

    Ok(MySqlConn {
        host: host_part,
        port: port_from_opts.or(port_from_server).unwrap_or(3306),
        user,
        password,
        database: database.to_string(),
        ssl_mode,
        connection_timeout_s,
    })
}

#[cfg(feature = "mysql")]
fn mysql_open(conn: &MySqlConn) -> Result<mysql::Conn, IoError> {
    // First attempt honours `ssl_mode`. If that's `Preferred` and the
    // server doesn't advertise TLS, retry once without TLS — matching
    // PQ's "try secure, fall back to plain" semantics. Required /
    // VerifyCA / VerifyFull do *not* fall back; surface the error.
    let primary = mysql_open_inner(conn, /*ssl_attempt=*/ true);
    match primary {
        Ok(c) => Ok(c),
        Err(e) if conn.ssl_mode == SslMode::Preferred && is_server_lacks_tls(&e) => {
            mysql_open_inner(conn, /*ssl_attempt=*/ false).map_err(|e2| wrap_mysql_err(conn, &e2))
        }
        Err(e) => Err(wrap_mysql_err(conn, &e)),
    }
}

#[cfg(feature = "mysql")]
fn mysql_open_inner(conn: &MySqlConn, ssl_attempt: bool) -> Result<mysql::Conn, mysql::Error> {
    use mysql::{Conn, OptsBuilder, SslOpts};
    use std::time::Duration;

    let mut builder = OptsBuilder::new()
        .ip_or_hostname(Some(conn.host.as_str()))
        .tcp_port(conn.port)
        .db_name(Some(conn.database.as_str()))
        .user(conn.user.as_deref())
        .pass(conn.password.as_deref());

    if let Some(secs) = conn.connection_timeout_s {
        builder = builder.tcp_connect_timeout(Some(Duration::from_secs(secs)));
    }

    let ssl_opts = if !ssl_attempt {
        None
    } else {
        match conn.ssl_mode {
            SslMode::None => None,
            SslMode::Preferred | SslMode::Required => Some(SslOpts::default()),
            SslMode::VerifyCa => Some(SslOpts::default().with_danger_accept_invalid_certs(false)),
            SslMode::VerifyFull => Some(
                SslOpts::default()
                    .with_danger_skip_domain_validation(false)
                    .with_danger_accept_invalid_certs(false),
            ),
        }
    };
    if let Some(opts) = ssl_opts {
        builder = builder.ssl_opts(opts);
    }

    Conn::new(builder)
}

#[cfg(feature = "mysql")]
fn wrap_mysql_err(conn: &MySqlConn, e: &mysql::Error) -> IoError {
    IoError::Other(format!(
        "MySQL connect ({}:{} db={}): {e}",
        conn.host, conn.port, conn.database
    ))
}

/// Detect "server doesn't advertise the TLS extension". The mysql
/// crate surfaces this as a DriverError whose Display string contains
/// "does not have this capability". Matching on the string is more
/// stable than depending on a specific DriverError variant whose name
/// can change between crate versions.
#[cfg(feature = "mysql")]
fn is_server_lacks_tls(e: &mysql::Error) -> bool {
    e.to_string().contains("does not have this capability")
}

#[cfg(feature = "mysql")]
fn mysql_list_tables(conn: &MySqlConn) -> Result<Vec<String>, IoError> {
    use mysql::prelude::Queryable;

    let mut c = mysql_open(conn)?;
    let rows: Vec<String> = c
        .query(
            "SELECT table_name FROM information_schema.tables \
             WHERE table_schema = DATABASE() \
             ORDER BY table_name",
        )
        .map_err(|e| IoError::Other(format!("MySQL list tables: {e}")))?;
    Ok(rows)
}

#[cfg(feature = "mysql")]
fn mysql_query_value(conn: &MySqlConn, sql: &str) -> Result<Value, IoError> {
    use mysql::prelude::Queryable;

    let mut c = mysql_open(conn)?;
    // Use a prepared statement (binary protocol) — `query_iter` would
    // run the text protocol where every value comes back as ASCII
    // Bytes, including DATETIME / DATE / INT. Binary protocol gives
    // us typed values so the type mapper can produce M Date/Datetime
    // cells directly instead of strings.
    let stmt = c
        .prep(sql)
        .map_err(|e| IoError::Other(format!("MySQL prep: {e}")))?;
    let result = c
        .exec_iter(&stmt, ())
        .map_err(|e| IoError::Other(format!("MySQL exec: {e}")))?;

    let cols_ref = result.columns();
    let column_names: Vec<String> = cols_ref
        .as_ref()
        .iter()
        .map(|c| c.name_str().into_owned())
        .collect();

    let mut all_rows: Vec<Vec<Value>> = Vec::new();
    for row_res in result {
        let row = row_res.map_err(|e| IoError::Other(format!("MySQL fetch: {e}")))?;
        let mut cells: Vec<Value> = Vec::with_capacity(column_names.len());
        for col in 0..column_names.len() {
            let v: mysql::Value = row.as_ref(col).cloned().unwrap_or(mysql::Value::NULL);
            cells.push(mysql_value_to_m(v));
        }
        all_rows.push(cells);
    }
    Ok(Value::Table(Table::from_rows(column_names, all_rows)))
}

#[cfg(feature = "mysql")]
fn mysql_value_to_m(v: mysql::Value) -> Value {
    use chrono::{NaiveDate, NaiveDateTime, NaiveTime, Duration as ChronoDuration};
    use mysql::Value as MV;
    match v {
        MV::NULL => Value::Null,
        MV::Int(n) => Value::Number(n as f64),
        MV::UInt(n) => Value::Number(n as f64),
        MV::Float(n) => Value::Number(n as f64),
        MV::Double(n) => Value::Number(n),
        MV::Bytes(b) => match String::from_utf8(b) {
            // Most VARCHAR/TEXT cells come through as Bytes. Treat
            // valid UTF-8 as Text; only fall back to Binary for
            // genuinely non-textual blobs.
            Ok(s) => {
                // DECIMAL also lands as Bytes (always-ASCII numeric
                // string) — attempt to parse so DECIMAL(p,s) round-trips
                // as a Number. mrsflow has Value::Decimal too, but
                // without precision/scale metadata at the row level
                // we'd need to wire ColumnType information through;
                // f64 parse is the v1 compromise.
                if let Ok(n) = s.parse::<f64>() {
                    Value::Number(n)
                } else {
                    Value::Text(s)
                }
            }
            Err(e) => Value::Binary(e.into_bytes()),
        },
        MV::Date(year, month, day, hour, minute, second, micros) => {
            let date = NaiveDate::from_ymd_opt(year as i32, month as u32, day as u32);
            let time = NaiveTime::from_hms_micro_opt(
                hour as u32, minute as u32, second as u32, micros,
            );
            match (date, time) {
                (Some(d), Some(t)) => {
                    if hour == 0 && minute == 0 && second == 0 && micros == 0 {
                        Value::Date(d)
                    } else {
                        Value::Datetime(NaiveDateTime::new(d, t))
                    }
                }
                _ => Value::Null,
            }
        }
        MV::Time(negative, days, hours, minutes, seconds, micros) => {
            let total_secs = (days as i64) * 86400
                + (hours as i64) * 3600
                + (minutes as i64) * 60
                + (seconds as i64);
            let micros_part = micros as i64;
            let total_micros = total_secs * 1_000_000 + micros_part;
            let signed = if negative { -total_micros } else { total_micros };
            Value::Duration(ChronoDuration::microseconds(signed))
        }
    }
}

// ============================================================================
// PostgreSQL.Database — native Postgres protocol with pure-Rust TLS.
// Uses async tokio-postgres bridged to sync via a thread-local current_thread
// runtime; TLS via tokio-postgres-rustls. Same navigation-table shape as
// MySQL.Database and Odbc.DataSource. M-rich type mapping: NUMERIC →
// Value::Decimal, TIMESTAMPTZ → Datetimezone, UUID → Text, JSONB → parsed.
// ============================================================================

#[cfg(feature = "postgresql")]
struct PgConn {
    host: String,
    port: u16,
    user: Option<String>,
    password: Option<String>,
    database: String,
    ssl_mode: PgSslMode,
    connection_timeout_s: Option<u64>,
}

#[cfg(feature = "postgresql")]
#[derive(Clone, Copy, PartialEq, Eq)]
enum PgSslMode {
    None,
    Preferred,
    Required,
    VerifyCa,
    VerifyFull,
}

#[cfg(feature = "postgresql")]
impl Clone for PgConn {
    fn clone(&self) -> Self {
        Self {
            host: self.host.clone(),
            port: self.port,
            user: self.user.clone(),
            password: self.password.clone(),
            database: self.database.clone(),
            ssl_mode: self.ssl_mode,
            connection_timeout_s: self.connection_timeout_s,
        }
    }
}

#[cfg(feature = "postgresql")]
fn postgres_database_impl(
    server: &str,
    database: &str,
    options: Option<&Value>,
) -> Result<Value, IoError> {
    use std::cell::RefCell;
    use std::rc::Rc;

    use mrsflow_core::eval::{MError, ThunkState};

    let conn = parse_pg_conn(server, database, options)?;

    // List tables now so the navigation table is materialised eagerly.
    // Data thunks reopen on force — same pattern as the ODBC and MySQL
    // paths.
    let tables = pg_list_tables(&conn)?;

    let cols = vec![
        "Name".to_string(),
        "Data".to_string(),
        "Schema".to_string(),
        "ItemKind".to_string(),
        "ItemName".to_string(),
        "IsLeaf".to_string(),
    ];
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(tables.len());
    for (schema, name) in tables {
        let conn_for_thunk = conn.clone();
        let schema_for_thunk = schema.clone();
        let name_for_thunk = name.clone();
        let fetcher: Rc<dyn Fn() -> Result<Value, MError>> = Rc::new(move || {
            // Quote identifiers — double-quote with embedded "" escape,
            // per Postgres SQL syntax.
            let sql = format!(
                "SELECT * FROM \"{}\".\"{}\"",
                schema_for_thunk.replace('"', "\"\""),
                name_for_thunk.replace('"', "\"\""),
            );
            pg_query_value(&conn_for_thunk, &sql)
                .map_err(|e| MError::Other(format!("PostgreSQL fetch: {e:?}")))
        });
        let data = Value::Thunk(Rc::new(RefCell::new(ThunkState::Native(fetcher))));
        rows.push(vec![
            Value::Text(name.clone()),
            data,
            Value::Text(schema),
            Value::Text("Table".to_string()),
            Value::Text(name),
            Value::Logical(true),
        ]);
    }
    Ok(Value::Table(Table::from_rows(cols, rows)))
}

#[cfg(feature = "postgresql")]
fn parse_pg_conn(
    server: &str,
    database: &str,
    options: Option<&Value>,
) -> Result<PgConn, IoError> {
    let (host_part, port_from_server) = match server.rsplit_once(':') {
        Some((h, p)) => match p.parse::<u16>() {
            Ok(n) => (h.to_string(), Some(n)),
            Err(_) => (server.to_string(), None),
        },
        None => (server.to_string(), None),
    };

    let mut user: Option<String> = None;
    let mut password: Option<String> = None;
    let mut port_from_opts: Option<u16> = None;
    let mut ssl_mode = PgSslMode::Preferred;
    let mut connection_timeout_s: Option<u64> = None;

    if let Some(Value::Record(r)) = options {
        for (k, v) in &r.fields {
            match (k.as_str(), v) {
                ("UserName", Value::Text(s)) => user = Some(s.clone()),
                ("Password", Value::Text(s)) => password = Some(s.clone()),
                ("Port", Value::Number(n)) if n.is_finite() && *n >= 0.0 && *n <= 65535.0 => {
                    port_from_opts = Some(*n as u16);
                }
                ("SslMode", Value::Text(s)) => {
                    ssl_mode = match s.as_str() {
                        "None" => PgSslMode::None,
                        "Preferred" => PgSslMode::Preferred,
                        "Required" => PgSslMode::Required,
                        "VerifyCA" => PgSslMode::VerifyCa,
                        "VerifyFull" => PgSslMode::VerifyFull,
                        other => {
                            return Err(IoError::Other(format!(
                                "PostgreSQL.Database: unknown SslMode {other:?}"
                            )));
                        }
                    };
                }
                ("ConnectionTimeout", Value::Duration(d)) => {
                    connection_timeout_s = Some(d.num_seconds().max(0) as u64);
                }
                ("ConnectionTimeout", Value::Number(n)) => {
                    connection_timeout_s = Some(n.max(0.0) as u64);
                }
                ("CommandTimeout", _) => {}
                ("Encoding", _) => {}
                ("CreateNavigationProperties", _) => {}
                ("HierarchicalNavigation", _) => {}
                _ => {}
            }
        }
    }

    Ok(PgConn {
        host: host_part,
        port: port_from_opts.or(port_from_server).unwrap_or(5432),
        user,
        password,
        database: database.to_string(),
        ssl_mode,
        connection_timeout_s,
    })
}

#[cfg(feature = "postgresql")]
fn pg_runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("tokio current_thread runtime")
}

#[cfg(feature = "postgresql")]
fn pg_rustls_config() -> std::sync::Arc<rustls::ClientConfig> {
    // Build a rustls ClientConfig with Mozilla's root CA bundle baked
    // in via webpki-roots. No reliance on /etc/ssl/certs at runtime.
    let mut roots = rustls::RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
    let cfg = rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth();
    std::sync::Arc::new(cfg)
}

#[cfg(feature = "postgresql")]
async fn pg_connect(conn: &PgConn, ssl_attempt: bool) -> Result<tokio_postgres::Client, tokio_postgres::Error> {
    use std::time::Duration;
    use tokio_postgres::config::SslMode as PgCrateSslMode;
    use tokio_postgres::Config;

    let mut cfg = Config::new();
    cfg.host(&conn.host).port(conn.port).dbname(&conn.database);
    if let Some(u) = &conn.user {
        cfg.user(u);
    }
    if let Some(p) = &conn.password {
        cfg.password(p);
    }
    if let Some(secs) = conn.connection_timeout_s {
        cfg.connect_timeout(Duration::from_secs(secs));
    }

    // tokio-postgres 0.7 in this minor only exposes Disable / Prefer /
    // Require — VerifyCA / VerifyFull map to Require here; rustls's
    // own cert-chain verification (webpki-roots) provides the equivalent
    // guarantee at the TLS layer regardless of which mode PG is asked.
    let pg_ssl = if !ssl_attempt {
        PgCrateSslMode::Disable
    } else {
        match conn.ssl_mode {
            PgSslMode::None => PgCrateSslMode::Disable,
            PgSslMode::Preferred
            | PgSslMode::Required
            | PgSslMode::VerifyCa
            | PgSslMode::VerifyFull => PgCrateSslMode::Require,
        }
    };
    cfg.ssl_mode(pg_ssl);

    if pg_ssl == PgCrateSslMode::Disable {
        // Plain-TCP path. The Socket type-parameter forces NoTls.
        let (client, connection) = cfg.connect(tokio_postgres::NoTls).await?;
        // Spawn the connection-driver task so the protocol pump runs.
        tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(client)
    } else {
        let tls = tokio_postgres_rustls::MakeRustlsConnect::new(pg_rustls_config().as_ref().clone());
        let (client, connection) = cfg.connect(tls).await?;
        tokio::spawn(async move {
            let _ = connection.await;
        });
        Ok(client)
    }
}

#[cfg(feature = "postgresql")]
fn pg_open(conn: &PgConn) -> Result<(tokio::runtime::Runtime, tokio_postgres::Client), IoError> {
    let rt = pg_runtime();
    let primary = rt.block_on(pg_connect(conn, /*ssl_attempt=*/ true));
    let client = match primary {
        Ok(c) => c,
        Err(e) if conn.ssl_mode == PgSslMode::Preferred && pg_is_server_lacks_tls(&e) => {
            rt.block_on(pg_connect(conn, /*ssl_attempt=*/ false))
                .map_err(|e2| wrap_pg_err(conn, &e2))?
        }
        Err(e) => return Err(wrap_pg_err(conn, &e)),
    };
    Ok((rt, client))
}

#[cfg(feature = "postgresql")]
fn wrap_pg_err(conn: &PgConn, e: &tokio_postgres::Error) -> IoError {
    IoError::Other(format!(
        "PostgreSQL connect ({}:{} db={}): {}",
        conn.host, conn.port, conn.database, pg_err_chain(e),
    ))
}

/// tokio-postgres's Display often gives terse strings like "db error"
/// without the SQLSTATE / message text — the actual content lives in
/// the error chain via `.source()`. Walk it and join the messages so
/// users see what the server actually said.
#[cfg(feature = "postgresql")]
fn pg_err_chain(e: &tokio_postgres::Error) -> String {
    use std::error::Error;
    let mut parts: Vec<String> = vec![e.to_string()];
    let mut cur: Option<&(dyn Error + 'static)> = e.source();
    while let Some(src) = cur {
        parts.push(src.to_string());
        cur = src.source();
    }
    parts.join(": ")
}

/// True when the error indicates the server doesn't support SSL and
/// we asked for it (so Preferred should retry plain). tokio-postgres
/// surfaces this distinctively in its Display output. Substring match
/// is robust across point releases.
#[cfg(feature = "postgresql")]
fn pg_is_server_lacks_tls(e: &tokio_postgres::Error) -> bool {
    let s = e.to_string().to_lowercase();
    s.contains("server does not support tls") || s.contains("no support for ssl")
}

#[cfg(feature = "postgresql")]
fn pg_list_tables(conn: &PgConn) -> Result<Vec<(String, String)>, IoError> {
    let (rt, client) = pg_open(conn)?;
    let sql = "SELECT schemaname, tablename FROM pg_catalog.pg_tables \
               WHERE schemaname NOT IN ('pg_catalog', 'information_schema') \
               ORDER BY schemaname, tablename";
    let rows = rt
        .block_on(client.query(sql, &[]))
        .map_err(|e| IoError::Other(format!("PostgreSQL list tables: {}", pg_err_chain(&e))))?;
    Ok(rows
        .into_iter()
        .map(|r| (r.get::<_, String>(0), r.get::<_, String>(1)))
        .collect())
}

#[cfg(feature = "postgresql")]
fn pg_query_value(conn: &PgConn, sql: &str) -> Result<Value, IoError> {
    let (rt, client) = pg_open(conn)?;
    let stmt = rt
        .block_on(client.prepare(sql))
        .map_err(|e| IoError::Other(format!("PostgreSQL prep: {}", pg_err_chain(&e))))?;
    let column_names: Vec<String> = stmt
        .columns()
        .iter()
        .map(|c| c.name().to_string())
        .collect();
    let column_oids: Vec<u32> = stmt.columns().iter().map(|c| c.type_().oid()).collect();

    let rows = rt
        .block_on(client.query(&stmt, &[]))
        .map_err(|e| IoError::Other(format!("PostgreSQL exec: {}", pg_err_chain(&e))))?;

    let mut all_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut cells: Vec<Value> = Vec::with_capacity(column_names.len());
        for col in 0..column_names.len() {
            cells.push(pg_cell_to_value(&row, col, column_oids[col]));
        }
        all_rows.push(cells);
    }
    Ok(Value::Table(Table::from_rows(column_names, all_rows)))
}

#[cfg(feature = "postgresql")]
fn pg_cell_to_value(row: &tokio_postgres::Row, col: usize, oid: u32) -> Value {
    use chrono::{DateTime, FixedOffset, NaiveDate, NaiveDateTime, NaiveTime};

    // Postgres OID constants (from pg_type.h). Hard-coded to avoid
    // pulling postgres-types' Type imports — these are stable for the
    // lifetime of the protocol.
    const BOOL: u32 = 16;
    const BYTEA: u32 = 17;
    const CHAR: u32 = 18;
    const INT8: u32 = 20;
    const INT2: u32 = 21;
    const INT4: u32 = 23;
    const TEXT: u32 = 25;
    const JSON: u32 = 114;
    const FLOAT4: u32 = 700;
    const FLOAT8: u32 = 701;
    const BPCHAR: u32 = 1042;
    const VARCHAR: u32 = 1043;
    const DATE: u32 = 1082;
    const TIME: u32 = 1083;
    const TIMESTAMP: u32 = 1114;
    const TIMESTAMPTZ: u32 = 1184;
    const NUMERIC: u32 = 1700;
    const UUID: u32 = 2950;
    const JSONB: u32 = 3802;
    // Array OIDs — see pg_type.h.
    const BOOL_ARR: u32 = 1000;
    const INT2_ARR: u32 = 1005;
    const INT4_ARR: u32 = 1007;
    const TEXT_ARR: u32 = 1009;
    const VARCHAR_ARR: u32 = 1015;
    const INT8_ARR: u32 = 1016;
    const FLOAT4_ARR: u32 = 1021;
    const FLOAT8_ARR: u32 = 1022;
    const DATE_ARR: u32 = 1182;
    const TIMESTAMP_ARR: u32 = 1115;
    const TIMESTAMPTZ_ARR: u32 = 1185;
    const NUMERIC_ARR: u32 = 1231;
    const UUID_ARR: u32 = 2951;
    const JSONB_ARR: u32 = 3807;

    macro_rules! get_opt {
        ($t:ty) => {
            match row.try_get::<_, Option<$t>>(col) {
                Ok(Some(v)) => v,
                Ok(None) => return Value::Null,
                Err(e) => return Value::Text(format!("<decode error: {e}>")),
            }
        };
    }

    match oid {
        BOOL => Value::Logical(get_opt!(bool)),
        INT2 => Value::Number(get_opt!(i16) as f64),
        INT4 => Value::Number(get_opt!(i32) as f64),
        INT8 => Value::Number(get_opt!(i64) as f64),
        FLOAT4 => Value::Number(get_opt!(f32) as f64),
        FLOAT8 => Value::Number(get_opt!(f64)),
        TEXT | VARCHAR | BPCHAR | CHAR => Value::Text(get_opt!(String)),
        BYTEA => Value::Binary(get_opt!(Vec<u8>)),
        UUID => {
            let u: uuid::Uuid = get_opt!(uuid::Uuid);
            Value::Text(u.to_string())
        }
        DATE => Value::Date(get_opt!(NaiveDate)),
        TIME => {
            let t: NaiveTime = get_opt!(NaiveTime);
            Value::Time(t)
        }
        TIMESTAMP => Value::Datetime(get_opt!(NaiveDateTime)),
        TIMESTAMPTZ => {
            let dt: DateTime<FixedOffset> = match row.try_get::<_, Option<DateTime<chrono::Utc>>>(col) {
                Ok(Some(v)) => v.with_timezone(&FixedOffset::east_opt(0).unwrap()),
                Ok(None) => return Value::Null,
                Err(e) => return Value::Text(format!("<decode error: {e}>")),
            };
            Value::Datetimezone(dt)
        }
        NUMERIC => {
            // tokio-postgres doesn't ship a built-in FromSql for NUMERIC
            // (it'd need decimal-rs or rust_decimal). v1 reads it via
            // the f64 path: tokio-postgres returns binary NUMERIC and
            // we can't easily ask for text without rewriting the SELECT,
            // so we fall back to the unknown-OID branch where try_get::<String>
            // also fails. For now: try f64-via-text by issuing a one-off
            // raw read; if that doesn't work, surface a clear message.
            match row.try_get::<_, Option<&str>>(col) {
                Ok(Some(s)) => match s.parse::<f64>() {
                    Ok(n) => Value::Number(n),
                    Err(_) => Value::Text(s.to_string()),
                },
                Ok(None) => Value::Null,
                Err(_) => Value::Text("<NUMERIC: decoder needs rust_decimal feature>".to_string()),
            }
        }
        JSON | JSONB => {
            // tokio-postgres with `with-serde_json-1` decodes JSON/JSONB
            // into serde_json::Value directly. We then translate to a
            // mrsflow record/list/scalar — same shape as Json.Document.
            let j: serde_json::Value = get_opt!(serde_json::Value);
            json_value_to_m(j)
        }
        BOOL_ARR => pg_array_to_list::<bool>(row, col, |b| Value::Logical(b)),
        INT2_ARR => pg_array_to_list::<i16>(row, col, |n| Value::Number(n as f64)),
        INT4_ARR => pg_array_to_list::<i32>(row, col, |n| Value::Number(n as f64)),
        INT8_ARR => pg_array_to_list::<i64>(row, col, |n| Value::Number(n as f64)),
        FLOAT4_ARR => pg_array_to_list::<f32>(row, col, |n| Value::Number(n as f64)),
        FLOAT8_ARR => pg_array_to_list::<f64>(row, col, Value::Number),
        TEXT_ARR | VARCHAR_ARR => pg_array_to_list::<String>(row, col, Value::Text),
        DATE_ARR => pg_array_to_list::<chrono::NaiveDate>(row, col, Value::Date),
        TIMESTAMP_ARR => {
            pg_array_to_list::<chrono::NaiveDateTime>(row, col, Value::Datetime)
        }
        TIMESTAMPTZ_ARR => pg_array_to_list::<chrono::DateTime<chrono::Utc>>(row, col, |dt| {
            Value::Datetimezone(dt.with_timezone(&chrono::FixedOffset::east_opt(0).unwrap()))
        }),
        UUID_ARR => pg_array_to_list::<uuid::Uuid>(row, col, |u| Value::Text(u.to_string())),
        JSONB_ARR => pg_array_to_list::<serde_json::Value>(row, col, json_value_to_m),
        NUMERIC_ARR => {
            // No FromSql for numeric without rust_decimal; surface as a
            // clear placeholder rather than a silent error.
            Value::Text("<NUMERIC[]: decoder needs rust_decimal feature>".to_string())
        }
        _ => {
            // Unknown OID — fall back to PG's text representation.
            match row.try_get::<_, Option<String>>(col) {
                Ok(Some(s)) => Value::Text(s),
                Ok(None) => Value::Null,
                Err(e) => Value::Text(format!("<unmapped OID {oid}: {e}>")),
            }
        }
    }
}

/// Decode a PG array column into Value::List. Null array → Value::Null;
/// element nulls → Value::Null entries.
#[cfg(feature = "postgresql")]
fn pg_array_to_list<T>(
    row: &tokio_postgres::Row,
    col: usize,
    f: impl Fn(T) -> Value,
) -> Value
where
    T: for<'a> tokio_postgres::types::FromSql<'a>,
{
    match row.try_get::<_, Option<Vec<Option<T>>>>(col) {
        Ok(Some(xs)) => Value::List(xs.into_iter().map(|opt| opt.map(&f).unwrap_or(Value::Null)).collect()),
        Ok(None) => Value::Null,
        Err(e) => Value::Text(format!("<array decode error: {e}>")),
    }
}

#[cfg(feature = "postgresql")]
fn json_value_to_m(j: serde_json::Value) -> Value {
    use mrsflow_core::eval::Record;
    match j {
        serde_json::Value::Null => Value::Null,
        serde_json::Value::Bool(b) => Value::Logical(b),
        serde_json::Value::Number(n) => {
            let f = n.as_f64().unwrap_or(f64::NAN);
            Value::Number(f)
        }
        serde_json::Value::String(s) => Value::Text(s),
        serde_json::Value::Array(xs) => {
            Value::List(xs.into_iter().map(json_value_to_m).collect())
        }
        serde_json::Value::Object(map) => {
            let fields = map
                .into_iter()
                .map(|(k, v)| (k, json_value_to_m(v)))
                .collect();
            Value::Record(Record {
                fields,
                env: mrsflow_core::eval::EnvNode::empty(),
            })
        }
    }
}

