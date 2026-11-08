//! `tv.normalised_income`: a normalised income capitalised at the horizon (SPEC-0006 R6;
//! Italian practice of income-based reasoning).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{check_below_rate, check_growth, check_growth_cap, GrowthCapCheck, NOT_DISCOUNTED};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{div, sub};

/// Input of `tv.normalised_income`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NormalisedIncomeInput {
    /// Normalised income R of the first period after the horizon, in currency units.
    pub income: Decimal,
    /// Capitalisation rate i, decimal fraction per period; must exceed `growth` (or 0 when no
    /// growth is given).
    pub capitalisation_rate: Decimal,
    /// Perpetual growth of the income, decimal fraction per period, greater than -1 and below
    /// the capitalisation rate. Default: none (R / i).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth: Option<Decimal>,
    /// Upper bound for the growth, decimal fraction per period. Default: none, reported as
    /// absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.normalised_income` inside `tv.compare`: the growth cap is shared; the
/// capitalisation rate stays a term of the technique.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NormalisedIncomeTerms {
    /// Normalised income R, in currency units.
    pub income: Decimal,
    /// Capitalisation rate i, decimal fraction per period.
    pub capitalisation_rate: Decimal,
    /// Perpetual growth, decimal fraction per period. Default: none (R / i).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth: Option<Decimal>,
}

impl NormalisedIncomeTerms {
    pub(crate) fn with(&self, growth_cap: Option<Decimal>) -> NormalisedIncomeInput {
        NormalisedIncomeInput {
            income: self.income,
            capitalisation_rate: self.capitalisation_rate,
            growth: self.growth,
            growth_cap,
        }
    }
}

/// Output of `tv.normalised_income`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NormalisedIncomeOutput {
    /// Terminal value at N, not discounted, in currency units.
    pub terminal_value: Decimal,
    /// Growth compared with the cap (0 when no growth is given); `null` when no cap is given.
    pub growth_cap: Option<GrowthCapCheck>,
}

/// TV_N = R / i, or R / (i − g) with a growth.
pub fn normalised_income(
    input: &NormalisedIncomeInput,
) -> Result<(NormalisedIncomeOutput, Explain)> {
    let (income, i) = (input.income, input.capitalisation_rate);
    let g = input.growth.unwrap_or_default();
    check_growth(g, "growth")?;
    check_below_rate(g, i)?;
    let mut ex = Explain::new(
        "tv.normalised_income",
        "TV_N = R / i; R / (i − g) with growth",
    );
    ex.convention(NOT_DISCOUNTED);
    ex.convention(format!("capitalisation rate {i} per period (given)"));
    let tv = match input.growth {
        None => {
            ex.convention("no growth: R / i (default)");
            let tv = div(income, i)?;
            ex.step("Terminal value at N", format!("{income} / {i}"), tv);
            tv
        }
        Some(g) => {
            ex.convention(format!(
                "income grows by {g} per period (given): R / (i − g)"
            ));
            let tv = div(income, sub(i, g)?)?;
            ex.step("Terminal value at N", format!("{income} / ({i} − {g})"), tv);
            tv
        }
    };
    let cap = check_growth_cap(&mut ex, "growth", g, input.growth_cap);
    Ok((
        NormalisedIncomeOutput {
            terminal_value: tv,
            growth_cap: cap,
        },
        ex,
    ))
}

/// `tv.normalised_income`: see the methodology page.
pub struct NormalisedIncome;

impl Method for NormalisedIncome {
    const ID: &'static str = "tv.normalised_income";
    const TITLE: &'static str = "Terminal value, capitalised normalised income";
    const DESCRIPTION: &'static str =
        "Terminal value at the horizon N as a normalised income capitalised at a rate i, R / i, \
        or R / (i − g) with growth (Italian practice).";
    const VERSION: &'static str = "1";
    type Input = NormalisedIncomeInput;
    type Output = NormalisedIncomeOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        normalised_income(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tv::d;

    fn input(i: &str, g: Option<&str>) -> NormalisedIncomeInput {
        NormalisedIncomeInput {
            income: d("120"),
            capitalisation_rate: d(i),
            growth: g.map(d),
            growth_cap: None,
        }
    }

    #[test]
    fn zero_growth_equals_no_growth() {
        let (a, _) = normalised_income(&input("0.08", None)).unwrap();
        let (b, _) = normalised_income(&input("0.08", Some("0"))).unwrap();
        assert_eq!(a.terminal_value, d("1500"));
        assert_eq!(a.terminal_value, b.terminal_value);
    }

    #[test]
    fn rate_not_above_growth_is_refused() {
        for (i, g) in [("0.05", Some("0.05")), ("0", None), ("-0.01", None)] {
            assert_eq!(
                normalised_income(&input(i, g)).unwrap_err().code(),
                "FM-E004"
            );
        }
    }
}
