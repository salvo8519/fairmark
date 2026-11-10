//! Mutants missed on the changed lines of #279 (CI run of 2026-11-10, job "Mutation testing ·
//! changed lines of the core"): the lower bounds of the checks are inclusive.

use fairmark_core::run;
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
