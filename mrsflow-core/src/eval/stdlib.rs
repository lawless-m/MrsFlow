//! Starter stdlib for eval-6: pure functions bound in the root env.
//!
//! Each function lives in this module as a `BuiltinFn`. `root_env()` builds
//! the initial env containing every binding, used by callers that want a
//! stdlib-aware environment instead of an empty one (`EnvNode::empty()`).
//!
//! Function scope is corpus-driven: the top non-Arrow stdlib calls in the
//! user's actual queries (`Text.Replace`, `Text.Contains`, `List.Transform`,
//! `Number.From`, …). Arrow-backed Table.* and date/datetime/duration land
//! in eval-7+.

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BooleanArray, Date32Array, DurationMicrosecondArray, Float64Array,
    NullArray, StringArray, TimestampMicrosecondArray,
};
use arrow::datatypes::{DataType, Field, Schema};
use arrow::record_batch::RecordBatch;

use crate::parser::Param;

use super::env::{Env, EnvNode, EnvOps};
use super::iohost::IoHost;
use super::value::{BuiltinFn, Closure, FnBody, MError, Record, Table, Value};

/// Build the initial environment containing every stdlib intrinsic plus
/// the two literal constants `#nan` and `#infinity`. Tests and shells pass
/// this as the starting env instead of `EnvNode::empty()`.
pub fn root_env() -> Env {
    let mut env = EnvNode::empty();
    for (name, params, body) in builtin_bindings() {
        let closure = Closure {
            params,
            body: FnBody::Builtin(body),
            env: EnvNode::empty(),
        };
        env = env.extend(name.to_string(), Value::Function(closure));
    }
    env = env.extend("#nan".into(), Value::Number(f64::NAN));
    env = env.extend("#infinity".into(), Value::Number(f64::INFINITY));

    // Type intrinsics (dotted-name values). Power Query exposes these as
    // type values; the corpus uses them in Table.AddColumn type args and
    // Table.TransformColumnTypes pairs. Several numeric intrinsics collapse
    // to TypeRep::Number for v1 (we have only f64 underlying) — the type
    // ascription path still works because TypeRep::Number → DataType::Float64.
    use super::value::TypeRep;
    for (name, tr) in [
        ("Number.Type",   TypeRep::Number),
        ("Int64.Type",    TypeRep::Number),
        ("Int32.Type",    TypeRep::Number),
        ("Int16.Type",    TypeRep::Number),
        ("Int8.Type",     TypeRep::Number),
        ("Currency.Type", TypeRep::Number),
        ("Decimal.Type",  TypeRep::Number),
        ("Single.Type",   TypeRep::Number),
        ("Double.Type",   TypeRep::Number),
        ("Percentage.Type", TypeRep::Number),
        ("Text.Type",     TypeRep::Text),
        ("Logical.Type",  TypeRep::Logical),
        ("Date.Type",     TypeRep::Date),
        ("DateTime.Type", TypeRep::Datetime),
        ("Duration.Type", TypeRep::Duration),
        ("Binary.Type",   TypeRep::Binary),
        ("Null.Type",     TypeRep::Null),
        ("Any.Type",      TypeRep::Any),
    ] {
        env = env.extend(name.to_string(), Value::Type(tr));
    }

    // JoinKind enum constants — numeric per Power Query M spec.
    for (name, n) in [
        ("JoinKind.Inner",      0.0),
        ("JoinKind.LeftOuter",  1.0),
        ("JoinKind.RightOuter", 2.0),
        ("JoinKind.FullOuter",  3.0),
        ("JoinKind.LeftAnti",   4.0),
        ("JoinKind.RightAnti",  5.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }
    env
}

fn one(name: &str) -> Vec<Param> {
    vec![Param {
        name: name.into(),
        optional: false,
        type_annotation: None,
    }]
}

fn two(a: &str, b: &str) -> Vec<Param> {
    vec![
        Param {
            name: a.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: b.into(),
            optional: false,
            type_annotation: None,
        },
    ]
}

fn three(a: &str, b: &str, c: &str) -> Vec<Param> {
    vec![
        Param {
            name: a.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: b.into(),
            optional: false,
            type_annotation: None,
        },
        Param {
            name: c.into(),
            optional: false,
            type_annotation: None,
        },
    ]
}

fn builtin_bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![
        ("Logical.ToText", one("logical"), logical_to_text),
        ("Character.FromNumber", one("number"), character_from_number),
        ("Character.ToNumber", one("text"), character_to_number),
        ("Guid.From", one("value"), guid_from),
        ("Text.NewGuid", vec![], text_new_guid),
        ("Number.From", one("value"), number_from),
        (
            "Number.Mod",
            vec![
                Param { name: "number".into(),    optional: false, type_annotation: None },
                Param { name: "divisor".into(),   optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            number_mod,
        ),
        (
            "Number.IntegerDivide",
            vec![
                Param { name: "number".into(),    optional: false, type_annotation: None },
                Param { name: "divisor".into(),   optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            number_integer_divide,
        ),
        ("Number.IsNaN", one("number"), number_is_nan),
        ("Number.IsOdd", one("number"), number_is_odd),
        ("Number.IsEven", one("number"), number_is_even),
        ("Number.Random", vec![], number_random),
        ("Number.RandomBetween", two("bottom", "top"), number_random_between),
        (
            "Number.RoundUp",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round_up,
        ),
        (
            "Number.RoundDown",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round_down,
        ),
        (
            "Number.RoundTowardZero",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round_toward_zero,
        ),
        (
            "Number.RoundAwayFromZero",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round_away_from_zero,
        ),
        ("Number.Acos", one("number"), number_acos),
        ("Number.Asin", one("number"), number_asin),
        ("Number.Atan", one("number"), number_atan),
        ("Number.Atan2", two("y", "x"), number_atan2),
        ("Number.Cos", one("number"), number_cos),
        ("Number.Cosh", one("number"), number_cosh),
        ("Number.Sin", one("number"), number_sin),
        ("Number.Sinh", one("number"), number_sinh),
        ("Number.Tan", one("number"), number_tan),
        ("Number.Tanh", one("number"), number_tanh),
        ("Number.Exp", one("number"), number_exp),
        ("Number.Ln", one("number"), number_ln),
        (
            "Number.Log",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "base".into(),   optional: true,  type_annotation: None },
            ],
            number_log,
        ),
        ("Number.Log10", one("number"), number_log10),
        ("Number.Factorial", one("number"), number_factorial),
        ("Number.Combinations", two("setSize", "combinationSize"), number_combinations),
        ("Number.Permutations", two("setSize", "combinationSize"), number_permutations),
        ("Byte.From", one("value"), number_from),
        ("Currency.From", one("value"), number_from),
        ("Decimal.From", one("value"), number_from),
        ("Double.From", one("value"), number_from),
        ("Int8.From", one("value"), number_from),
        ("Int16.From", one("value"), number_from),
        ("Int32.From", one("value"), number_from),
        ("Int64.From", one("value"), number_from),
        ("Percentage.From", one("value"), number_from),
        ("Single.From", one("value"), number_from),
        (
            "Number.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            number_from_text,
        ),
        (
            "Number.Round",
            vec![
                Param { name: "number".into(), optional: false, type_annotation: None },
                Param { name: "digits".into(), optional: true,  type_annotation: None },
            ],
            number_round,
        ),
        ("Number.Abs", one("number"), number_abs),
        (
            "Number.ToText",
            vec![
                Param { name: "number".into(),  optional: false, type_annotation: None },
                Param { name: "format".into(),  optional: true,  type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            number_to_text,
        ),
        ("Number.Sign", one("number"), number_sign),
        ("Number.Power", two("base", "exponent"), number_power),
        ("Number.Sqrt", one("number"), number_sqrt),
        ("Text.From", one("value"), text_from),
        ("Text.Contains", two("text", "substring"), text_contains),
        ("Text.Replace", three("text", "old", "new"), text_replace),
        ("Text.Trim", one("text"), text_trim),
        ("Text.Lower", one("text"), text_lower),
        ("Text.Upper", one("text"), text_upper),
        ("Text.Length", one("text"), text_length),
        ("Text.PositionOf", two("text", "substring"), text_position_of),
        ("Text.EndsWith", two("text", "suffix"), text_ends_with),
        ("Text.StartsWith", two("text", "prefix"), text_starts_with),
        ("Text.TrimEnd", one("text"), text_trim_end),
        (
            "Text.TrimStart",
            vec![
                Param { name: "text".into(), optional: false, type_annotation: None },
                Param { name: "trim".into(), optional: true,  type_annotation: None },
            ],
            text_trim_start,
        ),
        ("Text.Reverse", one("text"), text_reverse),
        ("Text.Proper", one("text"), text_proper),
        ("Text.At", two("text", "index"), text_at),
        (
            "Text.Range",
            vec![
                Param { name: "text".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            text_range,
        ),
        ("Text.Remove", two("text", "removeChars"), text_remove),
        (
            "Text.RemoveRange",
            vec![
                Param { name: "text".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            text_remove_range,
        ),
        ("Text.Insert", three("text", "offset", "newText"), text_insert),
        (
            "Text.ReplaceRange",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "offset".into(),  optional: false, type_annotation: None },
                Param { name: "count".into(),   optional: false, type_annotation: None },
                Param { name: "newText".into(), optional: false, type_annotation: None },
            ],
            text_replace_range,
        ),
        (
            "Text.PadStart",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "character".into(), optional: true,  type_annotation: None },
            ],
            text_pad_start,
        ),
        (
            "Text.PadEnd",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "character".into(), optional: true,  type_annotation: None },
            ],
            text_pad_end,
        ),
        ("Text.Repeat", two("text", "count"), text_repeat),
        ("Text.Select", two("text", "selectChars"), text_select),
        ("Text.ToList", one("text"), text_to_list),
        ("Text.SplitAny", two("text", "separators"), text_split_any),
        (
            "Text.PositionOfAny",
            vec![
                Param { name: "text".into(),       optional: false, type_annotation: None },
                Param { name: "characters".into(), optional: false, type_annotation: None },
                Param { name: "occurrence".into(), optional: true,  type_annotation: None },
            ],
            text_position_of_any,
        ),
        (
            "Text.BeforeDelimiter",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "delimiter".into(), optional: false, type_annotation: None },
                Param { name: "index".into(),     optional: true,  type_annotation: None },
            ],
            text_before_delimiter,
        ),
        (
            "Text.AfterDelimiter",
            vec![
                Param { name: "text".into(),      optional: false, type_annotation: None },
                Param { name: "delimiter".into(), optional: false, type_annotation: None },
                Param { name: "index".into(),     optional: true,  type_annotation: None },
            ],
            text_after_delimiter,
        ),
        (
            "Text.BetweenDelimiters",
            vec![
                Param { name: "text".into(),           optional: false, type_annotation: None },
                Param { name: "startDelimiter".into(), optional: false, type_annotation: None },
                Param { name: "endDelimiter".into(),   optional: false, type_annotation: None },
                Param { name: "startIndex".into(),     optional: true,  type_annotation: None },
                Param { name: "endIndex".into(),       optional: true,  type_annotation: None },
            ],
            text_between_delimiters,
        ),
        ("Text.InferNumberType", one("text"), text_infer_number_type),
        ("Text.Clean", one("text"), text_clean),
        (
            "Text.Format",
            vec![
                Param { name: "formatString".into(), optional: false, type_annotation: None },
                Param { name: "arguments".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(),      optional: true,  type_annotation: None },
            ],
            text_format,
        ),
        ("Text.Start", two("text", "count"), text_start),
        (
            "Text.Middle",
            vec![
                Param { name: "text".into(),   optional: false, type_annotation: None },
                Param { name: "offset".into(), optional: false, type_annotation: None },
                Param { name: "count".into(),  optional: true,  type_annotation: None },
            ],
            text_middle,
        ),
        ("Text.End", two("text", "count"), text_end),
        ("Text.Split", two("text", "separator"), text_split),
        (
            "Text.Combine",
            vec![
                Param { name: "texts".into(),     optional: false, type_annotation: None },
                Param { name: "separator".into(), optional: true,  type_annotation: None },
            ],
            text_combine,
        ),
        ("List.Transform", two("list", "transform"), list_transform),
        ("List.Select", two("list", "selection"), list_select),
        ("List.Sum", one("list"), list_sum),
        (
            "List.Average",
            vec![
                Param { name: "list".into(),      optional: false, type_annotation: None },
                Param { name: "precision".into(), optional: true,  type_annotation: None },
            ],
            list_average,
        ),
        ("List.Count", one("list"), list_count),
        ("List.Min", one("list"), list_min),
        ("List.Max", one("list"), list_max),
        ("List.Combine", one("lists"), list_combine),
        ("List.IsEmpty", one("list"), list_is_empty),
        (
            "List.First",
            vec![
                Param { name: "list".into(),    optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            list_first,
        ),
        (
            "List.Last",
            vec![
                Param { name: "list".into(),    optional: false, type_annotation: None },
                Param { name: "default".into(), optional: true,  type_annotation: None },
            ],
            list_last,
        ),
        ("List.Reverse", one("list"), list_reverse),
        (
            "List.Numbers",
            vec![
                Param { name: "start".into(),     optional: false, type_annotation: None },
                Param { name: "count".into(),     optional: false, type_annotation: None },
                Param { name: "increment".into(), optional: true,  type_annotation: None },
            ],
            list_numbers,
        ),
        (
            "List.PositionOf",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "value".into(),            optional: false, type_annotation: None },
                Param { name: "occurrence".into(),       optional: true,  type_annotation: None },
                Param { name: "equationCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_position_of,
        ),
        (
            "List.RemoveFirstN",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            list_remove_first_n,
        ),
        ("List.RemoveItems", two("list", "list2"), list_remove_items),
        ("List.Zip", one("lists"), list_zip),
        ("List.FirstN", two("list", "countOrCondition"), list_first_n),
        (
            "List.LastN",
            vec![
                Param { name: "list".into(),             optional: false, type_annotation: None },
                Param { name: "countOrCondition".into(), optional: true,  type_annotation: None },
            ],
            list_last_n,
        ),
        ("List.Skip", two("list", "countOrCondition"), list_skip),
        ("List.Distinct", one("list"), list_distinct),
        (
            "List.Sort",
            vec![
                Param { name: "list".into(),               optional: false, type_annotation: None },
                Param { name: "comparisonCriteria".into(), optional: true,  type_annotation: None },
            ],
            list_sort,
        ),
        (
            "List.RemoveMatchingItems",
            two("list", "items"),
            list_remove_matching_items,
        ),
        ("List.AnyTrue", one("list"), list_any_true),
        ("List.AllTrue", one("list"), list_all_true),
        ("Record.Field", two("record", "field"), record_field),
        ("Record.FieldNames", one("record"), record_field_names),
        ("Record.FieldValues", one("record"), record_field_values),
        ("Record.HasFields", two("record", "fields"), record_has_fields),
        ("Record.Combine", one("records"), record_combine),
        (
            "Record.FieldOrDefault",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "field".into(),        optional: false, type_annotation: None },
                Param { name: "defaultValue".into(), optional: true,  type_annotation: None },
            ],
            record_field_or_default,
        ),
        (
            "Record.RemoveFields",
            vec![
                Param { name: "record".into(),       optional: false, type_annotation: None },
                Param { name: "fields".into(),       optional: false, type_annotation: None },
                Param { name: "missingField".into(), optional: true,  type_annotation: None },
            ],
            record_remove_fields,
        ),
        ("Logical.From", one("value"), logical_from),
        ("Logical.FromText", one("text"), logical_from_text),
        ("#table", two("columns", "rows"), table_constructor),
        ("Table.ColumnNames", one("table"), table_column_names),
        ("Table.RenameColumns", two("table", "renames"), table_rename_columns),
        ("Table.RemoveColumns", two("table", "names"), table_remove_columns),
        (
            "#date",
            three("year", "month", "day"),
            date_constructor,
        ),
        (
            "#datetime",
            vec![
                Param { name: "year".into(),   optional: false, type_annotation: None },
                Param { name: "month".into(),  optional: false, type_annotation: None },
                Param { name: "day".into(),    optional: false, type_annotation: None },
                Param { name: "hour".into(),   optional: false, type_annotation: None },
                Param { name: "minute".into(), optional: false, type_annotation: None },
                Param { name: "second".into(), optional: false, type_annotation: None },
            ],
            datetime_constructor,
        ),
        (
            "#duration",
            vec![
                Param { name: "days".into(),    optional: false, type_annotation: None },
                Param { name: "hours".into(),   optional: false, type_annotation: None },
                Param { name: "minutes".into(), optional: false, type_annotation: None },
                Param { name: "seconds".into(), optional: false, type_annotation: None },
            ],
            duration_constructor,
        ),
        ("Parquet.Document", one("path"), parquet_document),
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
        (
            "List.Accumulate",
            three("list", "seed", "accumulator"),
            list_accumulate,
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
        ("Date.FromText", one("text"), date_from_text),
        ("Date.AddDays", two("date", "numberOfDays"), date_add_days),
        ("Date.AddMonths", two("date", "numberOfMonths"), date_add_months),
        ("Date.From", one("value"), date_from),
        ("Date.Year", one("date"), date_year),
        ("Date.Month", one("date"), date_month),
        ("Date.Day", one("date"), date_day),
        (
            "Date.ToText",
            vec![
                Param { name: "date".into(),   optional: false, type_annotation: None },
                Param { name: "format".into(), optional: true,  type_annotation: None },
            ],
            date_to_text,
        ),
        ("Odbc.Query", two("connection", "sql"), odbc_query),
    ]
}

fn type_mismatch(expected: &'static str, found: &Value) -> MError {
    MError::TypeMismatch {
        expected,
        found: super::type_name(found),
    }
}

// --- Number.* ---

fn logical_to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Logical(b) => Ok(Value::Text(if *b { "true".into() } else { "false".into() })),
        other => Err(type_mismatch("logical", other)),
    }
}

fn character_from_number(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            if !n.is_finite() || *n < 0.0 || n.fract() != 0.0 || *n > u32::MAX as f64 {
                return Err(MError::Other(format!(
                    "Character.FromNumber: not a valid codepoint: {}",
                    n
                )));
            }
            let cp = *n as u32;
            char::from_u32(cp)
                .map(|c| Value::Text(c.to_string()))
                .ok_or_else(|| MError::Other(format!(
                    "Character.FromNumber: invalid Unicode codepoint U+{:04X}",
                    cp
                )))
        }
        other => Err(type_mismatch("number", other)),
    }
}

fn character_to_number(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => s
            .chars()
            .next()
            .map(|c| Value::Number(c as u32 as f64))
            .ok_or_else(|| MError::Other("Character.ToNumber: empty text".into())),
        other => Err(type_mismatch("text", other)),
    }
}

fn guid_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => {
            // Validate 8-4-4-4-12 hex format. PQ's Guid value is text-shaped;
            // we keep it as Text but normalise to lowercase.
            let lower = s.to_lowercase();
            let bytes = lower.as_bytes();
            let dashes_at = [8, 13, 18, 23];
            if bytes.len() != 36
                || !dashes_at.iter().all(|&i| bytes[i] == b'-')
                || !bytes
                    .iter()
                    .enumerate()
                    .all(|(i, &b)| dashes_at.contains(&i) || b.is_ascii_hexdigit())
            {
                return Err(MError::Other(format!("Guid.From: invalid GUID: {:?}", s)));
            }
            Ok(Value::Text(lower))
        }
        other => Err(type_mismatch("text", other)),
    }
}

fn text_new_guid(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use rand::RngCore;
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    // RFC 4122 v4: set version (high nibble of byte 6) and variant (high bits of byte 8).
    bytes[6] = (bytes[6] & 0x0F) | 0x40;
    bytes[8] = (bytes[8] & 0x3F) | 0x80;
    let s = format!(
        "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
        bytes[0], bytes[1], bytes[2], bytes[3],
        bytes[4], bytes[5], bytes[6], bytes[7],
        bytes[8], bytes[9],
        bytes[10], bytes[11], bytes[12], bytes[13], bytes[14], bytes[15]
    );
    Ok(Value::Text(s))
}

fn number_mod(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let a = match &args[0] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    let b = match &args[1] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    if b == 0.0 {
        return Err(MError::Other("Number.Mod: division by zero".into()));
    }
    // Mathematical (floor) mod: result has the same sign as divisor.
    Ok(Value::Number(a - b * (a / b).floor()))
}

fn number_integer_divide(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let a = match &args[0] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    let b = match &args[1] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    if b == 0.0 {
        return Err(MError::Other("Number.IntegerDivide: division by zero".into()));
    }
    Ok(Value::Number((a / b).floor()))
}

fn number_is_nan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Logical(n.is_nan())),
        other => Err(type_mismatch("number", other)),
    }
}

fn number_is_odd(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            if !n.is_finite() || n.fract() != 0.0 {
                return Err(MError::Other(format!(
                    "Number.IsOdd: argument must be an integer (got {})", n
                )));
            }
            Ok(Value::Logical((*n as i64) % 2 != 0))
        }
        other => Err(type_mismatch("number", other)),
    }
}

fn number_is_even(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            if !n.is_finite() || n.fract() != 0.0 {
                return Err(MError::Other(format!(
                    "Number.IsEven: argument must be an integer (got {})", n
                )));
            }
            Ok(Value::Logical((*n as i64) % 2 == 0))
        }
        other => Err(type_mismatch("number", other)),
    }
}

fn number_exp(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::exp) }
fn number_ln(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::ln) }
fn number_log10(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::log10) }

fn number_log(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let base = match args.get(1) {
        Some(Value::Number(b)) => *b,
        Some(Value::Null) | None => std::f64::consts::E,
        Some(other) => return Err(type_mismatch("number (base)", other)),
    };
    Ok(Value::Number(n.log(base)))
}

fn factorial_f64(n: f64) -> Result<f64, MError> {
    if !n.is_finite() || n < 0.0 || n.fract() != 0.0 {
        return Err(MError::Other(format!(
            "Number.Factorial: argument must be a non-negative integer (got {})", n
        )));
    }
    let n = n as u64;
    if n > 170 {
        return Err(MError::Other("Number.Factorial: overflow (n > 170)".into()));
    }
    let mut acc = 1f64;
    for i in 2..=n {
        acc *= i as f64;
    }
    Ok(acc)
}

fn number_factorial(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    Ok(Value::Number(factorial_f64(n)?))
}

fn number_combinations(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let k = match &args[1] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    if k > n {
        return Err(MError::Other(
            "Number.Combinations: combinationSize must not exceed setSize".into(),
        ));
    }
    Ok(Value::Number(factorial_f64(n)? / (factorial_f64(k)? * factorial_f64(n - k)?)))
}

fn number_permutations(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let k = match &args[1] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    if k > n {
        return Err(MError::Other(
            "Number.Permutations: combinationSize must not exceed setSize".into(),
        ));
    }
    Ok(Value::Number(factorial_f64(n)? / factorial_f64(n - k)?))
}

fn unary_f64(args: &[Value], f: fn(f64) -> f64) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(f(*n))),
        other => Err(type_mismatch("number", other)),
    }
}

fn number_acos(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::acos) }
fn number_asin(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::asin) }
fn number_atan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::atan) }
fn number_cos(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::cos) }
fn number_cosh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::cosh) }
fn number_sin(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::sin) }
fn number_sinh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::sinh) }
fn number_tan(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::tan) }
fn number_tanh(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> { unary_f64(args, f64::tanh) }

fn number_atan2(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let x = match &args[1] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    Ok(Value::Number(y.atan2(x)))
}

fn apply_round_mode(args: &[Value], ctx: &str, mode: fn(f64) -> f64) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Number(n) => *n,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("number", other)),
    };
    let digits = match args.get(1) {
        Some(Value::Number(d)) if d.fract() == 0.0 => *d as i32,
        Some(Value::Null) | None => 0,
        Some(other) => {
            let _ = ctx;
            return Err(type_mismatch("integer (digits)", other));
        }
    };
    let scale = 10f64.powi(digits);
    Ok(Value::Number(mode(n * scale) / scale))
}

fn number_round_up(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundUp", f64::ceil)
}

fn number_round_down(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundDown", f64::floor)
}

fn number_round_toward_zero(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundTowardZero", f64::trunc)
}

fn number_round_away_from_zero(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    apply_round_mode(args, "Number.RoundAwayFromZero", |x| {
        if x >= 0.0 { (x + 0.5).floor() } else { (x - 0.5).ceil() }
    })
}

fn number_random(_args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    Ok(Value::Number(rand::random::<f64>()))
}

fn number_random_between(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let bottom = match &args[0] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let top = match &args[1] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    if !(bottom <= top) {
        return Err(MError::Other(format!(
            "Number.RandomBetween: bottom ({}) must be <= top ({})", bottom, top
        )));
    }
    Ok(Value::Number(bottom + rand::random::<f64>() * (top - bottom)))
}

fn number_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(*n)),
        Value::Logical(b) => Ok(Value::Number(if *b { 1.0 } else { 0.0 })),
        Value::Text(s) => s
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| MError::Other(format!("Number.From: cannot parse {:?}", s))),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

fn number_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => s
            .trim()
            .parse::<f64>()
            .map(Value::Number)
            .map_err(|_| MError::Other(format!("Number.FromText: cannot parse {:?}", s))),
        other => Err(type_mismatch("text", other)),
    }
}

fn number_to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => {
            if !matches!(args.get(1), Some(Value::Null) | None) {
                return Err(MError::NotImplemented(
                    "Number.ToText: format string not yet supported",
                ));
            }
            // PQ prints whole-number floats without a trailing ".0".
            let s = if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e16 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            };
            Ok(Value::Text(s))
        }
        other => Err(type_mismatch("number", other)),
    }
}

fn number_abs(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(n.abs())),
        other => Err(type_mismatch("number", other)),
    }
}

fn number_sign(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(if *n > 0.0 {
            1.0
        } else if *n < 0.0 {
            -1.0
        } else {
            0.0
        })),
        other => Err(type_mismatch("number", other)),
    }
}

fn number_power(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let base = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let exp = match &args[1] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    Ok(Value::Number(base.powf(exp)))
}

fn number_sqrt(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Number(n) => Ok(Value::Number(n.sqrt())),
        other => Err(type_mismatch("number", other)),
    }
}

fn number_round(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let digits = match args.get(1) {
        Some(Value::Number(d)) => *d as i32,
        Some(Value::Null) | None => 0,
        Some(other) => return Err(type_mismatch("number or null", other)),
    };
    // Simple half-away-from-zero. M's default is banker's, but the corpus
    // only relies on basic rounding for display.
    let factor = 10f64.powi(digits);
    Ok(Value::Number((n * factor).round() / factor))
}

// --- Text.* ---

fn text_from(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Text(s) => Ok(Value::Text(s.clone())),
        // {:?} for f64 matches `value_dump`'s canonical num format
        // (always-trailing fractional digit). Keeping parity here so
        // Text.From(42) prints the same as the differential's `(num 42.0)`.
        Value::Number(n) => Ok(Value::Text(format!("{:?}", n))),
        Value::Logical(b) => Ok(Value::Text(
            if *b { "true" } else { "false" }.to_string(),
        )),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

fn text_contains(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    Ok(Value::Logical(text.contains(sub)))
}

fn text_replace(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let old = expect_text(&args[1])?;
    let new = expect_text(&args[2])?;
    Ok(Value::Text(text.replace(old, new)))
}

fn text_trim(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.trim().to_string()))
}

fn text_lower(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.to_lowercase()))
}

fn text_upper(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.to_uppercase()))
}

fn text_length(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // M counts characters, not bytes — use char count.
    Ok(Value::Number(text.chars().count() as f64))
}

fn text_position_of(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sub = expect_text(&args[1])?;
    // Per spec: -1 when not found, byte offset on miss... but for parity
    // with the M spec (and the corpus), return a char index. The empty-sub
    // edge case isn't load-bearing for slice-6 tests.
    let idx = text.find(sub).map(|byte_idx| {
        text[..byte_idx].chars().count()
    });
    Ok(Value::Number(match idx {
        Some(i) => i as f64,
        None => -1.0,
    }))
}

fn text_ends_with(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let suffix = expect_text(&args[1])?;
    Ok(Value::Logical(text.ends_with(suffix)))
}

fn text_starts_with(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let prefix = expect_text(&args[1])?;
    Ok(Value::Logical(text.starts_with(prefix)))
}

fn text_trim_end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.trim_end().to_string()))
}

/// Helper: extract chars from a text-or-list-of-text "chars" argument.
fn chars_from_arg(v: &Value, ctx: &'static str) -> Result<Vec<char>, MError> {
    match v {
        Value::Text(s) => Ok(s.chars().collect()),
        Value::List(xs) => {
            let mut out = Vec::new();
            for x in xs {
                match x {
                    Value::Text(s) => out.extend(s.chars()),
                    other => return Err(MError::Other(format!(
                        "{}: list element must be text, got {}",
                        ctx, super::type_name(other)
                    ))),
                }
            }
            Ok(out)
        }
        other => Err(type_mismatch("text or list of text", other)),
    }
}

fn text_at(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let idx = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    text.chars()
        .nth(idx)
        .map(|c| Value::Text(c.to_string()))
        .ok_or_else(|| MError::Other(format!("Text.At: index {} out of range", idx)))
}

fn text_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars: Vec<char> = text.chars().collect();
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match args.get(2) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => chars.len().saturating_sub(offset),
        Some(other) => return Err(type_mismatch("non-negative integer or null", other)),
    };
    if offset > chars.len() {
        return Err(MError::Other("Text.Range: offset out of range".into()));
    }
    let end = (offset + count).min(chars.len());
    Ok(Value::Text(chars[offset..end].iter().collect()))
}

fn text_remove(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let drop = chars_from_arg(&args[1], "Text.Remove")?;
    Ok(Value::Text(text.chars().filter(|c| !drop.contains(c)).collect()))
}

fn text_remove_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars: Vec<char> = text.chars().collect();
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match args.get(2) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Null) | None => 1,
        Some(other) => return Err(type_mismatch("non-negative integer or null", other)),
    };
    if offset > chars.len() {
        return Err(MError::Other("Text.RemoveRange: offset out of range".into()));
    }
    let end = (offset + count).min(chars.len());
    let mut out: String = chars[..offset].iter().collect();
    out.extend(chars[end..].iter());
    Ok(Value::Text(out))
}

fn text_insert(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars: Vec<char> = text.chars().collect();
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let new_text = expect_text(&args[2])?;
    if offset > chars.len() {
        return Err(MError::Other("Text.Insert: offset out of range".into()));
    }
    let mut out: String = chars[..offset].iter().collect();
    out.push_str(new_text);
    out.extend(chars[offset..].iter());
    Ok(Value::Text(out))
}

fn text_replace_range(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars: Vec<char> = text.chars().collect();
    let offset = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let count = match &args[2] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let new_text = expect_text(&args[3])?;
    if offset > chars.len() {
        return Err(MError::Other("Text.ReplaceRange: offset out of range".into()));
    }
    let end = (offset + count).min(chars.len());
    let mut out: String = chars[..offset].iter().collect();
    out.push_str(new_text);
    out.extend(chars[end..].iter());
    Ok(Value::Text(out))
}

fn text_pad_start(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let target = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let pad_char = match args.get(2) {
        Some(Value::Text(s)) => s.chars().next().ok_or_else(||
            MError::Other("Text.PadStart: pad character is empty".into()))?,
        Some(Value::Null) | None => ' ',
        Some(other) => return Err(type_mismatch("text", other)),
    };
    let n = text.chars().count();
    if n >= target {
        Ok(Value::Text(text.to_string()))
    } else {
        let mut out: String = std::iter::repeat(pad_char).take(target - n).collect();
        out.push_str(text);
        Ok(Value::Text(out))
    }
}

fn text_pad_end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let target = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    let pad_char = match args.get(2) {
        Some(Value::Text(s)) => s.chars().next().ok_or_else(||
            MError::Other("Text.PadEnd: pad character is empty".into()))?,
        Some(Value::Null) | None => ' ',
        Some(other) => return Err(type_mismatch("text", other)),
    };
    let n = text.chars().count();
    if n >= target {
        Ok(Value::Text(text.to_string()))
    } else {
        let mut out = text.to_string();
        out.extend(std::iter::repeat(pad_char).take(target - n));
        Ok(Value::Text(out))
    }
}

fn text_repeat(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    Ok(Value::Text(text.repeat(count)))
}

fn text_select(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let keep = chars_from_arg(&args[1], "Text.Select")?;
    Ok(Value::Text(text.chars().filter(|c| keep.contains(c)).collect()))
}

fn text_to_list(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::List(text.chars().map(|c| Value::Text(c.to_string())).collect()))
}

fn text_split_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let seps_text = expect_text(&args[1])?;
    let seps: Vec<char> = seps_text.chars().collect();
    let parts: Vec<Value> = text
        .split(|c: char| seps.contains(&c))
        .map(|s| Value::Text(s.to_string()))
        .collect();
    Ok(Value::List(parts))
}

fn text_position_of_any(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let chars = chars_from_arg(&args[1], "Text.PositionOfAny")?;
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Text.PositionOfAny: occurrence arg not yet supported",
        ));
    }
    let idx = text
        .char_indices()
        .find(|(_, c)| chars.contains(c))
        .map(|(byte_idx, _)| text[..byte_idx].chars().count());
    Ok(Value::Number(match idx {
        Some(i) => i as f64,
        None => -1.0,
    }))
}

/// Find the byte offsets of every occurrence of `delim` in `text`.
fn delimiter_byte_offsets(text: &str, delim: &str) -> Vec<usize> {
    if delim.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut start = 0;
    while let Some(i) = text[start..].find(delim) {
        let abs = start + i;
        out.push(abs);
        start = abs + delim.len();
    }
    out
}

fn pick_delimiter_index(args_index: Option<&Value>, ctx: &str) -> Result<(usize, bool), MError> {
    // Returns (index, from_end). `from_end` true means count from the right.
    match args_index {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => Ok((*n as usize, false)),
        Some(Value::List(xs)) if xs.len() == 2 => {
            // {index, RelativePosition.FromEnd=1 or FromStart=0}
            let i = match &xs[0] {
                Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
                other => return Err(MError::Other(format!(
                    "{}: index list element 0 must be non-negative integer (got {})",
                    ctx, super::type_name(other)
                ))),
            };
            let from_end = matches!(&xs[1], Value::Number(n) if *n == 1.0);
            Ok((i, from_end))
        }
        Some(Value::Null) | None => Ok((0, false)),
        Some(other) => Err(type_mismatch("non-negative integer or {index, direction} list", other)),
    }
}

fn text_before_delimiter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let delim = expect_text(&args[1])?;
    let (index, from_end) = pick_delimiter_index(args.get(2), "Text.BeforeDelimiter")?;
    let offsets = delimiter_byte_offsets(text, delim);
    let pick = if from_end {
        offsets.get(offsets.len().wrapping_sub(1).wrapping_sub(index))
    } else {
        offsets.get(index)
    };
    match pick {
        Some(&byte_idx) => Ok(Value::Text(text[..byte_idx].to_string())),
        None => Ok(Value::Text(String::new())),
    }
}

fn text_after_delimiter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let delim = expect_text(&args[1])?;
    let (index, from_end) = pick_delimiter_index(args.get(2), "Text.AfterDelimiter")?;
    let offsets = delimiter_byte_offsets(text, delim);
    let pick = if from_end {
        offsets.get(offsets.len().wrapping_sub(1).wrapping_sub(index))
    } else {
        offsets.get(index)
    };
    match pick {
        Some(&byte_idx) => Ok(Value::Text(text[byte_idx + delim.len()..].to_string())),
        None => Ok(Value::Text(String::new())),
    }
}

fn text_between_delimiters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let start_delim = expect_text(&args[1])?;
    let end_delim = expect_text(&args[2])?;
    let (start_index, start_from_end) =
        pick_delimiter_index(args.get(3), "Text.BetweenDelimiters")?;
    let (end_index, end_from_end) =
        pick_delimiter_index(args.get(4), "Text.BetweenDelimiters")?;
    let start_offsets = delimiter_byte_offsets(text, start_delim);
    let start_pick = if start_from_end {
        start_offsets.get(start_offsets.len().wrapping_sub(1).wrapping_sub(start_index))
    } else {
        start_offsets.get(start_index)
    };
    let start_byte = match start_pick {
        Some(&b) => b + start_delim.len(),
        None => return Ok(Value::Text(String::new())),
    };
    let rest = &text[start_byte..];
    let end_offsets = delimiter_byte_offsets(rest, end_delim);
    let end_pick = if end_from_end {
        end_offsets.get(end_offsets.len().wrapping_sub(1).wrapping_sub(end_index))
    } else {
        end_offsets.get(end_index)
    };
    match end_pick {
        Some(&b) => Ok(Value::Text(rest[..b].to_string())),
        None => Ok(Value::Text(String::new())),
    }
}

fn text_clean(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(
        text.chars()
            .filter(|c| !(c.is_control() && *c != '\n' && *c != '\r' && *c != '\t'))
            .filter(|c| !c.is_control())
            .collect(),
    ))
}

/// Stringify a value for Text.Format substitution.
fn format_arg_to_text(v: &Value) -> String {
    match v {
        Value::Null => "".into(),
        Value::Text(s) => s.clone(),
        Value::Number(n) => {
            if n.is_finite() && n.fract() == 0.0 && n.abs() < 1e16 {
                format!("{}", *n as i64)
            } else {
                n.to_string()
            }
        }
        Value::Logical(b) => if *b { "true".into() } else { "false".into() },
        Value::Date(d) => d.to_string(),
        Value::Datetime(dt) => dt.to_string(),
        Value::Duration(d) => format!("{}", d),
        other => format!("{:?}", other),
    }
}

fn text_format(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let fmt = expect_text(&args[0])?;
    let mut out = String::with_capacity(fmt.len());
    let mut chars = fmt.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '#' && chars.peek() == Some(&'{') {
            chars.next(); // consume '{'
            let mut key = String::new();
            let mut closed = false;
            for kc in chars.by_ref() {
                if kc == '}' {
                    closed = true;
                    break;
                }
                key.push(kc);
            }
            if !closed {
                return Err(MError::Other(
                    "Text.Format: unterminated #{...} placeholder".into(),
                ));
            }
            let value = match &args[1] {
                Value::List(xs) => {
                    let idx: usize = key.parse().map_err(|_| MError::Other(format!(
                        "Text.Format: index {:?} not a number for list arguments", key
                    )))?;
                    xs.get(idx).cloned().ok_or_else(|| MError::Other(format!(
                        "Text.Format: index {} out of range", idx
                    )))?
                }
                Value::Record(r) => {
                    let raw = r
                        .fields
                        .iter()
                        .find(|(n, _)| n == &key)
                        .map(|(_, v)| v.clone())
                        .ok_or_else(|| MError::Other(format!(
                            "Text.Format: field {:?} not in arguments record", key
                        )))?;
                    super::force(raw, &mut |e, env| super::evaluate(e, env, host))?
                }
                other => return Err(type_mismatch("list or record (arguments)", other)),
            };
            out.push_str(&format_arg_to_text(&value));
        } else {
            out.push(c);
        }
    }
    Ok(Value::Text(out))
}

fn text_infer_number_type(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    if text.trim().parse::<f64>().is_ok() {
        Ok(Value::Type(super::value::TypeRep::Number))
    } else {
        Err(MError::Other(format!(
            "Text.InferNumberType: cannot infer numeric type from {:?}",
            text
        )))
    }
}

fn text_trim_start(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    match args.get(1) {
        Some(Value::Null) | None => Ok(Value::Text(text.trim_start().to_string())),
        Some(Value::Text(t)) => {
            let chars: Vec<char> = t.chars().collect();
            Ok(Value::Text(text.trim_start_matches(|c| chars.contains(&c)).to_string()))
        }
        Some(Value::List(xs)) => {
            let mut chars: Vec<char> = Vec::new();
            for v in xs {
                match v {
                    Value::Text(s) => chars.extend(s.chars()),
                    other => return Err(type_mismatch("text (in trim list)", other)),
                }
            }
            Ok(Value::Text(text.trim_start_matches(|c| chars.contains(&c)).to_string()))
        }
        Some(other) => Err(type_mismatch("text or list of text", other)),
    }
}

fn text_reverse(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    Ok(Value::Text(text.chars().rev().collect()))
}

fn text_proper(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let mut out = String::with_capacity(text.len());
    let mut start_of_word = true;
    for c in text.chars() {
        if c.is_whitespace() {
            out.push(c);
            start_of_word = true;
        } else if start_of_word {
            out.extend(c.to_uppercase());
            start_of_word = false;
        } else {
            out.extend(c.to_lowercase());
        }
    }
    Ok(Value::Text(out))
}

fn text_combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let texts = expect_list(&args[0])?;
    let sep = match args.get(1) {
        Some(Value::Text(s)) => s.as_str(),
        Some(Value::Null) | None => "",
        Some(other) => return Err(type_mismatch("text or null", other)),
    };
    let parts: Result<Vec<&str>, MError> = texts
        .iter()
        .map(|v| match v {
            Value::Text(s) => Ok(s.as_str()),
            other => Err(type_mismatch("text (in list)", other)),
        })
        .collect();
    Ok(Value::Text(parts?.join(sep)))
}

fn text_start(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if count <= 0 {
        return Ok(Value::Text(String::new()));
    }
    Ok(Value::Text(text.chars().take(count as usize).collect()))
}

fn text_end(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if count <= 0 {
        return Ok(Value::Text(String::new()));
    }
    let total = text.chars().count();
    let skip = total.saturating_sub(count as usize);
    Ok(Value::Text(text.chars().skip(skip).collect()))
}

fn text_middle(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let offset = match &args[1] {
        Value::Number(n) => *n as isize,
        other => return Err(type_mismatch("number", other)),
    };
    if offset < 0 {
        return Ok(Value::Text(String::new()));
    }
    // Optional 3rd arg: count. Null/missing → take rest of string.
    let count = match args.get(2) {
        Some(Value::Number(n)) => Some(*n as isize),
        Some(Value::Null) | None => None,
        Some(other) => return Err(type_mismatch("number or null", other)),
    };
    let mut iter = text.chars().skip(offset as usize);
    let result: String = match count {
        Some(c) if c <= 0 => String::new(),
        Some(c) => iter.by_ref().take(c as usize).collect(),
        None => iter.collect(),
    };
    Ok(Value::Text(result))
}

fn text_split(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    let sep = expect_text(&args[1])?;
    // Power Query Text.Split on empty separator returns a list of single-char
    // texts; we emulate that to be on the safe side.
    let parts: Vec<Value> = if sep.is_empty() {
        text.chars().map(|c| Value::Text(c.to_string())).collect()
    } else {
        text.split(sep).map(|s| Value::Text(s.to_string())).collect()
    };
    Ok(Value::List(parts))
}

fn expect_text(v: &Value) -> Result<&str, MError> {
    match v {
        Value::Text(s) => Ok(s.as_str()),
        other => Err(type_mismatch("text", other)),
    }
}

// --- List.* ---

fn list_transform(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let f = expect_function(&args[1])?;
    let mut out = Vec::with_capacity(list.len());
    for item in list {
        let v = invoke_builtin_callback(f, vec![item.clone()])?;
        out.push(v);
    }
    Ok(Value::List(out))
}

fn list_select(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let pred = expect_function(&args[1])?;
    let mut out = Vec::new();
    for item in list {
        let v = invoke_builtin_callback(pred, vec![item.clone()])?;
        match v {
            Value::Logical(true) => out.push(item.clone()),
            Value::Logical(false) => {}
            other => return Err(type_mismatch("logical (from predicate)", &other)),
        }
    }
    Ok(Value::List(out))
}

fn list_sum(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut total = 0.0;
    for v in list {
        total += match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
    }
    Ok(Value::Number(total))
}

fn list_average(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut total = 0.0f64;
    let mut n = 0usize;
    for v in list {
        match v {
            Value::Null => continue,
            Value::Number(x) => {
                total += x;
                n += 1;
            }
            other => return Err(type_mismatch("number (in list)", other)),
        }
    }
    if n == 0 {
        Ok(Value::Null)
    } else {
        Ok(Value::Number(total / n as f64))
    }
}

fn list_count(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Number(list.len() as f64))
}

fn list_zip(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lists = expect_list(&args[0])?;
    let inner: Vec<&Vec<Value>> = lists
        .iter()
        .map(|v| expect_list(v))
        .collect::<Result<_, _>>()?;
    let max_len = inner.iter().map(|l| l.len()).max().unwrap_or(0);
    let mut out: Vec<Value> = Vec::with_capacity(max_len);
    for i in 0..max_len {
        let row: Vec<Value> = inner
            .iter()
            .map(|l| l.get(i).cloned().unwrap_or(Value::Null))
            .collect();
        out.push(Value::List(row));
    }
    Ok(Value::List(out))
}

fn list_remove_first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let n = match args.get(1) {
        Some(Value::Number(n)) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(MError::Other(
                    "List.RemoveFirstN: count must be a non-negative integer".into(),
                ));
            }
            *n as usize
        }
        Some(Value::Function(_)) => {
            return Err(MError::NotImplemented(
                "List.RemoveFirstN: predicate (skip-while) form not yet supported",
            ));
        }
        Some(Value::Null) | None => 1,
        Some(other) => return Err(type_mismatch("number or function", other)),
    };
    Ok(Value::List(list.iter().skip(n).cloned().collect()))
}

fn list_remove_items(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let drop = expect_list(&args[1])?;
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for v in list {
        let mut keep = true;
        for d in drop {
            if values_equal_primitive(v, d)? {
                keep = false;
                break;
            }
        }
        if keep {
            out.push(v.clone());
        }
    }
    Ok(Value::List(out))
}

fn list_position_of(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let target = &args[1];
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "List.PositionOf: occurrence arg not yet supported",
        ));
    }
    if !matches!(args.get(3), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "List.PositionOf: equationCriteria not yet supported",
        ));
    }
    for (i, v) in list.iter().enumerate() {
        if values_equal_primitive(v, target)? {
            return Ok(Value::Number(i as f64));
        }
    }
    Ok(Value::Number(-1.0))
}

fn list_numbers(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let start = match &args[0] {
        Value::Number(n) => *n,
        other => return Err(type_mismatch("number", other)),
    };
    let count = match &args[1] {
        Value::Number(n) => {
            if !n.is_finite() || *n < 0.0 {
                return Err(MError::Other("List.Numbers: count must be a non-negative integer".into()));
            }
            *n as usize
        }
        other => return Err(type_mismatch("number", other)),
    };
    let increment = match args.get(2) {
        Some(Value::Number(n)) => *n,
        Some(Value::Null) | None => 1.0,
        Some(other) => return Err(type_mismatch("number", other)),
    };
    let mut out = Vec::with_capacity(count);
    for i in 0..count {
        out.push(Value::Number(start + (i as f64) * increment));
    }
    Ok(Value::List(out))
}

fn list_min(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut best: Option<f64> = None;
    for v in list {
        let n = match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
        best = Some(match best {
            None => n,
            Some(curr) => if n < curr { n } else { curr },
        });
    }
    Ok(Value::Number(best.unwrap()))
}

fn list_max(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if list.is_empty() {
        return Ok(Value::Null);
    }
    let mut best: Option<f64> = None;
    for v in list {
        let n = match v {
            Value::Number(n) => *n,
            other => return Err(type_mismatch("number (in list)", other)),
        };
        best = Some(match best {
            None => n,
            Some(curr) => if n > curr { n } else { curr },
        });
    }
    Ok(Value::Number(best.unwrap()))
}

fn list_is_empty(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    Ok(Value::Logical(list.is_empty()))
}

fn list_first(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if let Some(first) = list.first() {
        Ok(first.clone())
    } else {
        Ok(args.get(1).cloned().unwrap_or(Value::Null))
    }
}

fn list_last(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if let Some(last) = list.last() {
        Ok(last.clone())
    } else {
        Ok(args.get(1).cloned().unwrap_or(Value::Null))
    }
}

fn list_sort(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    if !matches!(args.get(1), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "List.Sort: comparisonCriteria not yet supported",
        ));
    }
    enum Kind { Empty, Num, Text }
    let mut kind = Kind::Empty;
    for v in list {
        let k = match v {
            Value::Number(_) => Kind::Num,
            Value::Text(_) => Kind::Text,
            other => return Err(type_mismatch("number or text (in list)", other)),
        };
        match (&kind, &k) {
            (Kind::Empty, _) => kind = k,
            (Kind::Num, Kind::Num) | (Kind::Text, Kind::Text) => {}
            _ => return Err(MError::Other(
                "List.Sort: mixed-type lists not supported (numbers and text together)".into(),
            )),
        }
    }
    let mut out: Vec<Value> = list.clone();
    match kind {
        Kind::Empty => {}
        Kind::Num => out.sort_by(|a, b| {
            let (Value::Number(x), Value::Number(y)) = (a, b) else { unreachable!() };
            x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal)
        }),
        Kind::Text => out.sort_by(|a, b| {
            let (Value::Text(x), Value::Text(y)) = (a, b) else { unreachable!() };
            x.cmp(y)
        }),
    }
    Ok(Value::List(out))
}

fn list_reverse(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut out = list.clone();
    out.reverse();
    Ok(Value::List(out))
}

fn list_first_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    // Power Query also accepts a predicate (take-while) form; not yet supported.
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Value::Function(_) => {
            return Err(MError::NotImplemented(
                "List.FirstN: predicate (take-while) form not yet supported",
            ));
        }
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    Ok(Value::List(list.iter().take(count).cloned().collect()))
}

fn list_last_n(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let count = match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Some(Value::Function(_)) => {
            return Err(MError::NotImplemented(
                "List.LastN: predicate form not yet supported",
            ));
        }
        Some(Value::Null) | None => 1,
        Some(other) => return Err(type_mismatch("non-negative integer", other)),
    };
    let n = list.len();
    let start = n.saturating_sub(count);
    Ok(Value::List(list[start..].to_vec()))
}

/// Structural equality for primitive cell types only — number, text, logical,
/// null, date, datetime, duration. Compound values (list/record/table/function/
/// type/thunk/binary) error out; the caller wraps the error.
pub(super) fn values_equal_primitive(a: &Value, b: &Value) -> Result<bool, MError> {
    match (a, b) {
        (Value::Null, Value::Null) => Ok(true),
        (Value::Logical(x), Value::Logical(y)) => Ok(x == y),
        (Value::Number(x), Value::Number(y)) => Ok(x == y),
        (Value::Text(x), Value::Text(y)) => Ok(x == y),
        (Value::Date(x), Value::Date(y)) => Ok(x == y),
        (Value::Datetime(x), Value::Datetime(y)) => Ok(x == y),
        (Value::Duration(x), Value::Duration(y)) => Ok(x == y),
        // Different primitive variants are not equal — null vs non-null included.
        (
            Value::Null
            | Value::Logical(_)
            | Value::Number(_)
            | Value::Text(_)
            | Value::Date(_)
            | Value::Datetime(_)
            | Value::Duration(_),
            Value::Null
            | Value::Logical(_)
            | Value::Number(_)
            | Value::Text(_)
            | Value::Date(_)
            | Value::Datetime(_)
            | Value::Duration(_),
        ) => Ok(false),
        _ => Err(MError::NotImplemented(
            "equality on compound values (list/record/table/etc.) deferred",
        )),
    }
}

fn list_any_true(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    for v in list {
        match v {
            Value::Logical(b) => {
                if *b {
                    return Ok(Value::Logical(true));
                }
            }
            other => return Err(type_mismatch("logical (in list)", other)),
        }
    }
    Ok(Value::Logical(false))
}

fn list_all_true(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    for v in list {
        match v {
            Value::Logical(b) => {
                if !*b {
                    return Ok(Value::Logical(false));
                }
            }
            other => return Err(type_mismatch("logical (in list)", other)),
        }
    }
    Ok(Value::Logical(true))
}

fn list_remove_matching_items(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let items = expect_list(&args[1])?;
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for v in list {
        let mut drop = false;
        for x in items {
            if values_equal_primitive(v, x)? {
                drop = true;
                break;
            }
        }
        if !drop {
            out.push(v.clone());
        }
    }
    Ok(Value::List(out))
}

fn list_distinct(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut out: Vec<Value> = Vec::with_capacity(list.len());
    for v in list {
        let mut seen = false;
        for kept in &out {
            if values_equal_primitive(kept, v)? {
                seen = true;
                break;
            }
        }
        if !seen {
            out.push(v.clone());
        }
    }
    Ok(Value::List(out))
}

fn list_skip(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let count = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 && *n >= 0.0 => *n as usize,
        Value::Function(_) => {
            return Err(MError::NotImplemented(
                "List.Skip: predicate (skip-while) form not yet supported",
            ));
        }
        other => return Err(type_mismatch("non-negative integer", other)),
    };
    Ok(Value::List(list.iter().skip(count).cloned().collect()))
}

fn list_combine(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let lists = expect_list(&args[0])?;
    let mut out: Vec<Value> = Vec::new();
    for v in lists {
        match v {
            Value::List(xs) => out.extend(xs.iter().cloned()),
            other => return Err(type_mismatch("list (in list)", other)),
        }
    }
    Ok(Value::List(out))
}

fn list_accumulate(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let list = expect_list(&args[0])?;
    let mut acc = args[1].clone();
    let f = expect_function(&args[2])?;
    for item in list {
        acc = invoke_callback_with_host(f, vec![acc, item.clone()], host)?;
    }
    Ok(acc)
}

fn expect_list(v: &Value) -> Result<&Vec<Value>, MError> {
    match v {
        Value::List(xs) => Ok(xs),
        other => Err(type_mismatch("list", other)),
    }
}

fn expect_function(v: &Value) -> Result<&Closure, MError> {
    match v {
        Value::Function(c) => Ok(c),
        other => Err(type_mismatch("function", other)),
    }
}

/// Call a closure from within a builtin. Builtins receive already-forced
/// values, so we need only mirror the body-dispatch part of the evaluator's
/// Invoke handling. For M-bodied closures we run the body in the captured
/// env; for nested builtins we recurse directly.
///
/// Slice-6 stdlib needs IoHost-free recursion only — List.Transform's
/// callback fn cannot itself perform IO since builtins don't carry a host.
/// Pass a no-op host to be safe.
fn invoke_builtin_callback(closure: &Closure, args: Vec<Value>) -> Result<Value, MError> {
    if args.len() != closure.params.len() {
        return Err(MError::Other(format!(
            "callback: arity mismatch: expected {}, got {}",
            closure.params.len(),
            args.len()
        )));
    }
    // Callbacks from List.Transform/Select can't reach the original host
    // — pass NoIoHost so IO-using callbacks fail loudly rather than picking
    // up some unrelated environment. If a future stdlib function needs to
    // thread the real host through callbacks, refactor this signature.
    let host = super::NoIoHost;
    match &closure.body {
        FnBody::Builtin(f) => f(&args, &host),
        FnBody::M(body) => {
            let mut call_env = closure.env.clone();
            for (param, value) in closure.params.iter().zip(args.into_iter()) {
                call_env = call_env.extend(param.name.clone(), value);
            }
            super::evaluate(body, &call_env, &host)
        }
    }
}

// --- Record.* ---

fn record_field(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let name = expect_text(&args[1])?;
    match record.fields.iter().find(|(n, _)| n == name) {
        Some((_, v)) => super::force(v.clone(), &mut |e, env| {
            super::evaluate(e, env, &super::NoIoHost)
        }),
        None => Err(MError::Other(format!("Record.Field: field not found: {}", name))),
    }
}

fn record_field_names(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let names: Vec<Value> = record
        .fields
        .iter()
        .map(|(n, _)| Value::Text(n.clone()))
        .collect();
    Ok(Value::List(names))
}

fn record_field_values(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let values: Result<Vec<Value>, MError> = record
        .fields
        .iter()
        .map(|(_, v)| super::force(v.clone(), &mut |e, env| super::evaluate(e, env, host)))
        .collect();
    Ok(Value::List(values?))
}

fn record_has_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let names: Vec<String> = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(xs) => xs
            .iter()
            .map(|v| match v {
                Value::Text(s) => Ok(s.clone()),
                other => Err(type_mismatch("text (in list)", other)),
            })
            .collect::<Result<_, _>>()?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    let has_all = names
        .iter()
        .all(|n| record.fields.iter().any(|(fname, _)| fname == n));
    Ok(Value::Logical(has_all))
}

fn record_combine(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let records = expect_list(&args[0])?;
    let mut fields: Vec<(String, Value)> = Vec::new();
    for rv in records {
        let rec = match rv {
            Value::Record(r) => r,
            other => return Err(type_mismatch("record (in list)", other)),
        };
        for (name, v) in &rec.fields {
            let forced = super::force(v.clone(), &mut |e, env| super::evaluate(e, env, host))?;
            if let Some(slot) = fields.iter_mut().find(|(n, _)| n == name) {
                slot.1 = forced;
            } else {
                fields.push((name.clone(), forced));
            }
        }
    }
    Ok(Value::Record(Record {
        fields,
        env: super::env::EnvNode::empty(),
    }))
}

fn record_field_or_default(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let name = expect_text(&args[1])?;
    let default = args.get(2).cloned().unwrap_or(Value::Null);
    match record.fields.iter().find(|(n, _)| n == name) {
        Some((_, v)) => super::force(v.clone(), &mut |e, env| super::evaluate(e, env, host)),
        None => Ok(default),
    }
}

fn record_remove_fields(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let record = match &args[0] {
        Value::Record(r) => r,
        other => return Err(type_mismatch("record", other)),
    };
    let drop_names: Vec<String> = match &args[1] {
        Value::Text(s) => vec![s.clone()],
        Value::List(xs) => xs
            .iter()
            .map(|v| match v {
                Value::Text(s) => Ok(s.clone()),
                other => Err(type_mismatch("text (in list)", other)),
            })
            .collect::<Result<_, _>>()?,
        other => return Err(type_mismatch("text or list of text", other)),
    };
    if !matches!(args.get(2), Some(Value::Null) | None) {
        return Err(MError::NotImplemented(
            "Record.RemoveFields: missingField option not yet supported",
        ));
    }
    // Default behaviour: any name not present in the record is an error.
    for n in &drop_names {
        if !record.fields.iter().any(|(fname, _)| fname == n) {
            return Err(MError::Other(format!(
                "Record.RemoveFields: field not found: {}",
                n
            )));
        }
    }
    let kept: Vec<(String, Value)> = record
        .fields
        .iter()
        .filter(|(n, _)| !drop_names.contains(n))
        .cloned()
        .collect();
    Ok(Value::Record(Record {
        fields: kept,
        env: record.env.clone(),
    }))
}

// --- Logical.* ---

fn logical_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let v = &args[0];
    match v {
        Value::Null => Ok(Value::Null),
        Value::Logical(b) => Ok(Value::Logical(*b)),
        Value::Number(n) => Ok(Value::Logical(*n != 0.0)),
        Value::Text(_) => logical_from_text(args, host),
        other => Err(type_mismatch("text/number/logical/null", other)),
    }
}

fn logical_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    match text.to_ascii_lowercase().as_str() {
        "true" => Ok(Value::Logical(true)),
        "false" => Ok(Value::Logical(false)),
        _ => Err(MError::Other(format!(
            "Logical.FromText: not a boolean: {:?}",
            text
        ))),
    }
}


// --- Table.* (eval-7a) ---
//
// #table(columns, rows) and the three top-corpus Table.* operations.
// Compound type expressions in the columns position aren't supported in
// this slice — only a list of text column names. Date/Datetime/Duration/
// Binary cells land in eval-7b alongside chrono.

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
        super::value::TableRepr::Arrow(batch) => {
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
        super::value::TableRepr::Rows { rows, .. } => {
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
        super::value::TableRepr::Arrow(batch) => {
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
        super::value::TableRepr::Rows { rows, .. } => {
            let new_rows: Vec<Vec<Value>> = rows
                .iter()
                .map(|row| keep_indices.iter().map(|&i| row[i].clone()).collect())
                .collect();
            Ok(Value::Table(Table::from_rows(new_names, new_rows)))
        }
    }
}

fn expect_table(v: &Value) -> Result<&Table, MError> {
    match v {
        Value::Table(t) => Ok(t),
        other => Err(type_mismatch("table", other)),
    }
}

fn expect_text_list(v: &Value, ctx: &str) -> Result<Vec<String>, MError> {
    let xs = expect_list(v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match x {
            Value::Text(s) => out.push(s.clone()),
            other => {
                return Err(MError::Other(format!(
                    "{}: expected list of text, got {}",
                    ctx,
                    super::type_name(other)
                )));
            }
        }
    }
    Ok(out)
}

fn expect_list_of_lists<'a>(v: &'a Value, ctx: &str) -> Result<Vec<Vec<Value>>, MError> {
    let xs = expect_list(v)?;
    let mut out = Vec::with_capacity(xs.len());
    for x in xs {
        match x {
            Value::List(inner) => out.push(inner.clone()),
            other => {
                return Err(MError::Other(format!(
                    "{}: expected list of lists, got {}",
                    ctx,
                    super::type_name(other)
                )));
            }
        }
    }
    Ok(out)
}

/// Build a Table from column names + row-major cells. Picks the Arrow-backed
/// representation when every column fits the uniform-column rule; falls back
/// to a Rows-backed Table when any column is heterogeneous (compound values,
/// mixed primitives, Binary).
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
        super::value::TableRepr::Arrow(b) => b,
        super::value::TableRepr::Rows { rows, .. } => {
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

fn date_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#date: year")?;
    let mo = expect_int(&args[1], "#date: month")?;
    let d = expect_int(&args[2], "#date: day")?;
    chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .map(Value::Date)
        .ok_or_else(|| MError::Other(format!("#date: invalid date {}-{:02}-{:02}", y, mo, d)))
}

fn datetime_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#datetime: year")?;
    let mo = expect_int(&args[1], "#datetime: month")?;
    let d = expect_int(&args[2], "#datetime: day")?;
    let h = expect_int(&args[3], "#datetime: hour")?;
    let mn = expect_int(&args[4], "#datetime: minute")?;
    let s = expect_int(&args[5], "#datetime: second")?;
    let date = chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .ok_or_else(|| MError::Other(format!("#datetime: invalid date {}-{:02}-{:02}", y, mo, d)))?;
    let time = chrono::NaiveTime::from_hms_opt(h as u32, mn as u32, s as u32)
        .ok_or_else(|| MError::Other(format!("#datetime: invalid time {:02}:{:02}:{:02}", h, mn, s)))?;
    Ok(Value::Datetime(chrono::NaiveDateTime::new(date, time)))
}

fn duration_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = expect_int(&args[0], "#duration: days")?;
    let h = expect_int(&args[1], "#duration: hours")?;
    let mn = expect_int(&args[2], "#duration: minutes")?;
    let s = expect_int(&args[3], "#duration: seconds")?;
    let total = d
        .checked_mul(86400)
        .and_then(|x| x.checked_add(h.checked_mul(3600)?))
        .and_then(|x| x.checked_add(mn.checked_mul(60)?))
        .and_then(|x| x.checked_add(s))
        .ok_or_else(|| MError::Other("#duration: overflow".into()))?;
    Ok(Value::Duration(chrono::Duration::seconds(total)))
}

fn expect_int(v: &Value, ctx: &str) -> Result<i64, MError> {
    match v {
        Value::Number(n) => {
            if n.fract() != 0.0 {
                return Err(MError::Other(format!("{}: not an integer: {}", ctx, n)));
            }
            Ok(*n as i64)
        }
        other => Err(type_mismatch("number", other)),
    }
}

// --- Parquet IO (eval-7c) ---
//
// The pure evaluator core can't open files; Parquet.Document just delegates
// to the shell's IoHost. CliIoHost in mrsflow-cli decodes the file via the
// `parquet` crate; NoIoHost (default in unit tests) errors. WASM shell will
// similarly error or proxy through DuckDB-Wasm later.

fn parquet_document(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let path = expect_text(&args[0])?;
    host.parquet_read(path).map_err(|e| {
        MError::Other(format!("Parquet.Document({:?}): {:?}", path, e))
    })
}

// --- Table.* expansion (eval-7d) ---
//
// Five more Table.* ops by corpus frequency. SelectRows and AddColumn
// invoke an M closure with a row-as-record value, matching the
// `each [ColumnName]` access pattern users write.

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
                    found: super::type_name(&other),
                });
            }
        }
    }
    match &table.repr {
        super::value::TableRepr::Arrow(batch) => {
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
        super::value::TableRepr::Rows { columns, rows } => {
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
            let forced = super::force(raw, &mut |e, env| super::evaluate(e, env, host))?;
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

    if let (super::value::TableRepr::Arrow(batch), Some((inferred_dtype, inferred_array))) =
        (&table.repr, &inferred)
    {
        // Fast path: Arrow input + Arrow-encodable new column.
        let (dtype, new_array, nullable) = match &target_type {
            Some(Value::Type(t)) if !matches!(t, super::value::TypeRep::Any) => {
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
                    super::type_name(&other)
                )));
            }
        }
    }
    // Drop row 0 from every column, keeping the existing column types.
    // Users who want a different type after promotion call TransformColumnTypes.
    let n_remaining = table.num_rows() - 1;
    match &table.repr {
        super::value::TableRepr::Arrow(batch) => {
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
        super::value::TableRepr::Rows { rows, .. } => {
            let new_rows: Vec<Vec<Value>> = rows.iter().skip(1).cloned().collect();
            Ok(Value::Table(Table::from_rows(new_names, new_rows)))
        }
    }
}

/// Build a record Value from one row of a table — column name → cell.
/// Dispatches on `TableRepr`.
pub(super) fn row_to_record(table: &Table, row: usize) -> Result<Value, MError> {
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
fn invoke_callback_with_host(
    closure: &Closure,
    args: Vec<Value>,
    host: &dyn IoHost,
) -> Result<Value, MError> {
    if args.len() != closure.params.len() {
        return Err(MError::Other(format!(
            "callback: arity mismatch: expected {}, got {}",
            closure.params.len(),
            args.len()
        )));
    }
    match &closure.body {
        FnBody::Builtin(f) => f(&args, host),
        FnBody::M(body) => {
            let mut call_env = closure.env.clone();
            for (param, value) in closure.params.iter().zip(args.into_iter()) {
                call_env = call_env.extend(param.name.clone(), value);
            }
            super::evaluate(body, &call_env, host)
        }
    }
}

// --- Table.* eval-7e: type-aware ops + concat ---

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
        super::value::TableRepr::Arrow(batch) => {
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
        super::value::TableRepr::Rows { columns, rows } => {
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
) -> Result<super::value::TypeRep, MError> {
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
        let mapped = if matches!(type_value, super::value::TypeRep::Any) {
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
fn type_rep_to_datatype(t: &super::value::TypeRep) -> Result<(DataType, bool), MError> {
    use super::value::TypeRep;
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
        | TypeRep::Table | TypeRep::Function | TypeRep::Type | TypeRep::Binary => {
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
            Some(t) if !matches!(t, super::value::TypeRep::Any) => {
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
    t: &super::value::TypeRep,
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
) -> Result<Vec<(String, &'a Closure, Option<super::value::TypeRep>)>, MError> {
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
        super::value::TableRepr::Arrow(batch) => {
            let remaining = n_rows - skip;
            let new_columns: Vec<ArrayRef> =
                batch.columns().iter().map(|c| c.slice(skip, remaining)).collect();
            let new_batch = RecordBatch::try_new(batch.schema(), new_columns)
                .map_err(|e| MError::Other(format!("Table.Skip: rebuild failed: {}", e)))?;
            Ok(Value::Table(Table::from_arrow(new_batch)))
        }
        super::value::TableRepr::Rows { columns, rows } => {
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
                    super::type_name(other)
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
                    super::type_name(other)
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
                    super::type_name(other)
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
                            super::type_name(other)
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
        .all(|t| matches!(&t.repr, super::value::TableRepr::Arrow(_)));
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
            let v = super::force(v, &mut |e, env| super::evaluate(e, env, host))?;
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

fn date_to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Date(d) => *d,
        other => return Err(type_mismatch("date", other)),
    };
    let chrono_fmt = match args.get(1) {
        Some(Value::Null) | None => "%Y-%m-%d",
        Some(Value::Text(s)) => match s.as_str() {
            "yyyy-MM-dd" => "%Y-%m-%d",
            "dd/MM/yyyy" => "%d/%m/%Y",
            "dd-MM-yyyy" => "%d-%m-%Y",
            "MM/dd/yyyy" => "%m/%d/%Y",
            "yyyy/MM/dd" => "%Y/%m/%d",
            other => {
                return Err(MError::Other(format!(
                    "Date.ToText: unsupported format {:?}; supported: yyyy-MM-dd, dd/MM/yyyy, dd-MM-yyyy, MM/dd/yyyy, yyyy/MM/dd",
                    other
                )));
            }
        },
        Some(other) => return Err(type_mismatch("text or null", other)),
    };
    Ok(Value::Text(d.format(chrono_fmt).to_string()))
}

fn date_from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Date(*d)),
        Value::Datetime(dt) => Ok(Value::Date(dt.date())),
        Value::Text(_) => date_from_text(args, host),
        other => Err(type_mismatch("date/datetime/text/null", other)),
    }
}

fn date_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.year() as f64)),
        other => Err(type_mismatch("date", other)),
    }
}

fn date_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.month() as f64)),
        other => Err(type_mismatch("date", other)),
    }
}

fn date_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.day() as f64)),
        other => Err(type_mismatch("date", other)),
    }
}

fn date_add_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n_days = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 => *n as i64,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("integer (numberOfDays)", other)),
    };
    let delta = chrono::Duration::days(n_days);
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => d
            .checked_add_signed(delta)
            .map(Value::Date)
            .ok_or_else(|| MError::Other("Date.AddDays: result out of range".into())),
        Value::Datetime(dt) => dt
            .checked_add_signed(delta)
            .map(Value::Datetime)
            .ok_or_else(|| MError::Other("Date.AddDays: result out of range".into())),
        other => Err(type_mismatch("date or datetime", other)),
    }
}

fn date_add_months(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 => *n as i64,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("integer (numberOfMonths)", other)),
    };
    fn shift_date(d: chrono::NaiveDate, n: i64) -> Option<chrono::NaiveDate> {
        if n >= 0 {
            d.checked_add_months(chrono::Months::new(n as u32))
        } else {
            d.checked_sub_months(chrono::Months::new((-n) as u32))
        }
    }
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => shift_date(*d, n)
            .map(Value::Date)
            .ok_or_else(|| MError::Other("Date.AddMonths: result out of range".into())),
        Value::Datetime(dt) => {
            let new_date = shift_date(dt.date(), n)
                .ok_or_else(|| MError::Other("Date.AddMonths: result out of range".into()))?;
            Ok(Value::Datetime(new_date.and_time(dt.time())))
        }
        other => Err(type_mismatch("date or datetime", other)),
    }
}

fn date_from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // Power Query's Date.FromText is locale-aware. Try ISO first, then a
    // couple of common UK/US forms. Not the full spec — just enough for the
    // corpus.
    for fmt in &["%Y-%m-%d", "%d-%m-%Y", "%m-%d-%Y", "%Y/%m/%d", "%d/%m/%Y"] {
        if let Ok(d) = chrono::NaiveDate::parse_from_str(text, fmt) {
            return Ok(Value::Date(d));
        }
    }
    Err(MError::Other(format!(
        "Date.FromText: cannot parse {:?}",
        text
    )))
}

// --- ODBC (eval-8) ---
//
// Delegates to the shell's IoHost. CliIoHost (built with `--features odbc`)
// uses odbc-api against an installed driver; NoIoHost and CliIoHost built
// without the feature return a NotSupported-style error. WASM shell will
// likewise return NotSupported when it lands.

fn odbc_query(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let conn = expect_text(&args[0])?;
    let sql = expect_text(&args[1])?;
    host.odbc_query(conn, sql, None)
        .map_err(|e| MError::Other(format!("Odbc.Query: {:?}", e)))
}
