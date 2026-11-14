//! `ratios.profitability`: does the company earn enough on what it uses? (SPEC-0007)

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    composite, defined_amount, given, over_equity, over_positive, quotient, require_definition,
    BalanceBasis, Balances, Given, NotComputedList, Ratio, RatioValue, MISSING_CONVENTION,
};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::not_computed::NotComputedEntry;
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

/// Flows of the period, in currency units. Every amount is optional: absent or null means not
/// given, and the ratios that need it are not computed (never taken as zero).
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfitabilityIncome {
    /// Revenue of the period, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revenue: Option<Decimal>,
    /// EBITDA of the period, in currency units, as defined in `ebitda_definition` (the EBITDA
    /// margin needs both).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ebitda: Option<Decimal>,
    /// EBIT (operating result) of the period, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ebit: Option<Decimal>,
    /// Net income of the period, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_income: Option<Decimal>,
}

/// Balance-sheet amounts at one date, in currency units. Every amount is optional: absent or
/// null means not given, and the ratios that need it are not computed.
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfitabilityBalances {
    /// Shareholders' equity, in currency units. May be negative.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub equity: Option<Decimal>,
    /// Total assets, in currency units.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub total_assets: Option<Decimal>,
    /// Operating invested capital, in currency units. Used when `operating_return_base` is
    /// `invested_capital` (ROI and capital turnover); not used otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invested_capital: Option<Decimal>,
    /// Debt D of the leverage form, in currency units, on the basis chosen in
    /// `leverage.debt_basis`. Used when `leverage` is given; not used otherwise.
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
    /// Basis of D: `gross_financial_debt` or `net_financial_position`. Required when `leverage`
    /// is given, no default.
    pub debt_basis: DebtBasis,
    /// Financial charges of the period, in currency units; i = financial charges / D. Absent
    /// or null: not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub financial_charges: Option<Decimal>,
    /// Income taxes of the period, in currency units; t = taxes / pre-tax income. Absent or
    /// null: not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub taxes: Option<Decimal>,
    /// Income before taxes of the period, in currency units. Absent or null: not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_tax_income: Option<Decimal>,
}

/// Input of `ratios.profitability` (version 2). Only `balances` and `operating_return_base`
/// are required; each ratio is computed when its own amounts are given (SPEC-0007 R7).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProfitabilityInput {
    /// Balances used by the ratios: `closing`, or `average` of opening and closing.
    pub balances: Balances,
    /// Capital on which ROI and the capital turnover are measured.
    pub operating_return_base: OperatingReturnBase,
    /// How EBITDA was measured, in words (for example "reported EBITDA, no adjustments");
    /// echoed in the conventions. Absent or null: the EBITDA margin is not computed; given
    /// blank: refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ebitda_definition: Option<String>,
    /// Flows of the period. Absent or null: no flow given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub income_statement: Option<ProfitabilityIncome>,
    /// Balance sheet at the end of the period. Absent or null: no amount given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub closing: Option<ProfitabilityBalances>,
    /// Balance sheet at the start of the period, used with average balances; not used with
    /// closing balances. Absent with average balances: the balance-sheet ratios are not
    /// computed.
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
    /// Outputs not computed because an input is missing, in the order of the fields above,
    /// each with the JSON pointers of the inputs it lacks (SPEC-0007 R7).
    pub not_computed: Vec<NotComputedEntry>,
}

pub fn profitability(input: &ProfitabilityInput) -> Result<(ProfitabilityOutput, Explain)> {
    let mut ex = Explain::new(
        Profitability::ID,
        "ROE = net income / equity; ROI = EBIT / operating return base; ROS = EBIT / revenue; \
         capital turnover = revenue / operating return base; ROE = net margin × asset turnover × \
         equity multiplier",
    );
    let definition = input.ebitda_definition.as_deref();
    require_definition("ebitda_definition", definition)?;
    let basis = BalanceBasis::new(input.balances, input.opening.is_some(), &mut ex);
    let base_name = match input.operating_return_base {
        OperatingReturnBase::InvestedCapital => "invested capital",
        OperatingReturnBase::TotalAssets => "total assets",
    };
    ex.convention(format!("operating return base: {base_name}"));
    let empty = ProfitabilityBalances::default();
    let close = input.closing.as_ref().unwrap_or(&empty);
    let open = input.opening.as_ref();
    let invested_capital_given =
        close.invested_capital.is_some() || open.is_some_and(|o| o.invested_capital.is_some());
    if input.operating_return_base == OperatingReturnBase::TotalAssets && invested_capital_given {
        ex.convention("invested capital given and not used");
    }
    let is = input.income_statement.clone().unwrap_or_default();
    match definition {
        Some(d) => ex.convention(format!("EBITDA as given: {d}")),
        None if is.ebitda.is_some() => ex.convention(
            "EBITDA given without ebitda_definition: the EBITDA margin is not computed",
        ),
        None => {}
    }
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
    ex.convention(MISSING_CONVENTION);
    let debt_given = close.debt.is_some() || open.is_some_and(|o| o.debt.is_some());
    if input.leverage.is_none() && debt_given {
        ex.convention("debt given and not used (no leverage inputs)");
    }

    let equity = basis.amount(
        &mut ex,
        "equity",
        "equity",
        close.equity,
        open.and_then(|o| o.equity),
    )?;
    let assets = basis.amount(
        &mut ex,
        "total assets",
        "total_assets",
        close.total_assets,
        open.and_then(|o| o.total_assets),
    )?;
    let base: Given = match input.operating_return_base {
        OperatingReturnBase::TotalAssets => assets.clone(),
        OperatingReturnBase::InvestedCapital => basis.amount(
            &mut ex,
            "invested capital",
            "invested_capital",
            close.invested_capital,
            open.and_then(|o| o.invested_capital),
        )?,
    };

    let revenue = given("revenue", is.revenue, "/income_statement/revenue");
    let ebitda = defined_amount(
        "EBITDA",
        is.ebitda,
        "/income_statement/ebitda",
        definition,
        "/ebitda_definition",
    );
    let ebit = given("EBIT", is.ebit, "/income_statement/ebit");
    let net_income = given("net income", is.net_income, "/income_statement/net_income");

    let mut nc = NotComputedList::default();
    let roe = over_equity("ROE", "net income / equity", &net_income, &equity).evaluate(&mut ex)?;
    let roi_formula = match input.operating_return_base {
        OperatingReturnBase::InvestedCapital => "EBIT / invested capital",
        OperatingReturnBase::TotalAssets => "EBIT / total assets",
    };
    let roi = match input.operating_return_base {
        OperatingReturnBase::InvestedCapital => over_positive("ROI", roi_formula, &ebit, &base),
        OperatingReturnBase::TotalAssets => quotient("ROI", roi_formula, &ebit, &base),
    }
    .evaluate(&mut ex)?;
    let roa = quotient("ROA", "EBIT / total assets", &ebit, &assets).evaluate(&mut ex)?;
    let ros = quotient("ROS", "EBIT / revenue", &ebit, &revenue).evaluate(&mut ex)?;
    let ebitda_margin =
        quotient("EBITDA margin", "EBITDA / revenue", &ebitda, &revenue).evaluate(&mut ex)?;
    let ebit_margin =
        quotient("EBIT margin", "EBIT / revenue", &ebit, &revenue).evaluate(&mut ex)?;
    let turnover_formula = match input.operating_return_base {
        OperatingReturnBase::InvestedCapital => "revenue / invested capital",
        OperatingReturnBase::TotalAssets => "revenue / total assets",
    };
    let capital_turnover =
        quotient("Capital turnover", turnover_formula, &revenue, &base).evaluate(&mut ex)?;

    let net_margin =
        quotient("Net margin", "net income / revenue", &net_income, &revenue).evaluate(&mut ex)?;
    let asset_turnover = quotient(
        "Asset turnover",
        "revenue / total assets",
        &revenue,
        &assets,
    )
    .evaluate(&mut ex)?;
    let equity_multiplier = over_equity(
        "Equity multiplier",
        "total assets / equity",
        &assets,
        &equity,
    )
    .evaluate(&mut ex)?;
    let three_factor_roe = composite(
        &mut ex,
        "ROE, three-factor decomposition",
        &[
            ("net margin", &net_margin),
            ("asset turnover", &asset_turnover),
            ("equity multiplier", &equity_multiplier),
        ],
        |ex, f| {
            let value = num::mul(num::mul(f[0], f[1])?, f[2])?;
            ex.step(
                "ROE, three-factor decomposition",
                format!(
                    "net margin × asset turnover × equity multiplier = {} × {} × {}",
                    f[0], f[1], f[2]
                ),
                value,
            );
            Ok(value)
        },
    )?;
    let roe_leverage = leverage_form(input, close, &basis, &mut ex, &roi, &equity)?;

    let output = ProfitabilityOutput {
        roe: nc.ratio("/roe", roe),
        roi: nc.ratio("/roi", roi),
        roa: nc.ratio("/roa", roa),
        ros: nc.ratio("/ros", ros),
        ebitda_margin: nc.ratio("/ebitda_margin", ebitda_margin),
        ebit_margin: nc.ratio("/ebit_margin", ebit_margin),
        capital_turnover: nc.ratio("/capital_turnover", capital_turnover),
        roe_three_factor: RoeThreeFactor {
            net_margin: nc.ratio("/roe_three_factor/net_margin", net_margin),
            asset_turnover: nc.ratio("/roe_three_factor/asset_turnover", asset_turnover),
            equity_multiplier: nc.ratio("/roe_three_factor/equity_multiplier", equity_multiplier),
            roe: nc.ratio("/roe_three_factor/roe", three_factor_roe),
        },
        roe_leverage: RoeLeverage {
            roi: nc.ratio("/roe_leverage/roi", roe_leverage.roi),
            cost_of_debt: nc.ratio("/roe_leverage/cost_of_debt", roe_leverage.cost_of_debt),
            debt_to_equity: nc.ratio("/roe_leverage/debt_to_equity", roe_leverage.debt_to_equity),
            leverage_effect: nc.ratio(
                "/roe_leverage/leverage_effect",
                roe_leverage.leverage_effect,
            ),
            pre_tax_roe: nc.ratio("/roe_leverage/pre_tax_roe", roe_leverage.pre_tax_roe),
            tax_rate: nc.ratio("/roe_leverage/tax_rate", roe_leverage.tax_rate),
            roe: nc.ratio("/roe_leverage/roe", roe_leverage.roe),
        },
        not_computed: Vec::new(),
    };
    Ok((
        ProfitabilityOutput {
            not_computed: nc.finish(Profitability::ID)?,
            ..output
        },
        ex,
    ))
}

const NO_LEVERAGE: &str = "leverage inputs not given";

/// ROE = [ROI + (ROI − i) × D/E] × (1 − t), or not defined without the leverage inputs.
fn leverage_form(
    input: &ProfitabilityInput,
    close: &ProfitabilityBalances,
    basis: &BalanceBasis,
    ex: &mut Explain,
    roi: &RatioValue,
    equity: &Given,
) -> Result<RoeLeverage> {
    let Some(lev) = &input.leverage else {
        ex.convention(format!("ROE, leverage form not defined: {NO_LEVERAGE}"));
        // Version 1 reason kept; the block is also listed as missing in `not_computed`.
        let none = || RatioValue::block_not_given(NO_LEVERAGE, "/leverage");
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
    let debt = basis.amount(
        ex,
        debt_name,
        "debt",
        close.debt,
        input.opening.as_ref().and_then(|o| o.debt),
    )?;
    let cost_of_debt = quotient(
        "Cost of debt i",
        "financial charges / D",
        &input_of(
            "financial charges",
            lev.financial_charges,
            "financial_charges",
        ),
        &debt,
    )
    .evaluate(ex)?;
    let debt_to_equity =
        over_equity("D/E, leverage form", "D / equity", &debt, equity).evaluate(ex)?;
    let tax_rate = quotient(
        "Effective tax rate t",
        "taxes / pre-tax income",
        &input_of("taxes", lev.taxes, "taxes"),
        &input_of("pre-tax income", lev.pre_tax_income, "pre_tax_income"),
    )
    .evaluate(ex)?;
    let parts = [
        ("ROI", roi),
        ("cost of debt", &cost_of_debt),
        ("D/E", &debt_to_equity),
    ];
    let leverage_effect = composite(ex, "Leverage effect", &parts, |ex, v| {
        let (r, i, de) = (v[0], v[1], v[2]);
        let effect = num::mul(num::sub(r, i)?, de)?;
        ex.step(
            "Leverage effect",
            format!("(ROI − i) × D/E = ({r} − {i}) × {de}"),
            effect,
        );
        Ok(effect)
    })?;
    let pre_tax_roe = composite(ex, "Pre-tax ROE, leverage form", &parts, |ex, v| {
        // The parts are those of the leverage effect, so it is defined here.
        let effect = leverage_effect.value.ok_or(FairmarkError::Internal(
            "leverage effect not defined with its parts",
        ))?;
        let pre_tax = num::add(v[0], effect)?;
        ex.step(
            "Pre-tax ROE, leverage form",
            format!("ROI + leverage effect = {} + {effect}", v[0]),
            pre_tax,
        );
        Ok(pre_tax)
    })?;
    let roe = composite(
        ex,
        "ROE, leverage form",
        &[("pre-tax ROE", &pre_tax_roe), ("tax rate", &tax_rate)],
        |ex, v| {
            let value = num::mul(v[0], num::sub(Decimal::ONE, v[1])?)?;
            ex.step(
                "ROE, leverage form",
                format!("pre-tax ROE × (1 − t) = {} × (1 − {})", v[0], v[1]),
                value,
            );
            Ok(value)
        },
    )?;
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

/// An amount of the `leverage` block.
fn input_of(name: &'static str, value: Option<Decimal>, field: &str) -> Given {
    given(name, value, &format!("/leverage/{field}"))
}

/// `ratios.profitability`: see the methodology page.
pub struct Profitability;

impl Method for Profitability {
    const ID: &'static str = "ratios.profitability";
    const TITLE: &'static str = "Ratios — profitability";
    const DESCRIPTION: &'static str =
        "ROE, ROI, ROA, ROS, EBITDA and EBIT margins, capital turnover and the ROE \
        decomposition in three factors and in the leverage form, on closing or average \
        balances. Every amount is optional: a ratio whose amounts are not all given is null \
        and listed in not_computed with the inputs it lacks; a ratio that is not meaningful \
        is reported as not defined with the reason.";
    const VERSION: &'static str = "2";
    type Input = ProfitabilityInput;
    type Output = ProfitabilityOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        crate::ratios::require_definition("ebitda_definition", input.ebitda_definition.as_deref())
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        profitability(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Option<Decimal> {
        Some(Decimal::from(v))
    }

    fn input() -> ProfitabilityInput {
        ProfitabilityInput {
            balances: Balances::Closing,
            operating_return_base: OperatingReturnBase::InvestedCapital,
            ebitda_definition: Some("reported".into()),
            income_statement: Some(ProfitabilityIncome {
                revenue: d(5000),
                ebitda: d(800),
                ebit: d(500),
                net_income: d(260),
            }),
            closing: Some(ProfitabilityBalances {
                equity: d(2000),
                total_assets: d(6000),
                invested_capital: d(4000),
                debt: d(2000),
            }),
            opening: None,
            leverage: Some(LeverageInputs {
                debt_basis: DebtBasis::NetFinancialPosition,
                financial_charges: d(100),
                taxes: d(140),
                pre_tax_income: d(400),
            }),
        }
    }

    fn closing(i: &mut ProfitabilityInput) -> &mut ProfitabilityBalances {
        i.closing.as_mut().unwrap()
    }

    #[test]
    fn roi_is_ros_times_capital_turnover() {
        let (out, _) = profitability(&input()).unwrap();
        let product = out.ros.value.unwrap() * out.capital_turnover.value.unwrap();
        assert_eq!(product, out.roi.value.unwrap());
        assert!(out.not_computed.is_empty());
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
        closing(&mut i).equity = Some(Decimal::ZERO);
        let (out, _) = profitability(&i).unwrap();
        assert_eq!(out.roe.not_defined.as_deref(), Some("equity is zero"));
        assert!(out.roi.value.is_some());
    }

    #[test]
    fn blank_ebitda_definition_is_refused() {
        let mut i = input();
        i.ebitda_definition = Some(" ".into());
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
    fn average_without_opening_invested_capital_leaves_roi_not_computed() {
        let mut i = input();
        i.balances = Balances::Average;
        i.opening = Some(ProfitabilityBalances {
            equity: d(1800),
            total_assets: d(5600),
            invested_capital: None,
            debt: d(1800),
        });
        let (out, _) = profitability(&i).unwrap();
        assert_eq!(out.roi.value, None);
        assert_eq!(out.not_computed[0].output, "/roi");
        assert_eq!(out.not_computed[0].missing, ["/opening/invested_capital"]);
        assert!(out.roe.value.is_some());
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
    fn without_leverage_inputs_the_form_is_not_defined_and_listed() {
        let mut i = input();
        i.leverage = None;
        let (out, ex) = profitability(&i).unwrap();
        assert_eq!(
            out.roe_leverage.roe.not_defined.as_deref(),
            Some("leverage inputs not given")
        );
        assert_eq!(out.not_computed.len(), 7);
        assert!(out.not_computed.iter().all(|e| e.missing == ["/leverage"]));
        assert!(ex
            .conventions
            .iter()
            .any(|c| c == "debt given and not used (no leverage inputs)"));
    }

    #[test]
    fn leverage_without_debt_leaves_the_ratios_on_debt_not_computed() {
        let mut i = input();
        closing(&mut i).debt = None;
        let (out, _) = profitability(&i).unwrap();
        let l = &out.roe_leverage;
        assert!(l.roi.value.is_some() && l.tax_rate.value.is_some());
        assert_eq!(l.cost_of_debt.value, None);
        assert_eq!(l.roe.value, None);
        assert_eq!(out.not_computed.len(), 5);
    }

    #[test]
    fn negative_invested_capital_leaves_roi_not_defined() {
        let mut i = input();
        closing(&mut i).invested_capital = d(-500);
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
        assert!(out.not_computed.is_empty());
    }

    #[test]
    fn ebitda_without_definition_leaves_only_the_margin_not_computed() {
        let mut i = input();
        i.ebitda_definition = None;
        let (out, ex) = profitability(&i).unwrap();
        assert_eq!(out.ebitda_margin.value, None);
        assert_eq!(out.not_computed.len(), 1);
        assert_eq!(out.not_computed[0].missing, ["/ebitda_definition"]);
        assert!(ex
            .conventions
            .iter()
            .any(|c| c.starts_with("EBITDA given without")));
    }
}
