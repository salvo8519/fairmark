//! `dcf.fcfe` version 1: equity DCF, free cash flows to equity discounted at the cost of equity
//! over explicit stages and a terminal value, then equity value directly (SPEC-0001 R1, R3, R7,
//! R19, R22, R24 to R29).
//!
//! The method is a second caller of the shared engine: stages, rates, terms, timing and the
//! terminal technique are those of `dcf.fcff`, on the equity basis. Debt is in the flows
//! (interest and net borrowing), so no net financial position is subtracted (R24).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::Serialize;

use super::bridge::{self, BridgeClaim};
use super::engine::{self, dcf_input, DiscountedValue};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, sub};
use crate::ratios::NetFinancialPosition;
use crate::tv::{Line, TerminalBasis};

dcf_input! {
    /// Input of `dcf.fcfe` (version 1). Every choice is required, with no default (R29).
    pub struct DcfFcfeInput {
        basis: TerminalBasis::Equity,
        /// One to three explicit stages of free cash flows to equity (FCFE), in order; the
        /// explicit horizon N is the sum of their periods. Each stage is `{"periods",
        /// "projection", ...}` with the fields of its projection; a `given` stage takes the FCFE
        /// as amounts (`flows`) or builds them up from line items with `interest_expense` and
        /// `net_borrowing` in every line (`lines`).
        stages,
        /// FCFE of the base period CF_0, in currency units: required when the first stage is
        /// `constant_growth` or `linear_decline`, refused when it is `given`.
        base_flow,
        /// For stages with `lines`: tax rate applied to EBIT, decimal fraction from 0 to below 1
        /// (0.24 = 24%), also the rate of the interest tax shield. Give this, or
        /// `operating_taxes` in every line, not both; refused when no stage has `lines`. Default:
        /// none.
        notional_tax_rate,
        /// For stages with `lines` and `notional_tax_rate`: treatment of a negative EBIT, `credit`
        /// (taxes = rate × EBIT) or `zero` (no tax, and no interest tax shield in that period);
        /// required with `notional_tax_rate`, no default.
        negative_ebit,
        equity_only
        /// For stages with `lines` and `operating_taxes`: the tax rate t of the interest tax
        /// shield, interest × (1 − t), decimal fraction from 0 to below 1; required in that case,
        /// refused otherwise. Default: none.
        interest_tax_rate,
        rates {
            /// One cost of equity for every period and for the terminal value, decimal fraction
            /// per period (0.11 = 11%), greater than -1. Give this or `cost_of_equity_per_period`,
            /// not both.
            cost_of_equity,
            /// One cost of equity per period of the explicit horizon, decimal fractions, exactly N
            /// of them, each greater than -1; requires `terminal_cost_of_equity`. Give this or
            /// `cost_of_equity`, not both.
            cost_of_equity_per_period,
            /// Cost of equity of the terminal phase, decimal fraction per period, greater than -1;
            /// required with `cost_of_equity_per_period`, refused with `cost_of_equity`.
            terminal_cost_of_equity,
        },
        /// The terminal technique on the equity basis, an object with exactly one key, the method
        /// id, for example `{"tv.gordon": {"growth", "first_flow", ...}}`: `tv.gordon`,
        /// `tv.h_model` or `tv.limited_horizon` on the FCFE, `tv.exit_multiple` on `earnings`
        /// (P/E) or `book_value` (P/BV), `tv.normalised_income` or `tv.liquidation` declared on
        /// the `equity` basis. `tv.value_driver` is not available on the equity basis in version
        /// 1.
        terminal,
        bridge {
            /// Not used on the equity basis and refused (`FM-E019`): debt is already in the FCFE,
            /// through interest and net borrowing, so the net financial position is not
            /// subtracted. Cash that the FCFE does not reflect goes in `non_operating_assets`.
            /// Accepted by the schema only so that the refusal names the fix.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub net_financial_position: Option<NetFinancialPosition>,
            /// Non-operating assets at the valuation date, each a label and a value in currency
            /// units (zero or more), added to the present value of the FCFE and of the terminal
            /// value: for example cash not needed for operations whose interest income is not in
            /// the FCFE, or an investment property. Default: none, reported.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub non_operating_assets: Vec<Line>,
            /// Debt-like items (debt equivalents) at the valuation date whose cash flows are not in
            /// the FCFE, for example unfunded pension obligations, each a label, a value in
            /// currency units (zero or more) and its measurement basis; subtracted. Default: none,
            /// reported.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub debt_like_items: Vec<BridgeClaim>,
            /// Non-controlling interests (minority interests) at the valuation date, when the FCFE
            /// are those of the consolidated group, each a label, a value in currency units (zero
            /// or more) and its measurement basis; subtracted. Default: none, reported.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub non_controlling_interests: Vec<BridgeClaim>,
        }
    }
}

/// From the present value of the FCFE to equity value, every line as given (R24, R25).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FcfeBridge {
    /// Present value of the explicit FCFE plus present value of the terminal value, in
    /// currency units.
    pub pv_explicit_and_terminal: Decimal,
    /// Each non-operating asset, label and value, as given.
    pub non_operating_assets: Vec<Line>,
    /// Sum of the non-operating assets, in currency units, added.
    pub non_operating_assets_total: Decimal,
    /// Each debt-like item, label, value and measurement basis, as given.
    pub debt_like_items: Vec<BridgeClaim>,
    /// Sum of the debt-like items, in currency units, subtracted.
    pub debt_like_items_total: Decimal,
    /// Each non-controlling interest, label, value and measurement basis, as given.
    pub non_controlling_interests: Vec<BridgeClaim>,
    /// Sum of the non-controlling interests, in currency units, subtracted.
    pub non_controlling_interests_total: Decimal,
    /// The present value of the explicit FCFE and the terminal value plus the non-operating
    /// assets, minus the debt-like items and the non-controlling interests, in currency units.
    pub equity_value: Decimal,
}

/// Output of `dcf.fcfe` (version 1). No value of operations and no enterprise value: the
/// equity basis values the equity directly (R24).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DcfFcfeOutput {
    /// `equity`: FCFE discounted at the cost of equity.
    pub basis: TerminalBasis,
    /// Present value of the explicit FCFE and of the terminal value, plus the non-operating
    /// assets, minus the debt-like items and the non-controlling interests, in currency units.
    pub equity_value: Decimal,
    /// Present value of the explicit FCFE, the terminal value weight, the periods, the terms
    /// and the terminal value, as JSON keys of this object.
    #[serde(flatten)]
    pub discounted: DiscountedValue,
    /// Every line from the present value of the FCFE to equity value.
    pub bridge: FcfeBridge,
}

const FORMULA: &str = "Equity value = Σ FCFE_k × DF_k + TV_N × DF_TV + Σ non-operating assets − \
    Σ debt-like items − Σ non-controlling interests, at the cost of equity";

pub fn fcfe(input: &DcfFcfeInput) -> Result<(DcfFcfeOutput, Explain)> {
    // R24, Decision 9: debt is in the flows; a net financial position would count it twice.
    if input.net_financial_position.is_some() {
        return Err(FairmarkError::MixedBasis(
            "`/net_financial_position` is given, but this DCF values on the equity basis: the \
             FCFE already include the debt through interest and net borrowing (SPEC-0001 R24), \
             so subtracting the net financial position would count it twice; remove it. Give \
             cash that the FCFE do not reflect in `/non_operating_assets`, or value the \
             enterprise with dcf.fcff, which subtracts the net financial position"
                .to_owned(),
        ));
    }
    let mut ex = Explain::new(Fcfe::ID, FORMULA);
    ex.convention(
        "basis: equity, free cash flows to equity (FCFE) discounted at the cost of equity",
    );
    ex.convention(
        "no net financial position is subtracted: debt is already in the flows through interest \
         and net borrowing (R24); equity value is computed directly",
    );
    let engine_input = input.engine_input();
    // Bridge items are checked before any calculation, like every other input.
    bridge::check_lines(
        &input.non_operating_assets,
        &input.debt_like_items,
        &input.non_controlling_interests,
        "a non-operating asset is zero or more (obligations whose cash flows are not in the \
         FCFE belong in `debt_like_items`)",
    )?;
    let out = engine::run(&engine_input, &mut ex)?;

    let pv_tv = out.terminal.present_value;
    let start = add(out.pv_explicit, pv_tv)?;
    ex.step(
        "Present value of the explicit FCFE and the terminal value",
        format!("{} + {pv_tv}", out.pv_explicit),
        start,
    );
    let bridge = equity_bridge(&mut ex, input, start)?;
    let weight = engine::terminal_weight(
        &mut ex,
        pv_tv,
        start,
        "the present value of the explicit FCFE and the terminal value is zero",
    )?;
    Ok((
        DcfFcfeOutput {
            basis: TerminalBasis::Equity,
            equity_value: bridge.equity_value,
            discounted: out.discounted(weight),
            bridge,
        },
        ex,
    ))
}

/// R24, R25: present value of the FCFE and the terminal value + non-operating assets −
/// debt-like items − non-controlling interests = equity value. Every line echoed in `explain`.
fn equity_bridge(ex: &mut Explain, input: &DcfFcfeInput, start: Decimal) -> Result<FcfeBridge> {
    ex.convention("bridge items measured at the valuation date, as given");
    ex.convention(
        "bridge signs (equity basis): non-operating assets added to the present value of the \
         explicit flows and the terminal value; debt-like items and non-controlling interests \
         subtracted; every debt-like item and non-controlling interest is zero or more",
    );
    let assets = bridge::non_operating_assets_total(ex, &input.non_operating_assets)?;
    let debt_like = bridge::claims_total(
        ex,
        &input.debt_like_items,
        "Debt-like item",
        "Debt-like items",
    )?;
    let non_controlling = bridge::claims_total(
        ex,
        &input.non_controlling_interests,
        "Non-controlling interest",
        "Non-controlling interests",
    )?;
    let equity_value = sub(sub(add(start, assets)?, debt_like)?, non_controlling)?;
    bridge::warn_if_negative(
        ex,
        equity_value,
        "the present value of the FCFE and the terminal value plus the non-operating assets is \
         below the claims subtracted",
    );
    ex.step(
        "Equity value",
        format!("{start} + {assets} − {debt_like} − {non_controlling}"),
        equity_value,
    );
    Ok(FcfeBridge {
        pv_explicit_and_terminal: start,
        non_operating_assets: input.non_operating_assets.clone(),
        non_operating_assets_total: assets,
        debt_like_items: input.debt_like_items.clone(),
        debt_like_items_total: debt_like,
        non_controlling_interests: input.non_controlling_interests.clone(),
        non_controlling_interests_total: non_controlling,
        equity_value,
    })
}

/// `dcf.fcfe`: see the methodology page.
pub struct Fcfe;

impl Method for Fcfe {
    const ID: &'static str = "dcf.fcfe";
    const TITLE: &'static str = "DCF — free cash flow to equity";
    const DESCRIPTION: &'static str =
        "Equity value from FCFE discounted at the cost of equity over one to three explicit \
        stages (given flows or flows built up from line items with interest and net borrowing, \
        constant growth, linear decline of growth) and a terminal value by a tv.* technique on \
        the equity basis, plus non-operating assets, minus debt-like items and non-controlling \
        interests; no net financial position, debt is in the flows. Returns every period and \
        step for audit.";
    const VERSION: &'static str = "1";
    type Input = DcfFcfeInput;
    type Output = DcfFcfeOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        fcfe(input)
    }
}

#[cfg(test)]
mod tests {
    use serde_json::{json, Value};

    use crate::registry::{input_schema, run};

    fn base() -> Value {
        json!({
            "stages": [{"periods": 3, "projection": "given", "flows": ["100", "110", "120"]}],
            "cash_flow_terms": "nominal",
            "discount_rate_terms": "nominal",
            "cost_of_equity": "0.11",
            "timing": "end_of_period",
            "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}
        })
    }

    fn with(mut v: Value, f: impl FnOnce(&mut serde_json::Map<String, Value>)) -> Value {
        f(v.as_object_mut().unwrap());
        v
    }

    fn err(v: &Value) -> crate::FairmarkError {
        run("dcf.fcfe", &v.to_string()).unwrap_err()
    }

    fn out(v: &Value) -> Value {
        run("dcf.fcfe", &v.to_string()).unwrap()
    }

    fn conventions(v: &Value) -> Vec<String> {
        serde_json::from_value(out(v)["explain"]["conventions"].clone()).unwrap()
    }

    fn d(o: &Value, p: &str) -> rust_decimal::Decimal {
        o["value"]
            .pointer(p)
            .unwrap()
            .as_str()
            .unwrap()
            .parse()
            .unwrap()
    }

    /// R24, AC13: equity value only, the identity of R24, and the weight of R22.
    #[test]
    fn equity_value_directly_without_enterprise_quantities() {
        let v = with(base(), |m| {
            m.insert(
                "non_operating_assets".into(),
                json!([{"label": "Excess cash", "value": "40"}]),
            );
            m.insert(
                "debt_like_items".into(),
                json!([{"label": "Pension", "value": "15", "measurement_basis": "present_value"}]),
            );
            m.insert(
                "non_controlling_interests".into(),
                json!([{"label": "Minority", "value": "7.5", "measurement_basis": "book_value"}]),
            );
        });
        let o = out(&v);
        assert_eq!(o["value"]["basis"], "equity");
        assert!(o["value"].get("value_of_operations").is_none());
        assert!(o["value"].get("enterprise_value").is_none());
        assert!(o["value"]["bridge"].get("net_financial_position").is_none());
        let start = d(&o, "/bridge/pv_explicit_and_terminal");
        assert_eq!(
            start,
            d(&o, "/pv_explicit_period") + d(&o, "/terminal/present_value")
        );
        assert_eq!(
            d(&o, "/equity_value"),
            start + "40".parse::<rust_decimal::Decimal>().unwrap()
                - "15".parse::<rust_decimal::Decimal>().unwrap()
                - "7.5".parse::<rust_decimal::Decimal>().unwrap()
        );
        assert_eq!(
            d(&o, "/terminal_value_weight"),
            d(&o, "/terminal/present_value") / start
        );
        let steps = o["explain"]["steps"].as_array().unwrap();
        let equity = steps.iter().find(|s| s["label"] == "Equity value").unwrap();
        assert!(
            equity["formula"]
                .as_str()
                .unwrap()
                .ends_with(" + 40 − 15 − 7.5"),
            "{equity}"
        );
        assert!(steps
            .iter()
            .any(|s| s["label"] == "Non-operating asset: Excess cash"));
    }

    /// R24, AC13: a net financial position is FM-E019 with the pointer and the fix, before any
    /// calculation.
    #[test]
    fn net_financial_position_is_refused() {
        let v = with(base(), |m| {
            m.insert(
                "net_financial_position".into(),
                json!({"gross_debt": "500", "cash": "80"}),
            );
        });
        let e = err(&v);
        assert_eq!(e.code(), "FM-E019");
        let text = e.to_string();
        assert!(text.contains("`/net_financial_position`"), "{text}");
        assert!(text.contains("`/non_operating_assets`"), "{text}");
        assert!(text.contains("dcf.fcff"), "{text}");
    }

    /// AC14 (R27): every error of the method names its field by a JSON pointer.
    #[test]
    fn every_error_names_its_field_by_a_json_pointer() {
        let lines = json!([
            {"ebit": "100", "depreciation_and_amortisation": "10", "capital_expenditure": "12",
             "change_in_net_working_capital": "3"},
            {"ebit": "100", "depreciation_and_amortisation": "10", "capital_expenditure": "12",
             "change_in_net_working_capital": "3"}
        ]);
        let cases: Vec<(Value, &str, &str)> = vec![
            (
                with(base(), |m| {
                    m.remove("cost_of_equity");
                }),
                "FM-E001",
                "/cost_of_equity",
            ),
            (
                with(base(), |m| {
                    m.insert("terminal_cost_of_equity".into(), json!("0.1"));
                }),
                "FM-E001",
                "/terminal_cost_of_equity",
            ),
            (
                with(base(), |m| {
                    m.remove("cost_of_equity");
                    m.insert(
                        "cost_of_equity_per_period".into(),
                        json!(["0.1", "0.1", "0.1"]),
                    );
                }),
                "FM-E001",
                "/terminal_cost_of_equity",
            ),
            (
                with(base(), |m| {
                    m.remove("cost_of_equity");
                    m.insert("cost_of_equity_per_period".into(), json!(["0.1", "0.1"]));
                    m.insert("terminal_cost_of_equity".into(), json!("0.1"));
                }),
                "FM-E025",
                "/cost_of_equity_per_period",
            ),
            (
                with(base(), |m| {
                    m.remove("cost_of_equity");
                    m.insert(
                        "cost_of_equity_per_period".into(),
                        json!(["0.1", "-1", "0.1"]),
                    );
                    m.insert("terminal_cost_of_equity".into(), json!("0.1"));
                }),
                "FM-E003",
                "/cost_of_equity_per_period/1",
            ),
            (
                with(base(), |m| {
                    m.insert("cost_of_equity".into(), json!("-1"));
                }),
                "FM-E003",
                "/cost_of_equity",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "terminal".into(),
                        json!({"tv.gordon": {"growth": "0.11", "first_flow": "last_flow_grown"}}),
                    );
                }),
                "FM-E004",
                "/terminal/tv.gordon/growth",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "terminal".into(),
                        json!({"tv.exit_multiple": {"multiple": "8", "metric": "ebitda",
                                                    "metric_value": "150"}}),
                    );
                }),
                "FM-E019",
                "/terminal/tv.exit_multiple/metric",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "terminal".into(),
                        json!({"tv.normalised_income": {"income": "120", "basis": "enterprise",
                                                        "growth": "0.01"}}),
                    );
                }),
                "FM-E019",
                "/terminal/tv.normalised_income/basis",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "stages".into(),
                        json!([{"periods": 2, "projection": "given", "lines": lines}]),
                    );
                    m.insert("notional_tax_rate".into(), json!("0.24"));
                    m.insert("negative_ebit".into(), json!("credit"));
                }),
                "FM-E001",
                "/stages/0/lines/0/interest_expense",
            ),
            (
                with(base(), |m| {
                    m.insert("interest_tax_rate".into(), json!("0.24"));
                }),
                "FM-E001",
                "/interest_tax_rate",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "debt_like_items".into(),
                        json!([{"label": "Pension", "value": "-1",
                                "measurement_basis": "book_value"}]),
                    );
                }),
                "FM-E001",
                "/debt_like_items/0/value",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "non_operating_assets".into(),
                        json!([{"label": "Cash", "value": "-1"}]),
                    );
                }),
                "FM-E001",
                "/non_operating_assets/0/value",
            ),
            (
                with(base(), |m| {
                    m.insert("cash_flow_terms".into(), json!("real"));
                }),
                "FM-E024",
                "/discount_rate_terms",
            ),
        ];
        for (input, code, pointer) in cases {
            let e = err(&input);
            assert_eq!(e.code(), code, "{e}");
            assert!(
                e.to_string().contains(&format!("`{pointer}")),
                "{pointer} not in: {e}"
            );
        }
    }

    /// R29: every choice is required; the WACC field of the enterprise basis is unknown here.
    #[test]
    fn every_choice_is_required_and_wacc_is_unknown() {
        for field in [
            "stages",
            "cash_flow_terms",
            "discount_rate_terms",
            "timing",
            "terminal",
        ] {
            let v = with(base(), |m| {
                m.remove(field);
            });
            assert_eq!(err(&v).code(), "FM-E001", "{field}");
        }
        let v = with(base(), |m| {
            m.insert("wacc".into(), json!("0.09"));
        });
        assert_eq!(err(&v).code(), "FM-E001");
    }

    /// R1, R28 (AC14): the basis, the absence of the net financial position and the bridge
    /// conventions are reported, with the conventions of the engine.
    #[test]
    fn every_convention_is_reported() {
        let c = conventions(&base()).join("\n");
        for needle in [
            "basis: equity, free cash flows to equity (FCFE) discounted at the cost of equity",
            "no net financial position is subtracted: debt is already in the flows",
            "terms: cash flows and growth rates nominal, discount rates nominal",
            "stage 1: periods 1 to 3, given flows",
            "discount rate: one rate, 0.11, for every period and for the terminal value",
            "timing: end of period",
            "terminal technique: tv.gordon version 1 on the equity basis",
            "bridge items measured at the valuation date, as given",
            "bridge signs (equity basis)",
            "non-operating assets: none given",
            "debt-like items: none given",
            "non-controlling interests: none given",
        ] {
            assert!(c.contains(needle), "missing '{needle}' in:\n{c}");
        }
        assert!(!c.contains("net financial position: none given"));
    }

    /// R3, R7: a stage of lines on the equity basis discounts the FCFE of the build-up, and
    /// the explain names the FCFE formula and the FCFE steps.
    #[test]
    fn lines_are_built_up_to_fcfe() {
        let v = with(base(), |m| {
            m.insert(
                "stages".into(),
                json!([{"periods": 2, "projection": "given", "lines": [
                    {"ebit": "100", "depreciation_and_amortisation": "10",
                     "capital_expenditure": "12", "change_in_net_working_capital": "3",
                     "interest_expense": "8", "net_borrowing": "5"},
                    {"ebit": "-20", "depreciation_and_amortisation": "10",
                     "capital_expenditure": "12", "change_in_net_working_capital": "3",
                     "interest_expense": "8", "net_borrowing": "0"}
                ]}]),
            );
            m.insert("notional_tax_rate".into(), json!("0.24"));
            m.insert("negative_ebit".into(), json!("zero"));
        });
        let o = out(&v);
        // FCFF 71 and −25; FCFE 71 − 8 × 0.76 + 5 = 69.92 and −25 − 8 + 0 = −33 (no shield).
        assert_eq!(d(&o, "/periods/0/flow"), "69.92".parse().unwrap());
        assert_eq!(d(&o, "/periods/1/flow"), "-33".parse().unwrap());
        let steps = o["explain"]["steps"].to_string();
        assert!(steps.contains("Build-up: FCFE, period 2"), "{steps}");
        assert!(
            steps.contains("built up from line items (step \\\"Build-up: FCFE, period 1\\\")"),
            "{steps}"
        );
        let c = conventions(&v).join("\n");
        assert!(
            c.contains(
                "for the stages with lines: FCFF_t = EBIT_t − taxes on EBIT_t + D&A_t − \
                 capex_t − ΔNWC_t + Σ adjustments_t; FCFE_t = FCFF_t − interest_t × (1 − t) \
                 + net borrowing_t"
            ),
            "{c}"
        );
    }

    /// R26: a negative equity value is a warning, reported and never floored at zero.
    #[test]
    fn negative_equity_value_is_a_warning() {
        let v = with(base(), |m| {
            m.insert(
                "debt_like_items".into(),
                json!([{"label": "Provision", "value": "5000", "measurement_basis": "book_value"}]),
            );
        });
        let o = out(&v);
        assert!(d(&o, "/equity_value").is_sign_negative());
        let c = o["explain"]["conventions"].to_string();
        assert!(c.contains("warning: the equity value is negative"), "{c}");
        assert!(!conventions(&base())
            .join("\n")
            .contains("equity value is negative"));
    }

    /// Open question of plan.md (#165): `tv.value_driver` states NOPAT and RONIC, enterprise
    /// measures; until the maintainer decides its equity form it is refused on the equity
    /// basis, so that no number is produced on an undecided reading.
    #[test]
    fn value_driver_is_refused_on_the_equity_basis_until_decided() {
        let v = with(base(), |m| {
            m.insert(
                "terminal".into(),
                json!({"tv.value_driver": {"nopat_next": "130", "growth": "0.02",
                                           "ronic": "0.15"}}),
            );
        });
        let e = err(&v);
        assert_eq!(e.code(), "FM-E019");
        assert!(e.to_string().contains("`/terminal/tv.value_driver`"), "{e}");
    }

    /// AC14 (R29): no choice in the schema is a boolean, at any depth.
    #[test]
    fn the_schema_has_no_boolean_choice() {
        fn walk(v: &Value, path: &str) {
            match v {
                Value::Object(m) => {
                    assert_ne!(m.get("type"), Some(&json!("boolean")), "boolean at {path}");
                    for (k, x) in m {
                        walk(x, &format!("{path}/{k}"));
                    }
                }
                Value::Array(a) => a
                    .iter()
                    .enumerate()
                    .for_each(|(i, x)| walk(x, &format!("{path}/{i}"))),
                _ => {}
            }
        }
        walk(&input_schema("dcf.fcfe").unwrap(), "");
    }
}
