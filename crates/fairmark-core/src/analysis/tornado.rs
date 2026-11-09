//! `analysis.tornado`: which inputs move one output of any method the most, each varied alone
//! between a low and a high value, ranked by swing (SPEC-0008 R20, R21).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{
    base_case, base_value, output, pointer, unique, with_values, BaseInput, Inner, InnerMethod,
};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::sub;

/// One input to vary, with its low and high values.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TornadoVariable {
    /// RFC 6901 JSON Pointer to a decimal input of the inner method, for example
    /// `/discount_rate`. Variables do not repeat.
    pub variable: String,
    /// Low value of the input, in its units; lower than `high`.
    pub low: Decimal,
    /// High value of the input, in its units.
    pub high: Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TornadoInput {
    /// Id of the method to evaluate, for example `dcf.fcff`. Analysis methods are not accepted.
    pub method: String,
    /// Input of that method, complete: the base case. Each variable is changed alone, the
    /// others keep their base values.
    pub base_input: BaseInput,
    /// RFC 6901 JSON Pointer to the decimal output to rank, in the `value` of the inner
    /// result, for example `/equity_value`.
    pub output: String,
    /// The inputs to vary; at least one.
    pub variables: Vec<TornadoVariable>,
}

/// One bar of the tornado.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TornadoBar {
    /// The input varied.
    pub variable: String,
    /// Low value of the input.
    pub low: Decimal,
    /// High value of the input.
    pub high: Decimal,
    /// The output with the input at `low`, in the output's units.
    pub output_low: Decimal,
    /// The output with the input at `high`, in the output's units.
    pub output_high: Decimal,
    /// `output_high − output_low`: negative when the output falls as the input rises.
    pub swing: Decimal,
    /// `|swing|`, the length of the bar and the ranking key.
    pub abs_swing: Decimal,
}

/// Output of `analysis.tornado`.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct TornadoOutput {
    /// The method evaluated, at the version run.
    pub inner_method: InnerMethod,
    /// The output ranked.
    pub output: String,
    /// The output for `base_input` unchanged, in the output's units.
    pub base: Decimal,
    /// The bars by decreasing `abs_swing`; equal swings keep the order of `variables`.
    pub ranking: Vec<TornadoBar>,
}

pub fn tornado(input: &TornadoInput) -> Result<(TornadoOutput, Explain)> {
    let mut inner = Inner::new(&input.method)?;
    let base = base_value(&input.base_input);
    if input.variables.is_empty() {
        return Err(FairmarkError::InvalidInput("variables is empty".to_owned())
            .at("/variables", crate::error::Reason::Length));
    }
    unique(
        "variable",
        input.variables.iter().map(|v| v.variable.as_str()),
    )?;
    for v in &input.variables {
        pointer::read_decimal(&base, &v.variable)?;
        if v.low >= v.high {
            return Err(FairmarkError::InvalidInput(format!(
                "{}: low ({}) must be lower than high ({})",
                v.variable, v.low, v.high
            ))
            .because(crate::error::Reason::Range));
        }
    }
    pointer::check_syntax(&input.output)?;
    inner.reserve(2 * input.variables.len() as u64 + 1, &base)?;

    let base_output = base_case(&mut inner, &base, &[input.output.as_str()])?[0];
    let mut ex = Explain::new(
        Tornado::ID,
        "swing = output(variable at high) − output(variable at low), other inputs at base; \
        bars ranked by |swing|",
    );
    ex.step(
        "Base case",
        format!("{} with base_input unchanged", input.output),
        base_output,
    );

    let mut bars = Vec::with_capacity(input.variables.len());
    for v in &input.variables {
        // R21: a failed evaluation leaves the ranking undefined, so its error is returned.
        let mut at = |x: Decimal| -> Result<Decimal> {
            let value = inner.evaluate(&with_values(&base, &[(v.variable.as_str(), x)])?)?;
            output(&value, &input.output)
        };
        let output_low = at(v.low)?;
        let output_high = at(v.high)?;
        let swing = sub(output_high, output_low)?;
        ex.step(
            format!("{} at low", v.variable),
            format!(
                "{} = {} → {} = {output_low}",
                v.variable, v.low, input.output
            ),
            output_low,
        );
        ex.step(
            format!("{} at high", v.variable),
            format!(
                "{} = {} → {} = {output_high}",
                v.variable, v.high, input.output
            ),
            output_high,
        );
        ex.step(
            format!("Swing of {}", v.variable),
            format!(
                "{output_high} − {output_low} = {swing}; |swing| = {}",
                swing.abs()
            ),
            swing,
        );
        bars.push(TornadoBar {
            variable: v.variable.clone(),
            low: v.low,
            high: v.high,
            output_low,
            output_high,
            swing,
            abs_swing: swing.abs(),
        });
    }
    // Stable sort: equal swings keep the declaration order (R20).
    bars.sort_by_key(|bar| std::cmp::Reverse(bar.abs_swing));

    ex.convention(format!(
        "inner method {} version {}, evaluated through the registry",
        inner.id(),
        inner.info().version
    ));
    if let Some(note) = inner.status_convention() {
        ex.convention(note);
    }
    ex.convention(format!("the bars measure {}", input.output));
    ex.convention("each variable is varied alone; the other inputs keep their base values");
    ex.convention("low and high values are given explicitly; no percentage shocks");
    ex.convention(
        "evaluation order: base case, then each variable in the order given, low before high",
    );
    ex.convention("ranking by |swing|, largest first; equal swings keep the order given");
    ex.convention(
        "any evaluation the inner method refuses ends the request with its error: a ranking \
        with a missing bar is not defined",
    );
    ex.convention("the variables are written into the inner input as decimal strings");
    ex.convention(format!(
        "{} evaluations of the inner method",
        inner.evaluations()
    ));

    Ok((
        TornadoOutput {
            inner_method: inner.info(),
            output: input.output.clone(),
            base: base_output,
            ranking: bars,
        },
        ex,
    ))
}

// ---- method registrations

/// `analysis.tornado`.
pub struct Tornado;

impl Method for Tornado {
    const ID: &'static str = "analysis.tornado";
    const TITLE: &'static str = "Tornado";
    const DESCRIPTION: &'static str =
        "Which inputs move one output of any method the most: each input varied alone between \
        explicit low and high values, ranked by the absolute swing of the output.";
    const VERSION: &'static str = "1";
    type Input = TornadoInput;
    type Output = TornadoOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        tornado(input)
    }

    fn evaluated_method(input: &Self::Input) -> Option<&str> {
        Some(&input.method)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    fn run(input: &Value) -> crate::Result<Value> {
        crate::run("analysis.tornado", &input.to_string())
    }

    fn request() -> Value {
        json!({
            "method": "dcf.fcff",
            "base_input": {"stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}],
                           "cash_flow_terms": "nominal", "discount_rate_terms": "nominal",
                           "wacc": "0.09", "timing": "end_of_period",
                           "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
                           "net_financial_position": {"gross_debt": 50, "cash": 0}},
            "output": "/equity_value",
            "variables": [
                {"variable": "/net_financial_position/gross_debt", "low": "40", "high": "60"},
                {"variable": "/wacc", "low": "0.08", "high": "0.10"}
            ]
        })
    }

    #[test]
    fn ranked_by_absolute_swing_with_signed_swing_kept() {
        let v = run(&request()).unwrap()["value"].clone();
        assert_eq!(v["ranking"][0]["variable"], "/wacc");
        assert_eq!(
            v["ranking"][1]["variable"],
            "/net_financial_position/gross_debt"
        );
        let dec = |x: &Value| {
            x.as_str()
                .unwrap()
                .parse::<rust_decimal::Decimal>()
                .unwrap()
        };
        assert_eq!(
            dec(&v["ranking"][1]["swing"]),
            rust_decimal::Decimal::from(-20)
        );
        assert_eq!(
            dec(&v["ranking"][1]["abs_swing"]),
            rust_decimal::Decimal::from(20)
        );
    }

    /// R9, AC18: a base case the inner method refuses returns its error unchanged.
    #[test]
    fn a_failing_base_case_returns_its_error() {
        let mut r = request();
        r["base_input"]["terminal"]["tv.gordon"]["growth"] = json!("0.3");
        assert_eq!(run(&r).unwrap_err().code(), "FM-E004");
    }

    #[test]
    fn invalid_requests_and_failed_evaluations_are_errors() {
        for (path, value, code) in [
            (
                "/variables/1/variable",
                json!("/net_financial_position/gross_debt"),
                "FM-E001",
            ),
            ("/variables/0/low", json!("60"), "FM-E001"),
            ("/variables", json!([]), "FM-E001"),
            ("/variables/0/variable", json!("/timing"), "FM-E015"),
            ("/method", json!("analysis.goal_seek"), "FM-E017"),
            // R21: growth above the rate at the high end: the ranking is not defined.
            (
                "/variables/1",
                json!({"variable": "/terminal/tv.gordon/growth", "low": "0.01",
                                    "high": "0.2"}),
                "FM-E004",
            ),
        ] {
            let mut r = request();
            *r.pointer_mut(path).unwrap() = value;
            assert_eq!(run(&r).unwrap_err().code(), code, "{path}");
        }
    }
}
