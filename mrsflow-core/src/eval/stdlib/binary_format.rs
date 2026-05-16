//! `BinaryFormat.*` — declarative parser-combinator framework for binary
//! streams ("Wireshark in M": describe a wire format, get a typed parser).
//!
//! Each `BinaryFormat.X` is a single-arg M function: given a `Binary`,
//! it parses some prefix and returns the parsed value. Composers (List,
//! Record, Choice, Group) come in later slices; this slice ships the
//! fixed-width primitive readers.
//!
//! Endianness for slice 1 is hard-coded little-endian — that's the
//! default in PQ and matches every test we have. `BinaryFormat.ByteOrder`
//! (slice 2) will introduce the wrapper that switches to big-endian.
//!
//! Decimal note: PQ's `BinaryFormat.Decimal` reads a 16-byte .NET-style
//! decimal-128 value. mrsflow stores numbers as f64, so the decimal is
//! converted lossily to f64 — same compromise we make everywhere else.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::type_mismatch;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("BinaryFormat.Byte",              one_binary(), parse_byte),
        ("BinaryFormat.SignedInteger16",   one_binary(), parse_i16_le),
        ("BinaryFormat.SignedInteger32",   one_binary(), parse_i32_le),
        ("BinaryFormat.SignedInteger64",   one_binary(), parse_i64_le),
        ("BinaryFormat.UnsignedInteger16", one_binary(), parse_u16_le),
        ("BinaryFormat.UnsignedInteger32", one_binary(), parse_u32_le),
        ("BinaryFormat.UnsignedInteger64", one_binary(), parse_u64_le),
        ("BinaryFormat.Single",            one_binary(), parse_f32_le),
        ("BinaryFormat.Double",            one_binary(), parse_f64_le),
        ("BinaryFormat.Decimal",           one_binary(), parse_decimal_le),
    ]
}

fn one_binary() -> Vec<Param> {
    vec![Param { name: "binary".into(), optional: false, type_annotation: None }]
}

fn expect_bytes<'a>(v: &'a Value, ctx: &str) -> Result<&'a [u8], MError> {
    match v {
        Value::Binary(b) => Ok(b),
        Value::Null => Err(MError::Other(format!("{ctx}: binary is null"))),
        other => Err(type_mismatch("binary", other)),
    }
}

fn need<'a>(bytes: &'a [u8], n: usize, ctx: &str) -> Result<&'a [u8], MError> {
    if bytes.len() < n {
        return Err(MError::Other(format!(
            "{ctx}: binary too short — expected {n} bytes, got {}",
            bytes.len()
        )));
    }
    Ok(&bytes[..n])
}

// --- 8-bit ---

fn parse_byte(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Byte")?;
    let bs = need(b, 1, "BinaryFormat.Byte")?;
    Ok(Value::Number(bs[0] as f64))
}

// --- 16-bit ---

fn parse_u16_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.UnsignedInteger16")?;
    let bs = need(b, 2, "BinaryFormat.UnsignedInteger16")?;
    Ok(Value::Number(u16::from_le_bytes([bs[0], bs[1]]) as f64))
}

fn parse_i16_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.SignedInteger16")?;
    let bs = need(b, 2, "BinaryFormat.SignedInteger16")?;
    Ok(Value::Number(i16::from_le_bytes([bs[0], bs[1]]) as f64))
}

// --- 32-bit ---

fn parse_u32_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.UnsignedInteger32")?;
    let bs = need(b, 4, "BinaryFormat.UnsignedInteger32")?;
    Ok(Value::Number(u32::from_le_bytes([bs[0], bs[1], bs[2], bs[3]]) as f64))
}

fn parse_i32_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.SignedInteger32")?;
    let bs = need(b, 4, "BinaryFormat.SignedInteger32")?;
    Ok(Value::Number(i32::from_le_bytes([bs[0], bs[1], bs[2], bs[3]]) as f64))
}

// --- 64-bit (precision note: u64/i64 max precisely representable in
//     f64 is ±2^53. Values outside that range will round. f64 is the
//     numeric representation mrsflow uses for everything, so this is
//     consistent with the rest of the engine.)

fn parse_u64_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.UnsignedInteger64")?;
    let bs = need(b, 8, "BinaryFormat.UnsignedInteger64")?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(bs);
    Ok(Value::Number(u64::from_le_bytes(buf) as f64))
}

fn parse_i64_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.SignedInteger64")?;
    let bs = need(b, 8, "BinaryFormat.SignedInteger64")?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(bs);
    Ok(Value::Number(i64::from_le_bytes(buf) as f64))
}

// --- floats ---

fn parse_f32_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Single")?;
    let bs = need(b, 4, "BinaryFormat.Single")?;
    Ok(Value::Number(f32::from_le_bytes([bs[0], bs[1], bs[2], bs[3]]) as f64))
}

fn parse_f64_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Double")?;
    let bs = need(b, 8, "BinaryFormat.Double")?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(bs);
    Ok(Value::Number(f64::from_le_bytes(buf)))
}

// --- decimal (16-byte .NET decimal128) ---

fn parse_decimal_le(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Decimal")?;
    let _bs = need(b, 16, "BinaryFormat.Decimal")?;
    // .NET decimal layout: 4 ints (low / mid / high / flags). The flags
    // word encodes sign bit (bit 31) and scale (bits 16-23). For mrsflow
    // (f64 backend) we don't get exact precision anyway; reconstruct
    // the unsigned 96-bit mantissa then apply sign and scale.
    let lo = u32::from_le_bytes([_bs[0],  _bs[1],  _bs[2],  _bs[3]])  as u64;
    let mid = u32::from_le_bytes([_bs[4],  _bs[5],  _bs[6],  _bs[7]]) as u64;
    let hi  = u32::from_le_bytes([_bs[8],  _bs[9],  _bs[10], _bs[11]]) as u64;
    let flags = u32::from_le_bytes([_bs[12], _bs[13], _bs[14], _bs[15]]);
    let mantissa = ((hi as u128) << 64) | ((mid as u128) << 32) | (lo as u128);
    let scale = ((flags >> 16) & 0xFF) as i32;
    let negative = (flags >> 31) & 1 == 1;
    let mut value = mantissa as f64;
    if scale > 0 {
        value /= 10f64.powi(scale);
    }
    if negative {
        value = -value;
    }
    Ok(Value::Number(value))
}
