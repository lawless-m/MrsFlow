//! `PostgreSQL.Database` — native PostgreSQL connector.
//!
//! Delegates to `IoHost::postgres_database`. The CLI shell implements it
//! via `tokio-postgres` + `tokio-postgres-rustls` (pure-Rust TLS) behind
//! the `postgresql` cargo feature; other shells return NotSupported.
//! Returns a navigation table identical in shape to what Odbc.DataSource
//! and MySQL.Database produce, so the
//! `PostgreSQL.Database(s, db){[Name="t"]}[Data]` chain works the same.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::expect_text;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![(
        "PostgreSQL.Database",
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
    let forced_opts: Option<Value> = match args.get(2) {
        None | Some(Value::Null) => None,
        Some(v) => Some(super::super::deep_force(v.clone(), host)?),
    };
    host.postgres_database(server, database_name, forced_opts.as_ref())
        .map_err(|e| MError::Other(format!("PostgreSQL.Database: {e:?}")))
}
