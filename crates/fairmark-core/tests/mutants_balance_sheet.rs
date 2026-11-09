//! Behaviour tests for the balance sheet of `statements.reorganise` (`statements/balance_sheet.rs`)
//! written to kill the missed mutants of the first mutation-testing baseline (issue #282,
//! `tests/mutants/triage.md`). Each test cites the requirement of SPEC-0005 it checks and
//! starts from a reviewed golden company, changing only the input under test; expected values
//! are the rules of the spec (origins, checks, not-computed outputs, explain steps), not
//! numbers copied from the engine.

use std::path::Path;

use serde_json::{json, Value};

const METHOD: &str = "statements.reorganise";

fn golden_input(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/statements.reorganise")
        .join(format!("{name}.yaml"));
    let case: Value = serde_norway::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    case["input"].clone()
}

fn ok(input: &Value) -> Value {
    fairmark_core::run(METHOD, &input.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

fn conventions(out: &Value) -> Vec<String> {
    serde_json::from_value(out["explain"]["conventions"].clone()).unwrap()
}

fn step_labels(out: &Value) -> Vec<String> {
    out["explain"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["label"].as_str().unwrap().to_owned())
        .collect()
}

/// SPEC-0005 R11: every assignment reports its origin, `analyst` when the destination comes
/// from a treatment the analyst stated. The ordinary company states the treatments of
/// financial fixed assets, shareholder loans and derivative liabilities.
#[test]
fn stated_treatments_are_assignments_of_analyst_origin() {
    let out = ok(&golden_input("balance-sheet-ordinary"));
    let assignments = out["value"]["years"][0]["assignments"].as_array().unwrap();
    let origin = |pointer_end: &str| {
        let a = assignments
            .iter()
            .find(|a| a["aggregate"].as_str().unwrap().ends_with(pointer_end))
            .unwrap_or_else(|| panic!("no assignment of {pointer_end}"));
        a["origin"].as_str().unwrap().to_owned()
    };
    assert_eq!(origin("/financial_fixed_assets"), "analyst");
    assert_eq!(origin("/payables/shareholder_loans"), "analyst");
    assert_eq!(
        origin("/provisions/of_which_derivative_liabilities"),
        "analyst"
    );
    assert_eq!(origin("/tangible_assets"), "mapping");
    let labels = step_labels(&out);
    for reference in [
        "B.III to non_operating_assets (analyst)",
        "liabilities D, shareholder loans to net_financial_position_gross_debt (analyst)",
        "liabilities B, derivatives to net_financial_position_other_financial_liability (analyst)",
    ] {
        assert!(
            labels.contains(&format!("Assignment (2024): {reference}")),
            "{reference}: {labels:?}"
        );
    }
}

/// SPEC-0005 R9 (review of PR 1): when some parts of a shareholder loans split have no maturity
/// beyond one year, their sum is not reconciled with the loans' amount beyond one year; only a
/// sum larger than it is refused. A sum equal to it is accepted and reported as not checked.
#[test]
fn split_beyond_one_year_equal_to_the_loans_is_not_refused() {
    let mut input = golden_input("not-checked-shareholder-loans-split");
    // The loans are 50000 beyond one year; the financial debt part states all of it, the
    // quasi-equity part states none, so the split is incomplete.
    input["years"][0]["treatment_overrides"]["shareholder_loans"]["split"][0]["beyond_one_year"] =
        json!("50000");
    let out = ok(&input);
    assert_eq!(
        out["value"]["years"][0]["checks"]["shareholder_loans_split_beyond_one_year"]["status"],
        "not_checked"
    );
    input["years"][0]["treatment_overrides"]["shareholder_loans"]["split"][0]["beyond_one_year"] =
        json!("50001");
    let e = fairmark_core::run(METHOD, &input.to_string()).expect_err("sum above the loans");
    assert_eq!(e.code(), "FM-E026", "{e}");
}

/// SPEC-0005 R21, R11 (Appendix C, lease): the prepayment of a lease leaves the separate line of
/// assets D when it is given; then the adjustment is applied and no note says otherwise.
#[test]
fn lease_prepayment_leaves_the_separate_line() {
    let out = ok(&golden_input("adjustment-lease"));
    let applied = out["value"]["years"][0]["adjustments_applied"]
        .as_array()
        .unwrap();
    let prepayment = applied
        .iter()
        .find(|a| a["line"] == "prepayment")
        .expect("the prepayment line");
    assert_eq!(
        prepayment["source"],
        "/years/0/statutory/balance_sheet/prepayments_and_accrued_income"
    );
    assert!(
        !conventions(&out).iter().any(|c| c.contains("not applied")),
        "{:?}",
        conventions(&out)
    );
}

/// SPEC-0005 R21, R8 (not computed, never guessed): without assets D, either as a separate line
/// or within C.II, the prepayment of a lease has no source; the result is returned, the note
/// names the missing source and the outputs it reaches are listed as not computed, the
/// current and non-current split in its own block.
#[test]
fn lease_prepayment_without_a_source_is_noted_and_not_computed() {
    let mut input = golden_input("adjustment-lease");
    input["years"][0]["statutory"]["balance_sheet"]
        .as_object_mut()
        .unwrap()
        .remove("prepayments_and_accrued_income");
    let out = ok(&input);
    assert!(
        conventions(&out).contains(
            &"adjustment 0 'Head office lease' not applied to 2024: its source \
              `/years/0/statutory/balance_sheet/prepayments_and_accrued_income` is missing; the \
              outputs it reaches are not computed"
                .to_owned()
        ),
        "{:?}",
        conventions(&out)
    );
    let not_computed = &out["value"]["years"][0]["not_computed"];
    let outputs = |block: &str| -> Vec<String> {
        not_computed[block]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["output"].as_str().unwrap().to_owned())
            .collect()
    };
    let cnc = outputs("current_non_current");
    assert!(
        cnc.contains(&"/years/0/current_non_current/current_assets".to_owned()),
        "{cnc:?}"
    );
    assert!(cnc
        .iter()
        .all(|o| o.starts_with("/years/0/current_non_current/")));
    assert!(outputs("balance_sheet")
        .iter()
        .all(|o| o.starts_with("/years/0/balance_sheet/")));
}

/// SPEC-0005 R13, R11: gross debt and cash of the net financial position are written once, with
/// their parts substituted; the restatement "as given" of the shared net financial position
/// function is not repeated.
#[test]
fn gross_debt_and_cash_steps_are_written_once() {
    let out = ok(&golden_input("adjustment-lease"));
    let labels = step_labels(&out);
    let count = |prefix: &str| labels.iter().filter(|l| l.starts_with(prefix)).count();
    assert_eq!(count("Gross debt"), 1, "{labels:?}");
    assert_eq!(count("Cash"), 1, "{labels:?}");
}
