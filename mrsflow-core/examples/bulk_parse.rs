//! Walk a directory of `.m` files, lex+parse each, tally OK / LEX / PARSE.
//!
//! Files extracted from corporate workbooks start with `shared <name> = `
//! (a section-member declaration); the parser only handles a top-level
//! expression, so we strip that prefix before parsing. Files starting with
//! `section ` are reported as SKIP (whole-section parsing is a separate
//! product).
//!
//! Usage: bulk_parse <dir>

use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;

fn strip_shared(src: &str) -> Option<&str> {
    // shared <bare_or_#""_name> = ...
    let s = src.trim_start();
    let after_shared = s.strip_prefix("shared")?;
    let after_shared = after_shared.trim_start();
    let (_, after_name) = if let Some(rest) = after_shared.strip_prefix("#\"") {
        let end = rest.find('"')?;
        ((), &rest[end + 1..])
    } else {
        let end = after_shared
            .find(|c: char| !c.is_alphanumeric() && c != '_')
            .unwrap_or(after_shared.len());
        ((), &after_shared[end..])
    };
    let after_eq = after_name.trim_start().strip_prefix('=')?;
    Some(after_eq.trim())
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    if let Ok(rd) = fs::read_dir(dir) {
        for entry in rd.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|s| s.to_str()) == Some("m") {
                out.push(p);
            }
        }
    }
}

fn main() {
    let dir = env::args().nth(1).expect("usage: bulk_parse <dir>");
    let mut files = Vec::new();
    walk(Path::new(&dir), &mut files);
    files.sort();
    eprintln!("found {} .m files", files.len());

    let mut ok = 0usize;
    let mut lex_err = 0usize;
    let mut parse_err = 0usize;
    let mut skip = 0usize;
    let mut fail_samples: Vec<(PathBuf, String, String)> = Vec::new();
    let mut all_fails: Vec<(PathBuf, String, String)> = Vec::new();

    for path in &files {
        let src = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(_) => {
                continue;
            }
        };
        let body = match strip_shared(&src) {
            Some(b) => b.trim_end_matches(';').trim().to_string(),
            None => {
                if src.trim_start().starts_with("section ") {
                    skip += 1;
                    continue;
                }
                src.trim().to_string()
            }
        };

        match tokenize(&body) {
            Err(e) => {
                lex_err += 1;
                let detail = format!("{e:?}");
                all_fails.push((path.clone(), "LEX".into(), detail.clone()));
                if fail_samples.len() < 30 {
                    fail_samples.push((path.clone(), "LEX".into(), detail));
                }
            }
            Ok(toks) => match parse(&toks) {
                Ok(_) => ok += 1,
                Err(e) => {
                    parse_err += 1;
                    let detail = format!("{e:?}");
                    all_fails.push((path.clone(), "PARSE".into(), detail.clone()));
                    if fail_samples.len() < 30 {
                        fail_samples.push((path.clone(), "PARSE".into(), detail));
                    }
                }
            },
        }
    }

    // dump all fails to /tmp for analysis
    let mut dump = String::new();
    for (p, k, e) in &all_fails {
        use std::fmt::Write;
        let _ = writeln!(dump, "{k}\t{e}\t{}", p.display());
    }
    let _ = fs::write("/tmp/rust_parse_fails.txt", &dump);

    println!("RESULTS: total={}  OK={}  LEX_ERR={}  PARSE_ERR={}  SKIP={}",
        files.len(), ok, lex_err, parse_err, skip);
    println!("\nFirst {} failures:", fail_samples.len());
    for (p, k, e) in &fail_samples {
        let trunc: String = e.chars().take(150).collect();
        println!("  [{k}] {}", p.display());
        println!("        {trunc}");
    }
}
