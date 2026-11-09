//! Rate conventions: compounding, growth and discount factors, rate conversion
//! (ADR 0008, SPEC-0002 R3, R4).
//!
//! Integer exponents use exact repeated multiplication. Fractional exponents, `exp` and `ln`
//! use `rust_decimal` series functions: deterministic on every platform and verified against
//! a 50-digit reference, but approximations (see the methodology pages).

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{rate_field, FairmarkError, Result};
use crate::explain::Explain;
use crate::frequency::Frequency;
use crate::method::Method;
use crate::num;

// The periodic values take their periods a year from the named frequencies of the engine
// (`frequency::Frequency`, SPEC-0010 R20).
/// How often interest is compounded within a year.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Compounding {
    /// Once a year: the rate is an effective annual rate.
    #[default]
    Annual,
    /// Twice a year (nominal rate / 2 per half year).
    SemiAnnual,
    /// Four times a year.
    Quarterly,
    /// Twelve times a year.
    Monthly,
    /// Continuously: growth factor e^(r t).
    Continuous,
    /// Six times a year, every two months (not twice a month): nominal rate / 6 per period.
    Bimonthly,
    /// Three times a year, every four months: nominal rate / 3 per period.
    FourMonthly,
}

impl Compounding {
    /// The named frequency of a periodic compounding, `None` for continuous compounding.
    pub fn frequency(self) -> Option<Frequency> {
        match self {
            Self::Annual => Some(Frequency::Annual),
            Self::SemiAnnual => Some(Frequency::SemiAnnual),
            Self::Quarterly => Some(Frequency::Quarterly),
            Self::Monthly => Some(Frequency::Monthly),
            Self::Bimonthly => Some(Frequency::Bimonthly),
            Self::FourMonthly => Some(Frequency::FourMonthly),
            Self::Continuous => None,
        }
    }

    /// The periodic compounding of a named frequency. The match is exhaustive, so a new
    /// `Frequency` fails to compile until it is mapped; a test checks that it is the inverse
    /// of [`Compounding::frequency`].
    pub fn of(frequency: Frequency) -> Compounding {
        match frequency {
            Frequency::Annual => Self::Annual,
            Frequency::SemiAnnual => Self::SemiAnnual,
            Frequency::FourMonthly => Self::FourMonthly,
            Frequency::Quarterly => Self::Quarterly,
            Frequency::Bimonthly => Self::Bimonthly,
            Frequency::Monthly => Self::Monthly,
        }
    }

    /// Compounding periods per year, `None` for continuous compounding.
    pub fn per_year(self) -> Option<u32> {
        self.frequency().map(Frequency::per_year)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Annual => "annual compounding (effective annual rate)",
            Self::SemiAnnual => "semi-annual compounding",
            Self::Quarterly => "quarterly compounding",
            Self::Monthly => "monthly compounding",
            Self::Continuous => "continuous compounding",
            Self::Bimonthly => "bimonthly compounding (every two months, 6 periods a year)",
            Self::FourMonthly => "four-monthly compounding (every four months, 3 periods a year)",
        }
    }
}

/// `base^exp`, exact for integer exponents.
pub(crate) fn pow(base: Decimal, exp: Decimal) -> Result<Decimal> {
    if exp.fract().is_zero() {
        let e = exp.to_i64().ok_or(FairmarkError::Overflow("exponent"))?;
        base.checked_powi(e)
    } else {
        base.checked_powd(exp)
    }
    .ok_or(FairmarkError::Overflow("power"))
}

/// `1 + rate / m`, validated (the rate per compounding period must exceed -100%).
fn period_base(rate: Decimal, m: Decimal) -> Result<Decimal> {
    let base = num::add(Decimal::ONE, num::div(rate, m)?)?;
    if base <= Decimal::ZERO {
        return Err(FairmarkError::RateOutOfRange(rate));
    }
    Ok(base)
}

/// Growth factor of one unit over `t` years at `rate` with the given compounding.
pub fn growth_factor(rate: Decimal, compounding: Compounding, t: Decimal) -> Result<Decimal> {
    match compounding.per_year() {
        Some(m) => {
            let m = Decimal::from(m);
            pow(period_base(rate, m)?, num::mul(m, t)?)
        }
        None => rate
            .checked_mul(t)
            .and_then(|x| x.checked_exp())
            .ok_or(FairmarkError::Overflow("exponential")),
    }
}

/// `base^-e` with full relative precision (audit D1, D2).
///
/// When `base >= 1` the growth `base^e` is computed and inverted; if it overflows, the
/// reciprocal is raised instead and may underflow towards zero, which is the correct limit.
/// When `base < 1` the reciprocal (greater than one) is raised directly, so the result keeps
/// all significant digits instead of inverting a tiny growth factor.
pub(crate) fn inverse_pow(base: Decimal, e: Decimal) -> Result<Decimal> {
    let reciprocal = || -> Result<Decimal> {
        let inv = Decimal::ONE
            .checked_div(base)
            .ok_or(FairmarkError::Overflow("discount factor"))?;
        pow(inv, e)
    };
    if base >= Decimal::ONE {
        match pow(base, e) {
            Ok(g) if !g.is_zero() => Decimal::ONE
                .checked_div(g)
                .ok_or(FairmarkError::Overflow("discount factor")),
            // Growth overflowed: the reciprocal underflows towards zero, the correct limit.
            _ => match reciprocal() {
                Err(e) if matches!(e.kind(), FairmarkError::Overflow(_)) => Ok(Decimal::ZERO),
                other => other,
            },
        }
    } else {
        reciprocal()
    }
}

/// Discount factor `1 / growth_factor`, computed without loss of relative precision.
pub fn discount_factor(rate: Decimal, compounding: Compounding, t: Decimal) -> Result<Decimal> {
    match compounding.per_year() {
        Some(m) => {
            let m = Decimal::from(m);
            inverse_pow(period_base(rate, m)?, num::mul(m, t)?)
        }
        None => (-rate)
            .checked_mul(t)
            .and_then(|x| x.checked_exp())
            .ok_or(FairmarkError::Overflow("exponential")),
    }
}

/// d(discount factor) / d(rate), used by the rate solvers.
pub fn discount_factor_derivative(
    rate: Decimal,
    compounding: Compounding,
    t: Decimal,
) -> Result<Decimal> {
    let df = discount_factor(rate, compounding, t)?;
    let slope = match compounding.per_year() {
        // d/dr (1 + r/m)^(-m t) = -t (1 + r/m)^(-m t - 1)
        Some(m) => num::div(-t, period_base(rate, Decimal::from(m))?)?,
        None => -t,
    };
    slope
        .checked_mul(df)
        .ok_or(FairmarkError::Overflow("discount factor derivative"))
}

/// Rate with compounding `to` that gives the same growth over one year as `rate` with `from`.
pub fn equivalent_rate(rate: Decimal, from: Compounding, to: Compounding) -> Result<Decimal> {
    if from == to {
        return Ok(rate);
    }
    let g = growth_factor(rate, from, Decimal::ONE)?;
    match to.per_year() {
        Some(1) => num::sub(g, Decimal::ONE),
        Some(m) => {
            let m = Decimal::from(m);
            let root = g
                .checked_ln()
                .ok_or(FairmarkError::Overflow("rate conversion"))
                .and_then(|l| num::div(l, m))?
                .checked_exp()
                .ok_or(FairmarkError::Overflow("rate conversion"))?;
            num::mul(m, num::sub(root, Decimal::ONE)?)
        }
        None => g.checked_ln().ok_or(FairmarkError::Overflow("logarithm")),
    }
}

// ---------------------------------------------------------------- rates.convert

/// Input of `rates.convert`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ConvertInput {
    /// Annual rate as a decimal fraction (0.12 = 12% a year), quoted with `from` compounding.
    pub rate: Decimal,
    /// Compounding of the given rate.
    pub from: Compounding,
    /// Compounding of the equivalent rate to return.
    pub to: Compounding,
}

/// Output of `rates.convert`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ConvertOutput {
    /// Equivalent annual rate with the target compounding.
    pub rate: Decimal,
    /// Growth of one unit over one year, identical for both rates.
    pub annual_growth_factor: Decimal,
}

/// The input check of `convert`, without computing: the rate within the domain of its
/// compounding (the growth over no time, which raises the same error as over one year).
fn check_convert(input: &ConvertInput) -> Result<()> {
    growth_factor(input.rate, input.from, Decimal::ZERO)
        .map(|_| ())
        .map_err(|e| {
            // The domain of the rate depends on its compounding (a continuous rate may be -2).
            rate_field("/rate", input.from.per_year())(e).between(["/rate", "/from"])
        })
}

pub fn convert(input: &ConvertInput) -> Result<(ConvertOutput, Explain)> {
    let mut ex = Explain::new(
        "rates.convert",
        "find r_to such that growth over one year is equal: (1 + r/m)^m or e^r",
    );
    ex.convention(format!("from: {}", input.from.label()));
    ex.convention(format!("to: {}", input.to.label()));
    // The domain of the rate, reported as the validation reports it (with `/from`).
    check_convert(input)?;
    let g = growth_factor(input.rate, input.from, Decimal::ONE)
        .map_err(rate_field("/rate", input.from.per_year()))?;
    ex.step(
        "Growth over one year",
        growth_formula(input.rate, input.from),
        g,
    );
    let rate = equivalent_rate(input.rate, input.from, input.to)?;
    ex.step("Equivalent rate", inverse_formula(g, input.to), rate);
    Ok((
        ConvertOutput {
            rate,
            annual_growth_factor: g,
        },
        ex,
    ))
}

fn growth_formula(rate: Decimal, c: Compounding) -> String {
    match c.per_year() {
        Some(1) => format!("1 + {rate}"),
        Some(m) => format!("(1 + {rate} / {m})^{m}"),
        None => format!("e^{rate}"),
    }
}

fn inverse_formula(g: Decimal, c: Compounding) -> String {
    match c.per_year() {
        Some(1) => format!("{g} − 1"),
        Some(m) => format!("{m} × ({g}^(1/{m}) − 1)"),
        None => format!("ln({g})"),
    }
}

// ---------------------------------------------------------------- rates.real_nominal

/// Direction of the Fisher conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FisherDirection {
    /// The given rate is nominal; return the real rate.
    ToReal,
    /// The given rate is real; return the nominal rate.
    ToNominal,
}

/// Input of `rates.real_nominal`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RealNominalInput {
    /// Rate to convert, decimal fraction per period (0.08 = 8%).
    pub rate: Decimal,
    /// Inflation over the same period, decimal fraction (0.03 = 3%).
    pub inflation: Decimal,
    /// `to_real` or `to_nominal`.
    pub direction: FisherDirection,
}

/// Output of `rates.real_nominal`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RealNominalOutput {
    /// The converted rate, decimal fraction per period.
    pub rate: Decimal,
}

/// Every input error of `real_nominal`, without computing: both fields are checked, and both
/// errors reported (ADR 0014 G5).
fn check_real_nominal(input: &RealNominalInput) -> Result<()> {
    let mut found = crate::error::Found::new();
    for (field, value) in [("/inflation", input.inflation), ("/rate", input.rate)] {
        if value <= -Decimal::ONE {
            found.push(FairmarkError::DiscountRateOutOfRange {
                field: field.to_owned(),
                rate: value,
                converted_with: None,
            });
        }
    }
    found.finish()
}

pub fn real_nominal(input: &RealNominalInput) -> Result<(RealNominalOutput, Explain)> {
    let one = Decimal::ONE;
    check_real_nominal(input)?;
    let mut ex = Explain::new(
        "rates.real_nominal",
        "(1 + nominal) = (1 + real) × (1 + inflation)",
    );
    ex.convention("exact Fisher relation (not the additive approximation)");
    let (rate, formula) = match input.direction {
        FisherDirection::ToReal => (
            num::sub(
                num::div(num::one_plus(input.rate)?, num::one_plus(input.inflation)?)?,
                one,
            )?,
            format!("(1 + {}) / (1 + {}) − 1", input.rate, input.inflation),
        ),
        FisherDirection::ToNominal => (
            num::sub(
                num::mul(num::one_plus(input.rate)?, num::one_plus(input.inflation)?)?,
                one,
            )?,
            format!("(1 + {}) × (1 + {}) − 1", input.rate, input.inflation),
        ),
    };
    let label = match input.direction {
        FisherDirection::ToReal => "Real rate",
        FisherDirection::ToNominal => "Nominal rate",
    };
    ex.step(label, formula, rate);
    Ok((RealNominalOutput { rate }, ex))
}

// ---------------------------------------------------------------- method registrations

/// `rates.convert`: see the methodology page.
pub struct Convert;

impl Method for Convert {
    const ID: &'static str = "rates.convert";
    const TITLE: &'static str = "Rate conversion between compounding conventions";
    const DESCRIPTION: &'static str =
        "Equivalent annual rate under another compounding (annual, semi-annual, four-monthly, \
        quarterly, bimonthly, monthly, continuous).";
    const VERSION: &'static str = "1";
    type Input = ConvertInput;
    type Output = ConvertOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check_convert(input)
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        convert(input)
    }
}

/// `rates.real_nominal`: see the methodology page.
pub struct RealNominal;

impl Method for RealNominal {
    const ID: &'static str = "rates.real_nominal";
    const TITLE: &'static str = "Real and nominal rates (Fisher)";
    const DESCRIPTION: &'static str =
        "Converts a nominal rate to a real rate, or the reverse, with the exact Fisher relation.";
    const VERSION: &'static str = "1";
    type Input = RealNominalInput;
    type Output = RealNominalOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check_real_nominal(input)
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        real_nominal(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn conversions_round_trip() {
        // SPEC-0010 AC14: every pair of the seven compounding values.
        let all = [
            Compounding::Annual,
            Compounding::SemiAnnual,
            Compounding::FourMonthly,
            Compounding::Quarterly,
            Compounding::Bimonthly,
            Compounding::Monthly,
            Compounding::Continuous,
        ];
        for from in all {
            for to in all {
                let r = d("0.0735");
                let there = equivalent_rate(r, from, to).unwrap();
                let back = equivalent_rate(there, to, from).unwrap();
                assert!((back - r).abs() < d("1e-20"), "{from:?} -> {to:?}: {back}");
            }
        }
    }

    #[test]
    fn annual_integer_growth_is_exact() {
        assert_eq!(
            growth_factor(d("0.1"), Compounding::Annual, d("2")).unwrap(),
            d("1.21")
        );
    }

    #[test]
    fn fisher_errors_name_the_field() {
        // Issue #176: the inflation is checked first and named.
        let e = real_nominal(&RealNominalInput {
            rate: d("-2"),
            inflation: d("-1"),
            direction: FisherDirection::ToReal,
        })
        .unwrap_err();
        assert_eq!(e.code(), "FM-E003");
        assert!(e.to_string().starts_with("`/inflation` is -1"), "{e}");
        let e = real_nominal(&RealNominalInput {
            rate: d("-1"),
            inflation: d("0.02"),
            direction: FisherDirection::ToNominal,
        })
        .unwrap_err();
        assert!(e.to_string().starts_with("`/rate` is -1"), "{e}");
    }

    /// SPEC-0010 AC14: with m = 3 the rate must exceed -3, named with the rule.
    #[test]
    fn four_monthly_rate_of_minus_three_is_rejected_with_the_rule() {
        let e = convert(&ConvertInput {
            rate: d("-3"),
            from: Compounding::FourMonthly,
            to: Compounding::Annual,
        })
        .unwrap_err();
        assert_eq!(e.code(), "FM-E003");
        let m = e.to_string();
        assert!(
            m.starts_with("`/rate` is -3: with 3 compounding periods a year"),
            "{m}"
        );
        assert!(m.contains("the rate must be greater than -3"), "{m}");
        assert!(convert(&ConvertInput {
            rate: d("-2.99"),
            from: Compounding::FourMonthly,
            to: Compounding::Bimonthly,
        })
        .is_ok());
    }

    /// SPEC-0010 R21: the new values are labelled in the conventions.
    #[test]
    fn new_compounding_values_are_labelled() {
        let (_, ex) = convert(&ConvertInput {
            rate: d("0.0735"),
            from: Compounding::Bimonthly,
            to: Compounding::FourMonthly,
        })
        .unwrap();
        assert_eq!(
            ex.conventions,
            [
                "from: bimonthly compounding (every two months, 6 periods a year)",
                "to: four-monthly compounding (every four months, 3 periods a year)",
            ]
        );
    }

    #[test]
    fn rate_below_minus_m_is_rejected() {
        assert!(growth_factor(d("-1"), Compounding::Annual, d("1")).is_err());
    }
}
