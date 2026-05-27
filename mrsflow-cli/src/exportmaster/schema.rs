//! Schema parser — decodes the 772-byte column-block region of a SELECT
//! response into typed [`Column`] descriptors. See protocol §4 + §6b.
//!
//! PoC reference: dbisam_client.py L411-435 (`parse schema`), and the
//! full type table in `DBISAM-PROTOCOL.md` §6b.

use mrsflow_core::eval::IoError;

/// Width of one column-descriptor block in the schema region.
pub const SCHEMA_BLOCK_STRIDE: usize = 772;

/// Marker bytes at the start of every block: `03 00 00`.
const BLOCK_MARKER: [u8; 3] = [0x03, 0x00, 0x00];

/// DBISAM field type codes, mapped from the `sub` byte at +0xA7 in each
/// column block. Protocol doc §6b is the source of truth.
///
/// Three of these (Blob, Integer, Float) are refined by an auxiliary
/// byte at +0xA8 or +0x250 within the same block — see [`FieldType`]'s
/// constructor for how that's resolved.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FieldType {
    /// `sub=0` — calculated/derived; no storage. Skip when parsing rows.
    Calculated,
    /// `sub=1` — ftString. ASCII chars, null-padded to `decl`.
    String,
    /// `sub=2` — ftDate. 4-byte LE u32, days since 0001-01-01.
    Date,
    /// `sub=3` + `+0xA8=0x00` — ftBlob (binary). 8-byte handle → §6a fetch.
    Blob,
    /// `sub=3` + `+0xA8=0x16` — ftMemo (text). Same handle; decode bytes as text.
    Memo,
    /// `sub=3` + `+0xA8=0x1A` — ftGraphic (image bytes).
    Graphic,
    /// `sub=4` — ftBoolean. 2-byte LE WordBool: FFFF=true, 0000=false.
    /// (DBISAM's enum, NOT Delphi-standard which puts ftWord at 4.)
    Boolean,
    /// `sub=5` — ftSmallint. 2-byte LE signed int.
    Smallint,
    /// `sub=6` + `+0xA8=0x00` — ftInteger. 4-byte LE signed int.
    Integer,
    /// `sub=6` + `+0xA8=0x1D` — ftAutoInc. 4-byte LE auto-increment.
    AutoInc,
    /// `sub=7` + `+0x250=0x0A` — ftCurrency. 8-byte LE Int64 / 10000.
    Currency,
    /// `sub=7` + `+0x250=0x00` — ftFloat. 8-byte LE IEEE 754 binary64.
    Float,
    /// `sub=9` — ftBytes. N raw bytes (fixed-length binary).
    Bytes,
    /// `sub=10` — ftTime. 4-byte LE u32, milliseconds since midnight.
    Time,
    /// `sub=11` — ftDateTime. 8-byte LE binary64, days since 1899-12-30
    /// (Delphi TDateTime; fractional part = time of day).
    DateTime,
    /// `sub=15` — ftVarBytes. Up to N bytes, length-prefixed.
    VarBytes,
    /// `sub=18` — ftLargeint. 8-byte LE signed int.
    Largeint,
    /// Anything we haven't seen — surfaces as an error during row parse.
    Unknown(u8, u8, u8),
}

impl FieldType {
    /// Resolve a field type from the per-block `sub` byte plus the
    /// auxiliary bytes at +0xA8 and +0x250.
    pub fn from_sub(sub: u8, byte_a8: u8, byte_250: u8) -> Self {
        match (sub, byte_a8, byte_250) {
            (0, _, _) => FieldType::Calculated,
            (1, _, _) => FieldType::String,
            (2, _, _) => FieldType::Date,
            (3, 0x00, _) => FieldType::Blob,
            (3, 0x16, _) => FieldType::Memo,
            (3, 0x1A, _) => FieldType::Graphic,
            (4, _, _) => FieldType::Boolean,
            (5, _, _) => FieldType::Smallint,
            (6, 0x1D, _) => FieldType::AutoInc,
            (6, _, _) => FieldType::Integer,
            (7, _, 0x0A) => FieldType::Currency,
            (7, _, _) => FieldType::Float,
            (9, _, _) => FieldType::Bytes,
            (10, _, _) => FieldType::Time,
            (11, _, _) => FieldType::DateTime,
            (15, _, _) => FieldType::VarBytes,
            (18, _, _) => FieldType::Largeint,
            _ => FieldType::Unknown(sub, byte_a8, byte_250),
        }
    }
}

/// A parsed column descriptor.
#[derive(Debug, Clone)]
pub struct Column {
    /// 1-based ordinal in the result set.
    pub ord: u16,
    /// Column name (ASCII, trimmed of Delphi ShortString stale-buffer noise).
    pub name: String,
    /// Field type. See [`FieldType`] for what each maps to.
    pub field_type: FieldType,
    /// Declared length (for ftString = max chars; 0 for fixed-size types).
    pub decl: u8,
    /// On-disk storage width in bytes (for ftString = decl+1 incl. length byte).
    pub max: u8,
    /// Byte offset of this field within the on-disk record (after the
    /// 25-byte row header). Field's null-flag byte lives at this offset;
    /// value bytes start at offset+1.
    pub row_offset: u16,
}

/// Parse all column descriptors from the schema region of a SELECT
/// response. Returns the parsed [`Column`] list and the byte offset
/// just past the last block (caller uses this to locate row data).
///
/// Strategy:
/// 1. Find the first `03 00 00 01 00` (block marker + ord=1 + namelen
///    nonzero) — that's column 1's block start.
/// 2. Walk blocks at 772-byte stride until the marker no longer appears
///    or the ordinal isn't sequential.
pub fn parse(server_payload: &[u8]) -> Result<(Vec<Column>, usize), IoError> {
    let first = find_first_block_start(server_payload)?;
    let mut columns = Vec::new();
    let mut off = first;
    let mut expected_ord = 1u16;
    while off + SCHEMA_BLOCK_STRIDE <= server_payload.len() {
        let col = match parse_one_block(&server_payload[off..off + SCHEMA_BLOCK_STRIDE]) {
            Some(c) => c,
            None => break, // not a valid block — schema region ended
        };
        if col.ord != expected_ord {
            // Out-of-sequence ordinal also marks the end (we may have
            // run into post-schema framing).
            break;
        }
        columns.push(col);
        expected_ord += 1;
        off += SCHEMA_BLOCK_STRIDE;
    }
    if columns.is_empty() {
        return Err(IoError::Other(
            "Exportmaster: schema parser found no columns".into(),
        ));
    }
    Ok((columns, off))
}

/// Find the first column block. Looks for `03 00 00 01 00` — the
/// block marker followed by ordinal=1 (LE u16) and a non-zero namelen.
fn find_first_block_start(payload: &[u8]) -> Result<usize, IoError> {
    // Search for the 5-byte signature; namelen byte at +5 must be > 0.
    let needle = [0x03, 0x00, 0x00, 0x01, 0x00];
    let mut i = 0;
    while i + 6 <= payload.len() {
        if payload[i..i + 5] == needle && payload[i + 5] > 0 {
            return Ok(i);
        }
        i += 1;
    }
    Err(IoError::Other(
        "Exportmaster: no schema block marker found in response".into(),
    ))
}

/// Parse one 772-byte block. Returns None if the leading marker
/// doesn't match (signals "no more blocks here").
fn parse_one_block(block: &[u8]) -> Option<Column> {
    if block.len() < SCHEMA_BLOCK_STRIDE {
        return None;
    }
    if block[..3] != BLOCK_MARKER {
        return None;
    }
    let ord = u16::from_le_bytes([block[3], block[4]]);
    let namelen = block[5] as usize;
    if namelen == 0 || 6 + namelen > block.len() {
        return None;
    }
    // Delphi ShortString — trust the length byte, not visible ASCII runs.
    let name = std::str::from_utf8(&block[6..6 + namelen])
        .ok()?
        .to_string();
    // 12-byte column descriptor at +0xA7. Layout per protocol §4:
    //   +0  sub          ftType code
    //   +1  00
    //   +2  decl         declared length
    //   +3  00 00
    //   +5  max          on-disk storage width
    //   +6  00 00
    //   +8  row_offset   u16 LE — byte offset within the record
    //   +10 00 00
    let meta = &block[0xA7..0xA7 + 12];
    let sub = meta[0];
    let decl = meta[2];
    let max = meta[5];
    let row_offset = u16::from_le_bytes([meta[8], meta[9]]);
    let byte_a8 = block[0xA8];
    let byte_250 = block[0x250];
    Some(Column {
        ord,
        name,
        field_type: FieldType::from_sub(sub, byte_a8, byte_250),
        decl,
        max,
        row_offset,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Synthesise one block with known field values, verify it round-trips.
    /// Doesn't exercise live wire data — that's the smoke test's job.
    #[test]
    fn parse_one_synthetic_string_block() {
        let mut block = vec![0u8; SCHEMA_BLOCK_STRIDE];
        block[..3].copy_from_slice(&BLOCK_MARKER);
        block[3..5].copy_from_slice(&7u16.to_le_bytes()); // ord=7
        block[5] = 4; // namelen
        block[6..10].copy_from_slice(b"CODE");
        // 12-byte descriptor at +0xA7:
        // sub=1 (String), decl=30, max=31, row_offset=25
        block[0xA7] = 1;
        block[0xA7 + 2] = 30;
        block[0xA7 + 5] = 31;
        block[0xA7 + 8..0xA7 + 10].copy_from_slice(&25u16.to_le_bytes());
        let col = parse_one_block(&block).expect("parse ok");
        assert_eq!(col.ord, 7);
        assert_eq!(col.name, "CODE");
        assert_eq!(col.field_type, FieldType::String);
        assert_eq!(col.decl, 30);
        assert_eq!(col.max, 31);
        assert_eq!(col.row_offset, 25);
    }

    /// Type-dispatch table from protocol §6b.
    #[test]
    fn field_type_dispatch_matches_doc() {
        assert_eq!(FieldType::from_sub(0, 0, 0), FieldType::Calculated);
        assert_eq!(FieldType::from_sub(1, 0, 0), FieldType::String);
        assert_eq!(FieldType::from_sub(2, 0, 0), FieldType::Date);
        assert_eq!(FieldType::from_sub(3, 0x00, 0), FieldType::Blob);
        assert_eq!(FieldType::from_sub(3, 0x16, 0), FieldType::Memo);
        assert_eq!(FieldType::from_sub(3, 0x1A, 0), FieldType::Graphic);
        assert_eq!(FieldType::from_sub(4, 0, 0), FieldType::Boolean);
        assert_eq!(FieldType::from_sub(5, 0, 0), FieldType::Smallint);
        assert_eq!(FieldType::from_sub(6, 0x00, 0), FieldType::Integer);
        assert_eq!(FieldType::from_sub(6, 0x1D, 0), FieldType::AutoInc);
        assert_eq!(FieldType::from_sub(7, 0, 0x0A), FieldType::Currency);
        assert_eq!(FieldType::from_sub(7, 0, 0x00), FieldType::Float);
        assert_eq!(FieldType::from_sub(9, 0, 0), FieldType::Bytes);
        assert_eq!(FieldType::from_sub(10, 0, 0), FieldType::Time);
        assert_eq!(FieldType::from_sub(11, 0, 0), FieldType::DateTime);
        assert_eq!(FieldType::from_sub(15, 0, 0), FieldType::VarBytes);
        assert_eq!(FieldType::from_sub(18, 0, 0), FieldType::Largeint);
        assert!(matches!(FieldType::from_sub(99, 0, 0), FieldType::Unknown(99, 0, 0)));
    }
}
