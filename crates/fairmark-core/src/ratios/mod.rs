//! Financial ratios organised by the question the analyst asks (SPEC-0007).
//!
//! Each group is one method: `ratios.profitability`, `ratios.capital_structure`,
//! `ratios.liquidity`, `ratios.working_capital` and `ratios.debt_service`. They share the
//! pieces of this module. `ratios.covenants` tests covenants on the definitions of the loan
//! agreement and uses only [`RatioValue`]:
//!
//! - [`Balances`]: closing balances, or the average of opening and closing (R2);
//! - [`DayBasis`]: 360 or 365 days for day-based ratios (R2);
//! - [`RatioValue`]: a ratio, or the reason it is not defined (R3), or not computed (R7);
//! - the [`Ratio`] trait (ADR 0009, point 5) with its two implementors [`Quotient`] and
//!   [`DayRatio`]: formula, numerator, denominator and not-defined handling in one place;
//! - `Given`: a term, or the input pointers it lacks, and `NotComputedList`, the
//!   `not_computed` list of every output (R7, R10);
//! - [`NfpComposition`]: the net financial position with its composition, every amount
//!   optional (R6, R7); [`NetFinancialPosition`], with required amounts, stays the type of the
//!   `dcf.*` inputs.
//!
//! A missing input is never taken as zero (R7): an output that lacks an input is null and
//! listed in `not_computed` with the input pointers it lacks.

pub mod capital_structure;
pub mod covenants;
pub mod debt_service;
pub mod liquidity;
pub mod profitability;
pub mod working_capital;

use std::collections::BTreeSet;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, MinimumInput, Result};
use crate::explain::{DefaultApplied, Explain};
use crate::not_computed::NotComputedEntry;
use crate::num;

/// Which balance-sheet amounts the ratios use (SPEC-0007 R2). No default: the analyst chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum Balances {
    /// Balances at the end of the period (`closing`).
    Closing,
    /// Average of the opening and closing balances: (opening + closing) / 2. Needs `opening`.
    Average,
}

/// Days in the year for day-based ratios (SPEC-0007 R2). No default: the analyst chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum DayBasis {
    /// A 360-day year.
    #[serde(rename = "360")]
    Days360,
    /// A 365-day year.
    #[serde(rename = "365")]
    Days365,
}

impl DayBasis {
    pub(crate) fn days(self) -> Decimal {
        match self {
            Self::Days360 => Decimal::from(360),
            Self::Days365 => Decimal::from(365),
        }
    }
}

/// A ratio, or the reason it is not defined (SPEC-0007 R3), or not computed (R7).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct RatioValue {
    /// The ratio, in the unit stated by the field that holds it (decimal fraction, multiple or
    /// days); null when the ratio is not defined or not computed.
    pub value: Option<Decimal>,
    /// Why the ratio is not defined, for example "equity is negative (-100)"; null when it is
    /// defined, and when it is not computed for a missing input (listed in `not_computed`).
    pub not_defined: Option<String>,
    /// Input pointers the ratio lacks; empty when it is computed or not defined. Reported in
    /// `not_computed`, not in the ratio.
    #[serde(skip)]
    #[schemars(skip)]
    pub(crate) missing: BTreeSet<String>,
    /// Every input pointer the ratio lacks, also when a given term already makes it not defined
    /// (R10); empty when the ratio has every input it needs. Used by the floor (R12).
    #[serde(skip)]
    #[schemars(skip)]
    pub(crate) lacking: BTreeSet<String>,
}

impl RatioValue {
    pub(crate) fn defined(value: Decimal) -> Self {
        Self {
            value: Some(value),
            not_defined: None,
            missing: BTreeSet::new(),
            lacking: BTreeSet::new(),
        }
    }

    /// Records the reason in the conventions, so the audit trail says why a ratio is missing.
    pub(crate) fn not_defined(ex: &mut Explain, label: &str, reason: String) -> Self {
        ex.convention(format!("{label} not defined: {reason}"));
        Self::undefined(reason)
    }

    /// Not defined, without a convention (the caller records one for a group of ratios).
    pub(crate) fn undefined(reason: String) -> Self {
        Self {
            value: None,
            not_defined: Some(reason),
            missing: BTreeSet::new(),
            lacking: BTreeSet::new(),
        }
    }

    /// Not defined by a given term although `lacking` inputs are also missing (R10): the ratio
    /// is not listed, but it does not count as complete for the floor (R12).
    pub(crate) fn lacking(self, lacking: BTreeSet<String>) -> Self {
        Self { lacking, ..self }
    }

    /// Not defined because the block `pointer` is not given (the leverage inputs, the debt
    /// schedule): the version 1 reason, and the block listed in `not_computed` (R7).
    pub(crate) fn block_not_given(reason: &str, pointer: &str) -> Self {
        let block = BTreeSet::from([pointer.to_owned()]);
        Self {
            missing: block.clone(),
            lacking: block,
            ..Self::undefined(reason.to_owned())
        }
    }

    /// Not computed because inputs are missing (R7); the conventions name them.
    pub(crate) fn not_computed(ex: &mut Explain, label: &str, missing: BTreeSet<String>) -> Self {
        ex.convention(format!(
            "{label} not computed: {} not given",
            ordered(&missing).join(", ")
        ));
        Self {
            value: None,
            not_defined: None,
            lacking: missing.clone(),
            missing,
        }
    }

    fn is_not_defined(&self) -> bool {
        self.value.is_none() && self.missing.is_empty()
    }
}

/// The input pointers an output lacks (R7), without duplicates.
pub(crate) type Missing = BTreeSet<String>;

/// Order of JSON pointers in every list of the result: token by token, array indices compared
/// as numbers and before names (`/periods/2` before `/periods/10`).
pub(crate) fn pointer_cmp(a: &str, b: &str) -> std::cmp::Ordering {
    fn key(t: &str) -> (u8, u64, &str) {
        match t.parse::<u64>() {
            Ok(n) if !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit()) => (0, n, ""),
            _ => (1, 0, t),
        }
    }
    a.split('/').map(key).cmp(b.split('/').map(key))
}

/// The pointers of `set` in the order of [`pointer_cmp`].
pub(crate) fn ordered(set: &Missing) -> Vec<String> {
    let mut v: Vec<String> = set.iter().cloned().collect();
    v.sort_by(|a, b| pointer_cmp(a, b));
    v
}

/// A named amount entering a ratio, already resolved under the balance convention.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Term {
    pub name: &'static str,
    pub value: Decimal,
}

/// A term, or the input pointers it lacks (R7).
pub(crate) type Given = std::result::Result<Term, Missing>;

pub(crate) fn term(name: &'static str, value: Decimal) -> Given {
    Ok(Term { name, value })
}

/// The input at `pointer`, or the pointer as missing.
pub(crate) fn given(name: &'static str, value: Option<Decimal>, pointer: &str) -> Given {
    value
        .map(|value| Term { name, value })
        .ok_or_else(|| BTreeSet::from([pointer.to_owned()]))
}

/// Every pointer missing among `parts`.
pub(crate) fn union<'a>(parts: impl IntoIterator<Item = &'a Given>) -> Missing {
    parts
        .into_iter()
        .filter_map(|p| p.as_ref().err())
        .flatten()
        .cloned()
        .collect()
}

/// An amount computed from given terms, or the pointers it lacks (margins, sums).
pub(crate) fn combine<const N: usize>(
    name: &'static str,
    parts: [&Given; N],
    f: impl FnOnce([Decimal; N]) -> Result<Decimal>,
) -> Result<Given> {
    let missing = union(parts);
    if !missing.is_empty() {
        return Ok(Err(missing));
    }
    let mut values = [Decimal::ZERO; N];
    for (slot, part) in values.iter_mut().zip(parts) {
        *slot = known(part)?.value;
    }
    Ok(term(name, f(values)?))
}

/// The `Term` of a given value; only called after the missing ones were handled.
fn known(g: &Given) -> Result<Term> {
    g.as_ref()
        .copied()
        .map_err(|_| FairmarkError::Internal("a missing term was used"))
}

/// One ratio: its formula, numerator, denominator and not-defined handling (ADR 0009, point 5).
/// The balance convention is applied before, when the terms are resolved.
pub(crate) trait Ratio {
    /// Label in the audit trail, for example "ROE".
    fn label(&self) -> &str;
    /// Formula in words, for example "net income / equity".
    fn formula(&self) -> &str;
    fn numerator(&self) -> &Given;
    fn denominator(&self) -> &Given;
    /// Days multiplying the quotient, for day-based ratios.
    fn days(&self) -> Option<Decimal> {
        None
    }
    /// Whether a negative denominator makes the ratio not meaningful (for example ratios over
    /// equity, NFP / EBITDA, DSCR).
    fn needs_positive_denominator(&self) -> bool {
        false
    }
    /// Whether a numerator at or below zero makes the ratio not meaningful (interest covers).
    fn needs_positive_numerator(&self) -> bool {
        false
    }

    /// The ratio with its explain step; not defined with the reason when a given term makes
    /// it so (R3, R10); otherwise not computed when a term is missing (R7).
    fn evaluate(&self, ex: &mut Explain) -> Result<RatioValue> {
        let (n, d) = (self.numerator(), self.denominator());
        let lacking = union([n, d]);
        if let Ok(d) = d {
            if d.value.is_zero() {
                let reason = format!("{} is zero", d.name);
                return Ok(RatioValue::not_defined(ex, self.label(), reason).lacking(lacking));
            }
            if self.needs_positive_denominator() && d.value.is_sign_negative() {
                let reason = format!("{} is negative ({})", d.name, d.value);
                return Ok(RatioValue::not_defined(ex, self.label(), reason).lacking(lacking));
            }
        }
        if let (true, Ok(n)) = (self.needs_positive_numerator(), n) {
            if n.value.is_zero() {
                let reason = format!("{} is zero", n.name);
                return Ok(RatioValue::not_defined(ex, self.label(), reason).lacking(lacking));
            }
            if n.value.is_sign_negative() {
                let reason = format!("{} is negative ({})", n.name, n.value);
                return Ok(RatioValue::not_defined(ex, self.label(), reason).lacking(lacking));
            }
        }
        if !lacking.is_empty() {
            return Ok(RatioValue::not_computed(ex, self.label(), lacking));
        }
        let (n, d) = (known(n)?, known(d)?);
        let (value, substituted) = match self.days() {
            None => (
                num::div(n.value, d.value)?,
                format!("{} / {}", n.value, d.value),
            ),
            Some(days) => (
                num::div(num::mul(n.value, days)?, d.value)?,
                format!("{} × {days} / {}", n.value, d.value),
            ),
        };
        ex.step(
            self.label(),
            format!("{} = {substituted}", self.formula()),
            value,
        );
        Ok(RatioValue::defined(value))
    }
}

/// numerator / denominator.
pub(crate) struct Quotient {
    pub label: &'static str,
    pub formula: &'static str,
    pub numerator: Given,
    pub denominator: Given,
    /// Not defined when the denominator is negative (zero is never defined).
    pub positive_denominator: bool,
    /// Not defined when the numerator is zero or negative.
    pub positive_numerator: bool,
}

impl Ratio for Quotient {
    fn label(&self) -> &str {
        self.label
    }
    fn formula(&self) -> &str {
        self.formula
    }
    fn numerator(&self) -> &Given {
        &self.numerator
    }
    fn denominator(&self) -> &Given {
        &self.denominator
    }
    fn needs_positive_denominator(&self) -> bool {
        self.positive_denominator
    }
    fn needs_positive_numerator(&self) -> bool {
        self.positive_numerator
    }
}

/// numerator × days / denominator.
pub(crate) struct DayRatio {
    pub label: &'static str,
    pub formula: &'static str,
    pub numerator: Given,
    pub denominator: Given,
    pub basis: DayBasis,
}

impl Ratio for DayRatio {
    fn label(&self) -> &str {
        self.label
    }
    fn formula(&self) -> &str {
        self.formula
    }
    fn numerator(&self) -> &Given {
        &self.numerator
    }
    fn denominator(&self) -> &Given {
        &self.denominator
    }
    fn days(&self) -> Option<Decimal> {
        Some(self.basis.days())
    }
}

/// `numerator / denominator` with the equity rule off.
pub(crate) fn quotient(
    label: &'static str,
    formula: &'static str,
    numerator: &Given,
    denominator: &Given,
) -> Quotient {
    Quotient {
        label,
        formula,
        numerator: numerator.clone(),
        denominator: denominator.clone(),
        positive_denominator: false,
        positive_numerator: false,
    }
}

/// `numerator / denominator`, not defined when the denominator is zero or negative.
pub(crate) fn over_positive(
    label: &'static str,
    formula: &'static str,
    numerator: &Given,
    denominator: &Given,
) -> Quotient {
    Quotient {
        positive_denominator: true,
        ..quotient(label, formula, numerator, denominator)
    }
}

/// `numerator / denominator`, not defined when either is zero or negative (interest covers).
pub(crate) fn both_positive(
    label: &'static str,
    formula: &'static str,
    numerator: &Given,
    denominator: &Given,
) -> Quotient {
    Quotient {
        positive_numerator: true,
        ..over_positive(label, formula, numerator, denominator)
    }
}

/// `numerator / equity`: not defined when equity is zero or negative.
pub(crate) fn over_equity(
    label: &'static str,
    formula: &'static str,
    numerator: &Given,
    equity: &Given,
) -> Quotient {
    over_positive(label, formula, numerator, equity)
}

/// The parts of a ratio built from other ratios (R10): their values; or the reason of the
/// first part not defined; or, when none is, the union of the pointers the parts lack.
pub(crate) enum Parts {
    Values(Vec<Decimal>),
    NotDefined(String),
    Missing(Missing),
}

pub(crate) fn parts(parts: &[(&str, &RatioValue)]) -> Parts {
    if let Some((name, _)) = parts.iter().find(|(_, r)| r.is_not_defined()) {
        return Parts::NotDefined(format!("{name} is not defined"));
    }
    let missing: Missing = parts
        .iter()
        .flat_map(|(_, r)| r.missing.iter().cloned())
        .collect();
    if !missing.is_empty() {
        return Parts::Missing(missing);
    }
    Parts::Values(parts.iter().filter_map(|(_, r)| r.value).collect())
}

/// A ratio built from other ratios: `compute` on the values with its explain step, or not
/// defined, or not computed (R10).
pub(crate) fn composite(
    ex: &mut Explain,
    label: &str,
    of: &[(&str, &RatioValue)],
    compute: impl FnOnce(&mut Explain, &[Decimal]) -> Result<Decimal>,
) -> Result<RatioValue> {
    Ok(match parts(of) {
        Parts::Values(v) => RatioValue::defined(compute(ex, &v)?),
        Parts::NotDefined(reason) => RatioValue::not_defined(ex, label, reason).lacking(
            of.iter()
                .flat_map(|(_, r)| r.lacking.iter().cloned())
                .collect(),
        ),
        Parts::Missing(missing) => RatioValue::not_computed(ex, label, missing),
    })
}

/// The outputs not computed of one result, in the order of the output fields (R7), and what
/// every output lacks, for the floor (R12).
#[derive(Default)]
pub(crate) struct NotComputedList {
    entries: Vec<NotComputedEntry>,
    /// What each output lacks; an empty set is an output with every input it needs.
    gaps: Vec<Missing>,
}

/// Blocks whose absence is reported by their own pointer: a set that names one is not a
/// complete alternative of the floor (the block also needs its amounts).
const BLOCKS: [&str; 2] = ["/leverage", "/debt_schedule"];

/// Convention of every `ratios.*` method (R7).
pub(crate) const MISSING_CONVENTION: &str =
    "a missing input is never taken as zero: an output that lacks an input is null and listed in \
     not_computed with the input pointers it lacks";

/// Reason given for every output not computed because an input is missing.
pub(crate) const MISSING_REASON: &str =
    "an input it needs is not given; a missing input is never taken as zero";

impl NotComputedList {
    pub(crate) fn push(&mut self, output: impl Into<String>, missing: &Missing) {
        self.entries.push(NotComputedEntry {
            output: output.into(),
            missing: ordered(missing),
            permitted: Vec::new(),
            reason: Some(MISSING_REASON.to_owned()),
        });
    }

    /// Records what one output lacks for the floor; empty when it has every input it needs.
    pub(crate) fn observe(&mut self, lacking: &Missing) {
        self.gaps.push(lacking.clone());
    }

    /// Lists `ratio` at `output` when it is not computed; returns it.
    pub(crate) fn ratio(&mut self, output: &str, ratio: RatioValue) -> RatioValue {
        if !ratio.missing.is_empty() {
            self.push(output, &ratio.missing);
        }
        self.observe(&ratio.lacking);
        ratio
    }

    /// The amount, or null listed at `output` with the pointers it lacks.
    pub(crate) fn amount(&mut self, output: &str, amount: &Given) -> Option<Decimal> {
        match amount {
            Ok(t) => {
                self.observe(&Missing::new());
                Some(t.value)
            }
            Err(missing) => {
                self.push(output, missing);
                self.observe(missing);
                None
            }
        }
    }

    /// The list, or `FM-E027` when no output has every input it needs (R12, ADR 0014 G10.1),
    /// with the minimal alternative sets of missing inputs, shortest first.
    pub(crate) fn finish(self, method: &'static str) -> Result<Vec<NotComputedEntry>> {
        if self.gaps.iter().any(BTreeSet::is_empty) {
            return Ok(self.entries);
        }
        let mut sets: Vec<&Missing> = Vec::new();
        for gap in &self.gaps {
            if gap.iter().any(|p| BLOCKS.contains(&p.as_str())) || sets.contains(&gap) {
                continue;
            }
            sets.push(gap);
        }
        let minimal: Vec<&Missing> = sets
            .iter()
            .filter(|s| !sets.iter().any(|o| o.len() < s.len() && o.is_subset(s)))
            .copied()
            .collect();
        let mut missing_any_of: Vec<Vec<String>> = minimal.into_iter().map(ordered).collect();
        missing_any_of.sort_by(|a, b| {
            a.len().cmp(&b.len()).then_with(|| {
                a.iter()
                    .zip(b)
                    .map(|(x, y)| pointer_cmp(x, y))
                    .find(|o| o.is_ne())
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
        });
        Err(FairmarkError::BelowMinimumInput(Box::new(MinimumInput {
            method,
            missing_any_of,
        })))
    }
}

/// Applies the balance convention and records it.
pub(crate) struct BalanceBasis {
    pub balances: Balances,
}

impl BalanceBasis {
    /// With closing balances an opening block that is given is not used, and the conventions
    /// say so. Average balances without the opening block are not an error: the amounts that
    /// need it are not computed (R7).
    pub(crate) fn new(balances: Balances, has_opening: bool, ex: &mut Explain) -> Self {
        match balances {
            Balances::Closing => {
                ex.convention("balances at closing");
                if has_opening {
                    ex.convention("opening balances given and not used");
                }
            }
            Balances::Average => {
                ex.convention("balances as the average of opening and closing");
                if !has_opening {
                    ex.convention(
                        "opening balance sheet not given: amounts that need it are not computed",
                    );
                }
            }
        }
        Self { balances }
    }

    pub(crate) fn is_average(&self) -> bool {
        self.balances == Balances::Average
    }

    /// The amount `path` of the balance sheets (for example `equity`): closing, or
    /// (opening + closing) / 2 with its explain step; the pointers it lacks otherwise.
    pub(crate) fn amount(
        &self,
        ex: &mut Explain,
        name: &'static str,
        path: &str,
        closing: Option<Decimal>,
        opening: Option<Decimal>,
    ) -> Result<Given> {
        let closing = given(name, closing, &format!("/closing/{path}"));
        let opening = given(name, opening, &format!("/opening/{path}"));
        self.resolve(ex, name, closing, opening)
    }

    /// The amount the ratios use from its closing and opening terms.
    pub(crate) fn resolve(
        &self,
        ex: &mut Explain,
        name: &'static str,
        closing: Given,
        opening: Given,
    ) -> Result<Given> {
        if !self.is_average() {
            return Ok(closing.map(|t| Term {
                name,
                value: t.value,
            }));
        }
        let (open, close) = match (&opening, &closing) {
            (Ok(o), Ok(c)) => (o.value, c.value),
            _ => return Ok(Err(union([&closing, &opening]))),
        };
        let avg = num::div(num::add(open, close)?, Decimal::TWO)?;
        ex.step(
            format!("Average {name}"),
            format!("({open} + {close}) / 2"),
            avg,
        );
        Ok(term(name, avg))
    }
}

/// A definition stated in words by the analyst (`ebitda_definition`, `interest_definition`):
/// optional, but when given it must say something, so an empty or blank text is `FM-E001`.
pub(crate) fn require_definition(field: &str, text: Option<&str>) -> Result<()> {
    if text.is_some_and(|t| t.trim().is_empty()) {
        return Err(FairmarkError::InvalidInput(format!(
            "`{field}` must state the definition used; it is empty"
        )));
    }
    Ok(())
}

/// An amount measured under a definition stated in words (EBITDA, interest): the ratios need
/// both, so a missing definition is a missing input (R9).
pub(crate) fn defined_amount(
    name: &'static str,
    value: Option<Decimal>,
    pointer: &str,
    definition: Option<&str>,
    definition_pointer: &str,
) -> Given {
    let amount = given(name, value, pointer);
    match (amount, definition) {
        (amount, Some(_)) => amount,
        (Ok(_), None) => Err(BTreeSet::from([definition_pointer.to_owned()])),
        (Err(mut missing), None) => {
            missing.insert(definition_pointer.to_owned());
            Err(missing)
        }
    }
}

/// One item of the net financial position other than gross debt and cash.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NfpItem {
    /// What the item is, for example "lease liabilities" or "derivative assets".
    pub name: String,
    /// Amount in currency units, added to the net financial position: positive for debt-like
    /// items, negative for financial assets.
    pub amount: Decimal,
}

/// The net financial position with its composition (SPEC-0007 R6). Fairmark adds what is
/// given and does not choose which items are included.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NetFinancialPosition {
    /// Gross financial debt (bank loans, bonds and other borrowings), in currency units.
    pub gross_debt: Decimal,
    /// Cash and cash equivalents, in currency units, subtracted from the gross debt.
    pub cash: Decimal,
    /// Other items included by the analyst, each added with its sign. Default: none.
    #[serde(default)]
    pub other_items: Vec<NfpItem>,
}

pub(crate) const NFP_CONVENTION: &str =
    "net financial position = gross debt − cash + other items, positive for net debt";

impl NetFinancialPosition {
    /// gross debt − cash + Σ other items, with every component echoed in the audit trail: the
    /// formula and the audit trail of [`NfpComposition::total`], on the same amounts.
    pub(crate) fn total(&self, ex: &mut Explain, when: &str) -> Result<Decimal> {
        let composition = NfpComposition {
            gross_debt: Some(self.gross_debt),
            cash: Some(self.cash),
            other_items: Some(
                self.other_items
                    .iter()
                    .map(|i| NfpOtherItem {
                        name: i.name.clone(),
                        amount: Some(i.amount),
                    })
                    .collect(),
            ),
        };
        Ok(known(&composition.total(ex, when, "")?)?.value)
    }
}

/// One item of the net financial position of a `ratios.*` input, other than gross debt and
/// cash.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NfpOtherItem {
    /// What the item is, for example "lease liabilities" or "derivative assets".
    pub name: String,
    /// Amount in currency units, added to the net financial position: positive for debt-like
    /// items, negative for financial assets. Absent or null: not given, and the net financial
    /// position is not computed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub amount: Option<Decimal>,
}

/// The net financial position of a `ratios.*` input, with its composition (SPEC-0007 R6).
/// Fairmark adds what is given and does not choose which items are included. Every amount is
/// optional (R7): without gross debt or cash the net financial position is not computed, never
/// taken as zero.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NfpComposition {
    /// Gross financial debt (bank loans, bonds and other borrowings), in currency units. Absent
    /// or null: not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gross_debt: Option<Decimal>,
    /// Cash and cash equivalents, in currency units, subtracted from the gross debt. Absent or
    /// null: not given.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cash: Option<Decimal>,
    /// Other items included by the analyst, each added with its sign; `[]` means none. Absent
    /// or null: none, a documented default (SPEC-0007 R13) listed in `meta.defaults_applied`
    /// with a warning when the net financial position is computed with it.
    #[serde(default, serialize_with = "items_or_none")]
    pub other_items: Option<Vec<NfpOtherItem>>,
}

/// The canonical form of `other_items` for `meta.input_hash`: the default filled in, so an
/// absent list and `[]` hash alike, as in version 1 (SPEC-0001 R31).
fn items_or_none<S: serde::Serializer>(
    items: &Option<Vec<NfpOtherItem>>,
    serializer: S,
) -> std::result::Result<S::Ok, S::Error> {
    serializer.collect_seq(items.iter().flatten())
}

/// The reason of the `other_items` default (R13).
pub(crate) const OTHER_ITEMS_REASON: &str =
    "no other items of the net financial position given: none, the net financial position is \
     gross debt less cash";

/// The `other_items` defaults of the balance sheets the ratios read (R13): one per sheet whose
/// net financial position gives gross debt and cash but no `other_items`.
pub(crate) fn other_items_defaults<'a>(
    sheets: impl IntoIterator<Item = (&'a str, Option<&'a NfpComposition>)>,
) -> Vec<DefaultApplied> {
    sheets
        .into_iter()
        .filter_map(|(sheet, nfp)| {
            let nfp = nfp?;
            (nfp.gross_debt.is_some() && nfp.cash.is_some() && nfp.other_items.is_none()).then(
                || {
                    DefaultApplied::new(
                        format!("/{sheet}/net_financial_position/other_items"),
                        serde_json::json!([]),
                        OTHER_ITEMS_REASON,
                    )
                },
            )
        })
        .collect()
}

/// The warning of every `other_items` default applied (G3, test 4: it determines the net
/// financial position).
pub(crate) fn warn_defaults(ex: &mut Explain, defaults: &[DefaultApplied], suffix: &str) {
    for d in defaults.iter().filter(|d| d.field.ends_with(suffix)) {
        ex.convention(format!(
            "warning: no other items of the net financial position given (`{}`): the net \
             financial position is gross debt less cash; list the other items, or give `[]` \
             for none",
            d.field
        ));
    }
}

impl NfpComposition {
    /// gross debt − cash + Σ other items, with every component given echoed in the audit
    /// trail; the pointers under `at` it lacks otherwise.
    pub(crate) fn total(&self, ex: &mut Explain, when: &str, at: &str) -> Result<Given> {
        let name = "net financial position";
        if let Some(g) = self.gross_debt {
            ex.step(format!("Gross debt ({when})"), "as given", g);
        }
        if let Some(c) = self.cash {
            ex.step(format!("Cash ({when})"), "as given", c);
        }
        let mut missing = Missing::new();
        if self.gross_debt.is_none() {
            missing.insert(format!("{at}/gross_debt"));
        }
        if self.cash.is_none() {
            missing.insert(format!("{at}/cash"));
        }
        let items = self.other_items.as_deref().unwrap_or(&[]);
        for (i, item) in items.iter().enumerate() {
            match item.amount {
                Some(a) => ex.step(format!("Other item: {} ({when})", item.name), "as given", a),
                None => {
                    missing.insert(format!("{at}/other_items/{i}/amount"));
                }
            }
        }
        let (Some(gross), Some(cash), true) = (self.gross_debt, self.cash, missing.is_empty())
        else {
            return Ok(Err(missing));
        };
        let mut total = num::sub(gross, cash)?;
        let mut formula = format!("{gross} − {cash}");
        for amount in items.iter().filter_map(|i| i.amount) {
            total = num::add(total, amount)?;
            if amount.is_sign_negative() && !amount.is_zero() {
                formula.push_str(&format!(" − {}", amount.abs()));
            } else {
                formula.push_str(&format!(" + {amount}"));
            }
        }
        ex.step(format!("Net financial position ({when})"), formula, total);
        Ok(term(name, total))
    }
}

/// The closing and opening net financial positions with their audit steps (the opening one is
/// read only with average balances); [`BalanceBasis::resolve`] then gives the amount the
/// ratios use.
pub(crate) fn nfp_totals(
    basis: &BalanceBasis,
    ex: &mut Explain,
    closing: Option<&NfpComposition>,
    opening: Option<&NfpComposition>,
) -> Result<(Given, Given)> {
    let at = |sheet: &str, c: Option<&NfpComposition>, ex: &mut Explain| -> Result<Given> {
        let ptr = format!("/{sheet}/net_financial_position");
        match c {
            Some(c) => c.total(ex, sheet, &ptr),
            None => Ok(Err(BTreeSet::from([
                format!("{ptr}/cash"),
                format!("{ptr}/gross_debt"),
            ]))),
        }
    };
    let close = at("closing", closing, ex)?;
    let open = if basis.is_average() {
        at("opening", opening, ex)?
    } else {
        Err(Missing::new())
    };
    Ok((close, open))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ex() -> Explain {
        Explain::new("test", "test")
    }

    fn known(name: &'static str, v: i64) -> Given {
        term(name, Decimal::from(v))
    }

    fn lacking(pointer: &str) -> Given {
        Err(BTreeSet::from([pointer.to_owned()]))
    }

    #[test]
    fn zero_denominator_is_not_defined_with_reason() {
        let mut e = ex();
        let r = quotient("R", "a / b", &known("a", 1), &known("b", 0))
            .evaluate(&mut e)
            .unwrap();
        assert_eq!(r.value, None);
        assert_eq!(r.not_defined.as_deref(), Some("b is zero"));
        assert_eq!(e.conventions, vec!["R not defined: b is zero"]);
    }

    #[test]
    fn negative_equity_is_not_defined_only_over_equity() {
        let mut e = ex();
        let eq = known("equity", -100);
        let r = over_equity("ROE", "n / e", &known("n", 10), &eq)
            .evaluate(&mut e)
            .unwrap();
        assert_eq!(r.not_defined.as_deref(), Some("equity is negative (-100)"));
        let r = quotient("x", "n / e", &known("n", 10), &eq)
            .evaluate(&mut e)
            .unwrap();
        assert_eq!(r.value, Some(Decimal::new(-1, 1)));
    }

    #[test]
    fn a_missing_term_is_not_computed_and_never_zero() {
        let mut e = ex();
        let r = quotient("R", "a / b", &lacking("/a"), &known("b", 4))
            .evaluate(&mut e)
            .unwrap();
        assert_eq!((r.value, r.not_defined.as_deref()), (None, None));
        assert_eq!(r.missing, BTreeSet::from(["/a".to_owned()]));
        assert_eq!(e.conventions, vec!["R not computed: /a not given"]);
        assert!(e.steps.is_empty());
    }

    #[test]
    fn a_given_term_that_makes_the_ratio_not_defined_wins_over_a_missing_one() {
        let r = quotient("R", "a / b", &lacking("/a"), &known("b", 0))
            .evaluate(&mut ex())
            .unwrap();
        assert_eq!(r.not_defined.as_deref(), Some("b is zero"));
        assert!(r.missing.is_empty());
        let r = both_positive("C", "a / b", &known("a", -1), &lacking("/b"))
            .evaluate(&mut ex())
            .unwrap();
        assert_eq!(r.not_defined.as_deref(), Some("a is negative (-1)"));
    }

    #[test]
    fn composite_prefers_not_defined_then_missing() {
        let mut e = ex();
        let undefined = RatioValue::undefined("x".into());
        let missing = RatioValue::not_computed(&mut e, "m", BTreeSet::from(["/m".to_owned()]));
        let ok = RatioValue::defined(Decimal::ONE);
        let r = composite(
            &mut e,
            "C",
            &[("m", &missing), ("u", &undefined)],
            |_, _| Ok(Decimal::ZERO),
        )
        .unwrap();
        assert_eq!(r.not_defined.as_deref(), Some("u is not defined"));
        let r = composite(&mut e, "C", &[("m", &missing), ("ok", &ok)], |_, _| {
            Ok(Decimal::ZERO)
        })
        .unwrap();
        assert_eq!(r.missing, BTreeSet::from(["/m".to_owned()]));
    }

    #[test]
    fn day_ratio_multiplies_before_dividing() {
        let mut e = ex();
        let r = DayRatio {
            label: "DIO",
            formula: "inventories × days / cost of sales",
            numerator: known("inventories", 550),
            denominator: known("cost of sales", 3000),
            basis: DayBasis::Days360,
        }
        .evaluate(&mut e)
        .unwrap();
        assert_eq!(r.value, Some(Decimal::from(66)));
        assert_eq!(
            e.steps[0].formula,
            "inventories × days / cost of sales = 550 × 360 / 3000"
        );
    }

    #[test]
    fn average_without_opening_lacks_the_opening_amount() {
        let mut e = ex();
        let b = BalanceBasis::new(Balances::Average, false, &mut e);
        let t = b
            .amount(&mut e, "equity", "equity", Some(Decimal::from(2000)), None)
            .unwrap();
        assert_eq!(
            t.unwrap_err(),
            BTreeSet::from(["/opening/equity".to_owned()])
        );
    }

    #[test]
    fn average_is_half_the_sum() {
        let mut e = ex();
        let b = BalanceBasis::new(Balances::Average, true, &mut e);
        let t = b
            .amount(
                &mut e,
                "equity",
                "equity",
                Some(Decimal::from(2000)),
                Some(Decimal::from(1801)),
            )
            .unwrap()
            .unwrap();
        assert_eq!(t.value, Decimal::new(19005, 1));
    }

    #[test]
    fn nfp_adds_signed_items() {
        let nfp = NfpComposition {
            gross_debt: Some(Decimal::from(2200)),
            cash: Some(Decimal::from(300)),
            other_items: Some(vec![
                NfpOtherItem {
                    name: "leases".into(),
                    amount: Some(Decimal::from(150)),
                },
                NfpOtherItem {
                    name: "derivative assets".into(),
                    amount: Some(Decimal::from(-50)),
                },
            ]),
        };
        let mut e = ex();
        let total = nfp
            .total(&mut e, "closing", "/closing/nfp")
            .unwrap()
            .unwrap();
        assert_eq!(total.value, Decimal::from(2000));
        assert_eq!(e.steps.last().unwrap().formula, "2200 − 300 + 150 − 50");
    }

    #[test]
    fn nfp_without_cash_or_an_item_amount_is_not_computed() {
        let nfp = NfpComposition {
            gross_debt: Some(Decimal::from(2200)),
            cash: None,
            other_items: Some(vec![NfpOtherItem {
                name: "leases".into(),
                amount: None,
            }]),
        };
        let missing = nfp.total(&mut ex(), "closing", "/c").unwrap().unwrap_err();
        assert_eq!(
            missing,
            BTreeSet::from(["/c/cash".to_owned(), "/c/other_items/0/amount".to_owned()])
        );
    }

    #[test]
    fn blank_definitions_are_refused_and_absent_ones_are_not() {
        let err = require_definition("ebitda_definition", Some("  \t ")).unwrap_err();
        assert_eq!(err.code(), "FM-E001");
        assert!(require_definition("ebitda_definition", Some("reported")).is_ok());
        assert!(require_definition("ebitda_definition", None).is_ok());
    }

    #[test]
    fn an_amount_without_its_definition_lacks_the_definition() {
        let g = defined_amount("EBITDA", Some(Decimal::TEN), "/e", None, "/d");
        assert_eq!(g.unwrap_err(), BTreeSet::from(["/d".to_owned()]));
        let g = defined_amount("EBITDA", None, "/e", None, "/d");
        assert_eq!(
            g.unwrap_err(),
            BTreeSet::from(["/d".to_owned(), "/e".to_owned()])
        );
    }
}
