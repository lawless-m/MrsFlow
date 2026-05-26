//! `Exportmaster.*` stdlib bindings.
//!
//! Native DBISAM client (no ODBC). Implementation lives in
//! `mrsflow-cli/src/exportmaster/` behind the `exportmaster` Cargo
//! feature. This module is the M-side surface that calls into the
//! IoHost trait, which routes to the live DBISAM TCP transport.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::env::EnvNode;
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Value};
use super::common::{expect_text, one, two};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        // Exportmaster.Query(host, sql, [opts]) → table
        (
            "Exportmaster.Query",
            vec![
                Param { name: "host".into(),     optional: false, type_annotation: None },
                Param { name: "sql".into(),      optional: false, type_annotation: None },
                Param { name: "options".into(),  optional: true,  type_annotation: None },
            ],
            query,
        ),
        // Exportmaster.Database(host, [opts]) → navigation record
        (
            "Exportmaster.Database",
            vec![
                Param { name: "host".into(),     optional: false, type_annotation: None },
                Param { name: "options".into(),  optional: true,  type_annotation: None },
            ],
            database,
        ),
    ]
}

fn query(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let host_str = expect_text(&args[0])?;
    let sql = expect_text(&args[1])?;
    let forced_opt = match args.get(2) {
        Some(v) => Some(super::super::deep_force(v.clone(), host)?),
        None => None,
    };
    host.exportmaster_query(host_str, sql, forced_opt.as_ref())
        .map_err(|e| MError::Other(format!("Exportmaster.Query: {e:?}")))
}

fn database(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let host_str = expect_text(&args[0])?;
    let forced_opt = match args.get(1) {
        Some(v) => Some(super::super::deep_force(v.clone(), host)?),
        None => None,
    };
    host.exportmaster_database(host_str, forced_opt.as_ref())
        .map_err(|e| MError::Other(format!("Exportmaster.Database: {e:?}")))
}
