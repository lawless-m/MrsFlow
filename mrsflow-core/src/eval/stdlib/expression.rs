//! `Expression.*` stdlib bindings. Enables M-from-M evaluation by exposing
//! the lex+parse+evaluate pipeline as a builtin (`Expression.Evaluate`).

#![allow(unused_imports)]

use crate::lexer::tokenize;
use crate::parser::{parse, Param};

use super::super::env::{EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Value};
use super::common::{expect_text, one, type_mismatch};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Expression.Constant", one("value"), constant),
        ("Expression.Identifier", one("name"), identifier),
        (
            "Expression.Evaluate",
            vec![
                Param { name: "document".into(),    optional: false, type_annotation: None },
                Param { name: "environment".into(), optional: true,  type_annotation: None },
            ],
            evaluate,
        ),
    ]
}

fn constant(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Text(render_constant(&args[0])))
}

fn render_constant(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Logical(true) => "true".to_string(),
        Value::Logical(false) => "false".to_string(),
        Value::Number(n) => {
            if n.is_nan() {
                "#nan".to_string()
            } else if !n.is_finite() {
                if *n > 0.0 { "#infinity".to_string() } else { "-#infinity".to_string() }
            } else if n.fract() == 0.0 && n.abs() < 1e16 {
                format!("{}", *n as i64)
            } else {
                format!("{}", n)
            }
        }
        Value::Text(s) => format!("\"{}\"", s.replace('"', "\"\"")),
        Value::Date(d) => {
            use chrono::Datelike;
            format!("#date({}, {}, {})", d.year(), d.month(), d.day())
        }
        Value::Datetime(dt) => {
            use chrono::{Datelike, Timelike};
            format!(
                "#datetime({}, {}, {}, {}, {}, {})",
                dt.year(), dt.month(), dt.day(), dt.hour(), dt.minute(), dt.second()
            )
        }
        Value::Datetimezone(dt) => {
            use chrono::{Datelike, Timelike};
            let off = dt.offset().local_minus_utc();
            let hours = off / 3600;
            let minutes = (off.abs() / 60) % 60;
            format!(
                "#datetimezone({}, {}, {}, {}, {}, {}, {}, {})",
                dt.year(), dt.month(), dt.day(),
                dt.hour(), dt.minute(), dt.second(),
                hours, minutes
            )
        }
        Value::Time(t) => {
            use chrono::Timelike;
            format!("#time({}, {}, {})", t.hour(), t.minute(), t.second())
        }
        Value::Duration(d) => {
            let total_secs = d.num_seconds();
            let days = total_secs.div_euclid(86400);
            let rem = total_secs.rem_euclid(86400);
            let hours = rem / 3600;
            let minutes = (rem / 60) % 60;
            let seconds = rem % 60;
            format!("#duration({}, {}, {}, {})", days, hours, minutes, seconds)
        }
        Value::Binary(_) => "#binary({...})".to_string(),
        Value::List(xs) => {
            let parts: Vec<String> = xs.iter().map(render_constant).collect();
            format!("{{{}}}", parts.join(", "))
        }
        Value::Record(r) => {
            let parts: Vec<String> = r
                .fields
                .iter()
                .map(|(n, v)| format!("{} = {}", quote_identifier(n), render_constant(v)))
                .collect();
            format!("[{}]", parts.join(", "))
        }
        Value::Table(_) => "#table({}, {})".to_string(),
        Value::Function(_) => "function ...".to_string(),
        Value::Type(_) => "type ...".to_string(),
        Value::Thunk(_) => "thunk ...".to_string(),
        Value::WithMetadata { inner, .. } => render_constant(inner),
    }
}

fn identifier(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let name = expect_text(&args[0])?;
    Ok(Value::Text(quote_identifier(name)))
}

/// Produce the M-source form of an identifier: bare if it matches the M
/// identifier grammar, else `#"...quoted..."`.
fn quote_identifier(name: &str) -> String {
    if is_valid_m_identifier(name) {
        name.to_string()
    } else {
        let escaped = name.replace('"', "\"\"");
        format!("#\"{}\"", escaped)
    }
}

fn is_valid_m_identifier(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphabetic() || c == '_' => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.')
}

fn evaluate(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let document = expect_text(&args[0])?;
    let tokens = tokenize(document)
        .map_err(|e| MError::Other(format!("Expression.Evaluate: lex error: {:?}", e)))?;
    let ast = parse(&tokens)
        .map_err(|e| MError::Other(format!("Expression.Evaluate: parse error: {:?}", e)))?;
    // Build the env: stdlib root + any bindings from the supplied environment record.
    let mut env = super::root_env();
    if let Some(env_val) = args.get(1) {
        match env_val {
            Value::Null => {}
            Value::Record(r) => {
                for (name, raw) in &r.fields {
                    let forced = super::super::force(raw.clone(), &mut |e, en| {
                        super::super::evaluate(e, en, host)
                    })?;
                    env = env.extend(name.clone(), forced);
                }
            }
            other => return Err(type_mismatch("record (environment)", other)),
        }
    }
    super::super::evaluate(&ast, &env, host)
}
