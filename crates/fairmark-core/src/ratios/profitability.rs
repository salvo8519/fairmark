//! `ratios.profitability`: does the company earn enough on what it uses? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    all_defined, over_equity, over_positive, quotient, require_definition, term, BalanceBasis,
    Balances, Ratio, RatioValue, Term,
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
    /// Debt D of the leverage form, in currency units, on the basis chosen in
    /// `leverage.debt_basis`. Required when `leverage` is given; not used otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt: Option<Decimal>,
}

/// What D measures in the leverage form of the ROE decomposition. No default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum DebtBasis {
    /// Gross financial debt.
    GrossFinancialDebt,
    /// Net financial position (gross debt minus cash and other items, as the analyst defines it).
    NetFinancialPosition,
}

/// Inputs of the leverage form ROE = [ROI + (ROI − i) × D/E] × (1 − t). The debt D itself is
/// given in the balance sheets (`debt`), so the balance convention applies to it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LeverageInputs {
    /// Basis of D: `gross_financial_debt` or `net_financial_position`.
    pub debt_basis: DebtBasis,
    /// Financial charges of the period, in currency units; i = financial charges / D.
    pub financial_charges: Decimal,
    /// Income taxes of the period, in currency units; t = taxes / pre-tax income.
    pub taxes: Decimal,
    /// Income before taxes of the period, in currency units.
    pub pre_tax_income: Decimal,
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
    /// Inputs of the leverage form of the ROE decomposition. Absent: the leverage form is not
    /// defined and the other ratios are computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub leverage: Option<LeverageInputs>,
}

/// ROE = [ROI + (ROI − i) × D/E] × (1 − t).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct RoeLeverage {
    /// ROI of the group (EBIT / operating return base), decimal fraction.
    pub roi: RatioValue,
    /// i = financial charges / D, decimal fraction.
    pub cost_of_debt: RatioValue,
    /// D / equity, multiple.
    pub debt_to_equity: RatioValue,
    /// (ROI − i) × D/E, decimal fraction.
    pub leverage_effect: RatioValue,
    /// ROI + leverage effect, decimal fraction.
    pub pre_tax_roe: RatioValue,
    /// t = taxes / pre-tax income (effective tax rate), decimal fraction.
    pub tax_rate: RatioValue,
    /// Pre-tax ROE × (1 − t), decimal fraction.
    pub roe: RatioValue,
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
    /// EBIT / total assets, decimal fraction.
    pub roa: RatioValue,
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
    /// Leverage form of the ROE decomposition.
    pub roe_leverage: RoeLeverage,
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
    ex.convention("ROA = EBIT / total assets");
    ex.convention("a ratio with a zero denominator is not defined");
    ex.convention("a ratio over equity is not defined when equity is zero or negative");
    if input.operating_return_base == OperatingReturnBase::InvestedCapital {
        ex.convention("ROI is not defined when invested capital is zero or negative");
    }
    let debt_given =
        input.closing.debt.is_some() || input.opening.as_ref().is_some_and(|o| o.debt.is_some());
    if input.leverage.is_none() && debt_given {
        ex.convention("debt given and not used (no leverage inputs)");
    }

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
    let roi = match input.operating_return_base {
        OperatingReturnBase::InvestedCapital => over_positive("ROI", roi_formula, ebit, base),
        OperatingReturnBase::TotalAssets => quotient("ROI", roi_formula, ebit, base),
    }
    .evaluate(&mut ex)?;
    let roa = quotient("ROA", "EBIT / total assets", ebit, assets).evaluate(&mut ex)?;
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
    let roe_leverage = leverage_form(input, &basis, &mut ex, &roi, equity)?;

    Ok((
        ProfitabilityOutput {
            roe,
            roi,
            roa,
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
            roe_leverage,
        },
        ex,
    ))
}

const NO_LEVERAGE: &str = "leverage inputs not given";

/// ROE = [ROI + (ROI − i) × D/E] × (1 − t), or not defined without the leverage inputs.
fn leverage_form(
    input: &ProfitabilityInput,
    basis: &BalanceBasis,
    ex: &mut Explain,
    roi: &RatioValue,
    equity: Term,
) -> Result<RoeLeverage> {
    let Some(lev) = &input.leverage else {
        ex.convention(format!("ROE, leverage form not defined: {NO_LEVERAGE}"));
        let none = || RatioValue {
            value: None,
            not_defined: Some(NO_LEVERAGE.to_owned()),
        };
        return Ok(RoeLeverage {
            roi: none(),
            cost_of_debt: none(),
            debt_to_equity: none(),
            leverage_effect: none(),
            pre_tax_roe: none(),
            tax_rate: none(),
            roe: none(),
        });
    };
    let (basis_name, debt_name) = match lev.debt_basis {
        DebtBasis::GrossFinancialDebt => ("gross financial debt", "gross financial debt"),
        DebtBasis::NetFinancialPosition => ("net financial position", "net financial position"),
    };
    ex.convention(format!(
        "ROE, leverage form: ROE = [ROI + (ROI − i) × D/E] × (1 − t), D = {basis_name}, \
         i = financial charges / D, t = taxes / pre-tax income (effective tax rate), ROI on \
         the operating return base"
    ));
    let missing = || {
        FairmarkError::InvalidInput(
            "`debt` is required in every balance sheet given when `leverage` is given".into(),
        )
    };
    let closing = input.closing.debt.ok_or_else(missing)?;
    let opening = match (input.balances, input.opening.as_ref()) {
        (Balances::Average, Some(o)) => Some(o.debt.ok_or_else(missing)?),
        _ => None,
    };
    let debt = basis.resolve(ex, debt_name, closing, opening)?;
    let cost_of_debt = quotient(
        "Cost of debt i",
        "financial charges / D",
        term("financial charges", lev.financial_charges),
        debt,
    )
    .evaluate(ex)?;
    let debt_to_equity =
        over_equity("D/E, leverage form", "D / equity", debt, equity).evaluate(ex)?;
    let tax_rate = quotient(
        "Effective tax rate t",
        "taxes / pre-tax income",
        term("taxes", lev.taxes),
        term("pre-tax income", lev.pre_tax_income),
    )
    .evaluate(ex)?;
    let parts = all_defined(&[
        ("ROI", roi),
        ("cost of debt", &cost_of_debt),
        ("D/E", &debt_to_equity),
    ]);
    let (leverage_effect, pre_tax_roe) = match parts {
        Ok(v) => {
            let (r, i, de) = (v[0], v[1], v[2]);
            let effect = num::mul(num::sub(r, i)?, de)?;
            ex.step(
                "Leverage effect",
                format!("(ROI − i) × D/E = ({r} − {i}) × {de}"),
                effect,
            );
            let pre_tax = num::add(r, effect)?;
            ex.step(
                "Pre-tax ROE, leverage form",
                format!("ROI + leverage effect = {r} + {effect}"),
                pre_tax,
            );
            (RatioValue::defined(effect), RatioValue::defined(pre_tax))
        }
        Err(reason) => (
            RatioValue::not_defined(ex, "Leverage effect", reason.clone()),
            RatioValue::not_defined(ex, "Pre-tax ROE, leverage form", reason),
        ),
    };
    let roe = match all_defined(&[("pre-tax ROE", &pre_tax_roe), ("tax rate", &tax_rate)]) {
        Ok(v) => {
            let value = num::mul(v[0], num::sub(Decimal::ONE, v[1])?)?;
            ex.step(
                "ROE, leverage form",
                format!("pre-tax ROE × (1 − t) = {} × (1 − {})", v[0], v[1]),
                value,
            );
            RatioValue::defined(value)
        }
        Err(reason) => RatioValue::not_defined(ex, "ROE, leverage form", reason),
    };
    Ok(RoeLeverage {
        roi: roi.clone(),
        cost_of_debt,
        debt_to_equity,
        leverage_effect,
        pre_tax_roe,
        tax_rate,
        roe,
    })
}

/// `ratios.profitability`: see the methodology page.
pub struct Profitability;

impl Method for Profitability {
    const ID: &'static str = "ratios.profitability";
    const TITLE: &'static str = "Ratios — profitability";
    const DESCRIPTION: &'static str =
        "ROE, ROI, ROA, ROS, EBITDA and EBIT margins, capital turnover and the ROE \
        decomposition in three factors and in the leverage form, on closing or average \
        balances. A ratio that cannot be computed is \
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
                debt: Some(d(2000)),
            },
            opening: None,
            leverage: Some(LeverageInputs {
                debt_basis: DebtBasis::NetFinancialPosition,
                financial_charges: d(100),
                taxes: d(140),
                pre_tax_income: d(400),
            }),
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
            debt: Some(d(1800)),
        });
        assert_eq!(profitability(&i).unwrap_err().code(), "FM-E001");
    }

    #[test]
    fn leverage_form_equals_roe_when_capital_is_equity_plus_debt() {
        let (out, _) = profitability(&input()).unwrap();
        let l = &out.roe_leverage;
        assert_eq!(l.cost_of_debt.value, Some(Decimal::new(5, 2)));
        assert_eq!(l.tax_rate.value, Some(Decimal::new(35, 2)));
        assert_eq!(l.roe.value, out.roe.value);
    }

    #[test]
    fn without_leverage_inputs_the_form_is_not_defined() {
        let mut i = input();
        i.leverage = None;
        let (out, ex) = profitability(&i).unwrap();
        assert_eq!(
            out.roe_leverage.roe.not_defined.as_deref(),
            Some("leverage inputs not given")
        );
        assert!(ex
            .conventions
            .iter()
            .any(|c| c == "debt given and not used (no leverage inputs)"));
    }

    #[test]
    fn leverage_needs_debt() {
        let mut i = input();
        i.closing.debt = None;
        assert_eq!(profitability(&i).unwrap_err().code(), "FM-E001");
    }

    #[test]
    fn negative_invested_capital_leaves_roi_not_defined() {
        let mut i = input();
        i.closing.invested_capital = Some(d(-500));
        let (out, _) = profitability(&i).unwrap();
        assert_eq!(
            out.roi.not_defined.as_deref(),
            Some("invested capital is negative (-500)")
        );
        assert_eq!(
            out.roe_leverage.pre_tax_roe.not_defined.as_deref(),
            Some("ROI is not defined")
        );
        assert!(out.capital_turnover.value.is_some());
    }
}
