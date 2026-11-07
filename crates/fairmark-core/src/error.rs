//! Typed errors with stable, documented codes.
//!
//! Codes never change meaning once released: agents and integrations can
//! match on them. The catalogue lives in `docs/reference/errors.md`.

use rust_decimal::Decimal;
use serde::Serialize;

/// Every failure the engine can report. Nothing is ever silently defaulted.
#[derive(Debug, Clone, PartialEq, thiserror::Error)]
pub enum FairmarkError {
    #[error("invalid input: {0}")]
    InvalidInput(String),

    #[error("the cash flow series is empty")]
    EmptyCashFlows,

    #[error("the rate must be greater than -100% (got {0})")]
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
