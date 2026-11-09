//! Cost of capital: CAPM with explicit premia, beta unlevering and relevering, peer betas,
//! cost of debt and WACC (SPEC-0003).
//!
//! All rates are annual effective rates, decimal fractions (`0.05` = 5%). Every formula here
//! is rational (additions, multiplications, divisions), so results are exact up to the
//! 28-digit decimal precision.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Found, Reason, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, div, mul, sub};

const ANNUAL: &str = "all rates are annual effective rates (decimal fractions)";

fn invalid(msg: impl Into<String>) -> FairmarkError {
    FairmarkError::InvalidInput(msg.into())
}

/// The JSON pointer of a field named as in the messages: `peers[0].tax_rate` is
/// `/peers/0/tax_rate`.
fn pointer(field: &str) -> String {
    format!("/{}", field.replace("].", "/").replace(['[', '.'], "/"))
}

fn check_tax_rate(t: Decimal, field: &str) -> Result<()> {
    if t < Decimal::ZERO || t >= Decimal::ONE {
        return Err(
            invalid(format!("{field} must be at least 0 and below 1 (got {t})"))
                .at(pointer(field), Reason::Range)
                .expected(["a decimal fraction at least 0 and below 1"]),
        );
    }
    Ok(())
}

fn check_leverage(de: Decimal, field: &str) -> Result<()> {
    if de < Decimal::ZERO {
        return Err(invalid(format!("{field} cannot be negative (got {de})"))
            .at(pointer(field), Reason::Sign)
            .expected(["zero or more"]));
    }
    Ok(())
}

// ---------------------------------------------------------------- beta

/// Formula linking levered and unlevered beta.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BetaFormula {
    /// Hamada: β_L = β_U + (β_U − β_D)(1 − t) D/E (debt at a fixed amount, tax shield at the
    /// cost of debt).
    Hamada,
    /// Harris-Pringle: β_L = β_U + (β_U − β_D) D/E (debt rebalanced to a target ratio).
    HarrisPringle,
}

impl BetaFormula {
    fn label(self) -> &'static str {
        match self {
            Self::Hamada => "Hamada: β_L = β_U + (β_U − β_D)(1 − t) D/E",
            Self::HarrisPringle => "Harris-Pringle: β_L = β_U + (β_U − β_D) D/E",
        }
    }

    /// Leverage multiplier: (1 − t) D/E for Hamada, D/E for Harris-Pringle.
    fn leverage(self, de: Decimal, t: Decimal) -> Result<Decimal> {
        match self {
            Self::Hamada => mul(sub(Decimal::ONE, t)?, de),
            Self::HarrisPringle => Ok(de),
        }
    }

    /// β_L from β_U.
    pub fn relever(self, bu: Decimal, de: Decimal, t: Decimal, bd: Decimal) -> Result<Decimal> {
        add(bu, mul(sub(bu, bd)?, self.leverage(de, t)?)?)
    }

    /// β_U from β_L (inverse of [`BetaFormula::relever`]).
    pub fn unlever(self, bl: Decimal, de: Decimal, t: Decimal, bd: Decimal) -> Result<Decimal> {
        let k = self.leverage(de, t)?;
        div(add(bl, mul(bd, k)?)?, add(Decimal::ONE, k)?)
    }
}

/// The relevering or unlevering calculation written out with the numbers substituted.
fn beta_formula_text(
    formula: BetaFormula,
    direction: BetaDirection,
    b: Decimal,
    k: Decimal,
    bd: Decimal,
) -> String {
    let _ = formula;
    match direction {
        BetaDirection::Relever => format!("{b} + ({b} − {bd}) × {k}"),
        BetaDirection::Unlever => format!("({b} + {bd} × {k}) / (1 + {k})"),
    }
}

/// Leverage factor written out: (1 − t) × D/E for Hamada, D/E for Harris-Pringle.
fn leverage_text(formula: BetaFormula, de: Decimal, t: Decimal) -> String {
    match formula {
        BetaFormula::Hamada => format!("(1 − {t}) × {de}"),
        BetaFormula::HarrisPringle => format!("{de} (tax rate not used)"),
    }
}

fn formula_conventions(ex: &mut Explain, formula: BetaFormula) {
    ex.convention(format!("formula: {}", formula.label()));
    if formula == BetaFormula::HarrisPringle {
        ex.convention("Harris-Pringle does not use the tax rate: the tax rates given are not used");
    }
}

/// Cost of equity by CAPM with additive premia: rf + β × ERP + CRP × λ + size + specific.
/// Building block for other methods (for example the DCF of SPEC-0001).
pub fn capm(
    rf: Decimal,
    beta: Decimal,
    erp: Decimal,
    crp_times_lambda: Decimal,
    size: Decimal,
    specific: Decimal,
) -> Result<Decimal> {
    add(
        add(add(add(rf, mul(beta, erp)?)?, crp_times_lambda)?, size)?,
        specific,
    )
}

/// WACC from the cost of equity, pre-tax cost of debt, tax rate and debt weight D/V.
/// Building block for other methods (for example the DCF of SPEC-0001).
pub fn wacc_rate(ke: Decimal, kd: Decimal, t: Decimal, debt_weight: Decimal) -> Result<Decimal> {
    let we = sub(Decimal::ONE, debt_weight)?;
    add(
        mul(we, ke)?,
        mul(debt_weight, mul(kd, sub(Decimal::ONE, t)?)?)?,
    )
}

/// Direction of a beta conversion.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum BetaDirection {
    /// The given beta is unlevered; return the levered beta.
    Relever,
    /// The given beta is levered; return the unlevered beta.
    Unlever,
}

/// Input of `coc.beta_relever`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BetaReleverInput {
    /// Beta to convert (unlevered for `relever`, levered for `unlever`), dimensionless.
    pub beta: Decimal,
    /// `relever` or `unlever`.
    pub direction: BetaDirection,
    /// Debt-to-equity ratio D/E, decimal (0.5 = debt equal to half of equity), not negative.
    pub debt_to_equity: Decimal,
    /// Tax rate of the tax shield, decimal fraction from 0 to below 1.
    pub tax_rate: Decimal,
    /// `hamada` or `harris_pringle`.
    pub formula: BetaFormula,
    /// Beta of debt, dimensionless. Default: 0 (risk-free debt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt_beta: Option<Decimal>,
}

/// Output of `coc.beta_relever`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BetaReleverOutput {
    /// The converted beta.
    pub beta: Decimal,
}

fn debt_beta_convention(ex: &mut Explain, bd: Option<Decimal>) -> Decimal {
    match bd {
        Some(b) => {
            ex.convention(format!("debt beta {b} (given)"));
            b
        }
        None => {
            ex.convention("debt beta 0 (default: risk-free debt)");
            Decimal::ZERO
        }
    }
}

/// Every input error of `beta_relever`, without computing (ADR 0014 G5).
fn check_beta_relever(input: &BetaReleverInput) -> Result<()> {
    // Every input error is reported (ADR 0014 G5), in the order of the checks.
    let mut found = Found::new();
    found.check(check_leverage(input.debt_to_equity, "debt_to_equity"));
    found.check(check_tax_rate(input.tax_rate, "tax_rate"));
    found.finish()
}

pub fn beta_relever(input: &BetaReleverInput) -> Result<(BetaReleverOutput, Explain)> {
    check_beta_relever(input)?;
    let headline = match input.direction {
        BetaDirection::Relever => "β_L = β_U + (β_U − β_D) × k",
        BetaDirection::Unlever => "β_U = (β_L + β_D × k) / (1 + k)",
    };
    let mut ex = Explain::new(
        "coc.beta_relever",
        &format!("{headline}; k = (1 − t) D/E (Hamada) or D/E (Harris-Pringle)"),
    );
    formula_conventions(&mut ex, input.formula);
    let bd = debt_beta_convention(&mut ex, input.debt_beta);
    let (de, t, b) = (input.debt_to_equity, input.tax_rate, input.beta);
    let k = input.formula.leverage(de, t)?;
    ex.step("Leverage factor k", leverage_text(input.formula, de, t), k);
    let (out, label) = match input.direction {
        BetaDirection::Relever => (input.formula.relever(b, de, t, bd)?, "Levered beta"),
        BetaDirection::Unlever => (input.formula.unlever(b, de, t, bd)?, "Unlevered beta"),
    };
    ex.step(
        label,
        beta_formula_text(input.formula, input.direction, b, k, bd),
        out,
    );
    Ok((BetaReleverOutput { beta: out }, ex))
}

// ---------------------------------------------------------------- peer betas

/// One comparable company.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Peer {
    /// Name of the comparable, used in the audit trail only.
    pub name: String,
    /// Observed levered beta, dimensionless.
    pub levered_beta: Decimal,
    /// Debt-to-equity ratio D/E of the comparable, decimal, not negative.
    pub debt_to_equity: Decimal,
    /// Tax rate of the comparable, decimal fraction from 0 to below 1.
    pub tax_rate: Decimal,
}

/// Statistic used to summarise the unlevered betas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PeerStatistic {
    /// Arithmetic mean.
    Mean,
    /// Median (mean of the two central values for an even count).
    Median,
}

/// Input of `coc.beta_peers`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BetaPeersInput {
    /// Comparable companies, at least one.
    pub peers: Vec<Peer>,
    /// `hamada` or `harris_pringle`, used to unlever and to relever.
    pub formula: BetaFormula,
    /// `mean` or `median` of the unlevered betas.
    pub statistic: PeerStatistic,
    /// Target debt-to-equity ratio D/E of the company being valued, decimal, not negative.
    pub target_debt_to_equity: Decimal,
    /// Tax rate of the company being valued, decimal fraction from 0 to below 1.
    pub target_tax_rate: Decimal,
    /// Beta of debt, dimensionless, for peers and target. Default: 0 (risk-free debt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt_beta: Option<Decimal>,
}

/// Output of `coc.beta_peers`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BetaPeersOutput {
    /// Unlevered beta of each peer, in input order.
    pub unlevered_betas: Vec<Decimal>,
    /// Arithmetic mean of the unlevered betas.
    pub mean_unlevered_beta: Decimal,
    /// Median of the unlevered betas.
    pub median_unlevered_beta: Decimal,
    /// The statistic selected by `statistic`.
    pub selected_unlevered_beta: Decimal,
    /// Selected unlevered beta relevered to the target structure.
    pub relevered_beta: Decimal,
}

/// Every input error of `beta_peers`, without computing (ADR 0014 G5).
fn check_beta_peers(input: &BetaPeersInput) -> Result<()> {
    if input.peers.is_empty() {
        return Err(
            invalid("peers must contain at least one comparable").at("/peers", Reason::Length)
        );
    }
    // Every input error is reported (ADR 0014 G5): the target, then each comparable.
    let mut found = Found::new();
    found.check(check_leverage(
        input.target_debt_to_equity,
        "target_debt_to_equity",
    ));
    found.check(check_tax_rate(input.target_tax_rate, "target_tax_rate"));
    for (i, p) in input.peers.iter().enumerate() {
        found.check(check_leverage(
            p.debt_to_equity,
            &format!("peers[{i}].debt_to_equity"),
        ));
        found.check(check_tax_rate(p.tax_rate, &format!("peers[{i}].tax_rate")));
    }
    found.finish()
}

pub fn beta_peers(input: &BetaPeersInput) -> Result<(BetaPeersOutput, Explain)> {
    if input.peers.is_empty() {
        return Err(
            invalid("peers must contain at least one comparable").at("/peers", Reason::Length)
        );
    }
    check_beta_peers(input)?;
    let mut ex = Explain::new(
        "coc.beta_peers",
        "β_U,i = (β_L,i + β_D × k_i) / (1 + k_i); statistic of β_U,i; β_L = β_U + (β_U − β_D) × k_target",
    );
    formula_conventions(&mut ex, input.formula);
    let bd = debt_beta_convention(&mut ex, input.debt_beta);
    ex.convention("one debt beta applies to every comparable and to the target");
    ex.convention(match input.statistic {
        PeerStatistic::Mean => "selected statistic: mean",
        PeerStatistic::Median => {
            "selected statistic: median (mean of the two central values for an even count)"
        }
    });
    let mut unlevered = Vec::with_capacity(input.peers.len());
    for (i, p) in input.peers.iter().enumerate() {
        check_leverage(p.debt_to_equity, &format!("peers[{i}].debt_to_equity"))?;
        check_tax_rate(p.tax_rate, &format!("peers[{i}].tax_rate"))?;
        let k = input.formula.leverage(p.debt_to_equity, p.tax_rate)?;
        ex.step(
            format!("Leverage factor, {}", p.name),
            leverage_text(input.formula, p.debt_to_equity, p.tax_rate),
            k,
        );
        let bu = input
            .formula
            .unlever(p.levered_beta, p.debt_to_equity, p.tax_rate, bd)?;
        ex.step(
            format!("Unlevered beta, {}", p.name),
            beta_formula_text(input.formula, BetaDirection::Unlever, p.levered_beta, k, bd),
            bu,
        );
        unlevered.push(bu);
    }
    // Sum in sorted order so that the mean does not depend on the order of the peers.
    let mut sorted = unlevered.clone();
    sorted.sort();
    let n = sorted.len();
    let total = sorted.iter().try_fold(Decimal::ZERO, |a, b| add(a, *b))?;
    let listed = sorted
        .iter()
        .map(|b| b.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    ex.step("Sum of unlevered betas", format!("Σ of [{listed}]"), total);
    let mean = div(total, Decimal::from(n))?;
    ex.step("Mean unlevered beta", format!("{total} / {n}"), mean);
    let median = if n % 2 == 1 {
        sorted[n / 2]
    } else {
        div(add(sorted[n / 2 - 1], sorted[n / 2])?, Decimal::TWO)?
    };
    let median_text = if n % 2 == 1 {
        format!("value {} of {n} sorted: [{listed}]", n / 2 + 1)
    } else {
        format!(
            "({} + {}) / 2, values {} and {} of {n} sorted",
            sorted[n / 2 - 1],
            sorted[n / 2],
            n / 2,
            n / 2 + 1
        )
    };
    ex.step("Median unlevered beta", median_text, median);
    let selected = match input.statistic {
        PeerStatistic::Mean => mean,
        PeerStatistic::Median => median,
    };
    let (tde, tt) = (input.target_debt_to_equity, input.target_tax_rate);
    let k = input.formula.leverage(tde, tt)?;
    ex.step(
        "Leverage factor, target",
        leverage_text(input.formula, tde, tt),
        k,
    );
    let relevered = input.formula.relever(selected, tde, tt, bd)?;
    ex.step(
        "Relevered beta at the target structure",
        beta_formula_text(input.formula, BetaDirection::Relever, selected, k, bd),
        relevered,
    );
    Ok((
        BetaPeersOutput {
            unlevered_betas: unlevered,
            mean_unlevered_beta: mean,
            median_unlevered_beta: median,
            selected_unlevered_beta: selected,
            relevered_beta: relevered,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- cost of equity

/// Unlevered beta to relever inside `coc.cost_of_equity`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Relever {
    /// Unlevered (asset) beta, dimensionless.
    pub unlevered_beta: Decimal,
    /// Target debt-to-equity ratio D/E, decimal, not negative.
    pub debt_to_equity: Decimal,
    /// Tax rate of the tax shield, decimal fraction from 0 to below 1.
    pub tax_rate: Decimal,
    /// `hamada` or `harris_pringle`.
    pub formula: BetaFormula,
    /// Beta of debt, dimensionless. Default: 0 (risk-free debt).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt_beta: Option<Decimal>,
}

/// Input of `coc.cost_of_equity`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CostOfEquityInput {
    /// Risk-free rate, annual decimal fraction.
    pub risk_free_rate: Decimal,
    /// Equity risk premium of the market, annual decimal fraction.
    pub equity_risk_premium: Decimal,
    /// Levered beta, dimensionless. Give exactly one of `levered_beta` and `relever`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub levered_beta: Option<Decimal>,
    /// Unlevered beta and target structure to relever. Give exactly one of `levered_beta` and
    /// `relever`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub relever: Option<Relever>,
    /// Country risk premium, annual decimal fraction, not negative. Default: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_risk_premium: Option<Decimal>,
    /// Exposure of the company to the country risk (lambda), dimensionless, not negative.
    /// Default: 1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub country_exposure: Option<Decimal>,
    /// Size premium, annual decimal fraction. Default: 0.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub size_premium: Option<Decimal>,
    /// Company-specific risk premium, annual decimal fraction. Default: 0. When not zero,
    /// `specific_risk_justification` is required.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specific_risk_premium: Option<Decimal>,
    /// Written reason for the specific risk premium, echoed in the audit trail.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub specific_risk_justification: Option<String>,
}

/// Output of `coc.cost_of_equity`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CostOfEquityOutput {
    /// Cost of equity, annual decimal fraction.
    pub cost_of_equity: Decimal,
    /// Levered beta used (given or relevered).
    pub levered_beta: Decimal,
}

fn premium(ex: &mut Explain, value: Option<Decimal>, name: &str) -> Decimal {
    match value {
        Some(v) => {
            ex.convention(format!("{name} {v} (given)"));
            v
        }
        None => {
            ex.convention(format!("{name} 0 (default)"));
            Decimal::ZERO
        }
    }
}

/// `FM-E001`: not exactly one of `levered_beta` and `relever`.
fn beta_form_error(levered_beta_given: bool) -> FairmarkError {
    invalid("give exactly one of `levered_beta` and `relever`")
        .at(
            "/levered_beta",
            if levered_beta_given {
                Reason::Conflict
            } else {
                Reason::Missing
            },
        )
        .between(["/levered_beta", "/relever"])
}

fn country_risk_premium_error(crp: Decimal) -> FairmarkError {
    invalid(format!(
        "country_risk_premium cannot be negative (got {crp})"
    ))
    .at("/country_risk_premium", Reason::Sign)
}

fn country_exposure_error(lambda: Decimal) -> FairmarkError {
    invalid(format!(
        "country_exposure cannot be negative (got {lambda})"
    ))
    .at("/country_exposure", Reason::Sign)
}

fn justification_error() -> FairmarkError {
    invalid("a non-zero specific_risk_premium requires specific_risk_justification")
        .at("/specific_risk_justification", Reason::Missing)
        .between(["/specific_risk_justification", "/specific_risk_premium"])
}

/// Every input error of `coc.cost_of_equity` (ADR 0014 G5), in the order of the checks of the
/// calculation, which repeats them.
fn check_cost_of_equity(input: &CostOfEquityInput) -> Result<()> {
    let mut found = Found::new();
    // The form first; then the terms of `relever`, when given, whatever the form.
    if input.levered_beta.is_some() == input.relever.is_some() {
        found.push(beta_form_error(input.levered_beta.is_some()));
    }
    if let Some(r) = &input.relever {
        found.check(check_leverage(r.debt_to_equity, "relever.debt_to_equity"));
        found.check(check_tax_rate(r.tax_rate, "relever.tax_rate"));
    }
    let crp = input.country_risk_premium.unwrap_or_default();
    if crp < Decimal::ZERO {
        found.push(country_risk_premium_error(crp));
    }
    let lambda = input.country_exposure.unwrap_or(Decimal::ONE);
    if lambda < Decimal::ZERO {
        found.push(country_exposure_error(lambda));
    }
    let specific = input.specific_risk_premium.unwrap_or_default();
    let justified = input
        .specific_risk_justification
        .as_deref()
        .is_some_and(|j| !j.trim().is_empty());
    if !specific.is_zero() && !justified {
        found.push(justification_error());
    }
    found.finish()
}

pub fn cost_of_equity(input: &CostOfEquityInput) -> Result<(CostOfEquityOutput, Explain)> {
    check_cost_of_equity(input)?;
    let mut ex = Explain::new(
        "coc.cost_of_equity",
        "ke = rf + β_L × ERP + CRP × λ + size premium + specific premium",
    );
    ex.convention(ANNUAL);
    let beta = match (&input.levered_beta, &input.relever) {
        (Some(b), None) => {
            ex.convention(format!("levered beta {b} (given)"));
            *b
        }
        (None, Some(r)) => {
            check_leverage(r.debt_to_equity, "relever.debt_to_equity")?;
            check_tax_rate(r.tax_rate, "relever.tax_rate")?;
            formula_conventions(&mut ex, r.formula);
            let bd = debt_beta_convention(&mut ex, r.debt_beta);
            let k = r.formula.leverage(r.debt_to_equity, r.tax_rate)?;
            ex.step(
                "Leverage factor k",
                leverage_text(r.formula, r.debt_to_equity, r.tax_rate),
                k,
            );
            let b = r
                .formula
                .relever(r.unlevered_beta, r.debt_to_equity, r.tax_rate, bd)?;
            ex.step(
                "Levered beta",
                beta_formula_text(r.formula, BetaDirection::Relever, r.unlevered_beta, k, bd),
                b,
            );
            b
        }
        _ => return Err(beta_form_error(input.levered_beta.is_some())),
    };
    let crp = premium(&mut ex, input.country_risk_premium, "country risk premium");
    if crp < Decimal::ZERO {
        return Err(country_risk_premium_error(crp));
    }
    let lambda = match input.country_exposure {
        Some(l) => {
            ex.convention(format!("country exposure (lambda) {l} (given)"));
            l
        }
        None => {
            ex.convention("country exposure (lambda) 1 (default)");
            Decimal::ONE
        }
    };
    if lambda < Decimal::ZERO {
        return Err(country_exposure_error(lambda));
    }
    let size = premium(&mut ex, input.size_premium, "size premium");
    let specific = premium(
        &mut ex,
        input.specific_risk_premium,
        "specific risk premium",
    );
    let justification = input
        .specific_risk_justification
        .as_deref()
        .map(str::trim)
        .filter(|j| !j.is_empty());
    match (specific.is_zero(), justification) {
        (false, Some(j)) => ex.convention(format!("specific risk premium justification: {j}")),
        (false, None) => return Err(justification_error()),
        (true, Some(j)) => ex.convention(format!(
            "specific risk justification given with a zero premium (recorded, not used): {j}"
        )),
        (true, None) => {}
    }
    let rf = input.risk_free_rate;
    let erp = input.equity_risk_premium;
    let market = mul(beta, erp)?;
    ex.step("Market risk component", format!("{beta} × {erp}"), market);
    let country = mul(crp, lambda)?;
    ex.step(
        "Country risk component",
        format!("{crp} × {lambda}"),
        country,
    );
    let ke = capm(rf, beta, erp, country, size, specific)?;
    ex.step(
        "Cost of equity",
        format!("{rf} + {market} + {country} + {size} + {specific}"),
        ke,
    );
    Ok((
        CostOfEquityOutput {
            cost_of_equity: ke,
            levered_beta: beta,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- cost of debt

/// Input of `coc.cost_of_debt`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CostOfDebtInput {
    /// Pre-tax cost of debt, annual decimal fraction. Give either this, or `risk_free_rate`
    /// and `credit_spread`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pre_tax_cost_of_debt: Option<Decimal>,
    /// Risk-free rate, annual decimal fraction (with `credit_spread`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub risk_free_rate: Option<Decimal>,
    /// Credit spread over the risk-free rate, annual decimal fraction (with `risk_free_rate`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub credit_spread: Option<Decimal>,
    /// Tax rate of the tax shield, decimal fraction from 0 to below 1.
    pub tax_rate: Decimal,
}

/// Output of `coc.cost_of_debt`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CostOfDebtOutput {
    /// Pre-tax cost of debt, annual decimal fraction.
    pub pre_tax_cost_of_debt: Decimal,
    /// After-tax cost of debt kd × (1 − t), annual decimal fraction.
    pub after_tax_cost_of_debt: Decimal,
}

/// `FM-E001`: neither or both forms of the pre-tax cost of debt.
fn debt_form_error(input: &CostOfDebtInput) -> FairmarkError {
    invalid("give either `pre_tax_cost_of_debt`, or both `risk_free_rate` and `credit_spread`")
        .at(
            "/pre_tax_cost_of_debt",
            if input.pre_tax_cost_of_debt.is_some() {
                Reason::Conflict
            } else {
                Reason::Missing
            },
        )
        .between(["/pre_tax_cost_of_debt", "/risk_free_rate", "/credit_spread"])
}

/// Every input error of `cost_of_debt`, without computing (ADR 0014 G5).
fn check_cost_of_debt(input: &CostOfDebtInput) -> Result<()> {
    // Every input error is reported (ADR 0014 G5): the tax rate, then the form of the rate.
    let mut found = Found::new();
    found.check(check_tax_rate(input.tax_rate, "tax_rate"));
    let one_form = matches!(
        (
            input.pre_tax_cost_of_debt,
            input.risk_free_rate,
            input.credit_spread
        ),
        (Some(_), None, None) | (None, Some(_), Some(_))
    );
    if !one_form {
        found.push(debt_form_error(input));
    }
    found.finish()
}

pub fn cost_of_debt(input: &CostOfDebtInput) -> Result<(CostOfDebtOutput, Explain)> {
    check_cost_of_debt(input)?;
    let mut ex = Explain::new("coc.cost_of_debt", "kd_after_tax = kd × (1 − t)");
    ex.convention(ANNUAL);
    let kd = match (
        input.pre_tax_cost_of_debt,
        input.risk_free_rate,
        input.credit_spread,
    ) {
        (Some(k), None, None) => {
            ex.convention("pre-tax cost of debt given");
            k
        }
        (None, Some(rf), Some(s)) => {
            ex.convention("pre-tax cost of debt = risk-free rate + credit spread");
            let k = add(rf, s)?;
            ex.step("Pre-tax cost of debt", format!("{rf} + {s}"), k);
            k
        }
        _ => return Err(debt_form_error(input)),
    };
    let t = input.tax_rate;
    ex.convention(format!("tax rate of the tax shield {t}"));
    let after = mul(kd, sub(Decimal::ONE, t)?)?;
    ex.step("After-tax cost of debt", format!("{kd} × (1 − {t})"), after);
    Ok((
        CostOfDebtOutput {
            pre_tax_cost_of_debt: kd,
            after_tax_cost_of_debt: after,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- WACC

/// Input of `coc.wacc`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct WaccInput {
    /// Cost of equity, annual decimal fraction.
    pub cost_of_equity: Decimal,
    /// Pre-tax cost of debt, annual decimal fraction.
    pub pre_tax_cost_of_debt: Decimal,
    /// Tax rate of the tax shield, decimal fraction from 0 to below 1.
    pub tax_rate: Decimal,
    /// Target debt-to-equity ratio D/E, decimal, not negative. Give exactly one of
    /// `debt_to_equity` and `debt_weight`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt_to_equity: Option<Decimal>,
    /// Target weight of debt D/(D + E), decimal fraction from 0 to below 1. Give exactly one of
    /// `debt_to_equity` and `debt_weight`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub debt_weight: Option<Decimal>,
}

/// Output of `coc.wacc`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct WaccOutput {
    /// Weighted average cost of capital, annual decimal fraction.
    pub wacc: Decimal,
    /// E / (D + E).
    pub equity_weight: Decimal,
    /// D / (D + E).
    pub debt_weight: Decimal,
    /// Implied D/E of the weights; compare it with the D/E used to relever the beta.
    pub debt_to_equity: Decimal,
    /// kd × (1 − t).
    pub after_tax_cost_of_debt: Decimal,
}

/// `FM-E001`: a debt weight outside [0, 1).
fn debt_weight_error(w: Decimal) -> FairmarkError {
    invalid(format!(
        "debt_weight must be at least 0 and below 1 (got {w})"
    ))
    .at("/debt_weight", Reason::Range)
    .expected(["a decimal fraction at least 0 and below 1"])
}

/// `FM-E001`: not exactly one of `debt_to_equity` and `debt_weight`.
fn weights_form_error(input: &WaccInput) -> FairmarkError {
    invalid("give exactly one of `debt_to_equity` and `debt_weight`")
        .at(
            "/debt_to_equity",
            if input.debt_to_equity.is_some() {
                Reason::Conflict
            } else {
                Reason::Missing
            },
        )
        .between(["/debt_to_equity", "/debt_weight"])
}

/// Every input error of `wacc`, without computing (ADR 0014 G5).
fn check_wacc(input: &WaccInput) -> Result<()> {
    // Every input error is reported (ADR 0014 G5): the tax rate, then the weights.
    let mut found = Found::new();
    found.check(check_tax_rate(input.tax_rate, "tax_rate"));
    // The form first; then each weight given is checked on its own, whatever the other.
    if input.debt_to_equity.is_some() == input.debt_weight.is_some() {
        found.push(weights_form_error(input));
    }
    if let Some(de) = input.debt_to_equity {
        found.check(check_leverage(de, "debt_to_equity"));
    }
    if let Some(w) = input
        .debt_weight
        .filter(|w| *w < Decimal::ZERO || *w >= Decimal::ONE)
    {
        found.push(debt_weight_error(w));
    }
    found.finish()
}

pub fn wacc(input: &WaccInput) -> Result<(WaccOutput, Explain)> {
    check_wacc(input)?;
    let mut ex = Explain::new("coc.wacc", "WACC = E/V × ke + D/V × kd × (1 − t)");
    ex.convention(ANNUAL);
    ex.convention(format!("tax rate of the tax shield {}", input.tax_rate));
    let (we, wd) = match (input.debt_to_equity, input.debt_weight) {
        (Some(de), None) => {
            check_leverage(de, "debt_to_equity")?;
            ex.convention(format!("target weights from D/E {de}"));
            // E/V = 1 / (1 + D/E) keeps full precision for large ratios.
            let we = div(Decimal::ONE, add(Decimal::ONE, de)?)?;
            if we.is_zero() {
                return Err(invalid(format!(
                    "debt_to_equity {de} is too large: the equity weight rounds to zero"
                ))
                .at("/debt_to_equity", Reason::Range));
            }
            ex.step("Equity weight", format!("1 / (1 + {de})"), we);
            let wd = sub(Decimal::ONE, we)?;
            ex.step("Debt weight", format!("1 − {we}"), wd);
            (we, wd)
        }
        (None, Some(w)) => {
            if w < Decimal::ZERO || w >= Decimal::ONE {
                return Err(debt_weight_error(w));
            }
            ex.convention(format!("target weights from D/(D + E) {w}"));
            ex.step("Debt weight", "given", w);
            let we = sub(Decimal::ONE, w)?;
            ex.step("Equity weight", format!("1 − {w}"), we);
            (we, w)
        }
        _ => return Err(weights_form_error(input)),
    };
    let de = div(wd, we)?;
    ex.step("Implied debt-to-equity ratio", format!("{wd} / {we}"), de);
    let (ke, kd, t) = (
        input.cost_of_equity,
        input.pre_tax_cost_of_debt,
        input.tax_rate,
    );
    let kd_after = mul(kd, sub(Decimal::ONE, t)?)?;
    ex.step(
        "After-tax cost of debt",
        format!("{kd} × (1 − {t})"),
        kd_after,
    );
    let w = add(mul(we, ke)?, mul(wd, kd_after)?)?;
    ex.step("WACC", format!("{we} × {ke} + {wd} × {kd_after}"), w);
    Ok((
        WaccOutput {
            wacc: w,
            equity_weight: we,
            debt_weight: wd,
            debt_to_equity: de,
            after_tax_cost_of_debt: kd_after,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- method registrations

/// `coc.cost_of_equity`: see the methodology page.
pub struct CostOfEquity;

impl Method for CostOfEquity {
    const ID: &'static str = "coc.cost_of_equity";
    const TITLE: &'static str = "Cost of equity (CAPM with explicit premia)";
    const DESCRIPTION: &'static str =
        "Cost of equity from risk-free rate, levered or relevered beta and equity risk premium, \
        plus optional country, size and specific premia, each reported.";
    const VERSION: &'static str = "1";
    type Input = CostOfEquityInput;
    type Output = CostOfEquityOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check_cost_of_equity(input)
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        cost_of_equity(input)
    }
}

/// `coc.beta_relever`: see the methodology page.
pub struct BetaRelever;

impl Method for BetaRelever {
    const ID: &'static str = "coc.beta_relever";
    const TITLE: &'static str = "Beta unlevering and relevering";
    const DESCRIPTION: &'static str =
        "Converts an unlevered beta to a levered beta at a target D/E, or the reverse, with the \
        Hamada or Harris-Pringle formula and an optional debt beta.";
    const VERSION: &'static str = "1";
    type Input = BetaReleverInput;
    type Output = BetaReleverOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check_beta_relever(input)
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        beta_relever(input)
    }
}

/// `coc.beta_peers`: see the methodology page.
pub struct BetaPeers;

impl Method for BetaPeers {
    const ID: &'static str = "coc.beta_peers";
    const TITLE: &'static str = "Beta from comparable companies";
    const DESCRIPTION: &'static str =
        "Unlevers the betas of comparable companies, takes the mean or median and relevers it to \
        the target capital structure.";
    const VERSION: &'static str = "1";
    type Input = BetaPeersInput;
    type Output = BetaPeersOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check_beta_peers(input)
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        beta_peers(input)
    }
}

/// `coc.cost_of_debt`: see the methodology page.
pub struct CostOfDebt;

impl Method for CostOfDebt {
    const ID: &'static str = "coc.cost_of_debt";
    const TITLE: &'static str = "Cost of debt";
    const DESCRIPTION: &'static str =
        "Pre-tax cost of debt, given or as risk-free rate plus credit spread, and the after-tax \
        cost of debt.";
    const VERSION: &'static str = "1";
    type Input = CostOfDebtInput;
    type Output = CostOfDebtOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check_cost_of_debt(input)
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        cost_of_debt(input)
    }
}

/// `coc.wacc`: see the methodology page.
pub struct Wacc;

impl Method for Wacc {
    const ID: &'static str = "coc.wacc";
    const TITLE: &'static str = "Weighted average cost of capital (WACC)";
    const DESCRIPTION: &'static str =
        "WACC from cost of equity, cost of debt, tax rate and target weights (D/E or D/(D+E)).";
    const VERSION: &'static str = "1";
    type Input = WaccInput;
    type Output = WaccOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check_wacc(input)
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        wacc(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    /// AC2: unlever then relever at the same structure returns the original beta.
    #[test]
    fn unlever_relever_round_trip() {
        for f in [BetaFormula::Hamada, BetaFormula::HarrisPringle] {
            for bd in ["0", "0.15"] {
                for (bl, de, t) in [
                    ("1.2", "0.6", "0.25"),
                    ("0.7", "0", "0.3"),
                    ("1.9", "2.5", "0"),
                ] {
                    let (bl, de, t, bd) = (d(bl), d(de), d(t), d(bd));
                    let bu = f.unlever(bl, de, t, bd).unwrap();
                    let back = f.relever(bu, de, t, bd).unwrap();
                    assert!(
                        (back - bl).abs() < d("1e-26"),
                        "{f:?} {bl} {de} {t} {bd}: {back}"
                    );
                }
            }
        }
    }

    #[test]
    fn harris_pringle_ignores_the_tax_rate() {
        let a = BetaFormula::HarrisPringle
            .relever(d("0.8"), d("0.5"), d("0"), d("0"))
            .unwrap();
        let b = BetaFormula::HarrisPringle
            .relever(d("0.8"), d("0.5"), d("0.3"), d("0"))
            .unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn wacc_weights_from_debt_to_equity_and_debt_weight_agree() {
        let base = WaccInput {
            cost_of_equity: d("0.11"),
            pre_tax_cost_of_debt: d("0.05"),
            tax_rate: d("0.24"),
            debt_to_equity: Some(d("0.25")),
            debt_weight: None,
        };
        assert_eq!(wacc(&base).unwrap().0.debt_to_equity, d("0.25"));
        let by_weight = WaccInput {
            debt_to_equity: None,
            debt_weight: Some(d("0.2")),
            ..base.clone()
        };
        assert_eq!(
            wacc(&base).unwrap().0.wacc,
            wacc(&by_weight).unwrap().0.wacc
        );
    }

    #[test]
    fn even_peer_count_takes_the_mean_of_the_central_values() {
        let peer = |b: &str| Peer {
            name: b.to_owned(),
            levered_beta: d(b),
            debt_to_equity: d("0"),
            tax_rate: d("0.2"),
        };
        let (out, _) = beta_peers(&BetaPeersInput {
            peers: vec![peer("1.0"), peer("0.6"), peer("1.4"), peer("0.8")],
            formula: BetaFormula::Hamada,
            statistic: PeerStatistic::Median,
            target_debt_to_equity: d("0"),
            target_tax_rate: d("0.2"),
            debt_beta: None,
        })
        .unwrap();
        assert_eq!(out.median_unlevered_beta, d("0.9"));
        assert_eq!(out.relevered_beta, d("0.9"));
    }

    #[test]
    fn invalid_inputs_are_rejected() {
        let err = cost_of_debt(&CostOfDebtInput {
            pre_tax_cost_of_debt: Some(d("0.05")),
            risk_free_rate: Some(d("0.03")),
            credit_spread: None,
            tax_rate: d("0.2"),
        })
        .unwrap_err();
        assert_eq!(err.code(), "FM-E001");
        let err = beta_relever(&BetaReleverInput {
            beta: d("1"),
            direction: BetaDirection::Relever,
            debt_to_equity: d("0.5"),
            tax_rate: d("1"),
            formula: BetaFormula::Hamada,
            debt_beta: None,
        })
        .unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }

    /// Audit D1: a D/E so large that the equity weight rounds to zero is an error.
    #[test]
    fn wacc_rejects_an_equity_weight_that_rounds_to_zero() {
        let err = wacc(&WaccInput {
            cost_of_equity: d("0.10"),
            pre_tax_cost_of_debt: d("0.06"),
            tax_rate: d("0.25"),
            debt_to_equity: Some(d("21000000000000000000000000000")),
            debt_weight: None,
        })
        .unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }

    /// Audit D3: the mean does not depend on the order of the peers.
    #[test]
    fn peer_mean_is_independent_of_order() {
        let peer = |b: &str, de: &str| Peer {
            name: b.to_owned(),
            levered_beta: d(b),
            debt_to_equity: d(de),
            tax_rate: d("0.27"),
        };
        let mut peers = vec![
            peer("1.73", "0.31"),
            peer("2.91", "1.7"),
            peer("0.97", "0.05"),
            peer("3.33", "2.2"),
            peer("1.11", "0.9"),
            peer("2.47", "0.44"),
            peer("4.05", "3.1"),
            peer("1.58", "0.12"),
            peer("2.02", "0.8"),
        ];
        let run = |peers: Vec<Peer>| {
            beta_peers(&BetaPeersInput {
                peers,
                formula: BetaFormula::Hamada,
                statistic: PeerStatistic::Mean,
                target_debt_to_equity: d("0.4"),
                target_tax_rate: d("0.24"),
                debt_beta: Some(d("0.0972")),
            })
            .unwrap()
            .0
        };
        let a = run(peers.clone());
        peers.reverse();
        peers.swap(1, 6);
        let b = run(peers);
        assert_eq!(a.mean_unlevered_beta, b.mean_unlevered_beta);
        assert_eq!(a.relevered_beta, b.relevered_beta);
    }
}
