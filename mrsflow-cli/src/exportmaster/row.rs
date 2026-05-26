//! Row parser — decodes record bytes into per-column values, dispatching
//! on the ftType sub-code from protocol §6b. Returns one decoded cell
//! per column per row; the caller accumulates into per-column Arrow
//! `ArrayRef`s via [`ColumnBuilders`].
//!
//! On-disk row layout (protocol §4):
//! ```text
//! +0    9 bytes    header (8-byte LE TDateTime at +1..+8, byte +0 = type/flag)
//! +9    16 bytes   MD5 hash of record[25..end]
//! +25   N bytes    field data, walked via schema.row_offset / schema.max
//! ```
//! Per-field format:
//! ```text
//! +0    1 byte     null-indicator: 0x00 = NULL, 0x01 = not null
//! +1    max bytes  value data, format depends on FieldType (§6b)
//! ```

use std::sync::Arc;

use arrow::array::{
    Array, ArrayRef, BinaryBuilder, BooleanArray, Date32Array, Float64Array, Int32Array,
    Int64Array, StringArray, TimestampMicrosecondArray,
};
use arrow::datatypes::{DataType, Field, Schema, TimeUnit};
use arrow::record_batch::RecordBatch;
use chrono::{Datelike, NaiveDate, NaiveDateTime, NaiveTime, Timelike};

use mrsflow_core::eval::IoError;

use super::schema::{Column, FieldType};

/// Header bytes preceding the record proper.
pub const RECORD_HEADER_LEN: usize = 25;

/// Decoded cell — what we accumulate per column.
#[derive(Debug, Clone)]
pub enum CellValue {
    Null,
    Text(String),
    Date(NaiveDate),
    DateTime(NaiveDateTime),
    Time(NaiveTime),
    Int32(i32),
    Int64(i64),
    Float(f64),
    Bool(bool),
    Binary(Vec<u8>),
    /// 8-byte blob handle (sub=3). Caller resolves via the blob fetch
    /// (protocol §6a) after the whole row is decoded.
    BlobHandle([u8; 8]),
}

/// Decode one record into per-column [`CellValue`]s, one per schema column.
///
/// `record` points to the **first column's null-flag byte on the wire**.
///
/// Schema arithmetic (confirmed against live wire capture):
/// - `c.row_offset` is the position of the field's null-flag byte
///   within the on-disk record.
/// - Each field is `1 (null-flag) + max (value)` bytes.
/// - On the wire, the on-disk header (25 bytes for CUSTOMER) is absent;
///   wire null-flag = `row_offset - first_col.row_offset`.
/// - Value starts one byte past the null-flag and is `max` bytes long.
pub fn decode_record(record: &[u8], columns: &[Column]) -> Result<Vec<CellValue>, IoError> {
    let first_offset = columns.first().map(|c| c.row_offset as usize).unwrap_or(0);
    let mut out = Vec::with_capacity(columns.len());
    for c in columns {
        let null_pos = (c.row_offset as usize).saturating_sub(first_offset);
        let value_start = null_pos + 1;
        if null_pos >= record.len() {
            return Err(IoError::Other(format!(
                "Exportmaster: row truncated; column {} (null at {}) past end (len {})",
                c.name, null_pos, record.len()
            )));
        }
        let null_flag = record[null_pos];
        if null_flag == 0 {
            out.push(CellValue::Null);
            continue;
        }
        if null_flag != 1 {
            return Err(IoError::Other(format!(
                "Exportmaster: bad null-indicator 0x{null_flag:02X} for column {} at offset {}",
                c.name, null_pos
            )));
        }
        let value_len = c.max as usize;
        let avail = record.len().saturating_sub(value_start);
        if value_len > avail {
            return Err(IoError::Other(format!(
                "Exportmaster: row truncated within column {}: need {} bytes, have {}",
                c.name, value_len, avail
            )));
        }
        let bytes = &record[value_start..value_start + value_len];
        out.push(decode_field(c, bytes)?);
    }
    Ok(out)
}

fn decode_field(col: &Column, bytes: &[u8]) -> Result<CellValue, IoError> {
    use FieldType::*;
    Ok(match col.field_type {
        Calculated => CellValue::Null, // no storage
        String => {
            // On-disk ftString: ASCII chars, null-terminated within a
            // (max - 1)-byte value buffer. No length prefix on the
            // wire — the null flag handled separately, and any trailing
            // bytes past the first 0x00 are zero-padding.
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            let s = std::string::String::from_utf8_lossy(&bytes[..end]).into_owned();
            CellValue::Text(s)
        }
        Date => {
            // 4-byte LE u32, days since 0001-01-01 (proleptic Gregorian).
            // Out-of-range values (garbage / sentinel rows in wide tables)
            // surface as Null rather than killing the query.
            let days = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let base = NaiveDate::from_ymd_opt(1, 1, 1).expect("0001-01-01 is valid");
            match base.checked_add_days(chrono::Days::new(days as u64)) {
                Some(date) => CellValue::Date(date),
                None => CellValue::Null,
            }
        }
        DateTime => {
            // 8-byte LE binary64 double, days since 1899-12-30 (Delphi TDateTime).
            // Out-of-range / NaN / sentinel values surface as Null rather
            // than killing the query — wide tables often contain "0000-00-00"
            // garbage rows whose payload is uninitialised memory.
            let serial = f64::from_le_bytes(bytes[..8].try_into().unwrap());
            // chrono::Duration::days panics outside ~i64::MAX/86_400_000 ms,
            // and NaiveDate's valid range is [-262144, 262143] years. Clamp
            // whole_days to chrono::Days::new's safe range conservatively.
            if !serial.is_finite() {
                CellValue::Null
            } else {
                let whole_days = serial.trunc() as i64;
                // chrono::Days range is conservative: keep whole_days within
                // ±100 million (~270k years), well inside chrono's limits.
                if whole_days.abs() > 100_000_000 {
                    CellValue::Null
                } else {
                    let epoch = NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
                    let date_opt = epoch.checked_add_signed(chrono::Duration::days(whole_days));
                    let frac = serial - whole_days as f64;
                    let micros = (frac.abs() * 86_400_000_000.0).round() as u64;
                    let secs = (micros / 1_000_000) as u32;
                    let us_part = (micros % 1_000_000) as u32;
                    let time_opt = NaiveTime::from_num_seconds_from_midnight_opt(secs, us_part * 1000);
                    match (date_opt, time_opt) {
                        (Some(date), Some(time)) => CellValue::DateTime(NaiveDateTime::new(date, time)),
                        _ => CellValue::Null,
                    }
                }
            }
        }
        Time => {
            // 4-byte LE u32, milliseconds since midnight.
            let ms = u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            let secs = ms / 1000;
            let ns = (ms % 1000) * 1_000_000;
            let t = NaiveTime::from_num_seconds_from_midnight_opt(secs, ns)
                .ok_or_else(|| IoError::Other(format!(
                    "Exportmaster: Time column {} out of range: {} ms",
                    col.name, ms
                )))?;
            CellValue::Time(t)
        }
        Integer | AutoInc => {
            let n = i32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]);
            CellValue::Int32(n)
        }
        Smallint => {
            let n = i16::from_le_bytes([bytes[0], bytes[1]]);
            CellValue::Int32(n as i32)
        }
        Largeint => {
            let n = i64::from_le_bytes(bytes[..8].try_into().unwrap());
            CellValue::Int64(n)
        }
        Boolean => {
            // 2-byte WordBool: FFFF = true, 0000 = false. Any other
            // pattern is treated as false (the documented values are
            // strict 0 and -1; we don't seen anything else in practice).
            let v = u16::from_le_bytes([bytes[0], bytes[1]]);
            CellValue::Bool(v != 0)
        }
        Float => {
            let v = f64::from_le_bytes(bytes[..8].try_into().unwrap());
            CellValue::Float(v)
        }
        Currency => {
            // 8-byte LE Int64, scaled by 10_000 (DBISAM Currency stores
            // four decimal places as a fixed-point integer).
            let raw = i64::from_le_bytes(bytes[..8].try_into().unwrap());
            CellValue::Float(raw as f64 / 10_000.0)
        }
        Blob | Memo | Graphic => {
            // 8-byte blob handle. The actual content lives server-side;
            // caller fetches it via the §6a 0x0280 reqcode. We carry
            // the opaque handle through to the column builder, which
            // either fetches it lazily (TODO: future) or surfaces the
            // raw bytes as Binary in v1.
            let mut h = [0u8; 8];
            h.copy_from_slice(&bytes[..8]);
            CellValue::BlobHandle(h)
        }
        Bytes => CellValue::Binary(bytes.to_vec()),
        VarBytes => {
            // Length-prefixed binary. Format not yet captured from a
            // live workbook; the protocol doc lists "up to N bytes,
            // variable-length binary; length prefix used" without
            // pinning the prefix width. For v1, treat the whole
            // value-buffer as opaque bytes — same as Bytes — and
            // refine when a real VarBytes column surfaces.
            CellValue::Binary(bytes.to_vec())
        }
        Unknown(sub, a8, t250) => {
            return Err(IoError::Other(format!(
                "Exportmaster: unsupported field type for column {} (sub={sub} +A8={a8:02X} +250={t250:02X})",
                col.name
            )));
        }
    })
}

/// Builds Arrow arrays from per-column [`CellValue`] sequences, one
/// builder per schema column. Push one [`CellValue`] per cell in row-
/// major order via [`Self::push_row`]; finalise into a `RecordBatch`
/// via [`Self::finish`].
pub struct ColumnBuilders<'a> {
    columns: &'a [Column],
    /// One per column, indexed in schema order. Each is the partly-built
    /// data for that column; finalised into an `ArrayRef` in `finish`.
    inner: Vec<ColumnBuilder>,
}

enum ColumnBuilder {
    Text(Vec<Option<String>>),
    Date(Vec<Option<i32>>),
    DateTime(Vec<Option<i64>>),
    Time(Vec<Option<i64>>), // microseconds since midnight
    Int32(Vec<Option<i32>>),
    Int64(Vec<Option<i64>>),
    Float(Vec<Option<f64>>),
    Bool(Vec<Option<bool>>),
    /// Blob handles end up here for v1 (8 raw bytes); a later commit
    /// will swap this for resolved blob content.
    Binary(Vec<Option<Vec<u8>>>),
    /// `Calculated` columns are skipped — they have no storage. We keep
    /// the builder slot so column ordering by index stays aligned, but
    /// `push_value` is a no-op and `finish` produces a single-null column.
    Calculated(usize), // row count
}

impl ColumnBuilder {
    fn for_type(field_type: FieldType, capacity: usize) -> Self {
        use FieldType::*;
        match field_type {
            String => ColumnBuilder::Text(Vec::with_capacity(capacity)),
            Date => ColumnBuilder::Date(Vec::with_capacity(capacity)),
            DateTime => ColumnBuilder::DateTime(Vec::with_capacity(capacity)),
            Time => ColumnBuilder::Time(Vec::with_capacity(capacity)),
            Integer | AutoInc | Smallint => ColumnBuilder::Int32(Vec::with_capacity(capacity)),
            Largeint => ColumnBuilder::Int64(Vec::with_capacity(capacity)),
            Boolean => ColumnBuilder::Bool(Vec::with_capacity(capacity)),
            Float | Currency => ColumnBuilder::Float(Vec::with_capacity(capacity)),
            Blob | Memo | Graphic | Bytes | VarBytes => {
                ColumnBuilder::Binary(Vec::with_capacity(capacity))
            }
            Calculated | Unknown(..) => ColumnBuilder::Calculated(0),
        }
    }

    fn push(&mut self, v: CellValue) {
        match (self, v) {
            (ColumnBuilder::Text(b), CellValue::Text(s)) => b.push(Some(s)),
            (ColumnBuilder::Text(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::Date(b), CellValue::Date(d)) => {
                let epoch = NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
                b.push(Some((d - epoch).num_days() as i32))
            }
            (ColumnBuilder::Date(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::DateTime(b), CellValue::DateTime(dt)) => {
                b.push(Some(dt.and_utc().timestamp_micros()))
            }
            (ColumnBuilder::DateTime(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::Time(b), CellValue::Time(t)) => {
                let us = t.num_seconds_from_midnight() as i64 * 1_000_000
                    + t.nanosecond() as i64 / 1000;
                b.push(Some(us))
            }
            (ColumnBuilder::Time(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::Int32(b), CellValue::Int32(n)) => b.push(Some(n)),
            (ColumnBuilder::Int32(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::Int64(b), CellValue::Int64(n)) => b.push(Some(n)),
            (ColumnBuilder::Int64(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::Float(b), CellValue::Float(v)) => b.push(Some(v)),
            (ColumnBuilder::Float(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::Bool(b), CellValue::Bool(v)) => b.push(Some(v)),
            (ColumnBuilder::Bool(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::Binary(b), CellValue::Binary(v)) => b.push(Some(v)),
            (ColumnBuilder::Binary(b), CellValue::BlobHandle(h)) => b.push(Some(h.to_vec())),
            (ColumnBuilder::Binary(b), CellValue::Null) => b.push(None),
            (ColumnBuilder::Calculated(n), _) => *n += 1,
            // Cell type doesn't match builder — fall through to null. The
            // schema parser is the type authority; mismatches here should
            // not happen in practice (decode_field follows the same enum)
            // but defaulting to Null avoids panicking on a surprise.
            (other, _) => {
                push_null_into(other);
            }
        }
    }

    fn finish(self) -> (DataType, ArrayRef) {
        match self {
            ColumnBuilder::Text(v) => (DataType::Utf8, Arc::new(StringArray::from(v)) as ArrayRef),
            ColumnBuilder::Date(v) => (DataType::Date32, Arc::new(Date32Array::from(v)) as ArrayRef),
            ColumnBuilder::DateTime(v) => (
                DataType::Timestamp(TimeUnit::Microsecond, None),
                Arc::new(TimestampMicrosecondArray::from(v)) as ArrayRef,
            ),
            ColumnBuilder::Time(v) => (
                // Use Int64 microseconds (no Arrow Time array in the
                // pipeline; downstream code treats this consistently).
                DataType::Int64,
                Arc::new(Int64Array::from(v)) as ArrayRef,
            ),
            ColumnBuilder::Int32(v) => (DataType::Int32, Arc::new(Int32Array::from(v)) as ArrayRef),
            ColumnBuilder::Int64(v) => (DataType::Int64, Arc::new(Int64Array::from(v)) as ArrayRef),
            ColumnBuilder::Float(v) => (DataType::Float64, Arc::new(Float64Array::from(v)) as ArrayRef),
            ColumnBuilder::Bool(v) => (DataType::Boolean, Arc::new(BooleanArray::from(v)) as ArrayRef),
            ColumnBuilder::Binary(v) => {
                let mut b = BinaryBuilder::with_capacity(v.len(), v.iter().filter_map(|x| x.as_ref()).map(|x| x.len()).sum());
                for cell in v {
                    match cell {
                        Some(bytes) => b.append_value(&bytes),
                        None => b.append_null(),
                    }
                }
                (DataType::Binary, Arc::new(b.finish()) as ArrayRef)
            }
            ColumnBuilder::Calculated(n) => {
                // All-null Utf8 column of the right length.
                let v: Vec<Option<String>> = vec![None; n];
                (DataType::Utf8, Arc::new(StringArray::from(v)) as ArrayRef)
            }
        }
    }
}

fn push_null_into(b: &mut ColumnBuilder) {
    match b {
        ColumnBuilder::Text(v) => v.push(None),
        ColumnBuilder::Date(v) => v.push(None),
        ColumnBuilder::DateTime(v) => v.push(None),
        ColumnBuilder::Time(v) => v.push(None),
        ColumnBuilder::Int32(v) => v.push(None),
        ColumnBuilder::Int64(v) => v.push(None),
        ColumnBuilder::Float(v) => v.push(None),
        ColumnBuilder::Bool(v) => v.push(None),
        ColumnBuilder::Binary(v) => v.push(None),
        ColumnBuilder::Calculated(n) => *n += 1,
    }
}

impl<'a> ColumnBuilders<'a> {
    pub fn new(columns: &'a [Column], capacity: usize) -> Self {
        let inner = columns
            .iter()
            .map(|c| ColumnBuilder::for_type(c.field_type, capacity))
            .collect();
        Self { columns, inner }
    }

    pub fn push_row(&mut self, cells: Vec<CellValue>) -> Result<(), IoError> {
        if cells.len() != self.inner.len() {
            return Err(IoError::Other(format!(
                "Exportmaster: row has {} cells, schema has {}",
                cells.len(),
                self.inner.len()
            )));
        }
        for (b, c) in self.inner.iter_mut().zip(cells.into_iter()) {
            b.push(c);
        }
        Ok(())
    }

    pub fn finish(self) -> Result<RecordBatch, IoError> {
        let mut fields = Vec::with_capacity(self.inner.len());
        let mut arrays = Vec::with_capacity(self.inner.len());
        for (col, builder) in self.columns.iter().zip(self.inner.into_iter()) {
            let (dtype, arr) = builder.finish();
            fields.push(Field::new(col.name.clone(), dtype, true));
            arrays.push(arr);
        }
        let schema = Arc::new(Schema::new(fields));
        RecordBatch::try_new(schema, arrays)
            .map_err(|e| IoError::Other(format!("Exportmaster: RecordBatch::try_new: {e}")))
    }
}
