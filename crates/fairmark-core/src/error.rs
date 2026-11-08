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

    #[error("the IRR solver did not converge after {iterations} iterations")]
    NoConvergence { iterations: u32 },

    #[error("cash flows need at least one sign change to have an IRR")]
    NoSignChange,

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

    #[error("more than one rate sets the present value to zero, in the intervals {0}; the result is ambiguous")]
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
        "the instalment ({instalment}) does not exceed the interest of the first period \
         ({interest}): the outstanding balance never decreases and the loan is never repaid"
    )]
    LoanNeverRepaid {
        instalment: Decimal,
        interest: Decimal,
    },
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
            Self::NoSignChange => "FM-E006",
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
