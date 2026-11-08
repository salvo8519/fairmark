//! Typed errors with stable, documented codes.
//!
//! Codes never change meaning once released: agents and integrations can
//! match on them. The catalogue lives in `docs/en/reference/errors.md`.

use rust_decimal::Decimal;
use serde::Serialize;

/// Every failure the engine can report. Nothing is ever silently defaulted.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FairmarkError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("the cash flow series is empty")]
    EmptyCashFlows,

    #[error("rate out of range (got {0}): one plus the rate per period, or per compounding period, must be positive")]
    RateOutOfRange(Decimal),

    #[error("terminal growth ({growth}) must be lower than the discount rate ({rate})")]
    GrowthNotBelowRate { growth: Decimal, rate: Decimal },

    #[error(
        "the {solver} solver did not converge after {iterations} iterations: the rate may lie \
         outside the decimal range or far from `guess`; give a `guess` near the expected rate"
    )]
    NoConvergence {
        /// `IRR` or `XIRR`.
        solver: &'static str,
        iterations: u32,
    },

    #[error(
        "`cash_flows` needs at least one positive and one negative amount{}",
        if *netted { " after flows on the same date are netted" } else { "" }
    )]
    NoSignChange {
        /// True when the check is made after netting flows at the same time (XIRR).
        netted: bool,
    },

    #[error("unknown method '{0}'")]
    UnknownMethod(String),

    #[error("arithmetic overflow while computing {0}")]
    Overflow(&'static str),

    #[error("invalid date: {0}")]
    InvalidDate(String),

    #[error("a cash flow dated {date} precedes the valuation date {valuation_date}")]
    FlowBeforeValuationDate {
        date: String,
        valuation_date: String,
    },

    #[error(
        "no single rate can be isolated: {0}; use `tvm.npv` at the cost of capital or \
         `tvm.mirr` with explicit rates"
    )]
    MultipleRates(String),

    #[error(
        "internal error ({0}); this is a defect in Fairmark, please report it with the input hash"
    )]
    Internal(&'static str),

    #[error("the target is not bracketed: {0}")]
    TargetNotBracketed(String),

    #[error("goal seek did not reach the tolerance after {iterations} iterations: {detail}")]
    GoalSeekNoConvergence { iterations: u32, detail: String },

    #[error("invalid analysis pointer '{pointer}': {reason}")]
    InvalidPointer { pointer: String, reason: String },

    #[error("evaluation limit exceeded: {0}")]
    EvaluationLimit(String),

    #[error("'{0}' is an analysis method; an analysis cannot evaluate another analysis")]
    NestedAnalysis(String),

    #[error("terminal value input outside its domain: {0}")]
    TerminalValueDomain(String),

    #[error("enterprise and equity measures are mixed: {0}")]
    MixedBasis(String),

    #[error("balloon amount out of range: {0}")]
    BalloonOutOfRange(String),

    #[error(
        "`instalment` ({instalment}) never repays the loan: it must be greater than 0 and \
         than the interest of the first period ({interest}), which is periodic rate × principal \
         for payments in arrears and periodic rate × principal / (1 + periodic rate) for \
         payments in advance; raise `instalment`"
    )]
    LoanNeverRepaid {
        instalment: Decimal,
        interest: Decimal,
    },

    #[error("principal grace period out of range: {0}")]
    GracePeriodOutOfRange(String),

    #[error("rounding too coarse for the loan: {0}")]
    RoundingTooCoarse(String),

    #[error(
        "`{field}`{} is {rate}: a rate must be greater than -1 (-100%); rates are decimal \
         fractions, 10% is 0.10{}",
        converted_note(.converted_with),
        inflation_fix(.converted_with)
    )]
    DiscountRateOutOfRange {
        /// JSON pointer of the rate as given.
        field: String,
        rate: Decimal,
        /// JSON pointer of the expected inflation the rate was converted with, if it was
        /// (SPEC-0001 R16); `None` for a rate used as given.
        converted_with: Option<Box<str>>,
    },

    #[error(
        "`{growth_field}` ({growth}) must be lower than the discount rate `{rate_field}`{} \
         ({rate}): a perpetual growth at or above the rate has no finite value; lower the \
         growth or check the rate{}",
        converted_note(.converted_with),
        inflation_fix(.converted_with)
    )]
    GrowthNotBelowTerminalRate {
        growth_field: String,
        growth: Decimal,
        /// JSON pointer of the rate as given.
        rate_field: String,
        rate: Decimal,
        /// JSON pointer of the expected inflation the rate was converted with, if it was
        /// (SPEC-0001 R16); `None` for a rate used as given.
        converted_with: Option<Box<str>>,
    },

    /// An input that cannot be read, located by the JSON pointer of the field (issue #174):
    /// a missing or unknown field, a wrong type or an unknown enumeration value.
    #[error("invalid input at `{pointer}`: {detail}")]
    InvalidInputAt {
        /// JSON pointer (RFC 6901) of the field, never empty.
        pointer: String,
        /// What is wrong, the values expected where they are known, and the fix.
        detail: String,
    },

    /// A date that cannot be read, located by the JSON pointer of the field (issue #174).
    #[error("invalid date at `{pointer}`: {detail}")]
    InvalidDateAt {
        /// JSON pointer (RFC 6901) of the date, never empty.
        pointer: String,
        /// What is wrong with the date.
        detail: String,
    },

    #[error("terms of cash flows and discount rates differ: {0}")]
    TermsMismatch(String),

    #[error("count inconsistent with the stages: {0}")]
    CountMismatch(String),
}

/// " (converted with `/inflation`)" when the rate was converted with expected inflation.
fn converted_note(converted_with: &Option<Box<str>>) -> String {
    converted_with
        .as_ref()
        .map(|p| format!(" (converted to the terms of the cash flows with `{p}`)"))
        .unwrap_or_default()
}

/// The fix that applies to a converted rate: the expected inflation may be the wrong input.
fn inflation_fix(converted_with: &Option<Box<str>>) -> String {
    converted_with
        .as_ref()
        .map(|p| format!(", or check the expected inflation `{p}`"))
        .unwrap_or_default()
}

impl FairmarkError {
    /// Stable machine-readable code, e.g. `FM-E004`.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "FM-E001",
            Self::EmptyCashFlows => "FM-E002",
            Self::RateOutOfRange(_) => "FM-E003",
            Self::GrowthNotBelowRate { .. } => "FM-E004",
            Self::NoConvergence { .. } => "FM-E005",
            Self::NoSignChange { .. } => "FM-E006",
            Self::UnknownMethod(_) => "FM-E007",
            Self::Overflow(_) => "FM-E008",
            Self::InvalidDate(_) => "FM-E009",
            Self::FlowBeforeValuationDate { .. } => "FM-E010",
            Self::MultipleRates(_) => "FM-E011",
            Self::Internal(_) => "FM-E012",
            Self::TargetNotBracketed(_) => "FM-E013",
            Self::GoalSeekNoConvergence { .. } => "FM-E014",
            Self::InvalidPointer { .. } => "FM-E015",
            Self::EvaluationLimit(_) => "FM-E016",
            Self::NestedAnalysis(_) => "FM-E017",
            Self::TerminalValueDomain(_) => "FM-E018",
            Self::MixedBasis(_) => "FM-E019",
            Self::BalloonOutOfRange(_) => "FM-E020",
            Self::LoanNeverRepaid { .. } => "FM-E021",
            Self::GracePeriodOutOfRange(_) => "FM-E022",
            Self::RoundingTooCoarse(_) => "FM-E023",
            // Same meanings as `RateOutOfRange` and `GrowthNotBelowRate`, with the JSON pointer
            // of the field in the message (SPEC-0001 R27).
            Self::DiscountRateOutOfRange { .. } => "FM-E003",
            Self::GrowthNotBelowTerminalRate { .. } => "FM-E004",
            // Same meanings as `InvalidInput` and `InvalidDate`, with the JSON pointer of the
            // field that could not be read (issue #174).
            Self::InvalidInputAt { .. } => "FM-E001",
            Self::InvalidDateAt { .. } => "FM-E009",
            Self::TermsMismatch(_) => "FM-E024",
            Self::CountMismatch(_) => "FM-E025",
        }
    }

    /// JSON-friendly representation used by every adapter (CLI, Python, MCP).
    pub fn to_report(&self) -> ErrorReport {
        ErrorReport {
            code: self.code(),
            message: self.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ErrorReport {
    pub code: &'static str,
    pub message: String,
}

pub type Result<T> = std::result::Result<T, FairmarkError>;
