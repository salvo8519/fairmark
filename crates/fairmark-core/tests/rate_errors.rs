//! Every `FM-E003` names the rate by its JSON pointer and states the rule that applies to it
//! (issue #174, SPEC-0001 R27): one plus the rate per period, or per compounding period, must
//! be positive.

use fairmark_core::run;
use serde_json::{json, Value};

/// Asserts `FM-E003`, the pointer and every part in the message.
fn assert_rate_error(method: &str, input: &Value, pointer: &str, parts: &[&str]) {
    let err = run(method, &input.to_string()).expect_err("the rate must be refused");
    let message = err.to_string();
    assert_eq!(err.code(), "FM-E003", "{message}");
    assert!(message.contains(&format!("`{pointer}`")), "{message}");
    assert!(message.contains("10% is 0.10"), "{message}");
    for part in parts {
        assert!(message.contains(part), "missing {part:?} in: {message}");
    }
}

fn dated_flows() -> Value {
    json!([
        {"date": "2024-01-01", "amount": -100},
        {"date": "2025-01-01", "amount": 110}
    ])
}

#[test]
fn rates_convert_names_the_rate_and_the_compounding_rule() {
    assert_rate_error(
        "rates.convert",
        &json!({"rate": "-1", "from": "annual", "to": "monthly"}),
        "/rate",
        &["is -1", "greater than -1 (-100%)"],
    );
    assert_rate_error(
        "rates.convert",
        &json!({"rate": "-12", "from": "monthly", "to": "annual"}),
        "/rate",
        &[
            "is -12",
            "12 compounding periods a year",
            "-12 / 12",
            "greater than -12",
        ],
    );
    // Inside the domain of monthly compounding: accepted.
    run(
        "rates.convert",
        &json!({"rate": "-1.5", "from": "monthly", "to": "annual"}).to_string(),
    )
    .unwrap();
}

#[test]
fn xnpv_names_the_rate() {
    assert_rate_error(
        "tvm.xnpv",
        &json!({"rate": "-1", "cash_flows": dated_flows()}),
        "/rate",
        &["is -1"],
    );
    assert_rate_error(
        "tvm.xnpv",
        &json!({"rate": "-4", "compounding": "quarterly", "cash_flows": dated_flows()}),
        "/rate",
        &["4 compounding periods a year", "greater than -4"],
    );
}

#[test]
fn loans_name_the_rate_of_either_form() {
    for method in [
        "loans.instalment",
        "loans.amortisation_schedule",
        "loans.number_of_instalments",
    ] {
        let size = if method == "loans.number_of_instalments" {
            ("instalment", json!("100"))
        } else {
            ("number_of_instalments", json!(12))
        };
        let mut form_b = json!({"principal": "1000", "periodic_rate": "-1", "timing": "ordinary"});
        form_b[size.0] = size.1.clone();
        if method == "loans.amortisation_schedule" {
            form_b["amortisation_type"] = json!("french");
        }
        assert_rate_error(method, &form_b, "/periodic_rate", &["is -1"]);

        let mut form_a = json!({
            "principal": "1000", "rate": "-30", "compounding": "monthly",
            "payment_frequency": "monthly", "timing": "ordinary"
        });
        form_a[size.0] = size.1;
        if method == "loans.amortisation_schedule" {
            form_a["amortisation_type"] = json!("french");
        }
        assert_rate_error(
            method,
            &form_a,
            "/rate",
            &["is -30", "12 compounding periods a year"],
        );
    }
}

#[test]
fn debt_service_names_the_discount_rate() {
    let input = json!({
        "balances": "closing",
        "ebitda_definition": "EBITDA as reported",
        "interest_definition": "gross interest expense",
        "income_statement": {"ebitda": "800", "ebit": "500", "interest": "100"},
        "closing": {"net_financial_position": {"gross_debt": "2200", "cash": "300"}},
        "debt_schedule": {
            "discount_rate": "-1",
            "debt_outstanding": "1000",
            "periods": [{"cfads": "250", "debt_service": "200"}]
        }
    });
    assert_rate_error(
        "ratios.debt_service",
        &input,
        "/debt_schedule/discount_rate",
        &["is -1"],
    );
}

#[test]
fn limited_horizon_names_the_rate() {
    assert_rate_error(
        "tv.limited_horizon",
        &json!({"rate": "-1", "flow": "100", "periods": 5}),
        "/rate",
        &["is -1"],
    );
}
