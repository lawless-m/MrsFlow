//! Cursor advance: post-query message sequence + multi-fetch loop.
//!
//! After the QUERY (`0x0320`) is sent, the server only returns the schema.
//! The actual row data arrives across several `(cursor-init, batch)`
//! round-trips. This module drives those round-trips and returns the
//! concatenated bytes for the row parser to chew on.
//!
//! Strategy (v1, matches PoC `customer-top3` mode):
//! 1. Send 11 captured cursor-init messages — replays from a known-good
//!    `select code, cpyname, contact, email from customer top 3` session.
//!    The first batch of rows is in these responses.
//! 2. Loop: scan the concatenated responses for the highest-seen primary
//!    key in the first column, splice it into captured ACK + Fetch
//!    templates, send. Server returns the next batch.
//! 3. Stop when `row_count >= target` or two consecutive iterations make
//!    no progress (cursor exhausted).
//!
//! See protocol §6 for the universal cursor-advance rule documented but
//! not yet implemented here. The captured-template approach handles
//! natural-PK and indexed-JOIN cases; ORDER BY and unindexed-JOIN need
//! the universal rule (TODO).

use std::net::TcpStream;

use mrsflow_core::eval::IoError;

use super::framing;
use super::row::{decode_record, CellValue, RECORD_HEADER_LEN};
use super::schema::Column;

/// Captured cursor-init message sequence (PoC `POST_QUERY_CUSTOMER_TOP3`).
/// 11 messages; replayed verbatim on every query. The 16-byte primary-key
/// slots inside some of these messages happen to contain customer codes
/// from the original capture (`1`, `1-680`) — they're substituted with
/// the actual PK as we advance via [`splice_key_into_template`].
///
/// Bytes lifted verbatim from `dbisam_client.py:POST_QUERY_CUSTOMER_TOP3`.
const CURSOR_INIT_BODIES: &[&[u8]] = &[
    &[
        0x00, 0x2A, 0x03, 0x22, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01,
        0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x50, 0x59, 0x4E,
    ],
    &[
        0x00, 0x0C, 0x03, 0x05, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00,
    ],
    &[
        0x00, 0xBE, 0x00, 0x08, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x02,
        0x00, 0x00, 0x00, 0x00,
    ],
    &[
        0x00, 0xFA, 0x00, 0x2F, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
        0x00, 0x00, 0x00, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x80, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x43, 0x54, 0x2C, 0x20, 0x45, 0x4D,
    ],
    &[
        0x00, 0xFA, 0x00, 0x2F, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
        0x00, 0x00, 0x00, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x80, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x43, 0x54, 0x2C, 0x20, 0x45, 0x4D,
    ],
    &[
        0x00, 0x54, 0x01, 0x27, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
        0x00, 0x00, 0x00, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x80, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x20, 0x00,
    ],
    &[
        0x00, 0x04, 0x01, 0x2F, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
        0x00, 0x00, 0x00, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x80, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x20, 0x00, 0x00, 0x00, 0x43, 0x54, 0x2C, 0x20, 0x45, 0x4D,
    ],
    &[
        0x00, 0x54, 0x01, 0x27, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
        0x00, 0x00, 0x00, 0x01, 0x31, 0x2D, 0x36, 0x38, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x80, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x20, 0x00,
    ],
    &[
        0x00, 0xFA, 0x00, 0x2F, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
        0x00, 0x00, 0x00, 0x01, 0x31, 0x2D, 0x36, 0x38, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x80, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x21, 0x00, 0x00, 0x00, 0x43, 0x54, 0x2C, 0x20, 0x45, 0x4D,
    ],
    &[
        0x00, 0x54, 0x01, 0x27, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
        0x00, 0x00, 0x00, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x80, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x21, 0x00,
    ],
    &[
        0x00, 0x04, 0x01, 0x2F, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
        0x00, 0x00, 0x00, 0x01, 0x31, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
        0x80, 0x00, 0x00, 0x01, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
        0x00, 0x00, 0x21, 0x00, 0x00, 0x00, 0x43, 0x54, 0x2C, 0x20, 0x45, 0x4D,
    ],
];

/// ACK template (PoC `ACK_TEMPLATE_C14`, 52 bytes). The 16-byte PK
/// slot is at offset 20.
const ACK_TEMPLATE: &[u8] = &[
    0x00, 0x54, 0x01, 0x27, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
    0x00, 0x00, 0x00, 0x01, 0x31, 0x2D, 0x36, 0x38, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x80, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
    0x00, 0x00, 0x21, 0x00,
];

/// Fetch template (PoC `FETCH_TEMPLATE_C15`, 60 bytes). The 16-byte PK
/// slot is at offset 20.
const FETCH_TEMPLATE: &[u8] = &[
    0x00, 0xFA, 0x00, 0x2F, 0x00, 0x00, 0x00, 0x04, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x11,
    0x00, 0x00, 0x00, 0x01, 0x31, 0x2D, 0x36, 0x38, 0x30, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01,
    0x80, 0x00, 0x00, 0x03, 0x01, 0x00, 0x00, 0x00, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x04, 0x00,
    0x00, 0x00, 0x21, 0x00, 0x00, 0x00, 0x43, 0x54, 0x2C, 0x20, 0x45, 0x4D,
];

/// Offset of the 16-byte primary-key slot in both ACK_TEMPLATE and
/// FETCH_TEMPLATE.
const KEY_SLOT_OFFSET: usize = 20;
const KEY_SLOT_LEN: usize = 16;

/// Drive a SELECT cursor to completion, returning concatenated server
/// bytes that contain the rows.
///
/// `target_rows` is a soft cap — we stop once we've decoded that many
/// unique rows. Set to `usize::MAX` for "all".
///
/// Returns the concatenated server responses (post-query messages +
/// fetch responses). The caller scans them via [`find_row_starts`] +
/// [`decode_record`] to materialise rows.
pub fn drive_cursor(
    stream: &mut TcpStream,
    columns: &[Column],
    target_rows: usize,
) -> Result<Vec<u8>, IoError> {
    let mut combined = Vec::new();

    // Phase 1: send the 11 captured cursor-init messages.
    for body in CURSOR_INIT_BODIES {
        let r = framing::send_recv(stream, body)?;
        combined.extend_from_slice(&r);
    }

    // Phase 2: loop, splicing the last-seen PK into ACK + Fetch templates.
    let mut prev_count = 0usize;
    let mut empty_iters = 0usize;
    let max_iters = target_rows.saturating_add(50);
    for _ in 0..max_iters {
        let row_starts = find_row_starts(&combined, columns);
        let unique = count_unique_first_col(&combined, &row_starts, columns);
        if unique >= target_rows {
            break;
        }
        if unique == prev_count {
            empty_iters += 1;
            if empty_iters > 3 {
                break; // cursor exhausted
            }
        } else {
            empty_iters = 0;
        }
        prev_count = unique;

        let last_key = last_first_col_value(&combined, &row_starts, columns)
            .unwrap_or_else(|| b"1".to_vec());

        let ack = splice_key_into_template(ACK_TEMPLATE, &last_key);
        let r_ack = framing::send_recv(stream, &ack)?;
        combined.extend_from_slice(&r_ack);

        let fetch = splice_key_into_template(FETCH_TEMPLATE, &last_key);
        let r_fetch = framing::send_recv(stream, &fetch)?;
        combined.extend_from_slice(&r_fetch);
    }

    Ok(combined)
}

/// Printable ASCII range, including space and the common code-separator
/// chars (`-`, `_`, `.`, `*`, `/`, alphanumerics). Used to filter
/// false-positive row_starts in pattern-matching.
fn is_printable_ascii(b: u8) -> bool {
    matches!(b, 0x20..=0x7E)
}

/// Splice a primary key into the 16-byte key slot of a captured template.
/// Key is left-aligned, null-padded.
fn splice_key_into_template(template: &[u8], key: &[u8]) -> Vec<u8> {
    let mut out = template.to_vec();
    let n = key.len().min(KEY_SLOT_LEN);
    for byte in &mut out[KEY_SLOT_OFFSET..KEY_SLOT_OFFSET + KEY_SLOT_LEN] {
        *byte = 0;
    }
    out[KEY_SLOT_OFFSET..KEY_SLOT_OFFSET + n].copy_from_slice(&key[..n]);
    out
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
pub fn find_row_starts(data: &[u8], columns: &[Column]) -> Vec<usize> {
    if columns.is_empty() {
        return Vec::new();
    }
    let first_off = columns[0].row_offset as usize;
    let last_col = columns.last().unwrap();
    // Wire row size from first column's null-flag through last column's
    // last value byte = (last.null_pos - first.null_pos) + 1 + max.
    let row_size = (last_col.row_offset as usize - first_off) + 1 + last_col.max as usize;
    if data.len() < row_size {
        return Vec::new();
    }
    let mut out = Vec::new();
    let last_idx = data.len() - row_size;
    for i in 0..=last_idx {
        let cells_ok = columns.iter().all(|c| {
            let null_pos = i + (c.row_offset as usize - first_off);
            null_pos < data.len() && (data[null_pos] == 0 || data[null_pos] == 1)
        });
        if !cells_ok {
            continue;
        }
        // First column (PK) must be non-null.
        if data[i] != 1 {
            continue;
        }
        // Additional filter to drop false positives that pattern-match
        // null-flag positions in framing/zero-padding regions: for the
        // PK column (column 0), the first value byte must be a printable
        // ASCII character. PKs in DBISAM tables are universally text
        // codes (CUSTOMER.CODE, PRODUCT.CODE, etc.) — letters, digits,
        // and a few separator chars. Anything else means we've matched
        // a pattern in framing bytes, not a real record.
        let first_byte = if i + 1 < data.len() { data[i + 1] } else { 0 };
        if !is_printable_ascii(first_byte) {
            continue;
        }
        out.push(i);
    }
    out
}

/// Decode the first column of every row at `starts` and return the
/// number of unique non-empty values. Used as the PoC's "progress"
/// metric for the cursor loop.
fn count_unique_first_col(data: &[u8], starts: &[usize], columns: &[Column]) -> usize {
    use std::collections::HashSet;
    let mut seen = HashSet::new();
    let first_off = columns[0].row_offset as usize;
    let last_col = columns.last().unwrap();
    let row_size = (last_col.row_offset as usize - first_off) + 1 + last_col.max as usize;
    for &s in starts {
        let end = s + row_size;
        if end > data.len() {
            continue;
        }
        if let Ok(cells) = decode_record(&data[s..end], &columns[..1]) {
            if let Some(CellValue::Text(text)) = cells.into_iter().next() {
                if !text.is_empty() {
                    seen.insert(text);
                }
            }
        }
    }
    seen.len()
}

/// Return the first-column text of the last row in physical order.
/// "Last" = highest offset, since server batches arrive in cursor-advance
/// order (the protocol's natural-PK mode).
fn last_first_col_value(
    data: &[u8],
    starts: &[usize],
    columns: &[Column],
) -> Option<Vec<u8>> {
    let first_off = columns[0].row_offset as usize;
    let last_col = columns.last().unwrap();
    let row_size = (last_col.row_offset as usize - first_off) + 1 + last_col.max as usize;
    for &s in starts.iter().rev() {
        let end = s + row_size;
        if end > data.len() {
            continue;
        }
        if let Ok(cells) = decode_record(&data[s..end], &columns[..1]) {
            if let Some(CellValue::Text(text)) = cells.into_iter().next() {
                if !text.is_empty() {
                    return Some(text.into_bytes());
                }
            }
        }
    }
    None
}
