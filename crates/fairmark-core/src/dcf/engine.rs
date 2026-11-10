//! The shared DCF engine (SPEC-0001): explicit stages, discount rates, terms, timing and the
//! terminal value, independent of the basis.
//!
//! Each DCF method (`dcf.fcff`, `dcf.fcfe`) maps its own fields to [`EngineInput`],
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

use crate::error::{FairmarkError, Found, Reason, Result};
use crate::explain::{DefaultApplied, Explain};
use crate::method::Method;
use crate::num::{add, div, mul, one_plus, sub};
use crate::rates::{real_nominal, FisherDirection, RealNominal, RealNominalInput};
use crate::tv::TerminalBasis;
use crate::tvm::{period_discount_factors, PeriodTiming};
use crate::types::Timing;

use super::free_cash_flows::{
    build_up, check_lines, FreeCashFlowLines, FreeCashFlows, Location, TaxChoice, FORMULA_FCFE,
    FORMULA_FCFF,
};
use super::terminal::{
    self, Terminal, TerminalChoice, TerminalContext, TerminalOutput, TerminalRateUse,
};
use std::borrow::Cow;

/// Explicit stages accepted (Decision 2). Checked at run time, not in the schema, so that more
/// stages remain an additive change (R29).
pub(crate) const MAX_STAGES: usize = 3;
/// Longest explicit horizon, in periods: a guard against inputs that would allocate without
/// bound, far beyond any business plan.
pub(crate) const MAX_PERIODS: u64 = 10_000;

// ---------------------------------------------------------------- input types

/// Reads a field with a documented default (Decision 24) and refuses an explicit `null`: the
/// default is asked for by leaving the field out, so that `null` is never a silent default.
/// Used with `#[serde(default)]`, so a missing field never reaches it.
pub(crate) fn not_null<'de, D, T>(d: D) -> std::result::Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    match Option::<T>::deserialize(d)? {
        Some(v) => Ok(Some(v)),
        None => Err(serde::de::Error::custom(
            crate::error::CustomRefusal::NullNotAccepted.text(),
        )),
    }
}

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
    /// Number of periods of the stage, at least 1; one period is one year. Default: for a
    /// `given` stage, the number of its `flows` or `lines`; required for `constant_growth` and
    /// `linear_decline`.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    #[schemars(with = "u32", range(min = 1))]
    pub periods: Option<u32>,
    /// `given`, `constant_growth` or `linear_decline`. Default: `given` when the stage has
    /// `flows` or `lines`; required otherwise.
    #[serde(
        default,
        skip_serializing_if = "Option::is_none",
        deserialize_with = "not_null"
    )]
    #[schemars(with = "Projection")]
    pub projection: Option<Projection>,
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

    fn other(self) -> Self {
        match self {
            Self::Nominal => Self::Real,
            Self::Real => Self::Nominal,
        }
    }
}

/// The conversion of the discount rates to the terms of the cash flows (R16, Decision 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RateConversion {
    /// The terms are equal: the rates are used as given.
    None,
    /// Nominal rates converted to real terms for real cash flows.
    ToReal,
    /// Real rates converted to nominal terms for nominal cash flows.
    ToNominal,
}

/// When the terminal value is discounted under mid-period timing (Decision 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerminalDiscountTime {
    /// At the horizon, t = N: factor DF_N.
    Horizon,
    /// Half a period earlier, t = N − 0.5: factor DF_N × (1 + r_T)^0.5, with r_T the terminal
    /// rate.
    MidPeriod,
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
    /// Discount rate used, in the terms of the cash flows, decimal fraction: the rate given,
    /// or the rate converted with the expected inflation of the period when the terms differ.
    pub rate_used: Decimal,
    /// Expected inflation of the period used to convert the discount rate, decimal fraction;
    /// `null` when no conversion is made.
    pub inflation: Option<Decimal>,
    /// Discount factor of the flow under the timing chosen.
    pub discount_factor: Decimal,
    /// Present value of the flow, in currency units.
    pub present_value: Decimal,
}

/// The terms of the flows and of the rates, and what was done with expected inflation
/// (R14 to R18).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TermsOutput {
    /// Terms of every amount and growth rate of the input, as given.
    pub cash_flow_terms: Terms,
    /// Terms of the discount rates, as given.
    pub discount_rate_terms: Terms,
    /// `none`, `to_real` or `to_nominal`: the conversion applied to every discount rate.
    pub conversion: RateConversion,
    /// Terminal rate as given (the single rate, or the terminal rate), decimal fraction; `null`
    /// when the terminal value uses no rate (no terminal value, or a given terminal value
    /// discounted at N).
    pub terminal_rate_given: Option<Decimal>,
    /// Terminal rate used, in the terms of the cash flows, decimal fraction; `null` when the
    /// terminal value uses no rate.
    pub terminal_rate_used: Option<Decimal>,
    /// Expected inflation of the terminal phase, decimal fraction; `null` without inflation or
    /// when the terminal value uses no rate.
    pub terminal_inflation: Option<Decimal>,
    /// Perpetual growth stated by the terminal technique, in the terms of the cash flows,
    /// decimal fraction; `null` when the technique states none.
    pub perpetual_growth: Option<Decimal>,
    /// The perpetual growth restated in the other terms with the terminal inflation (R17),
    /// decimal fraction; `null` without inflation or without a perpetual growth.
    pub perpetual_growth_other_terms: Option<Decimal>,
}

// ---------------------------------------------------------------- method inputs

/// Declares the input type of a DCF method and its mapping to [`EngineInput`], so that the
/// fields every basis shares, their types, serde and schema attributes, and their mapping to
/// the engine are written once. `serde(flatten)` cannot be used: it does not combine with
/// `deny_unknown_fields`.
///
/// The caller gives what differs by basis: the type name and its doc, the basis, the doc of
/// each field whose meaning names the flow (FCFF or FCFE), the three rate fields (names and
/// docs; the names become the JSON names reported in errors, R27), `interest_tax_rate` after
/// the marker `equity_only` (the enterprise basis has no financing lines) and its own bridge
/// fields, appended as written.
/// The field order is fixed here, so the serde messages and the JSON Schema of every method
/// keep their order. A shared input field (for example an optional field whose documented
/// default the engine resolves) is added here, once, for every method.
macro_rules! dcf_input {
    (
        $(#[$meta:meta])*
        pub struct $name:ident {
            basis: $basis:expr,
            $(#[$stages_doc:meta])* stages,
            $(#[$base_flow_doc:meta])* base_flow,
            $(#[$notional_doc:meta])* notional_tax_rate,
            $(#[$negative_doc:meta])* negative_ebit,
            $(equity_only $(#[$interest_doc:meta])* $interest:ident,)?
            rates {
                $(#[$single_doc:meta])* $single:ident,
                $(#[$per_period_doc:meta])* $per_period:ident,
                $(#[$terminal_rate_doc:meta])* $terminal_rate:ident,
            },
            $(#[$terminal_doc:meta])* terminal,
            bridge { $($bridge:tt)* }
        }
    ) => {
        $(#[$meta])*
        #[derive(Debug, Clone, ::serde::Serialize, ::serde::Deserialize, ::schemars::JsonSchema)]
        #[serde(deny_unknown_fields)]
        pub struct $name {
            $(#[$stages_doc])*
            #[schemars(length(min = 1))]
            pub stages: Vec<$crate::dcf::engine::Stage>,
            $(#[$base_flow_doc])*
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub base_flow: Option<::rust_decimal::Decimal>,
            $(#[$notional_doc])*
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub notional_tax_rate: Option<::rust_decimal::Decimal>,
            $(#[$negative_doc])*
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub negative_ebit: Option<$crate::dcf::free_cash_flows::NegativeEbit>,
            $(
                $(#[$interest_doc])*
                #[serde(default, skip_serializing_if = "Option::is_none")]
                pub $interest: Option<::rust_decimal::Decimal>,
            )?
            /// `nominal` or `real`: the terms of every amount and growth rate of the input.
            /// Default: `nominal`, reported.
            #[serde(
                default,
                skip_serializing_if = "Option::is_none",
                deserialize_with = "crate::dcf::engine::not_null"
            )]
            #[schemars(with = "crate::dcf::engine::Terms")]
            pub cash_flow_terms: Option<$crate::dcf::engine::Terms>,
            /// `nominal` or `real`: the terms of the discount rates. When they differ from
            /// `cash_flow_terms`, expected inflation is required (`inflation`, or
            /// `inflation_per_period` with `terminal_inflation`) and every rate is converted to the
            /// terms of the cash flows; the cash flows are never converted. Default: `nominal`,
            /// reported.
            #[serde(
                default,
                skip_serializing_if = "Option::is_none",
                deserialize_with = "crate::dcf::engine::not_null"
            )]
            #[schemars(with = "crate::dcf::engine::Terms")]
            pub discount_rate_terms: Option<$crate::dcf::engine::Terms>,
            /// Expected inflation, decimal fraction per period (0.02 = 2%), greater than -1, for every
            /// explicit period and for the terminal phase. Give this or `inflation_per_period`, or
            /// neither. With different terms it converts the discount rates by the exact Fisher
            /// relation; with equal terms it only restates the perpetual growth in the other terms.
            /// Default: none.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub inflation: Option<::rust_decimal::Decimal>,
            /// Expected inflation per period of the explicit horizon, decimal fractions, exactly N of
            /// them, each greater than -1; requires `terminal_inflation`. Give this or `inflation`,
            /// not both. Default: none.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub inflation_per_period: Option<Vec<::rust_decimal::Decimal>>,
            /// Expected inflation of the terminal phase, decimal fraction per period, greater than -1;
            /// required with `inflation_per_period` when the terminal value uses a terminal rate,
            /// refused otherwise.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub terminal_inflation: Option<::rust_decimal::Decimal>,
            $(#[$single_doc])*
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub $single: Option<::rust_decimal::Decimal>,
            $(#[$per_period_doc])*
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub $per_period: Option<Vec<::rust_decimal::Decimal>>,
            $(#[$terminal_rate_doc])*
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub $terminal_rate: Option<::rust_decimal::Decimal>,
            /// `end_of_period` (flow k at t = k) or `mid_period` (t = k − 0.5). Default:
            /// `end_of_period`, reported.
            #[serde(
                default,
                skip_serializing_if = "Option::is_none",
                deserialize_with = "crate::dcf::engine::not_null"
            )]
            #[schemars(with = "crate::types::Timing")]
            pub timing: Option<$crate::types::Timing>,
            /// With `mid_period` timing and a terminal value to discount, required: `horizon`
            /// (terminal value at t = N) or `mid_period` (t = N − 0.5). Refused with
            /// `end_of_period` and with no terminal value.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub terminal_discount_time: Option<$crate::dcf::engine::TerminalDiscountTime>,
            $(#[$terminal_doc])*
            #[serde(
                default,
                skip_serializing_if = "Option::is_none",
                deserialize_with = "crate::dcf::engine::not_null"
            )]
            #[schemars(with = "crate::dcf::terminal::Terminal")]
            pub terminal: Option<$crate::dcf::terminal::Terminal>,
            $($bridge)*
        }

        impl $name {
            /// Fills in the documented defaults of the fields not given (SPEC-0001 Decision 24,
            /// R31) and returns them in input order; a field given is never changed.
            pub fn apply_defaults(&mut self) -> Vec<$crate::explain::DefaultApplied> {
                $crate::dcf::engine::apply_defaults($crate::dcf::engine::Defaultable {
                    stages: &mut self.stages,
                    cash_flow_terms: &mut self.cash_flow_terms,
                    discount_rate_terms: &mut self.discount_rate_terms,
                    timing: &mut self.timing,
                    terminal: &mut self.terminal,
                })
            }

            /// The engine's view of an input whose defaults were filled in: every shared field,
            /// the basis, the JSON names of the rate fields and which terms were defaulted.
            pub(crate) fn engine_input(
                &self,
                defaults: &[$crate::explain::DefaultApplied],
            ) -> $crate::error::Result<$crate::dcf::engine::EngineInput<'_>> {
                // Only the equity basis has the field; elsewhere the rate is absent.
                let interest_tax_rate: Option<::rust_decimal::Decimal> =
                    None $(.or(self.$interest))?;
                let missing = || $crate::error::FairmarkError::Internal("DCF defaults not applied");
                Ok($crate::dcf::engine::EngineInput {
                    basis: $basis,
                    stages: &self.stages,
                    base_flow: self.base_flow,
                    cash_flow_terms: self.cash_flow_terms.ok_or_else(missing)?,
                    discount_rate_terms: self.discount_rate_terms.ok_or_else(missing)?,
                    defaulted: $crate::dcf::engine::Defaulted::of(defaults),
                    inflation: $crate::dcf::engine::InflationInput {
                        single: self.inflation,
                        per_period: self.inflation_per_period.as_deref(),
                        terminal: self.terminal_inflation,
                    },
                    rates: $crate::dcf::engine::RateInput {
                        single: self.$single,
                        per_period: self.$per_period.as_deref(),
                        terminal: self.$terminal_rate,
                        fields: $crate::dcf::engine::RateFields {
                            single: stringify!($single),
                            per_period: stringify!($per_period),
                            terminal: stringify!($terminal_rate),
                        },
                    },
                    timing: self.timing.ok_or_else(missing)?,
                    terminal_discount_time: self.terminal_discount_time,
                    terminal: self.terminal.as_ref().ok_or_else(missing)?,
                    taxes: $crate::dcf::free_cash_flows::TaxChoice {
                        notional_tax_rate: self.notional_tax_rate,
                        negative_ebit: self.negative_ebit,
                        interest_tax_rate,
                    },
                })
            }
        }
    };
}
pub(crate) use dcf_input;

// ---------------------------------------------------------------- documented defaults

/// A top-level field of a DCF input with a documented default (Decision 24); its JSON pointer
/// is the `field` of the entry in `meta.defaults_applied`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum DefaultedField {
    CashFlowTerms,
    DiscountRateTerms,
    Timing,
    Terminal,
}

impl DefaultedField {
    pub(crate) const fn pointer(self) -> &'static str {
        match self {
            Self::CashFlowTerms => "/cash_flow_terms",
            Self::DiscountRateTerms => "/discount_rate_terms",
            Self::Timing => "/timing",
            Self::Terminal => "/terminal",
        }
    }
}

/// Which choices took their default, for the messages that say so (Decision 24).
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Defaulted {
    pub cash_flow_terms: bool,
    pub discount_rate_terms: bool,
    pub timing: bool,
    pub terminal: bool,
    /// The projection of the first stage (`given`, from its flows or lines).
    pub first_projection: bool,
}

impl Defaulted {
    pub(crate) fn of(defaults: &[DefaultApplied]) -> Self {
        let has = |f: DefaultedField| defaults.iter().any(|d| d.field == f.pointer());
        Self {
            cash_flow_terms: has(DefaultedField::CashFlowTerms),
            discount_rate_terms: has(DefaultedField::DiscountRateTerms),
            timing: has(DefaultedField::Timing),
            terminal: has(DefaultedField::Terminal),
            first_projection: defaults.iter().any(|d| d.field == "/stages/0/projection"),
        }
    }
}

/// The fields of a DCF input that have a documented default (SPEC-0001 Decision 24).
pub struct Defaultable<'a> {
    pub stages: &'a mut [Stage],
    pub cash_flow_terms: &'a mut Option<Terms>,
    pub discount_rate_terms: &'a mut Option<Terms>,
    pub timing: &'a mut Option<Timing>,
    pub terminal: &'a mut Option<Terminal>,
}

const NOMINAL_REASON: &str = "business plans and discount rates (WACC, cost of equity) are \
    commonly stated in nominal terms, at current prices";

/// SPEC-0001 Decision 24, R31: fills in, once, every documented default of the DCF input and
/// returns one entry per field filled, in input order. A field given is never changed. A
/// stage whose default cannot be decided (no `flows` or `lines`, both, or none of them
/// non-empty) is left as it is, and the checks of the engine name the field to give.
pub fn apply_defaults(d: Defaultable<'_>) -> Vec<DefaultApplied> {
    let mut out = Vec::new();
    for (i, s) in d.stages.iter_mut().enumerate() {
        let amounts = match (&s.flows, &s.lines) {
            (Some(f), None) => Some((f.len(), "flows")),
            (None, Some(l)) => Some((l.len(), "lines")),
            _ => None,
        };
        if s.projection.is_none() && amounts.is_some() {
            s.projection = Some(Projection::Given);
            out.push(DefaultApplied::new(
                format!("/stages/{i}/projection"),
                serde_json::json!("given"),
                "a stage with flows or lines is a stage of given flows",
            ));
        }
        if let (None, Some(Projection::Given), Some((n, what))) = (s.periods, s.projection, amounts)
        {
            if let Ok(n) = u32::try_from(n) {
                if n > 0 {
                    s.periods = Some(n);
                    out.push(DefaultApplied::new(
                        format!("/stages/{i}/periods"),
                        serde_json::json!(n),
                        if what == "flows" {
                            "the number of flows given in the stage"
                        } else {
                            "the number of lines given in the stage"
                        },
                    ));
                }
            }
        }
    }
    for (field, terms) in [
        (DefaultedField::CashFlowTerms, d.cash_flow_terms),
        (DefaultedField::DiscountRateTerms, d.discount_rate_terms),
    ] {
        if terms.is_none() {
            *terms = Some(Terms::Nominal);
            out.push(DefaultApplied::new(
                field.pointer(),
                serde_json::json!("nominal"),
                NOMINAL_REASON,
            ));
        }
    }
    if d.timing.is_none() {
        *d.timing = Some(Timing::EndOfPeriod);
        out.push(DefaultApplied::new(
            DefaultedField::Timing.pointer(),
            serde_json::json!("end_of_period"),
            "each flow at the end of its period, the convention of tvm.npv and of most published \
             examples",
        ));
    }
    if d.terminal.is_none() {
        *d.terminal = Some(Terminal::none());
        out.push(DefaultApplied::new(
            DefaultedField::Terminal.pointer(),
            serde_json::json!({"none": {}}),
            "no terminal value: the value covers the given flows only (a holding period equal \
             to the explicit horizon); give `/terminal` to add one",
        ));
    }
    out
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

/// Expected inflation as given (R16): one value, or one per period plus a terminal value; all
/// `None` when not given. The JSON names are the same for every DCF method.
#[derive(Default)]
pub(crate) struct InflationInput<'a> {
    pub single: Option<Decimal>,
    pub per_period: Option<&'a [Decimal]>,
    pub terminal: Option<Decimal>,
}

pub(crate) struct EngineInput<'a> {
    pub basis: TerminalBasis,
    pub stages: &'a [Stage],
    pub base_flow: Option<Decimal>,
    pub cash_flow_terms: Terms,
    pub discount_rate_terms: Terms,
    /// Which choices took their default (Decision 24), for the messages of `FM-E024` and of
    /// the terminal discounting time.
    pub defaulted: Defaulted,
    pub inflation: InflationInput<'a>,
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
    pub terms: TermsOutput,
}

impl EngineOutput {
    /// The output fields every basis shares, with the terminal value weight of R22.
    pub(crate) fn discounted(self, terminal_value_weight: Option<Decimal>) -> DiscountedValue {
        DiscountedValue {
            pv_explicit_period: self.pv_explicit,
            terminal_value_weight,
            periods: self.periods,
            terms: self.terms,
            terminal: self.terminal,
        }
    }
}

/// The part of the result that every DCF method shares, flattened into its output (the JSON
/// keys are those of the method's own output, in this order).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DiscountedValue {
    /// Present value of the explicit flows (FCFF on the enterprise basis, FCFE on the equity
    /// basis), in currency units.
    pub pv_explicit_period: Decimal,
    /// Present value of the terminal value divided by the present value of the explicit flows
    /// plus the present value of the terminal value (the value of operations on the enterprise
    /// basis), decimal fraction (R22); `null` when that sum is zero.
    pub terminal_value_weight: Option<Decimal>,
    /// One row per period of the explicit horizon.
    pub periods: Vec<PeriodRow>,
    /// The terms of the flows and of the rates, the conversion of the rates and the perpetual
    /// growth in the other terms (R14 to R18).
    pub terms: TermsOutput,
    /// The terminal value, its technique and its discounting.
    pub terminal: TerminalOutput,
}

/// R22: the terminal value weight, the present value of the terminal value divided by
/// `denominator` (the present value of the explicit flows plus that of the terminal value),
/// with its step; `None` and a convention stating `zero_reason` when the denominator is zero.
pub(crate) fn terminal_weight(
    ex: &mut Explain,
    pv_terminal: Decimal,
    denominator: Decimal,
    zero_reason: &str,
) -> Result<Option<Decimal>> {
    if denominator.is_zero() {
        ex.convention(format!("terminal value weight not defined: {zero_reason}"));
        return Ok(None);
    }
    let w = div(pv_terminal, denominator)?;
    ex.step(
        "Terminal value weight",
        format!("{pv_terminal} / {denominator}"),
        w,
    );
    Ok(Some(w))
}

fn invalid(msg: String) -> FairmarkError {
    FairmarkError::InvalidInput(msg)
}

/// A count that must match the stages: it reads the stages and their periods.
fn count(msg: String) -> FairmarkError {
    FairmarkError::CountMismatch(msg).reads(["/stages"])
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
    /// The flow built up from `lines` (`FCFF` or `FCFE`), when it was.
    built_up: Option<&'static str>,
}

fn check_stages(
    stages: &[Stage],
    base_flow: Option<Decimal>,
    projection_defaulted: bool,
) -> Result<(Vec<ResolvedStage<'_>>, usize)> {
    if stages.is_empty() || stages.len() > MAX_STAGES {
        return Err(invalid(format!(
            "`/stages` has {} stages; give one to {MAX_STAGES} explicit stages, each followed \
             by the next and the last by the terminal value",
            stages.len()
        ))
        .at("/stages", Reason::Length)
        .expected([format!("1 to {MAX_STAGES} stages")]));
    }
    let mut horizon: u64 = 0;
    let mut resolved = Vec::with_capacity(stages.len());
    let mut found = Found::new();
    for (i, s) in stages.iter().enumerate() {
        // Each stage is checked: the first refusal of a stage ends its checks, not the others.
        // A stage's checks follow one another (its projection decides which fields it takes,
        // its periods how many flows): every error of a stage reads the whole stage.
        let stage = check_stage(i, s).map_err(|e| e.reads([format!("/stages/{i}")]));
        if let Some(stage) = found.take(stage) {
            horizon += u64::from(stage.periods);
            resolved.push(stage);
        }
    }
    found.finish()?;
    finish_stages(resolved, horizon, base_flow, projection_defaulted)
}

/// The checks of stage `i` (R7 to R9, Decision 24), in a fixed order.
fn check_stage(i: usize, s: &Stage) -> Result<ResolvedStage<'_>> {
    {
        let at = format!("/stages/{i}");
        if s.periods == Some(0) {
            return Err(invalid(format!(
                "`{at}/periods` is 0; a stage has at least 1 period"
            ))
            .at(format!("{at}/periods"), Reason::Range)
            .expected(["a whole number of periods, 1 or more"]));
        }
        // Decision 24: the projection defaults to `given` only for a stage with flows or lines.
        let Some(projection) = s.projection else {
            if s.flows.is_some() && s.lines.is_some() {
                return Err(invalid(format!(
                    "`{at}/flows` and `{at}/lines` are both given: give `flows` or `lines`, not \
                     both"
                ))
                .at(format!("{at}/flows"), Reason::Conflict)
                .between([format!("{at}/flows"), format!("{at}/lines")])
                .fix("give `flows` or `lines`, not both"));
            }
            return Err(invalid(format!(
                "`{at}/projection` is required: give `given` with `flows` or `lines`, \
                 `constant_growth` with `growth`, or `linear_decline` with `from_growth` and \
                 `to_growth`"
            ))
            .at(format!("{at}/projection"), Reason::Missing)
            .expected(["given", "constant_growth", "linear_decline"]));
        };
        if projection == Projection::Given {
            match (&s.flows, &s.lines) {
                (Some(_), Some(_)) => {
                    return Err(invalid(format!(
                        "`{at}/flows` and `{at}/lines` are both given; a `given` stage takes its \
                         flows as amounts (`flows`) or as line items (`lines`), not both"
                    ))
                    .at(format!("{at}/flows"), Reason::Conflict)
                    .between([format!("{at}/flows"), format!("{at}/lines")])
                    .fix("give `flows` or `lines`, not both"))
                }
                (None, None) => {
                    return Err(invalid(format!(
                        "`{at}/flows` or `{at}/lines` is required by the projection `given`: \
                         give the flows as amounts, or the line items to build them up"
                    ))
                    .at(format!("{at}/flows"), Reason::Missing)
                    .between([format!("{at}/flows"), format!("{at}/lines")]))
                }
                _ => {}
            }
        } else if s.lines.is_some() {
            return Err(invalid(format!(
                "`{at}/lines` is not used by the projection `{}`; remove it (line items belong \
                 to a `given` stage)",
                projection.label()
            ))
            .at(format!("{at}/lines"), Reason::Requires)
            .fix("remove `lines`, or make the stage `given`"));
        }
        // Decision 24: the periods of a `given` stage default to the number of its flows or
        // lines; a generated stage states them.
        let Some(periods) = s.periods else {
            let (field, reason) = match projection {
                Projection::Given if s.lines.is_some() => ("lines", Reason::Length),
                Projection::Given => ("flows", Reason::Length),
                _ => ("periods", Reason::Missing),
            };
            return Err(invalid(match projection {
                Projection::Given if s.lines.is_some() => format!(
                    "`{at}/lines` is empty: a stage has at least 1 period; give the line items \
                     of the stage"
                ),
                Projection::Given => format!(
                    "`{at}/flows` is empty: a stage has at least 1 period; give the flows of the \
                     stage"
                ),
                _ => format!(
                    "`{at}/periods` is required by the projection `{}`: give the number of \
                     periods of the stage",
                    projection.label()
                ),
            })
            .at(format!("{at}/{field}"), reason));
        };
        // For `given`, `flows` stands for `flows` or `lines`, checked above.
        let present = [
            ("flows", s.flows.is_some() || s.lines.is_some()),
            ("growth", s.growth.is_some()),
            ("from_growth", s.from_growth.is_some()),
            ("to_growth", s.to_growth.is_some()),
        ];
        let own: &[&str] = match projection {
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
                    projection.label(),
                    own.join(" and ")
                ))
                .at(format!("{at}/{field}"), Reason::Requires)
                .fix(format!("remove `{field}`")));
            }
            if !given && belongs {
                return Err(invalid(format!(
                    "`{at}/{field}` is required by the projection `{}`",
                    projection.label()
                ))
                .at(format!("{at}/{field}"), Reason::Missing)
                .fix(format!("add `{field}`")));
            }
        }
        if let Some(lines) = &s.lines {
            if lines.len() != periods as usize {
                return Err(count(format!(
                    "stage {} (`{at}/lines`) has {} lines for {} periods; give exactly {} \
                     lines or set `{at}/periods` to {}",
                    i + 1,
                    lines.len(),
                    periods,
                    periods,
                    lines.len()
                ))
                .at(format!("{at}/lines"), Reason::Length)
                .between([format!("{at}/lines"), format!("{at}/periods")]));
            }
        }
        if let Some(flows) = &s.flows {
            if flows.len() != periods as usize {
                return Err(count(format!(
                    "stage {} (`{at}/flows`) has {} flows for {} periods; give exactly {} \
                     flows or set `{at}/periods` to {}",
                    i + 1,
                    flows.len(),
                    periods,
                    periods,
                    flows.len()
                ))
                .at(format!("{at}/flows"), Reason::Length)
                .between([format!("{at}/flows"), format!("{at}/periods")]));
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
                    ))
                    .at(format!("{at}/{field}"), Reason::Range)
                    .expected(["a growth greater than -1 (-100%)"]));
                }
            }
        }
        let kind = match (
            projection,
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
        Ok(ResolvedStage {
            periods,
            projection,
            kind,
            built_up: None,
        })
    }
}

/// R6, R29: the explicit horizon and the base flow, once every stage is valid.
fn finish_stages(
    resolved: Vec<ResolvedStage<'_>>,
    horizon: u64,
    base_flow: Option<Decimal>,
    projection_defaulted: bool,
) -> Result<(Vec<ResolvedStage<'_>>, usize)> {
    if horizon > MAX_PERIODS {
        return Err(invalid(format!(
            "`/stages` add up to {horizon} periods; the explicit horizon is at most \
             {MAX_PERIODS} periods"
        ))
        .at("/stages", Reason::Range)
        .expected([format!("at most {MAX_PERIODS} periods in all")]));
    }
    let generated = resolved[0].projection != Projection::Given;
    match (generated, base_flow) {
        (true, None) => Err(count(format!(
            "`/base_flow` is missing: the first stage (`/stages/0`) is `{}` and grows from the \
             base flow CF_0; give `/base_flow`",
            resolved[0].projection.label()
        ))
        .at("/base_flow", Reason::Missing)
        .between(["/base_flow", "/stages/0/projection"])
        .fix("give `/base_flow`")),
        (false, Some(_)) => Err(count(
            "`/base_flow` is given but the first stage (`/stages/0`) is `given`; remove \
             `/base_flow`, or make the first stage a generated one"
                .to_owned(),
        )
        .at("/base_flow", Reason::Requires)
        // A projection left to its default (`given`, from the flows) is read, never named.
        .between(
            ["/base_flow", "/stages/0/projection"]
                .into_iter()
                .filter(|p| !(projection_defaulted && *p == "/stages/0/projection")),
        )
        .reads(["/stages/0/projection"])),
        _ => Ok((resolved, horizon as usize)),
    }
}

/// The period rates and the terminal rate with the JSON pointer of its field, if one is used.
type ResolvedRates = (Vec<Decimal>, Option<(Decimal, String)>);

/// R12, R13: one rate for every period and the terminal value, or one per period plus a
/// terminal rate. Returns the period rates and the terminal rate with its field name; no
/// terminal rate when the terminal value uses none (`rate_use` not used, Decision 24),
/// in which case a separate terminal rate is refused.
/// `n` is the explicit horizon, `None` when the stages are refused: the rates are then checked
/// on their own, without the count the horizon decides (ADR 0014 G5). Every error goes to
/// `found`; the rates are returned when their form is valid and every rate is in range, also
/// when a list does not have one rate per period, so that the checks against the rates are made.
fn resolve_rates(
    r: &RateInput<'_>,
    n: Option<usize>,
    rate_use: TerminalRateUse,
    found: &mut Found,
) -> Option<ResolvedRates> {
    let uses_terminal_rate = rate_use.is_used();
    let f = &r.fields;
    let (single, per_period, terminal_at) = (
        format!("/{}", f.single),
        format!("/{}", f.per_period),
        format!("/{}", f.terminal),
    );
    // Whether a terminal rate is used: the terminal option and the timing decide it.
    let decides_use = ["/terminal", "/timing", "/terminal_discount_time"];
    // The form of the rates first, then every rate given against its range: no refusal hides
    // another (ADR 0014 G5, review round 6).
    if !uses_terminal_rate && r.single.is_none() && r.per_period.is_some() && r.terminal.is_some() {
        found.push(
            invalid(format!(
                "`/{}` is not used: the terminal value takes no terminal rate (no terminal value, \
                 or a given terminal value discounted at the horizon N); remove `/{}`",
                f.terminal, f.terminal
            ))
            .at(terminal_at.clone(), Reason::Requires)
            .fix(format!("remove `/{}`", f.terminal))
            .reads([single.clone(), per_period.clone()])
            .reads(decides_use),
        );
    }
    let form = match (r.single, r.per_period, r.terminal) {
        (Some(_), Some(_), _) => Some(
            invalid(format!(
                "give exactly one of `/{}` and `/{}`, not both",
                f.single, f.per_period
            ))
            .at(single.clone(), Reason::Conflict)
            .between([single.clone(), per_period.clone()]),
        ),
        (None, None, _) => Some(
            invalid(format!(
                "give the discount rate: `/{}` for every period, or `/{}` with `/{}`",
                f.single, f.per_period, f.terminal
            ))
            .at(single.clone(), Reason::Missing)
            .between([single.clone(), per_period.clone()])
            .fix(format!("give `/{}`", f.single)),
        ),
        (Some(_), None, Some(_)) => Some(
            invalid(format!(
                "`/{}` is not accepted with `/{}`: one rate is used for every period and for \
                 the terminal value; remove `/{}`, or give `/{}` instead",
                f.terminal, f.single, f.terminal, f.per_period
            ))
            .at(terminal_at.clone(), Reason::Conflict)
            .between([terminal_at.clone(), single.clone()])
            .reads([per_period.clone()]),
        ),
        (None, Some(_), None) if uses_terminal_rate => Some(
            invalid(format!(
                "`/{}` is required with `/{}`: give the rate of the terminal phase{}",
                f.terminal,
                f.per_period,
                rate_use.note()
            ))
            .at(terminal_at.clone(), Reason::Missing)
            .between([terminal_at.clone(), per_period.clone()])
            .reads([single.clone()])
            .reads(decides_use),
        ),
        _ => None,
    };
    let form_valid = form.is_none();
    if let Some(e) = form {
        found.push(e);
    }
    if let (Some(list), Some(n)) = (r.per_period, n) {
        if list.len() != n {
            found.push(
                count(format!(
                    "`/{}` has {} rates for an explicit horizon of {n} periods (the sum of \
                     `/stages/*/periods`); give exactly {n} rates",
                    f.per_period,
                    list.len()
                ))
                .at(per_period.clone(), Reason::Length)
                .expected([format!("{n} rates")]),
            );
        }
    }
    // Every rate given out of range is reported; one rate given is reported once.
    let given = r
        .single
        .map(|rate| (single.clone(), rate))
        .into_iter()
        .chain(
            r.per_period
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(k, rate)| (format!("{per_period}/{k}"), *rate)),
        )
        .chain(r.terminal.map(|rate| (terminal_at.clone(), rate)));
    let mut in_range = true;
    for (field, rate) in given {
        if rate <= -Decimal::ONE {
            in_range = false;
            found.push(FairmarkError::DiscountRateOutOfRange {
                field,
                rate,
                converted_with: None,
            });
        }
    }
    let (rates, terminal, terminal_field) = match (form_valid && in_range, r.single, r.per_period) {
        (true, Some(rate), None) => (vec![rate; n.unwrap_or(1)], Some(rate), single),
        (true, None, Some(list)) => (list.to_vec(), r.terminal, terminal_at),
        _ => return None,
    };
    let terminal = terminal.filter(|_| uses_terminal_rate);
    Some((rates, terminal.map(|t| (t, terminal_field))))
}

/// Expected inflation after validation: the value of each explicit period, the value of the
/// terminal phase and the JSON pointers that name them.
struct Inflation {
    per_period: Vec<Decimal>,
    /// `None` when the terminal value uses no terminal rate (Decision 24).
    terminal: Option<Decimal>,
    /// True for one inflation (`/inflation`), false for the per-period form.
    single: bool,
}

impl Inflation {
    /// Pointer of the inflation of period k (0-based), or of the terminal phase for `None`.
    fn field(&self, k: Option<usize>) -> String {
        match (self.single, k) {
            (true, _) => "/inflation".to_owned(),
            (false, Some(k)) => format!("/inflation_per_period/{k}"),
            (false, None) => "/terminal_inflation".to_owned(),
        }
    }
}

/// R16: one expected inflation for every period and the terminal phase, or one per period with
/// a terminal inflation, or none; each greater than -1 (`FM-E003` otherwise, Decision 19).
/// Checked whenever given (R18 too). A terminal inflation next to one inflation is refused
/// (Decision 18).
///
/// `n` is the explicit horizon, `None` when the stages are refused: the values are then checked
/// on their own, without the count the horizon decides (ADR 0014 G5).
fn resolve_inflation(
    i: &InflationInput<'_>,
    n: Option<usize>,
    rate_use: TerminalRateUse,
    found: &mut Found,
) -> Option<Option<Inflation>> {
    let uses_terminal_rate = rate_use.is_used();
    // Whether a terminal rate is used: the terminal option and the timing decide it.
    let decides_use = ["/terminal", "/timing", "/terminal_discount_time"];
    // The form first, then the count and every value given against its range: no refusal
    // hides another (ADR 0014 G5, review round 6). The values are returned when their form is
    // valid and every value is in range, also when the list does not have one value per period.
    let form = match (i.single, i.per_period, i.terminal) {
        (None, Some(_), Some(_)) if !uses_terminal_rate => Some(
            invalid(
                "`/terminal_inflation` is not used: the terminal value takes no terminal rate \
                 (no terminal value, or a given terminal value discounted at the horizon N); \
                 remove `/terminal_inflation`"
                    .to_owned(),
            )
            .at("/terminal_inflation", Reason::Requires)
            .fix("remove `/terminal_inflation`")
            .reads(["/inflation", "/inflation_per_period"])
            .reads(decides_use),
        ),
        (Some(_), Some(_), _) => Some(
            invalid(
                "give exactly one of `/inflation` and `/inflation_per_period`, not both: one \
                 expected inflation for every period, or one per period with \
                 `/terminal_inflation`"
                    .to_owned(),
            )
            .at("/inflation", Reason::Conflict)
            .between(["/inflation", "/inflation_per_period"]),
        ),
        (Some(_), None, Some(_)) => Some(
            invalid(
                "`/terminal_inflation` is not accepted with `/inflation`: one expected inflation \
                 applies to every period and to the terminal phase; remove \
                 `/terminal_inflation`, or give `/inflation_per_period` instead"
                    .to_owned(),
            )
            .at("/terminal_inflation", Reason::Conflict)
            .between(["/terminal_inflation", "/inflation"])
            .reads(["/inflation_per_period"]),
        ),
        (None, Some(_), None) if uses_terminal_rate => Some(
            invalid(format!(
                "`/terminal_inflation` is required with `/inflation_per_period`: give the \
                 expected inflation of the terminal phase{}",
                rate_use.note()
            ))
            .at("/terminal_inflation", Reason::Missing)
            .between(["/terminal_inflation", "/inflation_per_period"])
            .reads(["/inflation"])
            .reads(decides_use),
        ),
        (None, None, Some(_)) => Some(
            invalid(
                "`/terminal_inflation` is used only with `/inflation_per_period`; give \
                 `/inflation_per_period` too, or give one expected inflation in `/inflation`"
                    .to_owned(),
            )
            .at("/terminal_inflation", Reason::Requires)
            .between(["/terminal_inflation", "/inflation_per_period"])
            .reads(["/inflation"]),
        ),
        _ => None,
    };
    let form_valid = form.is_none();
    if let Some(e) = form {
        found.push(e);
    }
    if let (Some(list), Some(n)) = (i.per_period, n) {
        if list.len() != n {
            found.push(
                count(format!(
                    "`/inflation_per_period` has {} values for an explicit horizon of {n} \
                     periods (the sum of `/stages/*/periods`); give exactly {n} values",
                    list.len()
                ))
                .at("/inflation_per_period", Reason::Length)
                .expected([format!("{n} values")]),
            );
        }
    }
    // Every value given out of range is reported; Decision 19: the code of
    // `rates.real_nominal` for the same value (FM-E003).
    let given = i
        .single
        .map(|v| ("/inflation".to_owned(), v))
        .into_iter()
        .chain(
            i.per_period
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(k, v)| (format!("/inflation_per_period/{k}"), *v)),
        )
        .chain(i.terminal.map(|v| ("/terminal_inflation".to_owned(), v)));
    let mut in_range = true;
    for (field, v) in given {
        if v <= -Decimal::ONE {
            in_range = false;
            found.push(FairmarkError::DiscountRateOutOfRange {
                field,
                rate: v,
                converted_with: None,
            });
        }
    }
    match (form_valid && in_range, i.single, i.per_period) {
        (true, None, None) => Some(None),
        (true, Some(one), None) => Some(Some(Inflation {
            per_period: vec![one; n.unwrap_or(1)],
            terminal: uses_terminal_rate.then_some(one),
            single: true,
        })),
        (true, None, Some(list)) => Some(Some(Inflation {
            per_period: list.to_vec(),
            terminal: i.terminal,
            single: false,
        })),
        _ => None,
    }
}

/// One rate restated in the terms `to` with the exact Fisher relation, through a typed call to
/// `rates.real_nominal` (ADR 0009, R16): the converted rate and its formula with the values
/// substituted.
fn fisher(rate: Decimal, inflation: Decimal, to: Terms) -> Result<(Decimal, String)> {
    let direction = match to {
        Terms::Real => FisherDirection::ToReal,
        Terms::Nominal => FisherDirection::ToNominal,
    };
    let (out, ex) = real_nominal(&RealNominalInput {
        rate,
        inflation,
        direction,
    })?;
    let formula = ex
        .steps
        .into_iter()
        .next()
        .map(|s| s.formula)
        .ok_or(FairmarkError::Internal("rates.real_nominal without a step"))?;
    Ok((out.rate, formula))
}

/// The discount rates in the terms of the cash flows (R14 to R16, R18).
struct RatesUsed {
    conversion: RateConversion,
    /// Rate used in each period, with the inflation and the Fisher formula when converted.
    per_period: Vec<(Decimal, Option<(Decimal, String)>)>,
    /// `None` when the terminal value uses no terminal rate (Decision 24).
    terminal: Option<Decimal>,
    terminal_formula: Option<String>,
    /// JSON pointer of the terminal rate as given; empty without a terminal rate.
    terminal_field: String,
    /// JSON pointer of the inflation the terminal rate was converted with, if it was.
    terminal_converted_with: Option<String>,
}

/// `FM-E024`: cash flows and discount rates in different terms without expected inflation
/// (R14, Decision 24 (c)). It needs no rate and no horizon.
fn terms_mismatch(input: &EngineInput<'_>) -> FairmarkError {
    let (flows, given) = (input.cash_flow_terms, input.discount_rate_terms);
    // Decision 24 (c): say which term came from its default and how to state it.
    let d = input.defaulted;
    let defaulted = if d.cash_flow_terms {
        Some(("cash_flow_terms", flows, given))
    } else if d.discount_rate_terms {
        Some(("discount_rate_terms", given, flows))
    } else {
        None
    };
    let note = match defaulted {
        Some((field, default, other)) => format!(
            "; `/{field}` was not given and defaults to {}: give `{field}: {}`, or give \
             expected inflation",
            default.label(),
            other.label()
        ),
        None => String::new(),
    };
    FairmarkError::TermsMismatch(format!(
        "`/cash_flow_terms` is {} and `/discount_rate_terms` is {}, and no expected \
         inflation is given; discounting {} flows at {} rates misstates the value. Either \
         give expected inflation in `/inflation` (or `/inflation_per_period` with \
         `/terminal_inflation`), so that the discount rates are converted to {} terms, or \
         restate the discount rates or the cash flows in the same terms{note}",
        flows.label(),
        given.label(),
        flows.label(),
        given.label(),
        flows.label()
    ))
    // Terms that differ are given without the inflation they need: `requires`, and
    // `/inflation`, absent by definition, is the field to give (review round 6). A terms field
    // left to its default is read, never named (final review of #279, finding 3).
    .at(
        if d.cash_flow_terms {
            "/discount_rate_terms"
        } else {
            "/cash_flow_terms"
        },
        Reason::Requires,
    )
    .between(
        [
            ("/cash_flow_terms", d.cash_flow_terms),
            ("/discount_rate_terms", d.discount_rate_terms),
            ("/inflation", false),
        ]
        .into_iter()
        .filter(|(_, defaulted)| !defaulted)
        .map(|(field, _)| field),
    )
    .reads(["/cash_flow_terms", "/discount_rate_terms"])
    // No inflation in any form: the per-period form is read too.
    .reads(["/inflation_per_period", "/terminal_inflation"])
}

/// R14 to R16, R18: equal terms use the rates as given; different terms are refused without
/// inflation (`FM-E024`) and otherwise every rate, the terminal rate included, is converted to
/// the terms of the cash flows. Flows and growth rates are never converted (Decision 5).
fn resolve_terms(
    ex: &mut Explain,
    input: &EngineInput<'_>,
    rates: &[Decimal],
    terminal: Option<(Decimal, &str)>,
    inflation: Option<&Inflation>,
) -> Result<RatesUsed> {
    let (flows, given) = (input.cash_flow_terms, input.discount_rate_terms);
    let f = &input.rates.fields;
    let terminal_field = terminal.map(|(_, f)| f).unwrap_or_default();
    match inflation {
        Some(i) if i.single => ex.convention(match i.terminal {
            Some(t) => format!(
                "expected inflation: one rate, {t}, for every period and for the terminal phase \
                 (`/inflation`)"
            ),
            None => format!(
                "expected inflation: one rate, {}, for every period (`/inflation`); the terminal \
                 value uses no terminal rate",
                i.per_period.first().copied().unwrap_or_default()
            ),
        }),
        Some(i) => ex.convention(match i.terminal {
            Some(t) => format!(
                "expected inflation: one rate per period (`/inflation_per_period`); terminal \
                 inflation {t} (`/terminal_inflation`)"
            ),
            None => "expected inflation: one rate per period (`/inflation_per_period`); no \
                     terminal inflation, the terminal value uses no terminal rate"
                .to_owned(),
        }),
        None => {}
    }
    if flows == given {
        if let Some(i) = inflation.filter(|i| !i.single) {
            let values: Vec<String> = i.per_period.iter().map(Decimal::to_string).collect();
            ex.convention(match i.terminal {
                Some(t) => format!(
                    "expected inflation per period ({}) validated but not used: the terms are \
                     equal, no discount rate is converted (R18); only the terminal inflation {t} \
                     restates the perpetual growth",
                    values.join(", ")
                ),
                None => format!(
                    "expected inflation per period ({}) validated but not used: the terms are \
                     equal, no discount rate is converted (R18)",
                    values.join(", ")
                ),
            });
        }
        ex.convention(match inflation {
            None => format!(
                "terms: cash flows and growth rates {}, discount rates {}; same terms, no \
                 conversion",
                flows.label(),
                given.label()
            ),
            Some(_) => format!(
                "terms: cash flows and growth rates {}, discount rates {}; same terms, no \
                 conversion: expected inflation is used only to restate the perpetual growth in \
                 the other terms (R18)",
                flows.label(),
                given.label()
            ),
        });
        return Ok(RatesUsed {
            conversion: RateConversion::None,
            per_period: rates.iter().map(|r| (*r, None)).collect(),
            terminal: terminal.map(|(r, _)| r),
            terminal_formula: None,
            terminal_field: terminal_field.to_owned(),
            terminal_converted_with: None,
        });
    }
    let Some(inflation) = inflation else {
        return Err(terms_mismatch(input));
    };
    let source = format!("{} v{}", RealNominal::ID, RealNominal::VERSION);
    ex.convention(format!(
        "terms: cash flows and growth rates {}, discount rates {}; every discount rate{}, \
         converted to {} terms with expected inflation by the exact Fisher relation \
         (1 + nominal) = (1 + real) × (1 + inflation) ({source}); cash flows and growth rates \
         are never converted (Decision 5)",
        flows.label(),
        given.label(),
        if terminal.is_some() {
            ", the terminal rate included"
        } else {
            ""
        },
        flows.label()
    ));
    // Every converted rate out of range is reported, the terminal rate included.
    let mut found = Found::new();
    let mut per_period = Vec::with_capacity(rates.len());
    for (k, (rate, infl)) in rates.iter().zip(&inflation.per_period).enumerate() {
        let (used, formula) = fisher(*rate, *infl, flows)?;
        if used <= -Decimal::ONE {
            let field = match input.rates.single {
                Some(_) => format!("/{}", f.single),
                None => format!("/{}/{k}", f.per_period),
            };
            found.push(FairmarkError::DiscountRateOutOfRange {
                field,
                rate: used,
                converted_with: Some(inflation.field(Some(k)).into()),
            });
        }
        per_period.push((used, Some((*infl, formula))));
    }
    let conversion = match flows {
        Terms::Real => RateConversion::ToReal,
        Terms::Nominal => RateConversion::ToNominal,
    };
    let Some((terminal_rate, _)) = terminal else {
        found.finish()?;
        return Ok(RatesUsed {
            conversion,
            per_period,
            terminal: None,
            terminal_formula: None,
            terminal_field: String::new(),
            terminal_converted_with: None,
        });
    };
    let terminal_inflation = inflation.terminal.ok_or(FairmarkError::Internal(
        "terminal rate without terminal inflation",
    ))?;
    let (used, formula) = fisher(terminal_rate, terminal_inflation, flows)?;
    let converted_with = inflation.field(None);
    if used <= -Decimal::ONE {
        found.push(FairmarkError::DiscountRateOutOfRange {
            field: terminal_field.to_owned(),
            rate: used,
            converted_with: Some(converted_with.as_str().into()),
        });
    }
    found.finish()?;
    Ok(RatesUsed {
        conversion,
        per_period,
        terminal: Some(used),
        terminal_formula: Some(formula),
        terminal_field: terminal_field.to_owned(),
        terminal_converted_with: Some(converted_with.as_str().into()),
    })
}

/// R17: the perpetual growth of the terminal technique restated in the other terms with the
/// inflation of the terminal phase, and for nominal flows a warning when it is below that
/// inflation (a real decline). Returns the growth and its restatement.
fn restate_growth(
    ex: &mut Explain,
    choice: TerminalChoice<'_>,
    flows: Terms,
    inflation: Option<&Inflation>,
) -> Result<(Option<Decimal>, Option<Decimal>)> {
    let growth = choice.perpetual_growth();
    let Some(inflation) = inflation else {
        return Ok((growth, None));
    };
    let id = choice.id();
    let Some(g) = growth else {
        ex.convention(format!(
            "perpetual growth not restated in {} terms: {id} states no perpetual growth (R17)",
            flows.other().label()
        ));
        return Ok((None, None));
    };
    let other = flows.other();
    let pi = inflation.terminal.ok_or(FairmarkError::Internal(
        "perpetual growth without terminal inflation",
    ))?;
    let (restated, formula) = fisher(g, pi, other)?;
    ex.step(
        format!("Perpetual growth in {} terms", other.label()),
        formula.clone(),
        restated,
    );
    ex.convention(format!(
        "perpetual growth of the terminal value ({id}): {g} in {} terms, {formula} = {restated} \
         in {} terms, with the expected inflation of the terminal phase {pi} (R17)",
        flows.label(),
        other.label()
    ));
    if flows == Terms::Nominal && g < pi {
        ex.convention(format!(
            "warning: the perpetual growth {g} (nominal) is below the expected inflation of the \
             terminal phase {pi}: the flows decline in real terms after the horizon (real growth \
             {restated})"
        ));
    }
    Ok((Some(g), Some(restated)))
}

/// R21: the terminal discounting time is required under mid-period timing and refused at end
/// of period; with no terminal value it is refused too (Decision 24).
fn check_timing(
    timing: Timing,
    time: Option<TerminalDiscountTime>,
    no_terminal_value: bool,
    timing_defaulted: bool,
    terminal_defaulted: bool,
) -> Result<()> {
    match (timing, time) {
        // Decision 24: say so when the timing was not given.
        (Timing::EndOfPeriod, Some(_)) if timing_defaulted => Err(invalid(
            "`/terminal_discount_time` is not accepted with `/timing` = end_of_period: \
             `/timing` was not given and defaults to end_of_period; give `timing: mid_period` \
             or remove `/terminal_discount_time`"
                .to_owned(),
        )
        // The timing left to its default is read, never named (final review, finding 3).
        .at("/terminal_discount_time", Reason::Requires)
        .reads(["/timing"])),
        (Timing::EndOfPeriod, Some(_)) => Err(invalid(
            "`/terminal_discount_time` is not accepted with `/timing` = end_of_period: the \
             terminal value is then discounted at the horizon N; remove it"
                .to_owned(),
        )
        .at("/terminal_discount_time", Reason::Requires)
        .between(["/terminal_discount_time", "/timing"])
        .fix("remove `/terminal_discount_time`")),
        (Timing::MidPeriod, Some(_)) if no_terminal_value => Err(invalid(
            "`/terminal_discount_time` is not used: there is no terminal value to discount \
             (`/terminal` is none); remove it, or give a terminal value in `/terminal`"
                .to_owned(),
        )
        .at("/terminal_discount_time", Reason::Requires)
        .between(if terminal_defaulted {
            Vec::new()
        } else {
            vec!["/terminal_discount_time", "/terminal"]
        })
        .reads(["/timing", "/terminal"])),
        (Timing::MidPeriod, None) if !no_terminal_value => Err(invalid(
            "`/terminal_discount_time` is required with `/timing` = mid_period: give `horizon` \
             (t = N) or `mid_period` (t = N - 0.5)"
                .to_owned(),
        )
        .at("/terminal_discount_time", Reason::Missing)
        .between(["/terminal_discount_time", "/timing"])
        .expected(["horizon", "mid_period"])
        .reads(["/terminal"])),
        _ => Ok(()),
    }
}

/// How far the engine goes: the input checks alone (the validate-only path of the Method
/// trait), or the checks and the calculation.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pass {
    /// Every input check, then stop: no flow is built up, projected or discounted.
    Validate,
    /// Every input check, then the calculation.
    Compute,
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
            StageKind::Given(_) if s.built_up.is_some() => format!(
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
                    let source = match s.built_up {
                        Some(name) => format!(
                            "built up from line items (step \"Build-up: {name}, period {k}\")"
                        ),
                        None => "as given".to_owned(),
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

/// R7: on the enterprise basis the DCF discounts FCFF, before financing: the first line of
/// each stage that gives `interest_expense` or `net_borrowing` is refused. Checked on the
/// stages as given, so that no other refusal of a stage (its periods, the count of its lines)
/// hides it, and it hides none.
fn financing_on_enterprise_basis(stages: &[Stage], basis: TerminalBasis) -> Vec<FairmarkError> {
    if basis != TerminalBasis::Enterprise {
        return Vec::new();
    }
    let mut errors = Vec::new();
    for (i, stage) in stages.iter().enumerate() {
        let Some(lines) = stage.lines.as_deref() else {
            continue;
        };
        let Some((k, l)) = lines
            .iter()
            .enumerate()
            .find(|(_, l)| l.interest_expense.is_some() || l.net_borrowing.is_some())
        else {
            continue;
        };
        let pointer = format!("/stages/{i}/lines");
        // Name only the financing fields actually given, each with its pointer.
        let given: Vec<String> = [
            ("interest_expense", l.interest_expense.is_some()),
            ("net_borrowing", l.net_borrowing.is_some()),
        ]
        .into_iter()
        .filter(|(_, present)| *present)
        .map(|(field, _)| format!("{pointer}/{k}/{field}"))
        .collect();
        let quoted: Vec<String> = given.iter().map(|p| format!("`{p}`")).collect();
        let (fields, verb, pronoun) = match quoted.as_slice() {
            [one] => (one.clone(), "is", "it"),
            _ => (quoted.join(" and "), "are", "them"),
        };
        // Pointers name only the fields given; the lines are read (the first line with
        // financing is the one named).
        let read = [pointer.clone()];
        let relation = if given.len() > 1 {
            given.clone()
        } else {
            Vec::new()
        };
        errors.push(
            invalid(format!(
                "{fields} {verb} not used on the enterprise basis: the DCF discounts FCFF, \
                 which is before financing; remove {pronoun}"
            ))
            .at(given[0].clone(), Reason::Requires)
            .between(relation)
            .reads(read)
            .fix(format!("remove {fields}")),
        );
    }
    errors
}

/// R7: the flows of the `given` stages with `lines`, built up as in `dcf.free_cash_flows`
/// (FCFF on the enterprise basis, FCFE on the equity basis), with every step in the explain and
/// the conventions of the build-up reported once. The tax fields are refused when no stage has
/// lines.
fn build_up_stages(
    raw: &[Stage],
    stages: &mut [ResolvedStage<'_>],
    basis: TerminalBasis,
    taxes: &TaxChoice,
    ex: &mut Explain,
    pass: Pass,
) -> Result<()> {
    let (_, positions) = stage_positions(raw);
    lines_of_stages(stages, basis, taxes, ex, pass, Some(&positions), true)
}

/// The checks of the lines of the stages before the first refused stage, when the stages are
/// refused (ADR 0014 G5): their numbering is known, so their errors are found as when no stage
/// is refused. The fields of the taxes are not checked against the lines: a refused stage may
/// hold lines.
fn check_lines_before_refusal(input: &EngineInput<'_>) -> Result<()> {
    if input.stages.len() > MAX_STAGES {
        return Ok(());
    }
    let (mut known, positions) = stage_positions(input.stages);
    let mut scratch = Explain::new("dcf", "");
    lines_of_stages(
        &mut known,
        input.basis,
        &input.taxes,
        &mut scratch,
        Pass::Validate,
        Some(&positions),
        false,
    )
}

/// The stages that resolve, each with its [`Position`], as far as the first periods are
/// known: the periods of a refused stage are known when given, or counted from its only list.
fn stage_positions(stages: &[Stage]) -> (Vec<ResolvedStage<'_>>, Vec<Position>) {
    let mut known = Vec::new();
    let mut positions: Vec<Position> = Vec::new();
    let mut first = 1u32;
    let mut earlier: Vec<String> = Vec::new();
    for (i, s) in stages.iter().enumerate() {
        let counted = match (&s.flows, &s.lines) {
            (Some(f), None) => u32::try_from(f.len()).ok(),
            (None, Some(l)) => u32::try_from(l.len()).ok(),
            _ => None,
        };
        let resolved = check_stage(i, s).ok();
        let periods = match (&resolved, s.periods) {
            (Some(stage), _) => Some(stage.periods),
            (None, Some(p)) if p > 0 => Some(p),
            (None, None) => counted.filter(|c| *c > 0),
            _ => None,
        };
        if let Some(stage) = resolved {
            known.push(stage);
            positions.push((i, first, earlier.clone()));
        }
        let Some(periods) = periods else { break };
        first = first.saturating_add(periods);
        // The fields that decide the periods of this stage, read by every later stage.
        earlier.push(format!("/stages/{i}/periods"));
        if s.periods.is_none() {
            earlier.push(format!("/stages/{i}/flows"));
            earlier.push(format!("/stages/{i}/lines"));
        }
    }
    (known, positions)
}

/// The index of a stage, its first period and the fields that decide its first period.
type Position = (usize, u32, Vec<String>);

fn lines_of_stages(
    stages: &mut [ResolvedStage<'_>],
    basis: TerminalBasis,
    taxes: &TaxChoice,
    ex: &mut Explain,
    pass: Pass,
    positions: Option<&[Position]>,
    every_stage: bool,
) -> Result<()> {
    // Each error declares what it read: the errors of a stage its stage and the periods of the
    // stages before it (which number its periods); the tax fields given without lines every
    // stage (which stages have lines).
    stage_lines(stages, basis, taxes, ex, pass, positions, every_stage)
}

fn stage_lines(
    stages: &mut [ResolvedStage<'_>],
    basis: TerminalBasis,
    taxes: &TaxChoice,
    ex: &mut Explain,
    pass: Pass,
    positions: Option<&[Position]>,
    every_stage: bool,
) -> Result<()> {
    let mut running = 1u32;
    let mut any = false;
    let mut conventions: Vec<String> = Vec::new();
    let mut found = Found::new();
    for (n, s) in stages.iter_mut().enumerate() {
        let (i, first, earlier) = match positions {
            Some(p) => p[n].clone(),
            None => (
                n,
                running,
                (0..n).map(|j| format!("/stages/{j}/periods")).collect(),
            ),
        };
        if let StageKind::Lines(lines) = s.kind {
            any = true;
            let pointer = format!("/stages/{i}/lines");
            // Lines with financing on the enterprise basis are refused by
            // [`financing_on_enterprise_basis`], with the other stages: not built up here.
            if basis == TerminalBasis::Enterprise
                && lines
                    .iter()
                    .any(|l| l.interest_expense.is_some() || l.net_borrowing.is_some())
            {
                running = running.saturating_add(s.periods);
                continue;
            }
            let at = Location {
                pointer_prefix: &pointer,
                first_period: first,
            };
            let missing_financing = || {
                invalid(format!(
                    "`{pointer}/0/interest_expense` and `{pointer}/0/net_borrowing` are \
                     missing: the equity basis discounts FCFE, which needs the interest \
                     expense and the net borrowing in every line of `{pointer}`"
                ))
                .at(format!("{pointer}/0/interest_expense"), Reason::Missing)
                .between([
                    format!("{pointer}/0/interest_expense"),
                    format!("{pointer}/0/net_borrowing"),
                ])
            };
            // No line gives financing: the FCFE cannot be built, whatever the taxes. Reported
            // with the errors of the lines, which do not hide it.
            let no_financing = basis == TerminalBasis::Equity
                && lines
                    .iter()
                    .all(|l| l.interest_expense.is_none() && l.net_borrowing.is_none());
            // Every stage is checked: a refused stage does not hide the others (ADR 0014 G5).
            if pass == Pass::Validate || no_financing || !found.is_empty() {
                let read: Vec<String> = std::iter::once(format!("/stages/{i}"))
                    .chain(earlier)
                    .collect();
                found.check(
                    check_lines(lines, taxes, &at)
                        .map(|_| ())
                        .map_err(|e| e.reads(read.clone())),
                );
                if no_financing {
                    found.push(missing_financing().reads(read));
                }
                running = running.saturating_add(s.periods);
                continue;
            }
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
            let (flows, name) = match basis {
                TerminalBasis::Enterprise => (out.free_cash_flows_to_firm, "FCFF"),
                TerminalBasis::Equity => (
                    out.free_cash_flows_to_equity
                        .ok_or_else(missing_financing)?,
                    "FCFE",
                ),
            };
            s.kind = StageKind::Given(Cow::Owned(flows));
            s.built_up = Some(name);
        }
        running = running.saturating_add(s.periods);
    }
    if !found.is_empty() {
        return found.finish();
    }
    if any {
        let source = format!("{} v{}", FreeCashFlows::ID, FreeCashFlows::VERSION);
        ex.convention(match basis {
            TerminalBasis::Enterprise => format!(
                "free cash flow build-up ({source}) for the stages with lines: {FORMULA_FCFF}"
            ),
            TerminalBasis::Equity => format!(
                "free cash flow build-up ({source}) for the stages with lines: {FORMULA_FCFF}; \
                 {FORMULA_FCFE}"
            ),
        });
        for c in conventions {
            ex.convention(format!("free cash flow build-up ({source}): {c}"));
        }
        return Ok(());
    }
    if !every_stage {
        return Ok(());
    }
    // Every field of the taxes given without lines is reported: none hides another.
    for (field, given) in [
        ("notional_tax_rate", taxes.notional_tax_rate.is_some()),
        ("negative_ebit", taxes.negative_ebit.is_some()),
        ("interest_tax_rate", taxes.interest_tax_rate.is_some()),
    ] {
        if given {
            found.push(
                invalid(format!(
                    "`/{field}` is used only by a `given` stage with `lines`; remove it, or give \
                     the line items of a stage in `lines`"
                ))
                .at(format!("/{field}"), Reason::Requires)
                .fix(format!(
                    "remove `/{field}`, or give the line items of a stage in `lines`"
                ))
                .reads(["/stages"]),
            );
        }
    }
    found.finish()
}

fn period_timing(t: Timing) -> PeriodTiming {
    match t {
        Timing::EndOfPeriod => PeriodTiming::EndOfPeriod,
        Timing::MidPeriod => PeriodTiming::MidPeriod,
    }
}

/// Runs the engine: validates every choice, projects the flows, discounts them, computes and
/// discounts the terminal value. The caller adds its basis convention and its bridge.
///
/// `found` holds the refusals of the caller's own checks (the bridge lines). The checks of the
/// input go on after a refusal and every refusal is reported (ADR 0014 G5), in the order of
/// the checks; a check that needs a refused input is skipped.
pub(crate) fn run(input: &EngineInput<'_>, ex: &mut Explain, found: Found) -> Result<EngineOutput> {
    run_pass(input, ex, found, Pass::Compute)?.ok_or(FairmarkError::Internal(
        "the compute pass stopped after the checks",
    ))
}

/// The validate-only path (the `validate` of the Method trait): every check that [`run`]
/// makes before it computes, every refusal reported, and nothing built up, projected or
/// discounted.
pub(crate) fn validate(input: &EngineInput<'_>, found: Found) -> Result<()> {
    let mut scratch = Explain::new("dcf", "");
    run_pass(input, &mut scratch, found, Pass::Validate).map(|_| ())
}

/// [`run`] and [`validate`]: `None` when `pass` is [`Pass::Validate`] and every check passed.
fn run_pass(
    input: &EngineInput<'_>,
    ex: &mut Explain,
    mut found: Found,
    pass: Pass,
) -> Result<Option<EngineOutput>> {
    // Refused stages leave the horizon unknown: the checks that need it are skipped, every other
    // check is made (ADR 0014 G5), and nothing is computed.
    let mut stages = found.take(check_stages(
        input.stages,
        input.base_flow,
        input.defaulted.first_projection,
    ));
    for e in financing_on_enterprise_basis(input.stages, input.basis) {
        found.push(e);
    }
    match stages.as_mut() {
        Some((stages, _)) => {
            found.check(build_up_stages(
                input.stages,
                stages,
                input.basis,
                &input.taxes,
                ex,
                pass,
            ));
        }
        // A refused stage, or stages refused together, do not hide the lines of the stages
        // before the first refused one: they are checked, nothing is built up.
        None => {
            found.check(check_lines_before_refusal(input));
        }
    }
    let n = stages.as_ref().map(|(_, n)| *n);
    // The terminal option is read first, because it decides whether a terminal rate is used
    // (Decision 24); an error in it is reported where the technique is checked, as before.
    let choice = terminal::choose(input.terminal);
    let rate_use = match &choice {
        Ok(c) => c.terminal_rate_use(input.timing, input.terminal_discount_time),
        Err(_) => TerminalRateUse::Technique,
    };
    let no_terminal_value = matches!(choice, Ok(TerminalChoice::None));
    let rates = resolve_rates(&input.rates, n, rate_use, &mut found);
    let inflation = resolve_inflation(&input.inflation, n, rate_use, &mut found);
    // The rates in the terms of the flows, also without a horizon: the terminal rate does not
    // depend on it, and the per-period rates are converted as far as both lists go.
    let used = match (&rates, &inflation) {
        (Some((given_rates, terminal_given)), Some(inflation)) => found.take(resolve_terms(
            ex,
            input,
            given_rates,
            terminal_given.as_ref().map(|(r, f)| (*r, f.as_str())),
            inflation.as_ref(),
        )),
        // Without the rates nothing is converted; terms that differ without expected inflation
        // are refused all the same.
        (_, Some(None)) if input.cash_flow_terms != input.discount_rate_terms => {
            found.push(terms_mismatch(input));
            None
        }
        _ => None,
    };
    found.check(check_timing(
        input.timing,
        input.terminal_discount_time,
        no_terminal_value,
        input.defaulted.timing,
        input.defaulted.terminal,
    ));
    // The terms of the terminal technique are checked with the rest of the input: with the
    // terminal rate when it is known, otherwise every term that does not depend on it.
    // The terminal rate is decided by the rates, the expected inflation and the terms: a
    // comparison with it reads them all.
    let f = &input.rates.fields;
    let decide_rate = [
        format!("/{}", f.single),
        format!("/{}", f.per_period),
        format!("/{}", f.terminal),
        "/inflation".to_owned(),
        "/inflation_per_period".to_owned(),
        "/terminal_inflation".to_owned(),
        "/cash_flow_terms".to_owned(),
        "/discount_rate_terms".to_owned(),
    ];
    let choice = match (found.take(choice), &used) {
        (Some(choice), Some(used)) => found.take(
            terminal::check(
                choice,
                used.terminal,
                &used.terminal_field,
                used.terminal_converted_with.as_deref(),
                input.basis,
            )
            .map_err(|e| {
                let mut each = Found::new();
                for e in e.into_errors() {
                    each.push(if terminal::compares_with_rate(&e) {
                        e.reads(decide_rate.clone())
                    } else {
                        e
                    });
                }
                each.into_error()
            }),
        ),
        (Some(choice), None) => {
            for e in terminal::check_terms(choice, input.basis) {
                found.push(e);
            }
            None
        }
        (None, _) => None,
    };
    let (
        Some(choice),
        Some(used),
        Some((given_rates, terminal_given)),
        Some(inflation),
        Some((stages, n)),
        true,
    ) = (choice, used, rates, inflation, stages, found.is_empty())
    else {
        return Err(found.into_error());
    };
    if pass == Pass::Validate {
        return Ok(None);
    }
    let terminal_rate = used.terminal;
    let rates: Vec<Decimal> = used.per_period.iter().map(|(r, _)| *r).collect();

    ex.convention(format!(
        "explicit horizon N = {n} periods; stages: {}; one period is one year, rates are \
         annual effective",
        input.stages.len()
    ));
    let as_given = match used.conversion {
        RateConversion::None => String::new(),
        _ => format!(
            " (as given, in {} terms)",
            input.discount_rate_terms.label()
        ),
    };
    ex.convention(match (input.rates.single, &terminal_given) {
        (Some(r), Some(_)) => format!(
            "discount rate: one rate, {r}, for every period and for the terminal value{as_given}"
        ),
        (Some(r), None) => format!(
            "discount rate: one rate, {r}, for every period{as_given}; the terminal value uses \
             no terminal rate"
        ),
        (None, Some((t, _))) => format!(
            "discount rate: one rate per period; terminal rate {t} given separately{as_given}"
        ),
        (None, None) => format!(
            "discount rate: one rate per period{as_given}; the terminal value uses no terminal \
             rate"
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
        ex.step(
            format!("Period {period}: discount rate"),
            "as given",
            given_rates[k],
        );
        let conversion = &used.per_period[k].1;
        if let Some((infl, fisher_formula)) = conversion {
            ex.step(
                format!("Period {period}: expected inflation"),
                "as given",
                *infl,
            );
            ex.step(
                format!(
                    "Period {period}: discount rate used ({} terms)",
                    input.cash_flow_terms.label()
                ),
                fisher_formula.clone(),
                *rate,
            );
        }
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
            discount_rate: given_rates[k],
            rate_used: *rate,
            inflation: conversion.as_ref().map(|(i, _)| *i),
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

    if let (Some(formula), Some(i), Some((given, _)), Some(rate)) = (
        &used.terminal_formula,
        &inflation,
        &terminal_given,
        terminal_rate,
    ) {
        ex.step("Terminal rate: as given", "as given", *given);
        if let Some(t) = i.terminal {
            ex.step(
                "Terminal rate: expected inflation of the terminal phase",
                "as given",
                t,
            );
        }
        ex.step(
            format!(
                "Terminal rate used ({} terms)",
                input.cash_flow_terms.label()
            ),
            formula.clone(),
            rate,
        );
    }
    let (perpetual_growth, perpetual_growth_other_terms) =
        restate_growth(ex, choice, input.cash_flow_terms, inflation.as_ref())?;

    let last = &projected[n - 1];
    let terminal = terminal::value(
        ex,
        &TerminalContext {
            choice,
            basis: input.basis,
            timing: input.timing,
            discount_time: input.terminal_discount_time,
            rate: terminal_rate,
            last_flow: last.flow,
            last_growth: last.growth,
            ends: &ends,
            g_n: growth_factors[n],
            converted_with: used.terminal_converted_with.as_deref(),
        },
    )?;
    Ok(Some(EngineOutput {
        periods,
        pv_explicit,
        terminal,
        terms: TermsOutput {
            cash_flow_terms: input.cash_flow_terms,
            discount_rate_terms: input.discount_rate_terms,
            conversion: used.conversion,
            terminal_rate_given: terminal_given.map(|(r, _)| r),
            terminal_rate_used: terminal_rate,
            terminal_inflation: inflation.as_ref().and_then(|i| i.terminal),
            perpetual_growth,
            perpetual_growth_other_terms,
        },
    }))
}

#[cfg(test)]
mod tests {
    //! Nominal and real terms (#166, R14 to R18), through `dcf.fcff`.

    use serde_json::{json, Value};

    use crate::registry::run;

    fn base() -> Value {
        json!({
            "stages": [{"periods": 3, "projection": "given", "flows": ["100", "110", "120"]}],
            "cash_flow_terms": "real",
            "discount_rate_terms": "nominal",
            "wacc": "0.10",
            "inflation": "0.02",
            "timing": "end_of_period",
            "terminal": {"tv.gordon": {"growth": "0.01", "first_flow": "last_flow_grown"}}
        })
    }

    fn with(mut v: Value, f: impl FnOnce(&mut serde_json::Map<String, Value>)) -> Value {
        f(v.as_object_mut().unwrap());
        v
    }

    fn out(v: &Value) -> Value {
        run("dcf.fcff", &v.to_string()).unwrap()
    }

    fn conventions(v: &Value) -> Vec<String> {
        serde_json::from_value(out(v)["explain"]["conventions"].clone()).unwrap()
    }

    fn err(v: &Value) -> crate::FairmarkError {
        run("dcf.fcff", &v.to_string()).unwrap_err()
    }

    /// R16: every rate converted with the substituted Fisher formula in the explain, through
    /// `rates.real_nominal`; the flows are not converted.
    #[test]
    fn rates_are_converted_and_flows_are_not() {
        let o = out(&base());
        let v = &o["value"];
        assert_eq!(v["terms"]["conversion"], "to_real");
        assert_eq!(v["periods"][0]["flow"], "100");
        assert_eq!(v["periods"][0]["discount_rate"], "0.10");
        assert_eq!(v["periods"][0]["inflation"], "0.02");
        assert_eq!(
            v["periods"][0]["rate_used"],
            "0.0784313725490196078431372549"
        );
        assert_eq!(v["terminal"]["rate"], v["terms"]["terminal_rate_used"]);
        assert_eq!(v["terms"]["terminal_rate_given"], "0.10");
        let steps = o["explain"]["steps"].as_array().unwrap();
        let used = steps
            .iter()
            .find(|s| s["label"] == "Period 2: discount rate used (real terms)")
            .unwrap();
        assert_eq!(used["formula"], "(1 + 0.10) / (1 + 0.02) − 1");
        assert!(steps
            .iter()
            .any(|s| s["label"] == "Terminal rate used (real terms)"
                && s["formula"] == "(1 + 0.10) / (1 + 0.02) − 1"));
        let c = conventions(&base());
        assert!(c.iter().any(|c| c
            .contains("every discount rate, the terminal rate included, converted to real terms")
            && c.contains("(rates.real_nominal v1)")
            && c.contains("cash flows and growth rates are never converted")));
        assert!(c
            .iter()
            .any(|c| c.starts_with("expected inflation: one rate, 0.02")));
    }

    /// R16: nominal flows at real rates are converted to nominal terms.
    #[test]
    fn real_rates_are_converted_to_nominal() {
        let input = with(base(), |m| {
            m.insert("cash_flow_terms".into(), json!("nominal"));
            m.insert("discount_rate_terms".into(), json!("real"));
            m.insert("wacc".into(), json!("0.08"));
        });
        let v = &out(&input)["value"];
        assert_eq!(v["terms"]["conversion"], "to_nominal");
        assert_eq!(v["periods"][2]["rate_used"], "0.1016");
        assert_eq!(v["terminal"]["rate"], "0.1016");
    }

    /// R18: equal terms with inflation, no conversion; the periods carry no inflation.
    #[test]
    fn equal_terms_are_not_converted() {
        let input = with(base(), |m| {
            m.insert("cash_flow_terms".into(), json!("nominal"));
        });
        let v = &out(&input)["value"];
        assert_eq!(v["terms"]["conversion"], "none");
        assert_eq!(v["periods"][0]["rate_used"], "0.10");
        assert_eq!(v["periods"][0]["inflation"], Value::Null);
        assert!(conventions(&input).iter().any(|c| c.contains(
            "same terms, no conversion: expected inflation is used only to restate the \
             perpetual growth in the other terms (R18)"
        )));
        let none = with(input, |m| {
            m.remove("inflation");
        });
        let v = &out(&none)["value"];
        assert_eq!(v["terms"]["terminal_inflation"], Value::Null);
        assert_eq!(v["terms"]["perpetual_growth"], "0.01");
        assert_eq!(v["terms"]["perpetual_growth_other_terms"], Value::Null);
        assert!(conventions(&none).iter().any(|c| c
            == "terms: cash flows and growth rates nominal, discount rates nominal; same \
                terms, no conversion"));
    }

    /// R17, AC10: the perpetual growth in the other terms; a nominal growth below inflation is
    /// a warning, never an error; real flows and growths at or above inflation carry none.
    #[test]
    fn growth_below_inflation_is_a_warning() {
        let nominal = with(base(), |m| {
            m.insert("cash_flow_terms".into(), json!("nominal"));
            m.insert("inflation".into(), json!("0.03"));
        });
        let o = out(&nominal);
        assert_eq!(
            o["value"]["terms"]["perpetual_growth_other_terms"]
                .as_str()
                .unwrap()[..5]
                .to_owned(),
            "-0.01"
        );
        let warning = |c: &Vec<String>| {
            c.iter()
                .any(|c| c.starts_with("warning: the perpetual growth 0.01 (nominal) is below"))
        };
        assert!(warning(&conventions(&nominal)));
        assert!(conventions(&nominal).iter().any(|c| c.starts_with(
            "perpetual growth of the terminal value (tv.gordon): 0.01 in nominal terms, (1 + \
             0.01) / (1 + 0.03) − 1 ="
        )));
        let above = with(nominal.clone(), |m| {
            m.insert("inflation".into(), json!("0.005"));
        });
        assert!(!warning(&conventions(&above)));
        // Real flows: restated in nominal terms, no warning.
        let real = with(base(), |m| {
            m.insert("discount_rate_terms".into(), json!("real"));
            m.insert("inflation".into(), json!("0.03"));
        });
        let v = &out(&real)["value"];
        assert_eq!(v["terms"]["perpetual_growth_other_terms"], "0.0403");
        assert!(!conventions(&real)
            .iter()
            .any(|c| c.starts_with("warning: the perpetual")));
    }

    /// R17: a technique with no perpetual growth is not restated, and the conventions say so.
    #[test]
    fn techniques_without_perpetual_growth_are_reported() {
        let input = with(base(), |m| {
            m.insert(
                "terminal".into(),
                json!({"tv.exit_multiple": {"multiple": "8", "metric": "ebitda", "metric_value": "150"}}),
            );
        });
        let v = &out(&input)["value"];
        assert_eq!(v["terms"]["perpetual_growth"], Value::Null);
        assert_eq!(v["terms"]["perpetual_growth_other_terms"], Value::Null);
        assert!(conventions(&input).iter().any(|c| c
            == "perpetual growth not restated in nominal terms: tv.exit_multiple states no \
                perpetual growth (R17)"));
    }

    /// R16: with inflation per period, each rate uses the inflation of its period and the
    /// terminal rate the terminal inflation.
    #[test]
    fn inflation_per_period_converts_each_rate() {
        let input = with(base(), |m| {
            m.remove("inflation");
            m.insert("inflation_per_period".into(), json!(["0.1", "0", "0.25"]));
            m.insert("terminal_inflation".into(), json!("0.0"));
        });
        let v = &out(&input)["value"];
        assert_eq!(v["periods"][0]["rate_used"], "0.0");
        assert_eq!(v["periods"][1]["rate_used"], "0.10");
        assert_eq!(v["periods"][2]["rate_used"], "-0.12");
        assert_eq!(v["periods"][2]["inflation"], "0.25");
        assert_eq!(v["terminal"]["rate"], "0.10");
        assert_eq!(v["terms"]["terminal_inflation"], "0.0");
    }

    /// R15, R27: `FM-E024` names both fields and states both fixes; every error of the
    /// inflation fields names its field by a JSON pointer.
    #[test]
    fn errors_name_the_fields_and_the_fixes() {
        let e = err(&with(base(), |m| {
            m.remove("inflation");
        }));
        assert_eq!(e.code(), "FM-E024");
        let text = e.to_string();
        for part in [
            "`/cash_flow_terms` is real",
            "`/discount_rate_terms` is nominal",
            "give expected inflation in `/inflation`",
            "restate the discount rates or the cash flows in the same terms",
        ] {
            assert!(text.contains(part), "{part}: {text}");
        }
        let cases: Vec<(Value, &str, &str)> = vec![
            (
                with(base(), |m| {
                    m.insert("inflation_per_period".into(), json!(["0", "0", "0"]));
                }),
                "FM-E001",
                "/inflation_per_period",
            ),
            (
                with(base(), |m| {
                    m.insert("terminal_inflation".into(), json!("0.02"));
                }),
                "FM-E001",
                "/terminal_inflation",
            ),
            (
                with(base(), |m| {
                    m.remove("inflation");
                    m.insert("inflation_per_period".into(), json!(["0", "0", "0"]));
                }),
                "FM-E001",
                "/terminal_inflation",
            ),
            (
                with(base(), |m| {
                    m.remove("inflation");
                    m.insert("terminal_inflation".into(), json!("0.02"));
                }),
                "FM-E001",
                "/inflation_per_period",
            ),
            (
                with(base(), |m| {
                    m.remove("inflation");
                    m.insert("inflation_per_period".into(), json!(["0", "0"]));
                    m.insert("terminal_inflation".into(), json!("0.02"));
                }),
                "FM-E025",
                "/inflation_per_period",
            ),
            (
                with(base(), |m| {
                    m.remove("inflation");
                    m.insert("inflation_per_period".into(), json!(["0", "-1", "0"]));
                    m.insert("terminal_inflation".into(), json!("0.02"));
                }),
                "FM-E003",
                "/inflation_per_period/1",
            ),
            (
                with(base(), |m| {
                    m.remove("inflation");
                    m.insert("inflation_per_period".into(), json!(["0", "0", "0"]));
                    m.insert("terminal_inflation".into(), json!("-1.5"));
                }),
                "FM-E003",
                "/terminal_inflation",
            ),
            (
                with(base(), |m| {
                    m.insert("inflation".into(), json!("-1"));
                }),
                "FM-E003",
                "/inflation",
            ),
            // The terminal rate converted to real terms (0.0194...) is below the growth.
            (
                with(base(), |m| {
                    m.insert("wacc".into(), json!("0.05"));
                    m.insert("inflation".into(), json!("0.03"));
                    m.insert(
                        "terminal".into(),
                        json!({"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}),
                    );
                }),
                "FM-E004",
                "`/wacc` (converted to the terms of the cash flows with `/inflation`)",
            ),
        ];
        for (input, code, pointer) in cases {
            let e = err(&input);
            assert_eq!(e.code(), code, "{e}");
            assert!(e.to_string().contains(pointer), "{pointer}: {e}");
        }
    }

    /// R13, R27: a converted rate keeps its plain JSON pointer in `field`; the inflation it was
    /// converted with is a separate context, and the fixes name it. A converted rate of -1 is
    /// reachable when the quotient underflows the decimal range.
    #[test]
    fn converted_rate_errors_carry_the_inflation_as_context() {
        use crate::FairmarkError;
        let growth = err(&with(base(), |m| {
            m.insert("wacc".into(), json!("0.05"));
            m.insert("inflation".into(), json!("0.03"));
            m.insert(
                "terminal".into(),
                json!({"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}),
            );
        }));
        // The variant within the details the report adds.
        match growth.kind() {
            FairmarkError::GrowthNotBelowTerminalRate {
                rate_field,
                converted_with,
                ..
            } => {
                assert_eq!(rate_field, "/wacc");
                assert_eq!(converted_with.as_deref(), Some("/inflation"));
            }
            other => panic!("{other:?}"),
        }
        assert!(growth.to_string().ends_with(
            "lower the growth or check the rate, or check the expected inflation `/inflation`"
        ));
        // Without conversion the message is unchanged.
        let plain = err(&with(base(), |m| {
            m.insert("discount_rate_terms".into(), json!("real"));
            m.insert("wacc".into(), json!("0.01"));
        }));
        assert_eq!(plain.code(), "FM-E004");
        assert!(!plain.to_string().contains("inflation"), "{plain}");

        // (1 + r) / (1 + i) with 1 + r = 1e-28 and i = 1e10 underflows to 0: rate used -1.
        let tiny = "-0.9999999999999999999999999999";
        let period = err(&with(base(), |m| {
            m.insert("wacc".into(), json!(tiny));
            m.insert("inflation".into(), json!("10000000000"));
        }));
        match &period {
            FairmarkError::DiscountRateOutOfRange {
                field,
                converted_with,
                ..
            } => {
                assert_eq!(field, "/wacc");
                assert_eq!(converted_with.as_deref(), Some("/inflation"));
            }
            other => panic!("{other:?}"),
        }
        assert!(period.to_string().contains(
            "`/wacc` (converted to the terms of the cash flows with `/inflation`) is -1"
        ));
        let terminal = err(&with(base(), |m| {
            m.remove("wacc");
            m.insert("wacc_per_period".into(), json!(["0.1", "0.1", "0.1"]));
            m.insert("terminal_wacc".into(), json!(tiny));
            m.insert("inflation".into(), json!("10000000000"));
        }));
        match &terminal {
            FairmarkError::DiscountRateOutOfRange {
                field,
                converted_with,
                ..
            } => {
                assert_eq!(field, "/terminal_wacc");
                assert_eq!(converted_with.as_deref(), Some("/inflation"));
            }
            other => panic!("{other:?}"),
        }
    }

    /// Contract: an expected inflation at the top of the decimal range overflows in the Fisher
    /// relation and is a typed `FM-E008`, never a panic (also in `tests/contracts`).
    #[test]
    fn inflation_at_the_decimal_maximum_is_an_overflow() {
        for (flows, rates) in [("real", "nominal"), ("nominal", "real")] {
            let e = err(&with(base(), |m| {
                m.insert("cash_flow_terms".into(), json!(flows));
                m.insert("discount_rate_terms".into(), json!(rates));
                m.insert("inflation".into(), json!("79228162514264337593543950335"));
            }));
            assert_eq!(e.code(), "FM-E008", "{e}");
        }
    }

    /// R18: with equal terms, inflation per period is validated but not used, and the
    /// conventions list its values.
    #[test]
    fn equal_terms_report_unused_inflation_per_period() {
        let input = with(base(), |m| {
            m.insert("cash_flow_terms".into(), json!("nominal"));
            m.remove("inflation");
            m.insert(
                "inflation_per_period".into(),
                json!(["0.03", "0.025", "0.02"]),
            );
            m.insert("terminal_inflation".into(), json!("0.02"));
        });
        assert!(conventions(&input).iter().any(|c| c
            == "expected inflation per period (0.03, 0.025, 0.02) validated but not used: the \
                terms are equal, no discount rate is converted (R18); only the terminal \
                inflation 0.02 restates the perpetual growth"));
    }

    /// The conventions of a converted rate say so: the one-rate convention states the terms of
    /// the rate as given, and the technique's rate convention says "converted", not "given".
    #[test]
    fn converted_rate_conventions_say_converted() {
        let c = conventions(&base());
        assert!(c.iter().any(|c| c
            == "discount rate: one rate, 0.10, for every period and for the terminal value (as \
                given, in nominal terms)"));
        let rate = c
            .iter()
            .find(|c| c.starts_with("terminal value (tv.gordon v1): rate of the terminal phase"))
            .unwrap();
        assert!(
            rate.contains("(converted to the terms of the cash flows with `/inflation`;"),
            "{rate}"
        );
        let same = with(base(), |m| {
            m.insert("cash_flow_terms".into(), json!("nominal"));
        });
        let c = conventions(&same);
        assert!(c
            .iter()
            .any(|c| c
                == "discount rate: one rate, 0.10, for every period and for the terminal value"));
        assert!(c.iter().any(|c| c
            .starts_with("terminal value (tv.gordon v1): rate of the terminal phase")
            && c.contains("(given;")));
    }

    /// AC9: the conversion gives exactly the valuation at the real rate given directly when
    /// the conversion is exact.
    #[test]
    fn converted_rate_equals_the_rate_given_directly() {
        let converted = with(base(), |m| {
            m.insert("wacc".into(), json!("0.1016"));
        });
        let direct = with(base(), |m| {
            m.insert("discount_rate_terms".into(), json!("real"));
            m.insert("wacc".into(), json!("0.08"));
            m.remove("inflation");
        });
        let (a, b) = (
            out(&converted)["value"].clone(),
            out(&direct)["value"].clone(),
        );
        assert_eq!(a["value_of_operations"], b["value_of_operations"]);
        assert_eq!(a["terminal"], b["terminal"]);
    }
}
