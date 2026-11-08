//! The audit trail: every result carries how it was computed.
//!
//! An agent must cite these steps instead of recomputing numbers itself.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::Serialize;

use crate::registry::Status;

/// One auditable step of a calculation.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Step {
    /// Human-readable label, e.g. "Present value of FCF, year 3".
    pub label: String,
    /// The formula with the actual numbers substituted.
    pub formula: String,
    /// The exact decimal result of this step.
    pub value: Decimal,
}

/// How a result was obtained.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Explain {
    /// Method identifier, e.g. `dcf.fcff`.
    pub method: String,
    /// The method's formula in plain notation.
    pub formula: String,
    /// Ordered calculation steps.
    pub steps: Vec<Step>,
    /// Conventions applied (timing, discounting of the terminal value, ...).
    pub conventions: Vec<String>,
}

impl Explain {
    pub fn new(method: &str, formula: &str) -> Self {
        Self {
            method: method.to_owned(),
            formula: formula.to_owned(),
            steps: Vec::new(),
            conventions: Vec::new(),
        }
    }

    pub fn step(&mut self, label: impl Into<String>, formula: impl Into<String>, value: Decimal) {
        self.steps.push(Step {
            label: label.into(),
            formula: formula.into(),
            value,
        });
    }

    pub fn convention(&mut self, text: impl Into<String>) {
        self.conventions.push(text.into());
    }
}

/// Reproducibility metadata attached to every result.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Meta {
    /// Version of the fairmark engine that produced the result.
    pub engine_version: String,
    /// Version of the method implementation. Bumped on any numerical change.
    pub method_version: String,
    /// SHA-256 of the canonical input: same hash + same versions = same result.
    pub input_hash: String,
    /// Arithmetic used for every operation.
    pub arithmetic: String,
    /// Status of the method that produced the result, `available` or `preview` (ADR 0012).
    pub method_status: Status,
    /// Analysis methods only: status of the method they evaluated (ADR 0012).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluated_method_status: Option<Status>,
}

/// The envelope every method returns.
#[derive(Debug, Clone, Serialize)]
pub struct Valuation<T: Serialize> {
    pub method: String,
    pub value: T,
    pub explain: Explain,
    pub meta: Meta,
}
