//! Predicate-pushdown tests for `Table.SelectRows` on a `LazyParquet`
//! source. Each fixture writes a parquet file with multiple row groups
//! so row-group elimination has something to actually eliminate.

use std::fs::File;
use std::sync::Arc;

use arrow::array::{Float64Array, Int64Array, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;
use parquet::arrow::ArrowWriter;
use parquet::file::properties::WriterProperties;

use mrsflow_cli::CliIoHost;
use mrsflow_core::eval::{deep_force, evaluate, root_env, IoHost, Value};
use mrsflow_core::lexer::tokenize;
use mrsflow_core::parser::parse;

/// Build a 9-row fixture in three explicit batches → three row groups
/// when written with max_row_group_size=3. Layout:
///   id:  1 2 3 | 4 5 6 | 7 8 9
///   tag: a b c | d e f | g h i
///   val: 1 2 3 | 4 5 6 | 7 8 9 (as Float64)
fn write_fixture(path: &std::path::Path) {
    let schema = Arc::new(Schema::new(vec![
        Field::new("id", DataType::Int64, false),
        Field::new("tag", DataType::Utf8, false),
        Field::new("val", DataType::Float64, false),
    ]));

    let props = WriterProperties::builder()
        .set_max_row_group_size(3)
        .build();
    let file = File::create(path).unwrap();
    let mut writer = ArrowWriter::try_new(file, schema.clone(), Some(props)).unwrap();

    for chunk in 0..3 {
        let lo = (chunk * 3 + 1) as i64;
        let ids = Int64Array::from(vec![lo, lo + 1, lo + 2]);
        let tags = StringArray::from(vec![
            char_at(chunk * 3),
            char_at(chunk * 3 + 1),
            char_at(chunk * 3 + 2),
        ]);
        let vals = Float64Array::from(vec![lo as f64, (lo + 1) as f64, (lo + 2) as f64]);
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![Arc::new(ids), Arc::new(tags), Arc::new(vals)],
        )
        .unwrap();
        writer.write(&batch).unwrap();
    }
    writer.close().unwrap();
}

fn char_at(i: usize) -> String {
    ((b'a' + i as u8) as char).to_string()
}

fn run(src: &str, host: &dyn IoHost) -> Value {
    let toks = tokenize(src).unwrap();
    let ast = parse(&toks).unwrap();
    let env = root_env();
    let v = evaluate(&ast, &env, host).unwrap();
    deep_force(v, host).unwrap()
}

#[test]
fn pushdown_simple_int_filter() {
    let dir = tempdir();
    let p = dir.join("f.parquet");
    write_fixture(&p);
    let host = CliIoHost::new();
    let src = format!(
        r#"Table.RowCount(Table.SelectRows(Parquet.Document("{}"), each [id] > 5))"#,
        p.display()
    );
    match run(&src, &host) {
        Value::Number(n) => assert_eq!(n, 4.0), // ids 6,7,8,9
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn pushdown_filter_no_rows_match_eliminates_groups() {
    let dir = tempdir();
    let p = dir.join("f.parquet");
    write_fixture(&p);
    let host = CliIoHost::new();
    let src = format!(
        r#"Table.RowCount(Table.SelectRows(Parquet.Document("{}"), each [id] > 100))"#,
        p.display()
    );
    match run(&src, &host) {
        Value::Number(n) => assert_eq!(n, 0.0),
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn pushdown_text_eq_filter() {
    let dir = tempdir();
    let p = dir.join("f.parquet");
    write_fixture(&p);
    let host = CliIoHost::new();
    let src = format!(
        r#"Table.RowCount(Table.SelectRows(Parquet.Document("{}"), each [tag] = "e"))"#,
        p.display()
    );
    match run(&src, &host) {
        Value::Number(n) => assert_eq!(n, 1.0),
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn pushdown_two_filters_anded() {
    let dir = tempdir();
    let p = dir.join("f.parquet");
    write_fixture(&p);
    let host = CliIoHost::new();
    // ids 5..=7
    let src = format!(
        r#"Table.RowCount(Table.SelectRows(Parquet.Document("{}"), each [id] >= 5 and [id] <= 7))"#,
        p.display()
    );
    match run(&src, &host) {
        Value::Number(n) => assert_eq!(n, 3.0),
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn pushdown_chained_with_select_columns() {
    // Filter then drop a column — verify the filter still applies even
    // after SelectColumns narrows past the filter column.
    let dir = tempdir();
    let p = dir.join("f.parquet");
    write_fixture(&p);
    let host = CliIoHost::new();
    let src = format!(
        r#"
        let
            t = Parquet.Document("{}"),
            filtered = Table.SelectRows(t, each [id] > 5),
            narrowed = Table.SelectColumns(filtered, {{"tag"}})
        in
            Table.RowCount(narrowed)
        "#,
        p.display()
    );
    match run(&src, &host) {
        Value::Number(n) => assert_eq!(n, 4.0),
        other => panic!("expected number, got {other:?}"),
    }
}

#[test]
fn pushdown_non_foldable_predicate_falls_back() {
    // Text.Length isn't in the foldable subset — should hit the eager
    // filter path and still produce the right count.
    let dir = tempdir();
    let p = dir.join("f.parquet");
    write_fixture(&p);
    let host = CliIoHost::new();
    let src = format!(
        r#"Table.RowCount(Table.SelectRows(Parquet.Document("{}"), each Text.Length([tag]) = 1))"#,
        p.display()
    );
    match run(&src, &host) {
        // All 9 tags are length-1 single chars.
        Value::Number(n) => assert_eq!(n, 9.0),
        other => panic!("expected number, got {other:?}"),
    }
}

fn tempdir() -> std::path::PathBuf {
    let mut p = std::env::temp_dir();
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    p.push(format!("mrsflow-pushdown-{nanos}"));
    std::fs::create_dir_all(&p).unwrap();
    p
}
