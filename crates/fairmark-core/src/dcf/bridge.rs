//! The lines from the discounted value to equity value, shared by both bases of the DCF
//! (SPEC-0001 R23 to R26): labelled non-operating assets added, labelled claims (debt-like
//! items, non-controlling interests) subtracted, each echoed in `explain`.
//!
//! Each basis assembles its own bridge from these pieces (`dcf.fcff`: R23, `dcf.fcfe`: R24);
//! the checks, the explain steps and the conventions are written once, here.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Found, Reason, Result};
use crate::explain::Explain;
use crate::num::add;
use crate::tv::Line;

/// A claim on the value of the company other than the net financial position, subtracted from
/// enterprise value to reach equity value: a debt-like item or a non-controlling interest
/// (R25). Fairmark subtracts the amount as given and does not choose how it is measured.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BridgeClaim {
    /// What the claim is, for example "Unfunded pension obligation" or "30% of Subsidiary A".
    /// Not empty.
    pub label: String,
    /// Amount at the valuation date, in currency units, zero or more; subtracted.
    pub value: Decimal,
    /// How the amount was measured, stated by the analyst: `book_value`, `market_value`,
    /// `fair_value`, `present_value` or `other` (then describe it in `basis_note`). Required,
    /// no default. Reported with the amount, never used in the calculation.
    pub measurement_basis: MeasurementBasis,
    /// Free text that qualifies the basis, for example "net of tax" or "49% of the book value
    /// of equity of the subsidiary"; required with `other`, optional otherwise, never empty
    /// when given. Reported, never used in the calculation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub basis_note: Option<String>,
}

/// How the amount of a bridge claim was measured (Decision 21). Fairmark does not choose the
/// basis: the analyst states it and the result reports it next to the amount.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum MeasurementBasis {
    /// Book value per the accounts at the valuation date.
    BookValue,
    /// Quoted market price at the valuation date.
    MarketValue,
    /// Fair value as defined by IFRS 13.
    FairValue,
    /// Present value of the obligation, for example unfunded pensions or provisions.
    PresentValue,
    /// Another basis, described in `basis_note` (required).
    Other,
}

impl MeasurementBasis {
    /// The JSON value and its meaning, as reported in `explain`.
    fn describe(self) -> (&'static str, &'static str) {
        match self {
            Self::BookValue => ("book_value", "book value per the accounts"),
            Self::MarketValue => ("market_value", "quoted market price"),
            Self::FairValue => ("fair_value", "fair value as defined by IFRS 13"),
            Self::PresentValue => ("present_value", "present value of the obligation"),
            Self::Other => ("other", "described in the note"),
        }
    }
}

/// R25, R27: the lines of the bridge are checked before any calculation: each non-operating
/// asset, debt-like item and non-controlling interest has a label and a value of zero or more,
/// and each claim a note when its basis is `other`. Every refusal names the field by its JSON
/// pointer and states the fix.
pub(crate) fn check_lines(
    non_operating_assets: &[Line],
    debt_like_items: &[BridgeClaim],
    non_controlling_interests: &[BridgeClaim],
    asset_sign_rule: &str,
    f: &mut Found,
) {
    for (i, a) in non_operating_assets.iter().enumerate() {
        check_labelled_amount(
            &format!("/non_operating_assets/{i}"),
            &a.label,
            a.value,
            "name the asset",
            asset_sign_rule,
            f,
        );
    }
    check_claims(
        debt_like_items,
        "debt_like_items",
        "a debt-like item is zero or more (an asset belongs in `non_operating_assets`)",
        f,
    );
    check_claims(
        non_controlling_interests,
        "non_controlling_interests",
        "a non-controlling interest is zero or more (SPEC-0001 R25); give zero or a positive \
         amount",
        f,
    );
}

/// R25, R27: a bridge line at `pointer` has a value of zero or more and a label that is not
/// empty; each refusal names the field by its JSON pointer and states the fix.
fn check_labelled_amount(
    pointer: &str,
    label: &str,
    value: Decimal,
    label_fix: &str,
    sign_rule: &str,
    f: &mut Found,
) {
    if value.is_sign_negative() && !value.is_zero() {
        f.push(
            FairmarkError::InvalidInput(format!("`{pointer}/value` is {value}; {sign_rule}"))
                .at(format!("{pointer}/value"), Reason::Sign)
                .expected(["zero or more"])
                .fix("give the amount as zero or more"),
        );
    }
    if label.trim().is_empty() {
        f.push(
            FairmarkError::InvalidInput(format!("`{pointer}/label` is empty; {label_fix}"))
                .at(format!("{pointer}/label"), Reason::Blank)
                .fix(label_fix),
        );
    }
}

/// R25, R27, Decision 21: every claim has a label, a value of zero or more and a note when the
/// basis is `other`; each refusal names the field by its JSON pointer and states the fix.
fn check_claims(claims: &[BridgeClaim], field: &str, sign_rule: &str, f: &mut Found) {
    for (i, c) in claims.iter().enumerate() {
        check_labelled_amount(
            &format!("/{field}/{i}"),
            &c.label,
            c.value,
            "name the item",
            sign_rule,
            f,
        );
        let at = format!("/{field}/{i}/basis_note");
        let blank = c.basis_note.as_deref().map(|n| n.trim().is_empty());
        match (blank, c.measurement_basis) {
            (None | Some(true), MeasurementBasis::Other) => {
                f.push(
                    FairmarkError::InvalidInput(format!(
                        "`{at}` is missing or empty; with `measurement_basis` `other`, \
                         describe in `basis_note` how the amount was measured, for example \
                         \"settlement amount, undiscounted\", or choose `book_value`, \
                         `market_value`, `fair_value` or `present_value`"
                    ))
                    .at(at.clone(), Reason::Missing)
                    .between([at, format!("/{field}/{i}/measurement_basis")])
                    .fix(
                        "describe in `basis_note` how the amount was measured, or choose \
                         another `measurement_basis`",
                    ),
                );
            }
            (Some(true), _) => {
                f.push(
                    FairmarkError::InvalidInput(format!(
                        "`{at}` is empty; write the note or remove the field"
                    ))
                    .at(at, Reason::Blank)
                    .fix("write the note, or remove the field"),
                );
            }
            _ => {}
        }
    }
}

/// R25: each non-operating asset as an explain step, "none given" when there is none, and
/// their sum.
pub(crate) fn non_operating_assets_total(ex: &mut Explain, assets: &[Line]) -> Result<Decimal> {
    let mut total = Decimal::ZERO;
    if assets.is_empty() {
        ex.convention("non-operating assets: none given");
    }
    for a in assets {
        ex.step(
            format!("Non-operating asset: {}", a.label),
            "as given",
            a.value,
        );
        total = add(total, a.value)?;
    }
    Ok(total)
}

/// R25: each claim as an explain step labelled with its basis, its stated basis as a
/// convention, and the total with the values substituted.
pub(crate) fn claims_total(
    ex: &mut Explain,
    claims: &[BridgeClaim],
    singular: &str,
    plural: &str,
) -> Result<Decimal> {
    if claims.is_empty() {
        ex.convention(format!("{}: none given", plural.to_lowercase()));
        return Ok(Decimal::ZERO);
    }
    let mut total = Decimal::ZERO;
    let mut terms = Vec::with_capacity(claims.len());
    for c in claims {
        let (basis, meaning) = c.measurement_basis.describe();
        let (in_label, note) = match &c.basis_note {
            Some(n) => (format!("{basis}: {n}"), format!("; note: {n}")),
            None => (basis.to_owned(), String::new()),
        };
        ex.step(
            format!("{singular}: {} ({in_label})", c.label),
            "as given",
            c.value,
        );
        ex.convention(format!(
            "{} '{}': measurement basis {basis} ({meaning}), stated by the analyst{note}",
            singular.to_lowercase(),
            c.label,
        ));
        total = add(total, c.value)?;
        terms.push(c.value.to_string());
    }
    ex.step(format!("{plural}, total"), terms.join(" + "), total);
    Ok(total)
}

/// R26: a negative equity value is reported as a warning, never floored at zero and never
/// refused; `reason` says what made it negative on the caller's basis.
pub(crate) fn warn_if_negative(ex: &mut Explain, equity_value: Decimal, reason: &str) {
    if equity_value.is_sign_negative() && !equity_value.is_zero() {
        ex.convention(format!(
            "warning: the equity value is negative ({equity_value}): {reason}; it is reported \
             as computed, not floored at zero"
        ));
    }
}
