//! `ratios.covenants`: does the company respect its covenants? (SPEC-0007 R5)
//!
//! Maintainer decision of 2026-11-08: the tested value comes from the numerator and the
//! denominator defined by the loan agreement, given as inputs (no standard ratio is
//! substituted); a ratio that cannot be computed is not tested, with the reason, never a
//! breach; the test date refers to the period ending on that date.
//!
//! Precision: the outcome is decided exactly; value and headroom are shown to the 28
//! significant digits of the decimal type, not rounded to the decimals of the agreement. The
//! decision is the sign of numerator − threshold × denominator (denominator positive), computed
//! in integer arithmetic on the decimal mantissas ([`exact_cmp`]), so a quotient rounded to 28
//! digits can never turn a breach into a pass.
//!
//! Not decided, so not implemented (open questions in `plan.md`): a negative denominator is
//! refused with `FM-E001`; the tested value is never rounded to the decimals of the agreement;
//! the relative headroom on a negative threshold is not defined.

use std::cmp::Ordering;

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
    #[schemars(length(min = 1))]
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
    fn name(self) -> &'static str {
        match self {
            Self::AtMost => "at_most",
            Self::LessThan => "less_than",
            Self::AtLeast => "at_least",
            Self::GreaterThan => "greater_than",
        }
    }

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

    /// Whether the test passes, given how the exact tested value compares with the threshold.
    fn passes(self, value_vs_threshold: Ordering) -> bool {
        match self {
            Self::AtMost => value_vs_threshold != Ordering::Greater,
            Self::LessThan => value_vs_threshold == Ordering::Less,
            Self::AtLeast => value_vs_threshold != Ordering::Less,
            Self::GreaterThan => value_vs_threshold == Ordering::Greater,
        }
    }
}

/// One covenant of the loan agreement at one test date.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Covenant {
    /// Name of the covenant as in the loan agreement, for example "net leverage"; not blank.
    #[schemars(length(min = 1))]
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
    #[schemars(length(min = 1))]
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
    /// `not_tested_code` and `not_tested_reason`.
    NotTested,
}

/// Why a covenant was not tested, machine-readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotTestedCode {
    /// The denominator amount is zero.
    ZeroDenominator,
    /// The tested value or the headroom exceeds the range of the decimal type.
    Overflow,
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
    /// `passed`, `breached` or `not_tested`, decided exactly (not on the rounded `value`).
    pub outcome: Outcome,
    /// Tested value = numerator / denominator, shown to the 28 significant digits of the
    /// decimal type, not rounded to the decimals of the agreement; in the unit of the
    /// threshold; null when not tested.
    pub value: Option<Decimal>,
    /// Distance from the threshold in the unit of the tested value: positive on the compliant
    /// side, negative on the breach side; threshold − value for a maximum, value − threshold
    /// for a minimum. Zero at the threshold, where an inclusive operator passes and a strict
    /// one breaches. Shown to 28 significant digits; null when not tested.
    pub headroom: Option<Decimal>,
    /// Headroom / threshold, decimal fraction (0.10 = 10% of the threshold), same sign as the
    /// headroom; not defined when the threshold is zero or negative, when the quotient
    /// overflows, or when not tested.
    pub headroom_relative: RatioValue,
    /// Why the covenant was not tested: `zero_denominator` or `overflow`; null when tested.
    pub not_tested_code: Option<NotTestedCode>,
    /// Why the covenant was not tested, in words, for example "denominator is zero:
    /// consolidated EBITDA for the 12 months ending on the test date"; null when tested.
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

// ------------------------------------------------------------------ exact comparison

/// Unsigned integer of any size, little-endian 64-bit limbs, without leading zero limbs.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Wide(Vec<u64>);

impl Wide {
    fn from_u128(v: u128) -> Self {
        // Truncation is intended: the two halves of the 128-bit value.
        Self(vec![v as u64, (v >> 64) as u64]).trimmed()
    }

    fn trimmed(mut self) -> Self {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
        self
    }

    fn mul(&self, other: &Self) -> Self {
        let mut out = vec![0u64; self.0.len() + other.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &b) in other.0.iter().enumerate() {
                let cur = u128::from(out[i + j]) + u128::from(a) * u128::from(b) + carry;
                out[i + j] = cur as u64;
                carry = cur >> 64;
            }
            out[i + other.0.len()] = carry as u64;
        }
        Self(out).trimmed()
    }

    /// self × 10^exp.
    fn times_pow10(&self, exp: u32) -> Self {
        let ten = Self::from_u128(10);
        (0..exp).fold(self.clone(), |acc, _| acc.mul(&ten))
    }

    fn cmp(&self, other: &Self) -> Ordering {
        self.0
            .len()
            .cmp(&other.0.len())
            .then_with(|| self.0.iter().rev().cmp(other.0.iter().rev()))
    }
}

fn sign(d: Decimal) -> Ordering {
    if d.is_zero() {
        Ordering::Equal
    } else if d.is_sign_negative() {
        Ordering::Less
    } else {
        Ordering::Greater
    }
}

/// How `numerator` compares with `threshold × denominator`, exactly. With a positive
/// denominator this is how numerator / denominator compares with the threshold, with no
/// rounding of the quotient. Every decimal is mantissa / 10^scale (mantissa below 2^96, scale
/// at most 28), so both sides are brought to a common power of ten and compared as integers.
pub(crate) fn exact_cmp(numerator: Decimal, threshold: Decimal, denominator: Decimal) -> Ordering {
    let sign_n = sign(numerator);
    let sign_td = match (sign(threshold), sign(denominator)) {
        (Ordering::Equal, _) | (_, Ordering::Equal) => Ordering::Equal,
        (a, b) if a == b => Ordering::Greater,
        _ => Ordering::Less,
    };
    if sign_n != sign_td || sign_n == Ordering::Equal {
        return sign_n.cmp(&sign_td);
    }
    let n = Wide::from_u128(numerator.mantissa().unsigned_abs());
    let td = Wide::from_u128(threshold.mantissa().unsigned_abs())
        .mul(&Wide::from_u128(denominator.mantissa().unsigned_abs()));
    let (scale_n, scale_td) = (numerator.scale(), threshold.scale() + denominator.scale());
    let common = scale_n.max(scale_td);
    let magnitude = n
        .times_pow10(common - scale_n)
        .cmp(&td.times_pow10(common - scale_td));
    if sign_n == Ordering::Less {
        magnitude.reverse()
    } else {
        magnitude
    }
}

// ------------------------------------------------------------------ method

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

fn not_tested(
    ex: &mut Explain,
    label: &str,
    result: CovenantResult,
    code: NotTestedCode,
    reason: String,
) -> CovenantResult {
    ex.convention(format!("{label}: not tested ({reason})"));
    CovenantResult {
        not_tested_code: Some(code),
        not_tested_reason: Some(reason),
        ..result
    }
}

const OVERFLOW_VALUE: &str =
    "the tested value exceeds the range of the decimal type (28-29 significant digits)";
const OVERFLOW_HEADROOM: &str =
    "the headroom exceeds the range of the decimal type (28-29 significant digits)";

/// Tests covenant number `k` (counted from 1 in the audit trail).
fn test(k: usize, c: &Covenant, ex: &mut Explain) -> CovenantResult {
    let label = format!("covenant {k}, {} ({})", c.name, c.test_date);
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
    let result = CovenantResult {
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
        not_tested_code: None,
        not_tested_reason: None,
    };
    let (n, d) = (c.numerator.amount, c.denominator.amount);
    if d.is_zero() {
        let reason = format!("denominator is zero: {}", c.denominator.definition);
        return not_tested(ex, &label, result, NotTestedCode::ZeroDenominator, reason);
    }

    let Ok(value) = num::div(n, d) else {
        let reason = OVERFLOW_VALUE.to_owned();
        return not_tested(ex, &label, result, NotTestedCode::Overflow, reason);
    };
    ex.step(
        format!("{label}: tested value"),
        format!("numerator / denominator = {n} / {d}"),
        value,
    );
    let (headroom, formula) = if op.is_maximum() {
        (
            num::sub(threshold, value),
            format!("threshold − tested value = {threshold} − {value}"),
        )
    } else {
        (
            num::sub(value, threshold),
            format!("tested value − threshold = {value} − {threshold}"),
        )
    };
    let Ok(headroom) = headroom else {
        let reason = OVERFLOW_HEADROOM.to_owned();
        return not_tested(ex, &label, result, NotTestedCode::Overflow, reason);
    };
    ex.step(format!("{label}: headroom"), formula, headroom);

    let relative_label = format!("{label}: relative headroom");
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
    } else if let Ok(relative) = num::div(headroom, threshold) {
        ex.step(
            relative_label,
            format!("headroom / threshold = {headroom} / {threshold}"),
            relative,
        );
        RatioValue::defined(relative)
    } else {
        RatioValue::not_defined(
            ex,
            &relative_label,
            "headroom / threshold exceeds the range of the decimal type".into(),
        )
    };

    // The decision: exact, on numerator − threshold × denominator (denominator positive here).
    let passed = op.passes(exact_cmp(n, threshold, d));
    let (outcome, verdict) = if passed {
        (Outcome::Passed, "passed")
    } else {
        (Outcome::Breached, "breached")
    };
    ex.step(
        format!("{label}: outcome"),
        format!(
            "{verdict} ({n} / {d} {} {threshold} is {passed}, decided exactly)",
            op.name()
        ),
        value,
    );
    CovenantResult {
        outcome,
        value: Some(value),
        headroom: Some(headroom),
        headroom_relative,
        ..result
    }
}

pub fn covenants(input: &CovenantsInput) -> Result<(CovenantsOutput, Explain)> {
    validate(input)?;
    let mut ex = Explain::new(
        Covenants::ID,
        "tested value = numerator / denominator (loan agreement definitions); outcome from the \
         exact sign of numerator − threshold × denominator; headroom = threshold − tested value \
         (maximum) or tested value − threshold (minimum); relative headroom = headroom / \
         threshold",
    );
    ex.convention(
        "tested value = numerator / denominator, each as defined by the loan agreement and \
         given as an amount; no standard ratio is substituted",
    );
    ex.convention("the amounts of each covenant refer to the period ending on its test date");
    ex.convention(
        "the outcome is decided exactly; value and headroom are shown to the 28 significant \
         digits of the decimal type, not rounded to the decimals of the agreement",
    );
    ex.convention(
        "exact decision: the sign of numerator − threshold × denominator, computed in integer \
         arithmetic on the decimal mantissas, with a positive denominator",
    );
    ex.convention(
        "headroom positive on the compliant side and negative on the breach side: threshold − \
         tested value for a maximum, tested value − threshold for a minimum",
    );
    ex.convention(
        "relative headroom = headroom / threshold, a decimal fraction; not defined when the \
         threshold is zero or negative",
    );
    ex.convention("a negative numerator is compared as computed");
    ex.convention(
        "a covenant whose tested value cannot be computed (zero denominator, or a value or \
         headroom beyond the range of the decimal type) is not tested, never breached",
    );
    ex.convention(
        "covenants are numbered from 1 in the audit trail; error messages name the zero-based \
         position, covenants[i]",
    );

    let mut results = Vec::with_capacity(input.covenants.len());
    let mut summary = CovenantSummary {
        passed: 0,
        breached: 0,
        not_tested: 0,
    };
    for (i, c) in input.covenants.iter().enumerate() {
        let r = test(i + 1, c, &mut ex);
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
    for (label, outcome, count) in [
        ("Covenants passed", "passed", summary.passed),
        ("Covenants breached", "breached", summary.breached),
        ("Covenants not tested", "not_tested", summary.not_tested),
    ] {
        ex.step(
            label,
            format!("count of covenants with outcome {outcome}"),
            Decimal::from(count),
        );
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
        breached, or not tested with a code and the reason, never a breach when the ratio \
        cannot be computed), decided exactly, the tested value and the headroom, absolute and \
        relative to the threshold.";
    const VERSION: &'static str = "1";
    type Input = CovenantsInput;
    type Output = CovenantsOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        covenants(input)
    }
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    fn d(v: i64) -> Decimal {
        Decimal::from(v)
    }

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    fn amount(definition: &str, v: Decimal) -> ContractualAmount {
        ContractualAmount {
            definition: definition.into(),
            amount: v,
        }
    }

    fn cov(num: Decimal, den: Decimal, operator: Operator, threshold: Decimal) -> Covenant {
        Covenant {
            name: "net leverage".into(),
            test_date: "2026-12-31".parse().unwrap(),
            numerator: amount("net debt", num),
            denominator: amount("EBITDA", den),
            operator,
            threshold,
        }
    }

    fn covenant(num: i64, den: i64, operator: Operator, threshold: Decimal) -> Covenant {
        cov(d(num), d(den), operator, threshold)
    }

    fn run(c: Vec<Covenant>) -> Result<(CovenantsOutput, Explain)> {
        covenants(&CovenantsInput { covenants: c })
    }

    fn one(c: Covenant) -> CovenantResult {
        run(vec![c]).unwrap().0.covenants.remove(0)
    }

    #[test]
    fn exact_outcome_does_not_depend_on_the_rounded_quotient() {
        // 3.0000000000000000000000000001 / 3 rounds to 1 at 28 digits, but exceeds 1.
        let r = one(cov(
            dec("3.0000000000000000000000000001"),
            d(3),
            Operator::AtMost,
            d(1),
        ));
        assert_eq!(r.outcome, Outcome::Breached);
        // 2.9999999999999999999999999999 / 3 rounds to 1, but is below 1.
        let r = one(cov(
            dec("2.9999999999999999999999999999"),
            d(3),
            Operator::AtLeast,
            d(1),
        ));
        assert_eq!(r.outcome, Outcome::Breached);
        // 2 / 3 rounds to 0.6666666666666666666666666667, but is below it.
        let r = one(cov(
            d(2),
            d(3),
            Operator::LessThan,
            dec("0.6666666666666666666666666667"),
        ));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.value, Some(dec("0.6666666666666666666666666667")));
        // 1 / MAX rounds to 0, but is positive.
        let r = one(cov(d(1), Decimal::MAX, Operator::GreaterThan, d(0)));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.value, Some(Decimal::ZERO));
    }

    #[test]
    fn exact_cmp_signs_and_scales() {
        assert_eq!(exact_cmp(d(-1), d(0), d(5)), Ordering::Less);
        assert_eq!(exact_cmp(d(0), d(0), d(5)), Ordering::Equal);
        assert_eq!(exact_cmp(d(-6), d(-2), d(3)), Ordering::Equal);
        assert_eq!(exact_cmp(dec("-6.1"), d(-2), d(3)), Ordering::Less);
        assert_eq!(exact_cmp(dec("1.50"), dec("0.5"), d(3)), Ordering::Equal);
        assert_eq!(
            exact_cmp(
                Decimal::MAX,
                Decimal::MAX,
                dec("0.0000000000000000000000000001")
            ),
            Ordering::Greater
        );
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
        assert_eq!(r.not_tested_code, Some(NotTestedCode::ZeroDenominator));
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
        let line =
            "covenant 1, net leverage (2026-12-31): not tested (denominator is zero: EBITDA)";
        assert!(ex.conventions.iter().any(|c| c == line));
    }

    #[test]
    fn negative_zero_denominator_is_not_tested() {
        let r = one(cov(d(300), dec("-0"), Operator::AtMost, d(3)));
        assert_eq!(r.outcome, Outcome::NotTested);
        assert_eq!(r.not_tested_code, Some(NotTestedCode::ZeroDenominator));
    }

    #[test]
    fn overflow_makes_one_covenant_not_tested_and_the_others_continue() {
        let tiny = dec("0.0000000000000000000000000001");
        let (out, _) = run(vec![
            cov(Decimal::MAX, tiny, Operator::AtMost, d(3)),
            cov(Decimal::MAX, d(1), Operator::AtMost, Decimal::MIN),
            covenant(300, 100, Operator::AtMost, d(4)),
        ])
        .unwrap();
        let first = &out.covenants[0];
        assert_eq!(first.not_tested_code, Some(NotTestedCode::Overflow));
        assert_eq!(first.not_tested_reason.as_deref(), Some(OVERFLOW_VALUE));
        let second = &out.covenants[1];
        assert_eq!(second.not_tested_code, Some(NotTestedCode::Overflow));
        assert_eq!(second.not_tested_reason.as_deref(), Some(OVERFLOW_HEADROOM));
        assert_eq!(out.covenants[2].outcome, Outcome::Passed);
        assert_eq!(out.summary.not_tested, 2);
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
    fn explain_shows_the_numbers_the_verdict_and_the_counts() {
        let (_, ex) = run(vec![
            covenant(1900, 600, Operator::AtMost, Decimal::new(35, 1)),
            covenant(600, 175, Operator::AtLeast, d(4)),
        ])
        .unwrap();
        let step = |label: &str| {
            ex.steps
                .iter()
                .find(|s| s.label == label)
                .unwrap_or_else(|| panic!("{label}"))
                .clone()
        };
        let first = "covenant 1, net leverage (2026-12-31)";
        assert_eq!(
            step(&format!("{first}: numerator")).formula,
            "as given: net debt"
        );
        assert_eq!(
            step(&format!("{first}: tested value")).formula,
            "numerator / denominator = 1900 / 600"
        );
        assert!(step(&format!("{first}: headroom"))
            .formula
            .starts_with("threshold − tested value = 3.5 − 3.1666"));
        assert!(step(&format!("{first}: relative headroom"))
            .formula
            .starts_with("headroom / threshold = 0.3333"));
        assert_eq!(
            step("covenant 2, net leverage (2026-12-31): outcome").formula,
            "breached (600 / 175 at_least 4 is false, decided exactly)"
        );
        assert_eq!(step("Covenants passed").value, d(1));
        assert_eq!(step("Covenants breached").value, d(1));
        assert_eq!(step("Covenants not tested").value, d(0));
    }
}
