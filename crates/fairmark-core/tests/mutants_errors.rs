//! Mutants missed on the changed lines of #279 (CI run of 2026-11-10, job "Mutation testing ·
//! changed lines of the core"): the lower bounds of the checks are inclusive. Then the
//! mutants of `coc.rs` and `error.rs` missed by the weekly run of 2026-11-22 (issue #368).

use fairmark_core::error::Reason;
use fairmark_core::{run, FairmarkError};
use rust_decimal::Decimal;
use serde_json::json;

/// `coc.rs` `check_tax_rate`: a tax rate of exactly 0 is accepted (`t < 0` is refused, not
/// `t <= 0`), and -0.0001 is refused.
#[test]
fn a_tax_rate_of_zero_is_accepted() {
    let input = json!({"risk_free_rate": "0.03", "credit_spread": "0.025", "tax_rate": "0"});
    let r = run("coc.cost_of_debt", &input.to_string()).unwrap();
    assert_eq!(r["value"]["after_tax_cost_of_debt"], "0.055");
    let input = json!({"risk_free_rate": "0.03", "credit_spread": "0.025", "tax_rate": "-0.0001"});
    let e = run("coc.cost_of_debt", &input.to_string()).unwrap_err();
    assert_eq!(e.details().pointer.as_deref(), Some("/tax_rate"));
}

/// `coc.rs` `check_cost_of_equity`: a country risk premium of exactly 0 is accepted
/// (`crp < 0` is refused, not `crp <= 0`), and a negative one is refused.
#[test]
fn a_country_risk_premium_of_zero_is_accepted() {
    let base = json!({"risk_free_rate": "0.03", "levered_beta": "1.2",
                      "equity_risk_premium": "0.05", "country_risk_premium": "0"});
    let r = run("coc.cost_of_equity", &base.to_string()).unwrap();
    assert_eq!(r["value"]["cost_of_equity"], "0.090");
    let mut negative = base;
    negative["country_risk_premium"] = json!("-0.01");
    let e = run("coc.cost_of_equity", &negative.to_string()).unwrap_err();
    assert_eq!(
        e.details().pointer.as_deref(),
        Some("/country_risk_premium")
    );
}

/// `coc.rs` `check_cost_of_equity`: a country exposure of exactly 0 is accepted (`lambda < 0`
/// is refused, not `lambda <= 0`): the country risk premium then adds nothing.
#[test]
fn a_country_exposure_of_zero_is_accepted() {
    let base = json!({"risk_free_rate": "0.03", "levered_beta": "1.2",
                      "equity_risk_premium": "0.05", "country_risk_premium": "0.02",
                      "country_exposure": "0"});
    let r = run("coc.cost_of_equity", &base.to_string()).unwrap();
    assert_eq!(r["value"]["cost_of_equity"], "0.090");
    let mut negative = base;
    negative["country_exposure"] = json!("-0.5");
    let e = run("coc.cost_of_equity", &negative.to_string()).unwrap_err();
    assert_eq!(e.details().pointer.as_deref(), Some("/country_exposure"));
}

// Mutants missed by the weekly run of 2026-11-22 (run 44972766259, shard 4; issue #368).

/// `coc.rs` `wacc`: a debt weight below 0 is refused as well as one of 1 or more (`w < 0 ||
/// w >= 1`, not `&&`), at `/debt_weight`, with the range of the field.
#[test]
fn a_debt_weight_below_zero_or_from_one_is_refused_at_the_field() {
    for w in ["-0.1", "1", "1.5"] {
        let input = json!({"cost_of_equity": "0.10", "pre_tax_cost_of_debt": "0.06",
                           "tax_rate": "0.25", "debt_weight": w});
        let e = run("coc.wacc", &input.to_string()).unwrap_err();
        let d = e.details();
        assert_eq!(d.pointer.as_deref(), Some("/debt_weight"), "{w}: {e}");
        assert_eq!(
            d.expected,
            ["a decimal fraction at least 0 and below 1"],
            "{w}: {e}"
        );
    }
    let input = json!({"cost_of_equity": "0.10", "pre_tax_cost_of_debt": "0.06",
                       "tax_rate": "0.25", "debt_weight": "0"});
    let r = run("coc.wacc", &input.to_string()).unwrap();
    assert_eq!(r["value"]["wacc"], "0.10");
}

/// `error.rs` `FairmarkError::details`: a discount rate converted with an expected inflation
/// names both fields in `pointers`, the rate first.
#[test]
fn a_converted_discount_rate_names_the_rate_and_the_inflation() {
    let e = FairmarkError::DiscountRateOutOfRange {
        field: "/discount_rate".to_owned(),
        rate: Decimal::NEGATIVE_ONE,
        converted_with: Some("/expected_inflation".into()),
    };
    let d = e.details();
    assert_eq!(d.pointer.as_deref(), Some("/discount_rate"));
    assert_eq!(d.pointers, ["/discount_rate", "/expected_inflation"]);
}

/// `error.rs` `FairmarkError::details`: a growth not below the terminal rate is a `range`
/// error.
#[test]
fn a_growth_not_below_the_terminal_rate_is_a_range_error() {
    let e = FairmarkError::GrowthNotBelowTerminalRate {
        growth_field: "/terminal/growth".to_owned(),
        growth: Decimal::new(5, 2),
        rate_field: "/discount_rate".to_owned(),
        rate: Decimal::new(4, 2),
        converted_with: None,
    };
    let d = e.details();
    assert_eq!(d.reason, Some(Reason::Range));
    assert_eq!(d.pointers, ["/terminal/growth", "/discount_rate"]);
}

/// `error.rs` `FairmarkError::details`: an invalid date states the accepted form.
#[test]
fn an_invalid_date_states_the_iso_form() {
    let d = FairmarkError::InvalidDate("2026-02-30".to_owned()).details();
    assert_eq!(d.reason, Some(Reason::Type));
    assert_eq!(d.expected, ["an ISO 8601 calendar date, YYYY-MM-DD"]);
}

/// `error.rs` `FairmarkError::details`: a count mismatch is a `length` error.
#[test]
fn a_count_mismatch_is_a_length_error() {
    let d = FairmarkError::CountMismatch("3 rates for 4 flows".to_owned()).details();
    assert_eq!(d.reason, Some(Reason::Length));
}
