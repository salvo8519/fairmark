//! A decimal with more digits than a `Decimal` holds (more than 28 decimal places, or more than
//! 28 to 29 significant digits) is accepted, rounded as the input is read, and reported: a
//! convention starting with `warning: ` names the field by its JSON pointer, the value given
//! and the value used (maintainer decision of 2026-11-10 on #279, ADR 0014 G7).

use fairmark_core::run;
use serde_json::{json, Value};

/// Thirty decimal places, twenty-eight significant digits: rounded to 28 places.
const LONG: &str = "0.009166666666666666666666666667";
const USED: &str = "0.0091666666666666666666666667";

fn conventions(result: &Value) -> Vec<String> {
    result["explain"]["conventions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_owned())
        .collect()
}

fn warnings(result: &Value) -> Vec<String> {
    conventions(result)
        .into_iter()
        .filter(|c| c.starts_with("warning: ") && c.contains("more digits than a decimal holds"))
        .collect()
}

fn future_value(rate: &str) -> String {
    format!(r#"{{"payment": "2000", "rate": {rate}, "periods": 35, "timing": "due"}}"#)
}

#[test]
fn a_long_decimal_string_is_rounded_and_reported() {
    let result = run("tvm.future_value", &future_value(&format!("\"{LONG}\""))).unwrap();
    let w = warnings(&result);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].starts_with("warning: `/rate` "), "{}", w[0]);
    assert!(w[0].contains(&format!("given {LONG}")), "{}", w[0]);
    assert!(w[0].contains(&format!("used {USED}")), "{}", w[0]);
    assert!(w[0].contains("half away from zero"), "{}", w[0]);
    // The value is the one computed with the rounded rate, bit for bit.
    let rounded = run("tvm.future_value", &future_value(&format!("\"{USED}\""))).unwrap();
    assert_eq!(result["value"], rounded["value"]);
    assert_eq!(result["meta"]["input_hash"], rounded["meta"]["input_hash"]);
}

#[test]
fn a_long_json_number_is_rounded_and_reported() {
    let result = run("tvm.future_value", &future_value(LONG)).unwrap();
    let w = warnings(&result);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].starts_with("warning: `/rate` "), "{}", w[0]);
    assert!(w[0].contains(&format!("given {LONG}")), "{}", w[0]);
    assert!(w[0].contains(&format!("used {USED}")), "{}", w[0]);
    let rounded = run("tvm.future_value", &future_value(USED)).unwrap();
    assert_eq!(result["value"], rounded["value"]);
}

#[test]
fn a_decimal_that_fits_has_no_warning() {
    // Exactly 28 decimal places, as a string and as a JSON number; trailing zeros beyond 28
    // places change nothing either.
    for rate in [
        format!("\"{USED}\""),
        USED.to_owned(),
        "\"0.0091666666666666666666666667000\"".to_owned(),
        "\"0.07\"".to_owned(),
    ] {
        let result = run("tvm.future_value", &future_value(&rate)).unwrap();
        assert!(
            warnings(&result).is_empty(),
            "{rate}: {:?}",
            conventions(&result)
        );
    }
}

#[test]
fn the_warning_names_the_pointer_inside_an_array() {
    let input = json!({
        "rate": "0.07",
        "cash_flows": [-5000, 1200, "1500.00000000000000000000000049", 1800, 2100]
    });
    let result = run("tvm.npv", &input.to_string()).unwrap();
    let w = warnings(&result);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].starts_with("warning: `/cash_flows/2` "), "{}", w[0]);
    assert!(
        w[0].contains("given 1500.00000000000000000000000049"),
        "{}",
        w[0]
    );
    assert!(
        w[0].contains("used 1500.0000000000000000000000005"),
        "{}",
        w[0]
    );
}

#[test]
fn rounding_is_half_away_from_zero() {
    // The first digit that does not fit decides: 5 or more rounds away from zero.
    let cases = [
        (
            "0.00000000000000000000000000015",
            "0.0000000000000000000000000002",
        ),
        (
            "-0.00000000000000000000000000015",
            "-0.0000000000000000000000000002",
        ),
        (
            "0.00000000000000000000000000014",
            "0.0000000000000000000000000001",
        ),
    ];
    for (given, used) in cases {
        let input = json!({"rate": "0.07", "cash_flows": [-1, given, 2]});
        let result = run("tvm.npv", &input.to_string()).unwrap();
        let w = warnings(&result);
        assert_eq!(w.len(), 1, "{given}: {w:?}");
        assert!(
            w[0].contains(&format!("given {given}, used {used}")),
            "{}",
            w[0]
        );
    }
}

#[test]
fn an_analysis_names_the_pointer_under_base_input() {
    let input = json!({
        "method": "tvm.future_value",
        "base_input": {"payment": "2000", "rate": LONG, "periods": 35, "timing": "due"},
        "output": "/future_value",
        "rows": {"variable": "/payment", "values": ["1000", "2000"]}
    });
    let result = run("analysis.sensitivity", &input.to_string()).unwrap();
    let w = warnings(&result);
    assert_eq!(w.len(), 1, "{w:?}");
    assert!(w[0].starts_with("warning: `/base_input/rate` "), "{}", w[0]);
}

/// `meta.input_hash` is computed, as before this rule, on the input after reading (with its
/// defaults filled in): a long decimal hashes as the value used, not as the text given.
#[test]
fn the_input_hash_is_computed_on_the_value_used() {
    let long = run("tvm.future_value", &future_value(&format!("\"{LONG}\""))).unwrap();
    let used = run("tvm.future_value", &future_value(&format!("\"{USED}\""))).unwrap();
    let number = run("tvm.future_value", &future_value(LONG)).unwrap();
    assert_eq!(long["meta"]["input_hash"], used["meta"]["input_hash"]);
    assert_eq!(number["meta"]["input_hash"], used["meta"]["input_hash"]);
}

/// A value beyond the range of a decimal (too many digits before the point) is not rounded: it
/// stays a structured error at its field, as a string and as a JSON number.
#[test]
fn a_value_beyond_the_decimal_range_is_refused() {
    let huge = "123456789012345678901234567890";
    for rate in [
        format!("\"{huge}\""),
        huge.to_owned(),
        format!("\"{huge}.5\""),
    ] {
        let err = run("tvm.future_value", &future_value(&rate)).expect_err(&rate);
        let report = serde_json::to_value(err.to_report()).unwrap();
        assert_eq!(report["code"], "FM-E001", "{rate}: {report:#}");
        assert_eq!(report["pointer"], "/rate", "{rate}: {report:#}");
    }
}

// ------------------------------------------------- round trip of the engine's own numbers

/// Every decimal of a result, with its pointer in `value`.
fn decimals(v: &Value, at: String, out: &mut Vec<(String, String)>) {
    match v {
        Value::Object(m) => m
            .iter()
            .for_each(|(k, x)| decimals(x, format!("{at}/{k}"), out)),
        Value::Array(a) => a
            .iter()
            .enumerate()
            .for_each(|(i, x)| decimals(x, format!("{at}/{i}"), out)),
        Value::String(s) if s.parse::<rust_decimal::Decimal>().is_ok() => out.push((at, s.clone())),
        _ => {}
    }
}

/// A number Fairmark outputs, fed back as an input (an agent chaining methods), is read as it
/// is written: no rounding warning, the same value. A `Decimal` never prints more digits than
/// it holds, so the round trip is exact by construction; this pins it on results with 28
/// decimal places.
#[test]
fn numbers_the_engine_outputs_are_read_back_unchanged() {
    let sources = [
        (
            "tvm.irr",
            json!({"cash_flows": [-5000, 1200, 1500, 1800, 2100]}),
        ),
        (
            "tvm.xirr",
            json!({"cash_flows": [
                {"date": "2008-01-01", "amount": "-10000"}, {"date": "2008-03-01", "amount": "2750"},
                {"date": "2008-10-30", "amount": "4250"}, {"date": "2009-02-15", "amount": "3250"},
                {"date": "2009-04-01", "amount": "2750"}]}),
        ),
        (
            "dcf.fcff",
            json!({"stages": [{"periods": 3, "projection": "given", "flows": [100, 110, 121]}],
                   "wacc": "0.0917", "terminal": {"tv.gordon": {"growth": "0.0213", "first_flow": "last_flow_grown"}}}),
        ),
    ];
    let mut long = 0;
    for (id, input) in sources {
        let result = run(id, &input.to_string()).unwrap();
        let mut found = Vec::new();
        decimals(&result["value"], String::new(), &mut found);
        assert!(!found.is_empty(), "{id}");
        for (at, text) in found {
            if text.split_once('.').is_some_and(|(_, f)| f.len() >= 20) {
                long += 1;
            }
            // As a rate (a string and a JSON number) and as a cash flow.
            for rate in [
                Value::from(text.as_str()),
                serde_json::from_str(&text).unwrap(),
            ] {
                let npv = json!({"rate": rate, "cash_flows": [-1, text.as_str(), 2]});
                let r = run("tvm.npv", &npv.to_string());
                let Ok(r) = r else { continue }; // a rate out of range: not this test
                assert!(
                    warnings(&r).is_empty(),
                    "{id} {at} = {text}: {:?}",
                    warnings(&r)
                );
            }
            let used: rust_decimal::Decimal = text.parse().unwrap();
            assert_eq!(used.to_string(), text, "{id} {at}");
        }
    }
    assert!(long >= 3, "results with long decimals: {long}");
}
