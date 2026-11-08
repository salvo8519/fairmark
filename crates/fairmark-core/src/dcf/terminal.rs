//! The terminal value inside the DCF (SPEC-0001 R19 to R22): the technique, written with its
//! `tv.*` method id, its checks, its computation through the typed `tv` function (ADR 0009) and
//! its discounting to the valuation date.
//!
//! Every technique is an optional property of [`Terminal`], exactly one present at run time,
//! so that a new technique is an additive change of the method's schema (R29; see
//! `specs/0001-dcf-complete/plan.md`, issue #43).

use rust_decimal::{Decimal, MathematicalOps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::engine::TerminalDiscountTime;
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{div, mul, one_plus, sub};
use crate::tv::{
    self, exit_multiple::ExitMultipleOutput, limited_horizon::LimitedHorizonOutput,
    normalised_income::NormalisedIncomeOutput, ExitMultipleInput, FieldError, GordonInput,
    GordonOutput, HModelInput, HModelOutput, LimitedHorizonInput, Line, LiquidationInput,
    LiquidationOutput, MultipleMetric, NormalisedIncomeInput, TerminalBasis, ValueDriverInput,
    ValueDriverOutput,
};
use crate::types::Timing;

// ---------------------------------------------------------------- input types

/// The flow at N + 1 of a technique that grows the last explicit flow (R20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FirstTerminalFlow {
    /// The technique grows the last explicit flow CF_N (`tv.gordon`: CF_N × (1 + g);
    /// `tv.h_model`: its own formula).
    LastFlowGrown,
    /// CF_{N+1} given in `normalised_next_flow`.
    NormalisedNextFlow,
}

/// The flow CF_N on which `tv.h_model` is computed (R20, Decision 15).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum HModelFirstFlow {
    /// The last explicit flow CF_N, grown by the H-model formula.
    LastFlowGrown,
    /// A normalised base flow at N, given in `normalised_base_flow`, replaces the last explicit
    /// flow; the standard formula applies, with no conversion from N + 1.
    NormalisedBaseFlow,
}

/// The terms of `tv.gordon` inside the DCF: the rate is the terminal rate of the DCF and the
/// last flow is the last explicit flow.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GordonTerminal {
    /// Perpetual growth after the horizon, decimal fraction per period, greater than -1 and
    /// below the terminal rate, in the terms of the cash flows.
    pub growth: Decimal,
    /// `last_flow_grown` or `normalised_next_flow`; no default.
    pub first_flow: FirstTerminalFlow,
    /// The flow at N + 1 in currency units, in the terms of the cash flows; required with
    /// `first_flow` = `normalised_next_flow`, refused otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalised_next_flow: Option<Decimal>,
    /// Upper bound for the perpetual growth, decimal fraction per period. Default: none,
    /// reported as absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.h_model` inside the DCF: the rate is the terminal rate of the DCF and the
/// flow CF_N is the last explicit flow or a normalised base flow at N (Decision 15).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct HModelTerminal {
    /// Growth at the start of the decline, g_s, decimal fraction per period, greater than -1,
    /// in the terms of the cash flows.
    pub short_term_growth: Decimal,
    /// Stable growth reached after 2H periods and kept forever, g_l, decimal fraction per
    /// period, greater than -1 and below the terminal rate, in the terms of the cash flows.
    pub long_term_growth: Decimal,
    /// Half-life H, in periods: the decline from g_s to g_l lasts 2H periods. Zero or
    /// positive; may be fractional.
    pub half_life: Decimal,
    /// `last_flow_grown` (the H-model grows the last explicit flow CF_N) or
    /// `normalised_base_flow` (a normalised CF_N given in `normalised_base_flow`); no default.
    pub first_flow: HModelFirstFlow,
    /// The normalised base flow CF_N at the horizon N, in currency units, in the terms of the
    /// cash flows; required with `first_flow` = `normalised_base_flow`, refused otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub normalised_base_flow: Option<Decimal>,
    /// Upper bound for the long-term growth, decimal fraction per period. Default: none,
    /// reported as absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.value_driver` inside the DCF: the rate is the terminal rate of the DCF.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ValueDriverTerminal {
    /// Net operating profit after tax of the first period after the horizon, NOPAT_{N+1}, in
    /// currency units, in the terms of the cash flows.
    pub nopat_next: Decimal,
    /// Perpetual growth, decimal fraction per period, greater than -1 and below the terminal
    /// rate, in the terms of the cash flows.
    pub growth: Decimal,
    /// Return on new invested capital (RONIC), decimal fraction per period, greater than 0.
    pub ronic: Decimal,
    /// Upper bound for the perpetual growth, decimal fraction per period. Default: none,
    /// reported as absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.exit_multiple` inside the DCF: the metric must be on the basis of the DCF
/// (`ebitda`, `ebit` or `revenue` for the enterprise basis).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ExitMultipleTerminal {
    /// The multiple, a pure number greater than 0 (for example 8 for 8 × EBITDA).
    pub multiple: Decimal,
    /// `ebitda`, `ebit`, `revenue` (enterprise basis), `earnings` or `book_value` (equity
    /// basis); must be on the basis of the DCF.
    pub metric: MultipleMetric,
    /// The metric at the horizon N (trailing), in currency units, greater than 0, in the terms
    /// of the cash flows.
    pub metric_value: Decimal,
    /// Flow at N + 1 on the basis of the DCF, in currency units, in the terms of the cash
    /// flows. Optional: when given, the perpetual growth implied by the multiple is computed
    /// at the terminal rate.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub next_flow: Option<Decimal>,
    /// Upper bound for the implied growth, decimal fraction per period. Default: none,
    /// reported as absent. Needs `next_flow`. Exceeding it is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.limited_horizon` inside the DCF: the rate is the terminal rate of the DCF.
/// Every term is required, with no default (R29).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LimitedHorizonTerminal {
    /// First flow after the horizon, at the end of period N + 1, in currency units, in the
    /// terms of the cash flows.
    pub flow: Decimal,
    /// Number of further periods n during which the flow lasts, from 1 to 10000.
    #[schemars(range(min = 1, max = 10000))]
    pub periods: u32,
    /// Growth of the flow per period, decimal fraction, greater than -1; 0 for a constant flow.
    pub growth: Decimal,
    /// Residual value at the end of period N + n, in currency units, in the terms of the cash
    /// flows; 0 for none.
    pub residual_value: Decimal,
    /// Upper bound for the growth, decimal fraction per period. Default: none, reported as
    /// absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terms of `tv.liquidation` inside the DCF: the declared basis must be the basis of the
/// DCF.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct LiquidationTerminal {
    /// Assets with their realisable values at N, in currency units, in the terms of the cash
    /// flows, each zero or positive; at least one.
    pub assets: Vec<Line>,
    /// Liabilities settled at N, in currency units, in the terms of the cash flows, each zero
    /// or positive.
    pub liabilities: Vec<Line>,
    /// Costs of the liquidation (legal fees, closure, severance), in currency units, in the
    /// terms of the cash flows, each zero or positive.
    pub liquidation_costs: Vec<Line>,
    /// Tax on the gains realised, as an amount in currency units, in the terms of the cash
    /// flows, zero or positive.
    pub tax_on_gains: Decimal,
    /// `enterprise` (the liabilities listed exclude financial debt) or `equity` (financial debt
    /// included); must be the basis of the DCF. Required, no default.
    pub basis: TerminalBasis,
}

/// The terms of `tv.normalised_income` inside the DCF: the capitalisation rate is the terminal
/// rate of the DCF (R19, Decision 13); no separate capitalisation rate is accepted.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NormalisedIncomeTerminal {
    /// Normalised income R of the first period after the horizon, in currency units, in the
    /// terms of the cash flows.
    pub income: Decimal,
    /// `enterprise` (an operating income) or `equity` (a net income); must be the basis of the
    /// DCF. Required, no default.
    pub basis: TerminalBasis,
    /// Perpetual growth of the income, decimal fraction per period, greater than -1 and below
    /// the terminal rate, in the terms of the cash flows; 0 for none (R / i).
    pub growth: Decimal,
    /// Upper bound for the growth, decimal fraction per period. Default: none, reported as
    /// absent. A growth above the cap is reported, not refused.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
}

/// The terminal technique, written as an object with exactly one key, the `tv.*` method id,
/// as in `tv.compare`: `{"tv.gordon": {...}}`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
#[schemars(extend("minProperties" = 1, "maxProperties" = 1))]
pub struct Terminal {
    /// Stable growth (`tv.gordon`): `growth`, `first_flow`, optional `normalised_next_flow`
    /// and `growth_cap`.
    #[serde(rename = "tv.gordon", default, skip_serializing_if = "Option::is_none")]
    pub gordon: Option<GordonTerminal>,
    /// Growth declining linearly to a stable rate (`tv.h_model`) on the last explicit flow:
    /// `short_term_growth`, `long_term_growth`, `half_life`, `first_flow`, optional
    /// `growth_cap`.
    #[serde(
        rename = "tv.h_model",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub h_model: Option<HModelTerminal>,
    /// Stable growth earned by reinvestment (`tv.value_driver`): `nopat_next`, `growth`,
    /// `ronic`, optional `growth_cap`.
    #[serde(
        rename = "tv.value_driver",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub value_driver: Option<ValueDriverTerminal>,
    /// The business sold at a multiple of a metric at N (`tv.exit_multiple`): `multiple`,
    /// `metric`, `metric_value`, optional `next_flow` and `growth_cap`.
    #[serde(
        rename = "tv.exit_multiple",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub exit_multiple: Option<ExitMultipleTerminal>,
    /// A flow lasting n more periods, then a residual value (`tv.limited_horizon`): `flow`,
    /// `periods`, `growth`, `residual_value`, optional `growth_cap`.
    #[serde(
        rename = "tv.limited_horizon",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub limited_horizon: Option<LimitedHorizonTerminal>,
    /// A normalised income capitalised at the terminal rate (`tv.normalised_income`): `income`,
    /// `basis`, `growth`, optional `growth_cap`.
    #[serde(
        rename = "tv.normalised_income",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub normalised_income: Option<NormalisedIncomeTerminal>,
    /// Assets realised and liabilities settled at N (`tv.liquidation`): `assets`,
    /// `liabilities`, `liquidation_costs`, `tax_on_gains`, `basis`.
    #[serde(
        rename = "tv.liquidation",
        default,
        skip_serializing_if = "Option::is_none"
    )]
    pub liquidation: Option<LiquidationTerminal>,
}

// ---------------------------------------------------------------- output types

/// The output of the terminal technique, as its own method returns it.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum TechniqueOutput {
    Gordon(GordonOutput),
    HModel(HModelOutput),
    ValueDriver(ValueDriverOutput),
    ExitMultiple(ExitMultipleOutput),
    LimitedHorizon(LimitedHorizonOutput),
    NormalisedIncome(NormalisedIncomeOutput),
    Liquidation(LiquidationOutput),
}

/// The terminal value and its discounting (R19 to R22).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TerminalOutput {
    /// Method id of the technique, for example `tv.gordon`.
    pub technique: String,
    /// Version of the technique's method that produced the value.
    pub technique_version: String,
    /// Rate of the terminal phase (the single rate, or the terminal rate given with rates per
    /// period), decimal fraction per period, in the terms of the cash flows; the conventions
    /// state whether the technique uses it.
    pub rate: Decimal,
    /// Terminal value at the horizon N, not discounted, in currency units.
    pub terminal_value: Decimal,
    /// Time at which the terminal value is discounted, in periods: N, or N − 0.5.
    pub discount_time: Decimal,
    /// Discount factor applied to the terminal value.
    pub discount_factor: Decimal,
    /// Present value of the terminal value, in currency units.
    pub present_value: Decimal,
    /// The technique's own output, with its implied assumptions.
    pub output: TechniqueOutput,
}

// ---------------------------------------------------------------- checks

/// The technique chosen, after validation.
#[derive(Clone, Copy)]
pub(crate) enum Technique<'a> {
    Gordon(&'a GordonTerminal),
    HModel(&'a HModelTerminal),
    ValueDriver(&'a ValueDriverTerminal),
    ExitMultiple(&'a ExitMultipleTerminal),
    LimitedHorizon(&'a LimitedHorizonTerminal),
    NormalisedIncome(&'a NormalisedIncomeTerminal),
    Liquidation(&'a LiquidationTerminal),
}

impl Technique<'_> {
    fn id(self) -> &'static str {
        match self {
            Self::Gordon(_) => tv::Gordon::ID,
            Self::HModel(_) => tv::HModel::ID,
            Self::ValueDriver(_) => tv::ValueDriver::ID,
            Self::ExitMultiple(_) => tv::ExitMultiple::ID,
            Self::LimitedHorizon(_) => tv::LimitedHorizon::ID,
            Self::NormalisedIncome(_) => tv::NormalisedIncome::ID,
            Self::Liquidation(_) => tv::Liquidation::ID,
        }
    }

    fn version(self) -> &'static str {
        match self {
            Self::Gordon(_) => tv::Gordon::VERSION,
            Self::HModel(_) => tv::HModel::VERSION,
            Self::ValueDriver(_) => tv::ValueDriver::VERSION,
            Self::ExitMultiple(_) => tv::ExitMultiple::VERSION,
            Self::LimitedHorizon(_) => tv::LimitedHorizon::VERSION,
            Self::NormalisedIncome(_) => tv::NormalisedIncome::VERSION,
            Self::Liquidation(_) => tv::Liquidation::VERSION,
        }
    }

    /// The perpetual growth the technique assumes after the horizon, for the continuity
    /// warning of R11; `None` for the techniques that state none.
    fn perpetual_growth(self) -> Option<Decimal> {
        match self {
            Self::Gordon(t) => Some(t.growth),
            Self::HModel(t) => Some(t.long_term_growth),
            Self::ValueDriver(t) => Some(t.growth),
            Self::NormalisedIncome(t) => Some(t.growth),
            Self::ExitMultiple(_) | Self::LimitedHorizon(_) | Self::Liquidation(_) => None,
        }
    }

    /// A value capitalising flows that grow forever: commonly discounted at N − 0.5 under
    /// mid-period timing, like the flows it capitalises.
    fn is_perpetual_growth(self) -> bool {
        matches!(
            self,
            Self::Gordon(_) | Self::HModel(_) | Self::ValueDriver(_) | Self::NormalisedIncome(_)
        )
    }

    /// A price at a date (a sale at a multiple or a liquidation at N; the residual value of a
    /// finite horizon at N + n): Decisions 6 and 17 warn when it is discounted at N − 0.5.
    fn price_at_date(self) -> Option<&'static str> {
        match self {
            Self::ExitMultiple(_) | Self::Liquidation(_) => {
                Some("is a price at the horizon N (a sale or a liquidation at a date)")
            }
            Self::LimitedHorizon(_) => {
                Some("includes a price at a date (the residual value at N + n)")
            }
            _ => None,
        }
    }
}

fn invalid(msg: String) -> FairmarkError {
    FairmarkError::InvalidInput(msg)
}

/// R19: the basis of the technique must be the basis of the DCF (`FM-E019`).
fn check_basis(
    at: &str,
    field: &str,
    found: TerminalBasis,
    what: String,
    dcf: TerminalBasis,
) -> Result<()> {
    if found == dcf {
        return Ok(());
    }
    let fix = match dcf {
        TerminalBasis::Enterprise => {
            "use an enterprise measure: an exit multiple on ebitda, ebit or revenue, a \
             normalised operating income or a liquidation whose liabilities exclude financial \
             debt (basis enterprise)"
        }
        TerminalBasis::Equity => {
            "use an equity measure: an exit multiple on earnings or book_value, a normalised \
             net income or a liquidation whose liabilities include financial debt (basis \
             equity)"
        }
    };
    Err(FairmarkError::MixedBasis(format!(
        "`{at}/{field}` is {what}, on the {} basis, but this DCF values on the {} basis; {fix}",
        found.name(),
        dcf.name()
    )))
}

/// The input of the technique's own method, built from its terms inside the DCF.
enum TvInput {
    Gordon(GordonInput),
    HModel(HModelInput),
    ValueDriver(ValueDriverInput),
    ExitMultiple(ExitMultipleInput),
    LimitedHorizon(LimitedHorizonInput),
    NormalisedIncome(NormalisedIncomeInput),
    Liquidation(LiquidationInput),
}

/// The technique's terms with the terminal rate and the last explicit flow CF_N.
fn tv_input(technique: Technique<'_>, rate: Decimal, last_flow: Decimal) -> TvInput {
    match technique {
        Technique::Gordon(t) => {
            let (next_flow, last) = match t.first_flow {
                FirstTerminalFlow::LastFlowGrown => (None, Some(last_flow)),
                FirstTerminalFlow::NormalisedNextFlow => (t.normalised_next_flow, None),
            };
            TvInput::Gordon(GordonInput {
                rate,
                growth: t.growth,
                next_flow,
                last_flow: last,
                growth_cap: t.growth_cap,
            })
        }
        Technique::HModel(t) => TvInput::HModel(HModelInput {
            last_flow: match t.first_flow {
                HModelFirstFlow::LastFlowGrown => last_flow,
                HModelFirstFlow::NormalisedBaseFlow => t.normalised_base_flow.unwrap_or_default(),
            },
            short_term_growth: t.short_term_growth,
            long_term_growth: t.long_term_growth,
            half_life: t.half_life,
            rate,
            growth_cap: t.growth_cap,
        }),
        Technique::ValueDriver(t) => TvInput::ValueDriver(ValueDriverInput {
            nopat_next: t.nopat_next,
            growth: t.growth,
            ronic: t.ronic,
            rate,
            growth_cap: t.growth_cap,
        }),
        Technique::ExitMultiple(t) => TvInput::ExitMultiple(ExitMultipleInput {
            multiple: t.multiple,
            metric: t.metric,
            metric_value: t.metric_value,
            rate: t.next_flow.map(|_| rate),
            next_flow: t.next_flow,
            growth_cap: t.growth_cap,
        }),
        Technique::LimitedHorizon(t) => TvInput::LimitedHorizon(LimitedHorizonInput {
            flow: t.flow,
            rate,
            periods: t.periods,
            growth: Some(t.growth),
            residual_value: Some(t.residual_value),
            growth_cap: t.growth_cap,
        }),
        Technique::NormalisedIncome(t) => TvInput::NormalisedIncome(NormalisedIncomeInput {
            income: t.income,
            capitalisation_rate: rate,
            basis: t.basis,
            growth: Some(t.growth),
            growth_cap: t.growth_cap,
        }),
        Technique::Liquidation(t) => TvInput::Liquidation(LiquidationInput {
            assets: t.assets.clone(),
            liabilities: t.liabilities.clone(),
            liquidation_costs: t.liquidation_costs.clone(),
            tax_on_gains: t.tax_on_gains,
            basis: t.basis,
        }),
    }
}

/// The checks of the technique's own method (SPEC-0006), each naming its field.
fn tv_check(input: &TvInput) -> std::result::Result<(), FieldError> {
    match input {
        TvInput::Gordon(i) => tv::gordon::check(i),
        TvInput::HModel(i) => tv::h_model::check(i),
        TvInput::ValueDriver(i) => tv::value_driver::check(i),
        TvInput::ExitMultiple(i) => tv::exit_multiple::check(i),
        TvInput::LimitedHorizon(i) => tv::limited_horizon::check(i),
        TvInput::NormalisedIncome(i) => tv::normalised_income::check(i),
        TvInput::Liquidation(i) => tv::liquidation::check(i),
    }
}

/// An error of the technique's own method, with its field turned into a JSON pointer of the
/// DCF input (R27): `/terminal/<id>/<field>`, or the field of the terminal rate for the rate
/// the DCF passes to the technique. The code is unchanged; a growth not below the rate becomes
/// the pointer-carrying variant of `FM-E004`.
fn at_pointer(e: FieldError, technique: Technique<'_>, rate_field: &str) -> FairmarkError {
    let at = format!("/terminal/{}", technique.id());
    let (pointer, note) = match (technique, e.field.as_str()) {
        (_, "rate" | "capitalisation_rate") => (
            rate_field.to_owned(),
            "; inside the DCF the technique uses the terminal rate",
        ),
        (Technique::Gordon(_), "next_flow") => (format!("{at}/normalised_next_flow"), ""),
        (_, field) => (format!("{at}/{field}"), ""),
    };
    match e.error {
        FairmarkError::InvalidInput(m) => {
            FairmarkError::InvalidInput(format!("`{pointer}`: {m}{note}"))
        }
        FairmarkError::TerminalValueDomain(m) => {
            FairmarkError::TerminalValueDomain(format!("`{pointer}`: {m}{note}"))
        }
        FairmarkError::GrowthNotBelowRate { growth, rate } => {
            FairmarkError::GrowthNotBelowTerminalRate {
                growth_field: pointer,
                growth,
                rate_field: rate_field.to_owned(),
                rate,
            }
        }
        FairmarkError::RateOutOfRange(rate) => FairmarkError::DiscountRateOutOfRange {
            field: rate_field.to_owned(),
            rate,
        },
        other => other,
    }
}

/// R19, R20: exactly one technique, on the basis of the DCF, with the first terminal flow
/// stated; the terms checked by the technique's own method, every error with its JSON pointer
/// (R27). Runs before any calculation.
pub(crate) fn check<'a>(
    t: &'a Terminal,
    rate: Decimal,
    rate_field: &str,
    basis: TerminalBasis,
) -> Result<Technique<'a>> {
    let mut given: Vec<Technique<'a>> = Vec::with_capacity(1);
    if let Some(x) = &t.gordon {
        given.push(Technique::Gordon(x));
    }
    if let Some(x) = &t.h_model {
        given.push(Technique::HModel(x));
    }
    if let Some(x) = &t.value_driver {
        given.push(Technique::ValueDriver(x));
    }
    if let Some(x) = &t.exit_multiple {
        given.push(Technique::ExitMultiple(x));
    }
    if let Some(x) = &t.limited_horizon {
        given.push(Technique::LimitedHorizon(x));
    }
    if let Some(x) = &t.normalised_income {
        given.push(Technique::NormalisedIncome(x));
    }
    if let Some(x) = &t.liquidation {
        given.push(Technique::Liquidation(x));
    }
    let technique = match given.as_slice() {
        [] => {
            return Err(invalid(
                "`/terminal` names no technique: give exactly one, written with its method id, \
                 for example {\"tv.gordon\": {\"growth\": 0.02, \"first_flow\": \"last_flow_grown\"}}"
                    .to_owned(),
            ))
        }
        [one] => *one,
        many => {
            let ids: Vec<&str> = many.iter().map(|t| t.id()).collect();
            return Err(invalid(format!(
                "`/terminal` names {} techniques ({}); give exactly one terminal technique",
                ids.len(),
                ids.join(", ")
            )));
        }
    };
    let at = format!("/terminal/{}", technique.id());
    let at = at.as_str();
    // What only the DCF knows: the first terminal flow (R20) and the basis (R19).
    match technique {
        Technique::Gordon(g) => match (g.first_flow, g.normalised_next_flow) {
            (FirstTerminalFlow::NormalisedNextFlow, None) => {
                return Err(invalid(format!(
                    "`{at}/normalised_next_flow` is required with `{at}/first_flow` = \
                     normalised_next_flow: give the flow at N + 1"
                )))
            }
            (FirstTerminalFlow::LastFlowGrown, Some(_)) => {
                return Err(invalid(format!(
                    "`{at}/normalised_next_flow` is not used with `{at}/first_flow` = \
                     last_flow_grown; remove it, or set `{at}/first_flow` to \
                     normalised_next_flow"
                )))
            }
            _ => {}
        },
        Technique::HModel(h) => match (h.first_flow, h.normalised_base_flow) {
            (HModelFirstFlow::NormalisedBaseFlow, None) => {
                return Err(invalid(format!(
                    "`{at}/normalised_base_flow` is required with `{at}/first_flow` = \
                     normalised_base_flow: give the normalised flow CF_N at the horizon N"
                )))
            }
            (HModelFirstFlow::LastFlowGrown, Some(_)) => {
                return Err(invalid(format!(
                    "`{at}/normalised_base_flow` is not used with `{at}/first_flow` = \
                     last_flow_grown; remove it, or set `{at}/first_flow` to \
                     normalised_base_flow"
                )))
            }
            _ => {}
        },
        Technique::ExitMultiple(e) => check_basis(
            at,
            "metric",
            e.metric.basis(),
            format!("{} ({})", e.metric.name(), e.metric.multiple_name()),
            basis,
        )?,
        Technique::NormalisedIncome(q) => check_basis(
            at,
            "basis",
            q.basis,
            format!("{} (declared)", q.basis.name()),
            basis,
        )?,
        Technique::Liquidation(q) => check_basis(
            at,
            "basis",
            q.basis,
            format!("{} (declared)", q.basis.name()),
            basis,
        )?,
        Technique::ValueDriver(_) | Technique::LimitedHorizon(_) => {}
    }
    // The terms, by the technique's own checks. CF_N is not known yet and no technique checks
    // its value: zero stands in for it.
    tv_check(&tv_input(technique, rate, Decimal::ZERO))
        .map_err(|e| at_pointer(e, technique, rate_field))?;
    Ok(technique)
}

// ---------------------------------------------------------------- calculation

/// The technique computed through its typed `tv` function (ADR 0009), with its conventions.
/// The terms were checked by [`check`]; what can still fail is arithmetic (overflow).
fn compute(
    ex: &mut Explain,
    technique: Technique<'_>,
    rate: Decimal,
    last_flow: Decimal,
    basis: TerminalBasis,
) -> Result<(Decimal, TechniqueOutput, Explain)> {
    let (id, version) = (technique.id(), technique.version());
    let on = format!(
        "terminal technique: {id} version {version} on the {} basis",
        basis.name()
    );
    let at_rate = format!("{on}, at the terminal rate {rate}, in the terms of the cash flows");
    match technique {
        Technique::Gordon(t) => {
            ex.convention(at_rate);
            ex.convention(match t.first_flow {
                FirstTerminalFlow::LastFlowGrown => {
                    "first terminal flow: the last explicit flow grown by the perpetual growth, \
                     CF_N × (1 + g)"
                }
                FirstTerminalFlow::NormalisedNextFlow => {
                    "first terminal flow: normalised flow at N + 1, given in the terms of the \
                     cash flows"
                }
            });
        }
        Technique::HModel(t) => {
            ex.convention(at_rate);
            ex.convention(match t.first_flow {
                HModelFirstFlow::LastFlowGrown => {
                    "first terminal flow: the last explicit flow CF_N, grown by the H-model, \
                     CF_N × [(1 + g_l) + H × (g_s − g_l)]"
                }
                HModelFirstFlow::NormalisedBaseFlow => {
                    "first terminal flow: normalised base flow CF_N at the horizon N, given in \
                     the terms of the cash flows, in place of the last explicit flow, grown by \
                     the H-model, CF_N × [(1 + g_l) + H × (g_s − g_l)] (Decision 15)"
                }
            });
        }
        Technique::ValueDriver(_) => {
            ex.convention(at_rate);
            ex.convention(
                "first terminal flow: NOPAT at N + 1 as given (`nopat_next`), less the \
                 reinvestment g / RONIC, in the terms of the cash flows",
            );
        }
        Technique::ExitMultiple(t) => {
            ex.convention(match t.next_flow {
                Some(_) => format!(
                    "{on}, in the terms of the cash flows; the terminal rate {rate} is used only \
                     for the growth implied by the multiple"
                ),
                None => format!(
                    "{on}, in the terms of the cash flows; no next flow given: the terminal \
                     rate is not used and no implied growth is computed"
                ),
            });
            ex.convention("terminal value from the metric at N; no terminal flow is projected");
            if t.next_flow.is_some() {
                ex.convention(
                    "the growth implied by the multiple is reported, not compared with the \
                     growth of the last explicit period: the continuity check (R11) applies to \
                     a perpetual growth stated as an input",
                );
            }
        }
        Technique::LimitedHorizon(_) => {
            ex.convention(at_rate);
            ex.convention(
                "first terminal flow: as given (`flow`), at the end of period N + 1, in the \
                 terms of the cash flows",
            );
        }
        Technique::NormalisedIncome(_) => {
            ex.convention(format!(
                "{on}, in the terms of the cash flows; capitalisation rate i = the terminal rate \
                 {rate} (R19, Decision 13): no separate capitalisation rate is accepted inside \
                 the DCF"
            ));
            ex.convention(
                "first terminal flow: the normalised income at N + 1 as given (`income`), in the \
                 terms of the cash flows",
            );
        }
        Technique::Liquidation(_) => {
            ex.convention(format!(
                "{on}, in the terms of the cash flows; the technique takes no discount rate"
            ));
            ex.convention(
                "terminal value from realisable values at N; no terminal flow is projected",
            );
        }
    }
    match tv_input(technique, rate, last_flow) {
        TvInput::Gordon(i) => {
            tv::gordon(&i).map(|(o, e)| (o.terminal_value, TechniqueOutput::Gordon(o), e))
        }
        TvInput::HModel(i) => {
            tv::h_model(&i).map(|(o, e)| (o.terminal_value, TechniqueOutput::HModel(o), e))
        }
        TvInput::ValueDriver(i) => tv::value_driver(&i)
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::ValueDriver(o), e)),
        TvInput::ExitMultiple(i) => tv::exit_multiple(&i)
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::ExitMultiple(o), e)),
        TvInput::LimitedHorizon(i) => tv::limited_horizon(&i)
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::LimitedHorizon(o), e)),
        TvInput::NormalisedIncome(i) => tv::normalised_income(&i)
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::NormalisedIncome(o), e)),
        TvInput::Liquidation(i) => {
            tv::liquidation(&i).map(|(o, e)| (o.terminal_value, TechniqueOutput::Liquidation(o), e))
        }
    }
}

/// What the engine passes to the terminal value: the rate, the last explicit flow and its
/// growth, the end-of-period factors of the explicit periods, and the compounding
/// factor G_N (`None` beyond the decimal range).
pub(crate) struct TerminalContext<'a> {
    pub technique: Technique<'a>,
    pub basis: TerminalBasis,
    pub timing: Timing,
    pub discount_time: Option<TerminalDiscountTime>,
    pub rate: Decimal,
    pub last_flow: Decimal,
    pub last_growth: Option<Decimal>,
    pub ends: &'a [Decimal],
    pub g_n: Option<Decimal>,
}

/// R19 to R22: the terminal value through the typed `tv` function, and its discounting.
pub(crate) fn value(ex: &mut Explain, c: &TerminalContext<'_>) -> Result<TerminalOutput> {
    let technique = c.technique;
    let (id, version) = (technique.id(), technique.version());
    let (tv_n, output, tv_ex) = compute(ex, technique, c.rate, c.last_flow, c.basis)?;
    for s in tv_ex.steps {
        ex.step(format!("Terminal value: {}", s.label), s.formula, s.value);
    }
    for conv in tv_ex.conventions {
        ex.convention(format!("terminal value ({id} v{version}): {conv}"));
    }
    if let (Some(last_growth), Some(g)) = (c.last_growth, technique.perpetual_growth()) {
        if last_growth != g {
            ex.convention(format!(
                "warning: the growth of the last explicit period ({last_growth}) differs from \
                 the perpetual growth of the terminal value ({g})"
            ));
        }
    }

    let n = c.ends.len();
    let df_n = c.ends[n - 1];
    let horizon = Decimal::from(n as u64);
    let g_n = c.g_n;
    let at_n = format!("Terminal value: discount factor at t = N = {n}, end of period {n}");
    let df_n_formula = match g_n {
        Some(g) => format!("DF_{n} = 1 / Π_(i ≤ {n}) (1 + r_i) = 1 / {g}"),
        None => format!("DF_{n}, the discount factor at the end of period {n}"),
    };
    let (time, df, pv) = match (c.timing, c.discount_time) {
        (Timing::MidPeriod, Some(TerminalDiscountTime::MidPeriod)) => {
            let r_t = c.rate;
            let root = one_plus(r_t)?
                .sqrt()
                .ok_or(FairmarkError::Overflow("square root"))?;
            let df = mul(df_n, root)?;
            ex.convention(format!(
                "terminal discounting time: mid_period, t = N − 0.5, factor DF_N × (1 + r_T)^0.5; \
                 the half-period shift uses the terminal rate r_T = {r_t} (R21, Decision 16)"
            ));
            if let Some(what) = technique.price_at_date() {
                ex.convention(format!(
                    "warning: {id} {what}; discounting it at N − 0.5 brings it forward half a \
                     period; it is discounted at N − 0.5 as chosen"
                ));
            }
            ex.step(at_n, df_n_formula, df_n);
            ex.step(
                format!(
                    "Terminal value: discount factor at t = N − 0.5 = {}",
                    horizon - Decimal::new(5, 1)
                ),
                format!("DF_{n} × (1 + r_T)^0.5 = {df_n} × (1 + {r_t})^0.5"),
                df,
            );
            let pv = match tv_n.checked_mul(root).zip(g_n) {
                Some((v, g)) => {
                    let pv = div(v, g)?;
                    ex.step(
                        "Present value of the terminal value",
                        format!("{tv_n} × (1 + {r_t})^0.5 / {g}"),
                        pv,
                    );
                    pv
                }
                None => {
                    let pv = mul(tv_n, df)?;
                    ex.step(
                        "Present value of the terminal value",
                        format!("{tv_n} × {df}"),
                        pv,
                    );
                    pv
                }
            };
            (sub(horizon, Decimal::new(5, 1))?, df, pv)
        }
        (timing, _) => {
            ex.convention(match timing {
                Timing::MidPeriod => "terminal discounting time: horizon, t = N, factor DF_N",
                Timing::EndOfPeriod => {
                    "terminal value discounted at the horizon, t = N, factor DF_N"
                }
            });
            if timing == Timing::MidPeriod && technique.is_perpetual_growth() {
                ex.convention(
                    "warning: under mid-period timing a perpetuity-growth terminal value is \
                     commonly discounted at N − 0.5, consistent with the flows it capitalises; \
                     it is discounted at the horizon N as chosen",
                );
            }
            ex.step(at_n, df_n_formula, df_n);
            let pv = match g_n {
                Some(g) => {
                    let pv = div(tv_n, g)?;
                    ex.step(
                        "Present value of the terminal value",
                        format!("{tv_n} / {g}"),
                        pv,
                    );
                    pv
                }
                None => {
                    let pv = mul(tv_n, df_n)?;
                    ex.step(
                        "Present value of the terminal value",
                        format!("{tv_n} × {df_n}"),
                        pv,
                    );
                    pv
                }
            };
            (horizon, df_n, pv)
        }
    };
    Ok(TerminalOutput {
        technique: id.to_owned(),
        technique_version: version.to_owned(),
        rate: c.rate,
        terminal_value: tv_n,
        discount_time: time,
        discount_factor: df,
        present_value: pv,
        output,
    })
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use rust_decimal::MathematicalOps;

    use crate::registry::run;

    fn with_terminal(terminal: Value, timing: Option<&str>) -> Value {
        let mut v = json!({
            "stages": [
                {"periods": 1, "projection": "given", "flows": ["100"]},
                {"periods": 2, "projection": "constant_growth", "growth": "0.03"}
            ],
            "base_flow": null,
            "cash_flow_terms": "nominal",
            "discount_rate_terms": "nominal",
            "wacc": "0.10",
            "timing": "end_of_period",
            "terminal": terminal
        });
        let m = v.as_object_mut().unwrap();
        m.remove("base_flow");
        if let Some(t) = timing {
            m.insert("timing".into(), json!("mid_period"));
            m.insert("terminal_discount_time".into(), json!(t));
        }
        v
    }

    fn techniques() -> Vec<(&'static str, Value)> {
        vec![
            (
                "tv.gordon",
                json!({"growth": "0.03", "first_flow": "last_flow_grown"}),
            ),
            (
                "tv.h_model",
                json!({"short_term_growth": "0.05", "long_term_growth": "0.02",
                       "half_life": "2", "first_flow": "last_flow_grown"}),
            ),
            (
                "tv.value_driver",
                json!({"nopat_next": "150", "growth": "0.03", "ronic": "0.12"}),
            ),
            (
                "tv.exit_multiple",
                json!({"multiple": "8", "metric": "ebitda", "metric_value": "160"}),
            ),
            (
                "tv.limited_horizon",
                json!({"flow": "110", "periods": 5, "growth": "0", "residual_value": "300"}),
            ),
            (
                "tv.normalised_income",
                json!({"income": "140", "basis": "enterprise", "growth": "0.01"}),
            ),
            (
                "tv.liquidation",
                json!({"assets": [{"label": "Plant", "value": "900"}], "liabilities": [],
                       "liquidation_costs": [], "tax_on_gains": "0", "basis": "enterprise"}),
            ),
        ]
    }

    fn conventions(v: &Value) -> Vec<String> {
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        serde_json::from_value(out["explain"]["conventions"].clone()).unwrap()
    }

    fn warnings(v: &Value) -> Vec<String> {
        conventions(v)
            .into_iter()
            .filter(|c| c.starts_with("warning: "))
            .collect()
    }

    /// R19, R28: every technique runs, reports its id and version, and states the technique and
    /// the first terminal flow (or why there is none) in the conventions.
    #[test]
    fn every_technique_reports_its_id_version_and_first_flow() {
        for (id, terms) in techniques() {
            let v = with_terminal(json!({ id: terms }), None);
            let out = run("dcf.fcff", &v.to_string()).unwrap();
            assert_eq!(out["value"]["terminal"]["technique"], id);
            assert_eq!(out["value"]["terminal"]["technique_version"], "1");
            let c = conventions(&v).join("\n");
            assert!(
                c.contains(&format!(
                    "terminal technique: {id} version 1 on the enterprise basis"
                )),
                "{id}: {c}"
            );
            assert!(
                c.contains("first terminal flow") || c.contains("no terminal flow"),
                "{id}: {c}"
            );
            assert!(
                c.contains(&format!("terminal value ({id} v1): ")),
                "{id}: {c}"
            );
        }
    }

    /// AC12, Decision 6: a price at a date discounted at N − 0.5 is a warning; a perpetual
    /// growth value discounted at N under mid-period timing is a warning; neither elsewhere.
    #[test]
    fn discounting_time_warnings_follow_the_technique() {
        let price = "brings it forward half a period";
        let perpetuity = "under mid-period timing a perpetuity-growth terminal value";
        for (id, terms) in techniques() {
            let at_mid = warnings(&with_terminal(
                json!({ id: terms.clone() }),
                Some("mid_period"),
            ));
            let at_n = warnings(&with_terminal(
                json!({ id: terms.clone() }),
                Some("horizon"),
            ));
            let end = warnings(&with_terminal(json!({ id: terms }), None));
            let has = |w: &[String], s: &str| w.iter().any(|x| x.contains(s));
            let is_price = matches!(
                id,
                "tv.exit_multiple" | "tv.liquidation" | "tv.limited_horizon"
            );
            let is_perpetuity = matches!(
                id,
                "tv.gordon" | "tv.h_model" | "tv.value_driver" | "tv.normalised_income"
            );
            assert_eq!(has(&at_mid, price), is_price, "{id} at N - 0.5: {at_mid:?}");
            assert_eq!(has(&at_n, perpetuity), is_perpetuity, "{id} at N: {at_n:?}");
            assert!(!has(&at_n, price) && !has(&at_mid, perpetuity), "{id}");
            assert!(!has(&end, price) && !has(&end, perpetuity), "{id}: {end:?}");
        }
    }

    /// R11: the growth of the last generated period is compared with the perpetual growth of
    /// the technique: `growth` for Gordon and the value driver, g_l for the H-model; no
    /// comparison for the techniques that state no perpetual growth.
    #[test]
    fn growth_continuity_uses_the_perpetual_growth_of_the_technique() {
        let differs = "differs from the perpetual growth";
        for (id, terms, warned) in [
            (
                "tv.gordon",
                json!({"growth": "0.03", "first_flow": "last_flow_grown"}),
                false,
            ),
            (
                "tv.gordon",
                json!({"growth": "0.02", "first_flow": "last_flow_grown"}),
                true,
            ),
            (
                "tv.h_model",
                json!({"short_term_growth": "0.05", "long_term_growth": "0.03",
                       "half_life": "2", "first_flow": "last_flow_grown"}),
                false,
            ),
            (
                "tv.h_model",
                json!({"short_term_growth": "0.03", "long_term_growth": "0.02",
                       "half_life": "2", "first_flow": "last_flow_grown"}),
                true,
            ),
            (
                "tv.value_driver",
                json!({"nopat_next": "150", "growth": "0.025", "ronic": "0.12"}),
                true,
            ),
            (
                "tv.exit_multiple",
                json!({"multiple": "8", "metric": "ebitda", "metric_value": "160",
                       "next_flow": "120"}),
                false,
            ),
            (
                "tv.normalised_income",
                json!({"income": "140", "basis": "enterprise", "growth": "0.03"}),
                false,
            ),
            (
                "tv.normalised_income",
                json!({"income": "140", "basis": "enterprise", "growth": "0"}),
                true,
            ),
        ] {
            let w = warnings(&with_terminal(json!({ id: terms }), None));
            assert_eq!(w.iter().any(|x| x.contains(differs)), warned, "{id}: {w:?}");
        }
    }

    /// AC11, AC14 (R19, R20, R27): every error of the terminal object names its field by a
    /// JSON pointer, with its stable code.
    #[test]
    fn terminal_errors_name_their_field() {
        let cases: Vec<(Value, &str, &str)> = vec![
            (
                json!({"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"},
                       "tv.liquidation": {"assets": [{"label": "A", "value": "1"}],
                       "liabilities": [], "liquidation_costs": [], "tax_on_gains": "0",
                       "basis": "enterprise"}}),
                "FM-E001",
                "/terminal",
            ),
            (
                json!({"tv.h_model": {"short_term_growth": "0.05", "long_term_growth": "0.02",
                       "half_life": "2", "first_flow": "normalised_base_flow"}}),
                "FM-E001",
                "/terminal/tv.h_model/normalised_base_flow",
            ),
            (
                json!({"tv.h_model": {"short_term_growth": "-1", "long_term_growth": "0.02",
                       "half_life": "2", "first_flow": "last_flow_grown"}}),
                "FM-E001",
                "/terminal/tv.h_model/short_term_growth",
            ),
            (
                json!({"tv.h_model": {"short_term_growth": "0.05", "long_term_growth": "0.10",
                       "half_life": "2", "first_flow": "last_flow_grown"}}),
                "FM-E004",
                "/terminal/tv.h_model/long_term_growth",
            ),
            (
                json!({"tv.h_model": {"short_term_growth": "0.05", "long_term_growth": "0.02",
                       "half_life": "-1", "first_flow": "last_flow_grown"}}),
                "FM-E018",
                "/terminal/tv.h_model/half_life",
            ),
            (
                json!({"tv.value_driver": {"nopat_next": "1", "growth": "0.11", "ronic": "0.2"}}),
                "FM-E004",
                "/terminal/tv.value_driver/growth",
            ),
            (
                json!({"tv.value_driver": {"nopat_next": "1", "growth": "0.02", "ronic": "0"}}),
                "FM-E018",
                "/terminal/tv.value_driver/ronic",
            ),
            (
                json!({"tv.exit_multiple": {"multiple": "12", "metric": "earnings",
                       "metric_value": "10"}}),
                "FM-E019",
                "/terminal/tv.exit_multiple/metric",
            ),
            (
                json!({"tv.exit_multiple": {"multiple": "0", "metric": "ebitda",
                       "metric_value": "10"}}),
                "FM-E018",
                "/terminal/tv.exit_multiple/multiple",
            ),
            (
                json!({"tv.exit_multiple": {"multiple": "8", "metric": "ebitda",
                       "metric_value": "-1"}}),
                "FM-E018",
                "/terminal/tv.exit_multiple/metric_value",
            ),
            (
                json!({"tv.exit_multiple": {"multiple": "8", "metric": "ebit",
                       "metric_value": "10", "growth_cap": "0.02"}}),
                "FM-E001",
                "/terminal/tv.exit_multiple/growth_cap",
            ),
            (
                json!({"tv.limited_horizon": {"flow": "1", "periods": 0, "growth": "0",
                       "residual_value": "0"}}),
                "FM-E018",
                "/terminal/tv.limited_horizon/periods",
            ),
            (
                json!({"tv.limited_horizon": {"flow": "1", "periods": 3, "growth": "-1",
                       "residual_value": "0"}}),
                "FM-E001",
                "/terminal/tv.limited_horizon/growth",
            ),
            (
                json!({"tv.liquidation": {"assets": [{"label": "A", "value": "1"}],
                       "liabilities": [], "liquidation_costs": [], "tax_on_gains": "0",
                       "basis": "equity"}}),
                "FM-E019",
                "/terminal/tv.liquidation/basis",
            ),
            (
                json!({"tv.liquidation": {"assets": [], "liabilities": [],
                       "liquidation_costs": [], "tax_on_gains": "0", "basis": "enterprise"}}),
                "FM-E001",
                "/terminal/tv.liquidation/assets",
            ),
            (
                json!({"tv.liquidation": {"assets": [{"label": "A", "value": "1"}],
                       "liabilities": [{"label": "B", "value": "-2"}],
                       "liquidation_costs": [], "tax_on_gains": "0", "basis": "enterprise"}}),
                "FM-E018",
                "/terminal/tv.liquidation/liabilities/0/value",
            ),
            (
                json!({"tv.liquidation": {"assets": [{"label": " ", "value": "1"}],
                       "liabilities": [], "liquidation_costs": [], "tax_on_gains": "0",
                       "basis": "enterprise"}}),
                "FM-E001",
                "/terminal/tv.liquidation/assets/0/label",
            ),
            (
                json!({"tv.liquidation": {"assets": [{"label": "A", "value": "1"}],
                       "liabilities": [], "liquidation_costs": [], "tax_on_gains": "-1",
                       "basis": "enterprise"}}),
                "FM-E018",
                "/terminal/tv.liquidation/tax_on_gains",
            ),
        ];
        let normalised = |extra: Value| {
            let mut terms = json!({"income": "100", "basis": "enterprise", "growth": "0.01"});
            for (k, v) in extra.as_object().unwrap() {
                terms[k] = v.clone();
            }
            json!({ "tv.normalised_income": terms })
        };
        let cases: Vec<(Value, &str, &str)> = cases
            .into_iter()
            .chain([
                (
                    normalised(json!({"basis": "equity"})),
                    "FM-E019",
                    "/terminal/tv.normalised_income/basis",
                ),
                (
                    normalised(json!({"growth": "0.10"})),
                    "FM-E004",
                    "/terminal/tv.normalised_income/growth",
                ),
                (
                    normalised(json!({"growth": "-1"})),
                    "FM-E001",
                    "/terminal/tv.normalised_income/growth",
                ),
            ])
            .collect();
        for (terminal, code, pointer) in cases {
            let v = with_terminal(terminal, None);
            let e = run("dcf.fcff", &v.to_string()).unwrap_err();
            assert_eq!(e.code(), code, "{pointer}: {e}");
            assert!(
                e.to_string().contains(&format!("`{pointer}")),
                "{pointer} not in: {e}"
            );
        }
    }

    /// AC11: the basis error states the fix.
    #[test]
    fn mixed_basis_states_the_fix() {
        let v = with_terminal(
            json!({"tv.exit_multiple": {"multiple": "1.2", "metric": "book_value",
                   "metric_value": "500"}}),
            None,
        );
        let e = run("dcf.fcff", &v.to_string()).unwrap_err().to_string();
        assert!(e.contains("book_value (P/BV), on the equity basis"), "{e}");
        assert!(e.contains("ebitda, ebit or revenue"), "{e}");
    }

    /// R22: a negative liquidation value is discounted as computed and the terminal value
    /// weight stays the share of the value of operations.
    #[test]
    fn negative_liquidation_value_is_discounted_as_computed() {
        let v = with_terminal(
            json!({"tv.liquidation": {"assets": [{"label": "Plant", "value": "100"}],
                   "liabilities": [{"label": "Payables", "value": "300"}],
                   "liquidation_costs": [], "tax_on_gains": "0", "basis": "enterprise"}}),
            None,
        );
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        let d = |p: &str| -> rust_decimal::Decimal {
            out["value"]
                .pointer(p)
                .unwrap()
                .as_str()
                .unwrap()
                .parse()
                .unwrap()
        };
        assert_eq!(
            d("/terminal/terminal_value"),
            rust_decimal::Decimal::from(-200)
        );
        assert!(d("/terminal/present_value").is_sign_negative());
        assert_eq!(
            d("/terminal_value_weight"),
            d("/terminal/present_value") / d("/value_of_operations")
        );
    }

    /// R19, Decision 13: the capitalisation rate of `tv.normalised_income` is the terminal
    /// rate; a separate one is refused, and a terminal rate of zero or below is named by its
    /// own field.
    #[test]
    fn normalised_income_is_capitalised_at_the_terminal_rate() {
        let terms = json!({"income": "100", "basis": "enterprise", "growth": "0.01"});
        let v = with_terminal(json!({"tv.normalised_income": terms.clone()}), None);
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        // 100 / (0.10 − 0.01)
        let tv: rust_decimal::Decimal = out["value"]["terminal"]["terminal_value"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(
            tv,
            rust_decimal::Decimal::from(100) / "0.09".parse::<rust_decimal::Decimal>().unwrap()
        );
        assert!(conventions(&v)
            .iter()
            .any(|c| c.contains("capitalisation rate i = the terminal rate 0.10")));

        let mut separate = terms.clone();
        separate["capitalisation_rate"] = json!("0.08");
        let e = run(
            "dcf.fcff",
            &with_terminal(json!({"tv.normalised_income": separate}), None).to_string(),
        )
        .unwrap_err();
        assert_eq!(e.code(), "FM-E001");
        assert!(e.to_string().contains("capitalisation_rate"), "{e}");

        let mut zero = with_terminal(json!({"tv.normalised_income": terms}), None);
        zero["wacc"] = json!("0");
        let e = run("dcf.fcff", &zero.to_string()).unwrap_err();
        assert_eq!(e.code(), "FM-E018", "{e}");
        assert!(e.to_string().contains("`/wacc`"), "{e}");
    }

    /// Decision 15: with a normalised base flow at N, the H-model gives the value it gives when
    /// that flow is the last explicit flow; the explicit flows are unchanged.
    #[test]
    fn h_model_normalised_base_flow_replaces_the_last_explicit_flow() {
        let terms = |first: &str| {
            let mut t = json!({"short_term_growth": "0.05", "long_term_growth": "0.02",
                               "half_life": "2", "first_flow": first});
            if first == "normalised_base_flow" {
                t["normalised_base_flow"] = json!("125");
            }
            json!({ "tv.h_model": t })
        };
        let normalised = run(
            "dcf.fcff",
            &with_terminal(terms("normalised_base_flow"), None).to_string(),
        )
        .unwrap();
        let mut as_last = with_terminal(terms("last_flow_grown"), None);
        as_last["stages"] = json!([{"periods": 3, "projection": "given",
                                    "flows": ["100", "103", "125"]}]);
        let as_last = run("dcf.fcff", &as_last.to_string()).unwrap();
        assert_eq!(
            normalised["value"]["terminal"]["terminal_value"],
            as_last["value"]["terminal"]["terminal_value"]
        );
        assert_eq!(normalised["value"]["periods"][2]["flow"], "106.0900");
        let c = normalised["explain"]["conventions"].to_string();
        assert!(
            c.contains("normalised base flow CF_N at the horizon N"),
            "{c}"
        );
    }

    /// Decision 16: at N − 0.5 the half-period shift uses the terminal rate r_T, also with rates
    /// per period: PV = TV × DF_N × (1 + r_T)^0.5.
    #[test]
    fn half_period_shift_uses_the_terminal_rate() {
        let mut v = with_terminal(
            json!({"tv.gordon": {"growth": "0.03", "first_flow": "last_flow_grown"}}),
            Some("mid_period"),
        );
        let m = v.as_object_mut().unwrap();
        m.remove("wacc");
        m.insert("wacc_per_period".into(), json!(["0.12", "0.11", "0.10"]));
        m.insert("terminal_wacc".into(), json!("0.08"));
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        let d = |p: &str| -> rust_decimal::Decimal {
            out["value"]
                .pointer(p)
                .unwrap()
                .as_str()
                .unwrap()
                .parse()
                .unwrap()
        };
        let one = rust_decimal::Decimal::ONE;
        let df_n = one
            / (d("/periods/0/discount_rate") + one)
            / (d("/periods/1/discount_rate") + one)
            / (d("/periods/2/discount_rate") + one);
        let root = ("1.08".parse::<rust_decimal::Decimal>().unwrap())
            .sqrt()
            .unwrap();
        let want = df_n * root;
        assert!((d("/terminal/discount_factor") - want).abs() < "1e-26".parse().unwrap());
        let c = out["explain"]["conventions"].to_string();
        assert!(
            c.contains("terminal rate r_T = 0.08 (R21, Decision 16)"),
            "{c}"
        );
    }
}
