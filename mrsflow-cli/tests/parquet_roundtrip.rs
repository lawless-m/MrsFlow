//! End-to-end Parquet smoke test for the CLI shell.
//!
//! Generates a small Parquet fixture in a tempdir, evaluates an M expression
//! that reads it via `Parquet.Document` and projects to column names, and
//! asserts the names match what we wrote. Then round-trips the table through
//! `parquet_write` and verifies the schema survives.

use std::fs::File;
use std::sync::Arc;

use arrow::array::{Float64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;

use mrsflow_cli::CliIoHost;
use mrsflow_core::eval::{deep_force, evaluate, root_env, IoHost, Value};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;

fn fixture_batch() -> RecordBatch {
    let schema = Arc::new(Schema::new(vec![
        Field::new("name", DataType::Utf8, false),
        Field::new("score", DataType::Float64, false),
    ]));
    let names = StringArray::from(vec!["alice", "bob", "carol"]);
    let scores = Float64Array::from(vec![1.0, 2.5, 3.5]);
    RecordBatch::try_new(schema, vec![Arc::new(names), Arc::new(scores)]).unwrap()
}

fn write_fixture(path: &std::path::Path) {
    let batch = fixture_batch();
    let file = File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, batch.schema(), None).unwrap();
    writer.write(&batch).unwrap();
    writer.close().unwrap();
}

fn run(src: &str, host: &dyn IoHost) -> Value {
    let toks = tokenize(src).unwrap();
    let ast = parse(&toks).unwrap();
    let env = root_env();
    let v = evaluate(&ast, &env, host).unwrap();
    deep_force(v, host).unwrap()
}

#[test]
fn parquet_document_reads_column_names() {
    let tmpdir = tempdir();
    let path = tmpdir.join("fixture.parquet");
    write_fixture(&path);
    let host = CliIoHost::new();
    let src = format!(
        r#"Table.ColumnNames(Parquet.Document("{}"))"#,
        path.display()
    );
    match run(&src, &host) {
        Value::List(xs) => {
            let names: Vec<String> = xs
                .into_iter()
                .map(|v| match v {
                    Value::Text(s) => s,
                    other => panic!("expected text, got {other:?}"),
                })
                .collect();
            assert_eq!(names, vec!["name".to_string(), "score".to_string()]);
        }
        other => panic!("expected list of column names, got {other:?}"),
    }
}

#[test]
fn parquet_roundtrip_preserves_schema() {
    let tmpdir = tempdir();
    let in_path = tmpdir.join("in.parquet");
    let out_path = tmpdir.join("out.parquet");
    write_fixture(&in_path);

    let host = CliIoHost::new();
    let table = run(
        &format!(r#"Parquet.Document("{}")"#, in_path.display()),
        &host,
    );
    host.parquet_write(out_path.to_str().unwrap(), &table)
        .unwrap();

    // Read back via the same path and check column names round-trip.
    let names = match run(
        &format!(
            r#"Table.ColumnNames(Parquet.Document("{}"))"#,
            out_path.display()
        ),
        &host,
    ) {
        Value::List(xs) => xs,
        other => panic!("expected list, got {other:?}"),
    };
    let names: Vec<String> = names
        .into_iter()
        .map(|v| match v {
            Value::Text(s) => s,
            other => panic!("expected text, got {other:?}"),
        })
        .collect();
    assert_eq!(names, vec!["name".to_string(), "score".to_string()]);
}

/// Bespoke tempdir helper — pulling in the `tempfile` crate just for this is
/// overkill. Each test gets a unique subdir in the cargo target tempdir.
fn tempdir() -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    // Cheap unique name: nanosecond-since-epoch + test-thread id-ish.
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("mrsflow-test-{nanos}"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
