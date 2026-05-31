//! Memo / blob fetch — reqcode 0x0280. See protocol §6a.
//!
//! Blob columns (sub-type 3, max_size 8) carry only an 8-byte handle on
//! the row; the content is fetched in a separate round-trip when the
//! application reads the field. This module issues one 0x0280 per
//! (cursor, field, row) tuple and returns the inline payload.
//!
//! Wire format authoritative — derived from disassembling
//! `TDataCursor.OpenBlob` (RVA 0x0ADFAC), `TDataCursor.ReadBlob`
//! (RVA 0x0AE624), and `TServerThread.DoOpenBlob` (RVA 0x04ED60) in
//! `dbisamr439delphi7.bpl`. The whole payload arrives in a single
//! response; the server never paginates remote blob reads regardless
//! of size.

use mrsflow_core::eval::IoError;

use super::framing;
use super::msg;
use super::response::PACK_STREAM_OFFSET;
use super::wire::Walker;
use super::Client;

/// Build the 56-byte physical-record bookmark — the slot the server
/// expects in a 0x0280 OpenBlob request. Layout reverse-engineered
/// from a verified dbsys.exe capture (`Derek/dbisam-capture-memo.pcapng`,
/// reqcode 0x0280, NIINGRED row NIEAN=`00715677478441`):
///
/// ```text
/// Pos 0       0x00 (record-active flag)
/// Pos 1..4    u32 LE PhysicalRecordNumber
/// Pos 5..8    u32 LE PhysicalRecordNumber (repeated)
/// Pos 9..24   16-byte row MD5 (= record bytes [9..25])
/// Pos 25      0x01 (PK column null flag)
/// Pos 26..    PK column raw bytes (1 byte * `pk_field_width`)
///             — for ftString PK with max=14, that's 13 chars + 0 pad
/// (middle)    zero padding for (slot_length - 16 - 9 - 1 - pk_w - 14) bytes
/// Pos L-14    0x01 (second null flag / bookmark marker)
/// Pos L-13..  u32 LE PhysicalRecordNumber
/// Pos L-9..   u32 LE PhysicalRecordNumber
/// Pos L-5..   5 bytes 0x00 (trailing pad)
/// ```
///
/// PhysicalRecordNumber comes from the cursor's 22-byte per-row
/// bookmark at offset 18: bytes 18..22 = `<high-bit-flag><3-byte BE
/// value>`. Strip the high bit of byte 18 and read the 4 bytes as
/// big-endian to recover the row's physical number.
///
/// `slot_length` is mode-dependent: 56 for natural-PK cursors on
/// short-PK tables; 72 for WHERE-filtered/materialised cursors. Caller
/// (typically `query_to_table_capped`) supplies it from the failing
/// fetch's response slot-echo if not known up front.
pub fn build_slot(
    physical_record_number: u32,
    row_md5: &[u8; 16],
    pk_field_bytes: &[u8],
    slot_length: usize,
) -> Result<Vec<u8>, IoError> {
    let trailer_len = 14; // 0x01 + 4 + 4 + 5
    let header_len = 9; // 0x00 + 4 + 4
    let md5_len = 16;
    let pk_block_len = 1 + pk_field_bytes.len(); // null flag + PK column bytes
    let used = header_len + md5_len + pk_block_len + trailer_len;
    if used > slot_length {
        return Err(IoError::Other(format!(
            "Exportmaster: slot_length={slot_length} too small for 56-byte layout with {}-byte PK field",
            pk_field_bytes.len()
        )));
    }
    let mut slot = vec![0u8; slot_length];
    // Header.
    slot[0] = 0x00;
    slot[1..5].copy_from_slice(&physical_record_number.to_le_bytes());
    slot[5..9].copy_from_slice(&physical_record_number.to_le_bytes());
    // MD5.
    slot[9..25].copy_from_slice(row_md5);
    // PK column.
    slot[25] = 0x01;
    slot[26..26 + pk_field_bytes.len()].copy_from_slice(pk_field_bytes);
    // (middle padding is already zero — slot was initialised to zeros)
    // Trailer.
    let t = slot_length - trailer_len;
    slot[t] = 0x01;
    slot[t + 1..t + 5].copy_from_slice(&physical_record_number.to_le_bytes());
    slot[t + 5..t + 9].copy_from_slice(&physical_record_number.to_le_bytes());
    // Last 5 bytes already zero.
    Ok(slot)
}

/// Extract `PhysicalRecordNumber` from a 22-byte cursor bookmark unit.
/// The position is encoded at offset 18 as `<u8 with high bit set><3 bytes BE value>`
/// — the §3 "high-bit-tagged integer" form. Strip the high bit and
/// read the 4 bytes as big-endian.
///
/// Returns 0 if the bookmark is too short or malformed (typical
/// non-natural-PK cursors don't carry a usable position here).
pub fn physical_record_number_from_bookmark(bookmark: &[u8]) -> u32 {
    if bookmark.len() < 22 {
        return 0;
    }
    let b0 = bookmark[18] & 0x7F;
    u32::from_be_bytes([b0, bookmark[19], bookmark[20], bookmark[21]])
}

/// Outcome of one 0x0280 round-trip: the payload bytes plus the server's
/// actual slot_length (echoed back as the first response unit). The
/// caller compares the echo length against the request — if they differ,
/// the server expected a different `cursor.@+0x3672` and the result
/// payload is empty/garbage (server looked up a row that doesn't exist
/// because part of the slot was uninitialised). Rebuild the slot with
/// `actual_slot_length` and retry; cache the value for subsequent
/// fetches in the same query.
#[derive(Debug)]
pub struct BlobFetchOutcome {
    pub payload: Vec<u8>,
    pub actual_slot_length: usize,
    /// The slot bytes echoed back by the server — NOT identical to the
    /// slot the request sent. The server modifies a few trailing bytes
    /// (typically `01 fe ff ff ff` where the request had
    /// `01 <u32 phys LE>`) as an "open in cache" marker. The 0x028A
    /// FreeBlob request must use these echoed bytes verbatim or the
    /// server won't find the buffer to free, leading the per-cursor
    /// blob cache to fill up and eventually corrupt OpenBlob responses.
    pub slot_echo: Vec<u8>,
}

/// Fetch one blob payload from the server. The slot bytes identify the
/// row (build via [`build_slot`]); `field_ord` is the 1-based column
/// ordinal of the blob column.
///
/// Returns the raw blob bytes plus the server's actual slot_length. If
/// the latter doesn't match `slot.len()`, the payload is garbage —
/// rebuild the slot and retry. See [`BlobFetchOutcome`].
pub fn fetch_blob(
    client: &mut Client,
    cursor_handle: u32,
    field_ord: u16,
    slot: &[u8],
) -> Result<BlobFetchOutcome, IoError> {
    let body = msg::build_open_blob(cursor_handle, field_ord, slot, 0, 0);
    let compression = client.compression();
    let resp = framing::send_recv_auto(client.stream_mut(), &body, compression)?;
    parse_open_blob_response(&resp)
}

/// Parse a 0x0280 response body. Per §6a the body after the 7-byte
/// header is 3 Pack units:
///   1. Slot echo (`cursor.@+0x3672` bytes — the server's expected
///      slot_length, independent of what the request sent)
///   2. `<u32 4><u32 blob_size>` — total payload size as a 4-byte u32
///   3. `<u32 blob_size><blob_size bytes>` — the payload
pub(crate) fn parse_open_blob_response(body: &[u8]) -> Result<BlobFetchOutcome, IoError> {
    if body.len() < PACK_STREAM_OFFSET {
        return Err(IoError::Other(format!(
            "Exportmaster: blob response too short ({} bytes)",
            body.len()
        )));
    }
    let mut w = Walker::new(body, PACK_STREAM_OFFSET);

    let slot_echo = w.next_unit()?.ok_or_else(|| {
        IoError::Other("Exportmaster: blob response missing slot echo unit".to_string())
    })?;
    let actual_slot_length = slot_echo.len();
    let slot_echo_owned = slot_echo.to_vec();

    let size_unit = w.next_unit()?.ok_or_else(|| {
        IoError::Other("Exportmaster: blob response missing size unit".to_string())
    })?;
    if size_unit.len() != 4 {
        return Err(IoError::Other(format!(
            "Exportmaster: blob size unit expected 4 bytes, got {}",
            size_unit.len()
        )));
    }
    let blob_size = u32::from_le_bytes([size_unit[0], size_unit[1], size_unit[2], size_unit[3]])
        as usize;

    let payload = w.next_unit()?.ok_or_else(|| {
        IoError::Other("Exportmaster: blob response missing payload unit".to_string())
    })?;
    if payload.len() != blob_size {
        return Err(IoError::Other(format!(
            "Exportmaster: blob payload length {} doesn't match declared size {}",
            payload.len(),
            blob_size
        )));
    }
    Ok(BlobFetchOutcome {
        payload: payload.to_vec(),
        actual_slot_length,
        slot_echo: slot_echo_owned,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_slot_matches_captured_wire() {
        // Verified against `Derek/dbisam-capture-memo.pcapng` C[18]:
        // for NIINGRED row NIEAN="0071567747844" (13 chars in a 14-byte
        // column, PhysicalRecordNumber=5), the wire slot is the
        // 56-byte sequence below. Build it from components and
        // byte-compare.
        let phys = 5u32;
        let md5: [u8; 16] = [
            0xa2, 0x8d, 0x18, 0xe6, 0x39, 0xee, 0xa2, 0xfb,
            0x75, 0x0c, 0xdb, 0x26, 0x61, 0x3c, 0xca, 0x3a,
        ];
        // 14-byte PK column: 13 ASCII chars + 1 zero pad.
        let mut pk_field = [0u8; 14];
        pk_field[..13].copy_from_slice(b"0071567747844");
        let slot = build_slot(phys, &md5, &pk_field, 56).unwrap();
        let expected: &[u8] = &[
            0x00, 0x05, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
            0xa2, 0x8d, 0x18, 0xe6, 0x39, 0xee, 0xa2, 0xfb, 0x75, 0x0c, 0xdb, 0x26, 0x61, 0x3c, 0xca, 0x3a,
            0x01, 0x30, 0x30, 0x37, 0x31, 0x35, 0x36, 0x37, 0x37, 0x34, 0x37, 0x38, 0x34, 0x34, 0x00,
            0x00, 0x00,
            0x01, 0x05, 0x00, 0x00, 0x00, 0x05, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00,
        ];
        assert_eq!(slot.len(), 56);
        assert_eq!(&slot[..], expected);
    }

    #[test]
    fn build_slot_rejects_too_small() {
        let md5 = [0u8; 16];
        let pk = [0u8; 14];
        // 9 header + 16 MD5 + 1 + 14 PK + 14 trailer = 54. So 53 is too small.
        assert!(build_slot(1, &md5, &pk, 53).is_err());
        assert!(build_slot(1, &md5, &pk, 54).is_ok());
    }

    #[test]
    fn extract_phys_from_bookmark() {
        // Verified against the same capture: cursor bookmark (22 bytes)
        // for the NIEAN="0071567747844" row has phys=5 at bytes 18..22
        // encoded as `80 00 00 05` (high-bit-flagged BE).
        let bookmark: [u8; 22] = [
            0x01, 0x30, 0x30, 0x37, 0x31, 0x35, 0x36, 0x37, 0x37, 0x34, 0x37, 0x38, 0x34, 0x34, 0x00,
            0x00, 0x00,
            0x01, 0x80, 0x00, 0x00, 0x05,
        ];
        assert_eq!(physical_record_number_from_bookmark(&bookmark), 5);
    }

    #[test]
    fn extract_phys_handles_large_values() {
        let mut bookmark = [0u8; 22];
        // PhysicalRecordNumber = 100000 = 0x000186A0; encoded BE in
        // bytes 19..22 (byte 18 high-bit set as the flag).
        bookmark[18] = 0x80;
        bookmark[19] = 0x01;
        bookmark[20] = 0x86;
        bookmark[21] = 0xA0;
        assert_eq!(physical_record_number_from_bookmark(&bookmark), 100_000);
    }

    fn pack_unit(buf: &mut Vec<u8>, payload: &[u8]) {
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(payload);
    }

    #[test]
    fn parse_response_decodes_payload_and_returns_actual_slot_length() {
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0x80, 0x02, 0, 0, 0, 0]); // 7-byte header
        pack_unit(&mut body, &[0xAAu8; 56]); // slot echo (actual = 56)
        pack_unit(&mut body, &12u32.to_le_bytes()); // <u32 4><u32 12>
        pack_unit(&mut body, b"Hello world!"); // payload
        let out = parse_open_blob_response(&body).unwrap();
        assert_eq!(out.payload, b"Hello world!");
        assert_eq!(out.actual_slot_length, 56);
    }

    #[test]
    fn parse_response_errors_on_payload_size_mismatch() {
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0x80, 0x02, 0, 0, 0, 0]);
        pack_unit(&mut body, &[0u8; 56]);
        pack_unit(&mut body, &20u32.to_le_bytes()); // declared 20 bytes
        pack_unit(&mut body, b"short"); // but only 5 follow
        assert!(parse_open_blob_response(&body).is_err());
    }

    #[test]
    fn parse_response_surfaces_mismatched_slot_echo() {
        // Server's actual slot_length differs from what the caller
        // sent — payload is empty/garbage, caller must rebuild slot
        // and retry. parse_open_blob_response itself doesn't error;
        // it just surfaces the actual length via BlobFetchOutcome.
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0x80, 0x02, 0, 0, 0, 0]);
        pack_unit(&mut body, &[0u8; 72]); // server echoed 72 even though we sent something else
        pack_unit(&mut body, &0u32.to_le_bytes());
        pack_unit(&mut body, b"");
        let out = parse_open_blob_response(&body).unwrap();
        assert_eq!(out.actual_slot_length, 72);
        assert!(out.payload.is_empty());
    }

    #[test]
    fn parse_response_handles_empty_blob() {
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0x80, 0x02, 0, 0, 0, 0]);
        pack_unit(&mut body, &[0u8; 56]);
        pack_unit(&mut body, &0u32.to_le_bytes()); // size = 0
        pack_unit(&mut body, b""); // empty payload
        let out = parse_open_blob_response(&body).unwrap();
        assert!(out.payload.is_empty());
        assert_eq!(out.actual_slot_length, 56);
    }
}
