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
        // --- Accessors + predicates batch (slice #158) ---
        (
            "Table.First",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            table_first,
        ),
        (
            "Table.Last",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            table_last,
        ),
        (
            "Table.FirstValue",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            table_first_value,
        ),
        ("Table.RowCount", one("table"), table_row_count),
        ("Table.ColumnCount", one("table"), table_column_count),
        ("Table.ApproximateRowCount", one("table"), table_row_count),
        (
            "Table.Range",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            table_range,
        ),
        (
            "Table.Contains",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "row".into(),              optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_contains,
        ),
        (
            "Table.ContainsAll",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "rows".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_contains_all,
        ),
        (
            "Table.ContainsAny",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "rows".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_contains_any,
        ),
        (
            "Table.IsDistinct",
            vec![
                Param { name: "table".into(),              optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_is_distinct,
        ),
        ("Table.HasColumns", two("table", "columns"), table_has_columns),
        ("Table.MatchesAllRows", two("table", "condition"), table_matches_all_rows),
        ("Table.MatchesAnyRows", two("table", "condition"), table_matches_any_rows),
        ("Table.FindText", two("table", "text"), table_find_text),
        (
            "Table.PositionOf",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "row".into(),              optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_position_of,
        ),
        (
            "Table.PositionOfAny",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "rows".into(),             optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_position_of_any,
        ),
        ("Table.Keys", one("table"), table_keys),
        ("Table.ColumnsOfType", two("table", "listOfTypes"), table_columns_of_type),
        // --- Slice #159: sort/fill/reverse ---
        ("Table.Sort", two("table", "comparisonCriteria"), table_sort),
        ("Table.FillUp", two("table", "columns"), table_fill_up),
        ("Table.FillDown", two("table", "columns"), table_fill_down),
        ("Table.ReverseRows", one("table"), table_reverse_rows),
        ("Table.SplitAt", two("table", "index"), table_split_at),
        (
            "Table.AlternateRows",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "skip".into(),   optional: false, type_annotation: None },
                Param { name: "take".into(),   optional: false, type_annotation: None },
            ],
            table_alternate_rows,
        ),
        ("Table.Repeat", two("table", "count"), table_repeat),
        ("Table.SingleRow", one("table"), table_single_row),
        // --- Slice #160: aggregations ---
        (
            "Table.Min",
            vec![
                Param { name: "table".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: false, type_annotation: None },
                Param { name: "default".into(),             optional: true,  type_annotation: None },
            ],
            table_min,
        ),
        (
            "Table.Max",
            vec![
                Param { name: "table".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: false, type_annotation: None },
                Param { name: "default".into(),             optional: true,  type_annotation: None },
            ],
            table_max,
        ),
        (
            "Table.MinN",
            three("table", "countOrCondition", "comparisonCriteria"),
            table_min_n,
        ),
        (
            "Table.MaxN",
            three("table", "countOrCondition", "comparisonCriteria"),
            table_max_n,
        ),
        (
            "Table.AggregateTableColumn",
            three("table", "column", "aggregations"),
            table_aggregate_table_column,
        ),
        // --- Slice #161: row mutation ---
        (
            "Table.RemoveFirstN",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            table_remove_first_n,
        ),
        (
            "Table.RemoveLastN",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            table_remove_last_n,
        ),
        (
            "Table.RemoveRows",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            table_remove_rows,
        ),
        (
            "Table.RemoveMatchingRows",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "rows".into(),             optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_remove_matching_rows,
        ),
        (
            "Table.RemoveRowsWithErrors",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "columns".into(), optional: true,  type_annotation: None },
            ],
            table_remove_rows_with_errors,
        ),
        (
            "Table.ReplaceMatchingRows",
            vec![
                Param { name: "table".into(),            optional: false, type_annotation: None },
                Param { name: "replacements".into(),     optional: false, type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            table_replace_matching_rows,
        ),
        (
            "Table.ReplaceRows",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: false, type_annotation: None },
                Param { name: "rows".into(),   optional: false, type_annotation: None },
            ],
            table_replace_rows,
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
            table_replace_value,
        ),
        (
            "Table.ReplaceErrorValues",
            two("table", "errorReplacement"),
            table_replace_error_values,
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
            table_combine_columns,
        ),
        (
            "Table.CombineColumnsToRecord",
            three("table", "newColumnName", "sourceColumns"),
            table_combine_columns_to_record,
        ),
        ("Table.DemoteHeaders", one("table"), table_demote_headers),
        (
            "Table.DuplicateColumn",
            three("table", "columnName", "newColumnName"),
            table_duplicate_column,
        ),
        ("Table.PrefixColumns", two("table", "prefix"), table_prefix_columns),
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
            table_split_column,
        ),
        (
            "Table.TransformColumnNames",
            vec![
                Param { name: "table".into(),         optional: false, type_annotation: None },
                Param { name: "nameGenerator".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),       optional: true,  type_annotation: None },
            ],
            table_transform_column_names,
        ),
        ("Table.Transpose", one("table"), table_transpose),
        (
            "Table.AddJoinColumn",
            vec![
                Param { name: "table1".into(),        optional: false, type_annotation: None },
                Param { name: "key1".into(),          optional: false, type_annotation: None },
                Param { name: "table2".into(),        optional: false, type_annotation: None },
                Param { name: "key2".into(),          optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
            ],
            table_add_join_column,
        ),
        // --- Slice #163: format converters ---
        (
            "Table.FromColumns",
            vec![
                Param { name: "lists".into(),       optional: false, type_annotation: None },
                Param { name: "columnNames".into(), optional: true,  type_annotation: None },
            ],
            table_from_columns,
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
            table_from_list,
        ),
        (
            "Table.FromValue",
            vec![
                Param { name: "value".into(),   optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            table_from_value,
        ),
        ("Table.ToColumns", one("table"), table_to_columns),
        (
            "Table.ToList",
            vec![
                Param { name: "table".into(),    optional: false, type_annotation: None },
                Param { name: "combiner".into(), optional: true,  type_annotation: None },
            ],
            table_to_list,
        ),
        ("Table.ToRows", one("table"), table_to_rows_value),
        ("Table.Schema", one("table"), table_schema),
        (
            "Table.Profile",
            vec![
                Param { name: "table".into(),                optional: false, type_annotation: None },
                Param { name: "additionalAggregates".into(), optional: true,  type_annotation: None },
            ],
            table_profile,
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
            table_group,
        ),
        (
            "Table.AddRankColumn",
            vec![
                Param { name: "table".into(),              optional: false, type_annotation: None },
                Param { name: "newColumnName".into(),      optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),            optional: true,  type_annotation: None },
            ],
            table_add_rank_column,
        ),
        ("Table.Split", two("table", "pageSize"), table_split),
        ("Table.Buffer", one("table"), table_buffer),
        // --- Slice #165: partitioning + miscellaneous tail ---
        (
            "Table.Partition",
            vec![
                Param { name: "table".into(),  optional: false, type_annotation: None },
                Param { name: "column".into(), optional: false, type_annotation: None },
                Param { name: "groups".into(), optional: false, type_annotation: None },
                Param { name: "hash".into(),   optional: false, type_annotation: None },
            ],
            table_partition,
        ),
        ("Table.PartitionKey", one("table"), table_partition_key),
        ("Table.PartitionValues", one("table"), table_partition_values),
        ("Table.ReplacePartitionKey", two("table", "key"), table_identity_passthrough),
        (
            "Table.FilterWithDataTable",
            vec![
                Param { name: "table".into(),     optional: false, type_annotation: None },
                Param { name: "dataTable".into(), optional: false, type_annotation: None },
            ],
            table_filter_with_data_table,
        ),
        (
            "Table.FromPartitions",
            vec![
                Param { name: "partitions".into(), optional: false, type_annotation: None },
                Param { name: "columnInfo".into(), optional: true,  type_annotation: None },
            ],
            table_from_partitions,
        ),
        ("Table.AddKey", three("table", "columns", "isPrimary"), table_identity_passthrough),
        ("Table.ReplaceKeys", two("table", "keys"), table_identity_passthrough),
        ("Table.ConformToPageReader", one("table"), table_identity_passthrough_one),
        ("Table.StopFolding", one("table"), table_identity_passthrough_one),
        ("Table.ReplaceRelationshipIdentity", two("table", "identity"), table_identity_passthrough),
        (
            "Table.SelectRowsWithErrors",
            vec![
                Param { name: "table".into(),   optional: false, type_annotation: None },
                Param { name: "columns".into(), optional: true,  type_annotation: None },
            ],
            table_select_rows_with_errors,
        ),
        ("Table.WithErrorContext", two("table", "errorContext"), table_identity_passthrough),
        // --- Slice #166: fuzzy + view stubs ---
        (
            "Table.AddFuzzyClusterColumn",
            vec![
                Param { name: "table".into(),         optional: false, type_annotation: None },
                Param { name: "columnName".into(),    optional: false, type_annotation: None },
                Param { name: "newColumnName".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),       optional: true,  type_annotation: None },
            ],
            table_fuzzy_not_implemented,
        ),
        (
            "Table.FuzzyGroup",
            vec![
                Param { name: "table".into(),             optional: false, type_annotation: None },
                Param { name: "key".into(),               optional: false, type_annotation: None },
                Param { name: "aggregatedColumns".into(), optional: false, type_annotation: None },
                Param { name: "options".into(),           optional: true,  type_annotation: None },
            ],
            table_fuzzy_not_implemented,
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
            table_fuzzy_not_implemented,
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
            table_fuzzy_not_implemented,
        ),
        ("Table.View", two("table", "handlers"), table_view_identity),
        ("Table.ViewError", one("record"), table_view_error_identity),
        ("Table.ViewFunction", one("function"), table_view_function_identity),
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


pub(super) fn values_to_table(column_names: &[String], rows: &[Vec<Value>]) -> Result<Table, MError> {
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
        | TypeRep::Time | TypeRep::Datetimezone
        | TypeRep::ListOf(_) | TypeRep::RecordOf { .. } | TypeRep::TableOf { .. }
        | TypeRep::FunctionOf { .. } => {
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

// --- Slice #158: accessors + predicates batch ---

fn table_first(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.num_rows() == 0 {
        return Ok(args.get(1).cloned().unwrap_or(Value::Null));
    }
    row_to_record(table, 0)
}

fn table_last(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = table.num_rows();
    if n == 0 {
        return Ok(args.get(1).cloned().unwrap_or(Value::Null));
    }
    row_to_record(table, n - 1)
}

fn table_first_value(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.num_rows() == 0 || table.num_columns() == 0 {
        return Ok(args.get(1).cloned().unwrap_or(Value::Null));
    }
    cell_to_value(table, 0, 0)
}

fn table_row_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    Ok(Value::Number(table.num_rows() as f64))
}

fn table_column_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    Ok(Value::Number(table.num_columns() as f64))
}

fn table_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = expect_int(&args[1], "Table.Range: offset")?;
    if offset < 0 {
        return Err(MError::Other("Table.Range: offset must be non-negative".into()));
    }
    let offset = offset as usize;
    let (names, rows) = table_to_rows(table)?;
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
        let cell = cell_to_value(table, col, row)?;
        let expected = super::super::force(expected.clone(), &mut |e, env| {
            super::super::evaluate(e, env, &super::super::NoIoHost)
        })?;
        if !values_equal_primitive(&cell, &expected)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn table_contains(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needle = match &args[1] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.Contains: equationCriteria not yet supported",
        ));
    }
    for row in 0..table.num_rows() {
        if row_matches_record(table, row, needle)? {
            return Ok(Value::Logical(true));
        }
    }
    Ok(Value::Logical(false))
}

fn table_contains_all(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needles = expect_list(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.ContainsAll: equationCriteria not yet supported",
        ));
    }
    for n in needles {
        let needle = match n {
            Value::Record(r) => r,
            other => return Err(type_mismatch("record (in list)", other)),
        };
        let mut found = false;
        for row in 0..table.num_rows() {
            if row_matches_record(table, row, needle)? {
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

fn table_contains_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needles = expect_list(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.ContainsAny: equationCriteria not yet supported",
        ));
    }
    for n in needles {
        let needle = match n {
            Value::Record(r) => r,
            other => return Err(type_mismatch("record (in list)", other)),
        };
        for row in 0..table.num_rows() {
            if row_matches_record(table, row, needle)? {
                return Ok(Value::Logical(true));
            }
        }
    }
    Ok(Value::Logical(false))
}

fn table_is_distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.IsDistinct: comparisonCriteria not yet supported",
        ));
    }
    let (_, rows) = table_to_rows(table)?;
    for i in 0..rows.len() {
        for j in (i + 1)..rows.len() {
            let mut all_eq = true;
            for (a, b) in rows[i].iter().zip(rows[j].iter()) {
                if !values_equal_primitive(a, b)? {
                    all_eq = false;
                    break;
                }
            }
            if all_eq {
                return Ok(Value::Logical(false));
            }
        }
    }
    Ok(Value::Logical(true))
}

fn table_has_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(_) => expect_text_list(&args[1], "Table.HasColumns")?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let have = table.column_names();
    let all_present = names.iter().all(|n| have.iter().any(|h| h == n));
    Ok(Value::Logical(all_present))
}

fn table_matches_all_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let cond = expect_function(&args[1])?;
    for row in 0..table.num_rows() {
        let rec = row_to_record(table, row)?;
        let result = invoke_callback_with_host(cond, vec![rec], host)?;
        match result {
            Value::Logical(true) => continue,
            Value::Logical(false) => return Ok(Value::Logical(false)),
            other => return Err(type_mismatch("logical (predicate result)", &other)),
        }
    }
    Ok(Value::Logical(true))
}

fn table_matches_any_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let cond = expect_function(&args[1])?;
    for row in 0..table.num_rows() {
        let rec = row_to_record(table, row)?;
        let result = invoke_callback_with_host(cond, vec![rec], host)?;
        match result {
            Value::Logical(true) => return Ok(Value::Logical(true)),
            Value::Logical(false) => continue,
            other => return Err(type_mismatch("logical (predicate result)", &other)),
        }
    }
    Ok(Value::Logical(false))
}

fn table_find_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needle = expect_text(&args[1])?;
    let (names, rows) = table_to_rows(table)?;
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

fn table_position_of(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needle = match &args[1] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.PositionOf: occurrence not yet supported",
        ));
    }
    if !matches!(args.get(3), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.PositionOf: equationCriteria not yet supported",
        ));
    }
    for row in 0..table.num_rows() {
        if row_matches_record(table, row, needle)? {
            return Ok(Value::Number(row as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

fn table_position_of_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needles_v = expect_list(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.PositionOfAny: occurrence not yet supported",
        ));
    }
    if !matches!(args.get(3), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.PositionOfAny: equationCriteria not yet supported",
        ));
    }
    for row in 0..table.num_rows() {
        for n in needles_v {
            let needle = match n {
                Value::Record(r) => r,
                other => return Err(type_mismatch("record (in list)", other)),
            };
            if row_matches_record(table, row, needle)? {
                return Ok(Value::Number(row as f64));
            }
        }
    }
    Ok(Value::Number(-1.0))
}

fn table_keys(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: we don't track key metadata — return an empty list.
    let _ = expect_table(&args[0])?;
    Ok(Value::List(Vec::new()))
}

fn table_columns_of_type(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let type_list = expect_list(&args[1])?;
    let mut targets: Vec<super::super::value::TypeRep> = Vec::with_capacity(type_list.len());
    for v in type_list {
        match v {
            Value::Type(t) => targets.push(t.clone()),
            other => return Err(type_mismatch("type (in list)", other)),
        }
    }
    let names = table.column_names();
    let mut out: Vec<Value> = Vec::new();
    'col: for (col_idx, name) in names.iter().enumerate() {
        // Inspect each cell's value and check it matches any target type.
        // Empty columns (all null) — skip; we can't infer a type.
        let mut saw_non_null = false;
        for row in 0..table.num_rows() {
            let cell = cell_to_value(table, col_idx, row)?;
            if matches!(cell, Value::Null) {
                continue;
            }
            saw_non_null = true;
            if !targets.iter().any(|t| type_matches(t, &cell)) {
                continue 'col;
            }
        }
        if saw_non_null {
            out.push(Value::Text(name.clone()));
        }
    }
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

fn table_sort(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    // Parse criteria into a list of (column_index, descending) tuples.
    let mut keys: Vec<(usize, bool)> = Vec::new();
    let pairs: Vec<&Value> = match &args[1] {
        Value::Text(_) => vec![&args[1]],
        Value::List(xs) => xs.iter().collect(),
        other => return Err(type_mismatch("text or list (sort criteria)", other)),
    };
    for p in pairs {
        let (col_name, desc) = match p {
            Value::Text(s) => (s.clone(), false),
            Value::List(inner) => {
                if inner.len() != 2 {
                    return Err(MError::Other(format!(
                        "Table.Sort: criterion pair must have 2 elements, got {}",
                        inner.len()
                    )));
                }
                let n = match &inner[0] {
                    Value::Text(s) => s.clone(),
                    other => return Err(type_mismatch("text (column name)", other)),
                };
                let d = match &inner[1] {
                    Value::Number(n) => *n != 0.0,
                    other => return Err(type_mismatch("number (Order.*)", other)),
                };
                (n, d)
            }
            other => return Err(type_mismatch("text or pair (sort criterion)", other)),
        };
        let idx = names
            .iter()
            .position(|n| n == &col_name)
            .ok_or_else(|| MError::Other(format!("Table.Sort: column not found: {}", col_name)))?;
        keys.push((idx, desc));
    }
    let (_, mut rows) = table_to_rows(table)?;
    rows.sort_by(|a, b| {
        for &(col, desc) in &keys {
            let ord = compare_cells(&a[col], &b[col]);
            if ord != std::cmp::Ordering::Equal {
                return if desc { ord.reverse() } else { ord };
            }
        }
        std::cmp::Ordering::Equal
    });
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
            .ok_or_else(|| MError::Other(format!("{}: column not found: {}", ctx, n)))?;
        out.push(idx);
    }
    Ok(out)
}

fn table_fill_down(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (names, mut rows) = table_to_rows(table)?;
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

fn table_fill_up(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (names, mut rows) = table_to_rows(table)?;
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

fn table_reverse_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (names, mut rows) = table_to_rows(table)?;
    rows.reverse();
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn table_split_at(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let index = expect_int(&args[1], "Table.SplitAt: index")?;
    if index < 0 {
        return Err(MError::Other("Table.SplitAt: index must be non-negative".into()));
    }
    let split = (index as usize).min(table.num_rows());
    let (names, rows) = table_to_rows(table)?;
    let (head, tail) = rows.split_at(split);
    let head_tbl = values_to_table(&names, head)?;
    let tail_tbl = values_to_table(&names, tail)?;
    Ok(Value::List(vec![
        Value::Table(head_tbl),
        Value::Table(tail_tbl),
    ]))
}

fn table_alternate_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
    let (names, rows) = table_to_rows(table)?;
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

fn table_repeat(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let count = expect_int(&args[1], "Table.Repeat: count")?;
    if count < 0 {
        return Err(MError::Other("Table.Repeat: count must be non-negative".into()));
    }
    let (names, rows) = table_to_rows(table)?;
    let mut out: Vec<Vec<Value>> = Vec::with_capacity(rows.len() * count as usize);
    for _ in 0..count {
        for r in &rows {
            out.push(r.clone());
        }
    }
    Ok(Value::Table(values_to_table(&names, &out)?))
}

fn table_single_row(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if table.num_rows() != 1 {
        return Err(MError::Other(format!(
            "Table.SingleRow: expected exactly 1 row, got {}",
            table.num_rows()
        )));
    }
    row_to_record(table, 0)
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
            .ok_or_else(|| MError::Other(format!("{}: column not found: {}", ctx, name))),
        _ => Err(MError::NotImplemented(
            "Table.Min/Max: comparisonCriteria must be a text column name in v1",
        )),
    }
}

fn table_min(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    if table.num_rows() == 0 {
        return Ok(args.get(2).cloned().unwrap_or(Value::Null));
    }
    let col = parse_min_max_criteria(&args[1], &names, "Table.Min")?;
    let mut best: usize = 0;
    for row in 1..table.num_rows() {
        let a = cell_to_value(table, col, best)?;
        let b = cell_to_value(table, col, row)?;
        if compare_cells(&b, &a) == std::cmp::Ordering::Less {
            best = row;
        }
    }
    row_to_record(table, best)
}

fn table_max(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    if table.num_rows() == 0 {
        return Ok(args.get(2).cloned().unwrap_or(Value::Null));
    }
    let col = parse_min_max_criteria(&args[1], &names, "Table.Max")?;
    let mut best: usize = 0;
    for row in 1..table.num_rows() {
        let a = cell_to_value(table, col, best)?;
        let b = cell_to_value(table, col, row)?;
        if compare_cells(&b, &a) == std::cmp::Ordering::Greater {
            best = row;
        }
    }
    row_to_record(table, best)
}

fn min_max_n_count(arg: &Value, ctx: &str) -> Result<usize, MError> {
    match arg {
        Value::Number(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => Ok(*n as usize),
        _ => Err(MError::Other(format!(
            "{}: countOrCondition must be a non-negative integer in v1",
            ctx
        ))),
    }
}

fn table_min_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = min_max_n_count(&args[1], "Table.MinN")?;
    let names = table.column_names();
    let col = parse_min_max_criteria(&args[2], &names, "Table.MinN")?;
    let (names_owned, mut rows) = table_to_rows(table)?;
    rows.sort_by(|a, b| compare_cells(&a[col], &b[col]));
    let kept: Vec<Vec<Value>> = rows.into_iter().take(n).collect();
    Ok(Value::Table(values_to_table(&names_owned, &kept)?))
}

fn table_max_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = min_max_n_count(&args[1], "Table.MaxN")?;
    let names = table.column_names();
    let col = parse_min_max_criteria(&args[2], &names, "Table.MaxN")?;
    let (names_owned, mut rows) = table_to_rows(table)?;
    rows.sort_by(|a, b| compare_cells(&b[col], &a[col])); // descending
    let kept: Vec<Vec<Value>> = rows.into_iter().take(n).collect();
    Ok(Value::Table(values_to_table(&names_owned, &kept)?))
}

fn table_aggregate_table_column(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let col_name = expect_text(&args[1])?.to_string();
    let agg_list = expect_list(&args[2])?;
    let names = table.column_names();
    let target_idx = names
        .iter()
        .position(|n| n == &col_name)
        .ok_or_else(|| {
            MError::Other(format!(
                "Table.AggregateTableColumn: column not found: {}",
                col_name
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

    let (_, rows) = table_to_rows(table)?;
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

fn table_remove_first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = parse_optional_count(args.get(1), 1, "Table.RemoveFirstN: count")?;
    let (names, rows) = table_to_rows(table)?;
    let kept: Vec<Vec<Value>> = rows.into_iter().skip(n).collect();
    Ok(Value::Table(values_to_table(&names, &kept)?))
}

fn table_remove_last_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n = parse_optional_count(args.get(1), 1, "Table.RemoveLastN: count")?;
    let (names, mut rows) = table_to_rows(table)?;
    let n = n.min(rows.len());
    rows.truncate(rows.len() - n);
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn table_remove_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = expect_int(&args[1], "Table.RemoveRows: offset")?;
    if offset < 0 {
        return Err(MError::Other("Table.RemoveRows: offset must be non-negative".into()));
    }
    let count = parse_optional_count(args.get(2), 1, "Table.RemoveRows: count")?;
    let offset = offset as usize;
    let (names, mut rows) = table_to_rows(table)?;
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

fn table_remove_matching_rows(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let needles = expect_list(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.RemoveMatchingRows: equationCriteria not yet supported",
        ));
    }
    let (names, rows) = table_to_rows(table)?;
    let mut kept: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    'row: for row in rows {
        for n in needles {
            let needle = match n {
                Value::Record(r) => r,
                other => return Err(type_mismatch("record (in list)", other)),
            };
            if row_matches_full_record(&names, &row, needle)? {
                continue 'row;
            }
        }
        kept.push(row);
    }
    Ok(Value::Table(values_to_table(&names, &kept)?))
}

fn table_remove_rows_with_errors(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: cells don't carry per-cell error state, so this is a no-op.
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

fn table_replace_matching_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let pairs = expect_list(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.ReplaceMatchingRows: equationCriteria not yet supported",
        ));
    }
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
    let (names, rows) = table_to_rows(table)?;
    let mut out: Vec<Vec<Value>> = Vec::with_capacity(rows.len());
    for row in rows {
        let mut replaced = false;
        for p in &owned {
            if row_matches_full_record(&names, &row, &p.old)? {
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

fn table_replace_rows(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let offset = expect_int(&args[1], "Table.ReplaceRows: offset")?;
    let count = expect_int(&args[2], "Table.ReplaceRows: count")?;
    if offset < 0 || count < 0 {
        return Err(MError::Other(
            "Table.ReplaceRows: offset/count must be non-negative".into(),
        ));
    }
    let new_records = expect_list(&args[3])?;
    let (names, mut rows) = table_to_rows(table)?;
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

fn table_replace_value(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let old_value = args[1].clone();
    let new_value = args[2].clone();
    let replacer = expect_function(&args[3])?;
    let cols_to_search = match &args[4] {
        Value::Text(s) => vec![s.clone()],
        Value::List(_) => expect_text_list(&args[4], "Table.ReplaceValue: columnsToSearch")?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let (names, mut rows) = table_to_rows(table)?;
    let mut col_indices: Vec<usize> = Vec::with_capacity(cols_to_search.len());
    for n in &cols_to_search {
        let idx = names
            .iter()
            .position(|h| h == n)
            .ok_or_else(|| MError::Other(format!("Table.ReplaceValue: column not found: {}", n)))?;
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

fn table_replace_error_values(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: cells don't carry per-cell error state, so this is a no-op.
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

// --- Slice #162: column mutation ---

fn table_combine_columns(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let sources = expect_text_list(&args[1], "Table.CombineColumns: sourceColumns")?;
    let combiner = expect_function(&args[2])?;
    let new_name = expect_text(&args[3])?.to_string();
    let (names, rows) = table_to_rows(table)?;
    let src_indices: Vec<usize> = sources
        .iter()
        .map(|n| {
            names
                .iter()
                .position(|h| h == n)
                .ok_or_else(|| MError::Other(format!("Table.CombineColumns: column not found: {}", n)))
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

fn table_combine_columns_to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let new_name = expect_text(&args[1])?.to_string();
    let sources = expect_text_list(&args[2], "Table.CombineColumnsToRecord: sourceColumns")?;
    let (names, rows) = table_to_rows(table)?;
    let src_indices: Vec<usize> = sources
        .iter()
        .map(|n| {
            names.iter().position(|h| h == n).ok_or_else(|| {
                MError::Other(format!(
                    "Table.CombineColumnsToRecord: column not found: {}",
                    n
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

fn table_demote_headers(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    let n_cols = names.len();
    // New headers: Column1, Column2, ...
    let new_names: Vec<String> = (1..=n_cols).map(|i| format!("Column{}", i)).collect();
    // First row: original header names as text cells.
    let header_row: Vec<Value> = names.iter().cloned().map(Value::Text).collect();
    let (_, mut rows) = table_to_rows(table)?;
    rows.insert(0, header_row);
    Ok(Value::Table(values_to_table(&new_names, &rows)?))
}

fn table_duplicate_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let src = expect_text(&args[1])?.to_string();
    let new_name = expect_text(&args[2])?.to_string();
    let (names, rows) = table_to_rows(table)?;
    let idx = names
        .iter()
        .position(|n| n == &src)
        .ok_or_else(|| MError::Other(format!("Table.DuplicateColumn: column not found: {}", src)))?;
    if names.iter().any(|n| n == &new_name) {
        return Err(MError::Other(format!(
            "Table.DuplicateColumn: new column name already exists: {}",
            new_name
        )));
    }
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

fn table_prefix_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let prefix = expect_text(&args[1])?;
    let (names, rows) = table_to_rows(table)?;
    let new_names: Vec<String> = names.iter().map(|n| format!("{}.{}", prefix, n)).collect();
    Ok(Value::Table(values_to_table(&new_names, &rows)?))
}

fn table_split_column(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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
    if !matches!(args.get(4), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.SplitColumn: default argument not yet supported",
        ));
    }
    if !matches!(args.get(5), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.SplitColumn: extraValues argument not yet supported",
        ));
    }
    let (names, rows) = table_to_rows(table)?;
    let src_idx = names
        .iter()
        .position(|n| n == &source)
        .ok_or_else(|| MError::Other(format!("Table.SplitColumn: column not found: {}", source)))?;

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
            .map(|i| format!("{}.{}", source, i))
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
                for j in 0..width {
                    new_row.push(parts.get(j).cloned().unwrap_or(Value::Null));
                }
            } else {
                new_row.push(cell);
            }
        }
        out_rows.push(new_row);
    }
    Ok(Value::Table(values_to_table(&out_names, &out_rows)?))
}

fn table_transform_column_names(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let gen = expect_function(&args[1])?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.TransformColumnNames: options not yet supported",
        ));
    }
    let (names, rows) = table_to_rows(table)?;
    let mut new_names: Vec<String> = Vec::with_capacity(names.len());
    for n in &names {
        let result = invoke_callback_with_host(gen, vec![Value::Text(n.clone())], host)?;
        match result {
            Value::Text(s) => new_names.push(s),
            other => return Err(type_mismatch("text (column name)", &other)),
        }
    }
    Ok(Value::Table(values_to_table(&new_names, &rows)?))
}

fn table_transpose(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n_cols = table.num_columns();
    let n_rows = table.num_rows();
    // Each new row corresponds to one source column; new column count = old row count.
    let new_names: Vec<String> = (1..=n_rows).map(|i| format!("Column{}", i)).collect();
    let mut new_rows: Vec<Vec<Value>> = Vec::with_capacity(n_cols);
    for col in 0..n_cols {
        let mut row: Vec<Value> = Vec::with_capacity(n_rows);
        for r in 0..n_rows {
            row.push(cell_to_value(table, col, r)?);
        }
        new_rows.push(row);
    }
    Ok(Value::Table(values_to_table(&new_names, &new_rows)?))
}

fn table_add_join_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table1 = expect_table(&args[0])?;
    let key1 = expect_text(&args[1])?.to_string();
    let table2 = expect_table(&args[2])?;
    let key2 = expect_text(&args[3])?.to_string();
    let new_col = expect_text(&args[4])?.to_string();
    let (names1, rows1) = table_to_rows(table1)?;
    let (names2, rows2) = table_to_rows(table2)?;
    let k1_idx = names1
        .iter()
        .position(|n| n == &key1)
        .ok_or_else(|| MError::Other(format!("Table.AddJoinColumn: key1 column not found: {}", key1)))?;
    let k2_idx = names2
        .iter()
        .position(|n| n == &key2)
        .ok_or_else(|| MError::Other(format!("Table.AddJoinColumn: key2 column not found: {}", key2)))?;
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

fn table_from_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let cols = expect_list(&args[0])?;
    // Each column is a list. Their lengths must match.
    let col_lists: Vec<&Vec<Value>> = cols
        .iter()
        .map(|v| match v {
            Value::List(xs) => Ok(xs),
            other => Err(type_mismatch("list (column)", other)),
        })
        .collect::<Result<_, _>>()?;
    let n_rows = col_lists.first().map(|c| c.len()).unwrap_or(0);
    for (i, col) in col_lists.iter().enumerate() {
        if col.len() != n_rows {
            return Err(MError::Other(format!(
                "Table.FromColumns: column {} length {} doesn't match column 0 length {}",
                i, col.len(), n_rows
            )));
        }
    }
    let names: Vec<String> = match args.get(1) {
        Some(Value::Null) | None => (1..=col_lists.len()).map(|i| format!("Column{}", i)).collect(),
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
        let row: Vec<Value> = col_lists.iter().map(|c| c[r].clone()).collect();
        rows.push(row);
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn table_from_list(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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
    if !matches!(args.get(3), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.FromList: default argument not yet supported",
        ));
    }
    if !matches!(args.get(4), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.FromList: extraValues argument not yet supported",
        ));
    }
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
        // Pad or truncate row to names.len().
        let mut row = row;
        while row.len() < names.len() {
            row.push(Value::Null);
        }
        row.truncate(names.len());
        rows.push(row);
    }
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn table_from_value(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.FromValue: options not yet supported",
        ));
    }
    let names = vec!["Value".to_string()];
    let rows = vec![vec![args[0].clone()]];
    Ok(Value::Table(values_to_table(&names, &rows)?))
}

fn table_to_columns(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let n_cols = table.num_columns();
    let n_rows = table.num_rows();
    let mut out: Vec<Value> = Vec::with_capacity(n_cols);
    for c in 0..n_cols {
        let mut col: Vec<Value> = Vec::with_capacity(n_rows);
        for r in 0..n_rows {
            col.push(cell_to_value(table, c, r)?);
        }
        out.push(Value::List(col));
    }
    Ok(Value::List(out))
}

fn table_to_list(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
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
            cells.push(cell_to_value(table, c, r)?);
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
                            let s = format!("{:?}", n);
                            s.trim_end_matches(".0").to_string()
                        }
                        Value::Null => String::new(),
                        Value::Logical(b) => (if *b { "true" } else { "false" }).to_string(),
                        _ => format!("{:?}", v),
                    })
                    .collect();
                Value::Text(strs.join(","))
            }
        };
        out.push(joined);
    }
    Ok(Value::List(out))
}

fn table_to_rows_value(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let (_, rows) = table_to_rows(table)?;
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
    }
}

fn table_schema(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let names = table.column_names();
    let n_rows = table.num_rows();
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(names.len());
    for (col_idx, name) in names.iter().enumerate() {
        // Infer column type from non-null cells. Mixed → Any.Type.
        let mut col_type: Option<&'static str> = None;
        let mut mixed = false;
        for r in 0..n_rows {
            let cell = cell_to_value(table, col_idx, r)?;
            if matches!(cell, Value::Null) {
                continue;
            }
            let t = typename_of(&cell);
            match col_type {
                None => col_type = Some(t),
                Some(existing) if existing == t => {}
                Some(_) => {
                    mixed = true;
                    break;
                }
            }
        }
        let type_name = if mixed {
            "Any.Type"
        } else {
            col_type.unwrap_or("Any.Type")
        };
        rows.push(vec![
            Value::Text(name.clone()),
            Value::Text(type_name.to_string()),
        ]);
    }
    Ok(Value::Table(values_to_table(
        &["Name".to_string(), "TypeName".to_string()],
        &rows,
    )?))
}

fn table_profile(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.Profile: additionalAggregates not yet supported",
        ));
    }
    let names = table.column_names();
    let n_rows = table.num_rows();
    let mut rows: Vec<Vec<Value>> = Vec::with_capacity(names.len());
    for (col_idx, name) in names.iter().enumerate() {
        let mut null_count = 0usize;
        let mut seen: Vec<Value> = Vec::new();
        for r in 0..n_rows {
            let cell = cell_to_value(table, col_idx, r)?;
            if matches!(cell, Value::Null) {
                null_count += 1;
                continue;
            }
            let already = seen.iter().any(|v| match values_equal_primitive(v, &cell) {
                Ok(b) => b,
                Err(_) => false,
            });
            if !already {
                seen.push(cell);
            }
        }
        rows.push(vec![
            Value::Text(name.clone()),
            Value::Number(n_rows as f64),
            Value::Number(null_count as f64),
            Value::Number(seen.len() as f64),
        ]);
    }
    Ok(Value::Table(values_to_table(
        &[
            "Column".to_string(),
            "Count".to_string(),
            "NullCount".to_string(),
            "DistinctCount".to_string(),
        ],
        &rows,
    )?))
}

// --- Slice #164: Group + AddRankColumn + Split + Buffer ---

fn table_group(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let keys: Vec<String> = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(_) => expect_text_list(&args[1], "Table.Group: key")?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let agg_list = expect_list(&args[2])?;
    if !matches!(args.get(3), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.Group: groupKind not yet supported",
        ));
    }
    if !matches!(args.get(4), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.Group: comparisonCriteria not yet supported",
        ));
    }

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

    let (names, rows) = table_to_rows(table)?;
    let key_indices: Vec<usize> = keys
        .iter()
        .map(|k| {
            names.iter().position(|n| n == k).ok_or_else(|| {
                MError::Other(format!("Table.Group: key column not found: {}", k))
            })
        })
        .collect::<Result<_, _>>()?;

    // Group rows by key tuple, preserving first-seen order.
    let mut groups: Vec<(Vec<Value>, Vec<Vec<Value>>)> = Vec::new();
    for row in rows {
        let key_tuple: Vec<Value> = key_indices.iter().map(|&i| row[i].clone()).collect();
        let mut placed = false;
        for (existing_key, group_rows) in groups.iter_mut() {
            let mut all_eq = true;
            for (a, b) in existing_key.iter().zip(key_tuple.iter()) {
                if !values_equal_primitive(a, b)? {
                    all_eq = false;
                    break;
                }
            }
            if all_eq {
                group_rows.push(row.clone());
                placed = true;
                break;
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

fn table_add_rank_column(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let new_col = expect_text(&args[1])?.to_string();
    let crit = &args[2];
    if !matches!(args.get(3), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.AddRankColumn: options not yet supported",
        ));
    }
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
        .ok_or_else(|| MError::Other(format!("Table.AddRankColumn: column not found: {}", col_name)))?;

    let (names_owned, rows) = table_to_rows(table)?;
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
    // Build rank-by-original-index using dense rank (equal values get equal rank).
    let mut rank_per_row: Vec<usize> = vec![0; rows.len()];
    let mut current_rank = 0usize;
    let mut prev: Option<&Value> = None;
    for (i, (orig_idx, val)) in idx_with_val.iter().enumerate() {
        match prev {
            Some(p) if compare_cells(p, val) == std::cmp::Ordering::Equal => {}
            _ => current_rank = i + 1,
        }
        rank_per_row[*orig_idx] = current_rank;
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

fn table_split(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let page_size = expect_int(&args[1], "Table.Split: pageSize")?;
    if page_size <= 0 {
        return Err(MError::Other("Table.Split: pageSize must be positive".into()));
    }
    let page_size = page_size as usize;
    let (names, rows) = table_to_rows(table)?;
    let mut out: Vec<Value> = Vec::new();
    for chunk in rows.chunks(page_size) {
        out.push(Value::Table(values_to_table(&names, chunk)?));
    }
    Ok(Value::List(out))
}

fn table_buffer(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

// --- Slice #165: partitioning + miscellaneous tail ---

fn table_partition(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let table = expect_table(&args[0])?;
    let col_name = expect_text(&args[1])?.to_string();
    let groups = expect_int(&args[2], "Table.Partition: groups")?;
    if groups <= 0 {
        return Err(MError::Other("Table.Partition: groups must be positive".into()));
    }
    let groups = groups as usize;
    let hash_fn = expect_function(&args[3])?;
    let (names, rows) = table_to_rows(table)?;
    let col_idx = names
        .iter()
        .position(|n| n == &col_name)
        .ok_or_else(|| MError::Other(format!("Table.Partition: column not found: {}", col_name)))?;
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

fn table_partition_key(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no partition key tracking.
    let _ = expect_table(&args[0])?;
    Ok(Value::List(Vec::new()))
}

fn table_partition_values(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: no partition key tracking.
    let _ = expect_table(&args[0])?;
    Ok(Value::List(Vec::new()))
}

fn table_identity_passthrough(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

fn table_identity_passthrough_one(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

fn table_filter_with_data_table(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: this is a query-folding hint with no semantic effect off-cloud.
    let _ = expect_table(&args[0])?;
    let _ = expect_table(&args[1])?;
    Ok(args[0].clone())
}

fn table_from_partitions(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let parts = expect_list(&args[0])?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Table.FromPartitions: columnInfo not yet supported",
        ));
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
                    "Table.FromPartitions: partition {} has different column set",
                    i
                )));
            }
        }
        rows.extend(r);
    }
    Ok(Value::Table(values_to_table(&names.unwrap(), &rows)?))
}

fn table_select_rows_with_errors(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: cells don't carry per-cell error state — no rows are "errored".
    let table = expect_table(&args[0])?;
    let (names, _rows) = table_to_rows(table)?;
    Ok(Value::Table(values_to_table(&names, &[])?))
}

// --- Slice #166: fuzzy + view stubs ---

fn table_fuzzy_not_implemented(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Err(MError::NotImplemented(
        "Table.Fuzzy*: fuzzy-match functions require similarity infra not built in v1",
    ))
}

fn table_view_identity(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // v1: View is purely a folding hook with no off-cloud effect.
    let _ = expect_table(&args[0])?;
    Ok(args[0].clone())
}

fn table_view_error_identity(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    // Pass the supplied record back; without metadata machinery we can't
    // attach the view-error tag, but the value is preserved.
    Ok(args[0].clone())
}

fn table_view_function_identity(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

