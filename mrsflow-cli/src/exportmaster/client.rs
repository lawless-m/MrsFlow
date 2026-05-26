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
use super::cursor::{drive_cursor, find_row_starts};
use super::row::{decode_record, ColumnBuilders, RECORD_HEADER_LEN};
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

/// Session-setup messages sent immediately after a successful login.
/// Replayed verbatim from a captured customer-top3 session. The 4th
/// mentions `NISAINT_CS` (a collation name) — if we ever need to talk
/// to a different DBISAM database with a different collation, this is
/// the first place to look.
const SESSION_SETUP_BODIES: &[&[u8]] = &[
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
    // C[4] — 28-byte body, mentions `NISAINT_CS` (the database name)
    &[
        0x00, 0x3C, 0x00, 0x13, 0x00, 0x00, 0x00, 0x0A, 0x00, 0x00, 0x00, 0x4E, 0x49, 0x53, 0x41, 0x49,
        0x4E, 0x54, 0x5F, 0x43, 0x53, 0x01, 0x00, 0x00, 0x00, 0x00, 0x64, 0x00,
    ],
    // C[5] — 20-byte body
    &[
        0x00, 0x16, 0x03, 0x08, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x49,
        0x4E, 0x54, 0x5F, 0x43,
    ],
];

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

        // 3) Session-setup — 4 captured messages, in order.
        for body in SESSION_SETUP_BODIES {
            let _ = framing::send_recv(&mut stream, body)?;
        }

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
    /// 3. Drive the cursor: 11 replayed init messages + ACK/Fetch loop
    ///    spliced with the highest-seen primary key.
    /// 4. Pattern-find row starts in concatenated cursor bytes; decode
    ///    each row with the schema; accumulate into per-column Arrow
    ///    arrays.
    /// 5. Wrap as Value::Table.
    ///
    /// **Known-broken**: step 4 currently produces tables with the right
    /// shape (rowcount × colcount) but garbage cell values. Root cause
    /// not fully decoded: on the wire, fields appear packed with extra
    /// zero-byte gaps between them that the schema's `row_offset` /
    /// `max` arithmetic doesn't account for. For example CUSTOMER's
    /// CPYNAME (max=41) is followed by 7 zero bytes before CONTACT's
    /// null-flag — the schema says CONTACT.row_offset = 79 (i.e.
    /// CPYNAME.row_offset + max + 1 = 37 + 41 + 1 = 79) but wire
    /// position is +7 further out. Possibilities: per-column padding
    /// rules per ftType, or row_offset means something subtly different
    /// from "value-start position in the on-disk record". Needs more
    /// captures + cross-reference against the Delphi `TDataset` source.
    ///
    /// `target_rows` is the soft cap for the cursor loop (default
    /// `usize::MAX` for "all rows", but the caller can cap when issuing
    /// `SELECT … TOP N` to avoid over-fetching).
    pub fn query_to_table(&mut self, sql: &str) -> Result<Value, IoError> {
        self.query_to_table_capped(sql, usize::MAX)
    }

    pub fn query_to_table_capped(&mut self, sql: &str, target_rows: usize) -> Result<Value, IoError> {
        let schema_resp = self.query_raw(sql)?;
        let (columns, _end_off) = parse_schema(&schema_resp)?;

        // Drive the cursor; this returns the concatenated server bytes
        // from the post-query fetch round-trips. Don't include the
        // schema response in the row-search corpus — it contains index
        // entries (PK-only) that the row-finder false-positives on,
        // producing garbage values when fed to decode_record.
        let combined = drive_cursor(self.stream_mut(), &columns, target_rows)?;

        let starts = find_row_starts(&combined, &columns);
        let first_off = columns[0].row_offset as usize;
        let last_col = columns.last().unwrap();
        let row_size = (last_col.row_offset as usize - first_off) + last_col.max as usize;

        // Dedup by first-column value (PoC's "seen_codes" set). Rows
        // can appear multiple times across batches (server resends).
        use std::collections::HashSet;
        let mut seen = HashSet::new();
        let mut builders = ColumnBuilders::new(&columns, starts.len());
        for &s in &starts {
            let end = s + row_size;
            if end > combined.len() {
                continue;
            }
            let cells = match decode_record(&combined[s..end], &columns) {
                Ok(c) => c,
                Err(_) => continue,
            };
            // Use first column's text representation as the dedup key.
            let key = match &cells[0] {
                super::row::CellValue::Text(s) => s.clone(),
                other => format!("{other:?}"),
            };
            if !seen.insert(key) {
                continue;
            }
            if seen.len() > target_rows {
                break;
            }
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

    /// Discover tables in the connected database and return them as an
    /// M navigation record. Real DBISAM has `SELECT * FROM SYSTABLES`
    /// for this; we'll port it once query_to_table works.
    pub fn list_tables_as_navigation(&mut self, _opts: &ConnOpts) -> Result<Value, IoError> {
        Err(IoError::Other(
            "Exportmaster.Database: navigation path not yet implemented".into(),
        ))
    }
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
