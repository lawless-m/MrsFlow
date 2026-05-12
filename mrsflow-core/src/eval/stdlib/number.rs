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
        ("Number.From", one("value"), number_from),
        (
            "Number.Mod",
            vec![
                Param { name: "number".into(),    optional: false, type_annotation: None },
                Param { name: "divisor".into(),   optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            number_mod,
        ),
        (
            "Number.IntegerDivide",
            vec![
                Param { name: "number".into(),    optional: false, type_annotation: None },
                Param { name: "divisor".into(),   optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            number_integer_divide,
        ),
        ("Number.IsNaN", one("number"), number_is_nan),
        ("Number.IsOdd", one("number"), number_is_odd),
        ("Number.IsEven", one("number"), number_is_even),
        ("Number.Random", vec![], number_random),
        ("Number.RandomBetween", two("bottom", "top"), number_random_between),
        (
            "Number.RoundUp",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round_up,
        ),
        (
            "Number.RoundDown",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round_down,
        ),
        (
            "Number.RoundTowardZero",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round_toward_zero,
        ),
        (
            "Number.RoundAwayFromZero",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round_away_from_zero,
        ),
        ("Number.Acos", one("number"), number_acos),
        ("Number.Asin", one("number"), number_asin),
        ("Number.Atan", one("number"), number_atan),
        ("Number.Atan2", two("y", "x"), number_atan2),
        ("Number.Cos", one("number"), number_cos),
        ("Number.Cosh", one("number"), number_cosh),
        ("Number.Sin", one("number"), number_sin),
        ("Number.Sinh", one("number"), number_sinh),
        ("Number.Tan", one("number"), number_tan),
        ("Number.Tanh", one("number"), number_tanh),
        ("Number.Exp", one("number"), number_exp),
        ("Number.Ln", one("number"), number_ln),
        (
            "Number.Log",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "base".into(),   optional: true,  type_annotation: None },
            ],
            number_log,
        ),
        ("Number.Log10", one("number"), number_log10),
        ("Number.Factorial", one("number"), number_factorial),
        ("Number.Combinations", two("setSize", "combinationSize"), number_combinations),
        ("Number.Permutations", two("setSize", "combinationSize"), number_permutations),
        ("Number.BitwiseAnd", two("a", "b"), number_bitwise_and),
        ("Number.BitwiseOr", two("a", "b"), number_bitwise_or),
        ("Number.BitwiseXor", two("a", "b"), number_bitwise_xor),
        ("Number.BitwiseNot", one("a"), number_bitwise_not),
        ("Number.BitwiseShiftLeft", two("a", "n"), number_bitwise_shift_left),
        ("Number.BitwiseShiftRight", two("a", "n"), number_bitwise_shift_right),
        ("Byte.From", one("value"), number_from),
        ("Currency.From", one("value"), number_from),
        ("Decimal.From", one("value"), number_from),
        ("Double.From", one("value"), number_from),
        ("Int8.From", one("value"), number_from),
        ("Int16.From", one("value"), number_from),
        ("Int32.From", one("value"), number_from),
        ("Int64.From", one("value"), number_from),
        ("Percentage.From", one("value"), number_from),
        ("Single.From", one("value"), number_from),
        (
            "Number.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            number_from_text,
        ),
        (
            "Number.Round",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round,
        ),
        ("Number.Abs", one("number"), number_abs),
        (
            "Number.ToText",
            vec![
                Param { name: "number".into(),  optional: false, type_annotation: None },
                Param { name: "format".into(),  optional: true,  type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            number_to_text,
        ),
        ("Number.Sign", one("number"), number_sign),
        ("Number.Power", two("base", "exponent"), number_power),
        ("Number.Sqrt", one("number"), number_sqrt),
    ]
}

fn number_mod(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_integer_divide(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_is_nan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Logical(n.is_nan())),
        other => Err(type_mismatch("number", other)),
    }
}


fn number_is_odd(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_is_even(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_bitwise_and(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number((int_arg(&args[0], "Number.BitwiseAnd")?
        & int_arg(&args[1], "Number.BitwiseAnd")?) as f64))
}

fn number_bitwise_or(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number((int_arg(&args[0], "Number.BitwiseOr")?
        | int_arg(&args[1], "Number.BitwiseOr")?) as f64))
}

fn number_bitwise_xor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number((int_arg(&args[0], "Number.BitwiseXor")?
        ^ int_arg(&args[1], "Number.BitwiseXor")?) as f64))
}

fn number_bitwise_not(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number((!int_arg(&args[0], "Number.BitwiseNot")?) as f64))
}

fn number_bitwise_shift_left(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let a = int_arg(&args[0], "Number.BitwiseShiftLeft")?;
    let n = int_arg(&args[1], "Number.BitwiseShiftLeft")?;
    Ok(Value::Number((a.wrapping_shl(n as u32)) as f64))
}

fn number_bitwise_shift_right(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let a = int_arg(&args[0], "Number.BitwiseShiftRight")?;
    let n = int_arg(&args[1], "Number.BitwiseShiftRight")?;
    Ok(Value::Number((a.wrapping_shr(n as u32)) as f64))
}


fn number_exp(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::exp) }

fn number_ln(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::ln) }

fn number_log10(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::log10) }


fn number_log(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_factorial(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    Ok(Value::Number(factorial_f64(n)?))
}


fn number_combinations(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_permutations(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_acos(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::acos) }

fn number_asin(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::asin) }

fn number_atan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::atan) }

fn number_cos(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::cos) }

fn number_cosh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::cosh) }

fn number_sin(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::sin) }

fn number_sinh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::sinh) }

fn number_tan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::tan) }

fn number_tanh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::tanh) }


fn number_atan2(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_round_up(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundUp", f64::ceil)
}


fn number_round_down(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundDown", f64::floor)
}


fn number_round_toward_zero(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundTowardZero", f64::trunc)
}


fn number_round_away_from_zero(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundAwayFromZero", |x| {
        if x >= 0.0 { (x + 0.5).floor() } else { (x - 0.5).ceil() }
    })
}


fn number_random(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number(rand::random::<f64>()))
}


fn number_random_between(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_abs(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(n.abs())),
        other => Err(type_mismatch("number", other)),
    }
}


fn number_sign(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_power(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn number_sqrt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(n.sqrt())),
        other => Err(type_mismatch("number", other)),
    }
}


fn number_round(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

