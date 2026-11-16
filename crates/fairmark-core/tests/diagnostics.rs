//! Outputs not computed, checks and defaults in the envelope of SPEC-0012 (R5 to R7, R10, R12;
//! AC4 to AC6, AC8, AC9, AC17). The inputs are those of golden cases, read from their files, or
//! written here.

use std::path::Path;

use serde_json::{json, Value};

fn golden(case: &str) -> (String, Value) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(case);
    let case: Value = serde_norway::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    (
        case["method"].as_str().unwrap().to_owned(),
        case["input"].clone(),
    )
}

fn run(method: &str, input: &Value) -> Value {
    fairmark_core::run(method, &input.to_string()).unwrap()
}

fn run_golden(case: &str) -> Value {
    let (method, input) = golden(case);
    run(&method, &input)
}

/// The entries of `diagnostics.not_computed` at `pointer`.
fn entries<'a>(out: &'a Value, pointer: &str) -> Vec<&'a Value> {
    out["diagnostics"]["not_computed"]
        .as_array()
        .unwrap_or_else(|| panic!("no diagnostics.not_computed in {out}"))
        .iter()
        .filter(|e| e["pointer"] == pointer)
        .collect()
}

/// The one entry at `pointer`.
fn entry<'a>(out: &'a Value, pointer: &str) -> &'a Value {
    let found = entries(out, pointer);
    assert_eq!(found.len(), 1, "{pointer}: {found:?}");
    found[0]
}

fn keys(v: &Value) -> Vec<&str> {
    v.as_object()
        .map(|m| m.keys().map(String::as_str).collect())
        .unwrap_or_default()
}

/// AC4 (R5, R6): the inputs `ratios.debt_service` lacks for the NFP / EBITDA ratio are the one
/// alternative of `missing_any_of`, and `value` has no list.
#[test]
#[ignore = "SPEC-0012: enabled by T13, ratios v3"]
fn a_ratio_not_computed_is_in_the_diagnostics() {
    let out = run(
        "ratios.debt_service",
        &json!({"balances": "closing", "interest_definition": "gross interest expense",
                "income_statement": {"ebit": "500", "interest": "100"}}),
    );
    assert!(out["value"].get("not_computed").is_none());
    let e = entry(&out, "/value/nfp_to_ebitda");
    assert_eq!(
        keys(e),
        [
            "pointer",
            "reason_code",
            "reason",
            "missing_any_of",
            "permitted"
        ]
    );
    assert_eq!(e["reason_code"], "missing_input");
    assert_eq!(
        e["missing_any_of"],
        json!([[
            "/closing/net_financial_position/cash",
            "/closing/net_financial_position/gross_debt",
            "/ebitda_definition",
            "/income_statement/ebitda"
        ]])
    );
    assert_eq!(e["permitted"], json!([]));
    assert!(e["reason"].as_str().is_some_and(|r| !r.is_empty()));
    assert_eq!(out["value"]["nfp_to_ebitda"], json!({"value": null}));
}

/// AC5 (R5, R6): a ratio a given term makes not defined is the object `{value}` with a null
/// value, and its reason is a `not_defined` entry.
#[test]
#[ignore = "SPEC-0012: enabled by T13, ratios v3"]
fn a_ratio_not_defined_is_in_the_diagnostics() {
    let out = run_golden("ratios.profitability/negative-equity.yaml");
    assert_eq!(out["value"]["roe"], json!({"value": null}));
    let e = entry(&out, "/value/roe");
    assert_eq!(e["reason_code"], "not_defined");
    assert_eq!(e["reason"], "equity is negative (-100)");
    assert_eq!(e["missing_any_of"], json!([]));
    assert_eq!(e["permitted"], json!([]));
}

/// AC17 (R5, decision 14): an output both not defined and missing an input has one entry,
/// `missing_input`, with the missing pointers of today.
#[test]
#[ignore = "SPEC-0012: enabled by T13, ratios v3"]
fn missing_input_comes_before_not_defined() {
    let out = run_golden("ratios.debt_service/no-debt-schedule.yaml");
    let e = entry(&out, "/value/llcr");
    assert_eq!(e["reason_code"], "missing_input");
    assert_eq!(e["missing_any_of"], json!([["/debt_schedule"]]));
    let out = run_golden("ratios.profitability/negative-invested-capital.yaml");
    for pointer in [
        "/value/roe_leverage/roe",
        "/value/roe_leverage/cost_of_debt",
    ] {
        let e = entry(&out, pointer);
        assert_eq!(e["reason_code"], "missing_input", "{pointer}");
        assert_eq!(e["missing_any_of"], json!([["/leverage"]]), "{pointer}");
    }
    // The ratio not defined by a given term keeps its entry.
    assert_eq!(entry(&out, "/value/roi")["reason_code"], "not_defined");
}

/// R13: the outputs not computed are ordered by pointer, array indices as numbers.
#[test]
#[ignore = "SPEC-0012: enabled by T13, ratios v3"]
fn not_computed_is_ordered_by_pointer() {
    let out = run_golden("ratios.debt_service/partial-schedule.yaml");
    let pointers: Vec<&str> = out["diagnostics"]["not_computed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["pointer"].as_str().unwrap())
        .collect();
    assert!(!pointers.is_empty());
    let numeric = |p: &str| -> Vec<(u8, u64, String)> {
        p.split('/')
            .map(|t| match t.parse::<u64>() {
                Ok(n) if t.bytes().all(|c| c.is_ascii_digit()) => (0, n, String::new()),
                _ => (1, 0, t.to_owned()),
            })
            .collect()
    };
    let mut sorted = pointers.clone();
    sorted.sort_by_key(|p| numeric(p));
    assert_eq!(pointers, sorted);
}

/// The outputs C15 reaches when the treatment of B.III is missing (SPEC-0005 P2, R8).
const REACHED_BY_C15: [&str; 8] = [
    "/value/years/0/income_statement/income_from_operating_investments",
    "/value/years/0/income_statement/investing_result",
    "/value/years/0/income_statement/net_income",
    "/value/years/0/income_statement/operating_profit",
    "/value/years/0/income_statement/profit_before_financing_and_taxes",
    "/value/years/0/income_statement/profit_before_tax",
    "/value/years/0/income_statement/reconciliations/operating_profit",
    "/value/years/0/income_statement/reconciliations/profit_before_financing_and_taxes",
];

/// AC6 (R5, R7, R12): a missing treatment is a choice to make, an output the form given cannot
/// supply is not applicable, every relation of every year is a check located at its year, and
/// the year holds neither list.
#[test]
#[ignore = "SPEC-0012: enabled by T14, statements v2"]
fn statements_report_choices_checks_and_outputs_not_applicable() {
    let out = run_golden("statements.reorganise/operating-investments-missing-treatment.yaml");
    for pointer in REACHED_BY_C15 {
        let e = entry(&out, pointer);
        assert_eq!(e["reason_code"], "choice_required", "{pointer}");
        assert_eq!(
            e["permitted"],
            json!([{"pointer": "/treatments/financial_fixed_assets",
                    "values": ["operating", "non_operating", "financial_asset"]}]),
            "{pointer}"
        );
        assert_eq!(
            e["missing_any_of"],
            json!([["/treatments/financial_fixed_assets"]]),
            "{pointer}"
        );
    }
    let year = &out["value"]["years"][0];
    assert!(year.get("checks").is_none() && year.get("not_computed").is_none());
    let assignment = &year["assignments"][0];
    assert_eq!(
        assignment["aggregate_pointer"],
        "/years/0/statutory/balance_sheet/subscribed_capital_unpaid"
    );
    assert!(assignment.get("aggregate").is_none());
    let checks = out["diagnostics"]["checks"].as_array().unwrap();
    assert_eq!(checks.len(), 35);
    for c in checks {
        assert_eq!(keys(c), ["id", "status", "pointer", "pointers"]);
        assert_eq!(c["pointer"], "/value/years/0");
    }

    let out = run_golden("statements.reorganise/reclassified-balance-sheet.yaml");
    let e = entry(&out, "/value/years/0/current_non_current/cash");
    assert_eq!(e["reason_code"], "not_applicable");
    assert_eq!(e["missing_any_of"], json!([]));
    assert!(e["reason"].as_str().is_some_and(|r| !r.is_empty()));
}

/// R7: the checks of every year, ordered by year, then by relation id.
#[test]
#[ignore = "SPEC-0012: enabled by T14, statements v2"]
fn checks_are_ordered_by_year_then_id() {
    let out = run_golden("statements.reorganise/treatment-once-three-years.yaml");
    let checks: Vec<(String, String)> = out["diagnostics"]["checks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| {
            (
                c["pointer"].as_str().unwrap().to_owned(),
                c["id"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let years: std::collections::BTreeSet<&str> = checks.iter().map(|(p, _)| p.as_str()).collect();
    assert_eq!(
        years.into_iter().collect::<Vec<_>>(),
        ["/value/years/0", "/value/years/1", "/value/years/2"]
    );
    let mut sorted = checks.clone();
    sorted.sort();
    assert_eq!(checks, sorted);
}

/// The renamed pointer fields of `statements.reorganise` v2 (R12).
#[test]
#[ignore = "SPEC-0012: enabled by T14, statements v2"]
fn statements_pointer_fields_follow_g4() {
    let out = run_golden("statements.reorganise/adjustment-lease.yaml");
    let year = &out["value"]["years"][0];
    let applied = &year["adjustments_applied"][0];
    assert!(applied["source_pointer"].is_string() && applied.get("source").is_none());
    let composition = year["balance_sheet"]["sources"]["net_financial_position"]["composition"]
        .as_array()
        .unwrap();
    assert!(composition
        .iter()
        .all(|c| c["source_pointer"].is_string() && c.get("source").is_none()));
    let lines = year["income_statement"]["reconciliations"]["operating_profit"]["lines"]
        .as_array()
        .unwrap();
    assert!(lines
        .iter()
        .all(|l| l["source_pointers"].is_array() && l.get("sources").is_none()));
}

/// The dcf.fcff input of the registry tests, without a net financial position.
fn dcf_without_nfp() -> Value {
    json!({"stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}],
           "cash_flow_terms": "nominal", "discount_rate_terms": "nominal", "wacc": "0.09",
           "timing": "end_of_period",
           "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}})
}

/// AC8 (R8, R10): the default zero of the net financial position is listed with its pointer
/// and warned; a net financial position given as zero is neither.
#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn the_default_net_financial_position_is_listed_and_warned() {
    let out = run("dcf.fcff", &dcf_without_nfp());
    let defaults = out["meta"]["defaults_applied"].as_array().unwrap();
    let d = defaults
        .iter()
        .find(|d| d["pointer"] == "/net_financial_position")
        .unwrap();
    assert_eq!(keys(d), ["pointer", "value", "reason"]);
    assert_eq!(d["value"], "0");
    let warned = |out: &Value| {
        out["diagnostics"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "FM-W012" && w["pointer"] == "/net_financial_position")
    };
    assert!(warned(&out));

    let mut given = dcf_without_nfp();
    given["net_financial_position"] = json!({"gross_debt": 0, "cash": 0});
    let out = run("dcf.fcff", &given);
    assert!(!out["meta"]["defaults_applied"]
        .as_array()
        .unwrap()
        .iter()
        .any(|d| d["pointer"] == "/net_financial_position"));
    assert!(!warned(&out));
}

/// AC9 (R10): no default of any golden result has the key `field`; each is `{pointer, value,
/// reason}`.
#[test]
#[ignore = "SPEC-0012: enabled by T14, which renames the key with the statements migration"]
fn defaults_name_their_field_pointer() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut seen = 0u32;
    let mut failures = Vec::new();
    for dir in std::fs::read_dir(&root).unwrap() {
        for file in std::fs::read_dir(dir.unwrap().path()).unwrap() {
            let file = file.unwrap().path();
            let case: Value =
                serde_norway::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            if case.get("expect_error").is_some() {
                continue;
            }
            let out = run(case["method"].as_str().unwrap(), &case["input"]);
            for d in out["meta"]["defaults_applied"].as_array().unwrap() {
                seen += 1;
                if keys(d) != ["pointer", "value", "reason"] {
                    failures.push(format!("{}: {d}", file.display()));
                }
            }
        }
    }
    assert!(seen > 100, "{seen}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
