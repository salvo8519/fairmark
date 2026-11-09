//! `ratios.capital_structure`: is the company financed soundly? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    combine, nfp_totals, other_items_defaults, over_equity, quotient, warn_defaults, BalanceBasis,
    Balances, NfpComposition, NotComputedList, Ratio, RatioValue, MISSING_CONVENTION,
    NFP_CONVENTION,
};
use crate::error::Result;
use crate::explain::{DefaultApplied, Explain};
use crate::method::Method;
use crate::not_computed::NotComputedEntry;
use crate::num;

/// Balance-sheet amounts at one date, in currency units. Every amount is optional: absent or
/// null means not given, and the outputs that need it are not computed (never taken as zero).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapitalStructureBalances {
    /// Shareholders' equity, in currency units. May be negative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equity: Option<Decimal>,
    /// Total assets, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_assets: Option<Decimal>,
    /// Fixed (non-current) assets, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fixed_assets: Option<Decimal>,
    /// Non-current (medium and long-term) liabilities, financial and operating, in currency
    /// units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub non_current_liabilities: Option<Decimal>,
    /// Net financial position with its composition, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_financial_position: Option<NfpComposition>,
}

/// Input of `ratios.capital_structure` (version 2). Only `balances` is required; each output is
/// computed when its own amounts are given (SPEC-0007 R7).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CapitalStructureInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// Balance sheet at the end of the period. Absent or null: no amount given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closing: Option<CapitalStructureBalances>,
    /// Balance sheet at the start of the period, used with average balances; not used with
    /// closing balances. Absent with average balances: no output is computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<CapitalStructureBalances>,
}

/// Output of `ratios.capital_structure`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CapitalStructureOutput {
    /// Net financial position used by the ratios, in currency units (positive for net debt);
    /// null when not computed.
    pub net_financial_position: Option<Decimal>,
    /// Gross debt / equity, multiple.
    pub debt_to_equity: RatioValue,
    /// Net financial position / equity, multiple.
    pub nfp_to_equity: RatioValue,
    /// Equity / total assets, decimal fraction.
    pub equity_ratio: RatioValue,
    /// Equity − fixed assets, in currency units; null when not computed.
    pub primary_structure_margin: Option<Decimal>,
    /// Equity + non-current liabilities − fixed assets, in currency units; null when not
    /// computed.
    pub secondary_structure_margin: Option<Decimal>,
    /// Equity / fixed assets, multiple.
    pub primary_fixed_asset_coverage: RatioValue,
    /// (Equity + non-current liabilities) / fixed assets, multiple.
    pub secondary_fixed_asset_coverage: RatioValue,
    /// Outputs not computed because an input is missing, in the order of the fields above,
    /// each with the JSON pointers of the inputs it lacks (SPEC-0007 R7).
    pub not_computed: Vec<NotComputedEntry>,
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
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex);
    ex.convention(NFP_CONVENTION);
    warn_defaults(&mut ex, &defaults(input), "/other_items");
    ex.convention(
        "D/E uses gross debt only: cash and the other items of the net financial position are \
         excluded",
    );
    ex.convention("a ratio with a zero denominator is not defined");
    ex.convention("a ratio over equity is not defined when equity is zero or negative");
    ex.convention(MISSING_CONVENTION);

    let empty = CapitalStructureBalances::default();
    let close = input.closing.as_ref().unwrap_or(&empty);
    let open = match input.balances {
        Balances::Average => input.opening.as_ref(),
        Balances::Closing => None,
    };
    let (closing_nfp, opening_nfp) = nfp_totals(
        &basis,
        &mut ex,
        close.net_financial_position.as_ref(),
        open.and_then(|o| o.net_financial_position.as_ref()),
    )?;

    let amount = |ex: &mut Explain, name, path, f: fn(&CapitalStructureBalances) -> _| {
        basis.amount(ex, name, path, f(close), open.and_then(f))
    };
    let equity = amount(&mut ex, "equity", "equity", |b| b.equity)?;
    let assets = amount(&mut ex, "total assets", "total_assets", |b| b.total_assets)?;
    let fixed = amount(&mut ex, "fixed assets", "fixed_assets", |b| b.fixed_assets)?;
    let ncl = amount(
        &mut ex,
        "non-current liabilities",
        "non_current_liabilities",
        |b| b.non_current_liabilities,
    )?;
    let gross = amount(
        &mut ex,
        "gross debt",
        "net_financial_position/gross_debt",
        |b| b.net_financial_position.as_ref().and_then(|n| n.gross_debt),
    )?;
    let nfp = basis.resolve(&mut ex, "net financial position", closing_nfp, opening_nfp)?;

    let debt_to_equity =
        over_equity("D/E", "gross debt / equity", &gross, &equity).evaluate(&mut ex)?;
    let nfp_to_equity =
        over_equity("NFP/E", "net financial position / equity", &nfp, &equity).evaluate(&mut ex)?;
    let equity_ratio =
        quotient("Equity ratio", "equity / total assets", &equity, &assets).evaluate(&mut ex)?;

    let primary_margin = combine("primary structure margin", [&equity, &fixed], |[e, f]| {
        let margin = num::sub(e, f)?;
        ex.step(
            "Primary structure margin",
            format!("equity − fixed assets = {e} − {f}"),
            margin,
        );
        Ok(margin)
    })?;
    let long_term = combine(
        "equity and non-current liabilities",
        [&equity, &ncl],
        |[e, n]| {
            let sum = num::add(e, n)?;
            ex.step(
                "Equity and non-current liabilities",
                format!("{e} + {n}"),
                sum,
            );
            Ok(sum)
        },
    )?;
    let secondary_margin = combine(
        "secondary structure margin",
        [&long_term, &fixed],
        |[lt, f]| {
            let margin = num::sub(lt, f)?;
            ex.step(
                "Secondary structure margin",
                format!("equity + non-current liabilities − fixed assets = {lt} − {f}"),
                margin,
            );
            Ok(margin)
        },
    )?;
    let primary_coverage = quotient(
        "Primary fixed-asset coverage",
        "equity / fixed assets",
        &equity,
        &fixed,
    )
    .evaluate(&mut ex)?;
    let secondary_coverage = quotient(
        "Secondary fixed-asset coverage",
        "(equity + non-current liabilities) / fixed assets",
        &long_term,
        &fixed,
    )
    .evaluate(&mut ex)?;

    let mut nc = NotComputedList::default();
    let output = CapitalStructureOutput {
        net_financial_position: nc.amount("/net_financial_position", &nfp),
        debt_to_equity: nc.ratio("/debt_to_equity", debt_to_equity),
        nfp_to_equity: nc.ratio("/nfp_to_equity", nfp_to_equity),
        equity_ratio: nc.ratio("/equity_ratio", equity_ratio),
        primary_structure_margin: nc.amount("/primary_structure_margin", &primary_margin),
        secondary_structure_margin: nc.amount("/secondary_structure_margin", &secondary_margin),
        primary_fixed_asset_coverage: nc.ratio("/primary_fixed_asset_coverage", primary_coverage),
        secondary_fixed_asset_coverage: nc
            .ratio("/secondary_fixed_asset_coverage", secondary_coverage),
        not_computed: Vec::new(),
    };
    Ok((
        CapitalStructureOutput {
            not_computed: nc.finish(CapitalStructure::ID)?,
            ..output
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
        composition stated. Every amount is optional: an output whose amounts are not all \
        given is null and listed in not_computed with the inputs it lacks; a ratio that is not \
        meaningful is reported as not defined with the reason.";
    const VERSION: &'static str = "2";
    type Input = CapitalStructureInput;
    type Output = CapitalStructureOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        capital_structure(input)
    }

    /// `other_items` not given: none (R13). Reported, and the canonical input writes the empty
    /// list, so the hash is that of version 1.
    fn apply_defaults(input: &mut Self::Input) -> Vec<DefaultApplied> {
        defaults(input)
    }
}

/// The documented defaults of an input (R13): `other_items` of each net financial position read.
fn defaults(input: &CapitalStructureInput) -> Vec<DefaultApplied> {
    fn nfp(b: &Option<CapitalStructureBalances>) -> Option<&NfpComposition> {
        b.as_ref().and_then(|b| b.net_financial_position.as_ref())
    }
    let opening = match input.balances {
        Balances::Average => nfp(&input.opening),
        Balances::Closing => None,
    };
    other_items_defaults([("closing", nfp(&input.closing)), ("opening", opening)])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ratios::NfpOtherItem;

    fn d(v: i64) -> Option<Decimal> {
        Some(Decimal::from(v))
    }

    fn balances(equity: i64) -> CapitalStructureBalances {
        CapitalStructureBalances {
            equity: d(equity),
            total_assets: d(6000),
            fixed_assets: d(3500),
            non_current_liabilities: d(1800),
            net_financial_position: Some(NfpComposition {
                gross_debt: d(2200),
                cash: d(300),
                other_items: Some(vec![NfpOtherItem {
                    name: "lease liabilities".into(),
                    amount: d(150),
                }]),
            }),
        }
    }

    fn input(equity: i64) -> CapitalStructureInput {
        CapitalStructureInput {
            balances: Balances::Closing,
            closing: Some(balances(equity)),
            opening: None,
        }
    }

    #[test]
    fn secondary_margin_is_primary_plus_non_current_liabilities() {
        let (out, _) = capital_structure(&input(2000)).unwrap();
        assert_eq!(
            out.secondary_structure_margin.unwrap(),
            out.primary_structure_margin.unwrap() + Decimal::from(1800)
        );
        assert!(out.not_computed.is_empty());
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

    #[test]
    fn without_non_current_liabilities_the_secondary_outputs_are_not_computed() {
        let mut i = input(2000);
        i.closing.as_mut().unwrap().non_current_liabilities = None;
        let (out, ex) = capital_structure(&i).unwrap();
        assert_eq!(out.secondary_structure_margin, None);
        assert!(out.primary_structure_margin.is_some());
        let listed: Vec<_> = out.not_computed.iter().map(|e| e.output.as_str()).collect();
        assert_eq!(
            listed,
            [
                "/secondary_structure_margin",
                "/secondary_fixed_asset_coverage"
            ]
        );
        assert!(!ex.steps.iter().any(|s| s.label.starts_with("Secondary")));
    }
}
