//! Canonical S-expression formatter for `Value`. Single source of truth so
//! the `value_dump` example, the `mrsflow` CLI binary, and the differential
//! harness all emit byte-identical output that matches `print_value/1` in
//! `tools/grammar-fuzz/evaluator.pl`.
//!
//! Callers must `deep_force` before formatting — unforced thunks land in the
//! catch-all `(thunk ...)` arm, which would diverge from the Prolog companion.

use super::stdlib::table::cell_to_value;
use super::value::{TypeRep, Value};

/// Render `v` to its canonical S-expression string.
pub fn value_to_sexpr(v: &Value) -> String {
    let mut out = String::new();
    write_value(&mut out, v);
    out
}

/// Append the S-expression rendering of `v` to `out`. Public so callers
/// that already own a `String` buffer can avoid the per-call allocation.
pub fn write_value(out: &mut String, v: &Value) {
    match v {
        Value::Null => out.push_str("(null)"),
        Value::Logical(true) => out.push_str("(bool true)"),
        Value::Logical(false) => out.push_str("(bool false)"),
        // {:?} formats floats with always-trailing fractional digit
        // (e.g. `42.0`, not `42`) which matches scryer's `~w` for floats.
        // Differential parity hinges on this.
        Value::Number(n) => out.push_str(&format!("(num {n:?})")),
        Value::Decimal { mantissa, scale, precision } => {
            // Render as (decimal MANTISSA SCALE PRECISION) — distinct
            // from (num ...) so differential parity against scryer
            // doesn't conflate Decimal with f64.
            out.push_str(&format!("(decimal {mantissa} {scale} {precision})"));
        }
        Value::Text(s) => {
            out.push_str("(text ");
            write_quoted(out, s);
            out.push(')');
        }
        Value::Date(d) => {
            use chrono::Datelike;
            out.push_str(&format!("(date {} {} {})", d.year(), d.month(), d.day()));
        }
        Value::Datetime(dt) => {
            use chrono::{Datelike, Timelike};
            out.push_str(&format!(
                "(datetime {} {} {} {} {} {})",
                dt.year(),
                dt.month(),
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second()
            ));
        }
        Value::Datetimezone(dt) => {
            use chrono::{Datelike, Timelike};
            let off = dt.offset().local_minus_utc();
            out.push_str(&format!(
                "(datetimezone {} {} {} {} {} {} {})",
                dt.year(),
                dt.month(),
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second(),
                off
            ));
        }
        Value::Time(t) => {
            use chrono::Timelike;
            out.push_str(&format!(
                "(time {} {} {})",
                t.hour(),
                t.minute(),
                t.second()
            ));
        }
        Value::Duration(dur) => {
            let s = dur.num_seconds() as f64;
            out.push_str(&format!("(duration {s:?})"));
        }
        Value::Binary(_) => out.push_str("(binary ...)"),
        Value::List(items) => {
            out.push_str("(list (");
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write_value(out, item);
            }
            out.push_str("))");
        }
        Value::Record(record) => {
            out.push_str("(record (");
            for (i, (name, value)) in record.fields.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                write_quoted(out, name);
                out.push(' ');
                write_value(out, value);
                out.push(')');
            }
            out.push_str("))");
        }
        Value::Table(t) => {
            // Force any lazy repr (LazyOdbc / LazyParquet / *View) before
            // walking cells — `cell_to_value` only accepts Arrow / Rows.
            // Errors during forcing render as `(table (force-error ...))`
            // so the s-expression printer remains infallible.
            let forced = match t.force() {
                Ok(c) => c,
                Err(e) => {
                    out.push_str("(table (force-error ");
                    write_quoted(out, &format!("{e:?}"));
                    out.push_str("))");
                    return;
                }
            };
            let t = forced.as_ref();
            out.push_str("(table ((cols (");
            let names = t.column_names();
            for (i, name) in names.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                write_quoted(out, name);
            }
            out.push_str(")) (rows (");
            let n_rows = t.num_rows();
            let n_cols = t.num_columns();
            for row in 0..n_rows {
                if row > 0 {
                    out.push(' ');
                }
                out.push('(');
                for col in 0..n_cols {
                    if col > 0 {
                        out.push(' ');
                    }
                    let cell = cell_to_value(t, col, row).unwrap_or(Value::Null);
                    write_value(out, &cell);
                }
                out.push(')');
            }
            out.push_str("))))");
        }
        Value::Function(_) => out.push_str("(function ...)"),
        Value::Type(t) => {
            out.push_str("(type-value ");
            write_type(out, t);
            out.push(')');
        }
        Value::Thunk(_) => out.push_str("(thunk ...)"),
        // Metadata is invisible to the canonical s-expression — render the
        // inner value so the Prolog companion (no metadata) stays in sync.
        Value::WithMetadata { inner, .. } => write_value(out, inner),
    }
}

fn write_type(out: &mut String, t: &TypeRep) {
    match t {
        TypeRep::Any => out.push_str("any"),
        TypeRep::AnyNonNull => out.push_str("anynonnull"),
        TypeRep::Null => out.push_str("null"),
        TypeRep::Logical => out.push_str("logical"),
        TypeRep::Number => out.push_str("number"),
        TypeRep::Text => out.push_str("text"),
        TypeRep::Date => out.push_str("date"),
        TypeRep::Datetime => out.push_str("datetime"),
        TypeRep::Datetimezone => out.push_str("datetimezone"),
        TypeRep::Time => out.push_str("time"),
        TypeRep::Duration => out.push_str("duration"),
        TypeRep::Binary => out.push_str("binary"),
        TypeRep::List => out.push_str("list"),
        TypeRep::Record => out.push_str("record"),
        TypeRep::Table => out.push_str("table"),
        TypeRep::Function => out.push_str("function"),
        TypeRep::Type => out.push_str("type"),
        TypeRep::Nullable(inner) => {
            out.push_str("(nullable ");
            write_type(out, inner);
            out.push(')');
        }
        TypeRep::ListOf(item) => {
            out.push_str("(list-of ");
            write_type(out, item);
            out.push(')');
        }
        TypeRep::RecordOf { fields, open } => {
            out.push_str("(record-of (");
            for (i, (name, t, opt)) in fields.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                out.push('"');
                out.push_str(name);
                out.push('"');
                out.push(' ');
                out.push_str(if *opt { "opt" } else { "req" });
                out.push(' ');
                write_type(out, t);
                out.push(')');
            }
            out.push_str(") ");
            out.push_str(if *open { "open" } else { "closed" });
            out.push(')');
        }
        TypeRep::TableOf { columns } => {
            out.push_str("(table-of (");
            for (i, (name, t)) in columns.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                out.push('"');
                out.push_str(name);
                out.push('"');
                out.push(' ');
                write_type(out, t);
                out.push(')');
            }
            out.push_str("))");
        }
        TypeRep::FunctionOf { params, return_type } => {
            out.push_str("(function-of (");
            for (i, (t, opt)) in params.iter().enumerate() {
                if i > 0 {
                    out.push(' ');
                }
                out.push('(');
                out.push_str(if *opt { "opt" } else { "req" });
                out.push(' ');
                write_type(out, t);
                out.push(')');
            }
            out.push_str(") ");
            write_type(out, return_type);
            out.push(')');
        }
    }
}

fn write_quoted(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            other => out.push(other),
        }
    }
    out.push('"');
}
