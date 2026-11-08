//! `tv.h_model`: growth declining linearly to a stable rate (SPEC-0006 R3).
//!
//! Source: Fuller, R. J. and Hsia, C.-C. (1984), "A Simplified Common Stock Valuation Model",
//! Financial Analysts Journal 40(5).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    check_below_rate, check_growth, check_growth_cap, domain, rate_convention, GrowthCapCheck,
    NOT_DISCOUNTED,
};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, div, mul, one_plus, sub};

/// Input of `tv.h_model`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HModelInput {
    /// Flow of the last explicit period, CF_N, in currency units.
    pub last_flow: Decimal,
    /// Growth at the start of the decline, g_s, decimal fraction per period, greater than -1.
    pub short_term_growth: Decimal,
    /// Stable growth reached after 2H periods and kept forever, g_l, decimal fraction per
    /// period, greater than -1 and below `rate`.
    pub long_term_growth: Decimal,
    /// Half-life H, in periods: the decline from g_s to g_l lasts 2H periods. Zero or positive;
    /// may be fractional.
    pub half_life: Decimal,
    /// Discount rate of the terminal phase, decimal fraction per period; must exceed
    /// `long_term_growth`.
    pub rate: Decimal,
    /// Upper bound for the long-term growth, decimal fraction per period. Default: none,
    /// reported as absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.h_model` inside `tv.compare`: the rate and the growth cap are shared.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HModelTerms {
    /// CF_N, in currency units.
    pub last_flow: Decimal,
    /// g_s, decimal fraction per period.
    pub short_term_growth: Decimal,
    /// g_l, decimal fraction per period, below the shared rate.
    pub long_term_growth: Decimal,
    /// H, in periods, zero or positive.
    pub half_life: Decimal,
}

impl HModelTerms {
    pub(crate) fn with(&self, rate: Decimal, growth_cap: Option<Decimal>) -> HModelInput {
        HModelInput {
            last_flow: self.last_flow,
            short_term_growth: self.short_term_growth,
            long_term_growth: self.long_term_growth,
            half_life: self.half_life,
            rate,
            growth_cap,
        }
    }
}

/// Output of `tv.h_model`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct HModelOutput {
    /// Terminal value at N, not discounted, in currency units.
    pub terminal_value: Decimal,
    /// Value at stable growth, CF_N × (1 + g_l) / (r − g_l), in currency units.
    pub stable_growth_value: Decimal,
    /// Value of the growth above g_l during the decline, CF_N × H × (g_s − g_l) / (r − g_l),
    /// in currency units.
    pub extra_growth_value: Decimal,
    /// Long-term growth compared with the cap; `null` when no cap is given.
    pub growth_cap: Option<GrowthCapCheck>,
}

/// TV_N = CF_N × [(1 + g_l) + H × (g_s − g_l)] / (r − g_l).
pub fn h_model(input: &HModelInput) -> Result<(HModelOutput, Explain)> {
    let (cf, gs, gl, h, r) = (
        input.last_flow,
        input.short_term_growth,
        input.long_term_growth,
        input.half_life,
        input.rate,
    );
    check_growth(gs, "short_term_growth")?;
    check_growth(gl, "long_term_growth")?;
    if h < Decimal::ZERO {
        return Err(domain(format!(
            "the half-life must be zero or positive (got {h})"
        )));
    }
    check_below_rate(gl, r)?;
    let mut ex = Explain::new(
        "tv.h_model",
        "TV_N = CF_N × [(1 + g_l) + H × (g_s − g_l)] / (r − g_l)",
    );
    ex.convention(NOT_DISCOUNTED);
    let two_h = mul(Decimal::TWO, h)?;
    ex.convention(format!(
        "H-model (Fuller and Hsia): approximation of growth declining linearly from {gs} to \
        {gl} over 2H = {two_h} periods, then {gl} forever"
    ));
    rate_convention(&mut ex, r);
    let spread = sub(r, gl)?;
    let stable = div(mul(cf, one_plus(gl)?)?, spread)?;
    ex.step(
        "Value at stable growth",
        format!("{cf} × (1 + {gl}) / ({r} − {gl})"),
        stable,
    );
    let extra = div(mul(mul(cf, h)?, sub(gs, gl)?)?, spread)?;
    ex.step(
        "Value of the extra growth",
        format!("{cf} × {h} × ({gs} − {gl}) / ({r} − {gl})"),
        extra,
    );
    let tv = add(stable, extra)?;
    ex.step("Terminal value at N", format!("{stable} + {extra}"), tv);
    let cap = check_growth_cap(&mut ex, "long-term growth", gl, input.growth_cap);
    Ok((
        HModelOutput {
            terminal_value: tv,
            stable_growth_value: stable,
            extra_growth_value: extra,
            growth_cap: cap,
        },
        ex,
    ))
}

/// `tv.h_model`: see the methodology page.
pub struct HModel;

impl Method for HModel {
    const ID: &'static str = "tv.h_model";
    const TITLE: &'static str = "Terminal value, H-model (declining growth)";
    const DESCRIPTION: &'static str =
        "Terminal value at the horizon N when growth declines linearly from g_s to g_l over 2H \
        periods: CF_N × [(1 + g_l) + H × (g_s − g_l)] / (r − g_l), an approximation.";
    const VERSION: &'static str = "1";
    type Input = HModelInput;
    type Output = HModelOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        h_model(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tv::{d, gordon, GordonInput};

    fn input(gs: &str, gl: &str, h: &str, r: &str) -> HModelInput {
        HModelInput {
            last_flow: d("100"),
            short_term_growth: d(gs),
            long_term_growth: d(gl),
            half_life: d(h),
            rate: d(r),
            growth_cap: None,
        }
    }

    /// AC3: with g_s = g_l the H-model equals the Gordon model on the last flow.
    #[test]
    fn ac3_equal_growths_give_gordon() {
        for (g, h, r) in [
            ("0.02", "5", "0.09"),
            ("0", "0", "0.07"),
            ("-0.01", "2.5", "0.11"),
            ("0.035", "10", "0.08"),
        ] {
            let (hm, _) = h_model(&input(g, g, h, r)).unwrap();
            let (gm, _) = gordon(&GordonInput {
                rate: d(r),
                growth: d(g),
                next_flow: None,
                last_flow: Some(d("100")),
                growth_cap: None,
            })
            .unwrap();
            assert_eq!(hm.extra_growth_value, Decimal::ZERO);
            assert_eq!(hm.terminal_value, gm.terminal_value, "g = {g}, r = {r}");
        }
    }

    #[test]
    fn invalid_inputs_are_refused() {
        assert_eq!(
            h_model(&input("0.1", "0.03", "-1", "0.09"))
                .unwrap_err()
                .code(),
            "FM-E018"
        );
        assert_eq!(
            h_model(&input("0.1", "0.09", "5", "0.09"))
                .unwrap_err()
                .code(),
            "FM-E004"
        );
        assert_eq!(
            h_model(&input("-1", "0.03", "5", "0.09"))
                .unwrap_err()
                .code(),
            "FM-E001"
        );
    }
}
