//! Native DBISAM client/server wire-protocol implementation.
//!
//! Talks directly to `dbsrvr.exe` over TCP, bypassing DBISAM's broken
//! ODBC driver (see `KNOWN_BUGS.md` §B1). Reverse-engineered from packet
//! captures + binary disassembly — protocol notes live in
//! `Derek/DBISAM-PROTOCOL.md` in the sibling Derek repo.
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

use mrsflow_core::eval::{IoError, Value};

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
    /// `Derek/DBISAM-PROTOCOL.md` §5 (`elevatesoft` encrypt password).
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

/// Run `sql` and return a `Value::Table`. Connects, logs in, executes,
/// drains the cursor, disconnects. One TCP session per call (matches the
/// `Odbc.Query` lifecycle — no connection pooling for v1).
pub fn query(opts: &ConnOpts, sql: &str) -> Result<Value, IoError> {
    let mut client = Client::connect_and_login(opts)?;
    client.query_to_table(sql)
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
