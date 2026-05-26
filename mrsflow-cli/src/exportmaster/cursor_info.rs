//! Cursor info: the 10-field structure the server writes after each
//! query/fetch (`TServerThread.PackCursorInfo`, RVA 0x49810, per
//! `Derek/DBISAM-PROTOCOL.md` §6d).
//!
//! Each field is one wire unit (`<u32 LE length><payload>`). The 8th
//! field — the **bookmark** — is the opaque cursor position the client
//! must echo verbatim into the next fetch. Different cursor modes
//! produce different bookmark sizes (single-PK = 17, composite =
//! variable, ORDER BY = 47, unindexed JOIN = 5); the client doesn't
//! need to know the format, just to copy the bytes.

use mrsflow_core::eval::IoError;

use super::wire::Walker;

/// Parsed cursor info. Only the bookmark and record counts are
/// surfaced; the other fields (record numbers, mtime, flags) are kept
/// as raw byte slices in case the caller wants them — but in practice
/// only the bookmark is needed for cursor advance.
#[derive(Debug, Clone)]
pub struct CursorInfo {
    pub record_number: u32,
    pub physical_record_number: u32,
    pub record_count: u32,
    pub physical_records_used: u32,
    pub last_auto_inc_id: u32,
    /// `TDateTime` double (8-byte LE binary64, days since 1899-12-30).
    /// Kept raw — callers that care decode it themselves.
    pub last_updated: [u8; 8],
    pub total_record_count: u32,
    /// THE opaque cursor position. Echo these bytes verbatim into the
    /// next fetch's bookmark slot to advance through any cursor mode.
    pub bookmark: Vec<u8>,
    pub flag_60e: u8,
    pub flag_60d: u8,
    /// Position in the buffer immediately past the last cursor-info
    /// unit. Row data (or whatever follows) starts here.
    pub end_offset: usize,
}

impl CursorInfo {
    /// Read the 10 cursor-info units starting at `walker`'s current
    /// position. Advances the walker past them.
    pub fn read(walker: &mut Walker<'_>) -> Result<Self, IoError> {
        let units = walker.next_n(10)?;
        Ok(Self {
            record_number: u32_le(units[0], "RecordNumber")?,
            physical_record_number: u32_le(units[1], "PhysicalRecordNumber")?,
            record_count: u32_le(units[2], "RecordCount")?,
            physical_records_used: u32_le(units[3], "PhysicalRecordsUsed")?,
            last_auto_inc_id: u32_le(units[4], "LastAutoIncID")?,
            last_updated: fixed_8(units[5], "LastUpdated")?,
            total_record_count: u32_le(units[6], "TotalRecordCount")?,
            bookmark: units[7].to_vec(),
            flag_60e: byte(units[8], "flag_60E")?,
            flag_60d: byte(units[9], "flag_60D")?,
            end_offset: walker.position(),
        })
    }
}

fn u32_le(buf: &[u8], name: &str) -> Result<u32, IoError> {
    if buf.len() != 4 {
        return Err(IoError::Other(format!(
            "Exportmaster: cursor-info field {name} expected 4 bytes, got {}",
            buf.len()
        )));
    }
    Ok(u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]))
}

fn fixed_8(buf: &[u8], name: &str) -> Result<[u8; 8], IoError> {
    if buf.len() != 8 {
        return Err(IoError::Other(format!(
            "Exportmaster: cursor-info field {name} expected 8 bytes, got {}",
            buf.len()
        )));
    }
    let mut out = [0u8; 8];
    out.copy_from_slice(buf);
    Ok(out)
}

fn byte(buf: &[u8], name: &str) -> Result<u8, IoError> {
    if buf.len() != 1 {
        return Err(IoError::Other(format!(
            "Exportmaster: cursor-info field {name} expected 1 byte, got {}",
            buf.len()
        )));
    }
    Ok(buf[0])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pack_unit(buf: &mut Vec<u8>, payload: &[u8]) {
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(payload);
    }

    #[test]
    fn reads_full_cursor_info() {
        // Synthesise a cursor-info: 10 units with deterministic values.
        let mut buf = Vec::new();
        pack_unit(&mut buf, &1u32.to_le_bytes()); // RecordNumber
        pack_unit(&mut buf, &2u32.to_le_bytes()); // PhysicalRecordNumber
        pack_unit(&mut buf, &25u32.to_le_bytes()); // RecordCount
        pack_unit(&mut buf, &30u32.to_le_bytes()); // PhysicalRecordsUsed
        pack_unit(&mut buf, &7u32.to_le_bytes()); // LastAutoIncID
        pack_unit(&mut buf, &[0u8; 8]); // LastUpdated
        pack_unit(&mut buf, &25u32.to_le_bytes()); // TotalRecordCount
        pack_unit(&mut buf, b"BOOKMARK-BYTES"); // Bookmark (14 bytes, arbitrary)
        pack_unit(&mut buf, &[0xAB]); // flag_60E
        pack_unit(&mut buf, &[0xCD]); // flag_60D

        let mut w = Walker::new(&buf, 0);
        let info = CursorInfo::read(&mut w).unwrap();
        assert_eq!(info.record_number, 1);
        assert_eq!(info.record_count, 25);
        assert_eq!(info.total_record_count, 25);
        assert_eq!(info.bookmark, b"BOOKMARK-BYTES");
        assert_eq!(info.flag_60e, 0xAB);
        assert_eq!(info.flag_60d, 0xCD);
        assert_eq!(info.end_offset, buf.len());
    }

    #[test]
    fn errors_on_wrong_field_size() {
        let mut buf = Vec::new();
        pack_unit(&mut buf, &[0u8; 3]); // RecordNumber: 3 bytes, not 4
        let mut w = Walker::new(&buf, 0);
        let err = CursorInfo::read(&mut w);
        assert!(err.is_err());
    }
}
