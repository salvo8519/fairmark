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

// ------------------------------------------------------------- every unknown field

/// Maintainer instruction of 2026-11-09 (item A): after the first unknown field, every other
/// unknown key of the input is reported in one pass, with the same message serde would give.
#[test]
fn every_unknown_field_is_reported() {
    let text = |a: &str, b: &str| {
        format!(r#"{{"rate": "0.1", "{a}": 1, "cash_flows": [-100, 60], "{b}": 2}}"#)
    };
    let r = report_text("tvm.npv", &text("typo", "other"));
    let pointers: Vec<&str> = r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["pointer"].as_str().unwrap())
        .collect();
    assert_eq!(pointers, ["/typo", "/other"]);
    for e in r["errors"].as_array().unwrap() {
        assert_eq!(e["reason"], "unknown_field");
        assert_eq!(e["expected"], json!(["rate", "cash_flows"]));
    }
    // The message of a key found by the walk is the one serde gives when it meets it first.
    let swapped = report_text("tvm.npv", &text("other", "typo"));
    assert_eq!(r["errors"][1]["message"], swapped["message"]);
    assert_eq!(swapped["errors"][1]["message"], r["message"]);
}

#[test]
fn unknown_fields_are_found_at_every_depth() {
    let mut input = dcf();
    let wacc = input.as_object_mut().unwrap().shift_remove("wacc").unwrap();
    input["wac"] = wacc;
    input["stages"][0]["flow"] = json!([1]);
    input["terminal"]["tv.gordon"]["growht"] = json!("0.02");
    input["net_financial_position"]["debt"] = json!(0);
    let r = report("dcf.fcff", &input);
    let found: Vec<(&str, &str)> = r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| (e["pointer"].as_str().unwrap(), e["fix"].as_str().unwrap()))
        .collect();
    assert_eq!(
        found,
        [
            ("/stages/0/flow", "rename `flow` to `flows`"),
            ("/terminal/tv.gordon/growht", "rename `growht` to `growth`"),
            (
                "/net_financial_position/debt",
                "remove `debt`, or rename it to one of the fields in `expected`"
            ),
            ("/wac", "rename `wac` to `wacc`"),
            // The method's own check (round 5, audit 1): no rate is given under its name. The
            // misspelt key stands for it (round 6): the same fix, at both.
            ("/wacc", "rename `wac` to `wacc`"),
        ]
    );
    let wacc = &r["errors"][4];
    assert_eq!(wacc["reason"], "missing");
    assert_eq!(wacc["pointers"], json!(["/wacc", "/wac"]));
}

#[test]
fn unknown_fields_inside_tagged_choices() {
    let mut input = golden_input("statements.reorganise/adjustment-non-recurring.yaml");
    input["adjustments"][0]["from_aggregate"] = json!("x");
    input["adjustments"][1]["labl"] = json!("x");
    input["years"][0]["statutory"]["balance_sheet"]["cashh"] = json!("1");
    let r = report("statements.reorganise", &input);
    let pointers: Vec<&str> = r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["pointer"].as_str().unwrap())
        .collect();
    assert_eq!(
        pointers,
        [
            "/years/0/statutory/balance_sheet/cashh",
            "/adjustments/0/from_aggregate",
            "/adjustments/1/labl",
        ]
    );
    let mut compare = golden_input("tv.compare/seven-techniques.yaml");
    compare["techniques"][0]["tv.gordon"]["grow"] = json!("0.02");
    compare["techniques"][1]["tv.value_driver"]["roic"] = json!("0.1");
    let r = report("tv.compare", &compare);
    assert_eq!(r["errors"].as_array().unwrap().len(), 2, "{r:#}");
    assert_eq!(
        r["errors"][1]["pointer"],
        "/techniques/1/tv.value_driver/roic"
    );
}

/// A wrong type stops `serde` at the first error; the walk still reports every unknown key.
#[test]
fn a_type_error_comes_with_every_unknown_key() {
    let r = report_text("tvm.npv", r#"{"rate": "x", "typo": 1, "cash_flows": [1]}"#);
    assert_eq!(r["reason"], "type");
    assert_eq!(r["errors"][1]["pointer"], "/typo");
    assert_eq!(r["errors"].as_array().unwrap().len(), 2);
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

// ------------------------------------------------------------- analysis

fn goal_seek() -> Value {
    json!({
        "method": "tvm.npv",
        "base_input": {"rate": "0.1", "cash_flows": [-100, 60, 60]},
        "variable": "/rate",
        "output": "/npv",
        "target": "0",
        "bracket": ["0.01", "0.5"]
    })
}

/// Review of #279, item 1: an error of the inner method is located from the root of the
/// analysis input, under `/base_input`; its message keeps the inner pointer.
#[test]
fn inner_errors_are_located_under_base_input() {
    let mut input = goal_seek();
    input["base_input"]["bogus"] = json!(1);
    let r = report("analysis.goal_seek", &input);
    assert_eq!(r["pointer"], "/base_input/bogus");
    assert!(r["message"].as_str().unwrap().contains("`/bogus`"));
    assert_eq!(r["errors"][0]["pointer"], "/base_input/bogus");

    let mut input = goal_seek();
    input["base_input"]["cash_flows"] = json!([]);
    let r = report("analysis.goal_seek", &input);
    assert_eq!(r["code"], "FM-E002");
    assert_eq!(r["pointer"], "/base_input/cash_flows");
}

#[test]
fn analysis_fields_are_named() {
    let mut input = goal_seek();
    input["variable"] = json!("/nope");
    assert_eq!(report("analysis.goal_seek", &input)["pointer"], "/variable");
    let mut input = goal_seek();
    input["output"] = json!("npv");
    assert_eq!(report("analysis.goal_seek", &input)["pointer"], "/output");
    let mut input = goal_seek();
    input["method"] = json!("tvm.nope");
    let r = report("analysis.goal_seek", &input);
    assert_eq!(
        (r["code"].as_str(), r["pointer"].as_str()),
        (Some("FM-E007"), Some("/method"))
    );
    input["method"] = json!("analysis.tornado");
    let r = report("analysis.goal_seek", &input);
    assert_eq!(
        (r["code"].as_str(), r["pointer"].as_str()),
        (Some("FM-E017"), Some("/method"))
    );
}

#[test]
fn tornado_and_scenarios_name_their_entries() {
    let tornado = json!({
        "method": "tvm.npv",
        "base_input": {"rate": "0.1", "cash_flows": [-100, 60, 60]},
        "output": "/npv",
        "variables": [{"variable": "/rate", "low": "0.2", "high": "0.1"}]
    });
    let r = report("analysis.tornado", &tornado);
    assert_eq!(r["pointer"], "/variables/0/low");
    assert_eq!(
        r["pointers"],
        json!(["/variables/0/low", "/variables/0/high"])
    );
    let mut dup = tornado.clone();
    dup["variables"] = json!([
        {"variable": "/rate", "low": "0.05", "high": "0.1"},
        {"variable": "/rate", "low": "0.05", "high": "0.1"}
    ]);
    let r = report("analysis.tornado", &dup);
    assert_eq!(r["pointer"], "/variables/1/variable");
    assert_eq!(r["reason"], "duplicate");

    let scenarios = |names: [&str; 2]| {
        json!({
            "method": "tvm.npv",
            "base_input": {"rate": "0.1", "cash_flows": [-100, 60, 60]},
            "outputs": ["/npv"],
            "scenarios": [
                {"name": names[0], "overrides": [{"pointer": "/rate", "value": "0.2"}]},
                {"name": names[1], "overrides": [{"pointer": "/rate", "value": "0.3"}]}
            ]
        })
    };
    let r = report("analysis.scenarios", &scenarios(["up", " "]));
    assert_eq!(r["pointer"], "/scenarios/1/name");
    assert_eq!(r["reason"], "blank");
    let r = report("analysis.scenarios", &scenarios(["up", "up"]));
    assert_eq!(r["pointer"], "/scenarios/1/name");
    assert_eq!(r["reason"], "duplicate");
}

#[test]
fn compare_names_the_technique() {
    let r = report("tv.compare", &golden_input("tv.compare/mixed-basis.yaml"));
    assert_eq!(r["code"], "FM-E019");
    assert_eq!(r["pointer"], "/techniques/1/tv.exit_multiple/metric");
}

#[test]
fn flow_before_valuation_date_names_both_dates() {
    let r = report(
        "tvm.xnpv",
        &json!({
            "rate": "0.1",
            "valuation_date": "2024-06-30",
            "cash_flows": [
                {"date": "2024-06-30", "amount": "-100"},
                {"date": "2024-01-31", "amount": "110"}
            ]
        }),
    );
    assert_eq!(r["code"], "FM-E010");
    assert_eq!(r["pointer"], "/cash_flows/1/date");
    assert_eq!(
        r["pointers"],
        json!(["/cash_flows/1/date", "/valuation_date"])
    );
}

// ------------------------------------------------------------- computation errors

/// Maintainer instruction of 2026-11-09 (item C): a computation error (here an overflow in the
/// build-up of the free cash flows) never sits in `errors` next to input errors. With an input
/// error, the input errors are reported; without, the computation error alone.
#[test]
fn computation_errors_are_never_listed_with_input_errors() {
    let max = "79228162514264337593543950335";
    let mut input = golden_input("dcf.fcff/stage-with-build-up-lines-tax-amounts.yaml");
    input["stages"][0]["lines"][0]["ebit"] = json!(max);
    input["stages"][0]["lines"][0]["depreciation_and_amortisation"] = json!(max);
    let r = report("dcf.fcff", &input);
    assert_eq!(r["code"], "FM-E008", "{r:#}");
    assert_eq!(r["errors"].as_array().unwrap().len(), 1);

    input["wacc"] = json!("-2");
    let r = report("dcf.fcff", &input);
    assert_eq!(r["code"], "FM-E003", "{r:#}");
    let codes: Vec<&str> = r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, ["FM-E003"]);
}

// ------------------------------------------------------------- every method collects

/// The pointers of every error of the report.
fn pointers_of(r: &Value) -> Vec<String> {
    r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["pointer"].as_str().unwrap_or("null").to_owned())
        .collect()
}

/// Maintainer instruction of 2026-11-09 (item B): dcf.free_cash_flows, tv.*, tvm.* and coc.*
/// report every input error of their validation, in the order of their checks.
#[test]
fn free_cash_flows_reports_every_input_error() {
    let mut input = golden_input("dcf.free_cash_flows/amounts-form-with-adjustments.yaml");
    input["periods"][0]["capital_expenditure"] = json!("-1");
    input["periods"][1]["depreciation_and_amortisation"] = json!("-2");
    input["periods"][1]["other_adjustments"][0]["label"] = json!(" ");
    input["interest_tax_rate"] = json!("1.5");
    let r = report("dcf.free_cash_flows", &input);
    assert_eq!(
        pointers_of(&r),
        [
            "/periods/0/capital_expenditure",
            "/periods/1/depreciation_and_amortisation",
            "/periods/1/other_adjustments/0/label",
            "/interest_tax_rate",
        ]
    );
}

#[test]
fn tv_methods_report_every_input_error() {
    let r = report(
        "tv.gordon",
        &json!({"rate": "0.1", "growth": "-2", "next_flow": "1", "last_flow": "1"}),
    );
    assert_eq!(pointers_of(&r), ["/growth", "/next_flow"]);
    let r = report(
        "tv.exit_multiple",
        &json!({"multiple": "0", "metric": "ebitda", "metric_value": "0"}),
    );
    assert_eq!(pointers_of(&r), ["/multiple", "/metric_value"]);
    let r = report(
        "tv.liquidation",
        &json!({"basis": "enterprise", "tax_on_gains": "-1",
                "assets": [{"label": " ", "value": "-1"}], "liabilities": [],
                "liquidation_costs": []}),
    );
    assert_eq!(
        pointers_of(&r),
        ["/tax_on_gains", "/assets/0/label", "/assets/0/value"]
    );
    let mut compare = golden_input("tv.compare/seven-techniques.yaml");
    compare["techniques"][0]["tv.gordon"]["growth"] = json!("-2");
    compare["techniques"][1]["tv.value_driver"]["ronic"] = json!("0");
    let r = report("tv.compare", &compare);
    assert_eq!(
        pointers_of(&r),
        [
            "/techniques/0/tv.gordon/growth",
            "/techniques/1/tv.value_driver/ronic"
        ]
    );
    let mut dcf = dcf();
    dcf["terminal"] =
        json!({"tv.value_driver": {"nopat_next": "100", "growth": "-2", "ronic": "0"}});
    let r = report("dcf.fcff", &dcf);
    assert_eq!(
        pointers_of(&r),
        [
            "/terminal/tv.value_driver/growth",
            "/terminal/tv.value_driver/ronic"
        ]
    );
}

#[test]
fn tvm_methods_report_every_input_error() {
    let r = report("tvm.npv", &json!({"rate": "-1", "cash_flows": []}));
    assert_eq!(pointers_of(&r), ["/cash_flows", "/rate"]);
    let r = report("tvm.irr", &json!({"cash_flows": [1, 2], "guess": "-1"}));
    assert_eq!(pointers_of(&r), ["/cash_flows", "/guess"]);
    let r = report(
        "tvm.mirr",
        &json!({"cash_flows": [1], "finance_rate": "-1", "reinvest_rate": "-2"}),
    );
    assert_eq!(
        pointers_of(&r),
        ["/cash_flows", "/finance_rate", "/reinvest_rate"]
    );
    let r = report(
        "tvm.present_value",
        &json!({"cash_flows": ["1", "1"], "period_rates": ["-1", "-2"], "timing": "end_of_period"}),
    );
    assert_eq!(pointers_of(&r), ["/period_rates/0", "/period_rates/1"]);
    let r = report(
        "tvm.xnpv",
        &json!({"rate": "-1", "valuation_date": "2024-06-30", "cash_flows": [
            {"date": "2024-01-31", "amount": "-100"},
            {"date": "2024-02-29", "amount": "110"}
        ]}),
    );
    assert_eq!(
        pointers_of(&r),
        ["/cash_flows/0/date", "/cash_flows/1/date", "/rate"]
    );
    let r = report(
        "tvm.annuity",
        &json!({"payment": "1", "rate": "-1", "periods": 0, "growth": "-1", "timing": "ordinary"}),
    );
    assert_eq!(pointers_of(&r), ["/periods", "/rate", "/growth"]);
    let r = report(
        "tvm.perpetuity",
        &json!({"payment": "1", "rate": "-1", "growth": "-1", "timing": "ordinary"}),
    );
    assert_eq!(pointers_of(&r), ["/rate", "/growth"]);
    let r = report(
        "tvm.future_value",
        &json!({"rate": "-1", "periods": 0, "timing": "ordinary"}),
    );
    assert_eq!(pointers_of(&r), ["/present_value", "/periods", "/rate"]);
}

#[test]
fn coc_methods_report_every_input_error() {
    let r = report(
        "coc.cost_of_equity",
        &json!({"risk_free_rate": "0.03", "equity_risk_premium": "0.05",
                "country_risk_premium": "-0.01", "country_exposure": "-1",
                "specific_risk_premium": "0.01"}),
    );
    assert_eq!(
        pointers_of(&r),
        [
            "/levered_beta",
            "/country_risk_premium",
            "/country_exposure",
            "/specific_risk_justification"
        ]
    );
    let r = report(
        "coc.wacc",
        &json!({"cost_of_equity": "0.1", "pre_tax_cost_of_debt": "0.05",
                "tax_rate": "1", "debt_weight": "1"}),
    );
    assert_eq!(pointers_of(&r), ["/tax_rate", "/debt_weight"]);
    let r = report(
        "coc.cost_of_debt",
        &json!({"risk_free_rate": "0.03", "tax_rate": "-0.1"}),
    );
    assert_eq!(pointers_of(&r), ["/tax_rate", "/pre_tax_cost_of_debt"]);
    let r = report(
        "coc.beta_relever",
        &json!({"beta": "1", "direction": "unlever", "debt_to_equity": "-1",
                "tax_rate": "1", "formula": "hamada"}),
    );
    assert_eq!(pointers_of(&r), ["/debt_to_equity", "/tax_rate"]);
    let mut peers = golden_input("coc.beta_peers/five-peers-median.yaml");
    peers["target_tax_rate"] = json!("1");
    peers["peers"][1]["debt_to_equity"] = json!("-1");
    peers["peers"][2]["tax_rate"] = json!("2");
    let r = report("coc.beta_peers", &peers);
    assert_eq!(
        pointers_of(&r),
        [
            "/target_tax_rate",
            "/peers/1/debt_to_equity",
            "/peers/2/tax_rate"
        ]
    );
}

// ------------------------------------------------------------- the rule of `expected`

/// ADR 0014 G5 as amended by the review of #279: `expected` lists literal values when the
/// reason is `enum` or `unknown_field`, and descriptions of the accepted form otherwise.
#[test]
fn expected_lists_literal_values_only_for_enum_and_unknown_field() {
    let r = report("dcf.fcff", &{
        let mut i = dcf();
        i["timing"] = json!("end");
        i
    });
    assert_eq!(r["expected"], json!(["end_of_period", "mid_period"]));
    let r = report(
        "loans.instalment",
        &golden_input("loans.instalment/error-principal-zero.yaml"),
    );
    assert_eq!(r["reason"], "range");
    assert_eq!(r["expected"], json!(["an amount greater than 0"]));
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
        // Inside an analysis, the pointer of an inner error is under `/base_input`, while the
        // message keeps the inner pointer.
        let analysis = method.starts_with("analysis.");
        // `FM-E027` names its inputs in `missing_any_of`, not in `pointer`.
        let below_minimum = r["code"] == "FM-E027";
        if names_a_pointer && !below_minimum && r["pointer"].is_null() {
            failures.push(format!("{}: no pointer for: {message}", file.display()));
        }
        if let Some(p) = r["pointer"].as_str() {
            let inner = p
                .strip_prefix("/base_input")
                .filter(|_| analysis)
                .unwrap_or(p);
            if names_a_pointer && !message.contains(inner) {
                failures.push(format!("{}: pointer {p} not in: {message}", file.display()));
            }
        }
        if r["code"] == "FM-E001" && r["reason"].is_null() {
            failures.push(format!(
                "{}: FM-E001 without reason: {message}",
                file.display()
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
