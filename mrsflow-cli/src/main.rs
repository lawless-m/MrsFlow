//! mrsflow CLI — evaluate M source file(s).
//!
//!   mrsflow <input.m> --sexpr
//!     Evaluate one file and dump the result in S-expression form (a debug
//!     serialisation, not user-facing — useful while developing a query).
//!
//!   mrsflow <q1.m> [<q2.m> ...] --out <name> [--out <name> ...] --out-dir <dir>
//!     Evaluate multiple files in a shared env (each file's filename stem
//!     becomes a binding name so queries can reference one another). Each
//!     `--out` name is forced and written as `<out-dir>/<name>.parquet`.
//!
//! `--sexpr` and `--out`/`--out-dir` are mutually exclusive. One of them
//! must be given.

use std::env;
use std::fs;
use std::path::PathBuf;
use std::process;

use mrsflow_cli::{run_multi_query, CliIoHost};
use mrsflow_core::eval::{deep_force, evaluate, root_env, value_to_sexpr};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;

fn usage_and_exit() -> ! {
    eprintln!(
        "usage: mrsflow <input.m> --sexpr\n\
         usage: mrsflow <input.m> [<input.m> ...] --out <name> [--out <name> ...] --out-dir <dir>"
    );
    process::exit(64);
}

#[derive(Default)]
struct CliArgs {
    inputs: Vec<String>,
    out_names: Vec<String>,
    out_dir: Option<String>,
    sexpr: bool,
}

fn parse_args(raw: Vec<String>) -> CliArgs {
    let mut a = CliArgs::default();
    let mut i = 0;
    while i < raw.len() {
        match raw[i].as_str() {
            "--out" => {
                i += 1;
                if i >= raw.len() {
                    usage_and_exit();
                }
                a.out_names.push(raw[i].clone());
            }
            "--out-dir" => {
                i += 1;
                if i >= raw.len() {
                    usage_and_exit();
                }
                if a.out_dir.is_some() {
                    eprintln!("ERROR: --out-dir given more than once");
                    process::exit(64);
                }
                a.out_dir = Some(raw[i].clone());
            }
            "--sexpr" => {
                a.sexpr = true;
            }
            other if other.starts_with('-') => {
                eprintln!("ERROR: unknown flag {other}");
                usage_and_exit();
            }
            _ => a.inputs.push(raw[i].clone()),
        }
        i += 1;
    }
    a
}

fn main() {
    let cli = parse_args(env::args().skip(1).collect());

    let want_outputs = !cli.out_names.is_empty() || cli.out_dir.is_some();

    if want_outputs && cli.sexpr {
        eprintln!("ERROR: --sexpr and --out/--out-dir are mutually exclusive");
        process::exit(64);
    }
    if !want_outputs && !cli.sexpr {
        eprintln!("ERROR: specify either --sexpr or --out NAME --out-dir DIR");
        usage_and_exit();
    }

    if want_outputs {
        let out_dir = cli.out_dir.unwrap_or_else(|| {
            eprintln!("ERROR: --out-dir is required when --out is given");
            process::exit(64);
        });
        if cli.out_names.is_empty() {
            eprintln!("ERROR: --out-dir given without any --out NAME");
            process::exit(64);
        }
        if cli.inputs.is_empty() {
            eprintln!("ERROR: no input .m files given");
            process::exit(64);
        }
        let inputs: Vec<PathBuf> = cli.inputs.into_iter().map(PathBuf::from).collect();
        let host = CliIoHost::new();
        match run_multi_query(&inputs, &cli.out_names, &PathBuf::from(&out_dir), &host) {
            Ok(written) => {
                for p in written {
                    println!("wrote {}", p.display());
                }
            }
            Err(e) => {
                eprintln!("{e}");
                process::exit(match e {
                    mrsflow_cli::MultiQueryError::Io(_) => 66,
                    mrsflow_cli::MultiQueryError::Lex(_) => 2,
                    mrsflow_cli::MultiQueryError::Parse(_) => 3,
                    mrsflow_cli::MultiQueryError::Eval(_) => 4,
                    mrsflow_cli::MultiQueryError::Write(_) => 5,
                    mrsflow_cli::MultiQueryError::NotATable { .. } => 6,
                    mrsflow_cli::MultiQueryError::DuplicateStem { .. }
                    | mrsflow_cli::MultiQueryError::UnknownOutName(_) => 64,
                });
            }
        }
        return;
    }

    if cli.inputs.len() != 1 {
        usage_and_exit();
    }
    let input = &cli.inputs[0];

    let src = fs::read_to_string(input).unwrap_or_else(|e| {
        eprintln!("read {input}: {e}");
        process::exit(66);
    });
    let toks = tokenize(&src).unwrap_or_else(|e| {
        eprintln!("LEX ERROR: {e:?}");
        process::exit(2);
    });
    let ast = parse(&toks).unwrap_or_else(|e| {
        eprintln!("PARSE ERROR: {e:?}");
        process::exit(3);
    });

    let env = root_env();
    let host = CliIoHost::new();
    let value = match evaluate(&ast, &env, &host) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("EVAL ERROR: {e:?}");
            process::exit(4);
        }
    };
    let value = match deep_force(value, &host) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("EVAL ERROR: {e:?}");
            process::exit(4);
        }
    };

    println!("{}", value_to_sexpr(&value));
}
