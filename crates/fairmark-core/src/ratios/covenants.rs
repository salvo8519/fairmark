//! `ratios.covenants`: does the company respect its covenants? (SPEC-0007 R5)
//!
//! Maintainer decision of 2026-11-08: the tested value comes from the numerator and the
//! denominator defined by the loan agreement, given as inputs (no standard ratio is
//! substituted); a ratio that cannot be computed is not tested, with the reason, never a
//! breach; the test date refers to the period ending on that date.
//!
//! Not decided, so not implemented (open questions in `plan.md`): a negative denominator is
//! refused with `FM-E001`; the tested value is never rounded; the relative headroom on a
//! negative threshold is not defined.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::RatioValue;
use crate::dates::Date;
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num;

/// One side of the covenant ratio, as the loan agreement defines it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractualAmount {
    /// The definition in the words of the loan agreement, for example "consolidated EBITDA
    /// for the 12 months ending on the test date"; not blank. Echoed in the audit trail.
    pub definition: String,
    /// The amount under that definition for the period ending on the test date, in currency
    /// units.
    pub amount: Decimal,
}

/// How the tested value is compared with the threshold. Required, no default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Operator {
    /// Maximum, inclusive: passes when tested value ≤ threshold.
    AtMost,
    /// Maximum, strict: passes when tested value < threshold.
    LessThan,
    /// Minimum, inclusive: passes when tested value ≥ threshold.
    AtLeast,
    /// Minimum, strict: passes when tested value > threshold.
    GreaterThan,
}

impl Operator {
    fn symbol(self) -> &'static str {
        match self {
            Self::AtMost => "≤",
            Self::LessThan => "<",
            Self::AtLeast => "≥",
            Self::GreaterThan => ">",
        }
    }

    fn is_maximum(self) -> bool {
        matches!(self, Self::AtMost | Self::LessThan)
    }

    fn describe(self) -> &'static str {
        match self {
            Self::AtMost => "maximum, inclusive",
            Self::LessThan => "maximum, strict",
            Self::AtLeast => "minimum, inclusive",
            Self::GreaterThan => "minimum, strict",
        }
    }

    fn passes(self, value: Decimal, threshold: Decimal) -> bool {
        match self {
            Self::AtMost => value <= threshold,
            Self::LessThan => value < threshold,
            Self::AtLeast => value >= threshold,
            Self::GreaterThan => value > threshold,
        }
    }
}

/// One covenant of the loan agreement at one test date.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Covenant {
    /// Name of the covenant as in the loan agreement, for example "net leverage"; not blank.
    pub name: String,
    /// Test date (ISO 8601, `YYYY-MM-DD`). The amounts refer to the period ending on this date.
    pub test_date: Date,
    /// Numerator of the covenant ratio, as defined by the loan agreement.
    pub numerator: ContractualAmount,
    /// Denominator of the covenant ratio, as defined by the loan agreement. Zero: the covenant
    /// is not tested. Negative: refused (`FM-E001`), the treatment is not specified.
    pub denominator: ContractualAmount,
    /// Comparison with the threshold: `at_most` (≤), `less_than` (<), `at_least` (≥) or
    /// `greater_than` (>). Required, no default.
    pub operator: Operator,
    /// Threshold in the unit of numerator / denominator: a multiple (3.5 = 3.5x) or a decimal
    /// fraction (0.30 = 30%), as the loan agreement states it.
    pub threshold: Decimal,
}

/// Input of `ratios.covenants`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CovenantsInput {
    /// The covenants to test, at least one. Each is tested on its own; the results keep this
    /// order.
    pub covenants: Vec<Covenant>,
}

/// Outcome of one covenant test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// The tested value satisfies the operator and the threshold.
    Passed,
    /// The tested value does not satisfy the operator and the threshold.
    Breached,
    /// The tested value cannot be computed; never a breach. The reason is in
    /// `not_tested_reason`.
    NotTested,
}

/// Result of one covenant test.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CovenantResult {
    /// Name of the covenant, as given.
    pub name: String,
    /// Test date, as given.
    pub test_date: Date,
    /// Comparison operator, as given.
    pub operator: Operator,
    /// Threshold, as given.
    pub threshold: Decimal,
    /// `passed`, `breached` or `not_tested`.
    pub outcome: Outcome,
    /// Tested value = numerator / denominator, exact (not rounded), in the unit of the
    /// threshold; null when not tested.
    pub value: Option<Decimal>,
    /// Distance from the threshold in the unit of the tested value: positive on the compliant
    /// side, negative on the breach side; threshold − value for a maximum, value − threshold
    /// for a minimum. Zero at the threshold, where an inclusive operator passes and a strict
    /// one breaches. Null when not tested.
    pub headroom: Option<Decimal>,
    /// Headroom / threshold, decimal fraction (0.10 = 10% of the threshold), same sign as the
    /// headroom; not defined when the threshold is zero or negative, or when not tested.
    pub headroom_relative: RatioValue,
    /// Why the covenant was not tested, for example "denominator is zero: consolidated EBITDA
    /// for the 12 months ending on the test date"; null when tested.
    pub not_tested_reason: Option<String>,
}

/// How many covenants have each outcome.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CovenantSummary {
    /// Number of covenants passed.
    pub passed: u32,
    /// Number of covenants breached.
    pub breached: u32,
    /// Number of covenants not tested.
    pub not_tested: u32,
}

/// Output of `ratios.covenants`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CovenantsOutput {
    /// One result per covenant, in the order of the input.
    pub covenants: Vec<CovenantResult>,
    /// Count of each outcome.
    pub summary: CovenantSummary,
}

fn invalid(message: String) -> FairmarkError {
    FairmarkError::InvalidInput(message)
}

/// Refuses what cannot be tested without a choice Fairmark does not make: blank names and
/// definitions, and negative denominators (open question 1 of `plan.md`).
fn validate(input: &CovenantsInput) -> Result<()> {
    if input.covenants.is_empty() {
        return Err(invalid(
            "`covenants` is empty: give at least one covenant to test".into(),
        ));
    }
    for (i, c) in input.covenants.iter().enumerate() {
        if c.name.trim().is_empty() {
            return Err(invalid(format!(
                "`covenants[{i}].name` is empty: name the covenant as in the loan agreement, \
                 for example \"net leverage\""
            )));
        }
        for (side, part) in [("numerator", &c.numerator), ("denominator", &c.denominator)] {
            if part.definition.trim().is_empty() {
                return Err(invalid(format!(
                    "`covenants[{i}].{side}.definition` is empty: state the {side} as the loan \
                     agreement defines it"
                )));
            }
        }
        let den = c.denominator.amount;
        if den.is_sign_negative() && !den.is_zero() {
            return Err(invalid(format!(
                "`covenants[{i}].denominator.amount` is negative ({den}): how a covenant is \
                 tested on a negative denominator is not specified (SPEC-0007, open question), \
                 so Fairmark does not test it; remove covenant {i} (\"{}\") from the list and \
                 assess it under the terms of the loan agreement",
                c.name
            )));
        }
    }
    Ok(())
}

fn test(c: &Covenant, ex: &mut Explain) -> Result<CovenantResult> {
    let label = format!("{} ({})", c.name, c.test_date);
    let op = c.operator;
    let threshold = c.threshold;
    ex.convention(format!(
        "{label}: passes when the tested value {} {threshold} ({})",
        op.symbol(),
        op.describe()
    ));
    ex.step(
        format!("{label}: numerator"),
        format!("as given: {}", c.numerator.definition),
        c.numerator.amount,
    );
    ex.step(
        format!("{label}: denominator"),
        format!("as given: {}", c.denominator.definition),
        c.denominator.amount,
    );
    let mut result = CovenantResult {
        name: c.name.clone(),
        test_date: c.test_date,
        operator: op,
        threshold,
        outcome: Outcome::NotTested,
        value: None,
        headroom: None,
        headroom_relative: RatioValue {
            value: None,
            not_defined: Some("covenant not tested".into()),
        },
        not_tested_reason: None,
    };
    if c.denominator.amount.is_zero() {
        let reason = format!("denominator is zero: {}", c.denominator.definition);
        ex.convention(format!("{label} not tested: {reason}"));
        result.not_tested_reason = Some(reason);
        return Ok(result);
    }

    let (n, d) = (c.numerator.amount, c.denominator.amount);
    let value = num::div(n, d)?;
    ex.step(
        format!("{label}: tested value"),
        format!("numerator / denominator = {n} / {d}"),
        value,
    );
    let (headroom, formula) = if op.is_maximum() {
        (
            num::sub(threshold, value)?,
            format!("threshold − tested value = {threshold} − {value}"),
        )
    } else {
        (
            num::sub(value, threshold)?,
            format!("tested value − threshold = {value} − {threshold}"),
        )
    };
    ex.step(format!("{label}: headroom"), formula, headroom);
    let relative_label = format!("{label} relative headroom");
    let headroom_relative = if threshold.is_zero() {
        RatioValue::not_defined(ex, &relative_label, "threshold is zero".into())
    } else if threshold.is_sign_negative() {
        RatioValue::not_defined(
            ex,
            &relative_label,
            format!(
                "threshold is negative ({threshold}); the relative headroom on a negative \
                 threshold is not specified"
            ),
        )
    } else {
        let relative = num::div(headroom, threshold)?;
        ex.step(
            format!("{label}: relative headroom"),
            format!("headroom / threshold = {headroom} / {threshold}"),
            relative,
        );
        RatioValue::defined(relative)
    };
    result.outcome = if op.passes(value, threshold) {
        Outcome::Passed
    } else {
        Outcome::Breached
    };
    result.value = Some(value);
    result.headroom = Some(headroom);
    result.headroom_relative = headroom_relative;
    Ok(result)
}

pub fn covenants(input: &CovenantsInput) -> Result<(CovenantsOutput, Explain)> {
    validate(input)?;
    let mut ex = Explain::new(
        Covenants::ID,
        "tested value = numerator / denominator (loan agreement definitions); headroom = \
         threshold − tested value (maximum) or tested value − threshold (minimum); relative \
         headroom = headroom / threshold",
    );
    ex.convention(
        "tested value = numerator / denominator, each as defined by the loan agreement and \
         given as an amount; no standard ratio is substituted",
    );
    ex.convention("the amounts of each covenant refer to the period ending on its test date");
    ex.convention("tested value compared with the threshold exactly, without rounding");
    ex.convention(
        "headroom positive on the compliant side and negative on the breach side: threshold − \
         tested value for a maximum, tested value − threshold for a minimum",
    );
    ex.convention(
        "relative headroom = headroom / threshold, a decimal fraction; not defined when the \
         threshold is zero or negative",
    );
    ex.convention(
        "a covenant whose tested value cannot be computed (zero denominator) is not tested, \
         never breached",
    );

    let mut results = Vec::with_capacity(input.covenants.len());
    let mut summary = CovenantSummary {
        passed: 0,
        breached: 0,
        not_tested: 0,
    };
    for c in &input.covenants {
        let r = test(c, &mut ex)?;
        let count = match r.outcome {
            Outcome::Passed => &mut summary.passed,
            Outcome::Breached => &mut summary.breached,
            Outcome::NotTested => &mut summary.not_tested,
        };
        *count = count
            .checked_add(1)
            .ok_or(FairmarkError::Overflow("the covenant count"))?;
        results.push(r);
    }
    Ok((
        CovenantsOutput {
            covenants: results,
            summary,
        },
        ex,
    ))
}

/// `ratios.covenants`: see the methodology page.
pub struct Covenants;

impl Method for Covenants {
    const ID: &'static str = "ratios.covenants";
    const TITLE: &'static str = "Ratios — covenants";
    const DESCRIPTION: &'static str =
        "Tests financial covenants on the numerator and denominator defined by the loan \
        agreement, against a threshold with an explicit operator (at most, less than, at \
        least, greater than) at a test date. Returns for each covenant the outcome (passed, \
        breached or not tested with the reason, never a breach when the ratio cannot be \
        computed), the tested value and the headroom, absolute and relative to the threshold.";
    const VERSION: &'static str = "1";
    type Input = CovenantsInput;
    type Output = CovenantsOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        covenants(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    fn amount(definition: &str, v: i64) -> ContractualAmount {
        ContractualAmount {
            definition: definition.into(),
            amount: d(v),
        }
    }

    fn covenant(num: i64, den: i64, operator: Operator, threshold: Decimal) -> Covenant {
        Covenant {
            name: "net leverage".into(),
            test_date: "2026-12-31".parse().unwrap(),
            numerator: amount("net debt", num),
            denominator: amount("EBITDA", den),
            operator,
            threshold,
        }
    }

    fn run(c: Vec<Covenant>) -> Result<(CovenantsOutput, Explain)> {
        covenants(&CovenantsInput { covenants: c })
    }

    fn one(c: Covenant) -> CovenantResult {
        run(vec![c]).unwrap().0.covenants.remove(0)
    }

    #[test]
    fn maximum_passes_with_positive_headroom_and_breaches_with_negative() {
        let r = one(covenant(300, 100, Operator::AtMost, d(4)));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.value, Some(d(3)));
        assert_eq!(r.headroom, Some(d(1)));
        assert_eq!(r.headroom_relative.value, Some(Decimal::new(25, 2)));
        let r = one(covenant(500, 100, Operator::AtMost, d(4)));
        assert_eq!(r.outcome, Outcome::Breached);
        assert_eq!(r.headroom, Some(d(-1)));
        assert_eq!(r.headroom_relative.value, Some(Decimal::new(-25, 2)));
    }

    #[test]
    fn minimum_headroom_is_value_minus_threshold() {
        let r = one(covenant(300, 100, Operator::AtLeast, d(2)));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.headroom, Some(d(1)));
        let r = one(covenant(300, 100, Operator::GreaterThan, d(4)));
        assert_eq!(r.outcome, Outcome::Breached);
        assert_eq!(r.headroom, Some(d(-1)));
    }

    #[test]
    fn at_the_threshold_inclusive_passes_and_strict_breaches() {
        for (op, outcome) in [
            (Operator::AtMost, Outcome::Passed),
            (Operator::LessThan, Outcome::Breached),
            (Operator::AtLeast, Outcome::Passed),
            (Operator::GreaterThan, Outcome::Breached),
        ] {
            let r = one(covenant(300, 100, op, d(3)));
            assert_eq!(r.outcome, outcome, "{op:?}");
            assert_eq!(r.headroom, Some(Decimal::ZERO));
        }
    }

    #[test]
    fn zero_denominator_is_not_tested_never_breached() {
        let (out, ex) = run(vec![
            covenant(300, 0, Operator::AtMost, d(3)),
            covenant(300, 100, Operator::AtMost, d(2)),
        ])
        .unwrap();
        let r = &out.covenants[0];
        assert_eq!(r.outcome, Outcome::NotTested);
        assert_eq!(r.value, None);
        assert_eq!(r.headroom, None);
        assert_eq!(
            r.not_tested_reason.as_deref(),
            Some("denominator is zero: EBITDA")
        );
        assert_eq!(
            r.headroom_relative.not_defined.as_deref(),
            Some("covenant not tested")
        );
        assert_eq!(out.covenants[1].outcome, Outcome::Breached);
        assert_eq!(
            (
                out.summary.passed,
                out.summary.breached,
                out.summary.not_tested
            ),
            (0, 1, 1)
        );
        assert!(ex
            .conventions
            .contains(&"net leverage (2026-12-31) not tested: denominator is zero: EBITDA".into()));
    }

    #[test]
    fn negative_numerator_is_compared_as_computed() {
        let r = one(covenant(-100, 200, Operator::AtMost, d(3)));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.value, Some(Decimal::new(-5, 1)));
        assert_eq!(r.headroom, Some(Decimal::new(35, 1)));
    }

    #[test]
    fn negative_denominator_is_refused_naming_the_field() {
        let err = run(vec![
            covenant(300, 100, Operator::AtMost, d(3)),
            covenant(300, -100, Operator::AtMost, d(3)),
        ])
        .unwrap_err();
        assert_eq!(err.code(), "FM-E001");
        assert!(err
            .to_string()
            .contains("`covenants[1].denominator.amount` is negative (-100)"));
    }

    #[test]
    fn relative_headroom_needs_a_positive_threshold() {
        let r = one(covenant(-100, 200, Operator::AtMost, Decimal::ZERO));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.headroom, Some(Decimal::new(5, 1)));
        assert_eq!(
            r.headroom_relative.not_defined.as_deref(),
            Some("threshold is zero")
        );
        let r = one(covenant(-100, 200, Operator::AtMost, Decimal::new(-25, 2)));
        assert_eq!(r.headroom, Some(Decimal::new(25, 2)));
        assert!(r
            .headroom_relative
            .not_defined
            .unwrap()
            .starts_with("threshold is negative (-0.25)"));
    }

    #[test]
    fn empty_list_and_blank_texts_are_refused() {
        assert_eq!(run(vec![]).unwrap_err().code(), "FM-E001");
        let mut c = covenant(300, 100, Operator::AtMost, d(3));
        c.name = " ".into();
        let err = run(vec![c]).unwrap_err();
        assert!(err.to_string().contains("`covenants[0].name` is empty"));
        let mut c = covenant(300, 100, Operator::AtMost, d(3));
        c.numerator.definition = "\t".into();
        let err = run(vec![c]).unwrap_err();
        assert!(err
            .to_string()
            .contains("`covenants[0].numerator.definition` is empty"));
    }

    #[test]
    fn explain_shows_the_numbers() {
        let (_, ex) = run(vec![covenant(
            1900,
            600,
            Operator::AtMost,
            Decimal::new(35, 1),
        )])
        .unwrap();
        let formulas: Vec<_> = ex.steps.iter().map(|s| s.formula.as_str()).collect();
        assert_eq!(formulas[0], "as given: net debt");
        assert_eq!(formulas[2], "numerator / denominator = 1900 / 600");
        assert!(formulas[3].starts_with("threshold − tested value = 3.5 − 3.1666"));
        assert!(formulas[4].starts_with("headroom / threshold = 0.3333"));
    }
}
