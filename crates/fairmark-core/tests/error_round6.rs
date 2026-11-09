//! Round 6 of the review of #279: when reading refuses a value, the method's own checks are
//! reported only when none of the fields they read is refused (declared reads, ADR 0014 G5).
//! The cases are those of the review and of the independent audit of round 5: errors that a
//! value put in place of a refused one raised, and independent errors that were dropped.

use std::path::Path;

use fairmark_core::run;
use serde_json::{json, Value};

fn report(method: &str, input: &Value) -> Value {
    let err = run(method, &input.to_string()).expect_err("the input must be refused");
    let report = serde_json::to_value(err.to_report()).unwrap();
    assert_eq!(report["message"], err.to_string());
    report
}

fn golden_input(relative: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(relative);
    let case: Value = serde_norway::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    case["input"].clone()
}

fn entries(r: &Value) -> Vec<(String, String)> {
    r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| {
            (
                e["pointer"].as_str().unwrap_or("null").to_owned(),
                e["reason"].as_str().unwrap_or("null").to_owned(),
            )
        })
        .collect()
}

fn entry(pointer: &str, reason: &str) -> (String, String) {
    (pointer.to_owned(), reason.to_owned())
}

fn has(r: &Value, pointer: &str, reason: &str) -> bool {
    entries(r).contains(&entry(pointer, reason))
}

// ------------------------------------------------- no error from a value put in place

#[test]
fn the_rate_rule_is_skipped_when_its_compounding_is_refused() {
    let cases = [
        (
            "loans.amortisation_schedule",
            "loans.amortisation_schedule/fixed-amortisation-quarterly.yaml",
            "/rate",
            "/compounding",
        ),
        (
            "rates.convert",
            "rates.convert/annual-to-bimonthly.yaml",
            "/rate",
            "/from",
        ),
        (
            "tvm.xnpv",
            "tvm.xnpv/act-act-quarterly.yaml",
            "/rate",
            "/compounding",
        ),
    ];
    for (method, file, rate, compounding) in cases {
        for refused in [json!(true), json!("bogus_value")] {
            let mut input = golden_input(file);
            input[&rate[1..]] = json!("-5");
            input[&compounding[1..]] = refused.clone();
            let r = report(method, &input);
            assert_eq!(entries(&r), [entry(compounding, "enum")], "{method}: {r:#}");
        }
    }
}

#[test]
fn the_rate_rule_names_the_compounding() {
    // A continuous rate may be -2: whether -5 is refused depends on the compounding.
    let mut input = golden_input("rates.convert/annual-to-bimonthly.yaml");
    input["rate"] = json!("-5");
    let r = report("rates.convert", &input);
    assert_eq!(r["pointers"], json!(["/rate", "/from"]), "{r:#}");
    let mut input = golden_input("loans.amortisation_schedule/fixed-amortisation-quarterly.yaml");
    input["rate"] = json!("-5");
    let r = report("loans.amortisation_schedule", &input);
    assert_eq!(r["pointers"], json!(["/rate", "/compounding"]), "{r:#}");
}

#[test]
fn mixed_basis_is_skipped_when_the_basis_is_refused() {
    for refused in [json!(true), json!("bogus_value")] {
        let mut input = golden_input("tv.compare/equity-basis.yaml");
        input["basis"] = refused;
        let r = report("tv.compare", &input);
        assert_eq!(entries(&r), [entry("/basis", "enum")], "{r:#}");
    }
    let mut input = golden_input("tv.compare/equity-basis.yaml");
    input.as_object_mut().unwrap().shift_remove("basis");
    let r = report("tv.compare", &input);
    assert_eq!(entries(&r), [entry("/basis", "missing")], "{r:#}");
}

#[test]
fn mixed_basis_names_the_basis() {
    let mut input = golden_input("tv.compare/equity-basis.yaml");
    input["basis"] = json!("enterprise");
    let r = report("tv.compare", &input);
    assert_eq!(r["code"], "FM-E019");
    assert_eq!(
        r["pointers"],
        json!(["/techniques/1/tv.exit_multiple/metric", "/basis"]),
        "{r:#}"
    );
}

#[test]
fn a_refused_projection_requires_no_flows() {
    for refused in [json!(true), json!("bogus_value")] {
        let mut input = golden_input("dcf.fcff/terms-inflation-per-period.yaml");
        input["stages"][1]["projection"] = refused;
        let r = report("dcf.fcff", &input);
        assert_eq!(
            entries(&r),
            [entry("/stages/1/projection", "enum")],
            "{r:#}"
        );
    }
}

#[test]
fn a_refused_timeline_decides_no_growth_basis() {
    let mut input = golden_input("series.project/bimonthly-annual-simple.yaml");
    input["timeline"]["periods"] = json!(5);
    let r = report("series.project", &input);
    assert_eq!(entries(&r), [entry("/timeline/periods", "type")], "{r:#}");
    let mut input = golden_input("series.project/bimonthly-annual-simple.yaml");
    input.as_object_mut().unwrap().shift_remove("timeline");
    let r = report("series.project", &input);
    assert_eq!(entries(&r), [entry("/timeline", "missing")], "{r:#}");
}

#[test]
fn a_refused_line_decides_no_interest_tax_rate() {
    for field in ["interest_expense", "net_borrowing"] {
        let mut input = golden_input("dcf.fcfe/stage-with-build-up-lines-tax-amounts.yaml");
        input["stages"][0]["lines"][0][field] = json!(true);
        input
            .as_object_mut()
            .unwrap()
            .shift_remove("interest_tax_rate");
        let r = report("dcf.fcfe", &input);
        let at = format!("/stages/0/lines/0/{field}");
        assert_eq!(entries(&r), [entry(&at, "type")], "{r:#}");
    }
}

#[test]
fn a_refused_metric_is_never_named_by_another_error() {
    for refused in [json!(true), json!("bogus_value")] {
        let mut input = golden_input("tv.exit_multiple/pbv-equity-basis.yaml");
        input["metric_value"] = json!("-1");
        input["metric"] = refused;
        let r = report("tv.exit_multiple", &input);
        // The only entry is the metric's own: no error names the value put in its place.
        assert_eq!(entries(&r), [entry("/metric", "enum")], "{r:#}");
    }
    // Given, the metric is a side of the error on its value.
    let mut input = golden_input("tv.exit_multiple/pbv-equity-basis.yaml");
    input["metric_value"] = json!("-1");
    let r = report("tv.exit_multiple", &input);
    assert_eq!(r["pointers"], json!(["/metric_value", "/metric"]), "{r:#}");
}

#[test]
fn a_misspelt_key_stands_for_its_field() {
    let mut input = golden_input("dcf.free_cash_flows/shield-negative-ebit-zero.yaml");
    let map = input.as_object_mut().unwrap();
    let rate = map.shift_remove("notional_tax_rate").unwrap();
    map.insert("notional_ta_rate".into(), rate);
    let r = report("dcf.free_cash_flows", &input);
    // No check that needs the rate runs: the key stands for it.
    for (pointer, _) in entries(&r) {
        assert!(
            ["/notional_ta_rate", "/notional_tax_rate"].contains(&pointer.as_str()),
            "{r:#}"
        );
    }
    assert!(has(&r, "/notional_ta_rate", "unknown_field"), "{r:#}");
}

#[test]
fn a_missing_field_next_to_its_misspelt_key_names_both() {
    let mut input = golden_input("coc.cost_of_debt/spread.yaml");
    let map = input.as_object_mut().unwrap();
    map.shift_remove("credit_spread");
    map.insert("credit_sprea".into(), json!("0.025"));
    let r = report("coc.cost_of_debt", &input);
    assert_eq!(
        entries(&r)[0],
        entry("/credit_sprea", "unknown_field"),
        "{r:#}"
    );
    for e in r["errors"].as_array().unwrap().iter().skip(1) {
        // Whatever else is reported is the missing field the key stands for, with its rename.
        assert_eq!(e["reason"], "missing", "{r:#}");
        assert_eq!(
            e["fix"], "rename `credit_sprea` to `credit_spread`",
            "{r:#}"
        );
    }

    let mut input = json!({"rate": "0.1", "cash_flows": [-100, 60]});
    let flows = input
        .as_object_mut()
        .unwrap()
        .shift_remove("cash_flows")
        .unwrap();
    input["cash_flow"] = flows;
    let r = report("tvm.npv", &input);
    let missing = r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|e| e["reason"] == "missing")
        .expect("the missing field is reported");
    assert_eq!(missing["pointer"], "/cash_flows", "{r:#}");
    assert_eq!(missing["pointers"], json!(["/cash_flows", "/cash_flow"]));
    assert_eq!(missing["fix"], "rename `cash_flow` to `cash_flows`");
}

#[test]
fn a_misspelt_technique_is_one_entry() {
    for (k, typo) in [(0, "tv.godon"), (1, "tv.valuedriver")] {
        let mut input = golden_input("tv.compare/seven-techniques.yaml");
        let entry_map = input["techniques"][k].as_object_mut().unwrap();
        let id = entry_map.keys().next().unwrap().clone();
        let terms = entry_map.shift_remove(&id).unwrap();
        entry_map.insert(typo.into(), terms);
        let r = report("tv.compare", &input);
        let at = format!("/techniques/{k}/{typo}");
        assert_eq!(entries(&r), [entry(&at, "unknown_field")], "{id}: {r:#}");
    }
}

#[test]
fn an_empty_terminal_lists_the_options() {
    let mut input = golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml");
    input["terminal"] = json!({});
    let r = report("dcf.fcff", &input);
    assert_eq!(entries(&r), [entry("/terminal", "length")], "{r:#}");
    assert_eq!(
        r["expected"],
        json!([
            "tv.gordon",
            "tv.h_model",
            "tv.value_driver",
            "tv.exit_multiple",
            "tv.limited_horizon",
            "tv.normalised_income",
            "tv.liquidation",
            "given",
            "none"
        ])
    );
}

// ------------------------------------------------- independent errors are kept

#[test]
fn a_refused_stage_keeps_the_rate_and_the_terminal() {
    let mut input = golden_input("dcf.fcfe/bridge-negative-equity-value.yaml");
    input["stages"][0]["periods"] = json!(-1);
    input["cost_of_equity"] = json!("-5");
    let r = report("dcf.fcfe", &input);
    assert!(has(&r, "/stages/0/periods", "range"), "{r:#}");
    assert!(has(&r, "/cost_of_equity", "range"), "{r:#}");

    let mut input = golden_input("dcf.fcff/terms-inflation-per-period.yaml");
    input["stages"][1]["projection"] = json!("bogus_value");
    input["terminal"]["tv.gordon"]["growth"] = json!("-1");
    let r = report("dcf.fcff", &input);
    assert!(has(&r, "/stages/1/projection", "enum"), "{r:#}");
    assert!(has(&r, "/terminal/tv.gordon/growth", "range"), "{r:#}");
}

#[test]
fn a_refused_first_flow_keeps_the_growth_of_the_technique() {
    let mut input = golden_input("dcf.fcff/terminal-h-model-rates-per-period.yaml");
    input["terminal"]["tv.h_model"]["short_term_growth"] = json!("-1");
    input["terminal"]["tv.h_model"]["first_flow"] = json!(true);
    let r = report("dcf.fcff", &input);
    assert!(has(&r, "/terminal/tv.h_model/first_flow", "enum"), "{r:#}");
    assert!(
        has(&r, "/terminal/tv.h_model/short_term_growth", "range"),
        "{r:#}"
    );
}

#[test]
fn a_missing_metric_keeps_the_multiple() {
    let mut input = golden_input("tv.exit_multiple/pbv-equity-basis.yaml");
    input.as_object_mut().unwrap().shift_remove("metric");
    input["multiple"] = json!("-5");
    let r = report("tv.exit_multiple", &input);
    assert_eq!(
        entries(&r),
        [entry("/metric", "missing"), entry("/multiple", "range")],
        "{r:#}"
    );
}

#[test]
fn a_refused_basis_keeps_the_terms_of_the_techniques() {
    let mut input = golden_input("tv.compare/equity-basis.yaml");
    input["basis"] = json!(true);
    input["techniques"][1]["tv.exit_multiple"]["multiple"] = json!("-1");
    let r = report("tv.compare", &input);
    assert_eq!(
        entries(&r),
        [
            entry("/basis", "enum"),
            entry("/techniques/1/tv.exit_multiple/multiple", "range")
        ],
        "{r:#}"
    );
    // With a basis given, the mixed basis and the terms are both reported.
    input["basis"] = json!("enterprise");
    let r = report("tv.compare", &input);
    assert!(
        has(&r, "/techniques/1/tv.exit_multiple/metric", "conflict"),
        "{r:#}"
    );
    assert!(
        has(&r, "/techniques/1/tv.exit_multiple/multiple", "range"),
        "{r:#}"
    );
}

#[test]
fn a_missing_rounding_mode_keeps_the_decimal_places() {
    let mut input = golden_input("loans.instalment/rounding-half-up.yaml");
    input["rounding"] = json!({"decimal_places": 20000});
    let r = report("loans.instalment", &input);
    assert!(has(&r, "/rounding/mode", "missing"), "{r:#}");
    assert!(has(&r, "/rounding/decimal_places", "range"), "{r:#}");
}

// ------------------------------------------------- analyses: the variable of a refused value

#[test]
fn an_analysis_variable_is_not_checked_against_a_refused_value() {
    let base = golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml");
    let mut refused = base.clone();
    refused["wacc"] = json!(true);
    let mut misspelt = base.clone();
    let wacc = misspelt
        .as_object_mut()
        .unwrap()
        .shift_remove("wacc")
        .unwrap();
    misspelt["wac"] = wacc;
    for (base_input, expected) in [
        (&refused, vec![entry("/base_input/wacc", "type")]),
        (
            &misspelt,
            vec![
                entry("/base_input/wac", "unknown_field"),
                entry("/base_input/wacc", "missing"),
            ],
        ),
    ] {
        let sensitivity = json!({
            "method": "dcf.fcff", "base_input": base_input, "output": "/value_of_operations",
            "rows": {"variable": "/wacc", "values": ["0.08", "0.09"]}
        });
        let r = report("analysis.sensitivity", &sensitivity);
        assert_eq!(entries(&r), expected, "sensitivity: {r:#}");

        let tornado = json!({
            "method": "dcf.fcff", "base_input": base_input, "output": "/value_of_operations",
            "variables": [{"variable": "/wacc", "low": "0.08", "high": "0.1"}]
        });
        let r = report("analysis.tornado", &tornado);
        assert_eq!(entries(&r), expected, "tornado: {r:#}");

        let scenarios = json!({
            "method": "dcf.fcff", "base_input": base_input, "outputs": ["/value_of_operations"],
            "scenarios": [{"name": "low", "overrides": [{"pointer": "/wacc", "value": "0.08"}]}]
        });
        let r = report("analysis.scenarios", &scenarios);
        assert_eq!(entries(&r), expected, "scenarios: {r:#}");

        let goal_seek = json!({
            "method": "dcf.fcff", "base_input": base_input, "variable": "/wacc",
            "output": "/value_of_operations", "target": "1000", "bracket": ["0.05", "0.2"]
        });
        let r = report("analysis.goal_seek", &goal_seek);
        assert_eq!(entries(&r), expected, "goal seek: {r:#}");
    }
}

#[test]
fn an_analysis_variable_that_names_no_input_is_still_refused() {
    let base = golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml");
    let sensitivity = json!({
        "method": "dcf.fcff", "base_input": base, "output": "/value_of_operations",
        "rows": {"variable": "/nope", "values": ["0.08"]}
    });
    let r = report("analysis.sensitivity", &sensitivity);
    assert_eq!(r["code"], "FM-E015", "{r:#}");
    assert_eq!(entries(&r)[0].0, "/rows/variable", "{r:#}");
}

// ------------------------------------------------- every named field can be acted on

/// The pointers of the first entry of a report.
fn named(r: &Value) -> Vec<String> {
    r["pointers"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| p.as_str().unwrap().to_owned())
        .collect()
}

#[test]
fn neither_flow_of_a_gordon_is_missing() {
    let r = report("tv.gordon", &json!({"rate": "0.09", "growth": "0.025"}));
    assert_eq!(entries(&r), vec![entry("/next_flow", "missing")], "{r:#}");
    assert_eq!(named(&r), vec!["/next_flow", "/last_flow"], "{r:#}");
    // Both given: a conflict between two fields the input holds.
    let r = report(
        "tv.gordon",
        &json!({"rate": "0.09", "growth": "0.025", "next_flow": "1", "last_flow": "1"}),
    );
    assert_eq!(entries(&r), vec![entry("/next_flow", "conflict")], "{r:#}");
    assert_eq!(named(&r), vec!["/next_flow", "/last_flow"], "{r:#}");
}

#[test]
fn neither_flow_of_a_compared_gordon_is_missing() {
    let r = report(
        "tv.compare",
        &json!({"rate": "0.09", "techniques": [{"tv.gordon": {"growth": "0.025"}}]}),
    );
    assert!(
        has(&r, "/techniques/0/tv.gordon/next_flow", "missing"),
        "{r:#}"
    );
}

#[test]
fn a_debt_form_conflict_names_only_the_fields_given() {
    let r = report(
        "coc.cost_of_debt",
        &json!({"pre_tax_cost_of_debt": "0.05", "credit_spread": "0.02", "tax_rate": "0.24"}),
    );
    assert_eq!(
        entries(&r),
        vec![entry("/pre_tax_cost_of_debt", "conflict")],
        "{r:#}"
    );
    assert_eq!(
        named(&r),
        vec!["/pre_tax_cost_of_debt", "/credit_spread"],
        "{r:#}"
    );
    // A missing form names every field the user can give.
    let r = report(
        "coc.cost_of_debt",
        &json!({"credit_spread": "0.02", "tax_rate": "0.24"}),
    );
    assert_eq!(
        entries(&r),
        vec![entry("/pre_tax_cost_of_debt", "missing")],
        "{r:#}"
    );
    assert_eq!(
        named(&r),
        vec!["/pre_tax_cost_of_debt", "/risk_free_rate", "/credit_spread"],
        "{r:#}"
    );
}

#[test]
fn a_default_compounding_is_never_named() {
    let mut input = golden_input("tvm.xnpv/ecma-376-example.yaml");
    input["rate"] = json!("-1");
    let r = report("tvm.xnpv", &input);
    assert_eq!(entries(&r), vec![entry("/rate", "range")], "{r:#}");
    assert!(named(&r).is_empty(), "{r:#}");
    // Given, the compounding is a side of the rule.
    input["compounding"] = json!("annual");
    let r = report("tvm.xnpv", &input);
    assert_eq!(named(&r), vec!["/rate", "/compounding"], "{r:#}");
}

#[test]
fn a_default_compounding_is_never_named_for_the_guess() {
    let mut input = golden_input("tvm.xirr/ecma-376-example.yaml");
    input["guess"] = json!("-1");
    let r = report("tvm.xirr", &input);
    assert_eq!(entries(&r), vec![entry("/guess", "range")], "{r:#}");
    assert!(named(&r).is_empty(), "{r:#}");
    input["compounding"] = json!("annual");
    let r = report("tvm.xirr", &input);
    assert_eq!(named(&r), vec!["/guess", "/compounding"], "{r:#}");
}

#[test]
fn terms_without_inflation_require_the_inflation() {
    let r = report(
        "dcf.fcff",
        &golden_input("dcf.fcff/error-nominal-flows-real-rates.yaml"),
    );
    assert_eq!(r["code"], "FM-E024", "{r:#}");
    assert_eq!(r["reason"], "requires", "{r:#}");
    assert_eq!(
        named(&r),
        vec!["/cash_flow_terms", "/discount_rate_terms", "/inflation"],
        "{r:#}"
    );
}
