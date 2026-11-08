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

use crate::error::{FairmarkError, Result};
use crate::explain::{DefaultApplied, Explain};
use crate::method::Method;
use crate::num::{add, div, mul, one_plus, sub};
use crate::rates::{real_nominal, FisherDirection, RealNominal, RealNominalInput};
use crate::tv::TerminalBasis;
use crate::tvm::{period_discount_factors, PeriodTiming};
use crate::types::Timing;

use super::free_cash_flows::{
    build_up, FreeCashFlowLines, FreeCashFlows, Location, TaxChoice, FORMULA_FCFE, FORMULA_FCFF,
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
            "null is not accepted: leave the field out to use the default, or give a value",
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
}

impl Defaulted {
    pub(crate) fn of(defaults: &[DefaultApplied]) -> Self {
        let has = |f: DefaultedField| defaults.iter().any(|d| d.field == f.pointer());
        Self {
            cash_flow_terms: has(DefaultedField::CashFlowTerms),
            discount_rate_terms: has(DefaultedField::DiscountRateTerms),
            timing: has(DefaultedField::Timing),
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
    /// The flow built up from `lines` (`FCFF` or `FCFE`), when it was.
    built_up: Option<&'static str>,
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
        if s.periods == Some(0) {
            return Err(invalid(format!(
                "`{at}/periods` is 0; a stage has at least 1 period"
            )));
        }
        // Decision 24: the projection defaults to `given` only for a stage with flows or lines.
        let Some(projection) = s.projection else {
            if s.flows.is_some() && s.lines.is_some() {
                return Err(invalid(format!(
                    "`{at}/flows` and `{at}/lines` are both given: give `flows` or `lines`, not \
                     both"
                )));
            }
            return Err(invalid(format!(
                "`{at}/projection` is required: give `given` with `flows` or `lines`, \
                 `constant_growth` with `growth`, or `linear_decline` with `from_growth` and \
                 `to_growth`"
            )));
        };
        if projection == Projection::Given {
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
                projection.label()
            )));
        }
        // Decision 24: the periods of a `given` stage default to the number of its flows or
        // lines; a generated stage states them.
        let Some(periods) = s.periods else {
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
            }));
        };
        horizon += u64::from(periods);
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
                )));
            }
            if !given && belongs {
                return Err(invalid(format!(
                    "`{at}/{field}` is required by the projection `{}`",
                    projection.label()
                )));
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
                )));
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
        resolved.push(ResolvedStage {
            periods,
            projection,
            kind,
            built_up: None,
        });
    }
    if horizon > MAX_PERIODS {
        return Err(invalid(format!(
            "`/stages` add up to {horizon} periods; the explicit horizon is at most \
             {MAX_PERIODS} periods"
        )));
    }
    let generated = resolved[0].projection != Projection::Given;
    match (generated, base_flow) {
        (true, None) => Err(count(format!(
            "`/base_flow` is missing: the first stage (`/stages/0`) is `{}` and grows from the \
             base flow CF_0; give `/base_flow`",
            resolved[0].projection.label()
        ))),
        (false, Some(_)) => Err(count(
            "`/base_flow` is given but the first stage (`/stages/0`) is `given`; remove \
             `/base_flow`, or make the first stage a generated one"
                .to_owned(),
        )),
        _ => Ok((resolved, horizon as usize)),
    }
}

/// The period rates and the terminal rate with the JSON pointer of its field, if one is used.
type ResolvedRates = (Vec<Decimal>, Option<(Decimal, String)>);

/// R12, R13: one rate for every period and the terminal value, or one per period plus a
/// terminal rate. Returns the period rates and the terminal rate with its field name; no
/// terminal rate when the terminal value uses none (`rate_use` not used, Decision 24),
/// in which case a separate terminal rate is refused.
fn resolve_rates(r: &RateInput<'_>, n: usize, rate_use: TerminalRateUse) -> Result<ResolvedRates> {
    let uses_terminal_rate = rate_use.is_used();
    let f = &r.fields;
    if !uses_terminal_rate && r.single.is_none() && r.per_period.is_some() && r.terminal.is_some() {
        return Err(invalid(format!(
            "`/{}` is not used: the terminal value takes no terminal rate (no terminal value, or \
             a given terminal value discounted at the horizon N); remove `/{}`",
            f.terminal, f.terminal
        )));
    }
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
        (None, Some(_), None) if uses_terminal_rate => {
            return Err(invalid(format!(
                "`/{}` is required with `/{}`: give the rate of the terminal phase{}",
                f.terminal,
                f.per_period,
                rate_use.note()
            )))
        }
        (Some(rate), None, None) => (vec![rate; n], Some(rate), format!("/{}", f.single)),
        (None, Some(list), term) => {
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
    let terminal = terminal.filter(|_| uses_terminal_rate);
    for (k, rate) in rates.iter().enumerate() {
        if *rate <= -Decimal::ONE {
            let field = match r.single {
                Some(_) => format!("/{}", f.single),
                None => format!("/{}/{k}", f.per_period),
            };
            return Err(FairmarkError::DiscountRateOutOfRange {
                field,
                rate: *rate,
                converted_with: None,
            });
        }
    }
    match terminal {
        Some(t) if t <= -Decimal::ONE => Err(FairmarkError::DiscountRateOutOfRange {
            field: terminal_field,
            rate: t,
            converted_with: None,
        }),
        _ => Ok((rates, terminal.map(|t| (t, terminal_field)))),
    }
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
fn resolve_inflation(
    i: &InflationInput<'_>,
    n: usize,
    rate_use: TerminalRateUse,
) -> Result<Option<Inflation>> {
    let uses_terminal_rate = rate_use.is_used();
    let resolved =
        match (i.single, i.per_period, i.terminal) {
            (None, None, None) => return Ok(None),
            (None, Some(_), Some(_)) if !uses_terminal_rate => {
                return Err(invalid(
                    "`/terminal_inflation` is not used: the terminal value takes no terminal rate \
                 (no terminal value, or a given terminal value discounted at the horizon N); \
                 remove `/terminal_inflation`"
                        .to_owned(),
                ))
            }
            (Some(_), Some(_), _) => {
                return Err(invalid(
                    "give exactly one of `/inflation` and `/inflation_per_period`, not both: one \
                 expected inflation for every period, or one per period with \
                 `/terminal_inflation`"
                        .to_owned(),
                ))
            }
            (Some(_), None, Some(_)) => return Err(invalid(
                "`/terminal_inflation` is not accepted with `/inflation`: one expected inflation \
                 applies to every period and to the terminal phase; remove \
                 `/terminal_inflation`, or give `/inflation_per_period` instead"
                    .to_owned(),
            )),
            (None, Some(_), None) if uses_terminal_rate => {
                return Err(invalid(format!(
                    "`/terminal_inflation` is required with `/inflation_per_period`: give the \
                     expected inflation of the terminal phase{}",
                    rate_use.note()
                )))
            }
            (None, None, Some(_)) => {
                return Err(invalid(
                    "`/terminal_inflation` is used only with `/inflation_per_period`; give \
                 `/inflation_per_period` too, or give one expected inflation in `/inflation`"
                        .to_owned(),
                ))
            }
            (Some(one), None, None) => Inflation {
                per_period: vec![one; n],
                terminal: uses_terminal_rate.then_some(one),
                single: true,
            },
            (None, Some(list), terminal) => {
                if list.len() != n {
                    return Err(count(format!(
                        "`/inflation_per_period` has {} values for an explicit horizon of {n} \
                     periods (the sum of `/stages/*/periods`); give exactly {n} values",
                        list.len()
                    )));
                }
                Inflation {
                    per_period: list.to_vec(),
                    terminal,
                    single: false,
                }
            }
        };
    let values = resolved
        .per_period
        .iter()
        .enumerate()
        .map(|(k, v)| (Some(k), *v))
        .chain(resolved.terminal.map(|t| (None, t)));
    for (k, v) in values {
        if v <= -Decimal::ONE {
            // Decision 19: the code of `rates.real_nominal` for the same value (FM-E003).
            return Err(FairmarkError::DiscountRateOutOfRange {
                field: resolved.field(k),
                rate: v,
                converted_with: None,
            });
        }
    }
    Ok(Some(resolved))
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
        return Err(FairmarkError::TermsMismatch(format!(
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
        )));
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
    let mut per_period = Vec::with_capacity(rates.len());
    for (k, (rate, infl)) in rates.iter().zip(&inflation.per_period).enumerate() {
        let (used, formula) = fisher(*rate, *infl, flows)?;
        if used <= -Decimal::ONE {
            let field = match input.rates.single {
                Some(_) => format!("/{}", f.single),
                None => format!("/{}/{k}", f.per_period),
            };
            return Err(FairmarkError::DiscountRateOutOfRange {
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
        return Err(FairmarkError::DiscountRateOutOfRange {
            field: terminal_field.to_owned(),
            rate: used,
            converted_with: Some(converted_with.as_str().into()),
        });
    }
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
) -> Result<()> {
    match (timing, time) {
        // Decision 24: say so when the timing was not given.
        (Timing::EndOfPeriod, Some(_)) if timing_defaulted => Err(invalid(
            "`/terminal_discount_time` is not accepted with `/timing` = end_of_period: \
             `/timing` was not given and defaults to end_of_period; give `timing: mid_period` \
             or remove `/terminal_discount_time`"
                .to_owned(),
        )),
        (Timing::EndOfPeriod, Some(_)) => Err(invalid(
            "`/terminal_discount_time` is not accepted with `/timing` = end_of_period: the \
             terminal value is then discounted at the horizon N; remove it"
                .to_owned(),
        )),
        (Timing::MidPeriod, Some(_)) if no_terminal_value => Err(invalid(
            "`/terminal_discount_time` is not used: there is no terminal value to discount \
             (`/terminal` is none); remove it, or give a terminal value in `/terminal`"
                .to_owned(),
        )),
        (Timing::MidPeriod, None) if !no_terminal_value => Err(invalid(
            "`/terminal_discount_time` is required with `/timing` = mid_period: give `horizon` \
             (t = N) or `mid_period` (t = N - 0.5)"
                .to_owned(),
        )),
        _ => Ok(()),
    }
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
            let (flows, name) = match basis {
                TerminalBasis::Enterprise => (out.free_cash_flows_to_firm, "FCFF"),
                TerminalBasis::Equity => (
                    out.free_cash_flows_to_equity.ok_or_else(|| {
                        invalid(format!(
                            "`{pointer}/0/interest_expense` and `{pointer}/0/net_borrowing` are \
                             missing: the equity basis discounts FCFE, which needs the interest \
                             expense and the net borrowing in every line of `{pointer}`"
                        ))
                    })?,
                    "FCFE",
                ),
            };
            s.kind = StageKind::Given(Cow::Owned(flows));
            s.built_up = Some(name);
        }
        first += s.periods;
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
    // The terminal option is read first, because it decides whether a terminal rate is used
    // (Decision 24); an error in it is reported where the technique is checked, as before.
    let choice = terminal::choose(input.terminal);
    let rate_use = match &choice {
        Ok(c) => c.terminal_rate_use(input.timing, input.terminal_discount_time),
        Err(_) => TerminalRateUse::Technique,
    };
    let no_terminal_value = matches!(choice, Ok(TerminalChoice::None));
    let (given_rates, terminal_given) = resolve_rates(&input.rates, n, rate_use)?;
    let inflation = resolve_inflation(&input.inflation, n, rate_use)?;
    let used = resolve_terms(
        ex,
        input,
        &given_rates,
        terminal_given.as_ref().map(|(r, f)| (*r, f.as_str())),
        inflation.as_ref(),
    )?;
    check_timing(
        input.timing,
        input.terminal_discount_time,
        no_terminal_value,
        input.defaulted.timing,
    )?;
    let terminal_rate = used.terminal;
    let choice = terminal::check(
        choice?,
        terminal_rate,
        &used.terminal_field,
        used.terminal_converted_with.as_deref(),
        input.basis,
    )?;
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
    Ok(EngineOutput {
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
    })
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
        match &growth {
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
