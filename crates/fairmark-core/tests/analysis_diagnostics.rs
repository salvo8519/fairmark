//! Diagnostics of an evaluated method inside an analysis (SPEC-0012 R14, decision 9; AC12): the
//! warnings of the base evaluation once, input pointers under `/base_input`; for goal seek, the
//! diagnostics of the solved evaluation, result pointers under `/value/inner_value`.

use serde_json::{json, Value};

fn run(method: &str, input: &Value) -> Value {
    fairmark_core::run(method, &input.to_string()).unwrap()
}

/// A dcf.fcff input without a net financial position (FM-W012).
fn dcf_without_nfp() -> Value {
    json!({"stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}],
           "cash_flow_terms": "nominal", "discount_rate_terms": "nominal", "wacc": "0.09",
           "timing": "end_of_period",
           "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}})
}

fn warnings(out: &Value) -> &Vec<Value> {
    out["diagnostics"]["warnings"]
        .as_array()
        .unwrap_or_else(|| panic!("no diagnostics.warnings in {out}"))
}

/// AC12: a table of 5 × 1 evaluations reports the warning of its base evaluation once, at the
/// pointer of the field in the analysis input, echoed once.
#[test]
#[ignore = "SPEC-0012: enabled by T15, analyses v2"]
fn a_table_reports_the_warnings_of_its_base_evaluation_once() {
    let out = run(
        "analysis.sensitivity",
        &json!({"method": "dcf.fcff", "base_input": dcf_without_nfp(),
                "output_pointer": "/enterprise_value",
                "rows": {"variable_pointer": "/wacc",
                         "values": ["0.07", "0.08", "0.09", "0.10", "0.11"]}}),
    );
    let fm_w012: Vec<&Value> = warnings(&out)
        .iter()
        .filter(|w| w["code"] == "FM-W012")
        .collect();
    assert_eq!(fm_w012.len(), 1, "{fm_w012:?}");
    assert_eq!(fm_w012[0]["pointer"], "/base_input/net_financial_position");
    // The preview warning of the analysis itself (FM-W016) is reported too.
    assert!(warnings(&out)
        .iter()
        .any(|w| w["code"] == "FM-W016" && w["pointer"] == "/method"));
    let echoes = out["explain"]["conventions"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c.as_str().unwrap().starts_with("warning FM-W012: "))
        .count();
    assert_eq!(echoes, 1);
    // The analysis input holds the field the pointer names, or its default does: the base
    // input names no net financial position, which the method filled in.
    assert_eq!(out["diagnostics"]["not_computed"], json!([]));
}

/// AC12: goal seek on a ratio method reports the outputs the solved evaluation did not compute,
/// under the inner value, with the missing inputs under `/base_input`.
#[test]
#[ignore = "SPEC-0012: enabled by T15, analyses v2"]
fn goal_seek_reports_the_diagnostics_of_the_solved_evaluation() {
    let out = run(
        "analysis.goal_seek",
        &json!({"method": "ratios.debt_service",
                "base_input": {"balances": "closing",
                               "interest_definition": "gross interest expense",
                               "income_statement": {"ebit": "500", "interest": "100"}},
                "variable_pointer": "/income_statement/ebit",
                "output_pointer": "/interest_cover_ebit/value",
                "target": "3", "bracket": ["100", "1000"]}),
    );
    let entries = out["diagnostics"]["not_computed"].as_array().unwrap();
    let nfp_to_ebitda = entries
        .iter()
        .find(|e| e["pointer"] == "/value/inner_value/nfp_to_ebitda")
        .unwrap_or_else(|| panic!("{entries:?}"));
    assert_eq!(nfp_to_ebitda["reason_code"], "missing_input");
    assert_eq!(
        nfp_to_ebitda["missing_any_of"],
        json!([[
            "/base_input/closing/net_financial_position/cash",
            "/base_input/closing/net_financial_position/gross_debt",
            "/base_input/ebitda_definition",
            "/base_input/income_statement/ebitda"
        ]])
    );
    for e in entries {
        let p = e["pointer"].as_str().unwrap();
        assert!(p.starts_with("/value/inner_value/"), "{p}");
        assert!(out.pointer(p).is_some(), "{p} does not resolve");
    }
    assert!(out["value"]["inner_value"].get("not_computed").is_none());
}

/// R14: goal seek reports the warnings of the solved evaluation once, input pointers under
/// `/base_input`, result pointers under `/value/inner_value`.
#[test]
#[ignore = "SPEC-0012: enabled by T15, analyses v2"]
fn goal_seek_reports_the_warnings_of_the_solved_evaluation() {
    let out = run(
        "analysis.goal_seek",
        &json!({"method": "dcf.fcff", "base_input": dcf_without_nfp(),
                "variable_pointer": "/wacc", "output_pointer": "/enterprise_value",
                "target": "1500", "bracket": ["0.05", "0.2"]}),
    );
    let fm_w012: Vec<&Value> = warnings(&out)
        .iter()
        .filter(|w| w["code"] == "FM-W012")
        .collect();
    assert_eq!(fm_w012.len(), 1, "{fm_w012:?}");
    assert_eq!(fm_w012[0]["pointer"], "/base_input/net_financial_position");
}
