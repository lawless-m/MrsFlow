//! `Date.*` stdlib bindings.

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
        (
            "#date",
            three("year", "month", "day"),
            constructor,
        ),
        (
            "Date.FromText",
            vec![
                Param { name: "text".into(),    optional: false, type_annotation: None },
                Param { name: "options".into(), optional: true,  type_annotation: None },
            ],
            from_text,
        ),
        ("Date.AddDays", two("date", "numberOfDays"), add_days),
        ("Date.AddMonths", two("date", "numberOfMonths"), add_months),
        ("Date.AddYears", two("date", "numberOfYears"), add_years),
        ("Date.AddQuarters", two("date", "numberOfQuarters"), add_quarters),
        ("Date.AddWeeks", two("date", "numberOfWeeks"), add_weeks),
        (
            "Date.DayOfWeek",
            vec![
                Param { name: "date".into(),            optional: false, type_annotation: None },
                Param { name: "firstDayOfWeek".into(),  optional: true,  type_annotation: None },
            ],
            day_of_week,
        ),
        (
            "Date.DayOfWeekName",
            vec![
                Param { name: "date".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            day_of_week_name,
        ),
        ("Date.DayOfYear", one("date"), day_of_year),
        ("Date.DaysInMonth", one("date"), days_in_month),
        (
            "Date.MonthName",
            vec![
                Param { name: "date".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            month_name,
        ),
        ("Date.QuarterOfYear", one("date"), quarter_of_year),
        ("Date.WeekOfMonth", one("date"), week_of_month),
        (
            "Date.WeekOfYear",
            vec![
                Param { name: "date".into(),           optional: false, type_annotation: None },
                Param { name: "firstDayOfWeek".into(), optional: true,  type_annotation: None },
            ],
            week_of_year,
        ),
        ("Date.IsLeapYear", one("date"), is_leap_year),
        ("Date.ToRecord", one("date"), to_record),
        ("Date.StartOfDay", one("date"), start_of_day),
        ("Date.EndOfDay", one("date"), end_of_day),
        (
            "Date.StartOfWeek",
            vec![
                Param { name: "date".into(),           optional: false, type_annotation: None },
                Param { name: "firstDayOfWeek".into(), optional: true,  type_annotation: None },
            ],
            start_of_week,
        ),
        (
            "Date.EndOfWeek",
            vec![
                Param { name: "date".into(),           optional: false, type_annotation: None },
                Param { name: "firstDayOfWeek".into(), optional: true,  type_annotation: None },
            ],
            end_of_week,
        ),
        ("Date.StartOfMonth", one("date"), start_of_month),
        ("Date.EndOfMonth", one("date"), end_of_month),
        ("Date.StartOfQuarter", one("date"), start_of_quarter),
        ("Date.EndOfQuarter", one("date"), end_of_quarter),
        ("Date.StartOfYear", one("date"), start_of_year),
        ("Date.EndOfYear", one("date"), end_of_year),
        ("Date.IsInCurrentDay", one("date"), is_in_current_day),
        ("Date.IsInCurrentWeek", one("date"), is_in_current_week),
        ("Date.IsInCurrentMonth", one("date"), is_in_current_month),
        ("Date.IsInCurrentQuarter", one("date"), is_in_current_quarter),
        ("Date.IsInCurrentYear", one("date"), is_in_current_year),
        ("Date.IsInNextDay", one("date"), is_in_next_day),
        ("Date.IsInNextWeek", one("date"), is_in_next_week),
        ("Date.IsInNextMonth", one("date"), is_in_next_month),
        ("Date.IsInNextQuarter", one("date"), is_in_next_quarter),
        ("Date.IsInNextYear", one("date"), is_in_next_year),
        ("Date.IsInPreviousDay", one("date"), is_in_previous_day),
        ("Date.IsInPreviousWeek", one("date"), is_in_previous_week),
        ("Date.IsInPreviousMonth", one("date"), is_in_previous_month),
        ("Date.IsInPreviousQuarter", one("date"), is_in_previous_quarter),
        ("Date.IsInPreviousYear", one("date"), is_in_previous_year),
        ("Date.IsInNextNDays", two("date", "numberOfDays"), is_in_next_n_days),
        ("Date.IsInNextNWeeks", two("date", "numberOfWeeks"), is_in_next_n_weeks),
        ("Date.IsInNextNMonths", two("date", "numberOfMonths"), is_in_next_n_months),
        ("Date.IsInNextNQuarters", two("date", "numberOfQuarters"), is_in_next_n_quarters),
        ("Date.IsInNextNYears", two("date", "numberOfYears"), is_in_next_n_years),
        ("Date.IsInPreviousNDays", two("date", "numberOfDays"), is_in_previous_n_days),
        ("Date.IsInPreviousNWeeks", two("date", "numberOfWeeks"), is_in_previous_n_weeks),
        ("Date.IsInPreviousNMonths", two("date", "numberOfMonths"), is_in_previous_n_months),
        ("Date.IsInPreviousNQuarters", two("date", "numberOfQuarters"), is_in_previous_n_quarters),
        ("Date.IsInPreviousNYears", two("date", "numberOfYears"), is_in_previous_n_years),
        ("Date.IsInYearToDate", one("date"), is_in_year_to_date),
        ("Date.From", one("value"), from),
        ("Date.Year", one("date"), year),
        ("Date.Month", one("date"), month),
        ("Date.Day", one("date"), day),
        (
            "Date.ToText",
            vec![
                Param { name: "date".into(),    optional: false, type_annotation: None },
                Param { name: "format".into(),  optional: true,  type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            to_text,
        ),
    ]
}

fn constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#date: year")?;
    let mo = expect_int(&args[1], "#date: month")?;
    let d = expect_int(&args[2], "#date: day")?;
    chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .map(Value::Date)
        .ok_or_else(|| MError::Other(format!("#date: invalid date {y}-{mo:02}-{d:02}")))
}


/// Translate an M `Date.ToText` format string into a chrono `format!`
/// spec. Tokens are matched longest-first so `yyyy` doesn't eat as two
/// `yy`s and `MMM` doesn't collide with `MM`. Unrecognised characters
/// pass through literally; an unrecognised letter run raises an error
/// so a typo fails loud rather than producing garbage.
fn translate_m_date_format(m_fmt: &str) -> Result<String, MError> {
    let mut out = String::with_capacity(m_fmt.len());
    let bytes = m_fmt.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        // Match runs of the same letter — M format tokens are runs.
        if c.is_ascii_alphabetic() {
            let mut j = i;
            while j < bytes.len() && bytes[j] == c {
                j += 1;
            }
            let run = &m_fmt[i..j];
            let token = match run {
                "yyyy" => "%Y",
                "yy"   => "%y",
                "MMMM" => "%B", // full month name
                "MMM"  => "%b", // abbreviated month
                "MM"   => "%m",
                "M"    => "%-m",
                "dddd" => "%A", // full weekday
                "ddd"  => "%a", // abbreviated weekday
                "dd"   => "%d",
                "d"    => "%-d",
                "HH"   => "%H",
                "H"    => "%-H",
                "hh"   => "%I",
                "h"    => "%-I",
                "mm"   => "%M",
                "ss"   => "%S",
                "tt"   => "%p",
                other => {
                    return Err(MError::Other(format!(
                        "Date.ToText: unsupported format token {other:?} in {m_fmt:?}"
                    )));
                }
            };
            out.push_str(token);
            i = j;
        } else {
            // Literal character — chrono needs `%%` for a literal `%`.
            if c == b'%' {
                out.push_str("%%");
            } else {
                out.push(c as char);
            }
            i += 1;
        }
    }
    Ok(out)
}

fn to_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match &args[0] {
        Value::Null => return Ok(Value::Null),
        Value::Date(d) => *d,
        other => return Err(type_mismatch("date", other)),
    };
    let culture = match args.get(2) {
        Some(Value::Text(c)) => Some(c.clone()),
        _ => None,
    };
    let chrono_fmt = match args.get(1) {
        Some(Value::Null) | None => "%Y-%m-%d".to_string(),
        Some(Value::Text(s)) => {
            // Expand single-letter standard codes first, then try the legacy
            // translator (errors on unknown tokens), then fall back to the
            // wider dotnet_to_strftime that handles literal `'…'` quoting.
            let expanded = expand_standard_date_format(s, culture.as_deref());
            match translate_m_date_format(&expanded) {
                Ok(v) => v,
                Err(_) => dotnet_to_strftime(&expanded),
            }
        }
        Some(other) => return Err(type_mismatch("text or null", other)),
    };
    Ok(Value::Text(d.format(&chrono_fmt).to_string()))
}


fn from(args: &[Value], host: &dyn IoHost) -> Result<Value, MError> {
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Date(*d)),
        Value::Datetime(dt) => Ok(Value::Date(dt.date())),
        Value::Text(_) => from_text(args, host),
        Value::Number(n) => {
            // PQ Date.From accepts a number as an OLE serial date —
            // days since 1899-12-30 (the day before Lotus's spurious
            // 1900 leap year). 45000 → 2023-03-15 confirms via the
            // Excel-side oracle.
            if !n.is_finite() {
                return Err(MError::Other(format!(
                    "Date.From: cannot convert non-finite number {n}"
                )));
            }
            let epoch = chrono::NaiveDate::from_ymd_opt(1899, 12, 30).unwrap();
            let days = n.trunc() as i64;
            match epoch.checked_add_signed(chrono::Duration::days(days)) {
                Some(d) => Ok(Value::Date(d)),
                None => Err(MError::Other(format!(
                    "Date.From: serial date {n} out of range"
                ))),
            }
        }
        other => Err(type_mismatch("date/datetime/text/number/null", other)),
    }
}


fn year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.year() as f64)),
        Value::Datetime(dt) => Ok(Value::Number(dt.year() as f64)),
        Value::Datetimezone(dt) => Ok(Value::Number(dt.year() as f64)),
        other => Err(type_mismatch("date/datetime/datetimezone", other)),
    }
}


fn month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.month() as f64)),
        Value::Datetime(dt) => Ok(Value::Number(dt.month() as f64)),
        Value::Datetimezone(dt) => Ok(Value::Number(dt.month() as f64)),
        other => Err(type_mismatch("date/datetime/datetimezone", other)),
    }
}


fn day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Number(d.day() as f64)),
        Value::Datetime(dt) => Ok(Value::Number(dt.day() as f64)),
        Value::Datetimezone(dt) => Ok(Value::Number(dt.day() as f64)),
        other => Err(type_mismatch("date/datetime/datetimezone", other)),
    }
}

/// Helper: extract a NaiveDate from a Date or Datetime cell.
fn extract_naive_date(v: &Value, ctx: &str) -> Result<chrono::NaiveDate, MError> {
    match v {
        Value::Date(d) => Ok(*d),
        Value::Datetime(dt) => Ok(dt.date()),
        other => Err(MError::Other(format!(
            "{}: argument must be a date or datetime (got {})",
            ctx,
            super::super::type_name(other)
        ))),
    }
}


fn day_of_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.DayOfWeek")?;
    let first = match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 && *n <= 6.0 => *n as u32,
        Some(Value::Null) | None => 1, // PQ default: Monday
        Some(other) => return Err(type_mismatch("integer 0..6 (Day.*)", other)),
    };
    // chrono's Weekday::num_days_from_monday returns 0..6 starting Monday=0.
    let from_monday = d.weekday().num_days_from_monday();
    // Sunday(0), Monday(1), ..., Saturday(6) → want days since `first`.
    let dow_sunday_first = (from_monday + 1) % 7; // 0=Sunday, 1=Monday, ...
    let result = (dow_sunday_first + 7 - first) % 7;
    Ok(Value::Number(result as f64))
}


fn day_of_week_name(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.DayOfWeekName")?;
    let name = match d.weekday() {
        chrono::Weekday::Mon => "Monday",
        chrono::Weekday::Tue => "Tuesday",
        chrono::Weekday::Wed => "Wednesday",
        chrono::Weekday::Thu => "Thursday",
        chrono::Weekday::Fri => "Friday",
        chrono::Weekday::Sat => "Saturday",
        chrono::Weekday::Sun => "Sunday",
    };
    Ok(Value::Text(name.into()))
}


fn day_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.DayOfYear")?;
    Ok(Value::Number(d.ordinal() as f64))
}


fn days_in_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.DaysInMonth")?;
    let (year, month) = (d.year(), d.month());
    // Days in month: chrono helper isn't directly available without features; compute.
    let next = if month == 12 {
        chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
    } else {
        chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
    };
    let first = chrono::NaiveDate::from_ymd_opt(year, month, 1).unwrap();
    let n = next.unwrap().signed_duration_since(first).num_days();
    Ok(Value::Number(n as f64))
}


fn month_name(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.MonthName")?;
    let name = match d.month() {
        1 => "January",
        2 => "February",
        3 => "March",
        4 => "April",
        5 => "May",
        6 => "June",
        7 => "July",
        8 => "August",
        9 => "September",
        10 => "October",
        11 => "November",
        12 => "December",
        _ => unreachable!(),
    };
    Ok(Value::Text(name.into()))
}


fn quarter_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.QuarterOfYear")?;
    Ok(Value::Number(((d.month() - 1) / 3 + 1) as f64))
}


fn week_of_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.WeekOfMonth")?;
    // PQ counts weeks bounded by the first-day-of-week (default Monday). Partial
    // week at the start of the month is week 1. Compute by aligning to the
    // week-start of day-1 and counting whole weeks past it, then +1.
    let first = first_day_of_week_arg(args.get(1), "Date.WeekOfMonth")?;
    let first_of_month = chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap();
    let dow = |dt: chrono::NaiveDate| -> u32 {
        let from_mon = dt.weekday().num_days_from_monday();
        let sunday_first = (from_mon + 1) % 7;
        (sunday_first + 7 - first) % 7
    };
    let week_start_of_first = first_of_month - chrono::Duration::days(dow(first_of_month) as i64);
    let days_since = d.signed_duration_since(week_start_of_first).num_days();
    Ok(Value::Number((days_since / 7 + 1) as f64))
}


fn week_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.WeekOfYear")?;
    // PQ's WeekOfYear: partial week at the start of the year is week 1.
    // Anchored to first-day-of-week (default Monday).
    let first = first_day_of_week_arg(args.get(1), "Date.WeekOfYear")?;
    let jan1 = chrono::NaiveDate::from_ymd_opt(d.year(), 1, 1).unwrap();
    let dow = |dt: chrono::NaiveDate| -> u32 {
        let from_mon = dt.weekday().num_days_from_monday();
        let sunday_first = (from_mon + 1) % 7;
        (sunday_first + 7 - first) % 7
    };
    let week_start_of_jan1 = jan1 - chrono::Duration::days(dow(jan1) as i64);
    let days_since = d.signed_duration_since(week_start_of_jan1).num_days();
    Ok(Value::Number((days_since / 7 + 1) as f64))
}


fn is_leap_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.IsLeapYear")?;
    let y = d.year();
    let leap = (y % 4 == 0 && y % 100 != 0) || (y % 400 == 0);
    Ok(Value::Logical(leap))
}

/// Apply a date-shape function while preserving Date vs Datetime kind.
/// For Datetime, `start` controls whether the returned time is 00:00:00
/// (start of day) or 23:59:59.999999 (end of day).
fn shape_date(v: &Value, ctx: &str, start: bool, f: impl Fn(chrono::NaiveDate) -> chrono::NaiveDate) -> Result<Value, MError> {
    match v {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => Ok(Value::Date(f(*d))),
        Value::Datetime(dt) => {
            let new_date = f(dt.date());
            let time = if start {
                chrono::NaiveTime::from_hms_opt(0, 0, 0).unwrap()
            } else {
                chrono::NaiveTime::from_hms_nano_opt(23, 59, 59, 999_999_999).unwrap()
            };
            Ok(Value::Datetime(new_date.and_time(time)))
        }
        other => Err(MError::Other(format!(
            "{}: argument must be a date or datetime (got {})",
            ctx, super::super::type_name(other)
        ))),
    }
}


fn start_of_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    shape_date(&args[0], "Date.StartOfDay", true, |d| d)
}


fn end_of_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    shape_date(&args[0], "Date.EndOfDay", false, |d| d)
}


fn start_of_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.StartOfMonth", true, |d| {
        chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap()
    })
}


fn end_of_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.EndOfMonth", false, |d| {
        let (y, m) = (d.year(), d.month());
        let first_next = if m == 12 {
            chrono::NaiveDate::from_ymd_opt(y + 1, 1, 1).unwrap()
        } else {
            chrono::NaiveDate::from_ymd_opt(y, m + 1, 1).unwrap()
        };
        first_next.pred_opt().unwrap()
    })
}


fn start_of_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.StartOfQuarter", true, |d| {
        let q_start_month = ((d.month() - 1) / 3) * 3 + 1;
        chrono::NaiveDate::from_ymd_opt(d.year(), q_start_month, 1).unwrap()
    })
}


fn end_of_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.EndOfQuarter", false, |d| {
        let q_end_month = ((d.month() - 1) / 3) * 3 + 3;
        let first_next = if q_end_month == 12 {
            chrono::NaiveDate::from_ymd_opt(d.year() + 1, 1, 1).unwrap()
        } else {
            chrono::NaiveDate::from_ymd_opt(d.year(), q_end_month + 1, 1).unwrap()
        };
        first_next.pred_opt().unwrap()
    })
}


fn start_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.StartOfYear", true, |d| {
        chrono::NaiveDate::from_ymd_opt(d.year(), 1, 1).unwrap()
    })
}


fn end_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.EndOfYear", false, |d| {
        chrono::NaiveDate::from_ymd_opt(d.year(), 12, 31).unwrap()
    })
}


fn first_day_of_week_arg(arg: Option<&Value>, ctx: &str) -> Result<u32, MError> {
    match arg {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 && *n <= 6.0 => Ok(*n as u32),
        Some(Value::Null) | None => Ok(1), // PQ default: Monday
        Some(other) => Err(MError::Other(format!(
            "{}: firstDayOfWeek must be 0..6 (got {})", ctx, super::super::type_name(other)
        ))),
    }
}


fn start_of_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    let first = first_day_of_week_arg(args.get(1), "Date.StartOfWeek")?;
    shape_date(&args[0], "Date.StartOfWeek", true, |d| {
        // Days since `first` to step back.
        let from_monday = d.weekday().num_days_from_monday();
        let dow_sunday_first = (from_monday + 1) % 7;
        let back = (dow_sunday_first + 7 - first) % 7;
        d - chrono::Duration::days(back as i64)
    })
}


fn end_of_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    let first = first_day_of_week_arg(args.get(1), "Date.EndOfWeek")?;
    shape_date(&args[0], "Date.EndOfWeek", false, |d| {
        let from_monday = d.weekday().num_days_from_monday();
        let dow_sunday_first = (from_monday + 1) % 7;
        let back = (dow_sunday_first + 7 - first) % 7;
        let start = d - chrono::Duration::days(back as i64);
        start + chrono::Duration::days(6)
    })
}


fn today() -> chrono::NaiveDate {
    chrono::Local::now().date_naive()
}


fn extract_date_opt(v: &Value, ctx: &str) -> Result<Option<chrono::NaiveDate>, MError> {
    match v {
        Value::Null => Ok(None),
        Value::Date(d) => Ok(Some(*d)),
        Value::Datetime(dt) => Ok(Some(dt.date())),
        other => Err(MError::Other(format!(
            "{}: argument must be a date or datetime (got {})",
            ctx, super::super::type_name(other)
        ))),
    }
}


fn start_of_week_naive(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    // PQ default: Monday-start. chrono's num_days_from_monday is already 0=Mon..6=Sun.
    let back = d.weekday().num_days_from_monday();
    d - chrono::Duration::days(back as i64)
}


fn start_of_month_naive(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap()
}


fn start_of_quarter_naive(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    let q = ((d.month() - 1) / 3) * 3 + 1;
    chrono::NaiveDate::from_ymd_opt(d.year(), q, 1).unwrap()
}


fn start_of_year_naive(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    chrono::NaiveDate::from_ymd_opt(d.year(), 1, 1).unwrap()
}


fn end_of_month_naive(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    let next = if d.month() == 12 {
        chrono::NaiveDate::from_ymd_opt(d.year() + 1, 1, 1).unwrap()
    } else {
        chrono::NaiveDate::from_ymd_opt(d.year(), d.month() + 1, 1).unwrap()
    };
    next.pred_opt().unwrap()
}


fn end_of_quarter_naive(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    let q_end = ((d.month() - 1) / 3) * 3 + 3;
    let next = if q_end == 12 {
        chrono::NaiveDate::from_ymd_opt(d.year() + 1, 1, 1).unwrap()
    } else {
        chrono::NaiveDate::from_ymd_opt(d.year(), q_end + 1, 1).unwrap()
    };
    next.pred_opt().unwrap()
}


fn end_of_year_naive(d: chrono::NaiveDate) -> chrono::NaiveDate {
    use chrono::Datelike;
    chrono::NaiveDate::from_ymd_opt(d.year(), 12, 31).unwrap()
}


fn in_range(target: chrono::NaiveDate, start: chrono::NaiveDate, end_inclusive: chrono::NaiveDate) -> bool {
    target >= start && target <= end_inclusive
}

// ----- IsInCurrent* (single-unit, based on today) -----


fn is_in_current_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentDay")? { Some(d) => d, None => return Ok(Value::Null) };
    Ok(Value::Logical(d == today()))
}

fn is_in_current_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentWeek")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_week_naive(today());
    Ok(Value::Logical(in_range(d, s, s + chrono::Duration::days(6))))
}

fn is_in_current_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentMonth")? { Some(d) => d, None => return Ok(Value::Null) };
    let t = today();
    Ok(Value::Logical(in_range(d, start_of_month_naive(t), end_of_month_naive(t))))
}

fn is_in_current_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentQuarter")? { Some(d) => d, None => return Ok(Value::Null) };
    let t = today();
    Ok(Value::Logical(in_range(d, start_of_quarter_naive(t), end_of_quarter_naive(t))))
}

fn is_in_current_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentYear")? { Some(d) => d, None => return Ok(Value::Null) };
    let t = today();
    Ok(Value::Logical(in_range(d, start_of_year_naive(t), end_of_year_naive(t))))
}

// ----- IsInNext* (single-unit) -----


fn is_in_next_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextDay")? { Some(d) => d, None => return Ok(Value::Null) };
    Ok(Value::Logical(d == today() + chrono::Duration::days(1)))
}

fn is_in_next_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextWeek")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_week_naive(today()) + chrono::Duration::days(7);
    Ok(Value::Logical(in_range(d, s, s + chrono::Duration::days(6))))
}

fn is_in_next_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextMonth")? { Some(d) => d, None => return Ok(Value::Null) };
    let next = shift_months_signed(start_of_month_naive(today()), 1).unwrap();
    Ok(Value::Logical(in_range(d, next, end_of_month_naive(next))))
}

fn is_in_next_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextQuarter")? { Some(d) => d, None => return Ok(Value::Null) };
    let next = shift_months_signed(start_of_quarter_naive(today()), 3).unwrap();
    Ok(Value::Logical(in_range(d, next, end_of_quarter_naive(next))))
}

fn is_in_next_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextYear")? { Some(d) => d, None => return Ok(Value::Null) };
    let next = shift_months_signed(start_of_year_naive(today()), 12).unwrap();
    Ok(Value::Logical(in_range(d, next, end_of_year_naive(next))))
}

// ----- IsInPrevious* (single-unit) -----


fn is_in_previous_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousDay")? { Some(d) => d, None => return Ok(Value::Null) };
    Ok(Value::Logical(d == today() - chrono::Duration::days(1)))
}

fn is_in_previous_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousWeek")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_week_naive(today()) - chrono::Duration::days(7);
    Ok(Value::Logical(in_range(d, s, s + chrono::Duration::days(6))))
}

fn is_in_previous_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousMonth")? { Some(d) => d, None => return Ok(Value::Null) };
    let prev = shift_months_signed(start_of_month_naive(today()), -1).unwrap();
    Ok(Value::Logical(in_range(d, prev, end_of_month_naive(prev))))
}

fn is_in_previous_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousQuarter")? { Some(d) => d, None => return Ok(Value::Null) };
    let prev = shift_months_signed(start_of_quarter_naive(today()), -3).unwrap();
    Ok(Value::Logical(in_range(d, prev, end_of_quarter_naive(prev))))
}

fn is_in_previous_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousYear")? { Some(d) => d, None => return Ok(Value::Null) };
    let prev = shift_months_signed(start_of_year_naive(today()), -12).unwrap();
    Ok(Value::Logical(in_range(d, prev, end_of_year_naive(prev))))
}

// ----- IsInNextN* / IsInPreviousN* -----


fn is_in_next_n_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNDays")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNDays")?;
    let start = today() + chrono::Duration::days(1);
    let end = today() + chrono::Duration::days(n);
    Ok(Value::Logical(in_range(d, start, end)))
}

fn is_in_previous_n_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNDays")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNDays")?;
    let start = today() - chrono::Duration::days(n);
    let end = today() - chrono::Duration::days(1);
    Ok(Value::Logical(in_range(d, start, end)))
}

fn is_in_next_n_weeks(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNWeeks")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNWeeks")?;
    let start = start_of_week_naive(today()) + chrono::Duration::weeks(1);
    let end = start + chrono::Duration::weeks(n) - chrono::Duration::days(1);
    Ok(Value::Logical(in_range(d, start, end)))
}

fn is_in_previous_n_weeks(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNWeeks")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNWeeks")?;
    let end = start_of_week_naive(today()) - chrono::Duration::days(1);
    let start = end - chrono::Duration::weeks(n) + chrono::Duration::days(1);
    Ok(Value::Logical(in_range(d, start, end)))
}

fn is_in_next_n_months(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNMonths")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNMonths")?;
    let start = shift_months_signed(start_of_month_naive(today()), 1).unwrap();
    let end_month_start = shift_months_signed(start, n - 1).unwrap();
    Ok(Value::Logical(in_range(d, start, end_of_month_naive(end_month_start))))
}

fn is_in_previous_n_months(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNMonths")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNMonths")?;
    let start = shift_months_signed(start_of_month_naive(today()), -n).unwrap();
    let end = start_of_month_naive(today()) - chrono::Duration::days(1);
    Ok(Value::Logical(in_range(d, start, end)))
}

fn is_in_next_n_quarters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNQuarters")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNQuarters")?;
    let start = shift_months_signed(start_of_quarter_naive(today()), 3).unwrap();
    let end_q = shift_months_signed(start, (n - 1) * 3).unwrap();
    Ok(Value::Logical(in_range(d, start, end_of_quarter_naive(end_q))))
}

fn is_in_previous_n_quarters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNQuarters")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNQuarters")?;
    let start = shift_months_signed(start_of_quarter_naive(today()), -n * 3).unwrap();
    let end = start_of_quarter_naive(today()) - chrono::Duration::days(1);
    Ok(Value::Logical(in_range(d, start, end)))
}

fn is_in_next_n_years(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNYears")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNYears")?;
    let start = shift_months_signed(start_of_year_naive(today()), 12).unwrap();
    let end_y = shift_months_signed(start, (n - 1) * 12).unwrap();
    Ok(Value::Logical(in_range(d, start, end_of_year_naive(end_y))))
}

fn is_in_previous_n_years(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNYears")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNYears")?;
    let start = shift_months_signed(start_of_year_naive(today()), -n * 12).unwrap();
    let end = start_of_year_naive(today()) - chrono::Duration::days(1);
    Ok(Value::Logical(in_range(d, start, end)))
}


fn is_in_year_to_date(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInYearToDate")? { Some(d) => d, None => return Ok(Value::Null) };
    let t = today();
    Ok(Value::Logical(in_range(d, start_of_year_naive(t), t)))
}


fn to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.ToRecord")?;
    Ok(Value::Record(Record {
        fields: vec![
            ("Year".into(),  Value::Number(d.year() as f64)),
            ("Month".into(), Value::Number(d.month() as f64)),
            ("Day".into(),   Value::Number(d.day() as f64)),
        ],
        env: super::super::env::EnvNode::empty(),
    }))
}


fn add_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn shift_months_signed(d: chrono::NaiveDate, n: i64) -> Option<chrono::NaiveDate> {
    if n >= 0 {
        d.checked_add_months(chrono::Months::new(n as u32))
    } else {
        d.checked_sub_months(chrono::Months::new((-n) as u32))
    }
}


fn add_years(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 => *n as i64,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("integer (numberOfYears)", other)),
    };
    let months = n * 12;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => shift_months_signed(*d, months)
            .map(Value::Date)
            .ok_or_else(|| MError::Other("Date.AddYears: result out of range".into())),
        Value::Datetime(dt) => {
            let nd = shift_months_signed(dt.date(), months)
                .ok_or_else(|| MError::Other("Date.AddYears: result out of range".into()))?;
            Ok(Value::Datetime(nd.and_time(dt.time())))
        }
        other => Err(type_mismatch("date or datetime", other)),
    }
}


fn add_quarters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 => *n as i64,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("integer (numberOfQuarters)", other)),
    };
    let months = n * 3;
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => shift_months_signed(*d, months)
            .map(Value::Date)
            .ok_or_else(|| MError::Other("Date.AddQuarters: result out of range".into())),
        Value::Datetime(dt) => {
            let nd = shift_months_signed(dt.date(), months)
                .ok_or_else(|| MError::Other("Date.AddQuarters: result out of range".into()))?;
            Ok(Value::Datetime(nd.and_time(dt.time())))
        }
        other => Err(type_mismatch("date or datetime", other)),
    }
}


fn add_weeks(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let n = match &args[1] {
        Value::Number(n) if n.fract() == 0.0 => *n as i64,
        Value::Null => return Ok(Value::Null),
        other => return Err(type_mismatch("integer (numberOfWeeks)", other)),
    };
    let delta = chrono::Duration::weeks(n);
    match &args[0] {
        Value::Null => Ok(Value::Null),
        Value::Date(d) => d
            .checked_add_signed(delta)
            .map(Value::Date)
            .ok_or_else(|| MError::Other("Date.AddWeeks: result out of range".into())),
        Value::Datetime(dt) => dt
            .checked_add_signed(delta)
            .map(Value::Datetime)
            .ok_or_else(|| MError::Other("Date.AddWeeks: result out of range".into())),
        other => Err(type_mismatch("date or datetime", other)),
    }
}


fn add_months(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn from_text(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let text = expect_text(&args[0])?;
    // Extract optional Format (.NET pattern like "dd/MM/yyyy") and Culture
    // (e.g. "en-GB", "en-US") from the second arg. PQ accepts either a Format
    // record `[Format=..., Culture=...]` or a culture string directly.
    let (format, culture) = parse_date_options(args.get(1))?;
    if let Some(fmt) = format.as_deref() {
        let strftime = dotnet_to_strftime(fmt);
        return chrono::NaiveDate::parse_from_str(text, &strftime)
            .map(Value::Date)
            .map_err(|_| MError::Raised(super::super::build_data_format_error(
                "We couldn't parse the input provided as a Date value.".into()
            )));
    }
    // No explicit format: try ISO first, then culture-appropriate locale
    // order. Also try natural-language month names (en + de) so PQ probes
    // like "June 15, 2024" and "15 Juni 2024" parse without a Format arg.
    let formats: &[&str] = match culture.as_deref().map(culture_lang).unwrap_or("") {
        "en-US" => &["%Y-%m-%d", "%m/%d/%Y", "%m-%d-%Y", "%Y/%m/%d",
                     "%B %d, %Y", "%b %d, %Y", "%d %B %Y", "%d %b %Y"],
        "de-DE" | "de" => &["%Y-%m-%d", "%d.%m.%Y", "%d/%m/%Y",
                            "%d %B %Y", "%d %b %Y"],
        "fr-FR" | "fr" => &["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y",
                            "%d %B %Y", "%d %b %Y"],
        _ => &["%Y-%m-%d", "%d/%m/%Y", "%d-%m-%Y", "%m/%d/%Y", "%m-%d-%Y",
               "%Y/%m/%d", "%B %d, %Y", "%d %B %Y"],
    };
    for fmt in formats {
        // chrono's locale parsing uses English by default. For de/fr month
        // names, translate to English before parsing.
        let translated = translate_month_names(text, culture.as_deref());
        if let Ok(d) = chrono::NaiveDate::parse_from_str(&translated, fmt) {
            return Ok(Value::Date(d));
        }
    }
    Err(MError::Raised(super::super::build_data_format_error(
        "We couldn't parse the input provided as a Date value.".into()
    )))
}

fn parse_date_options(arg: Option<&Value>) -> Result<(Option<String>, Option<String>), MError> {
    match arg {
        None | Some(Value::Null) => Ok((None, None)),
        Some(Value::Text(s)) => Ok((None, Some(s.clone()))),
        Some(Value::Record(r)) => {
            let mut format = None;
            let mut culture = None;
            for (k, v) in &r.fields {
                match (k.as_str(), v) {
                    ("Format",  Value::Text(s)) => format  = Some(s.clone()),
                    ("Culture", Value::Text(s)) => culture = Some(s.clone()),
                    ("Format",  Value::Null) | ("Culture", Value::Null) => {}
                    _ => {}
                }
            }
            Ok((format, culture))
        }
        Some(other) => Err(type_mismatch("text (culture) or record (options)", other)),
    }
}

fn translate_month_names(text: &str, culture: Option<&str>) -> String {
    let is_de = matches!(culture, Some(c) if c.to_ascii_lowercase().starts_with("de"));
    let is_fr = matches!(culture, Some(c) if c.to_ascii_lowercase().starts_with("fr"));
    if !is_de && !is_fr { return text.to_string(); }
    let pairs: &[(&str, &str)] = if is_de {
        &[("Januar","January"),("Februar","February"),("März","March"),("April","April"),
          ("Mai","May"),("Juni","June"),("Juli","July"),("August","August"),
          ("September","September"),("Oktober","October"),("November","November"),
          ("Dezember","December"),("Jan","Jan"),("Feb","Feb"),("Mrz","Mar"),("Apr","Apr"),
          ("Jun","Jun"),("Jul","Jul"),("Aug","Aug"),("Sep","Sep"),("Okt","Oct"),
          ("Nov","Nov"),("Dez","Dec")]
    } else {
        &[("janvier","January"),("février","February"),("mars","March"),("avril","April"),
          ("mai","May"),("juin","June"),("juillet","July"),("août","August"),
          ("septembre","September"),("octobre","October"),("novembre","November"),
          ("décembre","December")]
    };
    let mut out = text.to_string();
    for (src, dst) in pairs {
        out = out.replace(src, dst);
    }
    out
}

fn culture_lang(s: &str) -> &str {
    // Normalise to a small set of locale tags we handle directly.
    if s.eq_ignore_ascii_case("en-US") { "en-US" }
    else if s.eq_ignore_ascii_case("de-DE") { "de-DE" }
    else if s.eq_ignore_ascii_case("fr-FR") { "fr-FR" }
    else { s }
}

/// Expand .NET single-letter standard date/datetime format codes to their
/// full custom pattern equivalents. Returns the input unchanged if it's
/// already a custom pattern (length > 1 or unrecognised single letter).
pub(super) fn expand_standard_date_format(fmt: &str, culture: Option<&str>) -> String {
    if fmt.chars().count() != 1 {
        return fmt.to_string();
    }
    let c = fmt.chars().next().unwrap();
    // en-GB is the default Oracle culture. en-US has different short forms.
    let is_en_us = matches!(culture, Some(c) if c.eq_ignore_ascii_case("en-US"));
    let s = match c {
        // Short date
        'd' => if is_en_us { "M/d/yyyy" } else { "dd/MM/yyyy" },
        // Long date
        'D' => "dddd, MMMM d, yyyy",
        // Full date/time (short time)
        'f' => if is_en_us { "dddd, MMMM d, yyyy h:mm tt" } else { "dd MMMM yyyy HH:mm" },
        // Full date/time (long time)
        'F' => if is_en_us { "dddd, MMMM d, yyyy h:mm:ss tt" } else { "dd MMMM yyyy HH:mm:ss" },
        // General date/time (short time)
        'g' => if is_en_us { "M/d/yyyy h:mm tt" } else { "dd/MM/yyyy HH:mm" },
        // General date/time (long time)
        'G' => if is_en_us { "M/d/yyyy h:mm:ss tt" } else { "dd/MM/yyyy HH:mm:ss" },
        // Month/day
        'M' | 'm' => "MMMM d",
        // Round-trip ISO 8601
        'O' | 'o' => "yyyy-MM-ddTHH:mm:ss.fffffff",
        // RFC 1123
        'R' | 'r' => "ddd, dd MMM yyyy HH:mm:ss",
        // Sortable
        's' => "yyyy-MM-ddTHH:mm:ss",
        // Short time
        't' => if is_en_us { "h:mm tt" } else { "HH:mm" },
        // Long time
        'T' => if is_en_us { "h:mm:ss tt" } else { "HH:mm:ss" },
        // Universal sortable
        'u' => "yyyy-MM-dd HH:mm:ssZ",
        // Universal full
        'U' => "dddd, MMMM d, yyyy HH:mm:ss",
        // Year/month
        'Y' | 'y' => "MMMM yyyy",
        _ => fmt,
    };
    s.to_string()
}

/// Translate a .NET-style date format string (yyyy/MM/dd/HH/mm/ss) into a
/// chrono strftime pattern. Only the pieces Date.FromText / DateTime.ToText
/// care about — extend when needed.
pub(super) fn dotnet_to_strftime(fmt: &str) -> String {
    let mut out = String::with_capacity(fmt.len());
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        // Count run length of this character.
        let mut j = i;
        while j < chars.len() && chars[j] == c { j += 1; }
        let run = j - i;
        match (c, run) {
            ('y', 4) => out.push_str("%Y"),
            ('y', 2) => out.push_str("%y"),
            ('y', _) => out.push_str("%Y"),
            ('M', 4) => out.push_str("%B"),
            ('M', 3) => out.push_str("%b"),
            ('M', 2) => out.push_str("%m"),
            ('M', 1) => out.push_str("%-m"),
            ('d', 4) => out.push_str("%A"),
            ('d', 3) => out.push_str("%a"),
            ('d', 2) => out.push_str("%d"),
            ('d', 1) => out.push_str("%-d"),
            ('H', 2) => out.push_str("%H"),
            ('H', 1) => out.push_str("%-H"),
            ('h', 2) => out.push_str("%I"),
            ('h', 1) => out.push_str("%-I"),
            ('m', 2) => out.push_str("%M"),
            ('m', 1) => out.push_str("%-M"),
            ('s', 2) => out.push_str("%S"),
            ('s', 1) => out.push_str("%-S"),
            ('f', n) => { let _ = n; out.push_str("%f"); }
            ('t', 2) => out.push_str("%p"),
            ('\'', _) => {
                // Literal block: copy through to matching quote, skipping the quotes.
                i = j;
                while i < chars.len() && chars[i] != '\'' {
                    out.push(chars[i]);
                    i += 1;
                }
                if i < chars.len() { i += 1; }
                continue;
            }
            (other, _) => for _ in 0..run { out.push(other); }
        }
        i = j;
    }
    out
}

// --- ODBC (eval-8) ---
//
// Delegates to the shell's IoHost. CliIoHost (built with `--features odbc`)
// uses odbc-api against an installed driver; NoIoHost and CliIoHost built
// without the feature return a NotSupported-style error. WASM shell will
// likewise return NotSupported when it lands.

