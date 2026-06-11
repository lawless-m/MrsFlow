//! TCP framing for the DBISAM wire protocol.
//!
//! Every message on the wire is:
//!     <16-byte session GUID> <u32 LE total_len> <body>
//! where `total_len = 20 + len(body)`. See DBISAM-PROTOCOL.md §2.
//!
//! The 16-byte GUID is a constant in the captured sessions; we treat it
//! as a fixed protocol marker. (Hypothesis: it's a client-runtime
//! constant baked into dbsys.exe. If a real DBISAM server ever rejects
//! it during cross-host testing, we'd need to negotiate it.)

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use mrsflow_core::eval::IoError;

/// Cap on a single framed body, to stop a corrupt or hostile `total_len`
/// from triggering a multi-gigabyte allocation. 256 MiB is far above any
/// real DBISAM response (the largest observed is a few MiB).
const MAX_BODY_LEN: usize = 256 * 1024 * 1024;

/// Cap on the inflated size of a compressed body (zip-bomb guard). 1 GiB
/// is generous headroom over MAX_BODY_LEN's worth of compressible rows.
const MAX_INFLATED_LEN: u64 = 1024 * 1024 * 1024;

/// 16-byte session GUID — copied from PoC captures.
pub const GUID: [u8; 16] = [
    0x8A, 0xBE, 0x8E, 0x59, 0x23, 0x64, 0xCB, 0x40,
    0x3D, 0x71, 0xD2, 0xE3, 0xBC, 0x64, 0xD0, 0x01,
];

/// Wrap a body in the standard framing envelope: GUID + u32 LE total_len.
///
/// Per `ANSWERS-TO-DEREK-4.md`: total_size is aligned to 8 bytes
/// (`BlockOffset(size, 8) = (size + 7) & !7`). Uncompressed bodies in
/// our captures all happened to land on 8-byte boundaries naturally;
/// compressed bodies are arbitrary length, so we pad with zeros up to
/// the next multiple of 8 and report the padded size in `total_len`.
pub fn wrap(body: &[u8]) -> Vec<u8> {
    let raw_total = 20 + body.len();
    let aligned_total = (raw_total + 7) & !7;
    let pad = aligned_total - raw_total;
    let mut out = Vec::with_capacity(aligned_total);
    out.extend_from_slice(&GUID);
    out.extend_from_slice(&(aligned_total as u32).to_le_bytes());
    out.extend_from_slice(body);
    for _ in 0..pad {
        out.push(0);
    }
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

/// Convenience: pick `send_recv_compressed` or `send_recv` based on
/// the per-session compression flag.
pub fn send_recv_auto(
    stream: &mut TcpStream,
    body: &[u8],
    compression: bool,
) -> Result<Vec<u8>, IoError> {
    if compression {
        send_recv_compressed(stream, body)
    } else {
        send_recv(stream, body)
    }
}

/// Send a framed message with per-body Zlib compression and receive
/// one framed reply (decompressed if flagged).
///
/// Body layout per capture analysis of a live RemoteCompression=9 session:
///   - body[0] = flag byte. Low bit = "this body is deflated". High bits
///     observed as `0x5A` for post-Connect bodies. So:
///       0x5A = uncompressed (post-Connect, comp session)
///       0x5B = compressed (post-Connect, comp session)
///   - body[1..3] = reqcode u16 LE (plaintext header)
///   - body[3..7] = inner_len u32 LE (on-wire byte count of inner section,
///     compressed length when flag's low bit is set)
///   - body[7..7+inner_len] = either plaintext Pack stream (flag bit 0
///     clear) OR zlib-deflated Pack stream (flag bit 0 set)
///   - Bodies whose UNCOMPRESSED inner section is <= 16 bytes are NOT
///     compressed (zlib overhead would grow them).
///
/// Receive side: any flag byte with low bit set triggers inflate.
pub fn send_recv_compressed(stream: &mut TcpStream, body: &[u8]) -> Result<Vec<u8>, IoError> {
    const SESSION_STATE: u8 = 0x5A;
    // Connect (reqcode 0x0000) is special: its flag byte encodes the
    // requested compression level (e.g. 0x09 for level 9 in observed
    // capture). For post-Connect messages, use SESSION_STATE | low-bit.
    let reqcode = if body.len() >= 3 { u16::from_le_bytes([body[1], body[2]]) } else { 0xFFFF };
    let is_connect = reqcode == 0x0000;
    let to_send = if body.len() < 7 {
        // A compressed session expects every body to carry the flag byte
        // + reqcode + inner_len header (7 bytes). Sending a shorter body
        // raw would omit the flag and desync the session. No current
        // builder emits a sub-7-byte body, so this is a guard, not a path.
        return Err(IoError::Other(format!(
            "Exportmaster: refusing to send a {}-byte body on a compressed session \
             (needs the 7-byte flag+reqcode+len header)",
            body.len()
        )));
    } else {
        let inner = &body[7..];
        if inner.len() <= 16 {
            let mut out = Vec::with_capacity(body.len());
            out.push(if is_connect { 0x09 } else { SESSION_STATE });
            out.extend_from_slice(&body[1..]);
            out
        } else {
            let deflated = deflate(inner)?;
            let mut out = Vec::with_capacity(7 + deflated.len());
            out.push(if is_connect { 0x09 } else { SESSION_STATE | 0x01 });
            out.extend_from_slice(&body[1..3]);
            out.extend_from_slice(&(deflated.len() as u32).to_le_bytes());
            out.extend_from_slice(&deflated);
            out
        }
    };
    let pkt = wrap(&to_send);
    stream
        .write_all(&pkt)
        .map_err(|e| IoError::Other(format!("Exportmaster: send: {e}")))?;
    let raw = recv_msg(stream)?;
    decompress_body_if_flagged(&raw)
}

fn decompress_body_if_flagged(body: &[u8]) -> Result<Vec<u8>, IoError> {
    if body.len() < 7 || (body[0] & 0x01) == 0 {
        let mut out = body.to_vec();
        if !out.is_empty() {
            out[0] = 0x00;
        }
        return Ok(out);
    }
    let inner_len = u32::from_le_bytes([body[3], body[4], body[5], body[6]]) as usize;
    if 7 + inner_len > body.len() {
        return Err(IoError::Other(format!(
            "Exportmaster: compressed body inner_len {} exceeds available {}",
            inner_len,
            body.len() - 7
        )));
    }
    let inflated = inflate(&body[7..7 + inner_len])?;
    let mut out = Vec::with_capacity(7 + inflated.len());
    out.push(0x00);
    out.extend_from_slice(&body[1..3]);
    out.extend_from_slice(&(inflated.len() as u32).to_le_bytes());
    out.extend_from_slice(&inflated);
    Ok(out)
}

pub(super) fn deflate(body: &[u8]) -> Result<Vec<u8>, IoError> {
    // Compression level 1 (fast) matches what dbsys.exe uses in
    // captured sessions (zlib header byte `78 01`).
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;
    let mut enc = ZlibEncoder::new(Vec::new(), Compression::fast());
    enc.write_all(body)
        .map_err(|e| IoError::Other(format!("Exportmaster: deflate: {e}")))?;
    enc.finish()
        .map_err(|e| IoError::Other(format!("Exportmaster: deflate finish: {e}")))
}

pub(super) fn inflate(body: &[u8]) -> Result<Vec<u8>, IoError> {
    use flate2::read::ZlibDecoder;
    use std::io::Read;
    // Bound the read so a hostile/corrupt stream can't inflate without
    // limit. `take` caps the decoder; if it stops exactly at the cap we
    // can't tell whether more would have followed, so reject.
    let mut dec = ZlibDecoder::new(body).take(MAX_INFLATED_LEN + 1);
    let mut out = Vec::with_capacity((body.len() * 4).min(MAX_INFLATED_LEN as usize));
    dec.read_to_end(&mut out)
        .map_err(|e| IoError::Other(format!("Exportmaster: inflate: {e}")))?;
    if out.len() as u64 > MAX_INFLATED_LEN {
        return Err(IoError::Other(format!(
            "Exportmaster: inflated body exceeds {MAX_INFLATED_LEN}-byte cap"
        )));
    }
    Ok(out)
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
    if body_len > MAX_BODY_LEN {
        return Err(IoError::Other(format!(
            "Exportmaster: server body length {body_len} exceeds {MAX_BODY_LEN}-byte cap \
             — corrupt or hostile length field"
        )));
    }
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
    use std::net::ToSocketAddrs;
    let addr = format!("{host}:{port}");
    // Resolve via DNS so `host` may be a hostname (e.g. "rivsem01"), not just
    // an IP literal. `to_socket_addrs` handles both. Try each resolved
    // address until one connects within the timeout.
    let resolved = addr
        .to_socket_addrs()
        .map_err(|e| IoError::Other(format!("Exportmaster: resolve {addr}: {e}")))?;
    let mut stream = None;
    let mut last_err = None;
    for sa in resolved {
        match std::net::TcpStream::connect_timeout(&sa, Duration::from_secs(10)) {
            Ok(s) => {
                stream = Some(s);
                break;
            }
            Err(e) => last_err = Some(e),
        }
    }
    let stream = stream.ok_or_else(|| {
        let detail = last_err
            .map(|e| e.to_string())
            .unwrap_or_else(|| "no addresses resolved".to_string());
        IoError::Other(format!("Exportmaster: connect {addr}: {detail}"))
    })?;
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
