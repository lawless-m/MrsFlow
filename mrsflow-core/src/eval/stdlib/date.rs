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
            date_constructor,
        ),
        ("Date.FromText", one("text"), date_from_text),
        ("Date.AddDays", two("date", "numberOfDays"), date_add_days),
        ("Date.AddMonths", two("date", "numberOfMonths"), date_add_months),
        ("Date.AddYears", two("date", "numberOfYears"), date_add_years),
        ("Date.AddQuarters", two("date", "numberOfQuarters"), date_add_quarters),
        ("Date.AddWeeks", two("date", "numberOfWeeks"), date_add_weeks),
        (
            "Date.DayOfWeek",
            vec![
                Param { name: "date".into(),            optional: false, type_annotation: None },
                Param { name: "firstDayOfWeek".into(),  optional: true,  type_annotation: None },
            ],
            date_day_of_week,
        ),
        (
            "Date.DayOfWeekName",
            vec![
                Param { name: "date".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            date_day_of_week_name,
        ),
        ("Date.DayOfYear", one("date"), date_day_of_year),
        ("Date.DaysInMonth", one("date"), date_days_in_month),
        (
            "Date.MonthName",
            vec![
                Param { name: "date".into(),    optional: false, type_annotation: None },
                Param { name: "culture".into(), optional: true,  type_annotation: None },
            ],
            date_month_name,
        ),
        ("Date.QuarterOfYear", one("date"), date_quarter_of_year),
        ("Date.WeekOfMonth", one("date"), date_week_of_month),
        (
            "Date.WeekOfYear",
            vec![
                Param { name: "date".into(),           optional: false, type_annotation: None },
                Param { name: "firstDayOfWeek".into(), optional: true,  type_annotation: None },
            ],
            date_week_of_year,
        ),
        ("Date.IsLeapYear", one("date"), date_is_leap_year),
        ("Date.ToRecord", one("date"), date_to_record),
        ("Date.StartOfDay", one("date"), date_start_of_day),
        ("Date.EndOfDay", one("date"), date_end_of_day),
        (
            "Date.StartOfWeek",
            vec![
                Param { name: "date".into(),           optional: false, type_annotation: None },
                Param { name: "firstDayOfWeek".into(), optional: true,  type_annotation: None },
            ],
            date_start_of_week,
        ),
        (
            "Date.EndOfWeek",
            vec![
                Param { name: "date".into(),           optional: false, type_annotation: None },
                Param { name: "firstDayOfWeek".into(), optional: true,  type_annotation: None },
            ],
            date_end_of_week,
        ),
        ("Date.StartOfMonth", one("date"), date_start_of_month),
        ("Date.EndOfMonth", one("date"), date_end_of_month),
        ("Date.StartOfQuarter", one("date"), date_start_of_quarter),
        ("Date.EndOfQuarter", one("date"), date_end_of_quarter),
        ("Date.StartOfYear", one("date"), date_start_of_year),
        ("Date.EndOfYear", one("date"), date_end_of_year),
        ("Date.IsInCurrentDay", one("date"), date_is_in_current_day),
        ("Date.IsInCurrentWeek", one("date"), date_is_in_current_week),
        ("Date.IsInCurrentMonth", one("date"), date_is_in_current_month),
        ("Date.IsInCurrentQuarter", one("date"), date_is_in_current_quarter),
        ("Date.IsInCurrentYear", one("date"), date_is_in_current_year),
        ("Date.IsInNextDay", one("date"), date_is_in_next_day),
        ("Date.IsInNextWeek", one("date"), date_is_in_next_week),
        ("Date.IsInNextMonth", one("date"), date_is_in_next_month),
        ("Date.IsInNextQuarter", one("date"), date_is_in_next_quarter),
        ("Date.IsInNextYear", one("date"), date_is_in_next_year),
        ("Date.IsInPreviousDay", one("date"), date_is_in_previous_day),
        ("Date.IsInPreviousWeek", one("date"), date_is_in_previous_week),
        ("Date.IsInPreviousMonth", one("date"), date_is_in_previous_month),
        ("Date.IsInPreviousQuarter", one("date"), date_is_in_previous_quarter),
        ("Date.IsInPreviousYear", one("date"), date_is_in_previous_year),
        ("Date.IsInNextNDays", two("date", "numberOfDays"), date_is_in_next_n_days),
        ("Date.IsInNextNWeeks", two("date", "numberOfWeeks"), date_is_in_next_n_weeks),
        ("Date.IsInNextNMonths", two("date", "numberOfMonths"), date_is_in_next_n_months),
        ("Date.IsInNextNQuarters", two("date", "numberOfQuarters"), date_is_in_next_n_quarters),
        ("Date.IsInNextNYears", two("date", "numberOfYears"), date_is_in_next_n_years),
        ("Date.IsInPreviousNDays", two("date", "numberOfDays"), date_is_in_previous_n_days),
        ("Date.IsInPreviousNWeeks", two("date", "numberOfWeeks"), date_is_in_previous_n_weeks),
        ("Date.IsInPreviousNMonths", two("date", "numberOfMonths"), date_is_in_previous_n_months),
        ("Date.IsInPreviousNQuarters", two("date", "numberOfQuarters"), date_is_in_previous_n_quarters),
        ("Date.IsInPreviousNYears", two("date", "numberOfYears"), date_is_in_previous_n_years),
        ("Date.IsInYearToDate", one("date"), date_is_in_year_to_date),
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
    ]
}

fn date_constructor(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let y = expect_int(&args[0], "#date: year")?;
    let mo = expect_int(&args[1], "#date: month")?;
    let d = expect_int(&args[2], "#date: day")?;
    chrono::NaiveDate::from_ymd_opt(y as i32, mo as u32, d as u32)
        .map(Value::Date)
        .ok_or_else(|| MError::Other(format!("#date: invalid date {}-{:02}-{:02}", y, mo, d)))
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


fn date_day_of_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.DayOfWeek")?;
    let first = match args.get(1) {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 && *n <= 6.0 => *n as u32,
        Some(Value::Null) | None => 0, // Sunday
        Some(other) => return Err(type_mismatch("integer 0..6 (Day.*)", other)),
    };
    // chrono's Weekday::num_days_from_monday returns 0..6 starting Monday=0.
    let from_monday = d.weekday().num_days_from_monday();
    // Sunday(0), Monday(1), ..., Saturday(6) → want days since `first`.
    let dow_sunday_first = (from_monday + 1) % 7; // 0=Sunday, 1=Monday, ...
    let result = (dow_sunday_first + 7 - first) % 7;
    Ok(Value::Number(result as f64))
}


fn date_day_of_week_name(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_day_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.DayOfYear")?;
    Ok(Value::Number(d.ordinal() as f64))
}


fn date_days_in_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_month_name(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_quarter_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.QuarterOfYear")?;
    Ok(Value::Number(((d.month() - 1) / 3 + 1) as f64))
}


fn date_week_of_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.WeekOfMonth")?;
    Ok(Value::Number(((d.day() - 1) / 7 + 1) as f64))
}


fn date_week_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    if matches!(args[0], Value::Null) {
        return Ok(Value::Null);
    }
    let d = extract_naive_date(&args[0], "Date.WeekOfYear")?;
    // PQ's WeekOfYear with default first-day-of-week is approximately ISO week.
    // For v1, return ISO week number (1..53).
    Ok(Value::Number(d.iso_week().week() as f64))
}


fn date_is_leap_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_start_of_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    shape_date(&args[0], "Date.StartOfDay", true, |d| d)
}


fn date_end_of_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    shape_date(&args[0], "Date.EndOfDay", false, |d| d)
}


fn date_start_of_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.StartOfMonth", true, |d| {
        chrono::NaiveDate::from_ymd_opt(d.year(), d.month(), 1).unwrap()
    })
}


fn date_end_of_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_start_of_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.StartOfQuarter", true, |d| {
        let q_start_month = ((d.month() - 1) / 3) * 3 + 1;
        chrono::NaiveDate::from_ymd_opt(d.year(), q_start_month, 1).unwrap()
    })
}


fn date_end_of_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_start_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.StartOfYear", true, |d| {
        chrono::NaiveDate::from_ymd_opt(d.year(), 1, 1).unwrap()
    })
}


fn date_end_of_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    use chrono::Datelike;
    shape_date(&args[0], "Date.EndOfYear", false, |d| {
        chrono::NaiveDate::from_ymd_opt(d.year(), 12, 31).unwrap()
    })
}


fn first_day_of_week_arg(arg: Option<&Value>, ctx: &str) -> Result<u32, MError> {
    match arg {
        Some(Value::Number(n)) if n.fract() == 0.0 && *n >= 0.0 && *n <= 6.0 => Ok(*n as u32),
        Some(Value::Null) | None => Ok(0), // default Sunday
        Some(other) => Err(MError::Other(format!(
            "{}: firstDayOfWeek must be 0..6 (got {})", ctx, super::super::type_name(other)
        ))),
    }
}


fn date_start_of_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_end_of_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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
    // Default Sunday-start, matching Date.StartOfWeek default.
    let from_monday = d.weekday().num_days_from_monday();
    let dow_sunday_first = (from_monday + 1) % 7;
    d - chrono::Duration::days(dow_sunday_first as i64)
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


fn date_in_range(target: chrono::NaiveDate, start: chrono::NaiveDate, end_inclusive: chrono::NaiveDate) -> bool {
    target >= start && target <= end_inclusive
}

// ----- IsInCurrent* (single-unit, based on today) -----


fn date_is_in_current_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentDay")? { Some(d) => d, None => return Ok(Value::Null) };
    Ok(Value::Logical(d == today()))
}

fn date_is_in_current_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentWeek")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_week_naive(today());
    Ok(Value::Logical(date_in_range(d, s, s + chrono::Duration::days(6))))
}

fn date_is_in_current_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentMonth")? { Some(d) => d, None => return Ok(Value::Null) };
    let t = today();
    Ok(Value::Logical(date_in_range(d, start_of_month_naive(t), end_of_month_naive(t))))
}

fn date_is_in_current_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentQuarter")? { Some(d) => d, None => return Ok(Value::Null) };
    let t = today();
    Ok(Value::Logical(date_in_range(d, start_of_quarter_naive(t), end_of_quarter_naive(t))))
}

fn date_is_in_current_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInCurrentYear")? { Some(d) => d, None => return Ok(Value::Null) };
    let t = today();
    Ok(Value::Logical(date_in_range(d, start_of_year_naive(t), end_of_year_naive(t))))
}

// ----- IsInNext* (single-unit) -----


fn date_is_in_next_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextDay")? { Some(d) => d, None => return Ok(Value::Null) };
    Ok(Value::Logical(d == today() + chrono::Duration::days(1)))
}

fn date_is_in_next_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextWeek")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_week_naive(today()) + chrono::Duration::days(7);
    Ok(Value::Logical(date_in_range(d, s, s + chrono::Duration::days(6))))
}

fn date_is_in_next_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextMonth")? { Some(d) => d, None => return Ok(Value::Null) };
    let next = shift_months_signed(start_of_month_naive(today()), 1).unwrap();
    Ok(Value::Logical(date_in_range(d, next, end_of_month_naive(next))))
}

fn date_is_in_next_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextQuarter")? { Some(d) => d, None => return Ok(Value::Null) };
    let next = shift_months_signed(start_of_quarter_naive(today()), 3).unwrap();
    Ok(Value::Logical(date_in_range(d, next, end_of_quarter_naive(next))))
}

fn date_is_in_next_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextYear")? { Some(d) => d, None => return Ok(Value::Null) };
    let next = shift_months_signed(start_of_year_naive(today()), 12).unwrap();
    Ok(Value::Logical(date_in_range(d, next, end_of_year_naive(next))))
}

// ----- IsInPrevious* (single-unit) -----


fn date_is_in_previous_day(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousDay")? { Some(d) => d, None => return Ok(Value::Null) };
    Ok(Value::Logical(d == today() - chrono::Duration::days(1)))
}

fn date_is_in_previous_week(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousWeek")? { Some(d) => d, None => return Ok(Value::Null) };
    let s = start_of_week_naive(today()) - chrono::Duration::days(7);
    Ok(Value::Logical(date_in_range(d, s, s + chrono::Duration::days(6))))
}

fn date_is_in_previous_month(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousMonth")? { Some(d) => d, None => return Ok(Value::Null) };
    let prev = shift_months_signed(start_of_month_naive(today()), -1).unwrap();
    Ok(Value::Logical(date_in_range(d, prev, end_of_month_naive(prev))))
}

fn date_is_in_previous_quarter(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousQuarter")? { Some(d) => d, None => return Ok(Value::Null) };
    let prev = shift_months_signed(start_of_quarter_naive(today()), -3).unwrap();
    Ok(Value::Logical(date_in_range(d, prev, end_of_quarter_naive(prev))))
}

fn date_is_in_previous_year(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousYear")? { Some(d) => d, None => return Ok(Value::Null) };
    let prev = shift_months_signed(start_of_year_naive(today()), -12).unwrap();
    Ok(Value::Logical(date_in_range(d, prev, end_of_year_naive(prev))))
}

// ----- IsInNextN* / IsInPreviousN* -----


fn date_is_in_next_n_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNDays")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNDays")?;
    let start = today() + chrono::Duration::days(1);
    let end = today() + chrono::Duration::days(n);
    Ok(Value::Logical(date_in_range(d, start, end)))
}

fn date_is_in_previous_n_days(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNDays")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNDays")?;
    let start = today() - chrono::Duration::days(n);
    let end = today() - chrono::Duration::days(1);
    Ok(Value::Logical(date_in_range(d, start, end)))
}

fn date_is_in_next_n_weeks(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNWeeks")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNWeeks")?;
    let start = start_of_week_naive(today()) + chrono::Duration::weeks(1);
    let end = start + chrono::Duration::weeks(n) - chrono::Duration::days(1);
    Ok(Value::Logical(date_in_range(d, start, end)))
}

fn date_is_in_previous_n_weeks(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNWeeks")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNWeeks")?;
    let end = start_of_week_naive(today()) - chrono::Duration::days(1);
    let start = end - chrono::Duration::weeks(n) + chrono::Duration::days(1);
    Ok(Value::Logical(date_in_range(d, start, end)))
}

fn date_is_in_next_n_months(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNMonths")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNMonths")?;
    let start = shift_months_signed(start_of_month_naive(today()), 1).unwrap();
    let end_month_start = shift_months_signed(start, n - 1).unwrap();
    Ok(Value::Logical(date_in_range(d, start, end_of_month_naive(end_month_start))))
}

fn date_is_in_previous_n_months(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNMonths")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNMonths")?;
    let start = shift_months_signed(start_of_month_naive(today()), -n).unwrap();
    let end = start_of_month_naive(today()) - chrono::Duration::days(1);
    Ok(Value::Logical(date_in_range(d, start, end)))
}

fn date_is_in_next_n_quarters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNQuarters")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNQuarters")?;
    let start = shift_months_signed(start_of_quarter_naive(today()), 3).unwrap();
    let end_q = shift_months_signed(start, (n - 1) * 3).unwrap();
    Ok(Value::Logical(date_in_range(d, start, end_of_quarter_naive(end_q))))
}

fn date_is_in_previous_n_quarters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNQuarters")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNQuarters")?;
    let start = shift_months_signed(start_of_quarter_naive(today()), -n * 3).unwrap();
    let end = start_of_quarter_naive(today()) - chrono::Duration::days(1);
    Ok(Value::Logical(date_in_range(d, start, end)))
}

fn date_is_in_next_n_years(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInNextNYears")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInNextNYears")?;
    let start = shift_months_signed(start_of_year_naive(today()), 12).unwrap();
    let end_y = shift_months_signed(start, (n - 1) * 12).unwrap();
    Ok(Value::Logical(date_in_range(d, start, end_of_year_naive(end_y))))
}

fn date_is_in_previous_n_years(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInPreviousNYears")? { Some(d) => d, None => return Ok(Value::Null) };
    let n = int_n_arg(&args[1], "Date.IsInPreviousNYears")?;
    let start = shift_months_signed(start_of_year_naive(today()), -n * 12).unwrap();
    let end = start_of_year_naive(today()) - chrono::Duration::days(1);
    Ok(Value::Logical(date_in_range(d, start, end)))
}


fn date_is_in_year_to_date(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
    let d = match extract_date_opt(&args[0], "Date.IsInYearToDate")? { Some(d) => d, None => return Ok(Value::Null) };
    let t = today();
    Ok(Value::Logical(date_in_range(d, start_of_year_naive(t), t)))
}


fn date_to_record(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn shift_months_signed(d: chrono::NaiveDate, n: i64) -> Option<chrono::NaiveDate> {
    if n >= 0 {
        d.checked_add_months(chrono::Months::new(n as u32))
    } else {
        d.checked_sub_months(chrono::Months::new((-n) as u32))
    }
}


fn date_add_years(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_add_quarters(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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


fn date_add_weeks(args: &[Value], _host: &dyn IoHost) -> Result<Value, MError> {
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

