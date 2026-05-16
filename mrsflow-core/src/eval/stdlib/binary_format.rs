//! `BinaryFormat.*` — declarative parser-combinator framework for binary
//! streams ("Wireshark in M": describe a wire format, get a typed parser).
//!
//! Two combinator shapes:
//!
//!   - **Atoms** (slice 1) — `BinaryFormat.Byte`, `.UnsignedInteger32`,
//!     `.Single`, etc. Each is a single-arg M function `(binary) => value`
//!     that parses a fixed number of bytes from the start.
//!
//!   - **Combinator factories** (slice 2 onwards) — `BinaryFormat.Binary`,
//!     `.Text`, `.ByteOrder`, later `.List`/`.Record`/`.Choice`. These take
//!     parameters and return a new combinator (itself a `(binary) => value`
//!     function). Built via `make_format_closure` which captures the
//!     parameters in the closure env, then dispatches to a Rust impl.
//!
//! Endianness: slice 1 hard-codes little-endian (PQ's default). Slice 2's
//! `BinaryFormat.ByteOrder(order, inner)` wraps an existing combinator
//! and byte-swaps the input prefix before delegating — works for the
//! fixed-width primitives without each one needing to know about
//! endianness.
//!
//! Decimal note: PQ's `BinaryFormat.Decimal` reads a 16-byte .NET-style
//! decimal-128 value. mrsflow stores numbers as f64 throughout, so the
//! decimal is converted lossily — same compromise we make everywhere.

use crate::parser::{Expr, Param};

use super::super::env::{EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Value};
use super::common::type_mismatch;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        // Atoms (slice 1) — applied directly to a binary.
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
        // BinaryFormat.Null — atom that consumes 0 bytes and returns null.
        // (Used as a no-op terminator by Choice / List.)
        ("BinaryFormat.Null",              one_binary(), parse_null),

        // Combinator factories (slice 2) — return a new (binary) => value
        // combinator. Implemented as factories that wrap an internal
        // impl_fn closure that gets the captured parameters via env.
        (
            "BinaryFormat.Binary",
            vec![
                Param { name: "length".into(),  optional: true, type_annotation: None },
                Param { name: "padding".into(), optional: true, type_annotation: None },
            ],
            factory_binary,
        ),
        (
            "BinaryFormat.Text",
            vec![
                Param { name: "length".into(),   optional: false, type_annotation: None },
                Param { name: "encoding".into(), optional: true,  type_annotation: None },
            ],
            factory_text,
        ),
        (
            "BinaryFormat.ByteOrder",
            vec![
                Param { name: "format".into(),    optional: false, type_annotation: None },
                Param { name: "byteOrder".into(), optional: false, type_annotation: None },
            ],
            factory_byte_order,
        ),
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

// --- null atom (0 bytes -> null) ---

fn parse_null(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Doesn't even look at the binary — the atom consumes nothing.
    Ok(Value::Null)
}

// --- factory helper: build a closure that captures named values and
//     dispatches to an impl_fn with [binary, capture1, capture2, ...] ---

/// Build a single-arg `(binary) => value` combinator closure that
/// captures `captures` and dispatches to `impl_fn`. The closure body is
/// `impl_fn(binary, cap1, cap2, ...)` — `impl_fn` reads the captures
/// from its positional args after the binary.
fn make_format_closure(captures: Vec<(String, Value)>, impl_fn: BuiltinFn) -> Value {
    let mut env = EnvNode::empty();
    let mut impl_params: Vec<Param> = vec![Param {
        name: "binary".into(),
        optional: false,
        type_annotation: None,
    }];
    let mut call_args: Vec<Expr> = vec![Expr::Identifier("binary".into())];
    for (k, v) in &captures {
        env = env.extend(k.clone(), v.clone());
        impl_params.push(Param { name: k.clone(), optional: false, type_annotation: None });
        call_args.push(Expr::Identifier(k.clone()));
    }
    let impl_name = "__bformat_impl__".to_string();
    let impl_closure = Value::Function(Closure {
        params: impl_params,
        body: FnBody::Builtin(impl_fn),
        env: EnvNode::empty(),
    });
    env = env.extend(impl_name.clone(), impl_closure);
    Value::Function(Closure {
        params: vec![Param {
            name: "binary".into(),
            optional: false,
            type_annotation: None,
        }],
        body: FnBody::M(Box::new(Expr::Invoke {
            target: Box::new(Expr::Identifier(impl_name)),
            args: call_args,
        })),
        env,
    })
}

// --- BinaryFormat.Binary(length, [padding]) ---

fn factory_binary(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // length: number or null. If null/omitted, consume the whole binary.
    let len = match args.get(0) {
        None | Some(Value::Null) => None,
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => Some(*n as usize),
        Some(other) => return Err(type_mismatch("number (length)", other)),
    };
    // padding is documented but we ignore it; capture it just in case
    // future calls compare combinators by parameter shape.
    let captures = vec![(
        "length".to_string(),
        match len {
            Some(n) => Value::Number(n as f64),
            None => Value::Null,
        },
    )];
    Ok(make_format_closure(captures, parse_binary_impl))
}

fn parse_binary_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Binary")?;
    let len = match &args[1] {
        Value::Number(n) => Some(*n as usize),
        Value::Null => None,
        other => return Err(type_mismatch("number (length)", other)),
    };
    match len {
        None => Ok(Value::Binary(b.to_vec())),
        Some(n) => {
            let bs = need(b, n, "BinaryFormat.Binary")?;
            Ok(Value::Binary(bs.to_vec()))
        }
    }
}

// --- BinaryFormat.Text(length, [encoding]) ---

fn factory_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let len = match args.get(0) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => {
            return Err(MError::Other(
                "BinaryFormat.Text: length is required".into(),
            ))
        }
        Some(other) => return Err(type_mismatch("number (length)", other)),
    };
    // encoding: optional numeric code page (TextEncoding.X = numeric).
    // mrsflow only decodes UTF-8 (65001) cleanly; other codepages emit
    // a clear error at parse time per the strict-encodings policy.
    let encoding = match args.get(1) {
        None | Some(Value::Null) => 65001.0,
        Some(Value::Number(n)) => *n,
        Some(other) => return Err(type_mismatch("number (encoding)", other)),
    };
    let captures = vec![
        ("length".to_string(),   Value::Number(len as f64)),
        ("encoding".to_string(), Value::Number(encoding)),
    ];
    Ok(make_format_closure(captures, parse_text_impl))
}

fn parse_text_impl(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Text")?;
    let len = match &args[1] {
        Value::Number(n) => *n as usize,
        other => return Err(type_mismatch("number (length)", other)),
    };
    let encoding = match &args[2] {
        Value::Number(n) => *n as i64,
        other => return Err(type_mismatch("number (encoding)", other)),
    };
    let bs = need(b, len, "BinaryFormat.Text")?;
    if encoding != 65001 {
        return Err(MError::Other(format!(
            "BinaryFormat.Text: only UTF-8 (TextEncoding.Utf8 = 65001) is \
             supported; got encoding {encoding}"
        )));
    }
    let s = std::str::from_utf8(bs).map_err(|e| {
        MError::Other(format!("BinaryFormat.Text: invalid UTF-8: {e}"))
    })?;
    Ok(Value::Text(s.to_string()))
}

// --- BinaryFormat.ByteOrder(format, byteOrder) ---
//
// Wraps a fixed-width primitive combinator so the input bytes are
// reversed before delegating, effectively switching the read from
// little-endian (our default) to big-endian when byteOrder is
// ByteOrder.BigEndian (0.0). LittleEndian (1.0) is a pass-through.
// Only works on fixed-width primitives — variable-length combinators
// don't have a well-defined byte-swap behaviour at this layer.

fn factory_byte_order(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // PQ signature: BinaryFormat.ByteOrder(binaryFormat, byteOrder)
    // (format first, byteOrder second).
    if !matches!(&args[0], Value::Function(_)) {
        return Err(type_mismatch("function (format)", &args[0]));
    }
    let order = match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 => *n as i32,
        Some(other) => return Err(type_mismatch("number (byteOrder)", other)),
        None => return Err(MError::Other("BinaryFormat.ByteOrder: byteOrder required".into())),
    };
    let captures = vec![
        ("inner_fmt".to_string(), args[0].clone()),
        ("byte_order".to_string(), Value::Number(order as f64)),
    ];
    Ok(make_format_closure(captures, byte_order_impl))
}

fn byte_order_impl(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.ByteOrder")?;
    let inner = match &args[1] {
        Value::Function(c) => c.clone(),
        other => return Err(type_mismatch("function (inner format)", other)),
    };
    let order = match &args[2] {
        Value::Number(n) => *n as i32,
        other => return Err(type_mismatch("number (byteOrder)", other)),
    };
    // ByteOrder.BigEndian = 0, LittleEndian = 1 (matches the constants
    // registered in stdlib::mod.rs).
    let bytes_to_pass: Vec<u8> = if order == 0 {
        b.iter().rev().cloned().collect()
    } else {
        b.to_vec()
    };
    super::common::invoke_callback_with_host(
        &inner,
        vec![Value::Binary(bytes_to_pass)],
        host,
    )
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
