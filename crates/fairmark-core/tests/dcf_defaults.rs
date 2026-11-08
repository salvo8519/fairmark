//! SPEC-0001 Decision 24 (issue #197), R31: `dcf.fcff` and `dcf.fcfe` from a vector of cash
//! flows and a discount rate, with documented defaults reported in `explain.conventions` and
//! `meta.defaults_applied`, and `meta.input_hash` on the input with the defaults filled in.

use fairmark_core::run;
use serde_json::{json, Value};

const FLOWS: [&str; 5] = ["120", "135", "150", "160", "170"];

fn minimal() -> Value {
    json!({"stages": [{"flows": FLOWS}], "wacc": "0.09"})
}

fn explicit() -> Value {
    json!({
        "stages": [{"periods": 5, "projection": "given", "flows": FLOWS}],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": "0.09",
        "timing": "end_of_period",
        "terminal": {"none": {}}
    })
}

fn ok(method: &str, v: &Value) -> Value {
    run(method, &v.to_string()).unwrap_or_else(|e| panic!("{method}: {e}"))
}

fn conventions(out: &Value) -> Vec<String> {
    serde_json::from_value(out["explain"]["conventions"].clone()).unwrap()
}

fn defaulted_fields(out: &Value) -> Vec<String> {
    out["meta"]["defaults_applied"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|d| d["field"].as_str().unwrap().to_owned())
                .collect()
        })
        .unwrap_or_default()
}

/// Decision 24: the minimal input fills in every default, in input order, each with its
/// value and a reason, and reports each first in the conventions.
#[test]
fn minimal_input_reports_every_default() {
    let out = ok("dcf.fcff", &minimal());
    assert_eq!(
        defaulted_fields(&out),
        [
            "/stages/0/projection",
            "/stages/0/periods",
            "/cash_flow_terms",
            "/discount_rate_terms",
            "/timing",
            "/terminal"
        ]
    );
    let defaults = out["meta"]["defaults_applied"].as_array().unwrap();
    assert_eq!(defaults[0]["value"], "given");
    assert_eq!(defaults[1]["value"], 5);
    assert_eq!(defaults[2]["value"], "nominal");
    assert_eq!(defaults[4]["value"], "end_of_period");
    assert_eq!(defaults[5]["value"], json!({"none": {}}));
    assert!(defaults
        .iter()
        .all(|d| !d["reason"].as_str().unwrap().is_empty()));
    let c = conventions(&out);
    for (i, d) in defaults.iter().enumerate() {
        assert!(c[i].starts_with("default: "), "{}", c[i]);
        assert!(c[i].contains(d["field"].as_str().unwrap()), "{}", c[i]);
    }
    assert!(!c[defaults.len()].starts_with("default: "));
    // Texts are single-spaced (a broken string continuation shows as a run of spaces).
    assert!(c.iter().all(|c| !c.contains("  ")), "{c:?}");
}

/// Decision 24 (e): with no terminal value the result carries a warning, a terminal value of
/// zero and a weight of zero.
#[test]
fn no_terminal_value_is_a_warning() {
    let out = ok("dcf.fcff", &minimal());
    let c = conventions(&out);
    assert!(
        c.iter()
            .any(|c| c.starts_with("warning: no terminal value is included")),
        "{c:?}"
    );
    assert_eq!(out["value"]["terminal"]["technique"], "none");
    assert_eq!(out["value"]["terminal"]["terminal_value"], "0");
    assert_eq!(out["value"]["terminal"]["present_value"], "0");
    assert_eq!(out["value"]["terminal_value_weight"], "0");
    assert!(out["value"]["terminal"]["output"].is_null());
    // The explicit `none` carries the same warning, with no default entry.
    let c = conventions(&ok("dcf.fcff", &explicit()));
    assert!(c
        .iter()
        .any(|c| c.starts_with("warning: no terminal value is included")));
}

/// R31: explicit inputs prevail and produce no default entry; `meta.defaults_applied` is then
/// omitted; the hash covers the input with its defaults, so the minimal input and its explicit
/// equivalent hash identically and give the same value.
#[test]
fn explicit_equivalent_has_the_same_hash_and_no_default_entry() {
    let (a, b) = (ok("dcf.fcff", &minimal()), ok("dcf.fcff", &explicit()));
    assert!(b["meta"].get("defaults_applied").is_none());
    assert_eq!(a["meta"]["input_hash"], b["meta"]["input_hash"]);
    assert_eq!(a["value"], b["value"]);
    let without_defaults: Vec<String> = conventions(&a)
        .into_iter()
        .filter(|c| !c.starts_with("default: "))
        .collect();
    assert_eq!(without_defaults, conventions(&b));
}

/// R31: each default overridden alone removes exactly its own entry.
#[test]
fn each_explicit_field_prevails_alone() {
    let cases: Vec<(&str, Value)> = vec![
        ("/stages/0/projection", json!({"projection": "given"})),
        ("/stages/0/periods", json!({"periods": 5})),
        ("/cash_flow_terms", json!({"cash_flow_terms": "nominal"})),
        (
            "/discount_rate_terms",
            json!({"discount_rate_terms": "nominal"}),
        ),
        ("/timing", json!({"timing": "end_of_period"})),
        ("/terminal", json!({"terminal": {"none": {}}})),
    ];
    for (field, patch) in cases {
        let mut v = minimal();
        for (k, x) in patch.as_object().unwrap() {
            if field.starts_with("/stages") {
                v["stages"][0][k] = x.clone();
            } else {
                v[k] = x.clone();
            }
        }
        let out = ok("dcf.fcff", &v);
        let fields = defaulted_fields(&out);
        assert_eq!(fields.len(), 5, "{field}: {fields:?}");
        assert!(!fields.iter().any(|f| f == field), "{field}");
        assert_eq!(
            out["meta"]["input_hash"],
            ok("dcf.fcff", &minimal())["meta"]["input_hash"]
        );
    }
}

/// Decision 24, independent cross-check: the minimal DCF equals `tvm.npv` of the same flows at
/// the same rate, the first flow at t = 0 being zero. At the rate of the golden cases (9%) the
/// two agree to the last digit; at other rates they may differ in the last digits, because each
/// rounds its own operations at 28 digits: at 15% and at -2% the DCF gives the correctly rounded
/// 50-digit value (481.05522033241547257503212616 and 783.92388106187668145041620974), while
/// `tvm.npv` is 1 and 8 units lower in the last place.
#[test]
fn minimal_input_equals_tvm_npv() {
    let relative: rust_decimal::Decimal = "0.0000000000000000000000001".parse().unwrap();
    for rate in ["0.09", "0.0725", "0.15", "-0.02"] {
        let mut v = minimal();
        v["wacc"] = json!(rate);
        let dcf = ok("dcf.fcff", &v);
        let mut flows = vec!["0"];
        flows.extend(FLOWS);
        let npv = ok("tvm.npv", &json!({"rate": rate, "cash_flows": flows}));
        let a: rust_decimal::Decimal = dcf["value"]["value_of_operations"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        let b: rust_decimal::Decimal = npv["value"]["npv"].as_str().unwrap().parse().unwrap();
        if rate == "0.09" {
            assert_eq!(a, b, "rate {rate}");
        } else {
            assert!((a - b).abs() <= relative * a.abs(), "rate {rate}: {a} {b}");
        }
    }
}

/// Decision 24 on the equity basis: the same defaults through the shared engine.
#[test]
fn fcfe_minimal_input() {
    let out = ok(
        "dcf.fcfe",
        &json!({"stages": [{"flows": ["80", "85"]}], "cost_of_equity": "0.11"}),
    );
    assert_eq!(defaulted_fields(&out).len(), 6);
    assert_eq!(out["value"]["terminal"]["technique"], "none");
}

/// Decision 24 (c), R15: the message says when a term came from its default.
#[test]
fn terms_mismatch_names_the_default() {
    let mut v = minimal();
    v["cash_flow_terms"] = json!("real");
    let e = run("dcf.fcff", &v.to_string()).unwrap_err();
    assert_eq!(e.code(), "FM-E024");
    let m = e.to_string();
    assert!(
        m.contains("`/discount_rate_terms` was not given and defaults to nominal"),
        "{m}"
    );
    assert!(m.contains("`discount_rate_terms: real`"), "{m}");
    // Both given: the message of R15 alone.
    v["discount_rate_terms"] = json!("nominal");
    let m = run("dcf.fcff", &v.to_string()).unwrap_err().to_string();
    assert!(!m.contains("was not given"), "{m}");
}

/// Decision 24: a terminal value given by the analyst is reported with its basis; a negative
/// amount is a warning, not an error.
#[test]
fn given_terminal_value_states_basis_and_warns_when_negative() {
    let mut v = minimal();
    v["terminal"] = json!({"given": {"value": "-300"}});
    let out = ok("dcf.fcff", &v);
    let c = conventions(&out);
    assert!(
        c.iter()
            .any(|c| c.starts_with("terminal value: given by the analyst")
                && c.contains("enterprise basis")),
        "{c:?}"
    );
    assert!(c
        .iter()
        .any(|c| c.starts_with("warning: the given terminal value -300 is negative")));
    assert_eq!(out["value"]["terminal"]["terminal_value"], "-300");
    let mut e = json!({"stages": [{"flows": ["80", "85"]}], "cost_of_equity": "0.11"});
    e["terminal"] = json!({"given": {"value": "900"}});
    let c = conventions(&ok("dcf.fcfe", &e));
    assert!(c.iter().any(
        |c| c.starts_with("terminal value: given by the analyst") && c.contains("equity basis")
    ));
    assert!(!c.iter().any(|c| c.contains("is negative")));
}

/// The terminal rate is reported only when it is used (Decision 24, R12).
#[test]
fn terminal_rate_only_when_used() {
    let out = ok("dcf.fcff", &minimal());
    assert!(out["value"]["terms"]["terminal_rate_given"].is_null());
    assert!(out["value"]["terminal"]["rate"].is_null());
    let mut v = minimal();
    v["timing"] = json!("mid_period");
    v["terminal_discount_time"] = json!("mid_period");
    v["terminal"] = json!({"given": {"value": "1000"}});
    let out = ok("dcf.fcff", &v);
    assert_eq!(out["value"]["terminal"]["rate"], "0.09");
    // With rates per period, the half-period shift needs a terminal rate.
    let mut v = v.clone();
    v.as_object_mut().unwrap().remove("wacc");
    v["wacc_per_period"] = json!(["0.09", "0.09", "0.09", "0.09", "0.09"]);
    let e = run("dcf.fcff", &v.to_string()).unwrap_err();
    assert_eq!(e.code(), "FM-E001");
    assert!(e.to_string().contains("/terminal_wacc"), "{e}");
}

/// Methods without defaults never carry `meta.defaults_applied`.
#[test]
fn methods_without_defaults_omit_the_field() {
    let out = ok(
        "tvm.npv",
        &json!({"rate": "0.1", "cash_flows": [-100, 60, 60]}),
    );
    assert!(out["meta"].get("defaults_applied").is_none());
}

/// R29, Decision 24: the defaulted fields are optional in the schema, their defaults are
/// documented in the descriptions, and no `default` keyword is added (schema v3 additive).
#[test]
fn schema_documents_the_defaults() {
    for id in ["dcf.fcff", "dcf.fcfe"] {
        let s = fairmark_core::input_schema(id).unwrap();
        assert_eq!(s["required"], json!(["stages"]), "{id}");
        for field in [
            "cash_flow_terms",
            "discount_rate_terms",
            "timing",
            "terminal",
        ] {
            let p = &s["properties"][field];
            assert!(p.get("default").is_none(), "{id} {field}");
            assert!(
                p["description"].as_str().unwrap().contains("Default:"),
                "{id} {field}"
            );
        }
        let stage = &s["$defs"]["Stage"];
        assert_eq!(stage.get("required"), None, "{id}");
        assert!(stage["properties"]["periods"]["description"]
            .as_str()
            .unwrap()
            .contains("Default:"));
    }
}

/// Code review of #197: an explicit `null` is not a way to ask for the default; it is refused
/// with `FM-E001`, the pointer and the fix, for every field with a documented default.
#[test]
fn explicit_null_is_refused_for_defaulted_fields() {
    let cases: Vec<(&str, Value)> = vec![
        ("/timing", json!({"timing": null})),
        ("/terminal", json!({"terminal": null})),
        ("/cash_flow_terms", json!({"cash_flow_terms": null})),
        ("/discount_rate_terms", json!({"discount_rate_terms": null})),
        (
            "/stages/0/periods",
            json!({"stages": [{"flows": FLOWS, "periods": null}]}),
        ),
        (
            "/stages/0/projection",
            json!({"stages": [{"flows": FLOWS, "projection": null}]}),
        ),
    ];
    for (pointer, patch) in cases {
        let mut v = minimal();
        for (k, x) in patch.as_object().unwrap() {
            v[k] = x.clone();
        }
        let e = run("dcf.fcff", &v.to_string()).unwrap_err();
        assert_eq!(e.code(), "FM-E001", "{pointer}");
        let m = e.to_string();
        assert!(m.contains(&format!("`{pointer}`")), "{pointer}: {m}");
        assert!(
            m.contains("leave the field out to use the default, or give a value"),
            "{pointer}: {m}"
        );
    }
}

/// Code review of #197: an empty list of lines is named as `lines`, not `flows`; flows and
/// lines both given without a projection say "not both".
#[test]
fn empty_lines_and_both_lists_name_their_fields() {
    let v = json!({"stages": [{"lines": []}], "wacc": "0.09"});
    let m = run("dcf.fcff", &v.to_string()).unwrap_err().to_string();
    assert!(m.contains("`/stages/0/lines` is empty"), "{m}");
    let v = json!({"stages": [{"flows": ["1"], "lines": []}], "wacc": "0.09"});
    let m = run("dcf.fcff", &v.to_string()).unwrap_err().to_string();
    assert!(m.contains("give `flows` or `lines`, not both"), "{m}");
}

/// Code review of #197: a terminal discounting time with the default timing says that the
/// timing was defaulted and how to fix it.
#[test]
fn defaulted_timing_is_named_in_the_message() {
    let mut v = minimal();
    v["terminal"] = json!({"given": {"value": "100"}});
    v["terminal_discount_time"] = json!("horizon");
    let m = run("dcf.fcff", &v.to_string()).unwrap_err().to_string();
    assert!(
        m.contains(
            "`/timing` was not given and defaults to end_of_period; give `timing: mid_period` \
             or remove `/terminal_discount_time`"
        ),
        "{m}"
    );
    v["timing"] = json!("end_of_period");
    let m = run("dcf.fcff", &v.to_string()).unwrap_err().to_string();
    assert!(!m.contains("was not given"), "{m}");
}

/// Code review of #197: the typed Rust call fills in and reports the defaults exactly as the
/// registry does, first in the conventions.
#[test]
fn typed_call_lists_the_defaults_first() {
    let input: fairmark_core::dcf::DcfFcffInput = serde_json::from_value(minimal()).unwrap();
    let (out, ex) = fairmark_core::dcf::fcff(&input).unwrap();
    assert!(ex.conventions[0].starts_with("default: `/stages/0/projection`"));
    assert!(ex.conventions[5].starts_with("default: `/terminal`"));
    assert!(!ex.conventions[6].starts_with("default: "));
    let registry = ok("dcf.fcff", &minimal());
    assert_eq!(conventions(&registry), ex.conventions);
    assert_eq!(
        serde_json::to_value(&out).unwrap(),
        registry["value"],
        "typed and registry values"
    );
}

/// Code review of #197: with no terminal value nothing is discounted, so the discounting time
/// and factor are `null`; a negative given value warns that the weight is not meaningful; the
/// message for a missing terminal rate says that the half-period shift uses it.
#[test]
fn terminal_option_details() {
    let out = ok("dcf.fcff", &minimal());
    assert!(out["value"]["terminal"]["discount_time"].is_null());
    assert!(out["value"]["terminal"]["discount_factor"].is_null());
    let mut v = minimal();
    v["terminal"] = json!({"given": {"value": "-300"}});
    let c = conventions(&ok("dcf.fcff", &v));
    assert!(
        c.iter()
            .any(|c| c.starts_with("warning: the terminal value is negative")
                && c.contains("terminal value weight is not meaningful")),
        "{c:?}"
    );
    v.as_object_mut().unwrap().remove("wacc");
    v["wacc_per_period"] = json!(["0.09", "0.09", "0.09", "0.09", "0.09"]);
    v["timing"] = json!("mid_period");
    v["terminal_discount_time"] = json!("mid_period");
    let m = run("dcf.fcff", &v.to_string()).unwrap_err().to_string();
    assert!(
        m.contains("(the half-period shift of the given terminal value uses it)"),
        "{m}"
    );
}

/// Code review of #197: two terminal options, one of them `given` or `none`, are named as
/// terminal options with the three choices.
#[test]
fn two_terminal_options_name_the_choices() {
    let mut v = minimal();
    v["terminal"] = json!({"given": {"value": "1"}, "none": {}});
    let e = run("dcf.fcff", &v.to_string()).unwrap_err();
    assert_eq!(e.code(), "FM-E001");
    let m = e.to_string();
    assert!(m.contains("names 2 terminal options (given, none)"), "{m}");
    assert!(
        m.contains("`given` (a terminal value at N) or `none`"),
        "{m}"
    );
}

/// Numerical audit of #197 (Decisions 17 and 24): a given terminal value discounted at
/// N − 0.5 carries the warning of an amount stated at a date.
#[test]
fn given_terminal_value_at_n_minus_half_warns() {
    let mut v = minimal();
    v["terminal"] = json!({"given": {"value": "1000"}});
    v["timing"] = json!("mid_period");
    v["terminal_discount_time"] = json!("mid_period");
    let c = conventions(&ok("dcf.fcff", &v));
    assert!(
        c.iter().any(|c| c
            == "warning: given: the analyst's terminal value at the horizon N; discounting it \
                at N − 0.5 brings it forward half a period; it is discounted at N − 0.5 as \
                chosen"),
        "{c:?}"
    );
    v["terminal_discount_time"] = json!("horizon");
    let c = conventions(&ok("dcf.fcff", &v));
    assert!(!c.iter().any(|c| c.contains("brings it forward")), "{c:?}");
}
