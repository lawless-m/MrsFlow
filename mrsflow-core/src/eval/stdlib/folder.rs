//! `Folder.Contents` (immediate children) and `Folder.Files` (recursive,
//! files only). Both return tables with columns
//! `Content, Name, Extension, Date accessed, Date modified,
//!  Date created, Attributes, Folder Path`.
//!
//! v1 eagerly reads file Content into Binary. Filtering on Name/Extension/
//! Attributes before touching Content still walks every byte — cost
//! scales with directory size, not the filter selectivity. Acceptable for
//! the corpus's directories; switch to lazy reads when it bites.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{expect_text, one};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Folder.Contents", one("path"), contents),
        ("Folder.Files",    one("path"), files),
    ]
}

fn contents(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let path = expect_text(&args[0])?;
    host.folder_contents(path)
        .map_err(|e| MError::Other(format!("Folder.Contents({path:?}): {e:?}")))
}

fn files(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let path = expect_text(&args[0])?;
    host.folder_files(path)
        .map_err(|e| MError::Other(format!("Folder.Files({path:?}): {e:?}")))
}
