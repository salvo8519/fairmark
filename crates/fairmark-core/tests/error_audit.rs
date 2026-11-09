//! Round 3 of the review of #279 and the independent audit of the error report: every input
//! error of a request in one report, a pointer and a sub-kind for every input condition, and no
//! input that is silently read in another way than written (duplicate keys, positional arrays).

use std::path::Path;

use fairmark_core::run;
use serde_json::{json, Value};

fn report_text(method: &str, input: &str) -> Value {
    let err = run(method, input).expect_err("the input must be refused");
    let report = serde_json::to_value(err.to_report()).unwrap();
    assert_eq!(report["message"], err.to_string());
    report
}

fn report(method: &str, input: &Value) -> Value {
    report_text(method, &input.to_string())
}

fn golden_input(relative: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(relative);
    let case: Value = serde_norway::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    case["input"].clone()
}

fn pointers_of(r: &Value) -> Vec<String> {
    r["errors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["pointer"].as_str().unwrap_or("null").to_owned())
        .collect()
}

fn annuity() -> Value {
    json!({"payment": "1", "rate": "0.1", "periods": 2, "timing": "ordinary"})
}

fn series() -> Value {
    golden_input("series.project/annual-without-basis.yaml")
}

// ------------------------------------------------------------- audit 1: wrong kind of value

#[test]
fn a_wrong_kind_of_value_for_a_choice_is_named_with_its_values() {
    for bad in [json!(true), json!(5), json!(null)] {
        let mut input = annuity();
        input["timing"] = bad.clone();
        let r = report("tvm.annuity", &input);
        assert_eq!(r["pointer"], "/timing", "{bad}: {r:#}");
        assert_eq!(r["reason"], "enum", "{bad}");
        assert_eq!(r["expected"], json!(["ordinary", "due"]), "{bad}");
        assert!(!r["message"].as_str().unwrap().contains(" line "), "{r:#}");
    }
}

#[test]
fn a_single_key_choice_with_two_keys_names_the_extra_key() {
    let mut input = series();
    input["growth"]["x"] = json!(1);
    let r = report("series.project", &input);
    assert_eq!(r["pointer"], "/growth/x", "{r:#}");
    assert_eq!(r["reason"], "unknown_field");
    let mut input = series();
    input["growth"]["per_period"] = json!({"growth_rates": ["0.1"]});
    let r = report("series.project", &input);
    assert_eq!(r["pointer"], "/growth/per_period", "{r:#}");
    assert_eq!(r["reason"], "length");
}

#[test]
fn errors_inside_base_input_never_give_a_line_and_column() {
    let input = json!({
        "method": "tvm.annuity",
        "base_input": {"payment": "1", "rate": "0.1", "periods": 2, "timing": true},
        "variable": "/rate", "output": "/present_value", "target": "1",
        "bracket": ["0.01", "0.5"]
    });
    let r = report("analysis.goal_seek", &input);
    assert_eq!(r["pointer"], "/base_input/timing", "{r:#}");
    assert_eq!(r["reason"], "enum");
    assert!(!r["message"].as_str().unwrap().contains(" line "));
}

#[test]
fn syntax_is_only_for_text_that_is_not_json() {
    let r = report_text("tvm.npv", r#"{"rate": "#);
    assert_eq!(r["reason"], "syntax");
    assert!(r["pointer"].is_null());
}

// ------------------------------------------------------------- audit 6, 15: not an object

#[test]
fn an_input_that_is_not_an_object_is_refused_at_the_root() {
    for text in ["42", r#""x""#, "[0.1, [-100, 60, 60]]"] {
        let r = report_text("tvm.npv", text);
        assert_eq!(r["code"], "FM-E001", "{text}");
        assert_eq!(r["pointer"], "", "{text}: {r:#}");
        assert_eq!(r["reason"], "type", "{text}");
    }
}

/// Audit 15: an object written as a positional array is refused in every method, at any depth.
#[test]
fn positional_arrays_are_refused_in_every_method() {
    for m in fairmark_core::METHODS {
        let r = report_text(m.id, "[]");
        assert_eq!(r["pointer"], "", "{}", m.id);
        assert_eq!(r["reason"], "type", "{}", m.id);
    }
    let mut input = golden_input("tv.compare/seven-techniques.yaml");
    input["techniques"][0]["tv.gordon"] = json!(["0.02", "100"]);
    let r = report("tv.compare", &input);
    assert_eq!(r["pointer"], "/techniques/0/tv.gordon", "{r:#}");
    assert_eq!(r["reason"], "type");
}

// ------------------------------------------------------------- audit 14: duplicate keys

/// Audit 14: a key repeated in one object is refused in every method, never read with its last
/// value.
#[test]
fn duplicate_keys_are_refused_everywhere() {
    let r = report_text(
        "tv.compare",
        r#"{"rate": "0.1", "basis": "enterprise", "techniques": [
            {"tv.gordon": {"growth": "0.02", "next_flow": "1", "growth": "0.05"}}]}"#,
    );
    assert_eq!(r["pointer"], "/techniques/0/tv.gordon/growth", "{r:#}");
    assert_eq!(r["reason"], "duplicate");
    let r = report_text(
        "tv.compare",
        r#"{"rate": "0.1", "basis": "enterprise", "techniques": [
            {"tv.gordon": {"growth": "0.02", "next_flow": "1"},
             "tv.gordon": {"growth": "0.03", "next_flow": "1"}}]}"#,
    );
    assert_eq!(r["pointer"], "/techniques/0/tv.gordon", "{r:#}");
    assert_eq!(r["reason"], "duplicate");
    let r = report_text(
        "series.project",
        r#"{"base_value": "1", "base_period": "period_0",
            "growth": {"constant": {"growth": "0.1"}, "constant": {"growth": "0.2"}},
            "timeline": {"periods": {"number_of_periods": 2, "frequency": "annual"}}}"#,
    );
    assert_eq!(r["pointer"], "/growth/constant", "{r:#}");
    assert_eq!(r["reason"], "duplicate");
    for m in fairmark_core::METHODS {
        let r = report_text(m.id, r#"{"zz": 1, "zz": 2}"#);
        let entry = r["errors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|e| e["reason"] == "duplicate");
        assert!(
            entry.is_some_and(|e| e["pointer"] == "/zz"),
            "{}: {r:#}",
            m.id
        );
    }
}

// ------------------------------------------------------------- audit 4, 9: single-key choices

#[test]
fn technique_entries_name_their_keys() {
    let base = golden_input("tv.compare/seven-techniques.yaml");
    let mut empty = base.clone();
    empty["techniques"][0] = json!({});
    let r = report("tv.compare", &empty);
    assert_eq!(r["pointer"], "/techniques/0");
    assert_eq!(r["reason"], "length");
    assert!(r["expected"]
        .as_array()
        .unwrap()
        .contains(&json!("tv.gordon")));
    let mut two = base.clone();
    two["techniques"][0]["tv.liquidation"] = base["techniques"][6]["tv.liquidation"].clone();
    let r = report("tv.compare", &two);
    assert_eq!(r["pointer"], "/techniques/0/tv.liquidation", "{r:#}");
    assert_eq!(r["reason"], "length");
}

/// Audit 9: a misspelt key of a single-key choice is an unknown field at the key, with the
/// closest key, in every method that has one.
#[test]
fn misspelt_choice_keys_are_unknown_fields_with_the_closest_key() {
    let mut compare = golden_input("tv.compare/seven-techniques.yaml");
    let terms = compare["techniques"][0]["tv.gordon"].clone();
    compare["techniques"][0] = json!({"tv.gordo": terms});
    let mut series = series();
    series["growth"] = json!({"constnt": {"growth": "0.1"}});
    let mut timeline = series_valid();
    timeline["timeline"] = json!({"period": {"number_of_periods": 2, "frequency": "annual"}});
    let mut dcf = golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml");
    let terms = dcf["terminal"]["tv.gordon"].clone();
    dcf["terminal"] = json!({"tv.gordo": terms});
    for (method, input, at, closest) in [
        ("tv.compare", compare, "/techniques/0/tv.gordo", "tv.gordon"),
        ("series.project", series, "/growth/constnt", "constant"),
        ("series.project", timeline, "/timeline/period", "periods"),
        ("dcf.fcff", dcf, "/terminal/tv.gordo", "tv.gordon"),
    ] {
        let r = report(method, &input);
        assert_eq!(r["pointer"], at, "{method}: {r:#}");
        assert_eq!(r["reason"], "unknown_field", "{method}");
        assert!(r["expected"].as_array().unwrap().contains(&json!(closest)));
        let key = at.rsplit('/').next().unwrap();
        assert_eq!(
            r["fix"],
            format!("rename `{key}` to `{closest}`"),
            "{method}"
        );
    }
}

fn series_valid() -> Value {
    series()
}

#[test]
fn suggestions_ignore_case() {
    let mut input = json!({"rate": "0.1", "cash_flows": [-100, 60]});
    input["Cash_Flows"] = input["cash_flows"].take();
    input.as_object_mut().unwrap().remove("cash_flows");
    let r = report("tvm.npv", &input);
    assert_eq!(r["fix"], "rename `Cash_Flows` to `cash_flows`");
}

// ------------------------------------------------------------- audit 8: statements kind

#[test]
fn adjustment_kind_is_an_expected_field_and_its_misspelling_is_named() {
    let mut input = golden_input("statements.reorganise/adjustment-non-recurring.yaml");
    input["adjustments"][0]["labl"] = json!("x");
    let r = report("statements.reorganise", &input);
    assert!(
        r["expected"].as_array().unwrap().contains(&json!("kind")),
        "{r:#}"
    );
    let mut input = golden_input("statements.reorganise/adjustment-non-recurring.yaml");
    let kind = input["adjustments"][0]
        .as_object_mut()
        .unwrap()
        .shift_remove("kind")
        .unwrap();
    input["adjustments"][0]["kidn"] = kind;
    let r = report("statements.reorganise", &input);
    assert_eq!(r["pointer"], "/adjustments/0/kidn", "{r:#}");
    assert_eq!(r["reason"], "unknown_field");
    assert_eq!(r["fix"], "rename `kidn` to `kind`");
}

#[test]
fn a_fixed_aggregate_lists_the_aggregates_allowed() {
    let mut input = golden_input("statements.reorganise/adjustment-non-recurring.yaml");
    input["adjustments"][0]["aggregate"] = json!("write_downs_of_financial_assets");
    let r = report("statements.reorganise", &input);
    assert_eq!(r["reason"], "enum");
    let expected = r["expected"].as_array().unwrap();
    assert!(expected.contains(&json!("services")), "{r:#}");
    assert!(!expected.contains(&json!("write_downs_of_financial_assets")));
}

// ------------------------------------------------------------- audit 11: plain descriptions

#[test]
fn type_errors_describe_the_form_in_plain_words() {
    let mut input = annuity();
    input["periods"] = json!(-1);
    let r = report("tvm.annuity", &input);
    assert_eq!(r["reason"], "range", "{r:#}");
    assert_eq!(r["expected"], json!(["a whole number of 0 or more"]));
    let mut input = annuity();
    input["periods"] = json!("2");
    let r = report("tvm.annuity", &input);
    assert_eq!(r["reason"], "type");
    assert_eq!(r["expected"], json!(["a whole number of 0 or more"]));
    let r = report("tvm.npv", &json!({"rate": "0.1", "cash_flows": "1"}));
    assert_eq!(r["expected"], json!(["a list"]), "{r:#}");
    for e in [&r["expected"][0], &r["fix"]] {
        let text = e.as_str().unwrap();
        for rust in ["u32", "u64", "usize", "i64", "sequence", "map", "struct"] {
            assert!(!text.contains(rust), "{text}");
        }
    }
}

// ------------------------------------------------------------- audit 12, 13, 5, 4c, 7

#[test]
fn an_unknown_method_lists_the_methods() {
    let err = run("tvm.nope", "{}").unwrap_err();
    let r = serde_json::to_value(err.to_report()).unwrap();
    assert!(r["expected"]
        .as_array()
        .unwrap()
        .contains(&json!("tvm.npv")));
    let r = report(
        "analysis.goal_seek",
        &json!({"method": "tvm.nope", "base_input": {}, "variable": "/rate",
                "output": "/x", "target": "1", "bracket": ["0.01", "0.5"]}),
    );
    assert_eq!(r["pointer"], "/method");
    let expected = r["expected"].as_array().unwrap();
    assert!(expected.contains(&json!("tvm.npv")));
    assert!(!expected.contains(&json!("analysis.tornado")));
}

#[test]
fn a_default_growth_is_never_the_pointer() {
    let r = report(
        "tvm.perpetuity",
        &json!({"payment": "1", "rate": "0", "timing": "ordinary"}),
    );
    assert_eq!(r["code"], "FM-E004");
    assert_eq!(r["pointer"], "/rate", "{r:#}");
    assert_eq!(r["pointers"], json!(["/rate"]));
    assert!(r["fix"].as_str().unwrap().contains("growth"));
}

#[test]
fn growth_not_below_rate_names_both_fields() {
    for (method, input, growth, rate) in [
        (
            "tv.gordon",
            json!({"rate": "0.1", "growth": "0.2", "next_flow": "1"}),
            "/growth",
            "/rate",
        ),
        (
            "tv.value_driver",
            json!({"rate": "0.1", "growth": "0.2", "ronic": "0.1", "nopat_next": "1"}),
            "/growth",
            "/rate",
        ),
        (
            "tv.normalised_income",
            golden_set("tv.normalised_income", "/growth", json!("0.9")),
            "/growth",
            "/capitalisation_rate",
        ),
        (
            "tv.h_model",
            golden_set("tv.h_model", "/long_term_growth", json!("0.9")),
            "/long_term_growth",
            "/rate",
        ),
    ] {
        let r = report(method, &input);
        assert_eq!(r["code"], "FM-E004", "{method}: {r:#}");
        assert_eq!(r["pointers"], json!([growth, rate]), "{method}");
    }
    let mut compare = golden_input("tv.compare/seven-techniques.yaml");
    compare["techniques"][0]["tv.gordon"]["growth"] = json!("0.5");
    let r = report("tv.compare", &compare);
    assert_eq!(
        r["pointers"],
        json!(["/techniques/0/tv.gordon/growth", "/rate"]),
        "{r:#}"
    );
}

/// The first golden input of `method` with `pointer` set to `value`.
fn golden_set(method: &str, pointer: &str, value: Value) -> Value {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(method);
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect();
    files.sort();
    for file in files {
        let case: Value = serde_norway::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
        if case.get("expect_error").is_none() {
            let mut input = case["input"].clone();
            *input
                .pointer_mut(pointer)
                .expect("the field is in the golden input") = value;
            return input;
        }
    }
    panic!("no valid golden input for {method}")
}

#[test]
fn limited_horizon_growth_is_a_range_error() {
    let r = report(
        "tv.limited_horizon",
        &json!({"rate": "0.1", "growth": "-1", "periods": 2, "flow": "1"}),
    );
    assert_eq!(r["pointer"], "/growth");
    assert_eq!(r["reason"], "range");
    assert_eq!(r["expected"], json!(["a growth greater than -1 (-100%)"]));
}

#[test]
fn rounding_too_coarse_names_the_decimal_places() {
    let r = report(
        "loans.amortisation_schedule",
        &golden_input("loans.amortisation_schedule/error-rounding-too-coarse-fixed.yaml"),
    );
    assert_eq!(r["code"], "FM-E023");
    assert_eq!(r["pointer"], "/rounding/decimal_places");
    assert_eq!(r["reason"], "range");
}

// ------------------------------------------------------------- review blocking 1, audit 2, 3

fn goal_seek() -> Value {
    json!({
        "method": "tvm.annuity",
        "base_input": annuity(),
        "variable": "/rate",
        "output": "/present_value",
        "target": "1",
        "bracket": ["0.01", "0.5"]
    })
}

#[test]
fn analysis_checks_and_base_input_are_reported_together() {
    let mut input = goal_seek();
    input["bracket"] = json!(["0.5", "0.01"]);
    input["tolerance"] = json!("0");
    input["max_iterations"] = json!(0);
    input["base_input"]["payment"] = json!("x1");
    input["base_input"]["bogus"] = json!(1);
    input["base_input"]["timng"] = json!("ordinary");
    let r = report("analysis.goal_seek", &input);
    assert_eq!(
        pointers_of(&r),
        [
            "/bracket",
            "/tolerance",
            "/max_iterations",
            "/base_input/payment",
        ],
        "{r:#}"
    );
    let mut input = goal_seek();
    input["base_input"]["bogus"] = json!(1);
    input["base_input"]["timng"] = json!("ordinary");
    input["tolerance"] = json!("-1");
    let r = report("analysis.goal_seek", &input);
    assert_eq!(
        pointers_of(&r),
        ["/tolerance", "/base_input/bogus", "/base_input/timng"],
        "{r:#}"
    );
}

#[test]
fn a_variable_must_point_to_a_decimal_field() {
    let mut input = goal_seek();
    input["variable"] = json!("/periods");
    input["bracket"] = json!(["1", "5"]);
    let r = report("analysis.goal_seek", &input);
    assert_eq!(r["code"], "FM-E015", "{r:#}");
    assert_eq!(r["pointer"], "/variable");
}

#[test]
fn every_analysis_collects_its_request_errors() {
    let scenarios = json!({
        "method": "tvm.npv",
        "base_input": {"rate": "0.1", "cash_flows": [-100, 60, 60], "bogus": 1},
        "outputs": ["/npv", "/npv"],
        "scenarios": [
            {"name": " ", "overrides": [{"pointer": "/rate", "value": "0.2"}]},
            {"name": "b", "overrides": [{"pointer": "/rate", "value": "0.2"}]},
            {"name": "b", "overrides": [{"pointer": "/rate", "value": "0.3"}]}
        ]
    });
    let r = report("analysis.scenarios", &scenarios);
    assert_eq!(
        pointers_of(&r),
        [
            "/outputs/1",
            "/scenarios/0/name",
            "/scenarios/2/name",
            "/base_input/bogus"
        ],
        "{r:#}"
    );
    let tornado = json!({
        "method": "tvm.npv",
        "base_input": {"rate": "0.1", "cash_flows": [-100, 60, 60], "bogus": 1},
        "output": "/npv",
        "variables": [
            {"variable": "/rate", "low": "0.2", "high": "0.1"},
            {"variable": "/nope", "low": "0.1", "high": "0.2"}
        ]
    });
    let r = report("analysis.tornado", &tornado);
    assert_eq!(
        pointers_of(&r),
        [
            "/variables/0/low",
            "/variables/1/variable",
            "/base_input/bogus"
        ],
        "{r:#}"
    );
    let sensitivity = json!({
        "method": "tvm.npv",
        "base_input": {"rate": "0.1", "cash_flows": [-100, 60, 60], "bogus": 1},
        "output": "npv",
        "rows": {"variable": "/rate", "values": []}
    });
    let r = report("analysis.sensitivity", &sensitivity);
    assert_eq!(
        pointers_of(&r),
        ["/rows/values", "/output", "/base_input/bogus"],
        "{r:#}"
    );
}

#[test]
fn dcf_reports_every_rate_and_the_terminal_terms() {
    let mut input = golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml");
    let n: usize = input["stages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            s["periods"]
                .as_u64()
                .map_or_else(|| s["flows"].as_array().unwrap().len(), |p| p as usize)
        })
        .sum();
    input.as_object_mut().unwrap().remove("wacc");
    let mut rates = vec![json!("0.09"); n];
    rates[0] = json!("-1");
    rates[2] = json!("-2");
    input["wacc_per_period"] = json!(rates);
    input["terminal_wacc"] = json!("0.09");
    let r = report("dcf.fcff", &input);
    assert_eq!(
        pointers_of(&r),
        ["/wacc_per_period/0", "/wacc_per_period/2"],
        "{r:#}"
    );

    let mut input = golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml");
    input["wacc"] = json!("-2");
    input["terminal"]["tv.gordon"]["growth"] = json!("-2");
    let r = report("dcf.fcff", &input);
    assert_eq!(
        pointers_of(&r),
        ["/wacc", "/terminal/tv.gordon/growth"],
        "{r:#}"
    );
}

#[test]
fn loans_check_the_rate_with_the_principal() {
    let input = json!({
        "principal": "0", "number_of_instalments": 12, "timing": "ordinary",
        "rate": "-13", "compounding": "monthly", "payment_frequency": "monthly"
    });
    let r = report("loans.instalment", &input);
    assert_eq!(pointers_of(&r), ["/principal", "/rate"], "{r:#}");
    assert_eq!(r["errors"][1]["code"], "FM-E003");
}

#[test]
fn rates_and_series_check_both_fields() {
    let r = report(
        "rates.real_nominal",
        &json!({"rate": "-2", "inflation": "-2", "direction": "to_real"}),
    );
    assert_eq!(pointers_of(&r), ["/rate", "/inflation"], "{r:#}");
    let mut input = series();
    input["growth"] = json!({"linear": {"from_growth": "-2", "to_growth": "-3"}});
    let r = report("series.project", &input);
    assert_eq!(
        pointers_of(&r),
        ["/growth/linear/from_growth", "/growth/linear/to_growth"],
        "{r:#}"
    );
}
