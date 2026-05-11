//! Read source from a file, lex+parse+evaluate, print the resulting Value
//! in canonical S-expression form.
//!
//! Used by `tools/grammar-fuzz/diff_eval.sh` to differential-test the Rust
//! evaluator against the Prolog companion. Output format must match
//! `print_value/1` in `tools/grammar-fuzz/evaluator.pl` exactly. The
//! formatter itself lives in `mrsflow_core::eval::sexpr` so the `mrsflow`
//! CLI binary uses the same code.
//!
//! Usage: value_dump <path>

use mrsflow_core::eval::{deep_force, evaluate, root_env, value_to_sexpr, NoIoHost};
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
