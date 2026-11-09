//! Round 5 of the review of #279: the number grammar with signed exponents, reading errors that
//! do not hide the method's own checks, one entry per refused object, per condition and per tag,
//! and the same reasons inside `tv.compare` as for a technique given directly.

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

fn pointers(r: &Value) -> Vec<String> {
    entries(r).into_iter().map(|(p, _)| p).collect()
}

fn sorted(mut v: Vec<String>) -> Vec<String> {
    v.sort();
    v
}

// ------------------------------------------------------------- B1: signed exponents (G7)

#[test]
fn decimal_strings_accept_signed_exponents() {
    let out = run(
        "tvm.npv",
        &json!({"rate": "1E-7", "cash_flows": ["1e-07", "1E+3", "-0E-8", "-100"]}).to_string(),
    )
    .expect("signed exponents follow the number grammar");
    assert!(out["value"]["npv"].is_string());
    for bad in [
        "+0.1", ".1", "1.", "1_000", "1e", "1e+", "1E-", "--1", "1e1.5",
    ] {
        let r = report("tvm.npv", &json!({"rate": bad, "cash_flows": ["-1", "2"]}));
        assert_eq!(
            entries(&r),
            [("/rate".to_owned(), "type".to_owned())],
            "{bad}"
        );
    }
}

// ------------------------------------------------------------- audit 1: mixed kinds

#[test]
fn reading_errors_do_not_hide_the_checks_of_the_method() {
    let r = report(
        "coc.wacc",
        &json!({"cost_of_equity": "0.10", "pre_tax_cost_of_debt": "+0.1", "tax_rate": "-1",
                "debt_to_equity": ".1"}),
    );
    assert_eq!(
        sorted(pointers(&r)),
        ["/debt_to_equity", "/pre_tax_cost_of_debt", "/tax_rate"],
        "{r:#}"
    );
    assert_eq!(
        r["pointer"], "/pre_tax_cost_of_debt",
        "serde's first error stays first"
    );

    let mut loan = golden_input("loans.instalment/annual-rate-form.yaml");
    loan["principal"] = json!("-1");
    loan["number_of_instalments"] = json!(-1);
    let r = report("loans.instalment", &loan);
    assert_eq!(
        sorted(pointers(&r)),
        ["/number_of_instalments", "/principal"],
        "{r:#}"
    );

    let mut loan = golden_input("loans.instalment/annual-rate-form.yaml");
    loan["rounding"] = json!({"decimal_places": 2, "mood": "half_up"});
    let r = report("loans.instalment", &loan);
    let found = entries(&r);
    assert!(
        found.contains(&("/rounding/mood".to_owned(), "unknown_field".to_owned())),
        "{r:#}"
    );
    assert!(found.iter().any(|(p, _)| p == "/rounding/mode"), "{r:#}");
}

#[test]
fn values_put_in_place_of_refused_ones_never_raise_errors() {
    // A principal that cannot be read: the balloon is compared with nothing.
    let mut loan = golden_input("loans.instalment/annual-rate-form.yaml");
    loan["principal"] = json!("x");
    loan["balloon_amount"] = json!("500");
    let r = report("loans.instalment", &loan);
    assert_eq!(pointers(&r), ["/principal"], "{r:#}");
    // A stage that cannot be read: the count of the per-period rates is not compared with it.
    let mut dcf = golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml");
    dcf["stages"][0]["periods"] = json!("three");
    let r = report("dcf.fcff", &dcf);
    assert_eq!(pointers(&r), ["/stages/0/periods"], "{r:#}");
}

// ------------------------------------------------------------- audit 2: object as array

#[test]
fn an_object_given_as_an_array_is_one_entry_at_the_object() {
    let dcf = golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml");
    let fcf = golden_input("dcf.free_cash_flows/rate-form-fcff-and-fcfe.yaml");
    let ds = golden_input("ratios.debt_service/loan-schedule.yaml");
    let sp = golden_input("series.project/annual-without-basis.yaml");
    for (method, base, at) in [
        ("dcf.fcff", dcf, "/stages/1"),
        ("dcf.free_cash_flows", fcf, "/periods/0"),
        ("ratios.debt_service", ds, "/debt_schedule"),
        ("series.project", sp, "/growth"),
    ] {
        let mut input = base.clone();
        let object = input.pointer(at).unwrap().as_object().unwrap().clone();
        *input.pointer_mut(at).unwrap() = Value::Array(object.values().cloned().collect());
        let r = report(method, &input);
        assert_eq!(
            entries(&r),
            [(at.to_owned(), "type".to_owned())],
            "{method}: {r:#}"
        );
    }
}

// ------------------------------------------------------------- audit 3: one entry per condition

#[test]
fn one_condition_is_one_entry() {
    let mut input = golden_input("series.project/annual-without-basis.yaml");
    input["growth"] = json!({"constant": {"growth": "-5"}});
    input["timeline"]["periods"]["number_of_periods"] = json!(12);
    let r = report("series.project", &input);
    assert_eq!(pointers(&r), ["/growth/constant/growth"], "{r:#}");
}

// ------------------------------------------------------------- audit 4: tags

fn adjustment(edit: impl FnOnce(&mut serde_json::Map<String, Value>)) -> Value {
    let mut input = golden_input("statements.reorganise/adjustment-non-recurring.yaml");
    edit(input["adjustments"][0].as_object_mut().unwrap());
    report("statements.reorganise", &input)
}

#[test]
fn a_refused_tag_is_the_only_entry_of_its_object() {
    let r = adjustment(|a| {
        a.insert("kind".into(), json!("non_recurrent"));
    });
    assert_eq!(
        entries(&r),
        [("/adjustments/0/kind".to_owned(), "enum".to_owned())],
        "{r:#}"
    );
    let r = adjustment(|a| {
        a.remove("kind");
    });
    assert_eq!(
        entries(&r),
        [("/adjustments/0/kind".to_owned(), "missing".to_owned())],
        "{r:#}"
    );
    let r = adjustment(|a| {
        let kind = a.remove("kind").unwrap();
        a.insert("kidn".into(), kind);
    });
    assert_eq!(
        entries(&r),
        [
            ("/adjustments/0/kidn".to_owned(), "unknown_field".to_owned()),
            ("/adjustments/0/kind".to_owned(), "missing".to_owned())
        ],
        "{r:#}"
    );
    let expected = r["errors"][0]["expected"].as_array().unwrap();
    for field in ["kind", "label", "aggregate", "amounts"] {
        assert!(expected.contains(&json!(field)), "{field}: {r:#}");
    }
    assert_eq!(r["errors"][0]["fix"], "rename `kidn` to `kind`");
}

// ------------------------------------------------------------- audit 5, 9: goal seek

fn goal_seek() -> Value {
    json!({
        "method": "tvm.annuity",
        "base_input": {"payment": "1", "rate": "-5", "periods": 2, "timing": "ordinary"},
        "variable": "/rate",
        "output": "/present_value",
        "target": "1",
        "bracket": ["0.01", "0.5"]
    })
}

#[test]
fn goal_seek_never_checks_the_value_its_variable_replaces() {
    let mut input = goal_seek();
    input["tolerance"] = json!("x");
    let r = report("analysis.goal_seek", &input);
    assert_eq!(pointers(&r), ["/tolerance"], "{r:#}");
    // An independent error of base_input is still reported with it.
    input["base_input"]["timing"] = json!("sometimes");
    let r = report("analysis.goal_seek", &input);
    assert_eq!(pointers(&r), ["/tolerance", "/base_input/timing"], "{r:#}");
}

#[test]
fn an_unknown_inner_method_is_the_only_entry_about_it() {
    for variable in ["/nope", "/periods", "/rate"] {
        let mut input = goal_seek();
        input["method"] = json!("tvm.nope");
        input["variable"] = json!(variable);
        let r = report("analysis.goal_seek", &input);
        assert_eq!(pointers(&r), ["/method"], "{variable}: {r:#}");
        assert_eq!(r["code"], "FM-E007");
        // Also when the analysis input itself cannot be read.
        input["tolerance"] = json!("x");
        let r = report("analysis.goal_seek", &input);
        assert_eq!(
            sorted(pointers(&r)),
            ["/method", "/tolerance"],
            "{variable}: {r:#}"
        );
    }
}

// ------------------------------------------------------------- audit 6: closed values

#[test]
fn an_object_for_a_closed_value_is_enum_at_the_field() {
    let mut input = golden_input("coc.beta_relever/hamada-relever.yaml");
    input["formula"] = json!({"a": 1});
    let r = report("coc.beta_relever", &input);
    assert_eq!(
        entries(&r),
        [("/formula".to_owned(), "enum".to_owned())],
        "{r:#}"
    );
    assert_eq!(r["expected"], json!(["hamada", "harris_pringle"]));
}

// ------------------------------------------------------------- audit 7, 8: techniques

fn compare_with(position: usize, id: &str, field: &str, value: Value) -> Value {
    let mut input = golden_input("tv.compare/seven-techniques.yaml");
    input["techniques"][position][id][field] = value;
    report("tv.compare", &input)
}

#[test]
fn technique_fields_have_the_reasons_of_the_method_given_directly() {
    let r = compare_with(3, "tv.exit_multiple", "metric", json!(true));
    let e = &r["errors"][0];
    assert_eq!(
        e["pointer"], "/techniques/3/tv.exit_multiple/metric",
        "{r:#}"
    );
    assert_eq!(e["reason"], "enum", "{r:#}");
    let direct = report(
        "tv.exit_multiple",
        &json!({"multiple": "7.5", "metric": true, "metric_value": "180", "next_flow": "95",
                "rate": "0.1"}),
    );
    assert_eq!(e["expected"], direct["expected"]);

    let r = compare_with(4, "tv.limited_horizon", "periods", json!(-1));
    let e = &r["errors"][0];
    assert_eq!(
        e["pointer"], "/techniques/4/tv.limited_horizon/periods",
        "{r:#}"
    );
    assert_eq!(e["reason"], "range", "{r:#}");
    let direct = report(
        "tv.limited_horizon",
        &json!({"flow": "100", "periods": -1, "growth": "0.02", "residual_value": "500",
                "rate": "0.1"}),
    );
    assert_eq!(e["expected"], direct["expected"]);
    assert_eq!(e["reason"], direct["reason"]);
}

#[test]
fn an_empty_terminal_choice_is_length() {
    for method in ["dcf.fcff", "dcf.fcfe"] {
        let mut input = if method == "dcf.fcff" {
            golden_input("dcf.fcff/three-stage-given-linear-decline-gordon.yaml")
        } else {
            json!({"stages": [{"periods": 1, "projection": "given", "flows": ["100"]}],
                   "cost_of_equity": "0.1", "terminal": {"tv.gordon": {"growth": "0.02"}}})
        };
        input["terminal"] = json!({});
        let r = report(method, &input);
        assert_eq!(
            entries(&r),
            [("/terminal".to_owned(), "length".to_owned())],
            "{r:#}"
        );
    }
}

#[test]
fn an_extra_key_beside_a_technique_is_one_entry() {
    let mut input = golden_input("tv.compare/seven-techniques.yaml");
    input["techniques"][0]["foo"] = json!(1);
    let r = report("tv.compare", &input);
    assert_eq!(
        entries(&r),
        [("/techniques/0/foo".to_owned(), "unknown_field".to_owned())],
        "{r:#}"
    );
    assert!(r["expected"]
        .as_array()
        .unwrap()
        .contains(&json!("tv.value_driver")));
}
