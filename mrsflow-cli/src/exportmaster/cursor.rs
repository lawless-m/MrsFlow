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

/// Default batch size for ReadFirstRecordBlock / ReadNextRecordBlock.
/// Larger batches = fewer round-trips but more bytes in flight per
/// response. 500 is the Derek-suggested ballpark.
const DEFAULT_BATCH_SIZE: u32 = 500;

/// Max poll iterations on Receive before giving up. The server is
/// usually ready after 1-3 polls; 100 is well above any observed
/// preparation time for materialised cursors.
const MAX_RECEIVE_POLLS: usize = 100;

/// Drive a SELECT cursor to completion, accumulating row bytes into
/// `out_rows`. Each row is a self-contained `Vec<u8>` of the on-disk
/// record (header + column data) — caller slices from `+first_col.row_offset`
/// to call `decode_record`.
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
/// - `target_rows` rows have been collected, OR
/// - the max-iterations safety bound trips
pub fn drive_cursor(
    stream: &mut TcpStream,
    columns: &[Column],
    target_rows: usize,
    out_rows: &mut Vec<Vec<u8>>,
) -> Result<(), IoError> {
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

    // Parse a response body, harvesting rows into out_rows. Returns
    // true on end-of-cursor.
    let process_body = |body: &[u8],
                        kind: &ReplyKind,
                        out_rows: &mut Vec<Vec<u8>>|
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
            // Harvest rows even when result_code != OK: end-of-cursor
            // responses still carry the FINAL batch of rows.
            for row in &batch.rows {
                if out_rows.len() >= target_rows {
                    return Ok(true);
                }
                out_rows.push(row.to_vec());
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
    let r = framing::send_recv(stream, &msg::build_execute_statement(CURSOR_HANDLE))?;
    logr("ExecuteStatement", &r);
    if process_body(&r, &ReplyKind::SingleRow, out_rows)? {
        return Ok(());
    }

    // Phase 2: Receive poll loop. Re-issue Receive until the response
    // is no longer the polling sentinel (reqcode 0x2C14, result_code 3).
    // ExecuteStatement above counts as the first "poll" — its response
    // is also the sentinel, so the cursor is in the "preparing" state
    // when we get here.
    let mut poll_count = 0;
    loop {
        let r = framing::send_recv(stream, &msg::build_receive())?;
        logr(&format!("Receive[{}]", poll_count), &r);
        let is_sentinel = body_reqcode(&r) == REQCODE_POLLING_SENTINEL;
        // Also treat result_code 3 inside a "normal" response as "not
        // ready". The doc says the sentinel is reqcode 0x2C14 + result 3,
        // but both pieces are diagnostic.
        let is_not_ready_inner = {
            // Look at the first 2 bytes of the first Pack unit (the
            // result-code unit) without fully parsing.
            let pack_start = PACK_STREAM_OFFSET;
            if r.len() >= pack_start + 6 {
                let len = u32::from_le_bytes([r[pack_start], r[pack_start+1], r[pack_start+2], r[pack_start+3]]);
                if len == 2 {
                    let rc = u16::from_le_bytes([r[pack_start+4], r[pack_start+5]]);
                    rc == super::response::RESULT_NOT_READY
                } else { false }
            } else { false }
        };
        if process_body(&r, &ReplyKind::SingleRow, out_rows)? {
            return Ok(());
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
    // ReadFirstRecordBlock reads from the current position; without
    // SetToBegin, the cursor is at end-of-data and EoC is returned
    // immediately.
    let r = framing::send_recv(stream, &msg::build_set_to_begin(CURSOR_HANDLE))?;
    logr("SetToBegin", &r);
    if process_body(&r, &ReplyKind::SingleRow, out_rows)? {
        return Ok(());
    }

    // Phase 3: ReadFirstRecordBlock — first batch of rows.
    let first_n = (target_rows as u32).min(DEFAULT_BATCH_SIZE).max(1);
    let r = framing::send_recv(
        stream,
        &msg::build_read_first_record_block(CURSOR_HANDLE, first_n),
    )?;
    logr("ReadFirstRecordBlock", &r);
    if debug {
        // Dump full body to disk for offline analysis.
        let _ = std::fs::write(".em_tmp/rfb_resp.bin", &r);
        eprintln!("em:   (full body dumped to .em_tmp/rfb_resp.bin)");
    }
    if process_body(&r, &ReplyKind::RecordBlock, out_rows)? {
        return Ok(());
    }

    // Phase 4: ReadNextRecordBlock loop.
    let max_batches = (target_rows / DEFAULT_BATCH_SIZE as usize) + 10;
    for _ in 0..max_batches {
        if out_rows.len() >= target_rows {
            break;
        }
        let remaining = target_rows.saturating_sub(out_rows.len()) as u32;
        let n = remaining.min(DEFAULT_BATCH_SIZE).max(1);
        let r = framing::send_recv(
            stream,
            &msg::build_read_next_record_block(CURSOR_HANDLE, n),
        )?;
        if process_body(&r, &ReplyKind::RecordBlock, out_rows)? {
            break;
        }
    }

    Ok(())
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

