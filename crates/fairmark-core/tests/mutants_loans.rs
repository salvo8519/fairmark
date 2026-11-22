//! Mutants of `loans.rs` missed by the weekly mutation testing run of 2026-11-22 (run
//! 44972766259, shard 4; issue #368).

use std::str::FromStr;

use fairmark_core::run;
use rust_decimal::Decimal;
use serde_json::{json, Value};

/// The step of the explain whose label is `label`.
fn step<'a>(out: &'a Value, label: &str) -> &'a Value {
    out["explain"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .find(|s| s["label"] == label)
        .unwrap_or_else(|| panic!("no step {label:?}"))
}

/// `number_of_instalments`: when the logarithms give one instalment too few, the balance
/// recurrence adds it, and the final instalment is the amount owed on its payment date.
/// An instalment just above the interest on the principal (20,000 × 0.05 = 1,000) repays it
/// in 361 instalments, while n* = 359.99999999999999999998...: the recurrence goes up from
/// 360. Expected values: Python `decimal` at 28 digits, b_1 = 21,000 and
/// b_k = (b_(k−1) − a) × 1.05, first k with b_k − a ≤ 20,000 × 1e-20: k = 361, and b_361 of
/// the order of 1e-15 (Python 1.37e-15, the engine 2.96e-16). That amount is a residual at
/// the precision limit, multiplied by 1.05^361 (about 4.4e7) along the recurrence, so its
/// digits depend on the rounding of each step: only its order is checked.
#[test]
fn the_recurrence_adds_the_instalment_the_logarithms_miss() {
    let input = json!({"principal": "20000", "periodic_rate": "0.05", "timing": "ordinary",
                       "instalment": "1000.0000235424873127452507414"});
    let out = run("loans.number_of_instalments", &input.to_string()).unwrap();
    let v = &out["value"];
    assert_eq!(v["number_of_instalments"], 361);
    let last = Decimal::from_str(v["final_instalment"].as_str().unwrap()).unwrap();
    assert!(
        last > Decimal::ZERO && last < Decimal::from_str("1e-14").unwrap(),
        "{last}"
    );
    let formula = step(&out, "Final instalment (amount owed on its payment date)")["formula"]
        .as_str()
        .unwrap()
        .to_owned();
    assert!(
        formula.contains("for k = 2 to 361; final = b_361"),
        "{formula}"
    );
}

/// `amortisation_schedule`: the last period of a loan with a balloon amount opens with the
/// closing balance of the period before it; a loan of one period opens with the principal.
#[test]
fn the_balloon_period_opens_with_the_previous_closing_balance() {
    for (n, formula) in [(3, "closing balance of period 2"), (1, "the principal")] {
        let input = json!({"principal": "1000", "periodic_rate": "0.01",
                           "number_of_instalments": n, "timing": "ordinary",
                           "amortisation_type": "french", "balloon_amount": "100"});
        let out = run("loans.amortisation_schedule", &input.to_string()).unwrap();
        let label = format!("Period {n}: opening balance (balloon amount included)");
        assert_eq!(step(&out, &label)["formula"], formula, "{n}");
    }
}
