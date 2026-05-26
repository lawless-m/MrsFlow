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

/// One server "batch" within a response: a result code, the 10
/// cursor-info fields, and zero-or-more row records.
#[derive(Debug)]
pub struct CursorBatch<'a> {
    pub result_code: u16,
    pub cursor_info: CursorInfo,
    /// Slices into the response body — each is one on-disk record of
    /// `record_size` bytes (header + column data). Empty if the result
    /// code wasn't OK.
    pub rows: Vec<&'a [u8]>,
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
    let Some(rc_unit) = walker.next_unit()? else {
        return Ok(None);
    };
    if rc_unit.len() != 2 {
        return Err(IoError::Other(format!(
            "Exportmaster: expected 2-byte result code, got {}",
            rc_unit.len()
        )));
    }
    let result_code = u16::from_le_bytes([rc_unit[0], rc_unit[1]]);
    let cursor_info = CursorInfo::read(walker)?;

    let mut rows = Vec::new();
    let mut locked_size: Option<usize> = None;
    if result_code == RESULT_OK {
        loop {
            let saved = walker.position();
            let Some(unit) = walker.next_unit()? else { break };
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
    }

    Ok(Some(CursorBatch {
        result_code,
        cursor_info,
        rows,
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
    let Some(rc_unit) = walker.next_unit()? else {
        return Ok(None);
    };
    if rc_unit.len() != 2 {
        return Err(IoError::Other(format!(
            "Exportmaster: ReadRecordBlock: expected 2-byte result code, got {}",
            rc_unit.len()
        )));
    }
    let result_code = u16::from_le_bytes([rc_unit[0], rc_unit[1]]);
    let cursor_info = CursorInfo::read(walker)?;

    let mut rows = Vec::new();
    // Row count: 4-byte u32 LE.
    if let Some(count_unit) = walker.next_unit()? {
        if count_unit.len() == 4 {
            let row_count = u32::from_le_bytes([
                count_unit[0], count_unit[1], count_unit[2], count_unit[3],
            ]) as usize;
            // Row buffer: row_count × actual_record_size bytes packed.
            // We trust the wire's actual size — the schema-derived
            // expected_record_size is sometimes off by a few bytes
            // (trailing padding the formula doesn't account for).
            if let Some(row_buf) = walker.next_unit()? {
                if row_count > 0 && row_buf.len() % row_count == 0 {
                    let actual_record_size = row_buf.len() / row_count;
                    // Sanity: actual size shouldn't be wildly different
                    // from schema. Allow ±32 bytes — covers any trailing
                    // padding without admitting nonsense values.
                    let lo = expected_record_size.saturating_sub(32);
                    let hi = expected_record_size + 32;
                    if actual_record_size >= lo && actual_record_size <= hi {
                        for i in 0..row_count {
                            let start = i * actual_record_size;
                            rows.push(&row_buf[start..start + actual_record_size]);
                        }
                    }
                }
                // Bookmark buffer: per-row bookmarks. Consume but ignore.
                let _ = walker.next_unit()?;
            }
        }
    }

    Ok(Some(CursorBatch {
        result_code,
        cursor_info,
        rows,
    }))
}
