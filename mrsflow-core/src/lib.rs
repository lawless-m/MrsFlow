//! mrsflow-core — pure, synchronous core for evaluating Power Query M.
//!
//! Layered as: lexer → parser → AST → evaluator → stdlib.
//! No IO; the CLI and WASM shells layer that on top.

pub mod lexer;
pub mod parser;
