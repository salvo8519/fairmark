//! Mutants of `reading.rs` missed by the weekly mutation testing run of 2026-11-22 (run
//! 44972766259, shard 4; issue #368).

use fairmark_core::run;
use serde_json::json;

/// The message of the error `run` reports for a risk-free rate written as `rate`.
fn message(rate: &str) -> String {
    let input = json!({"risk_free_rate": rate, "credit_spread": "0.025", "tax_rate": "0.2"});
    run("coc.cost_of_debt", &input.to_string())
        .unwrap_err()
        .to_string()
}

/// `plain_digits`: a decimal written with an exponent is quoted in its plain digits when it
/// has at most 29 digits before the point, and as written beyond (`width − scale > 29`): no
/// decimal holds it, so the message quotes what the analyst gave.
#[test]
fn an_exponent_beyond_29_digits_is_quoted_as_written() {
    let at_29 = message("9.9e28");
    assert!(
        at_29.contains("string \"99000000000000000000000000000\""),
        "{at_29}"
    );
    let at_30 = message("1.5e29");
    assert!(at_30.contains("string \"1.5e29\""), "{at_30}");
}

/// `plain_digits`: an exponent is expanded only while the plain form has at most 120
/// characters (`scale.abs() + width > PLAIN_DIGITS_MAX`). `1e-119` (one digit, scale 119) is
/// read and rounded to the 28 decimal places a decimal holds, with warning `FM-W001`;
/// `1e-120` is refused and quoted as written. Found beside the mutants of #368 on the same
/// line.
#[test]
fn an_exponent_is_expanded_up_to_120_characters() {
    let read = |rate: &str| {
        let input = json!({"risk_free_rate": rate, "credit_spread": "0.025", "tax_rate": "0.2"});
        run("coc.cost_of_debt", &input.to_string())
    };
    let r = read("1e-119").unwrap();
    let warnings = r["diagnostics"]["warnings"].as_array().unwrap();
    assert!(
        warnings
            .iter()
            .any(|w| w["code"] == "FM-W001" && w["pointer"] == "/risk_free_rate"),
        "{warnings:?}"
    );
    let refused = message("1e-120");
    assert!(refused.contains("string \"1e-120\""), "{refused}");
}
