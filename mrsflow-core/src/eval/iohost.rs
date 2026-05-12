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
}
