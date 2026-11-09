//! SPEC-0007 R7 to R11, AC6: every row of the computability tables. For each method a complete
//! input is run, then one input at a time is removed: the outputs of the row become null and are
//! listed in `not_computed` with the pointer removed, and every other output is unchanged.

use serde_json::{json, Value};

/// How an output reads as null: a ratio object (`value` null, `not_defined` null) or an amount.
#[derive(Clone, Copy)]
enum Kind {
    Ratio,
    Amount,
}

struct Table {
    method: &'static str,
    input: Value,
    outputs: &'static [(&'static str, Kind)],
}

/// One row: the input removed, the pointer it leaves missing, the outputs not computed.
type Row<'a> = (&'a str, &'a str, &'a [&'a str]);

fn run(method: &str, input: &Value) -> Value {
    fairmark_core::run(method, &input.to_string())
        .unwrap_or_else(|e| panic!("{method}: {e} on {input}"))
}

fn remove(input: &Value, pointer: &str) -> Value {
    let mut out = input.clone();
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let target = if parent.is_empty() {
        &mut out
    } else {
        out.pointer_mut(parent).unwrap()
    };
    target
        .as_object_mut()
        .unwrap()
        .remove(key)
        .unwrap_or_else(|| panic!("{pointer} not in the input"));
    out
}

fn check(table: &Table, rows: &[Row<'_>]) {
    let full = run(table.method, &table.input);
    assert_eq!(full["value"]["not_computed"], json!([]), "{}", table.method);
    assert_eq!(full["meta"]["method_version"], "2", "{}", table.method);
    for (removed, missing, affected) in rows {
        let out = run(table.method, &remove(&table.input, removed));
        let listed: Vec<&str> = out["value"]["not_computed"]
            .as_array()
            .unwrap()
            .iter()
            .map(|e| e["output"].as_str().unwrap())
            .collect();
        assert_eq!(&listed, affected, "{} without {removed}", table.method);
        for entry in out["value"]["not_computed"].as_array().unwrap() {
            let pointers = entry["missing"].as_array().unwrap();
            assert!(
                pointers.iter().any(|p| p == missing),
                "{} without {removed}: {entry}",
                table.method
            );
            let mut sorted = pointers.clone();
            sorted.sort_by(|a, b| a.as_str().cmp(&b.as_str()));
            assert_eq!(&sorted, pointers, "missing pointers are sorted");
            assert!(entry["reason"].as_str().is_some_and(|r| !r.is_empty()));
            assert!(
                entry.get("permitted").is_none(),
                "no choice is ever missing"
            );
        }
        for (output, kind) in table.outputs {
            let got = out["value"].pointer(output).unwrap();
            if affected.contains(output) {
                match kind {
                    Kind::Ratio => assert!(got["value"].is_null(), "{output}: {got}"),
                    Kind::Amount => assert!(got.is_null(), "{output}: {got}"),
                }
            } else {
                assert_eq!(
                    got,
                    full["value"].pointer(output).unwrap(),
                    "{} without {removed}: {output} changed",
                    table.method
                );
            }
        }
    }
}

const PROFITABILITY: &[(&str, Kind)] = &[
    ("/roe", Kind::Ratio),
    ("/roi", Kind::Ratio),
    ("/roa", Kind::Ratio),
    ("/ros", Kind::Ratio),
    ("/ebitda_margin", Kind::Ratio),
    ("/ebit_margin", Kind::Ratio),
    ("/capital_turnover", Kind::Ratio),
    ("/roe_three_factor/net_margin", Kind::Ratio),
    ("/roe_three_factor/asset_turnover", Kind::Ratio),
    ("/roe_three_factor/equity_multiplier", Kind::Ratio),
    ("/roe_three_factor/roe", Kind::Ratio),
    ("/roe_leverage/roi", Kind::Ratio),
    ("/roe_leverage/cost_of_debt", Kind::Ratio),
    ("/roe_leverage/debt_to_equity", Kind::Ratio),
    ("/roe_leverage/leverage_effect", Kind::Ratio),
    ("/roe_leverage/pre_tax_roe", Kind::Ratio),
    ("/roe_leverage/tax_rate", Kind::Ratio),
    ("/roe_leverage/roe", Kind::Ratio),
];

const LEVERAGE_ALL: &[&str] = &[
    "/roe_leverage/roi",
    "/roe_leverage/cost_of_debt",
    "/roe_leverage/debt_to_equity",
    "/roe_leverage/leverage_effect",
    "/roe_leverage/pre_tax_roe",
    "/roe_leverage/tax_rate",
    "/roe_leverage/roe",
];

fn profitability(balances: &str) -> Table {
    let mut input = json!({
        "balances": balances,
        "operating_return_base": "invested_capital",
        "ebitda_definition": "reported EBITDA, no adjustments",
        "income_statement": {"revenue": "5000", "ebitda": "800", "ebit": "500",
                             "net_income": "260"},
        "closing": {"equity": "2000", "total_assets": "6000", "invested_capital": "4000",
                    "debt": "2000"},
        "leverage": {"debt_basis": "net_financial_position", "financial_charges": "100",
                     "taxes": "140", "pre_tax_income": "400"}
    });
    if balances == "average" {
        input["opening"] = json!({"equity": "1800", "total_assets": "5600",
                                  "invested_capital": "3800", "debt": "1800"});
    }
    Table {
        method: "ratios.profitability",
        input,
        outputs: PROFITABILITY,
    }
}

#[test]
fn profitability_rows() {
    let lev_tail: &[&str] = &[
        "/roe_leverage/leverage_effect",
        "/roe_leverage/pre_tax_roe",
        "/roe_leverage/roe",
    ];
    let with_tail = |head: &[&'static str]| -> Vec<&'static str> {
        head.iter().chain(lev_tail).copied().collect()
    };
    let ebit = with_tail(&["/roi", "/roa", "/ros", "/ebit_margin", "/roe_leverage/roi"]);
    let equity = [
        "/roe",
        "/roe_three_factor/equity_multiplier",
        "/roe_three_factor/roe",
        "/roe_leverage/debt_to_equity",
        "/roe_leverage/leverage_effect",
        "/roe_leverage/pre_tax_roe",
        "/roe_leverage/roe",
    ];
    let invested = with_tail(&["/roi", "/capital_turnover", "/roe_leverage/roi"]);
    let debt = with_tail(&["/roe_leverage/cost_of_debt", "/roe_leverage/debt_to_equity"]);
    let charges = with_tail(&["/roe_leverage/cost_of_debt"]);
    let taxes: &[&str] = &["/roe_leverage/tax_rate", "/roe_leverage/roe"];
    let revenue: &[&str] = &[
        "/ros",
        "/ebitda_margin",
        "/ebit_margin",
        "/capital_turnover",
        "/roe_three_factor/net_margin",
        "/roe_three_factor/asset_turnover",
        "/roe_three_factor/roe",
    ];
    let assets: &[&str] = &[
        "/roa",
        "/roe_three_factor/asset_turnover",
        "/roe_three_factor/equity_multiplier",
        "/roe_three_factor/roe",
    ];
    let net_income: &[&str] = &[
        "/roe",
        "/roe_three_factor/net_margin",
        "/roe_three_factor/roe",
    ];
    check(
        &profitability("closing"),
        &[
            (
                "/income_statement/revenue",
                "/income_statement/revenue",
                revenue,
            ),
            (
                "/income_statement/ebitda",
                "/income_statement/ebitda",
                &["/ebitda_margin"],
            ),
            (
                "/ebitda_definition",
                "/ebitda_definition",
                &["/ebitda_margin"],
            ),
            ("/income_statement/ebit", "/income_statement/ebit", &ebit),
            (
                "/income_statement/net_income",
                "/income_statement/net_income",
                net_income,
            ),
            ("/closing/equity", "/closing/equity", &equity),
            ("/closing/total_assets", "/closing/total_assets", assets),
            (
                "/closing/invested_capital",
                "/closing/invested_capital",
                &invested,
            ),
            ("/closing/debt", "/closing/debt", &debt),
            (
                "/leverage/financial_charges",
                "/leverage/financial_charges",
                &charges,
            ),
            ("/leverage/taxes", "/leverage/taxes", taxes),
            (
                "/leverage/pre_tax_income",
                "/leverage/pre_tax_income",
                taxes,
            ),
            ("/leverage", "/leverage", LEVERAGE_ALL),
        ],
    );
    check(
        &profitability("average"),
        &[
            ("/opening/equity", "/opening/equity", &equity),
            ("/opening/debt", "/opening/debt", &debt),
            ("/closing/total_assets", "/closing/total_assets", assets),
        ],
    );
}

#[test]
fn profitability_without_leverage_keeps_the_version_1_reason() {
    let input = remove(&profitability("closing").input, "/leverage");
    let out = run("ratios.profitability", &input);
    assert_eq!(
        out["value"]["roe_leverage"]["roe"]["not_defined"],
        "leverage inputs not given"
    );
}

#[test]
fn a_missing_amount_is_never_an_error_and_a_blank_definition_is() {
    let minimal = json!({"balances": "closing", "operating_return_base": "total_assets"});
    let out = run("ratios.profitability", &minimal);
    assert_eq!(
        out["value"]["not_computed"].as_array().unwrap().len(),
        PROFITABILITY.len()
    );
    let mut blank = profitability("closing").input;
    blank["ebitda_definition"] = json!("  ");
    let err = fairmark_core::run("ratios.profitability", &blank.to_string()).unwrap_err();
    assert_eq!(err.code(), "FM-E001");
    // Null equals absent.
    let mut null = profitability("closing").input;
    null["closing"]["equity"] = Value::Null;
    let out = run("ratios.profitability", &null);
    assert_eq!(out["value"]["not_computed"][0]["output"], "/roe");
}

#[test]
fn not_defined_by_a_given_term_wins_over_a_missing_term() {
    // Revenue zero: the margins are not defined even without EBIT or EBITDA.
    let input = json!({
        "balances": "closing", "operating_return_base": "total_assets",
        "income_statement": {"revenue": "0"}
    });
    let out = run("ratios.profitability", &input);
    assert_eq!(out["value"]["ros"]["not_defined"], "revenue is zero");
    let listed = out["value"]["not_computed"].to_string();
    assert!(!listed.contains("\"/ros\""), "{listed}");
}

const CAPITAL_STRUCTURE: &[(&str, Kind)] = &[
    ("/net_financial_position", Kind::Amount),
    ("/debt_to_equity", Kind::Ratio),
    ("/nfp_to_equity", Kind::Ratio),
    ("/equity_ratio", Kind::Ratio),
    ("/primary_structure_margin", Kind::Amount),
    ("/secondary_structure_margin", Kind::Amount),
    ("/primary_fixed_asset_coverage", Kind::Ratio),
    ("/secondary_fixed_asset_coverage", Kind::Ratio),
];

#[test]
fn capital_structure_rows() {
    let table = Table {
        method: "ratios.capital_structure",
        input: json!({
            "balances": "closing",
            "closing": {"equity": "2000", "total_assets": "6000", "fixed_assets": "3500",
                        "non_current_liabilities": "1800",
                        "net_financial_position": {"gross_debt": "2200", "cash": "300",
                            "other_items": [{"name": "lease liabilities", "amount": "150"}]}}
        }),
        outputs: CAPITAL_STRUCTURE,
    };
    let fixed: &[&str] = &[
        "/primary_structure_margin",
        "/secondary_structure_margin",
        "/primary_fixed_asset_coverage",
        "/secondary_fixed_asset_coverage",
    ];
    let nfp = "/closing/net_financial_position";
    check(
        &table,
        &[
            (
                "/closing/equity",
                "/closing/equity",
                &[
                    "/debt_to_equity",
                    "/nfp_to_equity",
                    "/equity_ratio",
                    "/primary_structure_margin",
                    "/secondary_structure_margin",
                    "/primary_fixed_asset_coverage",
                    "/secondary_fixed_asset_coverage",
                ],
            ),
            (
                "/closing/total_assets",
                "/closing/total_assets",
                &["/equity_ratio"],
            ),
            ("/closing/fixed_assets", "/closing/fixed_assets", fixed),
            (
                "/closing/non_current_liabilities",
                "/closing/non_current_liabilities",
                &[
                    "/secondary_structure_margin",
                    "/secondary_fixed_asset_coverage",
                ],
            ),
            (
                "/closing/net_financial_position/gross_debt",
                "/closing/net_financial_position/gross_debt",
                &[
                    "/net_financial_position",
                    "/debt_to_equity",
                    "/nfp_to_equity",
                ],
            ),
            (
                "/closing/net_financial_position/cash",
                "/closing/net_financial_position/cash",
                &["/net_financial_position", "/nfp_to_equity"],
            ),
            (
                nfp,
                "/closing/net_financial_position/gross_debt",
                &[
                    "/net_financial_position",
                    "/debt_to_equity",
                    "/nfp_to_equity",
                ],
            ),
        ],
    );
}

const LIQUIDITY: &[(&str, Kind)] = &[
    ("/current_ratio", Kind::Ratio),
    ("/quick_ratio", Kind::Ratio),
    ("/cash_ratio", Kind::Ratio),
    ("/net_working_capital", Kind::Amount),
    ("/treasury_margin", Kind::Amount),
];

#[test]
fn liquidity_rows() {
    let table = Table {
        method: "ratios.liquidity",
        input: json!({
            "balances": "average",
            "closing": {"current_assets": "2500", "inventories": "900", "cash": "200",
                        "current_liabilities": "2000"},
            "opening": {"current_assets": "2300", "inventories": "800", "cash": "150",
                        "current_liabilities": "1900"}
        }),
        outputs: LIQUIDITY,
    };
    let all: Vec<&str> = LIQUIDITY.iter().map(|(o, _)| *o).collect();
    check(
        &table,
        &[
            (
                "/closing/current_assets",
                "/closing/current_assets",
                &[
                    "/current_ratio",
                    "/quick_ratio",
                    "/net_working_capital",
                    "/treasury_margin",
                ],
            ),
            (
                "/opening/inventories",
                "/opening/inventories",
                &["/quick_ratio", "/treasury_margin"],
            ),
            ("/opening/cash", "/opening/cash", &["/cash_ratio"]),
            (
                "/closing/current_liabilities",
                "/closing/current_liabilities",
                &all,
            ),
        ],
    );
}

const WORKING_CAPITAL: &[(&str, Kind)] = &[
    ("/days_sales_outstanding", Kind::Ratio),
    ("/days_inventory_outstanding", Kind::Ratio),
    ("/days_payable_outstanding", Kind::Ratio),
    ("/cash_conversion_cycle", Kind::Ratio),
];

#[test]
fn working_capital_rows() {
    let table = Table {
        method: "ratios.working_capital",
        input: json!({
            "balances": "closing", "day_basis": "365",
            "income_statement": {"revenue": "5000", "cost_of_sales": "3000",
                                 "purchases": "2800"},
            "closing": {"trade_receivables": "1200", "inventories": "600",
                        "trade_payables": "700"}
        }),
        outputs: WORKING_CAPITAL,
    };
    let dso: &[&str] = &["/days_sales_outstanding", "/cash_conversion_cycle"];
    let dio: &[&str] = &["/days_inventory_outstanding", "/cash_conversion_cycle"];
    let dpo: &[&str] = &["/days_payable_outstanding", "/cash_conversion_cycle"];
    check(
        &table,
        &[
            (
                "/closing/trade_receivables",
                "/closing/trade_receivables",
                dso,
            ),
            (
                "/income_statement/revenue",
                "/income_statement/revenue",
                dso,
            ),
            ("/closing/inventories", "/closing/inventories", dio),
            (
                "/income_statement/cost_of_sales",
                "/income_statement/cost_of_sales",
                dio,
            ),
            ("/closing/trade_payables", "/closing/trade_payables", dpo),
            (
                "/income_statement/purchases",
                "/income_statement/purchases",
                dpo,
            ),
        ],
    );
}

const DEBT_SERVICE: &[(&str, Kind)] = &[
    ("/net_financial_position", Kind::Amount),
    ("/interest_cover_ebit", Kind::Ratio),
    ("/interest_cover_ebitda", Kind::Ratio),
    ("/nfp_to_ebitda", Kind::Ratio),
    ("/dscr/0", Kind::Ratio),
    ("/dscr/1", Kind::Ratio),
    ("/pv_cfads", Kind::Amount),
    ("/llcr", Kind::Ratio),
];

fn debt_service_input() -> Value {
    json!({
        "balances": "closing",
        "ebitda_definition": "reported EBITDA, no adjustments",
        "interest_definition": "gross interest expense",
        "income_statement": {"ebitda": "800", "ebit": "500", "interest": "100"},
        "closing": {"net_financial_position": {"gross_debt": "2200", "cash": "300"}},
        "debt_schedule": {"discount_rate": "0.06", "debt_outstanding": "1000",
                          "reserve_balance": "50",
                          "periods": [{"cfads": "250", "debt_service": "230"},
                                      {"cfads": "260", "debt_service": "235"}]}
    })
}

#[test]
fn debt_service_rows() {
    let table = Table {
        method: "ratios.debt_service",
        input: debt_service_input(),
        outputs: DEBT_SERVICE,
    };
    let ebitda: &[&str] = &["/interest_cover_ebitda", "/nfp_to_ebitda"];
    let covers: &[&str] = &["/interest_cover_ebit", "/interest_cover_ebitda"];
    let nfp: &[&str] = &["/net_financial_position", "/nfp_to_ebitda"];
    let schedule = "/debt_schedule";
    check(
        &table,
        &[
            (
                "/income_statement/ebitda",
                "/income_statement/ebitda",
                ebitda,
            ),
            ("/ebitda_definition", "/ebitda_definition", ebitda),
            (
                "/income_statement/ebit",
                "/income_statement/ebit",
                &["/interest_cover_ebit"],
            ),
            (
                "/income_statement/interest",
                "/income_statement/interest",
                covers,
            ),
            ("/interest_definition", "/interest_definition", covers),
            (
                "/closing/net_financial_position/gross_debt",
                "/closing/net_financial_position/gross_debt",
                nfp,
            ),
            (
                "/closing/net_financial_position/cash",
                "/closing/net_financial_position/cash",
                nfp,
            ),
            (
                "/debt_schedule/discount_rate",
                "/debt_schedule/discount_rate",
                &["/pv_cfads", "/llcr"],
            ),
            (
                "/debt_schedule/debt_outstanding",
                "/debt_schedule/debt_outstanding",
                &["/llcr"],
            ),
            (
                "/debt_schedule/periods/0/cfads",
                "/debt_schedule/periods/0/cfads",
                &["/dscr/0", "/pv_cfads", "/llcr"],
            ),
            (
                "/debt_schedule/periods/1/debt_service",
                "/debt_schedule/periods/1/debt_service",
                &["/dscr/1"],
            ),
        ],
    );
    // Without a schedule: DSCR empty, PV and LLCR listed, LLCR keeps its version 1 reason.
    let out = run(
        "ratios.debt_service",
        &remove(&debt_service_input(), schedule),
    );
    let v = &out["value"];
    assert_eq!(v["dscr"], json!([]));
    assert_eq!(v["llcr"]["not_defined"], "no debt schedule given");
    assert_eq!(v["not_computed"][0]["output"], "/pv_cfads");
    assert_eq!(v["not_computed"][0]["missing"], json!([schedule]));
    assert_eq!(v["not_computed"][1]["output"], "/llcr");
}

#[test]
fn covenant_rows() {
    let covenant = |numerator: Value, denominator: Value, rule: &str| {
        json!({"name": "net leverage", "test_date": "2026-12-31",
               "numerator": {"definition": "net debt", "amount": numerator},
               "denominator": {"definition": "EBITDA", "amount": denominator},
               "operator": "at_most", "threshold": "3.5",
               "if_denominator_not_positive": rule})
    };
    let input = json!({"covenants": [
        covenant(json!("3000"), json!("1000"), "breached"),
        covenant(json!("3000"), json!("1000"), "breached"),
    ]});
    let full = run("ratios.covenants", &input);
    assert_eq!(full["value"]["not_computed"], json!([]));
    let partial = remove(&input, "/covenants/0/numerator/amount");
    let out = run("ratios.covenants", &partial);
    let c = &out["value"]["covenants"][0];
    assert_eq!(c["outcome"], "not_tested");
    assert_eq!(c["not_tested_code"], "missing_input");
    assert!(c["value"].is_null() && c["headroom"].is_null());
    assert!(c["headroom_relative"]["value"].is_null());
    assert_eq!(out["value"]["covenants"][1], full["value"]["covenants"][1]);
    assert_eq!(out["value"]["summary"]["not_tested"], 1);
    let listed: Vec<&str> = out["value"]["not_computed"]
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["output"].as_str().unwrap())
        .collect();
    assert_eq!(
        listed,
        [
            "/covenants/0/value",
            "/covenants/0/headroom",
            "/covenants/0/headroom_relative"
        ]
    );
    // A zero denominator decides by its rule, with or without the numerator.
    let mut zero = remove(&input, "/covenants/1/numerator/amount");
    zero["covenants"][1]["denominator"]["amount"] = json!("0");
    let out = run("ratios.covenants", &zero);
    assert_eq!(out["value"]["covenants"][1]["outcome"], "breached");
    assert_eq!(out["value"]["not_computed"], json!([]));
}

#[test]
fn every_ratios_method_is_version_2() {
    for id in [
        "ratios.profitability",
        "ratios.capital_structure",
        "ratios.liquidity",
        "ratios.working_capital",
        "ratios.debt_service",
        "ratios.covenants",
    ] {
        assert_eq!(fairmark_core::method(id).unwrap().version, "2", "{id}");
    }
}
