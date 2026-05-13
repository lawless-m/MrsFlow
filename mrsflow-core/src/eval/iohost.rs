//! IO host trait — the boundary between the pure evaluator and the shell's
//! capabilities.
//!
//! See `mrsflow/07-evaluator-design.md` §"The IoHost trait" for the per-shell
//! capability table. Concrete implementations live in shell crates
//! (`mrsflow-cli`, `mrsflow-wasm`) once those exist; for now `NoIoHost`
//! provides a stub that always errors with `IoError::NotSupported`, useful
//! for evaluator unit tests that never need IO.

use super::value::Value;

#[derive(Debug, Clone)]
pub enum IoError {
    /// The shell doesn't implement this method (e.g. WASM has no ODBC).
    NotSupported,
    /// Generic; specific variants added when shell impls surface them.
    Other(String),
}

pub trait IoHost {
    fn parquet_read(&self, path: &str) -> Result<Value, IoError>;
    fn parquet_write(&self, path: &str, value: &Value) -> Result<(), IoError>;
    fn odbc_query(
        &self,
        connection_string: &str,
        sql: &str,
        options: Option<&Value>,
    ) -> Result<Value, IoError>;
    fn odbc_data_source(
        &self,
        connection_string: &str,
        options: Option<&Value>,
    ) -> Result<Value, IoError>;
    /// Read a file's bytes verbatim — backs `File.Contents`.
    fn file_read(&self, path: &str) -> Result<Vec<u8>, IoError>;
    /// Last-modified timestamp as a fixed-offset datetime — backs `File.Modified`.
    fn file_modified(
        &self,
        path: &str,
    ) -> Result<chrono::DateTime<chrono::FixedOffset>, IoError>;
    /// Parse XLSX/XLS bytes into M's Excel.Workbook shape — a Table with
    /// columns `Name, Data, Item, Kind, Hidden`. Kinds: `"Sheet"`,
    /// `"Table"` (Excel ListObjects, xlsx only), `"DefinedName"`. With
    /// `use_headers=true`, each Data table's first row becomes column
    /// names. With `delay_types=false`, DateTime cells are decoded to
    /// `Value::Datetime` instead of staying as Excel-serial floats.
    fn excel_workbook(
        &self,
        bytes: &[u8],
        use_headers: bool,
        delay_types: bool,
    ) -> Result<Value, IoError>;
    /// Fetch `url` with the supplied request headers; return the response
    /// body. If `content` is `Some`, the request is POST with that body
    /// (matches M's `Web.Contents(_, [Content=<binary>])` semantics);
    /// otherwise it's GET. Status codes in 2xx return Ok; codes in
    /// `manual_status` also return Ok (caller chose to handle them); any
    /// other status returns Err. Auth providers / timeout config aren't
    /// supported in v1; the caller sets Content-Type via Headers if needed.
    fn web_contents(
        &self,
        url: &str,
        headers: &[(String, String)],
        manual_status: &[u16],
        content: Option<&[u8]>,
    ) -> Result<Vec<u8>, IoError>;
    /// HEAD `url` and return the response headers as a list of
    /// (name, value) pairs. Backs `Web.Headers`. Hosts that can't
    /// perform HEAD requests (or have no HTTP at all) return
    /// `IoError::NotSupported`. Default impl delegates to a GET via
    /// `web_contents` and ignores the body — concrete shells should
    /// override with a real HEAD if they care about the bandwidth.
    fn web_headers(
        &self,
        url: &str,
        headers: &[(String, String)],
    ) -> Result<Vec<(String, String)>, IoError> {
        let _ = (url, headers);
        Err(IoError::NotSupported)
    }
    /// Immediate directory contents — backs `Folder.Contents`. Returns a
    /// Table with columns `Content, Name, Extension, Date accessed,
    /// Date modified, Date created, Attributes, Folder Path`. Folder
    /// entries have `Content = null` and `Attributes.Kind = "Folder"`.
    fn folder_contents(&self, path: &str) -> Result<Value, IoError>;
    /// Recursive file walk — backs `Folder.Files`. Same column shape as
    /// `folder_contents` but folders are descended-into and not emitted
    /// as rows.
    fn folder_files(&self, path: &str) -> Result<Value, IoError>;
    /// Connect to PostgreSQL and return a navigation table — backs
    /// `PostgreSQL.Database`. Same shape as `mysql_database` /
    /// `Odbc.DataSource`. Default impl returns `IoError::NotSupported`
    /// so non-CLI shells (and CLI builds without `--features postgresql`)
    /// surface a clear error.
    fn postgres_database(
        &self,
        server: &str,
        database: &str,
        options: Option<&Value>,
    ) -> Result<Value, IoError> {
        let _ = (server, database, options);
        Err(IoError::NotSupported)
    }
    /// Connect to MySQL and return a navigation table — backs `MySQL.Database`.
    /// `options` is the optional record passed as the third arg (UserName,
    /// Password, Port, SslMode, ConnectionTimeout, CommandTimeout, etc.).
    /// Returns a navigation table with columns `Name, Data, ItemKind,
    /// ItemName, IsLeaf` — same shape as `Odbc.DataSource` produces.
    /// Hosts with no MySQL support (WASM, default builds) return
    /// `IoError::NotSupported`.
    fn mysql_database(
        &self,
        server: &str,
        database: &str,
        options: Option<&Value>,
    ) -> Result<Value, IoError> {
        let _ = (server, database, options);
        Err(IoError::NotSupported)
    }
    /// Caller-injected workbook parameters — backs `Excel.CurrentWorkbook`.
    /// Returns a Table with columns `Name, Content` where each Content
    /// cell is a 1-row Table `[Value=…]`. The shell (`mrsflow-cli`) builds
    /// this from `--param NAME=VALUE` flags. Hosts with no parameter
    /// mechanism return `IoError::NotSupported` so `try … otherwise …`
    /// catches in queries that depend on it.
    fn current_workbook(&self) -> Result<Value, IoError>;
}

/// Always-fail IoHost — for evaluator unit tests on pure expressions where
/// any IO call would be a test bug, and as a conservative default.
pub struct NoIoHost;

impl IoHost for NoIoHost {
    fn parquet_read(&self, _: &str) -> Result<Value, IoError> {
        Err(IoError::NotSupported)
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
