//! Value series projected period by period on a timeline (SPEC-0010 R9 to R19).
//!
//! `series.project` grows a base value by a growth rule, V_k = V_(k−1) × (1 + g_k), period by
//! period, on a timeline that is either a number of periods (with an optional period length)
//! or a schedule built by `dates.schedule` (the same function, so the dates are identical by
//! construction). Annual growth rates are converted to the period length by a stated basis.

use rust_decimal::{Decimal, MathematicalOps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dates::{self, BuiltSchedule, Date, ScheduleInput};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::frequency::{self, check_number_of_periods, Frequency, PeriodLength};
use crate::method::Method;
use crate::num;
use crate::rates::{self, Compounding};
use crate::tvm::DatedCashFlow;

// ---------------------------------------------------------------- input

/// Which period the base value belongs to (R10). Required: the two readings differ by a full
/// period of growth on every value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum BasePeriod {
    /// The base value is the value before the first period (for example the last actual
    /// year): V_1 = V_0 × (1 + g_1), n growth steps.
    #[serde(rename = "period_0")]
    Period0,
    /// The base value is the value of period 1: V_1 = V_0, growth steps into periods 2 to n
    /// only (n − 1 steps).
    #[serde(rename = "period_1")]
    Period1,
}

/// A constant growth rate.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConstantGrowth {
    /// Growth rate applied to every growth step, decimal fraction (0.05 = 5%), greater than
    /// -1, read according to `growth_basis`.
    pub growth: Decimal,
}

/// A growth rate that moves in equal steps between two rates.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LinearGrowth {
    /// The growth before the first step, decimal fraction, greater than -1, read according to
    /// `growth_basis`: step j of s applies from_growth + (to_growth − from_growth) × j / s.
    pub from_growth: Decimal,
    /// The growth of the last step, decimal fraction, greater than -1, read according to
    /// `growth_basis`.
    pub to_growth: Decimal,
}

/// One growth rate per growth step.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PerPeriodGrowth {
    /// One growth rate per growth step, in order, decimal fractions, each greater than -1, read
    /// according to `growth_basis`: n rates under `period_0`, n − 1 under `period_1`.
    pub growth_rates: Vec<Decimal>,
}

/// The growth rule (R11, R12).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GrowthRule {
    /// The same rate for every growth step: `{"constant": {"growth": 0.05}}`.
    Constant(ConstantGrowth),
    /// From `from_growth` to `to_growth` in equal steps, reaching `to_growth` in the last step:
    /// `{"linear": {"from_growth": 0.10, "to_growth": 0.03}}`.
    Linear(LinearGrowth),
    /// One rate per growth step: `{"per_period": {"growth_rates": [0.10, 0.08]}}`.
    PerPeriod(PerPeriodGrowth),
}

/// How the rates of the growth rule are read (R13).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum GrowthBasis {
    /// Rates per period of the timeline, used as given.
    PerPeriod,
    /// Annual growth rates, converted to the rate per period that compounds exactly to the
    /// annual rate: (1 + g)^(1/m) − 1 with m periods a year (the equivalent rate of
    /// `rates.convert`), or (1 + g)^k − 1 with periods of k years.
    AnnualCompound,
    /// Annual growth rates, converted in proportion to the period length: g / m with m periods
    /// a year (m periods then compound to more than g), or g × k with periods of k years.
    AnnualSimple,
}

/// A timeline given as a number of periods, without dates.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PeriodsTimeline {
    /// Number of periods, from 1 to 10000.
    pub number_of_periods: u32,
    /// Optional period length as a named frequency: `monthly` (1 month), `bimonthly` (2 months,
    /// every two months), `quarterly` (3), `four_monthly` (4), `semi_annual` (6) or `annual`
    /// (12). At most one of `frequency` and `period_months`; without either the rates are per
    /// period and no annual basis applies.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frequency: Option<Frequency>,
    /// Optional period length as a positive whole number of months per period (1 = `monthly`,
    /// 3 = `quarterly`, 12 = `annual`, or any other such as 5 or 18). At most one of
    /// `frequency` and `period_months`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_months: Option<u32>,
}

/// The periods the values are projected on (R14).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Timeline {
    /// A number of periods with an optional period length, no dates:
    /// `{"periods": {"number_of_periods": 5, "frequency": "annual"}}`.
    Periods(PeriodsTimeline),
    /// A schedule defined as the input of `dates.schedule`; each period runs from and including
    /// its start date to but excluding its end date: `{"schedule": {"start_date":
    /// "2025-12-31", "frequency": "quarterly", "number_of_periods": 4}}`.
    Schedule(ScheduleInput),
}

/// Which date of its period a value is dated at, with a `schedule` timeline (R15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValueTiming {
    /// At the end date of the period, which is the first day of the next period.
    EndOfPeriod,
    /// At the start date of the period.
    BeginningOfPeriod,
}

/// Input of `series.project`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProjectInput {
    /// Base value V_0, in the units of the series (currency units for revenues or flows); may
    /// be negative.
    pub base_value: Decimal,
    /// `period_0` (the base value is before period 1, V_1 = V_0 × (1 + g_1)) or `period_1` (the
    /// base value is the value of period 1, V_1 = V_0). Required, no default.
    pub base_period: BasePeriod,
    /// Growth rule, externally tagged: `constant`, `linear` or `per_period`.
    pub growth: GrowthRule,
    /// How the growth rates are read: `per_period`, `annual_compound` or `annual_simple`.
    /// Required when the timeline has a period length other than 12 months; may be omitted with
    /// 12-month periods (the three bases coincide) or without a period length (rates per
    /// period). The annual bases need a period length that divides 12 or is a multiple of 12.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_basis: Option<GrowthBasis>,
    /// The periods: `periods` (a number of periods) or `schedule` (dated periods, built as by
    /// `dates.schedule`).
    pub timeline: Timeline,
    /// With a `schedule` timeline only, and then required: `end_of_period` (each value dated at
    /// the end date of its period, the first day of the next period) or `beginning_of_period`
    /// (at its start date).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timing: Option<ValueTiming>,
}

// ---------------------------------------------------------------- output

/// One projected period (R16).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProjectedPeriod {
    /// Period number, from 1.
    pub period: u32,
    /// Start date of the period, included in it (`schedule` timeline only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_date: Option<Date>,
    /// End date of the period, the first day of the next period, excluded from it (`schedule`
    /// timeline only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_date: Option<Date>,
    /// Year fraction of the period under the schedule's day count (`schedule` timeline with a
    /// day count only).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year_fraction: Option<Decimal>,
    /// The growth rate of the rule for the step into this period, as stated (before
    /// conversion); absent for period 1 under `period_1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stated_growth: Option<Decimal>,
    /// The growth per period applied into this period, after conversion; absent for period 1
    /// under `period_1`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub growth_per_period: Option<Decimal>,
    /// The annual growth implied by `growth_per_period`, (1 + g)^m − 1 with m periods a year;
    /// only when the months per period divide 12.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub effective_annual_growth: Option<Decimal>,
    /// The value of the period, in the units of the base value.
    pub value: Decimal,
}

/// Output of `series.project`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProjectOutput {
    /// One row per period.
    pub periods: Vec<ProjectedPeriod>,
    /// The values of periods 1 to n, the shape of `cash_flows` in `tvm.present_value`.
    pub values: Vec<Decimal>,
    /// With a `schedule` timeline: one `{date, amount}` per period, dated by `timing`, the shape
    /// of `cash_flows` in `tvm.xnpv` and `tvm.xirr`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dated_values: Option<Vec<DatedCashFlow>>,
    /// Months per period, when the timeline has a period length.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub months_per_period: Option<u32>,
    /// Periods a year, when the months per period divide 12.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub periods_per_year: Option<u32>,
}

// ---------------------------------------------------------------- calculation

/// How each stated rate becomes a rate per period (R13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Conversion {
    /// The rates are per period: used as given.
    None,
    /// m periods a year: the equivalent rate of `rates.convert` with compounding m, divided by
    /// m.
    CompoundRoot { m: u32, compounding: Compounding },
    /// m periods a year: g / m.
    SimpleShare { m: u32 },
    /// Periods of k years: (1 + g)^k − 1.
    CompoundPower { k: u32 },
    /// Periods of k years: g × k.
    SimpleMultiple { k: u32 },
}

const BASIS_FIX: &str = "give `per_period` with the rates per period of the timeline, or \
                         `annual_compound` (the rate per period that compounds to the annual rate) \
                         or `annual_simple` (the annual rate in proportion to the period length)";

/// The conversion of the rates, and the convention that states it (R13).
fn resolve_basis(
    basis: Option<GrowthBasis>,
    length: Option<PeriodLength>,
) -> Result<(Conversion, String)> {
    let at = |detail: String| FairmarkError::InvalidInputAt {
        pointer: "/growth_basis".to_owned(),
        detail,
    };
    let Some(length) = length else {
        return match basis {
            None | Some(GrowthBasis::PerPeriod) => Ok((
                Conversion::None,
                format!(
                    "growth basis: {}; the timeline has no period length, so the rates are per \
                     period and used as given",
                    given_or_absent(basis)
                ),
            )),
            Some(b) => Err(at(format!(
                "`{}` needs a period length: give `frequency` or `period_months` in \
                 `/timeline/periods`, or use `per_period` with rates per period",
                basis_name(b)
            ))),
        };
    };
    let p = length.months();
    if p == 12 {
        return Ok((
            Conversion::None,
            format!(
                "growth basis: {}; with 12-month periods the three bases coincide, so the \
                 rates are used as given",
                given_or_absent(basis)
            ),
        ));
    }
    let Some(basis) = basis else {
        return Err(at(format!(
            "required: the timeline has {p}-month periods, so the growth rates must be read \
             either per period or as annual rates; {BASIS_FIX}"
        )));
    };
    if basis == GrowthBasis::PerPeriod {
        return Ok((
            Conversion::None,
            format!(
                "growth basis: `per_period` (given); the rates are per {p}-month period and \
                 used as given"
            ),
        ));
    }
    let compound = basis == GrowthBasis::AnnualCompound;
    if let Some(m) = length.per_year() {
        let compounding = Compounding::of(
            Frequency::from_months(p).ok_or(FairmarkError::Internal("no frequency of tenor"))?,
        );
        return Ok(if compound {
            (
                Conversion::CompoundRoot { m, compounding },
                format!(
                    "growth basis: `annual_compound` (given); each annual rate g becomes the rate \
                     per {p}-month period that compounds to g over a year, (1 + g)^(1/{m}) − 1, \
                     computed as the equivalent rate with {} (rates.convert) divided by {m}",
                    compounding.label()
                ),
            )
        } else {
            (
                Conversion::SimpleShare { m },
                format!(
                    "growth basis: `annual_simple` (given); each annual rate g becomes g / {m} \
                     per {p}-month period, so {m} periods compound to (1 + g/{m})^{m} − 1, \
                     more than g for a positive g"
                ),
            )
        });
    }
    if let Some(k) = length.years() {
        return Ok(if compound {
            (
                Conversion::CompoundPower { k },
                format!(
                    "growth basis: `annual_compound` (given); each annual rate g becomes \
                     (1 + g)^{k} − 1 per {p}-month period ({k} years, exact integer power)"
                ),
            )
        } else {
            (
                Conversion::SimpleMultiple { k },
                format!(
                    "growth basis: `annual_simple` (given); each annual rate g becomes g × {k} \
                     per {p}-month period ({k} years)"
                ),
            )
        });
    }
    Err(at(format!(
        "`{}` is not defined for {p}-month periods: an annual rate converts only when the \
         months per period divide 12 or are a multiple of 12; use `per_period` with rates per \
         {p}-month period, or a period length that divides 12 or is a multiple of 12",
        basis_name(basis)
    )))
}

fn basis_name(b: GrowthBasis) -> &'static str {
    match b {
        GrowthBasis::PerPeriod => "per_period",
        GrowthBasis::AnnualCompound => "annual_compound",
        GrowthBasis::AnnualSimple => "annual_simple",
    }
}

fn given_or_absent(b: Option<GrowthBasis>) -> String {
    match b {
        Some(b) => format!("`{}` (given)", basis_name(b)),
        None => "not given".to_owned(),
    }
}

/// The rate per period of a stated rate, with the formula substituted.
fn convert(g: Decimal, c: Conversion) -> Result<(Decimal, String)> {
    Ok(match c {
        Conversion::None => (g, format!("{g}")),
        Conversion::CompoundRoot { m, compounding } => {
            let nominal = rates::equivalent_rate(g, Compounding::Annual, compounding)?;
            (
                num::div(nominal, Decimal::from(m))?,
                format!("{nominal} / {m}, with {nominal} = {m} × ((1 + {g})^(1/{m}) − 1)"),
            )
        }
        Conversion::SimpleShare { m } => (num::div(g, Decimal::from(m))?, format!("{g} / {m}")),
        Conversion::CompoundPower { k } => (
            num::sub(
                num::one_plus(g)?
                    .checked_powi(i64::from(k))
                    .ok_or(FairmarkError::Overflow("power"))?,
                Decimal::ONE,
            )?,
            format!("(1 + {g})^{k} − 1"),
        ),
        Conversion::SimpleMultiple { k } => (num::mul(g, Decimal::from(k))?, format!("{g} × {k}")),
    })
}

/// (1 + g)^m − 1, the annual growth implied by a rate per period with m periods a year.
fn effective_annual(g: Decimal, m: u32) -> Result<Decimal> {
    num::sub(
        num::one_plus(g)?
            .checked_powi(i64::from(m))
            .ok_or(FairmarkError::Overflow("power"))?,
        Decimal::ONE,
    )
}

/// A stated rate of the rule: its value, the JSON pointer of the input it comes from, and how
/// it was obtained.
struct Stated {
    rate: Decimal,
    pointer: String,
    formula: String,
}

/// The stated rates of the rule, one per growth step (R11, R12).
fn stated_rates(rule: &GrowthRule, steps: u32, n: u32, base: BasePeriod) -> Result<Vec<Stated>> {
    match rule {
        GrowthRule::Constant(c) => Ok((0..steps)
            .map(|_| Stated {
                rate: c.growth,
                pointer: "/growth/constant/growth".to_owned(),
                formula: "as given".to_owned(),
            })
            .collect()),
        GrowthRule::Linear(l) => {
            let s = Decimal::from(steps);
            let (from, to) = (l.from_growth, l.to_growth);
            (1..=steps)
                .map(|j| {
                    // The arithmetic of the `linear_decline` stage of `dcf.fcff` (SPEC-0001
                    // Decision 4), so that the two agree bit for bit.
                    let step = num::div(num::mul(num::sub(to, from)?, Decimal::from(j))?, s)?;
                    Ok(Stated {
                        rate: num::add(from, step)?,
                        pointer: "/growth/linear".to_owned(),
                        formula: format!("{from} + ({to} − {from}) × {j} / {steps}"),
                    })
                })
                .collect()
        }
        GrowthRule::PerPeriod(p) => {
            if p.growth_rates.len() != steps as usize {
                let why = match base {
                    BasePeriod::Period0 => format!(
                        "under `period_0` the base value precedes period 1, so each of the {n} \
                         periods has a growth step"
                    ),
                    BasePeriod::Period1 => format!(
                        "under `period_1` the base value is the value of period 1, so the {n} \
                         periods have {steps} growth steps, into periods 2 to {n}"
                    ),
                };
                return Err(FairmarkError::InvalidInputAt {
                    pointer: "/growth/per_period/growth_rates".to_owned(),
                    detail: format!(
                        "expected {steps} rates, one per growth step, got {}: {why}; give \
                         {steps} rates",
                        p.growth_rates.len()
                    ),
                });
            }
            Ok(p.growth_rates
                .iter()
                .enumerate()
                .map(|(i, g)| Stated {
                    rate: *g,
                    pointer: format!("/growth/per_period/growth_rates/{i}"),
                    formula: "as given".to_owned(),
                })
                .collect())
        }
    }
}

/// The growth rule with its parameters, for `explain.conventions`.
fn rule_convention(rule: &GrowthRule, steps: u32) -> String {
    match rule {
        GrowthRule::Constant(c) => {
            format!("growth rule: constant, {} for every growth step", c.growth)
        }
        GrowthRule::Linear(l) => {
            format!(
            "growth rule: linear from {} to {} over {steps} growth steps, g_j = {} + ({} − {}) × \
             j / {steps}, j = 1..{steps}: {} is the growth before the first step, {} is reached \
             in the last step",
            l.from_growth, l.to_growth, l.from_growth, l.to_growth, l.from_growth, l.from_growth,
            l.to_growth
        )
        }
        GrowthRule::PerPeriod(p) => format!(
            "growth rule: per period, {} rates applied in order, one per growth step",
            p.growth_rates.len()
        ),
    }
}

/// A rate of −1 or lower, as given or after conversion (R17).
fn check_rate(g: Decimal, pointer: &str, converted: Option<&str>) -> Result<()> {
    if g > -Decimal::ONE {
        return Ok(());
    }
    let detail = match converted {
        None => format!(
            "is {g}; a growth rate must be greater than -1 (-100%); growth rates are decimal \
             fractions, 5% is 0.05"
        ),
        Some(how) => format!(
            "converted with {how} to {g} per period, which is -100% or lower; a growth rate per \
             period must be greater than -1 (-100%): give a higher rate, or `annual_compound`"
        ),
    };
    Err(FairmarkError::InvalidInputAt {
        pointer: pointer.to_owned(),
        detail,
    })
}

pub fn project(input: &ProjectInput) -> Result<(ProjectOutput, Explain)> {
    // Timeline (R14) and timing (R15).
    let (n, length, schedule): (u32, Option<PeriodLength>, Option<BuiltSchedule>) = match &input
        .timeline
    {
        Timeline::Periods(t) => {
            let length =
                frequency::period_length(t.frequency, t.period_months, "/timeline/periods", false)?;
            let n = t.number_of_periods;
            check_number_of_periods(
                n,
                "/timeline/periods/number_of_periods",
                "periods to project",
            )?;
            if input.timing.is_some() {
                return Err(FairmarkError::InvalidInputAt {
                    pointer: "/timing".to_owned(),
                    detail: "applies only to a `schedule` timeline, whose periods have \
                                 dates; remove it, or give the timeline as `schedule`"
                        .to_owned(),
                });
            }
            (n, length, None)
        }
        Timeline::Schedule(s) => {
            let built = dates::build_schedule(s, "/timeline/schedule")?;
            if input.timing.is_none() {
                return Err(FairmarkError::InvalidInputAt {
                    pointer: "/timing".to_owned(),
                    detail: "required with a `schedule` timeline: give `end_of_period` \
                                 (each value dated at the end date of its period, the first \
                                 day of the next period) or `beginning_of_period` (at its \
                                 start date)"
                        .to_owned(),
                });
            }
            (s.number_of_periods, Some(built.length), Some(built))
        }
    };

    // Growth steps (R10), stated rates (R11, R12), their domain (R17), the basis (R13).
    let steps = match input.base_period {
        BasePeriod::Period0 => n,
        BasePeriod::Period1 => n - 1,
    };
    let stated = stated_rates(&input.growth, steps, n, input.base_period)?;
    if let GrowthRule::Linear(l) = &input.growth {
        // The ends are checked by name, even when no step reaches them (s = 0); every step
        // lies between them.
        check_rate(l.from_growth, "/growth/linear/from_growth", None)?;
        check_rate(l.to_growth, "/growth/linear/to_growth", None)?;
    }
    for s in &stated {
        check_rate(s.rate, &s.pointer, None)?;
    }
    let (conversion, basis_convention) = resolve_basis(input.growth_basis, length)?;
    let how = input.growth_basis.map(basis_name).unwrap_or("per_period");
    let mut applied = Vec::with_capacity(stated.len());
    for s in &stated {
        let (rate, formula) = convert(s.rate, conversion)?;
        check_rate(rate, &s.pointer, Some(&format!("`{how}`")))?;
        applied.push((rate, formula));
    }
    let per_year = length.and_then(PeriodLength::per_year);

    // The recurrence, period by period (R9).
    let mut values = Vec::with_capacity(n as usize);
    let mut rows = Vec::with_capacity(n as usize);
    let mut v = input.base_value;
    let mut first_value_formula = None;
    let mut last_value_formula = String::new();
    let offset = n - steps; // 0 under period_0, 1 under period_1
    for k in 1..=n {
        let step = (k - 1).checked_sub(offset).map(|i| i as usize);
        let (stated_growth, growth_per_period, effective) = match step {
            Some(i) => {
                let (g, _) = applied[i];
                let previous = v;
                v = num::mul(v, num::one_plus(g)?)?;
                last_value_formula = format!("{previous} × (1 + {g})");
                if first_value_formula.is_none() {
                    first_value_formula = Some(last_value_formula.clone());
                }
                let effective = match per_year {
                    Some(m) => Some(effective_annual(g, m)?),
                    None => None,
                };
                (Some(stated[i].rate), Some(g), effective)
            }
            None => {
                first_value_formula = Some(format!("{v}, the base value (`period_1`)"));
                (None, None, None)
            }
        };
        let idx = (k - 1) as usize;
        rows.push(ProjectedPeriod {
            period: k,
            start_date: schedule.as_ref().map(|s| s.start_dates[idx]),
            end_date: schedule.as_ref().map(|s| s.end_dates[idx]),
            year_fraction: schedule
                .as_ref()
                .and_then(|s| s.year_fractions.as_ref().map(|y| y[idx])),
            stated_growth,
            growth_per_period,
            effective_annual_growth: effective,
            value: v,
        });
        values.push(v);
    }
    let dated_values = match (&schedule, input.timing) {
        (Some(s), Some(timing)) => Some(
            values
                .iter()
                .enumerate()
                .map(|(i, amount)| DatedCashFlow {
                    date: match timing {
                        ValueTiming::EndOfPeriod => s.end_dates[i],
                        ValueTiming::BeginningOfPeriod => s.start_dates[i],
                    },
                    amount: *amount,
                })
                .collect(),
        ),
        _ => None,
    };

    // The audit trail (R19), bounded: the per-period numbers are in the rows.
    let mut ex = Explain::new(
        "series.project",
        "V_k = V_(k−1) × (1 + g_k), period by period, k = 1..n; g_k = growth per period into \
         period k",
    );
    ex.convention(match input.base_period {
        BasePeriod::Period0 => format!(
            "base period: `period_0`, the base value {} is the value before period 1; V_1 = V_0 \
             × (1 + g_1); {steps} growth steps",
            input.base_value
        ),
        BasePeriod::Period1 => format!(
            "base period: `period_1`, the base value {} is the value of period 1; V_1 = V_0; \
             {steps} growth steps, into periods 2 to {n}",
            input.base_value
        ),
    });
    ex.convention(rule_convention(&input.growth, steps));
    ex.convention(basis_convention);
    match (length, per_year) {
        (None, _) => ex.convention(format!(
            "timeline: {n} periods without a period length or dates"
        )),
        (Some(l), _) if schedule.is_none() => ex.convention(format!(
            "timeline: {n} periods without dates; {}",
            l.describe()
        )),
        _ => {}
    }
    if let Some(m) = per_year {
        let first_last = match (applied.first(), applied.last()) {
            (Some((a, _)), Some((b, _))) if applied.len() > 1 && a != b => format!(
                "{} for the first growth step, {} for the last; every row reports its own",
                effective_annual(*a, m)?,
                effective_annual(*b, m)?
            ),
            (Some((a, _)), _) => format!("{}", effective_annual(*a, m)?),
            _ => "none (no growth step)".to_owned(),
        };
        ex.convention(format!(
            "effective annual growth implied by the rate per period, (1 + g)^{m} − 1: {first_last}"
        ));
    }
    if input.base_value.is_sign_negative() && !input.base_value.is_zero() {
        ex.convention(format!(
            "warning: the base value {} is negative, so a positive growth makes the values more \
             negative; check the sign of the series",
            input.base_value
        ));
    }
    ex.convention("values: computed with 28 to 29 significant digits, not rounded");
    if let Some(s) = &schedule {
        dates::schedule_explain(s, &mut ex)?;
        ex.convention(match input.timing {
            Some(ValueTiming::EndOfPeriod) => {
                "value timing: end of period, each value dated at the end date of its period, \
                 which is the first day of the next period"
            }
            _ => {
                "value timing: beginning of period, each value dated at the start date of its \
                  period"
            }
        });
    }
    ex.step("Base value V_0", "as given", input.base_value);
    if let Some(s) = stated.first() {
        ex.step(
            format!("Stated growth, first growth step (`{}`)", s.pointer),
            s.formula.clone(),
            s.rate,
        );
        if conversion != Conversion::None {
            ex.step(
                "Growth per period, first step",
                applied[0].1.clone(),
                applied[0].0,
            );
        }
    }
    ex.step(
        "Value of period 1",
        first_value_formula.unwrap_or_default(),
        values[0],
    );
    if n > 1 {
        ex.step(
            format!("Value of period {n}"),
            last_value_formula,
            values[(n - 1) as usize],
        );
    }

    Ok((
        ProjectOutput {
            periods: rows,
            values,
            dated_values,
            months_per_period: length.map(PeriodLength::months),
            periods_per_year: per_year,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- method registration

/// `series.project`: see the methodology page.
pub struct Project;

impl Method for Project {
    const ID: &'static str = "series.project";
    const TITLE: &'static str = "Value series projected on a timeline";
    const DESCRIPTION: &'static str =
        "Values of periods 1 to n grown from a base value period by period, V_k = V_(k−1) × \
        (1 + g_k), with a constant, linear or per-period growth rule; annual rates converted to \
        the period length by a stated basis (compound or proportional). The timeline is a \
        number of periods or a `dates.schedule` definition, whose periods run from and \
        including the start date to but excluding the end date; dated values feed `tvm.xnpv`.";
    const VERSION: &'static str = "1";
    type Input = ProjectInput;
    type Output = ProjectOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        project(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rates::ConvertInput;
    use serde_json::{json, Value};
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn run(input: Value) -> Result<(ProjectOutput, Explain)> {
        project(&serde_json::from_value(input).unwrap())
    }

    fn input(growth: Value, basis: Option<&str>, timeline: Value) -> Value {
        let mut v = json!({"base_value": "1000", "base_period": "period_0", "growth": growth,
                           "timeline": timeline});
        if let Some(b) = basis {
            v["growth_basis"] = json!(b);
        }
        v
    }

    fn periods(n: u32, length: Option<(&str, Value)>) -> Value {
        let mut t = json!({"number_of_periods": n});
        if let Some((k, v)) = length {
            t[k] = v;
        }
        json!({"periods": t})
    }

    /// AC9: the rate per period of `annual_compound` is `rates.convert` to the compounding of
    /// the period, divided by m; `annual_simple` is g / m; `period_months` equals the named
    /// frequency of its tenor bit for bit.
    #[test]
    fn annual_bases_convert_as_stated_and_named_equals_months() {
        let g = d("0.12");
        for f in Frequency::ALL {
            if f == Frequency::Annual {
                continue;
            }
            let m = f.per_year();
            for basis in ["annual_compound", "annual_simple"] {
                let growth = json!({"constant": {"growth": "0.12"}});
                let (named, _) = run(input(
                    growth.clone(),
                    Some(basis),
                    periods(m, Some(("frequency", json!(f.name())))),
                ))
                .unwrap();
                let (months, _) = run(input(
                    growth,
                    Some(basis),
                    periods(m, Some(("period_months", json!(f.months())))),
                ))
                .unwrap();
                assert_eq!(
                    serde_json::to_string(&named).unwrap(),
                    serde_json::to_string(&months).unwrap(),
                    "{f:?} {basis}"
                );
                let applied = named.periods[0].growth_per_period.unwrap();
                if basis == "annual_compound" {
                    let (c, _) = rates::convert(&ConvertInput {
                        rate: g,
                        from: Compounding::Annual,
                        to: Compounding::of(f),
                    })
                    .unwrap();
                    assert_eq!(applied, c.rate / Decimal::from(m), "{f:?}");
                    let last = named.values[(m - 1) as usize];
                    let error = ((last - d("1120")) / d("1120")).abs();
                    assert!(error < d("1e-20"), "{f:?}: {last}");
                } else {
                    assert_eq!(applied, g / Decimal::from(m), "{f:?}");
                    let exact = d("1000") * (Decimal::ONE + g / Decimal::from(m)).powi(m as i64);
                    assert_eq!(named.values[(m - 1) as usize], exact, "{f:?}");
                }
            }
        }
    }

    /// AC9: the refusals and acceptances of the growth basis, each naming `/growth_basis` with
    /// the fix.
    #[test]
    fn growth_basis_rules() {
        let growth = json!({"constant": {"growth": "0.12"}});
        for months in [5, 18, 7] {
            for basis in ["annual_compound", "annual_simple"] {
                let e = run(input(
                    growth.clone(),
                    Some(basis),
                    periods(3, Some(("period_months", json!(months)))),
                ))
                .unwrap_err();
                assert_eq!(e.code(), "FM-E001");
                let m = e.to_string();
                assert!(m.starts_with("invalid input at `/growth_basis`"), "{m}");
                assert!(m.contains("use `per_period`"), "{m}");
            }
            run(input(
                growth.clone(),
                Some("per_period"),
                periods(3, Some(("period_months", json!(months)))),
            ))
            .unwrap();
        }
        let e = run(input(
            growth.clone(),
            None,
            periods(12, Some(("frequency", json!("monthly")))),
        ))
        .unwrap_err();
        let m = e.to_string();
        assert!(
            m.starts_with("invalid input at `/growth_basis`: required"),
            "{m}"
        );
        assert!(
            m.contains("`annual_compound`") && m.contains("`annual_simple`"),
            "{m}"
        );
        let e = run(input(
            growth.clone(),
            Some("annual_compound"),
            periods(3, None),
        ))
        .unwrap_err();
        let m = e.to_string();
        assert!(m.starts_with("invalid input at `/growth_basis`"), "{m}");
        assert!(m.contains("needs a period length"), "{m}");
        for length in [("frequency", json!("annual")), ("period_months", json!(12))] {
            let (out, ex) = run(input(growth.clone(), None, periods(2, Some(length)))).unwrap();
            assert_eq!(out.values, [d("1120"), d("1254.4")]);
            assert!(ex
                .conventions
                .iter()
                .any(|c| c.contains("with 12-month periods the three bases coincide")));
        }
        // 24 months: exact integer power and multiple.
        let (out, _) = run(input(
            growth.clone(),
            Some("annual_compound"),
            periods(2, Some(("period_months", json!(24)))),
        ))
        .unwrap();
        assert_eq!(out.periods[0].growth_per_period, Some(d("0.2544")));
        assert_eq!(out.values, [d("1254.4"), d("1573.51936")]);
        assert_eq!(
            (out.months_per_period, out.periods_per_year),
            (Some(24), None)
        );
        assert!(out.periods[0].effective_annual_growth.is_none());
        let (out, _) = run(input(
            growth,
            Some("annual_simple"),
            periods(2, Some(("period_months", json!(24)))),
        ))
        .unwrap();
        assert_eq!(out.periods[0].growth_per_period, Some(d("0.24")));
        assert_eq!(out.values, [d("1240"), d("1537.6")]);
    }

    /// AC8, AC10, AC12 and AC13: every error names its field by a JSON pointer and states the
    /// fix.
    #[test]
    fn every_error_names_its_field_and_the_fix() {
        let sched = json!({"start_date": "2026-01-01", "frequency": "quarterly",
                           "number_of_periods": 4});
        let cases: Vec<(Value, &str, &str, &str)> = vec![
            (
                json!({"base_value": "500", "base_period": "period_1",
                       "growth": {"per_period": {"growth_rates": ["0.1", "0.1", "0.1", "0.1"]}},
                       "timeline": periods(4, None)}),
                "FM-E001",
                "invalid input at `/growth/per_period/growth_rates`: expected 3 rates, one per \
                 growth step, got 4: under `period_1`",
                "give 3 rates",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"constant": {"growth": "-1"}}, "timeline": periods(3, None)}),
                "FM-E001",
                "invalid input at `/growth/constant/growth`: is -1",
                "5% is 0.05",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"per_period": {"growth_rates": ["0.1", "-1.5", "0.1"]}},
                       "timeline": periods(3, None)}),
                "FM-E001",
                "invalid input at `/growth/per_period/growth_rates/1`: is -1.5",
                "5% is 0.05",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"constant": {"growth": "-1"}}, "growth_basis": "annual_compound",
                       "timeline": periods(12, Some(("frequency", json!("monthly"))))}),
                "FM-E001",
                "invalid input at `/growth/constant/growth`: is -1",
                "5% is 0.05",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"linear": {"from_growth": "-2", "to_growth": "0.1"}},
                       "timeline": periods(3, None)}),
                "FM-E001",
                "invalid input at `/growth/linear/from_growth`: is -2",
                "5% is 0.05",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"constant": {"growth": "-0.6"}}, "growth_basis": "annual_simple",
                       "timeline": periods(2, Some(("period_months", json!(24))))}),
                "FM-E001",
                "invalid input at `/growth/constant/growth`: converted with `annual_simple` to \
                 -1.2 per period",
                "give a higher rate, or `annual_compound`",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"constant": {"growth": "0.1"}},
                       "timeline": periods(4, None), "timing": "end_of_period"}),
                "FM-E001",
                "invalid input at `/timing`: applies only to a `schedule` timeline",
                "remove it",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"constant": {"growth": "0.1"}}, "growth_basis": "per_period",
                       "timeline": {"schedule": sched}}),
                "FM-E001",
                "invalid input at `/timing`: required with a `schedule` timeline",
                "give `end_of_period`",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"constant": {"growth": "0.1"}},
                       "timeline": {"periods": {"number_of_periods": 4,
                                                "frequency": "quarterly", "period_months": 3}}}),
                "FM-E001",
                "invalid input: `/timeline/periods/frequency` and `/timeline/periods/period_months` \
                 are both given",
                "give exactly one",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"constant": {"growth": "0.1"}},
                       "timeline": periods(0, None)}),
                "FM-E001",
                "invalid input at `/timeline/periods/number_of_periods`: must be between 1",
                "give the number of periods",
            ),
            (
                json!({"base_value": "1", "base_period": "period_0",
                       "growth": {"constant": {"growth": "0.1"}}, "growth_basis": "per_period",
                       "timeline": {"schedule": {"start_date": "2199-06-30",
                                                 "frequency": "annual", "number_of_periods": 2}},
                       "timing": "end_of_period"}),
                "FM-E009",
                "invalid date at `/timeline/schedule/number_of_periods`: period 1 would end on \
                 2200-06-30",
                "give a shorter period length",
            ),
        ];
        for (json, code, start, fix) in cases {
            let e = run(json).unwrap_err();
            assert_eq!(e.code(), code, "{e}");
            let m = e.to_string();
            assert!(m.starts_with(start), "{m}\n  does not start with\n{start}");
            assert!(m.contains(fix), "{m}\n  lacks the fix: {fix}");
            assert!(m.contains("`/"), "{m}: no JSON pointer");
        }
        let e = run(json!({"base_value": "79228162514264337593543950335",
                           "base_period": "period_0", "growth": {"constant": {"growth": "1"}},
                           "timeline": periods(2, None)}))
        .unwrap_err();
        assert_eq!(e.code(), "FM-E008");
    }

    /// AC10: the dates are those of `dates.schedule`; `dated_values` is the `cash_flows` of
    /// `tvm.xnpv` and `tvm.xirr`, and the chain gives the XNPV of the flows typed by hand.
    #[test]
    fn schedule_timeline_dates_and_the_xnpv_chain() {
        let sched = json!({"start_date": "2026-01-01", "frequency": "quarterly",
                           "number_of_periods": 4, "day_count": "act_360"});
        let (dates_out, _) =
            crate::dates::schedule(&serde_json::from_value(sched.clone()).unwrap()).unwrap();
        let base = |timing: &str| {
            json!({"base_value": "300", "base_period": "period_0",
                   "growth": {"constant": {"growth": "0.02"}}, "growth_basis": "per_period",
                   "timeline": {"schedule": sched}, "timing": timing})
        };
        let (end, _) = run(base("end_of_period")).unwrap();
        let (begin, _) = run(base("beginning_of_period")).unwrap();
        for (k, row) in end.periods.iter().enumerate() {
            assert_eq!(row.start_date, Some(dates_out.start_dates[k]));
            assert_eq!(row.end_date, Some(dates_out.end_dates[k]));
            assert_eq!(
                row.year_fraction,
                Some(dates_out.year_fractions.as_ref().unwrap()[k])
            );
        }
        let dated: Vec<String> = end
            .dated_values
            .as_ref()
            .unwrap()
            .iter()
            .map(|f| f.date.to_string())
            .collect();
        assert_eq!(
            dated,
            ["2026-04-01", "2026-07-01", "2026-10-01", "2027-01-01"]
        );
        let begin_dates: Vec<Date> = begin
            .dated_values
            .as_ref()
            .unwrap()
            .iter()
            .map(|f| f.date)
            .collect();
        assert_eq!(begin_dates, dates_out.start_dates);

        // Contract: the JSON of `dated_values` is accepted as `cash_flows` by both methods.
        let mut flows = serde_json::to_value(end.dated_values.as_ref().unwrap()).unwrap();
        flows
            .as_array_mut()
            .unwrap()
            .insert(0, json!({"date": "2026-01-01", "amount": "-1000"}));
        let chained = crate::run(
            "tvm.xnpv",
            &json!({"rate": "0.10", "cash_flows": flows}).to_string(),
        )
        .unwrap();
        crate::run("tvm.xirr", &json!({"cash_flows": flows}).to_string()).unwrap();
        let by_hand = crate::run(
            "tvm.xnpv",
            &json!({"rate": "0.10", "cash_flows": [
                {"date": "2026-01-01", "amount": "-1000"},
                {"date": "2026-04-01", "amount": "306"},
                {"date": "2026-07-01", "amount": "312.12"},
                {"date": "2026-10-01", "amount": "318.3624"},
                {"date": "2027-01-01", "amount": "324.729648"}]})
            .to_string(),
        )
        .unwrap();
        assert_eq!(chained["value"]["xnpv"], by_hand["value"]["xnpv"]);
    }

    /// R18: a negative base value is projected, with a warning.
    #[test]
    fn negative_base_value_is_projected_with_a_warning() {
        let (out, ex) = run(json!({"base_value": "-200", "base_period": "period_0",
                                   "growth": {"constant": {"growth": "0.10"}},
                                   "timeline": periods(3, None)}))
        .unwrap();
        assert_eq!(out.values, [d("-220"), d("-242"), d("-266.2")]);
        assert!(ex.conventions.iter().any(|c| c.starts_with(
            "warning: the base value -200 is negative, so a positive growth makes the values \
             more negative"
        )));
    }

    /// AC13 and R19: every item of `explain`, with a number of steps independent of n.
    #[test]
    fn explain_is_complete_and_bounded() {
        let mk = |n: u32| {
            run(input(
                json!({"linear": {"from_growth": "0.12", "to_growth": "0.06"}}),
                Some("annual_compound"),
                json!({"schedule": {"start_date": "2025-12-31", "frequency": "quarterly",
                                    "number_of_periods": n, "day_count": "act_365_fixed"}}),
            ))
        };
        let with_timing = |n: u32| {
            let mut v = input(
                json!({"linear": {"from_growth": "0.12", "to_growth": "0.06"}}),
                Some("annual_compound"),
                json!({"schedule": {"start_date": "2025-12-31", "frequency": "quarterly",
                                    "number_of_periods": n, "day_count": "act_365_fixed"}}),
            );
            v["timing"] = json!("end_of_period");
            run(v).unwrap()
        };
        assert!(mk(4).is_err()); // no timing with a schedule
        let (out, ex) = with_timing(4);
        assert_eq!(
            ex.formula,
            "V_k = V_(k−1) × (1 + g_k), period by period, k = 1..n; g_k = growth per period \
             into period k"
        );
        let conventions = ex.conventions.join("\n");
        for item in [
            "base period: `period_0`, the base value 1000 is the value before period 1",
            "growth rule: linear from 0.12 to 0.06 over 4 growth steps",
            "growth basis: `annual_compound` (given); each annual rate g becomes the rate per \
             3-month period that compounds to g over a year, (1 + g)^(1/4) − 1",
            "effective annual growth implied by the rate per period, (1 + g)^4 − 1",
            "from and including its start date to but excluding its end date",
            "end-of-month rule",
            "value timing: end of period",
            "values: computed with 28 to 29 significant digits, not rounded",
        ] {
            assert!(conventions.contains(item), "{item} not in\n{conventions}");
        }
        let labels: Vec<&str> = ex.steps.iter().map(|s| s.label.as_str()).collect();
        for label in [
            "Base value V_0",
            "Stated growth, first growth step (`/growth/linear`)",
            "Growth per period, first step",
            "Value of period 1",
            "Value of period 4",
            "Year fraction of period 1, 2025-12-31 to 2026-03-31",
        ] {
            assert!(labels.contains(&label), "{label} not in {labels:?}");
        }
        let first = ex
            .steps
            .iter()
            .find(|s| s.label == "Value of period 1")
            .unwrap();
        assert_eq!(first.value, out.values[0]);
        assert_eq!(
            first.formula,
            format!("1000 × (1 + {})", out.periods[0].growth_per_period.unwrap())
        );
        let stated = ex
            .steps
            .iter()
            .find(|s| s.label.starts_with("Stated growth"))
            .unwrap();
        assert_eq!(stated.formula, "0.12 + (0.06 − 0.12) × 1 / 4");
        assert_eq!(with_timing(400).1.steps.len(), ex.steps.len());

        // Without a conversion, the conversion step is absent; with period_1, period 1 is the
        // base value.
        let (out, ex) = run(json!({"base_value": "100", "base_period": "period_1",
                                   "growth": {"constant": {"growth": "0.05"}},
                                   "timeline": periods(3, None)}))
        .unwrap();
        assert_eq!(out.values, [d("100"), d("105"), d("110.25")]);
        assert!(out.periods[0].stated_growth.is_none());
        assert!(out.periods[0].growth_per_period.is_none());
        assert!(!ex
            .steps
            .iter()
            .any(|s| s.label.starts_with("Growth per period")));
        assert!(ex.conventions.iter().any(|c| c.contains(
            "the timeline has no period length, so the rates are per period and used as given"
        )));
        let json = serde_json::to_value(&out).unwrap();
        assert!(json.get("dated_values").is_none());
        assert!(json.get("months_per_period").is_none());
    }
}
