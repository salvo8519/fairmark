//! `tv.limited_horizon`: a flow lasting n more periods, then a residual value
//! (SPEC-0006 R5; Italian practice for limited competitive advantage, concessions, licences).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{check_growth_cap, domain, rate_convention, GrowthCapCheck, NOT_DISCOUNTED};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, mul, one_plus};
use crate::rates;
use crate::tvm::{annuity, AnnuityInput, AnnuityTiming};

/// Input of `tv.limited_horizon`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LimitedHorizonInput {
    /// First flow after the horizon, at the end of period N + 1, in currency units.
    pub flow: Decimal,
    /// Discount rate of the terminal phase, decimal fraction per period, greater than -1.
    pub rate: Decimal,
    /// Number of further periods n during which the flow lasts, from 1 to 10000.
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
    /// Value at N of a first flow of 1 (annuity factor of `tvm.annuity`).
    pub annuity_factor: Decimal,
    /// (1 + r)^−n, the factor that brings the residual value from N + n to N.
    pub discount_factor: Decimal,
    /// Residual value brought to N, in currency units.
    pub residual_value_discounted: Decimal,
    /// Growth compared with the cap; `null` when no cap is given.
    pub growth_cap: Option<GrowthCapCheck>,
}

/// TV_N = flow × annuity factor(r, g, n) + RV × (1 + r)^−n.
pub fn limited_horizon(input: &LimitedHorizonInput) -> Result<(LimitedHorizonOutput, Explain)> {
    let (cf, r, n) = (input.flow, input.rate, input.periods);
    if n < 1 {
        return Err(domain("a finite horizon needs at least one period (got 0)"));
    }
    // Rate, growth and the upper bound of n are validated by `tvm.annuity` (R5: reused).
    let (flows, annuity_ex) = annuity(&AnnuityInput {
        payment: cf,
        rate: r,
        periods: n,
        growth: input.growth,
        timing: AnnuityTiming::Ordinary,
    })?;
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
    for s in annuity_ex.steps {
        ex.step(format!("tvm.annuity: {}", s.label), s.formula, s.value);
    }
    ex.step(
        "Value at N of the flows N + 1 to N + n",
        format!("tvm.annuity present value, {n} ordinary payments"),
        flows.present_value,
    );
    let df = rates::inverse_pow(one_plus(r)?, Decimal::from(n))?;
    ex.step(
        "Discount factor from N + n to N",
        format!("(1 + {r})^−{n}"),
        df,
    );
    let rv_n = mul(rv, df)?;
    ex.step("Residual value at N", format!("{rv} × {df}"), rv_n);
    let tv = add(flows.present_value, rv_n)?;
    ex.step(
        "Terminal value at N",
        format!("{} + {rv_n}", flows.present_value),
        tv,
    );
    let cap = check_growth_cap(&mut ex, "growth", g, input.growth_cap);
    Ok((
        LimitedHorizonOutput {
            terminal_value: tv,
            flows_value: flows.present_value,
            annuity_factor: flows.factor,
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
        plus a residual value at N + n (Italian practice); reuses tvm.annuity.";
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

    /// AC5: with n large the finite horizon converges to the perpetuity. The horizons keep
    /// (1 + r)^n near 10^25, within the decimal range, because `tvm.annuity` also computes the
    /// future value at N + n; the gap is then about the perpetuity × 10^−25.
    #[test]
    fn ac5_long_horizon_converges_to_the_perpetuity() {
        for (r, n) in [("0.06", 980), ("0.1", 600), ("0.13", 465)] {
            let (out, _) = limited_horizon(&input(r, n)).unwrap();
            let gap = (d("100") / d(r) - out.terminal_value).abs();
            assert!(gap <= d("0.00000000000000000001"), "r = {r}: {gap}");
        }
    }

    /// Past that range the reused annuity overflows: a typed error, never a wrong value.
    #[test]
    fn horizons_beyond_the_decimal_range_return_a_typed_error() {
        let err = limited_horizon(&input("0.1", 10000)).unwrap_err();
        assert_eq!(err.code(), "FM-E008");
    }

    #[test]
    fn residual_value_is_discounted_over_n_periods() {
        let mut i = input("0.1", 2);
        i.residual_value = Some(d("121"));
        let (out, _) = limited_horizon(&i).unwrap();
        assert_eq!(out.residual_value_discounted, d("100"));
    }

    #[test]
    fn zero_periods_are_refused() {
        assert_eq!(
            limited_horizon(&input("0.1", 0)).unwrap_err().code(),
            "FM-E018"
        );
    }
}
