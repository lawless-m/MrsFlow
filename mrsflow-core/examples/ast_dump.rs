//! Read source from a file, lex+parse, print the AST as canonical S-expression.
//!
//! Used by `tools/grammar-fuzz/diff_parser.sh` to differential-test the Rust
//! parser against the Prolog DCG. Output format must match `print_ast/1` in
//! `tools/grammar-fuzz/syntactic.pl` exactly.
//!
//! Usage: ast_dump <path>

use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;
use std::env;
use std::fs;
use std::process;

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        eprintln!("usage: ast_dump <path>");
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
    println!("{}", ast.to_sexpr());
}
