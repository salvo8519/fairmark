//! Discounted cash flow, unlevered (FCFF), with a Gordon growth terminal value.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::tvm::discount_factor;
use crate::types::Timing;

/// Input of `dcf.fcff`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DcfFcffInput {
    /// Free cash flows to the firm for explicit forecast years 1..n.
    pub free_cash_flows: Vec<Decimal>,
    /// Discount rate (WACC) per year as a decimal fraction (0.09 = 9%).
    pub discount_rate: Decimal,
    /// Perpetual growth rate after year n as a decimal fraction. Must be below the discount rate.
    pub terminal_growth: Decimal,
    /// When cash flows arrive within each year. Default: end of period.
    #[serde(default)]
    pub timing: Timing,
    /// Net financial position: debt minus cash. Negative if net cash. Default 0.
    #[serde(default)]
    pub net_debt: Decimal,
    /// Non-operating (surplus) assets added to equity value. Default 0.
    #[serde(default)]
    pub non_operating_assets: Decimal,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DcfFcffOutput {
    pub enterprise_value: Decimal,
    pub equity_value: Decimal,
    pub pv_explicit_period: Decimal,
    pub terminal_value: Decimal,
    pub pv_terminal_value: Decimal,
    /// Share of enterprise value coming from the terminal value (0..1).
    pub terminal_value_weight: Decimal,
    pub discount_factors: Vec<Decimal>,
    pub present_values: Vec<Decimal>,
}

fn ovf(what: &'static str) -> FairmarkError {
    FairmarkError::Overflow(what)
}

pub fn fcff(input: &DcfFcffInput) -> Result<(DcfFcffOutput, Explain)> {
    let n = input.free_cash_flows.len();
    if n == 0 {
        return Err(FairmarkError::EmptyCashFlows);
    }
    let r = input.discount_rate;
    let g = input.terminal_growth;
    if g >= r {
        return Err(FairmarkError::GrowthNotBelowRate { growth: g, rate: r });
    }

    let mut ex = Explain::new(
        "dcf.fcff",
        "EV = Σ FCFF_t × DF_t + TV × DF_n;  TV = FCFF_n × (1 + g) / (WACC − g);  Equity = EV − net debt + non-operating assets",
    );
    ex.convention(format!("cash flows discounted at {}", input.timing.label()));
    ex.convention("terminal value (Gordon growth) placed at the end of year n and discounted with DF_n at t = n");

    let half = Decimal::new(5, 1);
    let mut dfs = Vec::with_capacity(n);
    let mut pvs = Vec::with_capacity(n);
    for (i, fcf) in input.free_cash_flows.iter().enumerate() {
        let year = Decimal::from(i as u64 + 1);
        let t = match input.timing {
            Timing::EndOfPeriod => year,
            Timing::MidPeriod => year - half,
        };
        let df = discount_factor(r, t)?;
        let pv = fcf.checked_mul(df).ok_or(ovf("present value"))?;
        ex.step(
            format!("Discount factor, year {}", i + 1),
            format!("1 / (1 + {r})^{t}"),
            df,
        );
        ex.step(
            format!("Present value of FCFF, year {}", i + 1),
            format!("{fcf} × {df}"),
            pv,
        );
        dfs.push(df);
        pvs.push(pv);
    }

    let pv_explicit = pvs
        .iter()
        .try_fold(Decimal::ZERO, |a, v| a.checked_add(*v))
        .ok_or(ovf("explicit period"))?;
    ex.step("PV of explicit period", "Σ PV FCFF_t", pv_explicit);

    let last = input.free_cash_flows[n - 1];
    let tv = last
        .checked_mul(Decimal::ONE + g)
        .and_then(|v| v.checked_div(r - g))
        .ok_or(ovf("terminal value"))?;
    ex.step(
        "Terminal value at year n",
        format!("{last} × (1 + {g}) / ({r} − {g})"),
        tv,
    );

    let df_n = discount_factor(r, Decimal::from(n as u64))?;
    let pv_tv = tv.checked_mul(df_n).ok_or(ovf("terminal value"))?;
    ex.step("PV of terminal value", format!("{tv} × {df_n}"), pv_tv);

    let ev = pv_explicit
        .checked_add(pv_tv)
        .ok_or(ovf("enterprise value"))?;
    ex.step("Enterprise value", format!("{pv_explicit} + {pv_tv}"), ev);

    let equity = ev
        .checked_sub(input.net_debt)
        .and_then(|v| v.checked_add(input.non_operating_assets))
        .ok_or(ovf("equity value"))?;
    ex.step(
        "Equity value",
        format!("{ev} − {} + {}", input.net_debt, input.non_operating_assets),
        equity,
    );

    let weight = if ev.is_zero() {
        Decimal::ZERO
    } else {
        pv_tv.checked_div(ev).ok_or(ovf("terminal weight"))?
    };

    Ok((
        DcfFcffOutput {
            enterprise_value: ev,
            equity_value: equity,
            pv_explicit_period: pv_explicit,
            terminal_value: tv,
            pv_terminal_value: pv_tv,
            terminal_value_weight: weight,
            discount_factors: dfs,
            present_values: pvs,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- method registrations

/// `dcf.fcff`: see the methodology page.
pub struct Fcff;

impl Method for Fcff {
    const ID: &'static str = "dcf.fcff";
    const TITLE: &'static str = "DCF — unlevered free cash flow";
    const DESCRIPTION: &'static str =
        "Enterprise and equity value from explicit FCFF forecasts discounted at WACC, with a \
        Gordon growth terminal value. Returns every step for audit.";
    const VERSION: &'static str = "2";
    type Input = DcfFcffInput;
    type Output = DcfFcffOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        fcff(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn base() -> DcfFcffInput {
        DcfFcffInput {
            free_cash_flows: vec![d("100"), d("110"), d("120")],
            discount_rate: d("0.10"),
            terminal_growth: d("0.02"),
            timing: Timing::EndOfPeriod,
            net_debt: d("200"),
            non_operating_assets: Decimal::ZERO,
        }
    }

    #[test]
    fn terminal_value_gordon() {
        let (out, _) = fcff(&base()).unwrap();
        assert_eq!(out.terminal_value, d("1530"));
    }

    #[test]
    fn growth_must_be_below_rate() {
        let mut i = base();
        i.terminal_growth = d("0.10");
        assert!(matches!(
            fcff(&i),
            Err(FairmarkError::GrowthNotBelowRate { .. })
        ));
    }

    #[test]
    fn mid_period_increases_value() {
        let end = fcff(&base()).unwrap().0.enterprise_value;
        let mut i = base();
        i.timing = Timing::MidPeriod;
        assert!(fcff(&i).unwrap().0.enterprise_value > end);
    }
}
