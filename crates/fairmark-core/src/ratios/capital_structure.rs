//! `ratios.capital_structure`: is the company financed soundly? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    over_equity, quotient, term, BalanceBasis, Balances, NetFinancialPosition, Ratio, RatioValue,
    NFP_CONVENTION,
};
use crate::error::Result;
use crate::explain::Explain;
use crate::method::Method;
use crate::num;

/// Balance-sheet amounts at one date, in currency units.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapitalStructureBalances {
    /// Shareholders' equity, in currency units. May be negative.
    pub equity: Decimal,
    /// Total assets, in currency units.
    pub total_assets: Decimal,
    /// Fixed (non-current) assets, in currency units.
    pub fixed_assets: Decimal,
    /// Non-current (medium and long-term) liabilities, financial and operating, in currency
    /// units.
    pub non_current_liabilities: Decimal,
    /// Net financial position with its composition, in currency units.
    pub net_financial_position: NetFinancialPosition,
}

/// Input of `ratios.capital_structure`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapitalStructureInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// Balance sheet at the end of the period.
    pub closing: CapitalStructureBalances,
    /// Balance sheet at the start of the period. Required with average balances; not used
    /// with closing balances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<CapitalStructureBalances>,
}

/// Output of `ratios.capital_structure`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CapitalStructureOutput {
    /// Net financial position used by the ratios, in currency units (positive for net debt).
    pub net_financial_position: Decimal,
    /// Gross debt / equity, multiple.
    pub debt_to_equity: RatioValue,
    /// Net financial position / equity, multiple.
    pub nfp_to_equity: RatioValue,
    /// Equity / total assets, decimal fraction.
    pub equity_ratio: RatioValue,
    /// Equity − fixed assets, in currency units.
    pub primary_structure_margin: Decimal,
    /// Equity + non-current liabilities − fixed assets, in currency units.
    pub secondary_structure_margin: Decimal,
    /// Equity / fixed assets, multiple.
    pub primary_fixed_asset_coverage: RatioValue,
    /// (Equity + non-current liabilities) / fixed assets, multiple.
    pub secondary_fixed_asset_coverage: RatioValue,
}

pub fn capital_structure(
    input: &CapitalStructureInput,
) -> Result<(CapitalStructureOutput, Explain)> {
    let mut ex = Explain::new(
        CapitalStructure::ID,
        "D/E = gross debt / equity; NFP/E = net financial position / equity; equity ratio = \
         equity / total assets; structure margins = equity (+ non-current liabilities) − fixed \
         assets; fixed-asset coverage = equity (+ non-current liabilities) / fixed assets",
    );
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex)?;
    ex.convention(NFP_CONVENTION);
    ex.convention(
        "D/E uses gross debt only: cash and the other items of the net financial position are \
         excluded",
    );
    ex.convention("a ratio with a zero denominator is not defined");
    ex.convention("a ratio over equity is not defined when equity is zero or negative");

    let close = &input.closing;
    let open = match input.balances {
        Balances::Average => input.opening.as_ref(),
        Balances::Closing => None,
    };
    let closing_nfp = close.net_financial_position.total(&mut ex, "closing")?;
    let opening_nfp = match open {
        Some(o) => Some(o.net_financial_position.total(&mut ex, "opening")?),
        None => None,
    };

    let equity = basis.resolve(&mut ex, "equity", close.equity, open.map(|o| o.equity))?;
    let assets = basis.resolve(
        &mut ex,
        "total assets",
        close.total_assets,
        open.map(|o| o.total_assets),
    )?;
    let fixed = basis.resolve(
        &mut ex,
        "fixed assets",
        close.fixed_assets,
        open.map(|o| o.fixed_assets),
    )?;
    let ncl = basis.resolve(
        &mut ex,
        "non-current liabilities",
        close.non_current_liabilities,
        open.map(|o| o.non_current_liabilities),
    )?;
    let gross = basis.resolve(
        &mut ex,
        "gross debt",
        close.net_financial_position.gross_debt,
        open.map(|o| o.net_financial_position.gross_debt),
    )?;
    let nfp = basis.resolve(&mut ex, "net financial position", closing_nfp, opening_nfp)?;

    let debt_to_equity =
        over_equity("D/E", "gross debt / equity", gross, equity).evaluate(&mut ex)?;
    let nfp_to_equity =
        over_equity("NFP/E", "net financial position / equity", nfp, equity).evaluate(&mut ex)?;
    let equity_ratio =
        quotient("Equity ratio", "equity / total assets", equity, assets).evaluate(&mut ex)?;

    let primary_margin = num::sub(equity.value, fixed.value)?;
    ex.step(
        "Primary structure margin",
        format!("equity − fixed assets = {} − {}", equity.value, fixed.value),
        primary_margin,
    );
    let long_term = num::add(equity.value, ncl.value)?;
    ex.step(
        "Equity and non-current liabilities",
        format!("{} + {}", equity.value, ncl.value),
        long_term,
    );
    let secondary_margin = num::sub(long_term, fixed.value)?;
    ex.step(
        "Secondary structure margin",
        format!(
            "equity + non-current liabilities − fixed assets = {long_term} − {}",
            fixed.value
        ),
        secondary_margin,
    );
    let primary_coverage = quotient(
        "Primary fixed-asset coverage",
        "equity / fixed assets",
        equity,
        fixed,
    )
    .evaluate(&mut ex)?;
    let secondary_coverage = quotient(
        "Secondary fixed-asset coverage",
        "(equity + non-current liabilities) / fixed assets",
        term("equity and non-current liabilities", long_term),
        fixed,
    )
    .evaluate(&mut ex)?;

    Ok((
        CapitalStructureOutput {
            net_financial_position: nfp.value,
            debt_to_equity,
            nfp_to_equity,
            equity_ratio,
            primary_structure_margin: primary_margin,
            secondary_structure_margin: secondary_margin,
            primary_fixed_asset_coverage: primary_coverage,
            secondary_fixed_asset_coverage: secondary_coverage,
        },
        ex,
    ))
}

/// `ratios.capital_structure`: see the methodology page.
pub struct CapitalStructure;

impl Method for CapitalStructure {
    const ID: &'static str = "ratios.capital_structure";
    const TITLE: &'static str = "Ratios — capital structure";
    const DESCRIPTION: &'static str =
        "Debt to equity, net financial position to equity, equity ratio, primary and secondary \
        structure margins and fixed-asset coverage, with the net financial position and its \
        composition stated. A ratio that cannot be computed is reported as not defined with \
        the reason.";
    const VERSION: &'static str = "1";
    type Input = CapitalStructureInput;
    type Output = CapitalStructureOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        capital_structure(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ratios::NfpItem;

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    fn balances(equity: i64) -> CapitalStructureBalances {
        CapitalStructureBalances {
            equity: d(equity),
            total_assets: d(6000),
            fixed_assets: d(3500),
            non_current_liabilities: d(1800),
            net_financial_position: NetFinancialPosition {
                gross_debt: d(2200),
                cash: d(300),
                other_items: vec![NfpItem {
                    name: "lease liabilities".into(),
                    amount: d(150),
                }],
            },
        }
    }

    fn input(equity: i64) -> CapitalStructureInput {
        CapitalStructureInput {
            balances: Balances::Closing,
            closing: balances(equity),
            opening: None,
        }
    }

    #[test]
    fn secondary_margin_is_primary_plus_non_current_liabilities() {
        let (out, _) = capital_structure(&input(2000)).unwrap();
        assert_eq!(
            out.secondary_structure_margin,
            out.primary_structure_margin + d(1800)
        );
    }

    #[test]
    fn debt_to_equity_uses_gross_debt_only() {
        let (out, _) = capital_structure(&input(2000)).unwrap();
        assert_eq!(out.debt_to_equity.value, Some(Decimal::new(11, 1)));
        assert_eq!(out.net_financial_position, d(2050));
    }

    #[test]
    fn non_positive_equity_leaves_equity_ratios_not_defined_only() {
        for (equity, reason) in [(0, "equity is zero"), (-200, "equity is negative (-200)")] {
            let (out, _) = capital_structure(&input(equity)).unwrap();
            assert_eq!(out.debt_to_equity.not_defined.as_deref(), Some(reason));
            assert_eq!(out.nfp_to_equity.not_defined.as_deref(), Some(reason));
            assert!(out.secondary_fixed_asset_coverage.value.is_some());
        }
        let (out, _) = capital_structure(&input(-200)).unwrap();
        assert!(out.equity_ratio.value.is_some());
    }
}
