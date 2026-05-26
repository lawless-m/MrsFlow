//! Wire-strict walker for the universal `<u32 LE length><payload>` framing
//! rule documented in `Derive/DBISAM-PROTOCOL.md` §6c and confirmed by
//! disassembly of `TDataSession.Unpack` (RVA 0x07752C).
//!
//! Every unit on the wire is `<u32 LE length><length bytes>`. Higher-level
//! structures (cursor info, row data, field definitions) are sequences of
//! these units; their semantics depend on message type + order, but the
//! framing is uniform. This walker is the one place that interprets that
//! framing — every caller goes through it instead of scanning bytes ad-hoc.
//!
//! Usage:
//! ```ignore
//! let mut w = Walker::new(&body, start_offset);
//! while let Some(unit) = w.next_unit()? {
//!     // unit is &[u8], the payload (no length prefix)
//! }
//! ```
//!
//! `next_unit` returns `Ok(None)` when the buffer is exhausted, `Err` if
//! a length prefix points past the end of the buffer (malformed wire).

use mrsflow_core::eval::IoError;

pub struct Walker<'a> {
    buf: &'a [u8],
    pos: usize,
}

impl<'a> Walker<'a> {
    pub fn new(buf: &'a [u8], start: usize) -> Self {
        Self { buf, pos: start }
    }

    pub fn position(&self) -> usize {
        self.pos
    }

    /// Re-borrow the underlying buffer. Used by callers that need to
    /// rewind (peek a unit, decide it doesn't belong to them, restart
    /// the walker at a saved position).
    pub fn buf(&self) -> &'a [u8] {
        self.buf
    }

    /// Rewind to a previously recorded position.
    pub fn seek(&mut self, pos: usize) {
        self.pos = pos;
    }

    /// Read the next `<u32 LE length><payload>` unit. Returns `None` if
    /// the buffer is exhausted cleanly, `Err` if a length prefix points
    /// past the end.
    pub fn next_unit(&mut self) -> Result<Option<&'a [u8]>, IoError> {
        if self.pos >= self.buf.len() {
            return Ok(None);
        }
        if self.pos + 4 > self.buf.len() {
            return Err(IoError::Other(format!(
                "Exportmaster: wire walker ran past end at {} (need 4-byte length, have {})",
                self.pos,
                self.buf.len() - self.pos,
            )));
        }
        let length = u32::from_le_bytes([
            self.buf[self.pos],
            self.buf[self.pos + 1],
            self.buf[self.pos + 2],
            self.buf[self.pos + 3],
        ]) as usize;
        let start = self.pos + 4;
        let end = start + length;
        if end > self.buf.len() {
            return Err(IoError::Other(format!(
                "Exportmaster: wire walker length {length} at pos {} would overrun (buf len {})",
                self.pos,
                self.buf.len(),
            )));
        }
        self.pos = end;
        Ok(Some(&self.buf[start..end]))
    }

    /// Convenience: read N consecutive units. Errors if any unit is missing.
    pub fn next_n(&mut self, n: usize) -> Result<Vec<&'a [u8]>, IoError> {
        let mut out = Vec::with_capacity(n);
        for i in 0..n {
            match self.next_unit()? {
                Some(u) => out.push(u),
                None => {
                    return Err(IoError::Other(format!(
                        "Exportmaster: wire walker expected {n} units, got {i} before exhaustion"
                    )));
                }
            }
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_three_units() {
        // 3 units: [4 bytes "wxyz"], [2 bytes "ab"], [0 bytes ""]
        let mut buf = Vec::new();
        buf.extend_from_slice(&4u32.to_le_bytes());
        buf.extend_from_slice(b"wxyz");
        buf.extend_from_slice(&2u32.to_le_bytes());
        buf.extend_from_slice(b"ab");
        buf.extend_from_slice(&0u32.to_le_bytes());
        let mut w = Walker::new(&buf, 0);
        assert_eq!(w.next_unit().unwrap(), Some(b"wxyz".as_slice()));
        assert_eq!(w.next_unit().unwrap(), Some(b"ab".as_slice()));
        assert_eq!(w.next_unit().unwrap(), Some(b"".as_slice()));
        assert_eq!(w.next_unit().unwrap(), None);
    }

    #[test]
    fn errors_on_overrun() {
        // length says 10 but only 2 bytes follow
        let mut buf = Vec::new();
        buf.extend_from_slice(&10u32.to_le_bytes());
        buf.extend_from_slice(b"ab");
        let mut w = Walker::new(&buf, 0);
        assert!(w.next_unit().is_err());
    }

    #[test]
    fn empty_buf_returns_none() {
        let mut w = Walker::new(&[], 0);
        assert!(w.next_unit().unwrap().is_none());
    }

    #[test]
    fn starts_at_offset() {
        let mut buf = vec![0xAA, 0xBB, 0xCC];
        buf.extend_from_slice(&3u32.to_le_bytes());
        buf.extend_from_slice(b"foo");
        let mut w = Walker::new(&buf, 3);
        assert_eq!(w.next_unit().unwrap(), Some(b"foo".as_slice()));
    }
}
