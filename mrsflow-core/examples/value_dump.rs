//! Read source from a file, lex+parse+evaluate, print the resulting Value
//! in canonical S-expression form.
//!
//! Used by `tools/grammar-fuzz/diff_eval.sh` to differential-test the Rust
//! evaluator against the Prolog companion. Output format must match
//! `print_value/1` in `tools/grammar-fuzz/evaluator.pl` exactly.
//!
//! Currently the evaluator is a stub returning `MError::NotImplemented` —
//! this binary will exit non-zero with an error on stderr until slice-1
//! lands. The scaffold exists so the harness wiring is in place.
//!
//! Usage: value_dump <path>

use mrsflow_core::eval::{deep_force, evaluate, root_env, NoIoHost, TypeRep, Value};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;
use std::env;
use std::fs;
use std::process;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: value_dump <path>");
        process::exit(64);
    });
    let src = fs::read_to_string(&path).unwrap_or_else(|e| {
        eprintln!("read {}: {}", path, e);
        process::exit(66);
    });
    let toks = tokenize(&src).unwrap_or_else(|e| {
        eprintln!("LEX ERROR: {:?}", e);
        process::exit(2);
    });
    let ast = parse(&toks).unwrap_or_else(|e| {
        eprintln!("PARSE ERROR: {:?}", e);
        process::exit(3);
    });
    let env = root_env();
    let host = NoIoHost;
    let value = match evaluate(&ast, &env, &host) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("EVAL ERROR: {:?}", e);
            process::exit(4);
        }
    };
    match deep_force(value, &host) {
        Ok(forced) => println!("{}", value_to_sexpr(&forced)),
        Err(e) => {
            eprintln!("EVAL ERROR: {:?}", e);
            process::exit(4);
        }
    }
}

fn value_to_sexpr(v: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, v);
    out
}

fn write_value(out: &mut String, v: &Value) {
    match v {
        Value::Null => out.push_str("(null)"),
        Value::Logical(true) => out.push_str("(bool true)"),
        Value::Logical(false) => out.push_str("(bool false)"),
        // {:?} formats floats with always-trailing fractional digit
        // (e.g. `42.0`, not `42`) which matches scryer's `~w` for floats.
        // Differential parity hinges on this.
        Value::Number(n) => out.push_str(&format!("(num {:?})", n)),
        Value::Text(s) => {
            out.push_str("(text ");
            write_quoted(out, s);
            out.push(')');
        }
        Value::Date(s) => {
            out.push_str("(date ");
            write_quoted(out, s);
            out.push(')');
        }
        Value::Datetime(s) => {
            out.push_str("(datetime ");
            write_quoted(out, s);
            out.push(')');
        }
        Value::Duration(s) => {
            out.push_str("(duration ");
            write_quoted(out, s);
            out.push(')');
        }
        Value::Binary(_) => out.push_str("(binary ...)"),
        Value::List(items) => {
            out.push_str("(list (");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write_value(out, item);
            }
            out.push_str("))");
        }
        Value::Record(record) => {
            out.push_str("(record (");
            for (i, (name, value)) in record.fields.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, name);
                out.push(' ');
                write_value(out, value);
                out.push(')');
            }
            out.push_str("))");
        }
        Value::Table(_) => out.push_str("(table ...)"),
        Value::Function(_) => out.push_str("(function ...)"),
        Value::Type(t) => {
            out.push_str("(type-value ");
            write_type(out, t);
            out.push(')');
        }
        Value::Thunk(_) => out.push_str("(thunk ...)"),
    }
}

fn write_type(out: &mut String, t: &TypeRep) {
    match t {
        TypeRep::Any => out.push_str("any"),
        TypeRep::AnyNonNull => out.push_str("anynonnull"),
        TypeRep::Null => out.push_str("null"),
        TypeRep::Logical => out.push_str("logical"),
        TypeRep::Number => out.push_str("number"),
        TypeRep::Text => out.push_str("text"),
        TypeRep::Date => out.push_str("date"),
        TypeRep::Datetime => out.push_str("datetime"),
        TypeRep::Duration => out.push_str("duration"),
        TypeRep::Binary => out.push_str("binary"),
        TypeRep::List => out.push_str("list"),
        TypeRep::Record => out.push_str("record"),
        TypeRep::Table => out.push_str("table"),
        TypeRep::Function => out.push_str("function"),
        TypeRep::Type => out.push_str("type"),
        TypeRep::Nullable(inner) => {
            out.push_str("(nullable ");
            write_type(out, inner);
            out.push(')');
        }
    }
}

fn write_quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
}
