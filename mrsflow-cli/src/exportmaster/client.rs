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

use super::crypto::encrypt_login;
use super::framing;
use super::{ConnOpts};

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
    /// Wired up in subsequent commits — for v1 we only support a single
    /// SELECT per Client (no statement reuse, matching the PoC).
    pub fn query_to_table(&mut self, _sql: &str) -> Result<Value, IoError> {
        // TODO: implement once schema.rs + row.rs + cursor.rs land.
        Err(IoError::Other(
            "Exportmaster.Query: query path not yet implemented (login succeeded)".into(),
        ))
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
