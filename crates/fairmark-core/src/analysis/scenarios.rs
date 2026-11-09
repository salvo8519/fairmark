//! `analysis.scenarios`: the outputs of any method under named sets of input changes, with
//! their deltas against the base case (SPEC-0008 R18, R19).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    base_case, base_value, output, pointer, recorded, unique, with_values, BaseInput, Inner,
    InnerMethod, Outcome,
};
use crate::error::{FairmarkError, Found, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{div, sub};

/// One input change of a scenario.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Override {
    /// RFC 6901 JSON Pointer to a decimal input of the inner method, for example
    /// `/terminal_growth`.
    pub pointer: String,
    /// Value of that input in this scenario, in its units.
    pub value: Decimal,
}

/// A named case: the base input with some inputs changed.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Scenario {
    /// Name of the case, unique and not empty, for example `downside`.
    pub name: String,
    /// Inputs changed in this case; every other input keeps its base value. Pointers do not
    /// repeat within a scenario.
    pub overrides: Vec<Override>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScenariosInput {
    /// Id of the method to evaluate, for example `dcf.fcff`. Analysis methods are not accepted.
    pub method: String,
    /// Input of that method, complete: the base case.
    pub base_input: BaseInput,
    /// RFC 6901 JSON Pointers to the decimal outputs to report, in the `value` of the inner
    /// result, for example `["/enterprise_value", "/equity_value"]`; at least one, no repeats.
    pub outputs: Vec<String>,
    /// The cases, evaluated in this order; at least one.
    pub scenarios: Vec<Scenario>,
}

/// One output of the base case.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct BaseOutput {
    /// The output pointer.
    pub output: String,
    /// Its value for `base_input` unchanged, in the output's units.
    pub value: Decimal,
}

/// One output of a scenario, compared with the base case.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScenarioOutput {
    /// The output pointer.
    pub output: String,
    /// Its value in the scenario, in the output's units.
    pub value: Decimal,
    /// `value − base`, in the output's units.
    pub delta: Decimal,
    /// `delta / base`, a decimal fraction; `null` when the base output is zero.
    pub relative_delta: Option<Decimal>,
}

/// The result of one scenario.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScenarioResult {
    /// The scenario name.
    pub name: String,
    /// The outputs as `{"value": [...]}`, or `{"error": {"code", "message"}}` when the inner
    /// method refused the scenario or its deltas overflow.
    #[serde(flatten)]
    pub result: Outcome<Vec<ScenarioOutput>>,
}

/// Output of `analysis.scenarios`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct ScenariosOutput {
    /// The method evaluated, at the version run.
    pub inner_method: InnerMethod,
    /// The outputs of the base case.
    pub base: Vec<BaseOutput>,
    /// One result per scenario, in the order given.
    pub scenarios: Vec<ScenarioResult>,
    /// Number of scenarios refused by the inner method or whose deltas overflow.
    pub failed_scenarios: u64,
}

/// Every check of the request and of its base input, before any evaluation (ADR 0014 G5):
/// the inner method, ready to evaluate, when nothing is refused.
fn check(input: &ScenariosInput) -> Result<Inner> {
    // Every input error of the request and of its base input is reported (ADR 0014 G5).
    let mut found = Found::new();
    let inner = found.take(Inner::new(&input.method));
    let base = base_value(&input.base_input);
    if input.outputs.is_empty() {
        found.push(
            FairmarkError::InvalidInput("outputs is empty".to_owned())
                .at("/outputs", crate::error::Reason::Length),
        );
    }
    if input.scenarios.is_empty() {
        found.push(
            FairmarkError::InvalidInput("scenarios is empty".to_owned())
                .at("/scenarios", crate::error::Reason::Length),
        );
    }
    found.check(unique(
        "output",
        input.outputs.iter().map(String::as_str),
        |k| format!("/outputs/{k}"),
    ));
    for (k, o) in input.outputs.iter().enumerate() {
        found.check(pointer::check_syntax(o).map_err(|e| e.located(format!("/outputs/{k}"))));
    }
    for (i, s) in input.scenarios.iter().enumerate() {
        if s.name.trim().is_empty() || s.name.trim() != s.name {
            found.push(
                FairmarkError::InvalidInput(format!(
                    "scenario name '{}' is empty or has leading or trailing spaces",
                    s.name
                ))
                .at(format!("/scenarios/{i}/name"), crate::error::Reason::Blank),
            );
        }
        found.check(unique(
            &format!("in scenario '{}' the pointer", s.name),
            s.overrides.iter().map(|o| o.pointer.as_str()),
            |k| format!("/scenarios/{i}/overrides/{k}/pointer"),
        ));
        for (k, o) in s.overrides.iter().enumerate() {
            let at = |e: FairmarkError| e.located(format!("/scenarios/{i}/overrides/{k}/pointer"));
            match &inner {
                Some(inner) => found.check(inner.check_variable(&base, &o.pointer).map_err(at)),
                // A refused method is reported alone: its variables are not checked against it.
                None => false,
            };
        }
    }
    found.check(unique(
        "scenario",
        input.scenarios.iter().map(|s| s.name.as_str()),
        |k| format!("/scenarios/{k}/name"),
    ));
    if let Some(inner) = &inner {
        inner.check_input(&base, input.scenarios.len() as u64 + 1, &mut found);
    }
    found.finish()?;
    inner.ok_or(FairmarkError::Internal("inner method not checked"))
}

pub fn scenarios(input: &ScenariosInput) -> Result<(ScenariosOutput, Explain)> {
    let mut inner = check(input)?;
    let base = base_value(&input.base_input);
    inner.reserve(input.scenarios.len() as u64 + 1, &base)?;

    let pointers: Vec<&str> = input.outputs.iter().map(String::as_str).collect();
    let base_outputs = base_case(&mut inner, &base, &pointers, |k| format!("/outputs/{k}"))?;
    let mut ex = Explain::new(
        Scenarios::ID,
        "delta = output(scenario) − output(base); relative delta = delta / output(base)",
    );
    for (o, v) in input.outputs.iter().zip(&base_outputs) {
        ex.step(
            format!("Base case: {o}"),
            format!("{o} with base_input unchanged"),
            *v,
        );
    }

    let mut results = Vec::with_capacity(input.scenarios.len());
    let mut failed = 0u64;
    for s in &input.scenarios {
        let set: Vec<(&str, Decimal)> = s
            .overrides
            .iter()
            .map(|o| (o.pointer.as_str(), o.value))
            .collect();
        let candidate = with_values(&base, &set)?;
        // An inner error, or a delta that cannot be computed (overflow), is recorded in the
        // scenario and the others go on (R19); an output pointer that does not resolve is a
        // wrong request and ends it (R3).
        let result = match recorded(inner.evaluate(&candidate))? {
            Outcome::Value { value } => {
                let values = input
                    .outputs
                    .iter()
                    .enumerate()
                    .map(|(k, o)| output(&value, o).map_err(|e| e.located(format!("/outputs/{k}"))))
                    .collect::<Result<Vec<_>>>()?;
                let compared = input
                    .outputs
                    .iter()
                    .zip(values)
                    .zip(&base_outputs)
                    .map(|((o, v), b)| {
                        let delta = sub(v, *b)?;
                        let relative_delta = if b.is_zero() {
                            None
                        } else {
                            Some(div(delta, *b)?)
                        };
                        Ok(ScenarioOutput {
                            output: o.clone(),
                            value: v,
                            delta,
                            relative_delta,
                        })
                    })
                    .collect::<Result<Vec<_>>>();
                recorded(compared)?
            }
            Outcome::Error { error } => Outcome::Error { error },
        };
        match &result {
            Outcome::Value { value } => {
                let changes = set
                    .iter()
                    .map(|(p, v)| format!("{p} = {v}"))
                    .collect::<Vec<_>>()
                    .join(", ");
                for (o, base_value) in value.iter().zip(&base_outputs) {
                    let relative = o
                        .relative_delta
                        .map_or("not defined (base is zero)".to_owned(), |r| r.to_string());
                    ex.step(
                        format!("Scenario {}: {}", s.name, o.output),
                        format!(
                            "{changes}; {} = {}; delta {} − {} = {}; relative {}",
                            o.output, o.value, o.value, base_value, o.delta, relative
                        ),
                        o.value,
                    );
                }
            }
            Outcome::Error { .. } => failed += 1,
        }
        results.push(ScenarioResult {
            name: s.name.clone(),
            result,
        });
    }

    ex.convention(format!(
        "inner method {} version {}, evaluated through the registry",
        inner.id(),
        inner.info().version
    ));
    if let Some(note) = inner.status_convention() {
        ex.convention(note);
    }
    ex.convention("evaluation order: base case, then the scenarios in the order given");
    ex.convention("inputs a scenario does not change keep their base value");
    ex.convention("relative delta is null when the base output is zero");
    ex.convention(
        "a scenario the inner method refuses, or whose deltas overflow, holds its error code and \
        message and the others go on; a defect in Fairmark (FM-E012) aborts the request",
    );
    ex.convention("the overrides are written into the inner input as decimal strings");
    ex.convention(format!(
        "{} evaluations of the inner method; failed scenarios: {failed}",
        inner.evaluations()
    ));

    Ok((
        ScenariosOutput {
            inner_method: inner.info(),
            base: input
                .outputs
                .iter()
                .zip(base_outputs)
                .map(|(o, v)| BaseOutput {
                    output: o.clone(),
                    value: v,
                })
                .collect(),
            scenarios: results,
            failed_scenarios: failed,
        },
        ex,
    ))
}

// ---- method registrations

/// `analysis.scenarios`.
pub struct Scenarios;

impl Method for Scenarios {
    const ID: &'static str = "analysis.scenarios";
    const TITLE: &'static str = "Scenario analysis";
    const DESCRIPTION: &'static str =
        "Outputs of any method under named sets of input changes, with deltas and relative \
        deltas against the base case; a scenario the method refuses is recorded.";
    const VERSION: &'static str = "1";
    type Input = ScenariosInput;
    type Output = ScenariosOutput;

    fn validate(input: &Self::Input, _defaults: &[crate::explain::DefaultApplied]) -> Result<()> {
        check(input).map(|_| ())
    }

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        scenarios(input)
    }

    fn evaluated_method(input: &Self::Input) -> Option<&str> {
        Some(&input.method)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    fn run(input: &Value) -> crate::Result<Value> {
        crate::run("analysis.scenarios", &input.to_string())
    }

    fn request() -> Value {
        json!({
            "method": "dcf.fcff",
            "base_input": {"stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}],
                           "cash_flow_terms": "nominal", "discount_rate_terms": "nominal",
                           "wacc": "0.09", "timing": "end_of_period",
                           "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
                           "net_financial_position": {"gross_debt": 50, "cash": 0}},
            "outputs": ["/enterprise_value", "/equity_value"],
            "scenarios": [
                {"name": "low", "overrides": [{"pointer": "/wacc", "value": "0.12"}]},
                {"name": "bad", "overrides": [{"pointer": "/terminal/tv.gordon/growth", "value": "0.2"}]}
            ]
        })
    }

    #[test]
    fn deltas_are_exact_and_failures_are_recorded() {
        let v = run(&request()).unwrap()["value"].clone();
        let base: rust_decimal::Decimal = v["base"][0]["value"].as_str().unwrap().parse().unwrap();
        let low = &v["scenarios"][0]["value"][0];
        let value: rust_decimal::Decimal = low["value"].as_str().unwrap().parse().unwrap();
        let delta: rust_decimal::Decimal = low["delta"].as_str().unwrap().parse().unwrap();
        assert_eq!(value - base, delta);
        assert_eq!(v["scenarios"][1]["error"]["code"], "FM-E004");
        assert_eq!(v["failed_scenarios"], json!(1));
    }

    /// R9, AC18: a base case the inner method refuses returns its error unchanged.
    #[test]
    fn a_failing_base_case_returns_its_error() {
        let mut r = request();
        r["base_input"]["terminal"]["tv.gordon"]["growth"] = json!("0.3");
        assert_eq!(run(&r).unwrap_err().code(), "FM-E004");
    }

    /// A delta that overflows is recorded in its scenario; the other scenarios are returned.
    #[test]
    fn an_overflowing_delta_is_recorded_in_its_scenario() {
        let max = "79228162514264337593543950335";
        let r = json!({
            "method": "tvm.npv",
            "base_input": {"rate": "0", "cash_flows": [format!("-{max}")]},
            "outputs": ["/npv"],
            "scenarios": [
                {"name": "huge", "overrides": [{"pointer": "/cash_flows/0", "value": max}]},
                {"name": "small", "overrides": [{"pointer": "/cash_flows/0", "value": "-1"}]}
            ]
        });
        let v = run(&r).unwrap()["value"].clone();
        assert_eq!(v["scenarios"][0]["error"]["code"], "FM-E008");
        assert!(v["scenarios"][1]["value"][0]["value"].is_string());
        assert_eq!(v["failed_scenarios"], json!(1));
    }

    #[test]
    fn invalid_requests_are_refused() {
        for (path, value, code) in [
            ("/scenarios/1/name", json!("low"), "FM-E001"),
            ("/scenarios/0/name", json!(" "), "FM-E001"),
            ("/scenarios/0/name", json!("low "), "FM-E001"),
            (
                "/outputs",
                json!(["/equity_value", "/equity_value"]),
                "FM-E001",
            ),
            ("/outputs", json!([]), "FM-E001"),
            ("/scenarios", json!([]), "FM-E001"),
            (
                "/scenarios/0/overrides/0/pointer",
                json!("/nope"),
                "FM-E015",
            ),
            ("/outputs", json!(["/periods"]), "FM-E015"),
            ("/method", json!("analysis.scenarios"), "FM-E017"),
        ] {
            let mut r = request();
            *r.pointer_mut(path).unwrap() = value;
            assert_eq!(run(&r).unwrap_err().code(), code, "{path}");
        }
        let mut twice = request();
        twice["scenarios"][0]["overrides"] = json!([
            {"pointer": "/wacc", "value": "0.1"},
            {"pointer": "/wacc", "value": "0.2"}
        ]);
        assert_eq!(run(&twice).unwrap_err().code(), "FM-E001");
    }
}
