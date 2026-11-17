//! The audit trail: every result carries how it was computed.
//!
//! An agent must cite these steps instead of recomputing numbers itself.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::Serialize;

use crate::diagnostics::{self, Check, Diagnostics, NotComputed, Warning, WarningCode};
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
    /// The method's formula in plain notation.
    pub formula: String,
    /// Ordered calculation steps.
    pub steps: Vec<Step>,
    /// Conventions applied (timing, discounting of the terminal value, ...), the documented
    /// defaults (`default: ...`) and the echo of every warning (`warning FM-Wnnn: ...`).
    pub conventions: Vec<String>,
    /// The diagnostics written while the method computes; the registry moves them into the
    /// envelope (`diagnostics`), so they are not part of `explain`.
    #[serde(skip)]
    #[schemars(skip)]
    diagnostics: Diagnostics,
}

impl Explain {
    pub fn new(formula: &str) -> Self {
        Self {
            formula: formula.to_owned(),
            steps: Vec::new(),
            conventions: Vec::new(),
            diagnostics: Diagnostics::default(),
        }
    }

    pub fn step(&mut self, label: impl Into<String>, formula: impl Into<String>, value: Decimal) {
        self.steps.push(Step {
            label: label.into(),
            formula: formula.into(),
            value,
        });
    }

    /// A convention. A warning is never written here: [`Explain::warn`] writes it with its code
    /// (SPEC-0012 R9, R16).
    pub fn convention(&mut self, text: impl Into<String>) {
        let text = text.into();
        debug_assert!(
            !diagnostics::is_warning_text(&text),
            "write a warning with Explain::warn, not as a convention: {text}"
        );
        self.conventions.push(text);
    }

    /// A warning of the catalogue (SPEC-0012 R8, R9): its entry in `diagnostics.warnings` and
    /// its echo `warning FM-Wnnn: <message>` in the conventions, in one call, so that the two
    /// keep the same order. `pointer` is written from the root the catalogue states for
    /// `code`.
    pub fn warn(&mut self, code: WarningCode, pointer: Option<String>, message: impl Into<String>) {
        let message = message.into();
        self.conventions.push(diagnostics::echo(code, &message));
        self.diagnostics.warnings.push(Warning {
            code,
            pointer,
            message,
        });
    }

    /// Outputs not computed (SPEC-0012 R5); ordered by pointer when the envelope is built.
    pub fn not_computed(&mut self, entries: impl IntoIterator<Item = NotComputed>) {
        self.diagnostics.not_computed.extend(entries);
    }

    /// Relations verified or not verified (SPEC-0012 R7), in the order of the result.
    pub fn checks(&mut self, entries: impl IntoIterator<Item = Check>) {
        self.diagnostics.checks.extend(entries);
    }

    /// The diagnostics written so far.
    pub fn diagnostics(&self) -> &Diagnostics {
        &self.diagnostics
    }

    /// The diagnostics, moved out for the envelope, `not_computed` ordered by pointer
    /// (SPEC-0012 R13).
    pub(crate) fn take_diagnostics(&mut self) -> Diagnostics {
        let mut d = std::mem::take(&mut self.diagnostics);
        d.not_computed
            .sort_by(|a, b| diagnostics::pointer_cmp(&a.pointer, &b.pointer));
        d
    }
}

/// A documented default filled in because the field was not given (SPEC-0001 R31). Listed in
/// `meta.defaults_applied` and reported as a convention starting with "default:".
#[derive(Debug, Clone, PartialEq, Serialize, JsonSchema)]
pub struct DefaultApplied {
    /// JSON pointer of the field filled in, for example `/timing` or `/stages/0/periods`, or of
    /// the item given whose default destination applies, from the root of the input sent.
    pub pointer: String,
    /// The value filled in, as it would be written in the input.
    pub value: serde_json::Value,
    /// Why this value: one line, citing the practice or the rule it follows.
    pub reason: String,
    /// What the default fills in; only the wording of the convention depends on it, so it is
    /// not serialised.
    #[serde(skip)]
    kind: DefaultKind,
}

/// A field filled in, or a destination given to an input item by a documented mapping.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
enum DefaultKind {
    /// The field was not given and is filled in with its documented value.
    #[default]
    Field,
    /// The item was given; its destination is the documented default of the mapping, for
    /// example a statutory aggregate assigned to its default block (SPEC-0005 R8).
    Destination,
}

impl DefaultApplied {
    pub fn new(pointer: impl Into<String>, value: serde_json::Value, reason: &str) -> Self {
        Self {
            pointer: pointer.into(),
            value,
            reason: reason.to_owned(),
            kind: DefaultKind::Field,
        }
    }

    /// True for a field filled in because it was not given; false for the default destination
    /// of an item that was given.
    pub fn is_field(&self) -> bool {
        self.kind == DefaultKind::Field
    }

    /// The default destination of an input item that was given, named by its pointer
    /// (SPEC-0005 R8, as clarified on 2026-11-08): `value` is the destination.
    pub fn destination(pointer: impl Into<String>, destination: &str, reason: &str) -> Self {
        Self {
            pointer: pointer.into(),
            value: serde_json::Value::String(destination.to_owned()),
            reason: reason.to_owned(),
            kind: DefaultKind::Destination,
        }
    }

    /// The text reported in `explain.conventions`.
    pub fn convention(&self) -> String {
        match self.kind {
            DefaultKind::Field => format!(
                "default: `{}` not given, set to {}: {}",
                self.pointer, self.value, self.reason
            ),
            DefaultKind::Destination => format!(
                "default: `{}` assigned to {}: {}",
                self.pointer, self.value, self.reason
            ),
        }
    }
}

/// Version of the envelope `{method, value, explain, diagnostics, meta}` (ADR 0014 G4): changes
/// of `explain`, `diagnostics` or `meta` follow ADR 0014 G8 against this version, not against
/// the method version (SPEC-0012 R4).
pub const ENVELOPE_VERSION: u32 = 1;

/// Reproducibility metadata attached to every result.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Meta {
    /// Version of the fairmark engine that produced the result.
    pub engine_version: String,
    /// Version of the envelope, an integer: 1 for `{method, value, explain, diagnostics,
    /// meta}` (ADR 0014 G4).
    pub envelope_version: u32,
    /// Version of the method implementation. Bumped on any numerical change.
    pub method_version: String,
    /// Status of the method that produced the result, `available` or `preview` (ADR 0012).
    pub method_status: Status,
    /// Analysis methods only: status of the method they evaluated (ADR 0012).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evaluated_method_status: Option<Status>,
    /// SHA-256 of the canonical input, with the documented defaults filled in: same hash +
    /// same versions = same value.
    pub input_hash: String,
    /// Arithmetic used for every operation.
    pub arithmetic: String,
    /// Documented defaults filled in because their fields were not given, in input order;
    /// empty when none applies.
    pub defaults_applied: Vec<DefaultApplied>,
}

/// The envelope every method returns (ADR 0014 G4). Its JSON Schema, with `value` described by
/// the method's output schema, is `registry::result_schema`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Valuation<T: Serialize> {
    /// Id of the method that produced the result, for example `dcf.fcff`.
    pub method: String,
    /// The method's output: its schema is `registry::output_schema`, snapshotted as
    /// `schemas/<method>/v<N>.output.json`.
    pub value: T,
    /// How the result was obtained: formula, steps and conventions.
    pub explain: Explain,
    /// What the result does not contain, what the reader could miss and what was verified.
    pub diagnostics: Diagnostics,
    /// Reproducibility metadata: engine, envelope and method versions, status, input hash,
    /// defaults.
    pub meta: Meta,
}
