//! Pack-stream message builders for client→server requests.
//!
//! Every body is `<flag:u8=0><reqcode:u16 LE><inner_len:u32 LE><pack stream>`
//! per `Derek/DBISAM-PROTOCOL.md` §6c + §6g + ANSWERS-TO-DEREK.md.
//!
//! Pack stream is sequential `<u32 LE length><payload>` units; what each
//! unit means depends on the reqcode's `DoXxx` handler on the server.
//! Reqcode list is in the dispatch table (147 codes); we use a small
//! subset for SELECT-only extraction.

/// Reqcodes (subset — full table has 147 entries).
pub mod reqcode {
    pub const LOGIN: u16 = 0x0014;
    pub const SESSION_PARAMS: u16 = 0x0028;
    pub const OPEN_DATA_DIR: u16 = 0x003C;
    pub const CLOSE_DATA_DIR: u16 = 0x0046;

    pub const OPEN_CURSOR: u16 = 0x0096;
    pub const CLOSE_CURSOR: u16 = 0x00A0;
    pub const RESET_CURSOR: u16 = 0x00AA;
    pub const SET_INDEX_NAME: u16 = 0x00B4;
    pub const SET_TO_BEGIN: u16 = 0x00BE;
    pub const SET_TO_END: u16 = 0x00C8;
    pub const GET_CURRENT_RECORD: u16 = 0x00E6;
    pub const GET_NEXT_RECORD: u16 = 0x00FA;
    pub const GET_PRIOR_RECORD: u16 = 0x0104;
    pub const SET_TO_BOOKMARK: u16 = 0x0154;

    pub const OPEN_BLOB: u16 = 0x0280;
    pub const FREE_BLOB: u16 = 0x028A;

    pub const RECEIVE: u16 = 0x030C;
    pub const DATA_DIR_CTOR: u16 = 0x0316;
    pub const PREPARE_STATEMENT: u16 = 0x0320;
    pub const EXECUTE_STATEMENT: u16 = 0x032A;
    pub const RESET_STATEMENT: u16 = 0x0334;

    // Batched record-block family (RecordBlock = batch of N rows in one
    // round-trip; the single-row variants above are for cursor scrolling).
    pub const READ_NEXT_RECORD_BLOCK: u16 = 0x04F6;
    pub const READ_PRIOR_RECORD_BLOCK: u16 = 0x0500;
    pub const READ_FIRST_RECORD_BLOCK: u16 = 0x050A;
    pub const READ_LAST_RECORD_BLOCK: u16 = 0x0514;
    pub const READ_ABSOLUTE_RECORD_BLOCK: u16 = 0x051E;
    pub const READ_BOOKMARK_RECORD_BLOCK: u16 = 0x0528;
    pub const REFRESH_RECORD_BLOCK: u16 = 0x0532;
    pub const ADD_RECORD_BLOCK: u16 = 0x053C;
    pub const UPDATE_RECORD_BLOCK: u16 = 0x0546;
    pub const DELETE_RECORD_BLOCK: u16 = 0x0550;
}

/// Builder for a client request body. Writes the 7-byte header
/// `<flag=0><reqcode:u16 LE><inner_len:u32 LE>` then a sequence of
/// Pack units, then finishes by patching the inner_len based on the
/// total bytes written.
pub struct MsgBuilder {
    body: Vec<u8>,
    inner_start: usize,
}

impl MsgBuilder {
    pub fn new(reqcode: u16) -> Self {
        let mut body = Vec::with_capacity(64);
        body.push(0x00); // flag
        body.extend_from_slice(&reqcode.to_le_bytes());
        body.extend_from_slice(&0u32.to_le_bytes()); // inner_len placeholder
        let inner_start = body.len();
        Self { body, inner_start }
    }

    /// Push one `<u32 length><payload>` Pack unit.
    pub fn pack(&mut self, payload: &[u8]) -> &mut Self {
        self.body.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        self.body.extend_from_slice(payload);
        self
    }

    /// Convenience: pack a `u32` value (4 bytes LE).
    pub fn pack_u32(&mut self, v: u32) -> &mut Self {
        self.pack(&v.to_le_bytes())
    }

    /// Convenience: pack a `u8` value (1 byte).
    pub fn pack_u8(&mut self, v: u8) -> &mut Self {
        self.pack(&[v])
    }

    /// Finish: patch the inner_len field and return the body bytes.
    pub fn finish(mut self) -> Vec<u8> {
        let inner_len = (self.body.len() - self.inner_start) as u32;
        self.body[3..7].copy_from_slice(&inner_len.to_le_bytes());
        self.body
    }
}

// ---- Concrete request builders for SELECT-only flow ----

/// Connect handshake body (reqcode 0x0000, the very first message
/// after TCP connect). Per `DBISAM-PROTOCOL.md` §6g, 4 Pack fields:
///   1. u64 client version (= 0xAB7C — stable for DBISAM 4.39)
///   2. u8 RemoteCompression flag (1 = wire compression on)
///   3. AnsiString `<hostname><N>` — hostname + per-connection suffix
///   4. u32 random session nonce
///
/// Followed by 5 trailing zero bytes (padding).
///
/// `hostname` is the workstation name embedded in field 3; we use a
/// fixed value because the server doesn't validate it strictly.
/// `nonce` is whatever 4 bytes the caller wants — server stores it
/// but doesn't echo it back in a way that matters for SELECT flow.
pub fn build_connect(_compression: bool, hostname: &str, nonce: u32) -> Vec<u8> {
    // Per live-capture analysis of a dbsys session with RemoteCompression=9:
    // the Connect message's INNER field 2 is always 0. The session's
    // compression behaviour is determined by the OUTER body[0] flag
    // byte applied at the framing layer, not by this inner field.
    // Keep the parameter for API stability, but ignore.
    let mut m = MsgBuilder::new(0x0000); // CONNECT reqcode = 0
    m.pack(&0xAB7Cu64.to_le_bytes()); // field 1: version
    m.pack_u8(0); // field 2: always 0 per capture
    m.pack(hostname.as_bytes()); // field 3: hostname
    m.pack_u32(nonce); // field 4: nonce
    let mut body = m.finish();
    body.extend_from_slice(&[0, 0, 0, 0]); // 4 trailing zero bytes (padding)
    body
}

/// `TQueryStatement.PrepareStatement` (reqcode 0x0320). Sends the SQL
/// to be prepared; server responds with PackResultSetInfo (temp table
/// name) + PackResultSetFields (column schema) + PackCursorInfo
/// (initial bookmark). This is the workhorse "send query, get schema"
/// message.
///
/// The SQL is sent CRLF-terminated. The prelude/trailing flags are
/// observed verbatim from the PoC capture — exact field semantics not
/// fully decoded but the values are stable across queries.
pub fn build_prepare_statement(sql: &str) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::PREPARE_STATEMENT);
    // Inner prelude — 3 Pack units captured verbatim from PoC:
    //   <u32 len=4><u32 4>     ?
    //   <u32 len=4><u32 1>     statement handle / cursor index?
    //   <u32 len=4><u32 4>     ?
    // (Exact meaning TBD; pattern is stable across all observed queries.)
    m.pack_u32(4).pack_u32(1).pack_u32(4);

    // SQL is twice-length-prefixed (Delphi TStringField convention):
    //   <u32 sql_len><u32 sql_max_len><sql_bytes>
    // Even though we don't reuse Pack here for the double-length, the
    // shape matches what the server expects.
    let mut sql_bytes = sql.as_bytes().to_vec();
    sql_bytes.extend_from_slice(b"\r\n");
    let n = sql_bytes.len() as u32;
    // Embed the SQL as bytes within the inner section — these are NOT
    // Pack units; they're raw bytes the PrepareStatement handler reads
    // as a TStringField. Splice them directly into the body buffer.
    let mut tail_buf = Vec::new();
    tail_buf.extend_from_slice(&n.to_le_bytes());
    tail_buf.extend_from_slice(&n.to_le_bytes());
    tail_buf.extend_from_slice(&sql_bytes);
    // Trailing flags from PoC capture (stable across queries):
    const TRAIL: &[u8] = &[
        0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFF, 0xFF,
        0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00,
        0x01, 0x00, 0x00, 0x00,
    ];
    tail_buf.extend_from_slice(TRAIL);
    // The captured query body has these bytes appearing without Pack
    // framing — emit raw.
    let mut body = m.body;
    body.extend_from_slice(&tail_buf);
    // Patch inner_len.
    let inner_len = (body.len() - 7) as u32;
    body[3..7].copy_from_slice(&inner_len.to_le_bytes());
    // Final 5-byte outer trailer (also stable across queries).
    body.extend_from_slice(&[0x00, 0x00, 0x00, 0x00, 0x00]);
    body
}

/// `TQueryStatement.ExecuteStatement` (reqcode 0x032A). Kicks off
/// cursor execution after a successful PrepareStatement. Captured body
/// has 6 Pack units of small constants; we replay the structure.
pub fn build_execute_statement(cursor_handle: u32) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::EXECUTE_STATEMENT);
    m.pack_u32(cursor_handle); // cursor handle (= 1 for our single-cursor sessions)
    m.pack(&[0x00, 0x00]); // 2-byte payload (TBD — captured constant)
    m.pack_u8(0x00);
    m.pack_u8(0x01);
    m.pack_u8(0x00);
    m.pack_u8(0x00);
    m.finish()
}

/// `TQueryStatement.ExecuteStatement` (reqcode 0x032A) — DDL flavour.
/// Per Derek/DBISAM-PROTOCOL.md §7h, DDL's ExecuteStatement boilerplate
/// differs from DML at inner-body offset +23: `0x00` here vs `0x01` for
/// DML. Same 34-byte body otherwise.
pub fn build_execute_statement_ddl(cursor_handle: u32) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::EXECUTE_STATEMENT);
    m.pack_u32(cursor_handle);
    m.pack(&[0x00, 0x00]);
    m.pack_u8(0x00);
    m.pack_u8(0x00); // DDL flavour byte (DML uses 0x01 here)
    m.pack_u8(0x00);
    m.pack_u8(0x00);
    m.finish()
}

/// `TDataCursor.Receive` (reqcode 0x030C). Batched receive during
/// initial result-set transfer. Captured body is a single `<u32 len=1>
/// <0x00>` unit.
pub fn build_receive() -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::RECEIVE);
    m.pack_u8(0x00);
    m.finish()
}

/// `TDataCursor.SetToBegin` (reqcode 0x00BE). Positions the cursor at
/// the first row. Captured body is one Pack unit with the cursor handle.
pub fn build_set_to_begin(cursor_handle: u32) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::SET_TO_BEGIN);
    m.pack_u32(cursor_handle);
    m.finish()
}

/// `TDataCursor.GetNextRecord` (reqcode 0x00FA). Advances by one row.
/// Captured body has cursor + bookmark + 2 flags + counter; the
/// bookmark is echoed from the most recent server response per the
/// universal copy-paste rule.
pub fn build_get_next_record(cursor_handle: u32, bookmark: &[u8], counter: u32) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::GET_NEXT_RECORD);
    m.pack_u32(cursor_handle);
    m.pack(bookmark);
    m.pack_u8(0x00);
    m.pack_u8(0x00);
    m.pack_u32(counter);
    m.finish()
}

/// `TQueryStatement.ResetStatement` (reqcode 0x0334). Finalises a
/// statement after execution — captured body is a single Pack unit
/// carrying the cursor handle (`04 00 00 00 01 00 00 00` for handle=1).
/// Sent after the DML poll loop completes; commits/releases the work.
pub fn build_reset_statement(cursor_handle: u32) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::RESET_STATEMENT);
    m.pack_u32(cursor_handle);
    m.finish()
}

/// `TDataCursor.CloseCursor` (reqcode 0x00A0). Releases a server-side
/// cursor (and its backing temp table from a SELECT materialisation).
/// Captured body shape unverified; following the universal pattern of
/// "Pack u32 cursor_handle" used by every other cursor op.
pub fn build_close_cursor(cursor_handle: u32) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::CLOSE_CURSOR);
    m.pack_u32(cursor_handle);
    m.finish()
}

/// `TDataSession.RemoveAllRemoteMemoryTables` (reqcode 0x0029). Bulk-
/// releases every server-side temp table backing materialised SELECT
/// results in this session — empty inner body, just the reqcode.
/// Per Derek/DBISAM-PROTOCOL.md §7f + §7k: materialised SELECTs pin
/// their source table via `TDataTable.UseCount`, which blocks DDL
/// (DROP/ALTER) with `0x2B05` until the temp cursors close. Sending
/// `0x0029` after a SELECT clears the pin so a subsequent DROP can
/// proceed (RVA 0x07949C, identified by BPL disassembly).
pub fn build_remove_all_remote_memory_tables() -> Vec<u8> {
    MsgBuilder::new(0x0029).finish()
}

/// Begin-DML marker (reqcode 0x0316). Sent before every PrepareStatement
/// in DBSYS captures (DML and DDL alike — see Derek/DBISAM-PROTOCOL.md
/// §7a). Captured body is a single Pack unit carrying the cursor handle
/// (`04 00 00 00 01 00 00 00` for handle=1). Server replies with an ack.
pub fn build_begin_dml(cursor_handle: u32) -> Vec<u8> {
    // Reqcode constant isn't in our `reqcode` subset yet; pass the
    // literal to keep this builder self-contained.
    let mut m = MsgBuilder::new(0x0316);
    m.pack_u32(cursor_handle);
    m.finish()
}

/// `TDataCursor.SetToBookmark` (reqcode 0x0154). Positions by bookmark
/// (server-side, the bookmark bytes are opaque to the client per §6d).
/// Body is 4 Pack units per Derek's disassembly addendum.
pub fn build_set_to_bookmark(cursor_handle: u32, bookmark: &[u8], flag1: u8, flag2: u8) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::SET_TO_BOOKMARK);
    m.pack_u32(cursor_handle);
    m.pack(bookmark);
    m.pack_u8(flag1);
    m.pack_u8(flag2);
    m.finish()
}

/// `TDataCursor.ReadFirstRecordBlock` (reqcode 0x050A). Batched
/// "position-at-start + read N records" — one round-trip returns up to
/// `max_records` rows. Body layout (best-guess pending disassembly):
/// `<cursor_handle u32><max_records u32>`.
pub fn build_read_first_record_block(cursor_handle: u32, max_records: u32) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::READ_FIRST_RECORD_BLOCK);
    m.pack_u32(cursor_handle);
    m.pack_u32(max_records);
    m.finish()
}

/// `TDataCursor.ReadNextRecordBlock` (reqcode 0x04F6). Batched
/// "continue reading from current cursor position, return N more rows".
/// Per `ANSWERS-TO-DEREK-2.md` Q2: identical body shape to
/// `ReadFirstRecordBlock` — `<cursor_handle u32><batch_size u32>`. The
/// cursor's position is tracked server-side; no bookmark in the request.
pub fn build_read_next_record_block(cursor_handle: u32, max_records: u32) -> Vec<u8> {
    let mut m = MsgBuilder::new(reqcode::READ_NEXT_RECORD_BLOCK);
    m.pack_u32(cursor_handle);
    m.pack_u32(max_records);
    m.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header_is_flag_reqcode_le_innerlen_le() {
        let mut m = MsgBuilder::new(0x0320);
        m.pack_u32(1);
        let body = m.finish();
        assert_eq!(body[0], 0x00); // flag
        assert_eq!(&body[1..3], &[0x20, 0x03]); // reqcode 0x0320 LE
        // inner_len = 8 (one Pack unit: 4-byte length + 4-byte payload)
        assert_eq!(&body[3..7], &[0x08, 0x00, 0x00, 0x00]);
        assert_eq!(&body[7..11], &[0x04, 0x00, 0x00, 0x00]); // len=4
        assert_eq!(&body[11..15], &[0x01, 0x00, 0x00, 0x00]); // value=1
    }

    #[test]
    fn execute_statement_layout() {
        let body = build_execute_statement(1);
        // flag + reqcode 0x032A + inner_len + 6 Pack units
        assert_eq!(body[0], 0x00);
        assert_eq!(&body[1..3], &[0x2A, 0x03]);
        // 6 units = u32(1) + bytes(2) + u8 + u8 + u8 + u8
        // total inner = 4+4 + 4+2 + 4+1 + 4+1 + 4+1 + 4+1 = 34
        assert_eq!(u32::from_le_bytes([body[3], body[4], body[5], body[6]]), 34);
    }

    #[test]
    fn set_to_begin_layout() {
        let body = build_set_to_begin(1);
        // 7-byte header + 1 Pack unit (4 len + 4 payload) = 15 bytes
        assert_eq!(body.len(), 15);
        assert_eq!(&body[1..3], &[0xBE, 0x00]); // reqcode 0x00BE LE
    }

    /// Body shape for the single-Pack-u32(handle) family — used by
    /// BeginDML, ResetStatement, and CloseCursor. All three are
    /// captured as `04 00 00 00 01 00 00 00` for handle=1.
    fn single_handle_body_matches(body: &[u8], expected_reqcode: u16) {
        assert_eq!(body.len(), 15, "expected 7-byte header + 8-byte Pack unit");
        assert_eq!(body[0], 0x00);
        assert_eq!(&body[1..3], &expected_reqcode.to_le_bytes());
        assert_eq!(&body[3..7], &[0x08, 0x00, 0x00, 0x00]); // inner_len = 8
        assert_eq!(&body[7..11], &[0x04, 0x00, 0x00, 0x00]); // Pack len = 4
        assert_eq!(&body[11..15], &[0x01, 0x00, 0x00, 0x00]); // handle = 1
    }

    #[test]
    fn begin_dml_layout() {
        single_handle_body_matches(&build_begin_dml(1), 0x0316);
    }

    #[test]
    fn reset_statement_layout() {
        single_handle_body_matches(&build_reset_statement(1), reqcode::RESET_STATEMENT);
    }

    #[test]
    fn close_cursor_layout() {
        single_handle_body_matches(&build_close_cursor(1), reqcode::CLOSE_CURSOR);
    }

    #[test]
    fn remove_all_remote_memory_tables_is_just_the_header() {
        // No body — only flag + reqcode + inner_len(=0). 7 bytes total.
        let body = build_remove_all_remote_memory_tables();
        assert_eq!(body, vec![0x00, 0x29, 0x00, 0x00, 0x00, 0x00, 0x00]);
    }

    #[test]
    fn execute_statement_ddl_differs_at_offset_30() {
        // Inner offset +23 (= outer offset 30 after the 7-byte header) is
        // the DDL/DML flavour byte per Derek/DBISAM-PROTOCOL.md §7h:
        // 0x00 for DDL, 0x01 for DML. Everything else is identical.
        let dml = build_execute_statement(1);
        let ddl = build_execute_statement_ddl(1);
        assert_eq!(dml.len(), ddl.len(), "both 41 bytes");
        assert_eq!(dml.len(), 41);
        assert_eq!(dml[30], 0x01, "DML flavour byte at offset 30");
        assert_eq!(ddl[30], 0x00, "DDL flavour byte at offset 30");
        // Verify the surrounding bytes are otherwise identical.
        for i in (0..dml.len()).filter(|&i| i != 30) {
            assert_eq!(dml[i], ddl[i], "byte {i} should match between DML and DDL forms");
        }
    }
}
