//! Calendar dates, day-count conventions and schedules of accrual periods (ADR 0008, SPEC-0002
//! R1, R2, SPEC-0010 R1 to R8).
//!
//! `Date` is a proleptic Gregorian calendar date without time or time zone, valid from
//! 1900-01-01 to 2199-12-31 and written as ISO 8601 `YYYY-MM-DD`. Day counts follow the
//! 2006 ISDA Definitions, section 4.16. A schedule is a run of contiguous accrual periods, each
//! from and including its start date to but excluding its end date, with unadjusted dates
//! generated from the start date by whole months and the end-of-month rule.

use std::borrow::Cow;
use std::fmt;
use std::str::FromStr;

use rust_decimal::Decimal;
use schemars::{json_schema, JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::frequency::{self, check_number_of_periods, Frequency, PeriodLength};
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
        let (terms, formula) = self.terms(start, end)?;
        Ok((terms.value()?, formula))
    }

    /// The total year fraction of `periods` with a single rounding per denominator, and its
    /// formula with the numbers substituted (for example `121 / 365`, or `a / 365 + b / 366 +
    /// w` under Actual/Actual (ISDA)): the numerators of the exact fractions are added as
    /// integers first, so that under Actual/365 (Fixed) the total of contiguous periods equals
    /// their total days / 365 exactly (SPEC-0010 AC3). It may differ from the sum of the rounded
    /// year fractions by up to n half-units of their last digit. Each fraction is the one of
    /// [`DayCount::year_fraction`] (no second definition).
    pub(crate) fn year_fraction_sum(self, periods: &[(Date, Date)]) -> Result<(Decimal, String)> {
        // Numerators by denominator (360, 365, 366) and whole years, all exact integers.
        let mut by_denominator: Vec<(i64, i64)> = Vec::new();
        let mut whole: i64 = 0;
        for &(start, end) in periods {
            let (terms, _) = self.terms(start, end)?;
            whole = whole
                .checked_add(terms.whole)
                .ok_or(FairmarkError::Overflow("year fraction"))?;
            for (n, d) in terms.parts.into_iter().flatten() {
                match by_denominator.iter_mut().find(|(_, den)| *den == d) {
                    Some((sum, _)) => {
                        *sum = sum
                            .checked_add(n)
                            .ok_or(FairmarkError::Overflow("year fraction"))?
                    }
                    None => by_denominator.push((n, d)),
                }
            }
        }
        by_denominator.sort_by_key(|(_, d)| *d);
        let mut total = Decimal::from(whole);
        let mut terms = Vec::with_capacity(by_denominator.len() + 1);
        for (n, d) in by_denominator {
            total = num::add(total, num::div(Decimal::from(n), Decimal::from(d))?)?;
            terms.push(format!("{n} / {d}"));
        }
        if whole != 0 || terms.is_empty() {
            terms.push(whole.to_string());
        }
        Ok((total, terms.join(" + ")))
    }

    /// The exact terms of the year fraction from `start` to `end`, and its formula.
    fn terms(self, start: Date, end: Date) -> Result<(FractionTerms, String)> {
        if end < start {
            return Err(FairmarkError::InvalidInput(format!(
                "`end` ({end}) precedes `start` ({start}); swap them"
            ))
            .at("/end", crate::error::Reason::Range)
            .between(["/end", "/start"])
            .fix("swap `start` and `end`"));
        }
        let days = start.days_until(end);
        let one = |n: i64, d: i64| FractionTerms {
            parts: [Some((n, d)), None],
            whole: 0,
        };
        Ok(match self {
            Self::Act365Fixed => (one(days, 365), format!("{days} / 365")),
            Self::Act360 => (one(days, 360), format!("{days} / 360")),
            Self::ActActIsda => {
                if start.year == end.year {
                    let den = days_in_year(start.year);
                    (one(days, den), format!("{days} / {den}"))
                } else {
                    let next = Date::new(start.year + 1, 1, 1)?;
                    let first = start.days_until(next);
                    let jan1 = Date::new(end.year, 1, 1)?;
                    let last = jan1.days_until(end);
                    let whole = i64::from(end.year - start.year - 1);
                    let (d1, d2) = (days_in_year(start.year), days_in_year(end.year));
                    (
                        FractionTerms {
                            parts: [Some((first, d1)), Some((last, d2))],
                            whole,
                        },
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
                    one(numerator, 360),
                    format!(
                        "[360 × ({} − {}) + 30 × ({} − {}) + ({d2} − {d1})] / 360{adjusted}",
                        end.year, start.year, end.month, start.month
                    ),
                )
            }
        })
    }
}

/// A year fraction as exact integer terms: up to two fractions `numerator / denominator` and a
/// whole number of years (Actual/Actual (ISDA) across years).
struct FractionTerms {
    parts: [Option<(i64, i64)>; 2],
    whole: i64,
}

impl FractionTerms {
    /// The decimal value, in the order of operations of version 1 of `dates.year_fraction`:
    /// `n / d` for one fraction, `(n1 / d1 + whole) + n2 / d2` across years.
    fn value(&self) -> Result<Decimal> {
        let div = |(n, d): (i64, i64)| num::div(Decimal::from(n), Decimal::from(d));
        match self.parts {
            [Some(a), None] => div(a),
            [Some(a), Some(b)] => num::add(num::add(div(a)?, Decimal::from(self.whole))?, div(b)?),
            _ => Err(FairmarkError::Internal("year fraction without terms")),
        }
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

/// Output of `dates.year_fraction`.
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

// ---------------------------------------------------------------- dates.schedule

/// Input of `dates.schedule`. The period length is given in exactly one of two forms:
/// `frequency` or `period_months`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScheduleInput {
    /// Start date of the first period (ISO 8601). Each period runs from and including its start
    /// date to but excluding its end date, [start, end); the end date of a period is the start
    /// date of the next one.
    pub start_date: Date,
    /// Period length as a named frequency: `monthly` (1 month per period, 12 a year),
    /// `bimonthly` (2 months, every two months, 6 a year), `quarterly` (3 months, 4 a year),
    /// `four_monthly` (4 months, 3 a year), `semi_annual` (6 months, 2 a year) or `annual` (12
    /// months, 1 a year). Give exactly one of `frequency` and `period_months`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency: Option<Frequency>,
    /// Period length as a positive whole number of months per period (not periods a year), for
    /// tenors with or without a name: 1 = `monthly`, 2 = `bimonthly`, 3 = `quarterly`, 4 =
    /// `four_monthly`, 6 = `semi_annual`, 12 = `annual`, and any other such as 5, 18 or 24. Give
    /// exactly one of `frequency` and `period_months`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[schemars(range(min = 1))]
    pub period_months: Option<u32>,
    /// Number of periods, from 1 to 10000.
    #[schemars(range(min = 1, max = 10000))]
    pub number_of_periods: u32,
    /// Day count for the year fraction of each period, measured on [start, end). Optional:
    /// without it no year fraction is computed (none is assumed).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_count: Option<DayCount>,
}

/// Output of `dates.schedule`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScheduleOutput {
    /// Start date of each period, in order (ISO 8601). A period runs from and including its
    /// start date to but excluding its end date.
    pub start_dates: Vec<Date>,
    /// End date of each period, in order (ISO 8601): the first day of the next period, not part
    /// of the period. A period runs from and including its start date to but excluding its end
    /// date.
    pub end_dates: Vec<Date>,
    /// Year fraction of each period from its start date to its end date under `day_count`;
    /// absent without a day count.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_fractions: Option<Vec<Decimal>>,
    /// Months per period.
    pub months_per_period: u32,
    /// Periods a year (12 / months per period); absent when the months per period do not
    /// divide 12.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub periods_per_year: Option<u32>,
}

/// A schedule built from its definition, with what `explain` reports (R1 to R8). Shared with
/// `series.project`, which builds its periods with [`build_schedule`] (R14).
#[derive(Debug, Clone)]
pub(crate) struct BuiltSchedule {
    pub start_date: Date,
    pub start_dates: Vec<Date>,
    pub end_dates: Vec<Date>,
    pub length: PeriodLength,
    pub day_count: Option<DayCount>,
    /// Year fraction of each period, with a day count.
    pub year_fractions: Option<Vec<Decimal>>,
    /// The day-count calculation of the first period, with the numbers substituted.
    first_year_fraction: Option<String>,
    /// The start date is the last day of its month: every date is a month end.
    month_end_start: bool,
    /// How many generated dates fall on a day other than the start day, and the first of them;
    /// `None` when the rule changed no date.
    days_changed: Option<(usize, Date)>,
}

/// Whether `d` is the last day of its month.
fn is_month_end(d: Date) -> bool {
    d.day == days_in_month(d.year, d.month)
}

/// The date `months` months after `start`, counted from the start date (R2): the last day of the
/// month when `end_of_month`, otherwise the day of `start`, clamped to the length of the month.
/// Month arithmetic is in `i64`, so no accepted input can overflow (R7). A date after
/// 2199-12-31 is returned as `Err` with its text, for the message.
fn add_months(start: Date, months: i64, end_of_month: bool) -> std::result::Result<Date, String> {
    let index = i64::from(start.year) * 12 + i64::from(start.month) - 1 + months;
    let (year, month) = (index.div_euclid(12), index.rem_euclid(12) as u32 + 1);
    // The Gregorian leap pattern repeats every 400 years, so the length of a month of a year
    // beyond the supported range is that of the same year of the 2000s.
    let length = days_in_month((year.rem_euclid(400) + 2000) as i32, month);
    let day = if end_of_month {
        length
    } else {
        start.day.min(length)
    };
    match i32::try_from(year) {
        Ok(y) if (MIN_YEAR..=MAX_YEAR).contains(&y) => {
            Date::new(y, month, day).map_err(|_| format!("{year:04}-{month:02}-{day:02}"))
        }
        _ => Err(format!("{year:04}-{month:02}-{day:02}")),
    }
}

/// Builds the schedule of `input` (R1 to R7). `prefix` is the JSON pointer of the definition
/// (empty for `dates.schedule`, `/timeline/schedule` in `series.project`), so that every error
/// names the field. Validation order: period length, number of periods, then the dates (the
/// start date is checked when the input is read).
pub(crate) fn build_schedule(input: &ScheduleInput, prefix: &str) -> Result<BuiltSchedule> {
    let length = frequency::period_length(input.frequency, input.period_months, prefix, true)?
        .ok_or(FairmarkError::Internal("required period length missing"))?;
    let n = input.number_of_periods;
    check_number_of_periods(
        n,
        &format!("{prefix}/number_of_periods"),
        "periods of the schedule",
    )?;
    let start = input.start_date;
    let month_end_start = is_month_end(start);
    let p = i64::from(length.months());
    let mut dates = Vec::with_capacity(n as usize + 1);
    dates.push(start);
    for k in 1..=i64::from(n) {
        let date = add_months(start, k * p, month_end_start).map_err(|out| {
            let fix = if k == 1 {
                format!(
                    "no period of {p} months fits from {start}: give a shorter period length or \
                     an earlier start date"
                )
            } else {
                format!(
                    "the last period within the range is period {} (ending {}): give at most {} \
                     periods, or a shorter period length",
                    k - 1,
                    dates[dates.len() - 1],
                    k - 1
                )
            };
            FairmarkError::InvalidDateAt {
                pointer: format!("{prefix}/number_of_periods"),
                detail: format!(
                    "period {k} would end on {out}, after 2199-12-31, the last supported date; \
                     {fix}"
                ),
            }
            .because(crate::error::Reason::Range)
            .fix(fix.clone())
        })?;
        dates.push(date);
    }
    let changed: Vec<Date> = dates[1..]
        .iter()
        .copied()
        .filter(|d| d.day != start.day)
        .collect();
    let (year_fractions, first_year_fraction) = match input.day_count {
        Some(dc) => {
            let mut fractions = Vec::with_capacity(n as usize);
            let mut first = None;
            for pair in dates.windows(2) {
                let (yf, formula) = dc.year_fraction(pair[0], pair[1])?;
                first.get_or_insert(formula);
                fractions.push(yf);
            }
            (Some(fractions), first)
        }
        None => (None, None),
    };
    Ok(BuiltSchedule {
        start_date: start,
        start_dates: dates[..dates.len() - 1].to_vec(),
        end_dates: dates[1..].to_vec(),
        length,
        day_count: input.day_count,
        year_fractions,
        first_year_fraction,
        month_end_start,
        days_changed: changed.first().map(|first| (changed.len(), *first)),
    })
}

/// The period definition, the generation rule, the period length, the end-of-month rule and the
/// day count of a schedule, and its year-fraction steps (R8). The number of steps does not
/// depend on the number of periods; the output vectors carry the per-period numbers.
pub(crate) fn schedule_explain(s: &BuiltSchedule, ex: &mut Explain) -> Result<()> {
    let n = s.end_dates.len();
    let p = s.length.months();
    ex.convention(
        "periods: each period runs from and including its start date to but excluding its end \
         date, [start, end) (2006 ISDA Definitions, Calculation Period); the end date of a \
         period is the start date of the next one and is not part of the period",
    );
    ex.convention(format!(
        "date generation: every date from the start date, d_k = {} + k × {p} months, k = 1..{n} \
         (forward, no drift)",
        s.start_date
    ));
    ex.convention(s.length.describe());
    let effect = match s.days_changed {
        None => "no date changed by the rule".to_owned(),
        Some((count, first)) => format!(
            "{count} of {n} end dates fall on a day other than {} (first: {first})",
            s.start_date.day
        ),
    };
    ex.convention(if s.month_end_start {
        format!(
            "end-of-month rule: the start date {} is the last day of its month, so every date is \
             the last day of its month; {effect}",
            s.start_date
        )
    } else {
        format!(
            "end-of-month rule: the start date {} is not the last day of its month, so every date \
             keeps day {}, or the last day of a shorter month; {effect}",
            s.start_date, s.start_date.day
        )
    });
    ex.convention("dates unadjusted: no business-day calendar or convention is applied");
    match s.day_count {
        Some(dc) => ex.convention(format!(
            "day count: {} (given), year fraction of each period on [start, end)",
            dc.label()
        )),
        None => ex.convention("day count: none given, so no year fraction is computed"),
    }
    ex.step("Number of periods", format!("{n}"), Decimal::from(n));
    ex.step(
        "Months per period",
        match s.length {
            PeriodLength::Named(f) => format!("`{}`: 12 / {}", f.name(), f.per_year()),
            PeriodLength::Months(m) => format!("`period_months` {m}"),
        },
        Decimal::from(p),
    );
    if let Some(m) = s.length.per_year() {
        ex.step("Periods per year", format!("12 / {p}"), Decimal::from(m));
    }
    let last = s.end_dates[n - 1];
    ex.step(
        "Actual days in the schedule",
        format!("{} to {last}", s.start_date),
        Decimal::from(s.start_date.days_until(last)),
    );
    if let (Some(fractions), Some(formula)) = (&s.year_fractions, &s.first_year_fraction) {
        ex.step(
            format!(
                "Year fraction of period 1, {} to {}",
                s.start_dates[0], s.end_dates[0]
            ),
            formula.clone(),
            fractions[0],
        );
        let dc = s.day_count.ok_or(FairmarkError::Internal(
            "year fractions without a day count",
        ))?;
        let periods: Vec<(Date, Date)> = s
            .start_dates
            .iter()
            .copied()
            .zip(s.end_dates.iter().copied())
            .collect();
        let (total, formula) = dc.year_fraction_sum(&periods)?;
        ex.step(
            "Total year fraction of the schedule, exact (numerators added before dividing)",
            formula,
            total,
        );
    }
    Ok(())
}

pub fn schedule(input: &ScheduleInput) -> Result<(ScheduleOutput, Explain)> {
    let s = build_schedule(input, "")?;
    let mut ex = Explain::new(
        "dates.schedule",
        "period k = [d_(k−1), d_k), d_0 = start date, d_k = start date + k × p months \
         (end-of-month rule), k = 1..n",
    );
    schedule_explain(&s, &mut ex)?;
    Ok((
        ScheduleOutput {
            months_per_period: s.length.months(),
            periods_per_year: s.length.per_year(),
            start_dates: s.start_dates,
            end_dates: s.end_dates,
            year_fractions: s.year_fractions,
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

/// `dates.schedule`: see the methodology page.
pub struct Schedule;

impl Method for Schedule {
    const ID: &'static str = "dates.schedule";
    const TITLE: &'static str = "Schedule of accrual periods";
    const DESCRIPTION: &'static str =
        "Start and end dates of n contiguous accrual periods from a start date, each from and \
        including its start date to but excluding its end date (2006 ISDA Definitions): the end \
        date is the first day of the next period. Period length as a frequency (`monthly` 1 \
        month, `bimonthly` 2, `quarterly` 3, `four_monthly` 4, `semi_annual` 6, `annual` 12) or \
        as `period_months`; unadjusted dates with the end-of-month rule; optional year \
        fractions under a day count.";
    const VERSION: &'static str = "1";
    type Input = ScheduleInput;
    type Output = ScheduleOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        schedule(input)
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

    // ------------------------------------------------------------ dates.schedule

    fn sched(json: serde_json::Value) -> Result<(ScheduleOutput, Explain)> {
        schedule(&serde_json::from_value(json).unwrap())
    }

    fn err(json: serde_json::Value) -> FairmarkError {
        sched(json).unwrap_err()
    }

    /// AC4: errors name the field by its JSON pointer and state the fix.
    #[test]
    fn schedule_errors_name_the_field_and_the_fix() {
        use serde_json::json;
        for n in [0, 10_001] {
            let e = err(json!({"start_date": "2025-01-31", "frequency": "monthly",
                               "number_of_periods": n}));
            assert_eq!(e.code(), "FM-E001");
            assert!(
                e.to_string()
                    .starts_with("invalid input at `/number_of_periods`: must be between 1"),
                "{e}"
            );
        }
        let both = err(json!({"start_date": "2025-01-31", "frequency": "quarterly",
                              "period_months": 3, "number_of_periods": 4}));
        assert_eq!(both.code(), "FM-E001");
        let m = both.to_string();
        assert!(
            m.contains("`/frequency`") && m.contains("`/period_months`"),
            "{m}"
        );
        assert!(m.contains("give exactly one"), "{m}");
        let neither = err(json!({"start_date": "2025-01-31", "number_of_periods": 4}));
        let m = neither.to_string();
        assert!(
            m.contains("`/frequency`") && m.contains("`/period_months`"),
            "{m}"
        );
        assert!(m.contains("give exactly one"), "{m}");
        let zero = err(json!({"start_date": "2025-01-31", "period_months": 0,
                              "number_of_periods": 4}));
        assert!(
            zero.to_string()
                .starts_with("invalid input at `/period_months`"),
            "{zero}"
        );

        let late = err(json!({"start_date": "2199-06-30", "frequency": "annual",
                              "number_of_periods": 1}));
        assert_eq!(late.code(), "FM-E009");
        let m = late.to_string();
        assert!(m.starts_with("invalid date at `/number_of_periods`"), "{m}");
        assert!(m.contains("2200-06-30"), "{m}");
        assert!(m.contains("give a shorter period length"), "{m}");
        let m = err(json!({"start_date": "2190-01-01", "period_months": 120,
                           "number_of_periods": 1}))
        .to_string();
        assert!(m.contains("2200-01-01"), "{m}");
        // The last valid period and its end date are named.
        let m = err(json!({"start_date": "2197-12-31", "frequency": "annual",
                           "number_of_periods": 5}))
        .to_string();
        assert!(m.contains("period 3 would end on 2200-12-31"), "{m}");
        assert!(m.contains("the last period within the range is period 2 (ending 2199-12-31)"));
        assert!(m.contains("give at most 2 periods"), "{m}");
        // The largest period length with the largest number of periods: no overflow, no panic.
        let e = err(
            json!({"start_date": "2025-01-31", "period_months": u32::MAX,
                           "number_of_periods": 10_000}),
        );
        assert_eq!(e.code(), "FM-E009");
    }

    /// AC4 and AC5 through the registry: unknown fields are refused with their pointer, and
    /// the impossible start date is named.
    #[test]
    fn schedule_input_is_read_with_pointers() {
        for field in ["end_date", "calendar", "business_day_convention", "stub"] {
            let input = format!(
                r#"{{"start_date": "2025-01-31", "frequency": "monthly", "number_of_periods": 3,
                    "{field}": "2026-01-31"}}"#
            );
            let e = crate::run("dates.schedule", &input).unwrap_err();
            assert_eq!(e.code(), "FM-E001");
            assert!(
                e.to_string()
                    .starts_with(&format!("invalid input at `/{field}`")),
                "{e}"
            );
        }
        let e = crate::run(
            "dates.schedule",
            r#"{"start_date": "2023-02-29", "frequency": "monthly", "number_of_periods": 3}"#,
        )
        .unwrap_err();
        assert_eq!(e.code(), "FM-E009");
        assert!(
            e.to_string().starts_with("invalid date at `/start_date`"),
            "{e}"
        );
    }

    /// AC5: the method description and the schema state [start, end) and the name-to-tenor
    /// mapping.
    #[test]
    fn schedule_descriptions_state_the_period_definition_and_the_tenors() {
        let info = crate::method("dates.schedule").unwrap();
        let input = crate::input_schema("dates.schedule").unwrap();
        let output = serde_json::to_value(schemars::schema_for!(ScheduleOutput)).unwrap();
        let description = |schema: &serde_json::Value, field: &str| {
            schema["properties"][field]["description"]
                .as_str()
                .unwrap_or_else(|| panic!("{field} has no description"))
                .replace('\n', " ")
        };
        for text in [
            info.description.to_owned(),
            description(&input, "start_date"),
            description(&output, "start_dates"),
            description(&output, "end_dates"),
        ] {
            assert!(text.contains("from and including"), "{text}");
            assert!(text.contains("to but excluding"), "{text}");
        }
        let tenors = [
            ("`monthly`", "1"),
            ("`bimonthly`", "2"),
            ("`quarterly`", "3"),
            ("`four_monthly`", "4"),
            ("`semi_annual`", "6"),
            ("`annual`", "12"),
        ];
        for text in [
            info.description.to_owned(),
            description(&input, "frequency"),
            description(&input, "period_months"),
        ] {
            for (name, months) in tenors {
                let at = text
                    .find(name)
                    .unwrap_or_else(|| panic!("{name} in {text}"));
                let around = &text[at.saturating_sub(5)..(at + name.len() + 4).min(text.len())];
                assert!(around.contains(months), "{name} {months} in {text}");
            }
        }
    }

    /// AC5 and R8: every item of `explain`, and a number of steps independent of n.
    #[test]
    fn schedule_explain_is_complete_and_bounded() {
        use serde_json::json;
        let run = |n: u32, day_count: Option<&str>| {
            let mut input = json!({"start_date": "2024-01-31", "frequency": "monthly",
                                   "number_of_periods": n});
            if let Some(dc) = day_count {
                input["day_count"] = json!(dc);
            }
            sched(input).unwrap()
        };
        let (out, ex) = run(4, Some("act_365_fixed"));
        let conventions = ex.conventions.join("\n");
        for item in [
            "from and including its start date to but excluding its end date, [start, end)",
            "d_k = 2024-01-31 + k × 1 months",
            "period length: 1 month per period, given as `frequency` `monthly`, 12 periods a year",
            "end-of-month rule: the start date 2024-01-31 is the last day of its month",
            "2 of 4 end dates fall on a day other than 31 (first: 2024-02-29)",
            "dates unadjusted",
            "day count: Actual/365 (Fixed)",
        ] {
            assert!(conventions.contains(item), "{item} not in {conventions}");
        }
        let labels: Vec<&str> = ex.steps.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(
            labels,
            [
                "Number of periods",
                "Months per period",
                "Periods per year",
                "Actual days in the schedule",
                "Year fraction of period 1, 2024-01-31 to 2024-02-29",
                "Total year fraction of the schedule, exact (numerators added before dividing)",
            ]
        );
        assert_eq!(ex.steps[5].formula, "121 / 365");
        assert_eq!(ex.steps[4].formula, "29 / 365");
        assert_eq!(ex.steps[4].value, out.year_fractions.as_ref().unwrap()[0]);
        assert_eq!(
            run(400, Some("act_365_fixed")).1.steps.len(),
            ex.steps.len()
        );

        let (out, ex) = run(4, None);
        assert!(out.year_fractions.is_none());
        assert!(ex
            .conventions
            .contains(&"day count: none given, so no year fraction is computed".to_owned()));
        assert_eq!(ex.steps.len(), 4);
        assert_eq!(run(400, None).1.steps.len(), 4);
        let json = serde_json::to_value(&out).unwrap();
        assert!(json.get("year_fractions").is_none());

        let (out, ex) = sched(json!({"start_date": "2025-01-31", "period_months": 5,
                                     "number_of_periods": 3}))
        .unwrap();
        assert_eq!((out.months_per_period, out.periods_per_year), (5, None));
        assert!(serde_json::to_value(&out)
            .unwrap()
            .get("periods_per_year")
            .is_none());
        assert!(ex.conventions.iter().any(|c| c
            == "period length: 5 months per period, given as `period_months` 5 (no whole number \
                of periods a year)"));
        let (_, ex) = sched(json!({"start_date": "2025-01-30", "frequency": "monthly",
                                   "number_of_periods": 3}))
        .unwrap();
        assert!(ex.conventions.iter().any(|c| c.starts_with(
            "end-of-month rule: the start date 2025-01-30 is not the last day of its month, so \
             every date keeps day 30"
        )));
        let (_, ex) = sched(
            json!({"start_date": "2025-03-15", "frequency": "semi_annual",
                                   "number_of_periods": 3}),
        )
        .unwrap();
        assert!(ex
            .conventions
            .iter()
            .any(|c| c.ends_with("no date changed by the rule")));
    }

    /// AC3: every year fraction is that of `dates.year_fraction` on its period, and the sum
    /// under Actual/365 (Fixed) is the total days / 365 exactly.
    #[test]
    fn schedule_year_fractions_are_those_of_year_fraction() {
        let all = [
            DayCount::Act365Fixed,
            DayCount::Act360,
            DayCount::ActActIsda,
            DayCount::Thirty360,
            DayCount::ThirtyE360,
        ];
        for dc in all {
            for (start, length) in [("2023-06-30", 6), ("2024-01-31", 1), ("2025-12-31", 3)] {
                let input = ScheduleInput {
                    start_date: d(start),
                    frequency: None,
                    period_months: Some(length),
                    number_of_periods: 40,
                    day_count: Some(dc),
                };
                let (out, ex) = schedule(&input).unwrap();
                let fractions = out.year_fractions.unwrap();
                for (k, yf) in fractions.iter().enumerate() {
                    let (one, _) = year_fraction(&YearFractionInput {
                        start: out.start_dates[k],
                        end: out.end_dates[k],
                        day_count: dc,
                    })
                    .unwrap();
                    assert_eq!(*yf, one.year_fraction, "{dc:?} {start} period {}", k + 1);
                }
                let sum = ex.steps.last().unwrap().value;
                let formula = &ex.steps.last().unwrap().formula;
                match dc {
                    DayCount::Act365Fixed => assert!(formula.ends_with(" / 365"), "{formula}"),
                    DayCount::Act360 | DayCount::Thirty360 | DayCount::ThirtyE360 => {
                        assert!(formula.ends_with(" / 360"), "{formula}")
                    }
                    DayCount::ActActIsda => assert!(formula.contains(" / 36"), "{formula}"),
                }
                if dc == DayCount::Act365Fixed {
                    let days = out.start_dates[0].days_until(out.end_dates[39]);
                    assert_eq!(sum, Decimal::from(days) / Decimal::from(365), "{start}");
                }
            }
        }
    }

    /// The total of an Actual/Actual (ISDA) schedule shows its exact terms: days over 365, days
    /// over 366 and whole years.
    #[test]
    fn total_year_fraction_shows_the_exact_terms() {
        let periods = [
            (d("2023-06-30"), d("2023-12-31")),
            (d("2023-12-31"), d("2024-06-30")),
            (d("2024-06-30"), d("2026-06-30")),
        ];
        let (total, formula) = DayCount::ActActIsda.year_fraction_sum(&periods).unwrap();
        // Over 365: 184 + 1 days of 2023 and 180 days of 2026; over 366: 181 + 185 days of 2024;
        // one whole year, 2025. Numerators are added by denominator, never reduced.
        assert_eq!(formula, "365 / 365 + 366 / 366 + 1");
        assert_eq!(total, Decimal::from(3));
        let (_, f) = DayCount::Act365Fixed
            .year_fraction_sum(&[(d("2024-01-01"), d("2024-03-01"))])
            .unwrap();
        assert_eq!(f, "60 / 365");
    }

    /// R3: a named frequency and the equal number of months give the same values.
    #[test]
    fn named_frequency_equals_period_months() {
        for f in Frequency::ALL {
            let mk = |frequency, period_months| ScheduleInput {
                start_date: d("2024-02-29"),
                frequency,
                period_months,
                number_of_periods: 30,
                day_count: Some(DayCount::ActActIsda),
            };
            let (a, _) = schedule(&mk(Some(f), None)).unwrap();
            let (b, _) = schedule(&mk(None, Some(f.months()))).unwrap();
            assert_eq!(
                serde_json::to_string(&a).unwrap(),
                serde_json::to_string(&b).unwrap(),
                "{f:?}"
            );
        }
    }
}
