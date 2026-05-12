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

use crate::parser::Param;

use super::env::{Env, EnvNode, EnvOps};
use super::value::{BuiltinFn, Closure, FnBody, Value};

mod common;
mod number;
mod text;
mod logical;
mod list;
mod record;
mod table;
mod date;
mod time;
mod datetime;
mod datetimezone;
mod duration;
mod parquet;
mod odbc;
mod splitter;
mod combiner;
mod replacer;
mod comparer;
mod uri;
mod lines;
mod type_ops;
pub(super) mod value_ops;
mod binary;
mod function_ops;
mod expression;
mod error_ops;
mod json;
mod file;
mod excel;

// External callers of the old `stdlib::*` API expect these names at this path.
// `table_to_rows` is only reached from #[cfg(test)] code in eval::mod, hence
// the explicit allow.
pub use table::cell_to_value;
#[allow(unused_imports)]
pub(crate) use table::{row_to_record, table_to_rows};

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
        ("DateTimeZone.Type", TypeRep::Datetimezone),
        ("Time.Type",     TypeRep::Time),
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

    // ExtraValues.* constants — Table.FromList extraValues arg. Per M spec:
    // List = 0 (excess goes into the last column as a list), Ignore = 1
    // (excess is dropped), Error = 2 (excess raises an error).
    for (name, n) in [
        ("ExtraValues.List",   0.0),
        ("ExtraValues.Ignore", 1.0),
        ("ExtraValues.Error",  2.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // BinaryEncoding.* constants — Binary.FromText/ToText encoding arg.
    for (name, n) in [
        ("BinaryEncoding.Base64", 0.0),
        ("BinaryEncoding.Hex",    1.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Compression.* constants — Binary.Compress/Decompress compressionType arg.
    for (name, n) in [
        ("Compression.None",    0.0),
        ("Compression.GZip",    1.0),
        ("Compression.Deflate", 2.0),
        ("Compression.Brotli",  3.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Order.* constants — Table.Sort's per-column order argument.
    for (name, n) in [
        ("Order.Ascending",  0.0),
        ("Order.Descending", 1.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }

    // Day.* constants — Date.DayOfWeek's firstDayOfWeek argument.
    for (name, n) in [
        ("Day.Sunday",    0.0),
        ("Day.Monday",    1.0),
        ("Day.Tuesday",   2.0),
        ("Day.Wednesday", 3.0),
        ("Day.Thursday",  4.0),
        ("Day.Friday",    5.0),
        ("Day.Saturday",  6.0),
    ] {
        env = env.extend(name.to_string(), Value::Number(n));
    }
    env
}


fn builtin_bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    let mut all = Vec::new();
    for slice in [
        number::bindings(),
        text::bindings(),
        logical::bindings(),
        list::bindings(),
        record::bindings(),
        table::bindings(),
        date::bindings(),
        time::bindings(),
        datetime::bindings(),
        datetimezone::bindings(),
        duration::bindings(),
        parquet::bindings(),
        odbc::bindings(),
        splitter::bindings(),
        combiner::bindings(),
        replacer::bindings(),
        comparer::bindings(),
        uri::bindings(),
        lines::bindings(),
        type_ops::bindings(),
        value_ops::bindings(),
        binary::bindings(),
        function_ops::bindings(),
        expression::bindings(),
        error_ops::bindings(),
        json::bindings(),
        file::bindings(),
        excel::bindings(),
    ] {
        all.extend(slice);
    }
    all
}

