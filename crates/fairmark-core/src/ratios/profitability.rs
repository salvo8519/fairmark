//! `ratios.profitability`: does the company earn enough on what it uses? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    all_defined, over_equity, quotient, require_definition, term, BalanceBasis, Balances, Ratio,
    RatioValue, Term,
};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num;

/// The capital on which the operating return (ROI) and the capital turnover are measured.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OperatingReturnBase {
    /// Operating invested capital (`invested_capital` in the balance sheets).
    InvestedCapital,
    /// Total assets (`total_assets` in the balance sheets).
    TotalAssets,
}

/// Flows of the period, in currency units.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfitabilityIncome {
    /// Revenue of the period, in currency units.
    pub revenue: Decimal,
    /// EBITDA of the period, in currency units, as defined in `ebitda_definition`.
    pub ebitda: Decimal,
    /// EBIT (operating result) of the period, in currency units.
    pub ebit: Decimal,
    /// Net income of the period, in currency units.
    pub net_income: Decimal,
}

/// Balance-sheet amounts at one date, in currency units.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfitabilityBalances {
    /// Shareholders' equity, in currency units. May be negative.
    pub equity: Decimal,
    /// Total assets, in currency units.
    pub total_assets: Decimal,
    /// Operating invested capital, in currency units. Required when `operating_return_base` is
    /// `invested_capital`; not used otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invested_capital: Option<Decimal>,
}

/// Input of `ratios.profitability`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfitabilityInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// Capital on which ROI and the capital turnover are measured.
    pub operating_return_base: OperatingReturnBase,
    /// How EBITDA was measured, in words (for example "reported EBITDA, no adjustments");
    /// echoed in the conventions.
    pub ebitda_definition: String,
    /// Flows of the period.
    pub income_statement: ProfitabilityIncome,
    /// Balance sheet at the end of the period.
    pub closing: ProfitabilityBalances,
    /// Balance sheet at the start of the period. Required with average balances; not used
    /// with closing balances.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opening: Option<ProfitabilityBalances>,
}

/// ROE = net margin × asset turnover × equity multiplier (DuPont).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RoeThreeFactor {
    /// Net income / revenue, decimal fraction.
    pub net_margin: RatioValue,
    /// Revenue / total assets, times per period.
    pub asset_turnover: RatioValue,
    /// Total assets / equity, multiple.
    pub equity_multiplier: RatioValue,
    /// Product of the three factors, decimal fraction; equal to ROE.
    pub roe: RatioValue,
}

/// Output of `ratios.profitability`. Fractions: 0.13 means 13%.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ProfitabilityOutput {
    /// Net income / equity, decimal fraction.
    pub roe: RatioValue,
    /// EBIT / operating return base, decimal fraction.
    pub roi: RatioValue,
    /// EBIT / revenue, decimal fraction.
    pub ros: RatioValue,
    /// EBITDA / revenue, decimal fraction.
    pub ebitda_margin: RatioValue,
    /// EBIT / revenue, decimal fraction.
    pub ebit_margin: RatioValue,
    /// Revenue / operating return base, times per period (ROI = ROS × capital turnover).
    pub capital_turnover: RatioValue,
    /// Three-factor decomposition of ROE.
    pub roe_three_factor: RoeThreeFactor,
}

pub fn profitability(input: &ProfitabilityInput) -> Result<(ProfitabilityOutput, Explain)> {
    let mut ex = Explain::new(
        Profitability::ID,
        "ROE = net income / equity; ROI = EBIT / operating return base; ROS = EBIT / revenue; \
         capital turnover = revenue / operating return base; ROE = net margin × asset turnover × \
         equity multiplier",
    );
    require_definition("ebitda_definition", &input.ebitda_definition)?;
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex)?;
    let base_name = match input.operating_return_base {
        OperatingReturnBase::InvestedCapital => "invested capital",
        OperatingReturnBase::TotalAssets => "total assets",
    };
    ex.convention(format!("operating return base: {base_name}"));
    let invested_capital_given = input.closing.invested_capital.is_some()
        || input
            .opening
            .as_ref()
            .is_some_and(|o| o.invested_capital.is_some());
    if input.operating_return_base == OperatingReturnBase::TotalAssets && invested_capital_given {
        ex.convention("invested capital given and not used");
    }
    ex.convention(format!("EBITDA as given: {}", input.ebitda_definition));
    ex.convention("ROS = EBIT / revenue, equal to the EBIT margin");
    ex.convention(
        "three-factor ROE on total assets (asset turnover and equity multiplier), whatever the \
         operating return base",
    );
    ex.convention("a ratio with a zero denominator is not defined");
    ex.convention("a ratio over equity is not defined when equity is zero or negative");

    let open = input.opening.as_ref();
    let equity = basis.resolve(
        &mut ex,
        "equity",
        input.closing.equity,
        open.map(|o| o.equity),
    )?;
    let assets = basis.resolve(
        &mut ex,
        "total assets",
        input.closing.total_assets,
        open.map(|o| o.total_assets),
    )?;
    let base: Term = match input.operating_return_base {
        OperatingReturnBase::TotalAssets => term("total assets", assets.value),
        OperatingReturnBase::InvestedCapital => {
            let missing = || {
                FairmarkError::InvalidInput(
                    "`invested_capital` is required in every balance sheet given when \
                     `operating_return_base` is `invested_capital`"
                        .into(),
                )
            };
            let closing = input.closing.invested_capital.ok_or_else(missing)?;
            let opening = match (input.balances, open) {
                (Balances::Average, Some(o)) => Some(o.invested_capital.ok_or_else(missing)?),
                _ => None,
            };
            basis.resolve(&mut ex, "invested capital", closing, opening)?
        }
    };

    let is = &input.income_statement;
    let revenue = term("revenue", is.revenue);
    let ebitda = term("EBITDA", is.ebitda);
    let ebit = term("EBIT", is.ebit);
    let net_income = term("net income", is.net_income);

    let roe = over_equity("ROE", "net income / equity", net_income, equity).evaluate(&mut ex)?;
    let roi_formula = match input.operating_return_base {
        OperatingReturnBase::InvestedCapital => "EBIT / invested capital",
        OperatingReturnBase::TotalAssets => "EBIT / total assets",
    };
    let roi = quotient("ROI", roi_formula, ebit, base).evaluate(&mut ex)?;
    let ros = quotient("ROS", "EBIT / revenue", ebit, revenue).evaluate(&mut ex)?;
    let ebitda_margin =
        quotient("EBITDA margin", "EBITDA / revenue", ebitda, revenue).evaluate(&mut ex)?;
    let ebit_margin = quotient("EBIT margin", "EBIT / revenue", ebit, revenue).evaluate(&mut ex)?;
    let turnover_formula = match input.operating_return_base {
        OperatingReturnBase::InvestedCapital => "revenue / invested capital",
        OperatingReturnBase::TotalAssets => "revenue / total assets",
    };
    let capital_turnover =
        quotient("Capital turnover", turnover_formula, revenue, base).evaluate(&mut ex)?;

    let net_margin =
        quotient("Net margin", "net income / revenue", net_income, revenue).evaluate(&mut ex)?;
    let asset_turnover =
        quotient("Asset turnover", "revenue / total assets", revenue, assets).evaluate(&mut ex)?;
    let equity_multiplier =
        over_equity("Equity multiplier", "total assets / equity", assets, equity)
            .evaluate(&mut ex)?;
    let three_factor_roe = match all_defined(&[
        ("net margin", &net_margin),
        ("asset turnover", &asset_turnover),
        ("equity multiplier", &equity_multiplier),
    ]) {
        Ok(f) => {
            let value = num::mul(num::mul(f[0], f[1])?, f[2])?;
            ex.step(
                "ROE, three-factor decomposition",
                format!(
                    "net margin × asset turnover × equity multiplier = {} × {} × {}",
                    f[0], f[1], f[2]
                ),
                value,
            );
            RatioValue::defined(value)
        }
        Err(reason) => RatioValue::not_defined(&mut ex, "ROE, three-factor decomposition", reason),
    };

    Ok((
        ProfitabilityOutput {
            roe,
            roi,
            ros,
            ebitda_margin,
            ebit_margin,
            capital_turnover,
            roe_three_factor: RoeThreeFactor {
                net_margin,
                asset_turnover,
                equity_multiplier,
                roe: three_factor_roe,
            },
        },
        ex,
    ))
}

/// `ratios.profitability`: see the methodology page.
pub struct Profitability;

impl Method for Profitability {
    const ID: &'static str = "ratios.profitability";
    const TITLE: &'static str = "Ratios — profitability";
    const DESCRIPTION: &'static str =
        "ROE, ROI, ROS, EBITDA and EBIT margins, capital turnover and the three-factor ROE \
        decomposition, on closing or average balances. A ratio that cannot be computed is \
        reported as not defined with the reason.";
    const VERSION: &'static str = "1";
    type Input = ProfitabilityInput;
    type Output = ProfitabilityOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        profitability(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    fn input() -> ProfitabilityInput {
        ProfitabilityInput {
            balances: Balances::Closing,
            operating_return_base: OperatingReturnBase::InvestedCapital,
            ebitda_definition: "reported".into(),
            income_statement: ProfitabilityIncome {
                revenue: d(5000),
                ebitda: d(800),
                ebit: d(500),
                net_income: d(260),
            },
            closing: ProfitabilityBalances {
                equity: d(2000),
                total_assets: d(6000),
                invested_capital: Some(d(4000)),
            },
            opening: None,
        }
    }

    #[test]
    fn roi_is_ros_times_capital_turnover() {
        let (out, _) = profitability(&input()).unwrap();
        let product = out.ros.value.unwrap() * out.capital_turnover.value.unwrap();
        assert_eq!(product, out.roi.value.unwrap());
    }

    #[test]
    fn three_factor_product_is_roe() {
        let (out, _) = profitability(&input()).unwrap();
        let diff = out.roe_three_factor.roe.value.unwrap() - out.roe.value.unwrap();
        assert!(diff.abs() < Decimal::new(1, 25));
    }

    #[test]
    fn zero_equity_is_not_defined() {
        let mut i = input();
        i.closing.equity = Decimal::ZERO;
        let (out, _) = profitability(&i).unwrap();
        assert_eq!(out.roe.not_defined.as_deref(), Some("equity is zero"));
        assert!(out.roi.value.is_some());
    }

    #[test]
    fn blank_ebitda_definition_is_refused() {
        let mut i = input();
        i.ebitda_definition = " ".into();
        assert_eq!(profitability(&i).unwrap_err().code(), "FM-E001");
    }

    #[test]
    fn unused_invested_capital_is_reported() {
        let mut i = input();
        i.operating_return_base = OperatingReturnBase::TotalAssets;
        let (_, ex) = profitability(&i).unwrap();
        assert!(ex
            .conventions
            .iter()
            .any(|c| c == "invested capital given and not used"));
    }

    #[test]
    fn average_needs_invested_capital_in_opening() {
        let mut i = input();
        i.balances = Balances::Average;
        i.opening = Some(ProfitabilityBalances {
            equity: d(1800),
            total_assets: d(5600),
            invested_capital: None,
        });
        assert_eq!(profitability(&i).unwrap_err().code(), "FM-E001");
    }
}
