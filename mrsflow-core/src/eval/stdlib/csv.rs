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
    encoding: u32,
}

fn document(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    let mut bytes: Vec<u8> = match &args[0] {
        Value::Binary(b) => b.clone(),
        Value::Text(t) => t.as_bytes().to_vec(),
        other => return Err(type_mismatch("binary or text", other)),
    };

    let opts = match args.get(1) {
        None | Some(Value::Null) => Options { delimiter: None, quoting: true, encoding: 65001 },
        Some(Value::Record(r)) => parse_options(r, host)?,
        Some(other) => return Err(type_mismatch("record or null", other)),
    };

    // CP-1252 decode: map each byte through Windows-1252 to its Unicode
    // code point, then re-encode as UTF-8. Bytes 0x00..0x7F are ASCII;
    // 0xA0..0xFF map to Latin-1 (same code point); 0x80..0x9F have a
    // few special mappings.
    if opts.encoding == 1252 {
        bytes = cp1252_to_utf8(&bytes);
    }
    // Skip UTF-8 BOM if present.
    if bytes.starts_with(&[0xEF, 0xBB, 0xBF]) {
        bytes.drain(0..3);
    }

    let mut builder = ::csv::ReaderBuilder::new();
    builder.has_headers(false); // PQ leaves headers as row 0; promotion is separate
    if let Some(d) = opts.delimiter {
        builder.delimiter(d);
    }
    // Quote handling matching Power Query's QuoteStyle semantics:
    //   - QuoteStyle.Csv (default): RFC 4180 — quotes delimit fields,
    //     `""` inside a quoted field is an escape for a literal `"`.
    //     csv crate: quoting=true, double_quote=true (default).
    //   - QuoteStyle.None: quotes still delimit fields, but `""` is NOT
    //     an escape. PQ's reading; verified against Excel q10 in
    //     Oracle/cases/. csv crate: quoting=true, double_quote=false.
    // The naive interpretation ("quoting=false makes every quote
    // literal") splits `a,"b,c",d` into 4 fields, which diverged from
    // Excel's 3 in the oracle round.
    builder.quoting(true);
    builder.double_quote(opts.quoting);
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
    let mut out = Options { delimiter: None, quoting: true, encoding: 65001 };
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
                Value::Number(n) if n == 65001.0 => out.encoding = 65001,
                Value::Number(n) if n == 1252.0  => out.encoding = 1252,
                Value::Null => {}
                Value::Number(n) => {
                    return Err(MError::Other(format!(
                        "Csv.Document: Encoding={n} not supported (65001/UTF-8 or 1252/CP1252)"
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

/// Decode CP-1252 bytes into a UTF-8 byte vector. 0x00..0x7F passes through
/// (ASCII); 0xA0..0xFF maps directly to Latin-1 Unicode code points; the
/// 0x80..0x9F range has Windows-specific mappings (€, …).
fn cp1252_to_utf8(input: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(input.len());
    for &b in input {
        let cp: u32 = match b {
            0x80 => 0x20AC, 0x82 => 0x201A, 0x83 => 0x0192, 0x84 => 0x201E,
            0x85 => 0x2026, 0x86 => 0x2020, 0x87 => 0x2021, 0x88 => 0x02C6,
            0x89 => 0x2030, 0x8A => 0x0160, 0x8B => 0x2039, 0x8C => 0x0152,
            0x8E => 0x017D, 0x91 => 0x2018, 0x92 => 0x2019, 0x93 => 0x201C,
            0x94 => 0x201D, 0x95 => 0x2022, 0x96 => 0x2013, 0x97 => 0x2014,
            0x98 => 0x02DC, 0x99 => 0x2122, 0x9A => 0x0161, 0x9B => 0x203A,
            0x9C => 0x0153, 0x9E => 0x017E, 0x9F => 0x0178,
            0x81 | 0x8D | 0x8F | 0x90 | 0x9D => b as u32, // undefined — pass through
            _ => b as u32,                                 // 0x00..0x7F + 0xA0..0xFF
        };
        if let Some(c) = char::from_u32(cp) {
            let mut buf = [0u8; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    }
    out
}
