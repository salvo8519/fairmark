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

    /// A nominal rate whose rate per compounding period is −100% or lower, named by its JSON
    /// pointer (issue #174). With one compounding period a year it is `DiscountRateOutOfRange`.
    #[error(
        "`{field}` is {rate}: with {periods_per_year} compounding periods a year, the rate per \
         compounding period ({rate} / {periods_per_year}) must be greater than -1 (-100%), so \
         the rate must be greater than -{periods_per_year}; rates are decimal fractions, 10% is \
         0.10"
    )]
    CompoundedRateOutOfRange {
        /// JSON pointer of the rate as given.
        field: String,
        rate: Decimal,
        /// Compounding periods a year, 2 or more.
        periods_per_year: u32,
    },

    /// A `guess` of `tvm.irr` or `tvm.xirr` outside the domain of the rate (issue #190): the
    /// rule of the rate, then how to correct the guess.
    #[error(
        "{}; give a `guess` near the expected rate, or leave out `guess` to use the default 0.10",
        guess_rule(*.guess, *.periods_per_year)
    )]
    GuessOutOfRange {
        guess: Decimal,
        /// Compounding periods a year of the rate, 1 for `tvm.irr`.
        periods_per_year: u32,
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

    /// Two sides of a relation of the financial statements differ (SPEC-0005 R7, R10, R12): a
    /// total and its components, a split and its aggregate, an adjustment and what remains in
    /// its source. Each side is named with its JSON pointers; the comparison is exact. Boxed,
    /// so that every `Result` of the engine stays small.
    #[error(
        "statements do not reconcile: {} is {}, while {} is {} (difference {}); {}",
        .0.left_name, .0.left, .0.right_name, .0.right, .0.difference, .0.fix
    )]
    StatementsDoNotReconcile(Box<Reconciliation>),
}

/// The two sides of a relation of the financial statements that differ (`FM-E026`).
#[derive(Debug, Clone, PartialEq)]
pub struct Reconciliation {
    /// The first side, with its JSON pointers and the year, for example "total B, fixed
    /// assets (`/years/0/statutory/balance_sheet/fixed_assets`, 2024)".
    pub left_name: String,
    pub left: Decimal,
    /// The second side, with the pointers of its components.
    pub right_name: String,
    pub right: Decimal,
    /// `left − right`.
    pub difference: Decimal,
    /// How to correct the input.
    pub fix: String,
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

/// The rule a guess breaks, worded as the `FM-E003` of any other rate named `/guess`.
fn guess_rule(guess: Decimal, periods_per_year: u32) -> String {
    rate_field("/guess", Some(periods_per_year))(FairmarkError::RateOutOfRange(guess)).to_string()
}

/// Names the rate field in an `FM-E003` raised by a shared helper, which knows only the value
/// (issues #174, #176): the method knows the field and the compounding of the rate.
/// `periods_per_year` is that of the rate's compounding: `None` (continuous) or `Some(1)` gives
/// the rule for a rate per period, more periods the rule per compounding period. Every other
/// error passes unchanged.
pub(crate) fn rate_field(
    field: &str,
    periods_per_year: Option<u32>,
) -> impl Fn(FairmarkError) -> FairmarkError + '_ {
    move |e| match e {
        FairmarkError::RateOutOfRange(rate) => match periods_per_year {
            Some(m) if m > 1 => FairmarkError::CompoundedRateOutOfRange {
                field: field.to_owned(),
                rate,
                periods_per_year: m,
            },
            _ => FairmarkError::DiscountRateOutOfRange {
                field: field.to_owned(),
                rate,
                converted_with: None,
            },
        },
        other => other,
    }
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
            Self::CompoundedRateOutOfRange { .. } => "FM-E003",
            Self::GuessOutOfRange { .. } => "FM-E003",
            Self::GrowthNotBelowTerminalRate { .. } => "FM-E004",
            // Same meanings as `InvalidInput` and `InvalidDate`, with the JSON pointer of the
            // field that could not be read (issue #174).
            Self::InvalidInputAt { .. } => "FM-E001",
            Self::InvalidDateAt { .. } => "FM-E009",
            Self::TermsMismatch(_) => "FM-E024",
            Self::CountMismatch(_) => "FM-E025",
            Self::StatementsDoNotReconcile(_) => "FM-E026",
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

#[cfg(test)]
mod tests {
    use super::*;

    /// SPEC-0005 R24: `FM-E026` names both sides, both amounts, the difference and the fix.
    #[test]
    fn statements_do_not_reconcile_is_fm_e026_with_both_sides() {
        let e = FairmarkError::StatementsDoNotReconcile(Box::new(Reconciliation {
            left_name: "`/years/0/statutory/balance_sheet/fixed_assets` (total B, 2024)".into(),
            left: Decimal::from(1_170_001),
            right_name: "B.I + B.II + B.III".into(),
            right: Decimal::from(1_170_000),
            difference: Decimal::ONE,
            fix: "correct the total or the items".into(),
        }));
        assert_eq!(e.code(), "FM-E026");
        assert_eq!(
            e.to_string(),
            "statements do not reconcile: `/years/0/statutory/balance_sheet/fixed_assets` \
             (total B, 2024) is 1170001, while B.I + B.II + B.III is 1170000 (difference 1); \
             correct the total or the items"
        );
    }
}
