//! `analysis.goal_seek`: the value of one input that makes one output equal a target
//! (SPEC-0008 R10 to R14).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use super::{base_value, output, pointer, sorted, BaseInput, Inner, InnerMethod};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::sub;
use crate::solver::{brent, Point};

/// Default absolute tolerance on the output.
pub const DEFAULT_TOLERANCE: &str = "0.000000001";
/// Default and largest number of iterations after the two bracket ends.
pub const DEFAULT_MAX_ITERATIONS: u32 = 100;
pub const MAX_ITERATIONS_LIMIT: u32 = 1_000;

fn default_tolerance() -> Decimal {
    DEFAULT_TOLERANCE.parse().expect("valid constant")
}

fn default_max_iterations() -> u32 {
    DEFAULT_MAX_ITERATIONS
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct GoalSeekInput {
    /// Id of the method to evaluate, for example `dcf.fcff`. Analysis methods are not accepted.
    pub method: String,
    /// Input of that method, complete, including the variable at any starting value.
    pub base_input: BaseInput,
    /// RFC 6901 JSON Pointer to the decimal input to solve for, for example `/discount_rate`.
    pub variable: String,
    /// RFC 6901 JSON Pointer to the decimal output of the method, in its `value`, for example
    /// `/enterprise_value`.
    pub output: String,
    /// Value the output must reach, in the output's units.
    pub target: Decimal,
    /// `[low, high]`, low < high: the variable is searched in this interval, and the output
    /// minus the target must change sign between its ends (or meet the tolerance at one end).
    pub bracket: [Decimal; 2],
    /// Absolute tolerance on the output, in the output's units, greater than 0. Default:
    /// 0.000000001.
    #[serde(default = "default_tolerance")]
    pub tolerance: Decimal,
    /// Largest number of evaluations after the two bracket ends, from 1 to 1000. Default: 100.
    #[serde(default = "default_max_iterations")]
    pub max_iterations: u32,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct GoalSeekOutput {
    /// The method evaluated, at the version run.
    pub inner_method: InnerMethod,
    /// Value of the variable at which the output meets the target, in the variable's units.
    pub solution: Decimal,
    /// Output of the method at the solution, in the output's units.
    pub achieved: Decimal,
    /// `achieved − target`; its absolute value is at most the tolerance.
    pub residual: Decimal,
    /// Evaluations after the two bracket ends.
    pub iterations: u32,
    /// Tolerance applied, in the output's units.
    pub tolerance: Decimal,
    /// Input of the inner method at the solution (the variable written as a decimal string).
    /// Running the inner method on it reproduces `inner_value` and its full explain.
    pub solved_input: Value,
    /// `value` of the inner method's result at the solution.
    pub inner_value: Value,
}

pub fn goal_seek(input: &GoalSeekInput) -> Result<(GoalSeekOutput, Explain)> {
    let mut inner = Inner::new(&input.method)?;
    let [low, high] = input.bracket;
    if low >= high {
        return Err(FairmarkError::InvalidInput(format!(
            "bracket low ({low}) must be lower than high ({high})"
        )));
    }
    if input.tolerance <= Decimal::ZERO {
        return Err(FairmarkError::InvalidInput(format!(
            "tolerance must be greater than 0 (got {})",
            input.tolerance
        )));
    }
    if !(1..=MAX_ITERATIONS_LIMIT).contains(&input.max_iterations) {
        return Err(FairmarkError::InvalidInput(format!(
            "max_iterations must be between 1 and {MAX_ITERATIONS_LIMIT} (got {})",
            input.max_iterations
        )));
    }
    let base = base_value(&input.base_input);
    pointer::read_decimal(&base, &input.variable)?;
    pointer::check_syntax(&input.output)?;
    // R5: the most evaluations this request can need, checked before the first one.
    Inner::reserve(2 + u64::from(input.max_iterations))?;

    // One entry per evaluation, in the order of the solver's trace: (output, inner value).
    let mut values: Vec<(Decimal, Value)> = Vec::new();
    let root = brent(
        |x| {
            let mut candidate = base.clone();
            pointer::write_decimal(&mut candidate, &input.variable, x)?;
            let value = inner.evaluate(&candidate)?;
            let y = output(&value, &input.output)?;
            values.push((y, value));
            sub(y, input.target)
        },
        low,
        high,
        input.tolerance,
        input.max_iterations,
    )?;

    if values.len() != root.trace.len() {
        return Err(FairmarkError::Internal(
            "goal seek trace and evaluations differ",
        ));
    }
    let mut solved_input = base.clone();
    pointer::write_decimal(&mut solved_input, &input.variable, root.x)?;
    // Keys sorted, so the result does not depend on the order in which the caller wrote them.
    let solved_input = sorted(&solved_input);
    let outputs: Vec<Decimal> = values.iter().map(|(y, _)| *y).collect();
    let (achieved, inner_value) = values
        .pop()
        .ok_or(FairmarkError::Internal("goal seek has no evaluation"))?;

    let mut ex = Explain::new(
        GoalSeek::ID,
        "find x in [low, high] such that output(x) − target = 0 (Brent's method)",
    );
    for (i, (Point { x, fx, kind }, y)) in root.trace.iter().zip(&outputs).enumerate() {
        ex.step(
            format!("Evaluation {} ({})", i + 1, kind.label()),
            format!(
                "{} = {x} → {} = {y}; {y} − {} = {fx}",
                input.variable, input.output, input.target
            ),
            *x,
        );
    }
    ex.step(
        "Solution",
        format!(
            "{} = {}; {} = {achieved}; residual {} within tolerance {}",
            input.variable, root.x, input.output, root.fx, input.tolerance
        ),
        root.x,
    );
    ex.convention(format!(
        "inner method {} version {}, evaluated through the registry",
        inner.id(),
        inner.info().version
    ));
    ex.convention(
        "root finding: Brent's method (inverse quadratic interpolation, secant, bisection)",
    );
    ex.convention(format!(
        "bracket [{low}, {high}] given by the caller; no automatic search"
    ));
    ex.convention("evaluation order: low end, high end, then one evaluation per iteration");
    ex.convention("if the output meets the tolerance at both bracket ends, low is returned");
    ex.convention(
        "with several solutions in the bracket, one is returned, determined by the iteration \
        path; narrow the bracket to select another",
    );
    ex.convention(format!(
        "stop when |output − target| ≤ {} (absolute, in output units){}",
        input.tolerance,
        if input.tolerance == default_tolerance() {
            ", the default"
        } else {
            ""
        }
    ));
    ex.convention(format!(
        "at most {} iterations after the two bracket ends{}",
        input.max_iterations,
        if input.max_iterations == DEFAULT_MAX_ITERATIONS {
            ", the default"
        } else {
            ""
        }
    ));
    ex.convention("minimum step on the variable 10^-24 × max(1, |x|) at the current point x");
    ex.convention("the variable is written into the inner input as a decimal string");
    ex.convention(format!(
        "{} evaluations of the inner method",
        inner.evaluations()
    ));

    Ok((
        GoalSeekOutput {
            inner_method: inner.info(),
            solution: root.x,
            achieved,
            residual: root.fx,
            iterations: root.iterations,
            tolerance: input.tolerance,
            solved_input,
            inner_value,
        },
        ex,
    ))
}

// ---- method registrations

/// `analysis.goal_seek`.
pub struct GoalSeek;

impl Method for GoalSeek {
    const ID: &'static str = "analysis.goal_seek";
    const TITLE: &'static str = "Goal seek";
    const DESCRIPTION: &'static str =
        "Value of one decimal input of any method at which one of its outputs equals a target, \
        found with Brent's method in an explicit bracket; every evaluation is reported.";
    const VERSION: &'static str = "1";
    type Input = GoalSeekInput;
    type Output = GoalSeekOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        goal_seek(input)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    fn run(input: Value) -> crate::Result<Value> {
        crate::run("analysis.goal_seek", &input.to_string())
    }

    fn npv_request() -> Value {
        json!({
            "method": "tvm.npv",
            "base_input": {"rate": "0.05", "cash_flows": [-1000, 300, 400, 500]},
            "variable": "/rate", "output": "/npv", "target": "0", "bracket": ["0", "1"]
        })
    }

    fn code(input: Value) -> &'static str {
        run(input).unwrap_err().code()
    }

    #[test]
    fn invalid_parameters_are_e001() {
        let mut r = npv_request();
        r["bracket"] = json!(["1", "1"]);
        assert_eq!(code(r), "FM-E001");
        let mut r = npv_request();
        r["tolerance"] = json!("0");
        assert_eq!(code(r), "FM-E001");
        for n in [0, 1001] {
            let mut r = npv_request();
            r["max_iterations"] = json!(n);
            assert_eq!(code(r), "FM-E001");
        }
        let mut r = npv_request();
        r["unknown"] = json!(1);
        assert_eq!(code(r), "FM-E001");
    }

    #[test]
    fn unknown_inner_method_is_e007_and_wrong_pointers_e015() {
        let mut r = npv_request();
        r["method"] = json!("x.y");
        assert_eq!(code(r), "FM-E007");
        for (field, pointer) in [
            ("variable", "/missing"),
            ("variable", "rate"),
            ("output", "/present_values"),
        ] {
            let mut r = npv_request();
            r[field] = json!(pointer);
            assert_eq!(code(r), "FM-E015", "{field} {pointer}");
        }
    }

    #[test]
    fn errors_of_the_inner_method_keep_their_code() {
        // A bracket end at rate -1 is outside the domain of tvm.npv.
        let mut r = npv_request();
        r["bracket"] = json!(["-1", "1"]);
        assert_eq!(code(r), "FM-E003");
        // A base input the inner method rejects.
        let mut r = npv_request();
        r["base_input"]["extra"] = json!(1);
        assert_eq!(code(r), "FM-E001");
    }

    #[test]
    fn key_order_and_number_spelling_do_not_change_the_result() {
        let a = run(npv_request()).unwrap();
        let reordered = json!({
            "bracket": ["0", "1"], "target": "0", "output": "/npv", "variable": "/rate",
            "base_input": {"cash_flows": [-1000, 300, 400, 500], "rate": "0.05"},
            "method": "tvm.npv"
        });
        let b = run(reordered).unwrap();
        assert_eq!(
            a["value"].to_string(),
            b["value"].to_string(),
            "byte-identical value"
        );
        assert_eq!(a["meta"]["input_hash"], b["meta"]["input_hash"]);
        let mut numeric = npv_request();
        numeric["base_input"]["rate"] = serde_json::from_str("0.05").unwrap();
        assert_eq!(run(numeric).unwrap()["value"], a["value"]);
        assert_eq!(run(npv_request()).unwrap().to_string(), a.to_string());
    }

    #[test]
    fn the_result_names_the_inner_method_and_reproduces() {
        let out = run(npv_request()).unwrap();
        let v = &out["value"];
        let info = crate::method("tvm.npv").unwrap();
        assert_eq!(
            v["inner_method"],
            json!({"id": info.id, "version": info.version})
        );
        let again = crate::run("tvm.npv", &v["solved_input"].to_string()).unwrap();
        assert_eq!(again["value"], v["inner_value"]);
        assert!(v["solved_input"]["rate"].is_string());
        let steps = out["explain"]["steps"].as_array().unwrap().len() as u64;
        assert_eq!(steps, 2 + v["iterations"].as_u64().unwrap() + 1);
    }

    /// AC5 with the tolerance of AC1: two bracket ends, one labelled step per iteration, then
    /// the solution.
    #[test]
    fn explain_lists_every_evaluation_with_its_step_type() {
        let mut r = npv_request();
        r["tolerance"] = json!("0.000000000000000001");
        let out = run(r).unwrap();
        let steps = out["explain"]["steps"].as_array().unwrap();
        let iterations = out["value"]["iterations"].as_u64().unwrap() as usize;
        assert_eq!(steps.len(), 2 + iterations + 1);
        let label = |i: usize| steps[i]["label"].as_str().unwrap().to_owned();
        assert!(label(0).ends_with("(bracket end)") && label(1).ends_with("(bracket end)"));
        for i in 2..2 + iterations {
            let l = label(i);
            assert!(
                [
                    "(bisection)",
                    "(secant)",
                    "(inverse quadratic interpolation)"
                ]
                .iter()
                .any(|k| l.ends_with(k)),
                "{l}"
            );
        }
        assert_eq!(label(2 + iterations), "Solution");
    }
}
