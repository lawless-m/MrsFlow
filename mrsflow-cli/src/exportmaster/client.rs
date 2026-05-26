//! DBISAM client state machine: connect → login → session-setup →
//! ready-for-queries → cursor loop → disconnect.
//!
//! The Connect body and the 4 session-setup bodies are byte-for-byte
//! replays from the PoC's captured `dbsys.exe` session. We don't yet
//! understand every field in them; treating them as opaque blobs is
//! what the PoC does and what we know works against the live server.
//! Decoding them properly is open work — Derek/DBISAM-PROTOCOL.md §7.

use std::net::TcpStream;

use mrsflow_core::eval::{IoError, Value};

use mrsflow_core::eval::Table;

use super::crypto::encrypt_login;
use super::framing;
use super::ConnOpts;
use super::cursor::drive_cursor;
use super::row::{decode_record, ColumnBuilders};
use super::schema::parse as parse_schema;

/// Captured Connect body (52 bytes) — replayed verbatim. Workstation
/// name `RIVSEM048692` is embedded in the middle; we send it as-is
/// because the PoC does the same and it works. Substituting the local
/// hostname is a follow-up if it turns out the server cares.
const CONNECT_BODY: &[u8] = &[
    0x00, 0x00, 0x00, 0x29, 0x00, 0x00, 0x00, 0x08, 0x00, 0x00, 0x00, 0x7C, 0xAB, 0x00, 0x00, 0x00,
    0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x0C, 0x00, 0x00, 0x00, 0x52, 0x49, 0x56, 0x53,
    0x45, 0x4D, 0x30, 0x34, 0x38, 0x36, 0x39, 0x32, 0x04, 0x00, 0x00, 0x00, 0xE8, 0x1B, 0xA2, 0xE5,
    0x00, 0x00, 0x00, 0x00,
];

/// Fixed session-setup messages sent immediately after a successful
/// login. C[2] and C[3] are replayed verbatim from capture; their
/// internal field meanings are not fully decoded. C[4] (catalog
/// attach, reqcode 0x003c) is built from `opts.catalog` so callers can
/// target different databases. C[5] is a trailing handshake replay.
const SESSION_SETUP_PRE: &[&[u8]] = &[
    // C[2] — 44-byte body
    &[
        0x00, 0x28, 0x00, 0x20, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x00, 0x01, 0x00, 0x00, 0x00, 0x0F, 0x02, 0x00, 0x00, 0x00, 0x64, 0x00, 0x01, 0x00, 0x00, 0x00,
        0x01, 0x02, 0x00, 0x00, 0x00, 0x14, 0x00, 0x17, 0xF2, 0x43, 0x90, 0x00,
    ],
    // C[3] — 12-byte body
    &[
        0x00, 0x84, 0x03, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    ],
];

/// C[5] — 20-byte trailing session-setup body, sent after the catalog
/// attach. The trailing `49 4E 54 5F 43` ("INT_C") appears in every
/// session capture and is sent verbatim; its meaning hasn't been
/// decoded but the server accepts it.
const SESSION_SETUP_POST: &[u8] = &[
    0x00, 0x16, 0x03, 0x08, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x49,
    0x4E, 0x54, 0x5F, 0x43,
];

/// Build the catalog-attach message body (reqcode 0x003c) for the
/// given catalog name. Layout decoded from an uncompressed capture
/// of `pyodbc.connect(DSN=Exportmaster)`:
///
///   reqcode 0x003c BE | sub-flag 0x00 | inner_len LE u32 |
///   inner_len bytes: [u32 LE name_len][catalog name][5-byte trailer
///                     `01 00 00 00 00`] |
///   trailing `0x64 0x00` (2 bytes)
///
/// Verified equivalent to the byte-for-byte replay of `NISAINT_CS`.
fn build_catalog_attach_body(catalog: &str) -> Vec<u8> {
    let name = catalog.as_bytes();
    let inner_len = 4 + name.len() + 5;
    let mut body = Vec::with_capacity(3 + 4 + inner_len + 2);
    body.extend_from_slice(&[0x00, 0x3C, 0x00]);
    body.extend_from_slice(&(inner_len as u32).to_le_bytes());
    body.extend_from_slice(&(name.len() as u32).to_le_bytes());
    body.extend_from_slice(name);
    body.extend_from_slice(&[0x01, 0x00, 0x00, 0x00, 0x00]);
    body.extend_from_slice(&[0x64, 0x00]);
    body
}

/// An open, logged-in DBISAM session.
pub struct Client {
    stream: TcpStream,
}

impl Client {
    /// Connect, log in, run the post-login session-setup handshake.
    /// On success the session is ready for queries.
    pub fn connect_and_login(opts: &ConnOpts) -> Result<Self, IoError> {
        let mut stream = framing::connect(&opts.host, opts.port)?;

        // 1) Connect — replay captured body, ignore the server's reply
        //    (we don't decode it yet).
        let _ = framing::send_recv(&mut stream, CONNECT_BODY)?;

        // 2) Login — construct from cracked crypto.
        let ct = encrypt_login(
            opts.user.as_bytes(),
            opts.password.as_bytes(),
            opts.encrypt_password.as_bytes(),
        );
        let login_body = build_login_body(&ct);
        let _ = framing::send_recv(&mut stream, &login_body)?;

        // 3) Session-setup — fixed pre messages, then catalog attach
        //    (parameterised by opts.catalog), then trailing handshake.
        for body in SESSION_SETUP_PRE {
            let _ = framing::send_recv(&mut stream, body)?;
        }
        let catalog_body = build_catalog_attach_body(&opts.catalog);
        let _ = framing::send_recv(&mut stream, &catalog_body)?;
        let _ = framing::send_recv(&mut stream, SESSION_SETUP_POST)?;

        Ok(Self { stream })
    }

    /// Borrow the underlying stream. Submodules use this for query and
    /// cursor work. Crate-internal only.
    pub(super) fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Execute `sql` and materialise the full result as a Value::Table.
    /// One Client per query (matches the Exportmaster.Query M call shape).
    ///
    /// Pipeline:
    /// 1. Send the query packet, get the schema response.
    /// 2. Parse the schema (772-byte column blocks).
    /// 3. Drive the cursor — captured init messages + ACK/Fetch loop.
    /// 4. Walk the response bytes via the universal `<u32 length>
    ///    <payload>` framing rule (protocol §6c) — every chunk whose
    ///    length equals `record_size` is one row. Decode and accumulate.
    /// 5. Wrap as Value::Table.
    ///
    /// `target_rows` is the soft cap (default `usize::MAX` for "all
    /// rows", but the caller can cap when issuing `SELECT … TOP N` to
    /// avoid over-fetching).
    pub fn query_to_table(&mut self, sql: &str) -> Result<Value, IoError> {
        self.query_to_table_capped(sql, usize::MAX)
    }

    pub fn query_to_table_capped(&mut self, sql: &str, target_rows: usize) -> Result<Value, IoError> {
        let schema_resp = self.query_raw(sql)?;
        let (columns, _end_off) = parse_schema(&schema_resp)?;
        let first_off = columns[0].row_offset as usize;
        let last_col = columns.last().unwrap();
        let col_data_span = (last_col.row_offset as usize - first_off) + 1 + last_col.max as usize;

        let mut rows = Vec::with_capacity(target_rows.min(1024));
        drive_cursor(self.stream_mut(), &columns, target_rows, &mut rows)?;

        let mut builders = ColumnBuilders::new(&columns, rows.len());
        for row in &rows {
            // Row points at the start of the on-disk record; column
            // data begins at +first_off (= 25 for tables with first
            // column at row_offset 25).
            let col_end = first_off + col_data_span;
            if col_end > row.len() {
                continue;
            }
            let cells = decode_record(&row[first_off..col_end], &columns)?;
            builders.push_row(cells)?;
        }

        let batch = builders.finish()?;
        Ok(Value::Table(Table::from_arrow(batch)))
    }

    /// Issue a `SELECT … FROM <table>` and return the raw schema-bearing
    /// response (the first server message after the query packet). Used
    /// by callers that want to parse the schema themselves; also the
    /// hook the smoke test uses to exercise [`super::schema::parse`].
    pub fn query_raw(&mut self, sql: &str) -> Result<Vec<u8>, IoError> {
        let body = build_query_body(sql);
        framing::send_recv(&mut self.stream, &body)
    }

    /// Issue a `SELECT COUNT(*) FROM <table>` and return the integer
    /// result. Sized at 32 bits — DBISAM count(*) caps there (the count
    /// values observed in captures are encoded as 0x80 + 3-byte BE,
    /// so the wire format itself maxes at 2^24).
    ///
    /// Verified live: `select count(*) from product` returns 146728,
    /// matching the Python PoC and protocol doc §3.
    pub fn count(&mut self, sql: &str) -> Result<u32, IoError> {
        let query_body = build_query_body(sql);
        let query_resp = framing::send_recv(&mut self.stream, &query_body)?;

        // The count comes back across a few round-trips. The PoC sends
        // two follow-up messages after the query and then scans the
        // concatenated response for the 0x80 + 3-byte BE pattern.
        let mut combined = query_resp;
        for body in POST_QUERY_BODIES_COUNT {
            let r = framing::send_recv(&mut self.stream, body)?;
            combined.extend_from_slice(&r);
        }

        // Scan for the first `0x80 + 3-byte BE` integer in a plausible
        // count range. The PoC uses 1000..100_000_000; we widen the
        // upper bound to 2^24 - 1 (the maximum the 3-byte form can
        // encode). A real count of 0 wouldn't match this heuristic;
        // for v1 that's acceptable (`SELECT COUNT(*)` of an empty
        // table is uncommon enough to defer).
        for i in 0..combined.len().saturating_sub(3) {
            if combined[i] == 0x80 {
                let value = ((combined[i + 1] as u32) << 16)
                    | ((combined[i + 2] as u32) << 8)
                    | (combined[i + 3] as u32);
                if (1..(1 << 24)).contains(&value) {
                    return Ok(value);
                }
            }
        }
        Err(IoError::Other(format!(
            "Exportmaster.Count: no count value found in {}-byte response",
            combined.len()
        )))
    }

    /// Issue the SQLTables-equivalent native request (reqcode 0x0032)
    /// and return the list of table names. Layout decoded from an
    /// uncompressed capture of `pyodbc.connect(DSN=Exportmaster)`
    /// followed by `cursor.tables()`.
    ///
    /// Request body (20 bytes total):
    ///   reqcode 0x0032 BE | sub-flag 0x00 | inner_len LE u32 = 8 |
    ///   inner: `04 00 00 00 01 00 00 00` |
    ///   trailing 5 bytes `49 4E 54 5F 43` (replayed verbatim)
    ///
    /// Response body header is 11 bytes; the table count is a u32 LE at
    /// offset 7. Then `count` entries of `[u32 LE name_len][ASCII name]`.
    ///
    /// Live-verified against `NISAINT_CS` on rivsem01: returns 653 table
    /// names matching `pyodbc cursor.tables()`.
    pub fn list_tables(&mut self) -> Result<Vec<String>, IoError> {
        const SQLTABLES_BODY: &[u8] = &[
            0x00, 0x32, 0x00, 0x08, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
            0x49, 0x4E, 0x54, 0x5F, 0x43,
        ];
        let resp = framing::send_recv(&mut self.stream, SQLTABLES_BODY)?;
        parse_sqltables_response(&resp)
    }

    /// Discover tables in the connected database and return them as an
    /// M navigation table. Shape matches `Odbc.DataSource` /
    /// `MySQL.Database`: columns `Name, Data, ItemKind, ItemName,
    /// IsLeaf` where each `Data` is a thunk that, on force, runs
    /// `SELECT * FROM <table>` via a fresh `Client`.
    ///
    /// **Known limitation — cursor sub-protocol undecoded.** Forcing a
    /// `Data` thunk only works for tables whose cursor advance matches
    /// the CUSTOMER-shape capture we replay (single-column PK, simple
    /// SELECT). Tables with composite keys or multi-table joins produce
    /// a different cursor-advance sequence (0x0080/0x008a index-tuple
    /// pairs per row) that we don't yet generate. The thunk surfaces
    /// the underlying error verbatim; users who hit this need to call
    /// `Exportmaster.Query(host, sql, opts)` with explicit SQL.
    ///
    /// See `Derek/DBISAM-PROTOCOL.md` §7 — decoding the cursor advance
    /// is listed as the top open question.
    pub fn list_tables_as_navigation(&mut self, opts: &ConnOpts) -> Result<Value, IoError> {
        use std::cell::RefCell;
        use std::rc::Rc;

        use mrsflow_core::eval::{MError, ThunkState};

        let names = self.list_tables()?;

        let cols = vec![
            "Name".to_string(),
            "Data".to_string(),
            "ItemKind".to_string(),
            "ItemName".to_string(),
            "IsLeaf".to_string(),
        ];
        let mut rows: Vec<Vec<Value>> = Vec::with_capacity(names.len());
        for name in names {
            let opts_for_thunk = opts.clone();
            let table_for_thunk = name.clone();
            let fetcher: Rc<dyn Fn() -> Result<Value, MError>> = Rc::new(move || {
                let sql = format!("SELECT * FROM {}", table_for_thunk);
                let mut c = Client::connect_and_login(&opts_for_thunk)
                    .map_err(|e| MError::Other(format!("Exportmaster connect: {e:?}")))?;
                c.query_to_table(&sql)
                    .map_err(|e| MError::Other(format!("Exportmaster fetch: {e:?}")))
            });
            let data = Value::Thunk(Rc::new(RefCell::new(ThunkState::Native(fetcher))));
            rows.push(vec![
                Value::Text(name.clone()),
                data,
                Value::Text("Table".to_string()),
                Value::Text(name),
                Value::Logical(true),
            ]);
        }
        Ok(Value::Table(Table::from_rows(cols, rows)))
    }
}

/// Parse the bulk SQLTables response. `resp` is the body of the first
/// server message returned after `SQLTABLES_BODY`. Layout (per
/// `Derek/DBISAM-PROTOCOL.md` SQLTables-response section):
///
///   `[reqcode:u16 BE = 0x0000][inner_len:u16 BE]` envelope, then
///   `[3-byte type flag][4 unknown bytes][u32 LE count]` header (11
///   bytes total), then `count` entries of `[u32 LE name_len][ASCII name]`.
fn parse_sqltables_response(resp: &[u8]) -> Result<Vec<String>, IoError> {
    if resp.len() < 15 {
        return Err(IoError::Other(format!(
            "Exportmaster.Database: SQLTables response too short ({} bytes)",
            resp.len()
        )));
    }
    // Skip 4-byte envelope (reqcode u16 BE + inner_len u16 BE) and the
    // 11-byte payload header — count is the u32 LE at payload offset 7
    // (== response offset 11).
    let count_off = 4 + 7;
    let count = u32::from_le_bytes([
        resp[count_off],
        resp[count_off + 1],
        resp[count_off + 2],
        resp[count_off + 3],
    ]) as usize;
    if count > 1_000_000 {
        return Err(IoError::Other(format!(
            "Exportmaster.Database: implausible table count {count} — wire layout may have changed"
        )));
    }
    let mut pos = 4 + 11;
    let mut names = Vec::with_capacity(count);
    for k in 0..count {
        if pos + 4 > resp.len() {
            return Err(IoError::Other(format!(
                "Exportmaster.Database: truncated at name {k}/{count}"
            )));
        }
        let slen = u32::from_le_bytes([resp[pos], resp[pos + 1], resp[pos + 2], resp[pos + 3]]) as usize;
        pos += 4;
        if slen == 0 || slen > 256 || pos + slen > resp.len() {
            return Err(IoError::Other(format!(
                "Exportmaster.Database: bad name length {slen} at name {k}"
            )));
        }
        let name = std::str::from_utf8(&resp[pos..pos + slen])
            .map_err(|_| IoError::Other(format!("Exportmaster.Database: non-utf8 name at {k}")))?
            .to_string();
        names.push(name);
        pos += slen;
    }
    Ok(names)
}

/// Two post-query messages sent after a `count(*)` query to coax the
/// final count value out of the server. Verbatim from PoC capture; the
/// exact field meanings aren't fully decoded.
const POST_QUERY_BODIES_COUNT: &[&[u8]] = &[
    // C[7] (POST_QUERY #1) — 44 bytes
    &[
        0x00, 0x2A, 0x03, 0x22, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x29, 0x20, 0x66,
    ],
    // C[8] (POST_QUERY #2) — 12 bytes
    &[
        0x00, 0x0C, 0x03, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    ],
];

/// Build a QUERY message body for the given SQL.
///
/// Matches the captured `select count(*) from analysis\r\n` packet
/// (PoC `build_query`, dbisam_client.py L106-138). The SQL is sent
/// twice-length-prefixed (Delphi `TStringField` convention) with a
/// trailing CRLF.
fn build_query_body(sql: &str) -> Vec<u8> {
    let mut sql_bytes = sql.as_bytes().to_vec();
    sql_bytes.extend_from_slice(b"\r\n");
    let n = sql_bytes.len() as u32;

    // Inner pre: cursor handle etc.
    const INNER_PRE: &[u8] = &[
        0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
    ];
    // Inner trail: status / flags.
    const INNER_TRAIL: &[u8] = &[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF, 0x04, 0x00, 0x00,
        0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00,
    ];
    const OUTER_TRAIL: &[u8] = &[0x00, 0x00, 0x00, 0x00, 0x00];

    let inner_len: u32 = (INNER_PRE.len() + 8 + sql_bytes.len() + INNER_TRAIL.len()) as u32;

    let mut body = Vec::with_capacity(3 + 4 + inner_len as usize + OUTER_TRAIL.len());
    body.extend_from_slice(&[0x00, 0x20, 0x03]); // flag + reqcode 0x0320
    body.extend_from_slice(&inner_len.to_le_bytes());
    body.extend_from_slice(INNER_PRE);
    body.extend_from_slice(&n.to_le_bytes()); // sql_len
    body.extend_from_slice(&n.to_le_bytes()); // sql_max_len
    body.extend_from_slice(&sql_bytes);
    body.extend_from_slice(INNER_TRAIL);
    body.extend_from_slice(OUTER_TRAIL);
    body
}

/// Wrap the login ciphertext in the LOGIN message body — reqcode 0x14,
/// double-length prefix, single trailing zero. See protocol §5.
fn build_login_body(ct: &[u8]) -> Vec<u8> {
    let inner_len: u32 = (4 + 4 + 4 + ct.len()) as u32;
    let mut body = Vec::with_capacity(3 + 4 + inner_len as usize + 1);
    body.extend_from_slice(&[0x00, 0x14, 0x00]); // flag + reqcode 0x14
    body.extend_from_slice(&inner_len.to_le_bytes());
    body.extend_from_slice(&4u32.to_le_bytes()); // first inner field length
    body.extend_from_slice(&(ct.len() as u32).to_le_bytes()); // buf len
    body.extend_from_slice(&(ct.len() as u32).to_le_bytes()); // buf max len
    body.extend_from_slice(ct);
    body.push(0x00);
    body
}
