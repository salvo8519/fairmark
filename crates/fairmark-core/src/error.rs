//! Typed errors with stable, documented codes.
//!
//! Codes never change meaning once released: agents and integrations can
//! match on them. The catalogue lives in `docs/en/reference/errors.md`.

use rust_decimal::Decimal;
use schemars::JsonSchema;
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

    /// The input is below the minimum of the method (ADR 0014 G10.1, SPEC-0007 R12): no output
    /// has every input it needs, so the result would be made of nulls only. The message lists
    /// the alternative sets of inputs, any one of which lifts the input above the floor. Boxed,
    /// so that every `Result` of the engine stays small.
    #[error(
        "below the minimum input of `{}`: no output has every input it needs; give at least one \
         of these sets of inputs: {}; the method choices alone compute nothing",
        .0.method,
        .0.sets()
    )]
    BelowMinimumInput(Box<MinimumInput>),

    /// An error with the details of its report (ADR 0014 G5): the field to correct, the
    /// sub-kind, the accepted values and the fix. The code and the message are those of the
    /// error it carries, unchanged; only the report gains fields. Built with
    /// [`FairmarkError::at`] and the other builders, never directly.
    #[error("{}", .0.error)]
    Detailed(Box<Detailed>),

    /// Every input error a validation found, in the order it checks the input (ADR 0014 G5).
    /// The code and the message are those of the first, so a client that reads only `code`
    /// and `message` sees what it saw before; the report lists all of them in `errors`. Built
    /// only by [`Found`], with two or more errors.
    #[error("{}", .0.first())]
    Several(Box<Several>),
}

/// An error and the details of its report (`FairmarkError::Detailed`).
#[derive(Debug, Clone, PartialEq)]
pub struct Detailed {
    /// The error, which gives the code and the message.
    pub error: FairmarkError,
    /// What the report adds.
    pub details: Details,
}

/// Two or more input errors found by one validation (`FairmarkError::Several`).
#[derive(Debug, Clone, PartialEq)]
pub struct Several(Vec<FairmarkError>);

impl Several {
    /// The first error found: its code and message are those of the whole.
    pub fn first(&self) -> &FairmarkError {
        &self.0[0]
    }

    /// Every error found, in the order of the validation.
    pub fn errors(&self) -> &[FairmarkError] {
        &self.0
    }
}

/// The sub-kind of an error (ADR 0014 G5): it tells apart the conditions that share a code,
/// above all `FM-E001`, without new codes. Values are added, never renamed or removed; a
/// client accepts a value it does not know.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[schemars(rename = "FairmarkErrorReason")]
pub enum Reason {
    /// The text is not JSON (a syntax error, a truncated input, trailing characters): there
    /// is no field to name, and the message keeps the line and column.
    Syntax,
    /// A required field, or one required by the other fields given, is absent.
    Missing,
    /// A field the input does not define.
    UnknownField,
    /// A field given twice in the same object.
    Duplicate,
    /// A value of the wrong type: a text where a number is expected, an object where a list
    /// is expected, a date that is not a calendar date.
    Type,
    /// A value outside a closed list of values; `expected` lists them.
    Enum,
    /// Fields that exclude each other are given together, or contradict each other.
    Conflict,
    /// A field is given without the field it needs, or is not used by the choice made.
    Requires,
    /// An amount with the wrong sign for its field.
    Sign,
    /// A text that must say something is empty or blank.
    Blank,
    /// A number outside the domain of its field.
    Range,
    /// A list with the wrong number of entries: empty, too long, or not one entry per period
    /// or year.
    Length,
    /// Two sides of a relation that must agree differ: a total and its parts (`FM-E026`).
    Reconcile,
}

/// What an error report says beyond its code and message (ADR 0014 G5). Every field is
/// optional: a report says what is known about the error, nothing more.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Details {
    /// JSON pointer (RFC 6901) of the field to correct, from the root of the input.
    pub pointer: Option<String>,
    /// JSON pointers of every field of a relation: both sides of a reconciliation, both
    /// fields of a conflict. Empty when the error concerns one field.
    pub pointers: Vec<String>,
    /// The sub-kind of the error.
    pub reason: Option<Reason>,
    /// The accepted values, or a short description of the accepted form.
    pub expected: Vec<String>,
    /// One sentence that resolves the error.
    pub fix: Option<String>,
    /// `FM-E027` only: alternative sets of input pointers, any one of which is enough.
    pub missing_any_of: Vec<Vec<String>>,
}

/// Collects the input errors of a validation pass (ADR 0014 G5): the validation goes on after
/// a refusal, so that the report lists every input error it can find, in the order it checks
/// them. A computation error met during the pass (an overflow, a defect) is kept apart: it is
/// reported, alone, only when the input has no error.
#[derive(Debug, Default)]
pub(crate) struct Found {
    input: Vec<FairmarkError>,
    computation: Option<FairmarkError>,
}

impl Found {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    /// Records the error of a check, if any; returns true when the check passed.
    pub(crate) fn check(&mut self, result: Result<()>) -> bool {
        self.take(result).is_some()
    }

    /// The value of a check that passed, or `None` after recording its error.
    pub(crate) fn take<T>(&mut self, result: Result<T>) -> Option<T> {
        match result {
            Ok(v) => Some(v),
            Err(e) => {
                self.push(e);
                None
            }
        }
    }

    /// Records an error; the errors of a nested validation are kept in their order.
    pub(crate) fn push(&mut self, error: FairmarkError) {
        match error {
            FairmarkError::Several(several) => several.0.into_iter().for_each(|e| self.push(e)),
            e if e.is_input_error() => self.input.push(e),
            e => {
                self.computation.get_or_insert(e);
            }
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.input.is_empty() && self.computation.is_none()
    }

    /// The error of a validation that cannot go on: what was found, never empty when called
    /// after a refusal (an empty collection is a defect, `FM-E012`).
    pub(crate) fn into_error(self) -> FairmarkError {
        match self.finish() {
            Err(e) => e,
            Ok(()) => FairmarkError::Internal("a refused check recorded no error"),
        }
    }

    /// `Ok` when nothing was found; the input errors when there are any (one, or `Several`);
    /// otherwise the computation error, alone.
    pub(crate) fn finish(mut self) -> Result<()> {
        match self.input.len() {
            0 => self.computation.map_or(Ok(()), Err),
            1 => Err(self.input.remove(0)),
            _ => Err(FairmarkError::Several(Box::new(Several(self.input)))),
        }
    }
}

/// What an input below the floor lacks (`FM-E027`).
#[derive(Debug, Clone, PartialEq)]
pub struct MinimumInput {
    /// The method id, for example `ratios.liquidity`.
    pub method: &'static str,
    /// Alternative sets of JSON pointers of the input, shortest first; giving every input of
    /// any one set lifts the input above the floor.
    pub missing_any_of: Vec<Vec<String>>,
}

impl MinimumInput {
    /// "[`/a`, `/b`] or [`/c`]".
    fn sets(&self) -> String {
        self.missing_any_of
            .iter()
            .map(|set| {
                let names: Vec<String> = set.iter().map(|p| format!("`{p}`")).collect();
                format!("[{}]", names.join(", "))
            })
            .collect::<Vec<_>>()
            .join(" or ")
    }
}

/// The two sides of a relation of the financial statements that differ (`FM-E026`).
#[derive(Debug, Clone, PartialEq)]
pub struct Reconciliation {
    /// The first side, with its JSON pointers and the year, for example "total B, fixed
    /// assets (`/years/0/statutory/balance_sheet/fixed_assets`, 2024)".
    pub left_name: String,
    /// The amount of the first side, in the currency units of the statements.
    pub left: Decimal,
    /// The second side, with the pointers of its components.
    pub right_name: String,
    /// The amount of the second side, in the currency units of the statements.
    pub right: Decimal,
    /// `left − right`, in the currency units of the statements.
    pub difference: Decimal,
    /// How to correct the input.
    pub fix: String,
    /// JSON pointers of the fields of both sides, the first side first (the error report's
    /// `pointers`).
    pub pointers: Vec<String>,
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
            Self::BelowMinimumInput(_) => "FM-E027",
            // Carriers: the code of the error carried, of the first error found.
            Self::Detailed(d) => d.error.code(),
            Self::Several(s) => s.first().code(),
        }
    }

    /// True for an error of the input; false for an error of the computation (a solver that
    /// does not converge or cannot isolate a rate, an overflow, a defect), which never sits in
    /// `errors` next to input errors. The size of an analysis request (`FM-E016`) is a
    /// condition of the input, checked before any evaluation.
    pub fn is_input_error(&self) -> bool {
        !matches!(
            self.kind(),
            Self::NoConvergence { .. }
                | Self::Overflow(_)
                | Self::MultipleRates(_)
                | Self::Internal(_)
                | Self::TargetNotBracketed(_)
                | Self::GoalSeekNoConvergence { .. }
        )
    }

    /// This error with the JSON pointer of the field to correct and the sub-kind; the code and
    /// the message are unchanged (ADR 0014 G5).
    pub(crate) fn at(self, pointer: impl Into<String>, reason: Reason) -> Self {
        self.with(|d| {
            d.pointer = Some(pointer.into());
            d.reason = Some(reason);
        })
    }

    /// This error with the JSON pointer of the field to correct, unless it already has one.
    pub(crate) fn located(self, pointer: impl Into<String>) -> Self {
        if self.details().pointer.is_some() {
            return self;
        }
        let pointer = pointer.into();
        self.with(|d| d.pointer = Some(pointer))
    }

    /// This error with its sub-kind.
    pub(crate) fn because(self, reason: Reason) -> Self {
        self.with(|d| d.reason = Some(reason))
    }

    /// This error with the pointers of every field of the relation it breaks; the first is
    /// also the field to correct when none is set.
    pub(crate) fn between<S: Into<String>>(self, pointers: impl IntoIterator<Item = S>) -> Self {
        let pointers: Vec<String> = pointers.into_iter().map(Into::into).collect();
        self.with(|d| {
            if d.pointer.is_none() {
                d.pointer = pointers.first().cloned();
            }
            d.pointers = pointers;
        })
    }

    /// This error with the accepted values, or a short description of the accepted form.
    pub(crate) fn expected<S: Into<String>>(self, values: impl IntoIterator<Item = S>) -> Self {
        let values: Vec<String> = values.into_iter().map(Into::into).collect();
        self.with(|d| d.expected = values)
    }

    /// This error with the sentence that resolves it.
    pub(crate) fn fix(self, fix: impl Into<String>) -> Self {
        let fix = fix.into();
        self.with(|d| d.fix = Some(fix))
    }

    /// Applies `f` to the details of this error, which becomes `Detailed` if it was not.
    fn with(self, f: impl FnOnce(&mut Details)) -> Self {
        match self {
            Self::Detailed(mut d) => {
                f(&mut d.details);
                Self::Detailed(d)
            }
            // Several errors have no details of their own: the details go to the first, whose
            // code and message are those of the whole.
            Self::Several(mut several) => {
                let first = std::mem::replace(&mut several.0[0], Self::Internal("moved"));
                several.0[0] = first.with(f);
                Self::Several(several)
            }
            error => {
                let mut details = error.details();
                f(&mut details);
                Self::Detailed(Box::new(Detailed { error, details }))
            }
        }
    }

    /// This error located from `prefix`: its pointer, pointers and alternative sets gain the
    /// prefix, its message is unchanged. An error of the method evaluated by an analysis is
    /// located under `/base_input`, so that every pointer of a report starts at the root of
    /// the input the client sent.
    pub(crate) fn under(self, prefix: &str) -> Self {
        self.map_pointers(&|p: &str| format!("{prefix}{p}"))
    }

    /// This error with every pointer of its report (pointer, pointers, alternative sets, and
    /// those of every error it carries) rewritten by `f`; its message is unchanged.
    pub(crate) fn map_pointers(self, f: &dyn Fn(&str) -> String) -> Self {
        match self {
            Self::Several(several) => Self::Several(Box::new(Several(
                several.0.into_iter().map(|e| e.map_pointers(f)).collect(),
            ))),
            error => {
                let d = error.details();
                if d.pointer.is_none() && d.pointers.is_empty() && d.missing_any_of.is_empty() {
                    return error;
                }
                let at = |p: &mut String| *p = f(p);
                error.with(|d| {
                    d.pointer.iter_mut().for_each(at);
                    d.pointers.iter_mut().for_each(at);
                    d.missing_any_of.iter_mut().flatten().for_each(at);
                })
            }
        }
    }

    /// Every error this one carries: the errors of `Several`, or the error itself.
    pub(crate) fn into_errors(self) -> Vec<Self> {
        match self {
            Self::Several(s) => s.0,
            other => vec![other],
        }
    }

    /// The variant that gives the code and the message: the error carried by `Detailed`, the
    /// first of `Several`, the error itself otherwise. For a caller that matches a condition.
    pub fn kind(&self) -> &Self {
        match self {
            Self::Detailed(d) => d.error.kind(),
            Self::Several(s) => s.first().kind(),
            other => other,
        }
    }

    /// This error without the details set by the builders: the variant that gives its code
    /// and message, for a caller that rewrites the message and locates it again.
    pub(crate) fn plain(self) -> Self {
        match self {
            Self::Detailed(d) => d.error,
            other => other,
        }
    }

    /// The details of the report: those set by the builders, or those the variant carries.
    pub fn details(&self) -> Details {
        let range = Reason::Range;
        match self {
            Self::Detailed(d) => d.details.clone(),
            Self::Several(s) => s.first().details(),
            Self::RateOutOfRange(_) => Details {
                reason: Some(range),
                expected: vec!["a rate greater than -1 (-100%)".to_owned()],
                ..Details::default()
            },
            Self::DiscountRateOutOfRange {
                field,
                converted_with,
                ..
            } => Details {
                pointer: Some(field.clone()),
                pointers: converted_with
                    .as_ref()
                    .map(|c| vec![field.clone(), c.to_string()])
                    .unwrap_or_default(),
                reason: Some(range),
                expected: vec!["a rate greater than -1 (-100%)".to_owned()],
                fix: Some(match converted_with {
                    Some(c) => format!(
                        "give `{field}` as a decimal fraction greater than -1, or check the \
                         expected inflation `{c}`"
                    ),
                    None => format!("give `{field}` as a decimal fraction greater than -1"),
                }),
                ..Details::default()
            },
            Self::CompoundedRateOutOfRange {
                field,
                periods_per_year,
                ..
            } => Details {
                pointer: Some(field.clone()),
                reason: Some(range),
                expected: vec![format!("a rate greater than -{periods_per_year}")],
                fix: Some(format!(
                    "give `{field}` as a decimal fraction greater than -{periods_per_year}"
                )),
                ..Details::default()
            },
            Self::GuessOutOfRange {
                periods_per_year, ..
            } => Details {
                pointer: Some("/guess".to_owned()),
                reason: Some(range),
                expected: vec![format!("a rate greater than -{periods_per_year}")],
                fix: Some("give a `guess` near the expected rate, or leave out `guess`".to_owned()),
                ..Details::default()
            },
            Self::GrowthNotBelowRate { .. } => Details {
                reason: Some(range),
                expected: vec!["a growth lower than the discount rate".to_owned()],
                ..Details::default()
            },
            Self::GrowthNotBelowTerminalRate {
                growth_field,
                rate_field,
                converted_with,
                ..
            } => {
                let mut pointers = vec![growth_field.clone(), rate_field.clone()];
                pointers.extend(converted_with.as_ref().map(|c| c.to_string()));
                Details {
                    pointer: Some(growth_field.clone()),
                    pointers,
                    reason: Some(range),
                    expected: vec![format!("a growth lower than `{rate_field}`")],
                    fix: Some(format!(
                        "lower `{growth_field}` below the discount rate, or check `{rate_field}`"
                    )),
                    ..Details::default()
                }
            }
            Self::UnknownMethod(_) => Details {
                reason: Some(Reason::Enum),
                expected: crate::registry::METHODS
                    .iter()
                    .map(|m| m.id.to_owned())
                    .collect(),
                ..Details::default()
            },
            Self::EmptyCashFlows => Details {
                pointer: Some("/cash_flows".to_owned()),
                reason: Some(Reason::Length),
                fix: Some("give at least one cash flow".to_owned()),
                ..Details::default()
            },
            Self::NoSignChange { .. } => Details {
                pointer: Some("/cash_flows".to_owned()),
                reason: Some(Reason::Sign),
                fix: Some("give at least one positive and one negative cash flow".to_owned()),
                ..Details::default()
            },
            Self::InvalidInputAt { pointer, .. } => Details {
                pointer: Some(pointer.clone()),
                ..Details::default()
            },
            Self::InvalidDateAt { pointer, .. } => Details {
                pointer: Some(pointer.clone()),
                reason: Some(Reason::Type),
                expected: vec!["an ISO 8601 calendar date, YYYY-MM-DD".to_owned()],
                ..Details::default()
            },
            Self::InvalidDate(_) => Details {
                reason: Some(Reason::Type),
                expected: vec!["an ISO 8601 calendar date, YYYY-MM-DD".to_owned()],
                ..Details::default()
            },
            Self::StatementsDoNotReconcile(r) => Details {
                pointer: r.pointers.first().cloned(),
                pointers: r.pointers.clone(),
                reason: Some(Reason::Reconcile),
                fix: Some(r.fix.clone()),
                ..Details::default()
            },
            Self::BelowMinimumInput(m) => Details {
                missing_any_of: m.missing_any_of.clone(),
                reason: Some(Reason::Missing),
                fix: Some(
                    "give every input of at least one of the sets in `missing_any_of`".to_owned(),
                ),
                ..Details::default()
            },
            Self::LoanNeverRepaid { .. } => Details {
                pointer: Some("/instalment".to_owned()),
                reason: Some(range),
                fix: Some("raise `instalment` above the interest of the first period".to_owned()),
                ..Details::default()
            },
            // The rounding chosen is too coarse for the loan: an input condition, found while the
            // schedule is computed (FM-E023).
            Self::RoundingTooCoarse(_) => Details {
                pointer: Some("/rounding/decimal_places".to_owned()),
                reason: Some(range),
                fix: Some("round to more decimal places".to_owned()),
                ..Details::default()
            },
            Self::TerminalValueDomain(_)
            | Self::BalloonOutOfRange(_)
            | Self::GracePeriodOutOfRange(_) => Details {
                reason: Some(range),
                ..Details::default()
            },
            Self::MixedBasis(_) | Self::TermsMismatch(_) => Details {
                reason: Some(Reason::Conflict),
                ..Details::default()
            },
            Self::CountMismatch(_) => Details {
                reason: Some(Reason::Length),
                ..Details::default()
            },
            _ => Details::default(),
        }
    }

    /// The report of this error alone, without `errors`.
    fn entry(&self) -> ErrorEntry {
        let Details {
            pointer,
            pointers,
            reason,
            expected,
            fix,
            missing_any_of,
        } = self.details();
        ErrorEntry {
            code: self.code(),
            message: self.to_string(),
            pointer,
            pointers,
            reason,
            expected,
            fix,
            missing_any_of,
        }
    }

    /// JSON-friendly representation used by every adapter (CLI, Python, MCP): the first
    /// error at the top level, every error found in `errors` (ADR 0014 G5).
    pub fn to_report(&self) -> ErrorReport {
        let errors: Vec<ErrorEntry> = match self {
            Self::Several(s) => s.errors().iter().map(Self::entry).collect(),
            other => vec![other.entry()],
        };
        ErrorReport {
            first: errors[0].clone(),
            errors,
        }
    }

    /// The code and the message only: the error recorded in a cell of an analysis table,
    /// whose shape is part of the output of the analysis methods.
    pub fn to_compact_report(&self) -> CompactErrorReport {
        CompactErrorReport {
            code: self.code(),
            message: self.to_string(),
        }
    }
}

/// One error of a report (ADR 0014 G5): the stable code, the message in English, and what a
/// client needs to correct the input without reading the message.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(rename = "FairmarkErrorEntry")]
pub struct ErrorEntry {
    /// Stable error code, for example `FM-E004` (see the errors reference).
    pub code: &'static str,
    /// What is wrong and, where known, how to fix it.
    pub message: String,
    /// JSON pointer (RFC 6901) of the field to correct, always from the root of the input the
    /// client sent: an error of the method evaluated by an analysis is under `/base_input`,
    /// while its message keeps the pointer within `base_input`. `null` only for a text that is
    /// not JSON (`reason` `syntax`) and for an error raised during the computation that no
    /// single field causes (and for `FM-E027`, which names its inputs in `missing_any_of`).
    pub pointer: Option<String>,
    /// JSON pointers of every field of a relation (both sides of an `FM-E026`
    /// reconciliation, both fields of a conflict); empty when the error concerns one field.
    pub pointers: Vec<String>,
    /// The sub-kind of the error, which tells apart the conditions that share a code; `null`
    /// when the code has one meaning only.
    pub reason: Option<Reason>,
    /// What the field accepts. With `reason` `enum` or `unknown_field`, the literal values
    /// (the values of the enumeration, the fields of the object); with any other reason, short
    /// descriptions of the accepted form, for example "an amount greater than 0". Empty when
    /// not known.
    pub expected: Vec<String>,
    /// One sentence that resolves the error; `null` when the message is the only guidance.
    /// Its wording may change between releases; `code`, `pointer` and `reason` do not.
    pub fix: Option<String>,
    /// `FM-E027` only: alternative sets of input pointers, any one of which lifts the input
    /// above the minimum of the method. Omitted for every other code.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub missing_any_of: Vec<Vec<String>>,
}

/// The report of an error (ADR 0014 G5): the first error found, and every error found.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(rename = "FairmarkErrorReport")]
pub struct ErrorReport {
    /// The first error found; its code and message are those of the whole report.
    #[serde(flatten)]
    pub first: ErrorEntry,
    /// Every error found, at least one, the first equal to the top level: the validation of
    /// the input reports all the errors it finds, in the order of its checks; an error raised
    /// during the computation stops it and is the only entry.
    #[schemars(length(min = 1))]
    pub errors: Vec<ErrorEntry>,
}

/// What an adapter returns when a method fails: `{"error": <report>}`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(rename = "FairmarkErrorResponse")]
pub struct ErrorResponse {
    /// The report of the error.
    pub error: ErrorReport,
}

// The error recorded in a cell of an analysis table. Its schema keeps the name `ErrorReport`,
// under which it was published in the output snapshots of the analysis methods before the
// structured report existed; the full report is named `FairmarkErrorReport` in schemas, so the
// two never collide. Renaming it, or changing its doc comment (the schema description), would
// change those snapshots: hence a plain comment here.
/// The report of an error: its stable code and a message in English.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[schemars(rename = "ErrorReport")]
pub struct CompactErrorReport {
    /// Stable error code, for example `FM-E004` (see the errors reference).
    pub code: &'static str,
    /// What is wrong and, where known, how to fix it.
    pub message: String,
}

/// A refusal raised by one of the engine's own readers as a `serde` custom error. `serde`
/// carries only text, so the reader recognises the refusal by its fixed text, which the producer
/// writes through [`CustomRefusal::text`] and the reader finds within the message, and reports its sub-kind and fix. A new
/// custom refusal gets a variant here, never a text matched elsewhere.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CustomRefusal {
    /// `null` for a defaulted field of the DCF, which takes its default only when left out.
    NullNotAccepted,
    /// A non-recurring adjustment of D18 or D19, which are fixed in the investing result
    /// (SPEC-0005 R17); the producer writes the aggregate before the text.
    FixedAggregate,
    /// A technique of `tv.compare` that is not an object with exactly one key; the producer
    /// adds what it got.
    TechniqueKeys,
    /// A key repeated in an object that its type reads itself (a technique of `tv.compare`);
    /// the producer writes the key after the text.
    DuplicateKey,
}

impl CustomRefusal {
    /// Every refusal, for the registry's lookup.
    pub(crate) const ALL: [Self; 4] = [
        Self::NullNotAccepted,
        Self::FixedAggregate,
        Self::TechniqueKeys,
        Self::DuplicateKey,
    ];

    /// The fixed text of the refusal, written by its producer.
    pub(crate) const fn text(self) -> &'static str {
        match self {
            Self::NullNotAccepted => {
                "null is not accepted: leave the field out to use the default, or give a value"
            }
            Self::FixedAggregate => {
                "cannot be moved by an adjustment: D18 and D19 are fixed in the investing result \
                 (SPEC-0005 R17, Decision 17); remove the adjustment, or name the aggregate of \
                 Appendix A.2 the non-recurring part belongs to"
            }
            Self::TechniqueKeys => {
                "each technique is an object with exactly one key, the method id, for example \
                 {\"tv.gordon\": {...}}"
            }
            Self::DuplicateKey => "duplicate key",
        }
    }

    /// The accepted values the refusal lists, when they are a closed list of its own (the
    /// values of a field are otherwise read from the schema).
    pub(crate) fn expected(self) -> Vec<String> {
        match self {
            Self::TechniqueKeys => crate::tv::compare::TECHNIQUE_IDS
                .iter()
                .map(|id| (*id).to_owned())
                .collect(),
            _ => Vec::new(),
        }
    }

    pub(crate) const fn reason(self) -> Reason {
        match self {
            Self::NullNotAccepted => Reason::Type,
            Self::FixedAggregate => Reason::Enum,
            Self::TechniqueKeys => Reason::Length,
            Self::DuplicateKey => Reason::Duplicate,
        }
    }

    pub(crate) const fn fix(self) -> &'static str {
        match self {
            Self::NullNotAccepted => "leave the field out to use the default, or give a value",
            Self::FixedAggregate => {
                "remove the adjustment, or name the aggregate the non-recurring part belongs to"
            }
            Self::TechniqueKeys => {
                "give each technique as an object with one key, the method id, and add one entry \
                 per technique"
            }
            Self::DuplicateKey => "keep one",
        }
    }
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
            pointers: vec!["/years/0/statutory/balance_sheet/fixed_assets".into()],
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
