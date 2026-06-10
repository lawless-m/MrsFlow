//! Decode the structure of a cursor-response body per protocol §6c–§6f.
//!
//! Body layout (after `framing::recv_msg` has stripped the
//! `<GUID><total_len>` envelope):
//!
//! ```text
//! +0   u8        header flag (always 0x00)
//! +1   u16 LE    reqcode
//! +3   u32 LE    body_len (bytes from this point — not strictly verified)
//! +7              Pack stream begins here:
//!     <u32 length=2><u16 LE result_code>     OK = 0x0000, end-of-cursor = 0x2202
//!     10 cursor-info Pack units (see cursor_info.rs)
//!     [row record Pack units] each <u32 length=record_size><record bytes>
//!     ...repeats per server batch (multiple rows per response possible)
//! ```
//!
//! End-of-cursor: `result_code != 0` means "no rows to follow" — close
//! the cursor and stop driving it.

use mrsflow_core::eval::IoError;

use super::cursor_info::CursorInfo;
use super::wire::Walker;

/// Result code returned in the first Pack unit of a cursor-response body.
/// Documented values:
/// - `0x0000` — OK, the response carries cursor-info + rows
/// - `0x0003` — not ready, the cursor is still materialising server-side;
///              caller should re-issue `Receive (0x030C)` and check again
/// - `0x2202` — end-of-cursor / no more rows
pub const RESULT_OK: u16 = 0x0000;
pub const RESULT_NOT_READY: u16 = 0x0003;
pub const RESULT_END_OF_CURSOR: u16 = 0x2202;

/// Reqcode the server emits in its body header when responding with a
/// "not ready, poll again" status. Not in the client→server dispatch
/// table — it's a server-pushed status marker (per ANSWERS-TO-DEREK-2.md
/// Q4). The body's first Pack unit then carries `RESULT_NOT_READY`.
pub const REQCODE_POLLING_SENTINEL: u16 = 0x2C14;

/// Offset within the body where the Pack stream begins (after the
/// 7-byte body header `<u8 flag><u16 LE reqcode><u32 LE body_len>`).
pub const PACK_STREAM_OFFSET: usize = 7;

/// Read the body header's reqcode (u16 LE at offset 1). Used to spot
/// the polling sentinel before trying to parse the body as a normal
/// cursor response.
pub fn body_reqcode(body: &[u8]) -> u16 {
    if body.len() < 3 {
        return 0;
    }
    u16::from_le_bytes([body[1], body[2]])
}

/// Error if the body's reqcode is in one of the server's error families:
/// `0x2Bxx` statement errors (PrepareError/ExecuteError, §7f) or `0x2Cxx`
/// session errors — observed live against rivsem01: 0x2C17 = login
/// rejected, 0x2C1E = catalog attach failed, 0x2C2C = request before
/// login. The 0x2C14 polling sentinel is a status, not an error.
pub fn check_body_reqcode(context: &str, body: &[u8]) -> Result<(), IoError> {
    let code = body_reqcode(body);
    let family = code & 0xFF00;
    if family == 0x2B00 || (family == 0x2C00 && code != REQCODE_POLLING_SENTINEL) {
        let detail = error_identifier(body)
            .map(|s| format!(" — server identifies: {s:?}"))
            .unwrap_or_default();
        return Err(IoError::Other(format!(
            "Exportmaster: {context}: server error reqcode 0x{code:04X}{detail}"
        )));
    }
    Ok(())
}

/// Extract the offending identifier from an error body, if present.
/// Error bodies carry `<zero padding><u32 LE len><ASCII identifier>`
/// after the 7-byte header; the zero run is 4 bytes for session errors
/// (0x2C1E attach failure, observed live) and 8 bytes for statement
/// errors (0x2B02/0x2B05, §7f — zeroed timing slot).
pub(crate) fn error_identifier(body: &[u8]) -> Option<String> {
    let inner = body.get(7..)?;
    for zeros in [4usize, 8] {
        if inner.len() < zeros + 4 || inner[..zeros].iter().any(|&b| b != 0) {
            continue;
        }
        let len =
            u32::from_le_bytes(inner[zeros..zeros + 4].try_into().ok()?) as usize;
        if len == 0 || len > 256 || zeros + 4 + len > inner.len() {
            continue;
        }
        if let Ok(s) = std::str::from_utf8(&inner[zeros + 4..zeros + 4 + len]) {
            return Some(s.to_string());
        }
    }
    None
}

/// True if everything from `pos` to the end of the buffer is zero —
/// the 8-byte alignment padding both sides append to framed bodies
/// (`wrap()` does it client-side; the server does the same). A walker
/// error over such a tail is clean exhaustion, not a malformed body.
pub(crate) fn tail_is_padding(buf: &[u8], pos: usize) -> bool {
    buf[pos.min(buf.len())..].iter().all(|&b| b == 0)
}

/// One server "batch" within a response: a result code, the 10
/// cursor-info fields, and zero-or-more row records.
#[derive(Debug)]
pub struct CursorBatch<'a> {
    pub result_code: u16,
    /// Present for OK batches; non-OK bodies (not-ready, server error,
    /// sometimes a bare end-of-cursor) don't reliably carry one.
    pub cursor_info: Option<CursorInfo>,
    /// Slices into the response body — each is one on-disk record of
    /// `record_size` bytes (header + column data). Empty if the result
    /// code wasn't OK.
    pub rows: Vec<&'a [u8]>,
    /// Per-row bookmarks (one per row, same length as `rows`). Each is
    /// the row's physical-record bookmark — the exact byte sequence the
    /// server expects in the `slot` field of a 0x0280 OpenBlob request.
    /// `bookmarks[i]` corresponds to `rows[i]`. Empty for the single-row
    /// (`read_batch`) path; populated by `read_record_block_batch`.
    pub bookmarks: Vec<&'a [u8]>,
}

/// Parse one cursor batch starting at `walker`'s current position.
/// Reads: 1 result-code unit + 10 cursor-info units + N row-record units.
///
/// `expected_record_size` is what the schema arithmetic predicts the
/// per-row Pack length to be. It's used to recognise row records (vs
/// the next batch's result code or trailing units) — if the actual
/// first row size differs by 1-2 bytes (which happens because the
/// schema-derived formula sometimes underestimates by trailing
/// padding bytes), the actual size from the first row is adopted and
/// returned via the batch.
///
/// Heuristic for "is this a row?": length matches expected exactly,
/// or is within `expected ± 2` and we haven't seen a row yet (first-
/// row size disambiguation). Once one row is seen, subsequent units
/// must match that exact size.
///
/// Used for single-row responses (GetNextRecord). For batched
/// responses (ReadFirstRecordBlock / ReadNextRecordBlock), use
/// `read_record_block_batch` instead — the row buffer comes packed
/// inside a single Pack unit there.
pub fn read_batch<'a>(
    walker: &mut Walker<'a>,
    expected_record_size: usize,
) -> Result<Option<CursorBatch<'a>>, IoError> {
    let start = walker.position();
    let rc_unit = match walker.next_unit() {
        Ok(Some(u)) => u,
        Ok(None) => return Ok(None),
        Err(e) => {
            if tail_is_padding(walker.buf(), start) {
                return Ok(None);
            }
            return Err(e);
        }
    };
    if rc_unit.len() != 2 {
        // Not a batch body: SetToBegin-style responses open with a
        // 4-byte cursor-info field, and some bodies end in alignment
        // padding. Shape dispatch, not an error — rewind and decline.
        walker.seek(start);
        return Ok(None);
    }
    let result_code = u16::from_le_bytes([rc_unit[0], rc_unit[1]]);
    if result_code != RESULT_OK {
        // Non-OK bodies (not-ready, end-of-cursor, server errors) don't
        // reliably carry cursor-info or rows — a bare not-ready body is
        // just the result-code unit. Surface the code; the caller
        // decides whether it means poll-again, stop, or error.
        return Ok(Some(CursorBatch {
            result_code,
            cursor_info: None,
            rows: Vec::new(),
            bookmarks: Vec::new(),
        }));
    }
    let cursor_info = CursorInfo::read(walker)?;

    let mut rows = Vec::new();
    let mut locked_size: Option<usize> = None;
    loop {
        let saved = walker.position();
        let unit = match walker.next_unit() {
            Ok(Some(u)) => u,
            Ok(None) => break,
            Err(e) => {
                if tail_is_padding(walker.buf(), saved) {
                    break;
                }
                return Err(e);
            }
        };
        let matches = match locked_size {
            Some(s) => unit.len() == s,
            None => {
                let lo = expected_record_size.saturating_sub(2);
                let hi = expected_record_size + 2;
                if unit.len() >= lo && unit.len() <= hi {
                    locked_size = Some(unit.len());
                    true
                } else {
                    false
                }
            }
        };
        if matches {
            rows.push(unit);
        } else {
            walker.seek(saved);
            break;
        }
    }

    Ok(Some(CursorBatch {
        result_code,
        cursor_info: Some(cursor_info),
        rows,
        bookmarks: Vec::new(),
    }))
}

/// Parse a `ReadFirstRecordBlock` / `ReadNextRecordBlock` response
/// (reqcode 0x050A / 0x04F6). Layout decoded empirically from a live
/// CUSTOMER TOP 3 response:
///
/// ```text
/// Unpack 2 bytes          ← result code (0x0000 OK, 0x2202 last batch)
/// UnpackCursorInfo        ← 10-field standard cursor info
/// Unpack 4 bytes          ← row count in this batch (u32 LE)
/// Unpack <N × record_size bytes>   ← all rows concatenated, no inner framing
/// Unpack <N × bookmark_size bytes> ← all bookmarks concatenated (we ignore)
/// ```
///
/// Note: this differs from `ANSWERS-TO-DEREK-2.md` Q2's description
/// (which said each row is its own `<u32 record_size><bytes>` unit).
/// Empirically the rows arrive as ONE big buffer with row_count × row_size
/// bytes; the row-size is implicit from the schema.
///
/// Per the EoC convention, the last batch carries `result_code = 0x2202`
/// AND still contains rows — caller harvests rows regardless of
/// result_code, then exits on EoC.
pub fn read_record_block_batch<'a>(
    walker: &mut Walker<'a>,
    expected_record_size: usize,
) -> Result<Option<CursorBatch<'a>>, IoError> {
    let start = walker.position();
    let rc_unit = match walker.next_unit() {
        Ok(Some(u)) => u,
        Ok(None) => return Ok(None),
        Err(e) => {
            if tail_is_padding(walker.buf(), start) {
                return Ok(None);
            }
            return Err(e);
        }
    };
    if rc_unit.len() != 2 {
        if rc_unit.iter().all(|&b| b == 0) && tail_is_padding(walker.buf(), walker.position()) {
            return Ok(None); // alignment padding, clean exhaustion
        }
        return Err(IoError::Other(format!(
            "Exportmaster: ReadRecordBlock: expected 2-byte result code, got {}",
            rc_unit.len()
        )));
    }
    let result_code = u16::from_le_bytes([rc_unit[0], rc_unit[1]]);

    let empty = |result_code| CursorBatch {
        result_code,
        cursor_info: None,
        rows: Vec::new(),
        bookmarks: Vec::new(),
    };
    if result_code == RESULT_OK {
        let (cursor_info, rows, bookmarks) =
            read_block_payload(walker, expected_record_size)?;
        return Ok(Some(CursorBatch {
            result_code,
            cursor_info: Some(cursor_info),
            rows,
            bookmarks,
        }));
    }
    if result_code == RESULT_END_OF_CURSOR {
        // The last batch usually carries the full cursor-info + rows
        // shape ("0x2202 AND still contains rows"), but an empty result
        // set may answer with just the bare result code — tolerate that.
        let saved = walker.position();
        return Ok(Some(match read_block_payload(walker, expected_record_size) {
            Ok((cursor_info, rows, bookmarks)) => CursorBatch {
                result_code,
                cursor_info: Some(cursor_info),
                rows,
                bookmarks,
            },
            Err(_) => {
                walker.seek(saved);
                empty(result_code)
            }
        }));
    }
    // Not-ready or an unrecognised code — surface it; the caller
    // decides whether it means poll-again or a hard error.
    Ok(Some(empty(result_code)))
}

/// Parse the payload of an OK record-block batch: cursor-info, row
/// count, packed row buffer, packed bookmark buffer. Errors loudly on
/// any shape mismatch — a malformed mid-stream batch must not be
/// mistaken for end-of-cursor (that silently truncates the result).
fn read_block_payload<'a>(
    walker: &mut Walker<'a>,
    expected_record_size: usize,
) -> Result<(CursorInfo, Vec<&'a [u8]>, Vec<&'a [u8]>), IoError> {
    let cursor_info = CursorInfo::read(walker)?;

    let count_unit = walker.next_unit()?.ok_or_else(|| {
        IoError::Other("Exportmaster: ReadRecordBlock: missing row-count unit".to_string())
    })?;
    if count_unit.len() != 4 {
        return Err(IoError::Other(format!(
            "Exportmaster: ReadRecordBlock: row-count unit expected 4 bytes, got {}",
            count_unit.len()
        )));
    }
    let row_count = u32::from_le_bytes([
        count_unit[0], count_unit[1], count_unit[2], count_unit[3],
    ]) as usize;

    // Row buffer: row_count × actual_record_size bytes packed. We trust
    // the wire's actual size — the schema-derived expected_record_size
    // is sometimes off by a few bytes (trailing padding the formula
    // doesn't account for) — but only within ±32 bytes of the schema.
    let row_buf = walker.next_unit()?.ok_or_else(|| {
        IoError::Other("Exportmaster: ReadRecordBlock: missing row buffer".to_string())
    })?;
    let mut rows = Vec::new();
    let mut bookmarks = Vec::new();
    if row_count > 0 {
        if row_buf.len() % row_count != 0 {
            return Err(IoError::Other(format!(
                "Exportmaster: ReadRecordBlock: {}-byte row buffer not divisible by row count {row_count}",
                row_buf.len()
            )));
        }
        let actual_record_size = row_buf.len() / row_count;
        let lo = expected_record_size.saturating_sub(32);
        let hi = expected_record_size + 32;
        if actual_record_size < lo || actual_record_size > hi {
            return Err(IoError::Other(format!(
                "Exportmaster: ReadRecordBlock: wire record size {actual_record_size} \
                 implausible (schema predicts {expected_record_size})"
            )));
        }
        for i in 0..row_count {
            let start = i * actual_record_size;
            rows.push(&row_buf[start..start + actual_record_size]);
        }

        // Per-row bookmarks buffer. Each is `cursor.@+0x3672` bytes —
        // the slot the server expects in a 0x0280 OpenBlob request for
        // that row. Size isn't sent separately; it's
        // `bookmark_buf.len() / row_count`.
        let bookmark_buf = walker.next_unit()?.ok_or_else(|| {
            IoError::Other("Exportmaster: ReadRecordBlock: missing bookmark buffer".to_string())
        })?;
        if bookmark_buf.len() % row_count != 0 {
            return Err(IoError::Other(format!(
                "Exportmaster: ReadRecordBlock: {}-byte bookmark buffer not divisible by row count {row_count}",
                bookmark_buf.len()
            )));
        }
        let per_row = bookmark_buf.len() / row_count;
        for i in 0..row_count {
            let start = i * per_row;
            bookmarks.push(&bookmark_buf[start..start + per_row]);
        }
        if std::env::var("EM_BATCH_DEBUG").is_ok() {
            eprintln!(
                "[em-batch] rows={} bookmark_buf={} bytes (~{per_row}/row)",
                rows.len(),
                bookmark_buf.len(),
            );
        }
    }
    Ok((cursor_info, rows, bookmarks))
}
