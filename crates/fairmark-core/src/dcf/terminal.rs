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
    ExitMultipleInput, GordonInput, GordonOutput, HModelInput, HModelOutput, LimitedHorizonInput,
    Line, LiquidationInput, LiquidationOutput, MultipleMetric, TerminalBasis, ValueDriverInput,
    ValueDriverOutput,
};
use crate::types::Timing;

// ---------------------------------------------------------------- input types

/// The flow at N + 1 of a technique that grows the last explicit flow (R20).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum FirstTerminalFlow {
    /// CF_{N+1} = CF_N × (1 + g), the last explicit flow grown by the perpetual growth.
    LastFlowGrown,
    /// CF_{N+1} given in `normalised_next_flow`.
    NormalisedNextFlow,
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
/// flow CF_N is the last explicit flow.
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
    /// `last_flow_grown`: the H-model grows the last explicit flow CF_N; no default. A
    /// normalised flow at N + 1 is not accepted for this technique.
    pub first_flow: FirstTerminalFlow,
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
    /// Flow at N + 1 on the basis of the DCF, in currency units. Optional: when given, the
    /// perpetual growth implied by the multiple is computed at the terminal rate.
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
    /// Residual value at the end of period N + n, in currency units; 0 for none.
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
    /// Assets with their realisable values at N, in currency units, each zero or positive; at
    /// least one.
    pub assets: Vec<Line>,
    /// Liabilities settled at N, in currency units, each zero or positive.
    pub liabilities: Vec<Line>,
    /// Costs of the liquidation (legal fees, closure, severance), in currency units, each zero
    /// or positive.
    pub liquidation_costs: Vec<Line>,
    /// Tax on the gains realised, as an amount in currency units, zero or positive.
    pub tax_on_gains: Decimal,
    /// `enterprise` (the liabilities listed exclude financial debt) or `equity` (financial debt
    /// included); must be the basis of the DCF. Required, no default.
    pub basis: TerminalBasis,
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
            Self::ExitMultiple(_) | Self::LimitedHorizon(_) | Self::Liquidation(_) => None,
        }
    }

    /// A value capitalising flows that grow forever: commonly discounted at N − 0.5 under
    /// mid-period timing, like the flows it capitalises.
    fn is_perpetual_growth(self) -> bool {
        matches!(
            self,
            Self::Gordon(_) | Self::HModel(_) | Self::ValueDriver(_)
        )
    }

    /// A price at the date N (a sale at a multiple, a liquidation): Decision 6 warns when it is
    /// discounted at N − 0.5.
    fn is_price_at_date(self) -> bool {
        matches!(self, Self::ExitMultiple(_) | Self::Liquidation(_))
    }
}

fn invalid(msg: String) -> FairmarkError {
    FairmarkError::InvalidInput(msg)
}

fn domain(msg: String) -> FairmarkError {
    FairmarkError::TerminalValueDomain(msg)
}

fn check_growth(at: &str, field: &str, g: Decimal) -> Result<()> {
    if g <= -Decimal::ONE {
        return Err(invalid(format!(
            "`{at}/{field}` is {g}; a growth must be greater than -1 (-100%)"
        )));
    }
    Ok(())
}

fn check_below_rate(
    at: &str,
    field: &str,
    g: Decimal,
    rate: Decimal,
    rate_field: &str,
) -> Result<()> {
    if g >= rate {
        return Err(FairmarkError::GrowthNotBelowTerminalRate {
            growth_field: format!("{at}/{field}"),
            growth: g,
            rate_field: rate_field.to_owned(),
            rate,
        });
    }
    Ok(())
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
             liquidation whose liabilities exclude financial debt (basis enterprise)"
        }
        TerminalBasis::Equity => {
            "use an equity measure: an exit multiple on earnings or book_value, a liquidation \
             whose liabilities include financial debt (basis equity)"
        }
    };
    Err(FairmarkError::MixedBasis(format!(
        "`{at}/{field}` is {what}, on the {} basis, but this DCF values on the {} basis; {fix}",
        found.name(),
        dcf.name()
    )))
}

/// Lines of a liquidation group: a label and a value of zero or more (as `tv.liquidation`
/// requires), checked here so that the error names the line by its pointer.
fn check_lines(at: &str, field: &str, lines: &[Line]) -> Result<()> {
    for (i, line) in lines.iter().enumerate() {
        if line.label.trim().is_empty() {
            return Err(invalid(format!(
                "`{at}/{field}/{i}/label` is empty; name the line"
            )));
        }
        if line.value.is_sign_negative() && !line.value.is_zero() {
            return Err(domain(format!(
                "`{at}/{field}/{i}/value` is {}; a value is zero or more",
                line.value
            )));
        }
    }
    Ok(())
}

/// R19, R20: exactly one technique, on the basis of the DCF, with the first terminal flow
/// stated; every term checked with its JSON pointer (R27).
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
    match technique {
        Technique::Gordon(g) => {
            match (g.first_flow, g.normalised_next_flow) {
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
            }
            check_growth(at, "growth", g.growth)?;
            check_below_rate(at, "growth", g.growth, rate, rate_field)?;
        }
        Technique::HModel(h) => {
            if h.first_flow != FirstTerminalFlow::LastFlowGrown {
                return Err(invalid(format!(
                    "`{at}/first_flow` is normalised_next_flow, which tv.h_model does not \
                     accept: the H-model is defined on the last explicit flow CF_N; set \
                     `{at}/first_flow` to last_flow_grown, or use tv.gordon with a normalised \
                     flow"
                )));
            }
            check_growth(at, "short_term_growth", h.short_term_growth)?;
            check_growth(at, "long_term_growth", h.long_term_growth)?;
            if h.half_life.is_sign_negative() && !h.half_life.is_zero() {
                return Err(domain(format!(
                    "`{at}/half_life` is {}; the half-life is zero or more periods",
                    h.half_life
                )));
            }
            check_below_rate(at, "long_term_growth", h.long_term_growth, rate, rate_field)?;
        }
        Technique::ValueDriver(v) => {
            check_growth(at, "growth", v.growth)?;
            if v.ronic <= Decimal::ZERO {
                return Err(domain(format!(
                    "`{at}/ronic` is {}; the return on new invested capital must be greater \
                     than 0",
                    v.ronic
                )));
            }
            check_below_rate(at, "growth", v.growth, rate, rate_field)?;
        }
        Technique::ExitMultiple(e) => {
            check_basis(
                at,
                "metric",
                e.metric.basis(),
                format!("{} ({})", e.metric.name(), e.metric.multiple_name()),
                basis,
            )?;
            if e.multiple <= Decimal::ZERO {
                return Err(domain(format!(
                    "`{at}/multiple` is {}; the multiple must be greater than 0",
                    e.multiple
                )));
            }
            if e.metric_value <= Decimal::ZERO {
                return Err(domain(format!(
                    "`{at}/metric_value` is {}; the metric must be greater than 0 to carry a \
                     multiple",
                    e.metric_value
                )));
            }
            if e.growth_cap.is_some() && e.next_flow.is_none() {
                return Err(invalid(format!(
                    "`{at}/growth_cap` needs `{at}/next_flow`: the cap is checked on the growth \
                     implied by the multiple; give the flow at N + 1, or remove the cap"
                )));
            }
        }
        Technique::LimitedHorizon(l) => {
            if !(1..=crate::tvm::MAX_PERIODS).contains(&l.periods) {
                return Err(domain(format!(
                    "`{at}/periods` is {}; a finite horizon has 1 to {} periods",
                    l.periods,
                    crate::tvm::MAX_PERIODS
                )));
            }
            check_growth(at, "growth", l.growth)?;
        }
        Technique::Liquidation(q) => {
            check_basis(
                at,
                "basis",
                q.basis,
                format!("{} (declared)", q.basis.name()),
                basis,
            )?;
            if q.assets.is_empty() {
                return Err(invalid(format!(
                    "`{at}/assets` is empty; give at least one asset to liquidate"
                )));
            }
            check_lines(at, "assets", &q.assets)?;
            check_lines(at, "liabilities", &q.liabilities)?;
            check_lines(at, "liquidation_costs", &q.liquidation_costs)?;
            if q.tax_on_gains.is_sign_negative() && !q.tax_on_gains.is_zero() {
                return Err(domain(format!(
                    "`{at}/tax_on_gains` is {}; the tax on gains is zero or more",
                    q.tax_on_gains
                )));
            }
        }
    }
    Ok(technique)
}

// ---------------------------------------------------------------- calculation

/// An error raised by the technique itself names the technique's object by its pointer
/// (R27); its code is unchanged.
fn at_pointer(e: FairmarkError, at: &str) -> FairmarkError {
    match e {
        FairmarkError::InvalidInput(m) => FairmarkError::InvalidInput(format!("`{at}`: {m}")),
        FairmarkError::TerminalValueDomain(m) => {
            FairmarkError::TerminalValueDomain(format!("`{at}`: {m}"))
        }
        other => other,
    }
}

/// The technique computed through its typed `tv` function (ADR 0009), with its conventions.
fn compute(
    ex: &mut Explain,
    technique: Technique<'_>,
    rate: Decimal,
    last_flow: Decimal,
    basis: TerminalBasis,
) -> Result<(Decimal, TechniqueOutput, Explain)> {
    let (id, version) = (technique.id(), technique.version());
    let at = format!("/terminal/{id}");
    let on = format!(
        "terminal technique: {id} version {version} on the {} basis",
        basis.name()
    );
    let at_rate = format!("{on}, at the terminal rate {rate}, in the terms of the cash flows");
    let result = match technique {
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
            let (next_flow, last) = match t.first_flow {
                FirstTerminalFlow::LastFlowGrown => (None, Some(last_flow)),
                FirstTerminalFlow::NormalisedNextFlow => (t.normalised_next_flow, None),
            };
            tv::gordon(&GordonInput {
                rate,
                growth: t.growth,
                next_flow,
                last_flow: last,
                growth_cap: t.growth_cap,
            })
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::Gordon(o), e))
        }
        Technique::HModel(t) => {
            ex.convention(at_rate);
            ex.convention(
                "first terminal flow: the last explicit flow CF_N, grown by the H-model, \
                 CF_N × [(1 + g_l) + H × (g_s − g_l)]",
            );
            tv::h_model(&HModelInput {
                last_flow,
                short_term_growth: t.short_term_growth,
                long_term_growth: t.long_term_growth,
                half_life: t.half_life,
                rate,
                growth_cap: t.growth_cap,
            })
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::HModel(o), e))
        }
        Technique::ValueDriver(t) => {
            ex.convention(at_rate);
            ex.convention(
                "first terminal flow: NOPAT at N + 1 as given (`nopat_next`), less the \
                 reinvestment g / RONIC, in the terms of the cash flows",
            );
            tv::value_driver(&ValueDriverInput {
                nopat_next: t.nopat_next,
                growth: t.growth,
                ronic: t.ronic,
                rate,
                growth_cap: t.growth_cap,
            })
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::ValueDriver(o), e))
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
            tv::exit_multiple(&ExitMultipleInput {
                multiple: t.multiple,
                metric: t.metric,
                metric_value: t.metric_value,
                rate: t.next_flow.map(|_| rate),
                next_flow: t.next_flow,
                growth_cap: t.growth_cap,
            })
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::ExitMultiple(o), e))
        }
        Technique::LimitedHorizon(t) => {
            ex.convention(at_rate);
            ex.convention(
                "first terminal flow: as given (`flow`), at the end of period N + 1, in the \
                 terms of the cash flows",
            );
            tv::limited_horizon(&LimitedHorizonInput {
                flow: t.flow,
                rate,
                periods: t.periods,
                growth: Some(t.growth),
                residual_value: Some(t.residual_value),
                growth_cap: t.growth_cap,
            })
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::LimitedHorizon(o), e))
        }
        Technique::Liquidation(t) => {
            ex.convention(format!(
                "{on}, in the terms of the cash flows; the technique takes no discount rate"
            ));
            ex.convention(
                "terminal value from realisable values at N; no terminal flow is projected",
            );
            tv::liquidation(&LiquidationInput {
                assets: t.assets.clone(),
                liabilities: t.liabilities.clone(),
                liquidation_costs: t.liquidation_costs.clone(),
                tax_on_gains: t.tax_on_gains,
                basis: t.basis,
            })
            .map(|(o, e)| (o.terminal_value, TechniqueOutput::Liquidation(o), e))
        }
    };
    result.map_err(|e| at_pointer(e, &at))
}

/// What the engine passes to the terminal value: the rate, the last explicit flow and its
/// growth, the rates and end-of-period factors of the explicit periods, and the compounding
/// factor G_N (`None` beyond the decimal range).
pub(crate) struct TerminalContext<'a> {
    pub technique: Technique<'a>,
    pub basis: TerminalBasis,
    pub timing: Timing,
    pub discount_time: Option<TerminalDiscountTime>,
    pub rate: Decimal,
    pub last_flow: Decimal,
    pub last_growth: Option<Decimal>,
    pub rates: &'a [Decimal],
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
            let r_n = c.rates[n - 1];
            let root = one_plus(r_n)?
                .sqrt()
                .ok_or(FairmarkError::Overflow("square root"))?;
            let df = mul(df_n, root)?;
            ex.convention(format!(
                "terminal discounting time: mid_period, t = N − 0.5, factor DF_N × (1 + r_N)^0.5; \
                 the half-period shift uses the rate of period N, r_{n} = {r_n} (R21), not the \
                 terminal rate"
            ));
            if technique.is_price_at_date() {
                ex.convention(format!(
                    "warning: {id} is a price at the horizon N (a sale or a liquidation at a \
                     date); discounting it at N − 0.5 brings it forward half a period; it is \
                     discounted at N − 0.5 as chosen"
                ));
            }
            ex.step(at_n, df_n_formula, df_n);
            ex.step(
                format!(
                    "Terminal value: discount factor at t = N − 0.5 = {}",
                    horizon - Decimal::new(5, 1)
                ),
                format!("DF_{n} × (1 + r_{n})^0.5 = {df_n} × (1 + {r_n})^0.5"),
                df,
            );
            let pv = match tv_n.checked_mul(root).zip(g_n) {
                Some((v, g)) => {
                    let pv = div(v, g)?;
                    ex.step(
                        "Present value of the terminal value",
                        format!("{tv_n} × (1 + {r_n})^0.5 / {g}"),
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
        let price = "is a price at the horizon N";
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
            let is_price = matches!(id, "tv.exit_multiple" | "tv.liquidation");
            let is_perpetuity = matches!(id, "tv.gordon" | "tv.h_model" | "tv.value_driver");
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
                json!({"multiple": "8", "metric": "ebitda", "metric_value": "160"}),
                false,
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
                       "half_life": "2", "first_flow": "normalised_next_flow"}}),
                "FM-E001",
                "/terminal/tv.h_model/first_flow",
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
}
