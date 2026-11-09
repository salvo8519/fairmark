//! `tv.gordon`: stable growth terminal value (SPEC-0006 R1).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    check_below_rate, check_growth, check_growth_cap, invalid, on, rate_convention, Checked,
    GrowthCapCheck, NOT_DISCOUNTED,
};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{div, mul, one_plus, sub};

/// Input of `tv.gordon`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GordonInput {
    /// Discount rate of the terminal phase, decimal fraction per period; must exceed `growth`.
    /// It may differ from the rate of the explicit phase.
    pub rate: Decimal,
    /// Perpetual growth of the flow, decimal fraction per period, greater than -1 and below
    /// `rate`.
    pub growth: Decimal,
    /// Flow of the first period after the horizon, CF_{N+1}, in currency units (for example a
    /// normalised flow). Give this or `last_flow`, not both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_flow: Option<Decimal>,
    /// Flow of the last explicit period, CF_N, in currency units; grown by `growth` to
    /// CF_{N+1}. Give this or `next_flow`, not both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_flow: Option<Decimal>,
    /// Upper bound for the perpetual growth, decimal fraction per period (for example the
    /// risk-free rate or the nominal growth of the economy). Default: none, reported as absent.
    /// A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.gordon` inside `tv.compare`: the rate and the growth cap are shared.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GordonTerms {
    /// Perpetual growth of the flow, decimal fraction per period, below the shared rate.
    pub growth: Decimal,
    /// Flow at N + 1, in currency units. Give this or `last_flow`, not both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_flow: Option<Decimal>,
    /// Flow at N, in currency units, grown by `growth`. Give this or `next_flow`, not both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_flow: Option<Decimal>,
}

impl GordonTerms {
    pub(crate) fn with(&self, rate: Decimal, growth_cap: Option<Decimal>) -> GordonInput {
        GordonInput {
            rate,
            growth: self.growth,
            next_flow: self.next_flow,
            last_flow: self.last_flow,
            growth_cap,
        }
    }
}

/// How the flow at N + 1 was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FlowBasis {
    /// CF_{N+1} given by the analyst.
    NextFlowGiven,
    /// CF_{N+1} = CF_N × (1 + g).
    LastFlowGrown,
}

/// Output of `tv.gordon`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GordonOutput {
    /// Terminal value at N, not discounted, in currency units.
    pub terminal_value: Decimal,
    /// Flow at N + 1 used, in currency units.
    pub next_flow: Decimal,
    /// `next_flow_given` or `last_flow_grown`.
    pub flow_basis: FlowBasis,
    /// Growth compared with the cap; `null` when no cap is given.
    pub growth_cap: Option<GrowthCapCheck>,
}

/// Validates the input; each error names its field.
pub(crate) fn check(input: &GordonInput) -> Checked {
    let (r, g) = (input.rate, input.growth);
    check_growth(g, "growth").map_err(on("growth"))?;
    check_below_rate(g, r).map_err(on("growth"))?;
    if input.next_flow.is_some() == input.last_flow.is_some() {
        return Err(on("next_flow")(
            invalid("give exactly one of next_flow (CF_{N+1}) and last_flow (CF_N)")
                .because(crate::error::Reason::Conflict)
                .between(["/next_flow", "/last_flow"]),
        ));
    }
    Ok(())
}

/// TV_N = CF_{N+1} / (r − g).
pub fn gordon(input: &GordonInput) -> Result<(GordonOutput, Explain)> {
    check(input).map_err(super::FieldError::into_error)?;
    let (r, g) = (input.rate, input.growth);
    let mut ex = Explain::new("tv.gordon", "TV_N = CF_{N+1} / (r − g)");
    ex.convention(NOT_DISCOUNTED);
    let (next, basis) = match (input.next_flow, input.last_flow) {
        (Some(next), None) => {
            ex.convention("flow at N + 1 given (normalised by the analyst)");
            (next, FlowBasis::NextFlowGiven)
        }
        (None, Some(last)) => {
            ex.convention("flow at N + 1 is the last flow grown by g: CF_N × (1 + g)");
            let next = mul(last, one_plus(g)?)?;
            ex.step("Flow at N + 1", format!("{last} × (1 + {g})"), next);
            (next, FlowBasis::LastFlowGrown)
        }
        _ => {
            return Err(
                invalid("give exactly one of next_flow (CF_{N+1}) and last_flow (CF_N)")
                    .because(crate::error::Reason::Conflict)
                    .between(["/next_flow", "/last_flow"]),
            )
        }
    };
    rate_convention(&mut ex, r);
    let tv = div(next, sub(r, g)?)?;
    ex.step("Terminal value at N", format!("{next} / ({r} − {g})"), tv);
    let cap = check_growth_cap(&mut ex, "growth", g, input.growth_cap);
    Ok((
        GordonOutput {
            terminal_value: tv,
            next_flow: next,
            flow_basis: basis,
            growth_cap: cap,
        },
        ex,
    ))
}

/// `tv.gordon`: see the methodology page.
pub struct Gordon;

impl Method for Gordon {
    const ID: &'static str = "tv.gordon";
    const TITLE: &'static str = "Terminal value, stable growth (Gordon)";
    const DESCRIPTION: &'static str =
        "Terminal value at the horizon N of a flow growing at g forever, CF_{N+1} / (r − g), \
        from the next flow or the last flow grown by g; not discounted.";
    const VERSION: &'static str = "1";
    type Input = GordonInput;
    type Output = GordonOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        gordon(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tv::d;

    fn input(next: Option<&str>, last: Option<&str>) -> GordonInput {
        GordonInput {
            rate: d("0.09"),
            growth: d("0.02"),
            next_flow: next.map(d),
            last_flow: last.map(d),
            growth_cap: None,
        }
    }

    #[test]
    fn next_and_last_flow_agree() {
        let (a, _) = gordon(&input(Some("102"), None)).unwrap();
        let (b, ex) = gordon(&input(None, Some("100"))).unwrap();
        assert_eq!(a.terminal_value, b.terminal_value);
        assert_eq!(b.next_flow, d("102"));
        assert_eq!(ex.steps.len(), 2);
    }

    #[test]
    fn exactly_one_flow_is_required() {
        for (n, l) in [(None, None), (Some("1"), Some("1"))] {
            assert_eq!(gordon(&input(n, l)).unwrap_err().code(), "FM-E001");
        }
    }

    #[test]
    fn growth_not_below_rate_and_below_minus_one_are_refused() {
        let mut i = input(Some("1"), None);
        i.growth = d("0.09");
        assert_eq!(gordon(&i).unwrap_err().code(), "FM-E004");
        i.growth = d("-1");
        assert_eq!(gordon(&i).unwrap_err().code(), "FM-E001");
    }
}
