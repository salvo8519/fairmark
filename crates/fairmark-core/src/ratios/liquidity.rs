//! `ratios.liquidity`: can the company pay in the short term? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{quotient, term, BalanceBasis, Balances, Ratio, RatioValue};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num;

/// Balance-sheet amounts at one date, in currency units.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LiquidityBalances {
    /// Current assets, inventories and cash included, in currency units.
    pub current_assets: Decimal,
    /// Inventories (part of current assets), in currency units.
    pub inventories: Decimal,
    /// Cash and cash equivalents (part of current assets), in currency units.
    pub cash: Decimal,
    /// Current liabilities, in currency units.
    pub current_liabilities: Decimal,
}

/// Input of `ratios.liquidity`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LiquidityInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// Balance sheet at the end of the period.
    pub closing: LiquidityBalances,
    /// Balance sheet at the start of the period. Required with average balances; not used
    /// with closing balances.
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
    /// Current assets − current liabilities, in currency units.
    pub net_working_capital: Decimal,
    /// Current assets − inventories − current liabilities, in currency units.
    pub treasury_margin: Decimal,
}

pub fn liquidity(input: &LiquidityInput) -> Result<(LiquidityOutput, Explain)> {
    let mut ex = Explain::new(
        Liquidity::ID,
        "current ratio = current assets / current liabilities; quick ratio = (current assets − \
         inventories) / current liabilities; cash ratio = cash / current liabilities; net \
         working capital = current assets − current liabilities; treasury margin = current \
         assets − inventories − current liabilities",
    );
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex)?;
    ex.convention("a ratio with a zero denominator is not defined");

    let close = &input.closing;
    let open = input.opening.as_ref();
    let ca = basis.resolve(
        &mut ex,
        "current assets",
        close.current_assets,
        open.map(|o| o.current_assets),
    )?;
    let inv = basis.resolve(
        &mut ex,
        "inventories",
        close.inventories,
        open.map(|o| o.inventories),
    )?;
    let cash = basis.resolve(&mut ex, "cash", close.cash, open.map(|o| o.cash))?;
    let cl = basis.resolve(
        &mut ex,
        "current liabilities",
        close.current_liabilities,
        open.map(|o| o.current_liabilities),
    )?;

    let current_ratio = quotient(
        "Current ratio",
        "current assets / current liabilities",
        ca,
        cl,
    )
    .evaluate(&mut ex)?;
    let quick_assets = num::sub(ca.value, inv.value)?;
    ex.step(
        "Quick assets",
        format!(
            "current assets − inventories = {} − {}",
            ca.value, inv.value
        ),
        quick_assets,
    );
    let quick_ratio = quotient(
        "Quick ratio",
        "(current assets − inventories) / current liabilities",
        term("quick assets", quick_assets),
        cl,
    )
    .evaluate(&mut ex)?;
    let cash_ratio =
        quotient("Cash ratio", "cash / current liabilities", cash, cl).evaluate(&mut ex)?;
    let nwc = num::sub(ca.value, cl.value)?;
    ex.step(
        "Net working capital",
        format!(
            "current assets − current liabilities = {} − {}",
            ca.value, cl.value
        ),
        nwc,
    );
    let treasury = num::sub(quick_assets, cl.value)?;
    ex.step(
        "Treasury margin",
        format!(
            "current assets − inventories − current liabilities = {quick_assets} − {}",
            cl.value
        ),
        treasury,
    );

    Ok((
        LiquidityOutput {
            current_ratio,
            quick_ratio,
            cash_ratio,
            net_working_capital: nwc,
            treasury_margin: treasury,
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
        average balances. A ratio that cannot be computed is reported as not defined with the \
        reason.";
    const VERSION: &'static str = "1";
    type Input = LiquidityInput;
    type Output = LiquidityOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        liquidity(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    fn input(current_liabilities: i64) -> LiquidityInput {
        LiquidityInput {
            balances: Balances::Closing,
            closing: LiquidityBalances {
                current_assets: d(2500),
                inventories: d(900),
                cash: d(200),
                current_liabilities: d(current_liabilities),
            },
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
    }

    #[test]
    fn treasury_margin_is_working_capital_less_inventories() {
        let (out, _) = liquidity(&input(2000)).unwrap();
        assert_eq!(out.treasury_margin, out.net_working_capital - d(900));
    }
}
