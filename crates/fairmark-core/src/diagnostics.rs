//! The `diagnostics` block of the result envelope (ADR 0014 G4; SPEC-0012): what the result
//! does not contain (`not_computed`), what the reader could miss (`warnings`, with stable codes)
//! and what was verified (`checks`).
//!
//! A method writes its diagnostics through [`crate::explain::Explain`] while it computes:
//! [`Explain::warn`](crate::explain::Explain::warn) records a warning and its echo in the
//! conventions in one call, so that the two cannot diverge (R9);
//! [`Explain::not_computed`](crate::explain::Explain::not_computed) and
//! [`Explain::checks`](crate::explain::Explain::checks) hand over the lists of a method that
//! computes from partial input. The registry moves them into the envelope.
//!
//! Every warning code is declared once below, as `Variant = "FM-Wnnn"`, documented in
//! `docs/en/reference/warnings.md`, and never reused or renumbered (decision 3 of SPEC-0012); a
//! test reads this file and the catalogue and checks that they agree (R16).

use std::borrow::Cow;
use std::cmp::Ordering;

use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// What the result does not contain, what the reader could miss and what was verified (ADR
/// 0014 G4). Every list is always present, empty when there is nothing to report.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Diagnostics {
    /// The outputs not computed, one entry per output, ordered by `pointer` (array indices
    /// compared as numbers); empty when every output is computed.
    pub not_computed: Vec<NotComputed>,
    /// The warnings of the catalogue (`docs/en/reference/warnings.md`), in the order of their
    /// echo in `explain.conventions`; empty when nothing is warned.
    pub warnings: Vec<Warning>,
    /// Every relation the method verified or could not verify, ordered by result block, then by
    /// `id`; empty for a method without checks.
    pub checks: Vec<Check>,
}

impl Diagnostics {
    /// True when the three lists are empty.
    pub fn is_empty(&self) -> bool {
        self.not_computed.is_empty() && self.warnings.is_empty() && self.checks.is_empty()
    }
}

/// Why an output is not computed. Values may be added, never renamed (ADR 0014 G8).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ReasonCode {
    /// An input it needs is not given; `missing_any_of` lists the inputs that make it
    /// computable.
    MissingInput,
    /// A treatment or a choice must be given; `permitted` lists the values to choose from.
    ChoiceRequired,
    /// A given term makes the output not defined, for example a ratio over a negative equity.
    NotDefined,
    /// No input could supply it in the form given, for example a statutory output of an entry
    /// given in reclassified form.
    NotApplicable,
}

/// A choice the analyst must make for an output to be computed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Permitted {
    /// JSON pointer of the input to give, from the root of the input sent.
    pub pointer: String,
    /// Its permitted values.
    pub values: Vec<String>,
}

/// An output not computed, and why.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NotComputed {
    /// JSON pointer of the output, from the root of the result envelope (`/value/...`).
    pub pointer: String,
    /// Why it is not computed.
    pub reason_code: ReasonCode,
    /// The reason in one sentence, in English; guidance, not contract.
    pub reason: String,
    /// Alternative sets of input pointers, from the root of the input sent: any one set given
    /// makes the output computable. Empty for `not_defined` and `not_applicable`.
    pub missing_any_of: Vec<Vec<String>>,
    /// The choices to make for `choice_required`; empty otherwise.
    pub permitted: Vec<Permitted>,
}

/// The status of a relation. Values may be added, never renamed (ADR 0014 G8). A relation that
/// fails is an error (`FM-E026`), never a check.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum CheckStatus {
    /// Both sides given and equal.
    Reconciled,
    /// A side is missing: the relation could not be checked.
    NotChecked,
}

/// One relation the method verified.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Check {
    /// The relation id, snake_case, documented on the methodology page of the method.
    pub id: String,
    /// `reconciled` or `not_checked`.
    pub status: CheckStatus,
    /// JSON pointer of the result block the check belongs to, from the root of the result
    /// envelope, for example `/value/years/0`.
    pub pointer: String,
    /// JSON pointers of the inputs on both sides of the relation, from the root of the input
    /// sent, as read; empty when the sides are computed outputs.
    pub pointers: Vec<String>,
}

/// A condition the reader could miss (ADR 0014 G4).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Warning {
    /// The code of the condition, documented in `docs/en/reference/warnings.md`.
    pub code: WarningCode,
    /// JSON pointer of the field the warning concerns, from the root its catalogue entry states:
    /// the input sent, or the result envelope when it starts with `/value`; null only where the
    /// catalogue entry says so.
    pub pointer: Option<String>,
    /// The warning in one sentence, in English; guidance, not contract.
    pub message: String,
}

/// The root a warning pointer is written from (catalogue column "Pointer root").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PointerRoot {
    /// The input the client sent (inside an analysis, under `/base_input`).
    Input,
    /// The result envelope (`/value/...`).
    Envelope,
}

/// Declares the warning codes: one line `Variant = "FM-Wnnn",` each, read by the catalogue
/// test (R16).
macro_rules! warning_codes {
    ($($(#[doc = $doc:literal])+ $variant:ident = $code:literal,)+) => {
        /// The code of a warning of the catalogue (`docs/en/reference/warnings.md`): `FM-W`
        /// and three digits, never reused or renumbered. A new condition takes the next free
        /// code, which is additive (ADR 0014 G8).
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub enum WarningCode {
            $($(#[doc = $doc])+ $variant,)+
        }

        impl WarningCode {
            /// Every code, in the order of the catalogue.
            pub const ALL: &'static [WarningCode] = &[$(WarningCode::$variant),+];

            /// The code as written in a result, for example `FM-W001`.
            pub const fn code(self) -> &'static str {
                match self {
                    $(WarningCode::$variant => $code,)+
                }
            }
        }
    };
}

warning_codes! {
    /// A decimal of the input has more digits than a decimal holds and is rounded as it is
    /// read (ADR 0014 G7).
    RoundedOnReading = "FM-W001",
    /// `series.project`: the base value is negative.
    NegativeBaseValue = "FM-W002",
    /// `dcf.*`: a growth is applied to a negative flow, once per period.
    GrowthOnNegativeFlow = "FM-W003",
    /// `dcf.*`: the nominal perpetual growth is below the expected inflation of the terminal
    /// phase.
    PerpetualGrowthBelowInflation = "FM-W004",
    /// `dcf.*`: no terminal value is included.
    NoTerminalValue = "FM-W005",
    /// `dcf.*`: a given terminal value is negative.
    NegativeGivenTerminalValue = "FM-W006",
    /// `dcf.*`: the terminal value is negative, so its weight is not meaningful.
    TerminalValueWeightNotMeaningful = "FM-W007",
    /// `dcf.*`: the growth of the last explicit period differs from the perpetual growth.
    LastGrowthDiffersFromPerpetualGrowth = "FM-W008",
    /// `dcf.*`: a price at a date is discounted at N − 0.5.
    PriceAtDateDiscountedAtHalfPeriod = "FM-W009",
    /// `dcf.*`: under mid-period timing a perpetuity-growth terminal value is discounted at N.
    PerpetuityDiscountedAtHorizon = "FM-W010",
    /// `dcf.*`: the equity value is negative.
    NegativeEquityValue = "FM-W011",
    /// `dcf.fcff`: no net financial position given; zero is subtracted.
    NoNetFinancialPosition = "FM-W012",
    /// `ratios.*`: no other items of a net financial position given.
    NoOtherItemsOfNetFinancialPosition = "FM-W013",
    /// `ratios.debt_service`: no reserve balance given.
    NoReserveBalance = "FM-W014",
    /// `statements.reorganise`: a write-down is placed below EBITDA by the default treatment.
    WriteDownBelowEbitdaByDefault = "FM-W015",
    /// `analysis.*`: the evaluated method is preview (ADR 0012).
    EvaluatedMethodPreview = "FM-W016",
}

impl WarningCode {
    /// The root its pointer is written from (catalogue column "Pointer root").
    pub const fn root(self) -> PointerRoot {
        match self {
            WarningCode::TerminalValueWeightNotMeaningful | WarningCode::NegativeEquityValue => {
                PointerRoot::Envelope
            }
            _ => PointerRoot::Input,
        }
    }

    /// The code written as `text`, if it is one of the catalogue.
    pub fn from_code(text: &str) -> Option<Self> {
        Self::ALL.iter().copied().find(|c| c.code() == text)
    }
}

impl std::fmt::Display for WarningCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.code())
    }
}

impl Serialize for WarningCode {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.code())
    }
}

impl<'de> Deserialize<'de> for WarningCode {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        Self::from_code(&text)
            .ok_or_else(|| serde::de::Error::custom(format!("unknown warning code {text}")))
    }
}

impl JsonSchema for WarningCode {
    fn schema_name() -> Cow<'static, str> {
        "WarningCode".into()
    }

    /// A pattern, not an enumeration: a code added by a later release is additive (ADR 0014
    /// G8), and clients accept codes they do not know.
    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "string",
            "pattern": "^FM-W[0-9]{3}$",
            "description": "The code of a warning: FM-W and three digits, documented in \
                docs/en/reference/warnings.md, never reused or renumbered. Clients accept codes \
                they do not know."
        })
    }
}

/// The echo of a warning in `explain.conventions` (R9): `warning FM-Wnnn: <message>`.
pub(crate) fn echo(code: WarningCode, message: &str) -> String {
    format!("warning {code}: {message}")
}

/// True when a convention text reads as a warning, which only [`echo`] may write (R16).
pub(crate) fn is_warning_text(text: &str) -> bool {
    text.get(..7)
        .is_some_and(|head| head.eq_ignore_ascii_case("warning"))
}

/// Order of JSON pointers in every list of the result (R13): token by token, array indices
/// compared as numbers and before names (`/periods/2` before `/periods/10`).
pub(crate) fn pointer_cmp(a: &str, b: &str) -> Ordering {
    fn key(t: &str) -> (u8, u64, &str) {
        match t.parse::<u64>() {
            Ok(n) if !t.is_empty() && t.bytes().all(|c| c.is_ascii_digit()) => (0, n, ""),
            _ => (1, 0, t),
        }
    }
    a.split('/').map(key).cmp(b.split('/').map(key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_unique_and_in_order() {
        for (i, code) in WarningCode::ALL.iter().enumerate() {
            assert_eq!(code.code(), format!("FM-W{:03}", i + 1));
            assert_eq!(WarningCode::from_code(code.code()), Some(*code));
        }
        assert_eq!(WarningCode::from_code("FM-W999"), None);
    }

    #[test]
    fn codes_serialise_as_text_and_the_schema_is_a_pattern() {
        let w = Warning {
            code: WarningCode::NoReserveBalance,
            pointer: None,
            message: "m".into(),
        };
        let json = serde_json::to_value(&w).unwrap();
        assert_eq!(
            json,
            serde_json::json!({"code": "FM-W014", "pointer": null, "message": "m"})
        );
        assert_eq!(serde_json::from_value::<Warning>(json).unwrap(), w);
        let schema = serde_json::to_value(schemars::schema_for!(WarningCode)).unwrap();
        assert_eq!(schema["pattern"], "^FM-W[0-9]{3}$");
        assert!(schema.get("enum").is_none());
    }

    #[test]
    fn warning_texts_are_recognised() {
        assert!(is_warning_text("warning FM-W001: x"));
        assert!(is_warning_text("Warning: x"));
        assert!(!is_warning_text("warn"));
        assert!(!is_warning_text("default: x"));
    }

    #[test]
    fn warnings_are_echoed_with_their_code() {
        assert_eq!(
            echo(WarningCode::RoundedOnReading, "m"),
            "warning FM-W001: m"
        );
    }

    #[test]
    fn pointers_compare_indices_as_numbers() {
        assert_eq!(pointer_cmp("/periods/2", "/periods/10"), Ordering::Less);
        assert_eq!(pointer_cmp("/periods/10", "/periods/a"), Ordering::Less);
        assert_eq!(pointer_cmp("/a", "/a"), Ordering::Equal);
    }
}
