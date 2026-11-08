//! The shared DCF engine (SPEC-0001): explicit stages, discount rates, terms, timing and the
//! terminal value, independent of the basis.
//!
//! Each DCF method (`dcf.fcff`, later `dcf.fcfe`) maps its own fields to [`EngineInput`],
//! calls [`run`] through a typed call (ADR 0009) and adds its own bridge to equity value. The
//! engine never reads the method's field names except to name them in errors (R27), through
//! [`RateFields`].
//!
//! Every choice with several options is a flat object with a discriminating enumeration and
//! optional fields, checked here at run time, so that a new option is an additive change of
//! the method's schema (R29; see `specs/0001-dcf-complete/plan.md`).

use rust_decimal::{Decimal, MathematicalOps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, div, mul, one_plus, sub};
use crate::tv::{self, GordonInput, GordonOutput, TerminalBasis};
use crate::tvm::{period_discount_factors, PeriodTiming};
use crate::types::Timing;

use super::free_cash_flows::{
    build_up, FreeCashFlowLines, FreeCashFlows, Location, TaxChoice, FORMULA_FCFF,
};
use std::borrow::Cow;

/// Explicit stages accepted (Decision 2). Checked at run time, not in the schema, so that more
/// stages remain an additive change (R29).
pub(crate) const MAX_STAGES: usize = 3;
/// Longest explicit horizon, in periods: a guard against inputs that would allocate without
/// bound, far beyond any business plan.
pub(crate) const MAX_PERIODS: u64 = 10_000;

// ---------------------------------------------------------------- input types

/// How the flows of a stage are obtained (Decision 3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Projection {
    /// The flows are given, one per period, in `flows`, or built up from line items in
    /// `lines` (exactly one of the two).
    Given,
    /// Each flow is the previous flow × (1 + `growth`).
    ConstantGrowth,
    /// The growth moves from `from_growth` to `to_growth` in equal steps,
    /// g_k = g_from + (g_to − g_from) × k / m, reaching `to_growth` in the last period
    /// (Decision 4).
    LinearDecline,
}

impl Projection {
    fn label(self) -> &'static str {
        match self {
            Self::Given => "given",
            Self::ConstantGrowth => "constant_growth",
            Self::LinearDecline => "linear_decline",
        }
    }
}

/// One explicit stage: a number of periods and how its flows are obtained. Each projection
/// takes only its own fields: `flows` or `lines` for `given`, `growth` for `constant_growth`,
/// `from_growth` and `to_growth` for `linear_decline`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Stage {
    /// Number of periods of the stage, at least 1; one period is one year.
    #[schemars(range(min = 1))]
    pub periods: u32,
    /// `given`, `constant_growth` or `linear_decline`; no default.
    pub projection: Projection,
    /// For `given`: the flows of the stage in currency units, exactly one per period, in the
    /// terms of `cash_flow_terms`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub flows: Option<Vec<Decimal>>,
    /// For `given`, instead of `flows`: the line items of each period (EBIT, taxes on EBIT,
    /// depreciation and amortisation, capital expenditure, change in net working capital,
    /// adjustments), exactly one per period, in currency units in the terms of
    /// `cash_flow_terms`; the flow of each period is built up as in `dcf.free_cash_flows`, with
    /// the tax fields of the method (SPEC-0001 R7). `interest_expense` and `net_borrowing` apply
    /// only where FCFE is discounted: they are refused on the enterprise basis (`dcf.fcff`).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub lines: Option<Vec<FreeCashFlowLines>>,
    /// For `constant_growth`: growth per period, decimal fraction, greater than -1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth: Option<Decimal>,
    /// For `linear_decline`: the growth before the stage, decimal fraction per period, greater
    /// than -1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub from_growth: Option<Decimal>,
    /// For `linear_decline`: the growth of the last period of the stage, decimal fraction per
    /// period, greater than -1.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub to_growth: Option<Decimal>,
}

/// Whether amounts and growth rates, or discount rates, include expected inflation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Terms {
    /// At current prices: expected inflation included.
    Nominal,
    /// At constant prices: expected inflation excluded.
    Real,
}

impl Terms {
    fn label(self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::Real => "real",
        }
    }
}

/// When the terminal value is discounted under mid-period timing (Decision 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerminalDiscountTime {
    /// At the horizon, t = N: factor DF_N.
    Horizon,
    /// Half a period earlier, t = N − 0.5: factor DF_N × (1 + r_N)^0.5.
    MidPeriod,
}

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
}

// ---------------------------------------------------------------- output types

/// One period of the explicit horizon (R10).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PeriodRow {
    /// Period number k, 1 to N.
    pub period: u32,
    /// Stage number, 1 to 3.
    pub stage: u32,
    /// Projection of the stage.
    pub projection: Projection,
    /// Growth applied to the previous flow, decimal fraction; `null` for given flows.
    pub growth: Option<Decimal>,
    /// Flow of the period, in currency units.
    pub flow: Decimal,
    /// Discount rate given for the period, decimal fraction.
    pub discount_rate: Decimal,
    /// Discount rate used, in the terms of the cash flows, decimal fraction.
    pub rate_used: Decimal,
    /// Discount factor of the flow under the timing chosen.
    pub discount_factor: Decimal,
    /// Present value of the flow, in currency units.
    pub present_value: Decimal,
}

/// The output of the terminal technique, as its own method returns it.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum TechniqueOutput {
    Gordon(GordonOutput),
}

/// The terminal value and its discounting (R19 to R22).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TerminalOutput {
    /// Method id of the technique, for example `tv.gordon`.
    pub technique: String,
    /// Version of the technique's method that produced the value.
    pub technique_version: String,
    /// Rate of the terminal phase used, decimal fraction per period, in the terms of the cash
    /// flows.
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

// ---------------------------------------------------------------- engine input and output

/// JSON names of the rate fields of the calling method, for error messages.
pub(crate) struct RateFields {
    pub single: &'static str,
    pub per_period: &'static str,
    pub terminal: &'static str,
}

/// The rates as given by the calling method; exactly one form is valid (R12).
pub(crate) struct RateInput<'a> {
    pub single: Option<Decimal>,
    pub per_period: Option<&'a [Decimal]>,
    pub terminal: Option<Decimal>,
    pub fields: RateFields,
}

pub(crate) struct EngineInput<'a> {
    pub basis: TerminalBasis,
    pub stages: &'a [Stage],
    pub base_flow: Option<Decimal>,
    pub cash_flow_terms: Terms,
    pub discount_rate_terms: Terms,
    pub rates: RateInput<'a>,
    pub timing: Timing,
    pub terminal_discount_time: Option<TerminalDiscountTime>,
    pub terminal: &'a Terminal,
    /// Tax fields of the build-up for stages with `lines` (R7); all `None` otherwise.
    pub taxes: TaxChoice,
}

pub(crate) struct EngineOutput {
    pub periods: Vec<PeriodRow>,
    pub pv_explicit: Decimal,
    pub terminal: TerminalOutput,
}

fn invalid(msg: String) -> FairmarkError {
    FairmarkError::InvalidInput(msg)
}

fn count(msg: String) -> FairmarkError {
    FairmarkError::CountMismatch(msg)
}

// ---------------------------------------------------------------- checks

/// R6, R7, R8, R11, R29: stage count, horizon, the fields of each projection, base flow.
/// A stage after validation: the projection with its own, present, fields (R7 to R9).
enum StageKind<'a> {
    /// Given flows: as given in `flows`, or built up from `lines` by [`build_up_stages`].
    Given(Cow<'a, [Decimal]>),
    /// Given line items, before the build-up; never reaches the projection.
    Lines(&'a [FreeCashFlowLines]),
    ConstantGrowth(Decimal),
    LinearDecline {
        from: Decimal,
        to: Decimal,
    },
}

struct ResolvedStage<'a> {
    periods: u32,
    projection: Projection,
    kind: StageKind<'a>,
    /// True when the flows were built up from `lines`.
    built_up: bool,
}

fn check_stages(
    stages: &[Stage],
    base_flow: Option<Decimal>,
) -> Result<(Vec<ResolvedStage<'_>>, usize)> {
    if stages.is_empty() || stages.len() > MAX_STAGES {
        return Err(invalid(format!(
            "`/stages` has {} stages; give one to {MAX_STAGES} explicit stages, each followed \
             by the next and the last by the terminal value",
            stages.len()
        )));
    }
    let mut horizon: u64 = 0;
    let mut resolved = Vec::with_capacity(stages.len());
    for (i, s) in stages.iter().enumerate() {
        let at = format!("/stages/{i}");
        if s.periods == 0 {
            return Err(invalid(format!(
                "`{at}/periods` is 0; a stage has at least 1 period"
            )));
        }
        horizon += u64::from(s.periods);
        if s.projection == Projection::Given {
            match (&s.flows, &s.lines) {
                (Some(_), Some(_)) => {
                    return Err(invalid(format!(
                        "`{at}/flows` and `{at}/lines` are both given; a `given` stage takes its \
                         flows as amounts (`flows`) or as line items (`lines`), not both"
                    )))
                }
                (None, None) => {
                    return Err(invalid(format!(
                        "`{at}/flows` or `{at}/lines` is required by the projection `given`: \
                         give the flows as amounts, or the line items to build them up"
                    )))
                }
                _ => {}
            }
        } else if s.lines.is_some() {
            return Err(invalid(format!(
                "`{at}/lines` is not used by the projection `{}`; remove it (line items belong \
                 to a `given` stage)",
                s.projection.label()
            )));
        }
        // For `given`, `flows` stands for `flows` or `lines`, checked above.
        let present = [
            ("flows", s.flows.is_some() || s.lines.is_some()),
            ("growth", s.growth.is_some()),
            ("from_growth", s.from_growth.is_some()),
            ("to_growth", s.to_growth.is_some()),
        ];
        let own: &[&str] = match s.projection {
            Projection::Given => &["flows"],
            Projection::ConstantGrowth => &["growth"],
            Projection::LinearDecline => &["from_growth", "to_growth"],
        };
        for (field, given) in present {
            let belongs = own.contains(&field);
            if given && !belongs {
                return Err(invalid(format!(
                    "`{at}/{field}` is not used by the projection `{}`; remove it (the \
                     projection takes {})",
                    s.projection.label(),
                    own.join(" and ")
                )));
            }
            if !given && belongs {
                return Err(invalid(format!(
                    "`{at}/{field}` is required by the projection `{}`",
                    s.projection.label()
                )));
            }
        }
        if let Some(lines) = &s.lines {
            if lines.len() != s.periods as usize {
                return Err(count(format!(
                    "stage {} (`{at}/lines`) has {} lines for {} periods; give exactly {} \
                     lines or set `{at}/periods` to {}",
                    i + 1,
                    lines.len(),
                    s.periods,
                    s.periods,
                    lines.len()
                )));
            }
        }
        if let Some(flows) = &s.flows {
            if flows.len() != s.periods as usize {
                return Err(count(format!(
                    "stage {} (`{at}/flows`) has {} flows for {} periods; give exactly {} \
                     flows or set `{at}/periods` to {}",
                    i + 1,
                    flows.len(),
                    s.periods,
                    s.periods,
                    flows.len()
                )));
            }
        }
        for (field, g) in [
            ("growth", s.growth),
            ("from_growth", s.from_growth),
            ("to_growth", s.to_growth),
        ] {
            if let Some(g) = g {
                if g <= -Decimal::ONE {
                    return Err(invalid(format!(
                        "`{at}/{field}` is {g}; a growth must be greater than -1 (-100%); \
                         growth rates are decimal fractions, 5% is 0.05"
                    )));
                }
            }
        }
        let kind = match (
            s.projection,
            s.flows.as_deref(),
            s.growth,
            s.from_growth,
            s.to_growth,
        ) {
            (Projection::Given, Some(flows), ..) => StageKind::Given(Cow::Borrowed(flows)),
            (Projection::Given, None, ..) => match s.lines.as_deref() {
                Some(lines) => StageKind::Lines(lines),
                None => return Err(FairmarkError::Internal("given stage without flows")),
            },
            (Projection::ConstantGrowth, _, Some(g), ..) => StageKind::ConstantGrowth(g),
            (Projection::LinearDecline, _, _, Some(from), Some(to)) => {
                StageKind::LinearDecline { from, to }
            }
            _ => return Err(FairmarkError::Internal("stage fields not checked")),
        };
        resolved.push(ResolvedStage {
            periods: s.periods,
            projection: s.projection,
            kind,
            built_up: false,
        });
    }
    if horizon > MAX_PERIODS {
        return Err(invalid(format!(
            "`/stages` add up to {horizon} periods; the explicit horizon is at most \
             {MAX_PERIODS} periods"
        )));
    }
    let generated = stages[0].projection != Projection::Given;
    match (generated, base_flow) {
        (true, None) => Err(count(format!(
            "`/base_flow` is missing: the first stage (`/stages/0`) is `{}` and grows from the \
             base flow CF_0; give `/base_flow`",
            stages[0].projection.label()
        ))),
        (false, Some(_)) => Err(count(
            "`/base_flow` is given but the first stage (`/stages/0`) is `given`; remove \
             `/base_flow`, or make the first stage a generated one"
                .to_owned(),
        )),
        _ => Ok((resolved, horizon as usize)),
    }
}

/// R12, R13: one rate for every period and the terminal value, or one per period plus a
/// terminal rate. Returns the period rates and the terminal rate with its field name.
fn resolve_rates(r: &RateInput<'_>, n: usize) -> Result<(Vec<Decimal>, Decimal, String)> {
    let f = &r.fields;
    let (rates, terminal, terminal_field) = match (r.single, r.per_period, r.terminal) {
        (Some(_), Some(_), _) => {
            return Err(invalid(format!(
                "give exactly one of `/{}` and `/{}`, not both",
                f.single, f.per_period
            )))
        }
        (None, None, _) => {
            return Err(invalid(format!(
                "give the discount rate: `/{}` for every period, or `/{}` with `/{}`",
                f.single, f.per_period, f.terminal
            )))
        }
        (Some(_), None, Some(_)) => {
            return Err(invalid(format!(
                "`/{}` is not accepted with `/{}`: one rate is used for every period and for \
                 the terminal value; remove `/{}`, or give `/{}` instead",
                f.terminal, f.single, f.terminal, f.per_period
            )))
        }
        (None, Some(_), None) => {
            return Err(invalid(format!(
                "`/{}` is required with `/{}`: give the rate of the terminal phase",
                f.terminal, f.per_period
            )))
        }
        (Some(rate), None, None) => (vec![rate; n], rate, format!("/{}", f.single)),
        (None, Some(list), Some(term)) => {
            if list.len() != n {
                return Err(count(format!(
                    "`/{}` has {} rates for an explicit horizon of {n} periods (the sum of \
                     `/stages/*/periods`); give exactly {n} rates",
                    f.per_period,
                    list.len()
                )));
            }
            (list.to_vec(), term, format!("/{}", f.terminal))
        }
    };
    for (k, rate) in rates.iter().enumerate() {
        if *rate <= -Decimal::ONE {
            let field = match r.single {
                Some(_) => format!("/{}", f.single),
                None => format!("/{}/{k}", f.per_period),
            };
            return Err(FairmarkError::DiscountRateOutOfRange { field, rate: *rate });
        }
    }
    if terminal <= -Decimal::ONE {
        return Err(FairmarkError::DiscountRateOutOfRange {
            field: terminal_field,
            rate: terminal,
        });
    }
    Ok((rates, terminal, terminal_field))
}

/// R14, R15: the terms are required on both sides; different terms are refused until expected
/// inflation is accepted (#166).
fn check_terms(ex: &mut Explain, flows: Terms, rates: Terms) -> Result<()> {
    if flows != rates {
        return Err(FairmarkError::TermsMismatch(format!(
            "`/cash_flow_terms` is {} and `/discount_rate_terms` is {}, and no expected \
             inflation is given; discounting {} flows at {} rates misstates the value. Restate \
             the discount rates or the cash flows in the same terms",
            flows.label(),
            rates.label(),
            flows.label(),
            rates.label()
        )));
    }
    ex.convention(format!(
        "terms: cash flows and growth rates {}, discount rates {}; same terms, no conversion",
        flows.label(),
        rates.label()
    ));
    Ok(())
}

/// R21: the terminal discounting time is required under mid-period timing and refused at end
/// of period.
fn check_timing(timing: Timing, time: Option<TerminalDiscountTime>) -> Result<()> {
    match (timing, time) {
        (Timing::EndOfPeriod, Some(_)) => Err(invalid(
            "`/terminal_discount_time` is not accepted with `/timing` = end_of_period: the \
             terminal value is then discounted at the horizon N; remove it"
                .to_owned(),
        )),
        (Timing::MidPeriod, None) => Err(invalid(
            "`/terminal_discount_time` is required with `/timing` = mid_period: give `horizon` \
             (t = N) or `mid_period` (t = N - 0.5)"
                .to_owned(),
        )),
        _ => Ok(()),
    }
}

/// R19, R20: exactly one technique, with the first terminal flow stated.
fn check_terminal<'a>(
    t: &'a Terminal,
    rate: Decimal,
    rate_field: &str,
) -> Result<&'a GordonTerminal> {
    let Some(g) = &t.gordon else {
        return Err(invalid(
            "`/terminal` names no technique: give exactly one, written with its method id, \
             for example {\"tv.gordon\": {\"growth\": 0.02, \"first_flow\": \"last_flow_grown\"}}"
                .to_owned(),
        ));
    };
    let at = "/terminal/tv.gordon";
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
                 last_flow_grown; remove it, or set `{at}/first_flow` to normalised_next_flow"
            )))
        }
        _ => {}
    }
    if g.growth <= -Decimal::ONE {
        return Err(invalid(format!(
            "`{at}/growth` is {}; a growth must be greater than -1 (-100%)",
            g.growth
        )));
    }
    if g.growth >= rate {
        return Err(FairmarkError::GrowthNotBelowTerminalRate {
            growth_field: format!("{at}/growth"),
            growth: g.growth,
            rate_field: rate_field.to_owned(),
            rate,
        });
    }
    Ok(g)
}

// ---------------------------------------------------------------- calculation

struct Projected {
    stage: u32,
    projection: Projection,
    growth: Option<Decimal>,
    flow: Decimal,
}

/// R7 to R11: the flows of every period, with their explain steps and warnings.
fn project(
    ex: &mut Explain,
    stages: &[ResolvedStage<'_>],
    base_flow: Option<Decimal>,
) -> Result<Vec<Projected>> {
    let mut out: Vec<Projected> = Vec::new();
    let mut previous = base_flow;
    if let Some(b) = base_flow {
        ex.step("Base flow CF_0", "as given", b);
        ex.convention(format!(
            "base flow CF_0 = {b}, the flow of the period before the first generated stage"
        ));
    }
    let mut first = 1u32;
    for (i, s) in stages.iter().enumerate() {
        let stage = i as u32 + 1;
        let last = first + s.periods - 1;
        ex.convention(match s.kind {
            StageKind::Given(_) if s.built_up => format!(
                "stage {stage}: periods {first} to {last}, given flows built up from line items"
            ),
            StageKind::Given(_) => {
                format!("stage {stage}: periods {first} to {last}, given flows")
            }
            StageKind::Lines(_) => return Err(FairmarkError::Internal("lines not built up")),
            StageKind::ConstantGrowth(g) => format!(
                "stage {stage}: periods {first} to {last}, constant growth {g} per period: \
                 CF_k = CF_(k-1) × (1 + g)"
            ),
            StageKind::LinearDecline { from, to } => format!(
                "stage {stage}: periods {first} to {last}, growth declining in linear \
                 increments from {from} to {to}: g_j = g_from + (g_to − g_from) × j / {}, \
                 reaching g_to in the last period",
                s.periods
            ),
        });
        for j in 1..=s.periods {
            let k = first + j - 1;
            let growth = match s.kind {
                StageKind::Given(_) | StageKind::Lines(_) => None,
                StageKind::ConstantGrowth(g) => Some(g),
                StageKind::LinearDecline { from, to } => {
                    let m = Decimal::from(s.periods);
                    let step = div(mul(sub(to, from)?, Decimal::from(j))?, m)?;
                    let g = add(from, step)?;
                    ex.step(
                        format!("Period {k}: growth"),
                        format!("{from} + ({to} − {from}) × {j} / {m}"),
                        g,
                    );
                    Some(g)
                }
            };
            let flow = match (&s.kind, growth) {
                (StageKind::Given(flows), _) => {
                    let flow = flows[(j - 1) as usize];
                    let source = if s.built_up {
                        format!("built up from line items (step \"Build-up: FCFF, period {k}\")")
                    } else {
                        "as given".to_owned()
                    };
                    ex.step(format!("Period {k}: flow"), source, flow);
                    flow
                }
                (_, Some(g)) => {
                    let prev = previous.ok_or(FairmarkError::Internal(
                        "generated stage without a previous flow",
                    ))?;
                    if prev.is_sign_negative() && !prev.is_zero() {
                        ex.convention(format!(
                            "warning: period {k}: growth {g} applied to a negative flow ({prev}); \
                             check the sign of the projection"
                        ));
                    }
                    let flow = mul(prev, one_plus(g)?)?;
                    ex.step(
                        format!("Period {k}: flow"),
                        format!("{prev} × (1 + {g})"),
                        flow,
                    );
                    flow
                }
                (_, None) => return Err(FairmarkError::Internal("generated flow without growth")),
            };
            out.push(Projected {
                stage,
                projection: s.projection,
                growth,
                flow,
            });
            previous = Some(flow);
        }
        first = last + 1;
    }
    Ok(out)
}

/// R7: the flows of the `given` stages with `lines`, built up as in `dcf.free_cash_flows`
/// (FCFF on the enterprise basis, FCFE on the equity basis), with every step in the explain and
/// the conventions of the build-up reported once. The tax fields are refused when no stage has
/// lines.
fn build_up_stages(
    stages: &mut [ResolvedStage<'_>],
    basis: TerminalBasis,
    taxes: &TaxChoice,
    ex: &mut Explain,
) -> Result<()> {
    let mut first = 1u32;
    let mut any = false;
    let mut conventions: Vec<String> = Vec::new();
    for (i, s) in stages.iter_mut().enumerate() {
        if let StageKind::Lines(lines) = s.kind {
            any = true;
            let pointer = format!("/stages/{i}/lines");
            if basis == TerminalBasis::Enterprise {
                if let Some(k) = lines
                    .iter()
                    .position(|l| l.interest_expense.is_some() || l.net_borrowing.is_some())
                {
                    return Err(invalid(format!(
                        "`{pointer}/{k}/interest_expense` and `{pointer}/{k}/net_borrowing` are \
                         not used on the enterprise basis: the DCF discounts FCFF, which is \
                         before financing; remove them"
                    )));
                }
            }
            let at = Location {
                pointer_prefix: &pointer,
                first_period: first,
            };
            let mut own = Explain::new(FreeCashFlows::ID, FORMULA_FCFF);
            let out = build_up(lines, taxes, at, &mut own)?;
            for step in own.steps {
                ex.step(
                    format!("Build-up: {}", step.label),
                    step.formula,
                    step.value,
                );
            }
            for c in own.conventions {
                if !conventions.contains(&c) {
                    conventions.push(c);
                }
            }
            let flows = match basis {
                TerminalBasis::Enterprise => out.free_cash_flows_to_firm,
                TerminalBasis::Equity => out.free_cash_flows_to_equity.ok_or_else(|| {
                    invalid(format!(
                        "`{pointer}` has no `interest_expense` and `net_borrowing`: the equity \
                         basis discounts FCFE, which needs them in every period"
                    ))
                })?,
            };
            s.kind = StageKind::Given(Cow::Owned(flows));
            s.built_up = true;
        }
        first += s.periods;
    }
    if any {
        let source = format!("{} v{}", FreeCashFlows::ID, FreeCashFlows::VERSION);
        ex.convention(format!(
            "free cash flow build-up ({source}) for the stages with lines: {FORMULA_FCFF}"
        ));
        for c in conventions {
            ex.convention(format!("free cash flow build-up ({source}): {c}"));
        }
        return Ok(());
    }
    for (field, given) in [
        ("notional_tax_rate", taxes.notional_tax_rate.is_some()),
        ("negative_ebit", taxes.negative_ebit.is_some()),
        ("interest_tax_rate", taxes.interest_tax_rate.is_some()),
    ] {
        if given {
            return Err(invalid(format!(
                "`/{field}` is used only by a `given` stage with `lines`; remove it, or give the \
                 line items of a stage in `lines`"
            )));
        }
    }
    Ok(())
}

fn period_timing(t: Timing) -> PeriodTiming {
    match t {
        Timing::EndOfPeriod => PeriodTiming::EndOfPeriod,
        Timing::MidPeriod => PeriodTiming::MidPeriod,
    }
}

/// Runs the engine: validates every choice, projects the flows, discounts them, computes and
/// discounts the terminal value. The caller adds its basis convention and its bridge.
pub(crate) fn run(input: &EngineInput<'_>, ex: &mut Explain) -> Result<EngineOutput> {
    let (mut stages, n) = check_stages(input.stages, input.base_flow)?;
    build_up_stages(&mut stages, input.basis, &input.taxes, ex)?;
    let (rates, terminal_rate, terminal_field) = resolve_rates(&input.rates, n)?;
    check_terms(ex, input.cash_flow_terms, input.discount_rate_terms)?;
    check_timing(input.timing, input.terminal_discount_time)?;
    let gordon_terms = check_terminal(input.terminal, terminal_rate, &terminal_field)?;

    ex.convention(format!(
        "explicit horizon N = {n} periods; stages: {}; one period is one year, rates are \
         annual effective",
        input.stages.len()
    ));
    ex.convention(match input.rates.single {
        Some(r) => {
            format!("discount rate: one rate, {r}, for every period and for the terminal value")
        }
        None => format!(
            "discount rate: one rate per period; terminal rate {terminal_rate} given separately"
        ),
    });
    ex.convention(format!(
        "timing: {} (flow k discounted at t = {})",
        input.timing.label(),
        match input.timing {
            Timing::EndOfPeriod => "k",
            Timing::MidPeriod => "k − 0.5, half a period at the rate r_k",
        }
    ));

    // Flows, then discount factors through the tvm building block (SPEC-0002 R5).
    let projected = project(ex, &stages, input.base_flow)?;
    let dfs = period_discount_factors(&rates, period_timing(input.timing))?;
    let ends = period_discount_factors(&rates, PeriodTiming::EndOfPeriod)?;

    // Present values divide each amount by its compounding factor G_k = Π_(i ≤ k) (1 + r_i)
    // (mid period: G_(k−1) × (1 + r_k)^0.5) rather than multiplying by the discount factor:
    // at long horizons the factor is small and would carry few significant digits, while the
    // compounding factor keeps all of them. The factors of `tvm` are reported unchanged.
    let mut growth_factors = Vec::with_capacity(n + 1); // G_0 .. G_N; None beyond the range
    growth_factors.push(Some(Decimal::ONE));
    let mut fallback_from: Option<u32> = None;
    let mut periods = Vec::with_capacity(n);
    let mut pv_explicit = Decimal::ZERO;
    for (k, (p, (rate, df))) in projected.iter().zip(rates.iter().zip(&dfs)).enumerate() {
        let period = k as u32 + 1;
        let start = if k == 0 { Decimal::ONE } else { ends[k - 1] };
        let formula = match input.timing {
            Timing::EndOfPeriod => format!("{start} / (1 + {rate})"),
            Timing::MidPeriod => format!("{start} / (1 + {rate})^0.5"),
        };
        ex.step(format!("Period {period}: discount rate"), "as given", *rate);
        ex.step(format!("Period {period}: discount factor"), formula, *df);
        let base = one_plus(*rate)?;
        let previous = growth_factors[k];
        let current = previous.and_then(|g| g.checked_mul(base));
        growth_factors.push(current);
        let denominator = match input.timing {
            Timing::EndOfPeriod => current.map(|g| (g, format!("{g}"))),
            Timing::MidPeriod => match previous {
                Some(g) => {
                    let root = base.sqrt().ok_or(FairmarkError::Overflow("square root"))?;
                    g.checked_mul(root)
                        .map(|d| (d, format!("({g} × (1 + {rate})^0.5)")))
                }
                None => None,
            },
        };
        let pv = match denominator {
            Some((d, text)) => {
                if let Some(g) = current {
                    ex.step(
                        format!("Period {period}: compounding factor"),
                        format!("{} × (1 + {rate})", previous.unwrap_or(Decimal::ONE)),
                        g,
                    );
                }
                let pv = div(p.flow, d)?;
                ex.step(
                    format!("Period {period}: present value"),
                    format!("{} / {text}", p.flow),
                    pv,
                );
                pv
            }
            None => {
                fallback_from.get_or_insert(period);
                let pv = mul(p.flow, *df)?;
                ex.step(
                    format!("Period {period}: present value"),
                    format!("{} × {df}", p.flow),
                    pv,
                );
                pv
            }
        };
        pv_explicit = add(pv_explicit, pv)?;
        periods.push(PeriodRow {
            period,
            stage: p.stage,
            projection: p.projection,
            growth: p.growth,
            flow: p.flow,
            discount_rate: *rate,
            rate_used: *rate,
            discount_factor: *df,
            present_value: pv,
        });
    }
    ex.convention(match fallback_from {
        None => "present values: each flow divided by its compounding factor Π (1 + r_i), \
                 which keeps every significant digit at long horizons"
            .to_owned(),
        Some(k) => format!(
            "present values: each flow divided by its compounding factor Π (1 + r_i); from \
             period {k} the factor exceeds the decimal range and the flow is multiplied by its \
             discount factor instead"
        ),
    });
    ex.step(
        "Present value of the explicit flows",
        "Σ present values",
        pv_explicit,
    );

    let last = &projected[n - 1];
    let g_n = growth_factors[n];
    let terminal = terminal_value(
        ex,
        input,
        gordon_terms,
        terminal_rate,
        last,
        &rates,
        &ends,
        g_n,
    )?;
    Ok(EngineOutput {
        periods,
        pv_explicit,
        terminal,
    })
}

/// R19 to R22: the terminal value through the typed `tv` function, and its discounting.
#[allow(clippy::too_many_arguments)]
fn terminal_value(
    ex: &mut Explain,
    input: &EngineInput<'_>,
    terms: &GordonTerminal,
    rate: Decimal,
    last: &Projected,
    rates: &[Decimal],
    ends: &[Decimal],
    g_n: Option<Decimal>,
) -> Result<TerminalOutput> {
    let n = ends.len();
    let g = terms.growth;
    let (next_flow, last_flow) = match terms.first_flow {
        FirstTerminalFlow::LastFlowGrown => (None, Some(last.flow)),
        FirstTerminalFlow::NormalisedNextFlow => (terms.normalised_next_flow, None),
    };
    let (out, tv_ex) = tv::gordon(&GordonInput {
        rate,
        growth: g,
        next_flow,
        last_flow,
        growth_cap: terms.growth_cap,
    })?;
    let (id, version) = (tv::Gordon::ID, tv::Gordon::VERSION);
    ex.convention(format!(
        "terminal technique: {id} version {version} on the {} basis, at the terminal rate \
         {rate}, in the terms of the cash flows",
        input.basis.name()
    ));
    ex.convention(match terms.first_flow {
        FirstTerminalFlow::LastFlowGrown => {
            "first terminal flow: the last explicit flow grown by the perpetual growth, \
             CF_N × (1 + g)"
                .to_owned()
        }
        FirstTerminalFlow::NormalisedNextFlow => {
            "first terminal flow: normalised flow at N + 1, given in the terms of the cash \
             flows"
                .to_owned()
        }
    });
    for s in tv_ex.steps {
        ex.step(format!("Terminal value: {}", s.label), s.formula, s.value);
    }
    for c in tv_ex.conventions {
        ex.convention(format!("terminal value ({id} v{version}): {c}"));
    }
    if let Some(last_growth) = last.growth {
        if last_growth != g {
            ex.convention(format!(
                "warning: the growth of the last explicit period ({last_growth}) differs from \
                 the perpetual growth of the terminal value ({g})"
            ));
        }
    }

    let df_n = ends[n - 1];
    let horizon = Decimal::from(n as u64);
    let tv_n = out.terminal_value;
    let at_n = format!("Terminal value: discount factor at t = N = {n}, end of period {n}");
    let df_n_formula = match g_n {
        Some(g) => format!("DF_{n} = 1 / Π_(i ≤ {n}) (1 + r_i) = 1 / {g}"),
        None => format!("DF_{n}, the discount factor at the end of period {n}"),
    };
    let (time, df, pv) = match (input.timing, input.terminal_discount_time) {
        (Timing::MidPeriod, Some(TerminalDiscountTime::MidPeriod)) => {
            let r_n = rates[n - 1];
            let root = one_plus(r_n)?
                .sqrt()
                .ok_or(FairmarkError::Overflow("square root"))?;
            let df = mul(df_n, root)?;
            ex.convention(format!(
                "terminal discounting time: mid_period, t = N − 0.5, factor DF_N × (1 + r_N)^0.5; \
                 the half-period shift uses the rate of period N, r_{n} = {r_n} (R21), not the \
                 terminal rate"
            ));
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
            if timing == Timing::MidPeriod {
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
        rate,
        terminal_value: out.terminal_value,
        discount_time: time,
        discount_factor: df,
        present_value: pv,
        output: TechniqueOutput::Gordon(out),
    })
}
