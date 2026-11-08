//! Financial ratios organised by the question the analyst asks (SPEC-0007).
//!
//! Each group is one method: `ratios.profitability`, `ratios.capital_structure`,
//! `ratios.liquidity`, `ratios.working_capital` and `ratios.debt_service`. They share the
//! pieces of this module. `ratios.covenants` tests covenants on the definitions of the loan
//! agreement and uses only [`RatioValue`]:
//!
//! - [`Balances`]: closing balances, or the average of opening and closing (R2);
//! - [`DayBasis`]: 360 or 365 days for day-based ratios (R2);
//! - [`RatioValue`]: a ratio, or the reason it is not defined (R3);
//! - the [`Ratio`] trait (ADR 0009, point 5) with its two implementors [`Quotient`] and
//!   [`DayRatio`]: formula, numerator, denominator and not-defined handling in one place;
//! - [`NetFinancialPosition`]: the net financial position with its composition (R6).

pub mod capital_structure;
pub mod covenants;
pub mod debt_service;
pub mod liquidity;
pub mod profitability;
pub mod working_capital;

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
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

/// A ratio, or the reason it is not defined (SPEC-0007 R3).
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct RatioValue {
    /// The ratio, in the unit stated by the field that holds it (decimal fraction, multiple or
    /// days); null when the ratio is not defined.
    pub value: Option<Decimal>,
    /// Why the ratio is not defined, for example "equity is negative (-100)"; null when it is
    /// defined.
    pub not_defined: Option<String>,
}

impl RatioValue {
    pub(crate) fn defined(value: Decimal) -> Self {
        Self {
            value: Some(value),
            not_defined: None,
        }
    }

    /// Records the reason in the conventions, so the audit trail says why a ratio is missing.
    pub(crate) fn not_defined(ex: &mut Explain, label: &str, reason: String) -> Self {
        ex.convention(format!("{label} not defined: {reason}"));
        Self {
            value: None,
            not_defined: Some(reason),
        }
    }
}

/// A named amount entering a ratio, already resolved under the balance convention.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Term {
    pub name: &'static str,
    pub value: Decimal,
}

pub(crate) fn term(name: &'static str, value: Decimal) -> Term {
    Term { name, value }
}

/// One ratio: its formula, numerator, denominator and not-defined handling (ADR 0009, point 5).
/// The balance convention is applied before, when the terms are resolved.
pub(crate) trait Ratio {
    /// Label in the audit trail, for example "ROE".
    fn label(&self) -> &str;
    /// Formula in words, for example "net income / equity".
    fn formula(&self) -> &str;
    fn numerator(&self) -> Term;
    fn denominator(&self) -> Term;
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

    /// The ratio with its explain step, or not defined with the reason (R1, R3).
    fn evaluate(&self, ex: &mut Explain) -> Result<RatioValue> {
        let (n, d) = (self.numerator(), self.denominator());
        if d.value.is_zero() {
            let reason = format!("{} is zero", d.name);
            return Ok(RatioValue::not_defined(ex, self.label(), reason));
        }
        if self.needs_positive_denominator() && d.value.is_sign_negative() {
            let reason = format!("{} is negative ({})", d.name, d.value);
            return Ok(RatioValue::not_defined(ex, self.label(), reason));
        }
        if self.needs_positive_numerator() {
            if n.value.is_zero() {
                let reason = format!("{} is zero", n.name);
                return Ok(RatioValue::not_defined(ex, self.label(), reason));
            }
            if n.value.is_sign_negative() {
                let reason = format!("{} is negative ({})", n.name, n.value);
                return Ok(RatioValue::not_defined(ex, self.label(), reason));
            }
        }
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
    pub numerator: Term,
    pub denominator: Term,
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
    fn numerator(&self) -> Term {
        self.numerator
    }
    fn denominator(&self) -> Term {
        self.denominator
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
    pub numerator: Term,
    pub denominator: Term,
    pub basis: DayBasis,
}

impl Ratio for DayRatio {
    fn label(&self) -> &str {
        self.label
    }
    fn formula(&self) -> &str {
        self.formula
    }
    fn numerator(&self) -> Term {
        self.numerator
    }
    fn denominator(&self) -> Term {
        self.denominator
    }
    fn days(&self) -> Option<Decimal> {
        Some(self.basis.days())
    }
}

/// `numerator / denominator` with the equity rule off.
pub(crate) fn quotient(
    label: &'static str,
    formula: &'static str,
    numerator: Term,
    denominator: Term,
) -> Quotient {
    Quotient {
        label,
        formula,
        numerator,
        denominator,
        positive_denominator: false,
        positive_numerator: false,
    }
}

/// `numerator / denominator`, not defined when the denominator is zero or negative.
pub(crate) fn over_positive(
    label: &'static str,
    formula: &'static str,
    numerator: Term,
    denominator: Term,
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
    numerator: Term,
    denominator: Term,
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
    numerator: Term,
    equity: Term,
) -> Quotient {
    Quotient {
        positive_denominator: true,
        ..quotient(label, formula, numerator, equity)
    }
}

/// The values of `parts`, or the name of the first part that is not defined.
pub(crate) fn all_defined(
    parts: &[(&str, &RatioValue)],
) -> std::result::Result<Vec<Decimal>, String> {
    parts
        .iter()
        .map(|(name, r)| r.value.ok_or_else(|| format!("{name} is not defined")))
        .collect()
}

/// Applies the balance convention: checks the opening block and records the convention.
pub(crate) struct BalanceBasis {
    pub balances: Balances,
}

impl BalanceBasis {
    /// Average balances need `opening` (`FM-E001`); with closing balances an opening block
    /// that is given is not used, and the conventions say so.
    pub(crate) fn new(balances: Balances, has_opening: bool, ex: &mut Explain) -> Result<Self> {
        match balances {
            Balances::Closing => {
                ex.convention("balances at closing");
                if has_opening {
                    ex.convention("opening balances given and not used");
                }
            }
            Balances::Average => {
                if !has_opening {
                    return Err(FairmarkError::InvalidInput(
                        "average balances need the opening balance sheet in `opening`".into(),
                    ));
                }
                ex.convention("balances as the average of opening and closing");
            }
        }
        Ok(Self { balances })
    }

    /// The amount the ratios use: closing, or (opening + closing) / 2 with its explain step.
    /// `opening` is present whenever the convention is average (checked in [`Self::new`]).
    pub(crate) fn resolve(
        &self,
        ex: &mut Explain,
        name: &'static str,
        closing: Decimal,
        opening: Option<Decimal>,
    ) -> Result<Term> {
        let value = match (self.balances, opening) {
            (Balances::Closing, _) => closing,
            (Balances::Average, Some(open)) => {
                let avg = num::div(num::add(open, closing)?, Decimal::TWO)?;
                ex.step(
                    format!("Average {name}"),
                    format!("({open} + {closing}) / 2"),
                    avg,
                );
                avg
            }
            (Balances::Average, None) => {
                return Err(FairmarkError::Internal("average without opening balances"))
            }
        };
        Ok(term(name, value))
    }
}

/// A definition stated in words by the analyst (`ebitda_definition`, `interest_definition`):
/// it must say something, so an empty or blank text is `FM-E001`.
pub(crate) fn require_definition(field: &str, text: &str) -> Result<()> {
    if text.trim().is_empty() {
        return Err(FairmarkError::InvalidInput(format!(
            "`{field}` must state the definition used; it is empty"
        )));
    }
    Ok(())
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
    /// gross debt − cash + Σ other items, with every component echoed in the audit trail.
    pub(crate) fn total(&self, ex: &mut Explain, when: &str) -> Result<Decimal> {
        ex.step(format!("Gross debt ({when})"), "as given", self.gross_debt);
        ex.step(format!("Cash ({when})"), "as given", self.cash);
        let mut total = num::sub(self.gross_debt, self.cash)?;
        let mut formula = format!("{} − {}", self.gross_debt, self.cash);
        for item in &self.other_items {
            ex.step(
                format!("Other item: {} ({when})", item.name),
                "as given",
                item.amount,
            );
            total = num::add(total, item.amount)?;
            if item.amount.is_sign_negative() && !item.amount.is_zero() {
                formula.push_str(&format!(" − {}", item.amount.abs()));
            } else {
                formula.push_str(&format!(" + {}", item.amount));
            }
        }
        ex.step(format!("Net financial position ({when})"), formula, total);
        Ok(total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ex() -> Explain {
        Explain::new("test", "test")
    }

    #[test]
    fn zero_denominator_is_not_defined_with_reason() {
        let mut e = ex();
        let r = quotient(
            "R",
            "a / b",
            term("a", Decimal::ONE),
            term("b", Decimal::ZERO),
        )
        .evaluate(&mut e)
        .unwrap();
        assert_eq!(r.value, None);
        assert_eq!(r.not_defined.as_deref(), Some("b is zero"));
        assert_eq!(e.conventions, vec!["R not defined: b is zero"]);
    }

    #[test]
    fn negative_equity_is_not_defined_only_over_equity() {
        let mut e = ex();
        let eq = term("equity", Decimal::from(-100));
        let r = over_equity("ROE", "n / e", term("n", Decimal::TEN), eq)
            .evaluate(&mut e)
            .unwrap();
        assert_eq!(r.not_defined.as_deref(), Some("equity is negative (-100)"));
        let r = quotient("x", "n / e", term("n", Decimal::TEN), eq)
            .evaluate(&mut e)
            .unwrap();
        assert_eq!(r.value, Some(Decimal::new(-1, 1)));
    }

    #[test]
    fn day_ratio_multiplies_before_dividing() {
        let mut e = ex();
        let r = DayRatio {
            label: "DIO",
            formula: "inventories × days / cost of sales",
            numerator: term("inventories", Decimal::from(550)),
            denominator: term("cost of sales", Decimal::from(3000)),
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
    fn average_needs_opening() {
        let err = BalanceBasis::new(Balances::Average, false, &mut ex())
            .err()
            .unwrap();
        assert_eq!(err.code(), "FM-E001");
    }

    #[test]
    fn average_is_half_the_sum() {
        let mut e = ex();
        let b = BalanceBasis::new(Balances::Average, true, &mut e).unwrap();
        let t = b
            .resolve(
                &mut e,
                "equity",
                Decimal::from(2000),
                Some(Decimal::from(1801)),
            )
            .unwrap();
        assert_eq!(t.value, Decimal::new(19005, 1));
    }

    #[test]
    fn nfp_adds_signed_items() {
        let nfp = NetFinancialPosition {
            gross_debt: Decimal::from(2200),
            cash: Decimal::from(300),
            other_items: vec![
                NfpItem {
                    name: "leases".into(),
                    amount: Decimal::from(150),
                },
                NfpItem {
                    name: "derivative assets".into(),
                    amount: Decimal::from(-50),
                },
            ],
        };
        let mut e = ex();
        assert_eq!(nfp.total(&mut e, "closing").unwrap(), Decimal::from(2000));
        assert_eq!(e.steps.last().unwrap().formula, "2200 − 300 + 150 − 50");
    }

    #[test]
    fn blank_definitions_are_refused() {
        let err = require_definition("ebitda_definition", "  \t ").unwrap_err();
        assert_eq!(err.code(), "FM-E001");
        assert!(require_definition("ebitda_definition", "reported").is_ok());
    }
}
