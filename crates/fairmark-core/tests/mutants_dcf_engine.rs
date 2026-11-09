//! Behaviour tests for the shared DCF engine (`dcf/engine.rs`) written to kill the missed
//! mutants of the first mutation-testing baseline (issue #282, `tests/mutants/triage.md`).
//! Each test cites the requirement of SPEC-0001 it checks. Expected values are either the
//! rules of the spec themselves (bounds, messages that name the field, conventions and
//! warnings) or the values the same result reports, substituted in its audit trail
//! (constitution, principle II); no expected number is copied from the engine.

use fairmark_core::run;
use serde_json::{json, Value};

fn ok(method: &str, input: &Value) -> Value {
    run(method, &input.to_string()).unwrap_or_else(|e| panic!("{method}: {e}"))
}

fn err(method: &str, input: &Value) -> (String, String) {
    let e = run(method, &input.to_string()).expect_err("the input must be refused");
    (e.code().to_owned(), e.to_string())
}

fn conventions(out: &Value) -> Vec<String> {
    serde_json::from_value(out["explain"]["conventions"].clone()).unwrap()
}

fn step_formula(out: &Value, label: &str) -> String {
    out["explain"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["label"] == label)
        .unwrap_or_else(|| panic!("no step {label:?}"))["formula"]
        .as_str()
        .unwrap()
        .to_owned()
}

fn has_step(out: &Value, label: &str) -> bool {
    out["explain"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .any(|s| s["label"] == label)
}

fn assert_refused(input: &Value, code: &str, parts: &[&str]) {
    let (c, message) = err("dcf.fcff", input);
    assert_eq!(c, code, "{message}");
    for part in parts {
        assert!(message.contains(part), "missing {part:?} in: {message}");
    }
}

/// One complete build-up line (SPEC-0001 R2).
fn line() -> Value {
    json!({"ebit": "100", "depreciation_and_amortisation": "10", "capital_expenditure": "12",
           "change_in_net_working_capital": "3"})
}

/// A complete input around the given stages: nominal terms, one WACC, no terminal value.
fn with_stages(stages: Value) -> Value {
    json!({
        "stages": stages,
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": "0.10",
        "timing": "end_of_period",
        "terminal": {"none": {}},
        "net_financial_position": {"gross_debt": "0", "cash": "0"}
    })
}

// ------------------------------------------------------------- periods and explain (R10)

/// SPEC-0001 R10, SPEC-0002 R5: every period is returned with its number, 1 to N, and recorded
/// in `explain`; the discount factor of period k is DF_(k−1) / (1 + r_k), with the factor of
/// the previous period substituted.
#[test]
fn periods_are_numbered_from_one_and_discount_factors_chain() {
    let out = ok(
        "dcf.fcff",
        &with_stages(json!([{"periods": 3, "projection": "given",
                              "flows": ["100", "110", "120"]}])),
    );
    let periods = out["value"]["periods"].as_array().unwrap();
    let numbers: Vec<u64> = periods
        .iter()
        .map(|p| p["period"].as_u64().unwrap())
        .collect();
    assert_eq!(numbers, [1, 2, 3]);
    let df = |k: usize| periods[k]["discount_factor"].as_str().unwrap().to_owned();
    assert_eq!(
        step_formula(&out, "Period 1: discount factor"),
        "1 / (1 + 0.10)"
    );
    assert_eq!(
        step_formula(&out, "Period 2: discount factor"),
        format!("{} / (1 + 0.10)", df(0))
    );
    assert_eq!(
        step_formula(&out, "Period 3: discount factor"),
        format!("{} / (1 + 0.10)", df(1))
    );
    for k in 1..=3 {
        assert!(has_step(&out, &format!("Period {k}: discount rate")));
        assert!(has_step(&out, &format!("Period {k}: present value")));
    }
    assert!(!has_step(&out, "Period 0: discount rate"));
}

// ------------------------------------------------------------- stages (R6 to R8, R27)

/// SPEC-0001 R6: one to three explicit stages; three are accepted.
#[test]
fn three_stages_are_accepted() {
    let out = ok(
        "dcf.fcff",
        &with_stages(json!([
            {"periods": 1, "projection": "given", "flows": ["100"]},
            {"periods": 1, "projection": "constant_growth", "growth": "0.05"},
            {"periods": 2, "projection": "linear_decline", "from_growth": "0.04",
             "to_growth": "0.02"}
        ])),
    );
    assert_eq!(out["value"]["periods"].as_array().unwrap().len(), 4);
}

/// SPEC-0001 R6 and the documented limit of the explicit horizon (10,000 periods): a horizon of
/// exactly 10,000 periods is accepted, 10,001 is refused.
#[test]
fn horizon_of_ten_thousand_periods_is_accepted() {
    let input = |periods: u32| {
        let mut v = with_stages(json!([{"periods": periods, "projection": "constant_growth",
                                        "growth": "0"}]));
        v["base_flow"] = json!("1");
        v
    };
    let out = ok("dcf.fcff", &input(10_000));
    assert_eq!(out["value"]["periods"].as_array().unwrap().len(), 10_000);
    assert_refused(
        &input(10_001),
        "FM-E001",
        &["10001 periods", "at most 10000"],
    );
}

/// SPEC-0001 R27: every error names the projection it applies to by its input value.
#[test]
fn stage_errors_name_the_projection() {
    assert_refused(
        &with_stages(
            json!([{"periods": 1, "projection": "given", "flows": ["100"],
                             "growth": "0.02"}]),
        ),
        "FM-E001",
        &["`/stages/0/growth` is not used by the projection `given`"],
    );
    let mut generated = with_stages(json!([{"periods": 1, "projection": "constant_growth",
                                            "growth": "0.02",
                                            "lines": [line()]}]));
    generated["base_flow"] = json!("100");
    assert_refused(
        &generated,
        "FM-E001",
        &["`/stages/0/lines` is not used by the projection `constant_growth`"],
    );
    assert_refused(
        &with_stages(json!([
            {"periods": 1, "projection": "given", "flows": ["100"]},
            {"projection": "linear_decline", "from_growth": "0.04", "to_growth": "0.02"}
        ])),
        "FM-E001",
        &["`/stages/1/periods` is required by the projection `linear_decline`"],
    );
}

/// SPEC-0001 R7, R27: a `given` stage needs `flows` or `lines`; the message names both fields.
#[test]
fn given_stage_without_flows_or_lines_names_both_fields() {
    assert_refused(
        &with_stages(json!([{"periods": 3, "projection": "given"}])),
        "FM-E001",
        &["`/stages/0/flows` or `/stages/0/lines` is required by the projection `given`"],
    );
}

/// SPEC-0001 R6 (Decision 24), R27: an empty list of flows or lines gives no default number of
/// periods; the message names the empty field.
#[test]
fn empty_flows_or_lines_name_the_empty_field() {
    assert_refused(
        &with_stages(json!([{"projection": "given", "flows": []}])),
        "FM-E001",
        &["`/stages/0/flows` is empty"],
    );
    assert_refused(
        &with_stages(json!([{"projection": "given", "lines": []}])),
        "FM-E001",
        &["`/stages/0/lines` is empty"],
    );
}

/// SPEC-0001 R6 (Decision 24): the default number of periods is the number of flows, or of
/// lines, of the stage, and its reason says which.
#[test]
fn default_periods_reason_names_flows_or_lines() {
    let flows = ok(
        "dcf.fcff",
        &json!({"stages": [{"flows": ["100", "110"]}], "wacc": "0.10"}),
    );
    let lines = ok(
        "dcf.fcff",
        &json!({"stages": [{"lines": [
            {"ebit": "100", "depreciation_and_amortisation": "10", "capital_expenditure": "12",
             "change_in_net_working_capital": "3"},
            {"ebit": "110", "depreciation_and_amortisation": "11", "capital_expenditure": "13",
             "change_in_net_working_capital": "4"}
        ]}], "notional_tax_rate": "0.24", "negative_ebit": "zero", "wacc": "0.10"}),
    );
    let reason = |out: &Value| {
        out["meta"]["defaults_applied"]
            .as_array()
            .unwrap()
            .iter()
            .find(|d| d["field"] == "/stages/0/periods")
            .unwrap()["reason"]
            .as_str()
            .unwrap()
            .to_owned()
    };
    assert_eq!(reason(&flows), "the number of flows given in the stage");
    assert_eq!(reason(&lines), "the number of lines given in the stage");
}

/// SPEC-0001 R10: the conventions describe each stage; flows given as amounts are not
/// described as built up from line items.
#[test]
fn given_flows_stage_convention() {
    let out = ok(
        "dcf.fcff",
        &with_stages(json!([{"periods": 2, "projection": "given", "flows": ["100", "110"]}])),
    );
    assert!(
        conventions(&out).contains(&"stage 1: periods 1 to 2, given flows".to_owned()),
        "{:?}",
        conventions(&out)
    );
}

/// SPEC-0001 R11: growth applied to a negative flow is reported as a warning; applied to a
/// positive flow it is not.
#[test]
fn negative_flow_warning_only_for_negative_flows() {
    let input = |base: &str| {
        let mut v = with_stages(json!([{"periods": 2, "projection": "constant_growth",
                                        "growth": "0.05"}]));
        v["base_flow"] = json!(base);
        v
    };
    let warns = |out: &Value| {
        conventions(out)
            .iter()
            .filter(|c| c.contains("applied to a negative flow"))
            .count()
    };
    assert_eq!(warns(&ok("dcf.fcff", &input("100"))), 0);
    assert_eq!(warns(&ok("dcf.fcff", &input("-100"))), 2);
}

/// SPEC-0001 R1, R3: interest expense and net borrowing belong to FCFE; on the enterprise
/// basis either one alone is refused; the message names only the fields given, each with its
/// pointer to the line.
#[test]
fn financing_lines_refused_on_the_enterprise_basis_one_at_a_time() {
    /// The financing fields given on the line, those the message must name, those it must not.
    struct Case {
        given: &'static [(&'static str, &'static str)],
        named: &'static [&'static str],
        absent: &'static [&'static str],
    }
    let cases = [
        Case {
            given: &[("interest_expense", "10")],
            named: &["interest_expense"],
            absent: &["net_borrowing"],
        },
        Case {
            given: &[("net_borrowing", "5")],
            named: &["net_borrowing"],
            absent: &["interest_expense"],
        },
        Case {
            given: &[("interest_expense", "10"), ("net_borrowing", "5")],
            named: &["interest_expense", "net_borrowing"],
            absent: &[],
        },
    ];
    for Case {
        given,
        named,
        absent,
    } in cases
    {
        let mut line = line();
        for (field, value) in given {
            line[*field] = json!(value);
        }
        let mut input = with_stages(json!([{"periods": 1, "projection": "given",
                                            "lines": [line]}]));
        input["notional_tax_rate"] = json!("0.24");
        let pointers: Vec<String> = named
            .iter()
            .map(|f| format!("`/stages/0/lines/0/{f}`"))
            .collect();
        let mut parts: Vec<&str> = pointers.iter().map(String::as_str).collect();
        parts.extend(if named.len() == 1 {
            ["is not used on the enterprise basis", "remove it"]
        } else {
            ["are not used on the enterprise basis", "remove them"]
        });
        assert_refused(&input, "FM-E001", &parts);
        let (_, message) = err("dcf.fcff", &input);
        for f in absent {
            assert!(!message.contains(f), "{f} not given, named in: {message}");
        }
    }
}

// ------------------------------------------------------------- inflation (R12, R16 to R18)

fn real_flows_per_period_inflation() -> Value {
    let mut v = with_stages(json!([{"periods": 2, "projection": "given",
                                    "flows": ["100", "110"]}]));
    v["cash_flow_terms"] = json!("real");
    v["inflation_per_period"] = json!(["0.02", "0.03"]);
    v
}

/// SPEC-0001 R12: without a terminal value no terminal rate is used, so a terminal inflation
/// is refused and naming it is the fix.
#[test]
fn terminal_inflation_refused_without_a_terminal_rate() {
    let mut input = real_flows_per_period_inflation();
    input["terminal_inflation"] = json!("0.02");
    assert_refused(&input, "FM-E001", &["`/terminal_inflation` is not used"]);
}

/// SPEC-0001 R12, R16: inflation per period without a terminal inflation is accepted when the
/// terminal value uses no terminal rate, and the conventions report one inflation per period.
#[test]
fn per_period_inflation_without_terminal_rate_needs_no_terminal_inflation() {
    let out = ok("dcf.fcff", &real_flows_per_period_inflation());
    let c = conventions(&out);
    assert!(
        c.contains(
            &"expected inflation: one rate per period (`/inflation_per_period`); no terminal \
              inflation, the terminal value uses no terminal rate"
                .to_owned()
        ),
        "{c:?}"
    );
    assert!(
        !c.iter()
            .any(|c| c.starts_with("expected inflation: one rate,")),
        "{c:?}"
    );
}

/// SPEC-0001 R17: for nominal flows a perpetual growth below the expected inflation is a
/// warning; a growth equal to it is not a real decline and gives no warning.
#[test]
fn real_decline_warning_only_below_inflation() {
    let input = |growth: &str| {
        let mut v = with_stages(json!([{"periods": 2, "projection": "given",
                                        "flows": ["100", "110"]}]));
        v["inflation"] = json!("0.02");
        v["terminal"] = json!({"tv.gordon": {"growth": growth,
                                             "first_flow": "last_flow_grown"}});
        v
    };
    let warns = |out: &Value| {
        conventions(out)
            .iter()
            .any(|c| c.starts_with("warning: the perpetual growth"))
    };
    assert!(!warns(&ok("dcf.fcff", &input("0.02"))));
    assert!(warns(&ok("dcf.fcff", &input("0.019"))));
}
