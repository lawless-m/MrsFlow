//! Human-readable preview formatter for `Value`.
//!
//! For large Tables (the dominant case in real queries) `value_to_sexpr`
//! produces tens of megabytes of unreadable text. `value_summary` renders
//! a count header + a column-aligned preview of the first N rows. Non-
//! Table values are small and pass through to `value_to_sexpr` unchanged.
//!
//! Not byte-identical with anything else — the differential harness uses
//! `value_to_sexpr` directly. This is purely the user-facing CLI/WASM
//! display format.
//!
//! See `mrsflow/09-lazy-tables.md` and the WASM demo at
//! `mrsflow-wasm/demo/index.html`.
//!
//! Caller must `deep_force` before formatting — the cell renderer
//! expects already-forced primitive Values, just like sexpr.

use super::iohost::IoHost;
use super::sexpr::value_to_sexpr;
use super::stdlib::cell_to_value;
use super::value::{MError, Table, Value};

/// Render `v` as a human-readable summary. For Tables: row/col count
/// header + first `max_rows` rendered as an aligned text table. Non-
/// Tables fall through to the canonical sexpr (they're small).
pub fn value_summary(
    v: &Value,
    max_rows: usize,
    host: &dyn IoHost,
) -> Result<String, MError> {
    match v {
        Value::Table(t) => render_table_preview(t, max_rows, host),
        _ => Ok(value_to_sexpr(v)),
    }
}

fn render_table_preview(
    table: &Table,
    max_rows: usize,
    _host: &dyn IoHost,
) -> Result<String, MError> {
    // The table should already be forced to Arrow/Rows by deep_force, but
    // accept lazy variants too — they'd just force here.
    let forced_cow = table.force()?;
    let forced: &Table = &forced_cow;

    let names = forced.column_names();
    let total_rows = forced.num_rows();
    let total_cols = forced.num_columns();
    let preview_rows = total_rows.min(max_rows);

    let mut out = String::new();
    out.push_str(&format!(
        "Table: {total_rows} rows × {total_cols} columns\n\n"
    ));

    if total_cols == 0 || total_rows == 0 {
        out.push_str("(empty)\n");
        return Ok(out);
    }

    // Format every cell we'll display to a string, calculating column widths.
    // Each cell is capped at MAX_CELL_CHARS to keep rows readable.
    const MAX_CELL_CHARS: usize = 32;
    let mut col_widths: Vec<usize> = names.iter().map(|n| n.len()).collect();
    let mut cells: Vec<Vec<String>> = Vec::with_capacity(preview_rows);
    for r in 0..preview_rows {
        let mut row: Vec<String> = Vec::with_capacity(total_cols);
        for c in 0..total_cols {
            let v = cell_to_value(forced, c, r)?;
            let s = display_cell(&v, MAX_CELL_CHARS);
            col_widths[c] = col_widths[c].max(s.chars().count());
            row.push(s);
        }
        cells.push(row);
    }

    // Header row.
    for (i, name) in names.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&pad(name, col_widths[i]));
    }
    out.push('\n');

    // Separator (ASCII dashes — keeps the output mono-spaced friendly
    // in both terminals and the demo's <pre> block).
    for (i, w) in col_widths.iter().enumerate() {
        if i > 0 {
            out.push_str("  ");
        }
        out.push_str(&"-".repeat(*w));
    }
    out.push('\n');

    // Data rows.
    for row in &cells {
        for (i, cell) in row.iter().enumerate() {
            if i > 0 {
                out.push_str("  ");
            }
            out.push_str(&pad(cell, col_widths[i]));
        }
        out.push('\n');
    }

    if preview_rows < total_rows {
        out.push_str(&format!(
            "\n(showing rows 1..{preview_rows} of {total_rows})\n"
        ));
    }

    Ok(out)
}

/// Render a single `Value` cell to a display string, truncating to
/// `max_chars` with an ellipsis if needed. Stays one line — control
/// characters and newlines are escaped so a stray '\n' in a Text cell
/// doesn't break the row alignment.
fn display_cell(v: &Value, max_chars: usize) -> String {
    let raw = match v {
        Value::Null => "null".to_string(),
        Value::Logical(true) => "true".to_string(),
        Value::Logical(false) => "false".to_string(),
        Value::Number(n) => {
            if n.fract() == 0.0 && n.abs() < 1e15 {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        Value::Decimal { mantissa, scale, .. } => {
            crate::eval::value::decimal_to_f64(*mantissa, *scale).to_string()
        }
        Value::Text(s) => s.replace('\n', "\\n").replace('\t', "\\t"),
        Value::Date(d) => format!("{d}"),
        Value::Datetime(dt) => format!("{dt}"),
        Value::Datetimezone(dt) => format!("{dt}"),
        Value::Time(t) => format!("{t}"),
        Value::Duration(d) => format!("{d}"),
        Value::Binary(b) => format!("<binary {} bytes>", b.len()),
        Value::List(xs) => format!("<list {} items>", xs.len()),
        Value::Record(r) => format!("<record {} fields>", r.fields.len()),
        Value::Table(t) => {
            format!("<table {}×{}>", t.num_rows(), t.num_columns())
        }
        Value::Function(_) => "<function>".to_string(),
        Value::Type(_) => "<type>".to_string(),
        Value::WithMetadata { inner, .. } => display_cell(inner, max_chars),
        Value::Thunk(_) => "<thunk>".to_string(),
    };
    // Truncate by char count (handles non-ASCII safely).
    let chars: Vec<char> = raw.chars().collect();
    if chars.len() > max_chars {
        let mut truncated: String = chars.into_iter().take(max_chars - 1).collect();
        truncated.push('…');
        truncated
    } else {
        raw
    }
}

fn pad(s: &str, width: usize) -> String {
    let len = s.chars().count();
    if len >= width {
        s.to_string()
    } else {
        let mut out = s.to_string();
        out.push_str(&" ".repeat(width - len));
        out
    }
}
