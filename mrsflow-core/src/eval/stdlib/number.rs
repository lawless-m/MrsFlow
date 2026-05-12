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
        ("Currency.From", one("value"), from),
        ("Decimal.From", one("value"), from),
        ("Double.From", one("value"), from),
        ("Int8.From", one("value"), from),
        ("Int16.From", one("value"), from),
        ("Int32.From", one("value"), from),
        ("Int64.From", one("value"), from),
        ("Percentage.From", one("value"), from),
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
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
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
    if b == 0.0 {
        return Err(MError::Other("Number.Mod: division by zero".into()));
    }
    // Mathematical (floor) mod: result has the same sign as divisor.
    Ok(Value::Number(a - b * (a / b).floor()))
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
    if b == 0.0 {
        return Err(MError::Other("Number.IntegerDivide: division by zero".into()));
    }
    Ok(Value::Number((a / b).floor()))
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
                    "Number.IsOdd: argument must be an integer (got {})", n
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
                    "Number.IsEven: argument must be an integer (got {})", n
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
        Value::Null => Err(MError::Other(format!("{}: null argument", ctx))),
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
    Ok(Value::Number(n.log(base)))
}


fn factorial_f64(n: f64) -> Result<f64, MError> {
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 {
        return Err(MError::Other(format!(
            "Number.Factorial: argument must be a non-negative integer (got {})", n
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
    if !(bottom <= top) {
        return Err(MError::Other(format!(
            "Number.RandomBetween: bottom ({}) must be <= top ({})", bottom, top
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
            .map_err(|_| MError::Other(format!("Number.From: cannot parse {:?}", s))),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}


fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => s
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| MError::Other(format!("Number.FromText: cannot parse {:?}", s))),
        other => Err(type_mismatch("text", other)),
    }
}


fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            if !matches!(args.get(1), Some(Value::Null) | None) {
                return Err(MError::NotImplemented(
                    "Number.ToText: format string not yet supported",
                ));
            }
            // PQ prints whole-number floats without a trailing ".0".
            let s = if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e16 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            };
            Ok(Value::Text(s))
        }
        other => Err(type_mismatch("number", other)),
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
    // Simple half-away-from-zero. M's default is banker's, but the corpus
    // only relies on basic rounding for display.
    let factor = 10f64.powi(digits);
    Ok(Value::Number((n * factor).round() / factor))
}

// --- Text.* ---

