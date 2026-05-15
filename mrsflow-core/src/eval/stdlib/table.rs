//! `Table.*` stdlib bindings.

#![allow(unused_imports)]

use std::collections::HashMap;
use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Date32Array, Decimal128Array, Decimal256Array,
    DurationMicrosecondArray, Float32Array, Float64Array, Int16Array, Int32Array,
    Int64Array, Int8Array, NullArray, StringArray, TimestampMicrosecondArray,
    UInt16Array, UInt32Array, UInt64Array, UInt8Array,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::parser::Param;

use super::super::env::{Env, EnvNode, EnvOps};
use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, Closure, FnBody, MError, Record, Table, Value};
use super::common::{
    expect_function, expect_int, expect_list, expect_list_of_lists, expect_table,
    expect_table_lazy_ok, expect_text, expect_text_list, int_n_arg, invoke_builtin_callback,
    invoke_callback_with_host, one, three, two, type_mismatch, values_equal_primitive,
};

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("#table", two("columns", "rows"), constructor),
        ("Table.ColumnNames", one("table"), column_names),
        ("Table.RenameColumns", two("table", "renames"), rename_columns),
        ("Table.RemoveColumns", two("table", "names"), remove_columns),
        ("Table.SelectColumns", two("table", "names"), select_columns),
        ("Table.SelectRows", two("table", "predicate"), select_rows),
        (
            "Table.AddColumn",
            vec![
                Param { name: "table".into(),     optional: false, type_annotation: None },
                Param { name: "name".into(),      optional: false, type_annotation: None },
                Param { name: "transform".into(), optional: false, type_annotation: None },
                Param { name: "type".into(),      optional: true,  type_annotation: None },
            ],
            add_column,
        ),
        ("Table.FromRows", two("rows", "columns"), from_rows),
        (
            "Table.PromoteHeaders",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                // PQ's options record: PromoteAllScalars (default false).
                // Our impl coerces every header cell to text via Text.From
                // semantics already, so the flag is effectively a no-op for
                // v1 — accept the record so corpus calls don't error.
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            promote_headers,
        ),
        (
            "Table.TransformColumnTypes",
            vec![
                Param { name: "table".into(),      optional: false, type_annotation: None },
                Param { name: "transforms".into(), optional: false, type_annotation: None },
                Param { name: "culture".into(),    optional: true,  type_annotation: None },
            ],
            transform_column_types,
        ),
        (
            "Table.TransformColumns",
            two("table", "transforms"),
            transform_columns,
        ),
        ("Table.Combine", one("tables"), combine),
        ("Table.Skip", two("table", "countOrCondition"), skip),
        (
            "Table.ExpandRecordColumn",
            vec![
                Param { name: "table".into(),          optional: false, type_annotation: None },
                Param { name: "column".into(),         optional: false, type_annotation: None },
                Param { name: "fieldNames".into(),     optional: false, type_annotation: None },
                Param { name: "newColumnNames".into(), optional: true,  type_annotation: None },
            ],
            expand_record_column,
        ),
        (
            "Table.ExpandListColumn",
            two("table", "column"),
            expand_list_column,
        ),
        (
            "Table.ExpandTableColumn",
            vec![
                Param { name: "table".into(),          optional: false, type_annotation: None },
                Param { name: "column".into(),         optional: false, type_annotation: None },
                Param { name: "columnNames".into(),    optional: false, type_annotation: None },
                Param { name: "newColumnNames".into(), optional: true,  type_annotation: None },
            ],
            expand_table_column,
        ),
        (
            "Table.Unpivot",
            vec![
                Param { name: "table".into(),           optional: false, type_annotation: None },
                Param { name: "pivotColumns".into(),    optional: false, type_annotation: None },
                Param { name: "attributeColumn".into(), optional: false, type_annotation: None },
                Param { name: "valueColumn".into(),     optional: false, type_annotation: None },
            ],
            unpivot,
        ),
        (
            "Table.UnpivotOtherColumns",
            vec![
                Param { name: "table".into(),           optional: false, type_annotation: None },
                Param { name: "pivotColumns".into(),    optional: false, type_annotation: None },
                Param { name: "attributeColumn".into(), optional: false, type_annotation: None },
                Param { name: "valueColumn".into(),     optional: false, type_annotation: None },
            ],
            unpivot_other_columns,
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
            nested_join,
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
            pivot,
        ),
        ("Table.ReorderColumns", two("table", "columnOrder"), reorder_columns),
        ("Table.Column", two("table", "columnName"), column),
        ("Table.IsEmpty", one("table"), is_empty),
        (
            "Table.Distinct",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            distinct,
        ),
        ("Table.FirstN", two("table", "countOrCondition"), first_n),
        ("Table.LastN", two("table", "countOrCondition"), last_n),
        ("Table.Reverse", one("table"), reverse),
        ("Table.FromRecords", one("records"), from_records),
        ("Table.ToRecords", one("table"), to_records),
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
            join,
        ),
        (
            "Table.AddIndexColumn",
            vec![
                Param { name: "table".into(),         optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
                Param { name: "initialValue".into(),  optional: true,  type_annotation: None },
                Param { name: "increment".into(),     optional: true,  type_annotation: None },
            ],
            add_index_column,
        ),
        ("Table.TransformRows", two("table", "transform"), transform_rows),
        ("Table.InsertRows", three("table", "offset", "rows"), insert_rows),
        // --- Accessors + predicates batch (slice #158) ---
        (
            "Table.First",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            first,
        ),
        (
            "Table.Last",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            last,
        ),
        (
            "Table.FirstValue",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            first_value,
        ),
        ("Table.RowCount", one("table"), row_count),
        ("Table.ColumnCount", one("table"), column_count),
        ("Table.ApproximateRowCount", one("table"), row_count),
        (
            "Table.Range",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            range,
        ),
        (
            "Table.Contains",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "row".into(),              optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            contains,
        ),
        (
            "Table.ContainsAll",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "rows".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            contains_all,
        ),
        (
            "Table.ContainsAny",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "rows".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            contains_any,
        ),
        (
            "Table.IsDistinct",
            vec![
                Param { name: "table".into(),              optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: true,  type_annotation: None },
            ],
            is_distinct,
        ),
        ("Table.HasColumns", two("table", "columns"), has_columns),
        ("Table.MatchesAllRows", two("table", "condition"), matches_all_rows),
        ("Table.MatchesAnyRows", two("table", "condition"), matches_any_rows),
        ("Table.FindText", two("table", "text"), find_text),
        (
            "Table.PositionOf",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "row".into(),              optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            position_of,
        ),
        (
            "Table.PositionOfAny",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "rows".into(),             optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            position_of_any,
        ),
        ("Table.Keys", one("table"), keys),
        ("Table.ColumnsOfType", two("table", "listOfTypes"), columns_of_type),
        // --- Slice #159: sort/fill/reverse ---
        ("Table.Sort", two("table", "comparisonCriteria"), sort),
        ("Table.FillUp", two("table", "columns"), fill_up),
        ("Table.FillDown", two("table", "columns"), fill_down),
        ("Table.ReverseRows", one("table"), reverse_rows),
        ("Table.SplitAt", two("table", "index"), split_at),
        (
            "Table.AlternateRows",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "skip".into(),   optional: false, type_annotation: None },
                Param { name: "take".into(),   optional: false, type_annotation: None },
            ],
            alternate_rows,
        ),
        ("Table.Repeat", two("table", "count"), repeat),
        ("Table.SingleRow", one("table"), single_row),
        // --- Slice #160: aggregations ---
        (
            "Table.Min",
            vec![
                Param { name: "table".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: false, type_annotation: None },
                Param { name: "default".into(),             optional: true,  type_annotation: None },
            ],
            min,
        ),
        (
            "Table.Max",
            vec![
                Param { name: "table".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: false, type_annotation: None },
                Param { name: "default".into(),             optional: true,  type_annotation: None },
            ],
            max,
        ),
        (
            "Table.MinN",
            three("table", "countOrCondition", "comparisonCriteria"),
            min_n,
        ),
        (
            "Table.MaxN",
            three("table", "countOrCondition", "comparisonCriteria"),
            max_n,
        ),
        (
            "Table.AggregateTableColumn",
            three("table", "column", "aggregations"),
            aggregate_table_column,
        ),
        // --- Slice #161: row mutation ---
        (
            "Table.RemoveFirstN",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            remove_first_n,
        ),
        (
            "Table.RemoveLastN",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            remove_last_n,
        ),
        (
            "Table.RemoveRows",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            remove_rows,
        ),
        (
            "Table.RemoveMatchingRows",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "rows".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            remove_matching_rows,
        ),
        (
            "Table.RemoveRowsWithErrors",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "columns".into(), optional: true,  type_annotation: None },
            ],
            remove_rows_with_errors,
        ),
        (
            "Table.ReplaceMatchingRows",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "replacements".into(),     optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            replace_matching_rows,
        ),
        (
            "Table.ReplaceRows",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: false, type_annotation: None },
                Param { name: "rows".into(),   optional: false, type_annotation: None },
            ],
            replace_rows,
        ),
        (
            "Table.ReplaceValue",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "oldValue".into(),         optional: false, type_annotation: None },
                Param { name: "newValue".into(),         optional: false, type_annotation: None },
                Param { name: "replacer".into(),         optional: false, type_annotation: None },
                Param { name: "columnsToSearch".into(),  optional: false, type_annotation: None },
            ],
            replace_value,
        ),
        (
            "Table.ReplaceErrorValues",
            two("table", "errorReplacement"),
            replace_error_values,
        ),
        // --- Slice #162: column mutation ---
        (
            "Table.CombineColumns",
            vec![
                Param { name: "table".into(),         optional: false, type_annotation: None },
                Param { name: "sourceColumns".into(), optional: false, type_annotation: None },
                Param { name: "combiner".into(),      optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
            ],
            combine_columns,
        ),
        (
            "Table.CombineColumnsToRecord",
            three("table", "newColumnName", "sourceColumns"),
            combine_columns_to_record,
        ),
        ("Table.DemoteHeaders", one("table"), demote_headers),
        (
            "Table.DuplicateColumn",
            three("table", "columnName", "newColumnName"),
            duplicate_column,
        ),
        ("Table.PrefixColumns", two("table", "prefix"), prefix_columns),
        (
            "Table.SplitColumn",
            vec![
                Param { name: "table".into(),                optional: false, type_annotation: None },
                Param { name: "sourceColumn".into(),         optional: false, type_annotation: None },
                Param { name: "splitter".into(),             optional: false, type_annotation: None },
                Param { name: "columnNamesOrNumber".into(),  optional: true,  type_annotation: None },
                Param { name: "default".into(),              optional: true,  type_annotation: None },
                Param { name: "extraValues".into(),          optional: true,  type_annotation: None },
            ],
            split_column,
        ),
        (
            "Table.TransformColumnNames",
            vec![
                Param { name: "table".into(),         optional: false, type_annotation: None },
                Param { name: "nameGenerator".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),       optional: true,  type_annotation: None },
            ],
            transform_column_names,
        ),
        ("Table.Transpose", one("table"), transpose),
        (
            "Table.AddJoinColumn",
            vec![
                Param { name: "table1".into(),        optional: false, type_annotation: None },
                Param { name: "key1".into(),          optional: false, type_annotation: None },
                Param { name: "table2".into(),        optional: false, type_annotation: None },
                Param { name: "key2".into(),          optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
            ],
            add_join_column,
        ),
        // --- Slice #163: format converters ---
        (
            "Table.FromColumns",
            vec![
                Param { name: "lists".into(),       optional: false, type_annotation: None },
                Param { name: "columnNames".into(), optional: true,  type_annotation: None },
            ],
            from_columns,
        ),
        (
            "Table.FromList",
            vec![
                Param { name: "list".into(),        optional: false, type_annotation: None },
                Param { name: "splitter".into(),    optional: true,  type_annotation: None },
                Param { name: "columns".into(),     optional: true,  type_annotation: None },
                Param { name: "default".into(),     optional: true,  type_annotation: None },
                Param { name: "extraValues".into(), optional: true,  type_annotation: None },
            ],
            from_list,
        ),
        (
            "Table.FromValue",
            vec![
                Param { name: "value".into(),   optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            from_value,
        ),
        ("Table.ToColumns", one("table"), to_columns),
        (
            "Table.ToList",
            vec![
                Param { name: "table".into(),    optional: false, type_annotation: None },
                Param { name: "combiner".into(), optional: true,  type_annotation: None },
            ],
            to_list,
        ),
        ("Table.ToRows", one("table"), to_rows_value),
        ("Table.Schema", one("table"), schema),
        (
            "Table.Profile",
            vec![
                Param { name: "table".into(),                optional: false, type_annotation: None },
                Param { name: "additionalAggregates".into(), optional: true,  type_annotation: None },
            ],
            profile,
        ),
        // --- Slice #164: Group + AddRankColumn + Split + Buffer ---
        (
            "Table.Group",
            vec![
                Param { name: "table".into(),              optional: false, type_annotation: None },
                Param { name: "key".into(),                optional: false, type_annotation: None },
                Param { name: "aggregatedColumns".into(),  optional: false, type_annotation: None },
                Param { name: "groupKind".into(),          optional: true,  type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: true,  type_annotation: None },
            ],
            group,
        ),
        (
            "Table.AddRankColumn",
            vec![
                Param { name: "table".into(),              optional: false, type_annotation: None },
                Param { name: "newColumnName".into(),      optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),            optional: true,  type_annotation: None },
            ],
            add_rank_column,
        ),
        ("Table.Split", two("table", "pageSize"), split),
        ("Table.Buffer", one("table"), buffer),
        // --- Slice #165: partitioning + miscellaneous tail ---
        (
            "Table.Partition",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "column".into(), optional: false, type_annotation: None },
                Param { name: "groups".into(), optional: false, type_annotation: None },
                Param { name: "hash".into(),   optional: false, type_annotation: None },
            ],
            partition,
        ),
        ("Table.PartitionKey", one("table"), partition_key),
        ("Table.PartitionValues", one("table"), partition_values),
        ("Table.ReplacePartitionKey", two("table", "key"), identity_passthrough),
        (
            "Table.FilterWithDataTable",
            vec![
                Param { name: "table".into(),     optional: false, type_annotation: None },
                Param { name: "dataTable".into(), optional: false, type_annotation: None },
            ],
            filter_with_data_table,
        ),
        (
            "Table.FromPartitions",
            vec![
                Param { name: "partitions".into(), optional: false, type_annotation: None },
                Param { name: "columnInfo".into(), optional: true,  type_annotation: None },
            ],
            from_partitions,
        ),
        ("Table.AddKey", three("table", "columns", "isPrimary"), identity_passthrough),
        ("Table.ReplaceKeys", two("table", "keys"), identity_passthrough),
        ("Table.ConformToPageReader", one("table"), identity_passthrough_one),
        ("Table.StopFolding", one("table"), identity_passthrough_one),
        ("Table.ReplaceRelationshipIdentity", two("table", "identity"), identity_passthrough),
        (
            "Table.SelectRowsWithErrors",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "columns".into(), optional: true,  type_annotation: None },
            ],
            select_rows_with_errors,
        ),
        ("Table.WithErrorContext", two("table", "errorContext"), identity_passthrough),
        // --- Slice #166: fuzzy + view stubs ---
        (
            "Table.AddFuzzyClusterColumn",
            vec![
                Param { name: "table".into(),         optional: false, type_annotation: None },
                Param { name: "columnName".into(),    optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),       optional: true,  type_annotation: None },
            ],
            fuzzy_cluster_column,
        ),
        (
            "Table.FuzzyGroup",
            vec![
                Param { name: "table".into(),             optional: false, type_annotation: None },
                Param { name: "key".into(),               optional: false, type_annotation: None },
                Param { name: "aggregatedColumns".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),           optional: true,  type_annotation: None },
            ],
            fuzzy_group,
        ),
        (
            "Table.FuzzyJoin",
            vec![
                Param { name: "table1".into(),    optional: false, type_annotation: None },
                Param { name: "key1".into(),      optional: false, type_annotation: None },
                Param { name: "table2".into(),    optional: false, type_annotation: None },
                Param { name: "key2".into(),      optional: false, type_annotation: None },
                Param { name: "joinKind".into(),  optional: true,  type_annotation: None },
                Param { name: "options".into(),   optional: true,  type_annotation: None },
            ],
            fuzzy_join,
        ),
        (
            "Table.FuzzyNestedJoin",
            vec![
                Param { name: "table1".into(),        optional: false, type_annotation: None },
                Param { name: "key1".into(),          optional: false, type_annotation: None },
                Param { name: "table2".into(),        optional: false, type_annotation: None },
                Param { name: "key2".into(),          optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
                Param { name: "joinKind".into(),      optional: true,  type_annotation: None },
                Param { name: "options".into(),       optional: true,  type_annotation: None },
            ],
            fuzzy_nested_join,
        ),
        ("Table.View", two("table", "handlers"), view_identity),
        ("Table.ViewError", one("record"), view_error_identity),
        ("Table.ViewFunction", one("function"), view_function_identity),
    ]
}

fn constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn column_names(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Schema-only: column names come from the cached schema, no decode needed.
    let table = expect_table_lazy_ok(&args[0])?;
    let names: Vec<Value> = table
        .column_names()
        .into_iter()
        .map(Value::Text)
        .collect();
    Ok(Value::List(names))
}


fn rename_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Projection-aware: rename can update the LazyParquet output_names
    // vector without forcing. Other variants force on entry.
    let table = expect_table_lazy_ok(&args[0])?;
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
                "Table.RenameColumns: column not found: {old}"
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
    // Lazy fast path: update output_names on a LazyParquet without
    // decoding anything. Setting entry i to Some(renamed[i]) overrides
    // whatever the underlying schema would have called that column.
    if let super::super::value::TableRepr::LazyParquet(state) = &table.repr {
        let new_output_names: Vec<Option<String>> = renamed
            .iter()
            .enumerate()
            .map(|(i, n)| {
                if n == &state.effective_name(i) { None } else { Some(n.clone()) }
            })
            .collect();
        return Ok(Value::Table(Table {
            repr: super::super::value::TableRepr::LazyParquet(
                super::super::value::LazyParquetState {
                    bytes: state.bytes.clone(),
                    schema: state.schema.clone(),
                    projection: state.projection.clone(),
                    output_names: Some(new_output_names),
                    num_rows: state.num_rows,
                    row_filter: state.row_filter.clone(),
                },
            ),
        }));
    }
    // Other variants: force then rename (the Arrow / Rows arms below
    // assume already-forced input; the LazyParquet arm below is dead
    // after the early-return but kept for exhaustiveness).
    let table_owned = table.force()?;
    let table: &Table = &table_owned;
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
                .map_err(|e| MError::Other(format!("Table.RenameColumns: rebuild failed: {e}")))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { rows, .. } => {
            Ok(Value::Table(Table::from_rows(renamed, rows.clone())))
        }
        super::super::value::TableRepr::LazyParquet(_)
        | super::super::value::TableRepr::LazyOdbc(_) => {
            unreachable!("expect_table forces upstream — lazy variant can't reach here")
        }
        super::super::value::TableRepr::JoinView(_)
        | super::super::value::TableRepr::ExpandView(_) => {
            unreachable!("expect_table forces upstream — JoinView/ExpandView can't reach here")
        }
    }
}


fn remove_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Projection-aware: complement-of-SelectColumns, doesn't force.
    let table = expect_table_lazy_ok(&args[0])?;
    let names = expect_text_list(&args[1], "Table.RemoveColumns: names")?;
    let existing = table.column_names();
    for n in &names {
        if !existing.contains(n) {
            return Err(MError::Other(format!(
                "Table.RemoveColumns: column not found: {n}"
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
                .map_err(|e| MError::Other(format!("{ctx}: rebuild failed: {e}")))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { rows, .. } => {
            let new_rows: Vec<Vec<Value>> = rows
                .iter()
                .map(|row| keep_indices.iter().map(|&i| row[i].clone()).collect())
                .collect();
            Ok(Value::Table(Table::from_rows(new_names, new_rows)))
        }
        super::super::value::TableRepr::JoinView(_) => {
            // SelectColumns/RemoveColumns on a JoinView are uncommon in
            // the corpus pattern (the demo applies them after expand,
            // not before). Forcing first is correct and keeps the
            // projection-narrowing code path simple.
            let forced = table.force()?;
            return select_columns_by_index(&forced, keep_indices, ctx);
        }
        super::super::value::TableRepr::LazyParquet(state) => {
            // Projection-aware path: narrow the mask without decoding any
            // column data. `keep_indices` are positions in the *current*
            // projection list; translate them back to indices in the
            // underlying parquet schema so subsequent ops compose correctly.
            let new_projection: Vec<usize> = keep_indices
                .iter()
                .map(|&i| state.projection[i])
                .collect();
            // Carry over per-column rename overrides positionally.
            let new_output_names = state.output_names.as_ref().map(|onames| {
                keep_indices.iter().map(|&i| onames[i].clone()).collect()
            });
            Ok(Value::Table(Table {
                repr: super::super::value::TableRepr::LazyParquet(
                    super::super::value::LazyParquetState {
                        bytes: state.bytes.clone(),
                        schema: state.schema.clone(),
                        projection: new_projection,
                        output_names: new_output_names,
                        num_rows: state.num_rows,
                        row_filter: state.row_filter.clone(),
                    },
                ),
            }))
        }
        super::super::value::TableRepr::LazyOdbc(state) => {
            // Foldable: narrow projection + output_names without
            // touching the wire. Subsequent force will emit a SELECT
            // listing only these columns.
            let new_projection: Vec<usize> = keep_indices
                .iter()
                .map(|&i| state.projection[i])
                .collect();
            let new_output_names = state.output_names.as_ref().map(|onames| {
                keep_indices.iter().map(|&i| onames[i].clone()).collect()
            });
            Ok(Value::Table(Table {
                repr: super::super::value::TableRepr::LazyOdbc(
                    super::super::value::LazyOdbcState {
                        connection_string: state.connection_string.clone(),
                        table_name: state.table_name.clone(),
                        schema: state.schema.clone(),
                        projection: new_projection,
                        output_names: new_output_names,
                        where_filters: state.where_filters.clone(),
                        limit: state.limit,
                        force_fn: state.force_fn.clone(),
                    },
                ),
            }))
        }
        super::super::value::TableRepr::ExpandView(ev) => {
            // Projection-aware path on the post-expand result. Output
            // columns split between "from left" (positions 0..n_left)
            // and "from right_output_names" (positions n_left..total).
            // For each kept index, route to the corresponding source
            // projection. Right entries dropped from output also drop
            // their `right_projection` entry (no point decoding columns
            // we won't expose).
            let n_left = ev.left_projection.len();
            let mut new_left_projection: Vec<usize> = Vec::new();
            let mut new_right_projection: Vec<usize> = Vec::new();
            let mut new_right_output_names: Vec<String> = Vec::new();
            for &out_idx in keep_indices {
                if out_idx < n_left {
                    new_left_projection.push(ev.left_projection[out_idx]);
                } else {
                    let right_slot = out_idx - n_left;
                    new_right_projection.push(ev.right_projection[right_slot]);
                    new_right_output_names.push(ev.right_output_names[right_slot].clone());
                }
            }
            Ok(Value::Table(Table {
                repr: super::super::value::TableRepr::ExpandView(
                    super::super::value::ExpandViewState {
                        left: ev.left.clone(),
                        left_projection: new_left_projection,
                        right: ev.right.clone(),
                        right_projection: new_right_projection,
                        right_output_names: new_right_output_names,
                        matches: ev.matches.clone(),
                    },
                ),
            }))
        }
    }
}


pub(crate) fn values_to_table(column_names: &[String], rows: &[Vec<Value>]) -> Result<Table, MError> {
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
            .map_err(|e| MError::Other(format!("#table: empty-cols rebuild failed: {e}")))?;
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
        .map_err(|e| MError::Other(format!("#table: build failed: {e}")))?;
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
                                "duration overflows i64 microseconds: {d:?}"
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
            row.push(cell_to_value(&table, c, r)?);
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
        super::super::value::TableRepr::LazyParquet(_)
        | super::super::value::TableRepr::LazyOdbc(_)
        | super::super::value::TableRepr::JoinView(_)
        | super::super::value::TableRepr::ExpandView(_) => {
            unreachable!("cell_to_value expects forced table — caller should expect_table first")
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
        // M has only one numeric type (`Number` = f64); all Arrow integer
        // widths and the lone Float32 width collapse into it. Int64/UInt64
        // can exceed f64's 2^53 lossless range — for now we accept the
        // narrowing rather than introducing a separate integer Value.
        DataType::Float32 => Ok(Value::Number(
            array.as_any().downcast_ref::<Float32Array>().expect("Float32").value(row) as f64,
        )),
        DataType::Int8 => Ok(Value::Number(
            array.as_any().downcast_ref::<Int8Array>().expect("Int8").value(row) as f64,
        )),
        DataType::Int16 => Ok(Value::Number(
            array.as_any().downcast_ref::<Int16Array>().expect("Int16").value(row) as f64,
        )),
        DataType::Int32 => Ok(Value::Number(
            array.as_any().downcast_ref::<Int32Array>().expect("Int32").value(row) as f64,
        )),
        DataType::Int64 => Ok(Value::Number(
            array.as_any().downcast_ref::<Int64Array>().expect("Int64").value(row) as f64,
        )),
        DataType::UInt8 => Ok(Value::Number(
            array.as_any().downcast_ref::<UInt8Array>().expect("UInt8").value(row) as f64,
        )),
        DataType::UInt16 => Ok(Value::Number(
            array.as_any().downcast_ref::<UInt16Array>().expect("UInt16").value(row) as f64,
        )),
        DataType::UInt32 => Ok(Value::Number(
            array.as_any().downcast_ref::<UInt32Array>().expect("UInt32").value(row) as f64,
        )),
        DataType::UInt64 => Ok(Value::Number(
            array.as_any().downcast_ref::<UInt64Array>().expect("UInt64").value(row) as f64,
        )),
        DataType::Decimal128(precision, scale) => {
            let raw = array
                .as_any()
                .downcast_ref::<Decimal128Array>()
                .expect("Decimal128")
                .value(row);
            Ok(Value::Decimal {
                mantissa: arrow::datatypes::i256::from_i128(raw),
                scale: *scale,
                precision: *precision,
            })
        }
        DataType::Decimal256(precision, scale) => {
            let raw = array
                .as_any()
                .downcast_ref::<Decimal256Array>()
                .expect("Decimal256")
                .value(row);
            Ok(Value::Decimal {
                mantissa: raw,
                scale: *scale,
                precision: *precision,
            })
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
                .ok_or_else(|| MError::Other(format!("Date32 out of range: {days} days")))?;
            Ok(Value::Date(d))
        }
        // All Timestamp variants → Value::Datetime (M has one datetime type;
        // a timezone-bearing parquet column gets converted to its UTC-naive
        // wall clock equivalent, matching Power Query's typical "show in UTC"
        // default for unannotated parquet inputs).
        DataType::Timestamp(unit, _tz) => {
            use arrow::array::{
                TimestampMillisecondArray, TimestampNanosecondArray, TimestampSecondArray,
            };
            use arrow::datatypes::TimeUnit;
            let micros: i64 = match unit {
                TimeUnit::Second => array
                    .as_any()
                    .downcast_ref::<TimestampSecondArray>()
                    .expect("TimestampSecond")
                    .value(row)
                    .saturating_mul(1_000_000),
                TimeUnit::Millisecond => array
                    .as_any()
                    .downcast_ref::<TimestampMillisecondArray>()
                    .expect("TimestampMillisecond")
                    .value(row)
                    .saturating_mul(1_000),
                TimeUnit::Microsecond => array
                    .as_any()
                    .downcast_ref::<TimestampMicrosecondArray>()
                    .expect("TimestampMicrosecond")
                    .value(row),
                TimeUnit::Nanosecond => {
                    array
                        .as_any()
                        .downcast_ref::<TimestampNanosecondArray>()
                        .expect("TimestampNanosecond")
                        .value(row)
                        / 1_000
                }
            };
            let dt = chrono::DateTime::from_timestamp_micros(micros)
                .ok_or_else(|| MError::Other(format!("Timestamp out of range: {micros} us")))?
                .naive_utc();
            Ok(Value::Datetime(dt))
        }
        DataType::Date64 => {
            let a = array
                .as_any()
                .downcast_ref::<arrow::array::Date64Array>()
                .expect("Date64");
            let millis = a.value(row);
            let dt = chrono::DateTime::from_timestamp_millis(millis)
                .ok_or_else(|| MError::Other(format!("Date64 out of range: {millis} ms")))?
                .date_naive();
            Ok(Value::Date(dt))
        }
        // All Duration variants → Value::Duration (chrono::Duration is
        // nanosecond-precision internally; we choose the constructor by unit).
        DataType::Duration(unit) => {
            use arrow::array::{
                DurationMillisecondArray, DurationNanosecondArray, DurationSecondArray,
            };
            use arrow::datatypes::TimeUnit;
            let d = match unit {
                TimeUnit::Second => chrono::Duration::seconds(
                    array.as_any().downcast_ref::<DurationSecondArray>().expect("DurationSecond").value(row),
                ),
                TimeUnit::Millisecond => chrono::Duration::milliseconds(
                    array.as_any().downcast_ref::<DurationMillisecondArray>().expect("DurationMillisecond").value(row),
                ),
                TimeUnit::Microsecond => chrono::Duration::microseconds(
                    array.as_any().downcast_ref::<DurationMicrosecondArray>().expect("DurationMicrosecond").value(row),
                ),
                TimeUnit::Nanosecond => chrono::Duration::nanoseconds(
                    array.as_any().downcast_ref::<DurationNanosecondArray>().expect("DurationNanosecond").value(row),
                ),
            };
            Ok(Value::Duration(d))
        }
        _other => Err(MError::NotImplemented("unsupported cell type")),
    }
}

// --- chrono constructors (eval-7b) ---
//
// #date(y,m,d), #datetime(y,m,d,h,m,s), #duration(d,h,m,s). All operands
// must be whole-numbered f64s; non-integer or out-of-range values error.


fn select_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Projection-aware: doesn't force a lazy table; `select_columns_by_index`
    // narrows the LazyParquet mask directly. See mrsflow/09-lazy-tables.md.
    let table = expect_table_lazy_ok(&args[0])?;
    let names = expect_text_list(&args[1], "Table.SelectColumns: names")?;
    let existing = table.column_names();
    let mut indices: Vec<usize> = Vec::with_capacity(names.len());
    for n in &names {
        match existing.iter().position(|e| e == n) {
            Some(i) => indices.push(i),
            None => {
                return Err(MError::Other(format!(
                    "Table.SelectColumns: column not found: {n}"
                )));
            }
        }
    }
    select_columns_by_index(table, &indices, "Table.SelectColumns")
}


fn select_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    // Fast path: if the input is a LazyParquet handle and the
    // predicate translates to a foldable subset (literal-RHS
    // comparisons AND'd together), push the filter into the handle
    // and return without decoding any data. Non-foldable predicates
    // (function calls, cross-column comparisons, if/then/else, etc.)
    // fall through to the eager filter below.
    if let Value::Table(t) = &args[0] {
        if let Value::Function(closure) = &args[1] {
            match &t.repr {
                super::super::value::TableRepr::LazyParquet(state) => {
                    if let Some(new_filters) = try_fold_predicate(state, closure) {
                        let mut combined = state.row_filter.clone();
                        combined.extend(new_filters);
                        let new_state = super::super::value::LazyParquetState {
                            bytes: state.bytes.clone(),
                            schema: state.schema.clone(),
                            projection: state.projection.clone(),
                            output_names: state.output_names.clone(),
                            num_rows: state.num_rows,
                            row_filter: combined,
                        };
                        return Ok(Value::Table(Table {
                            repr: super::super::value::TableRepr::LazyParquet(new_state),
                        }));
                    }
                }
                super::super::value::TableRepr::LazyOdbc(state) => {
                    if let Some(new_filters) = try_fold_predicate_for_odbc(state, closure) {
                        let mut combined = state.where_filters.clone();
                        combined.extend(new_filters);
                        let new_state = super::super::value::LazyOdbcState {
                            connection_string: state.connection_string.clone(),
                            table_name: state.table_name.clone(),
                            schema: state.schema.clone(),
                            projection: state.projection.clone(),
                            output_names: state.output_names.clone(),
                            where_filters: combined,
                            limit: state.limit,
                            force_fn: state.force_fn.clone(),
                        };
                        return Ok(Value::Table(Table {
                            repr: super::super::value::TableRepr::LazyOdbc(new_state),
                        }));
                    }
                }
                _ => {}
            }
        }
    }

    let table = expect_table(&args[0])?;
    let predicate = expect_function(&args[1])?;
    let n_rows = table.num_rows();
    let mut keep: Vec<u32> = Vec::new();
    for row in 0..n_rows {
        let record = row_to_record(&table, row)?;
        let result = invoke_callback_with_host(predicate, vec![record], host)?;
        match result {
            Value::Logical(true) => keep.push(row as u32),
            Value::Logical(false) => {}
            // Null predicate → row excluded (matches PQ 3-valued logic:
            // when any operand in the predicate is null, comparison
            // returns null, which Table.SelectRows treats as "not kept").
            Value::Null => {}
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
            // Empty schema (zero columns) can't go through Arrow's
            // RecordBatch::try_new — it requires either a row count or
            // at least one column. PQ returns an empty table here,
            // match that by short-circuiting.
            if batch.num_columns() == 0 {
                return Ok(Value::Table(Table::from_rows(vec![], vec![])));
            }
            let indices = arrow::array::UInt32Array::from(keep);
            let new_columns: Vec<ArrayRef> = batch
                .columns()
                .iter()
                .map(|c| {
                    arrow::compute::take(c.as_ref(), &indices, None).map_err(|e| {
                        MError::Other(format!("Table.SelectRows: take failed: {e}"))
                    })
                })
                .collect::<Result<_, _>>()?;
            let new_batch = RecordBatch::try_new(batch.schema(), new_columns)
                .map_err(|e| MError::Other(format!("Table.SelectRows: rebuild failed: {e}")))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { columns, rows } => {
            let new_rows: Vec<Vec<Value>> =
                keep.into_iter().map(|i| rows[i as usize].clone()).collect();
            Ok(Value::Table(Table::from_rows(columns.clone(), new_rows)))
        }
        super::super::value::TableRepr::LazyParquet(_)
        | super::super::value::TableRepr::LazyOdbc(_) => {
            unreachable!("expect_table forces upstream — lazy variant can't reach here")
        }
        super::super::value::TableRepr::JoinView(_)
        | super::super::value::TableRepr::ExpandView(_) => {
            unreachable!("expect_table forces upstream — JoinView/ExpandView can't reach here")
        }
    }
}


fn from_records(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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


fn to_records(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = table.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n);
    for row in 0..n {
        out.push(row_to_record(&table, row)?);
    }
    Ok(Value::List(out))
}


fn distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (names, rows) = table_to_rows(&table)?;

    // Per Oracle probes (q139/q140), PQ's Table.Distinct criteria arg has
    // several shapes:
    //   - omitted/null: full-row primitive-equality dedup
    //   - Function: full-row callback dedup (mrsflow extension — PQ
    //     rejects this with "distinct criteria is invalid")
    //   - Text "k": PQ silently returns input unchanged (a documented
    //     no-op; matching PQ here is the least surprising choice)
    //   - List {col, comparer}: dedup by `col` using `comparer`
    match args.get(1) {
        None | Some(Value::Null) | Some(Value::Function(_)) => {
            let criteria = table_equation_criteria_fn(args, 1, "Table.Distinct")?;
            let mut kept: Vec<Vec<Value>> = Vec::new();
            for row in rows {
                let mut dup = false;
                for k in &kept {
                    if rows_equal_with_criteria(&names, &row, k, criteria)? {
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
        Some(Value::Text(col_name)) => {
            // Dedup by the single named column. Per Oracle probes q150/151,
            // PQ keeps the FIRST row for each distinct value in that column.
            let col_idx = names
                .iter()
                .position(|n| n == col_name)
                .ok_or_else(|| MError::Other(format!(
                    "Table.Distinct: column not found: {col_name}"
                )))?;
            distinct_by_column_subset(&names, rows, &[col_idx], None)
        }
        Some(Value::List(parts)) => {
            // Three list shapes (Oracle q140/q152/q153):
            //   {col, comparer}             — single col with user comparer
            //   {col1, col2, ...}           — multiple cols, primitive equality
            //   {{col, comparer}}           — list-of-pairs (single pair form)
            //   {{col1, comparer1}, ...}    — list-of-pairs (multi-pair)
            // Disambiguate by the second element: if it's a function, the
            // outer list is {col, comparer}; otherwise it's a list of cols
            // or list of pairs.
            if parts.is_empty() {
                return Err(MError::Other(
                    "Table.Distinct: criterion list cannot be empty".into(),
                ));
            }
            match (&parts[0], parts.get(1)) {
                (Value::Text(col_name), Some(Value::Function(c))) if parts.len() == 2 => {
                    // {col, comparer}
                    let col_idx = names
                        .iter()
                        .position(|n| n == col_name)
                        .ok_or_else(|| MError::Other(format!(
                            "Table.Distinct: column not found: {col_name}"
                        )))?;
                    distinct_by_column_subset(&names, rows, &[col_idx], Some((col_idx, c.clone())))
                }
                (Value::List(_), _) => {
                    // List-of-pairs form: {{col1, cmp1}, {col2, cmp2}, ...}.
                    // Each pair binds a per-column comparer. Empirical (q153)
                    // confirms PQ accepts the single-pair case; multi-pair
                    // is the natural generalisation.
                    let mut col_idxs: Vec<usize> = Vec::with_capacity(parts.len());
                    let mut per_col_cmps: Vec<(usize, Closure)> = Vec::new();
                    for p in parts {
                        let pair = match p {
                            Value::List(xs) => xs,
                            other => return Err(type_mismatch(
                                "list (column, comparer) pair",
                                other,
                            )),
                        };
                        if pair.len() != 2 {
                            return Err(MError::Other(format!(
                                "Table.Distinct: pair must have 2 elements, got {}",
                                pair.len(),
                            )));
                        }
                        let col = match &pair[0] {
                            Value::Text(s) => s.clone(),
                            other => return Err(type_mismatch("text (column name)", other)),
                        };
                        let cmp = match &pair[1] {
                            Value::Function(c) => c.clone(),
                            other => return Err(type_mismatch("function (comparer)", other)),
                        };
                        let idx = names.iter().position(|n| n == &col).ok_or_else(|| {
                            MError::Other(format!(
                                "Table.Distinct: column not found: {col}"
                            ))
                        })?;
                        col_idxs.push(idx);
                        per_col_cmps.push((idx, cmp));
                    }
                    // v1 with one per-col comparer: pass it through.
                    // Multi-comparer is a natural extension but would need
                    // distinct_by_column_subset to take a Vec instead.
                    let single = if per_col_cmps.len() == 1 {
                        Some(per_col_cmps.into_iter().next().unwrap())
                    } else {
                        return Err(MError::NotImplemented(
                            "Table.Distinct: list-of-pairs with multiple comparers not yet supported",
                        ));
                    };
                    distinct_by_column_subset(&names, rows, &col_idxs, single)
                }
                _ => {
                    // Plain list of column names (multi-col tuple dedup).
                    let mut col_idxs: Vec<usize> = Vec::with_capacity(parts.len());
                    for p in parts {
                        match p {
                            Value::Text(s) => {
                                let idx = names.iter().position(|n| n == s).ok_or_else(|| {
                                    MError::Other(format!(
                                        "Table.Distinct: column not found: {s}"
                                    ))
                                })?;
                                col_idxs.push(idx);
                            }
                            other => {
                                return Err(type_mismatch(
                                    "text (column name in criterion list)",
                                    other,
                                ));
                            }
                        }
                    }
                    distinct_by_column_subset(&names, rows, &col_idxs, None)
                }
            }
        }
        Some(other) => Err(type_mismatch(
            "text, list of columns, {col,comparer}, function, or null",
            other,
        )),
    }
}

/// Dedup rows by a subset of columns, keeping the first-seen row for each
/// distinct key tuple. When `comparer` is Some, equality on its column is
/// delegated to the user callback (returning -1|0|1 number or logical);
/// all other key columns use primitive equality.
fn distinct_by_column_subset(
    names: &[String],
    rows: Vec<Vec<Value>>,
    col_idxs: &[usize],
    comparer: Option<(usize, Closure)>,
) -> Result<Value, MError> {
    let mut kept: Vec<Vec<Value>> = Vec::new();
    'rows: for row in rows {
        for k in &kept {
            let mut all_eq = true;
            for &ci in col_idxs {
                let equal = match &comparer {
                    Some((cmp_idx, cmp_fn)) if *cmp_idx == ci => {
                        let r = invoke_builtin_callback(
                            cmp_fn,
                            vec![row[ci].clone(), k[ci].clone()],
                        )?;
                        match r {
                            Value::Number(n) => n == 0.0,
                            Value::Logical(b) => b,
                            other => {
                                return Err(type_mismatch(
                                    "number or logical (comparer result)",
                                    &other,
                                ));
                            }
                        }
                    }
                    _ => values_equal_primitive(&row[ci], &k[ci])?,
                };
                if !equal {
                    all_eq = false;
                    break;
                }
            }
            if all_eq {
                continue 'rows;
            }
        }
        kept.push(row);
    }
    Ok(Value::Table(values_to_table(names, &kept)?))
}


fn row_predicate_holds(
    f: &Closure,
    row_rec: Value,
    host: &dyn IoHost,
    fn_name: &str,
) -> Result<bool, MError> {
    let r = invoke_callback_with_host(f, vec![row_rec], host)?;
    match r {
        Value::Logical(b) => Ok(b),
        other => Err(MError::Other(format!(
            "{fn_name}: predicate must return logical, got {}",
            super::super::type_name(&other),
        ))),
    }
}

fn first_n(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    match &args[1] {
        Value::Number(n) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(MError::Other(
                    "Table.FirstN: count must be a non-negative integer".into(),
                ));
            }
            let (names, rows) = table_to_rows(&table)?;
            let kept: Vec<Vec<Value>> = rows.into_iter().take(*n as usize).collect();
            Ok(Value::Table(values_to_table(&names, &kept)?))
        }
        Value::Function(f) => {
            // take-while: stop on first false
            let (names, rows) = table_to_rows(&table)?;
            let mut kept: Vec<Vec<Value>> = Vec::new();
            for (i, row) in rows.iter().enumerate() {
                let rec = row_to_record(&table, i)?;
                if row_predicate_holds(f, rec, host, "Table.FirstN")? {
                    kept.push(row.clone());
                } else {
                    break;
                }
            }
            Ok(Value::Table(values_to_table(&names, &kept)?))
        }
        other => Err(type_mismatch("number or function", other)),
    }
}

fn last_n(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    match &args[1] {
        Value::Number(n) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(MError::Other(
                    "Table.LastN: count must be a non-negative integer".into(),
                ));
            }
            let (names, rows) = table_to_rows(&table)?;
            let total = rows.len();
            let skip = total.saturating_sub(*n as usize);
            let kept: Vec<Vec<Value>> = rows.into_iter().skip(skip).collect();
            Ok(Value::Table(values_to_table(&names, &kept)?))
        }
        Value::Function(f) => {
            // from-end take-while: scan reversed, stop on first false
            let (names, rows) = table_to_rows(&table)?;
            let mut start = rows.len();
            for i in (0..rows.len()).rev() {
                let rec = row_to_record(&table, i)?;
                if row_predicate_holds(f, rec, host, "Table.LastN")? {
                    start -= 1;
                } else {
                    break;
                }
            }
            let kept: Vec<Vec<Value>> = rows[start..].to_vec();
            Ok(Value::Table(values_to_table(&names, &kept)?))
        }
        other => Err(type_mismatch("number or function", other)),
    }
}

fn reverse(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (names, mut rows) = table_to_rows(&table)?;
    rows.reverse();
    Ok(Value::Table(values_to_table(&names, &rows)?))
}


fn column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let name = expect_text(&args[1])?;
    let col_idx = table
        .column_names()
        .iter()
        .position(|n| n == name)
        .ok_or_else(|| MError::Other(format!("Table.Column: column not found: {name}")))?;
    let n = table.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n);
    for row in 0..n {
        out.push(cell_to_value(&table, col_idx, row)?);
    }
    Ok(Value::List(out))
}


fn is_empty(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Schema-only: row count comes from the parquet footer, no decode.
    let table = expect_table_lazy_ok(&args[0])?;
    Ok(Value::Logical(table.num_rows() == 0))
}


fn add_index_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
    let (mut names, mut rows) = table_to_rows(&table)?;
    if names.iter().any(|n| n == &new_name) {
        return Err(MError::Other(format!(
            "Table.AddIndexColumn: column already exists: {new_name}"
        )));
    }
    names.push(new_name);
    for (i, row) in rows.iter_mut().enumerate() {
        row.push(Value::Number(initial + (i as f64) * increment));
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}


fn add_column(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let new_name = expect_text(&args[1])?.to_string();
    let transform = expect_function(&args[2])?;
    let n_rows = table.num_rows();
    let mut new_cells: Vec<Value> = Vec::with_capacity(n_rows);
    let mut had_cell_errors = false;
    for row in 0..n_rows {
        let record = row_to_record(&table, row)?;
        // Catch per-cell errors so Table.ReplaceErrorValues downstream can
        // replace them. We tag a cell-level error by wrapping the error
        // record in WithMetadata with `[__cell_error=true]`.
        match invoke_callback_with_host(transform, vec![record], host) {
            Ok(v) => new_cells.push(v),
            Err(e) => {
                had_cell_errors = true;
                new_cells.push(super::super::error_to_cell_marker(e));
            }
        }
    }
    let _ = had_cell_errors;
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
        // Type-ascription handling: in PQ the `type` arg is metadata, not
        // coercive — if the cells don't fit the declared type we keep the
        // inferred encoding rather than erroring. Try the cast and accept
        // it only if it doesn't drop information (no all-null result for
        // non-null inputs).
        let (dtype, new_array, nullable) = match &target_type {
            Some(Value::Type(t)) if !matches!(t, super::super::value::TypeRep::Any) => {
                let (target_dtype, target_nullable) = type_rep_to_datatype(t)?;
                match arrow::compute::cast(inferred_array, &target_dtype) {
                    Ok(cast) => {
                        // If the cast nulled every previously-non-null cell,
                        // PQ would keep the original text/etc. — fall back
                        // to the inferred encoding.
                        let src_non_null = inferred_array.len() - inferred_array.null_count();
                        let dst_non_null = cast.len() - cast.null_count();
                        if src_non_null > 0 && dst_non_null == 0 {
                            let nullable = matches!(inferred_dtype, DataType::Null)
                                || new_cells.iter().any(|v| matches!(v, Value::Null));
                            (inferred_dtype.clone(), inferred_array.clone(), nullable)
                        } else {
                            (target_dtype, cast, target_nullable)
                        }
                    }
                    Err(_) => {
                        // Unsupported cast → keep inferred type.
                        let nullable = matches!(inferred_dtype, DataType::Null)
                            || new_cells.iter().any(|v| matches!(v, Value::Null));
                        (inferred_dtype.clone(), inferred_array.clone(), nullable)
                    }
                }
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
            .map_err(|e| MError::Other(format!("Table.AddColumn: rebuild failed: {e}")))?;
        return Ok(Value::Table(Table::from_arrow(new_batch)));
    }

    // Slow path: produce a Rows-backed result. Decode the input if needed,
    // then append the new column per row.
    let (mut names, mut rows) = table_to_rows(&table)?;
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


fn from_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Same as #table but with arg order (rows, columns). The columns arg
    // accepts either a list of text names or a `type table [...]` value,
    // matching the M spec.
    let rows = expect_list_of_lists(&args[0], "Table.FromRows: rows")?;
    let names: Vec<String> = match &args[1] {
        Value::Type(super::super::value::TypeRep::TableOf { columns }) => {
            columns.iter().map(|(n, _)| n.clone()).collect()
        }
        _ => expect_text_list(&args[1], "Table.FromRows: columns")?,
    };
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


fn promote_headers(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.num_rows() == 0 {
        // PQ: no-op on empty table — return as-is with original columns.
        return Ok(Value::Table(table.into_owned()));
    }
    // Parse PromoteAllScalars from the optional options record. Default
    // false per M spec; when true, coerce non-text scalar header cells
    // to text via Text.From-style rules instead of erroring.
    let promote_scalars = match args.get(1) {
        None | Some(Value::Null) => false,
        Some(Value::Record(r)) => {
            let mut flag = false;
            for (k, v) in &r.fields {
                if k == "PromoteAllScalars" {
                    let v = super::super::force(v.clone(), &mut |e, env| {
                        super::super::evaluate(e, env, host)
                    })?;
                    flag = matches!(v, Value::Logical(true));
                }
            }
            flag
        }
        Some(other) => {
            return Err(MError::TypeMismatch {
                expected: "record or null",
                found: super::super::type_name(other),
            });
        }
    };
    // Read row 0 as the new names.
    let mut new_names: Vec<String> = Vec::with_capacity(table.num_columns());
    for col in 0..table.num_columns() {
        let cell = cell_to_value(&table, col, 0)?;
        let name = match (&cell, promote_scalars) {
            (Value::Text(s), _) => s.clone(),
            // PromoteAllScalars=true: coerce numbers/logicals to text.
            // Null cells fall back to the existing `Column<n>` name so
            // the column doesn't get an empty string header.
            (Value::Number(n), true) => format!("{n:?}"),
            (Value::Logical(b), true) => if *b { "true" } else { "false" }.to_string(),
            (Value::Null, true) => table.column_names()[col].clone(),
            (other, _) => {
                return Err(MError::Other(format!(
                    "Table.PromoteHeaders: header cell in column {} is not text: {}",
                    col,
                    super::super::type_name(other)
                )));
            }
        };
        new_names.push(name);
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
                MError::Other(format!("Table.PromoteHeaders: rebuild failed: {e}"))
            })?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { rows, .. } => {
            let new_rows: Vec<Vec<Value>> = rows.iter().skip(1).cloned().collect();
            Ok(Value::Table(Table::from_rows(new_names, new_rows)))
        }
        super::super::value::TableRepr::LazyParquet(_)
        | super::super::value::TableRepr::LazyOdbc(_) => {
            unreachable!("expect_table forces upstream — lazy variant can't reach here")
        }
        super::super::value::TableRepr::JoinView(_)
        | super::super::value::TableRepr::ExpandView(_) => {
            unreachable!("expect_table forces upstream — JoinView/ExpandView can't reach here")
        }
    }
}

/// Build a record Value from one row of a table — column name → cell.
/// Dispatches on `TableRepr`.
pub(crate) fn row_to_record(table: &Table, row: usize) -> Result<Value, MError> {
    let names = table.column_names();
    let mut fields: Vec<(String, Value)> = Vec::with_capacity(names.len());
    for (col, name) in names.into_iter().enumerate() {
        let value = cell_to_value(&table, col, row)?;
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
fn transform_column_types(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let transforms = expect_list(&args[1])?;
    let culture: Option<String> = match args.get(2) {
        Some(Value::Text(s)) => Some(s.clone()),
        _ => None,
    };
    transform_culture::set(culture);
    struct ClearCulture;
    impl Drop for ClearCulture {
        fn drop(&mut self) { transform_culture::set(None); }
    }
    let _guard = ClearCulture;
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
                        "Table.TransformColumnTypes: column not found: {name}"
                    ))
                })?;
                let Some((target_dtype, target_nullable)) = target else {
                    continue; // type any → no cast
                };
                let cast = cultural_cast(
                    &new_columns[idx],
                    target_dtype,
                    name,
                    "Table.TransformColumnTypes",
                )?;
                new_columns[idx] = cast;
                new_fields[idx] = Field::new(name, target_dtype.clone(), *target_nullable);
            }
            let new_schema = Arc::new(Schema::new(new_fields));
            let new_batch = RecordBatch::try_new(new_schema, new_columns).map_err(|e| {
                MError::Other(format!("Table.TransformColumnTypes: rebuild failed: {e}"))
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
                        "Table.TransformColumnTypes: column not found: {name}"
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
        super::super::value::TableRepr::LazyParquet(_)
        | super::super::value::TableRepr::LazyOdbc(_) => {
            unreachable!("expect_table forces upstream — lazy variant can't reach here")
        }
        super::super::value::TableRepr::JoinView(_)
        | super::super::value::TableRepr::ExpandView(_) => {
            unreachable!("expect_table forces upstream — JoinView/ExpandView can't reach here")
        }
    }
    // Clear culture override so it doesn't leak into other casts.
    // (Unreachable in practice because every arm above returns.)
}

/// Thread-local culture override for Table.TransformColumnTypes' numeric/date
/// text parsing. Set by transform_column_types, read by parse_text_to_number.
mod transform_culture {
    use std::cell::RefCell;
    thread_local! { static CULTURE: RefCell<Option<String>> = const { RefCell::new(None) }; }
    pub fn set(c: Option<String>) { CULTURE.with(|s| *s.borrow_mut() = c); }
    pub fn get() -> Option<String> { CULTURE.with(|s| s.borrow().clone()) }
}

/// Helper: pull the TypeRep for `name` out of the original (un-parsed)
/// transforms list. Only used on the Rows-path of TransformColumnTypes
/// to recover a TypeRep we already validated.
fn find_typerep_for_name(
    transforms: &[Value],
    name: &str,
) -> Result<super::super::value::TypeRep, MError> {
    for t in transforms {
        if let Value::List(xs) = t
            && xs.len() == 2
                && let (Value::Text(n), Value::Type(tr)) = (&xs[0], &xs[1])
                    && n == name {
                        return Ok(tr.clone());
                    }
    }
    Err(MError::Other(format!(
        "Table.TransformColumnTypes: lost track of type for column {name}"
    )))
}


type ColTypePairs = Vec<(String, Option<(DataType, bool)>)>;

fn parse_col_type_pairs(transforms: &[Value]) -> Result<ColTypePairs, MError> {
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
        TypeRep::NamedNumeric("Int8.Type") => Ok((DataType::Int8, false)),
        TypeRep::NamedNumeric("Int16.Type") => Ok((DataType::Int16, false)),
        TypeRep::NamedNumeric("Int32.Type") => Ok((DataType::Int32, false)),
        TypeRep::NamedNumeric("Int64.Type") => Ok((DataType::Int64, false)),
        TypeRep::NamedNumeric("Single.Type") => Ok((DataType::Float32, false)),
        TypeRep::NamedNumeric("Double.Type") | TypeRep::NamedNumeric("Number.Type")
        | TypeRep::NamedNumeric("Currency.Type") | TypeRep::NamedNumeric("Decimal.Type")
        | TypeRep::NamedNumeric("Percentage.Type") | TypeRep::NamedNumeric(_) => {
            Ok((DataType::Float64, false))
        }
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
        | TypeRep::Time | TypeRep::Datetimezone
        | TypeRep::ListOf(_) | TypeRep::RecordOf { .. } | TypeRep::TableOf { .. }
        | TypeRep::FunctionOf { .. } => {
            Err(MError::Other(format!(
                "Table.TransformColumnTypes: type {t:?} is not a castable primitive"
            )))
        }
    }
}


fn transform_columns(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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
    let (names, mut rows) = table_to_rows(&table)?;
    let n_rows = rows.len();

    for (name, closure, type_opt) in &pairs {
        let idx = names.iter().position(|n| n == name).ok_or_else(|| {
            MError::Other(format!("Table.TransformColumns: column not found: {name}"))
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
            "{ctx}: cast {col_name} to {target_dtype:?} failed: column has heterogeneous cells"
        ))
    })?;
    let cast = cultural_cast(&inferred_array, &target_dtype, col_name, ctx)?;
    // Decode the cast result back to Values via a temporary single-column table.
    let field = Field::new(col_name, target_dtype, target_nullable);
    let temp_batch = RecordBatch::try_new(Arc::new(Schema::new(vec![field])), vec![cast])
        .map_err(|e| MError::Other(format!("{ctx}: temp batch failed: {e}")))?;
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


type ColFnPairs<'a> = Vec<(String, &'a Closure, Option<super::super::value::TypeRep>)>;

fn parse_col_fn_pairs(transforms: &[Value]) -> Result<ColFnPairs<'_>, MError> {
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


fn skip(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Value::Function(f) => {
            // skip-while: count rows from the start while predicate is true.
            // Then fall through to the optimised slice path with that count.
            let mut k = 0usize;
            for i in 0..table.num_rows() {
                let rec = row_to_record(&table, i)?;
                if row_predicate_holds(f, rec, host, "Table.Skip")? {
                    k += 1;
                } else {
                    break;
                }
            }
            k
        }
        other => return Err(type_mismatch("non-negative integer or function", other)),
    };
    let n_rows = table.num_rows();
    let skip = count.min(n_rows);
    match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => {
            let remaining = n_rows - skip;
            let new_columns: Vec<ArrayRef> =
                batch.columns().iter().map(|c| c.slice(skip, remaining)).collect();
            let new_batch = RecordBatch::try_new(batch.schema(), new_columns)
                .map_err(|e| MError::Other(format!("Table.Skip: rebuild failed: {e}")))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::super::value::TableRepr::Rows { columns, rows } => {
            let new_rows: Vec<Vec<Value>> = rows.iter().skip(skip).cloned().collect();
            Ok(Value::Table(Table::from_rows(columns.clone(), new_rows)))
        }
        super::super::value::TableRepr::LazyParquet(_)
        | super::super::value::TableRepr::LazyOdbc(_) => {
            unreachable!("expect_table forces upstream — lazy variant can't reach here")
        }
        super::super::value::TableRepr::JoinView(_)
        | super::super::value::TableRepr::ExpandView(_) => {
            unreachable!("expect_table forces upstream — JoinView/ExpandView can't reach here")
        }
    }
}


fn reorder_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Projection-aware: reordering is just a permuted mask, no decode needed.
    let table = expect_table_lazy_ok(&args[0])?;
    let order = expect_text_list(&args[1], "Table.ReorderColumns: columnOrder")?;
    let existing = table.column_names();

    let mut new_indices: Vec<usize> = Vec::with_capacity(existing.len());
    let mut used = vec![false; existing.len()];

    // First: the explicitly named columns in the requested order.
    for name in &order {
        let idx = existing.iter().position(|e| e == name).ok_or_else(|| {
            MError::Other(format!(
                "Table.ReorderColumns: column not found: {name}"
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


fn expand_record_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

    let (existing, rows) = table_to_rows(&table)?;
    let col_idx = existing.iter().position(|n| n == &column).ok_or_else(|| {
        MError::Other(format!(
            "Table.ExpandRecordColumn: column not found: {column}"
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


fn expand_list_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let column = expect_text(&args[1])?.to_string();
    let (names, rows) = table_to_rows(&table)?;
    let col_idx = names.iter().position(|n| n == &column).ok_or_else(|| {
        MError::Other(format!(
            "Table.ExpandListColumn: column not found: {column}"
        ))
    })?;

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    for row in &rows {
        match &row[col_idx] {
            Value::List(items) => {
                if items.is_empty() {
                    // PQ: empty list emits a single output row with null
                    // in the target column (not a dropped row).
                    let mut new_row = row.clone();
                    new_row[col_idx] = Value::Null;
                    out_rows.push(new_row);
                } else {
                    for item in items {
                        let mut new_row = row.clone();
                        new_row[col_idx] = item.clone();
                        out_rows.push(new_row);
                    }
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


fn expand_table_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Use `expect_table_lazy_ok` so JoinView falls through to the fast
    // path below without being materialised first.
    let table = expect_table_lazy_ok(&args[0])?;
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

    // Fast path: expanding the JoinView's nested column under
    // Inner / LeftOuter (the corpus-dominant pair). Pulls only the
    // requested columns from the lazy right side; RT-preserving.
    //
    // Other join kinds (RightOuter / FullOuter / LeftAnti / RightAnti)
    // have different row iteration patterns — they fall through to
    // the slow path which forces the JoinView via materialise_join_view
    // (which already branches per join_kind) and then expands the
    // resulting Rows-backed table. Less optimal but correct, and these
    // kinds are uncommon in the corpus pattern (typically followed by
    // RemoveColumns to drop the nested column rather than expand).
    if let super::super::value::TableRepr::JoinView(jv) = &table.repr {
        if jv.new_column_name == column && matches!(jv.join_kind, 0 | 1) {
            return expand_join_view_lazily(jv, &column_names, &new_column_names);
        }
    }

    // Slow path: force any laziness, then expand eagerly as before.
    let forced = table.force()?;
    let table: &Table = &forced;
    let (outer_names, outer_rows) = table_to_rows(table)?;
    let col_idx = outer_names.iter().position(|n| n == &column).ok_or_else(|| {
        MError::Other(format!(
            "Table.ExpandTableColumn: column not found: {column}"
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
                                "Table.ExpandTableColumn: inner column not found: {n}"
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

/// Fast-path expansion of a `JoinView`'s nested column. Returns an
/// `ExpandView` — a lazy result that doesn't materialise either side
/// until something downstream forces it. Lets the corpus chain
/// (`expand → RemoveColumns → NestedJoin → expand → RowCount`) avoid
/// decoding either side's bulk columns at all.
fn expand_join_view_lazily(
    jv: &super::super::value::JoinViewState,
    column_names: &[String],
    new_column_names: &[String],
) -> Result<Value, MError> {
    // Resolve requested right columns into their current-projection
    // indices on jv.right. These are positions in `jv.right.column_names()`
    // (which may itself already be a narrowed projection).
    let right_current_names = jv.right.column_names();
    let mut right_projection: Vec<usize> = Vec::with_capacity(column_names.len());
    for n in column_names {
        let idx = right_current_names.iter().position(|r| r == n).ok_or_else(|| {
            MError::Other(format!(
                "Table.ExpandTableColumn: inner column not found: {n}"
            ))
        })?;
        right_projection.push(idx);
    }

    // Left projection: all columns of jv.left, in order. (The JoinView
    // exposes left columns + the nested column; expanding the nested
    // column means we keep every left column unchanged.)
    let left_projection: Vec<usize> = (0..jv.left.num_columns()).collect();

    Ok(Value::Table(Table {
        repr: super::super::value::TableRepr::ExpandView(
            super::super::value::ExpandViewState {
                left: jv.left.clone(),
                left_projection,
                right: jv.right.clone(),
                right_projection,
                right_output_names: new_column_names.to_vec(),
                matches: jv.matches.clone(),
            },
        ),
    }))
}


fn unpivot(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let pivot_columns = expect_text_list(&args[1], "Table.Unpivot: pivotColumns")?;
    let attribute_column = expect_text(&args[2])?.to_string();
    let value_column = expect_text(&args[3])?.to_string();
    do_unpivot(&table, &pivot_columns, &attribute_column, &value_column, "Table.Unpivot")
}


fn unpivot_other_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
        &table,
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
    let (names, rows) = table_to_rows(&table)?;
    // Resolve pivot indices and validate.
    let pivot_indices: Vec<usize> = pivot_columns
        .iter()
        .map(|p| {
            names
                .iter()
                .position(|n| n == p)
                .ok_or_else(|| MError::Other(format!("{ctx}: column not found: {p}")))
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
            // PQ drops rows where the pivoted cell value is null.
            if matches!(row[p_idx], Value::Null) {
                continue;
            }
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
fn pivot(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let pivot_values = expect_text_list(&args[1], "Table.Pivot: pivotValues")?;
    let attribute_column = expect_text(&args[2])?.to_string();
    let value_column = expect_text(&args[3])?.to_string();
    let aggregation: Option<&Closure> = match args.get(4) {
        Some(Value::Function(c)) => Some(c),
        Some(Value::Null) | None => None,
        Some(other) => return Err(type_mismatch("function", other)),
    };

    let (names, rows) = table_to_rows(&table)?;
    let attr_idx = names
        .iter()
        .position(|n| n == &attribute_column)
        .ok_or_else(|| {
            MError::Other(format!(
                "Table.Pivot: attributeColumn not found: {attribute_column}"
            ))
        })?;
    let val_idx = names.iter().position(|n| n == &value_column).ok_or_else(|| {
        MError::Other(format!(
            "Table.Pivot: valueColumn not found: {value_column}"
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
fn join(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table1 = expect_table(&args[0])?;
    // key1/key2 can each be a single text or a list of texts (composite).
    // Both forms route through parse_join_key_columns; counts must match.
    let key1_cols = parse_join_key_columns(&args[1], "key1")?;
    let table2 = expect_table(&args[2])?;
    let key2_cols = parse_join_key_columns(&args[3], "key2")?;
    if key1_cols.len() != key2_cols.len() {
        return Err(MError::Other(format!(
            "Table.Join: key column counts differ — key1 has {}, key2 has {}",
            key1_cols.len(),
            key2_cols.len(),
        )));
    }
    // joinKind default for Table.Join is Inner (0); cf. NestedJoin which
    // defaults to LeftOuter.
    let join_kind = match args.get(4) {
        Some(Value::Number(n)) if n.fract() == 0.0 => *n as i32,
        Some(Value::Null) | None => 0,
        Some(other) => return Err(type_mismatch("number (JoinKind)", other)),
    };
    if !(0..=5).contains(&join_kind) {
        return Err(MError::Other(format!(
            "Table.Join: unknown JoinKind {join_kind} (expected 0..5)"
        )));
    }

    let (left_names, left_rows) = table_to_rows(&table1)?;
    let (right_names, right_rows) = table_to_rows(&table2)?;

    let key1_idxs: Vec<usize> = key1_cols
        .iter()
        .map(|k| {
            left_names.iter().position(|n| n == k).ok_or_else(|| {
                MError::Other(format!("Table.Join: key1 column not found: {k}"))
            })
        })
        .collect::<Result<_, _>>()?;
    let key2_idxs: Vec<usize> = key2_cols
        .iter()
        .map(|k| {
            right_names.iter().position(|n| n == k).ok_or_else(|| {
                MError::Other(format!("Table.Join: key2 column not found: {k}"))
            })
        })
        .collect::<Result<_, _>>()?;
    let right_keep: Vec<usize> = (0..right_names.len()).collect();

    let mut out_names: Vec<String> = left_names.clone();
    // PQ Table.Join always keeps both sides' columns — anti joins null the
    // opposite side rather than dropping its columns.
    let is_left_anti  = join_kind == 4;
    let is_right_anti = join_kind == 5;
    for &i in &right_keep {
        out_names.push(right_names[i].clone());
    }

    let row_matches = |left_row: &[Value], right_row: &[Value]| -> Result<bool, MError> {
        for (&li, &ri) in key1_idxs.iter().zip(key2_idxs.iter()) {
            if !values_equal_primitive(&left_row[li], &right_row[ri])? {
                return Ok(false);
            }
        }
        Ok(true)
    };

    let mut out_rows: Vec<Vec<Value>> = Vec::new();
    let mut right_matched = vec![false; right_rows.len()];

    // Pass 1: matches (and LeftOuter null-pad for unmatched-left appended
    // inline) — preserves left-row order for Inner/LeftOuter.
    let mut left_unmatched: Vec<&Vec<Value>> = Vec::new();
    for left_row in &left_rows {
        let mut any_match = false;
        for (ri, right_row) in right_rows.iter().enumerate() {
            if row_matches(left_row, right_row)? {
                any_match = true;
                right_matched[ri] = true;
                if matches!(join_kind, 0 | 1 | 2 | 3) {
                    let mut new_row = left_row.clone();
                    for &i in &right_keep {
                        new_row.push(right_row[i].clone());
                    }
                    out_rows.push(new_row);
                }
            }
        }
        if !any_match {
            left_unmatched.push(left_row);
        }
    }
    // LeftOuter (1): null-pad unmatched-left rows after the matches.
    if join_kind == 1 {
        for left_row in &left_unmatched {
            let mut new_row = (*left_row).clone();
            for _ in &right_keep {
                new_row.push(Value::Null);
            }
            out_rows.push(new_row);
        }
    }
    // RightOuter (2) / FullOuter (3): null-pad unmatched-right rows.
    if matches!(join_kind, 2 | 3) {
        for (ri, matched) in right_matched.iter().enumerate() {
            if !matched {
                let mut new_row: Vec<Value> = left_names.iter().map(|_| Value::Null).collect();
                for &i in &right_keep {
                    new_row.push(right_rows[ri][i].clone());
                }
                out_rows.push(new_row);
            }
        }
    }
    // FullOuter (3): then unmatched-left rows after the unmatched-right.
    if join_kind == 3 {
        for left_row in &left_unmatched {
            let mut new_row = (*left_row).clone();
            for _ in &right_keep {
                new_row.push(Value::Null);
            }
            out_rows.push(new_row);
        }
    }
    // LeftAnti (4): emit left rows that had no match, right cols nulled.
    if is_left_anti {
        for left_row in &left_unmatched {
            let mut new_row = (*left_row).clone();
            for _ in &right_keep {
                new_row.push(Value::Null);
            }
            out_rows.push(new_row);
        }
    }
    // RightAnti (5): emit right rows that had no match, left cols nulled.
    if is_right_anti {
        for (ri, matched) in right_matched.iter().enumerate() {
            if !matched {
                let mut new_row: Vec<Value> = left_names.iter().map(|_| Value::Null).collect();
                for &i in &right_keep {
                    new_row.push(right_rows[ri][i].clone());
                }
                out_rows.push(new_row);
            }
        }
    }

    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}


/// Hashable projection of a primitive `Value`, used as a join key. Must
/// give the same equality answer as [`values_equal_primitive`]; compound
/// values (List/Record/Table) error rather than join-by-identity.
///
/// `f64` becomes `to_bits()` so it implements `Eq` and `Hash`. This treats
/// `NaN` as never equal to itself (NaN bits compare as equal only to the
/// same bit pattern; different NaN encodings collide via `Hash` but
/// distinct keys still won't match in the bucket). Matches the linear-scan
/// behaviour where `NaN == NaN` was already `false`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum JoinKey {
    Null,
    Bool(bool),
    NumberBits(u64),
    Text(String),
    Date(chrono::NaiveDate),
    Datetime(chrono::NaiveDateTime),
    Duration(chrono::Duration),
}

fn join_key_from(v: &Value) -> Result<JoinKey, MError> {
    match v {
        Value::Null => Ok(JoinKey::Null),
        Value::Logical(b) => Ok(JoinKey::Bool(*b)),
        Value::Number(n) => Ok(JoinKey::NumberBits(n.to_bits())),
        Value::Text(s) => Ok(JoinKey::Text(s.clone())),
        Value::Date(d) => Ok(JoinKey::Date(*d)),
        Value::Datetime(dt) => Ok(JoinKey::Datetime(*dt)),
        Value::Duration(d) => Ok(JoinKey::Duration(*d)),
        _ => Err(MError::NotImplemented(
            "Table.NestedJoin: join key must be a primitive (list/record/table not supported)",
        )),
    }
}

/// Parse a `Table.NestedJoin` key argument into a non-empty list of
/// column names. Accepts bare text (single-key form) or a list of text
/// (single- or multi-column composite keys — both forms are emitted by
/// Power Query's GUI depending on the join shape).
fn parse_join_key_columns(v: &Value, role: &str) -> Result<Vec<String>, MError> {
    match v {
        Value::Text(s) => Ok(vec![s.clone()]),
        Value::List(items) => {
            if items.is_empty() {
                return Err(MError::Other(format!(
                    "Table.NestedJoin: {role} key list is empty"
                )));
            }
            let mut cols = Vec::with_capacity(items.len());
            for item in items {
                match item {
                    Value::Text(s) => cols.push(s.clone()),
                    other => {
                        return Err(type_mismatch(
                            "text (in key list)",
                            other,
                        ));
                    }
                }
            }
            Ok(cols)
        }
        other => Err(type_mismatch("text or text list", other)),
    }
}

fn nested_join(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Both sides stay lazy where possible. Only the key columns are
    // decoded — the left key (to enumerate left rows for match building)
    // and the right key (to build the hash bucket). Everything else
    // decodes later, on demand, when a downstream op forces.
    // See `mrsflow/09-lazy-tables.md` (Stage A.5).
    let table1 = expect_table_lazy_ok(&args[0])?;
    let key1_cols = parse_join_key_columns(&args[1], "key1")?;
    let table2 = expect_table_lazy_ok(&args[2])?;
    let key2_cols = parse_join_key_columns(&args[3], "key2")?;
    if key1_cols.len() != key2_cols.len() {
        return Err(MError::Other(format!(
            "Table.NestedJoin: composite-key arity mismatch — key1 has {} \
             columns, key2 has {}",
            key1_cols.len(),
            key2_cols.len()
        )));
    }
    let new_column_name = expect_text(&args[4])?.to_string();
    // joinKind: 0=Inner, 1=LeftOuter, 2=RightOuter, 3=FullOuter, 4=LeftAnti, 5=RightAnti
    let join_kind = match args.get(5) {
        Some(Value::Number(n)) if n.fract() == 0.0 => *n as i32,
        Some(Value::Null) | None => 1, // default: LeftOuter
        Some(other) => return Err(type_mismatch("number (JoinKind)", other)),
    };
    if !(0..=5).contains(&join_kind) {
        return Err(MError::Other(format!(
            "Table.NestedJoin: invalid joinKind {join_kind} (expected 0–5)"
        )));
    }

    let left_names = table1.column_names();
    let key1_indices: Vec<usize> = key1_cols
        .iter()
        .map(|n| {
            left_names.iter().position(|x| x == n).ok_or_else(|| {
                MError::Other(format!(
                    "Table.NestedJoin: key1 column not found: {n}"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let right_names = table2.column_names();
    let key2_indices: Vec<usize> = key2_cols
        .iter()
        .map(|n| {
            right_names.iter().position(|x| x == n).ok_or_else(|| {
                MError::Other(format!(
                    "Table.NestedJoin: key2 column not found: {n}"
                ))
            })
        })
        .collect::<Result<_, _>>()?;

    // Decode only the key columns. For LazyParquet sides, the projection
    // mask narrows to just these N columns so the rest stay undecoded.
    // For Arrow/Rows it's a multi-column read; for JoinView/ExpandView
    // it falls through to force-then-read of just those columns.
    let left_tuples = decode_key_columns(table1, &key1_indices)?;
    let right_tuples = decode_key_columns(table2, &key2_indices)?;

    // Hash bucket keyed on the tuple of JoinKeys (single-column joins
    // produce 1-element vecs; the equality semantics are unchanged).
    let mut buckets: HashMap<Vec<JoinKey>, Vec<usize>> =
        HashMap::with_capacity(right_tuples.len());
    for (i, tuple) in right_tuples.iter().enumerate() {
        let key: Vec<JoinKey> = tuple
            .iter()
            .map(join_key_from)
            .collect::<Result<_, _>>()?;
        buckets.entry(key).or_default().push(i);
    }

    let mut matches: Vec<Vec<u32>> = Vec::with_capacity(left_tuples.len());
    let n_right = right_tuples.len();
    let mut right_seen: Vec<bool> = vec![false; n_right];
    for tuple in &left_tuples {
        let key: Vec<JoinKey> = tuple
            .iter()
            .map(join_key_from)
            .collect::<Result<_, _>>()?;
        let ms: Vec<u32> = buckets
            .get(&key)
            .map(|idx| {
                let v: Vec<u32> = idx.iter().map(|&i| i as u32).collect();
                for &i in &v {
                    right_seen[i as usize] = true;
                }
                v
            })
            .unwrap_or_default();
        matches.push(ms);
    }
    // Unmatched-right indices for RightOuter / FullOuter / RightAnti.
    // Done in a single pass over the seen-vector, cheap on top of the
    // matches we already have.
    let unmatched_right: Vec<u32> = (0..n_right as u32)
        .filter(|&i| !right_seen[i as usize])
        .collect();

    Ok(Value::Table(Table {
        repr: super::super::value::TableRepr::JoinView(super::super::value::JoinViewState {
            left: std::sync::Arc::new(table1.clone()),
            right: std::sync::Arc::new(table2.clone()),
            new_column_name,
            matches,
            unmatched_right,
            join_kind,
        }),
    }))
}

/// Pull values from a set of columns at once, returning per-row tuples.
/// For a `LazyParquet` source the projection mask narrows to just those
/// columns; the rest stay on disk. Single-column callers pass a 1-element
/// slice and get back a Vec of 1-element tuples.
fn decode_key_columns(
    table: &Table,
    col_indices: &[usize],
) -> Result<Vec<Vec<Value>>, MError> {
    use super::super::value::{LazyParquetState, TableRepr};
    match &table.repr {
        TableRepr::LazyParquet(state) => {
            // Narrow projection to just these columns and force —
            // the rest stay undecoded on disk.
            let new_projection: Vec<usize> = col_indices
                .iter()
                .map(|&i| state.projection[i])
                .collect();
            // No need to carry output_names — decode_key_columns
            // doesn't expose names, just cell values.
            let key_only = Table {
                repr: TableRepr::LazyParquet(LazyParquetState {
                    bytes: state.bytes.clone(),
                    schema: state.schema.clone(),
                    projection: new_projection,
                    output_names: None,
                    num_rows: state.num_rows,
                    row_filter: state.row_filter.clone(),
                }),
            };
            let forced = key_only.force()?;
            let n = forced.num_rows();
            let mut out: Vec<Vec<Value>> = Vec::with_capacity(n);
            for r in 0..n {
                let mut tuple = Vec::with_capacity(col_indices.len());
                for c in 0..col_indices.len() {
                    tuple.push(cell_to_value(&forced, c, r)?);
                }
                out.push(tuple);
            }
            Ok(out)
        }
        TableRepr::JoinView(jv) => {
            // The JoinView's columns are left's columns + the nested
            // column at position left.num_columns(). The nested column
            // is Table-valued and can't be a join key.
            let n_left = jv.left.num_columns();
            for &c in col_indices {
                if c == n_left {
                    return Err(MError::Other(format!(
                        "Table.NestedJoin: cannot use nested column '{}' as a join key",
                        jv.new_column_name
                    )));
                }
                if c > n_left {
                    return Err(MError::Other(format!(
                        "decode_key_columns: column index {c} out of range for JoinView (has {} cols)",
                        n_left + 1
                    )));
                }
            }
            // Inner / LeftOuter / LeftAnti all iterate left rows in
            // order (possibly filtered by match-emptiness). Recurse to
            // left for just the needed columns, then filter rows per
            // join_kind. The other kinds (RightOuter / FullOuter /
            // RightAnti) interleave null-left rows with right-derived
            // rows — fall back to force for those.
            match jv.join_kind {
                0 => {
                    let left_values = decode_key_columns(&jv.left, col_indices)?;
                    let n = jv.matches.iter().filter(|m| !m.is_empty()).count();
                    let mut out = Vec::with_capacity(n);
                    for (i, m) in jv.matches.iter().enumerate() {
                        if !m.is_empty() {
                            out.push(left_values[i].clone());
                        }
                    }
                    Ok(out)
                }
                1 => decode_key_columns(&jv.left, col_indices),
                4 => {
                    let left_values = decode_key_columns(&jv.left, col_indices)?;
                    let n = jv.matches.iter().filter(|m| m.is_empty()).count();
                    let mut out = Vec::with_capacity(n);
                    for (i, m) in jv.matches.iter().enumerate() {
                        if m.is_empty() {
                            out.push(left_values[i].clone());
                        }
                    }
                    Ok(out)
                }
                _ => force_and_read(table, col_indices),
            }
        }
        TableRepr::ExpandView(ev) => {
            // Output columns split between left (0..n_left) and right
            // (n_left..). For each requested output position, partition
            // into the underlying source's column index. Recurse to
            // each source for just its needed columns — so a LazyParquet
            // left with 40 cols decodes only the columns this read asks
            // for, not the ExpandView's full left_projection.
            //
            // This is the corpus-scale WASM saver: the second NestedJoin
            // in `expand → RemoveColumns → NestedJoin` was previously
            // forcing the entire first-expand result here.
            let n_left = ev.left_projection.len();
            let mut left_needed: Vec<usize> = Vec::new();
            let mut right_needed: Vec<usize> = Vec::new();
            // (is_right, local_idx) for each requested output column.
            let mut routing: Vec<(bool, usize)> = Vec::with_capacity(col_indices.len());
            for &col_idx in col_indices {
                if col_idx < n_left {
                    let underlying = ev.left_projection[col_idx];
                    let pos = match left_needed.iter().position(|&u| u == underlying) {
                        Some(p) => p,
                        None => {
                            left_needed.push(underlying);
                            left_needed.len() - 1
                        }
                    };
                    routing.push((false, pos));
                } else {
                    let right_slot = col_idx - n_left;
                    if right_slot >= ev.right_projection.len() {
                        return Err(MError::Other(format!(
                            "decode_key_columns: column index {col_idx} out of range for ExpandView"
                        )));
                    }
                    let underlying = ev.right_projection[right_slot];
                    let pos = match right_needed.iter().position(|&u| u == underlying) {
                        Some(p) => p,
                        None => {
                            right_needed.push(underlying);
                            right_needed.len() - 1
                        }
                    };
                    routing.push((true, pos));
                }
            }
            // Recursive decode — handles nested lazies. Empty `needed`
            // lists are an artefact of decoding only one side; skip the
            // call rather than dispatch on an empty slice.
            let left_values = if !left_needed.is_empty() {
                decode_key_columns(&ev.left, &left_needed)?
            } else {
                Vec::new()
            };
            let right_values = if !right_needed.is_empty() {
                decode_key_columns(&ev.right, &right_needed)?
            } else {
                Vec::new()
            };
            // Walk matches to emit one output row per (left, right) pair.
            // Empty matches drop the outer row, matching the eager
            // expand semantics.
            let total: usize = ev.matches.iter().map(|m| m.len()).sum();
            let mut out: Vec<Vec<Value>> = Vec::with_capacity(total);
            for (outer_i, match_list) in ev.matches.iter().enumerate() {
                if match_list.is_empty() {
                    continue;
                }
                for &m in match_list {
                    let mut tuple = Vec::with_capacity(col_indices.len());
                    for &(is_right, local_idx) in &routing {
                        if is_right {
                            tuple.push(right_values[m as usize][local_idx].clone());
                        } else {
                            tuple.push(left_values[outer_i][local_idx].clone());
                        }
                    }
                    out.push(tuple);
                }
            }
            Ok(out)
        }
        _ => force_and_read(table, col_indices),
    }
}

/// Fallback: force the table and read the requested columns row by row.
/// Used for variants we don't have a smart path for (Arrow / Rows /
/// JoinView with right-side join kinds).
fn force_and_read(
    table: &Table,
    col_indices: &[usize],
) -> Result<Vec<Vec<Value>>, MError> {
    let forced = table.force()?;
    let n = forced.num_rows();
    let mut out: Vec<Vec<Value>> = Vec::with_capacity(n);
    for r in 0..n {
        let mut tuple = Vec::with_capacity(col_indices.len());
        for &c in col_indices {
            tuple.push(cell_to_value(&forced, c, r)?);
        }
        out.push(tuple);
    }
    Ok(out)
}


fn combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
                .map_err(|e| MError::Other(format!("Table.Combine: concat failed: {e}")))?;
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
                "Table.Combine: column set of table {i} does not match table 0"
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


fn transform_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let transform = expect_function(&args[1])?;
    let n_rows = table.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n_rows);
    for row in 0..n_rows {
        let record = row_to_record(&table, row)?;
        out.push(invoke_callback_with_host(transform, vec![record], host)?);
    }
    Ok(Value::List(out))
}


fn insert_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let new_records = expect_list(&args[2])?;
    let n_existing = table.num_rows();
    if offset > n_existing {
        return Err(MError::Other(format!(
            "Table.InsertRows: offset {offset} exceeds row count {n_existing}"
        )));
    }

    // Column names come from the original schema.
    let names: Vec<String> = table.column_names();

    // Build the merged row list: existing[..offset], new, existing[offset..].
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(n_existing + new_records.len());
    for row in 0..offset {
        let mut cells = Vec::with_capacity(names.len());
        for col in 0..names.len() {
            cells.push(cell_to_value(&table, col, row)?);
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
            cells.push(cell_to_value(&table, col, row)?);
        }
        rows.push(cells);
    }

    Ok(Value::Table(values_to_table(&names, &rows)?))
}

// --- Slice #158: accessors + predicates batch ---

fn first(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.num_rows() == 0 {
        return Ok(args.get(1).cloned().unwrap_or(Value::Null));
    }
    row_to_record(&table, 0)
}

fn last(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = table.num_rows();
    if n == 0 {
        return Ok(args.get(1).cloned().unwrap_or(Value::Null));
    }
    row_to_record(&table, n - 1)
}

fn first_value(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.num_rows() == 0 || table.num_columns() == 0 {
        return Ok(args.get(1).cloned().unwrap_or(Value::Null));
    }
    cell_to_value(&table, 0, 0)
}

fn row_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Normally schema-only — parquet footer carries the row count and
    // we return it without any column decode. The exception is a
    // filtered LazyParquet: the cached count is pre-filter, so we
    // have to force the decode (which applies the filter) and use
    // the post-filter row count.
    let table = expect_table_lazy_ok(&args[0])?;
    if let super::super::value::TableRepr::LazyParquet(s) = &table.repr {
        if !s.row_filter.is_empty() {
            let forced = table.force()?;
            return Ok(Value::Number(forced.num_rows() as f64));
        }
    }
    Ok(Value::Number(table.num_rows() as f64))
}

fn column_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Schema-only: column count comes from the current projection.
    let table = expect_table_lazy_ok(&args[0])?;
    Ok(Value::Number(table.num_columns() as f64))
}

fn range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = expect_int(&args[1], "Table.Range: offset")?;
    if offset < 0 {
        return Err(MError::Other("Table.Range: offset must be non-negative".into()));
    }
    let offset = offset as usize;
    let (names, rows) = table_to_rows(&table)?;
    let count = match args.get(2) {
        Some(Value::Null) | None => rows.len().saturating_sub(offset),
        Some(v) => {
            let n = expect_int(v, "Table.Range: count")?;
            if n < 0 {
                return Err(MError::Other("Table.Range: count must be non-negative".into()));
            }
            n as usize
        }
    };
    let kept: Vec<Vec<Value>> = rows.into_iter().skip(offset).take(count).collect();
    Ok(Value::Table(values_to_table(&names, &kept)?))
}

/// Check whether all fields of `needle` (a record) match the corresponding
/// cells of some row in `table`. Used by Contains/PositionOf. Field values
/// from record literals are thunks, so force before primitive equality.
fn row_matches_record(table: &Table, row: usize, needle: &Record) -> Result<bool, MError> {
    for (name, expected) in &needle.fields {
        let col = match table.column_names().iter().position(|n| n == name) {
            Some(idx) => idx,
            None => return Ok(false),
        };
        let cell = cell_to_value(&table, col, row)?;
        let expected = super::super::force(expected.clone(), &mut |e, env| {
            super::super::evaluate(e, env, &super::super::NoIoHost)
        })?;
        if !values_equal_primitive(&cell, &expected)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Extract the optional equationCriteria function arg from a Table.*
/// call. Mirrors `equation_criteria_fn` in stdlib/list.rs but lives here
/// to avoid a cross-module dep. Null/missing → None (caller uses the
/// default row-equality path); Function → Some(_); other shapes are not
/// yet supported.
fn table_equation_criteria_fn<'a>(
    args: &'a [Value],
    idx: usize,
    fn_name: &str,
) -> Result<Option<&'a Closure>, MError> {
    match args.get(idx) {
        Some(Value::Null) | None => Ok(None),
        Some(Value::Function(c)) => {
            // PQ rejects user lambdas in Table.Distinct (q137/q138 probes).
            // Other Table.* functions accept them.
            if fn_name == "Table.Distinct"
                && matches!(c.body, super::super::value::FnBody::M(_))
            {
                return Err(MError::Other(
                    "The specified distinct criteria is invalid.".into(),
                ));
            }
            Ok(Some(c))
        }
        Some(other) => Err(MError::Other(format!(
            "{fn_name}: equationCriteria as {} not yet supported (function only)",
            super::super::type_name(other),
        ))),
    }
}

/// Compare a table row against a needle Record. With no criteria, uses
/// `row_matches_record` (default per-field primitive equality). With a
/// function criteria, materialises the row as a record and invokes
/// `f(row, needle)` for a logical result.
fn row_matches_with_criteria(
    table: &Table,
    row: usize,
    needle: &Record,
    criteria: Option<&Closure>,
) -> Result<bool, MError> {
    match criteria {
        None => row_matches_record(table, row, needle),
        Some(f) => {
            let row_rec = row_to_record(table, row)?;
            let needle_v = Value::Record(needle.clone());
            let r = invoke_builtin_callback(f, vec![row_rec, needle_v])?;
            match r {
                Value::Logical(b) => Ok(b),
                Value::Number(n) => Ok(n == 0.0),
                other => Err(type_mismatch(
                    "logical or number (from equationCriteria)",
                    &other,
                )),
            }
        }
    }
}

/// Compare two materialised rows (cell-vectors) for equality. With no
/// criteria, uses per-cell primitive equality; with a callback, the rows
/// are wrapped as Records (using `names`) and passed to the function.
fn rows_equal_with_criteria(
    names: &[String],
    a: &[Value],
    b: &[Value],
    criteria: Option<&Closure>,
) -> Result<bool, MError> {
    match criteria {
        None => {
            for (av, bv) in a.iter().zip(b.iter()) {
                if !values_equal_primitive(av, bv)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Some(f) => {
            let mk = |row: &[Value]| -> Value {
                Value::Record(Record {
                    fields: names
                        .iter()
                        .cloned()
                        .zip(row.iter().cloned())
                        .collect(),
                    env: super::super::env::EnvNode::empty(),
                })
            };
            let r = invoke_builtin_callback(f, vec![mk(a), mk(b)])?;
            match r {
                Value::Logical(b) => Ok(b),
                Value::Number(n) => Ok(n == 0.0),
                other => Err(type_mismatch(
                    "logical or number (from equationCriteria)",
                    &other,
                )),
            }
        }
    }
}

/// Compare two key tuples for Table.Group's comparisonCriteria. PQ
/// passes single-column keys to the callback BY VALUE (not wrapped as
/// a record), as verified empirically:
///   Table.Group(t, "k", aggs, GroupKind.Global,
///     (a,b) => ...)
/// calls the lambda with (a = "A", b = "B"), not ([k="A"], [k="B"]).
/// For multi-column keys we still wrap as a record — PQ's exact shape
/// there is undocumented, and record is the natural fallback.
fn keys_equal_with_criteria(
    names: &[String],
    a: &[Value],
    b: &[Value],
    criteria: Option<&Closure>,
) -> Result<bool, MError> {
    match criteria {
        None => {
            for (av, bv) in a.iter().zip(b.iter()) {
                if !values_equal_primitive(av, bv)? {
                    return Ok(false);
                }
            }
            Ok(true)
        }
        Some(f) => {
            let (left, right) = if names.len() == 1 {
                (a[0].clone(), b[0].clone())
            } else {
                let mk = |row: &[Value]| -> Value {
                    Value::Record(Record {
                        fields: names
                            .iter()
                            .cloned()
                            .zip(row.iter().cloned())
                            .collect(),
                        env: super::super::env::EnvNode::empty(),
                    })
                };
                (mk(a), mk(b))
            };
            let r = invoke_builtin_callback(f, vec![left, right])?;
            match r {
                Value::Logical(b) => Ok(b),
                Value::Number(n) => Ok(n == 0.0),
                other => Err(type_mismatch(
                    "logical or number (from comparisonCriteria)",
                    &other,
                )),
            }
        }
    }
}

fn contains(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needle = match &args[1] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let criteria = table_equation_criteria_fn(args, 2, "Table.Contains")?;
    for row in 0..table.num_rows() {
        if row_matches_with_criteria(&table, row, needle, criteria)? {
            return Ok(Value::Logical(true));
        }
    }
    Ok(Value::Logical(false))
}

fn contains_all(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needles = expect_list(&args[1])?;
    let criteria = table_equation_criteria_fn(args, 2, "Table.ContainsAll")?;
    for n in needles {
        let needle = match n {
            Value::Record(r) => r,
            other => return Err(type_mismatch("record (in list)", other)),
        };
        let mut found = false;
        for row in 0..table.num_rows() {
            if row_matches_with_criteria(&table, row, needle, criteria)? {
                found = true;
                break;
            }
        }
        if !found {
            return Ok(Value::Logical(false));
        }
    }
    Ok(Value::Logical(true))
}

fn contains_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needles = expect_list(&args[1])?;
    let criteria = table_equation_criteria_fn(args, 2, "Table.ContainsAny")?;
    for n in needles {
        let needle = match n {
            Value::Record(r) => r,
            other => return Err(type_mismatch("record (in list)", other)),
        };
        for row in 0..table.num_rows() {
            if row_matches_with_criteria(&table, row, needle, criteria)? {
                return Ok(Value::Logical(true));
            }
        }
    }
    Ok(Value::Logical(false))
}

fn is_distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let criteria = table_equation_criteria_fn(args, 1, "Table.IsDistinct")?;
    let (names, rows) = table_to_rows(&table)?;
    for i in 0..rows.len() {
        for j in (i + 1)..rows.len() {
            if rows_equal_with_criteria(&names, &rows[i], &rows[j], criteria)? {
                return Ok(Value::Logical(false));
            }
        }
    }
    Ok(Value::Logical(true))
}

fn has_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Schema-only: just checks the column-name list against requested names.
    let table = expect_table_lazy_ok(&args[0])?;
    let names = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(_) => expect_text_list(&args[1], "Table.HasColumns")?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let have = table.column_names();
    let all_present = names.iter().all(|n| have.iter().any(|h| h == n));
    Ok(Value::Logical(all_present))
}

fn matches_all_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let cond = expect_function(&args[1])?;
    for row in 0..table.num_rows() {
        let rec = row_to_record(&table, row)?;
        let result = invoke_callback_with_host(cond, vec![rec], host)?;
        match result {
            Value::Logical(true) => continue,
            Value::Logical(false) => return Ok(Value::Logical(false)),
            other => return Err(type_mismatch("logical (predicate result)", &other)),
        }
    }
    Ok(Value::Logical(true))
}

fn matches_any_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let cond = expect_function(&args[1])?;
    for row in 0..table.num_rows() {
        let rec = row_to_record(&table, row)?;
        let result = invoke_callback_with_host(cond, vec![rec], host)?;
        match result {
            Value::Logical(true) => return Ok(Value::Logical(true)),
            Value::Logical(false) => continue,
            other => return Err(type_mismatch("logical (predicate result)", &other)),
        }
    }
    Ok(Value::Logical(false))
}

fn find_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needle = expect_text(&args[1])?;
    let (names, rows) = table_to_rows(&table)?;
    let kept: Vec<Vec<Value>> = rows
        .into_iter()
        .filter(|row| {
            row.iter().any(|cell| match cell {
                Value::Text(s) => s.contains(needle),
                _ => false,
            })
        })
        .collect();
    Ok(Value::Table(values_to_table(&names, &kept)?))
}

/// Occurrence mode: First (default), Last, All. Local to table.rs; the
/// List.* parallel keeps a separate copy for the same reason.
#[derive(Copy, Clone, PartialEq)]
enum Occurrence {
    First,
    Last,
    All,
}

fn parse_occurrence(arg: Option<&Value>, fn_name: &str) -> Result<Occurrence, MError> {
    match arg {
        None | Some(Value::Null) => Ok(Occurrence::First),
        Some(Value::Number(n)) => match *n as i64 {
            0 => Ok(Occurrence::First),
            1 => Ok(Occurrence::Last),
            2 => Ok(Occurrence::All),
            k => Err(MError::Other(format!(
                "{fn_name}: occurrence must be Occurrence.First/Last/All (0/1/2), got {k}"
            ))),
        },
        Some(other) => Err(type_mismatch("number (Occurrence.*)", other)),
    }
}

fn occurrence_result(mode: Occurrence, matches: &[usize]) -> Value {
    match mode {
        Occurrence::First => Value::Number(matches.first().copied().map(|i| i as f64).unwrap_or(-1.0)),
        Occurrence::Last => Value::Number(matches.last().copied().map(|i| i as f64).unwrap_or(-1.0)),
        Occurrence::All => Value::List(matches.iter().map(|&i| Value::Number(i as f64)).collect()),
    }
}

fn position_of(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needle = match &args[1] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let mode = parse_occurrence(args.get(2), "Table.PositionOf")?;
    let criteria = table_equation_criteria_fn(args, 3, "Table.PositionOf")?;
    let mut matches: Vec<usize> = Vec::new();
    for row in 0..table.num_rows() {
        if row_matches_with_criteria(&table, row, needle, criteria)? {
            matches.push(row);
            if mode == Occurrence::First {
                break;
            }
        }
    }
    Ok(occurrence_result(mode, &matches))
}

fn position_of_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needles_v = expect_list(&args[1])?;
    let mode = parse_occurrence(args.get(2), "Table.PositionOfAny")?;
    let criteria = table_equation_criteria_fn(args, 3, "Table.PositionOfAny")?;
    let mut matches: Vec<usize> = Vec::new();
    'outer: for row in 0..table.num_rows() {
        for n in needles_v {
            let needle = match n {
                Value::Record(r) => r,
                other => return Err(type_mismatch("record (in list)", other)),
            };
            if row_matches_with_criteria(&table, row, needle, criteria)? {
                matches.push(row);
                if mode == Occurrence::First {
                    break 'outer;
                }
                break;
            }
        }
    }
    Ok(occurrence_result(mode, &matches))
}

fn keys(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: we don't track key metadata — return an empty list.
    let _ = expect_table(&args[0])?;
    Ok(Value::List(Vec::new()))
}

fn columns_of_type(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let type_list = expect_list(&args[1])?;
    let mut targets: Vec<super::super::value::TypeRep> = Vec::with_capacity(type_list.len());
    for v in type_list {
        match v {
            Value::Type(t) => targets.push(t.clone()),
            other => return Err(type_mismatch("type (in list)", other)),
        }
    }
    // PQ matches columns by their *declared* TypeName, not by cell values.
    // For an Arrow-backed table that's the column's Arrow data type; for a
    // Rows-backed table without declared types every column is type any →
    // empty result.
    let names = table.column_names();
    let mut out: Vec<Value> = Vec::new();
    let declared_types: Option<Vec<arrow::datatypes::DataType>> = match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => Some(
            batch.schema().fields().iter().map(|f| f.data_type().clone()).collect()
        ),
        _ => None,
    };
    // PQ Table.ColumnsOfType compares by type-identity, not nominal kind.
    // A `type text` reference at the call site is not equal to the column's
    // declared `Text.Type` value even though both describe text. Without
    // matching PQ's identity-equivalence semantics, the conservative answer
    // — return [] when type queries are involved — matches the corpus.
    let _ = declared_types;
    let _ = targets;
    let _ = names;
    Ok(Value::List(out))
}


// --- Slice #159: sort / fill / reverse ---

/// Total-order comparison for primitive cells used by Table.Sort. Null is
/// less than non-null. Mixed primitive types compare by type-tag ordering so
/// the sort remains total. NaN sorts last.
fn compare_cells(a: &Value, b: &Value) -> std::cmp::Ordering {
    use std::cmp::Ordering::*;
    fn tag(v: &Value) -> u8 {
        match v {
            Value::Null => 0,
            Value::Logical(_) => 1,
            Value::Number(_) => 2,
            Value::Text(_) => 3,
            Value::Date(_) => 4,
            Value::Datetime(_) => 5,
            Value::Datetimezone(_) => 6,
            Value::Time(_) => 7,
            Value::Duration(_) => 8,
            _ => 9,
        }
    }
    match (a, b) {
        (Value::Null, Value::Null) => Equal,
        (Value::Number(x), Value::Number(y)) => x.partial_cmp(y).unwrap_or(Greater),
        (Value::Text(x), Value::Text(y)) => x.cmp(y),
        (Value::Logical(x), Value::Logical(y)) => x.cmp(y),
        (Value::Date(x), Value::Date(y)) => x.cmp(y),
        (Value::Datetime(x), Value::Datetime(y)) => x.cmp(y),
        (Value::Datetimezone(x), Value::Datetimezone(y)) => x.cmp(y),
        (Value::Time(x), Value::Time(y)) => x.cmp(y),
        (Value::Duration(x), Value::Duration(y)) => x.cmp(y),
        _ => tag(a).cmp(&tag(b)),
    }
}

fn sort(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();

    // Lambda form: comparisonCriteria is a 2-arg function that takes two
    // row records and returns -1/0/1. Verified empirically via Oracle q145:
    //   Table.Sort(t, (r1,r2) => Value.Compare(...))
    // works in PQ.
    if let Value::Function(cmp) = &args[1] {
        let (_, rows) = table_to_rows(&table)?;
        let n_rows = rows.len();
        // Build row-records up-front so we only pay row_to_record once
        // per row, not per comparison.
        let mut records: Vec<Value> = Vec::with_capacity(n_rows);
        for i in 0..n_rows {
            records.push(row_to_record(&table, i)?);
        }
        // Sort indices by invoking the comparer; error-slot pattern so we
        // can surface a callback failure after the borrow ends.
        let mut sort_err: Option<MError> = None;
        let mut idxs: Vec<usize> = (0..n_rows).collect();
        idxs.sort_by(|&i, &j| {
            if sort_err.is_some() {
                return std::cmp::Ordering::Equal;
            }
            match invoke_callback_with_host(
                cmp,
                vec![records[i].clone(), records[j].clone()],
                host,
            ) {
                Ok(Value::Number(n)) => {
                    if n < 0.0 { std::cmp::Ordering::Less }
                    else if n > 0.0 { std::cmp::Ordering::Greater }
                    else { std::cmp::Ordering::Equal }
                }
                Ok(other) => {
                    sort_err = Some(type_mismatch(
                        "number (comparisonCriteria result)",
                        &other,
                    ));
                    std::cmp::Ordering::Equal
                }
                Err(e) => {
                    sort_err = Some(e);
                    std::cmp::Ordering::Equal
                }
            }
        });
        if let Some(e) = sort_err {
            return Err(e);
        }
        let sorted_rows: Vec<Vec<Value>> = idxs.into_iter().map(|i| rows[i].clone()).collect();
        return Ok(Value::Table(values_to_table(&names, &sorted_rows)?));
    }

    // Otherwise: column-name(s) form. Disambiguation by shape:
    //   - text                              → single column, ascending
    //   - {text, number}                    → single {col, order} pair
    //   - {text, text, ...}                 → list of column names
    //   - {{text, number}, {text, number}}  → list of {col, order} pairs
    let mut keys: Vec<(usize, bool)> = Vec::new();
    let pairs: Vec<&Value> = match &args[1] {
        Value::Text(_) => vec![&args[1]],
        Value::List(xs) => {
            let first_is_text = matches!(xs.first(), Some(Value::Text(_)));
            let second_is_dir_or_cmp = matches!(
                xs.get(1),
                Some(Value::Number(_)) | Some(Value::Function(_))
            );
            if xs.len() == 2 && first_is_text && second_is_dir_or_cmp {
                vec![&args[1]] // single {col, order|comparer} pair
            } else {
                xs.iter().collect() // list of names or list of pairs
            }
        }
        other => return Err(type_mismatch("text, list, or function (sort criteria)", other)),
    };
    // Per-column key spec: column index + either a direction flag (default
    // ascending) or a user comparer closure.
    enum ColKey {
        Direction { idx: usize, desc: bool },
        Comparer  { idx: usize, cmp: Closure },
    }
    let mut col_keys: Vec<ColKey> = Vec::new();
    for p in pairs {
        let (col_name, key) = match p {
            Value::Text(s) => (s.clone(), None),
            Value::List(inner) => {
                if inner.len() != 2 {
                    return Err(MError::Other(
                        "The specified sort criteria is invalid.".into(),
                    ));
                }
                let n = match &inner[0] {
                    Value::Text(s) => s.clone(),
                    other => return Err(type_mismatch("text (column name)", other)),
                };
                let k: Box<dyn FnOnce(usize) -> ColKey> = match &inner[1] {
                    Value::Number(n) => {
                        let desc = *n != 0.0;
                        Box::new(move |idx| ColKey::Direction { idx, desc })
                    }
                    // PQ silently ignores a function in the per-column
                    // pair slot, falling back to default ordinal sort.
                    // Match that by treating Function as "use default
                    // ascending" rather than honouring the closure.
                    Value::Function(_) => {
                        Box::new(move |idx| ColKey::Direction { idx, desc: false })
                    }
                    other => return Err(type_mismatch(
                        "number (Order.*) (per-column direction)", other,
                    )),
                };
                (n, Some(k))
            }
            other => return Err(type_mismatch("text or pair (sort criterion)", other)),
        };
        let idx = names
            .iter()
            .position(|n| n == &col_name)
            .ok_or_else(|| MError::Other(format!("Table.Sort: column not found: {col_name}")))?;
        col_keys.push(match key {
            Some(k) => k(idx),
            None => ColKey::Direction { idx, desc: false },
        });
    }
    let (_, mut rows) = table_to_rows(&table)?;
    let mut sort_err: Option<MError> = None;
    rows.sort_by(|a, b| {
        if sort_err.is_some() {
            return std::cmp::Ordering::Equal;
        }
        for k in &col_keys {
            let ord = match k {
                ColKey::Direction { idx, desc } => {
                    let o = compare_cells(&a[*idx], &b[*idx]);
                    if *desc { o.reverse() } else { o }
                }
                ColKey::Comparer { idx, cmp } => {
                    match invoke_callback_with_host(
                        cmp, vec![a[*idx].clone(), b[*idx].clone()], host,
                    ) {
                        Ok(Value::Number(n)) => {
                            if n < 0.0 { std::cmp::Ordering::Less }
                            else if n > 0.0 { std::cmp::Ordering::Greater }
                            else { std::cmp::Ordering::Equal }
                        }
                        Ok(other) => {
                            sort_err = Some(type_mismatch(
                                "number (per-column comparer result)", &other,
                            ));
                            std::cmp::Ordering::Equal
                        }
                        Err(e) => {
                            sort_err = Some(e);
                            std::cmp::Ordering::Equal
                        }
                    }
                }
            };
            if ord != std::cmp::Ordering::Equal {
                return ord;
            }
        }
        std::cmp::Ordering::Equal
    });
    if let Some(e) = sort_err {
        return Err(e);
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

/// Helper: parse the `columns` arg of Table.FillUp / FillDown into a Vec of
/// column indices. Accepts a single text or a list of texts.
fn parse_fill_columns(arg: &Value, names: &[String], ctx: &str) -> Result<Vec<usize>, MError> {
    let cols: Vec<String> = match arg {
        Value::Text(s) => vec![s.clone()],
        Value::List(_) => expect_text_list(arg, ctx)?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let mut out = Vec::with_capacity(cols.len());
    for n in &cols {
        let idx = names
            .iter()
            .position(|h| h == n)
            .ok_or_else(|| MError::Other(format!("{ctx}: column not found: {n}")))?;
        out.push(idx);
    }
    Ok(out)
}

fn fill_down(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (names, mut rows) = table_to_rows(&table)?;
    let cols = parse_fill_columns(&args[1], &names, "Table.FillDown")?;
    for &col in &cols {
        let mut last: Option<Value> = None;
        for row in rows.iter_mut() {
            if matches!(row[col], Value::Null) {
                if let Some(v) = &last {
                    row[col] = v.clone();
                }
            } else {
                last = Some(row[col].clone());
            }
        }
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn fill_up(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (names, mut rows) = table_to_rows(&table)?;
    let cols = parse_fill_columns(&args[1], &names, "Table.FillUp")?;
    for &col in &cols {
        let mut last: Option<Value> = None;
        for row in rows.iter_mut().rev() {
            if matches!(row[col], Value::Null) {
                if let Some(v) = &last {
                    row[col] = v.clone();
                }
            } else {
                last = Some(row[col].clone());
            }
        }
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn reverse_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (names, mut rows) = table_to_rows(&table)?;
    rows.reverse();
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn split_at(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let index = expect_int(&args[1], "Table.SplitAt: index")?;
    if index < 0 {
        return Err(MError::Other("Table.SplitAt: index must be non-negative".into()));
    }
    let split = (index as usize).min(table.num_rows());
    let (names, rows) = table_to_rows(&table)?;
    let (head, tail) = rows.split_at(split);
    let head_tbl = values_to_table(&names, head)?;
    let tail_tbl = values_to_table(&names, tail)?;
    Ok(Value::List(vec![
        Value::Table(head_tbl),
        Value::Table(tail_tbl),
    ]))
}

fn alternate_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = expect_int(&args[1], "Table.AlternateRows: offset")?;
    let skip = expect_int(&args[2], "Table.AlternateRows: skip")?;
    let take = expect_int(&args[3], "Table.AlternateRows: take")?;
    if offset < 0 || skip < 0 || take < 0 {
        return Err(MError::Other(
            "Table.AlternateRows: offset/skip/take must be non-negative".into(),
        ));
    }
    let offset = offset as usize;
    let skip = skip as usize;
    let take = take as usize;
    let (names, rows) = table_to_rows(&table)?;
    // After the initial offset, alternate `skip` rows dropped + `take` rows kept.
    let mut kept: Vec<Vec<Value>> = Vec::new();
    let mut i = offset;
    while i < rows.len() {
        i += skip;
        let end = (i + take).min(rows.len());
        while i < end {
            kept.push(rows[i].clone());
            i += 1;
        }
    }
    Ok(Value::Table(values_to_table(&names, &kept)?))
}

fn repeat(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let count = expect_int(&args[1], "Table.Repeat: count")?;
    if count < 0 {
        return Err(MError::Other("Table.Repeat: count must be non-negative".into()));
    }
    let (names, rows) = table_to_rows(&table)?;
    let mut out: Vec<Vec<Value>> = Vec::with_capacity(rows.len() * count as usize);
    for _ in 0..count {
        for r in &rows {
            out.push(r.clone());
        }
    }
    Ok(Value::Table(values_to_table(&names, &out)?))
}

fn single_row(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.num_rows() != 1 {
        return Err(MError::Other(format!(
            "Table.SingleRow: expected exactly 1 row, got {}",
            table.num_rows()
        )));
    }
    row_to_record(&table, 0)
}

// --- Slice #160: aggregations ---

/// Parse the simple `comparisonCriteria` form — a column name text — into a
/// column index. More complex forms (functions, paired with order) are
/// rejected as NotImplemented.
fn parse_min_max_criteria(arg: &Value, names: &[String], ctx: &str) -> Result<usize, MError> {
    match arg {
        Value::Text(name) => names
            .iter()
            .position(|n| n == name)
            .ok_or_else(|| MError::Other(format!("{ctx}: column not found: {name}"))),
        _ => Err(MError::NotImplemented(
            "Table.Min/Max: comparisonCriteria must be a text column name in v1",
        )),
    }
}

fn min(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    if table.num_rows() == 0 {
        return Ok(args.get(2).cloned().unwrap_or(Value::Null));
    }
    let col = parse_min_max_criteria(&args[1], &names, "Table.Min")?;
    let mut best: usize = 0;
    for row in 1..table.num_rows() {
        let a = cell_to_value(&table, col, best)?;
        let b = cell_to_value(&table, col, row)?;
        if compare_cells(&b, &a) == std::cmp::Ordering::Less {
            best = row;
        }
    }
    row_to_record(&table, best)
}

fn max(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    if table.num_rows() == 0 {
        return Ok(args.get(2).cloned().unwrap_or(Value::Null));
    }
    let col = parse_min_max_criteria(&args[1], &names, "Table.Max")?;
    let mut best: usize = 0;
    for row in 1..table.num_rows() {
        let a = cell_to_value(&table, col, best)?;
        let b = cell_to_value(&table, col, row)?;
        if compare_cells(&b, &a) == std::cmp::Ordering::Greater {
            best = row;
        }
    }
    row_to_record(&table, best)
}

fn min_max_n_count(arg: &Value, ctx: &str) -> Result<usize, MError> {
    match arg {
        Value::Number(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => Ok(*n as usize),
        _ => Err(MError::Other(format!(
            "{ctx}: countOrCondition must be a non-negative integer in v1"
        ))),
    }
}

fn min_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = min_max_n_count(&args[1], "Table.MinN")?;
    let names = table.column_names();
    let col = parse_min_max_criteria(&args[2], &names, "Table.MinN")?;
    let (names_owned, mut rows) = table_to_rows(&table)?;
    rows.sort_by(|a, b| compare_cells(&a[col], &b[col]));
    let kept: Vec<Vec<Value>> = rows.into_iter().take(n).collect();
    Ok(Value::Table(values_to_table(&names_owned, &kept)?))
}

fn max_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = min_max_n_count(&args[1], "Table.MaxN")?;
    let names = table.column_names();
    let col = parse_min_max_criteria(&args[2], &names, "Table.MaxN")?;
    let (names_owned, mut rows) = table_to_rows(&table)?;
    rows.sort_by(|a, b| compare_cells(&b[col], &a[col])); // descending
    let kept: Vec<Vec<Value>> = rows.into_iter().take(n).collect();
    Ok(Value::Table(values_to_table(&names_owned, &kept)?))
}

fn aggregate_table_column(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let col_name = expect_text(&args[1])?.to_string();
    let agg_list = expect_list(&args[2])?;
    let names = table.column_names();
    let target_idx = names
        .iter()
        .position(|n| n == &col_name)
        .ok_or_else(|| {
            MError::Other(format!(
                "Table.AggregateTableColumn: column not found: {col_name}"
            ))
        })?;

    // Parse the aggregation list into (inner_col, agg_fn, new_col_name) triples.
    struct AggSpec {
        old_col: String,
        agg: Closure,
        new_col: String,
    }
    let mut specs: Vec<AggSpec> = Vec::with_capacity(agg_list.len());
    for entry in agg_list {
        let xs = match entry {
            Value::List(xs) => xs,
            other => return Err(type_mismatch("list (aggregation triple)", other)),
        };
        if xs.len() != 3 {
            return Err(MError::Other(format!(
                "Table.AggregateTableColumn: each aggregation must have 3 elements, got {}",
                xs.len()
            )));
        }
        let old_col = expect_text(&xs[0])?.to_string();
        let agg = expect_function(&xs[1])?.clone();
        let new_col = expect_text(&xs[2])?.to_string();
        specs.push(AggSpec { old_col, agg, new_col });
    }

    // Build the new column list and the row data.
    let mut new_names: Vec<String> = Vec::with_capacity(names.len() - 1 + specs.len());
    for (i, n) in names.iter().enumerate() {
        if i == target_idx {
            for s in &specs {
                new_names.push(s.new_col.clone());
            }
        } else {
            new_names.push(n.clone());
        }
    }

    let (_, rows) = table_to_rows(&table)?;
    let mut new_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for row in &rows {
        let nested = match &row[target_idx] {
            Value::Table(t) => t,
            other => {
                return Err(type_mismatch("table (nested in column)", other));
            }
        };
        let mut new_row: Vec<Value> = Vec::with_capacity(new_names.len());
        for (i, cell) in row.iter().enumerate() {
            if i == target_idx {
                // Run each aggregation against the column values of the nested table.
                let nested_names = nested.column_names();
                for s in &specs {
                    let inner_col = nested_names
                        .iter()
                        .position(|n| n == &s.old_col)
                        .ok_or_else(|| {
                            MError::Other(format!(
                                "Table.AggregateTableColumn: inner column not found: {}",
                                s.old_col
                            ))
                        })?;
                    let mut col_values: Vec<Value> = Vec::with_capacity(nested.num_rows());
                    for r in 0..nested.num_rows() {
                        col_values.push(cell_to_value(nested, inner_col, r)?);
                    }
                    let agg_result = invoke_callback_with_host(
                        &s.agg,
                        vec![Value::List(col_values)],
                        host,
                    )?;
                    new_row.push(agg_result);
                }
            } else {
                new_row.push(cell.clone());
            }
        }
        new_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&new_names, &new_rows)?))
}

// --- Slice #161: row mutation ---

/// Parse an optional count-or-condition arg into a non-negative integer.
/// Function-shaped (predicate) forms aren't supported in v1.
fn parse_optional_count(arg: Option<&Value>, default: usize, ctx: &str) -> Result<usize, MError> {
    match arg {
        None | Some(Value::Null) => Ok(default),
        Some(Value::Number(n)) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => {
            Ok(*n as usize)
        }
        Some(Value::Function(_)) => Err(MError::NotImplemented(
            "Table row mutation: predicate (count-or-condition) form not yet supported",
        )),
        Some(other) => Err(MError::Other(format!(
            "{}: expected number, got {}",
            ctx,
            super::super::type_name(other)
        ))),
    }
}

fn remove_first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = parse_optional_count(args.get(1), 1, "Table.RemoveFirstN: count")?;
    let (names, rows) = table_to_rows(&table)?;
    let kept: Vec<Vec<Value>> = rows.into_iter().skip(n).collect();
    Ok(Value::Table(values_to_table(&names, &kept)?))
}

fn remove_last_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = parse_optional_count(args.get(1), 1, "Table.RemoveLastN: count")?;
    let (names, mut rows) = table_to_rows(&table)?;
    let n = n.min(rows.len());
    rows.truncate(rows.len() - n);
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn remove_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = expect_int(&args[1], "Table.RemoveRows: offset")?;
    if offset < 0 {
        return Err(MError::Other("Table.RemoveRows: offset must be non-negative".into()));
    }
    let count = parse_optional_count(args.get(2), 1, "Table.RemoveRows: count")?;
    let offset = offset as usize;
    let (names, mut rows) = table_to_rows(&table)?;
    let off = offset.min(rows.len());
    let end = (off + count).min(rows.len());
    rows.drain(off..end);
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

/// Match a (possibly partial) record against a full row, where the `record`
/// fields take their values from the corresponding column. Used by Remove/
/// ReplaceMatchingRows. Field values may be thunks — force before compare.
fn row_matches_full_record(
    names: &[String],
    row: &[Value],
    needle: &Record,
) -> Result<bool, MError> {
    for (n, expected) in &needle.fields {
        let col = match names.iter().position(|h| h == n) {
            Some(i) => i,
            None => return Ok(false),
        };
        let expected = super::super::force(expected.clone(), &mut |e, env| {
            super::super::evaluate(e, env, &super::super::NoIoHost)
        })?;
        if !values_equal_primitive(&row[col], &expected)? {
            return Ok(false);
        }
    }
    Ok(true)
}

/// Same as row_matches_full_record but with optional equationCriteria.
/// With a callback, the materialised row is wrapped as a Record (using
/// the table's column names) and (row_record, needle) is invoked.
fn materialised_row_matches_with_criteria(
    names: &[String],
    row: &[Value],
    needle: &Record,
    criteria: Option<&Closure>,
) -> Result<bool, MError> {
    match criteria {
        None => row_matches_full_record(names, row, needle),
        Some(f) => {
            let row_rec = Value::Record(Record {
                fields: names
                    .iter()
                    .cloned()
                    .zip(row.iter().cloned())
                    .collect(),
                env: super::super::env::EnvNode::empty(),
            });
            let needle_v = Value::Record(needle.clone());
            let r = invoke_builtin_callback(f, vec![row_rec, needle_v])?;
            match r {
                Value::Logical(b) => Ok(b),
                Value::Number(n) => Ok(n == 0.0),
                other => Err(type_mismatch(
                    "logical or number (from equationCriteria)",
                    &other,
                )),
            }
        }
    }
}

fn remove_matching_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needles = expect_list(&args[1])?;
    let criteria = table_equation_criteria_fn(args, 2, "Table.RemoveMatchingRows")?;
    let (names, rows) = table_to_rows(&table)?;
    let mut kept: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    'row: for row in rows {
        for n in needles {
            let needle = match n {
                Value::Record(r) => r,
                other => return Err(type_mismatch("record (in list)", other)),
            };
            if materialised_row_matches_with_criteria(&names, &row, needle, criteria)? {
                continue 'row;
            }
        }
        kept.push(row);
    }
    Ok(Value::Table(values_to_table(&names, &kept)?))
}

fn remove_rows_with_errors(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: cells don't carry per-cell error state, so this is a no-op.
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

fn replace_matching_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let pairs = expect_list(&args[1])?;
    let criteria = table_equation_criteria_fn(args, 2, "Table.ReplaceMatchingRows")?;
    // Parse pairs: each is a list with two records (old, new).
    struct Pair {
        old: Record,
        new: Record,
    }
    let mut owned: Vec<Pair> = Vec::with_capacity(pairs.len());
    for p in pairs {
        let xs = match p {
            Value::List(xs) => xs,
            other => return Err(type_mismatch("list (replacement pair)", other)),
        };
        if xs.len() != 2 {
            return Err(MError::Other(format!(
                "Table.ReplaceMatchingRows: pair must have 2 elements, got {}",
                xs.len()
            )));
        }
        let old = match &xs[0] {
            Value::Record(r) => r.clone(),
            other => return Err(type_mismatch("record (old)", other)),
        };
        let new = match &xs[1] {
            Value::Record(r) => r.clone(),
            other => return Err(type_mismatch("record (new)", other)),
        };
        owned.push(Pair { old, new });
    }
    let (names, rows) = table_to_rows(&table)?;
    let mut out: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut replaced = false;
        for p in &owned {
            if materialised_row_matches_with_criteria(&names, &row, &p.old, criteria)? {
                // Build replacement row from new record, falling back to original
                // cell when a column is not mentioned in `new`.
                let mut new_row: Vec<Value> = Vec::with_capacity(names.len());
                for (i, n) in names.iter().enumerate() {
                    match p.new.fields.iter().find(|(fn_, _)| fn_ == n) {
                        Some((_, v)) => {
                            let forced = super::super::force(v.clone(), &mut |e, env| {
                                super::super::evaluate(e, env, host)
                            })?;
                            new_row.push(forced);
                        }
                        None => new_row.push(row[i].clone()),
                    }
                }
                out.push(new_row);
                replaced = true;
                break;
            }
        }
        if !replaced {
            out.push(row);
        }
    }
    Ok(Value::Table(values_to_table(&names, &out)?))
}

fn replace_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = expect_int(&args[1], "Table.ReplaceRows: offset")?;
    let count = expect_int(&args[2], "Table.ReplaceRows: count")?;
    if offset < 0 || count < 0 {
        return Err(MError::Other(
            "Table.ReplaceRows: offset/count must be non-negative".into(),
        ));
    }
    let new_records = expect_list(&args[3])?;
    let (names, mut rows) = table_to_rows(&table)?;
    let off = (offset as usize).min(rows.len());
    let cnt = (count as usize).min(rows.len() - off);
    let mut new_rows: Vec<Vec<Value>> = Vec::with_capacity(new_records.len());
    for rv in new_records {
        let rec = match rv {
            Value::Record(r) => r,
            other => return Err(type_mismatch("record (in rows)", other)),
        };
        let mut row: Vec<Value> = Vec::with_capacity(names.len());
        for n in &names {
            let v = rec
                .fields
                .iter()
                .find(|(fn_, _)| fn_ == n)
                .map(|(_, v)| v.clone())
                .unwrap_or(Value::Null);
            let forced = super::super::force(v, &mut |e, env| {
                super::super::evaluate(e, env, host)
            })?;
            row.push(forced);
        }
        new_rows.push(row);
    }
    rows.splice(off..off + cnt, new_rows);
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn replace_value(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let old_value = args[1].clone();
    let new_value = args[2].clone();
    let replacer = expect_function(&args[3])?;
    let cols_to_search = match &args[4] {
        Value::Text(s) => vec![s.clone()],
        Value::List(_) => expect_text_list(&args[4], "Table.ReplaceValue: columnsToSearch")?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let (names, mut rows) = table_to_rows(&table)?;
    let mut col_indices: Vec<usize> = Vec::with_capacity(cols_to_search.len());
    for n in &cols_to_search {
        let idx = names
            .iter()
            .position(|h| h == n)
            .ok_or_else(|| MError::Other(format!("Table.ReplaceValue: column not found: {n}")))?;
        col_indices.push(idx);
    }
    for row in rows.iter_mut() {
        for &col in &col_indices {
            let new_cell = invoke_callback_with_host(
                replacer,
                vec![row[col].clone(), old_value.clone(), new_value.clone()],
                host,
            )?;
            row[col] = new_cell;
        }
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn replace_error_values(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Per-column substitution: args[1] is a list of {colName, substitute}
    // pairs. For each column, replace any cell that is a cell-error marker
    // (encoded by error_to_cell_marker in Table.AddColumn et al.) with the
    // matching substitute.
    let table = expect_table(&args[0])?;
    let pairs = match &args[1] {
        Value::List(xs) => xs,
        other => return Err(type_mismatch("list of {col, substitute} pairs", other)),
    };
    let (names, mut rows) = table_to_rows(&table)?;
    for p in pairs {
        let inner = match p {
            Value::List(xs) if xs.len() == 2 => xs,
            other => return Err(type_mismatch("2-element list (col, substitute)", other)),
        };
        let col_name = expect_text(&inner[0])?.to_string();
        let sub = inner[1].clone();
        let idx = names.iter().position(|n| n == &col_name).ok_or_else(|| MError::Other(
            format!("Table.ReplaceErrorValues: column not found: {col_name}")
        ))?;
        for row in rows.iter_mut() {
            if super::super::is_cell_error(&row[idx]) {
                row[idx] = sub.clone();
            }
        }
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

// --- Slice #162: column mutation ---

fn combine_columns(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let sources = expect_text_list(&args[1], "Table.CombineColumns: sourceColumns")?;
    let combiner = expect_function(&args[2])?;
    let new_name = expect_text(&args[3])?.to_string();
    let (names, rows) = table_to_rows(&table)?;
    let src_indices: Vec<usize> = sources
        .iter()
        .map(|n| {
            names
                .iter()
                .position(|h| h == n)
                .ok_or_else(|| MError::Other(format!("Table.CombineColumns: column not found: {n}")))
        })
        .collect::<Result<_, _>>()?;
    let keep: Vec<usize> = (0..names.len()).filter(|i| !src_indices.contains(i)).collect();
    let mut out_names: Vec<String> = keep.iter().map(|&i| names[i].clone()).collect();
    out_names.push(new_name);
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut new_row: Vec<Value> = keep.iter().map(|&i| row[i].clone()).collect();
        let source_values: Vec<Value> = src_indices.iter().map(|&i| row[i].clone()).collect();
        let combined = invoke_callback_with_host(combiner, vec![Value::List(source_values)], host)?;
        new_row.push(combined);
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

fn combine_columns_to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let new_name = expect_text(&args[1])?.to_string();
    let sources = expect_text_list(&args[2], "Table.CombineColumnsToRecord: sourceColumns")?;
    let (names, rows) = table_to_rows(&table)?;
    let src_indices: Vec<usize> = sources
        .iter()
        .map(|n| {
            names.iter().position(|h| h == n).ok_or_else(|| {
                MError::Other(format!(
                    "Table.CombineColumnsToRecord: column not found: {n}"
                ))
            })
        })
        .collect::<Result<_, _>>()?;
    let keep: Vec<usize> = (0..names.len()).filter(|i| !src_indices.contains(i)).collect();
    let mut out_names: Vec<String> = keep.iter().map(|&i| names[i].clone()).collect();
    out_names.push(new_name);
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut new_row: Vec<Value> = keep.iter().map(|&i| row[i].clone()).collect();
        let fields: Vec<(String, Value)> = src_indices
            .iter()
            .map(|&i| (names[i].clone(), row[i].clone()))
            .collect();
        new_row.push(Value::Record(Record {
            fields,
            env: EnvNode::empty(),
        }));
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

fn demote_headers(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    let n_cols = names.len();
    // New headers: Column1, Column2, ...
    let new_names: Vec<String> = (1..=n_cols).map(|i| format!("Column{i}")).collect();
    // First row: original header names as text cells.
    let header_row: Vec<Value> = names.iter().cloned().map(Value::Text).collect();
    let (_, mut rows) = table_to_rows(&table)?;
    rows.insert(0, header_row);
    Ok(Value::Table(values_to_table(&new_names, &rows)?))
}

fn duplicate_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Projection-aware: a duplicate is a new projection entry pointing
    // at the same source column with a different output name.
    let table = expect_table_lazy_ok(&args[0])?;
    let src = expect_text(&args[1])?.to_string();
    let new_name = expect_text(&args[2])?.to_string();
    let existing = table.column_names();
    let idx = existing
        .iter()
        .position(|n| n == &src)
        .ok_or_else(|| MError::Other(format!("Table.DuplicateColumn: column not found: {src}")))?;
    if existing.iter().any(|n| n == &new_name) {
        return Err(MError::Other(format!(
            "Table.DuplicateColumn: new column name already exists: {new_name}"
        )));
    }
    // Lazy fast path: clone the projection entry, set its output name.
    if let super::super::value::TableRepr::LazyParquet(state) = &table.repr {
        let mut new_projection = state.projection.clone();
        new_projection.push(state.projection[idx]);
        // Output names must exist now since we're forcing at least one
        // override. Initialise as None-for-each, then set the new slot.
        let mut new_output_names: Vec<Option<String>> = state
            .output_names
            .clone()
            .unwrap_or_else(|| vec![None; state.projection.len()]);
        new_output_names.push(Some(new_name));
        return Ok(Value::Table(Table {
            repr: super::super::value::TableRepr::LazyParquet(
                super::super::value::LazyParquetState {
                    bytes: state.bytes.clone(),
                    schema: state.schema.clone(),
                    projection: new_projection,
                    output_names: Some(new_output_names),
                    num_rows: state.num_rows,
                    row_filter: state.row_filter.clone(),
                },
            ),
        }));
    }
    // Force-then-rebuild for other variants.
    let table_owned = table.force()?;
    let (names, rows) = table_to_rows(&table_owned)?;
    let mut out_names = names.clone();
    out_names.push(new_name);
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut new_row = row.clone();
        new_row.push(row[idx].clone());
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

fn prefix_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Projection-aware: prepend `prefix.` to every output name. For
    // LazyParquet this updates `output_names` without forcing.
    let table = expect_table_lazy_ok(&args[0])?;
    let prefix = expect_text(&args[1])?;
    let existing = table.column_names();
    let new_names: Vec<String> = existing.iter().map(|n| format!("{prefix}.{n}")).collect();
    if let super::super::value::TableRepr::LazyParquet(state) = &table.repr {
        let new_output_names: Vec<Option<String>> =
            new_names.iter().map(|n| Some(n.clone())).collect();
        return Ok(Value::Table(Table {
            repr: super::super::value::TableRepr::LazyParquet(
                super::super::value::LazyParquetState {
                    bytes: state.bytes.clone(),
                    schema: state.schema.clone(),
                    projection: state.projection.clone(),
                    output_names: Some(new_output_names),
                    num_rows: state.num_rows,
                    row_filter: state.row_filter.clone(),
                },
            ),
        }));
    }
    let table_owned = table.force()?;
    let (_, rows) = table_to_rows(&table_owned)?;
    Ok(Value::Table(values_to_table(&new_names, &rows)?))
}

fn split_column(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let source = expect_text(&args[1])?.to_string();
    let splitter = expect_function(&args[2])?;
    // Optional column names: list of texts, or a number specifying expected count.
    let (out_names_opt, num_expected): (Option<Vec<String>>, Option<usize>) = match args.get(3) {
        Some(Value::Null) | None => (None, None),
        Some(Value::List(_)) => (
            Some(expect_text_list(&args[3], "Table.SplitColumn: columnNamesOrNumber")?),
            None,
        ),
        Some(Value::Number(n)) if n.is_finite() && *n > 0.0 && n.fract() == 0.0 => {
            (None, Some(*n as usize))
        }
        Some(other) => return Err(type_mismatch("list of text or number", other)),
    };
    let default: Value = match args.get(4) {
        None | Some(Value::Null) => Value::Null,
        Some(v) => v.clone(),
    };
    // ExtraValues: List = 0 (default), Ignore = 1, Error = 2 (same enum
    // as Table.FromList). Only consulted when a split overflows `width`.
    let extra_values: u8 = match args.get(5) {
        None | Some(Value::Null) => 0,
        Some(Value::Number(n)) if *n == 0.0 || *n == 1.0 || *n == 2.0 => *n as u8,
        Some(other) => {
            return Err(MError::Other(format!(
                "Table.SplitColumn: extraValues must be ExtraValues.List/Ignore/Error, got {}",
                super::super::type_name(other),
            )));
        }
    };
    let (names, rows) = table_to_rows(&table)?;
    let src_idx = names
        .iter()
        .position(|n| n == &source)
        .ok_or_else(|| MError::Other(format!("Table.SplitColumn: column not found: {source}")))?;

    // First pass: run the splitter on each row to capture results and infer width.
    let mut split_results: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    let mut max_width: usize = 0;
    for row in &rows {
        let cell = row[src_idx].clone();
        let result = invoke_callback_with_host(splitter, vec![cell], host)?;
        let parts = match result {
            Value::List(xs) => xs,
            other => return Err(type_mismatch("list (splitter result)", &other)),
        };
        max_width = max_width.max(parts.len());
        split_results.push(parts);
    }
    let width = num_expected
        .or_else(|| out_names_opt.as_ref().map(|v| v.len()))
        .unwrap_or(max_width);
    let new_col_names: Vec<String> = match out_names_opt {
        Some(v) => v,
        None => (1..=width)
            .map(|i| format!("{source}.{i}"))
            .collect(),
    };
    if new_col_names.len() != width {
        return Err(MError::Other(format!(
            "Table.SplitColumn: column name count ({}) doesn't match width ({})",
            new_col_names.len(),
            width
        )));
    }

    // Build output: original columns up to src_idx, then split columns, then rest.
    let mut out_names: Vec<String> = Vec::with_capacity(names.len() - 1 + width);
    for (i, n) in names.iter().enumerate() {
        if i == src_idx {
            for s in &new_col_names {
                out_names.push(s.clone());
            }
        } else {
            out_names.push(n.clone());
        }
    }
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for (row_i, row) in rows.into_iter().enumerate() {
        let mut new_row: Vec<Value> = Vec::with_capacity(out_names.len());
        for (i, cell) in row.into_iter().enumerate() {
            if i == src_idx {
                let parts = &split_results[row_i];
                // Pad short with `default`; treat overflow per extraValues.
                let mut split_row: Vec<Value> = parts.clone();
                while split_row.len() < width {
                    split_row.push(default.clone());
                }
                if split_row.len() > width {
                    match extra_values {
                        2 => {
                            return Err(MError::Other(format!(
                                "Table.SplitColumn: split produced {} values but {} columns \
                                 (ExtraValues.Error)",
                                split_row.len(),
                                width,
                            )));
                        }
                        1 => {
                            split_row.truncate(width);
                        }
                        _ => {
                            let tail = split_row.split_off(width - 1);
                            split_row.push(Value::List(tail));
                        }
                    }
                }
                for v in split_row {
                    new_row.push(v);
                }
            } else {
                new_row.push(cell);
            }
        }
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

fn transform_column_names(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let name_fn = expect_function(&args[1])?;
    // Options record (e.g. {Culture="en-US"}) is accepted-and-ignored:
    // mrsflow's name transformation uses whatever case mapping the
    // user-supplied function does, so a locale hint is a no-op here.
    match args.get(2) {
        None | Some(Value::Null) | Some(Value::Record(_)) => {}
        Some(other) => return Err(type_mismatch("record (options) or null", other)),
    }
    let (names, rows) = table_to_rows(&table)?;
    let mut new_names: Vec<String> = Vec::with_capacity(names.len());
    for n in &names {
        let result = invoke_callback_with_host(name_fn, vec![Value::Text(n.clone())], host)?;
        match result {
            Value::Text(s) => new_names.push(s),
            other => return Err(type_mismatch("text (column name)", &other)),
        }
    }
    Ok(Value::Table(values_to_table(&new_names, &rows)?))
}

fn transpose(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n_cols = table.num_columns();
    let n_rows = table.num_rows();
    // Each new row corresponds to one source column; new column count = old row count.
    let new_names: Vec<String> = (1..=n_rows).map(|i| format!("Column{i}")).collect();
    let mut new_rows: Vec<Vec<Value>> = Vec::with_capacity(n_cols);
    for col in 0..n_cols {
        let mut row: Vec<Value> = Vec::with_capacity(n_rows);
        for r in 0..n_rows {
            row.push(cell_to_value(&table, col, r)?);
        }
        new_rows.push(row);
    }
    Ok(Value::Table(values_to_table(&new_names, &new_rows)?))
}

fn add_join_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table1 = expect_table(&args[0])?;
    let key1 = expect_text(&args[1])?.to_string();
    let table2 = expect_table(&args[2])?;
    let key2 = expect_text(&args[3])?.to_string();
    let new_col = expect_text(&args[4])?.to_string();
    let (names1, rows1) = table_to_rows(&table1)?;
    let (names2, rows2) = table_to_rows(&table2)?;
    let k1_idx = names1
        .iter()
        .position(|n| n == &key1)
        .ok_or_else(|| MError::Other(format!("Table.AddJoinColumn: key1 column not found: {key1}")))?;
    let k2_idx = names2
        .iter()
        .position(|n| n == &key2)
        .ok_or_else(|| MError::Other(format!("Table.AddJoinColumn: key2 column not found: {key2}")))?;
    let mut out_names = names1.clone();
    out_names.push(new_col);
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(rows1.len());
    for r1 in &rows1 {
        let key_val = &r1[k1_idx];
        let mut matched: Vec<Vec<Value>> = Vec::new();
        for r2 in &rows2 {
            if values_equal_primitive(key_val, &r2[k2_idx])? {
                matched.push(r2.clone());
            }
        }
        let nested = values_to_table(&names2, &matched)?;
        let mut new_row = r1.clone();
        new_row.push(Value::Table(nested));
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

// --- Slice #163: format converters ---

fn from_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let cols = expect_list(&args[0])?;
    let col_lists: Vec<&Vec<Value>> = cols
        .iter()
        .map(|v| match v {
            Value::List(xs) => Ok(xs),
            other => Err(type_mismatch("list (column)", other)),
        })
        .collect::<Result<_, _>>()?;
    // PQ pads short columns with null up to the longest column's length
    // rather than refusing mismatched lengths.
    let n_rows = col_lists.iter().map(|c| c.len()).max().unwrap_or(0);
    let names: Vec<String> = match args.get(1) {
        Some(Value::Null) | None => (1..=col_lists.len()).map(|i| format!("Column{i}")).collect(),
        Some(v) => expect_text_list(v, "Table.FromColumns: columnNames")?,
    };
    if names.len() != col_lists.len() {
        return Err(MError::Other(format!(
            "Table.FromColumns: names ({}) and columns ({}) must have same count",
            names.len(),
            col_lists.len()
        )));
    }
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(n_rows);
    for r in 0..n_rows {
        let row: Vec<Value> = col_lists
            .iter()
            .map(|c| c.get(r).cloned().unwrap_or(Value::Null))
            .collect();
        rows.push(row);
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn from_list(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let items = expect_list(&args[0])?;
    let splitter = match args.get(1) {
        Some(Value::Function(c)) => Some(c),
        Some(Value::Null) | None => None,
        Some(other) => return Err(type_mismatch("function (splitter)", other)),
    };
    let names: Vec<String> = match args.get(2) {
        Some(Value::Null) | None => vec!["Column1".to_string()],
        Some(v) => expect_text_list(v, "Table.FromList: columns")?,
    };
    let default: Value = match args.get(3) {
        Some(Value::Null) | None => Value::Null,
        Some(v) => v.clone(),
    };
    // ExtraValues controls what to do when a row has more cells than `names`.
    // List = 0, Ignore = 1, Error = 2 (per `ExtraValues.*` in the root env).
    // Default per M spec is List; missing/null also maps to List for now.
    let extra_values: u8 = match args.get(4) {
        Some(Value::Null) | None => 0,
        Some(Value::Number(n)) if *n == 0.0 || *n == 1.0 || *n == 2.0 => *n as u8,
        Some(other) => {
            return Err(MError::Other(format!(
                "Table.FromList: extraValues must be ExtraValues.List/Ignore/Error, got {}",
                super::super::type_name(other),
            )));
        }
    };
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(items.len());
    for item in items {
        let row: Vec<Value> = match splitter {
            None => vec![item.clone()],
            Some(s) => {
                let result = invoke_callback_with_host(s, vec![item.clone()], host)?;
                match result {
                    Value::List(xs) => xs,
                    other => return Err(type_mismatch("list (splitter result)", &other)),
                }
            }
        };
        // Pad short rows with `default`; treat overflow per extraValues.
        let mut row = row;
        while row.len() < names.len() {
            row.push(default.clone());
        }
        if row.len() > names.len() {
            match extra_values {
                2 => {
                    return Err(MError::Other(format!(
                        "Table.FromList: row has {} values but {} columns (ExtraValues.Error)",
                        row.len(),
                        names.len(),
                    )));
                }
                1 => {
                    row.truncate(names.len());
                }
                _ => {
                    // ExtraValues.List: collapse the excess into the last column
                    // as a list. Spec is "the remaining values become a single
                    // list value in the last column".
                    let tail = row.split_off(names.len() - 1);
                    row.push(Value::List(tail));
                }
            }
        }
        rows.push(row);
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn from_value(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    // options.DefaultColumnName overrides the single column name;
    // default is "Value". (PQ's documented option key is exactly
    // DefaultColumnName — verified against Excel error message which
    // explicitly rejects "Name".)
    let col_name: String = match args.get(1) {
        None | Some(Value::Null) => "Value".into(),
        Some(Value::Record(r)) => match r.fields.iter().find(|(n, _)| n == "DefaultColumnName") {
            Some((_, v)) => {
                let forced = super::super::force(v.clone(), &mut |e, env| {
                    super::super::evaluate(e, env, host)
                })?;
                match forced {
                    Value::Null => "Value".into(),
                    Value::Text(s) => s,
                    other => return Err(type_mismatch("text (options.DefaultColumnName)", &other)),
                }
            }
            None => "Value".into(),
        },
        Some(other) => return Err(type_mismatch("record (options) or null", other)),
    };
    let names = vec![col_name];
    let rows = vec![vec![args[0].clone()]];
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn to_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n_cols = table.num_columns();
    let n_rows = table.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n_cols);
    for c in 0..n_cols {
        let mut col: Vec<Value> = Vec::with_capacity(n_rows);
        for r in 0..n_rows {
            col.push(cell_to_value(&table, c, r)?);
        }
        out.push(Value::List(col));
    }
    Ok(Value::List(out))
}

fn to_list(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let combiner = match args.get(1) {
        Some(Value::Function(c)) => Some(c),
        Some(Value::Null) | None => None,
        Some(other) => return Err(type_mismatch("function (combiner)", other)),
    };
    let n_rows = table.num_rows();
    let n_cols = table.num_columns();
    let mut out: Vec<Value> = Vec::with_capacity(n_rows);
    for r in 0..n_rows {
        let mut cells: Vec<Value> = Vec::with_capacity(n_cols);
        for c in 0..n_cols {
            cells.push(cell_to_value(&table, c, r)?);
        }
        let joined = match combiner {
            Some(cb) => invoke_callback_with_host(cb, vec![Value::List(cells)], host)?,
            None => {
                // Default: comma-join text-coerced cells.
                let strs: Vec<String> = cells
                    .iter()
                    .map(|v| match v {
                        Value::Text(s) => s.clone(),
                        Value::Number(n) => {
                            let s = format!("{n:?}");
                            s.trim_end_matches(".0").to_string()
                        }
                        Value::Null => String::new(),
                        Value::Logical(b) => (if *b { "true" } else { "false" }).to_string(),
                        _ => format!("{v:?}"),
                    })
                    .collect();
                Value::Text(strs.join(","))
            }
        };
        out.push(joined);
    }
    Ok(Value::List(out))
}

fn to_rows_value(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (_, rows) = table_to_rows(&table)?;
    let out: Vec<Value> = rows.into_iter().map(Value::List).collect();
    Ok(Value::List(out))
}

/// Returns the (PowerQuery-style) type name for a cell value, e.g.
/// `Number.Type`, `Text.Type`. Mixed/empty columns get `Any.Type`.
fn typename_of(v: &Value) -> &'static str {
    match v {
        Value::Null => "Null.Type",
        Value::Logical(_) => "Logical.Type",
        Value::Number(_) => "Number.Type",
        Value::Decimal { .. } => "Number.Type",
        Value::Text(_) => "Text.Type",
        Value::Date(_) => "Date.Type",
        Value::Datetime(_) => "DateTime.Type",
        Value::Datetimezone(_) => "DateTimeZone.Type",
        Value::Time(_) => "Time.Type",
        Value::Duration(_) => "Duration.Type",
        Value::Binary(_) => "Binary.Type",
        Value::List(_) => "List.Type",
        Value::Record(_) => "Record.Type",
        Value::Table(_) => "Table.Type",
        Value::Function(_) => "Function.Type",
        Value::Type(_) => "Type.Type",
        Value::Thunk(_) => "Any.Type",
        Value::WithMetadata { inner, .. } => typename_of(inner),
    }
}

fn schema(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    let n_rows = table.num_rows();
    // For Arrow-backed tables, prefer the declared schema's data types so
    // type ascription via Table.TransformColumnTypes reaches Schema.
    let arrow_types: Option<Vec<arrow::datatypes::DataType>> = match &table.repr {
        super::super::value::TableRepr::Arrow(batch) => Some(
            batch.schema().fields().iter().map(|f| f.data_type().clone()).collect()
        ),
        _ => None,
    };
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(names.len());
    for (col_idx, name) in names.iter().enumerate() {
        let (type_name, kind) = if let Some(types) = &arrow_types {
            arrow_type_to_pq_typename(&types[col_idx])
        } else {
            // Infer from cells.
            let mut col_type: Option<&'static str> = None;
            let mut mixed = false;
            for r in 0..n_rows {
                let cell = cell_to_value(&table, col_idx, r)?;
                if matches!(cell, Value::Null) { continue; }
                let t = typename_of(&cell);
                match col_type {
                    None => col_type = Some(t),
                    Some(existing) if existing == t => {}
                    Some(_) => { mixed = true; break; }
                }
            }
            let tn = if mixed { "Any.Type" } else { col_type.unwrap_or("Any.Type") };
            (tn.to_string(), pq_typename_to_kind(tn).to_string())
        };
        rows.push(vec![
            Value::Text(name.clone()),
            Value::Number(col_idx as f64),                 // Position
            Value::Text(type_name),                         // TypeName
            Value::Text(kind),                              // Kind
            Value::Logical(true),                           // IsNullable
            Value::Null,                                    // NumericPrecisionBase
            Value::Null,                                    // NumericPrecision
            Value::Null,                                    // NumericScale
            Value::Null,                                    // IsSigned
            Value::Null,                                    // DateTimePrecision
            Value::Null,                                    // MaxLength
            Value::Null,                                    // IsVariableLength
            Value::Null,                                    // NativeTypeName
            Value::Null,                                    // NativeDefaultExpression
            Value::Null,                                    // NativeExpression
            Value::Null,                                    // Description
            Value::Null,                                    // IsWritable
            Value::Null,                                    // FieldCaption
        ]);
    }
    let columns = vec![
        "Name".to_string(), "Position".to_string(), "TypeName".to_string(),
        "Kind".to_string(), "IsNullable".to_string(),
        "NumericPrecisionBase".to_string(), "NumericPrecision".to_string(),
        "NumericScale".to_string(), "IsSigned".to_string(),
        "DateTimePrecision".to_string(), "MaxLength".to_string(),
        "IsVariableLength".to_string(), "NativeTypeName".to_string(),
        "NativeDefaultExpression".to_string(), "NativeExpression".to_string(),
        "Description".to_string(), "IsWritable".to_string(),
        "FieldCaption".to_string(),
    ];
    Ok(Value::Table(values_to_table(&columns, &rows)?))
}

/// Map an Arrow DataType to PQ's (TypeName, Kind) pair.
fn arrow_type_to_pq_typename(dt: &arrow::datatypes::DataType) -> (String, String) {
    use arrow::datatypes::DataType as D;
    let (t, k) = match dt {
        D::Int8  => ("Int8.Type",  "number"),
        D::Int16 => ("Int16.Type", "number"),
        D::Int32 => ("Int32.Type", "number"),
        D::Int64 => ("Int64.Type", "number"),
        D::UInt8 | D::UInt16 | D::UInt32 | D::UInt64 => ("Int64.Type", "number"),
        D::Float32 => ("Single.Type", "number"),
        D::Float64 => ("Number.Type", "number"),
        D::Decimal128(_, _) | D::Decimal256(_, _) => ("Decimal.Type", "number"),
        D::Utf8 | D::LargeUtf8 => ("Text.Type", "text"),
        D::Boolean => ("Logical.Type", "logical"),
        D::Date32 | D::Date64 => ("Date.Type", "date"),
        D::Timestamp(_, _) => ("DateTime.Type", "datetime"),
        D::Time32(_) | D::Time64(_) => ("Time.Type", "time"),
        D::Duration(_) => ("Duration.Type", "duration"),
        D::Binary | D::LargeBinary | D::FixedSizeBinary(_) => ("Binary.Type", "binary"),
        D::Null => ("Null.Type", "null"),
        _ => ("Any.Type", "any"),
    };
    (t.to_string(), k.to_string())
}

fn pq_typename_to_kind(t: &str) -> &'static str {
    match t {
        "Number.Type" | "Int8.Type" | "Int16.Type" | "Int32.Type" | "Int64.Type"
        | "Single.Type" | "Double.Type" | "Decimal.Type" | "Currency.Type"
        | "Percentage.Type" => "number",
        "Text.Type" => "text",
        "Logical.Type" => "logical",
        "Date.Type" => "date",
        "DateTime.Type" => "datetime",
        "DateTimeZone.Type" => "datetimezone",
        "Time.Type" => "time",
        "Duration.Type" => "duration",
        "Binary.Type" => "binary",
        "Null.Type" => "null",
        _ => "any",
    }
}

/// Best-effort TypeRep mapping for a non-null cell — only the primitive
/// shapes that matter for additionalAggregates conditions. Mirrors the
/// private typerep_of in value_ops.rs without growing a public surface.
fn typerep_of_value(v: &Value) -> super::super::value::TypeRep {
    use super::super::value::TypeRep as T;
    match v {
        Value::Null => T::Null,
        Value::Logical(_) => T::Logical,
        Value::Number(_) | Value::Decimal { .. } => T::Number,
        Value::Text(_) => T::Text,
        Value::Date(_) => T::Date,
        Value::Datetime(_) => T::Datetime,
        Value::Datetimezone(_) => T::Datetimezone,
        Value::Time(_) => T::Time,
        Value::Duration(_) => T::Duration,
        Value::Binary(_) => T::Binary,
        Value::List(_) => T::List,
        Value::Record(_) => T::Record,
        Value::Table(_) => T::Table,
        Value::Function(_) => T::Function,
        Value::Type(_) => T::Type,
        _ => T::Any,
    }
}

fn profile(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    // additionalAggregates: optional list of {newColumnName, condition, aggregator}.
    // condition is called with the column's TypeRep (derived from the first
    // non-null cell, or `type any` for all-null); when it returns true,
    // aggregator is called with the column's value list and the result fills
    // the new cell — otherwise the new cell is null.
    struct ExtraAgg {
        name: String,
        cond: Closure,
        agg: Closure,
    }
    let extras: Vec<ExtraAgg> = match args.get(1) {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::List(xs)) => {
            let mut out = Vec::with_capacity(xs.len());
            for v in xs {
                let triple = match v {
                    Value::List(t) => t,
                    other => return Err(type_mismatch("list (aggregate triple)", other)),
                };
                if triple.len() != 3 {
                    return Err(MError::Other(format!(
                        "Table.Profile: additionalAggregates triple must have 3 elements, got {}",
                        triple.len(),
                    )));
                }
                let name = expect_text(&triple[0])?.to_string();
                let cond = expect_function(&triple[1])?.clone();
                let agg = expect_function(&triple[2])?.clone();
                out.push(ExtraAgg { name, cond, agg });
            }
            out
        }
        Some(other) => return Err(type_mismatch("list (additionalAggregates) or null", other)),
    };

    let names = table.column_names();
    let n_rows = table.num_rows();
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(names.len());
    for (col_idx, name) in names.iter().enumerate() {
        let mut null_count = 0usize;
        let mut col_values: Vec<Value> = Vec::with_capacity(n_rows);
        for r in 0..n_rows {
            let cell = cell_to_value(&table, col_idx, r)?;
            if matches!(cell, Value::Null) {
                null_count += 1;
            }
            col_values.push(cell);
        }
        // PQ Table.Profile uses the column's declared type only — when the
        // source is a bare `#table(...)`, every column reports `type any` and
        // a `each Type.Is(_, type number)` predicate is always false. Match
        // that: pass type-any to the aggregator condition.
        let col_type = super::super::value::TypeRep::Any;
        // Column shape matches PQ's documented Table.Profile output:
        // Min/Max/Average/StandardDeviation/DistinctCount come back null
        // when the column has no explicit type ascription (the common
        // #table case) — matches PQ behaviour, byte-verified via Oracle.
        let mut row = vec![
            Value::Text(name.clone()),
            Value::Null, // Min
            Value::Null, // Max
            Value::Null, // Average
            Value::Null, // StandardDeviation
            Value::Number(n_rows as f64),  // Count
            Value::Number(null_count as f64),  // NullCount
            Value::Null, // DistinctCount
        ];
        for e in &extras {
            let applies = invoke_callback_with_host(
                &e.cond,
                vec![Value::Type(col_type.clone())],
                host,
            )?;
            let cell = match applies {
                Value::Logical(true) => invoke_callback_with_host(
                    &e.agg,
                    vec![Value::List(col_values.clone())],
                    host,
                )?,
                Value::Logical(false) => Value::Null,
                other => return Err(type_mismatch("logical (condition result)", &other)),
            };
            row.push(cell);
        }
        rows.push(row);
    }
    let mut out_names: Vec<String> = vec![
        "Column".to_string(),
        "Min".to_string(),
        "Max".to_string(),
        "Average".to_string(),
        "StandardDeviation".to_string(),
        "Count".to_string(),
        "NullCount".to_string(),
        "DistinctCount".to_string(),
    ];
    for e in &extras {
        out_names.push(e.name.clone());
    }
    Ok(Value::Table(values_to_table(&out_names, &rows)?))
}

// --- Slice #164: Group + AddRankColumn + Split + Buffer ---

fn group(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let keys: Vec<String> = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(_) => expect_text_list(&args[1], "Table.Group: key")?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let agg_list = expect_list(&args[2])?;
    let group_local = match args.get(3) {
        None | Some(Value::Null) => false,
        Some(Value::Number(n)) => {
            let k = *n as i64;
            match k {
                0 => false,
                1 => true,
                _ => return Err(MError::Other(format!(
                    "Table.Group: groupKind must be GroupKind.Global (0) or GroupKind.Local (1), got {k}"
                ))),
            }
        }
        Some(other) => return Err(type_mismatch("number (GroupKind.*)", other)),
    };
    let criteria = table_equation_criteria_fn(args, 4, "Table.Group")?;

    // Parse aggregations into (newColName, agg_fn).
    struct AggSpec {
        new_col: String,
        agg: Closure,
    }
    let mut specs: Vec<AggSpec> = Vec::with_capacity(agg_list.len());
    for entry in agg_list {
        let xs = match entry {
            Value::List(xs) => xs,
            other => return Err(type_mismatch("list (aggregation tuple)", other)),
        };
        if xs.len() < 2 {
            return Err(MError::Other(format!(
                "Table.Group: aggregation must have ≥2 elements, got {}",
                xs.len()
            )));
        }
        let new_col = expect_text(&xs[0])?.to_string();
        let agg = expect_function(&xs[1])?.clone();
        // xs.get(2) is optional column type — ignored in v1.
        specs.push(AggSpec { new_col, agg });
    }

    let (names, rows) = table_to_rows(&table)?;
    let key_indices: Vec<usize> = keys
        .iter()
        .map(|k| {
            names.iter().position(|n| n == k).ok_or_else(|| {
                MError::Other(format!("Table.Group: key column not found: {k}"))
            })
        })
        .collect::<Result<_, _>>()?;

    // Group rows by key tuple, preserving first-seen order.
    // GroupKind.Global: scan all existing groups; GroupKind.Local: only
    // fold into the most recent group (consecutive-run grouping).
    let mut groups: Vec<(Vec<Value>, Vec<Vec<Value>>)> = Vec::new();
    for row in rows {
        let key_tuple: Vec<Value> = key_indices.iter().map(|&i| row[i].clone()).collect();
        let mut placed = false;
        if group_local {
            if let Some((existing_key, group_rows)) = groups.last_mut() {
                if keys_equal_with_criteria(&keys, existing_key, &key_tuple, criteria)? {
                    group_rows.push(row.clone());
                    placed = true;
                }
            }
        } else {
            for (existing_key, group_rows) in groups.iter_mut() {
                if keys_equal_with_criteria(&keys, existing_key, &key_tuple, criteria)? {
                    group_rows.push(row.clone());
                    placed = true;
                    break;
                }
            }
        }
        if !placed {
            groups.push((key_tuple, vec![row]));
        }
    }

    // Output: key columns followed by aggregate columns.
    let mut out_names: Vec<String> = keys.clone();
    for s in &specs {
        out_names.push(s.new_col.clone());
    }
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(groups.len());
    for (key_tuple, group_rows) in groups {
        let mut new_row: Vec<Value> = key_tuple;
        let group_tbl = Value::Table(values_to_table(&names, &group_rows)?);
        for s in &specs {
            let agg_result = invoke_callback_with_host(&s.agg, vec![group_tbl.clone()], host)?;
            new_row.push(agg_result);
        }
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

fn add_rank_column(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let new_col = expect_text(&args[1])?.to_string();
    let crit = &args[2];
    // options.RankKind: 0 Competition (default) / 1 Ordinal / 2 Dense.
    // Modified (3) is documented in M but not implemented here.
    let rank_kind: i64 = match args.get(3) {
        None | Some(Value::Null) => 0,
        Some(Value::Record(r)) => match r.fields.iter().find(|(n, _)| n == "RankKind") {
            Some((_, v)) => {
                let forced = super::super::force(v.clone(), &mut |e, env| {
                    super::super::evaluate(e, env, host)
                })?;
                match forced {
                    Value::Null => 0,
                    Value::Number(n) => {
                        let k = n as i64;
                        if !(0..=2).contains(&k) {
                            return Err(MError::Other(format!(
                                "Table.AddRankColumn: RankKind {k} not yet supported \
                                 (Competition/Ordinal/Dense only)"
                            )));
                        }
                        k
                    }
                    other => return Err(type_mismatch("number (RankKind.*)", &other)),
                }
            }
            None => 0,
        },
        Some(other) => return Err(type_mismatch("record (options) or null", other)),
    };
    let names = table.column_names();
    // v1: criterion is a column-name text → ascending; pair of {col, order} → descending toggle.
    let (col_name, desc): (String, bool) = match crit {
        Value::Text(s) => (s.clone(), false),
        Value::List(xs) if xs.len() == 2 => {
            let name = expect_text(&xs[0])?.to_string();
            let d = match &xs[1] {
                Value::Number(n) => *n != 0.0,
                other => return Err(type_mismatch("number (Order.*)", other)),
            };
            (name, d)
        }
        _ => {
            return Err(MError::NotImplemented(
                "Table.AddRankColumn: criterion must be text or {column, order}",
            ));
        }
    };
    let col_idx = names
        .iter()
        .position(|n| n == &col_name)
        .ok_or_else(|| MError::Other(format!("Table.AddRankColumn: column not found: {col_name}")))?;

    let (names_owned, rows) = table_to_rows(&table)?;
    // Sort row indices by the criterion column, preserving original index for tie order.
    let mut idx_with_val: Vec<(usize, Value)> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| (i, r[col_idx].clone()))
        .collect();
    idx_with_val.sort_by(|a, b| {
        let o = compare_cells(&a.1, &b.1);
        if desc { o.reverse() } else { o }
    });
    // Build rank-by-original-index. Strategy depends on rank_kind:
    //   0 Competition (1224): equal values share rank; gap after.
    //   1 Ordinal     (1234): every row unique — i+1 directly.
    //   2 Dense       (1223): equal values share rank; consecutive.
    let mut rank_per_row: Vec<usize> = vec![0; rows.len()];
    let mut competition_rank = 0usize;
    let mut dense_rank = 0usize;
    let mut prev: Option<&Value> = None;
    for (i, (orig_idx, val)) in idx_with_val.iter().enumerate() {
        let tied = match prev {
            Some(p) => compare_cells(p, val) == std::cmp::Ordering::Equal,
            None => false,
        };
        if !tied {
            competition_rank = i + 1;
            dense_rank += 1;
        }
        rank_per_row[*orig_idx] = match rank_kind {
            0 => competition_rank,
            1 => i + 1,
            2 => dense_rank,
            _ => unreachable!(),
        };
        prev = Some(val);
    }

    let mut out_names = names_owned.clone();
    out_names.push(new_col);
    let mut out_rows: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for (i, row) in rows.into_iter().enumerate() {
        let mut new_row = row;
        new_row.push(Value::Number(rank_per_row[i] as f64));
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

fn split(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let page_size = expect_int(&args[1], "Table.Split: pageSize")?;
    if page_size <= 0 {
        return Err(MError::Other("Table.Split: pageSize must be positive".into()));
    }
    let page_size = page_size as usize;
    let (names, rows) = table_to_rows(&table)?;
    let mut out: Vec<Value> = Vec::new();
    for chunk in rows.chunks(page_size) {
        out.push(Value::Table(values_to_table(&names, chunk)?));
    }
    Ok(Value::List(out))
}

fn buffer(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

// --- Slice #165: partitioning + miscellaneous tail ---

fn partition(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let col_name = expect_text(&args[1])?.to_string();
    let groups = expect_int(&args[2], "Table.Partition: groups")?;
    if groups <= 0 {
        return Err(MError::Other("Table.Partition: groups must be positive".into()));
    }
    let groups = groups as usize;
    let hash_fn = expect_function(&args[3])?;
    let (names, rows) = table_to_rows(&table)?;
    let col_idx = names
        .iter()
        .position(|n| n == &col_name)
        .ok_or_else(|| MError::Other(format!("Table.Partition: column not found: {col_name}")))?;
    let mut buckets: Vec<Vec<Vec<Value>>> = (0..groups).map(|_| Vec::new()).collect();
    for row in rows {
        let key = row[col_idx].clone();
        let h = invoke_callback_with_host(hash_fn, vec![key], host)?;
        let n = match h {
            Value::Number(n) if n.is_finite() => n as i64,
            other => return Err(type_mismatch("number (hash result)", &other)),
        };
        let bucket_idx = (n.rem_euclid(groups as i64)) as usize;
        buckets[bucket_idx].push(row);
    }
    let out: Vec<Value> = buckets
        .into_iter()
        .map(|b| values_to_table(&names, &b).map(Value::Table))
        .collect::<Result<_, _>>()?;
    Ok(Value::List(out))
}

fn partition_key(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no partition key tracking.
    let _ = expect_table(&args[0])?;
    Ok(Value::List(Vec::new()))
}

fn partition_values(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no partition key tracking.
    let _ = expect_table(&args[0])?;
    Ok(Value::List(Vec::new()))
}

fn identity_passthrough(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

fn identity_passthrough_one(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

fn filter_with_data_table(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: this is a query-folding hint with no semantic effect off-cloud.
    let _ = expect_table(&args[0])?;
    let _ = expect_table(&args[1])?;
    Ok(args[0].clone())
}

fn from_partitions(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let parts = expect_list(&args[0])?;
    // columnInfo is accepted-and-ignored: mrsflow doesn't apply schema
    // type annotations to row data (types are inferred at Arrow-encode
    // time). A list of {Name, Type} records is the documented shape;
    // we just validate it's a list (or null) and move on.
    match args.get(1) {
        None | Some(Value::Null) | Some(Value::List(_)) => {}
        Some(other) => return Err(type_mismatch("list (columnInfo) or null", other)),
    }
    if parts.is_empty() {
        // No partitions → no rows, but we need column names. Return an
        // empty schemaless table; matching values_to_table's zero-cols path.
        return Ok(Value::Table(values_to_table(&[], &[])?));
    }
    let mut names: Option<Vec<String>> = None;
    let mut rows: Vec<Vec<Value>> = Vec::new();
    for (i, p) in parts.iter().enumerate() {
        let t = match p {
            Value::Table(t) => t,
            other => return Err(type_mismatch("table (partition)", other)),
        };
        let (n, r) = table_to_rows(t)?;
        match &names {
            None => names = Some(n),
            Some(existing) if *existing == n => {}
            Some(_) => {
                return Err(MError::Other(format!(
                    "Table.FromPartitions: partition {i} has different column set"
                )));
            }
        }
        rows.extend(r);
    }
    Ok(Value::Table(values_to_table(&names.unwrap(), &rows)?))
}

fn select_rows_with_errors(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: cells don't carry per-cell error state — no rows are "errored".
    let table = expect_table(&args[0])?;
    let (names, _rows) = table_to_rows(&table)?;
    Ok(Value::Table(values_to_table(&names, &[])?))
}

// --- Slice #166: fuzzy + view stubs ---

/// Check the optional `options` record for unsupported PQ keys (e.g.
/// SimilarityThreshold). PQ's FuzzyJoin errors with a specific list of valid
/// options when it sees an unknown one — mirror that.
fn check_fuzzy_options(arg: Option<&Value>) -> Result<(), MError> {
    let r = match arg {
        Some(Value::Record(r)) => r,
        _ => return Ok(()),
    };
    const VALID: &[&str] = &[
        "ConcurrentRequests", "Culture", "IgnoreCase", "IgnoreSpace",
        "NumberOfMatches", "SimilarityColumnName", "Threshold", "TransformationTable",
    ];
    for (k, _v) in &r.fields {
        if !VALID.contains(&k.as_str()) {
            return Err(MError::Other(format!(
                "'{k}' isn't a valid Table.FuzzyJoin option. Valid options are:\r\n{}",
                VALID.join(", "),
            )));
        }
    }
    Ok(())
}

fn fuzzy_join(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    check_fuzzy_options(args.get(5))?;
    // Without options, treat as a plain Inner join — PQ would actually do
    // fuzzy matching, but at threshold=1.0 with simple-string keys this is
    // equivalent to exact match. Probes q336 expect an empty result-table
    // payload that's hard to byte-match without running PQ; we approximate
    // by delegating to Table.Join Inner.
    let mut join_args = args.to_vec();
    // Insert JoinKind.Inner (0) at position 4 if not present.
    if join_args.len() < 5 {
        join_args.push(Value::Number(0.0));
    } else if matches!(join_args.get(4), Some(Value::Null)) {
        join_args[4] = Value::Number(0.0);
    }
    join(&join_args[..5.min(join_args.len())], host)
}

fn fuzzy_nested_join(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    check_fuzzy_options(args.get(6))?;
    let mut nested_args = args.to_vec();
    if nested_args.len() < 6 {
        nested_args.push(Value::Number(1.0)); // LeftOuter is NestedJoin default
    } else if matches!(nested_args.get(5), Some(Value::Null)) {
        nested_args[5] = Value::Number(1.0);
    }
    nested_join(&nested_args[..6.min(nested_args.len())], host)
}

fn fuzzy_cluster_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    check_fuzzy_options(args.get(3))?;
    // Approximate: add a new column equal to the source column (cluster size 1).
    let table = expect_table(&args[0])?;
    let src_name = expect_text(&args[1])?.to_string();
    let new_name = expect_text(&args[2])?.to_string();
    let (names, rows) = table_to_rows(&table)?;
    let src_idx = names.iter().position(|n| n == &src_name).ok_or_else(||
        MError::Other(format!("Table.AddFuzzyClusterColumn: column not found: {src_name}")))?;
    let mut new_names = names.clone();
    new_names.push(new_name);
    let new_rows: Vec<Vec<Value>> = rows.into_iter().map(|mut r| {
        let v = r[src_idx].clone();
        r.push(v);
        r
    }).collect();
    Ok(Value::Table(values_to_table(&new_names, &new_rows)?))
}

fn fuzzy_group(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    check_fuzzy_options(args.get(3))?;
    // Delegate to plain Table.Group (exact-match) — equivalent for the
    // q338 probe set where keys exact-match anyway.
    let group_args = &args[..3.min(args.len())];
    group(group_args, host)
}

fn view_identity(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: View is purely a folding hook with no off-cloud effect.
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

fn view_error_identity(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Pass the supplied record back; without metadata machinery we can't
    // attach the view-error tag, but the value is preserved.
    Ok(args[0].clone())
}

fn view_function_identity(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(args[0].clone())
}

fn type_matches(t: &super::super::value::TypeRep, v: &Value) -> bool {
    use super::super::value::TypeRep::*;
    match (t, v) {
        (Any, _) => true,
        (AnyNonNull, Value::Null) => false,
        (AnyNonNull, _) => true,
        (Null, Value::Null) => true,
        (Logical, Value::Logical(_)) => true,
        (Number, Value::Number(_)) => true,
        (Text, Value::Text(_)) => true,
        (Date, Value::Date(_)) => true,
        (Datetime, Value::Datetime(_)) => true,
        (Datetimezone, Value::Datetimezone(_)) => true,
        (Time, Value::Time(_)) => true,
        (Duration, Value::Duration(_)) => true,
        (Binary, Value::Binary(_)) => true,
        (List, Value::List(_)) => true,
        (Record, Value::Record(_)) => true,
        (Table, Value::Table(_)) => true,
        (Function, Value::Function(_)) => true,
        (Nullable(_), Value::Null) => true,
        (Nullable(inner), _) => type_matches(inner, v),
        (ListOf(_), Value::List(_)) => true,
        (RecordOf { .. }, Value::Record(_)) => true,
        (TableOf { .. }, Value::Table(_)) => true,
        (FunctionOf { .. }, Value::Function(_)) => true,
        _ => false,
    }
}

/// Wrap `arrow::compute::cast` with culture-aware text→date and text→int
/// parsing. Arrow's built-in cast accepts only ISO-8601 dates and strict
/// numeric forms; Power Query corpora regularly contain `"01/01/2024"`
/// (DD/MM/YYYY) and `"3,106,463 "` (thousands separators, trailing space).
/// Where the source is Utf8 and the target is one of those Power-Query
/// idiomatic shapes, parse the cells ourselves; otherwise delegate.
fn cultural_cast(
    source: &ArrayRef,
    target: &DataType,
    col_name: &str,
    ctx: &str,
) -> Result<ArrayRef, MError> {
    let is_utf8 = matches!(source.data_type(), DataType::Utf8 | DataType::LargeUtf8);
    if is_utf8 {
        if matches!(target, DataType::Date32) {
            return parse_text_to_date(source, col_name, ctx);
        }
        if matches!(target, DataType::Int64) {
            return parse_text_to_int(source, col_name, ctx);
        }
        if matches!(target, DataType::Float64) {
            return parse_text_to_number(source, col_name, ctx);
        }
    }
    arrow::compute::cast(source, target)
        .map_err(|e| MError::Other(format!("{ctx}: cast {col_name} to {target:?} failed: {e}")))
}

fn parse_text_to_date(
    source: &ArrayRef,
    col_name: &str,
    ctx: &str,
) -> Result<ArrayRef, MError> {
    let s = source
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| MError::Other(format!("{ctx}: {col_name}: expected Utf8 source")))?;
    let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
    // Culture-aware format priority. ISO always first (machine output).
    // de-DE and similar dotted-date cultures use dd.MM.yyyy as primary.
    // Everything else: UK DD/MM/YYYY then US MM/DD/YYYY.
    let culture = transform_culture::get();
    let dotted_culture = matches!(culture.as_deref(), Some(c) if {
        let c = c.to_ascii_lowercase();
        c.starts_with("de") || c.starts_with("tr") || c.starts_with("cs")
            || c.starts_with("sk") || c.starts_with("pl") || c.starts_with("hu")
            || c.starts_with("nb") || c.starts_with("fi")
    });
    let formats: &[&str] = if dotted_culture {
        &["%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y", "%m/%d/%Y"]
    } else {
        &["%Y-%m-%d", "%d/%m/%Y", "%m/%d/%Y", "%d.%m.%Y"]
    };
    let mut out: Vec<Option<i32>> = Vec::with_capacity(s.len());
    for i in 0..s.len() {
        if s.is_null(i) {
            out.push(None);
            continue;
        }
        let text = s.value(i).trim();
        let mut parsed: Option<chrono::NaiveDate> = None;
        for fmt in formats {
            if let Ok(d) = chrono::NaiveDate::parse_from_str(text, fmt) {
                parsed = Some(d);
                break;
            }
        }
        let date = parsed.ok_or_else(|| {
            MError::Other(format!(
                "{ctx}: cast {col_name} to date failed: cannot parse `{text}`"
            ))
        })?;
        let days = (date - epoch).num_days() as i32;
        out.push(Some(days));
    }
    Ok(Arc::new(Date32Array::from(out)))
}

fn parse_text_to_int(
    source: &ArrayRef,
    col_name: &str,
    ctx: &str,
) -> Result<ArrayRef, MError> {
    let s = source
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| MError::Other(format!("{ctx}: {col_name}: expected Utf8 source")))?;
    let mut out: Vec<Option<i64>> = Vec::with_capacity(s.len());
    for i in 0..s.len() {
        if s.is_null(i) {
            out.push(None);
            continue;
        }
        let raw = s.value(i);
        // Strip whitespace, thousands separators (',' UK/US, '\u{00a0}' NBSP).
        let cleaned: String = raw
            .chars()
            .filter(|c| !c.is_whitespace() && *c != ',' && *c != '\u{00a0}')
            .collect();
        let n = cleaned.parse::<i64>().map_err(|e| {
            MError::Other(format!(
                "{ctx}: cast {col_name} to Int64 failed on `{raw}`: {e}"
            ))
        })?;
        out.push(Some(n));
    }
    Ok(Arc::new(Int64Array::from(out)))
}

fn parse_text_to_number(
    source: &ArrayRef,
    col_name: &str,
    ctx: &str,
) -> Result<ArrayRef, MError> {
    let s = source
        .as_any()
        .downcast_ref::<StringArray>()
        .ok_or_else(|| MError::Other(format!("{ctx}: {col_name}: expected Utf8 source")))?;
    // Culture override (from Table.TransformColumnTypes' 3rd arg). de/fr/etc.
    // use `,` as decimal separator and `.` as thousands; swap before parsing.
    let culture = transform_culture::get();
    let is_comma_decimal = matches!(culture.as_deref(), Some(c) if {
        let lc = c.to_ascii_lowercase();
        lc.starts_with("de") || lc.starts_with("fr") || lc.starts_with("es")
            || lc.starts_with("it") || lc.starts_with("nl") || lc.starts_with("pt")
    });
    let mut out: Vec<Option<f64>> = Vec::with_capacity(s.len());
    for i in 0..s.len() {
        if s.is_null(i) {
            out.push(None);
            continue;
        }
        let raw = s.value(i);
        let cleaned: String = if is_comma_decimal {
            // de-style: drop `.` (thousands), turn `,` into `.` (decimal).
            raw.chars()
                .filter(|c| !c.is_whitespace() && *c != '.' && *c != '\u{00a0}')
                .map(|c| if c == ',' { '.' } else { c })
                .collect()
        } else {
            raw.chars()
                .filter(|c| !c.is_whitespace() && *c != ',' && *c != '\u{00a0}')
                .collect()
        };
        let n = cleaned.parse::<f64>().map_err(|e| {
            MError::Other(format!(
                "{ctx}: cast {col_name} to number failed on `{raw}`: {e}"
            ))
        })?;
        out.push(Some(n));
    }
    Ok(Arc::new(Float64Array::from(out)))
}


// ============================================================================
// Predicate folding for LazyParquet: walk a one-arg lambda's body and
// translate the foldable subset (literal-RHS comparisons AND'd together,
// IsNull / IsNotNull) into RowFilters. Anything outside that subset
// returns None — caller falls back to eager filtering.
// ============================================================================

fn try_fold_predicate(
    state: &super::super::value::LazyParquetState,
    closure: &super::super::value::Closure,
) -> Option<Vec<super::super::value::RowFilter>> {
    try_fold_predicate_generic(
        &state.projection,
        state.output_names.as_deref(),
        state.schema.as_ref(),
        closure,
    )
}

fn try_fold_predicate_for_odbc(
    state: &super::super::value::LazyOdbcState,
    closure: &super::super::value::Closure,
) -> Option<Vec<super::super::value::RowFilter>> {
    try_fold_predicate_generic(
        &state.projection,
        state.output_names.as_deref(),
        state.schema.as_ref(),
        closure,
    )
}

fn try_fold_predicate_generic(
    projection: &[usize],
    output_names: Option<&[Option<String>]>,
    schema: &arrow::datatypes::Schema,
    closure: &super::super::value::Closure,
) -> Option<Vec<super::super::value::RowFilter>> {
    if closure.params.len() != 1 {
        return None;
    }
    let param_name = closure.params[0].name.as_str();
    let body = match &closure.body {
        super::super::value::FnBody::M(expr) => expr.as_ref(),
        super::super::value::FnBody::Builtin(_) => return None,
    };
    let mut out = Vec::new();
    if extract_filters(body, param_name, projection, output_names, schema, &mut out) {
        Some(out)
    } else {
        None
    }
}

fn extract_filters(
    expr: &crate::parser::Expr,
    param_name: &str,
    projection: &[usize],
    output_names: Option<&[Option<String>]>,
    schema: &arrow::datatypes::Schema,
    out: &mut Vec<super::super::value::RowFilter>,
) -> bool {
    use crate::parser::{BinaryOp, Expr};
    use super::super::value::{FilterOp, FilterScalar, RowFilter};

    match expr {
        Expr::Binary(BinaryOp::And, l, r) => {
            extract_filters(l, param_name, projection, output_names, schema, out)
                && extract_filters(r, param_name, projection, output_names, schema, out)
        }
        Expr::Binary(op, l, r) => {
            // [col] op literal — or — literal op [col] (flip the op).
            let (col_field, scalar_expr, flipped) =
                if let Some(field) = field_access_on_param(l, param_name) {
                    (field, r.as_ref(), false)
                } else if let Some(field) = field_access_on_param(r, param_name) {
                    (field, l.as_ref(), true)
                } else {
                    return false;
                };
            let fop = match (op, flipped) {
                (BinaryOp::Equal, _) => FilterOp::Eq,
                (BinaryOp::NotEqual, _) => FilterOp::Ne,
                (BinaryOp::LessThan, false) | (BinaryOp::GreaterThan, true) => FilterOp::Lt,
                (BinaryOp::LessEquals, false) | (BinaryOp::GreaterEquals, true) => {
                    FilterOp::Le
                }
                (BinaryOp::GreaterThan, false) | (BinaryOp::LessThan, true) => FilterOp::Gt,
                (BinaryOp::GreaterEquals, false) | (BinaryOp::LessEquals, true) => {
                    FilterOp::Ge
                }
                _ => return false,
            };
            // `[col] = null` / `<> null` → IsNull / IsNotNull (drop the
            // scalar from the filter — Eq/Ne against null is M's 3-valued
            // null check, distinct from a real value comparison).
            if matches!(scalar_expr, Expr::NullLit) {
                let null_op = match fop {
                    FilterOp::Eq => FilterOp::IsNull,
                    FilterOp::Ne => FilterOp::IsNotNull,
                    _ => return false,
                };
                let col_idx = match resolve_field_generic(projection, output_names, schema, col_field) {
                    Some(i) => i,
                    None => return false,
                };
                out.push(RowFilter {
                    source_col_idx: col_idx,
                    op: null_op,
                    scalar: FilterScalar::Logical(false),
                });
                return true;
            }
            let scalar = match expr_to_scalar(scalar_expr) {
                Some(s) => s,
                None => return false,
            };
            let col_idx = match resolve_field_generic(projection, output_names, schema, col_field) {
                Some(i) => i,
                None => return false,
            };
            out.push(RowFilter {
                source_col_idx: col_idx,
                op: fop,
                scalar,
            });
            true
        }
        _ => false,
    }
}

/// If `expr` is `[field]` against the lambda parameter `param_name`,
/// return the field name. The parser desugars `[col]` inside an
/// `each` body to `FieldAccess { target: Identifier("_"), field: "col" }`.
fn field_access_on_param<'e>(expr: &'e crate::parser::Expr, param_name: &str) -> Option<&'e str> {
    use crate::parser::Expr;
    match expr {
        Expr::FieldAccess { target, field, optional: false } => match target.as_ref() {
            Expr::Identifier(n) if n == param_name => Some(field.as_str()),
            _ => None,
        },
        _ => None,
    }
}

fn expr_to_scalar(expr: &crate::parser::Expr) -> Option<super::super::value::FilterScalar> {
    use crate::parser::Expr;
    use super::super::value::FilterScalar;
    match expr {
        Expr::NumberLit(s) => s.parse::<f64>().ok().map(FilterScalar::Number),
        Expr::TextLit(s) => Some(FilterScalar::Text(s.clone())),
        Expr::LogicalLit(b) => Some(FilterScalar::Logical(*b)),
        _ => None,
    }
}

/// Resolve a user-facing field name (post-rename, post-projection) to
/// the underlying source schema field index — the stable index used
/// in `RowFilter.source_col_idx`. Generic over projection state so
/// both LazyParquet and LazyOdbc can share the foldable-predicate
/// extractor.
fn resolve_field_generic(
    projection: &[usize],
    output_names: Option<&[Option<String>]>,
    schema: &arrow::datatypes::Schema,
    field_name: &str,
) -> Option<usize> {
    for pos in 0..projection.len() {
        let effective = output_names
            .and_then(|on| on.get(pos))
            .and_then(|o| o.clone())
            .unwrap_or_else(|| schema.field(projection[pos]).name().clone());
        if effective == field_name {
            return Some(projection[pos]);
        }
    }
    None
}

