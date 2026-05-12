//! End-to-end tests for multi-query CLI mode.
//!
//! Each input `.m` file's filename stem becomes a binding in a shared env;
//! `--out NAME` selects which bindings are written as Parquet. These tests
//! exercise the cross-reference path (q2 references q1) and the duplicate-
//! stem rejection.

use std::fs::{self, File};
use std::path::PathBuf;
use std::sync::Arc;

use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::arrow_reader::ParquetRecordBatchReaderBuilder;
use parquet::arrow::ArrowWriter;

use mrsflow_cli::{run_multi_query, CliIoHost, MultiQueryError};

fn fixture_batch() -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
    ]));
    let names = StringArray::from(vec!["alice", "bob", "carol"]);
    let scores = Float64Array::from(vec![1.0, 2.5, 3.5]);
    RecordBatch::try_new(schema, vec![Arc::new(names), Arc::new(scores)]).unwrap()
}

fn write_parquet_fixture(path: &std::path::Path) {
    let batch = fixture_batch();
    let file = File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}

fn read_parquet_columns(path: &std::path::Path) -> Vec<String> {
    let file = File::open(path).unwrap();
    let builder = ParquetRecordBatchReaderBuilder::try_new(file).unwrap();
    builder
        .schema()
        .fields()
        .iter()
        .map(|f| f.name().clone())
        .collect()
}

#[test]
fn multi_query_cross_reference_writes_both() {
    let tmp = tempdir();
    let fixture_path = tmp.join("data.parquet");
    write_parquet_fixture(&fixture_path);

    // q1 loads the parquet, q2 projects q1 down to just the "name" column.
    let q1_path = tmp.join("q1.m");
    let q2_path = tmp.join("q2.m");
    fs::write(
        &q1_path,
        format!(r#"Parquet.Document("{}")"#, fixture_path.display()),
    )
    .unwrap();
    fs::write(&q2_path, r#"Table.SelectColumns(q1, {"name"})"#).unwrap();

    let out_dir = tmp.join("out");
    let host = CliIoHost::new();
    let written = run_multi_query(
        &[q1_path, q2_path],
        &["q1".to_string(), "q2".to_string()],
        &out_dir,
        &host,
    )
    .expect("run_multi_query");

    assert_eq!(written.len(), 2);
    let q1_out = out_dir.join("q1.parquet");
    let q2_out = out_dir.join("q2.parquet");
    assert!(q1_out.exists(), "q1.parquet should exist");
    assert!(q2_out.exists(), "q2.parquet should exist");

    assert_eq!(read_parquet_columns(&q1_out), vec!["name", "score"]);
    assert_eq!(read_parquet_columns(&q2_out), vec!["name"]);
}

#[test]
fn multi_query_skips_unreferenced_inputs() {
    // q3 is in the input list but neither in --out nor referenced by q2.
    // It should never evaluate — even if its source would error.
    let tmp = tempdir();
    let fixture_path = tmp.join("data.parquet");
    write_parquet_fixture(&fixture_path);

    let q1_path = tmp.join("q1.m");
    let q2_path = tmp.join("q2.m");
    let q3_path = tmp.join("q3.m");
    fs::write(
        &q1_path,
        format!(r#"Parquet.Document("{}")"#, fixture_path.display()),
    )
    .unwrap();
    fs::write(&q2_path, "q1").unwrap();
    // q3 references a non-existent binding; evaluating it would error.
    fs::write(&q3_path, "nonexistent_binding").unwrap();

    let out_dir = tmp.join("out");
    let host = CliIoHost::new();
    let written = run_multi_query(
        &[q1_path, q2_path, q3_path],
        &["q2".to_string()],
        &out_dir,
        &host,
    )
    .expect("run_multi_query should succeed because q3 is never forced");

    assert_eq!(written.len(), 1);
    assert!(out_dir.join("q2.parquet").exists());
    assert!(!out_dir.join("q3.parquet").exists());
}

#[test]
fn multi_query_duplicate_stem_errors() {
    let tmp = tempdir();
    let a_dir = tmp.join("a");
    let b_dir = tmp.join("b");
    fs::create_dir_all(&a_dir).unwrap();
    fs::create_dir_all(&b_dir).unwrap();

    let q1_a = a_dir.join("q1.m");
    let q1_b = b_dir.join("q1.m");
    fs::write(&q1_a, "1").unwrap();
    fs::write(&q1_b, "2").unwrap();

    let host = CliIoHost::new();
    let result = run_multi_query(
        &[q1_a.clone(), q1_b.clone()],
        &["q1".to_string()],
        &tmp.join("out"),
        &host,
    );

    match result {
        Err(MultiQueryError::DuplicateStem { name, .. }) => assert_eq!(name, "q1"),
        other => panic!("expected DuplicateStem error, got {other:?}"),
    }
}

#[test]
fn multi_query_unknown_out_name_errors() {
    let tmp = tempdir();
    let q1_path = tmp.join("q1.m");
    fs::write(&q1_path, "1").unwrap();

    let host = CliIoHost::new();
    let result = run_multi_query(
        &[q1_path],
        &["q2".to_string()],
        &tmp.join("out"),
        &host,
    );

    match result {
        Err(MultiQueryError::UnknownOutName(name)) => assert_eq!(name, "q2"),
        other => panic!("expected UnknownOutName error, got {other:?}"),
    }
}

#[test]
fn multi_query_non_table_out_errors() {
    let tmp = tempdir();
    let q1_path = tmp.join("q1.m");
    fs::write(&q1_path, "42").unwrap(); // a number, not a table

    let host = CliIoHost::new();
    let result = run_multi_query(
        &[q1_path],
        &["q1".to_string()],
        &tmp.join("out"),
        &host,
    );

    match result {
        Err(MultiQueryError::NotATable { name, kind }) => {
            assert_eq!(name, "q1");
            assert_eq!(kind, "number");
        }
        other => panic!("expected NotATable error, got {other:?}"),
    }
}

fn tempdir() -> PathBuf {
    let mut p = std::env::temp_dir();
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("mrsflow-multi-test-{nanos}"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
