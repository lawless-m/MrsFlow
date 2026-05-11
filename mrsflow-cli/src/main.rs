//! mrsflow CLI — evaluate an M source file, optionally writing the resulting
//! table to a Parquet file.
//!
//! Usage:
//!   mrsflow <input.m>                 # print result as S-expression to stdout
//!   mrsflow <input.m> -o <output.pq>  # write Value::Table result to Parquet
//!
//! Non-table results with `-o` error; table results without `-o` print as
//! S-expression like any other value.

use std::env;
use std::fs;
use std::process;

use mrsflow_cli::CliIoHost;
use mrsflow_core::eval::{deep_force, evaluate, root_env, value_to_sexpr, IoHost, Value};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;

fn usage_and_exit() -> ! {
    eprintln!("usage: mrsflow <input.m> [-o <output.parquet>]");
    process::exit(64);
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let (input, output) = match args.as_slice() {
        [input] => (input.clone(), None),
        [input, flag, output] if flag == "-o" => (input.clone(), Some(output.clone())),
        _ => usage_and_exit(),
    };

    let src = fs::read_to_string(&input).unwrap_or_else(|e| {
        eprintln!("read {}: {}", input, e);
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
    let host = CliIoHost::new();
    let value = match evaluate(&ast, &env, &host) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("EVAL ERROR: {:?}", e);
            process::exit(4);
        }
    };
    let value = match deep_force(value, &host) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("EVAL ERROR: {:?}", e);
            process::exit(4);
        }
    };

    match output {
        Some(path) => match &value {
            Value::Table(_) => {
                if let Err(e) = host.parquet_write(&path, &value) {
                    eprintln!("WRITE ERROR: {:?}", e);
                    process::exit(5);
                }
            }
            _ => {
                eprintln!("ERROR: -o requires a table-valued result, got {:?}", kind(&value));
                process::exit(6);
            }
        },
        None => {
            println!("{}", value_to_sexpr(&value));
        }
    }
}

fn kind(v: &Value) -> &'static str {
    match v {
        Value::Null => "null",
        Value::Logical(_) => "logical",
        Value::Number(_) => "number",
        Value::Text(_) => "text",
        Value::Date(_) => "date",
        Value::Datetime(_) => "datetime",
        Value::Duration(_) => "duration",
        Value::Binary(_) => "binary",
        Value::List(_) => "list",
        Value::Record(_) => "record",
        Value::Table(_) => "table",
        Value::Function(_) => "function",
        Value::Type(_) => "type",
        Value::Thunk(_) => "thunk",
    }
}
