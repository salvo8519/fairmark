//! `ratios.covenants`: does the company respect its covenants? (SPEC-0007 R5)
//!
//! Maintainer decisions of 2026-11-08: the tested value comes from the numerator and the
//! denominator defined by the loan agreement, given as inputs (no standard ratio is
//! substituted); the test date refers to the period ending on that date. The engine never
//! decides what the loan agreement decides: the outcome for a denominator that is not positive
//! (`if_denominator_not_positive`) and the contractual rounding (`rounding`) are explicit
//! inputs, so every outcome traces back to a clause.
//!
//! Precision: the outcome is decided exactly. Without rounding it is the sign of
//! numerator − threshold × denominator, computed in integer arithmetic on the decimal mantissas
//! ([`exact_cmp`]); with rounding the value is rounded exactly from the rational quotient
//! ([`round_exact`]) and compared with the threshold. Value and headroom are shown to the 28
//! significant digits of the decimal type.
//!
//! Partial input (amendment of 2026-11-09, R11): the amounts are optional. A covenant whose
//! denominator amount is missing, or whose denominator is positive and numerator amount is
//! missing, is not tested with the code `missing_input`, never breached; a denominator given
//! zero or negative gives the outcome of `if_denominator_not_positive` whatever the numerator.

use std::cmp::Ordering;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{Missing, NotComputedList, RatioValue, MISSING_CONVENTION};
use crate::dates::Date;
use crate::error::{FairmarkError, Found, Reason, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::not_computed::NotComputedEntry;
use crate::num;

/// Most decimal places a contractual rounding may state.
pub const MAX_DECIMAL_PLACES: u32 = 6;

/// One side of the covenant ratio, as the loan agreement defines it.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ContractualAmount {
    /// The definition in the words of the loan agreement, for example "consolidated EBITDA
    /// for the 12 months ending on the test date"; not blank. Echoed in the audit trail.
    #[schemars(length(min = 1))]
    pub definition: String,
    /// The amount under that definition for the period ending on the test date, in currency
    /// units. Absent or null: not given, and the covenant is not tested (`missing_input`)
    /// unless a denominator given zero or negative decides it.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Decimal>,
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

/// The outcome the loan agreement assigns to a covenant whose denominator is zero or negative.
/// Required, no default.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum IfDenominatorNotPositive {
    /// The covenant is breached.
    Breached,
    /// The covenant is passed.
    Passed,
    /// The covenant is not tested (never a breach); code `denominator_not_positive`.
    NotTested,
}

/// How halves are rounded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RoundingMode {
    /// A half is rounded away from zero (3.505 → 3.51, −3.505 → −3.51).
    HalfUp,
    /// A half is rounded to the even last digit (3.505 → 3.50, 3.515 → 3.52).
    HalfEven,
}

impl RoundingMode {
    fn name(self) -> &'static str {
        match self {
            Self::HalfUp => "half_up",
            Self::HalfEven => "half_even",
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Self::HalfUp => "halves away from zero",
            Self::HalfEven => "halves to the even digit",
        }
    }
}

/// The rounding of the tested value that the loan agreement states.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Rounding {
    /// Decimal places of the rounded tested value, 0 to 6 (2 rounds 3.504 to 3.50).
    #[schemars(range(min = 0, max = 6))]
    pub decimal_places: u32,
    /// `half_up` (halves away from zero) or `half_even` (halves to the even digit).
    pub mode: RoundingMode,
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
    /// Denominator of the covenant ratio, as defined by the loan agreement. Zero or negative:
    /// the outcome is `if_denominator_not_positive`.
    pub denominator: ContractualAmount,
    /// Comparison with the threshold: `at_most` (≤), `less_than` (<), `at_least` (≥) or
    /// `greater_than` (>). Required, no default.
    pub operator: Operator,
    /// Threshold in the unit of numerator / denominator: a multiple (3.5 = 3.5x) or a decimal
    /// fraction (0.30 = 30%), as the loan agreement states it.
    pub threshold: Decimal,
    /// Outcome when the denominator is zero or negative, as the loan agreement states:
    /// `breached`, `passed` or `not_tested`. Required, no default.
    pub if_denominator_not_positive: IfDenominatorNotPositive,
    /// Rounding of the tested value before the comparison, as the loan agreement states.
    /// Absent: no rounding; the outcome is decided on the exact value.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rounding: Option<Rounding>,
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
    /// The covenant is respected.
    Passed,
    /// The covenant is breached.
    Breached,
    /// The covenant is not tested; never a breach. The reason is in `not_tested_code` and
    /// `not_tested_reason`.
    NotTested,
}

/// What decided the outcome, machine-readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeBasis {
    /// The tested value (rounded when `rounding` is given) compared with the threshold.
    TestedValue,
    /// The denominator is zero or negative: the outcome is the input
    /// `if_denominator_not_positive`.
    IfDenominatorNotPositive,
}

/// Why a covenant was not tested, machine-readable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NotTestedCode {
    /// The denominator is zero or negative and `if_denominator_not_positive` is `not_tested`.
    DenominatorNotPositive,
    /// The tested value, the rounded value or the headroom exceeds the range of the decimal
    /// type.
    Overflow,
    /// An amount the test needs is not given: the denominator, or the numerator with a
    /// positive denominator. The outputs are listed in `not_computed`.
    MissingInput,
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
    /// `passed`, `breached` or `not_tested`, decided exactly (never on a value rounded by the
    /// decimal type).
    pub outcome: Outcome,
    /// What decided the outcome: `tested_value` or `if_denominator_not_positive`; null when the
    /// covenant is not tested because of an overflow (`not_tested_code` `overflow`).
    pub outcome_basis: Option<OutcomeBasis>,
    /// The value compared with the threshold: numerator / denominator to the 28 significant
    /// digits of the decimal type, or rounded as `rounding` states; in the unit of the
    /// threshold. Null when the denominator is not positive or the covenant is not tested.
    pub value: Option<Decimal>,
    /// With `rounding`: numerator / denominator before the rounding, to 28 significant digits.
    /// Null without `rounding`, or when `value` is null.
    pub value_unrounded: Option<Decimal>,
    /// Distance of `value` from the threshold: positive on the compliant side, negative on the
    /// breach side; threshold − value for a maximum, value − threshold for a minimum. Zero at
    /// the threshold, where an inclusive operator passes and a strict one breaches. Null when
    /// `value` is null.
    pub headroom: Option<Decimal>,
    /// Headroom / threshold, decimal fraction (0.10 = 10% of the threshold), same sign as the
    /// headroom; not defined when the threshold is zero or negative, when the quotient
    /// overflows, or when `value` is null.
    pub headroom_relative: RatioValue,
    /// Why the covenant was not tested: `denominator_not_positive`, `overflow` or
    /// `missing_input`; null when the outcome is `passed` or `breached`.
    pub not_tested_code: Option<NotTestedCode>,
    /// Why the covenant was not tested, in words, for example "denominator is zero:
    /// consolidated EBITDA for the 12 months ending on the test date"; null when the outcome is
    /// `passed` or `breached`.
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
    /// Outputs not computed because an amount is missing, in the order of the covenants, each
    /// with the JSON pointers of the amounts it lacks (SPEC-0007 R11).
    pub not_computed: Vec<NotComputedEntry>,
}

// ------------------------------------------------------------------ exact arithmetic

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

    fn add(&self, other: &Self) -> Self {
        let len = self.0.len().max(other.0.len());
        let mut out = Vec::with_capacity(len + 1);
        let mut carry = 0u128;
        for i in 0..len {
            let a = u128::from(self.0.get(i).copied().unwrap_or(0));
            let b = u128::from(other.0.get(i).copied().unwrap_or(0));
            let cur = a + b + carry;
            out.push(cur as u64);
            carry = cur >> 64;
        }
        out.push(carry as u64);
        Self(out).trimmed()
    }

    /// self − other, with self ≥ other.
    fn sub(&self, other: &Self) -> Self {
        let mut out = Vec::with_capacity(self.0.len());
        let mut borrow = 0u64;
        for (i, &a) in self.0.iter().enumerate() {
            let b = other.0.get(i).copied().unwrap_or(0);
            let (d1, o1) = a.overflowing_sub(b);
            let (d2, o2) = d1.overflowing_sub(borrow);
            out.push(d2);
            borrow = u64::from(o1 || o2);
        }
        Self(out).trimmed()
    }

    fn bits(&self) -> usize {
        self.0
            .last()
            .map_or(0, |top| self.0.len() * 64 - top.leading_zeros() as usize)
    }

    fn bit(&self, i: usize) -> bool {
        self.0
            .get(i / 64)
            .is_some_and(|limb| limb >> (i % 64) & 1 == 1)
    }

    /// (self / other, self mod other), other not zero; binary long division.
    fn div_rem(&self, other: &Self) -> (Self, Self) {
        let mut quotient = vec![0u64; self.0.len()];
        let mut rem = Self(Vec::new());
        for i in (0..self.bits()).rev() {
            rem = rem.add(&rem);
            if self.bit(i) {
                rem = rem.add(&Self::from_u128(1));
            }
            if rem.cmp(other) != Ordering::Less {
                rem = rem.sub(other);
                quotient[i / 64] |= 1 << (i % 64);
            }
        }
        (Self(quotient).trimmed(), rem)
    }

    fn to_u128(&self) -> Option<u128> {
        match self.0.as_slice() {
            [] => Some(0),
            [lo] => Some(u128::from(*lo)),
            [lo, hi] => Some(u128::from(*lo) | (u128::from(*hi) << 64)),
            _ => None,
        }
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

fn magnitude(d: Decimal) -> Wide {
    Wide::from_u128(d.mantissa().unsigned_abs())
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
    let td = magnitude(threshold).mul(&magnitude(denominator));
    let (scale_n, scale_td) = (numerator.scale(), threshold.scale() + denominator.scale());
    let common = scale_n.max(scale_td);
    let order = magnitude(numerator)
        .times_pow10(common - scale_n)
        .cmp(&td.times_pow10(common - scale_td));
    if sign_n == Ordering::Less {
        order.reverse()
    } else {
        order
    }
}

/// numerator / denominator rounded to `rounding.decimal_places` with its mode, computed from the
/// exact rational value (no intermediate 28-digit quotient, so no double rounding). The
/// denominator is positive. `None` when the rounded value exceeds the decimal type.
pub(crate) fn round_exact(
    numerator: Decimal,
    denominator: Decimal,
    rounding: Rounding,
) -> Option<Decimal> {
    let places = rounding.decimal_places;
    // |n| / d = (Mn / 10^sn) / (Md / 10^sd); scaled by 10^places:
    // |n| × 10^places / d = Mn × 10^(sd + places) / (Md × 10^sn).
    let top = magnitude(numerator).times_pow10(denominator.scale() + places);
    let bottom = magnitude(denominator).times_pow10(numerator.scale());
    let (quotient, rem) = top.div_rem(&bottom);
    let twice = rem.add(&rem);
    let round_up = match twice.cmp(&bottom) {
        Ordering::Greater => true,
        Ordering::Less => false,
        Ordering::Equal => match rounding.mode {
            RoundingMode::HalfUp => true,
            RoundingMode::HalfEven => quotient.bit(0),
        },
    };
    let quotient = if round_up {
        quotient.add(&Wide::from_u128(1))
    } else {
        quotient
    };
    // The rounded value is quotient / 10^places. Trailing zeros carry no value: drop them,
    // lowering the scale, until the mantissa fits the 96 bits of the decimal type. Only a
    // rounded value that still does not fit exceeds the decimal range.
    let ten = Wide::from_u128(10);
    let (mut quotient, mut scale) = (quotient, places);
    while quotient.cmp(&Wide::from_u128(MAX_MANTISSA)) == Ordering::Greater && scale > 0 {
        let (q, r) = quotient.div_rem(&ten);
        if !r.0.is_empty() {
            return None;
        }
        quotient = q;
        scale -= 1;
    }
    let mantissa = i128::try_from(quotient.to_u128()?).ok()?;
    if mantissa > MAX_MANTISSA as i128 {
        return None;
    }
    let signed = if numerator.is_sign_negative() {
        -mantissa
    } else {
        mantissa
    };
    Decimal::try_from_i128_with_scale(signed, scale).ok()
}

/// Largest mantissa of the decimal type, 2^96 − 1.
const MAX_MANTISSA: u128 = (1 << 96) - 1;

// ------------------------------------------------------------------ method

fn invalid(message: String) -> FairmarkError {
    FairmarkError::InvalidInput(message)
}

/// Refuses inputs that state nothing: an empty list, blank names and definitions, and a
/// rounding to more decimal places than supported.
fn validate(input: &CovenantsInput) -> Result<()> {
    if input.covenants.is_empty() {
        return Err(
            invalid("`covenants` is empty: give at least one covenant to test".into())
                .at("/covenants", Reason::Length)
                .fix("give at least one covenant to test"),
        );
    }
    // Every input error is reported (ADR 0014 G5), in the order of the checks.
    let mut found = Found::new();
    for (i, c) in input.covenants.iter().enumerate() {
        if c.name.trim().is_empty() {
            found.push(
                invalid(format!(
                    "`covenants[{i}].name` is empty: name the covenant as in the loan \
                     agreement, for example \"net leverage\""
                ))
                .at(format!("/covenants/{i}/name"), Reason::Blank)
                .fix("name the covenant as in the loan agreement"),
            );
        }
        for (side, part) in [("numerator", &c.numerator), ("denominator", &c.denominator)] {
            if part.definition.trim().is_empty() {
                found.push(
                    invalid(format!(
                        "`covenants[{i}].{side}.definition` is empty: state the {side} as the \
                         loan agreement defines it"
                    ))
                    .at(format!("/covenants/{i}/{side}/definition"), Reason::Blank)
                    .fix(format!("state the {side} as the loan agreement defines it")),
                );
            }
        }
        if let Some(r) = c.rounding {
            if r.decimal_places > MAX_DECIMAL_PLACES {
                found.push(
                    invalid(format!(
                        "`covenants[{i}].rounding.decimal_places` is {}: give 0 to \
                         {MAX_DECIMAL_PLACES}, the decimal places stated by the loan agreement",
                        r.decimal_places
                    ))
                    .at(
                        format!("/covenants/{i}/rounding/decimal_places"),
                        Reason::Range,
                    )
                    .expected([format!("a whole number from 0 to {MAX_DECIMAL_PLACES}")]),
                );
            }
        }
    }
    found.finish()
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

/// Not tested because an amount is missing (R11): the outputs are null and listed.
fn not_tested_missing(
    ex: &mut Explain,
    label: &str,
    result: CovenantResult,
    missing: &Missing,
) -> CovenantResult {
    let side = |s: &str| missing.iter().any(|p| p.ends_with(&format!("/{s}/amount")));
    let reason = match (side("numerator"), side("denominator")) {
        (true, true) => "numerator and denominator amounts not given",
        (true, false) => "numerator amount not given",
        _ => "denominator amount not given",
    }
    .to_owned();
    let result = CovenantResult {
        headroom_relative: RatioValue {
            value: None,
            not_defined: None,
            missing: missing.clone(),
            lacking: missing.clone(),
        },
        ..result
    };
    not_tested(ex, label, result, NotTestedCode::MissingInput, reason)
}

const OVERFLOW_VALUE: &str =
    "the tested value exceeds the range of the decimal type (28-29 significant digits)";
const OVERFLOW_ROUNDED: &str =
    "the rounded tested value exceeds the range of the decimal type (28-29 significant digits)";
const OVERFLOW_HEADROOM: &str =
    "the headroom exceeds the range of the decimal type (28-29 significant digits)";

/// Tests the covenant at position `i` (numbered from 1 in the audit trail); also returns the
/// amounts it lacks, empty when none.
fn test(i: usize, c: &Covenant, ex: &mut Explain) -> (CovenantResult, Missing) {
    let k = i + 1;
    let label = format!("covenant {k}, {} ({})", c.name, c.test_date);
    let op = c.operator;
    let threshold = c.threshold;
    let rule = c.if_denominator_not_positive;
    ex.convention(format!(
        "{label}: passes when the tested value {} {threshold} ({})",
        op.symbol(),
        op.describe()
    ));
    let rule_name = match rule {
        IfDenominatorNotPositive::Breached => "breached",
        IfDenominatorNotPositive::Passed => "passed",
        IfDenominatorNotPositive::NotTested => "not tested",
    };
    ex.convention(format!(
        "{label}: a zero or negative denominator gives the outcome {rule_name} \
         (input if_denominator_not_positive)"
    ));
    match c.rounding {
        Some(r) => ex.convention(format!(
            "{label}: tested value rounded to {} decimal places, {} ({}), before the comparison \
             (input rounding)",
            r.decimal_places,
            r.mode.name(),
            r.mode.describe()
        )),
        None => ex.convention(format!(
            "{label}: tested value not rounded (no rounding given); outcome decided on the \
             exact value"
        )),
    }
    let mut missing = Missing::new();
    for (side, part) in [("numerator", &c.numerator), ("denominator", &c.denominator)] {
        match part.amount {
            Some(amount) => ex.step(
                format!("{label}: {side}"),
                format!("as given: {}", part.definition),
                amount,
            ),
            None => {
                missing.insert(format!("/covenants/{i}/{side}/amount"));
            }
        }
    }
    let result = CovenantResult {
        name: c.name.clone(),
        test_date: c.test_date,
        operator: op,
        threshold,
        outcome: Outcome::NotTested,
        outcome_basis: None,
        value: None,
        value_unrounded: None,
        headroom: None,
        headroom_relative: RatioValue::undefined("no tested value".into()),
        not_tested_code: None,
        not_tested_reason: None,
    };
    let d = match (c.denominator.amount, missing.is_empty()) {
        (Some(d), true) => d,
        (Some(d), false) if d.is_zero() || d.is_sign_negative() => d,
        _ => return (not_tested_missing(ex, &label, result, &missing), missing),
    };
    // A numerator not given was handled above unless the denominator is not positive, where
    // the rule decides without it.
    let numerator = c.numerator.amount;

    if d.is_zero() || d.is_sign_negative() {
        let reason = if d.is_zero() {
            format!("denominator is zero: {}", c.denominator.definition)
        } else {
            format!(
                "denominator is negative ({d}): {}",
                c.denominator.definition
            )
        };
        let relative = RatioValue::undefined(format!("no tested value: {reason}"));
        let result = CovenantResult {
            outcome_basis: Some(OutcomeBasis::IfDenominatorNotPositive),
            ..result
        };
        ex.step(
            format!("{label}: outcome"),
            format!("{rule_name} ({reason}; outcome set by input if_denominator_not_positive)"),
            d,
        );
        let outcome = match rule {
            IfDenominatorNotPositive::NotTested => {
                return (
                    not_tested(
                        ex,
                        &label,
                        CovenantResult {
                            headroom_relative: relative,
                            ..result
                        },
                        NotTestedCode::DenominatorNotPositive,
                        reason,
                    ),
                    Missing::new(),
                );
            }
            IfDenominatorNotPositive::Breached => Outcome::Breached,
            IfDenominatorNotPositive::Passed => Outcome::Passed,
        };
        return (
            CovenantResult {
                outcome,
                headroom_relative: relative,
                ..result
            },
            Missing::new(),
        );
    }

    let Some(n) = numerator else {
        return (not_tested_missing(ex, &label, result, &missing), missing);
    };
    let Ok(unrounded) = num::div(n, d) else {
        let reason = OVERFLOW_VALUE.to_owned();
        return (
            not_tested(ex, &label, result, NotTestedCode::Overflow, reason),
            Missing::new(),
        );
    };
    ex.step(
        format!("{label}: tested value"),
        format!("numerator / denominator = {n} / {d}"),
        unrounded,
    );
    let (value, value_unrounded) = match c.rounding {
        None => (unrounded, None),
        Some(r) => {
            let Some(rounded) = round_exact(n, d, r) else {
                let reason = OVERFLOW_ROUNDED.to_owned();
                return (
                    not_tested(ex, &label, result, NotTestedCode::Overflow, reason),
                    Missing::new(),
                );
            };
            ex.step(
                format!("{label}: rounded tested value"),
                format!(
                    "{n} / {d} ({unrounded}) rounded to {} decimal places, {}, from the exact \
                     quotient",
                    r.decimal_places,
                    r.mode.name()
                ),
                rounded,
            );
            (rounded, Some(unrounded))
        }
    };
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
        return (
            not_tested(ex, &label, result, NotTestedCode::Overflow, reason),
            Missing::new(),
        );
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

    // The decision, exact: on numerator − threshold × denominator (denominator positive), or on
    // the rounded value, which is exact, against the threshold.
    let (order, compared) = match value_unrounded {
        None => (exact_cmp(n, threshold, d), format!("{n} / {d}")),
        Some(_) => (
            value.cmp(&threshold),
            format!("rounded tested value {value}"),
        ),
    };
    let passed = op.passes(order);
    let (outcome, verdict) = if passed {
        (Outcome::Passed, "passed")
    } else {
        (Outcome::Breached, "breached")
    };
    ex.step(
        format!("{label}: outcome"),
        format!(
            "{verdict} ({compared} {} {threshold} is {passed}, decided exactly)",
            op.name()
        ),
        value,
    );
    // The headroom shown has 28 significant digits; its sign can disagree with the exact
    // position of the tested value (for example 0 shown for a value just past the threshold).
    let exact_side = if op.is_maximum() {
        order.reverse()
    } else {
        order
    };
    let shown_side = sign(headroom);
    if shown_side != exact_side {
        let shown = if shown_side == Ordering::Equal {
            "rounds to 0".to_owned()
        } else {
            format!("({headroom}) has the other sign")
        };
        let note = format!(
            "displayed headroom {shown} at 28 significant digits; the exact comparison decides \
             {verdict}"
        );
        ex.convention(format!("{label}: {note}"));
        ex.step(format!("{label}: display note"), note, headroom);
    }
    (
        CovenantResult {
            outcome,
            outcome_basis: Some(OutcomeBasis::TestedValue),
            value: Some(value),
            value_unrounded,
            headroom: Some(headroom),
            headroom_relative,
            ..result
        },
        Missing::new(),
    )
}

pub fn covenants(input: &CovenantsInput) -> Result<(CovenantsOutput, Explain)> {
    validate(input)?;
    let mut ex = Explain::new(
        Covenants::ID,
        "tested value = numerator / denominator (loan agreement definitions), rounded only as \
         the agreement states; outcome decided exactly; headroom = threshold − tested value \
         (maximum) or tested value − threshold (minimum); relative headroom = headroom / \
         threshold",
    );
    ex.convention(
        "tested value = numerator / denominator, each as defined by the loan agreement and \
         given as an amount; no standard ratio is substituted",
    );
    ex.convention("the amounts of each covenant refer to the period ending on its test date");
    ex.convention(
        "the outcome is decided exactly: on the exact tested value, or on the value rounded as \
         the agreement states when rounding is given; value and headroom are shown to the 28 \
         significant digits of the decimal type, and are rounded to the decimals of the \
         agreement only when rounding is given",
    );
    ex.convention(
        "exact decision without rounding: the sign of numerator − threshold × denominator, \
         computed in integer arithmetic on the decimal mantissas, with a positive denominator; \
         with rounding: the rounded value, computed from the exact quotient, compared with the \
         threshold",
    );
    ex.convention(
        "a zero or negative denominator gives the outcome set by if_denominator_not_positive; \
         value and headroom are then null",
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
        "outcome_basis of each covenant: tested_value when the tested value decides the \
         outcome, if_denominator_not_positive when the input rule does; null on overflow",
    );
    ex.convention(
        "a covenant whose tested value, rounded value or headroom exceeds the range of the \
         decimal type is not tested, never breached; the other covenants are tested",
    );
    ex.convention(MISSING_CONVENTION);
    ex.convention(
        "a covenant whose denominator amount, or whose numerator amount with a positive \
         denominator, is not given is not tested (missing_input), never breached; a \
         denominator given zero or negative gives the outcome of if_denominator_not_positive \
         whatever the numerator",
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
    let mut nc = NotComputedList::default();
    for (i, c) in input.covenants.iter().enumerate() {
        let (r, missing) = test(i, c, &mut ex);
        if !missing.is_empty() {
            for output in ["value", "headroom", "headroom_relative"] {
                nc.push(format!("/covenants/{i}/{output}"), &missing);
            }
        }
        // A covenant tested, or decided by its rule, has every input it needs (R12).
        nc.observe(&missing);
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
            not_computed: nc.finish(Covenants::ID)?,
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
        least, greater than) at a test date. The agreement's own rules are explicit inputs: the \
        outcome when the denominator is zero or negative, and the rounding of the tested \
        value. Returns for each covenant the outcome (passed, breached, or not tested with a \
        code and the reason), decided exactly, the tested value and the headroom, absolute and \
        relative to the threshold. The amounts are optional: a covenant without the amounts \
        it needs is not tested (missing_input), never breached, and listed in not_computed.";
    const VERSION: &'static str = "2";
    type Input = CovenantsInput;
    type Output = CovenantsOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        validate(input)
    }

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
            amount: Some(v),
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
            if_denominator_not_positive: IfDenominatorNotPositive::NotTested,
            rounding: None,
        }
    }

    fn covenant(num: i64, den: i64, operator: Operator, threshold: Decimal) -> Covenant {
        cov(d(num), d(den), operator, threshold)
    }

    fn rounded(c: Covenant, decimal_places: u32, mode: RoundingMode) -> Covenant {
        Covenant {
            rounding: Some(Rounding {
                decimal_places,
                mode,
            }),
            ..c
        }
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
    fn round_exact_follows_the_mode_on_the_exact_quotient() {
        let r = |n: &str, den: &str, places, mode| {
            round_exact(
                dec(n),
                dec(den),
                Rounding {
                    decimal_places: places,
                    mode,
                },
            )
            .unwrap()
        };
        use RoundingMode::{HalfEven, HalfUp};
        assert_eq!(r("3504", "1000", 2, HalfUp), dec("3.50"));
        assert_eq!(r("3505", "1000", 2, HalfUp), dec("3.51"));
        assert_eq!(r("3505", "1000", 2, HalfEven), dec("3.50"));
        assert_eq!(r("3515", "1000", 2, HalfEven), dec("3.52"));
        assert_eq!(r("-3505", "1000", 2, HalfUp), dec("-3.51"));
        assert_eq!(r("-3505", "1000", 2, HalfEven), dec("-3.50"));
        assert_eq!(r("2", "3", 6, HalfUp), dec("0.666667"));
        assert_eq!(r("7", "2", 0, HalfEven), dec("4"));
        assert_eq!(r("5", "2", 0, HalfEven), dec("2"));
        assert_eq!(r("1.5", "0.5", 0, HalfUp), dec("3"));
        // No double rounding: the exact quotient 0.00499999...9667 is below the half, while its
        // 28-digit rounding 0.0050000000000000000000000000 is on it.
        let n = dec("0.0149999999999999999999999999");
        assert_eq!(
            num::div(n, d(3)).unwrap(),
            dec("0.0050000000000000000000000000")
        );
        assert_eq!(
            r("0.0149999999999999999999999999", "3", 2, HalfUp),
            dec("0.00")
        );
        // Beyond the decimal type: 90000000000000000000000.333333 needs a mantissa above
        // 2^96 − 1 and has no trailing zero to drop.
        assert_eq!(
            round_exact(
                dec("270000000000000000000001"),
                d(3),
                Rounding {
                    decimal_places: 6,
                    mode: HalfUp
                }
            ),
            None
        );
    }

    #[test]
    fn contractual_rounding_decides_the_outcome_on_the_rounded_value() {
        let c = covenant(3504, 1000, Operator::AtMost, dec("3.50"));
        let r = one(c.clone());
        assert_eq!(r.outcome, Outcome::Breached);
        assert_eq!(r.value, Some(dec("3.504")));
        assert_eq!(r.value_unrounded, None);
        assert_eq!(r.headroom, Some(dec("-0.004")));
        let r = one(rounded(c, 2, RoundingMode::HalfUp));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.value, Some(dec("3.50")));
        assert_eq!(r.value_unrounded, Some(dec("3.504")));
        assert_eq!(r.headroom, Some(Decimal::ZERO));
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
    fn denominator_not_positive_follows_the_input() {
        for (den, rule, outcome) in [
            (0, IfDenominatorNotPositive::NotTested, Outcome::NotTested),
            (
                -100,
                IfDenominatorNotPositive::NotTested,
                Outcome::NotTested,
            ),
            (0, IfDenominatorNotPositive::Breached, Outcome::Breached),
            (-100, IfDenominatorNotPositive::Breached, Outcome::Breached),
            (0, IfDenominatorNotPositive::Passed, Outcome::Passed),
            (-100, IfDenominatorNotPositive::Passed, Outcome::Passed),
        ] {
            let mut c = covenant(300, den, Operator::AtMost, d(3));
            c.if_denominator_not_positive = rule;
            let r = one(c);
            assert_eq!(r.outcome, outcome, "{den} {rule:?}");
            assert_eq!((r.value, r.headroom), (None, None));
            assert_eq!(
                r.outcome_basis,
                Some(OutcomeBasis::IfDenominatorNotPositive)
            );
            let tested = outcome != Outcome::NotTested;
            assert_eq!(r.not_tested_code.is_none(), tested);
        }
        let mut c = covenant(300, -100, Operator::AtMost, d(3));
        c.if_denominator_not_positive = IfDenominatorNotPositive::NotTested;
        let r = one(c);
        assert_eq!(
            r.not_tested_code,
            Some(NotTestedCode::DenominatorNotPositive)
        );
        assert_eq!(
            r.not_tested_reason.as_deref(),
            Some("denominator is negative (-100): EBITDA")
        );
    }

    #[test]
    fn zero_denominator_not_tested_never_breaches_and_others_continue() {
        let (out, ex) = run(vec![
            covenant(300, 0, Operator::AtMost, d(3)),
            covenant(300, 100, Operator::AtMost, d(2)),
        ])
        .unwrap();
        let r = &out.covenants[0];
        assert_eq!(r.outcome, Outcome::NotTested);
        assert_eq!(
            r.not_tested_code,
            Some(NotTestedCode::DenominatorNotPositive)
        );
        assert_eq!(
            r.not_tested_reason.as_deref(),
            Some("denominator is zero: EBITDA")
        );
        assert_eq!(
            r.headroom_relative.not_defined.as_deref(),
            Some("no tested value: denominator is zero: EBITDA")
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
    fn negative_zero_denominator_is_not_positive() {
        let r = one(cov(d(300), dec("-0"), Operator::AtMost, d(3)));
        assert_eq!(r.outcome, Outcome::NotTested);
        assert_eq!(
            r.not_tested_code,
            Some(NotTestedCode::DenominatorNotPositive)
        );
        assert_eq!(
            r.not_tested_reason.as_deref(),
            Some("denominator is zero: EBITDA")
        );
    }

    #[test]
    fn overflow_makes_one_covenant_not_tested_and_the_others_continue() {
        let tiny = dec("0.0000000000000000000000000001");
        let (out, _) = run(vec![
            cov(Decimal::MAX, tiny, Operator::AtMost, d(3)),
            cov(Decimal::MAX, d(1), Operator::AtMost, Decimal::MIN),
            rounded(
                cov(
                    dec("270000000000000000000001"),
                    d(3),
                    Operator::AtMost,
                    d(3),
                ),
                6,
                RoundingMode::HalfUp,
            ),
            covenant(300, 100, Operator::AtMost, d(4)),
        ])
        .unwrap();
        let reasons: Vec<_> = out.covenants[..3]
            .iter()
            .map(|r| {
                assert_eq!(r.not_tested_code, Some(NotTestedCode::Overflow));
                r.not_tested_reason.as_deref().unwrap()
            })
            .collect();
        assert_eq!(
            reasons,
            vec![OVERFLOW_VALUE, OVERFLOW_HEADROOM, OVERFLOW_ROUNDED]
        );
        assert!(out.covenants[..3].iter().all(|r| r.outcome_basis.is_none()));
        assert_eq!(out.covenants[3].outcome, Outcome::Passed);
        assert_eq!(
            out.covenants[3].outcome_basis,
            Some(OutcomeBasis::TestedValue)
        );
        assert_eq!(out.summary.not_tested, 3);
    }

    #[test]
    fn negative_numerator_is_compared_as_computed() {
        let r = one(covenant(-100, 200, Operator::AtMost, d(3)));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.value, Some(Decimal::new(-5, 1)));
        assert_eq!(r.headroom, Some(Decimal::new(35, 1)));
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
    fn invalid_inputs_are_refused_naming_the_field() {
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
        let c = rounded(
            covenant(300, 100, Operator::AtMost, d(3)),
            7,
            RoundingMode::HalfUp,
        );
        let err = run(vec![covenant(300, 100, Operator::AtMost, d(3)), c]).unwrap_err();
        assert_eq!(err.code(), "FM-E001");
        assert!(err
            .to_string()
            .contains("`covenants[1].rounding.decimal_places` is 7: give 0 to 6"));
    }

    #[test]
    fn explain_shows_the_numbers_the_rules_the_verdict_and_the_counts() {
        let mut breached_by_rule = covenant(600, -5, Operator::AtLeast, d(4));
        breached_by_rule.if_denominator_not_positive = IfDenominatorNotPositive::Breached;
        let (_, ex) = run(vec![
            covenant(1900, 600, Operator::AtMost, Decimal::new(35, 1)),
            covenant(600, 175, Operator::AtLeast, d(4)),
            rounded(
                covenant(3504, 1000, Operator::AtMost, dec("3.50")),
                2,
                RoundingMode::HalfUp,
            ),
            breached_by_rule,
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
        let third = "covenant 3, net leverage (2026-12-31)";
        let rounding = step(&format!("{third}: rounded tested value"));
        assert_eq!(
            rounding.formula,
            "3504 / 1000 (3.504) rounded to 2 decimal places, half_up, from the exact quotient"
        );
        assert_eq!(rounding.value, dec("3.50"));
        assert_eq!(
            step(&format!("{third}: outcome")).formula,
            "passed (rounded tested value 3.50 at_most 3.50 is true, decided exactly)"
        );
        assert_eq!(
            step("covenant 4, net leverage (2026-12-31): outcome").formula,
            "breached (denominator is negative (-5): EBITDA; outcome set by input \
             if_denominator_not_positive)"
        );
        assert!(ex.conventions.iter().any(|c| c
            == "covenant 4, net leverage (2026-12-31): a zero or negative denominator gives the \
                outcome breached (input if_denominator_not_positive)"));
        assert_eq!(step("Covenants passed").value, d(2));
        assert_eq!(step("Covenants breached").value, d(2));
        assert_eq!(step("Covenants not tested").value, d(0));
    }

    #[test]
    fn rounding_at_the_decimal_boundary_lowers_the_scale_instead_of_overflowing() {
        use RoundingMode::HalfUp;
        let max = Decimal::MAX;
        for places in [0, 6] {
            let r = one(rounded(
                cov(max, d(1), Operator::AtMost, max),
                places,
                HalfUp,
            ));
            assert_eq!(r.outcome, Outcome::Passed, "{places} places");
            assert_eq!(r.value, Some(max));
            assert_eq!(r.not_tested_code, None);
        }
        let big = dec("79228162514264337593544");
        let r = one(rounded(cov(big, d(1), Operator::AtMost, big), 6, HalfUp));
        assert_eq!(r.outcome, Outcome::Passed);
        assert_eq!(r.value, Some(big));
        // A rounded value whose own digits exceed 96 bits still overflows:
        // 90000000000000000000000.333333 needs 29 significant digits above 2^96.
        let r = one(rounded(
            cov(
                dec("270000000000000000000001"),
                d(3),
                Operator::AtMost,
                d(1),
            ),
            6,
            HalfUp,
        ));
        assert_eq!(r.not_tested_code, Some(NotTestedCode::Overflow));
    }

    #[test]
    fn a_displayed_zero_headroom_is_explained_when_the_exact_comparison_differs() {
        // (2^96 − 3) / 2 = 39614081257132168796771975166.5: shown as the threshold.
        let t = dec("39614081257132168796771975166");
        let (out, ex) = run(vec![cov(
            dec("79228162514264337593543950333"),
            d(2),
            Operator::AtMost,
            t,
        )])
        .unwrap();
        let r = &out.covenants[0];
        assert_eq!(r.outcome, Outcome::Breached);
        assert_eq!(r.headroom, Some(Decimal::ZERO));
        let note = "covenant 1, net leverage (2026-12-31): displayed headroom rounds to 0 at 28 \
                    significant digits; the exact comparison decides breached";
        assert!(
            ex.conventions.iter().any(|c| c == note),
            "{:?}",
            ex.conventions
        );
        assert!(ex
            .steps
            .iter()
            .any(|s| s.label == "covenant 1, net leverage (2026-12-31): display note"));
        // 1 / MAX > 0: shown as 0, passed.
        let (out, ex) = run(vec![cov(d(1), Decimal::MAX, Operator::GreaterThan, d(0))]).unwrap();
        assert_eq!(out.covenants[0].outcome, Outcome::Passed);
        assert_eq!(out.covenants[0].value, Some(Decimal::ZERO));
        assert!(ex.conventions.iter().any(|c| c.ends_with(
            "displayed headroom rounds to 0 at 28 significant digits; the exact comparison \
             decides passed"
        )));
        // No note when the display agrees.
        let (_, ex) = run(vec![covenant(300, 100, Operator::AtMost, d(3))]).unwrap();
        assert!(!ex
            .conventions
            .iter()
            .any(|c| c.contains("displayed headroom")));
    }

    #[test]
    fn not_tested_by_the_rule_has_a_verdict_step() {
        let (_, ex) = run(vec![covenant(300, 0, Operator::AtMost, d(3))]).unwrap();
        let step = ex
            .steps
            .iter()
            .find(|s| s.label == "covenant 1, net leverage (2026-12-31): outcome")
            .unwrap();
        assert_eq!(
            step.formula,
            "not tested (denominator is zero: EBITDA; outcome set by input \
             if_denominator_not_positive)"
        );
    }
}
