//! Cursor advance: post-query message sequence + batched fetch loop.
//!
//! Per Derek's disassembly (ANSWERS-TO-DEREK-2.md), the correct flow
//! after PrepareStatement (0x0320) returns the schema is:
//!
//! ```text
//! 0x032A ExecuteStatement       ← kicks off cursor execution
//! 0x030C Receive (LOOP)         ← poll until server signals "ready"
//! 0x050A ReadFirstRecordBlock   ← batched read, N rows in one round-trip
//! 0x04F6 ReadNextRecordBlock    ← repeat until end-of-cursor
//! 0x00A0 CloseCursor            ← cleanup
//! ```
//!
//! The Receive poll is critical for ARCVCFG and other composite-key /
//! materialised cursors — the server returns reqcode `0x2C14` with
//! `result_code = RESULT_NOT_READY (0x0003)` while it's still preparing
//! the result set, and we have to re-issue Receive until we get back
//! a proper cursor-info response.
//!
//! All bodies are generated via `msg::` builders. Cursor handle is
//! `1` per-session (sequential, only one cursor per Client).

use std::net::TcpStream;

use mrsflow_core::eval::IoError;

use super::framing;
use super::msg;
use super::schema::Column;


/// Per-session cursor handle. Currently always 1 since we open at most
/// one cursor per Client (one Client per query). Per Derek's analysis,
/// this is sequential per-session, not per-query.
const CURSOR_HANDLE: u32 = 1;


/// Max poll iterations on Receive before giving up. The server is
/// usually ready after 1-3 polls; 100 is well above any observed
/// preparation time for materialised cursors.
const MAX_RECEIVE_POLLS: usize = 100;

/// Drive a SELECT cursor to completion, invoking `on_row` once per
/// decoded row. The row bytes are passed as a borrowed slice into the
/// response buffer — the callback must consume them in place (e.g.
/// decode into Arrow builders) since they're invalidated when the
/// next batch arrives.
///
/// Returns the number of rows actually delivered.
///
/// Flow (per `ANSWERS-TO-DEREK-2.md`):
/// 1. `ExecuteStatement (0x032A)` — kicks off cursor execution
/// 2. `Receive (0x030C)` loop until server stops sending the
///    "not ready, poll again" sentinel (reqcode 0x2C14, result_code 3)
/// 3. `ReadFirstRecordBlock (0x050A)` — first batch of rows
/// 4. `ReadNextRecordBlock (0x04F6)` loop until end-of-cursor
///
/// Stops when:
/// - a response carries `RESULT_END_OF_CURSOR (0x2202)`, OR
/// - `target_rows` rows have been delivered, OR
/// - the max-iterations safety bound trips
pub fn drive_cursor(
    stream: &mut TcpStream,
    columns: &[Column],
    target_rows: usize,
    batch_size: u32,
    compression: bool,
    mut on_row: impl FnMut(&[u8]) -> Result<(), IoError>,
) -> Result<usize, IoError> {
    use super::response::{
        body_reqcode, read_batch, read_record_block_batch, PACK_STREAM_OFFSET,
        REQCODE_POLLING_SENTINEL, RESULT_OK,
    };
    use super::wire::Walker;

    /// Which response shape to expect when parsing a server reply.
    enum ReplyKind {
        /// Single-row response (GetNextRecord etc): cursor-info + N
        /// row Pack units each `record_size` bytes.
        SingleRow,
        /// Batched response (ReadFirstRecordBlock / ReadNextRecordBlock):
        /// cursor-info + 1 Pack buffer containing N rows + 2 trailing
        /// buffers (bookmarks, flags).
        RecordBlock,
    }

    let record_size = compute_record_size(columns);
    let mut rows_seen: usize = 0;

    // Parse a response body, invoking `on_row` once per row. Returns
    // true on end-of-cursor (response carries result_code != OK, or
    // we've hit target_rows). Borrows row bytes directly from `body`
    // — no per-row allocation.
    let mut process_body = |body: &[u8],
                            kind: &ReplyKind,
                            rows_seen: &mut usize,
                            on_row: &mut dyn FnMut(&[u8]) -> Result<(), IoError>|
     -> Result<bool, IoError> {
        if body.len() < PACK_STREAM_OFFSET + 6 || body_reqcode(body) == REQCODE_POLLING_SENTINEL {
            return Ok(false);
        }
        let mut walker = Walker::new(body, PACK_STREAM_OFFSET);
        loop {
            let batch_res = match kind {
                ReplyKind::SingleRow => read_batch(&mut walker, record_size),
                ReplyKind::RecordBlock => read_record_block_batch(&mut walker, record_size),
            };
            let batch = match batch_res {
                Ok(Some(b)) => b,
                Ok(None) | Err(_) => return Ok(false),
            };
            for row in &batch.rows {
                if *rows_seen >= target_rows {
                    return Ok(true);
                }
                on_row(row)?;
                *rows_seen += 1;
            }
            if batch.result_code != RESULT_OK {
                return Ok(true);
            }
            if walker.position() >= body.len() {
                return Ok(false);
            }
        }
    };

    let debug = std::env::var("EM_DEBUG").is_ok();
    let logr = |label: &str, r: &[u8]| {
        if debug {
            eprintln!("em: {} resp={} bytes rc=0x{:04x} first 80: {}", label, r.len(),
                body_reqcode(r),
                r[..r.len().min(80)].iter().map(|b| format!("{:02x}", b)).collect::<String>());
        }
    };

    // Phase 1: ExecuteStatement.
    let r = framing::send_recv_auto(stream, &msg::build_execute_statement(CURSOR_HANDLE), compression)?;
    logr("ExecuteStatement", &r);
    if process_body(&r, &ReplyKind::SingleRow, &mut rows_seen, &mut on_row)? {
        return Ok(rows_seen);
    }

    // Phase 2: Receive poll loop.
    let mut poll_count = 0;
    loop {
        let r = framing::send_recv_auto(stream, &msg::build_receive(), compression)?;
        logr(&format!("Receive[{}]", poll_count), &r);
        let is_sentinel = body_reqcode(&r) == REQCODE_POLLING_SENTINEL;
        let is_not_ready_inner = {
            let pack_start = PACK_STREAM_OFFSET;
            if r.len() >= pack_start + 6 {
                let len = u32::from_le_bytes([r[pack_start], r[pack_start+1], r[pack_start+2], r[pack_start+3]]);
                if len == 2 {
                    let rc = u16::from_le_bytes([r[pack_start+4], r[pack_start+5]]);
                    rc == super::response::RESULT_NOT_READY
                } else { false }
            } else { false }
        };
        if process_body(&r, &ReplyKind::SingleRow, &mut rows_seen, &mut on_row)? {
            return Ok(rows_seen);
        }
        if !is_sentinel && !is_not_ready_inner {
            break;
        }
        poll_count += 1;
        if poll_count >= MAX_RECEIVE_POLLS {
            return Err(IoError::Other(format!(
                "Exportmaster: cursor still 'not ready' after {MAX_RECEIVE_POLLS} Receive polls"
            )));
        }
    }

    // Phase 2.5: SetToBegin to position the cursor at row 1.
    let r = framing::send_recv_auto(stream, &msg::build_set_to_begin(CURSOR_HANDLE), compression)?;
    logr("SetToBegin", &r);
    if process_body(&r, &ReplyKind::SingleRow, &mut rows_seen, &mut on_row)? {
        return Ok(rows_seen);
    }

    // Phase 3: ReadFirstRecordBlock — first batch of rows.
    let first_n = (target_rows as u32).min(batch_size).max(1);
    let r = framing::send_recv_auto(
        stream,
        &msg::build_read_first_record_block(CURSOR_HANDLE, first_n),
        compression,
    )?;
    logr("ReadFirstRecordBlock", &r);
    if process_body(&r, &ReplyKind::RecordBlock, &mut rows_seen, &mut on_row)? {
        return Ok(rows_seen);
    }

    // Phase 4: ReadNextRecordBlock loop.
    let max_batches = (target_rows / batch_size as usize) + 10;
    for _ in 0..max_batches {
        if rows_seen >= target_rows {
            break;
        }
        let remaining = target_rows.saturating_sub(rows_seen) as u32;
        let n = remaining.min(batch_size).max(1);
        let r = framing::send_recv_auto(
            stream,
            &msg::build_read_next_record_block(CURSOR_HANDLE, n),
            compression,
        )?;
        if process_body(&r, &ReplyKind::RecordBlock, &mut rows_seen, &mut on_row)? {
            break;
        }
    }

    Ok(rows_seen)
}

/// Pattern-match row starts in a byte blob.
///
/// Schema arithmetic (confirmed against live wire capture):
/// - `row_offset` is the position of the field's **null-flag byte**
///   within the on-disk record.
/// - Each field is `1 (null-flag) + max (value)` bytes, so the next
///   field's null-flag is at `prev.row_offset + prev.max + 1`.
/// - On the wire, the 25-byte on-disk record header is absent; null-flag
///   positions are wire-relative = `row_offset - first_col.row_offset`.
///
/// A candidate `i` is a row start iff every column's null-flag byte
/// (at `i + row_offset - first_off`) is 0x00 or 0x01, AND the first
/// column's flag is 0x01 (PK always present).
/// Find row data starts deterministically via the universal `<u32 LE
/// length><payload>` framing rule (protocol §6c, derived from
/// `TDataSession.Unpack` BPL disassembly).
///
/// Every piece of data on the wire is `<u32 LE length><length bytes>`.
/// Row records are one such unit with `length == record_size`, where:
///
/// ```text
/// record_size = last_col.row_offset + last_col.max + 1
/// ```
///
/// `record_size` is the full on-disk record width — it INCLUDES the
/// 25-byte on-disk header (because `last_col.row_offset` is in on-disk
/// coordinates that count from the start of the header). The first
/// column's null-flag therefore lives at `byte_after_length_prefix +
/// columns[0].row_offset`, which for CUSTOMER is `+25`.
///
/// Returns offsets into `data` pointing at the **first column's null-
/// flag** (skipping past both the 4-byte length prefix and the 25-byte
/// on-disk header), so that `decode_record` can be called directly on
/// `&data[off..off + col_data_span]`.
///
/// Algorithm: scan the response, read `u32 LE` at each position; when
/// it equals `record_size` treat the next `record_size` bytes as one
/// row record. Non-matching lengths are skipped — they're cursor
/// metadata (also length-prefixed but variable-size).
pub fn find_row_starts_via_framing(data: &[u8], columns: &[Column]) -> Vec<usize> {
    let record_size = compute_record_size(columns);
    let header_len = columns[0].row_offset as usize;
    let mut out = Vec::new();
    let mut i = 0;
    while i + 4 + record_size <= data.len() {
        let length = u32::from_le_bytes([data[i], data[i + 1], data[i + 2], data[i + 3]]) as usize;
        if length == record_size {
            out.push(i + 4 + header_len);
            i += 4 + record_size;
        } else {
            i += 1;
        }
    }
    out
}

/// Total on-disk record width per protocol §6c: row_offset of last
/// column + that column's max bytes + 1 byte for its null-flag.
pub(super) fn compute_record_size(columns: &[Column]) -> usize {
    let last = columns.last().expect("schema must have at least one column");
    last.row_offset as usize + last.max as usize + 1
}

