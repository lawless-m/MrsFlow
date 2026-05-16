//! `Sql.*` — native SQL Server connector.
//!
//! Delegates to `IoHost::sql_database` / `sql_databases`. The CLI shell
//! implements them via the `tiberius` crate (async TDS over `tokio`,
//! `rustls` TLS); other shells return `NotSupported`. Returns a
//! navigation table with the same shape `MySQL.Database` and
//! `Odbc.DataSource` produce, so `Sql.Databases(s){[Name=db]}[Data]{[Schema=s,Item=t]}[Data]`
//! works the same way as in Power Query.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::expect_text;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        (
            "Sql.Database",
            vec![
                Param { name: "server".into(),   optional: false, type_annotation: None },
                Param { name: "database".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),  optional: true,  type_annotation: None },
            ],
            database,
        ),
        (
            "Sql.Databases",
            vec![
                Param { name: "server".into(),  optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            databases,
        ),
    ]
}

fn database(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let server = expect_text(&args[0])?;
    let database_name = expect_text(&args[1])?;
    let forced_opts: Option<Value> = match args.get(2) {
        None | Some(Value::Null) => None,
        Some(v) => Some(super::super::deep_force(v.clone(), host)?),
    };
    host.sql_database(server, database_name, forced_opts.as_ref())
        .map_err(|e| MError::Other(format!("Sql.Database: {e:?}")))
}

fn databases(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let server = expect_text(&args[0])?;
    let forced_opts: Option<Value> = match args.get(1) {
        None | Some(Value::Null) => None,
        Some(v) => Some(super::super::deep_force(v.clone(), host)?),
    };
    host.sql_databases(server, forced_opts.as_ref())
        .map_err(|e| MError::Other(format!("Sql.Databases: {e:?}")))
}
