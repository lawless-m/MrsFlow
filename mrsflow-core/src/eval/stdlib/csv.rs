//! `Csv.Document` — parse CSV text or binary into a Rows-backed table.
//!
//! Output shape: columns are `Column1..ColumnN`, every cell is a
//! `Value::Text`. No header promotion, no type inference — chain
//! `Table.PromoteHeaders` and `Table.TransformColumnTypes` downstream.
//!
//! Supported options-record fields:
//! - `Delimiter` — single character, default `,`
//! - `QuoteStyle` — `QuoteStyle.Csv` (default, RFC 4180 quoting) or
//!   `QuoteStyle.None` (quotes are literal characters)
//! - `Encoding` — only `65001` (UTF-8) accepted; other code pages
//!   error so silent mis-decoding can't happen
//!
//! `Columns` (optional name list or count) is not yet supported — the
//! corpus doesn't use it.

use crate::parser::Param;

use super::super::iohost::IoHost;
use super::super::value::{BuiltinFn, MError, Record, Table, Value};
use super::common::type_mismatch;

pub(super) fn bindings() -> Vec<(&'static str, Vec<Param>, BuiltinFn)> {
    vec![(
        "Csv.Document",
        vec![
            Param { name: "source".into(),  optional: false, type_annotation: None },
            Param { name: "options".into(), optional: true,  type_annotation: None },
        ],
        document,
    )]
}

#[derive(Default)]
struct Options {
    delimiter: Option<u8>,
    quoting: bool,
}

fn document(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let bytes: Vec<u8> = match &args[0] {
        Value::Binary(b) => b.clone(),
        Value::Text(t) => t.as_bytes().to_vec(),
        other => return Err(type_mismatch("binary or text", other)),
    };

    let opts = match args.get(1) {
        None | Some(Value::Null) => Options { delimiter: None, quoting: true },
        Some(Value::Record(r)) => parse_options(r, host)?,
        Some(other) => return Err(type_mismatch("record or null", other)),
    };

    let mut builder = ::csv::ReaderBuilder::new();
    builder.has_headers(false); // PQ leaves headers as row 0; promotion is separate
    if let Some(d) = opts.delimiter {
        builder.delimiter(d);
    }
    builder.quoting(opts.quoting);
    builder.flexible(true); // ragged-row tolerance, normalised below

    let mut reader = builder.from_reader(bytes.as_slice());
    let mut rows: Vec<Vec<Value>> = Vec::new();
    let mut max_width = 0usize;
    for rec in reader.records() {
        let rec = rec.map_err(|e| MError::Other(format!("Csv.Document: {e}")))?;
        let row: Vec<Value> = rec.iter().map(|s| Value::Text(s.to_string())).collect();
        if row.len() > max_width {
            max_width = row.len();
        }
        rows.push(row);
    }
    // Pad short rows with Null so the table is rectangular.
    for r in &mut rows {
        while r.len() < max_width {
            r.push(Value::Null);
        }
    }

    let columns: Vec<String> = (1..=max_width).map(|i| format!("Column{i}")).collect();
    Ok(Value::Table(Table::from_rows(columns, rows)))
}

fn parse_options(r: &Record, host: &dyn IoHost) -> Result<Options, MError> {
    let mut out = Options { delimiter: None, quoting: true };
    for (k, v) in &r.fields {
        let v = force_value(v.clone(), host)?;
        match k.as_str() {
            "Delimiter" => match v {
                Value::Text(s) => {
                    let bytes = s.as_bytes();
                    if bytes.len() != 1 {
                        return Err(MError::Other(format!(
                            "Csv.Document: Delimiter must be a single-byte character, got {s:?}"
                        )));
                    }
                    out.delimiter = Some(bytes[0]);
                }
                Value::Null => {}
                other => return Err(type_mismatch("text", &other)),
            },
            "QuoteStyle" => match v {
                // QuoteStyle.Csv = 1 (RFC 4180 quoting), QuoteStyle.None = 0
                Value::Number(n) if n == 0.0 => out.quoting = false,
                Value::Number(n) if n == 1.0 => out.quoting = true,
                Value::Null => {}
                Value::Number(n) => {
                    return Err(MError::Other(format!(
                        "Csv.Document: QuoteStyle must be 0 (None) or 1 (Csv), got {n}"
                    )));
                }
                other => return Err(type_mismatch("number", &other)),
            },
            "Encoding" => match v {
                // 65001 is UTF-8 (our only supported code page). Reject
                // others rather than silently mis-decode.
                Value::Number(n) if n == 65001.0 => {}
                Value::Null => {}
                Value::Number(n) => {
                    return Err(MError::Other(format!(
                        "Csv.Document: Encoding={n} not supported (only 65001/UTF-8)"
                    )));
                }
                other => return Err(type_mismatch("number", &other)),
            },
            _ => {} // Columns and other PQ fields ignored for v1
        }
    }
    Ok(out)
}

fn force_value(v: Value, host: &dyn IoHost) -> Result<Value, MError> {
    super::super::force(v, &mut |e, env| super::super::evaluate(e, env, host))
}
