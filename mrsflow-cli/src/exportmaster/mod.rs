//! Native DBISAM client/server wire-protocol implementation.
//!
//! Talks directly to `dbsrvr.exe` over TCP, bypassing DBISAM's broken
//! ODBC driver (see `KNOWN_BUGS.md` §B1). Reverse-engineered from packet
//! captures + binary disassembly — protocol notes live in
//! `DBISAM-PROTOCOL.md` next to this file (vendored snapshot from the
//! Derek reverse-engineering repo at
//! dw.ramsden-international.com/matthew.heath/Derek).
//!
//! The public surface for the rest of mrsflow-cli is `query()` and
//! `database()` (this file). Internals split by concern:
//!
//! - `framing` — TCP send/recv with the 20-byte GUID+length envelope.
//! - `crypto`  — Blowfish-CBC login: key = MD5("elevatesoft").
//! - `client`  — connect + login + session-setup state machine; one
//!               `Client` per Exportmaster.Query call.
//! - `schema`  — parse the 772-byte column-block region of a SELECT
//!               response into `Column` descriptors.
//! - `row`     — decode wire bytes into Arrow ArrayRefs per ftType code
//!               (see protocol §6b).
//! - `cursor`  — universal cursor-advance: extract+splice the 32-byte
//!               cursor-state block from server responses into the next
//!               client fetch.
//! - `blob`    — opaque blob/memo handles + the `0x0280` fetch reqcode.
//!
//! Debug logging: set `EM_DML_DEBUG=1` to dump per-step request/response
//! bytes for the DML/DDL execute path (BeginDML, PrepareStatement,
//! ExecuteStatement, ResetStatement). Useful when a write-path query
//! fails with a server reqcode the parser doesn't recognise.

pub mod blob;
pub mod client;
pub mod crypto;
pub mod cursor;
pub mod cursor_info;
pub mod framing;
pub mod msg;
pub mod response;
pub mod row;
pub mod schema;
pub mod wire;

// Re-export pub items in a flat namespace for examples and downstream
// callers — keeps the surface predictable as the internal layout evolves.

pub use client::Client;

use mrsflow_core::eval::{EnvNode, IoError, Record, Value};

/// Connection options parsed from M's optional record argument
/// (`Exportmaster.Query(host, sql, [User=…, Password=…, …])`).
#[derive(Debug, Clone)]
pub struct ConnOpts {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub password: String,
    pub encrypt_password: String,
    /// DBISAM catalog name attached during session setup (request 0x003c).
    /// Default `NISAINT_CS` matches the only catalog we've tested against.
    pub catalog: String,
    /// Wire compression (Zlib deflate end-to-end). Maps to Connect
    /// handshake field 2 per `DBISAM-PROTOCOL.md` §6g.
    /// Default `true` — schema-heavy responses see 3-10× reduction.
    pub compression: bool,
    /// Number of rows requested per ReadFirstRecordBlock /
    /// ReadNextRecordBlock call. Bigger = fewer round trips but
    /// larger responses. Default 5000.
    pub batch_size: u32,
}

impl ConnOpts {
    /// Defaults match the ex3win / dbsys.exe baseline documented in
    /// `DBISAM-PROTOCOL.md` §5 (`elevatesoft` encrypt password).
    /// User and password have no sensible default — caller must supply.
    pub fn new(host: impl Into<String>, user: impl Into<String>, password: impl Into<String>) -> Self {
        Self {
            host: host.into(),
            port: 12005,
            user: user.into(),
            password: password.into(),
            encrypt_password: "elevatesoft".to_string(),
            catalog: "NISAINT_CS".to_string(),
            // Default off: on LAN the deflate/inflate CPU cost dominates
            // the bandwidth saving (measured 8m12s with compression vs
            // 2m36s without, for `SELECT * FROM ANALYSIS` over a fast
            // LAN). Opt in via `[Compression=true]` for slow/remote
            // links where bandwidth dominates.
            compression: false,
            batch_size: 5000,
        }
    }

    /// Read an M record value and overlay any present fields on `self`.
    /// Recognised fields: `Port` (number), `EncryptPassword` (text),
    /// `Catalog` (text), `Compression` (logical), `BatchSize` (number).
    /// The required `User` / `Password` are extracted by the caller.
    pub fn apply_options(&mut self, opts: Option<&Value>) -> Result<(), IoError> {
        let Some(Value::Record(r)) = opts else { return Ok(()) };
        for (name, v) in r.fields.iter() {
            match name.as_str() {
                "Port" => {
                    if let Value::Number(n) = v {
                        self.port = *n as u16;
                    }
                }
                "EncryptPassword" => {
                    if let Value::Text(s) = v {
                        self.encrypt_password = s.clone();
                    }
                }
                "Catalog" => {
                    if let Value::Text(s) = v {
                        self.catalog = s.clone();
                    }
                }
                "Compression" => {
                    if let Value::Logical(b) = v {
                        self.compression = *b;
                    }
                }
                "BatchSize" => {
                    if let Value::Number(n) = v {
                        if *n >= 1.0 && *n <= 100_000.0 {
                            self.batch_size = *n as u32;
                        }
                    }
                }
                _ => {} // unknown options ignored — same as PQ
            }
        }
        Ok(())
    }
}

/// Run `sql` and return a result `Value`. Connects, logs in, executes,
/// disconnects. One TCP session per call (matches the `Odbc.Query`
/// lifecycle — no connection pooling for v1).
///
/// Dispatches on the first SQL keyword:
/// - DML (`UPDATE` / `INSERT` / `DELETE`) and DDL (`CREATE` / `ALTER`
///   / `DROP` / `TRUNCATE`) → returns `Value::Record` with
///   `[RowsAffected = N]` via the poll-and-finalise path
///   ([`Client::execute_dml`]). DDL uses the same wire flow as DML
///   on DBISAM (Derek/dbisam-capture-autoinc.pcapng confirms: 0x0316
///   → 0x0320 → 0x032A → 0x0334 for `CREATE TABLE`); the affected
///   count for DDL is whatever the server reports (typically 0).
/// - Anything else (SELECT, EXEC, ...) → returns `Value::Table` via
///   the cursor-fetch path ([`Client::query_to_table`]).
pub fn query(opts: &ConnOpts, sql: &str) -> Result<Value, IoError> {
    let mut client = Client::connect_and_login(opts)?;
    if is_no_result_statement(sql) {
        let ddl = is_ddl(sql);
        let affected = client.execute_dml(sql, ddl)?;
        // Per DBISAM-PROTOCOL.md §7h, DDL has no meaningful row
        // count. Observed with the DML-flavoured ExecuteStatement,
        // DROP returned `0x74726168` ("hart" ASCII) at the count offset
        // — likely an adjacent field bleed-through. Normalise DDL to 0.
        let reported = if ddl { 0 } else { affected };
        Ok(rows_affected_record(reported))
    } else {
        client.query_to_table(sql)
    }
}

fn is_ddl(sql: &str) -> bool {
    matches!(
        first_keyword(sql).to_ascii_uppercase().as_str(),
        "CREATE" | "ALTER" | "DROP" | "TRUNCATE"
    )
}

/// First non-whitespace ASCII-alpha run from `sql` — the SQL command
/// keyword used for read-vs-write dispatch.
fn first_keyword(sql: &str) -> &str {
    let trimmed = sql.trim_start();
    let end = trimmed
        .find(|c: char| !c.is_ascii_alphabetic())
        .unwrap_or(trimmed.len());
    &trimmed[..end]
}

/// True for SQL that produces no result set on DBISAM — DML
/// (UPDATE/INSERT/DELETE) and DDL (CREATE/ALTER/DROP/TRUNCATE). All of
/// these travel the byte-identical wire path documented in
/// DBISAM-PROTOCOL.md §7, so the client dispatches them through
/// the same `execute_dml` poll-and-finalise loop.
fn is_no_result_statement(sql: &str) -> bool {
    let kw = first_keyword(sql);
    matches!(
        kw.to_ascii_uppercase().as_str(),
        "UPDATE" | "INSERT" | "DELETE" | "CREATE" | "ALTER" | "DROP" | "TRUNCATE"
    )
}

fn rows_affected_record(n: u32) -> Value {
    Value::Record(Record {
        fields: vec![("RowsAffected".to_string(), Value::Number(n as f64))],
        env: EnvNode::empty(),
    })
}

/// Return a navigation `Value::Record` listing tables in the connected
/// database. Each field's value is a `Value::Table` for that table —
/// realised lazily on access. Matches the shape produced by
/// `Odbc.DataSource(_, [HierarchicalNavigation=true])` so existing M
/// queries that walk that nav structure can switch transports without
/// changing their indexing.
pub fn database(opts: &ConnOpts) -> Result<Value, IoError> {
    let mut client = Client::connect_and_login(opts)?;
    client.list_tables_as_navigation(opts)
}

#[cfg(test)]
mod tests {
    use super::{first_keyword, is_ddl, is_no_result_statement};

    #[test]
    fn first_keyword_strips_leading_whitespace_and_stops_at_punctuation() {
        assert_eq!(first_keyword("SELECT * FROM t"), "SELECT");
        assert_eq!(first_keyword("  select 1"), "select");
        assert_eq!(first_keyword("\n\tUPDATE x SET y=1"), "UPDATE");
        assert_eq!(first_keyword(""), "");
        assert_eq!(first_keyword("   "), "");
        // First non-alpha terminates the keyword.
        assert_eq!(first_keyword("DROP\tTABLE foo"), "DROP");
    }

    #[test]
    fn is_no_result_statement_covers_dml_and_ddl() {
        for sql in [
            "UPDATE customer SET pay2 = 'x'",
            "insert into t values (1)",
            "DELETE FROM t",
            "CREATE TABLE t (pk autoinc)",
            "alter table t add column c int",
            "drop table t",
            "TRUNCATE TABLE t",
            "  Update t SET v=1", // leading whitespace + mixed case
        ] {
            assert!(is_no_result_statement(sql), "expected execute-only: {sql:?}");
        }

        for sql in ["SELECT * FROM t", "select 1", "exec sp_foo", ""] {
            assert!(!is_no_result_statement(sql), "expected cursor path: {sql:?}");
        }
    }

    #[test]
    fn is_ddl_only_matches_ddl_keywords() {
        for sql in [
            "CREATE TABLE t (pk autoinc)",
            "drop TABLE t",
            "ALTER TABLE t ADD c int",
            "truncate table t",
        ] {
            assert!(is_ddl(sql), "expected DDL: {sql:?}");
        }
        for sql in [
            "UPDATE t SET x=1",
            "INSERT INTO t VALUES (1)",
            "DELETE FROM t",
            "SELECT * FROM t",
        ] {
            assert!(!is_ddl(sql), "expected NOT DDL: {sql:?}");
        }
    }
}
