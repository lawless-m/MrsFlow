//! DBISAM client state machine: connect → login → session-setup →
//! ready-for-queries → cursor loop → disconnect.
//!
//! The Connect body and the 4 session-setup bodies are byte-for-byte
//! replays from the PoC's captured `dbsys.exe` session. We don't yet
//! understand every field in them; treating them as opaque blobs is
//! what the PoC does and what we know works against the live server.
//! Decoding them properly is open work — DBISAM-PROTOCOL.md §7.

use std::net::TcpStream;

use mrsflow_core::eval::{IoError, Value};

use mrsflow_core::eval::Table;

use super::blob;
use super::crypto::encrypt_login;
use super::framing;
use super::ConnOpts;
use super::cursor::drive_cursor;
use super::row::{decode_record, CellValue, ColumnBuilders};
use super::schema::{parse as parse_schema, FieldType};

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
    /// Whether to deflate every subsequent body before sending and
    /// inflate every received body. Set during `connect_and_login`
    /// based on `ConnOpts::compression`. The Connect message itself
    /// is always uncompressed (the server doesn't know the flag yet).
    compression: bool,
    /// Batch size for ReadFirstRecordBlock / ReadNextRecordBlock,
    /// forwarded to `drive_cursor`.
    pub(super) batch_size: u32,
    /// Blob fetch slot_length (cursor.@+0x3672 — see ConnOpts).
    pub(super) blob_slot_length: usize,
}

impl Client {
    /// Connect, log in, run the post-login session-setup handshake.
    /// On success the session is ready for queries.
    pub fn connect_and_login(opts: &ConnOpts) -> Result<Self, IoError> {
        let mut stream = framing::connect(&opts.host, opts.port)?;

        // 1) Connect — built from opts so the compression flag is set
        //    correctly. Per capture analysis, Connect itself is also
        //    compressed when RemoteCompression is on (the server
        //    detects the comp byte and inflates accordingly).
        let connect_body = super::msg::build_connect(
            opts.compression,
            "RIVSEM048692", // stable hostname suffix — server doesn't validate strictly
            0xE5A21BE8,     // fixed nonce — server stores but doesn't echo
        );
        let r = framing::send_recv_auto(&mut stream, &connect_body, opts.compression)?;
        check_handshake_response("Connect", &r)?;

        // From here on, if compression is enabled, every body is deflated.
        let send = |stream: &mut TcpStream, body: &[u8]| -> Result<Vec<u8>, IoError> {
            if opts.compression {
                framing::send_recv_compressed(stream, body)
            } else {
                framing::send_recv(stream, body)
            }
        };

        // 2) Login — construct from cracked crypto.
        let ct = encrypt_login(
            opts.user.as_bytes(),
            opts.password.as_bytes(),
            opts.encrypt_password.as_bytes(),
        )?;
        let login_body = build_login_body(&ct);
        let r = send(&mut stream, &login_body)?;
        check_handshake_response("Login", &r)?;

        // 3) Session-setup — fixed pre messages, then catalog attach
        //    (parameterised by opts.catalog), then trailing handshake.
        for body in SESSION_SETUP_PRE {
            let r = send(&mut stream, body)?;
            check_handshake_response("session setup", &r)?;
        }
        let catalog_body = build_catalog_attach_body(&opts.catalog);
        let r = send(&mut stream, &catalog_body)?;
        check_handshake_response("catalog attach", &r)?;
        let r = send(&mut stream, SESSION_SETUP_POST)?;
        check_handshake_response("session setup (post)", &r)?;

        Ok(Self {
            stream,
            compression: opts.compression,
            batch_size: opts.batch_size,
            blob_slot_length: opts.blob_slot_length,
        })
    }

    /// Borrow the underlying stream. Submodules use this for query and
    /// cursor work. Crate-internal only.
    pub(super) fn stream_mut(&mut self) -> &mut TcpStream {
        &mut self.stream
    }

    /// Whether this session uses wire compression. Submodules consult
    /// this to choose between `framing::send_recv` and
    /// `framing::send_recv_compressed`.
    pub(super) fn compression(&self) -> bool {
        self.compression
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
        self.query_to_table_streaming(sql, usize::MAX)
    }

    /// Stream rows via `GetNextRecord` (0x00FA) — matches the pattern ODBC
    /// and dbsys use. The server packs multiple rows per response (each as
    /// `<u16 result_code><10 cursor-info units><56-byte slot>`), we walk
    /// them one by one and resolve any blob columns inline (`0x0280` +
    /// `0x028A` per row).
    ///
    /// Two reasons this exists alongside [`query_to_table_capped`]:
    ///
    /// 1. **Scale.** `ReadFirstRecordBlock` (0x050A) materialises every
    ///    requested row server-side at once. After a few hundred OpenBlobs
    ///    against that materialised set, the server returns `0x2303`
    ///    ("blob not found") for every subsequent fetch — observed at
    ///    task 644 of a `TOP 1000` extract regardless of `FreeBlob` or
    ///    `FreeAllBlobs` flushing. The interleaved `GetNextRecord` /
    ///    `OpenBlob` pattern lets the server advance through (and free)
    ///    materialised rows as we go.
    /// 2. **Memory.** Per-batch streaming bounds memory to one
    ///    `GetNextRecord` response's worth of rows instead of buffering
    ///    the whole result before resolving blobs.
    pub fn query_to_table_streaming(&mut self, sql: &str, target_rows: usize) -> Result<Value, IoError> {
        use super::response::{PACK_STREAM_OFFSET, RESULT_END_OF_CURSOR, RESULT_NOT_READY, RESULT_OK};
        use super::wire::Walker;
        use super::cursor_info::CursorInfo;

        let schema_resp = self.query_raw(sql)?;
        super::response::check_body_reqcode("PrepareStatement", &schema_resp)?;
        let (columns, _end_off) = parse_schema(&schema_resp)?;
        let compression = self.compression;
        let first_off = columns[0].row_offset as usize;
        let last_col = columns.last().unwrap();
        let col_data_span = (last_col.row_offset as usize - first_off) + 1 + last_col.max as usize;
        let col_end_offset = first_off + col_data_span;

        let blob_col_indices: Vec<usize> = columns
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.field_type, FieldType::Blob | FieldType::Memo | FieldType::Graphic))
            .map(|(i, _)| i)
            .collect();

        // Phase 1: ExecuteStatement + Receive poll until cursor materialised.
        let mut resp = framing::send_recv_auto(
            &mut self.stream,
            &super::msg::build_execute_statement(1),
            compression,
        )?;
        const POLL_SENTINEL: u16 = 0x2C14;
        let mut polls = 0;
        const MAX_POLLS: usize = 600;
        loop {
            let body_rc = if resp.len() >= 3 { u16::from_le_bytes([resp[1], resp[2]]) } else { 0 };
            let inner_rc = if resp.len() >= PACK_STREAM_OFFSET + 6 {
                let p = PACK_STREAM_OFFSET;
                let len = u32::from_le_bytes([resp[p], resp[p + 1], resp[p + 2], resp[p + 3]]);
                if len == 2 {
                    u16::from_le_bytes([resp[p + 4], resp[p + 5]])
                } else {
                    0
                }
            } else {
                0
            };
            if body_rc != POLL_SENTINEL && inner_rc != RESULT_NOT_READY {
                break;
            }
            if polls >= MAX_POLLS {
                return Err(IoError::Other(format!(
                    "Exportmaster: cursor still 'not ready' after {polls} Receive polls"
                )));
            }
            resp = framing::send_recv_auto(
                &mut self.stream,
                &super::msg::build_receive(),
                compression,
            )?;
            polls += 1;
        }
        super::response::check_body_reqcode("ExecuteStatement", &resp)?;

        // Phase 2: SetToBegin. The response carries the FIRST row's
        // bookmark — the seed for the GetNextRecord loop. Extract it
        // by walking the cursor-info units (bookmark is the 8th unit).
        let setbegin_resp = framing::send_recv_auto(
            &mut self.stream,
            &super::msg::build_set_to_begin(1),
            compression,
        )?;
        super::response::check_body_reqcode("SetToBegin", &setbegin_resp)?;
        // SetToBegin's response — unlike GetNextRecord — does NOT carry
        // a leading result-code unit; the 10 cursor-info units start
        // directly at PACK_STREAM_OFFSET.
        let starting_bookmark = {
            let mut w = Walker::new(&setbegin_resp, PACK_STREAM_OFFSET);
            let ci = CursorInfo::read(&mut w)?;
            ci.bookmark
        };

        // Phase 3: GetNextRecord loop.
        let mut builders = ColumnBuilders::new(&columns, target_rows.min(1024));
        let mut next_bookmark = starting_bookmark;
        let mut rows_seen: usize = 0;
        let request_batch = self.batch_size.min(50).max(1); // ODBC uses ~50

        'outer: loop {
            if rows_seen >= target_rows {
                break;
            }
            let body = super::msg::build_get_next_record(1, &next_bookmark, request_batch);
            let resp = framing::send_recv_auto(&mut self.stream, &body, compression)?;
            super::response::check_body_reqcode("GetNextRecord", &resp)?;
            let mut walker = Walker::new(&resp, PACK_STREAM_OFFSET);
            let mut got_eoc = false;
            let mut rows_in_batch = 0usize;

            loop {
                let saved = walker.position();
                let rc_unit = match walker.next_unit() {
                    Ok(Some(u)) => u,
                    Ok(None) => break,
                    Err(e) => {
                        // A length prefix running off the end is clean
                        // exhaustion only if what remains is alignment
                        // padding; otherwise the body is malformed.
                        if super::response::tail_is_padding(&resp, saved) {
                            break;
                        }
                        return Err(e);
                    }
                };
                if rc_unit.len() != 2 {
                    // Trailing alignment padding parses as a zero-length
                    // (or odd) unit — stop cleanly. Anything else is a
                    // shape we don't understand: refuse to guess.
                    if rc_unit.iter().all(|&b| b == 0)
                        && super::response::tail_is_padding(&resp, walker.position())
                    {
                        break;
                    }
                    return Err(IoError::Other(format!(
                        "Exportmaster: GetNextRecord: expected 2-byte result code, got {} \
                         after {rows_seen} rows",
                        rc_unit.len()
                    )));
                }
                let result_code = u16::from_le_bytes([rc_unit[0], rc_unit[1]]);
                if result_code == RESULT_END_OF_CURSOR {
                    got_eoc = true;
                    break;
                }
                if result_code == RESULT_NOT_READY {
                    break; // re-issue GetNextRecord
                }
                if result_code != RESULT_OK {
                    return Err(IoError::Other(format!(
                        "Exportmaster: GetNextRecord returned result code 0x{result_code:04X} \
                         after {rows_seen} rows — refusing to treat as end-of-cursor"
                    )));
                }
                let cursor_info = CursorInfo::read(&mut walker)?;
                let slot = match walker.next_unit()? {
                    Some(u) => u.to_vec(),
                    None => {
                        return Err(IoError::Other(format!(
                            "Exportmaster: GetNextRecord: row slot missing after {rows_seen} rows"
                        )));
                    }
                };
                if slot.len() < col_end_offset {
                    return Err(IoError::Other(format!(
                        "Exportmaster: GetNextRecord: row slot {} bytes, need {col_end_offset} \
                         (after {rows_seen} rows)",
                        slot.len()
                    )));
                }

                // Decode columns from slot[first_off..col_end_offset].
                let cells_raw = decode_record(&slot[first_off..col_end_offset], &columns)?;

                // For blob columns: OpenBlob + FreeBlob inline using
                // the slot as-is (it's already exactly what the server
                // expects for this row).
                let mut cells: Vec<CellValue> = Vec::with_capacity(cells_raw.len());
                for (ci, cell) in cells_raw.into_iter().enumerate() {
                    if blob_col_indices.contains(&ci) {
                        if let CellValue::BlobHandle(h) = cell {
                            if h == [0u8; 8] {
                                cells.push(CellValue::Null);
                            } else {
                                let outcome = blob::fetch_blob(self, 1, (ci as u16) + 1, &slot)?;
                                let free_body = super::msg::build_free_blob(
                                    1,
                                    (ci as u16) + 1,
                                    &outcome.slot_echo,
                                    0,
                                );
                                let _ = framing::send_recv_auto(
                                    &mut self.stream,
                                    &free_body,
                                    compression,
                                )?;
                                cells.push(CellValue::Binary(outcome.payload));
                            }
                            continue;
                        }
                    }
                    cells.push(cell);
                }

                builders.push_row(cells)?;
                next_bookmark = cursor_info.bookmark;
                rows_seen += 1;
                rows_in_batch += 1;
                if rows_seen >= target_rows {
                    break 'outer;
                }
            }

            if got_eoc || rows_in_batch == 0 {
                break;
            }
        }

        // Cleanup — same teardown sequence as the legacy path.
        let close_body = super::msg::build_close_cursor(1);
        let _ = framing::send_recv_auto(&mut self.stream, &close_body, compression);
        let reset_body = super::msg::build_reset_statement(1);
        let _ = framing::send_recv_auto(&mut self.stream, &reset_body, compression);
        let release_body = super::msg::build_remove_all_remote_memory_tables();
        let _ = framing::send_recv_auto(&mut self.stream, &release_body, compression);

        let batch = builders.finish()?;
        Ok(Value::Table(Table::from_arrow(batch)))
    }

    pub fn query_to_table_capped(&mut self, sql: &str, target_rows: usize) -> Result<Value, IoError> {
        let schema_resp = self.query_raw(sql)?;
        let (columns, _end_off) = parse_schema(&schema_resp)?;
        let first_off = columns[0].row_offset as usize;
        let last_col = columns.last().unwrap();
        let col_data_span = (last_col.row_offset as usize - first_off) + 1 + last_col.max as usize;
        let batch_size = self.batch_size;
        let compression = self.compression;
        let blob_slot_length = self.blob_slot_length;

        // Pre-compute which columns are blob/memo/graphic — for each
        // such column, every non-zero handle we see during the row
        // scan generates a deferred 0x0280 fetch that runs after the
        // cursor loop finishes (the cursor stays open until CloseCursor
        // below, so handles remain valid).
        let blob_col_indices: Vec<usize> = columns
            .iter()
            .enumerate()
            .filter(|(_, c)| matches!(c.field_type, FieldType::Blob | FieldType::Memo | FieldType::Graphic))
            .map(|(i, _)| i)
            .collect();
        let has_blobs = !blob_col_indices.is_empty();

        // The deferred blob path reconstructs each row's 0x0280 slot from
        // (phys, MD5, PK bytes), reading the PK from `row[first_off+1..]`
        // — i.e. it treats `columns[0]` as the table's primary key. That
        // holds only when the projection's first column is the leading
        // on-disk field, which protocol §4 establishes is always the PK
        // (offset == RECORD_HEADER_LEN). A projection like
        // `SELECT memo, code FROM t` makes `columns[0]` a non-PK (or the
        // blob itself), so the reconstructed slot would point the server
        // at the wrong record. Refuse rather than build a garbage slot;
        // the streaming default path (`query_to_table`) has no such
        // constraint because it echoes the server's slot verbatim.
        if has_blobs {
            validate_blob_projection(&columns)?;
        }

        if std::env::var("EM_SCHEMA_DEBUG").is_ok() {
            for (i, c) in columns.iter().enumerate() {
                eprintln!(
                    "[em-schema] col[{i}] name={} type={:?} decl={} max={} row_offset={}",
                    c.name, c.field_type, c.decl, c.max, c.row_offset
                );
            }
        }

        // Deferred blob fetches captured during the row callback. The
        // slot is the per-row bookmark the server sent alongside the
        // row — already in exactly the wire format 0x0280 expects —
        // so we hold it verbatim instead of reconstructing it from
        // MD5+PK (the §6a "build a slot from components" path is left
        // in `blob::build_slot` for tests but isn't used here; the
        // physical-record bookmark has 9 bytes of cursor-state prefix
        // and a trailing position field that can't be derived from row
        // data alone).
        struct BlobTask {
            row_idx: usize,
            col_idx: usize,
            md5: [u8; 16],
            pk_field: Vec<u8>,
            phys: u32,
        }
        let mut blob_tasks: Vec<BlobTask> = Vec::new();

        // Decode rows straight into the column builders as they
        // arrive — no per-row `Vec<u8>` allocation. The callback
        // borrows row bytes from the current response buffer.
        let mut builders = ColumnBuilders::new(&columns, target_rows.min(1024));
        let col_end_offset = first_off + col_data_span;
        let columns_ref = &columns;
        let blob_indices_ref = &blob_col_indices;
        let mut row_counter: usize = 0;
        let pk_field_width = if has_blobs { columns[0].max as usize } else { 0 };
        drive_cursor(self.stream_mut(), &columns, target_rows, batch_size, compression,
            |row: &[u8], bookmark: &[u8]| {
                if col_end_offset > row.len() {
                    return Ok(());
                }
                let mut cells = decode_record(&row[first_off..col_end_offset], columns_ref)?;

                // For blob columns: capture MD5 (row [9..25]), PK
                // column bytes (row [first_off+1 ..]), and the
                // PhysicalRecordNumber (extracted from the cursor's
                // per-row bookmark). The actual 56-byte slot is built
                // in the resolve loop where we know the final
                // slot_length (in case it differs from 56 — e.g. a
                // WHERE-filtered cursor returns a 72-byte slot).
                // Replace BlobHandle cells with Null so push_row writes
                // a None placeholder; the resolver overwrites it after
                // drive_cursor returns.
                if has_blobs {
                    if std::env::var("EM_ROW_DEBUG").is_ok() && (660..720).contains(&row_counter) {
                        eprintln!("[em-row] row={} ({} bytes): {}", row_counter, row.len(),
                            row.iter().take(80).map(|b| format!("{b:02x}")).collect::<String>());
                    }
                    for &ci in blob_indices_ref {
                        if let CellValue::BlobHandle(h) = cells[ci] {
                            if h != [0u8; 8] {
                                if row.len() < 25 || row.len() < first_off + 1 + pk_field_width {
                                    return Err(IoError::Other(format!(
                                        "Exportmaster: row too short for blob slot ({} bytes)",
                                        row.len()
                                    )));
                                }
                                let mut md5 = [0u8; 16];
                                md5.copy_from_slice(&row[9..25]);
                                let mut pk_field = vec![0u8; pk_field_width];
                                pk_field.copy_from_slice(&row[first_off + 1..first_off + 1 + pk_field_width]);
                                let phys = blob::physical_record_number_from_bookmark(bookmark);
                                blob_tasks.push(BlobTask {
                                    row_idx: row_counter,
                                    col_idx: ci,
                                    md5,
                                    pk_field,
                                    phys,
                                });
                            }
                            cells[ci] = CellValue::Null;
                        }
                    }
                }

                builders.push_row(cells)?;
                row_counter += 1;
                Ok(())
            })?;

        // Resolve deferred blob fetches while the cursor is still open.
        // One 0x0280 round-trip per non-null handle. Field ordinals are
        // 1-based on the wire (col_idx is 0-based). The bookmark we
        // collected during the row pass IS the slot the server expects
        // — no length-detect logic needed; the server is the source of
        // truth for the slot's wire bytes.
        let blob_debug = std::env::var("EM_BLOB_DEBUG").is_ok();
        let mut effective_slot_length = blob_slot_length;
        // Periodically bulk-evict the server-side blob buffer cache.
        // The cache is capacity-bounded (~640 entries observed); without
        // periodic FreeAllBlobs (0x0294) the cache fills and subsequent
        // OpenBlob responses come back as 0x2303 errors. Cap below the
        // observed limit by a wide margin.
        const FLUSH_EVERY: usize = 50;
        for (task_idx, task) in blob_tasks.iter().enumerate() {
            if task_idx > 0 && task_idx % FLUSH_EVERY == 0 {
                let flush_body = super::msg::build_free_all_blobs(1, 0);
                let _ = framing::send_recv_auto(self.stream_mut(), &flush_body, compression)?;
                if blob_debug {
                    eprintln!("[em-blob] FreeAllBlobs flush at task {}", task_idx);
                }
            }
            let mut slot = blob::build_slot(task.phys, &task.md5, &task.pk_field, effective_slot_length)?;
            if blob_debug {
                eprintln!(
                    "[em-blob] row={} col={} phys={} slot_len={}",
                    task.row_idx, task.col_idx, task.phys, effective_slot_length
                );
            }
            let mut outcome = blob::fetch_blob(
                self,
                1, // CURSOR_HANDLE in cursor.rs
                (task.col_idx as u16) + 1,
                &slot,
            )
            .map_err(|e| {
                let pk_str = std::str::from_utf8(&task.pk_field)
                    .map(|s| s.trim_end_matches('\0').to_string())
                    .unwrap_or_else(|_| "<non-utf8>".into());
                IoError::Other(format!(
                    "Exportmaster: blob fetch failed for row {} (col {}, pk {pk_str:?}, phys {}): {e:?} \
                     — refusing to emit a silent NULL for a non-empty blob handle",
                    task.row_idx, task.col_idx, task.phys
                ))
            })?;
            if blob_debug {
                eprintln!(
                    "[em-blob]   -> payload={} bytes, server_slot_len={}",
                    outcome.payload.len(), outcome.actual_slot_length
                );
            }
            if outcome.actual_slot_length != effective_slot_length {
                // Filtered/materialised cursors echo a wider slot
                // (e.g. 72 vs 56). Rebuild at the corrected length
                // and re-issue this task.
                effective_slot_length = outcome.actual_slot_length;
                slot = blob::build_slot(task.phys, &task.md5, &task.pk_field, effective_slot_length)?;
                outcome = blob::fetch_blob(
                    self,
                    1,
                    (task.col_idx as u16) + 1,
                    &slot,
                )?;
                if outcome.actual_slot_length != effective_slot_length {
                    return Err(IoError::Other(format!(
                        "Exportmaster: blob slot_length unstable after retry ({} then {})",
                        effective_slot_length, outcome.actual_slot_length
                    )));
                }
            }
            builders.overwrite_binary_cell(task.col_idx, task.row_idx, Some(outcome.payload))?;
            // FreeBlob (0x028A) is REQUIRED, not optional: the server's
            // per-cursor blob buffer cache is bounded (~256 buffers per
            // `TDBISAMEngine.SetMaxTableBlobBufferCount`). Without
            // FreeBlob after each OpenBlob, the cache fills up and
            // subsequent OpenBlob responses come back malformed
            // (empty size unit). Both dbsys.exe and the DBISAM ODBC
            // driver always send the pair.
            //
            // FreeBlob must use the server's ECHOED slot, not the
            // client's original — the server marks the cached buffer
            // with a few modified bytes (typically `01 fe ff ff ff`
            // where the request had `01 <u32 phys LE>`) and matches
            // FreeBlob against those marker bytes.
            let free_body = super::msg::build_free_blob(
                1,
                (task.col_idx as u16) + 1,
                &outcome.slot_echo,
                0,
            );
            let _ = framing::send_recv_auto(self.stream_mut(), &free_body, compression)?;
        }

        // Release the server-side cursor + materialised temp table.
        // The full sequence DBSYS uses to clear the pin that materialised
        // SELECTs leave on their source table (see KNOWN_BUGS.md B3 and
        // DBISAM-PROTOCOL.md §7f / §7l):
        //   1. CloseCursor (0x00A0) releases the cursor itself
        //   2. ResetStatement (0x0334) closes the statement transaction
        //   3. RemoveAllRemoteMemoryTables (0x0029) drops every temp
        //      table the session is holding — this is what decrements
        //      `TDataTable.UseCount` back to 0 on the source table, so
        //      a subsequent DROP / ALTER on the same table doesn't
        //      come back with `0x2B05 ExecuteError (locked)`.
        let close_body = super::msg::build_close_cursor(1);
        let _ = framing::send_recv_auto(self.stream_mut(), &close_body, compression);
        let reset_body = super::msg::build_reset_statement(1);
        let _ = framing::send_recv_auto(self.stream_mut(), &reset_body, compression);
        let release_body = super::msg::build_remove_all_remote_memory_tables();
        let _ = framing::send_recv_auto(self.stream_mut(), &release_body, compression);

        let batch = builders.finish()?;
        Ok(Value::Table(Table::from_arrow(batch)))
    }

    /// Issue a `SELECT … FROM <table>` and return the raw schema-bearing
    /// response (the first server message after the query packet). Used
    /// by callers that want to parse the schema themselves; also the
    /// hook the smoke test uses to exercise [`super::schema::parse`].
    pub fn query_raw(&mut self, sql: &str) -> Result<Vec<u8>, IoError> {
        let body = build_query_body(sql);
        framing::send_recv_auto(&mut self.stream, &body, self.compression)
    }

    /// Execute a DML statement (UPDATE / INSERT / DELETE) and return
    /// the affected row count.
    ///
    /// Wire flow reverse-engineered from a DBSYS UPDATE capture
    /// (`Derek/dbisam-capture-update.pcapng`, decoded via
    /// `Derek/decode_update.py`):
    ///
    /// 1. `PrepareStatement` (0x0320) carrying the SQL — same body
    ///    shape as the SELECT path.
    /// 2. `ExecuteStatement` (0x032A) — kicks off server-side work.
    /// 3. Loop: send `Receive` (0x030C) and read the response. While
    ///    the response reqcode is `PollNotReady` (0x2C14), keep
    ///    polling — the server's progress counter (offset 10 of the
    ///    inner body) ticks up by 5 each cycle, reaching 100 at
    ///    completion.
    /// 4. The first non-poll response carries two Pack units:
    ///    `<u32 len=8><8-byte TDateTime mtime>` then
    ///    `<u32 len=4><u32 affected_count>`.
    /// 5. `ResetStatement` (0x0334) finalises / commits the work.
    ///    Skipping it leaves the operation uncommitted — the server
    ///    rolls back on disconnect.
    pub fn execute_dml(&mut self, sql: &str, is_ddl: bool) -> Result<u32, IoError> {
        let debug = std::env::var("EM_DML_DEBUG").is_ok();
        if debug { eprintln!("[em-dml] sql: {sql:?}  is_ddl: {is_ddl}"); }

        // Begin-DML marker (0x0316) per DBISAM-PROTOCOL.md §7a.
        let begin_body = super::msg::build_begin_dml(1);
        let begin_resp = framing::send_recv_auto(&mut self.stream, &begin_body, self.compression)?;
        if debug { eprintln!("[em-dml] begin (0x0316) resp ({} bytes): {}", begin_resp.len(), hex_dump(&begin_resp)); }

        let prepare_body = build_query_body(sql);
        let prepare_resp =
            framing::send_recv_auto(&mut self.stream, &prepare_body, self.compression)?;
        if debug { eprintln!("[em-dml] prepare (0x0320) resp ({} bytes): {}", prepare_resp.len(), hex_dump(&prepare_resp)); }

        // PrepareError path (DBISAM-PROTOCOL.md §7f): server returns
        // 0x2B02 with the offending identifier (unknown table, bad column,
        // etc.). Skip ExecuteStatement, send ResetStatement to close the
        // transaction, surface the identifier in the error message.
        const PREPARE_ERROR: u16 = 0x2B02;
        if body_reqcode(&prepare_resp) == Some(PREPARE_ERROR) {
            let ident = parse_prepare_error(&prepare_resp)
                .unwrap_or_else(|| "<unparseable>".to_string());
            let reset_body = super::msg::build_reset_statement(1);
            let _ = framing::send_recv_auto(&mut self.stream, &reset_body, self.compression);
            return Err(IoError::Other(format!(
                "Exportmaster: DBISAM PrepareStatement rejected SQL — offending identifier: {ident:?}"
            )));
        }

        // The +23 byte of ExecuteStatement encodes "this statement
        // produces a result cursor" — false for pure DDL (no cursor),
        // true for DML which surfaces a cursor for the rows-affected
        // count. See `build_execute_statement{,_ddl}` doc-comments and
        // DBISAM-PROTOCOL.md §7h.
        let exec_body = if is_ddl {
            super::msg::build_execute_statement_ddl(1)
        } else {
            super::msg::build_execute_statement(1)
        };
        if debug { eprintln!("[em-dml] execute (0x032A) body ({} bytes): {}", exec_body.len(), hex_dump(&exec_body)); }
        let mut resp =
            framing::send_recv_auto(&mut self.stream, &exec_body, self.compression)?;
        if debug { eprintln!("[em-dml] execute (0x032A) resp ({} bytes): {}", resp.len(), hex_dump(&resp)); }

        // Catch the wider `0x2B__` error family (e.g. 0x2B05 ExecuteError
        // observed when DROP TABLE is rejected — see conversation
        // forensics 2026-05-27). Body shape matches PrepareError:
        // 8 zero bytes + length-prefixed identifier. Close the
        // transaction with ResetStatement and surface a real error.
        if let Some(code) = body_reqcode(&resp) {
            if (code & 0xFF00) == 0x2B00 && code != 0x2B02 {
                let ident = parse_prepare_error(&resp)
                    .unwrap_or_else(|| "<unparseable>".to_string());
                let reset_body = super::msg::build_reset_statement(1);
                let _ = framing::send_recv_auto(&mut self.stream, &reset_body, self.compression);
                return Err(IoError::Other(format!(
                    "Exportmaster: DBISAM ExecuteStatement rejected — reqcode 0x{code:04X}, identifier: {ident:?}"
                )));
            }
        }

        const POLL_NOT_READY: u16 = 0x2C14;
        const MAX_POLLS: usize = 600;
        let receive_body = super::msg::build_receive();
        let mut polls = 0;
        while body_reqcode(&resp) == Some(POLL_NOT_READY) {
            if polls >= MAX_POLLS {
                return Err(IoError::Other(format!(
                    "Exportmaster: DML still polling after {polls} Receive cycles"
                )));
            }
            resp =
                framing::send_recv_auto(&mut self.stream, &receive_body, self.compression)?;
            polls += 1;
        }

        let affected = parse_dml_result(&resp)?;

        let reset_body = super::msg::build_reset_statement(1);
        let reset_resp = framing::send_recv_auto(&mut self.stream, &reset_body, self.compression)?;
        if debug { eprintln!("[em-dml] reset (0x0334) resp ({} bytes): {}", reset_resp.len(), hex_dump(&reset_resp)); }

        Ok(affected)
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
        let query_resp = framing::send_recv_auto(&mut self.stream, &query_body, self.compression)?;

        // The count comes back across a few round-trips. The PoC sends
        // two follow-up messages after the query and then scans the
        // concatenated response for the 0x80 + 3-byte BE pattern.
        let mut combined = query_resp;
        for body in POST_QUERY_BODIES_COUNT {
            let r = framing::send_recv_auto(&mut self.stream, body, self.compression)?;
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
        let resp = framing::send_recv_auto(&mut self.stream, SQLTABLES_BODY, self.compression)?;
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
    /// See `DBISAM-PROTOCOL.md` §7 — decoding the cursor advance
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
            // `[Data]` resolves to a foldable `LazyOdbc` plan (DBISAM
            // dialect), so `Table.SelectRows`/`SelectColumns`/`FirstN`
            // push down into the SELECT instead of pulling the whole
            // table over the native wire. Schema is probed lazily on
            // `[Data]` access, mirroring the Odbc.DataSource bridge.
            let fetcher: Rc<dyn Fn() -> Result<Value, MError>> = Rc::new(move || {
                build_lazy_exportmaster_table(&opts_for_thunk, &table_for_thunk)
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

/// Build a foldable `LazyOdbc` table for one native-DBISAM table.
///
/// Probes the table's schema with a zero-row `… WHERE 1=0` SELECT — which
/// returns the column description without ever driving the cursor, so it
/// sidesteps the undecoded cursor-advance limitation that bites full-table
/// fetches (see `list_tables_as_navigation`). The returned plan carries the
/// `Dbisam` dialect; `render_sql` therefore emits DBISAM SQL (`TOP n`,
/// `#…#` dates) on force. Foldable `Table.*` ops narrow the plan first, so
/// only the filtered/projected rows cross the wire.
fn build_lazy_exportmaster_table(
    opts: &ConnOpts,
    table_name: &str,
) -> Result<Value, mrsflow_core::eval::MError> {
    use std::rc::Rc;

    use mrsflow_core::eval::{LazyOdbcState, MError, TableRepr};
    use mrsflow_core::plan::SqlDialect;

    let probe_sql = format!("SELECT * FROM \"{}\" WHERE 1=0", table_name);
    let mut probe_client = Client::connect_and_login(opts)
        .map_err(|e| MError::Other(format!("Exportmaster probe connect: {e:?}")))?;
    let probe = probe_client
        .query_to_table_capped(&probe_sql, 0)
        .map_err(|e| MError::Other(format!("Exportmaster probe: {e:?}")))?;
    let schema = match probe {
        Value::Table(t) => t
            .try_to_arrow()
            .map_err(|e| MError::Other(format!("Exportmaster probe schema: {e:?}")))?
            .schema(),
        _ => return Err(MError::Other("Exportmaster probe: expected table".into())),
    };

    let opts_for_force = opts.clone();
    let force_fn: Rc<dyn Fn(&str) -> Result<arrow::record_batch::RecordBatch, MError>> =
        Rc::new(move |sql: &str| {
            let mut c = Client::connect_and_login(&opts_for_force)
                .map_err(|e| MError::Other(format!("Exportmaster connect: {e:?}")))?;
            let v = c
                .query_to_table(sql)
                .map_err(|e| MError::Other(format!("Exportmaster fold force: {e:?}")))?;
            match v {
                Value::Table(t) => t
                    .try_to_arrow()
                    .map_err(|e| MError::Other(format!("Exportmaster force arrow: {e:?}"))),
                _ => Err(MError::Other("Exportmaster fold force: expected table".into())),
            }
        });

    let projection: Vec<usize> = (0..schema.fields().len()).collect();
    let state = LazyOdbcState {
        connection_string: opts.host.clone(),
        table_name: table_name.to_string(),
        schema,
        projection,
        output_names: None,
        where_filters: vec![],
        limit: None,
        dialect: SqlDialect::Dbisam,
        force_fn,
    };
    Ok(Value::Table(Table { repr: TableRepr::LazyOdbc(state) }))
}

/// Parse the bulk SQLTables response. `resp` is the body of the first
/// server message returned after `SQLTABLES_BODY`. Layout (per
/// `DBISAM-PROTOCOL.md` SQLTables-response section):
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

/// Render a body as space-separated hex bytes (capped) for debug logs.
fn hex_dump(body: &[u8]) -> String {
    const MAX: usize = 96;
    let n = body.len().min(MAX);
    let mut s = String::with_capacity(n * 3 + 16);
    for (i, b) in body[..n].iter().enumerate() {
        if i > 0 && i % 16 == 0 { s.push_str(" | "); }
        s.push_str(&format!("{b:02X} "));
    }
    if body.len() > MAX { s.push_str(&format!("... +{} more", body.len() - MAX)); }
    s
}

/// Validate a handshake-step response (Connect, Login, session setup,
/// catalog attach). Success responses carry body reqcode 0x0000;
/// failures use the 0x2Cxx session-error family — observed live
/// against rivsem01: 0x2C17 = login rejected, 0x2C1E = catalog attach
/// failed (body carries the offending name), 0x2C2C = request before
/// login. Previously these responses were discarded, so a bad password
/// surfaced later as a baffling schema-parse failure.
fn check_handshake_response(step: &str, body: &[u8]) -> Result<(), IoError> {
    let code = body_reqcode(body).ok_or_else(|| {
        IoError::Other(format!(
            "Exportmaster: {step}: response too short ({} bytes)",
            body.len()
        ))
    })?;
    if code == 0x0000 {
        return Ok(());
    }
    let mut msg = format!("Exportmaster: {step} rejected by server (reqcode 0x{code:04X})");
    if code == 0x2C17 {
        msg.push_str(" — login failed; check User / Password / EncryptPassword");
    }
    if let Some(ident) = super::response::error_identifier(body) {
        msg.push_str(&format!(" — server identifies: {ident:?}"));
    }
    Err(IoError::Other(msg))
}

/// Validate that a blob/memo extract via the deferred (capped) path can
/// safely reconstruct each row's 0x0280 slot. That path reads the PK
/// from `row[columns[0].row_offset + 1 ..]`, so it's correct only when
/// `columns[0]` is the table's primary key — which protocol §4
/// establishes is always the leading on-disk field (offset ==
/// `RECORD_HEADER_LEN`). Projections that don't lead with the PK, or
/// that put a blob in column 0, would build a slot pointing the server
/// at the wrong record; reject them with guidance.
fn validate_blob_projection(columns: &[super::schema::Column]) -> Result<(), IoError> {
    let lead = &columns[0];
    if lead.row_offset as usize != super::row::RECORD_HEADER_LEN {
        return Err(IoError::Other(format!(
            "Exportmaster: this blob/memo extract path requires the primary key to be the \
             first selected column, but column 0 is {:?} (on-disk offset {}, not the leading \
             field at {}). Lead the projection with the PK, or use Exportmaster.Query \
             (the default path handles any projection).",
            lead.name, lead.row_offset, super::row::RECORD_HEADER_LEN
        )));
    }
    if matches!(
        lead.field_type,
        FieldType::Blob | FieldType::Memo | FieldType::Graphic
    ) {
        return Err(IoError::Other(format!(
            "Exportmaster: column 0 ({:?}) is itself a blob/memo column; the blob extract \
             path needs a scalar primary key in column 0. Lead the projection with the PK.",
            lead.name
        )));
    }
    Ok(())
}

/// Extract the reqcode (u16 LE) from a response body. The body
/// layout is `[flag u8][reqcode u16 LE][inner_len u32 LE][...]`;
/// returns `None` if the body is too short to contain a reqcode.
fn body_reqcode(body: &[u8]) -> Option<u16> {
    if body.len() < 3 {
        None
    } else {
        Some(u16::from_le_bytes([body[1], body[2]]))
    }
}

/// Parse a DML final-response body into the affected row count.
/// Per DBISAM-PROTOCOL.md §7d, the inner section is:
///   +0   u32 LE = 8        length-prefix
///   +4   f64 LE            execution time in seconds (informational)
///   +12  u32 LE = 4        length-prefix
///   +16  u32 LE            rows affected
/// `body[0..7]` is the `[flag][reqcode][inner_len]` envelope; doc
/// offsets are relative to inner, so we add 7 to reach them.
fn parse_dml_result(body: &[u8]) -> Result<u32, IoError> {
    let count_off = 7 + 16;
    if body.len() < count_off + 4 {
        return Err(IoError::Other(format!(
            "Exportmaster: DML result body too short ({} bytes, need {}+)",
            body.len(),
            count_off + 4
        )));
    }
    Ok(u32::from_le_bytes([
        body[count_off],
        body[count_off + 1],
        body[count_off + 2],
        body[count_off + 3],
    ]))
}

/// Parse the offending identifier from a `PrepareError` (0x2B02) body.
/// Per DBISAM-PROTOCOL.md §7f, the inner section is:
///   +0..+8   8 zero bytes (timing slot, unused on parse failure)
///   +8       u32 LE identifier length
///   +12      ASCII identifier (table/column/keyword the parser choked on)
fn parse_prepare_error(body: &[u8]) -> Option<String> {
    if body.len() < 7 + 12 {
        return None;
    }
    let inner = &body[7..];
    let ident_len =
        u32::from_le_bytes([inner[8], inner[9], inner[10], inner[11]]) as usize;
    if ident_len == 0 || ident_len > 256 || 12 + ident_len > inner.len() {
        return None;
    }
    std::str::from_utf8(&inner[12..12 + ident_len])
        .ok()
        .map(String::from)
}

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

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper: assemble a fake response body `[flag][reqcode LE][inner_len LE][...inner]`
    /// for unit-testing the parsers in isolation.
    fn make_response(reqcode: u16, inner: &[u8]) -> Vec<u8> {
        let mut body = Vec::with_capacity(7 + inner.len());
        body.push(0x00); // flag
        body.extend_from_slice(&reqcode.to_le_bytes());
        body.extend_from_slice(&(inner.len() as u32).to_le_bytes());
        body.extend_from_slice(inner);
        body
    }

    #[test]
    fn body_reqcode_reads_bytes_1_and_2_as_u16_le() {
        let body = make_response(0x2C14, &[]);
        assert_eq!(body_reqcode(&body), Some(0x2C14));
        let body = make_response(0x2B05, &[]);
        assert_eq!(body_reqcode(&body), Some(0x2B05));
        // Too short → None.
        assert_eq!(body_reqcode(&[]), None);
        assert_eq!(body_reqcode(&[0x00, 0x14]), None);
    }

    #[test]
    fn parse_dml_result_extracts_affected_count_at_offset_16() {
        // Per DBISAM-PROTOCOL.md §7d:
        //   inner +0..+4   u32 LE = 8        length-prefix
        //   inner +4..+12  f64 LE            execution time (ignored here)
        //   inner +12..+16 u32 LE = 4        length-prefix
        //   inner +16..+20 u32 LE            rows affected
        let mut inner = Vec::new();
        inner.extend_from_slice(&8u32.to_le_bytes()); // len-prefix
        inner.extend_from_slice(&0.485f64.to_le_bytes()); // timing
        inner.extend_from_slice(&4u32.to_le_bytes()); // len-prefix
        inner.extend_from_slice(&9360u32.to_le_bytes()); // affected
        let body = make_response(0x0000, &inner);
        assert_eq!(parse_dml_result(&body).unwrap(), 9360);
    }

    #[test]
    fn parse_dml_result_rejects_truncated_body() {
        // 7-byte header alone, nothing inside.
        let body = make_response(0x0000, &[]);
        assert!(parse_dml_result(&body).is_err());
    }

    #[test]
    fn parse_prepare_error_extracts_identifier_at_offset_12() {
        // Per DBISAM-PROTOCOL.md §7f:
        //   inner +0..+8   8 zero bytes (unused timing slot)
        //   inner +8..+12  u32 LE identifier length
        //   inner +12..    identifier bytes
        let mut inner = Vec::new();
        inner.extend_from_slice(&[0u8; 8]); // zero timing
        inner.extend_from_slice(&8u32.to_le_bytes()); // ident length
        inner.extend_from_slice(b"MikaTest"); // ident
        inner.extend_from_slice(&[0u8; 4]); // trailing zeros (server pads)
        let body = make_response(0x2B02, &inner);
        assert_eq!(parse_prepare_error(&body), Some("MikaTest".to_string()));
    }

    #[test]
    fn validate_blob_projection_requires_pk_to_lead() {
        use crate::exportmaster::schema::{Column, FieldType};
        use crate::exportmaster::row::RECORD_HEADER_LEN;

        let col = |name: &str, ft: FieldType, row_offset: u16| Column {
            ord: 1,
            name: name.to_string(),
            field_type: ft,
            decl: 8,
            max: 8,
            row_offset,
        };

        // PK (scalar) leads at the on-disk header offset → OK.
        let ok = vec![
            col("CODE", FieldType::String, RECORD_HEADER_LEN as u16),
            col("LONGDESC", FieldType::Memo, 60),
        ];
        assert!(validate_blob_projection(&ok).is_ok());

        // Column 0 isn't the leading on-disk field (offset != 25) →
        // PK doesn't lead the projection → reject.
        let not_leading = vec![
            col("CPYNAME", FieldType::String, 60),
            col("LONGDESC", FieldType::Memo, 200),
        ];
        assert!(validate_blob_projection(&not_leading).is_err());

        // Column 0 is itself a blob → reject even at the right offset.
        let blob_first = vec![
            col("LONGDESC", FieldType::Memo, RECORD_HEADER_LEN as u16),
            col("CODE", FieldType::String, 60),
        ];
        assert!(validate_blob_projection(&blob_first).is_err());
    }

    #[test]
    fn parse_prepare_error_returns_none_for_oversize_or_bogus_length() {
        // Ridiculously large identifier length → reject.
        let mut inner = Vec::new();
        inner.extend_from_slice(&[0u8; 8]);
        inner.extend_from_slice(&999u32.to_le_bytes()); // > 256 cap
        inner.extend_from_slice(b"short");
        let body = make_response(0x2B02, &inner);
        assert_eq!(parse_prepare_error(&body), None);
    }
}
