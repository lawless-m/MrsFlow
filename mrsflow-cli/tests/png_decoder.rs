//! Integration test: drive the M-language PNG decoder
//! (`tools/png-decoder/m/Decode.m`) over PngSuite files, hash the
//! resulting RGBA8 buffer, and compare against an oracle computed by
//! the Rust `image` crate. Stage 1 scope: greyscale 8-bit non-interlaced
//! (with all five filter types — PngSuite's basn0g08.png uses Sub on
//! some rows so filter-0-only would never pass a real test file).
//!
//! Adding a fixture: drop the .png into tools/png-decoder/png-suite/
//! and add an entry to the slice below. The oracle hash is computed on
//! first run from the `image` crate's decode; mismatch between Rust and
//! the M decoder means one of them is wrong.

use std::path::{Path, PathBuf};

use mrsflow_cli::CliIoHost;
use mrsflow_core::eval::{deep_force, evaluate, root_env, IoHost, Value};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR points to mrsflow-cli/, parent is repo root.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("repo root")
        .to_path_buf()
}

fn decoder_path() -> PathBuf {
    repo_root().join("tools/png-decoder/m/Decode.m")
}

fn png_path(filename: &str) -> PathBuf {
    repo_root().join("tools/png-decoder/png-suite").join(filename)
}

/// Build an M expression that loads the decoder, applies it to the
/// given PNG file, and returns the decoded record. We embed paths as
/// text literals — the decoder reads via File.Contents.
fn driver_source(decoder: &Path, png: &Path) -> String {
    // Forward-slash paths work in M's File.Contents on Windows too.
    let decoder_s = decoder.display().to_string().replace('\\', "/");
    let png_s     = png.display().to_string().replace('\\', "/");
    format!(
        r#"let
    decoderSrc = Text.FromBinary(File.Contents("{decoder}"), TextEncoding.Utf8),
    PngDecode = Expression.Evaluate(decoderSrc, #shared),
    input = File.Contents("{png}"),
    r = PngDecode(input)
in
    r"#,
        decoder = decoder_s,
        png = png_s,
    )
}

fn run_decoder(decoder: &Path, png: &Path) -> Value {
    let host = CliIoHost::new();
    let src = driver_source(decoder, png);
    let toks = tokenize(&src).expect("lex");
    let ast = parse(&toks).expect("parse");
    let env = root_env();
    let v = evaluate(&ast, &env, &host).expect("evaluate");
    deep_force(v, &host).expect("deep_force")
}

fn extract_record(v: &Value) -> &mrsflow_core::eval::Record {
    match v {
        Value::Record(r) => r,
        other => panic!("expected record, got {other:?}"),
    }
}

fn field<'a>(r: &'a mrsflow_core::eval::Record, name: &str) -> &'a Value {
    r.fields
        .iter()
        .find(|(k, _)| k == name)
        .map(|(_, v)| v)
        .unwrap_or_else(|| panic!("missing field {name}"))
}

fn expect_bool(v: &Value) -> bool {
    match v {
        Value::Logical(b) => *b,
        other => panic!("expected logical, got {other:?}"),
    }
}

fn expect_number(v: &Value) -> f64 {
    match v {
        Value::Number(n) => *n,
        other => panic!("expected number, got {other:?}"),
    }
}

fn expect_text(v: &Value) -> &str {
    match v {
        Value::Text(s) => s,
        other => panic!("expected text, got {other:?}"),
    }
}

fn expect_binary(v: &Value) -> &[u8] {
    match v {
        Value::Binary(b) => b,
        other => panic!("expected binary, got {other:?}"),
    }
}

/// Decode the same PNG via the Rust `image` crate, return the canonical
/// row-major RGBA8 buffer (the oracle).
fn oracle_rgba8(png_path: &Path) -> Vec<u8> {
    let img = image::open(png_path).unwrap_or_else(|e| {
        panic!("image crate failed to decode {}: {e}", png_path.display())
    });
    img.to_rgba8().into_raw()
}

#[test]
fn basn0g08_greyscale_8bit_non_interlaced() {
    let decoder = decoder_path();
    let png = png_path("basn0g08.png");
    assert!(decoder.exists(), "decoder.m missing: {}", decoder.display());
    assert!(png.exists(), "test PNG missing: {}", png.display());

    let result = run_decoder(&decoder, &png);
    let rec = extract_record(&result);

    let success = expect_bool(field(rec, "Success"));
    if !success {
        let err = expect_text(field(rec, "Error"));
        panic!("PngDecode reported failure: {err}");
    }

    let width = expect_number(field(rec, "Width")) as u32;
    let height = expect_number(field(rec, "Height")) as u32;
    let buf = expect_binary(field(rec, "RGBA8"));

    assert_eq!(width, 32);
    assert_eq!(height, 32);
    assert_eq!(buf.len() as u32, width * height * 4);

    // Compare against the image-crate oracle byte-by-byte.
    let oracle = oracle_rgba8(&png);
    assert_eq!(
        buf, oracle.as_slice(),
        "M decoder output differs from image-crate oracle"
    );
}
