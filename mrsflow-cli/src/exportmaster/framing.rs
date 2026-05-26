//! TCP framing for the DBISAM wire protocol.
//!
//! Every message on the wire is:
//!     <16-byte session GUID> <u32 LE total_len> <body>
//! where `total_len = 20 + len(body)`. See Derek/DBISAM-PROTOCOL.md §2.
//!
//! The 16-byte GUID is a constant in the captured sessions; we treat it
//! as a fixed protocol marker. (Hypothesis: it's a client-runtime
//! constant baked into dbsys.exe. If a real DBISAM server ever rejects
//! it during cross-host testing, we'd need to negotiate it.)

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use mrsflow_core::eval::IoError;

/// 16-byte session GUID — copied from PoC captures.
pub const GUID: [u8; 16] = [
    0x8A, 0xBE, 0x8E, 0x59, 0x23, 0x64, 0xCB, 0x40,
    0x3D, 0x71, 0xD2, 0xE3, 0xBC, 0x64, 0xD0, 0x01,
];

/// Wrap a body in the standard framing envelope: GUID + u32 LE total_len.
pub fn wrap(body: &[u8]) -> Vec<u8> {
    let total = 20 + body.len();
    let mut out = Vec::with_capacity(total);
    out.extend_from_slice(&GUID);
    out.extend_from_slice(&(total as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}

/// Send a framed message and receive one framed reply. Returns the body
/// (without the 20-byte envelope). The caller can call this repeatedly
/// on the same stream — DBISAM is strictly request/response.
pub fn send_recv(stream: &mut TcpStream, body: &[u8]) -> Result<Vec<u8>, IoError> {
    let pkt = wrap(body);
    stream
        .write_all(&pkt)
        .map_err(|e| IoError::Other(format!("Exportmaster: send: {e}")))?;
    recv_msg(stream)
}

/// Receive one full framed message. Reads exactly 20 bytes for the
/// header, then `total_len - 20` bytes for the body. Returns the body.
pub fn recv_msg(stream: &mut TcpStream) -> Result<Vec<u8>, IoError> {
    let mut head = [0u8; 20];
    read_exact(stream, &mut head)?;
    if head[..16] != GUID {
        return Err(IoError::Other(format!(
            "Exportmaster: unexpected envelope prefix: {:02x?}",
            &head[..16]
        )));
    }
    let total = u32::from_le_bytes([head[16], head[17], head[18], head[19]]) as usize;
    if total < 20 {
        return Err(IoError::Other(format!(
            "Exportmaster: bad total_len in header: {total}"
        )));
    }
    let body_len = total - 20;
    let mut body = vec![0u8; body_len];
    read_exact(stream, &mut body)?;
    Ok(body)
}

fn read_exact(stream: &mut TcpStream, buf: &mut [u8]) -> Result<(), IoError> {
    let mut got = 0;
    while got < buf.len() {
        match stream.read(&mut buf[got..]) {
            Ok(0) => {
                return Err(IoError::Other(format!(
                    "Exportmaster: connection closed (got {} of {})",
                    got,
                    buf.len()
                )));
            }
            Ok(n) => got += n,
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e) => {
                return Err(IoError::Other(format!("Exportmaster: read: {e}")));
            }
        }
    }
    Ok(())
}

/// Open a TCP connection with sensible timeouts. The defaults match
/// PoC behaviour (10s connect, 5s per-message).
pub fn connect(host: &str, port: u16) -> Result<TcpStream, IoError> {
    let addr = format!("{host}:{port}");
    let stream = std::net::TcpStream::connect_timeout(
        &addr
            .parse()
            .map_err(|e| IoError::Other(format!("Exportmaster: bad address {addr}: {e}")))?,
        Duration::from_secs(10),
    )
    .map_err(|e| IoError::Other(format!("Exportmaster: connect {addr}: {e}")))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| IoError::Other(format!("Exportmaster: set_read_timeout: {e}")))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| IoError::Other(format!("Exportmaster: set_write_timeout: {e}")))?;
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_layout_matches_poc() {
        // A 4-byte body should produce a 24-byte packet: GUID + u32(24) + body.
        let body = [0xAA, 0xBB, 0xCC, 0xDD];
        let pkt = wrap(&body);
        assert_eq!(pkt.len(), 24);
        assert_eq!(&pkt[..16], &GUID);
        assert_eq!(&pkt[16..20], &[24, 0, 0, 0]); // u32 LE total_len
        assert_eq!(&pkt[20..], &body);
    }
}
