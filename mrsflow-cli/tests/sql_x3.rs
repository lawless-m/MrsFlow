//! Integration tests for `Sql.Database` / `Sql.Databases` against the
//! Ramsden Sage X3 SQL Server.
//!
//! Gated three ways:
//!   1. Compile-time: only enabled with `--features sql`.
//!   2. Runtime: requires `MRSFLOW_X3_TEST=1` to opt in (the test infra
//!      shouldn't reach out to the corporate network on a normal `cargo test`).
//!   3. Credentials: read from `/mnt/RIVSPROD02_RI_SERVICES/Credentials/X3SQL.json`.
//!      If the file isn't accessible the tests skip with a printed reason.
//!
//! Run locally with:
//!     MRSFLOW_X3_TEST=1 cargo test -p mrsflow-cli --features sql --test sql_x3 -- --nocapture

#![cfg(feature = "sql")]

use mrsflow_cli::CliIoHost;
use mrsflow_core::eval::{EnvNode, IoHost, Record, TableRepr, Value};

const CRED_PATH: &str = "/mnt/RIVSPROD02_RI_SERVICES/Credentials/X3SQL.json";
const X3_HOST: &str = "10.80.42.21";

fn load_creds() -> Option<Value> {
    if std::env::var("MRSFLOW_X3_TEST").ok().as_deref() != Some("1") {
        eprintln!("skipping: set MRSFLOW_X3_TEST=1 to run X3 tests");
        return None;
    }
    let raw = match std::fs::read_to_string(CRED_PATH) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("skipping: cannot read {CRED_PATH}: {e}");
            return None;
        }
    };
    let user = extract(&raw, "username")?;
    let pass = extract(&raw, "password")?;
    Some(Value::Record(Record {
        fields: vec![
            ("UserName".to_string(), Value::Text(user)),
            ("Password".to_string(), Value::Text(pass)),
        ],
        env: EnvNode::empty(),
    }))
}

fn extract(raw: &str, key: &str) -> Option<String> {
    let pat = format!("\"{key}\"");
    let i = raw.find(&pat)?;
    let after = &raw[i + pat.len()..];
    let colon = after.find(':')?;
    let after = &after[colon + 1..];
    let q1 = after.find('"')?;
    let after = &after[q1 + 1..];
    let q2 = after.find('"')?;
    Some(after[..q2].to_string())
}

#[test]
fn sql_databases_lists_sagex3() {
    let Some(opts) = load_creds() else { return };
    let host = CliIoHost::new();
    let res = host.sql_databases(X3_HOST, Some(&opts));
    let v = res.expect("Sql.Databases against X3 must succeed");
    let Value::Table(t) = v else { panic!("expected Table, got {v:?}") };
    let TableRepr::Rows { rows, .. } = &t.repr else {
        panic!("expected TableRepr::Rows, got {:?}", t.column_names());
    };
    let names: Vec<String> = rows
        .iter()
        .filter_map(|r| match &r[0] {
            Value::Text(s) => Some(s.clone()),
            _ => None,
        })
        .collect();
    eprintln!("databases on {X3_HOST}: {names:?}");
    assert!(
        names.iter().any(|n| n.eq_ignore_ascii_case("sagex3") || n.eq_ignore_ascii_case("x3live")),
        "expected sagex3 or x3live in {names:?}",
    );
}

#[test]
fn sql_database_navigates_to_ramlive_salesrep() {
    let Some(opts) = load_creds() else { return };
    let host = CliIoHost::new();
    let v = host.sql_database(X3_HOST, "sagex3", Some(&opts))
        .expect("Sql.Database(sagex3) must succeed");
    let Value::Table(t) = v else { panic!("expected Table, got {v:?}") };
    let TableRepr::Rows { rows, .. } = &t.repr else {
        panic!("expected TableRepr::Rows");
    };
    // Columns: Name, Data, Schema, Item, ItemKind, ItemName, IsLeaf
    let found = rows.iter().any(|r| {
        matches!((&r.get(2), &r.get(3)),
            (Some(Value::Text(s)), Some(Value::Text(n)))
            if s == "RAMLIVE" && n == "SALESREP")
    });
    assert!(found, "expected RAMLIVE.SALESREP in nav table; rows={}", rows.len());
}
