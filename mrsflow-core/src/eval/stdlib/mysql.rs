//! `MySQL.Database` — native MySQL connector.
//!
//! Delegates to `IoHost::mysql_database`. The CLI shell implements it
//! via the `mysql` crate (sync, rustls TLS); other shells return
//! NotSupported. Returns a navigation table with the same shape
//! `Odbc.DataSource` produces, so `MySQL.Database(s, db){[Name="t"]}[Data]`
//! works the same way as in Power Query.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::expect_text;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![(
        "MySQL.Database",
        vec![
            Param { name: "server".into(),   optional: false, type_annotation: None },
            Param { name: "database".into(), optional: false, type_annotation: None },
            Param { name: "options".into(),  optional: true,  type_annotation: None },
        ],
        database,
    )]
}

fn database(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let server = expect_text(&args[0])?;
    let database_name = expect_text(&args[1])?;
    // Deep-force the options record so the host gets fully-forced primitive
    // fields without needing to thread an evaluator callback through the
    // trait. Null and absent are equivalent here — no options.
    let forced_opts: Option<Value> = match args.get(2) {
        None | Some(Value::Null) => None,
        Some(v) => Some(super::super::deep_force(v.clone(), host)?),
    };
    host.mysql_database(server, database_name, forced_opts.as_ref())
        .map_err(|e| MError::Other(format!("MySQL.Database: {e:?}")))
}
