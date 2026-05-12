//! `Binary.*` stdlib bindings on `Value::Binary(Vec<u8>)`.

#![allow(unused_imports)]

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{expect_int, expect_list, expect_text, one, two, three, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Binary.Length", one("binary"), length),
        ("Binary.ApproximateLength", one("binary"), length),
        ("Binary.From", one("value"), from),
        ("Binary.FromList", one("list"), from_list),
        (
            "Binary.FromText",
            vec![
                Param { name: "text".into(),     optional: false, type_annotation: None },
                Param { name: "encoding".into(), optional: true,  type_annotation: None },
            ],
            from_text,
        ),
        (
            "Binary.ToText",
            vec![
                Param { name: "binary".into(),   optional: false, type_annotation: None },
                Param { name: "encoding".into(), optional: true,  type_annotation: None },
            ],
            to_text,
        ),
        ("Binary.ToList", one("binary"), to_list),
        (
            "Binary.Range",
            vec![
                Param { name: "binary".into(), optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            range,
        ),
        ("Binary.Combine", one("binaries"), combine),
        ("Binary.Buffer", one("binary"), buffer),
        ("Binary.Split", two("binary", "pageSize"), split),
        ("Binary.InferContentType", one("binary"), infer_content_type),
        // --- Slice #173: compression + view stubs ---
        ("Binary.Compress", two("binary", "compressionType"), compress),
        ("Binary.Decompress", two("binary", "compressionType"), decompress),
        ("Binary.View", two("binary", "handlers"), view),
        ("Binary.ViewError", one("record"), view_passthrough),
        ("Binary.ViewFunction", one("function"), view_passthrough),
    ]
}

fn expect_binary(v: &Value) -> Result<&[u8], MError> {
    match v {
        Value::Binary(b) => Ok(b),
        other => Err(type_mismatch("binary", other)),
    }
}

fn length(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_binary(&args[0])?;
    Ok(Value::Number(b.len() as f64))
}

fn from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Binary(b) => Ok(Value::Binary(b.clone())),
        Value::Text(s) => Ok(Value::Binary(s.as_bytes().to_vec())),
        Value::Null => Ok(Value::Null),
        other => Err(type_mismatch("text/binary/null", other)),
    }
}

fn from_list(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    let mut bytes: Vec<u8> = Vec::with_capacity(xs.len());
    for v in xs {
        let n = expect_int(v, "Binary.FromList")?;
        if !(0..=255).contains(&n) {
            return Err(MError::Other(format!(
                "Binary.FromList: byte value out of range: {}",
                n
            )));
        }
        bytes.push(n as u8);
    }
    Ok(Value::Binary(bytes))
}

/// Encoding mode for Binary.FromText / Binary.ToText.
#[derive(Clone, Copy)]
enum Encoding {
    Base64,
    Hex,
}

fn parse_encoding(v: Option<&Value>, ctx: &str) -> Result<Encoding, MError> {
    match v {
        None | Some(Value::Null) => Ok(Encoding::Base64),
        Some(Value::Number(n)) if *n == 0.0 => Ok(Encoding::Base64),
        Some(Value::Number(n)) if *n == 1.0 => Ok(Encoding::Hex),
        Some(Value::Text(s)) => {
            let n = s.to_ascii_lowercase();
            let n = n.trim_start_matches("binaryencoding.");
            match n.as_ref() {
                "base64" => Ok(Encoding::Base64),
                "hex" => Ok(Encoding::Hex),
                _ => Err(MError::Other(format!(
                    "{}: unknown encoding: {}",
                    ctx, s
                ))),
            }
        }
        Some(other) => Err(type_mismatch("text or number (encoding)", other)),
    }
}

fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let enc = parse_encoding(args.get(1), "Binary.FromText")?;
    Ok(Value::Binary(match enc {
        Encoding::Base64 => base64_decode(text)?,
        Encoding::Hex => hex_decode(text)?,
    }))
}

fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bytes = expect_binary(&args[0])?;
    let enc = parse_encoding(args.get(1), "Binary.ToText")?;
    Ok(Value::Text(match enc {
        Encoding::Base64 => base64_encode(bytes),
        Encoding::Hex => hex_encode(bytes),
    }))
}

fn to_list(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bytes = expect_binary(&args[0])?;
    Ok(Value::List(bytes.iter().map(|b| Value::Number(*b as f64)).collect()))
}

fn range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bytes = expect_binary(&args[0])?;
    let offset = expect_int(&args[1], "Binary.Range: offset")?;
    if offset < 0 {
        return Err(MError::Other("Binary.Range: offset must be non-negative".into()));
    }
    let offset = (offset as usize).min(bytes.len());
    let count = match args.get(2) {
        Some(Value::Null) | None => bytes.len() - offset,
        Some(v) => {
            let n = expect_int(v, "Binary.Range: count")?;
            if n < 0 {
                return Err(MError::Other("Binary.Range: count must be non-negative".into()));
            }
            (n as usize).min(bytes.len() - offset)
        }
    };
    Ok(Value::Binary(bytes[offset..offset + count].to_vec()))
}

fn combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let xs = expect_list(&args[0])?;
    let mut out: Vec<u8> = Vec::new();
    for v in xs {
        match v {
            Value::Binary(b) => out.extend_from_slice(b),
            other => return Err(type_mismatch("binary (in list)", other)),
        }
    }
    Ok(Value::Binary(out))
}

fn buffer(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_binary(&args[0])?;
    Ok(args[0].clone())
}

fn split(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bytes = expect_binary(&args[0])?;
    let page_size = expect_int(&args[1], "Binary.Split: pageSize")?;
    if page_size <= 0 {
        return Err(MError::Other("Binary.Split: pageSize must be positive".into()));
    }
    let page_size = page_size as usize;
    let out: Vec<Value> = bytes
        .chunks(page_size)
        .map(|c| Value::Binary(c.to_vec()))
        .collect();
    Ok(Value::List(out))
}

fn infer_content_type(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no sniffing — return null. (PQ uses this for HTTP content-types.)
    let _ = expect_binary(&args[0])?;
    Ok(Value::Null)
}

// --- Base64 / hex codecs (no external crate dependency) ---

const B64_ALPHABET: &[u8; 64] =
    b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(((bytes.len() + 2) / 3) * 4);
    let chunks = bytes.chunks(3);
    for chunk in chunks {
        let n = chunk.len();
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);
        let i0 = (b0 >> 2) as usize;
        let i1 = (((b0 & 0x03) << 4) | (b1 >> 4)) as usize;
        let i2 = (((b1 & 0x0F) << 2) | (b2 >> 6)) as usize;
        let i3 = (b2 & 0x3F) as usize;
        out.push(B64_ALPHABET[i0] as char);
        out.push(B64_ALPHABET[i1] as char);
        out.push(if n >= 2 { B64_ALPHABET[i2] as char } else { '=' });
        out.push(if n >= 3 { B64_ALPHABET[i3] as char } else { '=' });
    }
    out
}

fn base64_decode(s: &str) -> Result<Vec<u8>, MError> {
    // Strip whitespace and the optional "=" padding.
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 4 != 0 {
        return Err(MError::Other("Binary.FromText: invalid base64 length".into()));
    }
    let mut out: Vec<u8> = Vec::with_capacity(s.len() / 4 * 3);
    for chunk in s.as_bytes().chunks(4) {
        let mut vals = [0u8; 4];
        let mut pad = 0;
        for (i, c) in chunk.iter().enumerate() {
            vals[i] = if *c == b'=' {
                pad += 1;
                0
            } else {
                match B64_ALPHABET.iter().position(|x| x == c) {
                    Some(p) => p as u8,
                    None => return Err(MError::Other(format!(
                        "Binary.FromText: invalid base64 char: {}",
                        *c as char
                    ))),
                }
            };
        }
        let b0 = (vals[0] << 2) | (vals[1] >> 4);
        let b1 = ((vals[1] & 0x0F) << 4) | (vals[2] >> 2);
        let b2 = ((vals[2] & 0x03) << 6) | vals[3];
        out.push(b0);
        if pad < 2 {
            out.push(b1);
        }
        if pad < 1 {
            out.push(b2);
        }
    }
    Ok(out)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0F) as usize] as char);
    }
    out
}

fn hex_decode(s: &str) -> Result<Vec<u8>, MError> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    if s.len() % 2 != 0 {
        return Err(MError::Other("Binary.FromText: hex must have even length".into()));
    }
    let mut out: Vec<u8> = Vec::with_capacity(s.len() / 2);
    let bytes = s.as_bytes();
    for i in (0..bytes.len()).step_by(2) {
        let hi = decode_hex_nibble(bytes[i])?;
        let lo = decode_hex_nibble(bytes[i + 1])?;
        out.push((hi << 4) | lo);
    }
    Ok(out)
}

// --- Slice #173: compression ---

#[derive(Clone, Copy)]
enum Compression {
    None,
    GZip,
    Deflate,
    Brotli,
}

fn parse_compression(v: &Value, ctx: &str) -> Result<Compression, MError> {
    match v {
        Value::Number(n) if *n == 0.0 => Ok(Compression::None),
        Value::Number(n) if *n == 1.0 => Ok(Compression::GZip),
        Value::Number(n) if *n == 2.0 => Ok(Compression::Deflate),
        Value::Number(n) if *n == 3.0 => Ok(Compression::Brotli),
        other => Err(MError::Other(format!(
            "{}: expected Compression.* constant, got {:?}",
            ctx, other
        ))),
    }
}

fn compress(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use std::io::Write;
    let bytes = expect_binary(&args[0])?;
    let kind = parse_compression(&args[1], "Binary.Compress")?;
    let out: Vec<u8> = match kind {
        Compression::None => bytes.to_vec(),
        Compression::GZip => {
            let mut enc = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(bytes)
                .map_err(|e| MError::Other(format!("Binary.Compress: gzip failed: {}", e)))?;
            enc.finish()
                .map_err(|e| MError::Other(format!("Binary.Compress: gzip finish failed: {}", e)))?
        }
        Compression::Deflate => {
            let mut enc =
                flate2::write::DeflateEncoder::new(Vec::new(), flate2::Compression::default());
            enc.write_all(bytes)
                .map_err(|e| MError::Other(format!("Binary.Compress: deflate failed: {}", e)))?;
            enc.finish().map_err(|e| {
                MError::Other(format!("Binary.Compress: deflate finish failed: {}", e))
            })?
        }
        Compression::Brotli => {
            return Err(MError::NotImplemented(
                "Binary.Compress: Brotli not supported in v1 (no brotli crate dependency)",
            ));
        }
    };
    Ok(Value::Binary(out))
}

fn decompress(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use std::io::Read;
    let bytes = expect_binary(&args[0])?;
    let kind = parse_compression(&args[1], "Binary.Decompress")?;
    let out: Vec<u8> = match kind {
        Compression::None => bytes.to_vec(),
        Compression::GZip => {
            let mut dec = flate2::read::GzDecoder::new(bytes);
            let mut buf = Vec::new();
            dec.read_to_end(&mut buf)
                .map_err(|e| MError::Other(format!("Binary.Decompress: gzip failed: {}", e)))?;
            buf
        }
        Compression::Deflate => {
            let mut dec = flate2::read::DeflateDecoder::new(bytes);
            let mut buf = Vec::new();
            dec.read_to_end(&mut buf).map_err(|e| {
                MError::Other(format!("Binary.Decompress: deflate failed: {}", e))
            })?;
            buf
        }
        Compression::Brotli => {
            return Err(MError::NotImplemented(
                "Binary.Decompress: Brotli not supported in v1",
            ));
        }
    };
    Ok(Value::Binary(out))
}

fn view(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: View is a folding hook — pass binary through unchanged.
    let _ = expect_binary(&args[0])?;
    Ok(args[0].clone())
}

fn view_passthrough(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(args[0].clone())
}

fn decode_hex_nibble(b: u8) -> Result<u8, MError> {
    match b {
        b'0'..=b'9' => Ok(b - b'0'),
        b'a'..=b'f' => Ok(b - b'a' + 10),
        b'A'..=b'F' => Ok(b - b'A' + 10),
        _ => Err(MError::Other(format!(
            "Binary.FromText: invalid hex char: {}",
            b as char
        ))),
    }
}
