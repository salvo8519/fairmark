//! Errors raised while reading the input name the field by its JSON pointer (issue #174,
//! SPEC-0001 R27). The code never changes: a malformed input is `FM-E001`, an invalid date
//! `FM-E009`.

use fairmark_core::run;
use serde_json::{json, Value};

/// Runs `method` on `input` and returns `(code, message)` of the error it must raise.
fn error(method: &str, input: &Value) -> (String, String) {
    let err = run(method, &input.to_string()).expect_err("the input must be refused");
    (err.code().to_owned(), err.to_string())
}

/// Asserts the code and that the message names `pointer` and contains every `parts`.
fn assert_error(method: &str, input: &Value, code: &str, pointer: &str, parts: &[&str]) {
    let (got, message) = error(method, input);
    assert_eq!(got, code, "{message}");
    assert!(
        message.contains(&format!("`{pointer}`")),
        "the message should name `{pointer}`: {message}"
    );
    for part in parts {
        assert!(message.contains(part), "missing {part:?} in: {message}");
    }
    assert!(
        !message.contains(" at line "),
        "a data error gives the pointer, not the line: {message}"
    );
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

fn covenants() -> Value {
    let covenant = json!({
        "name": "leverage",
        "test_date": "2026-12-31",
        "numerator": {"definition": "net financial debt", "amount": "1750"},
        "denominator": {"definition": "EBITDA", "amount": "500"},
        "operator": "at_most",
        "threshold": "3.5",
        "if_denominator_not_positive": "not_tested"
    });
    json!({"covenants": [covenant.clone(), covenant]})
}

#[test]
fn the_fixtures_are_valid() {
    run("dcf.fcff", &dcf().to_string()).unwrap();
    run("ratios.covenants", &covenants().to_string()).unwrap();
}

// ---------------------------------------------------------------- dcf.fcff

#[test]
fn dcf_missing_field_inside_a_tagged_technique() {
    let mut input = dcf();
    input["terminal"]["tv.gordon"]
        .as_object_mut()
        .unwrap()
        .remove("first_flow");
    assert_error(
        "dcf.fcff",
        &input,
        "FM-E001",
        "/terminal/tv.gordon/first_flow",
        &["missing required field `first_flow`", "add it"],
    );
}

#[test]
fn dcf_unknown_field_inside_an_array() {
    let mut input = dcf();
    input["stages"][0]["flowz"] = json!([1]);
    assert_error(
        "dcf.fcff",
        &input,
        "FM-E001",
        "/stages/0/flowz",
        &[
            "unknown field `flowz`",
            "expected one of",
            "`flows`",
            "remove or rename it",
        ],
    );
}

#[test]
fn dcf_wrong_enum_variant() {
    let mut input = dcf();
    input["stages"][0]["projection"] = json!("constant");
    assert_error(
        "dcf.fcff",
        &input,
        "FM-E001",
        "/stages/0/projection",
        &["unknown variant `constant`", "`constant_growth`"],
    );
    let mut input = dcf();
    input["timing"] = json!("end");
    assert_error(
        "dcf.fcff",
        &input,
        "FM-E001",
        "/timing",
        &["unknown variant `end`", "`end_of_period`"],
    );
}

#[test]
fn dcf_wrong_type_with_an_index() {
    let mut input = dcf();
    input["stages"][0]["flows"][1] = json!(true);
    assert_error(
        "dcf.fcff",
        &input,
        "FM-E001",
        "/stages/0/flows/1",
        &["invalid type: boolean `true`"],
    );
}

#[test]
fn dcf_unknown_terminal_technique_is_an_unknown_field() {
    let mut input = dcf();
    input["terminal"] = json!({"tv.gordo": {"growth": "0.02"}});
    assert_error(
        "dcf.fcff",
        &input,
        "FM-E001",
        "/terminal/tv.gordo",
        &["unknown field `tv.gordo`", "`tv.gordon`"],
    );
}

// ---------------------------------------------------------------- tvm.irr and tvm.xirr

#[test]
fn irr_missing_field_at_the_top_level() {
    assert_error(
        "tvm.irr",
        &json!({"guess": "0.1"}),
        "FM-E001",
        "/cash_flows",
        &["missing required field `cash_flows`"],
    );
}

#[test]
fn irr_unknown_field() {
    assert_error(
        "tvm.irr",
        &json!({"cash_flows": [-100, 110], "gues": "0.1"}),
        "FM-E001",
        "/gues",
        &["unknown field `gues`", "`guess`"],
    );
}

#[test]
fn irr_wrong_type_with_an_index() {
    assert_error(
        "tvm.irr",
        &json!({"cash_flows": [-100, 60, {"amount": 60}]}),
        "FM-E001",
        "/cash_flows/2",
        &["invalid type: map"],
    );
    assert_error(
        "tvm.irr",
        &json!({"cash_flows": [-100, 60, "ten"]}),
        "FM-E001",
        "/cash_flows/2",
        &[],
    );
}

#[test]
fn a_json_number_out_of_range_names_its_own_pointer() {
    // Numbers keep their exact text (`arbitrary_precision`): the private key serde_json uses
    // for them must not appear in the pointer.
    let err = run("tvm.irr", r#"{"cash_flows": [-100, 1e400]}"#).unwrap_err();
    let message = err.to_string();
    assert_eq!(err.code(), "FM-E001", "{message}");
    assert!(message.contains("`/cash_flows/1`"), "{message}");
    assert!(!message.contains("serde_json"), "{message}");
}

#[test]
fn xirr_wrong_enum_and_invalid_date() {
    let flows = json!([
        {"date": "2024-01-01", "amount": -100},
        {"date": "2025-01-01", "amount": 110}
    ]);
    assert_error(
        "tvm.xirr",
        &json!({"cash_flows": flows, "day_count": "act_365"}),
        "FM-E001",
        "/day_count",
        &["unknown variant `act_365`", "`act_365_fixed`"],
    );
    let mut bad = flows.clone();
    bad[1]["date"] = json!("2023-02-29");
    assert_error(
        "tvm.xirr",
        &json!({"cash_flows": bad}),
        "FM-E009",
        "/cash_flows/1/date",
        &["2023-02-29"],
    );
    let mut missing = flows;
    missing[0].as_object_mut().unwrap().remove("amount");
    assert_error(
        "tvm.xirr",
        &json!({"cash_flows": missing}),
        "FM-E001",
        "/cash_flows/0/amount",
        &["missing required field `amount`"],
    );
}

// ---------------------------------------------------------------- ratios

#[test]
fn covenants_errors_inside_the_list() {
    let mut input = covenants();
    input["covenants"][1]["operator"] = json!("at_least_or_equal");
    assert_error(
        "ratios.covenants",
        &input,
        "FM-E001",
        "/covenants/1/operator",
        &["unknown variant `at_least_or_equal`", "`at_least`"],
    );

    let mut input = covenants();
    input["covenants"][0]
        .as_object_mut()
        .unwrap()
        .remove("threshold");
    assert_error(
        "ratios.covenants",
        &input,
        "FM-E001",
        "/covenants/0/threshold",
        &["missing required field `threshold`"],
    );

    let mut input = covenants();
    input["covenants"][1]["numerator"]["amnt"] = json!("1");
    assert_error(
        "ratios.covenants",
        &input,
        "FM-E001",
        "/covenants/1/numerator/amnt",
        &["unknown field `amnt`", "`amount`"],
    );

    let mut input = covenants();
    input["covenants"][0]["threshold"] = json!([3.5]);
    assert_error(
        "ratios.covenants",
        &input,
        "FM-E001",
        "/covenants/0/threshold",
        &["invalid type: sequence"],
    );

    let mut input = covenants();
    input["covenants"][1]["test_date"] = json!("31/12/2026");
    assert_error(
        "ratios.covenants",
        &input,
        "FM-E009",
        "/covenants/1/test_date",
        &["YYYY-MM-DD"],
    );
}

#[test]
fn liquidity_wrong_enum_and_nested_missing_field() {
    let closing = json!({
        "current_assets": "2500", "inventories": "900", "cash": "200",
        "current_liabilities": "2000"
    });
    assert_error(
        "ratios.liquidity",
        &json!({"balances": "closing_balances", "closing": closing}),
        "FM-E001",
        "/balances",
        &[
            "unknown variant `closing_balances`",
            "`closing`",
            "`average`",
        ],
    );
    let mut partial = closing;
    partial.as_object_mut().unwrap().remove("cash");
    assert_error(
        "ratios.liquidity",
        &json!({"balances": "closing", "closing": partial}),
        "FM-E001",
        "/closing/cash",
        &["missing required field `cash`"],
    );
}

// ---------------------------------------------------------------- tv.compare

#[test]
fn compare_errors_inside_a_technique_entry() {
    let input = json!({
        "rate": "0.09",
        "basis": "enterprise",
        "techniques": [
            {"tv.gordon": {"growth": "0.02", "next_flow": "100"}},
            {"tv.gordon": {"growth": "0.02", "next_flow": "abc"}}
        ]
    });
    assert_error(
        "tv.compare",
        &input,
        "FM-E001",
        "/techniques/1/tv.gordon/next_flow",
        &[],
    );
    let two_keys = json!({
        "rate": "0.09",
        "basis": "enterprise",
        "techniques": [{"tv.gordon": {"growth": "0.02", "next_flow": "100"}, "tv.h_model": {}}]
    });
    assert_error(
        "tv.compare",
        &two_keys,
        "FM-E001",
        "/techniques/0",
        &["exactly one key", "got 2 keys: tv.gordon, tv.h_model"],
    );
    let unknown = json!({
        "rate": "0.09",
        "basis": "enterprise",
        "techniques": [{"tv.gordo": {"growth": "0.02"}}]
    });
    // The unknown key is named as in `dcf.fcff` (`/terminal/tv.gordo`).
    assert_error(
        "tv.compare",
        &unknown,
        "FM-E001",
        "/techniques/0/tv.gordo",
        &["unknown variant `tv.gordo`", "`tv.gordon`"],
    );
    let missing = json!({
        "rate": "0.09",
        "basis": "enterprise",
        "techniques": [{"tv.gordon": {"growth": "0.02", "next_flow": "1"}},
                       {"tv.value_driver": {"growth": "0.02", "ronic": "0.1"}}]
    });
    assert_error(
        "tv.compare",
        &missing,
        "FM-E001",
        "/techniques/1/tv.value_driver/nopat_next",
        &["missing required field `nopat_next`"],
    );
}

/// `tv.compare` is available: reading entries in one pass must not change which inputs it
/// accepts. A key repeated in one entry is accepted and the last occurrence wins, even when an
/// earlier one is invalid; with two distinct keys the entry is refused before any terms are
/// read, so an error inside the first terms does not hide the "one key" message.
#[test]
fn compare_entries_keep_their_behaviour() {
    let text = |entry: &str| {
        format!(r#"{{"rate": "0.09", "basis": "enterprise", "techniques": [{entry}]}}"#)
    };
    let last = run(
        "tv.compare",
        &text(r#"{"tv.gordon": {"growth": "x"}, "tv.gordon": {"growth": "0.02", "next_flow": "100"}}"#),
    )
    .expect("a repeated key is accepted, the last one wins");
    let once = run(
        "tv.compare",
        &text(r#"{"tv.gordon": {"growth": "0.02", "next_flow": "100"}}"#),
    )
    .unwrap();
    assert_eq!(last["value"], once["value"]);
    assert_eq!(last["meta"]["input_hash"], once["meta"]["input_hash"]);

    let err = run(
        "tv.compare",
        &text(r#"{"tv.gordon": {"growth": "x"}, "tv.h_model": {}}"#),
    )
    .unwrap_err();
    assert_eq!(err.code(), "FM-E001");
    let m = err.to_string();
    assert!(m.contains("`/techniques/0`"), "{m}");
    assert!(m.contains("got 2 keys: tv.gordon, tv.h_model"), "{m}");
}

#[test]
fn a_repeated_field_is_named() {
    // serde refuses a repeated struct field (as before); the message now names it.
    let err = run(
        "tvm.irr",
        r#"{"cash_flows": [-100, 110], "guess": "0.1", "guess": "0.2"}"#,
    )
    .unwrap_err();
    let m = err.to_string();
    assert_eq!(err.code(), "FM-E001", "{m}");
    assert!(m.contains("`/guess`"), "{m}");
    assert!(m.contains("appears more than once; keep one"), "{m}");
    let err = run(
        "ratios.covenants",
        &covenants().to_string().replacen(
            r#""operator":"at_most""#,
            r#""operator":"at_most","operator":"at_most""#,
            1,
        ),
    )
    .unwrap_err();
    let m = err.to_string();
    assert!(m.contains("`/covenants/0/operator`"), "{m}");
}

#[test]
fn rust_type_names_are_not_shown() {
    // An object is expected where a string is given: no Rust struct name in the message.
    let mut input = dcf();
    input["stages"][0] = json!("given");
    let (code, m) = error("dcf.fcff", &input);
    assert_eq!(code, "FM-E001");
    assert!(m.contains("`/stages/0`"), "{m}");
    assert!(m.contains("expected an object"), "{m}");
    assert!(!m.contains("struct"), "{m}");
}

#[test]
fn every_decimal_error_uses_the_same_wording() {
    const DECIMAL: &str = r#"expected a decimal number (a JSON number or a string such as "0.10")"#;
    for (value, kind) in [
        (json!(true), "invalid type: boolean `true`"),
        (json!("ten"), "invalid value: string \"ten\""),
        (json!({}), "invalid type: map"),
        (json!({"amount": 1}), "invalid type: map"),
        (json!(null), "invalid type: null"),
    ] {
        let (code, m) = error("tvm.irr", &json!({"cash_flows": [-100, value]}));
        assert_eq!(code, "FM-E001", "{m}");
        assert!(m.contains("`/cash_flows/1`"), "{m}");
        assert!(m.contains(kind), "{kind}: {m}");
        assert!(m.contains(DECIMAL), "{m}");
        assert!(!m.contains("fixed-point"), "{m}");
    }
    // Optional decimals use another visitor in rust_decimal: same wording.
    let (_, m) = error(
        "tvm.irr",
        &json!({"cash_flows": [-100, 110], "guess": true}),
    );
    assert!(m.contains("`/guess`") && m.contains(DECIMAL), "{m}");
}

// ---------------------------------------------------------------- pointers and syntax

#[test]
fn pointer_escapes_slash_and_tilde() {
    assert_error(
        "tvm.irr",
        &json!({"cash_flows": [-100, 110], "a/b~c": 1}),
        "FM-E001",
        "/a~1b~0c",
        &["unknown field `a/b~c`"],
    );
}

#[test]
fn a_root_of_the_wrong_type_has_no_pointer() {
    let err = run("tvm.irr", r#""cash flows""#).unwrap_err();
    assert_eq!(err.code(), "FM-E001");
    assert!(
        err.to_string().starts_with("invalid input: invalid type"),
        "{err}"
    );
}

#[test]
fn syntax_errors_keep_the_line_and_column() {
    let err = run("tvm.irr", r#"{"cash_flows": [-100, 110],}"#).unwrap_err();
    assert_eq!(err.code(), "FM-E001");
    assert!(err.to_string().contains("line 1 column"), "{err}");
    let err = run("tvm.irr", r#"{"cash_flows": [-100, 110]} x"#).unwrap_err();
    assert_eq!(err.code(), "FM-E001");
    assert!(err.to_string().contains("trailing characters"), "{err}");
}
