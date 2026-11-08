//! `statements.reorganise` (SPEC-0005), first and second pull requests: contract tests with the
//! methods that take its outputs (AC9), error messages (R24), determinism and the schema (AC13),
//! `meta.defaults_applied` (R8), exhaustiveness of `not_computed` (AC2), the properties AC7 and
//! AC8 on random balanced statements, and AC10 and AC11 on random income statements.

use std::path::Path;
use std::str::FromStr;

use proptest::prelude::*;
use rust_decimal::Decimal;
use serde_json::{json, Value};

const METHOD: &str = "statements.reorganise";

/// The input of a golden case of this method, so the tests reuse the reviewed companies.
fn golden_input(name: &str) -> Value {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden/statements.reorganise")
        .join(format!("{name}.yaml"));
    let case: Value = serde_norway::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    case["input"].clone()
}

fn run(input: &Value) -> Value {
    fairmark_core::run(METHOD, &input.to_string()).unwrap_or_else(|e| panic!("{e}"))
}

fn err(input: &Value) -> fairmark_core::FairmarkError {
    fairmark_core::run(METHOD, &input.to_string()).expect_err("an error")
}

fn dec(v: &Value) -> Decimal {
    Decimal::from_str(v.as_str().unwrap_or_else(|| panic!("not a decimal: {v}"))).unwrap()
}

fn conventions(out: &Value) -> String {
    out["explain"]["conventions"].to_string()
}

// ---------------------------------------------------------------- AC9: contract tests

/// The net financial position, the non-operating assets and the debt-like items are accepted
/// by `dcf.fcff` unchanged, and the bridge subtracts exactly what the reorganisation returned.
#[test]
fn bridge_items_are_accepted_by_dcf_fcff_unchanged() {
    let mut input = golden_input("balance-sheet-ordinary");
    input["adjustments"] = json!([{"kind": "debt_like_provision", "label": "Provision for litigation",
        "reason": "Settlement expected in cash", "aggregate": "provisions", "amounts": ["20000"]}]);
    let out = run(&input);
    let bs = &out["value"]["years"][0]["balance_sheet"];
    let sources = &bs["sources"];
    let fcff = json!({
        "stages": [{"flows": ["100000", "110000", "120000"]}],
        "wacc": "0.09",
        "terminal": {"none": {}},
        "net_financial_position": sources["net_financial_position"]["position"],
        "non_operating_assets": bs["non_operating_assets"],
        "debt_like_items": sources["debt_like_items"],
    });
    let v = fairmark_core::run("dcf.fcff", &fcff.to_string()).unwrap();
    let bridge = &v["value"]["bridge"];
    assert_eq!(
        dec(&bridge["net_financial_position"]),
        dec(&sources["net_financial_position"]["total"])
    );
    assert_eq!(
        dec(&bridge["non_operating_assets_total"]),
        dec(&bs["non_operating_assets_total"])
    );
    assert_eq!(
        dec(&bridge["debt_like_items_total"]),
        dec(&sources["debt_like_items_total"])
    );
    assert_eq!(
        bridge["debt_like_items"][0]["measurement_basis"],
        "book_value"
    );
}

/// `current_non_current` and the balance sheet give `ratios.liquidity` and
/// `ratios.capital_structure` their inputs under the same field names.
#[test]
fn current_non_current_feeds_liquidity_and_capital_structure() {
    let out = run(&golden_input("balance-sheet-ordinary"));
    let y = &out["value"]["years"][0];
    let cnc = &y["current_non_current"];
    let liquidity = json!({"balances": "closing", "closing": {
        "current_assets": cnc["current_assets"], "inventories": cnc["inventories"],
        "cash": cnc["cash"], "current_liabilities": cnc["current_liabilities"]}});
    let l = fairmark_core::run("ratios.liquidity", &liquidity.to_string()).unwrap();
    assert_eq!(
        dec(&l["value"]["net_working_capital"]),
        Decimal::from(530000)
    );
    let capital = json!({"balances": "closing", "closing": {
        "equity": y["balance_sheet"]["sources"]["equity"],
        "total_assets": cnc["total_assets"], "fixed_assets": cnc["fixed_assets"],
        "non_current_liabilities": cnc["non_current_liabilities"],
        "net_financial_position": y["balance_sheet"]["sources"]["net_financial_position"]["position"]}});
    let c = fairmark_core::run("ratios.capital_structure", &capital.to_string()).unwrap();
    assert_eq!(
        dec(&c["value"]["net_financial_position"]),
        Decimal::from(710000)
    );
    assert_eq!(
        dec(&c["value"]["primary_structure_margin"]),
        Decimal::from(900000 - 1220000)
    );
}

// ---------------------------------------------------------------- AC8: the identity, by construction

/// The net financial position total equals the SPEC-0007 total of the object returned.
fn nfp_total_of_position(position: &Value) -> Decimal {
    let mut t = dec(&position["gross_debt"]) - dec(&position["cash"]);
    for item in position["other_items"].as_array().unwrap() {
        t += dec(&item["amount"]);
    }
    t
}

fn check_identities(out: &Value) {
    for y in out["value"]["years"].as_array().unwrap() {
        let bs = &y["balance_sheet"];
        let s = &bs["sources"];
        if bs["total_funds_invested"].is_null() || s["total_sources"].is_null() {
            continue;
        }
        assert_eq!(dec(&bs["total_funds_invested"]), dec(&s["total_sources"]));
        let total = dec(&s["net_financial_position"]["total"]);
        assert_eq!(
            nfp_total_of_position(&s["net_financial_position"]["position"]),
            total
        );
        let cur = dec(&s["net_financial_position"]["current"]);
        let ncur = dec(&s["net_financial_position"]["non_current"]);
        assert_eq!(cur + ncur, total);
        let op = &bs["operating"];
        assert_eq!(
            dec(&op["invested_capital"]),
            dec(&op["operating_fixed_assets"]) + dec(&op["operating_working_capital"])
                - dec(&op["operating_provisions"])
        );
        assert_eq!(
            dec(&bs["total_funds_invested"]),
            dec(&op["invested_capital"]) + dec(&bs["non_operating_assets_total"])
        );
    }
}

/// A balanced statutory balance sheet from random amounts, with random treatments and
/// adjustments within their sources.
fn balanced(a: &[i64], choices: &[usize]) -> Value {
    let s = |x: i64| x.to_string();
    let (b1, b2, b3, c1, c2, c3, c4, d) = (a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7]);
    let (prov, tfr, fd, sl, other, e) = (a[8], a[9], a[10], a[11], a[12], a[13]);
    let deriv = prov * (a[14] % 100) / 100;
    let payables = fd + sl + other;
    let assets = b1 + b2 + b3 + c1 + c2 + c3 + c4 + d;
    let equity = assets - prov - tfr - payables - e;
    let ffa = ["operating", "non_operating", "financial_asset"][choices[0] % 3];
    let sld = ["financial_debt", "quasi_equity"][choices[1] % 2];
    let dvd = ["financial_liability", "operating_provision"][choices[2] % 2];
    let part = |x: i64, k: usize| x * (choices[k] as i64 % 50) / 100;
    json!({
        "years": [{"statutory": {"balance_sheet": {
            "subscribed_capital_unpaid": "0",
            "intangible_assets": {"total": s(b1)}, "tangible_assets": {"total": s(b2)},
            "financial_fixed_assets": s(b3), "inventories": s(c1),
            "receivables": {"total": s(c2), "beyond_one_year": s(part(c2, 3))},
            "current_financial_assets": s(c3), "cash": s(c4),
            "prepayments_and_accrued_income": {"total": s(d)},
            "total_assets": s(assets),
            "equity": {"total": s(equity)},
            "provisions": {"total": s(prov), "of_which_derivative_liabilities": s(deriv)},
            "employee_severance_indemnity": {"total": s(tfr)},
            "payables": {"total": s(payables), "beyond_one_year": s(part(fd, 4)),
                "financial_debt": {"total": s(fd), "beyond_one_year": s(part(fd, 4))},
                "shareholder_loans": {"total": s(sl), "beyond_one_year": "0"}},
            "accruals_and_deferred_income": {"total": s(e)},
            "total_liabilities_and_equity": s(assets)}}}],
        "treatments": {"financial_fixed_assets": ffa, "shareholder_loans": sld,
            "derivative_liabilities": dvd},
        "adjustments": [
            {"kind": "non_operating_asset", "label": "Idle plant", "reason": "Not used",
             "aggregate": "tangible_assets", "amounts": [s(part(b2, 5))]},
            {"kind": "financial_receivable", "label": "Loan to affiliate", "reason": "Financing",
             "maturity": "beyond_one_year", "amounts": [s(part(c2, 6))]},
            {"kind": "operating_cash", "label": "Minimum cash", "reason": "Operations",
             "amounts": [s(part(c4, 7))]},
            {"kind": "accrued_interest", "label": "Accrued interest", "reason": "Cost of debt",
             "amounts": [s(part(e, 8))]},
            {"kind": "tax_instalments", "label": "Tax instalments", "reason": "Deferred taxes",
             "maturity": "beyond_one_year", "amounts": [s(part(other, 9))]},
            {"kind": "non_current_trade_payables", "label": "Extended supplier", "reason": "Financing",
             "amounts": [s(part(other, 10))]},
            {"kind": "debt_like_provision", "label": "Litigation", "reason": "Cash settlement",
             "aggregate": "employee_severance_indemnity", "amounts": [s(part(tfr, 11))]},
            {"kind": "dividends_declared", "label": "Dividend", "reason": "Declared",
             "amounts": [s(part(equity.max(0), 12))]}
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// AC8 and AC7: on random balanced statements, total funds invested equal the sources, the
    /// net financial position equals the SPEC-0007 total of its object and the sum of its
    /// maturities, and the assigned amounts of each aggregate sum to the aggregate.
    #[test]
    fn identities_hold_on_random_balanced_statements(
        a in proptest::collection::vec(0i64..5_000_000, 15),
        choices in proptest::collection::vec(0usize..1000, 13),
    ) {
        let input = balanced(&a, &choices);
        let out = run(&input);
        check_identities(&out);
        let y = &out["value"]["years"][0];
        let bs = &input["years"][0]["statutory"]["balance_sheet"];
        let base = "/years/0/statutory/balance_sheet";
        for (key, amount) in [
            ("financial_fixed_assets", bs["financial_fixed_assets"].clone()),
            ("receivables", bs["receivables"]["total"].clone()),
            ("payables", bs["payables"]["total"].clone()),
            ("provisions", bs["provisions"]["total"].clone()),
        ] {
            let pointer = format!("{base}/{key}");
            let sum: Decimal = y["assignments"].as_array().unwrap().iter()
                .filter(|e| e["aggregate"].as_str().unwrap().starts_with(&pointer))
                .map(|e| dec(&e["amount"]))
                .sum();
            prop_assert_eq!(sum, dec(&amount), "{}", key);
        }
    }

    /// AC8: a non-operating asset never enters invested capital: moving an amount out of B.II
    /// lowers invested capital by that amount, raises the non-operating assets by the same
    /// amount and leaves total funds invested unchanged.
    #[test]
    fn non_operating_assets_are_outside_invested_capital(moved in 0i64..900_000) {
        let mut input = golden_input("balance-sheet-ordinary");
        let base = run(&input);
        input["adjustments"] = json!([{"kind": "non_operating_asset", "label": "Idle plant",
            "reason": "Not used", "aggregate": "tangible_assets", "amounts": [moved.to_string()]}]);
        let out = run(&input);
        let (b, o) = (&base["value"]["years"][0]["balance_sheet"], &out["value"]["years"][0]["balance_sheet"]);
        let m = Decimal::from(moved);
        prop_assert_eq!(dec(&o["operating"]["invested_capital"]), dec(&b["operating"]["invested_capital"]) - m);
        prop_assert_eq!(dec(&o["non_operating_assets_total"]), dec(&b["non_operating_assets_total"]) + m);
        prop_assert_eq!(dec(&o["total_funds_invested"]), dec(&b["total_funds_invested"]));
    }
}

// ---------------------------------------------------------------- AC13, R24: errors

/// Every refusal names the field by its JSON pointer and states the fix.
#[test]
fn every_error_names_a_pointer_and_the_fix() {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/statements.reorganise");
    let mut seen = 0;
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let case: Value = serde_norway::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        if case.get("expect_error").is_none() {
            continue;
        }
        let e = err(&case["input"]);
        let msg = e.to_string();
        assert!(
            msg.contains("`/"),
            "{}: no JSON pointer in {msg}",
            path.display()
        );
        assert!(
            msg.contains("; ") || msg.contains(": "),
            "{}: no fix in {msg}",
            path.display()
        );
        seen += 1;
    }
    assert!(seen >= 40, "only {seen} error cases");
}

#[test]
fn a_reconciliation_error_gives_both_amounts_and_the_difference() {
    let mut input = golden_input("balance-sheet-ordinary");
    input["years"][0]["statutory"]["balance_sheet"]["fixed_assets"] = json!("1170001");
    let e = err(&input);
    assert_eq!(e.code(), "FM-E026");
    let msg = e.to_string();
    for part in [
        "/years/0/statutory/balance_sheet/fixed_assets",
        "1170001",
        "1170000",
        "difference 1",
    ] {
        assert!(msg.contains(part), "missing '{part}' in {msg}");
    }
}

#[test]
fn an_override_of_another_aggregate_is_refused_at_its_pointer() {
    let mut input = golden_input("balance-sheet-ordinary");
    input["years"][0]["treatment_overrides"] = json!({"provisions": {"destination": "operating"}});
    let e = err(&input);
    assert_eq!(e.code(), "FM-E001");
    assert!(
        e.to_string()
            .contains("/years/0/treatment_overrides/provisions"),
        "{e}"
    );
}

/// The income statement kinds are applied (the refusal of the first pull request is removed):
/// each appears in `adjustments_applied` with its source and target.
#[test]
fn income_statement_adjustments_are_applied() {
    for (adj, target) in [
        (
            json!({"kind": "investing", "label": "Rent", "reason": "Investment property",
               "aggregate": "other_revenue_and_income", "amounts": ["1000"]}),
            "investing_result",
        ),
        (
            json!({"kind": "non_recurring", "label": "Gain", "reason": "Disposal",
               "aggregate": "revenue", "amounts": ["1000"]}),
            "non_recurring_result",
        ),
        (
            json!({"kind": "lease", "label": "Head office", "reason": "IFRS 16 view",
               "amounts": [{"asset": "1", "liability": "1", "liability_current": "0",
                            "prepayment": "0", "payments": "0", "depreciation": "0",
                            "interest": "0"}]}),
            "operating_fixed_assets",
        ),
    ] {
        let mut input = golden_input("ordinary-aggregates");
        input["adjustments"] = json!([adj]);
        let out = run(&input);
        let applied = &out["value"]["years"][0]["adjustments_applied"];
        assert_eq!(applied[0]["index"], 0, "{applied}");
        assert_eq!(applied[0]["target"], target, "{applied}");
        check_income_statement(&out, &input);
    }
}

/// AC11, R17: D18 and D19 cannot be moved; the message names the field and says why.
#[test]
fn a_non_recurring_adjustment_of_d18_or_d19_is_refused_with_its_reason() {
    for aggregate in [
        "write_downs_of_financial_assets",
        "revaluations_of_financial_assets",
    ] {
        let mut input = golden_input("balance-sheet-ordinary");
        input["adjustments"] = json!([{"kind": "non_recurring", "label": "Impairment",
            "reason": "One-off", "aggregate": aggregate, "amounts": ["1"]}]);
        let e = err(&input);
        assert_eq!(e.code(), "FM-E001");
        let msg = e.to_string();
        assert!(msg.contains("`/adjustments/0/aggregate`"), "{msg}");
        assert!(
            msg.contains("D18 and D19 are fixed in the investing result"),
            "{msg}"
        );
    }
}

/// The ordinary company with its shareholder loans (100000, of which 50000 beyond one year)
/// split by `parts` for the year.
fn split_loans(parts: Value) -> Value {
    let mut input = golden_input("balance-sheet-ordinary");
    input["years"][0]["treatment_overrides"] = json!({"shareholder_loans": {"split": parts}});
    input
}

/// Review of PR 1 (item 4): a quasi-equity part has no maturity in the net financial position.
#[test]
fn a_quasi_equity_part_with_an_amount_beyond_one_year_is_refused() {
    let e = err(&split_loans(json!([
        {"destination": "financial_debt", "amount": "60000", "label": "Loan A",
         "beyond_one_year": "30000"},
        {"destination": "quasi_equity", "amount": "40000", "label": "Subordinated",
         "beyond_one_year": "20000"}])));
    assert_eq!(e.code(), "FM-E001");
    let msg = e.to_string();
    assert!(
        msg.contains("/years/0/treatment_overrides/shareholder_loans/split/1/beyond_one_year"),
        "{msg}"
    );
}

/// Review of PR 1 (item 4): a part's amount beyond one year cannot exceed the part.
#[test]
fn a_part_beyond_one_year_larger_than_the_part_is_refused() {
    let e = err(&split_loans(json!([
        {"destination": "financial_debt", "amount": "60000", "label": "Loan A",
         "beyond_one_year": "60001"},
        {"destination": "financial_debt", "amount": "40000", "label": "Loan B",
         "beyond_one_year": "0"}])));
    assert_eq!(e.code(), "FM-E026");
    assert!(e.to_string().contains("split/0/beyond_one_year"), "{e}");
}

/// Review of PR 1 (item 4): the parts' amounts beyond one year sum to the loans' when every part
/// is financial debt with its maturity given; otherwise the relation is not checked, and a sum
/// already larger than the loans' is refused.
#[test]
fn the_parts_beyond_one_year_reconcile_with_the_loans() {
    let both = |a: &str, b: &str| {
        split_loans(json!([
            {"destination": "financial_debt", "amount": "60000", "label": "Loan A",
             "beyond_one_year": a},
            {"destination": "financial_debt", "amount": "40000", "label": "Loan B",
             "beyond_one_year": b}]))
    };
    let out = run(&both("30000", "20000"));
    let checks = &out["value"]["years"][0]["checks"];
    assert_eq!(
        checks["shareholder_loans_split_beyond_one_year"]["status"],
        "reconciled"
    );
    assert_eq!(
        checks["shareholder_loans_split_parts_beyond_one_year"]["status"],
        "reconciled"
    );
    assert_eq!(
        out["value"]["years"][0]["balance_sheet"]["sources"]["net_financial_position"]
            ["non_current"],
        "610000"
    );
    let e = err(&both("30000", "20001"));
    assert_eq!(e.code(), "FM-E026");
    assert!(
        e.to_string().contains(
            "/years/0/statutory/balance_sheet/payables/shareholder_loans/beyond_one_year"
        ),
        "{e}"
    );
    let mixed = split_loans(json!([
        {"destination": "financial_debt", "amount": "60000", "label": "Loan A",
         "beyond_one_year": "30000"},
        {"destination": "quasi_equity", "amount": "40000", "label": "Subordinated"}]));
    let out = run(&mixed);
    assert_eq!(
        out["value"]["years"][0]["checks"]["shareholder_loans_split_beyond_one_year"]["status"],
        "not_checked"
    );
    let over = split_loans(json!([
        {"destination": "financial_debt", "amount": "60000", "label": "Loan A",
         "beyond_one_year": "50001"},
        {"destination": "quasi_equity", "amount": "40000", "label": "Subordinated"}]));
    assert_eq!(err(&over).code(), "FM-E026");
}

#[test]
fn an_empty_list_of_years_is_refused() {
    let e = err(&json!({"years": []}));
    assert_eq!(e.code(), "FM-E001");
    assert!(e.to_string().contains("`/years`"), "{e}");
}

// ---------------------------------------------------------------- R11, constitution II: explain

/// The step with `label`, or a panic listing the labels.
fn step<'a>(out: &'a Value, label: &str) -> &'a Value {
    let steps = out["explain"]["steps"].as_array().unwrap();
    steps
        .iter()
        .find(|s| s["label"] == label)
        .unwrap_or_else(|| {
            let labels: Vec<&Value> = steps.iter().map(|s| &s["label"]).collect();
            panic!("no step '{label}' in {labels:?}")
        })
}

/// Review of PR 1 (item 3): every number of the net financial position is a step with its parts
/// substituted, one labelled term per component.
#[test]
fn the_net_financial_position_shows_every_part_substituted() {
    let out = run(&golden_input("balance-sheet-ordinary"));
    for (label, formula, value) in [
        (
            "Gross debt (2024)",
            "Financial debt (D) 900000 + Shareholder loans (D) 100000",
            "1000000",
        ),
        ("Cash (2024)", "C.IV 240000 − operating cash 0", "240000"),
        (
            "Excess cash (2024)",
            "C.IV 240000 − operating cash 0",
            "240000",
        ),
        (
            "Net financial position, current (2024)",
            "Financial debt (D) 350000 + Shareholder loans (D) 50000 + Cash (C.IV) -240000 + \
             Current financial assets (C.III) -60000 + Derivative liabilities (B) 0",
            "100000",
        ),
        (
            "Net financial position, non-current (2024)",
            "Financial debt (D) 550000 + Shareholder loans (D) 50000 + Cash (C.IV) 0 + \
             Current financial assets (C.III) 0 + Derivative liabilities (B) 10000",
            "610000",
        ),
    ] {
        let s = step(&out, label);
        assert_eq!(s["formula"], formula, "{label}");
        assert_eq!(s["value"], value, "{label}");
    }
    // Review round 2 (nit): gross debt is explained before the parts of the position.
    let labels: Vec<&str> = out["explain"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| s["label"].as_str().unwrap())
        .collect();
    let at = |l: &str| labels.iter().position(|x| *x == l).unwrap();
    assert!(at("Gross debt (2024)") < at("Net financial position, current (2024)"));
}

/// Review of PR 1 (item 3): in the reclassified form, the totals of the lists and the excess
/// cash are steps too.
#[test]
fn the_reclassified_totals_of_lists_and_excess_cash_are_steps() {
    let out = run(&golden_input("reclassified-balance-sheet"));
    for (label, formula, value) in [
        (
            "Non-operating assets (2024)",
            "Financial fixed assets (B.III) 150000",
            "150000",
        ),
        ("Quasi-equity items (2024)", "none", "0"),
        ("Debt-like items (2024)", "none", "0"),
        (
            "Excess cash (2024)",
            "cash of the net financial position 240000",
            "240000",
        ),
    ] {
        let s = step(&out, label);
        assert_eq!(s["formula"], formula, "{label}");
        assert_eq!(s["value"], value, "{label}");
    }
}

// ---------------------------------------------------------------- R15: current and non-current

/// Review of PR 1 (item 10): the block uses the totals given (B, C, total assets) when their
/// components are not all given, with item A deducted as everywhere else (D1).
#[test]
fn current_non_current_uses_the_totals_given() {
    let mut input = golden_input("balance-sheet-ordinary");
    let bs = &mut input["years"][0]["statutory"]["balance_sheet"];
    let m = bs.as_object_mut().unwrap();
    m.remove("intangible_assets");
    m.remove("cash");
    m.insert("subscribed_capital_unpaid".into(), json!("0"));
    let out = run(&input);
    let cnc = &out["value"]["years"][0]["current_non_current"];
    assert_eq!(cnc["fixed_assets"], "1220000");
    assert_eq!(cnc["current_assets"], "1630000");
    assert_eq!(cnc["total_assets"], "2850000");
    let mut input = golden_input("balance-sheet-ordinary");
    let bs = &mut input["years"][0]["statutory"]["balance_sheet"];
    let m = bs.as_object_mut().unwrap();
    m.remove("fixed_assets");
    m.remove("tangible_assets");
    let out = run(&input);
    let cnc = &out["value"]["years"][0]["current_non_current"];
    assert!(cnc["fixed_assets"].is_null());
    assert_eq!(cnc["total_assets"], "2850000");
}

/// Review round 2 (audit F): with receivables absent, total assets derived from total B, total
/// C and assets D, with item A within C.II none (D2), are the same as when total assets are
/// given; the parts beyond one year cancel out and are not needed.
#[test]
fn current_non_current_total_assets_derived_as_when_given() {
    let mut input = golden_input("balance-sheet-ordinary");
    let bs = &mut input["years"][0]["statutory"]["balance_sheet"];
    let m = bs.as_object_mut().unwrap();
    m.remove("receivables");
    m.remove("total_assets");
    let out = run(&input);
    let cnc = &out["value"]["years"][0]["current_non_current"];
    assert_eq!(cnc["total_assets"], "2850000");
    assert!(cnc["fixed_assets"].is_null());
}

// ---------------------------------------------------------------- R5: the reclassified form

/// Review of PR 1 (item 5): an output that only the statutory form gives is listed without a
/// pointer to give (giving `statutory` in the same entry is `FM-E001`), and a convention says
/// why; inventories come from the operating working capital.
#[test]
fn reclassified_outputs_of_the_statutory_form_are_explained_not_pointed_at() {
    let out = run(&golden_input("reclassified-balance-sheet"));
    let y = &out["value"]["years"][0];
    assert_eq!(y["current_non_current"]["inventories"], "400000");
    for block in ["balance_sheet", "current_non_current"] {
        for entry in y["not_computed"][block].as_array().unwrap() {
            let missing = entry["missing"].as_array().unwrap();
            assert!(
                missing
                    .iter()
                    .all(|m| !m.as_str().unwrap().contains("/statutory")),
                "{entry}"
            );
        }
    }
    let outputs: Vec<&str> = y["not_computed"]["current_non_current"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["output"].as_str().unwrap())
        .collect();
    assert!(!outputs.contains(&"/years/0/current_non_current/inventories"));
    let conv = conventions(&out);
    assert!(
        conv.contains(
            "year 2024: the current and non-current block (except inventories) and \
                       the composition of the net financial position need the statutory form"
        ),
        "{conv}"
    );
}

/// Review of PR 1 (item 5): with only its current and non-current parts, the net financial
/// position total is their sum, and the maturity relation, which would compare the total with
/// itself, is not checked.
#[test]
fn the_reclassified_net_financial_position_is_derived_from_its_parts() {
    let mut input = golden_input("reclassified-balance-sheet");
    input["years"][0]["reclassified"]["balance_sheet"]["net_financial_position"] =
        json!({"current": "100000", "non_current": "610000"});
    let out = run(&input);
    let y = &out["value"]["years"][0];
    assert_eq!(
        y["balance_sheet"]["sources"]["net_financial_position"]["total"],
        "710000"
    );
    assert_eq!(
        y["checks"]["net_financial_position_maturity"]["status"],
        "not_checked"
    );
    assert_eq!(
        y["checks"]["funds_invested_equals_sources"]["status"],
        "reconciled"
    );
    let s = step(&out, "Net financial position from its parts (2024)");
    assert_eq!(s["formula"], "current 100000 + non-current 610000");
}

// ---------------------------------------------------------------- AC13: stability and schema

#[test]
fn two_runs_give_identical_output_and_key_order_does_not_change_the_hash() {
    let input = golden_input("balance-sheet-ordinary");
    let a = run(&input);
    let b = run(&input);
    assert_eq!(a.to_string(), b.to_string());
    let mut reordered = serde_json::Map::new();
    let obj = input.as_object().unwrap();
    for k in obj.keys().rev() {
        reordered.insert(k.clone(), obj[k].clone());
    }
    let c = run(&Value::Object(reordered));
    assert_eq!(a["meta"]["input_hash"], c["meta"]["input_hash"]);
    assert_eq!(a["value"], c["value"]);
}

/// D4 of 2026-11-08: `external_costs` and `other_charges_below_ebitda` of the reclassified
/// income statement are deferred to the second pull request, as additive fields; the spec does
/// not define them, so they wait for the maintainer's definition and stay unknown fields.
#[test]
fn the_deferred_reclassified_income_statement_fields_are_refused() {
    let schema = fairmark_core::input_schema(METHOD).unwrap().to_string();
    for field in ["external_costs", "other_charges_below_ebitda"] {
        assert!(!schema.contains(field), "{field}");
        let input = json!({"years": [{"reclassified": {"income_statement": {field: "1"}}}]});
        let e = err(&input);
        assert_eq!(e.code(), "FM-E001");
        assert!(e.to_string().contains(field), "{e}");
    }
}

#[test]
fn the_input_schema_has_no_boolean() {
    let schema = fairmark_core::input_schema(METHOD).unwrap().to_string();
    assert!(!schema.contains("\"boolean\""), "{schema}");
}

// ---------------------------------------------------------------- R8: meta.defaults_applied

#[test]
fn every_default_destination_is_listed_and_no_fixed_one() {
    let out = run(&golden_input("balance-sheet-ordinary"));
    let defaults = out["meta"]["defaults_applied"].as_array().unwrap();
    let fields: Vec<&str> = defaults
        .iter()
        .map(|d| d["field"].as_str().unwrap())
        .collect();
    let base = "/years/0/statutory/balance_sheet";
    for key in [
        "intangible_assets",
        "tangible_assets",
        "receivables",
        "current_financial_assets",
        "cash",
        "prepayments_and_accrued_income",
        "provisions",
        "employee_severance_indemnity",
        "payables",
        "accruals_and_deferred_income",
    ] {
        assert!(fields.contains(&format!("{base}/{key}").as_str()), "{key}");
    }
    for fixed in [
        "inventories",
        "equity",
        "financial_fixed_assets",
        "subscribed_capital_unpaid",
    ] {
        assert!(
            !fields.contains(&format!("{base}/{fixed}").as_str()),
            "{fixed}"
        );
    }
    assert!(fields.contains(&"/treatments/write_downs_of_receivables"));
    for d in defaults {
        assert!(!d["reason"].as_str().unwrap().trim().is_empty());
    }
    let conv = conventions(&out);
    assert!(
        conv.contains("default: `/years/0/statutory/balance_sheet/provisions` assigned to"),
        "{conv}"
    );
    assert!(
        conv.contains("default: `/treatments/write_downs_of_receivables` not given, set to"),
        "{conv}"
    );
}

#[test]
fn a_default_destination_of_an_aggregate_not_given_is_not_listed() {
    let input = json!({"years": [{"statutory": {"balance_sheet": {"cash": "100"}}}],
        "treatments": {"write_downs_of_receivables": "investing"}});
    let out = run(&input);
    let destinations: Vec<&Value> = out["meta"]["defaults_applied"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["value"] != "0")
        .collect();
    assert_eq!(destinations.len(), 1);
    assert_eq!(
        destinations[0]["field"],
        "/years/0/statutory/balance_sheet/cash"
    );
}

/// Review of PR 1: a default destination is listed only when its aggregate is given, not when
/// the node holds only a part of it.
#[test]
fn a_node_without_its_aggregate_lists_no_default_destination() {
    let input = json!({"years": [{"statutory": {"balance_sheet": {
        "receivables": {"trade": "5"},
        "provisions": {"beyond_one_year": "1"},
        "intangible_assets": {"gross": "10", "accumulated": "4"},
        "tangible_assets": {"gross": "10"}}}}],
        "treatments": {"write_downs_of_receivables": "investing"}});
    let out = run(&input);
    let destinations: Vec<&str> = out["meta"]["defaults_applied"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|d| d["value"] != "0")
        .map(|d| d["field"].as_str().unwrap())
        .collect();
    assert_eq!(
        destinations,
        ["/years/0/statutory/balance_sheet/intangible_assets"]
    );
}

/// D1, D2 of 2026-11-08: item A and the "of which" parts not given are zero, each listed in
/// `meta.defaults_applied` after the destinations and the input default, also when the parent
/// node is absent.
#[test]
fn items_taken_as_zero_are_listed_also_without_their_parent() {
    let input = json!({"years": [{"statutory": {"balance_sheet": {"cash": "100"}}}],
        "treatments": {"write_downs_of_receivables": "investing"}});
    let out = run(&input);
    let fields: Vec<(String, String)> = out["meta"]["defaults_applied"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| {
            (
                d["field"].as_str().unwrap().to_owned(),
                d["value"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let b = "/years/0/statutory/balance_sheet";
    let zero = |k: &str| (format!("{b}/{k}"), "0".to_owned());
    assert_eq!(
        fields,
        [
            (
                format!("{b}/cash"),
                "net_financial_position_cash".to_owned()
            ),
            zero("subscribed_capital_unpaid"),
            zero("receivables/subscribed_capital_unpaid"),
            zero("receivables/prepayments_and_accrued_income"),
            zero("provisions/of_which_derivative_liabilities"),
            zero("payables/accruals_and_deferred_income"),
        ]
    );
    let conv = conventions(&out);
    assert!(
        conv.contains(&format!(
            "default: `{b}/provisions/of_which_derivative_liabilities` not given, set to"
        )),
        "{conv}"
    );
}

/// D1: without item A, equity is computed with A = 0, and the same zero is used for the total
/// assets of the current and non-current block.
#[test]
fn item_a_not_given_is_zero_everywhere() {
    let mut input = golden_input("balance-sheet-ordinary");
    let bs = &mut input["years"][0]["statutory"]["balance_sheet"];
    bs.as_object_mut()
        .unwrap()
        .remove("subscribed_capital_unpaid");
    let out = run(&input);
    let y = &out["value"]["years"][0];
    assert_eq!(y["balance_sheet"]["sources"]["equity"], "900000");
    assert_eq!(y["current_non_current"]["total_assets"], "2850000");
    assert_eq!(y["checks"]["total_assets_total"]["status"], "not_checked");
    let fields: Vec<&str> = out["meta"]["defaults_applied"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["field"].as_str().unwrap())
        .collect();
    assert!(fields.contains(&"/years/0/statutory/balance_sheet/subscribed_capital_unpaid"));
}

// ---------------------------------------------------------------- R7: only items given

/// Review of PR 1 and M2: a part presented within C.II or D counts as given, so total assets
/// rebuilt as A + B + C + D are compared; item A taken as zero (D1) is never compared, so
/// without it the same relations are not checked.
#[test]
fn a_total_rebuilt_from_an_item_not_given_is_not_checked() {
    let mut input = golden_input("balance-sheet-ordinary");
    let bs = &mut input["years"][0]["statutory"]["balance_sheet"];
    let m = bs.as_object_mut().unwrap();
    m.remove("total_assets");
    m.remove("prepayments_and_accrued_income");
    bs["receivables"]["total"] = json!("980000");
    bs["receivables"]["prepayments_and_accrued_income"] = json!("30000");
    bs["current_assets"] = json!("1680000");
    let out = run(&input);
    let checks = &out["value"]["years"][0]["checks"];
    assert_eq!(checks["total_assets_total"]["status"], "not_checked");
    let relation = "total_assets_equals_total_liabilities_and_equity";
    assert_eq!(checks[relation]["status"], "reconciled");
    assert!(checks[relation]["pointers"]
        .to_string()
        .contains("receivables/prepayments_and_accrued_income"));
    input["years"][0]["statutory"]["balance_sheet"]
        .as_object_mut()
        .unwrap()
        .remove("subscribed_capital_unpaid");
    let out = run(&input);
    let checks = &out["value"]["years"][0]["checks"];
    assert_eq!(checks[relation]["status"], "not_checked");
    assert_eq!(
        out["value"]["years"][0]["balance_sheet"]["total_funds_invested"],
        "1610000"
    );
}

/// With assets D presented within C.II but a receivables total that leaves them out, the items
/// given do not balance: `FM-E026` names the items read, never a total not given.
#[test]
fn a_difference_names_the_items_taken_as_zero_never_a_total_not_given() {
    let mut input = golden_input("balance-sheet-ordinary");
    let bs = &mut input["years"][0]["statutory"]["balance_sheet"];
    let m = bs.as_object_mut().unwrap();
    m.remove("total_assets");
    m.remove("prepayments_and_accrued_income");
    bs["receivables"]["prepayments_and_accrued_income"] = json!("30000");
    let e = err(&input);
    assert_eq!(e.code(), "FM-E026");
    let msg = e.to_string();
    let b = "/years/0/statutory/balance_sheet";
    assert!(!msg.contains(&format!("`{b}/total_assets`")), "{msg}");
    assert!(
        msg.contains(&format!("`{b}/receivables/prepayments_and_accrued_income`")),
        "{msg}"
    );
    assert!(msg.contains("difference -30000"), "{msg}");
}

/// Review round 2 (blocking 2), M1: when funds invested differ from the sources because a
/// separate line taken as zero is not zero, the message names only such lines, never an "of
/// which" part, and its fix resolves the error.
#[test]
fn the_identity_names_only_separate_lines_taken_as_zero() {
    let e = err(&golden_input(
        "reconcile-funds-invested-equals-sources-statutory",
    ));
    assert_eq!(e.code(), "FM-E026");
    let msg = e.to_string();
    let b = "/years/0/statutory/balance_sheet";
    assert!(
        msg.contains(&format!("`{b}/subscribed_capital_unpaid`")),
        "{msg}"
    );
    for part in [
        "receivables/subscribed_capital_unpaid",
        "receivables/prepayments_and_accrued_income",
        "of_which_derivative_liabilities",
        "payables/accruals_and_deferred_income",
        "total_assets`",
    ] {
        assert!(!msg.contains(part), "'{part}' in {msg}");
    }
    assert!(
        msg.contains("an amount given differs, or one of these items taken as zero is not zero"),
        "{msg}"
    );
    assert!(msg.contains("difference -30000"), "{msg}");
}

/// Every relation reported as reconciled names only pointers of items given (R7).
#[test]
fn a_reconciled_relation_names_only_items_given() {
    let dir =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden/statements.reorganise");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        let case: Value = serde_norway::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        if case.get("expect_error").is_some() {
            continue;
        }
        let out = run(&case["input"]);
        for y in out["value"]["years"].as_array().unwrap() {
            for (relation, check) in y["checks"].as_object().unwrap() {
                if check["status"] != "reconciled" {
                    continue;
                }
                for pointer in check["pointers"].as_array().unwrap() {
                    let pointer = pointer.as_str().unwrap();
                    assert!(
                        case["input"].pointer(pointer).is_some(),
                        "{}: {relation} names {pointer}, not given",
                        path.display()
                    );
                }
            }
        }
    }
}

/// R8, R17, R18, AC13: the default destinations of A5, B14 and B10 d are listed when given, in
/// that order before the input default; B10 d is not listed when the analyst states the
/// treatment, and no fixed destination of the income statement is listed.
#[test]
fn the_income_statement_defaults_are_listed_and_no_fixed_one() {
    let base = "/years/0/statutory/income_statement";
    let input = json!({"years": [{"statutory": {"income_statement": {
        "revenue": "100", "purchases": "40", "income_from_equity_investments": "3",
        "other_revenue_and_income": "10", "other_operating_expenses": "5",
        "depreciation_and_write_downs": {"write_downs_of_receivables": "500"}}}}]});
    let pairs = |out: &Value| -> Vec<(String, String)> {
        out["meta"]["defaults_applied"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| {
                (
                    d["field"].as_str().unwrap().to_owned(),
                    d["value"].as_str().unwrap().to_owned(),
                )
            })
            .collect()
    };
    let out = run(&input);
    let s = |a: &str, b: &str| (a.to_owned(), b.to_owned());
    assert_eq!(
        pairs(&out),
        [
            s(
                &format!("{base}/other_revenue_and_income"),
                "value_of_production"
            ),
            s(&format!("{base}/other_operating_expenses"), "value_added"),
            s(
                &format!("{base}/depreciation_and_write_downs/write_downs_of_receivables"),
                "investing_result"
            ),
            s("/treatments/write_downs_of_receivables", "investing"),
        ]
    );
    let conv = conventions(&out);
    assert!(
        conv.contains(&format!(
            "default: `{base}/other_operating_expenses` assigned to"
        )),
        "{conv}"
    );
    let mut stated = input.clone();
    stated["treatments"] = json!({"write_downs_of_receivables": "operating"});
    let out = run(&stated);
    assert_eq!(
        pairs(&out),
        [
            s(
                &format!("{base}/other_revenue_and_income"),
                "value_of_production"
            ),
            s(&format!("{base}/other_operating_expenses"), "value_added"),
        ]
    );
    let b10d = out["value"]["years"][0]["assignments"]
        .as_array()
        .unwrap()
        .iter()
        .find(|a| a["reference"] == "B10 d")
        .unwrap()
        .clone();
    assert_eq!(b10d["destination"], "operating_profit");
    assert_eq!(b10d["origin"], "analyst");
}

// ---------------------------------------------------------------- R6, AC2: not_computed

#[test]
fn not_computed_lists_exactly_the_outputs_that_need_the_missing_treatment() {
    let out = run(&golden_input("missing-treatment"));
    let nc = out["value"]["years"][0]["not_computed"]["balance_sheet"]
        .as_array()
        .unwrap();
    let outputs: Vec<&str> = nc.iter().map(|e| e["output"].as_str().unwrap()).collect();
    let b = "/years/0/balance_sheet";
    assert_eq!(
        outputs,
        [
            format!("{b}/non_operating_assets"),
            format!("{b}/non_operating_assets_total"),
            format!("{b}/operating/invested_capital"),
            format!("{b}/operating/operating_fixed_assets"),
            format!("{b}/sources/net_financial_position/composition"),
            format!("{b}/sources/net_financial_position/non_current"),
            format!("{b}/sources/net_financial_position/position"),
            format!("{b}/sources/net_financial_position/total"),
            format!("{b}/sources/total_sources"),
            format!("{b}/total_funds_invested"),
        ]
    );
    assert!(
        out["value"]["years"][0]["not_computed"]["current_non_current"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

/// Every null output is listed, and no listed output has a value (R6).
#[test]
fn every_null_output_is_listed_and_none_is_zero_for_lack_of_input() {
    let input = json!({"years": [{"statutory": {"balance_sheet": {"cash": "100",
        "inventories": "50"}}}]});
    let out = run(&input);
    let y = &out["value"]["years"][0];
    let listed: Vec<String> = ["balance_sheet", "current_non_current"]
        .iter()
        .flat_map(|b| y["not_computed"][b].as_array().unwrap().iter())
        .map(|e| e["output"].as_str().unwrap().to_owned())
        .collect();
    fn nulls(v: &Value, at: String, out: &mut Vec<String>) {
        match v {
            Value::Null => out.push(at),
            Value::Object(m) => {
                for (k, x) in m {
                    nulls(x, format!("{at}/{k}"), out);
                }
            }
            _ => {}
        }
    }
    let mut found = Vec::new();
    nulls(
        &y["balance_sheet"],
        "/years/0/balance_sheet".into(),
        &mut found,
    );
    nulls(
        &y["current_non_current"],
        "/years/0/current_non_current".into(),
        &mut found,
    );
    found.sort();
    let mut l = listed.clone();
    l.sort();
    assert_eq!(found, l);
    assert_eq!(y["current_non_current"]["cash"], "100");
    assert_eq!(y["current_non_current"]["inventories"], "50");
    assert!(y["balance_sheet"]["excess_cash"] == "100");
}

#[test]
fn an_entry_with_no_form_computes_nothing_and_says_so() {
    let out = run(&json!({"years": [{"label": "2024"}]}));
    let y = &out["value"]["years"][0];
    assert!(y["form"].is_null());
    assert!(y["balance_sheet"]["operating"]["invested_capital"].is_null());
    assert!(
        conventions(&out).contains("no form given"),
        "{}",
        conventions(&out)
    );
}

// ---------------------------------------------------------------- conventions (AC13)

#[test]
fn conventions_state_the_order_assumed_and_the_treatment_not_needed() {
    let mut input = golden_input("balance-sheet-ordinary");
    input["years"][0].as_object_mut().unwrap().remove("date");
    input["years"][0]["statutory"]["balance_sheet"]["financial_fixed_assets"] = json!("0");
    input["years"][0]["statutory"]["balance_sheet"]["fixed_assets"] = json!("1020000");
    input["years"][0]["statutory"]["balance_sheet"]["total_assets"] = json!("2700000");
    input["years"][0]["statutory"]["balance_sheet"]["equity"] = json!({"total": "750000"});
    input["years"][0]["statutory"]["balance_sheet"]["total_liabilities_and_equity"] =
        json!("2700000");
    input["years"][0]["statutory"]["income_statement"] = json!({});
    input["treatments"]
        .as_object_mut()
        .unwrap()
        .remove("financial_fixed_assets");
    let out = run(&input);
    let conv = conventions(&out);
    for text in [
        "no date given: the order of `years` is taken as oldest first",
        "financial fixed assets (B.III) are zero: no treatment is needed",
        "missing inputs are never taken as zero",
        "checks are exact, with no tolerance",
        "income statement by nature reorganised into the layers of Appendix B.2 (R16)",
        "measurement basis `book_value`",
    ] {
        assert!(conv.contains(text), "missing '{text}' in {conv}");
    }
}

/// AC13, review of PR 1 (item 7): every row of the conventions table that applies to a call
/// appears in `explain.conventions`: a statutory year with its layout, item A and the "of which"
/// parts not given, the R15 readings, an adjustment whose source is missing, and a reclassified
/// year.
#[test]
fn every_applicable_convention_is_stated() {
    let mut statutory = golden_input("balance-sheet-ordinary")["years"][0].clone();
    statutory["label"] = json!("2023");
    statutory["date"] = json!("2023-12-31");
    let bs = statutory["statutory"]["balance_sheet"]
        .as_object_mut()
        .unwrap();
    bs.remove("cash");
    bs.remove("subscribed_capital_unpaid");
    bs.remove("current_assets");
    bs.remove("total_assets");
    let reclassified = golden_input("reclassified-balance-sheet")["years"][0].clone();
    let input = json!({
        "years": [statutory, reclassified],
        "treatments": golden_input("balance-sheet-ordinary")["treatments"],
        "adjustments": [{"kind": "operating_cash", "label": "Minimum cash",
            "reason": "Operations", "amounts": ["10", null]}],
    });
    let out = run(&input);
    let conv = conventions(&out);
    let b = "/years/0/statutory/balance_sheet";
    for text in [
        // Years, forms and layout (R2, Decision 1).
        "year 2023: statutory form",
        "year 2024: reclassified form",
        "year 2023: layout `ordinary` echoed; no number depends on it",
        // Best effort, checks and signs (R3, R6, R7).
        "missing inputs are never taken as zero",
        "checks are exact, with no tolerance",
        "amounts as presented in the statutory layouts",
        // Defaults (R8, D1, D2, R17).
        &format!("default: `{b}/subscribed_capital_unpaid` not given, set to"),
        &format!("default: `{b}/receivables/prepayments_and_accrued_income` not given, set to"),
        &format!("default: `{b}/provisions` assigned to"),
        "default: `/treatments/write_downs_of_receivables` not given, set to",
        "deducted from equity (Decision 14); not given, it is zero (D1)",
        // Net financial position and bridge items (Decisions 11, 15; D3; SPEC-0007).
        "net financial position = gross debt − cash + other items",
        "C.III and all of C.IV inside, operating cash out only by an `operating_cash` \
         adjustment (Decision 11)",
        "measurement basis `book_value` (Decision 15)",
        "are non-current in the net financial position (D3)",
        "derivative assets are not given separately and follow their aggregate (D3)",
        // Current and non-current (R15).
        "read as twelve months (R15)",
        "year 2023: provisions (B) read as non-current: the part beyond one year is not given",
        "year 2023: employee severance indemnity (C) read as non-current",
        "year 2023: prepayments and accrued income (assets D) read as current",
        "year 2023: accruals and deferred income (liabilities E) read as current",
        // Adjustments (R10) and the reclassified form (R5).
        &format!(
            "adjustment 0 'Minimum cash' not applied to 2023: its source `{b}/cash` is missing"
        ),
        "treatments and adjustments do not apply to the reclassified entries: 2024",
        "year 2024: the current and non-current block (except inventories)",
        // The income statement (R16 to R20, Decision 17).
        "income statement by nature reorganised into the layers of Appendix B.2 (R16)",
        "D18, D19, B10 c and B10 d in the investing result",
        "A5 and B14 are core by default (Decision 7, R18)",
        "income taxes: item 20 as given",
        "cost_of_sales is not returned (Decision 13)",
        "year 2024: the reconciliations of the income statement with A − B need the statutory \
         form",
    ] {
        assert!(conv.contains(text), "missing '{text}' in {conv}");
    }
}

/// Review of PR 1 (nit): an entry with no form is not a statutory entry, so the conventions of
/// the statutory form are not stated for it.
#[test]
fn an_entry_with_no_form_states_no_statutory_convention() {
    let out = run(&json!({"years": [{"label": "2024"}]}));
    let conv = conventions(&out);
    for text in [
        "amounts as presented in the statutory layouts",
        "income statement by nature reorganised",
        "(Decision 11)",
    ] {
        assert!(!conv.contains(text), "'{text}' in {conv}");
    }
}

#[test]
fn an_adjustment_that_does_not_apply_to_a_year_is_reported() {
    let out = run(&golden_input("adjustment-dividends-declared"));
    let conv = conventions(&out);
    assert!(
        conv.contains("adjustment 0 'Dividend declared on the 2024 profit' does not apply to 2023"),
        "{conv}"
    );
}

// ---------------------------------------------------------------- income statement (PR 2)

/// AC10, R16, R19: the layer identities of Appendix B.2 and the reconciliations of R19 in every
/// statutory year of `out` whose amounts are known; without a lease, profit before tax equals
/// A − B ± C ± D and net income equals item 21 when given.
fn check_income_statement(out: &Value, input: &Value) {
    let leases = input["adjustments"]
        .as_array()
        .is_some_and(|a| a.iter().any(|x| x["kind"] == "lease"));
    for (i, y) in out["value"]["years"].as_array().unwrap().iter().enumerate() {
        let is = &y["income_statement"];
        let k = |key: &str| (!is[key].is_null()).then(|| dec(&is[key]));
        let all = |keys: &[&str]| keys.iter().map(|x| k(x)).collect::<Option<Vec<_>>>();
        if let Some(v) = all(&["value_added", "personnel_costs", "ebitda"]) {
            assert_eq!(v[0] - v[1], v[2], "EBITDA");
        }
        if let Some(v) = all(&[
            "operating_profit",
            "investing_result",
            "profit_before_financing_and_taxes",
        ]) {
            assert_eq!(v[0] + v[1], v[2], "profit before financing and taxes");
        }
        if let Some(v) = all(&[
            "profit_before_financing_and_taxes",
            "non_recurring_result",
            "financing_result",
            "profit_before_tax",
        ]) {
            assert_eq!(v[0] + v[1] + v[2], v[3], "profit before tax");
        }
        if let Some(v) = all(&["profit_before_tax", "income_taxes", "net_income"]) {
            assert_eq!(v[0] - v[1], v[2], "net income");
        }
        for (name, end) in [
            ("operating_profit", "operating_profit"),
            (
                "profit_before_financing_and_taxes",
                "profit_before_financing_and_taxes",
            ),
        ] {
            let r = &is["reconciliations"][name];
            if r.is_null() {
                continue;
            }
            let lines: Decimal = r["lines"]
                .as_array()
                .unwrap()
                .iter()
                .map(|l| dec(&l["amount"]))
                .sum();
            assert_eq!(dec(&r["start"]) + lines, dec(&r["end"]), "{name}");
            assert_eq!(dec(&r["end"]), dec(&is[end]), "{name}");
        }
        let st = &input["years"][i]["statutory"]["income_statement"];
        if leases || st.is_null() {
            continue;
        }
        for (given, output) in [
            ("profit_before_tax", "profit_before_tax"),
            ("profit_or_loss_for_the_year", "net_income"),
        ] {
            if let (false, Some(v)) = (st[given].is_null(), k(output)) {
                assert_eq!(dec(&st[given]), v, "{output} against {given}");
            }
        }
    }
}

/// A statutory income statement from random amounts, its totals consistent with its items,
/// some totals left out, with random adjustments within their sources.
fn random_income_statement(a: &[i64], choices: &[usize]) -> Value {
    let s = |x: i64| x.to_string();
    let (a1, a23, a4, a5) = (a[0], a[1] - 50_000, a[2], a[3]);
    let (b6, b7, b8, b9, ab, c10, d10) = (a[4], a[5], a[6], a[7], a[8], a[9], a[10]);
    let (b11, b12, b13, b14) = (a[11] - 40_000, a[12], a[13], a[14]);
    let (c15, c16, c17, c17b) = (a[15], a[16], a[17], a[18] - 30_000);
    let (d18, d19, taxes) = (a[19], a[20], a[21] - 20_000);
    let total_a = a1 + a23 + a4 + a5;
    let total_b = b6 + b7 + b8 + b9 + ab + c10 + d10 + b11 + b12 + b13 + b14;
    let total_c = c15 + c16 - c17 + c17b;
    let total_d = d18 - d19;
    let pbt = total_a - total_b + total_c + total_d;
    let mut is = json!({
        "revenue": s(a1), "change_in_product_inventories": s(a23),
        "own_work_capitalised": s(a4), "other_revenue_and_income": s(a5),
        "purchases": s(b6), "services": s(b7), "use_of_third_party_assets": s(b8),
        "personnel_costs": s(b9),
        "depreciation_and_write_downs": {"total": s(ab + c10 + d10),
            "value_adjustments_of_fixed_assets": s(ab + c10),
            "depreciation_and_amortisation": s(ab), "write_downs_of_fixed_assets": s(c10),
            "write_downs_of_receivables": s(d10)},
        "change_in_raw_material_inventories": s(b11), "provisions_for_risks": s(b12),
        "other_provisions": s(b13), "other_operating_expenses": s(b14),
        "income_from_equity_investments": s(c15), "other_financial_income": s(c16),
        "interest_and_other_financial_charges": s(c17), "exchange_gains_and_losses": s(c17b),
        "revaluations_of_financial_assets": s(d18), "write_downs_of_financial_assets": s(d19),
        "value_of_production": s(total_a), "costs_of_production": s(total_b),
        "difference_between_value_and_costs_of_production": s(total_a - total_b),
        "financial_income_and_charges": s(total_c),
        "value_adjustments_of_financial_assets": s(total_d),
        "profit_before_tax": s(pbt), "income_taxes": s(taxes),
        "profit_or_loss_for_the_year": s(pbt - taxes),
    });
    let m = is.as_object_mut().unwrap();
    for (j, key) in [
        "value_of_production",
        "costs_of_production",
        "difference_between_value_and_costs_of_production",
        "financial_income_and_charges",
        "value_adjustments_of_financial_assets",
        "profit_before_tax",
    ]
    .iter()
    .enumerate()
    {
        if choices[j].is_multiple_of(3) {
            m.remove(*key);
        }
    }
    let part = |x: i64, c: usize| x.max(0) * (choices[c] as i64 % 60) / 100;
    let nr_sources = [
        ("revenue", a1),
        ("services", b7),
        ("personnel_costs", b9),
        ("write_downs_of_fixed_assets", c10),
        ("write_downs_of_receivables", d10),
        ("provisions_for_risks", b12),
        ("income_from_equity_investments", c15),
        ("interest_and_other_financial_charges", c17),
    ];
    let (nr_key, nr_amount) = nr_sources[choices[6] % nr_sources.len()];
    let mut treatments = json!({});
    if choices[7] % 3 == 1 {
        treatments["write_downs_of_receivables"] = json!("operating");
    }
    json!({
        "years": [{"statutory": {"income_statement": is}}],
        "treatments": treatments,
        "adjustments": [
            {"kind": "investing", "label": "Rent", "reason": "Investment property",
             "aggregate": "other_revenue_and_income", "amounts": [s(part(a5, 8))]},
            {"kind": "investing", "label": "Upkeep", "reason": "Investment property",
             "aggregate": "other_operating_expenses", "amounts": [s(part(b14, 9))]},
            {"kind": "non_recurring", "label": "One-off", "reason": "art. 2427 n. 13",
             "aggregate": nr_key, "amounts": [s(part(nr_amount, 10))]},
        ]
    })
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(64))]

    /// AC10: on random income statements with adjustments, profit before financing and taxes =
    /// operating profit + investing result, profit before tax = A − B ± C ± D, net income =
    /// item 21, and the reconciliations add up to their subtotals from A − B.
    #[test]
    fn income_statement_identities_hold(
        a in proptest::collection::vec(0i64..3_000_000, 22),
        choices in proptest::collection::vec(0usize..1000, 11),
    ) {
        let input = random_income_statement(&a, &choices);
        let out = run(&input);
        let is = &out["value"]["years"][0]["income_statement"];
        prop_assert!(!is["net_income"].is_null());
        prop_assert!(!is["reconciliations"]["operating_profit"].is_null());
        check_income_statement(&out, &input);
    }

    /// AC11, R17: the treatment `operating` moves B10 d from the investing result to operating
    /// profit: EBITDA and profit before financing and taxes are the same either way, and
    /// operating profit is lower by B10 d (core).
    #[test]
    fn the_operating_treatment_moves_only_b10_d(
        a in proptest::collection::vec(0i64..3_000_000, 22),
        choices in proptest::collection::vec(0usize..1000, 11),
    ) {
        let mut input = random_income_statement(&a, &choices);
        input["treatments"] = json!({"write_downs_of_receivables": "investing"});
        let inv = run(&input);
        input["treatments"] = json!({"write_downs_of_receivables": "operating"});
        let op = run(&input);
        let (x, y) = (&inv["value"]["years"][0]["income_statement"],
                      &op["value"]["years"][0]["income_statement"]);
        prop_assert_eq!(&x["ebitda"], &y["ebitda"]);
        prop_assert_eq!(&x["profit_before_financing_and_taxes"],
                        &y["profit_before_financing_and_taxes"]);
        let d10 = dec(&input["years"][0]["statutory"]["income_statement"]
            ["depreciation_and_write_downs"]["write_downs_of_receivables"]);
        let moved: Decimal = input["adjustments"].as_array().unwrap().iter()
            .filter(|a| a["aggregate"] == "write_downs_of_receivables")
            .map(|a| dec(&a["amounts"][0])).sum();
        prop_assert_eq!(dec(&x["operating_profit"]) - dec(&y["operating_profit"]), d10 - moved);
    }
}

/// AC9, R20: the income statement outputs feed the methods that take them under the same
/// names: `operating_profit` and `depreciation_and_amortisation` as `FreeCashFlowLines`;
/// revenue, EBITDA, net income and invested capital in `ratios.profitability` with the returned
/// `ebitda_definition`; profit before financing and taxes and interest in `ratios.debt_service`;
/// revenue and purchases in `ratios.working_capital`, whose cost of sales the analyst gives. No
/// `cost_of_sales` is returned (Decision 13).
#[test]
fn income_statement_outputs_feed_the_methods_that_take_them() {
    let out = run(&golden_input("ordinary-aggregates"));
    let y = &out["value"]["years"][0];
    let is = &y["income_statement"];
    assert!(is.get("cost_of_sales").is_none());
    assert!(!out["value"].to_string().contains("cost_of_sales"));
    let fcf = json!({"periods": [{
        "ebit": is["operating_profit"], "operating_taxes": "90000",
        "depreciation_and_amortisation": is["depreciation_and_amortisation"],
        "capital_expenditure": "150000", "change_in_net_working_capital": "20000"}]});
    let v = fairmark_core::run("dcf.free_cash_flows", &fcf.to_string()).unwrap();
    let flow = dec(&v["value"]["periods"][0]["free_cash_flow_to_firm"]);
    assert_eq!(
        flow,
        Decimal::from(301000 - 90000 + 180000 - 150000 - 20000)
    );
    let profitability = json!({"balances": "closing", "operating_return_base": "invested_capital",
        "ebitda_definition": is["ebitda_definition"],
        "income_statement": {"revenue": is["revenue"], "ebitda": is["ebitda"],
            "ebit": is["operating_profit"], "net_income": is["net_income"]},
        "closing": {"equity": y["balance_sheet"]["sources"]["equity"],
            "total_assets": y["current_non_current"]["total_assets"],
            "invested_capital": y["balance_sheet"]["operating"]["invested_capital"]}});
    let p = fairmark_core::run("ratios.profitability", &profitability.to_string()).unwrap();
    assert!(conventions(&p).contains("EBITDA as given: EBITDA (margine operativo lordo"));
    let debt = json!({"balances": "closing", "ebitda_definition": is["ebitda_definition"],
        "interest_definition": "C17 interest and other financial charges",
        "income_statement": {"ebitda": is["ebitda"],
            "ebit": is["profit_before_financing_and_taxes"],
            "interest": is["interest_and_other_financial_charges"]},
        "closing": {"net_financial_position":
            y["balance_sheet"]["sources"]["net_financial_position"]["position"]}});
    let d = fairmark_core::run("ratios.debt_service", &debt.to_string()).unwrap();
    assert_eq!(
        dec(&d["value"]["interest_cover_ebit"]["value"]),
        Decimal::from(275000) / Decimal::from(60000)
    );
    // Decision 13: an income statement by nature has no cost of sales; the analyst gives it.
    let wc = json!({"balances": "closing", "day_basis": "365",
        "income_statement": {"revenue": is["revenue"], "purchases": is["purchases"],
            "cost_of_sales": "3000000"},
        "closing": {"trade_receivables": "800000", "trade_payables": "500000",
            "inventories": y["current_non_current"]["inventories"]}});
    fairmark_core::run("ratios.working_capital", &wc.to_string()).unwrap();
}

/// AC2, R6: in `few-totals`, every null output of the income statement is listed in
/// `not_computed`, no listed output has a value, and the outputs computed are exactly those the
/// totals given allow.
#[test]
fn few_totals_lists_every_income_statement_output_not_computed() {
    let out = run(&golden_input("few-totals"));
    let y = &out["value"]["years"][0];
    let listed: Vec<String> = y["not_computed"]["income_statement"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["output"].as_str().unwrap().to_owned())
        .collect();
    let mut nulls = Vec::new();
    let mut known = Vec::new();
    let is = y["income_statement"].as_object().unwrap();
    for (k, v) in is {
        if k == "reconciliations" {
            for (r, x) in v.as_object().unwrap() {
                assert!(x.is_null(), "{r}");
                nulls.push(format!("/years/0/income_statement/reconciliations/{r}"));
            }
        } else if v.is_null() {
            nulls.push(format!("/years/0/income_statement/{k}"));
        } else {
            known.push(k.as_str());
        }
    }
    nulls.sort();
    assert_eq!(nulls, listed);
    known.sort_unstable();
    assert_eq!(
        known,
        [
            "ebitda_definition",
            "non_recurring_result",
            "personnel_costs",
            "revenue"
        ]
    );
}

/// AC12, R12, R21: a lease restatement keeps total funds invested equal to the sources, states
/// its equity effect with its formula, and lists each of its amounts in `adjustments_applied`.
#[test]
fn a_lease_restatement_keeps_the_identity_and_shows_its_lines() {
    let input = golden_input("adjustment-lease");
    let out = run(&input);
    check_identities(&out);
    check_income_statement(&out, &input);
    let s = step(
        &out,
        "Lease equity effect, adjustment 0 'Head office lease' (2024)",
    );
    assert_eq!(
        s["formula"],
        "right-of-use asset 240000 − lease liability 200000 − prepayment removed 10000"
    );
    assert_eq!(s["value"], "30000");
    let lines: Vec<(&str, &str)> = out["value"]["years"][0]["adjustments_applied"]
        .as_array()
        .unwrap()
        .iter()
        .map(|a| (a["line"].as_str().unwrap(), a["target"].as_str().unwrap()))
        .collect();
    assert_eq!(
        lines,
        [
            ("asset", "operating_fixed_assets"),
            (
                "liability",
                "net_financial_position_other_financial_liability"
            ),
            ("prepayment", "equity"),
            ("equity_effect", "equity"),
            ("payments", "removed_by_restatement"),
            ("depreciation", "operating_profit"),
            ("interest", "financing_result"),
        ]
    );
    let conv = conventions(&out);
    assert!(conv.contains("no deferred taxes"), "{conv}");
}

/// R21: with assets D presented within C.II (abridged layout), a lease prepayment leaves C.II;
/// the identity of R12 holds and a prepayment larger than that part is `FM-E026`.
#[test]
fn a_lease_prepayment_within_receivables_leaves_receivables() {
    let mut input = golden_input("abridged-aggregates");
    input["adjustments"] = json!([{"kind": "lease", "label": "Head office lease",
        "reason": "Financial method", "amounts": [{"asset": "240000", "liability": "200000",
        "liability_current": "50000", "prepayment": "10000", "payments": "60000",
        "depreciation": "40000", "interest": "8000"}]}]);
    let out = run(&input);
    check_identities(&out);
    let bs = &out["value"]["years"][0]["balance_sheet"];
    assert_eq!(bs["operating"]["operating_working_capital"], "670000");
    assert_eq!(bs["sources"]["equity"], "930000");
    input["adjustments"][0]["amounts"][0]["prepayment"] = json!("30001");
    let e = err(&input);
    assert_eq!(e.code(), "FM-E026");
    assert!(
        e.to_string().contains(
            "/years/0/statutory/balance_sheet/receivables/prepayments_and_accrued_income"
        ),
        "{e}"
    );
}

/// R5: a reclassified income statement lists its reconciliations with no pointer to give (they
/// need the statutory form), and its derived totals are steps with their terms substituted.
#[test]
fn a_reclassified_income_statement_derives_and_explains_its_totals() {
    let mut input = golden_input("reclassified-income-statement");
    let is = input["years"][0]["reclassified"]["income_statement"]
        .as_object_mut()
        .unwrap();
    for k in [
        "ebitda",
        "profit_before_financing_and_taxes",
        "profit_before_tax",
        "net_income",
    ] {
        is.remove(k);
    }
    let out = run(&input);
    let y = &out["value"]["years"][0];
    assert_eq!(y["income_statement"]["net_income"], "150000");
    let s = step(&out, "Net income from its components (2024)");
    assert_eq!(
        s["formula"],
        "profit before tax 214000 − income taxes 64000"
    );
    for e in y["not_computed"]["income_statement"].as_array().unwrap() {
        assert!(
            e["output"]
                .as_str()
                .unwrap()
                .contains("/income_statement/reconciliations/"),
            "{e}"
        );
        assert!(e["missing"].as_array().unwrap().is_empty(), "{e}");
    }
}

/// R11: every aggregate of the income statement given is assigned once, to its layer, and the
/// assigned amounts of the ordinary company are the amounts given.
#[test]
fn every_income_statement_aggregate_given_is_assigned_to_its_layer() {
    let out = run(&golden_input("ordinary-aggregates"));
    let assignments = out["value"]["years"][0]["assignments"].as_array().unwrap();
    let of = |r: &str| {
        assignments
            .iter()
            .find(|a| a["reference"] == r)
            .unwrap_or_else(|| panic!("{r}"))
    };
    for (reference, destination, amount) in [
        ("A5", "value_of_production", "50000"),
        ("B9", "ebitda", "1000000"),
        ("B10 a, b", "operating_profit", "180000"),
        ("B10 c", "investing_result", "20000"),
        ("B10 d", "investing_result", "15000"),
        ("B14", "value_added", "40000"),
        ("C15", "investing_result", "12000"),
        ("C17-bis", "financing_result", "-1000"),
        ("D19", "investing_result", "8000"),
        ("20", "income_taxes", "64000"),
    ] {
        let a = of(reference);
        assert_eq!(a["destination"], destination, "{reference}");
        assert_eq!(a["amount"], amount, "{reference}");
        assert_eq!(a["origin"], "mapping", "{reference}");
    }
}
