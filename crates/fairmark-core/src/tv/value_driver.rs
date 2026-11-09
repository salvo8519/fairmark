//! `tv.value_driver`: stable growth earned by reinvestment (SPEC-0006 R2).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    check_below_rate, check_growth, check_growth_cap, checked, domain, on, rate_convention,
    Checked, GrowthCapCheck, NOT_DISCOUNTED,
};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{div, mul, sub};

/// Input of `tv.value_driver`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValueDriverInput {
    /// Net operating profit after tax of the first period after the horizon, NOPAT_{N+1}, in
    /// currency units.
    pub nopat_next: Decimal,
    /// Perpetual growth, decimal fraction per period, greater than -1 and below `rate`.
    pub growth: Decimal,
    /// Return on new invested capital (RONIC), decimal fraction per period, greater than 0.
    pub ronic: Decimal,
    /// Discount rate of the terminal phase, decimal fraction per period; must exceed `growth`.
    pub rate: Decimal,
    /// Upper bound for the perpetual growth, decimal fraction per period. Default: none,
    /// reported as absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.value_driver` inside `tv.compare`: the rate and the growth cap are shared.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValueDriverTerms {
    /// NOPAT_{N+1}, in currency units.
    pub nopat_next: Decimal,
    /// Perpetual growth, decimal fraction per period, below the shared rate.
    pub growth: Decimal,
    /// Return on new invested capital, decimal fraction per period, greater than 0.
    pub ronic: Decimal,
}

impl ValueDriverTerms {
    pub(crate) fn with(&self, rate: Decimal, growth_cap: Option<Decimal>) -> ValueDriverInput {
        ValueDriverInput {
            nopat_next: self.nopat_next,
            growth: self.growth,
            ronic: self.ronic,
            rate,
            growth_cap,
        }
    }
}

/// Whether growth adds value, from the return on new capital against the rate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ValueOfGrowth {
    /// RONIC above the rate: growth creates value.
    Creates,
    /// RONIC equal to the rate: growth creates no value; TV = NOPAT_{N+1} / r for any g.
    #[serde(rename = "none")]
    Neutral,
    /// RONIC below the rate: growth destroys value.
    Destroys,
}

/// Output of `tv.value_driver`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ValueDriverOutput {
    /// Terminal value at N, not discounted, in currency units.
    pub terminal_value: Decimal,
    /// Share of NOPAT reinvested to earn the growth, g / RONIC, decimal fraction.
    pub reinvestment_rate: Decimal,
    /// Free cash flow at N + 1, NOPAT_{N+1} × (1 − g / RONIC), in currency units.
    pub next_free_cash_flow: Decimal,
    /// `creates`, `none` or `destroys`: RONIC above, equal to or below the rate.
    pub value_of_growth: ValueOfGrowth,
    /// Growth compared with the cap; `null` when no cap is given.
    pub growth_cap: Option<GrowthCapCheck>,
}

/// Validates the input; each error names its field.
pub(crate) fn check(input: &ValueDriverInput) -> Checked {
    let (g, ronic) = (input.growth, input.ronic);
    let mut errors = Vec::new();
    let growth_valid = match check_growth(g, "growth") {
        Err(e) => {
            errors.push(on("growth")(e));
            false
        }
        Ok(()) => true,
    };
    if ronic <= Decimal::ZERO {
        errors.push(on("ronic")(domain(format!(
            "the return on new invested capital must be greater than 0 (got {ronic})"
        ))));
    }
    if growth_valid {
        if let Err(e) = check_below_rate(g, input.rate) {
            errors.push(on("growth")(e));
        }
    }
    checked(errors)
}

/// TV_N = NOPAT_{N+1} × (1 − g / RONIC) / (r − g); NOPAT_{N+1} / r when RONIC = r.
pub fn value_driver(input: &ValueDriverInput) -> Result<(ValueDriverOutput, Explain)> {
    check(input).map_err(super::into_error)?;
    let (nopat, g, ronic, r) = (input.nopat_next, input.growth, input.ronic, input.rate);
    let mut ex = Explain::new(
        "tv.value_driver",
        "TV_N = NOPAT_{N+1} × (1 − g / RONIC) / (r − g); NOPAT_{N+1} / r when RONIC = r",
    );
    ex.convention(NOT_DISCOUNTED);
    ex.convention("growth is earned by reinvesting g / RONIC of NOPAT_{N+1}");
    rate_convention(&mut ex, r);
    let rr = div(g, ronic)?;
    ex.step("Reinvestment rate", format!("{g} / {ronic}"), rr);
    if rr > Decimal::ONE {
        // Reported, not refused (open question in plan.md).
        ex.convention(
            "reinvestment rate above 100%: free cash flow at N + 1 is negative \
            (RONIC below the growth)",
        );
    }
    let fcf = mul(nopat, sub(Decimal::ONE, rr)?)?;
    ex.step(
        "Free cash flow at N + 1",
        format!("{nopat} × (1 − {rr})"),
        fcf,
    );
    let value_of_growth = match ronic.cmp(&r) {
        std::cmp::Ordering::Greater => ValueOfGrowth::Creates,
        std::cmp::Ordering::Equal => ValueOfGrowth::Neutral,
        std::cmp::Ordering::Less => ValueOfGrowth::Destroys,
    };
    let tv = if value_of_growth == ValueOfGrowth::Neutral {
        // Computed directly so that the result does not depend on g in decimal arithmetic
        // either (AC2).
        ex.convention(
            "return on new invested capital equals the rate: growth creates no value, \
            TV = NOPAT_{N+1} / r for any g",
        );
        let tv = div(nopat, r)?;
        ex.step(
            "Terminal value at N (RONIC = r)",
            format!("{nopat} / {r}"),
            tv,
        );
        tv
    } else {
        let verb = match value_of_growth {
            ValueOfGrowth::Creates => "above the rate: growth creates value",
            _ => "below the rate: growth destroys value",
        };
        ex.convention(format!("return on new invested capital {ronic} {verb}"));
        let tv = div(fcf, sub(r, g)?)?;
        ex.step("Terminal value at N", format!("{fcf} / ({r} − {g})"), tv);
        tv
    };
    let cap = check_growth_cap(&mut ex, "growth", g, input.growth_cap);
    Ok((
        ValueDriverOutput {
            terminal_value: tv,
            reinvestment_rate: rr,
            next_free_cash_flow: fcf,
            value_of_growth,
            growth_cap: cap,
        },
        ex,
    ))
}

/// `tv.value_driver`: see the methodology page.
pub struct ValueDriver;

impl Method for ValueDriver {
    const ID: &'static str = "tv.value_driver";
    const TITLE: &'static str = "Terminal value, value driver (growth from reinvestment)";
    const DESCRIPTION: &'static str =
        "Terminal value at the horizon N with growth earned by reinvesting g / RONIC of NOPAT: \
        NOPAT_{N+1} × (1 − g / RONIC) / (r − g); reports the reinvestment rate and whether \
        growth creates value.";
    const VERSION: &'static str = "1";
    type Input = ValueDriverInput;
    type Output = ValueDriverOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        value_driver(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tv::d;

    fn input(g: &str, ronic: &str) -> ValueDriverInput {
        ValueDriverInput {
            nopat_next: d("100"),
            growth: d(g),
            ronic: d(ronic),
            rate: d("0.1"),
            growth_cap: None,
        }
    }

    /// AC2: with RONIC = r the terminal value does not change with g.
    #[test]
    fn ac2_ronic_equal_to_rate_makes_growth_irrelevant() {
        for g in ["-0.05", "0", "0.01", "0.03", "0.0999"] {
            let (out, _) = value_driver(&input(g, "0.1")).unwrap();
            assert_eq!(out.terminal_value, d("1000"), "g = {g}");
            assert_eq!(out.value_of_growth, ValueOfGrowth::Neutral);
        }
    }

    #[test]
    fn value_of_growth_follows_ronic_against_rate() {
        let (up, _) = value_driver(&input("0.02", "0.12")).unwrap();
        let (down, _) = value_driver(&input("0.02", "0.08")).unwrap();
        assert_eq!(up.value_of_growth, ValueOfGrowth::Creates);
        assert_eq!(down.value_of_growth, ValueOfGrowth::Destroys);
        assert!(up.terminal_value > d("1000") && down.terminal_value < d("1000"));
    }

    #[test]
    fn reinvestment_above_one_hundred_percent_is_reported() {
        let (out, ex) = value_driver(&input("0.05", "0.04")).unwrap();
        assert!(out.next_free_cash_flow < Decimal::ZERO);
        assert!(ex
            .conventions
            .iter()
            .any(|c| c.starts_with("reinvestment rate above 100%")));
    }

    #[test]
    fn ronic_not_positive_and_growth_not_below_rate_are_refused() {
        assert_eq!(
            value_driver(&input("0.02", "0")).unwrap_err().code(),
            "FM-E018"
        );
        assert_eq!(
            value_driver(&input("0.02", "-0.1")).unwrap_err().code(),
            "FM-E018"
        );
        assert_eq!(
            value_driver(&input("0.1", "0.2")).unwrap_err().code(),
            "FM-E004"
        );
    }
}
