//! `tv.compare`: several terminal value techniques on one consistent set of inputs
//! (SPEC-0006 R9).
//!
//! Consistency is by construction: the rate of the terminal phase and the growth cap are given
//! once and passed to every technique that uses them, and the basis is declared once; an exit
//! multiple of the other basis is refused (`FM-E019`). The techniques are called through typed
//! functions (ADR 0009), not through the registry.
//!
//! Every row records the version of the technique that produced it. A numerical change in any
//! `tv.*` method therefore requires a new version of `tv.compare` too; a test pins the versions
//! together.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::exit_multiple::{ExitMultipleOutput, ExitMultipleTerms};
use super::gordon::GordonTerms;
use super::h_model::HModelTerms;
use super::limited_horizon::{LimitedHorizonOutput, LimitedHorizonTerms};
use super::normalised_income::{NormalisedIncomeOutput, NormalisedIncomeTerms};
use super::value_driver::ValueDriverTerms;
use super::{
    exit_multiple, gordon, h_model, invalid, limited_horizon, liquidation, normalised_income,
    value_driver, ExitMultiple, Gordon, GordonOutput, HModel, HModelOutput, LimitedHorizon,
    Liquidation, LiquidationInput, LiquidationOutput, NormalisedIncome, TerminalBasis, ValueDriver,
    ValueDriverOutput, NOT_DISCOUNTED,
};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;

/// A technique entry as written in the input: an object with exactly one key, the method id.
/// Deserialised through a JSON value so that an entry with several keys gets a clear message;
/// numbers keep their exact text (`serde_json` is built with `arbitrary_precision`).
#[derive(Debug, Clone, Deserialize)]
#[serde(try_from = "serde_json::Value")]
pub struct TechniqueEntry(pub Technique);

impl Serialize for TechniqueEntry {
    fn serialize<S: serde::Serializer>(&self, s: S) -> std::result::Result<S::Ok, S::Error> {
        self.0.serialize(s)
    }
}

impl TryFrom<serde_json::Value> for TechniqueEntry {
    type Error = String;

    fn try_from(v: serde_json::Value) -> std::result::Result<Self, String> {
        match &v {
            serde_json::Value::Object(map) if map.len() == 1 => {}
            serde_json::Value::Object(map) => {
                let keys: Vec<&str> = map.keys().map(String::as_str).collect();
                return Err(format!(
                    "each technique is an object with exactly one key, the method id, for \
                    example {{\"tv.gordon\": {{...}}}} (got {} keys: {})",
                    keys.len(),
                    keys.join(", ")
                ));
            }
            _ => {
                return Err(
                    "each technique is an object with exactly one key, the method id".to_owned(),
                )
            }
        }
        serde_json::from_value(v)
            .map(TechniqueEntry)
            .map_err(|e| e.to_string())
    }
}

/// One technique to compare, named by its method id, with its own terms. The shared rate and
/// growth cap are not repeated here.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub enum Technique {
    /// Stable growth: `growth` and one of `next_flow`, `last_flow`.
    #[serde(rename = "tv.gordon")]
    Gordon(GordonTerms),
    /// Growth from reinvestment: `nopat_next`, `growth`, `ronic`.
    #[serde(rename = "tv.value_driver")]
    ValueDriver(ValueDriverTerms),
    /// Declining growth: `last_flow`, `short_term_growth`, `long_term_growth`, `half_life`.
    #[serde(rename = "tv.h_model")]
    HModel(HModelTerms),
    /// Exit multiple on the basis of the comparison; `next_flow` optional.
    #[serde(rename = "tv.exit_multiple")]
    ExitMultiple(ExitMultipleTerms),
    /// Finite horizon: `flow`, `periods`, optional `growth` and `residual_value`.
    #[serde(rename = "tv.limited_horizon")]
    LimitedHorizon(LimitedHorizonTerms),
    /// Normalised income with its own capitalisation rate and basis (checked).
    #[serde(rename = "tv.normalised_income")]
    NormalisedIncome(NormalisedIncomeTerms),
    /// Liquidation: the analyst lists the liabilities of the declared basis.
    #[serde(rename = "tv.liquidation")]
    Liquidation(LiquidationInput),
}

/// Input of `tv.compare`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CompareInput {
    /// Discount rate of the terminal phase, decimal fraction per period, shared by every
    /// technique that uses a discount rate.
    pub rate: Decimal,
    /// `enterprise` or `equity`: the basis of every terminal value compared. An exit multiple
    /// of the other basis is refused.
    pub basis: TerminalBasis,
    /// Upper bound for the perpetual growth, decimal fraction per period, shared by every
    /// growth-based technique. Default: none, reported as absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth_cap: Option<Decimal>,
    /// Techniques to run, at least one, each written `{"tv.<name>": {terms}}`; the same
    /// technique may appear more than once.
    #[schemars(with = "Vec<Technique>", length(min = 1))]
    pub techniques: Vec<TechniqueEntry>,
}

/// The full output of one technique, as its own method returns it.
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum TechniqueOutput {
    Gordon(GordonOutput),
    ValueDriver(ValueDriverOutput),
    HModel(HModelOutput),
    ExitMultiple(ExitMultipleOutput),
    LimitedHorizon(LimitedHorizonOutput),
    NormalisedIncome(NormalisedIncomeOutput),
    Liquidation(LiquidationOutput),
}

/// One row of the comparison.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ComparedTechnique {
    /// Method id of the technique, for example `tv.gordon`.
    pub method: String,
    /// Version of the technique's method that produced this row.
    pub method_version: String,
    /// Terminal value at N, not discounted, in currency units.
    pub terminal_value: Decimal,
    /// The technique's own output, with its implied assumptions.
    pub output: TechniqueOutput,
}

/// Output of `tv.compare`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct CompareOutput {
    /// The basis of every terminal value in `results`.
    pub basis: TerminalBasis,
    /// One row per technique, in the order requested.
    pub results: Vec<ComparedTechnique>,
}

/// Runs one technique with the shared inputs; returns its id, value, output and explain.
fn run_one(
    t: &Technique,
    rate: Decimal,
    cap: Option<Decimal>,
    basis: TerminalBasis,
    position: usize,
) -> Result<(
    &'static str,
    &'static str,
    Decimal,
    TechniqueOutput,
    Explain,
)> {
    Ok(match t {
        Technique::Gordon(terms) => {
            let (o, ex) = gordon(&terms.with(rate, cap))?;
            (
                Gordon::ID,
                Gordon::VERSION,
                o.terminal_value,
                TechniqueOutput::Gordon(o),
                ex,
            )
        }
        Technique::ValueDriver(terms) => {
            let (o, ex) = value_driver(&terms.with(rate, cap))?;
            let tv = o.terminal_value;
            (
                ValueDriver::ID,
                ValueDriver::VERSION,
                tv,
                TechniqueOutput::ValueDriver(o),
                ex,
            )
        }
        Technique::HModel(terms) => {
            let (o, ex) = h_model(&terms.with(rate, cap))?;
            (
                HModel::ID,
                HModel::VERSION,
                o.terminal_value,
                TechniqueOutput::HModel(o),
                ex,
            )
        }
        Technique::ExitMultiple(terms) => {
            if terms.metric.basis() != basis {
                return Err(FairmarkError::MixedBasis(format!(
                    "technique [{position}] tv.exit_multiple uses {} ({}, {} basis) in a \
                    comparison declared on the {} basis",
                    terms.metric.name(),
                    terms.metric.multiple_name(),
                    terms.metric.basis().name(),
                    basis.name()
                )));
            }
            let (o, ex) = exit_multiple(&terms.with(rate, cap))?;
            let tv = o.terminal_value;
            (
                ExitMultiple::ID,
                ExitMultiple::VERSION,
                tv,
                TechniqueOutput::ExitMultiple(o),
                ex,
            )
        }
        Technique::LimitedHorizon(terms) => {
            let (o, ex) = limited_horizon(&terms.with(rate, cap))?;
            let tv = o.terminal_value;
            (
                LimitedHorizon::ID,
                LimitedHorizon::VERSION,
                tv,
                TechniqueOutput::LimitedHorizon(o),
                ex,
            )
        }
        Technique::NormalisedIncome(terms) => {
            if terms.basis != basis {
                return Err(FairmarkError::MixedBasis(format!(
                    "technique [{position}] tv.normalised_income is declared on the {} basis in \
                    a comparison declared on the {} basis",
                    terms.basis.name(),
                    basis.name()
                )));
            }
            let (o, ex) = normalised_income(&terms.with(cap))?;
            let tv = o.terminal_value;
            (
                NormalisedIncome::ID,
                NormalisedIncome::VERSION,
                tv,
                TechniqueOutput::NormalisedIncome(o),
                ex,
            )
        }
        Technique::Liquidation(input) => {
            let (o, ex) = liquidation(input)?;
            let tv = o.terminal_value;
            (
                Liquidation::ID,
                Liquidation::VERSION,
                tv,
                TechniqueOutput::Liquidation(o),
                ex,
            )
        }
    })
}

/// Runs every technique on the shared inputs and returns the terminal values side by side.
pub fn compare(input: &CompareInput) -> Result<(CompareOutput, Explain)> {
    if input.techniques.is_empty() {
        return Err(invalid("give at least one technique to compare"));
    }
    let (r, cap, basis) = (input.rate, input.growth_cap, input.basis);
    let mut ex = Explain::new(
        "tv.compare",
        "each technique computed by its own method on the shared rate, growth cap and basis",
    );
    ex.convention(NOT_DISCOUNTED);
    ex.convention(format!(
        "basis: {} (declared; every terminal value compared is on this basis)",
        basis.name()
    ));
    ex.convention(format!(
        "shared rate {r} per period for every technique that uses a discount rate; \
        tv.normalised_income keeps its own capitalisation rate"
    ));
    ex.convention(match cap {
        Some(c) => format!("shared growth cap {c} for every growth-based technique"),
        None => "growth cap: none given (not checked)".to_owned(),
    });
    let has = |f: fn(&Technique) -> bool| input.techniques.iter().any(|t| f(&t.0));
    if cap.is_some() && has(|t| matches!(t, Technique::ExitMultiple(e) if e.next_flow.is_none())) {
        ex.convention(
            "tv.exit_multiple without next_flow: no implied growth, so not checked against the \
            growth cap",
        );
    }
    if has(|t| matches!(t, Technique::Liquidation(_))) {
        ex.convention(match basis {
            TerminalBasis::Enterprise => {
                "tv.liquidation on the enterprise basis: the liabilities listed exclude \
                financial debt"
            }
            TerminalBasis::Equity => {
                "tv.liquidation on the equity basis: the liabilities listed include financial \
                debt"
            }
        });
    }
    let mut results = Vec::with_capacity(input.techniques.len());
    for (k, t) in input.techniques.iter().enumerate() {
        let position = k + 1;
        let (id, version, tv, output, inner) = run_one(&t.0, r, cap, basis, position)?;
        for c in inner.conventions {
            if c != NOT_DISCOUNTED {
                ex.convention(format!("[{position}] {id}: {c}"));
            }
        }
        for s in inner.steps {
            ex.step(
                format!("[{position}] {id}: {}", s.label),
                s.formula,
                s.value,
            );
        }
        results.push(ComparedTechnique {
            method: id.to_owned(),
            method_version: version.to_owned(),
            terminal_value: tv,
            output,
        });
    }
    Ok((CompareOutput { basis, results }, ex))
}

/// `tv.compare`: see the methodology page.
pub struct Compare;

impl Method for Compare {
    const ID: &'static str = "tv.compare";
    const TITLE: &'static str = "Terminal value, comparison of techniques";
    const DESCRIPTION: &'static str =
        "Runs several terminal value techniques on one rate, growth cap and basis and returns \
        each terminal value at the horizon N with its implied assumptions, side by side.";
    const VERSION: &'static str = "1";
    type Input = CompareInput;
    type Output = CompareOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        compare(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tv::d;
    use crate::tv::exit_multiple::MultipleMetric;

    fn gordon_terms() -> Technique {
        Technique::Gordon(GordonTerms {
            growth: d("0.02"),
            next_flow: Some(d("100")),
            last_flow: None,
        })
    }

    fn exit(metric: MultipleMetric) -> Technique {
        Technique::ExitMultiple(ExitMultipleTerms {
            multiple: d("10"),
            metric,
            metric_value: d("100"),
            next_flow: None,
        })
    }

    fn input(techniques: Vec<Technique>, basis: TerminalBasis) -> CompareInput {
        CompareInput {
            rate: d("0.1"),
            basis,
            growth_cap: Some(d("0.03")),
            techniques: techniques.into_iter().map(TechniqueEntry).collect(),
        }
    }

    /// A numerical change in any technique requires a new version of `tv.compare`: when one
    /// of these versions changes, bump `Compare::VERSION` and update this list.
    #[test]
    fn technique_versions_are_pinned_with_the_compare_version() {
        let versions = [
            (Gordon::ID, Gordon::VERSION),
            (ValueDriver::ID, ValueDriver::VERSION),
            (HModel::ID, HModel::VERSION),
            (ExitMultiple::ID, ExitMultiple::VERSION),
            (LimitedHorizon::ID, LimitedHorizon::VERSION),
            (NormalisedIncome::ID, NormalisedIncome::VERSION),
            (Liquidation::ID, Liquidation::VERSION),
            (Compare::ID, Compare::VERSION),
        ];
        assert_eq!(
            versions,
            [
                ("tv.gordon", "1"),
                ("tv.value_driver", "1"),
                ("tv.h_model", "1"),
                ("tv.exit_multiple", "1"),
                ("tv.limited_horizon", "1"),
                ("tv.normalised_income", "1"),
                ("tv.liquidation", "1"),
                ("tv.compare", "1"),
            ]
        );
    }

    #[test]
    fn an_entry_with_two_keys_gets_a_clear_message() {
        let json = r#"{"rate": "0.1", "basis": "enterprise", "techniques": [
            {"tv.gordon": {"growth": "0", "next_flow": 1}, "tv.h_model": {}}]}"#;
        let err = crate::run("tv.compare", json).unwrap_err();
        assert_eq!(err.code(), "FM-E001");
        assert!(err.to_string().contains("exactly one key"), "{err}");
    }

    #[test]
    fn rows_record_the_technique_version() {
        let (out, _) = compare(&input(vec![gordon_terms()], TerminalBasis::Enterprise)).unwrap();
        assert_eq!(out.results[0].method_version, Gordon::VERSION);
    }

    #[test]
    fn each_row_equals_its_method_run_alone() {
        let (out, ex) = compare(&input(
            vec![gordon_terms(), exit(MultipleMetric::Ebit)],
            TerminalBasis::Enterprise,
        ))
        .unwrap();
        assert_eq!(out.results[0].terminal_value, d("1250"));
        assert_eq!(out.results[1].terminal_value, d("1000"));
        assert!(ex.steps[0].label.starts_with("[1] tv.gordon: "));
        assert!(ex
            .conventions
            .iter()
            .any(|c| c == "[1] tv.gordon: growth 0.02 respects the growth cap 0.03"));
    }

    #[test]
    fn bases_are_never_mixed() {
        let err = compare(&input(
            vec![gordon_terms(), exit(MultipleMetric::Earnings)],
            TerminalBasis::Enterprise,
        ))
        .unwrap_err();
        assert_eq!(err.code(), "FM-E019");
        let err = compare(&input(
            vec![exit(MultipleMetric::Revenue)],
            TerminalBasis::Equity,
        ))
        .unwrap_err();
        assert_eq!(err.code(), "FM-E019");
    }

    /// Decision of 2026-11-08: normalised income declares its basis and is checked like the
    /// multiples.
    #[test]
    fn normalised_income_of_the_other_basis_is_refused() {
        let ni = |basis| {
            Technique::NormalisedIncome(NormalisedIncomeTerms {
                income: d("95"),
                capitalisation_rate: d("0.11"),
                basis,
                growth: None,
            })
        };
        let err = compare(&input(
            vec![gordon_terms(), ni(TerminalBasis::Equity)],
            TerminalBasis::Enterprise,
        ))
        .unwrap_err();
        assert_eq!(err.code(), "FM-E019");
        assert!(compare(&input(
            vec![ni(TerminalBasis::Equity), exit(MultipleMetric::BookValue)],
            TerminalBasis::Equity,
        ))
        .is_ok());
    }

    #[test]
    fn an_empty_list_is_refused() {
        let err = compare(&input(vec![], TerminalBasis::Equity)).unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }
}
