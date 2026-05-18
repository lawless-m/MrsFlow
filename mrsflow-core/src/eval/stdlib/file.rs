//! `File.*` stdlib bindings: `File.Contents` and `File.Modified`.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{expect_text, one};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        (
            "File.Contents",
            vec![
                Param { name: "path".into(),    optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            contents,
        ),
        ("File.Modified", one("path"), modified),
    ]
}

fn contents(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let path = expect_text(&args[0])?;
    require_absolute_path(path)?;
    // `options` (args.get(1)) is accepted-and-ignored: M's options record
    // (Query, ApiKeyName, IsRetry, …) targets Web.Contents-style sources;
    // none of its fields are meaningful for a local filesystem read.
    let bytes = host
        .file_read(path)
        .map_err(|e| MError::Other(format!("File.Contents({path:?}): {e:?}")))?;
    Ok(Value::Binary(bytes))
}

fn modified(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let path = expect_text(&args[0])?;
    require_absolute_path(path)?;
    let ts = host
        .file_modified(path)
        .map_err(|e| MError::Other(format!("File.Modified({path:?}): {e:?}")))?;
    Ok(Value::Datetimezone(ts))
}

/// PQ rejects relative paths with a fixed error message before any I/O.
fn require_absolute_path(path: &str) -> Result<(), MError> {
    let is_absolute = std::path::Path::new(path).is_absolute()
        // Treat http(s) URLs as already-qualified — Excel doesn't, but
        // File.Contents has no business with them and the caller should
        // reach a real read failure rather than this gate.
        || path.starts_with("http://")
        || path.starts_with("https://");
    if is_absolute {
        Ok(())
    } else {
        Err(MError::Other(
            "The supplied file path must be a valid absolute path.".into(),
        ))
    }
}
