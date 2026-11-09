//! `ratios.liquidity`: can the company pay in the short term? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    combine, quotient, BalanceBasis, Balances, NotComputedList, Ratio, RatioValue,
    MISSING_CONVENTION,
};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::not_computed::NotComputedEntry;
use crate::num;

/// Balance-sheet amounts at one date, in currency units. Every amount is optional: absent or
/// null means not given, and the outputs that need it are not computed (never taken as zero).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LiquidityBalances {
    /// Current assets, inventories and cash included, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_assets: Option<Decimal>,
    /// Inventories (part of current assets), in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventories: Option<Decimal>,
    /// Cash and cash equivalents (part of current assets), in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cash: Option<Decimal>,
    /// Current liabilities, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_liabilities: Option<Decimal>,
}

/// Input of `ratios.liquidity` (version 2). Only `balances` is required; each output is
/// computed when its own amounts are given (SPEC-0007 R7).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LiquidityInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// Balance sheet at the end of the period. Absent or null: no amount given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closing: Option<LiquidityBalances>,
    /// Balance sheet at the start of the period, used with average balances; not used with
    /// closing balances. Absent with average balances: no output is computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<LiquidityBalances>,
}

/// Output of `ratios.liquidity`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct LiquidityOutput {
    /// Current assets / current liabilities, multiple.
    pub current_ratio: RatioValue,
    /// (Current assets − inventories) / current liabilities, multiple.
    pub quick_ratio: RatioValue,
    /// Cash / current liabilities, multiple.
    pub cash_ratio: RatioValue,
    /// Current assets − current liabilities, in currency units; null when not computed.
    pub net_working_capital: Option<Decimal>,
    /// Current assets − inventories − current liabilities, in currency units; null when not
    /// computed.
    pub treasury_margin: Option<Decimal>,
    /// Outputs not computed because an input is missing, in the order of the fields above,
    /// each with the JSON pointers of the inputs it lacks (SPEC-0007 R7).
    pub not_computed: Vec<NotComputedEntry>,
}

pub fn liquidity(input: &LiquidityInput) -> Result<(LiquidityOutput, Explain)> {
    let mut ex = Explain::new(
        Liquidity::ID,
        "current ratio = current assets / current liabilities; quick ratio = (current assets − \
         inventories) / current liabilities; cash ratio = cash / current liabilities; net \
         working capital = current assets − current liabilities; treasury margin = current \
         assets − inventories − current liabilities",
    );
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex);
    ex.convention("a ratio with a zero denominator is not defined");
    ex.convention(MISSING_CONVENTION);

    let empty = LiquidityBalances::default();
    let close = input.closing.as_ref().unwrap_or(&empty);
    let open = input.opening.as_ref();
    let amount = |ex: &mut Explain, name, path, f: fn(&LiquidityBalances) -> Option<Decimal>| {
        basis.amount(ex, name, path, f(close), open.and_then(f))
    };
    let ca = amount(&mut ex, "current assets", "current_assets", |b| {
        b.current_assets
    })?;
    let inv = amount(&mut ex, "inventories", "inventories", |b| b.inventories)?;
    let cash = amount(&mut ex, "cash", "cash", |b| b.cash)?;
    let cl = amount(&mut ex, "current liabilities", "current_liabilities", |b| {
        b.current_liabilities
    })?;

    let current_ratio = quotient(
        "Current ratio",
        "current assets / current liabilities",
        &ca,
        &cl,
    )
    .evaluate(&mut ex)?;
    let quick_assets = combine("quick assets", [&ca, &inv], |[a, i]| {
        let quick = num::sub(a, i)?;
        ex.step(
            "Quick assets",
            format!("current assets − inventories = {a} − {i}"),
            quick,
        );
        Ok(quick)
    })?;
    let quick_ratio = quotient(
        "Quick ratio",
        "(current assets − inventories) / current liabilities",
        &quick_assets,
        &cl,
    )
    .evaluate(&mut ex)?;
    let cash_ratio =
        quotient("Cash ratio", "cash / current liabilities", &cash, &cl).evaluate(&mut ex)?;
    let nwc = combine("net working capital", [&ca, &cl], |[a, l]| {
        let nwc = num::sub(a, l)?;
        ex.step(
            "Net working capital",
            format!("current assets − current liabilities = {a} − {l}"),
            nwc,
        );
        Ok(nwc)
    })?;
    let treasury = combine("treasury margin", [&quick_assets, &cl], |[q, l]| {
        let margin = num::sub(q, l)?;
        ex.step(
            "Treasury margin",
            format!("current assets − inventories − current liabilities = {q} − {l}"),
            margin,
        );
        Ok(margin)
    })?;

    let mut nc = NotComputedList::default();
    let output = LiquidityOutput {
        current_ratio: nc.ratio("/current_ratio", current_ratio),
        quick_ratio: nc.ratio("/quick_ratio", quick_ratio),
        cash_ratio: nc.ratio("/cash_ratio", cash_ratio),
        net_working_capital: nc.amount("/net_working_capital", &nwc),
        treasury_margin: nc.amount("/treasury_margin", &treasury),
        not_computed: Vec::new(),
    };
    Ok((
        LiquidityOutput {
            not_computed: nc.finish(),
            ..output
        },
        ex,
    ))
}

/// `ratios.liquidity`: see the methodology page.
pub struct Liquidity;

impl Method for Liquidity {
    const ID: &'static str = "ratios.liquidity";
    const TITLE: &'static str = "Ratios — liquidity";
    const DESCRIPTION: &'static str =
        "Current, quick and cash ratios, net working capital and treasury margin, on closing or \
        average balances. Every amount is optional: an output whose amounts are not all given \
        is null and listed in not_computed with the inputs it lacks; a ratio with a zero \
        denominator is reported as not defined with the reason.";
    const VERSION: &'static str = "2";
    type Input = LiquidityInput;
    type Output = LiquidityOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        liquidity(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Option<Decimal> {
        Some(Decimal::from(v))
    }

    fn input(current_liabilities: i64) -> LiquidityInput {
        LiquidityInput {
            balances: Balances::Closing,
            closing: Some(LiquidityBalances {
                current_assets: d(2500),
                inventories: d(900),
                cash: d(200),
                current_liabilities: d(current_liabilities),
            }),
            opening: None,
        }
    }

    #[test]
    fn zero_current_liabilities_leave_margins_defined() {
        let (out, _) = liquidity(&input(0)).unwrap();
        for r in [&out.current_ratio, &out.quick_ratio, &out.cash_ratio] {
            assert_eq!(r.value, None);
            assert_eq!(
                r.not_defined.as_deref(),
                Some("current liabilities is zero")
            );
        }
        assert_eq!(out.net_working_capital, d(2500));
        assert_eq!(out.treasury_margin, d(1600));
        assert!(out.not_computed.is_empty());
    }

    #[test]
    fn treasury_margin_is_working_capital_less_inventories() {
        let (out, _) = liquidity(&input(2000)).unwrap();
        assert_eq!(
            out.treasury_margin.unwrap(),
            out.net_working_capital.unwrap() - Decimal::from(900)
        );
    }

    #[test]
    fn missing_current_liabilities_leave_every_output_not_computed() {
        let mut i = input(2000);
        i.closing.as_mut().unwrap().current_liabilities = None;
        let (out, ex) = liquidity(&i).unwrap();
        assert_eq!(out.not_computed.len(), 5);
        assert_eq!(out.net_working_capital, None);
        assert!(out
            .not_computed
            .iter()
            .all(|e| e.missing == ["/closing/current_liabilities"]));
        // Quick assets are still computed and shown.
        assert!(ex.steps.iter().any(|s| s.label == "Quick assets"));
    }
}
