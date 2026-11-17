//! Outputs `statements.reorganise` could not compute, with the inputs they lack (ADR 0014, G2;
//! SPEC-0005 R6), in the form the computation records them, and their conversion to entries of
//! `diagnostics.not_computed` (SPEC-0012 R5).

use crate::diagnostics::{self, NotComputed, ReasonCode};

/// A missing treatment and the destinations it permits (SPEC-0005 R8).
#[derive(Debug, Clone)]
pub struct Permitted {
    /// JSON pointer of the treatment to give.
    pub treatment: String,
    /// Its permitted values.
    pub destinations: Vec<&'static str>,
}

/// An output not computed, with the pointers of its missing inputs.
#[derive(Debug, Clone)]
pub struct NotComputedEntry {
    /// JSON pointer of the output in the result value.
    pub output: String,
    /// JSON pointers of the inputs it needs and lacks, sorted.
    pub missing: Vec<String>,
    /// For a missing treatment, its permitted destinations; empty otherwise.
    pub permitted: Vec<Permitted>,
    /// Why the output is not computed, one sentence; `None` for the reason of its code.
    pub reason: Option<String>,
}

/// The reason of an output that waits for a treatment.
const CHOICE_REQUIRED: &str =
    "a treatment it needs is not given: choose one of the permitted values; a missing treatment \
     is never assumed";
/// The reason of an output the form given cannot supply.
const NOT_APPLICABLE: &str = "no input supplies it in the form the year is given in";

impl NotComputedEntry {
    /// The entry of `diagnostics.not_computed` (SPEC-0012 R5): `choice_required` when a
    /// treatment must be given (the treatment is also in `missing_any_of`), `not_applicable`
    /// when no input could supply it in the form given (no missing input), `missing_input`
    /// otherwise; the pointer from the envelope root.
    pub(crate) fn into_diagnostic(self) -> NotComputed {
        let reason_code = match (self.permitted.is_empty(), self.missing.is_empty()) {
            (false, _) => ReasonCode::ChoiceRequired,
            (true, true) => ReasonCode::NotApplicable,
            (true, false) => ReasonCode::MissingInput,
        };
        let reason = self.reason.unwrap_or_else(|| {
            match reason_code {
                ReasonCode::ChoiceRequired => CHOICE_REQUIRED,
                ReasonCode::NotApplicable => NOT_APPLICABLE,
                _ => diagnostics::MISSING_INPUT_REASON,
            }
            .to_owned()
        });
        NotComputed {
            pointer: format!("/value{}", self.output),
            reason_code,
            reason,
            missing_any_of: if self.missing.is_empty() {
                Vec::new()
            } else {
                vec![self.missing]
            },
            permitted: self
                .permitted
                .into_iter()
                .map(|p| diagnostics::Permitted {
                    pointer: p.treatment,
                    values: p.destinations.into_iter().map(str::to_owned).collect(),
                })
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(missing: &[&str], permitted: bool, reason: Option<&str>) -> NotComputedEntry {
        NotComputedEntry {
            output: "/years/0/income_statement/net_income".to_owned(),
            missing: missing.iter().map(|p| (*p).to_owned()).collect(),
            permitted: if permitted {
                vec![Permitted {
                    treatment: "/treatments/financial_fixed_assets".to_owned(),
                    destinations: vec!["operating", "non_operating"],
                }]
            } else {
                Vec::new()
            },
            reason: reason.map(str::to_owned),
        }
    }

    /// SPEC-0012 R5: each reason of the computation becomes its reason code, with the reason of
    /// that code when the entry gives none (mutants of #361).
    #[test]
    fn each_reason_converts_to_its_code_and_reason() {
        let choice = entry(&["/treatments/financial_fixed_assets"], true, None).into_diagnostic();
        assert_eq!(choice.reason_code, ReasonCode::ChoiceRequired);
        assert_eq!(choice.reason, CHOICE_REQUIRED);
        assert_eq!(
            choice.missing_any_of,
            [["/treatments/financial_fixed_assets".to_owned()]]
        );
        assert_eq!(choice.permitted[0].values, ["operating", "non_operating"]);
        assert_eq!(choice.pointer, "/value/years/0/income_statement/net_income");

        let not_applicable = entry(&[], false, None).into_diagnostic();
        assert_eq!(not_applicable.reason_code, ReasonCode::NotApplicable);
        assert_eq!(not_applicable.reason, NOT_APPLICABLE);
        assert!(not_applicable.missing_any_of.is_empty());

        let missing =
            entry(&["/years/0/statutory/income_statement"], false, None).into_diagnostic();
        assert_eq!(missing.reason_code, ReasonCode::MissingInput);
        assert_eq!(missing.reason, diagnostics::MISSING_INPUT_REASON);

        let given = entry(&[], false, Some("as the method says")).into_diagnostic();
        assert_eq!(given.reason, "as the method says");
    }
}
