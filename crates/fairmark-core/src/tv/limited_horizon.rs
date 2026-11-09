//! `tv.limited_horizon`: a flow lasting n more periods, then a residual value
//! (SPEC-0006 R5; Italian practice for limited competitive advantage, concessions, licences).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    check_growth_cap, checked, domain, on, rate_convention, Checked, GrowthCapCheck, NOT_DISCOUNTED,
};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, mul, one_plus};
use crate::rates;
use crate::tvm::{annuity_factor, AnnuityTiming, MAX_PERIODS};

/// Input of `tv.limited_horizon`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LimitedHorizonInput {
    /// First flow after the horizon, at the end of period N + 1, in currency units.
    pub flow: Decimal,
    /// Discount rate of the terminal phase, decimal fraction per period, greater than -1.
    pub rate: Decimal,
    /// Number of further periods n during which the flow lasts, from 1 to 10000.
    #[schemars(range(min = 1, max = 10000))]
    pub periods: u32,
    /// Growth of the flow per period, decimal fraction, greater than -1. Default: 0 (constant
    /// flow).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth: Option<Decimal>,
    /// Residual value at the end of period N + n, in currency units. Default: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residual_value: Option<Decimal>,
    /// Upper bound for the growth, decimal fraction per period. Default: none, reported as
    /// absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.limited_horizon` inside `tv.compare`: the rate and the growth cap are
/// shared.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LimitedHorizonTerms {
    /// First flow, at the end of period N + 1, in currency units.
    pub flow: Decimal,
    /// Number of further periods n, from 1 to 10000.
    #[schemars(range(min = 1, max = 10000))]
    pub periods: u32,
    /// Growth of the flow per period, decimal fraction. Default: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth: Option<Decimal>,
    /// Residual value at N + n, in currency units. Default: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub residual_value: Option<Decimal>,
}

impl LimitedHorizonTerms {
    pub(crate) fn with(&self, rate: Decimal, growth_cap: Option<Decimal>) -> LimitedHorizonInput {
        LimitedHorizonInput {
            flow: self.flow,
            rate,
            periods: self.periods,
            growth: self.growth,
            residual_value: self.residual_value,
            growth_cap,
        }
    }
}

/// Output of `tv.limited_horizon`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct LimitedHorizonOutput {
    /// Terminal value at N, not discounted to the valuation date, in currency units: the value
    /// of the n flows plus the discounted residual value.
    pub terminal_value: Decimal,
    /// Value at N of the flows of periods N + 1 to N + n, in currency units.
    pub flows_value: Decimal,
    /// Value at N of a first flow of 1 (the annuity factor shared with `tvm.annuity`).
    pub annuity_factor: Decimal,
    /// (1 + r)^−n, the factor that brings the residual value from N + n to N.
    pub discount_factor: Decimal,
    /// Residual value brought to N, in currency units.
    pub residual_value_discounted: Decimal,
    /// Growth compared with the cap; `null` when no cap is given.
    pub growth_cap: Option<GrowthCapCheck>,
}

/// Validates the input; each error names its field. The rate and the growth are checked as
/// the annuity factor of `tvm.annuity` checks them, with the same errors.
pub(crate) fn check(input: &LimitedHorizonInput) -> Checked {
    let (r, n) = (input.rate, input.periods);
    let mut errors = Vec::new();
    if !(1..=MAX_PERIODS).contains(&n) {
        errors.push(on("periods")(domain(format!(
            "a finite horizon needs between 1 and {MAX_PERIODS} periods (got {n})"
        ))));
    }
    if r <= -Decimal::ONE {
        errors.push(on("rate")(FairmarkError::RateOutOfRange(r)));
    }
    let g = input.growth.unwrap_or_default();
    if g <= -Decimal::ONE {
        errors.push(on("growth")(FairmarkError::InvalidInput(format!(
            "growth must be greater than -1 (got {g})"
        ))));
    }
    checked(errors)
}

/// TV_N = flow × annuity factor(r, g, n) + RV × (1 + r)^−n.
pub fn limited_horizon(input: &LimitedHorizonInput) -> Result<(LimitedHorizonOutput, Explain)> {
    check(input).map_err(super::into_error)?;
    let (cf, r, n) = (input.flow, input.rate, input.periods);
    // Rate and growth are validated by the annuity factor shared with `tvm.annuity` (R5:
    // reused). Only the factor is used: no value at N + n is computed, so long horizons do not
    // leave the decimal range.
    let (factor, factor_steps) = annuity_factor(
        r,
        input.growth.unwrap_or_default(),
        n,
        AnnuityTiming::Ordinary,
    )?;
    let mut ex = Explain::new(
        "tv.limited_horizon",
        "TV_N = CF_{N+1} × [1 − ((1 + g)/(1 + r))^n] / (r − g) + RV × (1 + r)^−n",
    );
    ex.convention(NOT_DISCOUNTED);
    ex.convention(format!(
        "flows at the end of periods N + 1 to N + {n} ({n} periods, explicit), then the \
        residual value"
    ));
    let g = input.growth.unwrap_or_default();
    ex.convention(match input.growth {
        Some(g) => format!("flow grows by {g} per period (given)"),
        None => "flow grows by 0 per period (default: constant flow)".to_owned(),
    });
    let rv = input.residual_value.unwrap_or_default();
    ex.convention(match input.residual_value {
        Some(rv) => format!("residual value at N + {n}: {rv} (given)"),
        None => format!("residual value at N + {n}: 0 (default)"),
    });
    rate_convention(&mut ex, r);
    ex.steps.extend(factor_steps);
    let flows = mul(cf, factor)?;
    ex.step(
        "Value at N of the flows N + 1 to N + n",
        format!("{cf} × {factor}"),
        flows,
    );
    let df = rates::inverse_pow(one_plus(r)?, Decimal::from(n))?;
    ex.step(
        "Discount factor from N + n to N",
        format!("(1 + {r})^−{n}"),
        df,
    );
    let rv_n = mul(rv, df)?;
    ex.step("Residual value at N", format!("{rv} × {df}"), rv_n);
    let tv = add(flows, rv_n)?;
    ex.step("Terminal value at N", format!("{flows} + {rv_n}"), tv);
    let cap = check_growth_cap(&mut ex, "growth", g, input.growth_cap);
    Ok((
        LimitedHorizonOutput {
            terminal_value: tv,
            flows_value: flows,
            annuity_factor: factor,
            discount_factor: df,
            residual_value_discounted: rv_n,
            growth_cap: cap,
        },
        ex,
    ))
}

/// `tv.limited_horizon`: see the methodology page.
pub struct LimitedHorizon;

impl Method for LimitedHorizon {
    const ID: &'static str = "tv.limited_horizon";
    const TITLE: &'static str = "Terminal value, finite horizon";
    const DESCRIPTION: &'static str =
        "Terminal value at the horizon N of a flow lasting n more periods, constant or growing, \
        plus a residual value at N + n (Italian practice); reuses the annuity factor of \
        tvm.annuity.";
    const VERSION: &'static str = "1";
    type Input = LimitedHorizonInput;
    type Output = LimitedHorizonOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        limited_horizon(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tv::d;

    fn input(r: &str, n: u32) -> LimitedHorizonInput {
        LimitedHorizonInput {
            flow: d("100"),
            rate: d(r),
            periods: n,
            growth: None,
            residual_value: None,
            growth_cap: None,
        }
    }

    /// AC5: with n large the finite horizon converges to the perpetuity, up to n = 10,000.
    #[test]
    fn ac5_long_horizon_converges_to_the_perpetuity() {
        for r in ["0.06", "0.1", "0.13"] {
            let (out, _) = limited_horizon(&input(r, 10000)).unwrap();
            let gap = (d("100") / d(r) - out.terminal_value).abs();
            assert!(gap <= d("0.0000000000000000000001"), "r = {r}: {gap}");
        }
    }

    #[test]
    fn residual_value_is_discounted_over_n_periods() {
        let mut i = input("0.1", 2);
        i.residual_value = Some(d("121"));
        let (out, _) = limited_horizon(&i).unwrap();
        assert_eq!(out.residual_value_discounted, d("100"));
    }

    /// One error code for the whole range of n.
    #[test]
    fn periods_outside_one_to_ten_thousand_are_refused() {
        for n in [0, 10001, u32::MAX] {
            assert_eq!(
                limited_horizon(&input("0.1", n)).unwrap_err().code(),
                "FM-E018"
            );
        }
    }
}
