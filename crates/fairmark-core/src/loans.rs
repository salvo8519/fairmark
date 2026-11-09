//! Loans: instalment, number of instalments and amortisation schedules (SPEC-0009).
//!
//! Conventions:
//! * the rate is given in exactly one of two forms: an annual borrowing rate with its
//!   compounding and the payment frequency (form a), or a rate per payment period (form b);
//! * amounts are in currency units, rates are decimal fractions;
//! * the periodic rate of form (a) is the equivalent rate with compounding equal to the payment
//!   frequency (`rates.convert`) divided by the payments per year;
//! * the annuity factor is the one of `tvm.annuity` (no second implementation).

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps, RoundingStrategy};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{rate_field, FairmarkError, Found, Reason, Result};
use crate::explain::Explain;
use crate::frequency::Frequency;
use crate::method::Method;
use crate::num;
use crate::rates::{self, Compounding, ConvertInput};
use crate::tvm::{self, AnnuityTiming, MAX_PERIODS};

/// An exact number of periods within this distance of an integer is taken as that integer
/// (`loans.number_of_instalments`): the logarithms are series approximations.
const WHOLE_PERIOD_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 20); // 1e-20
/// The number of instalments is confirmed by the balance recurrence: an amount owed within
/// this fraction of the principal above the instalment is paid by that instalment.
const BALANCE_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 20); // 1e-20

// ---------------------------------------------------------------- shared inputs

// The values take their periods a year from the named frequencies of the engine
// (`frequency::Frequency`, SPEC-0010 R20).
/// How often instalments are paid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PaymentFrequency {
    /// One instalment a year.
    Annual,
    /// Two instalments a year.
    SemiAnnual,
    /// Four instalments a year.
    Quarterly,
    /// Twelve instalments a year.
    Monthly,
    /// Six instalments a year, every two months (not twice a month).
    Bimonthly,
    /// Three instalments a year, every four months.
    FourMonthly,
}

impl PaymentFrequency {
    /// The named frequency of the payments.
    pub(crate) fn frequency(self) -> Frequency {
        match self {
            Self::Annual => Frequency::Annual,
            Self::SemiAnnual => Frequency::SemiAnnual,
            Self::Quarterly => Frequency::Quarterly,
            Self::Monthly => Frequency::Monthly,
            Self::Bimonthly => Frequency::Bimonthly,
            Self::FourMonthly => Frequency::FourMonthly,
        }
    }

    fn per_year(self) -> u32 {
        self.frequency().per_year()
    }

    /// The compounding whose period is the payment period.
    pub(crate) fn compounding(self) -> Compounding {
        Compounding::of(self.frequency())
    }

    fn name(self) -> &'static str {
        self.frequency().name()
    }
}

/// How a half is rounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoundingMode {
    /// A half is rounded away from zero (0.125 to 0.13).
    HalfUp,
    /// A half is rounded to the even digit (0.125 to 0.12, 0.135 to 0.14).
    HalfEven,
}

impl RoundingMode {
    fn name(self) -> &'static str {
        match self {
            Self::HalfUp => "half_up",
            Self::HalfEven => "half_even",
        }
    }
}

/// Rounding of the amount that the schedule type keeps constant (SPEC-0009 R19).
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
// Both fields are required: `mode` is optional only in Rust, so that its absence is reported with
// a message naming `rounding.mode` instead of the generic missing-field text.
#[schemars(extend("required" = ["decimal_places", "mode"]))]
pub struct Rounding {
    /// Number of decimal places, from 0 to 6 (2 rounds to the cent).
    pub decimal_places: u32,
    /// `half_up` or `half_even`; required, there is no default mode. A missing mode is
    /// reported as `FM-E001` naming `rounding.mode` and its allowed values.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mode: Option<RoundingMode>,
}

impl Rounding {
    fn validate(&self) -> Result<()> {
        if self.mode.is_none() {
            return Err(FairmarkError::InvalidInput(
                "`rounding.mode` is required when `rounding` is given: `half_up` or `half_even` \
                 (there is no default mode); omit `rounding` for no rounding"
                    .to_owned(),
            )
            .at("/rounding/mode", Reason::Missing)
            .expected(["half_up", "half_even"])
            .fix("give `mode`, or omit `rounding` for no rounding"));
        }
        if self.decimal_places > 6 {
            return Err(FairmarkError::InvalidInput(format!(
                "`rounding.decimal_places` must be between 0 and 6 (got {})",
                self.decimal_places
            ))
            .at("/rounding/decimal_places", Reason::Range)
            .expected(["a whole number from 0 to 6"]));
        }
        Ok(())
    }

    fn apply(&self, x: Decimal) -> Decimal {
        // `validate` has checked that the mode is given; `half_even` is never a silent default.
        let strategy = match self.mode.unwrap_or(RoundingMode::HalfEven) {
            RoundingMode::HalfUp => RoundingStrategy::MidpointAwayFromZero,
            RoundingMode::HalfEven => RoundingStrategy::MidpointNearestEven,
        };
        x.round_dp_with_strategy(self.decimal_places, strategy)
    }

    fn describe(&self) -> String {
        format!(
            "{} decimal places, {}",
            self.decimal_places,
            self.mode.map_or("no mode", RoundingMode::name)
        )
    }
}

/// The rate fields shared by the loan inputs (R1).
struct RateForm {
    rate: Option<Decimal>,
    compounding: Option<Compounding>,
    payment_frequency: Option<PaymentFrequency>,
    periodic_rate: Option<Decimal>,
}

/// R1, R1a, R3: one rate form, complete, and a periodic rate above -100%; the checks that need
/// no conversion, so that they are reported with the other input errors (ADR 0014 G5).
fn check_rate_form(form: &RateForm) -> Result<()> {
    let form_a = [
        ("rate", form.rate.is_some()),
        ("compounding", form.compounding.is_some()),
        ("payment_frequency", form.payment_frequency.is_some()),
    ];
    let given: Vec<&str> = form_a.iter().filter(|f| f.1).map(|f| f.0).collect();
    if let Some(i) = form.periodic_rate {
        if !given.is_empty() {
            let mut pointers = vec!["/periodic_rate".to_owned()];
            pointers.extend(given.iter().map(|g| format!("/{g}")));
            return Err(FairmarkError::InvalidInput(format!(
                "give the rate in one form only: `periodic_rate` (form b) conflicts with `{}` \
                 (form a); remove `periodic_rate` or remove `rate`, `compounding` and \
                 `payment_frequency`",
                given.join("`, `")
            ))
            .at("/periodic_rate", Reason::Conflict)
            .between(pointers)
            .fix(
                "remove `periodic_rate`, or remove `rate`, `compounding` and `payment_frequency`",
            ));
        }
        if i <= -Decimal::ONE {
            return Err(rate_field("/periodic_rate", None)(
                FairmarkError::RateOutOfRange(i),
            ));
        }
        return Ok(());
    }
    if form.rate.is_some() && form.compounding.is_some() && form.payment_frequency.is_some() {
        return Ok(());
    }
    if given.is_empty() {
        return Err(FairmarkError::InvalidInput(
            "no rate: give `periodic_rate` (form b), or `rate` with `compounding` and \
             `payment_frequency` (form a)"
                .to_owned(),
        )
        .at("/periodic_rate", Reason::Missing)
        .between(["/periodic_rate", "/rate"])
        .fix("give `periodic_rate`, or `rate` with `compounding` and `payment_frequency`"));
    }
    let missing: Vec<&str> = form_a.iter().filter(|f| !f.1).map(|f| f.0).collect();
    Err(FairmarkError::InvalidInput(format!(
        "rate form (a) needs `rate`, `compounding` and `payment_frequency`; missing `{}`: \
         add it, or give `periodic_rate` (form b) instead",
        missing.join("`, `")
    ))
    .at(format!("/{}", missing[0]), Reason::Missing)
    .between(missing.iter().map(|m| format!("/{m}")))
    .fix(format!(
        "add `{}`, or give `periodic_rate` instead",
        missing.join("`, `")
    )))
}

/// Reports the rate form and returns the periodic rate (R1, R1a, R3, R5); the form was checked
/// by [`check_rate_form`].
fn periodic_rate(form: &RateForm, ex: &mut Explain) -> Result<Decimal> {
    check_rate_form(form)?;
    if let Some(i) = form.periodic_rate {
        ex.convention("rate form (b): rate per payment period, as given");
        ex.convention(format!("periodic rate: {i}"));
        ex.step("Periodic rate (given)", format!("{i}"), i);
        return Ok(i);
    }
    let (Some(rate), Some(compounding), Some(frequency)) =
        (form.rate, form.compounding, form.payment_frequency)
    else {
        return Err(FairmarkError::Internal("rate form not checked"));
    };
    let target = frequency.compounding();
    // The rule is that of the given compounding: the conversion raises the rate's error.
    let at_rate = rate_field("/rate", compounding.per_year());
    let (converted, conversion) = rates::convert(&ConvertInput {
        rate,
        from: compounding,
        to: target,
    })
    .map_err(&at_rate)?;
    for step in conversion.steps {
        ex.step(
            format!("Rate conversion: {}", step.label.to_lowercase()),
            step.formula,
            step.value,
        );
    }
    let per_year = frequency.per_year();
    let i = num::div(converted.rate, Decimal::from(per_year))?;
    if i <= -Decimal::ONE {
        // Not reached: a rate that passed the conversion has a periodic rate above -1, so
        // reaching this is a defect, not an input error.
        return Err(FairmarkError::Internal(
            "periodic rate at or below -1 after a valid rate conversion",
        ));
    }
    ex.step(
        "Periodic rate",
        format!("{} / {per_year}", converted.rate),
        i,
    );
    ex.convention(format!(
        "rate form (a): annual borrowing rate {rate} converted to the rate per payment period"
    ));
    ex.convention(format!("source compounding: {}", compounding.label()));
    ex.convention(format!(
        "payment frequency: {} ({per_year} instalments a year); periodic rate = equivalent rate \
         with {} / {per_year}",
        frequency.name(),
        target.label()
    ));
    ex.convention(format!("periodic rate: {i}"));
    Ok(i)
}

fn timing_convention(timing: AnnuityTiming) -> &'static str {
    match timing {
        AnnuityTiming::Ordinary => {
            "payments in arrears: instalment k at the end of period k (time k)"
        }
        AnnuityTiming::Due => {
            "payments in advance: instalment k at the beginning of period k (time k − 1)"
        }
    }
}

fn check_principal(principal: Decimal) -> Result<()> {
    if principal <= Decimal::ZERO {
        return Err(FairmarkError::InvalidInput(format!(
            "`principal` must be greater than 0 (got {principal})"
        ))
        .at("/principal", Reason::Range)
        .expected(["an amount greater than 0"]));
    }
    Ok(())
}

fn check_instalments(n: u32) -> Result<()> {
    if n == 0 || n > MAX_PERIODS {
        return Err(FairmarkError::InvalidInput(format!(
            "`number_of_instalments` must be between 1 and {MAX_PERIODS} (got {n})"
        ))
        .at("/number_of_instalments", Reason::Range)
        .expected([format!("a whole number from 1 to {MAX_PERIODS}")]));
    }
    Ok(())
}

/// The balloon amount with its convention; `FM-E020` outside 0 ≤ B < P (R7, R17).
fn balloon(principal: Decimal, balloon: Option<Decimal>, ex: &mut Explain) -> Result<Decimal> {
    let b = balloon.unwrap_or_default();
    if b < Decimal::ZERO || b >= principal {
        return Err(FairmarkError::BalloonOutOfRange(format!(
            "`balloon_amount` must be at least 0 and below `principal` ({principal}); got {b}"
        ))
        .at("/balloon_amount", Reason::Range)
        .between(["/balloon_amount", "/principal"]));
    }
    ex.convention(match balloon {
        Some(v) => format!("balloon amount: {v} (given), paid with the last instalment"),
        None => "balloon amount: 0 (default)".to_owned(),
    });
    Ok(b)
}

/// The constant instalment of R6, unrounded, with its steps: the present value at the periodic
/// rate `i` of `n` instalments and of `b` at the end of period `n` equals `p`.
fn level_instalment(
    p: Decimal,
    b: Decimal,
    i: Decimal,
    n: u32,
    timing: AnnuityTiming,
    ex: &mut Explain,
) -> Result<Decimal> {
    if i.is_zero() {
        let a = num::div(num::sub(p, b)?, Decimal::from(n))?;
        ex.step(
            "Instalment (rate zero, limit form)",
            format!("({p} − {b}) / {n}"),
            a,
        );
        return Ok(a);
    }
    let financed = if b.is_zero() {
        p
    } else {
        let v_n = tvm::discount_factor(i, Decimal::from(n))?;
        ex.step(
            "Discount factor of period n",
            format!("(1 + {i})^−{n}"),
            v_n,
        );
        let pv_b = num::mul(b, v_n)?;
        ex.step(
            "Present value of the balloon amount",
            format!("{b} × {v_n}"),
            pv_b,
        );
        let financed = num::sub(p, pv_b)?;
        ex.step(
            "Principal less the present value of the balloon amount",
            format!("{p} − {pv_b}"),
            financed,
        );
        financed
    };
    let (factor, steps) = tvm::annuity_factor(i, Decimal::ZERO, n, timing)?;
    ex.steps.extend(tvm::level_factor_steps(steps, i, n));
    let a = num::div(financed, factor)?;
    ex.step("Instalment", format!("{financed} / {factor}"), a);
    Ok(a)
}

fn rounding_convention(rounding: Option<&Rounding>, what: &str) -> String {
    match rounding {
        Some(r) => format!("rounding: {}, applied to the {what}", r.describe()),
        None => format!("rounding: none (default); the {what} is exact to 28 significant digits"),
    }
}

/// Below this magnitude `ln(1 + x) / x` is summed from its series instead of dividing the
/// logarithm of `1 + x` by `x`: decimals keep at most 28 decimal places, so a small `x` (a rate
/// of 1e-9 per period) would leave the logarithm with few significant digits.
const LN_1P_SERIES: Decimal = Decimal::from_parts(1, 0, 0, false, 2); // 0.01

/// `ln(1 + x) / x` for `x > −1`, `x ≠ 0`, with full relative precision: for |x| < 0.01 the
/// series `2 / (2 + x) × (1 + y²/3 + y⁴/5 + …)`, `y = x / (2 + x)`, summed until a term
/// vanishes at 28 digits (a few terms), so the result is near 1 and keeps 28 significant
/// digits.
fn ln_1p_over_x(x: Decimal) -> Result<Decimal> {
    if x.abs() >= LN_1P_SERIES {
        let ln = num::one_plus(x)?
            .checked_ln()
            .ok_or(FairmarkError::Overflow("logarithm"))?;
        return num::div(ln, x);
    }
    let two_x = num::add(Decimal::TWO, x)?;
    let y = num::div(x, two_x)?;
    let y2 = num::mul(y, y)?;
    let (mut power, mut sum) = (Decimal::ONE, Decimal::ONE);
    for k in 1..=64u32 {
        power = num::mul(power, y2)?;
        let term = num::div(power, Decimal::from(2 * k + 1))?;
        if term.is_zero() {
            break;
        }
        sum = num::add(sum, term)?;
    }
    num::div(num::mul(Decimal::TWO, sum)?, two_x)
}

// ---------------------------------------------------------------- loans.instalment

/// Input of `loans.instalment`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InstalmentInput {
    /// Amount borrowed, in currency units, greater than zero.
    pub principal: Decimal,
    /// Annual borrowing rate, decimal fraction (0.05 = 5% a year), quoted with `compounding`.
    /// Rate form (a): give it with `compounding` and `payment_frequency`, or give
    /// `periodic_rate` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
    /// Compounding of `rate` (form a); required with `rate`, no default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compounding: Option<Compounding>,
    /// Instalments per year (form a): `annual`, `semi_annual`, `four_monthly` (3 a year),
    /// `quarterly`, `bimonthly` (every two months, 6 a year) or `monthly`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_frequency: Option<PaymentFrequency>,
    /// Rate per payment period, decimal fraction (form b), greater than -1; instead of `rate`,
    /// `compounding` and `payment_frequency`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub periodic_rate: Option<Decimal>,
    /// Number of instalments, from 1 to 10000.
    pub number_of_instalments: u32,
    /// `ordinary` (in arrears: instalment k at the end of period k) or `due` (in advance: at the
    /// beginning of period k).
    pub timing: AnnuityTiming,
    /// Amount repaid at the end of the last period on top of the last instalment, in currency
    /// units, at least 0 and lower than the principal. Default: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balloon_amount: Option<Decimal>,
    /// Rounding of the instalment. Default: none (no rounding).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounding: Option<Rounding>,
}

/// Output of `loans.instalment`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InstalmentOutput {
    /// Constant instalment, in currency units (rounded when `rounding` is given).
    pub instalment: Decimal,
    /// Rate per payment period used, decimal fraction.
    pub periodic_rate: Decimal,
    /// Instalments times their number, plus the balloon amount, in currency units.
    pub total_paid: Decimal,
    /// Total paid less the principal, in currency units.
    pub total_interest: Decimal,
}

pub fn instalment(input: &InstalmentInput) -> Result<(InstalmentOutput, Explain)> {
    // Every input error is reported (ADR 0014 G5), in the order of the checks.
    let mut found = Found::new();
    found.check(check_principal(input.principal));
    found.check(check_instalments(input.number_of_instalments));
    if let Some(r) = &input.rounding {
        found.check(r.validate());
    }
    found.check(check_rate_form(&RateForm {
        rate: input.rate,
        compounding: input.compounding,
        payment_frequency: input.payment_frequency,
        periodic_rate: input.periodic_rate,
    }));
    found.finish()?;
    let mut ex = Explain::new(
        "loans.instalment",
        "A = (P − B (1 + i)^−n) × i / (1 − (1 + i)^−n), divided by (1 + i) if due; \
         (P − B) / n when i = 0",
    );
    ex.convention(timing_convention(input.timing));
    let i = periodic_rate(
        &RateForm {
            rate: input.rate,
            compounding: input.compounding,
            payment_frequency: input.payment_frequency,
            periodic_rate: input.periodic_rate,
        },
        &mut ex,
    )?;
    let b = balloon(input.principal, input.balloon_amount, &mut ex)?;
    ex.convention(rounding_convention(input.rounding.as_ref(), "instalment"));
    ex.convention(
        "totals: number of instalments × instalment (as returned) + balloon amount; with \
         rounding, the final-period adjustment is computed by loans.amortisation_schedule",
    );
    let n = input.number_of_instalments;
    let mut a = level_instalment(input.principal, b, i, n, input.timing, &mut ex)?;
    if let Some(r) = &input.rounding {
        let rounded = r.apply(a);
        ex.step(
            "Rounded instalment",
            format!("round({a}, {})", r.describe()),
            rounded,
        );
        a = rounded;
    }
    let total_paid = num::add(num::mul(a, Decimal::from(n))?, b)?;
    ex.step("Total paid", format!("{n} × {a} + {b}"), total_paid);
    let total_interest = num::sub(total_paid, input.principal)?;
    ex.step(
        "Total interest",
        format!("{total_paid} − {}", input.principal),
        total_interest,
    );
    Ok((
        InstalmentOutput {
            instalment: a,
            periodic_rate: i,
            total_paid,
            total_interest,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- loans.number_of_instalments

/// Input of `loans.number_of_instalments`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NumberOfInstalmentsInput {
    /// Amount borrowed, in currency units, greater than zero.
    pub principal: Decimal,
    /// Annual borrowing rate, decimal fraction (0.05 = 5% a year), quoted with `compounding`.
    /// Rate form (a): give it with `compounding` and `payment_frequency`, or give
    /// `periodic_rate` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
    /// Compounding of `rate` (form a); required with `rate`, no default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compounding: Option<Compounding>,
    /// Instalments per year (form a): `annual`, `semi_annual`, `four_monthly` (3 a year),
    /// `quarterly`, `bimonthly` (every two months, 6 a year) or `monthly`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_frequency: Option<PaymentFrequency>,
    /// Rate per payment period, decimal fraction (form b), greater than -1; instead of `rate`,
    /// `compounding` and `payment_frequency`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub periodic_rate: Option<Decimal>,
    /// Constant instalment, in currency units; it must exceed the interest of the first period.
    pub instalment: Decimal,
    /// `ordinary` (in arrears: instalment k at the end of period k) or `due` (in advance: at the
    /// beginning of period k).
    pub timing: AnnuityTiming,
}

/// Output of `loans.number_of_instalments`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NumberOfInstalmentsOutput {
    /// Rate per payment period used, decimal fraction.
    pub periodic_rate: Decimal,
    /// Exact number of periods n* that repays the loan with the given instalment, in periods.
    pub exact_number_of_periods: Decimal,
    /// Number of instalments (a count of payment periods), the smallest whole number not
    /// below n*, from 1 to 10000.
    pub number_of_instalments: u32,
    /// Amount of the last instalment, in currency units: the amount owed on its payment date,
    /// so that the outstanding balance is exactly zero: lower than the instalment when n* is not
    /// whole, above it by at most 1e-20 × principal when n* is taken as a whole number.
    pub final_instalment: Decimal,
}

pub fn number_of_instalments(
    input: &NumberOfInstalmentsInput,
) -> Result<(NumberOfInstalmentsOutput, Explain)> {
    let mut found = Found::new();
    found.check(check_principal(input.principal));
    found.check(check_rate_form(&RateForm {
        rate: input.rate,
        compounding: input.compounding,
        payment_frequency: input.payment_frequency,
        periodic_rate: input.periodic_rate,
    }));
    found.finish()?;
    let (p, a) = (input.principal, input.instalment);
    let mut ex = Explain::new(
        "loans.number_of_instalments",
        "n* = −ln(1 − i P' / A) / ln(1 + i), P' = P (ordinary) or P / (1 + i) (due); \
         n* = P / A when i = 0; N = ⌈n*⌉",
    );
    ex.convention(timing_convention(input.timing));
    let i = periodic_rate(
        &RateForm {
            rate: input.rate,
            compounding: input.compounding,
            payment_frequency: input.payment_frequency,
            periodic_rate: input.periodic_rate,
        },
        &mut ex,
    )?;
    ex.convention("rounding: none (the instalment is used as given)");
    ex.convention(format!(
        "number of instalments N = ⌈n*⌉, an n* within {WHOLE_PERIOD_TOLERANCE} of an integer \
         taken as that integer (within 1e-20 of an integer: the logarithms are series \
         approximations); N is confirmed by the balance recurrence as the first instalment \
         whose amount owed exceeds the instalment by at most {BALANCE_TOLERANCE} × principal"
    ));
    ex.convention(
        "final instalment: the amount owed on its payment date, so that the outstanding balance \
         is exactly zero; lower than the instalment when n* is not whole, and above it by at \
         most 1e-20 × principal when n* is taken as a whole number",
    );
    let one_i = num::one_plus(i)?;
    let p_eff = match input.timing {
        AnnuityTiming::Ordinary => p,
        AnnuityTiming::Due => {
            let v = num::div(p, one_i)?;
            ex.step(
                "Principal one period before the first instalment",
                format!("{p} / (1 + {i})"),
                v,
            );
            v
        }
    };
    let interest = num::mul(i, p_eff)?;
    // One condition, one code: an instalment that never repays the loan is FM-E021 whatever
    // the sign of the rate (with a negative rate and an instalment <= 0 the balance never
    // reaches zero either). Decision 3, maintainer, 2026-11-08.
    if a <= interest || a <= Decimal::ZERO {
        return Err(FairmarkError::LoanNeverRepaid {
            instalment: a,
            interest,
        });
    }
    let exact = if i.is_zero() {
        let e = num::div(p, a)?;
        ex.step(
            "Exact number of periods (rate zero, limit form)",
            format!("{p} / {a}"),
            e,
        );
        e
    } else {
        ex.step(
            "Interest of the first period on P'",
            format!("{i} × {p_eff}"),
            interest,
        );
        // n* = −ln(1 − i q) / ln(1 + i) with q = P' / A, written as
        // q × [ln(1 − i q) / (−i q)] / [ln(1 + i) / i] so that small rates keep every digit.
        let q = num::div(p_eff, a)?;
        ex.step("P' / A", format!("{p_eff} / {a}"), q);
        let ratio = num::mul(i, q)?;
        let f_ratio = ln_1p_over_x(-ratio)?;
        let f_i = ln_1p_over_x(i)?;
        let e = num::div(num::mul(q, f_ratio)?, f_i)?;
        ex.step(
            "Exact number of periods",
            format!(
                "−ln(1 − {i} × {q}) / ln(1 + {i}) = {q} × {f_ratio} / {f_i}, \
                 with ln(1 + x) / x for x = −{ratio} and x = {i}"
            ),
            e,
        );
        e
    };
    let max = Decimal::from(MAX_PERIODS);
    let too_many = || {
        FairmarkError::InvalidInput(format!(
            "with this `instalment` the loan needs more than the {MAX_PERIODS} instalments \
             allowed (n* = {exact}): raise `instalment`"
        ))
        .at("/instalment", Reason::Range)
        .fix("raise `instalment`")
    };
    // n* comes from series logarithms: far above the limit it cannot be within one period of
    // it, so the limit is applied at once; near the limit it is confirmed by the recurrence.
    if exact > max + Decimal::TWO {
        return Err(too_many());
    }
    let nearest = exact.round();
    let candidate = if (exact - nearest).abs() <= WHOLE_PERIOD_TOLERANCE {
        nearest
    } else {
        exact.ceil()
    }
    .max(Decimal::ONE)
    .to_u32()
    .ok_or_else(too_many)?;
    // Confirm N with the balance recurrence (exact up to the 28 significant digits): the
    // amount owed on the payment date of instalment k, time k (ordinary) or k − 1 (due).
    // N is the smallest k whose amount owed does not exceed the instalment by more than
    // BALANCE_TOLERANCE × principal; at small rates the logarithms can miss it by one.
    let tolerance = num::mul(p, BALANCE_TOLERANCE)?;
    let first = match input.timing {
        AnnuityTiming::Ordinary => num::mul(p, one_i)?,
        AnnuityTiming::Due => p,
    };
    let mut owed = vec![first];
    while (owed.len() as u32) < candidate {
        let last = owed[owed.len() - 1];
        owed.push(num::mul(num::sub(last, a)?, one_i)?);
    }
    // Down: the loan is already repaid by instalment N − 1.
    while owed.len() > 1 && num::sub(owed[owed.len() - 2], a)? <= tolerance {
        owed.pop();
    }
    // Up: the amount owed at N still exceeds the instalment.
    while num::sub(owed[owed.len() - 1], a)? > tolerance {
        if owed.len() as u32 >= MAX_PERIODS {
            return Err(too_many());
        }
        let last = owed[owed.len() - 1];
        owed.push(num::mul(num::sub(last, a)?, one_i)?);
    }
    let count = owed.len() as u32;
    if count > MAX_PERIODS {
        return Err(too_many());
    }
    let owed = owed[owed.len() - 1];
    ex.step(
        "Number of instalments",
        format!(
            "⌈{exact}⌉, confirmed by the balance recurrence (first k whose amount owed exceeds \
             {a} by at most {tolerance})"
        ),
        Decimal::from(count),
    );
    ex.step(
        "Final instalment (amount owed on its payment date)",
        format!(
            "b_1 = {first}, b_k = (b_(k−1) − {a}) × (1 + {i}) for k = 2 to {count}; final = b_{count}"
        ),
        owed,
    );
    Ok((
        NumberOfInstalmentsOutput {
            periodic_rate: i,
            exact_number_of_periods: exact,
            number_of_instalments: count,
            final_instalment: owed,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- loans.amortisation_schedule

/// How the principal is repaid (Regulation (EU) 2016/867, Annex IV, "amortisation type").
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AmortisationType {
    /// Constant instalment (French amortisation, "ammortamento alla francese").
    French,
    /// Constant principal portion ("ammortamento all'italiana").
    FixedAmortisationSchedule,
    /// Interest only; the full principal is repaid with the last instalment.
    Bullet,
}

impl AmortisationType {
    fn describe(self) -> &'static str {
        match self {
            Self::French => "french (constant instalment)",
            Self::FixedAmortisationSchedule => {
                "fixed_amortisation_schedule (constant principal portion)"
            }
            Self::Bullet => "bullet (interest only, principal repaid with the last instalment)",
        }
    }
}

/// Input of `loans.amortisation_schedule`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AmortisationScheduleInput {
    /// Amount borrowed, in currency units, greater than zero.
    pub principal: Decimal,
    /// Annual borrowing rate, decimal fraction (0.05 = 5% a year), quoted with `compounding`.
    /// Rate form (a): give it with `compounding` and `payment_frequency`, or give
    /// `periodic_rate` instead.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
    /// Compounding of `rate` (form a); required with `rate`, no default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compounding: Option<Compounding>,
    /// Instalments per year (form a): `annual`, `semi_annual`, `four_monthly` (3 a year),
    /// `quarterly`, `bimonthly` (every two months, 6 a year) or `monthly`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment_frequency: Option<PaymentFrequency>,
    /// Rate per payment period, decimal fraction (form b), greater than -1; instead of `rate`,
    /// `compounding` and `payment_frequency`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub periodic_rate: Option<Decimal>,
    /// Number of instalments over the full term, grace periods included, from 1 to 10000.
    pub number_of_instalments: u32,
    /// `ordinary` (in arrears: instalment k at the end of period k, time k). Version 1 accepts
    /// only `ordinary`; `due` is refused with `FM-E001`.
    pub timing: AnnuityTiming,
    /// `french` (constant instalment), `fixed_amortisation_schedule` (constant principal
    /// portion) or `bullet` (principal repaid with the last instalment).
    pub amortisation_type: AmortisationType,
    /// Number of interest-only periods at the start, lower than `number_of_instalments`.
    /// Default: 0. Not accepted with `bullet`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub principal_grace_periods: Option<u32>,
    /// Principal repaid with the last instalment on top of the amortised amount, in currency
    /// units, at least 0 and lower than the principal. Default: 0. Not accepted with `bullet`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub balloon_amount: Option<Decimal>,
    /// Rounding of the amount the schedule type keeps constant: the instalment (`french`) or
    /// the principal portion (`fixed_amortisation_schedule`). Interest is never rounded.
    /// Default: none (no rounding).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounding: Option<Rounding>,
}

/// One period of the schedule.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScheduleRow {
    /// Period number (a count of payment periods), from 1 to n.
    pub period: u32,
    /// Payment time of the instalment, in periods from the start (time k for period k).
    pub time: u32,
    /// Amount paid, interest plus principal portion (balloon amount included in the last row),
    /// in currency units.
    pub instalment: Decimal,
    /// Periodic rate × outstanding balance at the start of the period, in currency units; not
    /// rounded.
    pub interest_portion: Decimal,
    /// Principal repaid in the period (balloon amount included in the last row), in currency
    /// units.
    pub principal_portion: Decimal,
    /// Balloon amount repaid with the instalment, in currency units; present in the last row
    /// only.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub balloon_amount: Option<Decimal>,
    /// Outstanding balance after the instalment, in currency units.
    pub outstanding_balance: Decimal,
}

/// Sums over all the rows.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScheduleTotals {
    /// Sum of the instalments, in currency units.
    pub instalments: Decimal,
    /// Sum of the interest portions, in currency units.
    pub interest: Decimal,
    /// Sum of the principal portions (equal to the principal), in currency units.
    pub principal: Decimal,
}

/// Output of `loans.amortisation_schedule`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AmortisationScheduleOutput {
    /// Rate per payment period used, decimal fraction.
    pub periodic_rate: Decimal,
    /// One row per period, amounts in currency units.
    pub rows: Vec<ScheduleRow>,
    /// Totals of instalments, interest and principal, in currency units.
    pub totals: ScheduleTotals,
    /// Last instalment less the balloon amount less the regular instalment of the last period:
    /// the correction that brings the outstanding balance exactly to zero, in currency units.
    pub final_adjustment: Decimal,
}

/// The amount the schedule keeps constant: the instalment, the principal portion or nothing.
#[derive(Clone, Copy)]
enum Regular {
    Instalment(Decimal),
    PrincipalPortion(Decimal),
    InterestOnly,
}

pub fn amortisation_schedule(
    input: &AmortisationScheduleInput,
) -> Result<(AmortisationScheduleOutput, Explain)> {
    // Every input error is reported (ADR 0014 G5), in the order of the checks.
    let mut found = Found::new();
    if input.timing == AnnuityTiming::Due {
        found.push(
            FairmarkError::InvalidInput(
                "`timing` must be `ordinary`: loans.amortisation_schedule version 1 supports \
                 payments in arrears only (got `due`)"
                    .to_owned(),
            )
            .at("/timing", Reason::Enum)
            .expected(["ordinary"]),
        );
    }
    found.check(check_principal(input.principal));
    let instalments_valid = found.check(check_instalments(input.number_of_instalments));
    if let Some(r) = &input.rounding {
        found.check(r.validate());
    }
    let kind = input.amortisation_type;
    let mut grace_checked = false;
    if kind == AmortisationType::Bullet {
        if let Some(b) = input.balloon_amount {
            found.push(
                FairmarkError::BalloonOutOfRange(format!(
                    "`balloon_amount` ({b}) cannot be given with `amortisation_type` `bullet`, \
                     which repays the full principal with the last instalment: omit \
                     `balloon_amount`"
                ))
                .at("/balloon_amount", Reason::Requires)
                .between(["/balloon_amount", "/amortisation_type"])
                .fix("omit `balloon_amount`"),
            );
        }
        if let Some(g) = input.principal_grace_periods {
            grace_checked = true;
            found.push(
                FairmarkError::GracePeriodOutOfRange(format!(
                    "`principal_grace_periods` ({g}) cannot be given with `amortisation_type` \
                     `bullet`, which is interest only until the last instalment: omit \
                     `principal_grace_periods`"
                ))
                .at("/principal_grace_periods", Reason::Requires)
                .between(["/principal_grace_periods", "/amortisation_type"])
                .fix("omit `principal_grace_periods`"),
            );
        }
    }
    let (p, n) = (input.principal, input.number_of_instalments);
    let g = input.principal_grace_periods.unwrap_or_default();
    if g >= n && instalments_valid && !grace_checked {
        found.push(
            FairmarkError::GracePeriodOutOfRange(format!(
                "`principal_grace_periods` must be lower than `number_of_instalments` ({n}); got \
                 {g}"
            ))
            .at("/principal_grace_periods", Reason::Range)
            .between(["/principal_grace_periods", "/number_of_instalments"]),
        );
    }
    found.check(check_rate_form(&RateForm {
        rate: input.rate,
        compounding: input.compounding,
        payment_frequency: input.payment_frequency,
        periodic_rate: input.periodic_rate,
    }));
    found.finish()?;
    let mut ex = Explain::new(
        "loans.amortisation_schedule",
        "interest_k = i × opening balance_k; principal_k = instalment − interest_k (french), \
         (P − B) / (n − g) (fixed_amortisation_schedule), 0 before the last period (bullet) \
         and in the grace periods; closing balance_k = opening balance_k − principal_k; last \
         principal = opening balance (balloon included)",
    );
    ex.convention(timing_convention(input.timing));
    ex.convention(format!("amortisation type: {}", kind.describe()));
    let i = periodic_rate(
        &RateForm {
            rate: input.rate,
            compounding: input.compounding,
            payment_frequency: input.payment_frequency,
            periodic_rate: input.periodic_rate,
        },
        &mut ex,
    )?;
    let b = if kind == AmortisationType::Bullet {
        Decimal::ZERO
    } else {
        balloon(p, input.balloon_amount, &mut ex)?
    };
    if kind != AmortisationType::Bullet {
        ex.convention(match input.principal_grace_periods {
            Some(v) => format!("principal grace periods: {v} (given), interest only"),
            None => "principal grace periods: 0 (default)".to_owned(),
        });
    }
    ex.convention(match (kind, input.rounding.as_ref()) {
        (AmortisationType::French, r) => rounding_convention(r, "instalment"),
        (AmortisationType::FixedAmortisationSchedule, r) => {
            rounding_convention(r, "principal portion")
        }
        (AmortisationType::Bullet, Some(r)) => format!(
            "rounding: {}; rounding does not change a bullet schedule (it has no constant \
             amount other than its interest, which is not rounded)",
            r.describe()
        ),
        (AmortisationType::Bullet, None) => "rounding: none (default)".to_owned(),
    });
    ex.convention(
        "interest of a period = periodic rate × opening balance (compound interest regime); \
         interest portions are not rounded",
    );
    ex.convention(
        "final-period adjustment: the last principal portion equals the opening balance of the \
         last period (balloon amount included), so that the final outstanding balance is \
         exactly zero; final_adjustment = last instalment − balloon amount − regular instalment \
         of the last period",
    );
    let amortising = n - g;
    let regular = match kind {
        AmortisationType::French => {
            let a = level_instalment(p, b, i, amortising, AnnuityTiming::Ordinary, &mut ex)?;
            Regular::Instalment(round_regular(
                a,
                input.rounding.as_ref(),
                "instalment",
                &mut ex,
            ))
        }
        AmortisationType::FixedAmortisationSchedule => {
            let c = num::div(num::sub(p, b)?, Decimal::from(amortising))?;
            ex.step("Principal portion", format!("({p} − {b}) / ({n} − {g})"), c);
            Regular::PrincipalPortion(round_regular(
                c,
                input.rounding.as_ref(),
                "principal portion",
                &mut ex,
            ))
        }
        AmortisationType::Bullet => Regular::InterestOnly,
    };
    ex.step(
        "Recurrence of every period k (value: the periodic rate i; the rows carry the numbers)",
        match regular {
            Regular::Instalment(a) => format!(
                "interest_k = {i} × opening balance_k; principal_k = {a} − interest_k \
                 (0 in the grace periods); closing balance_k = opening balance_k − principal_k"
            ),
            Regular::PrincipalPortion(c) => format!(
                "interest_k = {i} × opening balance_k; principal_k = {c} (0 in the grace \
                 periods); instalment_k = principal_k + interest_k; closing balance_k = opening \
                 balance_k − principal_k"
            ),
            Regular::InterestOnly => format!(
                "interest_k = {i} × opening balance_k; principal_k = 0 before the last period; \
                 instalment_k = interest_k"
            ),
        },
        i,
    );

    let rounded = input.rounding.is_some();
    let mut rows = Vec::with_capacity(n as usize);
    let mut balance = p;
    let mut final_adjustment = Decimal::ZERO;
    for k in 1..=n {
        let opening = balance;
        let interest = num::mul(i, opening)?;
        let in_grace = k <= g;
        let row = if k == n {
            let principal = opening;
            let instalment = num::add(interest, principal)?;
            let (adjustment, formula) = match regular {
                Regular::Instalment(a) => (
                    num::sub(num::sub(instalment, b)?, a)?,
                    format!("{instalment} − {b} − {a}"),
                ),
                Regular::PrincipalPortion(c) => (
                    num::sub(num::sub(principal, b)?, c)?,
                    format!(
                        "({principal} − {b}) − {c}, the principal portion above the regular one"
                    ),
                ),
                Regular::InterestOnly => (
                    num::sub(principal, p)?,
                    format!("{principal} − {p}, the principal repaid as scheduled"),
                ),
            };
            if rounded && (principal < Decimal::ZERO || instalment < Decimal::ZERO) {
                return Err(FairmarkError::RoundingTooCoarse(format!(
                    "with `rounding` the last principal portion ({principal}) or instalment \
                     ({instalment}) would be negative: raise `rounding.decimal_places` (up to 6) \
                     or omit `rounding`"
                )));
            }
            ex.step(
                format!("Period {k}: opening balance (balloon amount included)"),
                if k == 1 {
                    "the principal".to_owned()
                } else {
                    format!("closing balance of period {}", k - 1)
                },
                opening,
            );
            ex.step(
                format!("Period {k}: interest portion"),
                format!("{i} × {opening}"),
                interest,
            );
            ex.step(
                format!("Period {k}: principal portion (the opening balance)"),
                format!("{opening}"),
                principal,
            );
            ex.step(
                format!("Period {k}: last instalment"),
                format!("{interest} + {principal}"),
                instalment,
            );
            ex.step("Final adjustment", formula, adjustment);
            final_adjustment = adjustment;
            ScheduleRow {
                period: k,
                time: k,
                instalment,
                interest_portion: interest,
                principal_portion: principal,
                balloon_amount: Some(b),
                outstanding_balance: num::sub(opening, principal)?,
            }
        } else {
            let (principal, instalment) = match regular {
                _ if in_grace => (Decimal::ZERO, interest),
                Regular::InterestOnly => (Decimal::ZERO, interest),
                Regular::Instalment(a) => (num::sub(a, interest)?, a),
                Regular::PrincipalPortion(c) => (c, num::add(c, interest)?),
            };
            let closing = num::sub(opening, principal)?;
            if rounded && closing < Decimal::ZERO {
                return Err(FairmarkError::RoundingTooCoarse(format!(
                    "with `rounding` the outstanding balance would become negative ({closing}) in \
                     period {k} of {n}: raise `rounding.decimal_places` (up to 6) or omit \
                     `rounding`"
                )));
            }
            // The first period, and the first amortising period after a grace period, with the
            // numbers substituted (R22); the rows carry every other period.
            if k == 1 || (g > 0 && k == g + 1 && kind != AmortisationType::Bullet) {
                first_period_steps(
                    &mut ex, k, i, opening, interest, principal, instalment, closing, regular,
                    in_grace,
                );
            }
            ScheduleRow {
                period: k,
                time: k,
                instalment,
                interest_portion: interest,
                principal_portion: principal,
                balloon_amount: None,
                outstanding_balance: closing,
            }
        };
        balance = row.outstanding_balance;
        rows.push(row);
    }
    let totals = ScheduleTotals {
        instalments: sum(rows.iter().map(|r| r.instalment))?,
        interest: sum(rows.iter().map(|r| r.interest_portion))?,
        principal: sum(rows.iter().map(|r| r.principal_portion))?,
    };
    ex.step(
        "Total instalments",
        "Σ instalments of the rows",
        totals.instalments,
    );
    ex.step(
        "Total interest",
        "Σ interest portions of the rows",
        totals.interest,
    );
    ex.step(
        "Total principal",
        "Σ principal portions of the rows",
        totals.principal,
    );
    Ok((
        AmortisationScheduleOutput {
            periodic_rate: i,
            rows,
            totals,
            final_adjustment,
        },
        ex,
    ))
}

fn round_regular(x: Decimal, rounding: Option<&Rounding>, what: &str, ex: &mut Explain) -> Decimal {
    match rounding {
        Some(r) => {
            let rounded = r.apply(x);
            ex.step(
                format!("Rounded {what}"),
                format!("round({x}, {})", r.describe()),
                rounded,
            );
            rounded
        }
        None => x,
    }
}

#[allow(clippy::too_many_arguments)]
fn first_period_steps(
    ex: &mut Explain,
    k: u32,
    i: Decimal,
    opening: Decimal,
    interest: Decimal,
    principal: Decimal,
    instalment: Decimal,
    closing: Decimal,
    regular: Regular,
    in_grace: bool,
) {
    ex.step(
        format!("Period {k}: interest portion"),
        format!("{i} × {opening}"),
        interest,
    );
    let (principal_formula, instalment_formula) = match regular {
        _ if in_grace => (
            "0 (grace period, interest only)".to_owned(),
            format!("{interest} (interest only)"),
        ),
        Regular::InterestOnly => (
            "0 (interest only before the last period)".to_owned(),
            format!("{interest} (interest only)"),
        ),
        Regular::Instalment(a) => (format!("{a} − {interest}"), format!("{a} (constant)")),
        Regular::PrincipalPortion(c) => (format!("{c} (constant)"), format!("{c} + {interest}")),
    };
    ex.step(
        format!("Period {k}: principal portion"),
        principal_formula,
        principal,
    );
    ex.step(
        format!("Period {k}: instalment"),
        instalment_formula,
        instalment,
    );
    ex.step(
        format!("Period {k}: outstanding balance"),
        format!("{opening} − {principal}"),
        closing,
    );
}

fn sum(mut values: impl Iterator<Item = Decimal>) -> Result<Decimal> {
    values.try_fold(Decimal::ZERO, num::add)
}

// ---------------------------------------------------------------- method registrations

/// `loans.instalment`: see the methodology page.
pub struct Instalment;

impl Method for Instalment {
    const ID: &'static str = "loans.instalment";
    const TITLE: &'static str = "Loan instalment (constant instalment, optional balloon)";
    const DESCRIPTION: &'static str =
        "Constant instalment of a loan from its principal, rate, number of instalments and \
        timing, with an optional balloon amount and an explicit rounding.";
    const VERSION: &'static str = "1";
    type Input = InstalmentInput;
    type Output = InstalmentOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        instalment(input)
    }
}

/// `loans.amortisation_schedule`: see the methodology page.
pub struct AmortisationSchedule;

impl Method for AmortisationSchedule {
    const ID: &'static str = "loans.amortisation_schedule";
    const TITLE: &'static str = "Amortisation schedule (French, constant principal, bullet)";
    const DESCRIPTION: &'static str =
        "Period-by-period schedule of a loan with instalment, interest portion, principal \
        portion and outstanding balance; French, fixed amortisation schedule or bullet, with an \
        optional grace period, balloon amount and rounding.";
    const VERSION: &'static str = "1";
    type Input = AmortisationScheduleInput;
    type Output = AmortisationScheduleOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        amortisation_schedule(input)
    }
}

/// `loans.number_of_instalments`: see the methodology page.
pub struct NumberOfInstalments;

impl Method for NumberOfInstalments {
    const ID: &'static str = "loans.number_of_instalments";
    const TITLE: &'static str = "Number of instalments of a loan";
    const DESCRIPTION: &'static str =
        "Number of instalments needed to repay a loan with a given instalment, the exact number \
        of periods and the final instalment.";
    const VERSION: &'static str = "1";
    type Input = NumberOfInstalmentsInput;
    type Output = NumberOfInstalmentsOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        number_of_instalments(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn loan(n: u32, timing: AnnuityTiming) -> InstalmentInput {
        InstalmentInput {
            principal: d("20000"),
            rate: Some(d("0.06")),
            compounding: Some(Compounding::Monthly),
            payment_frequency: Some(PaymentFrequency::Monthly),
            periodic_rate: None,
            number_of_instalments: n,
            timing,
            balloon_amount: None,
            rounding: None,
        }
    }

    /// AC12a: form (a) and the equivalent periodic rate in form (b) give the same instalment.
    #[test]
    fn rate_forms_agree() {
        let a = loan(36, AnnuityTiming::Ordinary);
        let b = InstalmentInput {
            rate: None,
            compounding: None,
            payment_frequency: None,
            periodic_rate: Some(d("0.005")),
            ..a.clone()
        };
        let (x, _) = instalment(&a).unwrap();
        let (y, _) = instalment(&b).unwrap();
        assert_eq!(x.periodic_rate, d("0.005"));
        assert_eq!(x.instalment, y.instalment);
    }

    /// R1a: the conflicting or missing fields are named.
    #[test]
    fn rate_form_errors_name_the_fields() {
        let mut both = loan(12, AnnuityTiming::Ordinary);
        both.periodic_rate = Some(d("0.01"));
        let e = instalment(&both).unwrap_err();
        assert_eq!(e.code(), "FM-E001");
        assert!(e.to_string().contains("payment_frequency"), "{e}");
        let mut partial = loan(12, AnnuityTiming::Ordinary);
        partial.compounding = None;
        let e = instalment(&partial).unwrap_err();
        assert!(e.to_string().contains("missing `compounding`"), "{e}");
    }

    /// AC11, R4: rate zero uses the limit form in a step of its own.
    #[test]
    fn rate_zero_limit_forms_are_separate_steps() {
        let mut input = loan(24, AnnuityTiming::Due);
        input.rate = Some(Decimal::ZERO);
        let (out, ex) = instalment(&input).unwrap();
        assert_eq!(out.instalment, d("20000") / d("24"));
        assert!(ex.steps.iter().any(|s| s.label.contains("rate zero")));
        let (out, ex) = number_of_instalments(&NumberOfInstalmentsInput {
            principal: d("1000"),
            rate: None,
            compounding: None,
            payment_frequency: None,
            periodic_rate: Some(Decimal::ZERO),
            instalment: d("300"),
            timing: AnnuityTiming::Due,
        })
        .unwrap();
        assert_eq!(
            (out.number_of_instalments, out.final_instalment),
            (4, d("100"))
        );
        assert!(ex.steps.iter().any(|s| s.label.contains("rate zero")));
    }

    /// AC3: the number of instalments recomputes n from the unrounded instalment, both timings,
    /// and the final instalment equals the instalment.
    #[test]
    fn number_of_instalments_inverts_the_instalment() {
        for timing in [AnnuityTiming::Ordinary, AnnuityTiming::Due] {
            for n in [1, 7, 60, 360] {
                let (a, _) = instalment(&loan(n, timing)).unwrap();
                let (out, _) = number_of_instalments(&NumberOfInstalmentsInput {
                    principal: d("20000"),
                    rate: Some(d("0.06")),
                    compounding: Some(Compounding::Monthly),
                    payment_frequency: Some(PaymentFrequency::Monthly),
                    periodic_rate: None,
                    instalment: a.instalment,
                    timing,
                })
                .unwrap();
                assert_eq!(out.number_of_instalments, n, "{timing:?} {n}");
                let gap = (out.exact_number_of_periods - Decimal::from(n)).abs();
                assert!(gap < d("1e-15"), "{timing:?} {n}: {gap}");
                let diff = (out.final_instalment - a.instalment).abs();
                assert!(diff < d("1e-15"), "{timing:?} {n}: {diff}");
            }
        }
    }

    /// AC13, R5: conventions name the timing, the rate form, the periodic rate, the rounding
    /// and the defaults.
    #[test]
    fn instalment_reports_its_conventions() {
        let (_, ex) = instalment(&loan(12, AnnuityTiming::Ordinary)).unwrap();
        let all = ex.conventions.join("\n");
        for text in [
            "payments in arrears",
            "rate form (a)",
            "source compounding: monthly compounding",
            "payment frequency: monthly",
            "periodic rate: 0.005",
            "rounding: none (default)",
            "balloon amount: 0 (default)",
        ] {
            assert!(all.contains(text), "missing '{text}' in {all}");
        }
        assert!(ex.steps.iter().any(|s| s.label == "Periodic rate"));
    }

    fn schedule_input(
        kind: AmortisationType,
        i: &str,
        n: u32,
        grace: Option<u32>,
        balloon_amount: Option<Decimal>,
        rounding: Option<Rounding>,
    ) -> AmortisationScheduleInput {
        AmortisationScheduleInput {
            principal: d("150000"),
            rate: None,
            compounding: None,
            payment_frequency: None,
            periodic_rate: Some(d(i)),
            number_of_instalments: n,
            timing: AnnuityTiming::Ordinary,
            amortisation_type: kind,
            principal_grace_periods: grace,
            balloon_amount,
            rounding,
        }
    }

    /// AC14, AC9, AC8 on a grid of schedules: principal portions add up to the principal,
    /// instalments to interest plus principal, balances never rise after the grace period,
    /// the final balance is zero, interest is never rounded, the regular amount is kept, the
    /// payment time is k, and without rounding the present value of the instalments is the
    /// principal and the final adjustment vanishes.
    #[test]
    fn schedule_properties_hold_on_a_grid() {
        // Relative to the principal (AC14 within 1e-18 relative).
        let tol = d("1e-18") * d("150000");
        let roundings = [
            None,
            Some(Rounding {
                decimal_places: 2,
                mode: Some(RoundingMode::HalfUp),
            }),
            Some(Rounding {
                decimal_places: 0,
                mode: Some(RoundingMode::HalfEven),
            }),
        ];
        let (mut checked_exact, mut checked_rounded) = (0, 0);
        for kind in [
            AmortisationType::French,
            AmortisationType::FixedAmortisationSchedule,
            AmortisationType::Bullet,
        ] {
            for i in ["0", "0.004", "0.0125", "-0.001", "0.05"] {
                for n in [1u32, 2, 12, 37, 360, 1200] {
                    for grace in [None, Some(2)] {
                        for balloon_amount in [None, Some(d("30000"))] {
                            for rounding in roundings {
                                let bullet = kind == AmortisationType::Bullet;
                                if bullet && (grace.is_some() || balloon_amount.is_some()) {
                                    continue;
                                }
                                if grace.unwrap_or_default() >= n {
                                    continue;
                                }
                                let input =
                                    schedule_input(kind, i, n, grace, balloon_amount, rounding);
                                let (out, _) = match amortisation_schedule(&input) {
                                    Err(e)
                                        if matches!(
                                            e.kind(),
                                            FairmarkError::RoundingTooCoarse(_)
                                        ) =>
                                    {
                                        continue
                                    }
                                    other => other.unwrap(),
                                };
                                let rate = out.periodic_rate;
                                let label = format!("{kind:?} i={i} n={n} g={grace:?} b={balloon_amount:?} r={rounding:?}");
                                assert_eq!(out.rows.len(), n as usize, "{label}");
                                let p = input.principal;
                                // AC14: the principal portions add up to the principal to
                                // the last of the 28 significant digits.
                                assert!(
                                    (out.totals.principal - p).abs() <= d("1e-26") * p,
                                    "{label}: {}",
                                    out.totals.principal
                                );
                                assert!(
                                    (out.totals.instalments
                                        - out.totals.interest
                                        - out.totals.principal)
                                        .abs()
                                        < tol,
                                    "{label}"
                                );
                                let g = grace.unwrap_or_default();
                                let mut opening = p;
                                let mut pv = Decimal::ZERO;
                                for (k, row) in out.rows.iter().enumerate() {
                                    let k = k as u32 + 1;
                                    assert_eq!((row.period, row.time), (k, k), "{label}");
                                    assert_eq!(row.interest_portion, rate * opening, "{label}");
                                    // AC14: row identities to the last of the 28 significant
                                    // digits (a few units of 1e-27 relative to the principal).
                                    let ulp = d("1e-26") * p;
                                    assert!(
                                        (row.instalment
                                            - row.interest_portion
                                            - row.principal_portion)
                                            .abs()
                                            <= ulp,
                                        "{label}: row {k}"
                                    );
                                    assert!(
                                        (opening - row.principal_portion - row.outstanding_balance)
                                            .abs()
                                            <= ulp,
                                        "{label}: row {k}"
                                    );
                                    if k > g {
                                        assert!(row.outstanding_balance <= opening, "{label}");
                                    }
                                    if k < n && k > g {
                                        match (kind, rounding) {
                                            (AmortisationType::French, Some(r)) => {
                                                assert_eq!(
                                                    row.instalment.round_dp(r.decimal_places),
                                                    row.instalment,
                                                    "{label}"
                                                );
                                                assert_eq!(
                                                    row.instalment, out.rows[g as usize].instalment,
                                                    "{label}"
                                                );
                                            }
                                            (AmortisationType::FixedAmortisationSchedule, _) => {
                                                assert_eq!(
                                                    row.principal_portion,
                                                    out.rows[g as usize].principal_portion,
                                                    "{label}"
                                                );
                                            }
                                            _ => {}
                                        }
                                    }
                                    pv += row.instalment
                                        * tvm::discount_factor(rate, Decimal::from(k)).unwrap();
                                    opening = row.outstanding_balance;
                                }
                                assert!(opening.is_zero(), "{label}: final balance {opening}");
                                if rounding.is_none() {
                                    assert!((pv - p).abs() < tol, "{label}: pv {pv}");
                                    // AC9 (precision stated after the audit): the
                                    // adjustment absorbs the last stored digit of the
                                    // instalment amplified by (1 + i)^n; it is below 1e-18 of
                                    // the principal while (1 + i)^n <= 1e10 (about 470
                                    // periods at 5%, 1200 at 1.9%) and is documented beyond.
                                    let growth = (Decimal::ONE + rate).powi(n.into());
                                    if growth <= d("1e10") {
                                        assert!(
                                            out.final_adjustment.abs() < tol,
                                            "{label}: {}",
                                            out.final_adjustment
                                        );
                                    }
                                }
                                if rounding.is_some() {
                                    checked_rounded += 1;
                                } else {
                                    checked_exact += 1;
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(
            checked_exact >= 200,
            "only {checked_exact} unrounded schedules checked"
        );
        assert!(
            checked_rounded >= 400,
            "only {checked_rounded} rounded schedules checked"
        );
    }

    /// AC12a: the same loan in form (a) and form (b) gives the same rows.
    #[test]
    fn schedule_rate_forms_agree() {
        let b = schedule_input(AmortisationType::French, "0.005", 48, Some(3), None, None);
        let a = AmortisationScheduleInput {
            rate: Some(d("0.06")),
            compounding: Some(Compounding::Monthly),
            payment_frequency: Some(PaymentFrequency::Monthly),
            periodic_rate: None,
            ..b.clone()
        };
        let (x, _) = amortisation_schedule(&a).unwrap();
        let (y, _) = amortisation_schedule(&b).unwrap();
        assert_eq!(x.periodic_rate, y.periodic_rate);
        assert_eq!(x.final_adjustment, y.final_adjustment);
        for (r, q) in x.rows.iter().zip(&y.rows) {
            assert_eq!(
                (
                    r.instalment,
                    r.interest_portion,
                    r.principal_portion,
                    r.outstanding_balance
                ),
                (
                    q.instalment,
                    q.interest_portion,
                    q.principal_portion,
                    q.outstanding_balance
                )
            );
        }
    }

    /// AC13, R22: the explain of a schedule has the same steps whatever the number of periods,
    /// and states the recurrence once, the first period, the final adjustment and the totals.
    #[test]
    fn schedule_explain_is_bounded() {
        for kind in [
            AmortisationType::French,
            AmortisationType::FixedAmortisationSchedule,
        ] {
            let (_, short) =
                amortisation_schedule(&schedule_input(kind, "0.004", 12, Some(1), None, None))
                    .unwrap();
            let (_, long) =
                amortisation_schedule(&schedule_input(kind, "0.004", 10000, Some(1), None, None))
                    .unwrap();
            assert_eq!(short.steps.len(), long.steps.len(), "{kind:?}");
            assert!(long.steps.len() < 40, "{kind:?}: {}", long.steps.len());
            let labels: Vec<&str> = long.steps.iter().map(|s| s.label.as_str()).collect();
            for wanted in [
                "Recurrence of every period k (value: the periodic rate i; the rows carry the numbers)",
                "Period 1: interest portion",
                "Period 2: principal portion",
                "Final adjustment",
                "Total instalments",
                "Total interest",
                "Total principal",
            ] {
                assert!(labels.contains(&wanted), "{kind:?}: missing {wanted}");
            }
        }
    }

    /// R2, AC8: payments in advance are refused by version 1.
    #[test]
    fn schedule_refuses_payments_in_advance() {
        let mut input = schedule_input(AmortisationType::French, "0.01", 12, None, None, None);
        input.timing = AnnuityTiming::Due;
        assert_eq!(amortisation_schedule(&input).unwrap_err().code(), "FM-E001");
    }

    /// B1 (audit 2026-11-08): small rates of either sign give back exactly the number of
    /// instalments of the instalment computed for it, never n + 1, an internal error or a false
    /// limit error.
    #[test]
    fn small_rates_give_back_their_number_of_instalments() {
        for i in [
            "-0.000000005",
            "-0.000000001",
            "-0.0000000003",
            "-0.00000000001",
            "0.00000000001",
            "0.000000001",
        ] {
            for n in [1200u32, 5000, 10000] {
                for timing in [AnnuityTiming::Ordinary, AnnuityTiming::Due] {
                    let mut input = loan(n, timing);
                    input.rate = None;
                    input.compounding = None;
                    input.payment_frequency = None;
                    input.periodic_rate = Some(d(i));
                    let (a, _) = instalment(&input).unwrap();
                    let (out, _) = number_of_instalments(&NumberOfInstalmentsInput {
                        principal: input.principal,
                        rate: None,
                        compounding: None,
                        payment_frequency: None,
                        periodic_rate: Some(d(i)),
                        instalment: a.instalment,
                        timing,
                    })
                    .unwrap_or_else(|e| panic!("{i} {n} {timing:?}: {e}"));
                    assert_eq!(out.number_of_instalments, n, "{i} {timing:?}");
                    assert!((out.exact_number_of_periods - Decimal::from(n)).abs() < d("1e-15"));
                    let gap = (out.final_instalment - a.instalment).abs();
                    assert!(
                        gap < d("1e-18") * input.principal,
                        "{i} {n} {timing:?}: {gap}"
                    );
                }
            }
        }
    }
}
