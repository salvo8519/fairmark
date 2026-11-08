//! `statements.reorganise` (SPEC-0005), first pull request: contract tests with the methods that
//! take its outputs (AC9, balance sheet part), error messages (R24), determinism and the schema
//! (AC13), `meta.defaults_applied` (R8), exhaustiveness of `not_computed` (AC2) and the
//! properties AC7 and AC8 on random balanced statements.

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

/// Between the pull requests of SPEC-0005 (plan, "Behaviour between pull requests"): an income
/// statement adjustment is refused, never ignored.
#[test]
fn income_statement_adjustments_are_refused_in_this_build() {
    for adj in [
        json!({"kind": "investing", "label": "Rent", "reason": "Investment property",
               "aggregate": "other_revenue_and_income", "amounts": ["1000"]}),
        json!({"kind": "non_recurring", "label": "Gain", "reason": "Disposal",
               "aggregate": "revenue", "amounts": ["1000"]}),
        json!({"kind": "lease", "label": "Head office", "reason": "IFRS 16 view",
               "amounts": [{"asset": "1", "liability": "1", "liability_current": "0",
                            "prepayment": "0", "payments": "0", "depreciation": "0",
                            "interest": "0"}]}),
    ] {
        let mut input = golden_input("balance-sheet-ordinary");
        input["adjustments"] = json!([adj]);
        let e = err(&input);
        assert_eq!(e.code(), "FM-E001");
        let msg = e.to_string();
        assert!(msg.contains("/adjustments/0"), "{msg}");
        assert!(msg.contains("not implemented in this build"), "{msg}");
    }
}

#[test]
fn a_non_recurring_adjustment_of_d19_is_refused() {
    let mut input = golden_input("balance-sheet-ordinary");
    input["adjustments"] = json!([{"kind": "non_recurring", "label": "Impairment",
        "reason": "One-off", "aggregate": "write_downs_of_financial_assets", "amounts": ["1"]}]);
    assert_eq!(err(&input).code(), "FM-E001");
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

/// Review of PR 1: a relation is compared only on items the user gave. Without total assets
/// and without a separate line D (assets D given within C.II), total assets rebuilt from
/// A + B + C + D would rest on a line taken as zero: the relation is not checked.
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
    for relation in [
        "total_assets_total",
        "total_assets_equals_total_liabilities_and_equity",
    ] {
        assert_eq!(checks[relation]["status"], "not_checked", "{relation}");
    }
    assert_eq!(
        out["value"]["years"][0]["balance_sheet"]["total_funds_invested"],
        "1610000"
    );
}

/// With the same presentation but a receivables total that leaves out assets D, the inputs
/// given do not balance: `FM-E026` names the line taken as zero, never a total not given.
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
    assert!(
        !msg.contains(&format!("total assets (`{b}/total_assets`")),
        "{msg}"
    );
    assert!(
        msg.contains(&format!("`{b}/prepayments_and_accrued_income/total`")),
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

/// Review of PR 1 (item 9): the income statement is not reorganised in this build, so no
/// destination of A5, B14 or B10 d is listed; only the input default of the treatment is.
#[test]
fn no_income_statement_destination_is_listed_before_it_is_reorganised() {
    let input = json!({"years": [{"statutory": {"income_statement": {
        "other_revenue_and_income": "10", "other_operating_expenses": "5",
        "depreciation_and_write_downs": {"write_downs_of_receivables": "500"}}}}]});
    let out = run(&input);
    let fields: Vec<String> = out["meta"]["defaults_applied"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d["field"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(fields, ["/treatments/write_downs_of_receivables"]);
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
        "the income statement is not reorganised in this build",
        "measurement basis `book_value`",
    ] {
        assert!(conv.contains(text), "missing '{text}' in {conv}");
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
