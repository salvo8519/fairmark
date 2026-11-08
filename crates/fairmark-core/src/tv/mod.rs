//! Terminal value: a family of explicit methods (SPEC-0006).
//!
//! Every method returns the terminal value at the horizon N, not discounted to the valuation
//! date (the DCF of SPEC-0001 discounts it), together with the assumptions it implies. Rates
//! are decimal fractions per period of the terminal phase; amounts are in currency units.
//!
//! The functions are public so that other methods compose them through typed calls
//! (ADR 0009): `tv.compare` here, the multi-stage DCF later.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Result};
use crate::explain::Explain;

pub mod compare;
pub mod exit_multiple;
pub mod gordon;
pub mod h_model;
pub mod limited_horizon;
pub mod liquidation;
pub mod normalised_income;
pub mod value_driver;

pub use compare::{compare, Compare, CompareInput, CompareOutput, Technique};
pub use exit_multiple::{exit_multiple, ExitMultiple, ExitMultipleInput, MultipleMetric};
pub use gordon::{gordon, Gordon, GordonInput, GordonOutput};
pub use h_model::{h_model, HModel, HModelInput, HModelOutput};
pub use limited_horizon::{limited_horizon, LimitedHorizon, LimitedHorizonInput};
pub use liquidation::{liquidation, Liquidation, LiquidationInput, LiquidationOutput};
pub use normalised_income::{normalised_income, NormalisedIncome, NormalisedIncomeInput};
pub use value_driver::{value_driver, ValueDriver, ValueDriverInput, ValueDriverOutput};

/// Reported by every method: what the number is.
pub(crate) const NOT_DISCOUNTED: &str =
    "terminal value at the horizon N, not discounted to the valuation date";

/// Whether a terminal value is a value of the whole business or of its equity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum TerminalBasis {
    /// Enterprise value: flows to all capital providers (FCFF), discounted at the WACC;
    /// multiples EV/EBITDA, EV/EBIT, EV/Sales.
    Enterprise,
    /// Equity value: flows to shareholders (FCFE, dividends), discounted at the cost of
    /// equity; multiple P/E.
    Equity,
}

impl TerminalBasis {
    pub(crate) fn name(self) -> &'static str {
        match self {
            Self::Enterprise => "enterprise",
            Self::Equity => "equity",
        }
    }
}

/// Outcome of the growth cap check (SPEC-0006 R8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CapStatus {
    /// The growth is at or below the cap.
    Respected,
    /// The growth is above the cap: reported, not refused.
    Exceeded,
}

/// The perpetual growth compared with the cap the analyst gave.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GrowthCapCheck {
    /// The perpetual growth checked, decimal fraction per period.
    pub growth: Decimal,
    /// The cap given, decimal fraction per period.
    pub cap: Decimal,
    /// `respected` when growth ≤ cap, `exceeded` otherwise.
    pub status: CapStatus,
}

/// R8: compares `growth` with the cap when one is given and reports the outcome; reports the
/// absence of a cap otherwise. Never an error.
pub(crate) fn check_growth_cap(
    ex: &mut Explain,
    what: &str,
    growth: Decimal,
    cap: Option<Decimal>,
) -> Option<GrowthCapCheck> {
    let Some(cap) = cap else {
        ex.convention("growth cap: none given (not checked)");
        return None;
    };
    let status = if growth <= cap {
        CapStatus::Respected
    } else {
        CapStatus::Exceeded
    };
    let verb = match status {
        CapStatus::Respected => "respects",
        CapStatus::Exceeded => "exceeds",
    };
    ex.convention(format!("{what} {growth} {verb} the growth cap {cap}"));
    Some(GrowthCapCheck {
        growth,
        cap,
        status,
    })
}

/// A labelled amount, for example an asset and its realisable value.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Line {
    /// What the amount is, for example "Land" or "Bank loan". Not empty.
    pub label: String,
    /// Amount in currency units, zero or positive.
    pub value: Decimal,
}

/// An input error of a `tv.*` method together with the input field it concerns, for example
/// `growth` or `assets/0/value`. The standalone methods report `error`, with the rate named
/// by its pointer (see [`FieldError::into_error`]); a method that composes a technique (the
/// DCF) turns `field` into a JSON pointer of its own input (R27 of SPEC-0001).
#[derive(Debug)]
pub(crate) struct FieldError {
    pub field: String,
    pub error: FairmarkError,
}

impl FieldError {
    /// The error a standalone method, or `tv.compare` with its shared rate, reports: an
    /// `FM-E003` names the rate field (issue #174), every other error is unchanged.
    pub(crate) fn into_error(self) -> FairmarkError {
        crate::error::rate_field(&format!("/{}", self.field), None)(self.error)
    }
}

/// Validation result of a `tv.*` input.
pub(crate) type Checked = std::result::Result<(), FieldError>;

/// Attaches the field to an error.
pub(crate) fn on(field: impl Into<String>) -> impl FnOnce(FairmarkError) -> FieldError {
    let field = field.into();
    move |error| FieldError { field, error }
}

pub(crate) fn invalid(msg: impl Into<String>) -> FairmarkError {
    FairmarkError::InvalidInput(msg.into())
}

/// `FM-E018`: an input outside its economic domain (R10).
pub(crate) fn domain(msg: impl Into<String>) -> FairmarkError {
    FairmarkError::TerminalValueDomain(msg.into())
}

/// A growth of −100% or lower makes the flows vanish or alternate in sign; refused as in
/// `tvm.perpetuity` (`FM-E001`).
pub(crate) fn check_growth(g: Decimal, field: &str) -> Result<()> {
    if g <= -Decimal::ONE {
        return Err(invalid(format!(
            "{field} must be greater than -1 (got {g})"
        )));
    }
    Ok(())
}

/// `FM-E004` when the perpetual growth is not below the rate (R10).
pub(crate) fn check_below_rate(g: Decimal, r: Decimal) -> Result<()> {
    if g >= r {
        return Err(FairmarkError::GrowthNotBelowRate { growth: g, rate: r });
    }
    Ok(())
}

pub(crate) fn rate_convention(ex: &mut Explain, r: Decimal) {
    ex.convention(format!(
        "rate of the terminal phase {r} per period (given; may differ from the explicit phase)"
    ));
}

#[cfg(test)]
pub(crate) fn d(s: &str) -> Decimal {
    use std::str::FromStr;
    Decimal::from_str(s).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn growth_cap_is_reported_never_refused() {
        let mut ex = Explain::new("tv.test", "");
        let c = check_growth_cap(&mut ex, "growth", d("0.03"), Some(d("0.03"))).unwrap();
        assert_eq!(c.status, CapStatus::Respected);
        let c = check_growth_cap(&mut ex, "growth", d("0.031"), Some(d("0.03"))).unwrap();
        assert_eq!(c.status, CapStatus::Exceeded);
        assert!(check_growth_cap(&mut ex, "growth", d("0.5"), None).is_none());
        assert_eq!(
            ex.conventions,
            [
                "growth 0.03 respects the growth cap 0.03",
                "growth 0.031 exceeds the growth cap 0.03",
                "growth cap: none given (not checked)",
            ]
        );
    }
}
