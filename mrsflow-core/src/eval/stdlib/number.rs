//! `Number.*` stdlib bindings.

#![allow(unused_imports)]

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Date32Array, DurationMicrosecondArray, Float64Array,
    NullArray, StringArray, TimestampMicrosecondArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::parser::Param;

use super::super::env::{Env, EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Record, Table, Value};
use super::common::{
    expect_function, expect_int, expect_list, expect_list_of_lists, expect_table,
    expect_text, expect_text_list, int_n_arg, invoke_builtin_callback,
    invoke_callback_with_host, one, three, two, type_mismatch, values_equal_primitive,
};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Number.From", one("value"), from),
        (
            "Number.Mod",
            vec![
                Param { name: "number".into(),    optional: false, type_annotation: None },
                Param { name: "divisor".into(),   optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            mod_,
        ),
        (
            "Number.IntegerDivide",
            vec![
                Param { name: "number".into(),    optional: false, type_annotation: None },
                Param { name: "divisor".into(),   optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            integer_divide,
        ),
        ("Number.IsNaN", one("number"), is_nan),
        ("Number.IsOdd", one("number"), is_odd),
        ("Number.IsEven", one("number"), is_even),
        ("Number.Random", vec![], random),
        ("Number.RandomBetween", two("bottom", "top"), random_between),
        (
            "Number.RoundUp",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            round_up,
        ),
        (
            "Number.RoundDown",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            round_down,
        ),
        (
            "Number.RoundTowardZero",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            round_toward_zero,
        ),
        (
            "Number.RoundAwayFromZero",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            round_away_from_zero,
        ),
        ("Number.Acos", one("number"), acos),
        ("Number.Asin", one("number"), asin),
        ("Number.Atan", one("number"), atan),
        ("Number.Atan2", two("y", "x"), atan2),
        ("Number.Cos", one("number"), cos),
        ("Number.Cosh", one("number"), cosh),
        ("Number.Sin", one("number"), sin),
        ("Number.Sinh", one("number"), sinh),
        ("Number.Tan", one("number"), tan),
        ("Number.Tanh", one("number"), tanh),
        ("Number.Exp", one("number"), exp),
        ("Number.Ln", one("number"), ln),
        (
            "Number.Log",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "base".into(),   optional: true,  type_annotation: None },
            ],
            log,
        ),
        ("Number.Log10", one("number"), log10),
        ("Number.Factorial", one("number"), factorial),
        ("Number.Combinations", two("setSize", "combinationSize"), combinations),
        ("Number.Permutations", two("setSize", "combinationSize"), permutations),
        ("Number.BitwiseAnd", two("a", "b"), bitwise_and),
        ("Number.BitwiseOr", two("a", "b"), bitwise_or),
        ("Number.BitwiseXor", two("a", "b"), bitwise_xor),
        ("Number.BitwiseNot", one("a"), bitwise_not),
        ("Number.BitwiseShiftLeft", two("a", "n"), bitwise_shift_left),
        ("Number.BitwiseShiftRight", two("a", "n"), bitwise_shift_right),
        ("Byte.From", one("value"), from),
        ("Currency.From", one("value"), currency_from),
        ("Decimal.From", one("value"), from),
        ("Double.From", one("value"), from),
        ("Int8.From", one("value"), int_from),
        ("Int16.From", one("value"), int_from),
        ("Int32.From", one("value"), int_from),
        ("Int64.From", one("value"), int_from),
        (
            "Percentage.From",
            vec![
                Param { name: "value".into(),   optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            percentage_from,
        ),
        ("Single.From", one("value"), from),
        (
            "Number.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            from_text,
        ),
        (
            "Number.Round",
            vec![
                Param { name: "number".into(),        optional: false, type_annotation: None },
                Param { name: "digits".into(),        optional: true,  type_annotation: None },
                Param { name: "roundingMode".into(),  optional: true,  type_annotation: None },
            ],
            round,
        ),
        ("Number.Abs", one("number"), abs),
        (
            "Number.ToText",
            vec![
                Param { name: "number".into(),  optional: false, type_annotation: None },
                Param { name: "format".into(),  optional: true,  type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            to_text,
        ),
        ("Number.Sign", one("number"), sign),
        ("Number.Power", two("base", "exponent"), power),
        ("Number.Sqrt", one("number"), sqrt),
    ]
}

fn mod_(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let a = match &args[0] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    let b = match &args[1] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    // PQ semantics: divide-by-zero, NaN, and Infinity divisor → null.
    // For finite a and infinite b, PQ returns a (1 mod ∞ = 1).
    if b == 0.0 || a.is_nan() || b.is_nan() {
        return Ok(Value::Null);
    }
    if b.is_infinite() {
        return if a.is_infinite() { Ok(Value::Null) } else { Ok(Value::Number(a)) };
    }
    // Truncated mod: result has the sign of the dividend (PQ behavior).
    Ok(Value::Number(a - b * (a / b).trunc()))
}


fn integer_divide(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let a = match &args[0] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    let b = match &args[1] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    if b == 0.0 || a.is_nan() || b.is_nan() || a.is_infinite() {
        return Ok(Value::Null);
    }
    // PQ: finite ÷ infinite → 0 (the infinite divisor "absorbs" the dividend).
    if b.is_infinite() {
        return Ok(Value::Number(0.0));
    }
    // Truncate toward zero (sign of dividend).
    Ok(Value::Number((a / b).trunc()))
}


fn is_nan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Logical(n.is_nan())),
        other => Err(type_mismatch("number", other)),
    }
}


fn is_odd(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            if !n.is_finite() || n.fract() != 0.0 {
                return Err(MError::Other(format!(
                    "Number.IsOdd: argument must be an integer (got {n})"
                )));
            }
            Ok(Value::Logical((*n as i64) % 2 != 0))
        }
        other => Err(type_mismatch("number", other)),
    }
}


fn is_even(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            if !n.is_finite() || n.fract() != 0.0 {
                return Err(MError::Other(format!(
                    "Number.IsEven: argument must be an integer (got {n})"
                )));
            }
            Ok(Value::Logical((*n as i64) % 2 == 0))
        }
        other => Err(type_mismatch("number", other)),
    }
}


fn int_arg(v: &Value, ctx: &str) -> Result<i64, MError> {
    match v {
        Value::Number(n) if n.is_finite() && n.fract() == 0.0 => Ok(*n as i64),
        Value::Null => Err(MError::Other(format!("{ctx}: null argument"))),
        other => Err(MError::Other(format!(
            "{}: argument must be an integer (got {})", ctx, super::super::type_name(other)
        ))),
    }
}


fn bitwise_and(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number((int_arg(&args[0], "Number.BitwiseAnd")?
        & int_arg(&args[1], "Number.BitwiseAnd")?) as f64))
}

fn bitwise_or(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number((int_arg(&args[0], "Number.BitwiseOr")?
        | int_arg(&args[1], "Number.BitwiseOr")?) as f64))
}

fn bitwise_xor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number((int_arg(&args[0], "Number.BitwiseXor")?
        ^ int_arg(&args[1], "Number.BitwiseXor")?) as f64))
}

fn bitwise_not(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number((!int_arg(&args[0], "Number.BitwiseNot")?) as f64))
}

fn bitwise_shift_left(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let a = int_arg(&args[0], "Number.BitwiseShiftLeft")?;
    let n = int_arg(&args[1], "Number.BitwiseShiftLeft")?;
    Ok(Value::Number((a.wrapping_shl(n as u32)) as f64))
}

fn bitwise_shift_right(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let a = int_arg(&args[0], "Number.BitwiseShiftRight")?;
    let n = int_arg(&args[1], "Number.BitwiseShiftRight")?;
    Ok(Value::Number((a.wrapping_shr(n as u32)) as f64))
}


fn exp(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::exp) }

fn ln(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::ln) }

fn log10(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::log10) }


fn log(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let base = match args.get(1) {
        Some(Value::Number(b)) => *b,
        Some(Value::Null) | None => std::f64::consts::E,
        Some(other) => return Err(type_mismatch("number (base)", other)),
    };
    // PQ: undefined inputs → null. Non-positive operand, non-positive or
    // 1 base, or any NaN.
    if !n.is_finite() || !base.is_finite() || n.is_nan() || base.is_nan()
        || n <= 0.0 || base <= 0.0 || base == 1.0
    {
        return Ok(Value::Null);
    }
    Ok(Value::Number(n.log(base)))
}


fn factorial_f64(n: f64) -> Result<f64, MError> {
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 {
        return Err(MError::Other(format!(
            "Number.Factorial: argument must be a non-negative integer (got {n})"
        )));
    }
    let n = n as u64;
    if n > 170 {
        return Err(MError::Other("Number.Factorial: overflow (n > 170)".into()));
    }
    let mut acc = 1f64;
    for i in 2..=n {
        acc *= i as f64;
    }
    Ok(acc)
}


fn factorial(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    Ok(Value::Number(factorial_f64(n)?))
}


fn combinations(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let k = match &args[1] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    if k > n {
        return Err(MError::Other(
            "Number.Combinations: combinationSize must not exceed setSize".into(),
        ));
    }
    Ok(Value::Number(factorial_f64(n)? / (factorial_f64(k)? * factorial_f64(n - k)?)))
}


fn permutations(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let k = match &args[1] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    if k > n {
        return Err(MError::Other(
            "Number.Permutations: combinationSize must not exceed setSize".into(),
        ));
    }
    Ok(Value::Number(factorial_f64(n)? / factorial_f64(n - k)?))
}


fn unary_f64(args: &[Value], f: fn(f64) -> f64) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(f(*n))),
        other => Err(type_mismatch("number", other)),
    }
}


fn acos(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::acos) }

fn asin(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::asin) }

fn atan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::atan) }

fn cos(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::cos) }

fn cosh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::cosh) }

fn sin(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::sin) }

fn sinh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::sinh) }

fn tan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::tan) }

fn tanh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::tanh) }


fn atan2(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let x = match &args[1] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    Ok(Value::Number(y.atan2(x)))
}


fn apply_round_mode(args: &[Value], ctx: &str, mode: fn(f64) -> f64) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    let digits = match args.get(1) {
        Some(Value::Number(d)) if d.fract() == 0.0 => *d as i32,
        Some(Value::Null) | None => 0,
        Some(other) => {
            let _ = ctx;
            return Err(type_mismatch("integer (digits)", other));
        }
    };
    let scale = 10f64.powi(digits);
    Ok(Value::Number(mode(n * scale) / scale))
}


fn round_up(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundUp", f64::ceil)
}


fn round_down(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundDown", f64::floor)
}


fn round_toward_zero(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundTowardZero", f64::trunc)
}


fn round_away_from_zero(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundAwayFromZero", |x| {
        if x >= 0.0 { (x + 0.5).floor() } else { (x - 0.5).ceil() }
    })
}


fn random(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number(rand::random::<f64>()))
}


fn random_between(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bottom = match &args[0] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let top = match &args[1] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    if bottom.is_nan() || top.is_nan() || bottom > top {
        return Err(MError::Other(format!(
            "Number.RandomBetween: bottom ({bottom}) must be <= top ({top})"
        )));
    }
    Ok(Value::Number(bottom + rand::random::<f64>() * (top - bottom)))
}


fn from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(*n)),
        Value::Logical(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
        Value::Text(s) => s
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| MError::Other(format!("Number.From: cannot parse {s:?}"))),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

/// Int*.From — coerce to integer using banker's rounding (PQ semantics).
fn int_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match from(args, host)? {
        Value::Null => Ok(Value::Null),
        Value::Number(n) if n.is_finite() => Ok(Value::Number(round_half_to_even(n))),
        Value::Number(n) => Ok(Value::Number(n)),
        other => Ok(other),
    }
}

/// Percentage.From — text input strips a trailing "%" before numeric coercion.
/// Culture (2nd arg) accepted; de/fr swap `,` for `.` before parsing.
fn percentage_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    if let Value::Text(s) = &args[0] {
        let t = s.trim();
        if let Some(stripped) = t.strip_suffix('%') {
            let culture = match args.get(1) {
                Some(Value::Text(c)) => Some(c.to_ascii_lowercase()),
                _ => None,
            };
            let is_comma_decimal = matches!(culture.as_deref(), Some(c) if {
                c.starts_with("de") || c.starts_with("fr") || c.starts_with("es")
                    || c.starts_with("it") || c.starts_with("nl") || c.starts_with("pt")
            });
            let s_norm: String = if is_comma_decimal {
                stripped.trim().chars()
                    .filter(|c| *c != '.')
                    .map(|c| if c == ',' { '.' } else { c })
                    .collect()
            } else {
                stripped.trim().to_string()
            };
            let n: f64 = s_norm
                .parse()
                .map_err(|_| MError::Other(format!("Percentage.From: cannot parse {s:?}")))?;
            return Ok(Value::Number(n / 100.0));
        }
    }
    // Strip the optional culture arg before delegating to `from` (which is
    // 1-arg only).
    from(&args[..1], host)
}

/// Currency.From — rounds to 4 decimal places (Currency.Type precision).
fn currency_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match from(args, host)? {
        Value::Null => Ok(Value::Null),
        Value::Number(n) if n.is_finite() => {
            Ok(Value::Number(round_half_to_even(n * 10_000.0) / 10_000.0))
        }
        other => Ok(other),
    }
}


fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => s
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| MError::Other(format!("Number.FromText: cannot parse {s:?}"))),
        other => Err(type_mismatch("text", other)),
    }
}


fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            // Format string (arg 1): "G"/"g" (general) is equivalent to no
            // format and renders integer-valued floats without `.0`. Other
            // .NET format specs ("F2", "N", "P", "C", custom "0.00", etc.)
            // aren't implemented — return NotImplemented with the actual
            // format in the message so the caller knows which one to
            // expand support for.
            let fmt = match args.get(1) {
                None | Some(Value::Null) => None,
                Some(Value::Text(f)) => {
                    let t = f.trim();
                    if t.is_empty() || matches!(t, "G" | "g") { None } else { Some(f.clone()) }
                }
                Some(other) => return Err(type_mismatch("text (format)", other)),
            };
            // Culture arg (2) accepted; en-GB inferred for currency (£) when
            // present, else en-US shapes used.
            let culture = match args.get(2) {
                Some(Value::Text(c)) => Some(c.clone()),
                _ => None,
            };
            let s = match fmt {
                None => default_number_text(*n),
                Some(f) => format_number_dotnet(*n, &f, culture.as_deref())?,
            };
            Ok(Value::Text(s))
        }
        other => Err(type_mismatch("number", other)),
    }
}

/// Format `n` according to a .NET-style numeric format string.
/// Supported standard codes: F[N]/N[N]/P[N]/C[N]/D[N]/E[N] and the
/// uppercase/lowercase variants. Falls back to a basic custom-pattern
/// renderer for patterns containing `#`, `0`, `,`, `.`, `%`, `E+`.
fn format_number_dotnet(n: f64, fmt: &str, culture: Option<&str>) -> Result<String, MError> {
    let f = fmt.trim();
    if f.is_empty() {
        return Ok(default_number_text(n));
    }
    // Standard format codes: a single letter optionally followed by precision.
    let bytes = f.as_bytes();
    if bytes.len() >= 1 && bytes[0].is_ascii_alphabetic() {
        let code = bytes[0] as char;
        let prec_str: String = f[1..].chars().collect();
        let precision: Option<i32> = if prec_str.is_empty() {
            None
        } else {
            prec_str.parse::<i32>().ok()
        };
        // Only treat as a standard code when EVERYTHING after the leading
        // letter is digits — otherwise fall through to custom pattern.
        if precision.is_some() || prec_str.is_empty() {
            let result: Option<String> = match code {
                'F' | 'f' => Some(format_fixed(n, precision.unwrap_or(2))),
                'N' | 'n' => Some(format_number_grouped(n, precision.unwrap_or(2))),
                'P' | 'p' => Some(format_percent(n, precision.unwrap_or(2))),
                'C' | 'c' => {
                    let symbol = currency_symbol_for(culture);
                    Some(format!("{}{}", symbol, format_number_grouped(n, precision.unwrap_or(2))))
                }
                'D' | 'd' => Some(format_integer_padded(n, precision.unwrap_or(0))),
                'E' | 'e' => Some(format_exponent(n, precision.unwrap_or(6), code == 'E')),
                'R' | 'r' => Some(n.to_string()),
                'X' | 'x' => {
                    let i = n as i64;
                    let prec = precision.unwrap_or(0) as usize;
                    Some(if code == 'X' {
                        format!("{i:0prec$X}")
                    } else {
                        format!("{i:0prec$x}")
                    })
                }
                _ => None,
            };
            if let Some(s) = result {
                let s = apply_culture_decimal(s, culture);
                let s = if matches!(code, 'P' | 'p') {
                    apply_culture_percent_space(s, culture)
                } else { s };
                return Ok(s);
            }
        }
    }
    // Custom pattern fallback — handles `0.00`, `#,##0.00`, `0.00%`, `#.##E+0`.
    Ok(format_custom_pattern(n, f))
}

fn round_half_even(x: f64) -> f64 {
    let r = x.round();
    if (x - x.trunc()).abs() == 0.5 {
        let t = x.trunc();
        if (t as i64) % 2 == 0 { t } else { r }
    } else {
        r
    }
}

fn format_fixed(n: f64, prec: i32) -> String {
    if n.is_nan() { return "NaN".to_string(); }
    if n.is_infinite() {
        return if n > 0.0 { "∞".to_string() } else { "-∞".to_string() };
    }
    let p = prec.max(0) as usize;
    // .NET's "F" format uses MidpointRounding.AwayFromZero and rounds at
    // the requested precision via text-based round-half-up. Render with
    // extra digits, then truncate-with-rounding to avoid f64-scaling
    // imprecision (e.g. 1234567.123456 * 1e10 doesn't land on the integer
    // 12345671234560 due to IEEE rep).
    text_round_half_away(n, p)
}

/// Format `n` to exactly `prec` fractional digits, rounding half-away-from
/// zero. .NET's "F"/"N" formats convert f64 → Decimal first (taking the
/// shortest decimal that round-trips), then pad/round to prec digits — so
/// `1234567.123456` formatted to N10 is `1234567.1234560000`, not
/// `1234567.1234559999`. We mimic by using Rust's `to_string` (which is
/// also shortest-round-trip) as the source, then doing text-based rounding.
fn text_round_half_away(n: f64, prec: usize) -> String {
    // Use shortest round-trip representation as the "true" decimal.
    let short = format!("{n}");
    // Parse into sign / int_part / frac_part (no exponent expected for
    // typical use; for very large/small magnitudes Rust may emit exponent
    // form — fall back to {:.prec+2} rendering in that case).
    if short.contains('e') || short.contains('E') {
        // Magnitude outside f64::to_string's plain-notation range. Fall back
        // to high-digit rendering.
        let extra = prec + 2;
        let s = format!("{n:.*}", extra);
        return finalise_rounded(&s, prec);
    }
    finalise_rounded(&short, prec)
}

fn finalise_rounded(s: &str, prec: usize) -> String {
    let neg = s.starts_with('-');
    let body = if neg { &s[1..] } else { &s };
    let dot = body.find('.').unwrap_or(body.len());
    let int_part = &body[..dot];
    let frac_full = if dot < body.len() { &body[dot + 1..] } else { "" };
    // Truncate frac_full to prec+1 digits, round on the trailing digit.
    if frac_full.len() <= prec {
        // Already short enough — pad with zeros to reach prec.
        let mut out = String::new();
        if neg { out.push('-'); }
        out.push_str(int_part);
        if prec > 0 {
            out.push('.');
            out.push_str(frac_full);
            for _ in frac_full.len()..prec { out.push('0'); }
        }
        return out;
    }
    let kept = &frac_full[..prec];
    let next_digit = frac_full.as_bytes()[prec];
    let round_up = next_digit >= b'5';
    // Combine int_part + "." + kept, then add 1 in the last decimal place
    // if round_up. Carry across digits.
    let combined: String = format!("{int_part}{kept}");
    let combined_rounded = if round_up {
        carry_add_one(&combined)
    } else {
        combined
    };
    // Now combined_rounded has (int_part.len() + prec) digits, possibly +1
    // if the carry rolled over (e.g. 999...9 → 1000...0). Split back.
    let total = combined_rounded.len();
    let int_target_len = total.saturating_sub(prec);
    let (final_int, final_frac) = combined_rounded.split_at(int_target_len);
    let mut out = String::new();
    if neg && !is_all_zeros(&combined_rounded) { out.push('-'); }
    out.push_str(final_int);
    if prec > 0 {
        out.push('.');
        out.push_str(final_frac);
    }
    out
}

/// Increment a decimal-digit string by 1 with carry, e.g. "129" → "130",
/// "999" → "1000". Returns a new owned String.
fn carry_add_one(s: &str) -> String {
    let mut bytes: Vec<u8> = s.bytes().collect();
    let mut i = bytes.len();
    while i > 0 {
        i -= 1;
        if bytes[i] < b'9' {
            bytes[i] += 1;
            return String::from_utf8(bytes).unwrap();
        }
        bytes[i] = b'0';
    }
    // All digits were 9 — prepend a '1'.
    let mut out = String::with_capacity(bytes.len() + 1);
    out.push('1');
    out.push_str(std::str::from_utf8(&bytes).unwrap());
    out
}

fn is_all_zeros(s: &str) -> bool {
    s.bytes().all(|b| b == b'0')
}

fn format_number_grouped(n: f64, prec: i32) -> String {
    let fixed = format_fixed(n, prec);
    insert_thousands(&fixed)
}

fn insert_thousands(s: &str) -> String {
    // Insert `,` every 3 digits before the decimal point.
    let (sign, rest) = if let Some(r) = s.strip_prefix('-') { ("-", r) } else { ("", s) };
    let (int_part, frac_part) = match rest.find('.') {
        Some(i) => (&rest[..i], Some(&rest[i..])),
        None => (rest, None),
    };
    let mut out = String::new();
    let chars: Vec<char> = int_part.chars().collect();
    for (i, c) in chars.iter().enumerate() {
        let pos_from_end = chars.len() - i;
        out.push(*c);
        if pos_from_end > 1 && pos_from_end % 3 == 1 {
            out.push(',');
        }
    }
    if let Some(f) = frac_part {
        out.push_str(f);
    }
    format!("{sign}{out}")
}

fn format_percent(n: f64, prec: i32) -> String {
    // PQ: NaN/Inf get no "%" suffix.
    if n.is_nan() { return "NaN".to_string(); }
    if n.is_infinite() {
        return if n > 0.0 { "∞".to_string() } else { "-∞".to_string() };
    }
    let scaled = n * 100.0;
    let body = format_number_grouped(scaled, prec);
    format!("{body}%")
}

/// Insert a regular space between the number and the "%" percent sign
/// for locales that use one (de, fr, es, it, nl, pt). en-* / Invariant
/// uses no space. PQ uses a normal space (U+0020), not NBSP, even for
/// fr-FR — only the thousands separator there is NBSP.
fn apply_culture_percent_space(s: String, culture: Option<&str>) -> String {
    if !s.ends_with('%') { return s; }
    let lc = culture.map(str::to_ascii_lowercase);
    let needs_space = matches!(lc.as_deref(), Some(c) if {
        c.starts_with("de") || c.starts_with("fr") || c.starts_with("es")
            || c.starts_with("it") || c.starts_with("nl") || c.starts_with("pt")
    });
    if !needs_space { return s; }
    let without = &s[..s.len() - 1];
    format!("{without} %")
}

fn format_integer_padded(n: f64, width: i32) -> String {
    let i = n as i64;
    let neg = i < 0;
    let abs_str = i.unsigned_abs().to_string();
    let w = (width.max(0) as usize).saturating_sub(abs_str.len());
    let zeros: String = std::iter::repeat('0').take(w).collect();
    let body = format!("{zeros}{abs_str}");
    if neg { format!("-{body}") } else { body }
}

fn format_exponent(n: f64, prec: i32, upper: bool) -> String {
    let p = prec.max(0) as usize;
    let raw = format!("{n:.*e}", p);
    // Rust produces "1.23e4"; .NET produces "1.23E+004" (E with sign and
    // 3-digit exponent). Normalise.
    let (mantissa, exp) = match raw.find('e') {
        Some(i) => (&raw[..i], &raw[i + 1..]),
        None => (raw.as_str(), "0"),
    };
    let exp_n: i32 = exp.parse().unwrap_or(0);
    let e_char = if upper { 'E' } else { 'e' };
    let sign = if exp_n < 0 { '-' } else { '+' };
    format!("{mantissa}{e_char}{sign}{:03}", exp_n.abs())
}

/// Swap separators for comma-decimal locales. fr-FR uses NBSP ( ,
/// narrow no-break space) for thousands; de/es/it/nl/pt use `.`. All use
/// `,` for the decimal. Currency symbol stays as-is.
fn apply_culture_decimal(s: String, culture: Option<&str>) -> String {
    let lc = culture.map(str::to_ascii_lowercase);
    let is_comma_decimal = matches!(lc.as_deref(), Some(c) if {
        c.starts_with("de") || c.starts_with("fr") || c.starts_with("es")
            || c.starts_with("it") || c.starts_with("nl") || c.starts_with("pt")
    });
    if !is_comma_decimal { return s; }
    let thousands_sep: char = if matches!(lc.as_deref(), Some(c) if c.starts_with("fr")) {
        '\u{202F}' // narrow no-break space
    } else {
        '.'
    };
    // Use a placeholder for the original thousands `,` so the decimal `.`
    // swap doesn't collide with it. Then swap placeholder → thousands_sep.
    let placeholder = '\u{1}';
    let with_holder: String = s.chars().map(|c| if c == ',' { placeholder } else { c }).collect();
    let dec_swapped: String = with_holder.chars().map(|c| if c == '.' { ',' } else { c }).collect();
    dec_swapped.chars().map(|c| if c == placeholder { thousands_sep } else { c }).collect()
}

fn currency_symbol_for(culture: Option<&str>) -> &'static str {
    match culture.map(str::to_ascii_lowercase).as_deref() {
        Some(c) if c.starts_with("en-gb") => "£",
        Some(c) if c.starts_with("en-us") => "$",
        Some(c) if c.starts_with("de") || c.starts_with("fr") => "€",
        Some(c) if c.starts_with("ja") => "¥",
        _ => "£", // Corpus runs en-GB by default
    }
}

/// Render `n` using a custom .NET pattern. Supported tokens:
///   `0`     digit (zero-pad)
///   `#`     digit (no zero-pad)
///   `,`     thousands separator (between digits in integer part)
///   `.`     decimal point
///   `%`     multiply by 100, append `%`
///   `E+0`   exponent
fn format_custom_pattern(n: f64, pat: &str) -> String {
    let has_percent = pat.contains('%');
    let value = if has_percent { n * 100.0 } else { n };
    let has_exp = pat.to_ascii_uppercase().contains('E');
    if has_exp {
        // Find E+0 or E+00 etc.
        let upper_idx = pat.find(|c: char| c == 'E' || c == 'e');
        if let Some(idx) = upper_idx {
            let mantissa_pat = &pat[..idx];
            let exp_pat = &pat[idx..]; // includes E
            // Figure out exponent digit width.
            let exp_digits = exp_pat.chars().filter(|c| *c == '0').count().max(1);
            let raw = format!("{value:E}");
            let (m, e) = raw.split_once('E').unwrap_or((raw.as_str(), "0"));
            let e_n: i32 = e.parse().unwrap_or(0);
            let m_f: f64 = m.parse().unwrap_or(value);
            let m_rendered = render_digit_pattern(m_f, mantissa_pat);
            let e_char = if exp_pat.starts_with('E') { 'E' } else { 'e' };
            let sign = if e_n < 0 { '-' } else { '+' };
            return format!("{m_rendered}{e_char}{sign}{:0width$}", e_n.abs(), width = exp_digits);
        }
    }
    let body = render_digit_pattern(value, pat.trim_end_matches('%'));
    if has_percent { format!("{body}%") } else { body }
}

fn render_digit_pattern(n: f64, pat: &str) -> String {
    // Split pattern at the decimal point.
    let (int_pat, frac_pat) = match pat.find('.') {
        Some(i) => (&pat[..i], Some(&pat[i + 1..])),
        None => (pat, None),
    };
    let frac_digits = frac_pat.map(|p| p.chars().filter(|c| *c == '0' || *c == '#').count()).unwrap_or(0);
    let fixed = format_fixed(n, frac_digits as i32);
    let grouped = if int_pat.contains(',') {
        insert_thousands(&fixed)
    } else {
        fixed
    };
    // The pattern's int part may demand a minimum number of digits (`0`s).
    // For simplicity, leave `grouped` as-is when no zero-pad is needed.
    let min_int_digits = int_pat.chars().filter(|c| *c == '0').count();
    if min_int_digits == 0 {
        return grouped;
    }
    let (sign, rest) = if let Some(r) = grouped.strip_prefix('-') { ("-", r) } else { ("", grouped.as_str()) };
    let (int_part, frac_part) = match rest.find('.') {
        Some(i) => (&rest[..i], Some(&rest[i..])),
        None => (rest, None),
    };
    let bare_digit_count = int_part.chars().filter(char::is_ascii_digit).count();
    let pad = min_int_digits.saturating_sub(bare_digit_count);
    let zeros: String = std::iter::repeat('0').take(pad).collect();
    let mut out = format!("{sign}{zeros}{int_part}");
    if let Some(f) = frac_part { out.push_str(f); }
    out
}

fn default_number_text(n: f64) -> String {
    if n.is_nan() {
        return "NaN".to_string();
    }
    if n.is_infinite() {
        return if n > 0.0 { "∞".to_string() } else { "-∞".to_string() };
    }
    if n.fract() == 0.0 && n.abs() < 1e16 {
        format!("{}", n as i64)
    } else {
        n.to_string()
    }
}


fn abs(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(n.abs())),
        other => Err(type_mismatch("number", other)),
    }
}


fn sign(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(if *n > 0.0 {
            1.0
        } else if *n < 0.0 {
            -1.0
        } else {
            0.0
        })),
        other => Err(type_mismatch("number", other)),
    }
}


fn power(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let base = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let exp = match &args[1] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    // PQ: 0^0 is indeterminate → null. Math convention says 1; PQ disagrees.
    if base == 0.0 && exp == 0.0 {
        return Ok(Value::Null);
    }
    // PQ: NaN in either operand → null. Infinite exponent → null. Infinite
    // base with non-negative exponent → null (only the negative-exponent case
    // survives, giving 0 via IEEE).
    if base.is_nan() || exp.is_nan() || exp.is_infinite() {
        return Ok(Value::Null);
    }
    if base.is_infinite() && exp >= 0.0 {
        return Ok(Value::Null);
    }
    Ok(Value::Number(base.powf(exp)))
}


fn sqrt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(n.sqrt())),
        other => Err(type_mismatch("number", other)),
    }
}


fn round(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let digits = match args.get(1) {
        Some(Value::Number(d)) => *d as i32,
        Some(Value::Null) | None => 0,
        Some(other) => return Err(type_mismatch("number or null", other)),
    };
    // RoundingMode: 0 AwayFromZero, 1 Down, 2 ToEven (default), 3 TowardZero, 4 Up.
    let mode = match args.get(2) {
        Some(Value::Number(m)) => *m as i32,
        Some(Value::Null) | None => 2,
        Some(other) => return Err(type_mismatch("number or null", other)),
    };
    let factor = 10f64.powi(digits);
    let scaled = n * factor;
    let rounded = match mode {
        0 => scaled.round(),                    // AwayFromZero (Rust's f64::round)
        1 => scaled.floor(),                    // Down
        2 => round_half_to_even(scaled),        // ToEven (banker's)
        3 => scaled.trunc(),                    // TowardZero
        4 => scaled.ceil(),                     // Up
        _ => return Err(MError::Other(format!("Number.Round: unknown rounding mode {mode}"))),
    };
    Ok(Value::Number(rounded / factor))
}

fn round_half_to_even(x: f64) -> f64 {
    let r = x.round();
    // round() goes away from zero; correct the .5 case toward even.
    if (x - x.trunc()).abs() == 0.5 {
        let t = x.trunc();
        if (t as i64) % 2 == 0 { t } else { r }
    } else {
        r
    }
}

// --- Text.* ---

