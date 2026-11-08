//! Calendar dates and day-count conventions (ADR 0008, SPEC-0002 R1, R2).
//!
//! `Date` is a proleptic Gregorian calendar date without time or time zone, valid from
//! 1900-01-01 to 2199-12-31 and written as ISO 8601 `YYYY-MM-DD`. Day counts follow the
//! 2006 ISDA Definitions, section 4.16.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num;

/// First and last supported years.
pub const MIN_YEAR: i32 = 1900;
pub const MAX_YEAR: i32 = 2199;

/// Marker that lets the registry map a date parsing failure to `FM-E009`.
pub(crate) const INVALID_DATE: &str = "invalid date";

/// A calendar date (ISO 8601 `YYYY-MM-DD`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Date {
    year: i32,
    month: u32,
    day: u32,
}

pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

pub fn days_in_year(year: i32) -> i64 {
    if is_leap_year(year) {
        366
    } else {
        365
    }
}

pub fn days_in_month(year: i32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

impl Date {
    /// A validated date.
    pub fn new(year: i32, month: u32, day: u32) -> Result<Self> {
        if !(MIN_YEAR..=MAX_YEAR).contains(&year) {
            return Err(FairmarkError::InvalidDate(format!(
                "{year:04}-{month:02}-{day:02}: year outside {MIN_YEAR}..={MAX_YEAR}"
            )));
        }
        if !(1..=12).contains(&month) || day == 0 || day > days_in_month(year, month) {
            return Err(FairmarkError::InvalidDate(format!(
                "{year:04}-{month:02}-{day:02} does not exist"
            )));
        }
        Ok(Self { year, month, day })
    }

    pub fn year(self) -> i32 {
        self.year
    }
    pub fn month(self) -> u32 {
        self.month
    }
    pub fn day(self) -> u32 {
        self.day
    }

    /// Days since 1970-01-01 (negative before). Civil-from-days algorithm by H. Hinnant.
    pub fn day_number(self) -> i64 {
        let y = i64::from(self.year) - i64::from(self.month <= 2);
        let era = y.div_euclid(400);
        let yoe = y - era * 400;
        let m = i64::from(self.month);
        let mp = (m + 9) % 12;
        let doy = (153 * mp + 2) / 5 + i64::from(self.day) - 1;
        let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
        era * 146_097 + doe - 719_468
    }

    /// Actual days from `self` to `other` (negative if `other` is earlier).
    pub fn days_until(self, other: Date) -> i64 {
        other.day_number() - self.day_number()
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

impl FromStr for Date {
    type Err = FairmarkError;

    /// Strict ISO 8601 calendar date: exactly `YYYY-MM-DD`.
    fn from_str(s: &str) -> Result<Self> {
        let b = s.as_bytes();
        let shape_ok = b.len() == 10
            && b[4] == b'-'
            && b[7] == b'-'
            && b.iter()
                .enumerate()
                .all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
        if !shape_ok {
            return Err(FairmarkError::InvalidDate(format!(
                "'{s}' is not an ISO 8601 date (YYYY-MM-DD)"
            )));
        }
        let num = |r: std::ops::Range<usize>| s[r].parse::<u32>().unwrap_or(0);
        Date::new(num(0..4) as i32, num(5..7), num(8..10))
    }
}

impl Serialize for Date {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Date {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse::<Date>().map_err(|e| {
            let msg = e.to_string();
            let detail = msg.strip_prefix("invalid date: ").unwrap_or(&msg);
            serde::de::Error::custom(format!("{INVALID_DATE}: {detail}"))
        })
    }
}

impl JsonSchema for Date {
    fn schema_name() -> Cow<'static, str> {
        "Date".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        json_schema!({
            "type": "string",
            "format": "date",
            "pattern": "^[0-9]{4}-[0-9]{2}-[0-9]{2}$",
            "description": "Calendar date, ISO 8601 YYYY-MM-DD, from 1900-01-01 to 2199-12-31."
        })
    }
}

/// Day-count conventions of the 2006 ISDA Definitions, section 4.16.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DayCount {
    /// Actual/365 (Fixed), §4.16(d): actual days / 365.
    #[serde(rename = "act_365_fixed")]
    Act365Fixed,
    /// Actual/360, §4.16(e): actual days / 360.
    #[serde(rename = "act_360")]
    Act360,
    /// Actual/Actual (ISDA), §4.16(b): days in leap years / 366 + other days / 365.
    ActActIsda,
    /// 30/360 (Bond Basis), §4.16(f): day 31 becomes 30; the end day 31 becomes 30 only if the
    /// start day is 30 or 31.
    #[serde(rename = "thirty_360")]
    Thirty360,
    /// 30E/360 (Eurobond Basis), §4.16(g): every day 31 becomes 30.
    #[serde(rename = "thirty_e_360")]
    ThirtyE360,
}

impl DayCount {
    pub fn label(self) -> &'static str {
        match self {
            Self::Act365Fixed => "Actual/365 (Fixed), ISDA 2006 §4.16(d)",
            Self::Act360 => "Actual/360, ISDA 2006 §4.16(e)",
            Self::ActActIsda => "Actual/Actual (ISDA), ISDA 2006 §4.16(b)",
            Self::Thirty360 => "30/360 (Bond Basis), ISDA 2006 §4.16(f)",
            Self::ThirtyE360 => "30E/360 (Eurobond Basis), ISDA 2006 §4.16(g)",
        }
    }

    /// Year fraction from `start` to `end` (`end` must not precede `start`), with the
    /// calculation written out for the audit trail.
    pub fn year_fraction(self, start: Date, end: Date) -> Result<(Decimal, String)> {
        if end < start {
            return Err(FairmarkError::InvalidInput(format!(
                "`end` ({end}) precedes `start` ({start}); swap them"
            )));
        }
        let days = start.days_until(end);
        let div = |n: i64, d: i64| num::div(Decimal::from(n), Decimal::from(d));
        Ok(match self {
            Self::Act365Fixed => (div(days, 365)?, format!("{days} / 365")),
            Self::Act360 => (div(days, 360)?, format!("{days} / 360")),
            Self::ActActIsda => {
                if start.year == end.year {
                    let den = days_in_year(start.year);
                    (div(days, den)?, format!("{days} / {den}"))
                } else {
                    let next = Date::new(start.year + 1, 1, 1)?;
                    let first = start.days_until(next);
                    let jan1 = Date::new(end.year, 1, 1)?;
                    let last = jan1.days_until(end);
                    let whole = i64::from(end.year - start.year - 1);
                    let (d1, d2) = (days_in_year(start.year), days_in_year(end.year));
                    (
                        num::add(
                            num::add(div(first, d1)?, Decimal::from(whole))?,
                            div(last, d2)?,
                        )?,
                        format!("{first} / {d1} + {whole} + {last} / {d2}"),
                    )
                }
            }
            Self::Thirty360 | Self::ThirtyE360 => {
                let mut d1 = start.day;
                let mut d2 = end.day;
                // Day adjustments, written next to the formula for the audit trail.
                let mut notes = Vec::new();
                if d1 == 31 {
                    d1 = 30;
                    notes.push("start day 31 adjusted to 30".to_owned());
                }
                if d2 == 31 {
                    if self == Self::ThirtyE360 {
                        d2 = 30;
                        notes.push("end day 31 adjusted to 30".to_owned());
                    } else if d1 > 29 {
                        d2 = 30;
                        notes.push(
                            "end day 31 adjusted to 30 (the start day is 30 or 31)".to_owned(),
                        );
                    } else {
                        notes.push(format!("end day 31 kept (the start day {d1} is below 30)"));
                    }
                }
                let numerator = 360 * i64::from(end.year - start.year)
                    + 30 * (i64::from(end.month) - i64::from(start.month))
                    + (i64::from(d2) - i64::from(d1));
                let adjusted = if notes.is_empty() {
                    String::new()
                } else {
                    format!("; {}", notes.join(", "))
                };
                (
                    div(numerator, 360)?,
                    format!(
                        "[360 × ({} − {}) + 30 × ({} − {}) + ({d2} − {d1})] / 360{adjusted}",
                        end.year, start.year, end.month, start.month
                    ),
                )
            }
        })
    }
}

// ---------------------------------------------------------------- dates.year_fraction

/// Input of `dates.year_fraction`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct YearFractionInput {
    /// Start date (ISO 8601).
    pub start: Date,
    /// End date (ISO 8601), not earlier than the start date.
    pub end: Date,
    /// Day-count convention.
    pub day_count: DayCount,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct YearFractionOutput {
    /// Fraction of a year between the two dates under the chosen day count.
    pub year_fraction: Decimal,
    /// Actual calendar days between the two dates.
    pub actual_days: i64,
}

pub fn year_fraction(input: &YearFractionInput) -> Result<(YearFractionOutput, Explain)> {
    let (yf, formula) = input.day_count.year_fraction(input.start, input.end)?;
    let mut ex = Explain::new(
        "dates.year_fraction",
        "year fraction = day count(start, end)",
    );
    ex.convention(format!("day count: {}", input.day_count.label()));
    let days = input.start.days_until(input.end);
    ex.step(
        "Actual days",
        format!("{} to {}", input.start, input.end),
        Decimal::from(days),
    );
    ex.step("Year fraction", formula, yf);
    Ok((
        YearFractionOutput {
            year_fraction: yf,
            actual_days: days,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- method registrations

/// `dates.year_fraction`: see the methodology page.
pub struct YearFraction;

impl Method for YearFraction {
    const ID: &'static str = "dates.year_fraction";
    const TITLE: &'static str = "Year fraction (day count)";
    const DESCRIPTION: &'static str =
        "Fraction of a year between two dates under an ISDA 2006 day-count convention: Actual/365 \
        Fixed, Actual/360, Actual/Actual ISDA, 30/360, 30E/360.";
    const VERSION: &'static str = "1";
    type Input = YearFractionInput;
    type Output = YearFractionOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        year_fraction(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> Date {
        s.parse().unwrap()
    }

    #[test]
    fn day_numbers_match_known_epochs() {
        assert_eq!(d("1970-01-01").day_number(), 0);
        assert_eq!(d("2000-03-01").day_number(), 11_017);
        assert_eq!(d("1900-01-01").day_number(), -25_567);
    }

    #[test]
    fn day_number_is_consecutive_across_the_whole_range() {
        let mut prev = d("1900-01-01").day_number();
        for y in MIN_YEAR..=MAX_YEAR {
            for m in 1..=12 {
                for day in 1..=days_in_month(y, m) {
                    let n = Date::new(y, m, day).unwrap().day_number();
                    if (y, m, day) != (MIN_YEAR, 1, 1) {
                        assert_eq!(n, prev + 1, "{y}-{m}-{day}");
                    }
                    prev = n;
                }
            }
        }
    }

    #[test]
    fn rejects_malformed_and_impossible_dates() {
        for bad in [
            "2023-02-29",
            "2024-13-01",
            "2024-00-10",
            "2024-1-01",
            "24-01-01",
            "2024/01/01",
            "1899-12-31",
            "2200-01-01",
            "2024-04-31",
            "",
        ] {
            assert!(
                matches!(bad.parse::<Date>(), Err(FairmarkError::InvalidDate(_))),
                "{bad}"
            );
        }
        assert!(d("2024-02-29") < d("2024-03-01"));
    }

    #[test]
    fn round_trips_through_text() {
        assert_eq!(d("2008-10-30").to_string(), "2008-10-30");
    }

    #[test]
    fn century_leap_rules() {
        assert!(is_leap_year(2000));
        assert!(!is_leap_year(1900));
        assert!(!is_leap_year(2100));
        assert!(is_leap_year(2024));
    }

    #[test]
    fn thirty_360_variants_differ_only_on_day_31() {
        let (a, _) = DayCount::Thirty360
            .year_fraction(d("2024-02-15"), d("2024-03-31"))
            .unwrap();
        let (b, _) = DayCount::ThirtyE360
            .year_fraction(d("2024-02-15"), d("2024-03-31"))
            .unwrap();
        assert_eq!(a, Decimal::from(46) / Decimal::from(360));
        assert_eq!(b, Decimal::from(45) / Decimal::from(360));
    }

    #[test]
    fn end_before_start_is_rejected() {
        let e = DayCount::Act360
            .year_fraction(d("2024-02-02"), d("2024-02-01"))
            .unwrap_err();
        assert_eq!(e.code(), "FM-E001");
        assert_eq!(
            e.to_string(),
            "invalid input: `end` (2024-02-01) precedes `start` (2024-02-02); swap them"
        );
    }

    #[test]
    fn thirty_360_formula_states_the_day_adjustments() {
        // Issue #176: the adjusted days are written next to the formula.
        let f = |dc: DayCount, a: &str, b: &str| dc.year_fraction(d(a), d(b)).unwrap().1;
        assert!(f(DayCount::Thirty360, "2024-01-31", "2024-03-31").ends_with(
            "; start day 31 adjusted to 30, end day 31 adjusted to 30 (the start day is 30 or 31)"
        ));
        assert!(f(DayCount::Thirty360, "2024-02-15", "2024-03-31")
            .ends_with("; end day 31 kept (the start day 15 is below 30)"));
        assert!(f(DayCount::ThirtyE360, "2024-02-15", "2024-03-31")
            .ends_with("; end day 31 adjusted to 30"));
        assert!(f(DayCount::Thirty360, "2024-02-15", "2024-03-15").ends_with("] / 360"));
    }
}
