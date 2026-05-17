//! `BinaryFormat.*` — declarative parser-combinator framework for binary
//! streams ("Wireshark in M": describe a wire format, get a typed parser).
//!
//! ## Public shape
//!
//! Every `BinaryFormat.X` is a single-arg M function `(binary) => value`.
//! Atoms apply directly:
//!   `BinaryFormat.UnsignedInteger32(#binary({1,0,0,0})) = 1`
//! Factories take parameters and return a combinator:
//!   `BinaryFormat.Binary(3)(#binary({1,2,3,4,5})) = Binary(0x01,0x02,0x03)`
//! Composers combine inner combinators:
//!   `BinaryFormat.Record([a = BinaryFormat.UnsignedInteger16,
//!                        b = BinaryFormat.Byte])(input) = [a=..., b=...]`
//!
//! ## Internal protocol (size-aware)
//!
//! Composers need to know how many bytes each inner combinator consumed
//! so they can advance through the buffer. The public closure shape
//! (`binary -> value`) doesn't expose this. Solution:
//!
//!   - Every BinaryFormat combinator closure carries a captured
//!     `__bf_id__` text field naming the combinator (e.g. "Byte",
//!     "Binary", "Record").
//!   - The `parse_with_size(value, bytes, host) -> (Value, usize)`
//!     dispatcher reads `__bf_id__` from the closure's env, looks up
//!     the matching Rust parser in a static table, and returns both
//!     the parsed value and the byte count consumed.
//!   - Public closures still return only the value; composers call the
//!     internal `parse_with_size` directly on inner combinators.
//!
//! This makes every combinator size-aware without changing user-facing
//! M behaviour. Composers can nest variable-length combinators (varints,
//! Binary(n), Choice) because each reports its actual consumption.
//!
//! ## Endianness
//!
//! Atoms default to little-endian (PQ's default). `BinaryFormat.ByteOrder
//! (format, byteOrder)` wraps a fixed-width primitive and reverses the
//! input prefix before delegating. Variable-length combinators (varints,
//! Binary(n), Text(n)) ignore byte-order — there's no well-defined
//! reverse for them.
//!
//! ## Decimal precision
//!
//! `BinaryFormat.Decimal` reads a 16-byte .NET decimal128. mrsflow
//! stores numbers as f64, so the decimal is converted lossily — same
//! compromise everywhere else in the engine.

use std::sync::Arc;

use crate::parser::{Expr, Param};

use super::super::env::{EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Record, Value};
use super::common::{invoke_callback_with_host, type_mismatch};

// =====================================================================
// Public bindings
// =====================================================================

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        // Atoms — registered via factories that build tagged closures.
        ("BinaryFormat.Byte",              vec![], factory_atom_byte),
        ("BinaryFormat.SignedInteger16",   vec![], factory_atom_i16),
        ("BinaryFormat.SignedInteger32",   vec![], factory_atom_i32),
        ("BinaryFormat.SignedInteger64",   vec![], factory_atom_i64),
        ("BinaryFormat.UnsignedInteger16", vec![], factory_atom_u16),
        ("BinaryFormat.UnsignedInteger32", vec![], factory_atom_u32),
        ("BinaryFormat.UnsignedInteger64", vec![], factory_atom_u64),
        ("BinaryFormat.Single",            vec![], factory_atom_f32),
        ("BinaryFormat.Double",            vec![], factory_atom_f64),
        ("BinaryFormat.Decimal",           vec![], factory_atom_decimal),
        ("BinaryFormat.Null",              vec![], factory_atom_null),
        ("BinaryFormat.7BitEncodedUnsignedInteger", vec![], factory_atom_varint_u),
        ("BinaryFormat.7BitEncodedSignedInteger",   vec![], factory_atom_varint_s),

        // Factories — take parameters, return a new combinator.
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

        // Composers — orchestrate inner combinators.
        (
            "BinaryFormat.Record",
            vec![Param { name: "fields".into(), optional: false, type_annotation: None }],
            factory_record,
        ),
        (
            "BinaryFormat.Group",
            vec![Param { name: "formats".into(), optional: false, type_annotation: None }],
            factory_group,
        ),
        (
            "BinaryFormat.List",
            vec![
                Param { name: "format".into(),     optional: false, type_annotation: None },
                Param { name: "countOrEnd".into(), optional: true,  type_annotation: None },
            ],
            factory_list,
        ),
        (
            "BinaryFormat.Choice",
            vec![
                Param { name: "keyFormat".into(), optional: false, type_annotation: None },
                Param { name: "chooser".into(),   optional: false, type_annotation: None },
            ],
            factory_choice,
        ),

        // Meta combinators.
        (
            "BinaryFormat.Length",
            vec![
                Param { name: "format".into(),     optional: false, type_annotation: None },
                Param { name: "lengthFormat".into(), optional: true, type_annotation: None },
            ],
            factory_length,
        ),
        (
            "BinaryFormat.Transform",
            vec![
                Param { name: "format".into(),   optional: false, type_annotation: None },
                Param { name: "function".into(), optional: false, type_annotation: None },
            ],
            factory_transform,
        ),
    ]
}

// =====================================================================
// Closure construction — every BinaryFormat combinator is a closure
// tagged with __bf_id__ so parse_with_size can dispatch.
// =====================================================================

const BF_ID: &str = "__bf_id__";

/// Build a combinator closure: `(binary) => __bf_impl__(binary, ...captures)`.
/// `id` is the BinaryFormat name (e.g. "Byte") that parse_with_size will
/// use to look up the size-aware parser.
fn make_combinator(id: &str, captures: Vec<(String, Value)>, impl_fn: BuiltinFn) -> Value {
    let mut env = EnvNode::empty();
    let mut impl_params: Vec<Param> = vec![Param {
        name: "binary".into(),
        optional: false,
        type_annotation: None,
    }];
    let mut call_args: Vec<Expr> = vec![Expr::Identifier("binary".into())];

    // Always capture the id first.
    env = env.extend(BF_ID.to_string(), Value::Text(id.to_string()));

    // Capture user-supplied parameters; bind them in env AND add them to
    // the impl function's positional args.
    for (k, v) in &captures {
        env = env.extend(k.clone(), v.clone());
        impl_params.push(Param {
            name: k.clone(),
            optional: false,
            type_annotation: None,
        });
        call_args.push(Expr::Identifier(k.clone()));
    }

    let impl_name = "__bf_impl__".to_string();
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

// =====================================================================
// Internal parse_with_size dispatcher
// =====================================================================

/// Parse `bytes` using a `BinaryFormat.*` combinator value, returning
/// `(parsed_value, bytes_consumed)`. The combinator must be a closure
/// built via `make_combinator` (i.e. tagged with `__bf_id__`).
///
/// Used by composers to walk a byte stream while tracking exact
/// consumption.
fn parse_with_size(
    combinator: &Value,
    bytes: &[u8],
    host: &dyn IoHost,
) -> Result<(Value, usize), MError> {
    let closure = match combinator {
        Value::Function(c) => c,
        other => return Err(type_mismatch("function (BinaryFormat combinator)", other)),
    };
    let id = closure
        .env
        .lookup(BF_ID)
        .ok_or_else(|| MError::Other(
            "BinaryFormat composer: inner value is not a BinaryFormat combinator \
             (missing __bf_id__ tag)".into(),
        ))?;
    let id_str = match id {
        Value::Text(s) => s,
        _ => return Err(MError::Other(
            "BinaryFormat composer: __bf_id__ is not text".into(),
        )),
    };
    // Look up the matching size-aware parser.
    match id_str.as_str() {
        "Byte"              => parse_byte_sz(bytes),
        "SignedInteger16"   => parse_i16_sz(bytes),
        "SignedInteger32"   => parse_i32_sz(bytes),
        "SignedInteger64"   => parse_i64_sz(bytes),
        "UnsignedInteger16" => parse_u16_sz(bytes),
        "UnsignedInteger32" => parse_u32_sz(bytes),
        "UnsignedInteger64" => parse_u64_sz(bytes),
        "Single"            => parse_f32_sz(bytes),
        "Double"            => parse_f64_sz(bytes),
        "Decimal"           => parse_decimal_sz(bytes),
        "Null"              => Ok((Value::Null, 0)),
        "7BitEncodedUnsignedInteger" => parse_varint_u_sz(bytes),
        "7BitEncodedSignedInteger"   => parse_varint_s_sz(bytes),
        "Binary"            => parse_binary_sz(closure, bytes),
        "Text"              => parse_text_sz(closure, bytes),
        "ByteOrder"         => parse_byte_order_sz(closure, bytes, host),
        "Record"            => parse_record_sz(closure, bytes, host),
        "Group"             => parse_group_sz(closure, bytes, host),
        "List"              => parse_list_sz(closure, bytes, host),
        "Choice"            => parse_choice_sz(closure, bytes, host),
        "Length"            => parse_length_sz(closure, bytes, host),
        "Transform"         => parse_transform_sz(closure, bytes, host),
        other => Err(MError::Other(format!(
            "BinaryFormat composer: unknown combinator id {other:?}"
        ))),
    }
}

// =====================================================================
// Atom factories — build a tagged closure with no captures
// =====================================================================

fn factory_atom_byte(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("Byte", vec![], impl_atom_byte))
}
fn factory_atom_i16(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("SignedInteger16", vec![], impl_atom_i16))
}
fn factory_atom_i32(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("SignedInteger32", vec![], impl_atom_i32))
}
fn factory_atom_i64(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("SignedInteger64", vec![], impl_atom_i64))
}
fn factory_atom_u16(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("UnsignedInteger16", vec![], impl_atom_u16))
}
fn factory_atom_u32(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("UnsignedInteger32", vec![], impl_atom_u32))
}
fn factory_atom_u64(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("UnsignedInteger64", vec![], impl_atom_u64))
}
fn factory_atom_f32(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("Single", vec![], impl_atom_f32))
}
fn factory_atom_f64(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("Double", vec![], impl_atom_f64))
}
fn factory_atom_decimal(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("Decimal", vec![], impl_atom_decimal))
}
fn factory_atom_null(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("Null", vec![], impl_atom_null))
}
fn factory_atom_varint_u(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("7BitEncodedUnsignedInteger", vec![], impl_atom_varint_u))
}
fn factory_atom_varint_s(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(make_combinator("7BitEncodedSignedInteger", vec![], impl_atom_varint_s))
}

// Atom builtins — public path (just project the value out of the
// internal size-aware parse).
macro_rules! impl_atom {
    ($name:ident, $sz:ident) => {
        fn $name(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
            let b = expect_bytes(&args[0], "BinaryFormat atom")?;
            Ok($sz(b)?.0)
        }
    };
}
impl_atom!(impl_atom_byte,     parse_byte_sz);
impl_atom!(impl_atom_i16,      parse_i16_sz);
impl_atom!(impl_atom_i32,      parse_i32_sz);
impl_atom!(impl_atom_i64,      parse_i64_sz);
impl_atom!(impl_atom_u16,      parse_u16_sz);
impl_atom!(impl_atom_u32,      parse_u32_sz);
impl_atom!(impl_atom_u64,      parse_u64_sz);
impl_atom!(impl_atom_f32,      parse_f32_sz);
impl_atom!(impl_atom_f64,      parse_f64_sz);
impl_atom!(impl_atom_decimal,  parse_decimal_sz);
impl_atom!(impl_atom_varint_u, parse_varint_u_sz);
impl_atom!(impl_atom_varint_s, parse_varint_s_sz);

fn impl_atom_null(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Null)
}

// =====================================================================
// Helpers
// =====================================================================

fn expect_bytes<'a>(v: &'a Value, ctx: &str) -> Result<&'a [u8], MError> {
    match v {
        Value::Binary(b) => Ok(b),
        Value::Null => Err(MError::Other(format!("{ctx}: binary is null"))),
        other => Err(type_mismatch("binary", other)),
    }
}

fn need(bytes: &[u8], n: usize, ctx: &str) -> Result<(), MError> {
    if bytes.len() < n {
        return Err(MError::Other(format!(
            "{ctx}: binary too short — expected {n} bytes, got {}",
            bytes.len()
        )));
    }
    Ok(())
}

fn capture(closure: &Closure, name: &str) -> Option<Value> {
    closure.env.lookup(name)
}

fn capture_required(closure: &Closure, name: &str, ctx: &str) -> Result<Value, MError> {
    capture(closure, name).ok_or_else(|| MError::Other(format!(
        "{ctx}: missing required capture {name}"
    )))
}

fn as_usize(v: &Value, ctx: &str) -> Result<usize, MError> {
    match v {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => Ok(*n as usize),
        other => Err(MError::Other(format!(
            "{ctx}: expected non-negative integer, found {:?}", other
        ))),
    }
}

fn as_opt_usize(v: &Value, ctx: &str) -> Result<Option<usize>, MError> {
    match v {
        Value::Null => Ok(None),
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => Ok(Some(*n as usize)),
        other => Err(MError::Other(format!(
            "{ctx}: expected non-negative integer or null, found {:?}", other
        ))),
    }
}

fn as_i32(v: &Value, ctx: &str) -> Result<i32, MError> {
    match v {
        Value::Number(n) if n.fract() == 0.0 => Ok(*n as i32),
        other => Err(MError::Other(format!(
            "{ctx}: expected integer, found {:?}", other
        ))),
    }
}

fn as_i64(v: &Value, ctx: &str) -> Result<i64, MError> {
    match v {
        Value::Number(n) => Ok(*n as i64),
        other => Err(MError::Other(format!(
            "{ctx}: expected integer, found {:?}", other
        ))),
    }
}

fn as_closure(v: &Value, ctx: &str) -> Result<Closure, MError> {
    match v {
        Value::Function(c) => Ok(c.clone()),
        other => Err(MError::Other(format!(
            "{ctx}: expected function, found {:?}", other
        ))),
    }
}

// =====================================================================
// Atom parsers (size-aware) — return (value, bytes_consumed)
// =====================================================================

fn parse_byte_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 1, "BinaryFormat.Byte")?;
    Ok((Value::Number(b[0] as f64), 1))
}

fn parse_u16_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 2, "BinaryFormat.UnsignedInteger16")?;
    Ok((Value::Number(u16::from_le_bytes([b[0], b[1]]) as f64), 2))
}

fn parse_i16_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 2, "BinaryFormat.SignedInteger16")?;
    Ok((Value::Number(i16::from_le_bytes([b[0], b[1]]) as f64), 2))
}

fn parse_u32_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 4, "BinaryFormat.UnsignedInteger32")?;
    Ok((Value::Number(u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64), 4))
}

fn parse_i32_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 4, "BinaryFormat.SignedInteger32")?;
    Ok((Value::Number(i32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64), 4))
}

fn parse_u64_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 8, "BinaryFormat.UnsignedInteger64")?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&b[..8]);
    Ok((Value::Number(u64::from_le_bytes(buf) as f64), 8))
}

fn parse_i64_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 8, "BinaryFormat.SignedInteger64")?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&b[..8]);
    Ok((Value::Number(i64::from_le_bytes(buf) as f64), 8))
}

fn parse_f32_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 4, "BinaryFormat.Single")?;
    Ok((Value::Number(f32::from_le_bytes([b[0], b[1], b[2], b[3]]) as f64), 4))
}

fn parse_f64_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 8, "BinaryFormat.Double")?;
    let mut buf = [0u8; 8];
    buf.copy_from_slice(&b[..8]);
    Ok((Value::Number(f64::from_le_bytes(buf)), 8))
}

fn parse_decimal_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    need(b, 16, "BinaryFormat.Decimal")?;
    let lo  = u32::from_le_bytes([b[0],  b[1],  b[2],  b[3]])  as u64;
    let mid = u32::from_le_bytes([b[4],  b[5],  b[6],  b[7]])  as u64;
    let hi  = u32::from_le_bytes([b[8],  b[9],  b[10], b[11]]) as u64;
    let flags = u32::from_le_bytes([b[12], b[13], b[14], b[15]]);
    let mantissa = ((hi as u128) << 64) | ((mid as u128) << 32) | (lo as u128);
    let scale = ((flags >> 16) & 0xFF) as i32;
    let negative = (flags >> 31) & 1 == 1;
    let mut value = mantissa as f64;
    if scale > 0 { value /= 10f64.powi(scale); }
    if negative { value = -value; }
    Ok((Value::Number(value), 16))
}

fn parse_varint_u_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    for (i, &byte) in b.iter().enumerate() {
        if i >= 10 {
            return Err(MError::Other(
                "BinaryFormat.7BitEncodedUnsignedInteger: varint exceeds 10 bytes".into(),
            ));
        }
        result |= ((byte & 0x7F) as u64) << shift;
        if byte & 0x80 == 0 {
            return Ok((Value::Number(result as f64), i + 1));
        }
        shift += 7;
    }
    Err(MError::Other(
        "BinaryFormat.7BitEncodedUnsignedInteger: varint truncated".into(),
    ))
}

fn parse_varint_s_sz(b: &[u8]) -> Result<(Value, usize), MError> {
    let (val, consumed) = parse_varint_u_sz(b)?;
    match val {
        Value::Number(n) => Ok((Value::Number(n as u64 as i64 as f64), consumed)),
        other => Ok((other, consumed)),
    }
}

// =====================================================================
// Factories (with captures) and their size-aware parsers
// =====================================================================

// --- BinaryFormat.Binary(length, [padding]) ---

fn factory_binary(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let len = match args.get(0) {
        None | Some(Value::Null) => Value::Null,
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => Value::Number(*n),
        Some(other) => return Err(type_mismatch("number (length)", other)),
    };
    Ok(make_combinator("Binary", vec![("length".into(), len)], impl_factory_binary))
}

fn impl_factory_binary(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // args: [binary, length]
    let b = expect_bytes(&args[0], "BinaryFormat.Binary")?;
    let len = &args[1];
    let n = match len {
        Value::Number(n) => Some(*n as usize),
        Value::Null => None,
        other => return Err(type_mismatch("number (length)", other)),
    };
    match n {
        None => Ok(Value::Binary(b.to_vec())),
        Some(k) => {
            need(b, k, "BinaryFormat.Binary")?;
            Ok(Value::Binary(b[..k].to_vec()))
        }
    }
}

fn parse_binary_sz(closure: &Closure, b: &[u8]) -> Result<(Value, usize), MError> {
    let len_v = capture_required(closure, "length", "BinaryFormat.Binary")?;
    let n = as_opt_usize(&len_v, "BinaryFormat.Binary length")?;
    match n {
        None => Ok((Value::Binary(b.to_vec()), b.len())),
        Some(k) => {
            need(b, k, "BinaryFormat.Binary")?;
            Ok((Value::Binary(b[..k].to_vec()), k))
        }
    }
}

// --- BinaryFormat.Text(length, [encoding]) ---

fn factory_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let len = match args.get(0) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n,
        _ => return Err(MError::Other("BinaryFormat.Text: length required".into())),
    };
    let encoding = match args.get(1) {
        None | Some(Value::Null) => 65001.0,
        Some(Value::Number(n)) => *n,
        Some(other) => return Err(type_mismatch("number (encoding)", other)),
    };
    Ok(make_combinator(
        "Text",
        vec![
            ("length".into(),   Value::Number(len)),
            ("encoding".into(), Value::Number(encoding)),
        ],
        impl_factory_text,
    ))
}

fn impl_factory_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Text")?;
    let len = match &args[1] {
        Value::Number(n) => *n as usize,
        other => return Err(type_mismatch("number (length)", other)),
    };
    let encoding = match &args[2] {
        Value::Number(n) => *n as i64,
        other => return Err(type_mismatch("number (encoding)", other)),
    };
    need(b, len, "BinaryFormat.Text")?;
    if encoding != 65001 {
        return Err(MError::Other(format!(
            "BinaryFormat.Text: only UTF-8 (65001) supported; got {encoding}"
        )));
    }
    let s = std::str::from_utf8(&b[..len])
        .map_err(|e| MError::Other(format!("BinaryFormat.Text: invalid UTF-8: {e}")))?;
    Ok(Value::Text(s.to_string()))
}

fn parse_text_sz(closure: &Closure, b: &[u8]) -> Result<(Value, usize), MError> {
    let len_v = capture_required(closure, "length", "BinaryFormat.Text")?;
    let enc_v = capture_required(closure, "encoding", "BinaryFormat.Text")?;
    let len = as_usize(&len_v, "BinaryFormat.Text length")?;
    let encoding = as_i64(&enc_v, "BinaryFormat.Text encoding")?;
    need(b, len, "BinaryFormat.Text")?;
    if encoding != 65001 {
        return Err(MError::Other(format!(
            "BinaryFormat.Text: only UTF-8 (65001) supported; got {encoding}"
        )));
    }
    let s = std::str::from_utf8(&b[..len])
        .map_err(|e| MError::Other(format!("BinaryFormat.Text: invalid UTF-8: {e}")))?;
    Ok((Value::Text(s.to_string()), len))
}

// --- BinaryFormat.ByteOrder(format, byteOrder) ---

fn factory_byte_order(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if !matches!(&args[0], Value::Function(_)) {
        return Err(type_mismatch("function (format)", &args[0]));
    }
    let order = match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 => *n as i32,
        Some(other) => return Err(type_mismatch("number (byteOrder)", other)),
        None => return Err(MError::Other("BinaryFormat.ByteOrder: byteOrder required".into())),
    };
    Ok(make_combinator(
        "ByteOrder",
        vec![
            ("inner_fmt".into(), args[0].clone()),
            ("byte_order".into(), Value::Number(order as f64)),
        ],
        impl_factory_byte_order,
    ))
}

fn impl_factory_byte_order(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    // args: [binary, inner_fmt, byte_order]
    let b = expect_bytes(&args[0], "BinaryFormat.ByteOrder")?;
    let inner = match &args[1] {
        Value::Function(c) => c.clone(),
        other => return Err(type_mismatch("function (inner format)", other)),
    };
    let order = match &args[2] {
        Value::Number(n) => *n as i32,
        other => return Err(type_mismatch("number (byteOrder)", other)),
    };
    // Reverse only the prefix the inner combinator consumes — reversing
    // the whole buffer would change which bytes the inner reads when the
    // caller passes a slice larger than the combinator needs (which is
    // typical when chunking a wire format).
    let inner_v = Value::Function(inner.clone());
    let (_, consumed) = parse_with_size(&inner_v, b, host)?;
    let prefix: Vec<u8> = if order == 0 {
        b[..consumed].iter().rev().cloned().collect()
    } else {
        b[..consumed].to_vec()
    };
    invoke_callback_with_host(&inner, vec![Value::Binary(prefix)], host)
}

fn parse_byte_order_sz(
    closure: &Closure,
    b: &[u8],
    host: &dyn IoHost,
) -> Result<(Value, usize), MError> {
    let inner_v = capture_required(closure, "inner_fmt", "BinaryFormat.ByteOrder")?;
    let order_v = capture_required(closure, "byte_order", "BinaryFormat.ByteOrder")?;
    let order = as_i32(&order_v, "BinaryFormat.ByteOrder")?;
    // Discover inner's byte size by parsing forward first.
    let (_, consumed) = parse_with_size(&inner_v, b, host)?;
    let prefix: Vec<u8> = if order == 0 {
        b[..consumed].iter().rev().cloned().collect()
    } else {
        b[..consumed].to_vec()
    };
    let (val, _) = parse_with_size(&inner_v, &prefix, host)?;
    Ok((val, consumed))
}

// =====================================================================
// Composers
// =====================================================================

// --- BinaryFormat.Record([field = format, ...]) ---
//
// Take a record where each value is an inner combinator. Parse them in
// field order, building a result record.

fn factory_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let rec = match &args[0] {
        Value::Record(r) => r.clone(),
        other => return Err(type_mismatch("record (fields)", other)),
    };
    Ok(make_combinator(
        "Record",
        vec![("fields".into(), Value::Record(rec))],
        impl_factory_record,
    ))
}

fn impl_factory_record(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Record")?;
    let fields = match &args[1] {
        Value::Record(r) => r.clone(),
        other => return Err(type_mismatch("record (fields)", other)),
    };
    let mut offset = 0;
    let mut out_fields: Vec<(String, Value)> = Vec::with_capacity(fields.fields.len());
    for (name, fmt) in &fields.fields {
        // Record literal fields are stored as thunks — force before
        // dispatching as a combinator.
        let forced = super::super::force(fmt.clone(), &mut |e, env| {
            super::super::evaluate(e, env, host)
        })?;
        let (val, consumed) = parse_with_size(&forced, &b[offset..], host)?;
        out_fields.push((name.clone(), val));
        offset += consumed;
    }
    Ok(Value::Record(Record { fields: out_fields, env: EnvNode::empty() }))
}

fn parse_record_sz(
    closure: &Closure,
    b: &[u8],
    host: &dyn IoHost,
) -> Result<(Value, usize), MError> {
    let fields_v = capture_required(closure, "fields", "BinaryFormat.Record")?;
    let fields = match &fields_v {
        Value::Record(r) => r.clone(),
        other => return Err(type_mismatch("record (fields)", other)),
    };
    let mut offset = 0;
    let mut out_fields: Vec<(String, Value)> = Vec::with_capacity(fields.fields.len());
    for (name, fmt) in &fields.fields {
        let forced = super::super::force(fmt.clone(), &mut |e, env| {
            super::super::evaluate(e, env, host)
        })?;
        let (val, consumed) = parse_with_size(&forced, &b[offset..], host)?;
        out_fields.push((name.clone(), val));
        offset += consumed;
    }
    Ok((
        Value::Record(Record { fields: out_fields, env: EnvNode::empty() }),
        offset,
    ))
}

// --- BinaryFormat.Group({format, ...}) ---
//
// Like Record but takes a list of unnamed combinators; result is a list
// of parsed values.

fn factory_group(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let items = match &args[0] {
        Value::List(l) => l.clone(),
        other => return Err(type_mismatch("list (formats)", other)),
    };
    Ok(make_combinator(
        "Group",
        vec![("formats".into(), Value::List(items))],
        impl_factory_group,
    ))
}

fn impl_factory_group(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Group")?;
    let formats = match &args[1] {
        Value::List(l) => l.clone(),
        other => return Err(type_mismatch("list (formats)", other)),
    };
    let mut offset = 0;
    let mut out: Vec<Value> = Vec::with_capacity(formats.len());
    for fmt in formats.iter() {
        let (val, consumed) = parse_with_size(fmt, &b[offset..], host)?;
        out.push(val);
        offset += consumed;
    }
    Ok(Value::list_of(out))
}

fn parse_group_sz(
    closure: &Closure,
    b: &[u8],
    host: &dyn IoHost,
) -> Result<(Value, usize), MError> {
    let formats_v = capture_required(closure, "formats", "BinaryFormat.Group")?;
    let formats = match &formats_v {
        Value::List(l) => l.clone(),
        other => return Err(type_mismatch("list (formats)", other)),
    };
    let mut offset = 0;
    let mut out: Vec<Value> = Vec::with_capacity(formats.len());
    for fmt in formats.iter() {
        let (val, consumed) = parse_with_size(fmt, &b[offset..], host)?;
        out.push(val);
        offset += consumed;
    }
    Ok((Value::list_of(out), offset))
}

// --- BinaryFormat.List(format, [countOrEnd]) ---
//
// Parse `format` repeatedly. The 2nd arg is either a number (parse
// exactly that many times) or a terminator format value (parse until
// the terminator matches — we don't implement the terminator form for
// v1; require a count). Without a count, consume until input exhausted.

fn factory_list(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let fmt = args.get(0).cloned().ok_or_else(|| {
        MError::Other("BinaryFormat.List: format required".into())
    })?;
    let count_or_end = args.get(1).cloned().unwrap_or(Value::Null);
    Ok(make_combinator(
        "List",
        vec![
            ("format".into(),     fmt),
            ("countOrEnd".into(), count_or_end),
        ],
        impl_factory_list,
    ))
}

fn impl_factory_list(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.List")?;
    let fmt = &args[1];
    let count_or_end = &args[2];
    let count = match count_or_end {
        Value::Null => None,
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => Some(*n as usize),
        Value::Function(_) => {
            return Err(MError::Other(
                "BinaryFormat.List: terminator-format end marker not yet supported; \
                 pass a numeric count".into(),
            ));
        }
        other => return Err(type_mismatch("number or function (countOrEnd)", other)),
    };
    let mut offset = 0;
    let mut out: Vec<Value> = Vec::new();
    if let Some(n) = count {
        for _ in 0..n {
            let (val, consumed) = parse_with_size(fmt, &b[offset..], host)?;
            out.push(val);
            offset += consumed;
        }
    } else {
        while offset < b.len() {
            let (val, consumed) = parse_with_size(fmt, &b[offset..], host)?;
            out.push(val);
            offset += consumed;
            if consumed == 0 {
                // Guard against infinite loop with a zero-byte combinator.
                break;
            }
        }
    }
    Ok(Value::list_of(out))
}

fn parse_list_sz(
    closure: &Closure,
    b: &[u8],
    host: &dyn IoHost,
) -> Result<(Value, usize), MError> {
    let fmt = capture_required(closure, "format", "BinaryFormat.List")?;
    let count_or_end = capture_required(closure, "countOrEnd", "BinaryFormat.List")?;
    let count = match &count_or_end {
        Value::Null => None,
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => Some(*n as usize),
        Value::Function(_) => {
            return Err(MError::Other(
                "BinaryFormat.List: terminator-format end marker not yet supported".into(),
            ));
        }
        other => return Err(type_mismatch("number or function (countOrEnd)", other)),
    };
    let mut offset = 0;
    let mut out: Vec<Value> = Vec::new();
    if let Some(n) = count {
        for _ in 0..n {
            let (val, consumed) = parse_with_size(&fmt, &b[offset..], host)?;
            out.push(val);
            offset += consumed;
        }
    } else {
        while offset < b.len() {
            let (val, consumed) = parse_with_size(&fmt, &b[offset..], host)?;
            out.push(val);
            offset += consumed;
            if consumed == 0 {
                break;
            }
        }
    }
    Ok((Value::list_of(out), offset))
}

// --- BinaryFormat.Choice(keyFormat, chooser) ---
//
// Parse keyFormat to get a tag. Apply `chooser` (an M function: key ->
// inner format) to pick the next combinator. Parse the rest with it.

fn factory_choice(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let key_fmt = args.get(0).cloned().ok_or_else(|| {
        MError::Other("BinaryFormat.Choice: keyFormat required".into())
    })?;
    let chooser = args.get(1).cloned().ok_or_else(|| {
        MError::Other("BinaryFormat.Choice: chooser required".into())
    })?;
    if !matches!(chooser, Value::Function(_)) {
        return Err(type_mismatch("function (chooser)", &chooser));
    }
    Ok(make_combinator(
        "Choice",
        vec![
            ("keyFormat".into(), key_fmt),
            ("chooser".into(),   chooser),
        ],
        impl_factory_choice,
    ))
}

fn impl_factory_choice(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Choice")?;
    let key_fmt = &args[1];
    let chooser = match &args[2] {
        Value::Function(c) => c.clone(),
        other => return Err(type_mismatch("function (chooser)", other)),
    };
    let (key, consumed_key) = parse_with_size(key_fmt, b, host)?;
    let inner_fmt = invoke_callback_with_host(&chooser, vec![key], host)?;
    let (val, _) = parse_with_size(&inner_fmt, &b[consumed_key..], host)?;
    Ok(val)
}

fn parse_choice_sz(
    closure: &Closure,
    b: &[u8],
    host: &dyn IoHost,
) -> Result<(Value, usize), MError> {
    let key_fmt = capture_required(closure, "keyFormat", "BinaryFormat.Choice")?;
    let chooser_v = capture_required(closure, "chooser", "BinaryFormat.Choice")?;
    let chooser = as_closure(&chooser_v, "BinaryFormat.Choice chooser")?;
    let (key, consumed_key) = parse_with_size(&key_fmt, b, host)?;
    let inner_fmt = invoke_callback_with_host(&chooser, vec![key], host)?;
    let (val, consumed_inner) = parse_with_size(&inner_fmt, &b[consumed_key..], host)?;
    Ok((val, consumed_key + consumed_inner))
}

// =====================================================================
// Meta combinators
// =====================================================================

// --- BinaryFormat.Length(format, [lengthFormat]) ---
//
// Read a length using lengthFormat (default = u32), then parse the
// inner `format` against exactly that many bytes. Result is just the
// inner value; consumed = length-bytes + inner-bytes.

fn factory_length(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let fmt = args.get(0).cloned().ok_or_else(|| {
        MError::Other("BinaryFormat.Length: format required".into())
    })?;
    // Default lengthFormat is u32 little-endian — same as PQ.
    let length_fmt = match args.get(1) {
        Some(v) if !matches!(v, Value::Null) => v.clone(),
        _ => make_combinator("UnsignedInteger32", vec![], impl_atom_u32),
    };
    Ok(make_combinator(
        "Length",
        vec![
            ("format".into(),       fmt),
            ("lengthFormat".into(), length_fmt),
        ],
        impl_factory_length,
    ))
}

fn impl_factory_length(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Length")?;
    let fmt = &args[1];
    let length_fmt = &args[2];
    let (len_val, consumed_len) = parse_with_size(length_fmt, b, host)?;
    let n = match len_val {
        Value::Number(n) if n >= 0.0 => n as usize,
        other => return Err(type_mismatch("number (length result)", &other)),
    };
    need(&b[consumed_len..], n, "BinaryFormat.Length")?;
    let (val, _) = parse_with_size(fmt, &b[consumed_len..consumed_len + n], host)?;
    Ok(val)
}

fn parse_length_sz(
    closure: &Closure,
    b: &[u8],
    host: &dyn IoHost,
) -> Result<(Value, usize), MError> {
    let fmt = capture_required(closure, "format", "BinaryFormat.Length")?;
    let length_fmt = capture_required(closure, "lengthFormat", "BinaryFormat.Length")?;
    let (len_val, consumed_len) = parse_with_size(&length_fmt, b, host)?;
    let n = match len_val {
        Value::Number(n) if n >= 0.0 => n as usize,
        other => return Err(type_mismatch("number (length result)", &other)),
    };
    need(&b[consumed_len..], n, "BinaryFormat.Length")?;
    let (val, _) = parse_with_size(&fmt, &b[consumed_len..consumed_len + n], host)?;
    Ok((val, consumed_len + n))
}

// --- BinaryFormat.Transform(format, function) ---
//
// Parse with `format`, apply `function` to the result, return that.
// Byte count = whatever the inner format consumed.

fn factory_transform(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let fmt = args.get(0).cloned().ok_or_else(|| {
        MError::Other("BinaryFormat.Transform: format required".into())
    })?;
    let func = args.get(1).cloned().ok_or_else(|| {
        MError::Other("BinaryFormat.Transform: function required".into())
    })?;
    if !matches!(func, Value::Function(_)) {
        return Err(type_mismatch("function", &func));
    }
    Ok(make_combinator(
        "Transform",
        vec![
            ("format".into(),   fmt),
            ("function".into(), func),
        ],
        impl_factory_transform,
    ))
}

fn impl_factory_transform(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let b = expect_bytes(&args[0], "BinaryFormat.Transform")?;
    let fmt = &args[1];
    let func = match &args[2] {
        Value::Function(c) => c.clone(),
        other => return Err(type_mismatch("function", other)),
    };
    let (val, _) = parse_with_size(fmt, b, host)?;
    invoke_callback_with_host(&func, vec![val], host)
}

fn parse_transform_sz(
    closure: &Closure,
    b: &[u8],
    host: &dyn IoHost,
) -> Result<(Value, usize), MError> {
    let fmt = capture_required(closure, "format", "BinaryFormat.Transform")?;
    let func_v = capture_required(closure, "function", "BinaryFormat.Transform")?;
    let func = as_closure(&func_v, "BinaryFormat.Transform function")?;
    let (val, consumed) = parse_with_size(&fmt, b, host)?;
    let transformed = invoke_callback_with_host(&func, vec![val], host)?;
    Ok((transformed, consumed))
}

// Suppress unused-import warning when no test compiles needs Arc.
#[allow(dead_code)]
fn _arc_anchor() -> Option<Arc<()>> { None }
