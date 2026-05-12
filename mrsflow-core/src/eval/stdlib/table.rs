//! `Table.*` stdlib bindings.

#![allow(unused_imports)]

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Date32Array, DurationMicrosecondArray, Float64Array,
    NullArray, StringArray, TimestampMicrosecondArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::parser::Param;

use super::super::env::{Env, EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Record, Table, Value};
use super::common::{
    expect_function, expect_int, expect_list, expect_list_of_lists, expect_table,
    expect_text, expect_text_list, int_n_arg, invoke_builtin_callback,
    invoke_callback_with_host, one, three, two, type_mismatch, values_equal_primitive,
};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("#table", two("columns", "rows"), table_constructor),
        ("Table.ColumnNames", one("table"), table_column_names),
        ("Table.RenameColumns", two("table", "renames"), table_rename_columns),
        ("Table.RemoveColumns", two("table", "names"), table_remove_columns),
        ("Table.SelectColumns", two("table", "names"), table_select_columns),
        ("Table.SelectRows", two("table", "predicate"), table_select_rows),
        (
            "Table.AddColumn",
            vec![
                Param { name: "table".into(),     optional: false, type_annotation: None },
                Param { name: "name".into(),      optional: false, type_annotation: None },
                Param { name: "transform".into(), optional: false, type_annotation: None },
                Param { name: "type".into(),      optional: true,  type_annotation: None },
            ],
            table_add_column,
        ),
        ("Table.FromRows", two("rows", "columns"), table_from_rows),
        ("Table.PromoteHeaders", one("table"), table_promote_headers),
        (
            "Table.TransformColumnTypes",
            two("table", "transforms"),
            table_transform_column_types,
        ),
        (
            "Table.TransformColumns",
            two("table", "transforms"),
            table_transform_columns,
        ),
        ("Table.Combine", one("tables"), table_combine),
        ("Table.Skip", two("table", "countOrCondition"), table_skip),
        (
            "Table.ExpandRecordColumn",
            vec![
                Param { name: "table".into(),          optional: false, type_annotation: None },
                Param { name: "column".into(),         optional: false, type_annotation: None },
                Param { name: "fieldNames".into(),     optional: false, type_annotation: None },
                Param { name: "newColumnNames".into(), optional: true,  type_annotation: None },
            ],
            table_expand_record_column,
        ),
        (
            "Table.ExpandListColumn",
            two("table", "column"),
            table_expand_list_column,
        ),
        (
            "Table.ExpandTableColumn",
            vec![
                Param { name: "table".into(),          optional: false, type_annotation: None },
                Param { name: "column".into(),         optional: false, type_annotation: None },
                Param { name: "columnNames".into(),    optional: false, type_annotation: None },
                Param { name: "newColumnNames".into(), optional: true,  type_annotation: None },
            ],
            table_expand_table_column,
        ),
        (
            "Table.Unpivot",
            vec![
                Param { name: "table".into(),           optional: false, type_annotation: None },
                Param { name: "pivotColumns".into(),    optional: false, type_annotation: None },
                Param { name: "attributeColumn".into(), optional: false, type_annotation: None },
                Param { name: "valueColumn".into(),     optional: false, type_annotation: None },
            ],
            table_unpivot,
        ),
        (
            "Table.UnpivotOtherColumns",
            vec![
                Param { name: "table".into(),           optional: false, type_annotation: None },
                Param { name: "pivotColumns".into(),    optional: false, type_annotation: None },
                Param { name: "attributeColumn".into(), optional: false, type_annotation: None },
                Param { name: "valueColumn".into(),     optional: false, type_annotation: None },
            ],
            table_unpivot_other_columns,
        ),
        (
            "Table.NestedJoin",
            vec![
                Param { name: "table1".into(),        optional: false, type_annotation: None },
                Param { name: "key1".into(),          optional: false, type_annotation: None },
                Param { name: "table2".into(),        optional: false, type_annotation: None },
                Param { name: "key2".into(),          optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
                Param { name: "joinKind".into(),      optional: true,  type_annotation: None },
            ],
            table_nested_join,
        ),
        (
            "Table.Pivot",
            vec![
                Param { name: "table".into(),               optional: false, type_annotation: None },
                Param { name: "pivotValues".into(),         optional: false, type_annotation: None },
                Param { name: "attributeColumn".into(),     optional: false, type_annotation: None },
                Param { name: "valueColumn".into(),         optional: false, type_annotation: None },
                Param { name: "aggregationFunction".into(), optional: true,  type_annotation: None },
            ],
            table_pivot,
        ),
        ("Table.ReorderColumns", two("table", "columnOrder"), table_reorder_columns),
        ("Table.Column", two("table", "columnName"), table_column),
        ("Table.IsEmpty", one("table"), table_is_empty),
        (
            "Table.Distinct",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_distinct,
        ),
        ("Table.FirstN", two("table", "countOrCondition"), table_first_n),
        ("Table.FromRecords", one("records"), table_from_records),
        ("Table.ToRecords", one("table"), table_to_records),
        (
            "Table.Join",
            vec![
                Param { name: "table1".into(),                optional: false, type_annotation: None },
                Param { name: "key1".into(),                  optional: false, type_annotation: None },
                Param { name: "table2".into(),                optional: false, type_annotation: None },
                Param { name: "key2".into(),                  optional: false, type_annotation: None },
                Param { name: "joinKind".into(),              optional: true,  type_annotation: None },
                Param { name: "joinAlgorithm".into(),         optional: true,  type_annotation: None },
                Param { name: "keyEqualityComparers".into(),  optional: true,  type_annotation: None },
            ],
            table_join,
        ),
        (
            "Table.AddIndexColumn",
            vec![
                Param { name: "table".into(),         optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
                Param { name: "initialValue".into(),  optional: true,  type_annotation: None },
                Param { name: "increment".into(),     optional: true,  type_annotation: None },
            ],
            table_add_index_column,
        ),
        ("Table.TransformRows", two("table", "transform"), table_transform_rows),
        ("Table.InsertRows", three("table", "offset", "rows"), table_insert_rows),
    ]
}

fn table_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let names = expect_text_list(&args[0], "#table: columns")?;
    let rows = expect_list_of_lists(&args[1], "#table: rows")?;
    for (i, row) in rows.iter().enumerate() {
        if row.len() != names.len() {
            return Err(MError::Other(format!(
                "#table: row {} has {} cells, expected {}",
                i,
                row.len(),
                names.len()
            )));
        }
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}


fn table_column_names(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names: Vec<Value> = table
        .column_names()
        .into_iter()
        .map(Value::Text)
        .collect();
    Ok(Value::List(names))
}


fn table_rename_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let renames = expect_list(&args[1])?;
    let mut pairs: Vec<(String, String)> = Vec::new();
    for r in renames {
        let inner = match r {
            Value::List(xs) => xs,
            other => {
                return Err(type_mismatch(
                    "list (each rename must be {old, new})",
                    other,
                ));
            }
        };
        if inner.len() != 2 {
            return Err(MError::Other(format!(
                "Table.RenameColumns: each rename must be a 2-element list, got {}",
                inner.len()
            )));
        }
        let old = expect_text(&inner[0])?.to_string();
        let new = expect_text(&inner[1])?.to_string();
        pairs.push((old, new));
    }
    let existing = table.column_names();
    for (old, _new) in &pairs {
        if !existing.contains(old) {
            return Err(MError::Other(format!(
                "Table.RenameColumns: column not found: {}",
                old
            )));
        }
    }
    let renamed: Vec<String> = existing
        .iter()
        .map(|n| {
            let mut name = n.clone();
            for (old, new) in &pairs {
                if &name == old {
                    name = new.clone();
                    break;
                }
            }
            name
        })
        .collect();
    match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => {
            let schema = batch.schema();
            let new_fields: Vec<Field> = schema
                .fields()
                .iter()
                .zip(renamed.iter())
                .map(|(f, n)| Field::new(n, f.data_type().clone(), f.is_nullable()))
                .collect();
            let new_schema = Arc::new(Schema::new(new_fields));
            let columns: Vec<ArrayRef> = batch.columns().to_vec();
            let new_batch = RecordBatch::try_new(new_schema, columns)
                .map_err(|e| MError::Other(format!("Table.RenameColumns: rebuild failed: {}", e)))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { rows, .. } => {
            Ok(Value::Table(Table::from_rows(renamed, rows.clone())))
        }
    }
}


fn table_remove_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = expect_text_list(&args[1], "Table.RemoveColumns: names")?;
    let existing = table.column_names();
    for n in &names {
        if !existing.contains(n) {
            return Err(MError::Other(format!(
                "Table.RemoveColumns: column not found: {}",
                n
            )));
        }
    }
    let keep_indices: Vec<usize> = (0..existing.len())
        .filter(|&i| !names.contains(&existing[i]))
        .collect();
    select_columns_by_index(table, &keep_indices, "Table.RemoveColumns")
}

// --- Table helpers ---

/// Project a table to the columns named by `keep_indices` (in order). Works
/// for both Arrow- and Rows-backed inputs; preserves the input backing.
/// Used by Table.RemoveColumns, Table.SelectColumns, Table.ReorderColumns.

fn select_columns_by_index(
    table: &Table,
    keep_indices: &[usize],
    ctx: &str,
) -> Result<Value, MError> {
    let existing = table.column_names();
    let new_names: Vec<String> = keep_indices.iter().map(|&i| existing[i].clone()).collect();
    match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => {
            let schema = batch.schema();
            let new_fields: Vec<Field> = keep_indices
                .iter()
                .map(|&i| schema.field(i).clone())
                .collect();
            let new_schema = Arc::new(Schema::new(new_fields));
            let new_columns: Vec<ArrayRef> =
                keep_indices.iter().map(|&i| batch.column(i).clone()).collect();
            let new_batch = RecordBatch::try_new(new_schema, new_columns)
                .map_err(|e| MError::Other(format!("{}: rebuild failed: {}", ctx, e)))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { rows, .. } => {
            let new_rows: Vec<Vec<Value>> = rows
                .iter()
                .map(|row| keep_indices.iter().map(|&i| row[i].clone()).collect())
                .collect();
            Ok(Value::Table(Table::from_rows(new_names, new_rows)))
        }
    }
}


fn values_to_table(column_names: &[String], rows: &[Vec<Value>]) -> Result<Table, MError> {
    let n_rows = rows.len();
    let n_cols = column_names.len();

    // Special case: schema with zero columns isn't constructible via the
    // standard RecordBatch path. Build an empty-schema Arrow batch with the
    // correct row count.
    if n_cols == 0 {
        let schema = Arc::new(Schema::empty());
        let options =
            arrow::record_batch::RecordBatchOptions::new().with_row_count(Some(n_rows));
        let batch = RecordBatch::try_new_with_options(schema, vec![], &options)
            .map_err(|e| MError::Other(format!("#table: empty-cols rebuild failed: {}", e)))?;
        return Ok(Table::from_arrow(batch));
    }

    // First pass: try Arrow encoding for each column. If any column
    // returns None (heterogeneous), give up the Arrow path and build a
    // Rows-backed Table from the row data unchanged.
    let mut fields: Vec<Field> = Vec::with_capacity(n_cols);
    let mut columns: Vec<ArrayRef> = Vec::with_capacity(n_cols);
    for col_idx in 0..n_cols {
        let cells: Vec<&Value> = rows.iter().map(|r| &r[col_idx]).collect();
        match infer_cells(&cells)? {
            Some((dtype, array)) => {
                let is_nullable = matches!(dtype, DataType::Null)
                    || cells.iter().any(|v| matches!(v, Value::Null));
                fields.push(Field::new(column_names[col_idx].clone(), dtype, is_nullable));
                columns.push(array);
            }
            None => {
                return Ok(Table::from_rows(column_names.to_vec(), rows.to_vec()));
            }
        }
    }
    let schema = Arc::new(Schema::new(fields));
    let batch = RecordBatch::try_new(schema, columns)
        .map_err(|e| MError::Other(format!("#table: build failed: {}", e)))?;
    Ok(Table::from_arrow(batch))
}

/// Try to infer an Arrow column from a slice of cells.
/// `Ok(Some(...))` — cells fit Arrow's uniform-column rule.
/// `Ok(None)` — cells need a Rows-backed fallback (compound values, mixed
/// primitive types, or Binary). Caller decides what to do with the signal.
/// `Err(...)` — reserved for genuine internal errors (none currently).

pub(crate) fn infer_cells(
    cells: &[&Value],
) -> Result<Option<(DataType, ArrayRef)>, MError> {
    let n_rows = cells.len();
    // Find first non-null cell to determine column kind.
    let mut kind: Option<&'static str> = None;
    for v in cells {
        match v {
            Value::Null => {}
            Value::Number(_) => {
                kind = Some("number");
                break;
            }
            Value::Text(_) => {
                kind = Some("text");
                break;
            }
            Value::Logical(_) => {
                kind = Some("logical");
                break;
            }
            Value::Date(_) => {
                kind = Some("date");
                break;
            }
            Value::Datetime(_) => {
                kind = Some("datetime");
                break;
            }
            Value::Duration(_) => {
                kind = Some("duration");
                break;
            }
            // Compound or Binary — needs Rows fallback at the table level.
            _ => return Ok(None),
        }
    }
    match kind {
        None => Ok(Some((
            DataType::Null,
            Arc::new(NullArray::new(n_rows)) as ArrayRef,
        ))),
        Some("number") => {
            let mut values: Vec<Option<f64>> = Vec::with_capacity(n_rows);
            for v in cells {
                match v {
                    Value::Null => values.push(None),
                    Value::Number(n) => values.push(Some(*n)),
                    _ => return Ok(None), // mixed → Rows fallback
                }
            }
            Ok(Some((DataType::Float64, Arc::new(Float64Array::from(values)))))
        }
        Some("text") => {
            let mut values: Vec<Option<String>> = Vec::with_capacity(n_rows);
            for v in cells {
                match v {
                    Value::Null => values.push(None),
                    Value::Text(s) => values.push(Some(s.clone())),
                    _ => return Ok(None),
                }
            }
            Ok(Some((DataType::Utf8, Arc::new(StringArray::from(values)))))
        }
        Some("logical") => {
            let mut values: Vec<Option<bool>> = Vec::with_capacity(n_rows);
            for v in cells {
                match v {
                    Value::Null => values.push(None),
                    Value::Logical(b) => values.push(Some(*b)),
                    _ => return Ok(None),
                }
            }
            Ok(Some((DataType::Boolean, Arc::new(BooleanArray::from(values)))))
        }
        Some("date") => {
            // Date32 stores days since 1970-01-01.
            let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let mut values: Vec<Option<i32>> = Vec::with_capacity(n_rows);
            for v in cells {
                match v {
                    Value::Null => values.push(None),
                    Value::Date(d) => {
                        values.push(Some(d.signed_duration_since(epoch).num_days() as i32))
                    }
                    _ => return Ok(None),
                }
            }
            Ok(Some((DataType::Date32, Arc::new(Date32Array::from(values)))))
        }
        Some("datetime") => {
            // Timestamp(Microsecond, None): i64 microseconds since unix epoch.
            let mut values: Vec<Option<i64>> = Vec::with_capacity(n_rows);
            for v in cells {
                match v {
                    Value::Null => values.push(None),
                    Value::Datetime(dt) => values.push(Some(dt.and_utc().timestamp_micros())),
                    _ => return Ok(None),
                }
            }
            Ok(Some((
                DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
                Arc::new(TimestampMicrosecondArray::from(values)),
            )))
        }
        Some("duration") => {
            // Duration(Microsecond): i64 microseconds.
            let mut values: Vec<Option<i64>> = Vec::with_capacity(n_rows);
            for v in cells {
                match v {
                    Value::Null => values.push(None),
                    Value::Duration(d) => match d.num_microseconds() {
                        Some(us) => values.push(Some(us)),
                        None => {
                            return Err(MError::Other(format!(
                                "duration overflows i64 microseconds: {:?}",
                                d
                            )));
                        }
                    },
                    _ => return Ok(None),
                }
            }
            Ok(Some((
                DataType::Duration(arrow::datatypes::TimeUnit::Microsecond),
                Arc::new(DurationMicrosecondArray::from(values)),
            )))
        }
        _ => unreachable!(),
    }
}

/// Materialise a table as row-major Value cells. Works for both backings:
/// Arrow variant decodes via cell_to_value; Rows variant clones. Used by
/// Table.* ops that need to land their result in the Rows representation.

pub(crate) fn table_to_rows(table: &Table) -> Result<(Vec<String>, Vec<Vec<Value>>), MError> {
    let names = table.column_names();
    let n_rows = table.num_rows();
    let n_cols = names.len();
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(n_rows);
    for r in 0..n_rows {
        let mut row = Vec::with_capacity(n_cols);
        for c in 0..n_cols {
            row.push(cell_to_value(table, c, r)?);
        }
        rows.push(row);
    }
    Ok((names, rows))
}

/// Convert a single cell of a table back to a Value. Dispatches on the
/// table's `TableRepr`: Arrow-backed reads via Array downcast (existing
/// path); Rows-backed just clones the stored cell value.

pub fn cell_to_value(table: &Table, col: usize, row: usize) -> Result<Value, MError> {
    let batch = match &table.repr {
        super::super::value::TableRepr::Arrow(b) => b,
        super::super::value::TableRepr::Rows { rows, .. } => {
            return Ok(rows[row][col].clone());
        }
    };
    let array = batch.column(col);
    if array.is_null(row) {
        return Ok(Value::Null);
    }
    match array.data_type() {
        DataType::Float64 => {
            let a = array
                .as_any()
                .downcast_ref::<Float64Array>()
                .expect("Float64");
            Ok(Value::Number(a.value(row)))
        }
        DataType::Utf8 => {
            let a = array.as_any().downcast_ref::<StringArray>().expect("Utf8");
            Ok(Value::Text(a.value(row).to_string()))
        }
        DataType::Boolean => {
            let a = array
                .as_any()
                .downcast_ref::<BooleanArray>()
                .expect("Boolean");
            Ok(Value::Logical(a.value(row)))
        }
        DataType::Null => Ok(Value::Null),
        DataType::Date32 => {
            let a = array
                .as_any()
                .downcast_ref::<Date32Array>()
                .expect("Date32");
            let days = a.value(row);
            let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
            let d = epoch
                .checked_add_signed(chrono::Duration::days(days as i64))
                .ok_or_else(|| MError::Other(format!("Date32 out of range: {} days", days)))?;
            Ok(Value::Date(d))
        }
        DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None) => {
            let a = array
                .as_any()
                .downcast_ref::<TimestampMicrosecondArray>()
                .expect("TimestampMicrosecond");
            let micros = a.value(row);
            let dt = chrono::DateTime::from_timestamp_micros(micros)
                .ok_or_else(|| MError::Other(format!("Timestamp out of range: {} us", micros)))?
                .naive_utc();
            Ok(Value::Datetime(dt))
        }
        DataType::Duration(arrow::datatypes::TimeUnit::Microsecond) => {
            let a = array
                .as_any()
                .downcast_ref::<DurationMicrosecondArray>()
                .expect("DurationMicrosecond");
            let micros = a.value(row);
            Ok(Value::Duration(chrono::Duration::microseconds(micros)))
        }
        other => Err(MError::NotImplemented(match other {
            DataType::Date64 | DataType::Timestamp(_, _) => {
                "non-microsecond timestamp decode (deferred)"
            }
            _ => "unsupported cell type",
        })),
    }
}

// --- chrono constructors (eval-7b) ---
//
// #date(y,m,d), #datetime(y,m,d,h,m,s), #duration(d,h,m,s). All operands
// must be whole-numbered f64s; non-integer or out-of-range values error.


fn table_select_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = expect_text_list(&args[1], "Table.SelectColumns: names")?;
    let existing = table.column_names();
    let mut indices: Vec<usize> = Vec::with_capacity(names.len());
    for n in &names {
        match existing.iter().position(|e| e == n) {
            Some(i) => indices.push(i),
            None => {
                return Err(MError::Other(format!(
                    "Table.SelectColumns: column not found: {}",
                    n
                )));
            }
        }
    }
    select_columns_by_index(table, &indices, "Table.SelectColumns")
}


fn table_select_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let predicate = expect_function(&args[1])?;
    let n_rows = table.num_rows();
    let mut keep: Vec<u32> = Vec::new();
    for row in 0..n_rows {
        let record = row_to_record(table, row)?;
        let result = invoke_callback_with_host(predicate, vec![record], host)?;
        match result {
            Value::Logical(true) => keep.push(row as u32),
            Value::Logical(false) => {}
            other => {
                return Err(MError::TypeMismatch {
                    expected: "logical (from row predicate)",
                    found: super::super::type_name(&other),
                });
            }
        }
    }
    match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => {
            let indices = arrow::array::UInt32Array::from(keep);
            let new_columns: Vec<ArrayRef> = batch
                .columns()
                .iter()
                .map(|c| {
                    arrow::compute::take(c.as_ref(), &indices, None).map_err(|e| {
                        MError::Other(format!("Table.SelectRows: take failed: {}", e))
                    })
                })
                .collect::<Result<_, _>>()?;
            let new_batch = RecordBatch::try_new(batch.schema(), new_columns)
                .map_err(|e| MError::Other(format!("Table.SelectRows: rebuild failed: {}", e)))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { columns, rows } => {
            let new_rows: Vec<Vec<Value>> =
                keep.into_iter().map(|i| rows[i as usize].clone()).collect();
            Ok(Value::Table(Table::from_rows(columns.clone(), new_rows)))
        }
    }
}


fn table_from_records(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let records = expect_list(&args[0])?;
    if records.is_empty() {
        return Ok(Value::Table(values_to_table(&[], &[])?));
    }
    // Take column names from the first record (insertion order).
    let first = match &records[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record (in list)", other)),
    };
    let names: Vec<String> = first.fields.iter().map(|(n, _)| n.clone()).collect();

    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(records.len());
    for rec_v in records {
        let rec = match rec_v {
            Value::Record(r) => r,
            other => return Err(type_mismatch("record (in list)", other)),
        };
        let mut row: Vec<Value> = Vec::with_capacity(names.len());
        for name in &names {
            let raw = rec
                .fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .unwrap_or(Value::Null);
            let forced = super::super::force(raw, &mut |e, env| super::super::evaluate(e, env, host))?;
            row.push(forced);
        }
        rows.push(row);
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}


fn table_to_records(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = table.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n);
    for row in 0..n {
        out.push(row_to_record(table, row)?);
    }
    Ok(Value::List(out))
}


fn table_distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.Distinct: equationCriteria not yet supported",
        ));
    }
    let (names, rows) = table_to_rows(table)?;
    let mut kept: Vec<Vec<Value>> = Vec::new();
    for row in rows {
        let mut dup = false;
        for k in &kept {
            let mut all_eq = true;
            for (a, b) in row.iter().zip(k.iter()) {
                if !values_equal_primitive(a, b)? {
                    all_eq = false;
                    break;
                }
            }
            if all_eq {
                dup = true;
                break;
            }
        }
        if !dup {
            kept.push(row);
        }
    }
    Ok(Value::Table(values_to_table(&names, &kept)?))
}


fn table_first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = match &args[1] {
        Value::Number(n) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(MError::Other(
                    "Table.FirstN: count must be a non-negative integer".into(),
                ));
            }
            *n as usize
        }
        Value::Function(_) => {
            return Err(MError::NotImplemented(
                "Table.FirstN: predicate (take-while) form not yet supported",
            ));
        }
        other => return Err(type_mismatch("number or function", other)),
    };
    let (names, rows) = table_to_rows(table)?;
    let kept: Vec<Vec<Value>> = rows.into_iter().take(n).collect();
    Ok(Value::Table(values_to_table(&names, &kept)?))
}


fn table_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let name = expect_text(&args[1])?;
    let col_idx = table
        .column_names()
        .iter()
        .position(|n| n == name)
        .ok_or_else(|| MError::Other(format!("Table.Column: column not found: {}", name)))?;
    let n = table.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n);
    for row in 0..n {
        out.push(cell_to_value(table, col_idx, row)?);
    }
    Ok(Value::List(out))
}


fn table_is_empty(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    Ok(Value::Logical(table.num_rows() == 0))
}


fn table_add_index_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let new_name = expect_text(&args[1])?.to_string();
    let initial = match args.get(2) {
        Some(Value::Number(n)) => *n,
        Some(Value::Null) | None => 0.0,
        Some(other) => return Err(type_mismatch("number", other)),
    };
    let increment = match args.get(3) {
        Some(Value::Number(n)) => *n,
        Some(Value::Null) | None => 1.0,
        Some(other) => return Err(type_mismatch("number", other)),
    };
    let (mut names, mut rows) = table_to_rows(table)?;
    if names.iter().any(|n| n == &new_name) {
        return Err(MError::Other(format!(
            "Table.AddIndexColumn: column already exists: {}",
            new_name
        )));
    }
    names.push(new_name);
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(Value::Number(initial + (i as f64) * increment));
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}


fn table_add_column(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let new_name = expect_text(&args[1])?.to_string();
    let transform = expect_function(&args[2])?;
    let n_rows = table.num_rows();
    let mut new_cells: Vec<Value> = Vec::with_capacity(n_rows);
    for row in 0..n_rows {
        let record = row_to_record(table, row)?;
        let v = invoke_callback_with_host(transform, vec![record], host)?;
        new_cells.push(v);
    }
    // Try to encode the new column as Arrow. Three result shapes:
    //   - Some + input Arrow + no type-any cast: Arrow result (fast path)
    //   - Some + input Rows: Rows result (the new column joins the row list)
    //   - None (heterogeneous result column): Rows result (decode input + append)
    let cell_refs: Vec<&Value> = new_cells.iter().collect();
    let inferred = infer_cells(&cell_refs)?;
    let target_type = args.get(3).cloned();

    if let (super::super::value::TableRepr::Arrow(batch), Some((inferred_dtype, inferred_array))) =
        (&table.repr, &inferred)
    {
        // Fast path: Arrow input + Arrow-encodable new column.
        let (dtype, new_array, nullable) = match &target_type {
            Some(Value::Type(t)) if !matches!(t, super::super::value::TypeRep::Any) => {
                let (target_dtype, target_nullable) = type_rep_to_datatype(t)?;
                let cast = arrow::compute::cast(inferred_array, &target_dtype).map_err(|e| {
                    MError::Other(format!(
                        "Table.AddColumn: cast {} to {:?} failed: {}",
                        new_name, target_dtype, e
                    ))
                })?;
                (target_dtype, cast, target_nullable)
            }
            Some(Value::Type(_)) | Some(Value::Null) | None => {
                let nullable = matches!(inferred_dtype, DataType::Null)
                    || new_cells.iter().any(|v| matches!(v, Value::Null));
                (inferred_dtype.clone(), inferred_array.clone(), nullable)
            }
            Some(other) => return Err(type_mismatch("type or null", other)),
        };
        let schema = batch.schema();
        let mut fields: Vec<Field> = schema.fields().iter().map(|f| (**f).clone()).collect();
        fields.push(Field::new(new_name, dtype, nullable));
        let new_schema = Arc::new(Schema::new(fields));
        let mut new_columns: Vec<ArrayRef> = batch.columns().to_vec();
        new_columns.push(new_array);
        let new_batch = RecordBatch::try_new(new_schema, new_columns)
            .map_err(|e| MError::Other(format!("Table.AddColumn: rebuild failed: {}", e)))?;
        return Ok(Value::Table(Table::from_arrow(new_batch)));
    }

    // Slow path: produce a Rows-backed result. Decode the input if needed,
    // then append the new column per row.
    let (mut names, mut rows) = table_to_rows(table)?;
    if rows.len() != new_cells.len() {
        return Err(MError::Other(
            "Table.AddColumn: row count mismatch (internal)".into(),
        ));
    }
    names.push(new_name);
    for (row, cell) in rows.iter_mut().zip(new_cells.into_iter()) {
        row.push(cell);
    }
    // If the caller supplied a typed cast and our new column happens to be
    // uniformly that type, the values_to_table normalisation will pick Arrow
    // for it once all columns fit. For mixed cases we stay Rows.
    Ok(Value::Table(values_to_table(&names, &rows)?))
}


fn table_from_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Same as #table but with arg order (rows, columns).
    let rows = expect_list_of_lists(&args[0], "Table.FromRows: rows")?;
    let names = expect_text_list(&args[1], "Table.FromRows: columns")?;
    for (i, row) in rows.iter().enumerate() {
        if row.len() != names.len() {
            return Err(MError::Other(format!(
                "Table.FromRows: row {} has {} cells, expected {}",
                i,
                row.len(),
                names.len()
            )));
        }
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}


fn table_promote_headers(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.num_rows() == 0 {
        return Err(MError::Other(
            "Table.PromoteHeaders: table has no header row".into(),
        ));
    }
    // Read row 0 as the new names; every cell must be text.
    let mut new_names: Vec<String> = Vec::with_capacity(table.num_columns());
    for col in 0..table.num_columns() {
        match cell_to_value(table, col, 0)? {
            Value::Text(s) => new_names.push(s),
            other => {
                return Err(MError::Other(format!(
                    "Table.PromoteHeaders: header cell in column {} is not text: {}",
                    col,
                    super::super::type_name(&other)
                )));
            }
        }
    }
    // Drop row 0 from every column, keeping the existing column types.
    // Users who want a different type after promotion call TransformColumnTypes.
    let n_remaining = table.num_rows() - 1;
    match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => {
            let new_columns: Vec<ArrayRef> =
                batch.columns().iter().map(|c| c.slice(1, n_remaining)).collect();
            let new_fields: Vec<Field> = batch
                .schema()
                .fields()
                .iter()
                .zip(new_names.iter())
                .map(|(f, n)| Field::new(n.clone(), f.data_type().clone(), f.is_nullable()))
                .collect();
            let new_schema = Arc::new(Schema::new(new_fields));
            let new_batch = RecordBatch::try_new(new_schema, new_columns).map_err(|e| {
                MError::Other(format!("Table.PromoteHeaders: rebuild failed: {}", e))
            })?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { rows, .. } => {
            let new_rows: Vec<Vec<Value>> = rows.iter().skip(1).cloned().collect();
            Ok(Value::Table(Table::from_rows(new_names, new_rows)))
        }
    }
}

/// Build a record Value from one row of a table — column name → cell.
/// Dispatches on `TableRepr`.

pub(crate) fn row_to_record(table: &Table, row: usize) -> Result<Value, MError> {
    let names = table.column_names();
    let mut fields: Vec<(String, Value)> = Vec::with_capacity(names.len());
    for (col, name) in names.into_iter().enumerate() {
        let value = cell_to_value(table, col, row)?;
        fields.push((name, value));
    }
    Ok(Value::Record(Record {
        fields,
        env: EnvNode::empty(),
    }))
}

/// Like `invoke_builtin_callback` but threads the real host through. Used
/// when a Table.* op invokes its callback in a context where the original
/// host should propagate (so an Odbc-using row predicate could in theory
/// work — though none of slice 7d's tests exercise that).

fn table_transform_column_types(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let transforms = expect_list(&args[1])?;
    // Auto-wrap single `{name, type}` pair to match Power Query leniency.
    let owned: Vec<Value>;
    let transforms: &[Value] = if is_single_col_type_pair(transforms) {
        owned = vec![Value::List(transforms.to_vec())];
        &owned
    } else {
        transforms
    };
    let pairs = parse_col_type_pairs(transforms)?;

    match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => {
            let schema = batch.schema();
            let mut new_fields: Vec<Field> =
                schema.fields().iter().map(|f| (**f).clone()).collect();
            let mut new_columns: Vec<ArrayRef> = batch.columns().to_vec();
            for (name, target) in &pairs {
                let idx = schema.index_of(name).map_err(|_| {
                    MError::Other(format!(
                        "Table.TransformColumnTypes: column not found: {}",
                        name
                    ))
                })?;
                let Some((target_dtype, target_nullable)) = target else {
                    continue; // type any → no cast
                };
                let cast = arrow::compute::cast(&new_columns[idx], target_dtype).map_err(|e| {
                    MError::Other(format!(
                        "Table.TransformColumnTypes: cast {} to {:?} failed: {}",
                        name, target_dtype, e
                    ))
                })?;
                new_columns[idx] = cast;
                new_fields[idx] = Field::new(name, target_dtype.clone(), *target_nullable);
            }
            let new_schema = Arc::new(Schema::new(new_fields));
            let new_batch = RecordBatch::try_new(new_schema, new_columns).map_err(|e| {
                MError::Other(format!("Table.TransformColumnTypes: rebuild failed: {}", e))
            })?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { columns, rows } => {
            // Rows-backed path: per-column type-cast via Arrow round-trip for
            // typed targets; type-any columns pass through unchanged. If a
            // typed column is heterogeneous (cells don't share an Arrow
            // dtype), this errors — matching PQ's "typed cast on mixed
            // column = error" behaviour. The escape hatch is `type any`.
            let mut new_rows = rows.clone();
            for (name, target) in &pairs {
                let idx = columns.iter().position(|c| c == name).ok_or_else(|| {
                    MError::Other(format!(
                        "Table.TransformColumnTypes: column not found: {}",
                        name
                    ))
                })?;
                let Some(_) = target else {
                    continue; // type any → pass through
                };
                // Reconstruct the column's Value cells, cast them, write back.
                let cells: Vec<Value> = new_rows.iter().map(|r| r[idx].clone()).collect();
                // Re-look up the TypeRep from the original transform list so we
                // can call cast_cells_to_type.
                let trep = find_typerep_for_name(transforms, name)?;
                let cast = cast_cells_to_type(
                    &cells,
                    &trep,
                    name,
                    "Table.TransformColumnTypes",
                )?;
                for (row, c) in new_rows.iter_mut().zip(cast.into_iter()) {
                    row[idx] = c;
                }
            }
            Ok(Value::Table(values_to_table(columns, &new_rows)?))
        }
    }
}

/// Helper: pull the TypeRep for `name` out of the original (un-parsed)
/// transforms list. Only used on the Rows-path of TransformColumnTypes
/// to recover a TypeRep we already validated.

fn find_typerep_for_name(
    transforms: &[Value],
    name: &str,
) -> Result<super::super::value::TypeRep, MError> {
    for t in transforms {
        if let Value::List(xs) = t {
            if xs.len() == 2 {
                if let (Value::Text(n), Value::Type(tr)) = (&xs[0], &xs[1]) {
                    if n == name {
                        return Ok(tr.clone());
                    }
                }
            }
        }
    }
    Err(MError::Other(format!(
        "Table.TransformColumnTypes: lost track of type for column {}",
        name
    )))
}


fn parse_col_type_pairs(
    transforms: &[Value],
) -> Result<Vec<(String, Option<(DataType, bool)>)>, MError> {
    let mut out = Vec::with_capacity(transforms.len());
    for t in transforms {
        let inner = match t {
            Value::List(xs) => xs,
            other => {
                return Err(type_mismatch(
                    "list (each transform must be {name, type})",
                    other,
                ));
            }
        };
        if inner.len() != 2 {
            return Err(MError::Other(format!(
                "Table.TransformColumnTypes: each transform must be a 2-element list, got {}",
                inner.len()
            )));
        }
        let name = expect_text(&inner[0])?.to_string();
        let type_value = match &inner[1] {
            Value::Type(t) => t.clone(),
            other => return Err(type_mismatch("type", other)),
        };
        // `type any` → None (no-cast). Anything else must be castable.
        let mapped = if matches!(type_value, super::super::value::TypeRep::Any) {
            None
        } else {
            Some(type_rep_to_datatype(&type_value)?)
        };
        out.push((name, mapped));
    }
    Ok(out)
}

/// Map a TypeRep to (DataType, nullable). Compound and non-primitive types
/// error — eval-7e supports the primitive set only.

fn type_rep_to_datatype(t: &super::super::value::TypeRep) -> Result<(DataType, bool), MError> {
    use super::super::value::TypeRep;
    match t {
        TypeRep::Null => Ok((DataType::Null, true)),
        TypeRep::Logical => Ok((DataType::Boolean, false)),
        TypeRep::Number => Ok((DataType::Float64, false)),
        TypeRep::Text => Ok((DataType::Utf8, false)),
        TypeRep::Date => Ok((DataType::Date32, false)),
        TypeRep::Datetime => Ok((
            DataType::Timestamp(arrow::datatypes::TimeUnit::Microsecond, None),
            false,
        )),
        TypeRep::Duration => Ok((
            DataType::Duration(arrow::datatypes::TimeUnit::Microsecond),
            false,
        )),
        TypeRep::Nullable(inner) => {
            let (dt, _) = type_rep_to_datatype(inner)?;
            Ok((dt, true))
        }
        TypeRep::Any | TypeRep::AnyNonNull | TypeRep::List | TypeRep::Record
        | TypeRep::Table | TypeRep::Function | TypeRep::Type | TypeRep::Binary
        | TypeRep::Time | TypeRep::Datetimezone => {
            Err(MError::Other(format!(
                "Table.TransformColumnTypes: type {:?} is not a castable primitive",
                t
            )))
        }
    }
}


fn table_transform_columns(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let transforms = expect_list(&args[1])?;
    // Real Power Query accepts both `{name, fn}` (single pair) and
    // `{{name, fn}, ...}` (list of pairs). Auto-wrap the single-pair form.
    let owned: Vec<Value>;
    let transforms: &[Value] = if is_single_col_fn_pair(transforms) {
        owned = vec![Value::List(transforms.to_vec())];
        &owned
    } else {
        transforms
    };
    let pairs = parse_col_fn_pairs(transforms)?;

    // Row-major fallback: works for both Arrow- and Rows-backed inputs.
    // Each transform runs cell-by-cell; the result lands in values_to_table
    // which picks Arrow if the resulting columns are all uniform-typed, or
    // Rows if any column ends up heterogeneous after the transform.
    let (names, mut rows) = table_to_rows(table)?;
    let n_rows = rows.len();

    for (name, closure, type_opt) in &pairs {
        let idx = names.iter().position(|n| n == name).ok_or_else(|| {
            MError::Other(format!("Table.TransformColumns: column not found: {}", name))
        })?;
        let mut new_cells: Vec<Value> = Vec::with_capacity(n_rows);
        for row in &rows {
            let cell = row[idx].clone();
            let v = invoke_callback_with_host(closure, vec![cell], host)?;
            new_cells.push(v);
        }
        // Optional 3rd transform element: target type for the new column.
        // For `type any` or no spec, the cells pass through unchanged. For a
        // specific type, cast via Arrow's cast (errors if a cell doesn't
        // fit — matches PQ's typed-cast semantics).
        let final_cells: Vec<Value> = match type_opt {
            Some(t) if !matches!(t, super::super::value::TypeRep::Any) => {
                cast_cells_to_type(&new_cells, t, name, "Table.TransformColumns")?
            }
            _ => new_cells,
        };
        for (row, cell) in rows.iter_mut().zip(final_cells.into_iter()) {
            row[idx] = cell;
        }
    }

    Ok(Value::Table(values_to_table(&names, &rows)?))
}

/// Cast a column of Values to a target M type by round-tripping through
/// Arrow's `cast`. Errors when the column is heterogeneous (no uniform
/// Arrow dtype) or when the cast itself fails (cells don't fit the type).

fn cast_cells_to_type(
    cells: &[Value],
    t: &super::super::value::TypeRep,
    col_name: &str,
    ctx: &str,
) -> Result<Vec<Value>, MError> {
    let (target_dtype, target_nullable) = type_rep_to_datatype(t)?;
    let cell_refs: Vec<&Value> = cells.iter().collect();
    let (_, inferred_array) = infer_cells(&cell_refs)?.ok_or_else(|| {
        MError::Other(format!(
            "{}: cast {} to {:?} failed: column has heterogeneous cells",
            ctx, col_name, target_dtype
        ))
    })?;
    let cast = arrow::compute::cast(&inferred_array, &target_dtype).map_err(|e| {
        MError::Other(format!(
            "{}: cast {} to {:?} failed: {}",
            ctx, col_name, target_dtype, e
        ))
    })?;
    // Decode the cast result back to Values via a temporary single-column table.
    let field = Field::new(col_name, target_dtype, target_nullable);
    let temp_batch = RecordBatch::try_new(Arc::new(Schema::new(vec![field])), vec![cast])
        .map_err(|e| MError::Other(format!("{}: temp batch failed: {}", ctx, e)))?;
    let temp_table = Table::from_arrow(temp_batch);
    let mut decoded = Vec::with_capacity(cells.len());
    for r in 0..cells.len() {
        decoded.push(cell_to_value(&temp_table, 0, r)?);
    }
    Ok(decoded)
}


fn is_single_col_fn_pair(xs: &[Value]) -> bool {
    // Either `{name, fn}` or `{name, fn, type}` as a single transform.
    let head_ok = !xs.is_empty()
        && matches!(xs.first(), Some(Value::Text(_)))
        && matches!(xs.get(1), Some(Value::Function(_)));
    match xs.len() {
        2 => head_ok,
        3 => head_ok && matches!(xs[2], Value::Type(_) | Value::Null),
        _ => false,
    }
}


fn is_single_col_type_pair(xs: &[Value]) -> bool {
    xs.len() == 2 && matches!(xs[0], Value::Text(_)) && matches!(xs[1], Value::Type(_))
}


fn parse_col_fn_pairs<'a>(
    transforms: &'a [Value],
) -> Result<Vec<(String, &'a Closure, Option<super::super::value::TypeRep>)>, MError> {
    let mut out = Vec::with_capacity(transforms.len());
    for t in transforms {
        let inner = match t {
            Value::List(xs) => xs,
            other => {
                return Err(type_mismatch(
                    "list (each transform must be {name, function} or {name, function, type})",
                    other,
                ));
            }
        };
        if inner.len() != 2 && inner.len() != 3 {
            return Err(MError::Other(format!(
                "Table.TransformColumns: each transform must be 2 or 3 elements, got {}",
                inner.len()
            )));
        }
        let name = expect_text(&inner[0])?.to_string();
        let closure = match &inner[1] {
            Value::Function(c) => c,
            other => return Err(type_mismatch("function", other)),
        };
        let type_opt = if inner.len() == 3 {
            match &inner[2] {
                Value::Type(t) => Some(t.clone()),
                Value::Null => None,
                other => return Err(type_mismatch("type or null", other)),
            }
        } else {
            None
        };
        out.push((name, closure, type_opt));
    }
    Ok(out)
}


fn table_skip(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Value::Function(_) => {
            return Err(MError::NotImplemented(
                "Table.Skip: predicate (skip-while) form not yet supported",
            ));
        }
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let n_rows = table.num_rows();
    let skip = count.min(n_rows);
    match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => {
            let remaining = n_rows - skip;
            let new_columns: Vec<ArrayRef> =
                batch.columns().iter().map(|c| c.slice(skip, remaining)).collect();
            let new_batch = RecordBatch::try_new(batch.schema(), new_columns)
                .map_err(|e| MError::Other(format!("Table.Skip: rebuild failed: {}", e)))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { columns, rows } => {
            let new_rows: Vec<Vec<Value>> = rows.iter().skip(skip).cloned().collect();
            Ok(Value::Table(Table::from_rows(columns.clone(), new_rows)))
        }
    }
}


fn table_reorder_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let order = expect_text_list(&args[1], "Table.ReorderColumns: columnOrder")?;
    let existing = table.column_names();

    let mut new_indices: Vec<usize> = Vec::with_capacity(existing.len());
    let mut used = vec![false; existing.len()];

    // First: the explicitly named columns in the requested order.
    for name in &order {
        let idx = existing.iter().position(|e| e == name).ok_or_else(|| {
            MError::Other(format!(
                "Table.ReorderColumns: column not found: {}",
                name
            ))
        })?;
        new_indices.push(idx);
        used[idx] = true;
    }
    // Then: any unspecified columns, in original order.
    for (idx, used_flag) in used.iter().enumerate() {
        if !used_flag {
            new_indices.push(idx);
        }
    }

    select_columns_by_index(table, &new_indices, "Table.ReorderColumns")
}


fn table_expand_record_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let column = expect_text(&args[1])?.to_string();
    let field_names = expect_text_list(&args[2], "Table.ExpandRecordColumn: fieldNames")?;
    let new_column_names = match args.get(3) {
        Some(Value::Null) | None => field_names.clone(),
        Some(other) => expect_text_list(other, "Table.ExpandRecordColumn: newColumnNames")?,
    };
    if new_column_names.len() != field_names.len() {
        return Err(MError::Other(format!(
            "Table.ExpandRecordColumn: newColumnNames has {} items, expected {}",
            new_column_names.len(),
            field_names.len()
        )));
    }

    let (existing, rows) = table_to_rows(table)?;
    let col_idx = existing.iter().position(|n| n == &column).ok_or_else(|| {
        MError::Other(format!(
            "Table.ExpandRecordColumn: column not found: {}",
            column
        ))
    })?;

    // Build new column-name list: replace `column` at col_idx with new_column_names.
    let mut out_names: Vec<String> = Vec::with_capacity(existing.len() + field_names.len() - 1);
    out_names.extend_from_slice(&existing[..col_idx]);
    out_names.extend_from_slice(&new_column_names);
    out_names.extend_from_slice(&existing[col_idx + 1..]);

    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for row in &rows {
        let mut new_row: Vec<Value> = Vec::with_capacity(out_names.len());
        new_row.extend_from_slice(&row[..col_idx]);
        match &row[col_idx] {
            Value::Record(rec) => {
                for fname in &field_names {
                    let v = rec
                        .fields
                        .iter()
                        .find(|(n, _)| n == fname)
                        .map(|(_, v)| v.clone())
                        .unwrap_or(Value::Null);
                    new_row.push(v);
                }
            }
            Value::Null => {
                for _ in &field_names {
                    new_row.push(Value::Null);
                }
            }
            other => {
                return Err(MError::Other(format!(
                    "Table.ExpandRecordColumn: cell at column {} is not a record (got {})",
                    column,
                    super::super::type_name(other)
                )));
            }
        }
        new_row.extend_from_slice(&row[col_idx + 1..]);
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}


fn table_expand_list_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let column = expect_text(&args[1])?.to_string();
    let (names, rows) = table_to_rows(table)?;
    let col_idx = names.iter().position(|n| n == &column).ok_or_else(|| {
        MError::Other(format!(
            "Table.ExpandListColumn: column not found: {}",
            column
        ))
    })?;

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for row in &rows {
        match &row[col_idx] {
            Value::List(items) => {
                // One output row per list item; empty list drops the input row.
                for item in items {
                    let mut new_row = row.clone();
                    new_row[col_idx] = item.clone();
                    out_rows.push(new_row);
                }
            }
            Value::Null => {
                // Null cell → emit a single row with null in the target column.
                out_rows.push(row.clone());
            }
            other => {
                return Err(MError::Other(format!(
                    "Table.ExpandListColumn: cell at column {} is not a list (got {})",
                    column,
                    super::super::type_name(other)
                )));
            }
        }
    }
    Ok(Value::Table(values_to_table(&names, &out_rows)?))
}


fn table_expand_table_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let column = expect_text(&args[1])?.to_string();
    let column_names = expect_text_list(&args[2], "Table.ExpandTableColumn: columnNames")?;
    let new_column_names = match args.get(3) {
        Some(Value::Null) | None => column_names.clone(),
        Some(other) => expect_text_list(other, "Table.ExpandTableColumn: newColumnNames")?,
    };
    if new_column_names.len() != column_names.len() {
        return Err(MError::Other(format!(
            "Table.ExpandTableColumn: newColumnNames has {} items, expected {}",
            new_column_names.len(),
            column_names.len()
        )));
    }

    let (outer_names, outer_rows) = table_to_rows(table)?;
    let col_idx = outer_names.iter().position(|n| n == &column).ok_or_else(|| {
        MError::Other(format!(
            "Table.ExpandTableColumn: column not found: {}",
            column
        ))
    })?;

    // Output column order: outer columns up to col_idx, then lifted columns,
    // then outer columns after col_idx (the target column is removed).
    let mut out_names: Vec<String> = Vec::with_capacity(outer_names.len() + new_column_names.len() - 1);
    out_names.extend_from_slice(&outer_names[..col_idx]);
    out_names.extend_from_slice(&new_column_names);
    out_names.extend_from_slice(&outer_names[col_idx + 1..]);

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for row in &outer_rows {
        match &row[col_idx] {
            Value::Table(inner) => {
                let (inner_names, inner_rows) = table_to_rows(inner)?;
                // Resolve indices once per outer row.
                let lifted_indices: Result<Vec<usize>, MError> = column_names
                    .iter()
                    .map(|n| {
                        inner_names.iter().position(|x| x == n).ok_or_else(|| {
                            MError::Other(format!(
                                "Table.ExpandTableColumn: inner column not found: {}",
                                n
                            ))
                        })
                    })
                    .collect();
                let lifted_indices = lifted_indices?;
                // Empty inner table → outer row drops (matches PQ).
                for inner_row in &inner_rows {
                    let mut new_row = Vec::with_capacity(out_names.len());
                    new_row.extend_from_slice(&row[..col_idx]);
                    for &i in &lifted_indices {
                        new_row.push(inner_row[i].clone());
                    }
                    new_row.extend_from_slice(&row[col_idx + 1..]);
                    out_rows.push(new_row);
                }
            }
            Value::Null => {
                // Null cell → emit one row with all lifted columns null.
                let mut new_row = Vec::with_capacity(out_names.len());
                new_row.extend_from_slice(&row[..col_idx]);
                for _ in &column_names {
                    new_row.push(Value::Null);
                }
                new_row.extend_from_slice(&row[col_idx + 1..]);
                out_rows.push(new_row);
            }
            other => {
                return Err(MError::Other(format!(
                    "Table.ExpandTableColumn: cell at column {} is not a table (got {})",
                    column,
                    super::super::type_name(other)
                )));
            }
        }
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}


fn table_unpivot(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let pivot_columns = expect_text_list(&args[1], "Table.Unpivot: pivotColumns")?;
    let attribute_column = expect_text(&args[2])?.to_string();
    let value_column = expect_text(&args[3])?.to_string();
    do_unpivot(table, &pivot_columns, &attribute_column, &value_column, "Table.Unpivot")
}


fn table_unpivot_other_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let keep_columns = expect_text_list(&args[1], "Table.UnpivotOtherColumns: pivotColumns")?;
    let attribute_column = expect_text(&args[2])?.to_string();
    let value_column = expect_text(&args[3])?.to_string();
    // "Other" form: pivotColumns is the set to KEEP; everything else gets unpivoted.
    let all_names = table.column_names();
    let pivot_columns: Vec<String> = all_names
        .into_iter()
        .filter(|n| !keep_columns.contains(n))
        .collect();
    do_unpivot(
        table,
        &pivot_columns,
        &attribute_column,
        &value_column,
        "Table.UnpivotOtherColumns",
    )
}

/// Shared core for both Unpivot variants. For each input row, for each
/// pivot column, emit one output row: [non-pivoted columns..., attribute, value].

fn do_unpivot(
    table: &Table,
    pivot_columns: &[String],
    attribute_column: &str,
    value_column: &str,
    ctx: &str,
) -> Result<Value, MError> {
    let (names, rows) = table_to_rows(table)?;
    // Resolve pivot indices and validate.
    let pivot_indices: Vec<usize> = pivot_columns
        .iter()
        .map(|p| {
            names
                .iter()
                .position(|n| n == p)
                .ok_or_else(|| MError::Other(format!("{}: column not found: {}", ctx, p)))
        })
        .collect::<Result<_, _>>()?;
    let keep_indices: Vec<usize> = (0..names.len())
        .filter(|i| !pivot_indices.contains(i))
        .collect();

    // Output columns: kept columns (in original order) + attribute + value.
    let mut out_names: Vec<String> = keep_indices.iter().map(|&i| names[i].clone()).collect();
    out_names.push(attribute_column.to_string());
    out_names.push(value_column.to_string());

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for row in &rows {
        let kept: Vec<Value> = keep_indices.iter().map(|&i| row[i].clone()).collect();
        for &p_idx in &pivot_indices {
            let mut new_row = kept.clone();
            new_row.push(Value::Text(names[p_idx].clone()));
            new_row.push(row[p_idx].clone());
            out_rows.push(new_row);
        }
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

/// Table.Pivot — inverse of Unpivot. Group input rows by the row-key columns
/// (everything except attributeColumn and valueColumn). For each group, emit
/// one output row whose extra columns (one per pivotValue) hold the
/// valueColumn cell whose attributeColumn cell matches that pivotValue. When
/// multiple input rows match the same (group, pivotValue) pair, apply the
/// optional aggregationFunction; otherwise default to the *last* matching
/// value (PQ's documented default).

fn table_pivot(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let pivot_values = expect_text_list(&args[1], "Table.Pivot: pivotValues")?;
    let attribute_column = expect_text(&args[2])?.to_string();
    let value_column = expect_text(&args[3])?.to_string();
    let aggregation: Option<&Closure> = match args.get(4) {
        Some(Value::Function(c)) => Some(c),
        Some(Value::Null) | None => None,
        Some(other) => return Err(type_mismatch("function", other)),
    };

    let (names, rows) = table_to_rows(table)?;
    let attr_idx = names
        .iter()
        .position(|n| n == &attribute_column)
        .ok_or_else(|| {
            MError::Other(format!(
                "Table.Pivot: attributeColumn not found: {}",
                attribute_column
            ))
        })?;
    let val_idx = names.iter().position(|n| n == &value_column).ok_or_else(|| {
        MError::Other(format!(
            "Table.Pivot: valueColumn not found: {}",
            value_column
        ))
    })?;
    let key_indices: Vec<usize> = (0..names.len())
        .filter(|i| *i != attr_idx && *i != val_idx)
        .collect();

    // Group rows by key tuple (preserving first-seen order).
    let mut groups: Vec<(Vec<Value>, Vec<usize>)> = Vec::new(); // (key, row indices)
    for (row_i, row) in rows.iter().enumerate() {
        let key: Vec<Value> = key_indices.iter().map(|&i| row[i].clone()).collect();
        let mut placed = false;
        for (existing_key, idxs) in groups.iter_mut() {
            let mut all_match = true;
            for (a, b) in existing_key.iter().zip(key.iter()) {
                if !values_equal_primitive(a, b)? {
                    all_match = false;
                    break;
                }
            }
            if all_match {
                idxs.push(row_i);
                placed = true;
                break;
            }
        }
        if !placed {
            groups.push((key, vec![row_i]));
        }
    }

    // Output: key columns + one column per pivot value.
    let mut out_names: Vec<String> = key_indices.iter().map(|&i| names[i].clone()).collect();
    for pv in &pivot_values {
        out_names.push(pv.clone());
    }

    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(groups.len());
    for (key, row_idxs) in &groups {
        let mut out_row = key.clone();
        for pv in &pivot_values {
            // Collect all valueColumn cells whose attributeColumn cell equals pv.
            let mut matches: Vec<Value> = Vec::new();
            for &ri in row_idxs {
                let attr_cell = &rows[ri][attr_idx];
                let attr_text = match attr_cell {
                    Value::Text(s) => s.as_str(),
                    Value::Null => continue,
                    other => {
                        return Err(MError::Other(format!(
                            "Table.Pivot: attributeColumn cell is not text (got {})",
                            super::super::type_name(other)
                        )));
                    }
                };
                if attr_text == pv.as_str() {
                    matches.push(rows[ri][val_idx].clone());
                }
            }
            let cell = match (matches.len(), aggregation) {
                (0, _) => Value::Null,
                (_, Some(f)) => {
                    invoke_callback_with_host(f, vec![Value::List(matches)], host)?
                }
                // No aggregator: PQ's default is the last matching value.
                (_, None) => matches.pop().unwrap(),
            };
            out_row.push(cell);
        }
        out_rows.push(out_row);
    }

    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

/// Table.Join — flat join. Like NestedJoin but matched rows merge into a
/// single output row whose columns are the union of both tables'. The
/// right-side key column is dropped.

fn table_join(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table1 = expect_table(&args[0])?;
    let key1 = match &args[1] {
        Value::Text(s) => s.clone(),
        Value::List(_) => {
            return Err(MError::NotImplemented(
                "Table.Join: composite keys (text-list form) not yet supported",
            ));
        }
        other => return Err(type_mismatch("text", other)),
    };
    let table2 = expect_table(&args[2])?;
    let key2 = match &args[3] {
        Value::Text(s) => s.clone(),
        Value::List(_) => {
            return Err(MError::NotImplemented(
                "Table.Join: composite keys (text-list form) not yet supported",
            ));
        }
        other => return Err(type_mismatch("text", other)),
    };
    // joinKind default for Table.Join is Inner (0); cf. NestedJoin which
    // defaults to LeftOuter.
    let join_kind = match args.get(4) {
        Some(Value::Number(n)) if n.fract() == 0.0 => *n as i32,
        Some(Value::Null) | None => 0,
        Some(other) => return Err(type_mismatch("number (JoinKind)", other)),
    };
    if !matches!(join_kind, 0 | 1) {
        return Err(MError::NotImplemented(
            "Table.Join: only Inner (0) and LeftOuter (1) join kinds supported",
        ));
    }

    let (left_names, left_rows) = table_to_rows(table1)?;
    let (right_names, right_rows) = table_to_rows(table2)?;

    let key1_idx = left_names.iter().position(|n| n == &key1).ok_or_else(|| {
        MError::Other(format!("Table.Join: key1 column not found: {}", key1))
    })?;
    let key2_idx = right_names.iter().position(|n| n == &key2).ok_or_else(|| {
        MError::Other(format!("Table.Join: key2 column not found: {}", key2))
    })?;
    let right_keep: Vec<usize> = (0..right_names.len()).filter(|i| *i != key2_idx).collect();

    let mut out_names: Vec<String> = left_names.clone();
    for &i in &right_keep {
        out_names.push(right_names[i].clone());
    }

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for left_row in &left_rows {
        let lkey = &left_row[key1_idx];
        let mut any_match = false;
        for right_row in &right_rows {
            if values_equal_primitive(lkey, &right_row[key2_idx])? {
                let mut new_row = left_row.clone();
                for &i in &right_keep {
                    new_row.push(right_row[i].clone());
                }
                out_rows.push(new_row);
                any_match = true;
            }
        }
        if !any_match && join_kind == 1 {
            let mut new_row = left_row.clone();
            for _ in &right_keep {
                new_row.push(Value::Null);
            }
            out_rows.push(new_row);
        }
    }

    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}


fn table_nested_join(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table1 = expect_table(&args[0])?;
    let key1 = match &args[1] {
        Value::Text(s) => s.clone(),
        Value::List(_) => {
            return Err(MError::NotImplemented(
                "Table.NestedJoin: composite keys (text-list form) not yet supported",
            ));
        }
        other => return Err(type_mismatch("text", other)),
    };
    let table2 = expect_table(&args[2])?;
    let key2 = match &args[3] {
        Value::Text(s) => s.clone(),
        Value::List(_) => {
            return Err(MError::NotImplemented(
                "Table.NestedJoin: composite keys (text-list form) not yet supported",
            ));
        }
        other => return Err(type_mismatch("text", other)),
    };
    let new_column_name = expect_text(&args[4])?.to_string();
    // joinKind: 0=Inner, 1=LeftOuter, 2=RightOuter, 3=FullOuter, 4=LeftAnti, 5=RightAnti
    let join_kind = match args.get(5) {
        Some(Value::Number(n)) if n.fract() == 0.0 => *n as i32,
        Some(Value::Null) | None => 1, // default: LeftOuter
        Some(other) => return Err(type_mismatch("number (JoinKind)", other)),
    };
    if !matches!(join_kind, 0 | 1) {
        return Err(MError::NotImplemented(
            "Table.NestedJoin: only Inner (0) and LeftOuter (1) join kinds supported",
        ));
    }

    let (left_names, left_rows) = table_to_rows(table1)?;
    let (right_names, right_rows) = table_to_rows(table2)?;

    let key1_idx = left_names.iter().position(|n| n == &key1).ok_or_else(|| {
        MError::Other(format!(
            "Table.NestedJoin: key1 column not found: {}",
            key1
        ))
    })?;
    let key2_idx = right_names.iter().position(|n| n == &key2).ok_or_else(|| {
        MError::Other(format!(
            "Table.NestedJoin: key2 column not found: {}",
            key2
        ))
    })?;

    // Linear-scan match (O(n*m), fine for corpus-scale tables — no Hash on
    // Value yet, and primitive-only equality keeps the code simple).
    let mut out_names: Vec<String> = left_names.clone();
    out_names.push(new_column_name);
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(left_rows.len());

    for left_row in &left_rows {
        let lkey = &left_row[key1_idx];
        let mut matched: Vec<Vec<Value>> = Vec::new();
        for right_row in &right_rows {
            if values_equal_primitive(lkey, &right_row[key2_idx])? {
                matched.push(right_row.clone());
            }
        }
        let inner_table =
            Table::from_rows(right_names.clone(), matched.clone());
        match join_kind {
            0 => {
                // Inner: drop left rows with no matches.
                if matched.is_empty() {
                    continue;
                }
                let mut new_row = left_row.clone();
                new_row.push(Value::Table(inner_table));
                out_rows.push(new_row);
            }
            1 => {
                // LeftOuter: keep every left row, even with empty nested table.
                let mut new_row = left_row.clone();
                new_row.push(Value::Table(inner_table));
                out_rows.push(new_row);
            }
            _ => unreachable!(),
        }
    }

    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}


fn table_combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let tables_list = expect_list(&args[0])?;
    if tables_list.is_empty() {
        return Err(MError::Other("Table.Combine: empty table list".into()));
    }
    // Collect input tables.
    let tables: Vec<&Table> = tables_list
        .iter()
        .map(|t| match t {
            Value::Table(table) => Ok(table),
            other => Err(type_mismatch("table (in list)", other)),
        })
        .collect::<Result<_, _>>()?;

    // Fast path: all Arrow + identical schemas → arrow concat.
    let all_arrow = tables
        .iter()
        .all(|t| matches!(&t.repr, super::super::value::TableRepr::Arrow(_)));
    if all_arrow {
        let batches: Vec<RecordBatch> = tables
            .iter()
            .map(|t| t.try_to_arrow())
            .collect::<Result<_, _>>()?;
        if batches.len() == 1 {
            return Ok(Value::Table(Table::from_arrow(
                batches.into_iter().next().unwrap(),
            )));
        }
        let schema = batches[0].schema();
        let schemas_match = batches.iter().skip(1).all(|b| b.schema() == schema);
        if schemas_match {
            let combined = arrow::compute::concat_batches(&schema, &batches)
                .map_err(|e| MError::Other(format!("Table.Combine: concat failed: {}", e)))?;
            return Ok(Value::Table(Table::from_arrow(combined)));
        }
        // Schemas mismatch — fall through to Rows path which unions columns.
    }

    // Row-major fallback: take column names from the first table; verify
    // subsequent tables have the same names in the same order (PQ's Combine
    // requires aligned column sets); concatenate rows.
    let names = tables[0].column_names();
    for (i, t) in tables.iter().enumerate().skip(1) {
        if t.column_names() != names {
            return Err(MError::Other(format!(
                "Table.Combine: column set of table {} does not match table 0",
                i
            )));
        }
    }
    let mut all_rows: Vec<Vec<Value>> = Vec::new();
    for t in &tables {
        let (_, rows) = table_to_rows(t)?;
        all_rows.extend(rows);
    }
    Ok(Value::Table(values_to_table(&names, &all_rows)?))
}


fn table_transform_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let transform = expect_function(&args[1])?;
    let n_rows = table.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n_rows);
    for row in 0..n_rows {
        let record = row_to_record(table, row)?;
        out.push(invoke_callback_with_host(transform, vec![record], host)?);
    }
    Ok(Value::List(out))
}


fn table_insert_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let new_records = expect_list(&args[2])?;
    let n_existing = table.num_rows();
    if offset > n_existing {
        return Err(MError::Other(format!(
            "Table.InsertRows: offset {} exceeds row count {}",
            offset, n_existing
        )));
    }

    // Column names come from the original schema.
    let names: Vec<String> = table.column_names();

    // Build the merged row list: existing[..offset], new, existing[offset..].
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(n_existing + new_records.len());
    for row in 0..offset {
        let mut cells = Vec::with_capacity(names.len());
        for col in 0..names.len() {
            cells.push(cell_to_value(table, col, row)?);
        }
        rows.push(cells);
    }
    for r in new_records {
        let record = match r {
            Value::Record(rec) => rec,
            other => return Err(type_mismatch("record (in rows)", other)),
        };
        let mut cells = Vec::with_capacity(names.len());
        for name in &names {
            let v = record
                .fields
                .iter()
                .find(|(n, _)| n == name)
                .map(|(_, v)| v.clone())
                .unwrap_or(Value::Null);
            // Record literal fields are thunks per the spec — force before
            // pushing to the Arrow batch builder.
            let v = super::super::force(v, &mut |e, env| super::super::evaluate(e, env, host))?;
            cells.push(v);
        }
        rows.push(cells);
    }
    for row in offset..n_existing {
        let mut cells = Vec::with_capacity(names.len());
        for col in 0..names.len() {
            cells.push(cell_to_value(table, col, row)?);
        }
        rows.push(cells);
    }

    Ok(Value::Table(values_to_table(&names, &rows)?))
}

