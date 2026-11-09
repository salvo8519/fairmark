//! The structured error report (ADR 0014 G5, issue #258): the field to correct as a JSON
//! pointer, the sub-kind (`reason`), the accepted values (`expected`), the fix, the alternative
//! minimum sets of `FM-E027` (`missing_any_of`), and every input error a validation finds
//! (`errors`). Codes and messages are those reported before the report gained these fields.

use std::path::{Path, PathBuf};

use fairmark_core::run;
use serde_json::{json, Value};

/// The JSON report of the error `method` must raise on `input`.
fn report(method: &str, input: &Value) -> Value {
    report_text(method, &input.to_string())
}

fn report_text(method: &str, input: &str) -> Value {
    let err = run(method, input).expect_err("the input must be refused");
    let report = serde_json::to_value(err.to_report()).unwrap();
    assert_eq!(report["code"], err.code());
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

fn npv() -> Value {
    json!({"rate": "0.1", "cash_flows": [-100, 60, 60]})
}

fn dcf() -> Value {
    json!({
        "stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": "0.09",
        "timing": "end_of_period",
        "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
        "net_financial_position": {"gross_debt": 0, "cash": 0}
    })
}

/// The entries of `errors` without their own key, compared with the top level.
fn top_level_is_the_first_entry(report: &Value) {
    let errors = report["errors"].as_array().expect("errors is a list");
    assert!(!errors.is_empty(), "errors lists at least the error itself");
    let mut top = report.as_object().unwrap().clone();
    top.remove("errors");
    assert_eq!(
        Value::Object(top),
        errors[0],
        "the first entry is the top level"
    );
    for e in errors {
        assert!(
            e.get("errors").is_none(),
            "an entry has no errors of its own"
        );
    }
}

// ------------------------------------------------------------- the shape of every report

#[test]
fn every_report_has_the_seven_fields_and_lists_itself() {
    let r = report("tvm.npv", &json!({"rate": "0.1"}));
    for key in [
        "code", "message", "pointer", "pointers", "reason", "expected", "fix",
    ] {
        assert!(r.get(key).is_some(), "{key} missing in {r}");
    }
    assert!(r.get("missing_any_of").is_none(), "only FM-E027 lists sets");
    assert_eq!(r["errors"].as_array().unwrap().len(), 1);
    top_level_is_the_first_entry(&r);
}

// ------------------------------------------------------------- errors raised while reading

#[test]
fn missing_field_is_reason_missing_with_its_pointer() {
    let r = report("tvm.npv", &json!({"rate": "0.1"}));
    assert_eq!(r["code"], "FM-E001");
    assert_eq!(r["pointer"], "/cash_flows");
    assert_eq!(r["reason"], "missing");
    assert_eq!(r["fix"], "add the required field `cash_flows`");
}

#[test]
fn unknown_field_is_reason_unknown_field() {
    let mut input = npv();
    input["typo"] = json!(1);
    let r = report("tvm.npv", &input);
    assert_eq!(r["pointer"], "/typo");
    assert_eq!(r["reason"], "unknown_field");
    assert_eq!(r["expected"], json!(["rate", "cash_flows"]));
    assert_eq!(
        r["fix"],
        "remove `typo`, or rename it to one of the fields in `expected`"
    );
}

#[test]
fn duplicate_field_is_reason_duplicate() {
    let r = report_text(
        "tvm.npv",
        r#"{"rate": "0.1", "rate": "0.2", "cash_flows": [1]}"#,
    );
    assert_eq!(r["pointer"], "/rate");
    assert_eq!(r["reason"], "duplicate");
    assert_eq!(r["fix"], "keep one `rate`");
}

#[test]
fn wrong_type_is_reason_type_with_the_accepted_form() {
    let mut input = npv();
    input["cash_flows"][1] = json!({"amount": 60});
    let r = report("tvm.npv", &input);
    assert_eq!(r["pointer"], "/cash_flows/1");
    assert_eq!(r["reason"], "type");
    assert_eq!(
        r["expected"],
        json!(["a decimal number (a JSON number or a string such as \"0.10\")"])
    );
    assert_eq!(
        r["fix"],
        "give a decimal number (a JSON number or a string such as \"0.10\")"
    );
}

#[test]
fn unknown_enumeration_value_is_reason_enum_with_the_values() {
    let mut input = dcf();
    input["timing"] = json!("end");
    let r = report("dcf.fcff", &input);
    assert_eq!(r["pointer"], "/timing");
    assert_eq!(r["reason"], "enum");
    assert_eq!(r["expected"], json!(["end_of_period", "mid_period"]));
    assert_eq!(r["fix"], "use one of the values in `expected`");
}

#[test]
fn malformed_json_has_no_pointer_and_reason_syntax() {
    let r = report_text("tvm.npv", r#"{"rate": "0.1", "#);
    assert_eq!(r["code"], "FM-E001");
    assert_eq!(r["pointer"], Value::Null);
    assert_eq!(r["reason"], "syntax");
    assert!(r["message"].as_str().unwrap().contains("line 1 column"));
    assert_eq!(r["fix"], "send one well-formed JSON object");
}

#[test]
fn invalid_date_is_fm_e009_reason_type() {
    let r = report(
        "dates.year_fraction",
        &json!({"start": "2024-02-30", "end": "2024-12-31", "day_count": "act_365_fixed"}),
    );
    assert_eq!(r["code"], "FM-E009");
    assert_eq!(r["pointer"], "/start");
    assert_eq!(r["reason"], "type");
    assert_eq!(
        r["expected"],
        json!(["an ISO 8601 calendar date, YYYY-MM-DD"])
    );
}

/// ADR 0013: `serde` stops at the first error it reads, so a text that cannot be read reports
/// one error; validation of an input that was read reports all of them.
#[test]
fn reading_stops_at_the_first_error() {
    let r = report("tvm.npv", &json!({"typo": 1, "other": 2}));
    assert_eq!(r["errors"].as_array().unwrap().len(), 1);
}

// ------------------------------------------------------------- statements.reorganise

#[test]
fn statements_sign_error_names_the_field() {
    let r = report(
        "statements.reorganise",
        &golden_input("statements.reorganise/error-negative-unsigned-amount.yaml"),
    );
    assert_eq!(r["code"], "FM-E001");
    assert_eq!(r["pointer"], "/years/0/statutory/balance_sheet/cash");
    assert_eq!(r["reason"], "sign");
    assert_eq!(r["expected"], json!(["zero or more"]));
    assert_eq!(r["fix"], "give the amount without sign");
    top_level_is_the_first_entry(&r);
}

#[test]
fn statements_validation_reports_every_error() {
    let mut input = golden_input("statements.reorganise/error-negative-unsigned-amount.yaml");
    let bs = &mut input["years"][0]["statutory"]["balance_sheet"];
    bs["inventories"] = json!("-5");
    bs["equity"]["negative_reserve_for_own_shares"] = json!("3");
    input["adjustments"] = json!([{
        "kind": "non_recurring",
        "label": " ",
        "reason": "",
        "aggregate": "other_revenue_and_income",
        "amounts": ["10"]
    }]);
    let r = report("statements.reorganise", &input);
    let at = |i: usize| r["errors"][i]["pointer"].as_str().unwrap().to_owned();
    let errors = r["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 5, "{r:#}");
    let base = "/years/0/statutory/balance_sheet";
    assert_eq!(at(0), format!("{base}/inventories"));
    assert_eq!(at(1), format!("{base}/cash"));
    assert_eq!(
        at(2),
        format!("{base}/equity/negative_reserve_for_own_shares")
    );
    assert_eq!(at(3), "/adjustments/0/label");
    assert_eq!(at(4), "/adjustments/0/reason");
    assert_eq!(r["errors"][2]["reason"], "sign");
    assert_eq!(r["errors"][3]["reason"], "blank");
    for e in errors {
        assert_eq!(e["code"], "FM-E001");
    }
    top_level_is_the_first_entry(&r);
}

#[test]
fn statements_reconciliation_names_both_sides() {
    let r = report(
        "statements.reorganise",
        &golden_input("statements.reorganise/error-lease-liability-current.yaml"),
    );
    assert_eq!(r["code"], "FM-E026");
    assert_eq!(r["reason"], "reconcile");
    assert_eq!(
        r["pointers"],
        json!([
            "/adjustments/0/amounts/0/liability_current",
            "/adjustments/0/amounts/0/liability"
        ])
    );
    assert_eq!(r["pointer"], "/adjustments/0/amounts/0/liability_current");
}

/// A reconciliation of the computation stops it: one error.
#[test]
fn a_computation_error_is_reported_alone() {
    let r = report(
        "statements.reorganise",
        &golden_input("statements.reorganise/error-adjustment-larger-than-source.yaml"),
    );
    assert_eq!(r["code"], "FM-E026");
    assert_eq!(r["errors"].as_array().unwrap().len(), 1);
    assert!(!r["pointers"].as_array().unwrap().is_empty());
}

// ------------------------------------------------------------- ratios

#[test]
fn below_minimum_lists_the_alternative_sets() {
    let r = report(
        "ratios.liquidity",
        &golden_input("ratios.liquidity/below-minimum.yaml"),
    );
    assert_eq!(r["code"], "FM-E027");
    assert_eq!(r["reason"], "missing");
    assert_eq!(
        r["missing_any_of"],
        json!([["/closing/cash"], ["/closing/current_assets"]])
    );
    assert_eq!(
        r["fix"],
        "give every input of at least one of the sets in `missing_any_of`"
    );
}

#[test]
fn ratios_blank_definitions_are_all_reported() {
    let mut input = golden_input("ratios.debt_service/level-cfads.yaml");
    input["ebitda_definition"] = json!(" ");
    input["interest_definition"] = json!("");
    let r = report("ratios.debt_service", &input);
    assert_eq!(r["errors"].as_array().unwrap().len(), 2, "{r:#}");
    assert_eq!(r["errors"][0]["pointer"], "/ebitda_definition");
    assert_eq!(r["errors"][1]["pointer"], "/interest_definition");
    assert_eq!(r["errors"][1]["reason"], "blank");
}

// ------------------------------------------------------------- dcf

#[test]
fn dcf_validation_reports_the_bridge_and_the_stages() {
    let mut input = dcf();
    input["non_operating_assets"] = json!([{"label": " ", "value": "-5"}]);
    input["debt_like_items"] =
        json!([{"label": "Pensions", "value": "-1", "measurement_basis": "book_value"}]);
    input["stages"][0]["periods"] = json!(0);
    let r = report("dcf.fcff", &input);
    let pointers: Vec<&str> = r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["pointer"].as_str().unwrap())
        .collect();
    assert_eq!(
        pointers,
        [
            "/non_operating_assets/0/value",
            "/non_operating_assets/0/label",
            "/debt_like_items/0/value",
            "/stages/0/periods",
        ]
    );
    assert_eq!(r["errors"][0]["reason"], "sign");
    assert_eq!(r["errors"][1]["reason"], "blank");
    assert_eq!(r["errors"][3]["reason"], "range");
    top_level_is_the_first_entry(&r);
}

#[test]
fn dcf_conflicting_rates_name_both_fields() {
    let mut input = dcf();
    input["wacc_per_period"] = json!(["0.09", "0.09"]);
    let r = report("dcf.fcff", &input);
    assert_eq!(r["code"], "FM-E001");
    assert_eq!(r["reason"], "conflict");
    assert_eq!(r["pointers"], json!(["/wacc", "/wacc_per_period"]));
}

#[test]
fn dcf_rate_out_of_range_names_the_rate() {
    let mut input = dcf();
    input["wacc"] = json!("-1");
    let r = report("dcf.fcff", &input);
    assert_eq!(r["code"], "FM-E003");
    assert_eq!(r["pointer"], "/wacc");
    assert_eq!(r["reason"], "range");
}

// ------------------------------------------------------------- loans

#[test]
fn loans_validation_reports_every_error() {
    let input = json!({
        "principal": "0",
        "number_of_instalments": 0,
        "timing": "ordinary",
        "rounding": {"decimal_places": 9, "mode": "half_up"},
        "periodic_rate": "0.01",
        "rate": "0.12"
    });
    let r = report("loans.instalment", &input);
    let pointers: Vec<&str> = r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["pointer"].as_str().unwrap())
        .collect();
    assert_eq!(
        pointers,
        [
            "/principal",
            "/number_of_instalments",
            "/rounding/decimal_places",
            "/periodic_rate",
        ]
    );
    assert_eq!(r["errors"][3]["reason"], "conflict");
    assert_eq!(
        r["errors"][3]["pointers"],
        json!(["/periodic_rate", "/rate"])
    );
    top_level_is_the_first_entry(&r);
}

#[test]
fn rounding_without_mode_lists_the_modes() {
    let input = golden_input("loans.instalment/error-rounding-without-mode.yaml");
    let r = report("loans.instalment", &input);
    assert_eq!(r["pointer"], "/rounding/mode");
    assert_eq!(r["reason"], "missing");
    assert_eq!(r["expected"], json!(["half_up", "half_even"]));
}

// ------------------------------------------------------------- every golden error case

fn golden_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            golden_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "yaml") {
            out.push(path);
        }
    }
}

/// Every golden error case: the report lists itself first, and a message that names a field
/// of the input by its JSON pointer reports that pointer, so that no client parses the text.
#[test]
fn every_golden_error_names_its_pointer_in_the_report() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut files = Vec::new();
    golden_files(&root, &mut files);
    files.sort();
    let mut failures = Vec::new();
    for file in files {
        let case: Value = serde_norway::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        if case.get("expect_error").is_none() {
            continue;
        }
        let method = case["method"].as_str().unwrap();
        let r = report(method, &case["input"]);
        top_level_is_the_first_entry(&r);
        let message = r["message"].as_str().unwrap();
        let names_a_pointer = message.contains("`/");
        let analysis = method.starts_with("analysis.");
        // `FM-E027` names its inputs in `missing_any_of`, not in `pointer`.
        let below_minimum = r["code"] == "FM-E027";
        if names_a_pointer && !analysis && !below_minimum && r["pointer"].is_null() {
            failures.push(format!("{}: no pointer for: {message}", file.display()));
        }
        if let Some(p) = r["pointer"].as_str() {
            if names_a_pointer && !message.contains(p) {
                failures.push(format!("{}: pointer {p} not in: {message}", file.display()));
            }
        }
        if r["code"] == "FM-E001" && r["reason"].is_null() && !analysis {
            failures.push(format!(
                "{}: FM-E001 without reason: {message}",
                file.display()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
