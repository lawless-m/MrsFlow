//! DBISAM login crypto: Blowfish-CBC with IV=0 and key=MD5(encrypt_password).
//!
//! Recovered from `dbsys.exe` BPL disassembly + dictionary attack against
//! the captured login bytes — see DBISAM-PROTOCOL.md §5.

use blowfish::Blowfish;
use cbc::cipher::{BlockEncryptMut, KeyIvInit};
use md5::{Digest, Md5};

use mrsflow_core::eval::IoError;

type BlowfishCbcEnc = cbc::Encryptor<Blowfish>;

/// Encrypt a DBISAM login plaintext.
///
/// `plaintext` format: `<u8 ulen> <username> <u8 plen> <password>`,
/// zero-padded to a multiple of 8 (Blowfish block size).
///
/// Returns the ciphertext exactly as written to the wire — same length
/// as the padded plaintext. Errors if the username or password exceeds
/// 255 bytes: each is framed with a single-byte length prefix, so a
/// longer value would wrap and silently authenticate as a truncated
/// credential.
pub fn encrypt_login(
    username: &[u8],
    password: &[u8],
    encrypt_password: &[u8],
) -> Result<Vec<u8>, IoError> {
    if username.len() > 255 {
        return Err(IoError::Other(format!(
            "Exportmaster: username is {} bytes; the login frame caps it at 255",
            username.len()
        )));
    }
    if password.len() > 255 {
        return Err(IoError::Other(format!(
            "Exportmaster: password is {} bytes; the login frame caps it at 255",
            password.len()
        )));
    }
    // Build plaintext: length-prefixed user + password.
    let mut pt = Vec::with_capacity(2 + username.len() + password.len() + 8);
    pt.push(username.len() as u8);
    pt.extend_from_slice(username);
    pt.push(password.len() as u8);
    pt.extend_from_slice(password);
    // Pad to multiple of 8 with zero bytes (Blowfish block size).
    let pad = (8 - (pt.len() % 8)) % 8;
    pt.extend(std::iter::repeat(0u8).take(pad));

    // Key = MD5(encrypt_password) — 16 bytes.
    let mut hasher = Md5::new();
    hasher.update(encrypt_password);
    let key = hasher.finalize();

    // Blowfish-CBC encrypt in place. IV = 8 zero bytes per the protocol.
    let iv = [0u8; 8];
    let cipher = BlowfishCbcEnc::new_from_slices(&key, &iv)
        .expect("MD5 digest is 16 bytes — valid Blowfish key length");
    let mut out = pt.clone();
    // encrypt_padded_mut takes the buffer with the message already at the
    // start and a message length; we've already pre-padded so passing
    // the full length is what we want and there's no further padding.
    // However the cbc helper insists on adding padding via the Padding
    // type param — easier to drive the cipher directly block-by-block.
    use cbc::cipher::generic_array::GenericArray;
    use cbc::cipher::BlockEncryptMut as _;
    // Re-create as a mutable cipher we can call block_encrypt_mut on.
    let mut cipher = BlowfishCbcEnc::new_from_slices(&key, &iv).unwrap();
    for chunk in out.chunks_exact_mut(8) {
        let block = GenericArray::from_mut_slice(chunk);
        cipher.encrypt_block_mut(block);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Worked example from DBISAM-PROTOCOL.md §5:
    /// user `e3user`, password `e3usernew`, encrypt password `elevatesoft`.
    #[test]
    fn login_ciphertext_matches_doc_worked_example() {
        let ct = encrypt_login(b"e3user", b"e3usernew", b"elevatesoft").unwrap();
        let expected = [
            0x57, 0x25, 0x56, 0x8E, 0x56, 0x01, 0xB0, 0x58,
            0xD1, 0x7E, 0xE1, 0x77, 0x20, 0xB6, 0x95, 0x24,
            0x78, 0x1F, 0x5A, 0x02, 0x17, 0xF2, 0x43, 0x90,
        ];
        assert_eq!(ct, expected);
    }

    #[test]
    fn rejects_credentials_over_255_bytes() {
        let long = vec![b'a'; 256];
        assert!(encrypt_login(&long, b"pw", b"elevatesoft").is_err());
        assert!(encrypt_login(b"user", &long, b"elevatesoft").is_err());
        // 255 is the boundary and must still succeed.
        let max = vec![b'a'; 255];
        assert!(encrypt_login(&max, b"pw", b"elevatesoft").is_ok());
    }

    #[test]
    fn key_is_md5_of_encrypt_password() {
        // MD5("elevatesoft") is reproducible — quick sanity check that
        // our md-5 crate behaves as the BPL did.
        let mut h = Md5::new();
        h.update(b"elevatesoft");
        let digest = h.finalize();
        assert_eq!(
            &digest[..],
            &[
                0xCE, 0x85, 0x01, 0xAA, 0xC5, 0x39, 0xB4, 0xBD,
                0x4C, 0x54, 0x32, 0x7E, 0x41, 0xD9, 0x75, 0xB0,
            ]
        );
    }
}
