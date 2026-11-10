//! `dcf.fcff` version 3: enterprise DCF, free cash flows to the firm discounted at the WACC
//! over explicit stages and a terminal value, then value of operations, enterprise value and
//! equity value (SPEC-0001 R1, R6 to R15, R19 to R23, R25, R26).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::Serialize;

use super::bridge::{self, BridgeClaim};
use super::engine::{self, dcf_input, DiscountedValue};
use crate::error::{Found, Result};
use crate::explain::{DefaultApplied, Explain};
use crate::method::Method;
use crate::num::{add, sub};
use crate::ratios::{NetFinancialPosition, NFP_CONVENTION};
use crate::tv::{Line, TerminalBasis};

dcf_input! {
    /// Input of `dcf.fcff` (version 3). The minimal input is a vector of FCFF and one WACC,
    /// `{"stages": [{"flows": [...]}], "wacc": "0.09"}`; every other choice is required or has
    /// a default documented on its field, reported in the result (R29, Decision 24).
    pub struct DcfFcffInput {
        basis: TerminalBasis::Enterprise,
        /// One to three explicit stages, in order; the explicit horizon N is the sum of their
        /// periods. Each stage is `{"periods", "projection", ...}` with the fields of its
        /// projection; a stage of given FCFF can be just `{"flows": [...]}` (projection `given`
        /// and periods the number of flows, by default).
        stages,
        /// FCFF of the base period CF_0, in currency units: required when the first stage is
        /// `constant_growth` or `linear_decline`, refused when it is `given`.
        base_flow,
        /// For stages with `lines`: tax rate applied to EBIT, decimal fraction from 0 to below 1
        /// (0.24 = 24%). Give this, or `operating_taxes` in every line, not both; refused when no
        /// stage has `lines`. Default: none.
        notional_tax_rate,
        /// For stages with `lines` and `notional_tax_rate`: treatment of a negative EBIT, `credit`
        /// (taxes = rate × EBIT) or `zero` (no tax); required with `notional_tax_rate`, no default.
        negative_ebit,
        rates {
            /// One WACC for every period and for the terminal value, decimal fraction per period
            /// (0.09 = 9%), greater than -1. Give this or `wacc_per_period`, not both.
            wacc,
            /// One WACC per period of the explicit horizon, decimal fractions, exactly N of them,
            /// each greater than -1; requires `terminal_wacc`. Give this or `wacc`, not both.
            wacc_per_period,
            /// WACC of the terminal phase, decimal fraction per period, greater than -1; required
            /// with `wacc_per_period`, refused with `wacc`.
            terminal_wacc,
        },
        /// The terminal option, an object with exactly one key: a technique by its method id,
        /// for example `{"tv.gordon": {"growth", "first_flow", ...}}` (`tv.gordon`,
        /// `tv.h_model`, `tv.value_driver`, `tv.exit_multiple`, `tv.limited_horizon`,
        /// `tv.normalised_income` or `tv.liquidation`, on the enterprise basis);
        /// `{"given": {"value": ...}}`, a terminal value at N given by the analyst (the value of
        /// operations at N); or `{"none": {}}`, no terminal value. Default: `none`, the value
        /// covers the given flows only, reported with a warning.
        terminal,
        bridge {
            /// Net financial position at the valuation date (SPEC-0007): gross debt − cash + signed
            /// other items, positive for net debt, subtracted from enterprise value. Default: zero,
            /// reported in `meta.defaults_applied` with a warning that the equity value then equals
            /// the enterprise value (less the debt-like items and non-controlling interests); a
            /// net financial position given, even zero, carries neither.
            #[serde(default, skip_serializing_if = "Option::is_none")]
            pub net_financial_position: Option<NetFinancialPosition>,
            /// Non-operating assets at the valuation date, each a label and a value in currency units
            /// (zero or more), added to the value of operations. Default: none, reported.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub non_operating_assets: Vec<Line>,
            /// Debt-like items (debt equivalents) at the valuation date, for example unfunded pension
            /// obligations or provisions, each a label, a value in currency units (zero or more) and
            /// its measurement basis, subtracted from enterprise value. Do not repeat an item
            /// already among the `other_items` of the net financial position. Default: none, reported.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub debt_like_items: Vec<BridgeClaim>,
            /// Non-controlling interests (minority interests) at the valuation date, each a label, a
            /// value in currency units (zero or more) and its measurement basis (for example
            /// `book_value` or `fair_value`), subtracted from enterprise value. Default: none, reported.
            #[serde(default, skip_serializing_if = "Vec::is_empty")]
            pub non_controlling_interests: Vec<BridgeClaim>,
        }
    }
}

/// From value of operations to equity value, every line as given (R23, R25).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FcffBridge {
    /// Present value of the explicit FCFF plus present value of the terminal value.
    pub value_of_operations: Decimal,
    /// Each non-operating asset, label and value, as given.
    pub non_operating_assets: Vec<Line>,
    /// Sum of the non-operating assets, in currency units.
    pub non_operating_assets_total: Decimal,
    /// Value of operations plus the non-operating assets.
    pub enterprise_value: Decimal,
    /// Net financial position subtracted, positive for net debt; `null` when none is given,
    /// and then zero is subtracted as a documented default (Decision 25).
    pub net_financial_position: Option<Decimal>,
    /// Each debt-like item, label, value and measurement basis, as given.
    pub debt_like_items: Vec<BridgeClaim>,
    /// Sum of the debt-like items, in currency units, subtracted.
    pub debt_like_items_total: Decimal,
    /// Each non-controlling interest, label, value and measurement basis, as given.
    pub non_controlling_interests: Vec<BridgeClaim>,
    /// Sum of the non-controlling interests, in currency units, subtracted.
    pub non_controlling_interests_total: Decimal,
    /// Enterprise value minus the net financial position, the debt-like items and the
    /// non-controlling interests.
    pub equity_value: Decimal,
}

/// Output of `dcf.fcff` (version 3).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DcfFcffOutput {
    /// `enterprise`: FCFF discounted at the WACC.
    pub basis: TerminalBasis,
    /// Present value of the explicit FCFF plus present value of the terminal value.
    pub value_of_operations: Decimal,
    /// Value of operations plus the non-operating assets.
    pub enterprise_value: Decimal,
    /// Enterprise value minus the net financial position, the debt-like items and the
    /// non-controlling interests.
    pub equity_value: Decimal,
    /// Present value of the explicit FCFF, the terminal value weight, the periods, the terms
    /// and the terminal value, as JSON keys of this object.
    #[serde(flatten)]
    pub discounted: DiscountedValue,
    /// Every line from value of operations to equity value.
    pub bridge: FcffBridge,
}

/// JSON pointer of the net financial position, the field of its default (Decision 25).
const NFP_POINTER: &str = "/net_financial_position";

const FORMULA: &str = "Value of operations = Σ FCFF_k × DF_k + TV_N × DF_TV; Enterprise value = \
    value of operations + Σ non-operating assets; Equity value = enterprise value − net \
    financial position − Σ debt-like items − Σ non-controlling interests";

/// `dcf.fcff` through a typed call (ADR 0009): the documented defaults are filled in and
/// reported first in the conventions, as through the registry (SPEC-0001 R31).
pub fn fcff(input: &DcfFcffInput) -> Result<(DcfFcffOutput, Explain)> {
    let mut input = input.clone();
    Fcff::run_typed(&mut input).map(|(output, explain, _)| (output, explain))
}

/// The validate-only path: every check of [`fcff_with_defaults`], no calculation.
fn validate_fcff(input: &DcfFcffInput, defaults: &[DefaultApplied]) -> Result<()> {
    let engine_input = input.engine_input(defaults)?;
    engine::validate(&engine_input, bridge_lines(input))
}

/// The bridge lines checked, every refusal kept for the engine to report with its own.
fn bridge_lines(input: &DcfFcffInput) -> Found {
    let mut found = Found::new();
    bridge::check_lines(
        &input.non_operating_assets,
        &input.debt_like_items,
        &input.non_controlling_interests,
        "a non-operating asset is zero or more (obligations belong in the net financial \
         position)",
        &mut found,
    );
    found
}

/// The calculation on an input whose defaults were filled in; `defaults` names them, for the
/// messages that say when a value came from a default.
fn fcff_with_defaults(
    input: &DcfFcffInput,
    defaults: &[DefaultApplied],
) -> Result<(DcfFcffOutput, Explain)> {
    let mut ex = Explain::new("dcf.fcff", FORMULA);
    ex.convention("basis: enterprise, free cash flows to the firm (FCFF) discounted at the WACC");
    let engine_input = input.engine_input(defaults)?;
    // Bridge items are checked before any calculation, like every other input.
    let found = bridge_lines(input);
    let out = engine::run(&engine_input, &mut ex, found)?;

    let pv_tv = out.terminal.present_value;
    let value_of_operations = add(out.pv_explicit, pv_tv)?;
    ex.step(
        "Value of operations",
        format!("{} + {pv_tv}", out.pv_explicit),
        value_of_operations,
    );
    let bridge = bridge(&mut ex, input, value_of_operations)?;
    let weight = engine::terminal_weight(
        &mut ex,
        pv_tv,
        value_of_operations,
        "the value of operations is zero",
    )?;
    Ok((
        DcfFcffOutput {
            basis: TerminalBasis::Enterprise,
            value_of_operations,
            enterprise_value: bridge.enterprise_value,
            equity_value: bridge.equity_value,
            discounted: out.discounted(weight),
            bridge,
        },
        ex,
    ))
}

/// R23, R25: value of operations + non-operating assets = enterprise value; enterprise value −
/// net financial position − debt-like items − non-controlling interests = equity value. Every
/// line echoed in `explain`.
fn bridge(
    ex: &mut Explain,
    input: &DcfFcffInput,
    value_of_operations: Decimal,
) -> Result<FcffBridge> {
    ex.convention("bridge items measured at the valuation date, as given");
    ex.convention(
        "bridge signs: non-operating assets added to the value of operations; net financial \
         position (positive for net debt), debt-like items and non-controlling interests \
         subtracted from enterprise value; every debt-like item and non-controlling interest \
         is zero or more",
    );
    let total = bridge::non_operating_assets_total(ex, &input.non_operating_assets)?;
    let enterprise_value = add(value_of_operations, total)?;
    ex.step(
        "Enterprise value",
        format!("{value_of_operations} + {total}"),
        enterprise_value,
    );
    let nfp = match &input.net_financial_position {
        Some(p) => {
            ex.convention(NFP_CONVENTION);
            Some(p.total(ex, "valuation date")?)
        }
        None => None,
    };
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
    if nfp.is_none() {
        // Decision 25: zero, the documented default reported in `meta.defaults_applied`.
        // Name only the claims actually given.
        let rest = match (
            input.debt_like_items.is_empty(),
            input.non_controlling_interests.is_empty(),
        ) {
            (true, true) => "",
            (false, true) => " less the debt-like items given",
            (true, false) => " less the non-controlling interests given",
            (false, false) => " less the debt-like items and non-controlling interests given",
        };
        ex.convention(format!(
            "warning: no net financial position given (`{NFP_POINTER}`): zero is subtracted, so \
             the equity value equals the enterprise value{rest}; give the net financial position \
             to value the equity"
        ));
    }
    let nfp_subtracted = nfp.unwrap_or(Decimal::ZERO);
    let equity_value = sub(
        sub(sub(enterprise_value, nfp_subtracted)?, debt_like)?,
        non_controlling,
    )?;
    bridge::warn_if_negative(
        ex,
        equity_value,
        "the claims subtracted exceed the enterprise value",
    );
    ex.step(
        "Equity value",
        format!("{enterprise_value} − {nfp_subtracted} − {debt_like} − {non_controlling}"),
        equity_value,
    );
    Ok(FcffBridge {
        value_of_operations,
        non_operating_assets: input.non_operating_assets.clone(),
        non_operating_assets_total: total,
        enterprise_value,
        net_financial_position: nfp,
        debt_like_items: input.debt_like_items.clone(),
        debt_like_items_total: debt_like,
        non_controlling_interests: input.non_controlling_interests.clone(),
        non_controlling_interests_total: non_controlling,
        equity_value,
    })
}

/// `dcf.fcff`: see the methodology page.
pub struct Fcff;

impl Method for Fcff {
    const ID: &'static str = "dcf.fcff";
    const TITLE: &'static str = "DCF — unlevered free cash flow";
    const DESCRIPTION: &'static str =
        "Value of operations, enterprise value and equity value from FCFF discounted at the \
        WACC over one to three explicit stages (given flows, constant growth, linear decline \
        of growth) and a terminal value by a tv.* technique on the enterprise basis (stable \
        growth, H-model, value driver, exit multiple, finite horizon, normalised income, \
        liquidation), given as an amount, or none. The minimal input is a list of FCFF and \
        one WACC; the defaults applied are reported. Returns every period and step for audit.";
    const VERSION: &'static str = "3";
    type Input = DcfFcffInput;
    type Output = DcfFcffOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        fcff(input)
    }

    fn apply_defaults(input: &mut Self::Input) -> Vec<DefaultApplied> {
        let mut defaults = input.apply_defaults();
        // Decision 25: the default is reported, not written into the input, so the hash is
        // unchanged and a given zero (no warning) stays distinct from it.
        if input.net_financial_position.is_none() {
            defaults.push(DefaultApplied::new(
                NFP_POINTER,
                serde_json::json!("0"),
                "net financial position not given",
            ));
        }
        defaults
    }

    fn compute_with_defaults(
        input: &Self::Input,
        defaults: &[DefaultApplied],
    ) -> Result<(Self::Output, Explain)> {
        fcff_with_defaults(input, defaults)
    }

    fn validate(input: &Self::Input, defaults: &[DefaultApplied]) -> Result<()> {
        validate_fcff(input, defaults)
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
            "wacc": "0.10",
            "timing": "end_of_period",
            "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}
        })
    }

    fn with(mut v: Value, f: impl FnOnce(&mut serde_json::Map<String, Value>)) -> Value {
        f(v.as_object_mut().unwrap());
        v
    }

    fn err(v: &Value) -> crate::FairmarkError {
        run("dcf.fcff", &v.to_string()).unwrap_err()
    }

    fn conventions(v: &Value) -> Vec<String> {
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        serde_json::from_value(out["explain"]["conventions"].clone()).unwrap()
    }

    /// R7 (#38): a `given` stage with build-up lines values exactly like the same stage with
    /// the built-up flows; the build-up is in the explain and errors point into the lines.
    #[test]
    fn lines_equal_the_built_up_flows() {
        let lines = json!([
            {"ebit": "100", "depreciation_and_amortisation": "20", "capital_expenditure": "15",
             "change_in_net_working_capital": "5"},
            {"ebit": "-10", "depreciation_and_amortisation": "20", "capital_expenditure": "15",
             "change_in_net_working_capital": "-5"},
            {"ebit": "130", "depreciation_and_amortisation": "22", "capital_expenditure": "18",
             "change_in_net_working_capital": "4"}
        ]);
        let built = with(base(), |m| {
            m.insert(
                "stages".into(),
                json!([{"periods": 3, "projection": "given", "lines": lines}]),
            );
            m.insert("notional_tax_rate".into(), json!("0.24"));
            m.insert("negative_ebit".into(), json!("credit"));
        });
        // 100 − 24 + 20 − 15 − 5; −10 + 2.4 + 20 − 15 + 5; 130 − 31.2 + 22 − 18 − 4.
        let flows = with(base(), |m| {
            m.insert(
                "stages".into(),
                json!([{"periods": 3, "projection": "given",
                        "flows": ["76.00", "2.40", "98.80"]}]),
            );
        });
        let a = run("dcf.fcff", &built.to_string()).unwrap();
        let b = run("dcf.fcff", &flows.to_string()).unwrap();
        assert_eq!(a["value"], b["value"]);
        let steps = a["explain"]["steps"].as_array().unwrap();
        assert!(steps
            .iter()
            .any(|s| s["label"] == "Build-up: FCFF, period 2"));
        assert!(conventions(&built).iter().any(|c| c.starts_with(
            "free cash flow build-up (dcf.free_cash_flows v2): tax form: rate on EBIT"
        )));
        let mut bad = built.clone();
        bad["stages"][0]["lines"][1]["capital_expenditure"] = json!("-1");
        let e = err(&bad);
        assert_eq!(e.code(), "FM-E001");
        assert!(
            e.to_string()
                .contains("`/stages/0/lines/1/capital_expenditure` (period 2)"),
            "{e}"
        );
    }

    /// R7 (#38): lines in a later stage keep the period numbering and the pointers of the
    /// engine's input.
    #[test]
    fn lines_after_a_stage_of_flows_keep_periods_and_pointers() {
        let line = json!({"ebit": "100", "depreciation_and_amortisation": "10",
                          "capital_expenditure": "12", "change_in_net_working_capital": "3"});
        let input = with(base(), |m| {
            m.insert(
                "stages".into(),
                json!([
                    {"periods": 2, "projection": "given", "flows": ["50", "60"]},
                    {"periods": 2, "projection": "given", "lines": [line.clone(), line.clone()]}
                ]),
            );
            m.insert("notional_tax_rate".into(), json!("0.24"));
            m.insert("negative_ebit".into(), json!("credit"));
        });
        let out = run("dcf.fcff", &input.to_string()).unwrap();
        let steps = out["explain"]["steps"].as_array().unwrap();
        assert!(steps
            .iter()
            .any(|s| s["label"] == "Build-up: FCFF, period 3"));
        assert!(!steps
            .iter()
            .any(|s| s["label"] == "Build-up: FCFF, period 1"));
        assert_eq!(out["value"]["periods"][2]["flow"], "71.00");
        let mut bad = input.clone();
        bad["stages"][1]["lines"][0]["depreciation_and_amortisation"] = json!("-1");
        let e = err(&bad);
        assert!(
            e.to_string()
                .contains("`/stages/1/lines/0/depreciation_and_amortisation` (period 3)"),
            "{e}"
        );
        // The standalone conventions of dcf.free_cash_flows are not reported by the DCF.
        let conventions = conventions(&input);
        let build_up: Vec<&String> = conventions
            .iter()
            .filter(|c| c.starts_with("free cash flow build-up"))
            .collect();
        assert!(!build_up.is_empty());
        assert!(build_up
            .iter()
            .all(|c| !c.contains("not discounted") && !c.contains("FCFE: not computed")));
        assert!(conventions.iter().any(|c| c.ends_with(
            "for the stages with lines: FCFF_t = EBIT_t − taxes on EBIT_t + D&A_t − capex_t − \
             ΔNWC_t + Σ adjustments_t"
        )));
    }

    /// AC14 (R27): every error raised by the engine names its field by a JSON pointer and
    /// states the fix; the cases cover every check of this issue.
    #[test]
    fn every_error_names_its_field_by_a_json_pointer() {
        let cases: Vec<(Value, &str, &str)> = vec![
            (
                with(base(), |m| {
                    m.insert("stages".into(), json!([]));
                }),
                "FM-E001",
                "/stages",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "stages".into(),
                        json!([{"periods": 0, "projection": "given", "flows": []}]),
                    );
                }),
                "FM-E001",
                "/stages/0/periods",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "stages".into(),
                        json!([{"periods": 2, "projection": "given", "flows": ["1"]}]),
                    );
                }),
                "FM-E025",
                "/stages/0/flows",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "stages".into(),
                        json!([{"periods": 2, "projection": "given"}]),
                    );
                }),
                "FM-E001",
                "/stages/0/flows",
            ),
            (
                with(base(), |m| {
                    m.insert("stages".into(), json!([{"periods": 2, "projection": "linear_decline", "from_growth": "0.1"}]));
                    m.insert("base_flow".into(), json!("1"));
                }),
                "FM-E001",
                "/stages/0/to_growth",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "stages".into(),
                        json!([{"periods": 2, "projection": "constant_growth", "growth": "-1.5"}]),
                    );
                    m.insert("base_flow".into(), json!("1"));
                }),
                "FM-E001",
                "/stages/0/growth",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "stages".into(),
                        json!([{"periods": 2, "projection": "constant_growth", "growth": "0.1"}]),
                    );
                }),
                "FM-E025",
                "/base_flow",
            ),
            (
                with(base(), |m| {
                    m.insert("base_flow".into(), json!("1"));
                }),
                "FM-E025",
                "/base_flow",
            ),
            (
                with(base(), |m| {
                    m.remove("wacc");
                }),
                "FM-E001",
                "/wacc",
            ),
            (
                with(base(), |m| {
                    m.insert("wacc_per_period".into(), json!(["0.1", "0.1", "0.1"]));
                }),
                "FM-E001",
                "/wacc_per_period",
            ),
            (
                with(base(), |m| {
                    m.insert("terminal_wacc".into(), json!("0.1"));
                }),
                "FM-E001",
                "/terminal_wacc",
            ),
            (
                with(base(), |m| {
                    m.remove("wacc");
                    m.insert("wacc_per_period".into(), json!(["0.1", "0.1", "0.1"]));
                }),
                "FM-E001",
                "/terminal_wacc",
            ),
            (
                with(base(), |m| {
                    m.remove("wacc");
                    m.insert("wacc_per_period".into(), json!(["0.1", "0.1"]));
                    m.insert("terminal_wacc".into(), json!("0.1"));
                }),
                "FM-E025",
                "/wacc_per_period",
            ),
            (
                with(base(), |m| {
                    m.remove("wacc");
                    m.insert("wacc_per_period".into(), json!(["0.1", "-1", "0.1"]));
                    m.insert("terminal_wacc".into(), json!("0.1"));
                }),
                "FM-E003",
                "/wacc_per_period/1",
            ),
            (
                with(base(), |m| {
                    m.remove("wacc");
                    m.insert("wacc_per_period".into(), json!(["0.1", "0.1", "0.1"]));
                    m.insert("terminal_wacc".into(), json!("-1"));
                }),
                "FM-E003",
                "/terminal_wacc",
            ),
            (
                with(base(), |m| {
                    m.insert("wacc".into(), json!("-2"));
                }),
                "FM-E003",
                "/wacc",
            ),
            (
                with(base(), |m| {
                    m.insert("cash_flow_terms".into(), json!("real"));
                }),
                "FM-E024",
                "/discount_rate_terms",
            ),
            (
                with(base(), |m| {
                    m.insert("terminal_discount_time".into(), json!("horizon"));
                }),
                "FM-E001",
                "/terminal_discount_time",
            ),
            (
                with(base(), |m| {
                    m.insert("timing".into(), json!("mid_period"));
                }),
                "FM-E001",
                "/terminal_discount_time",
            ),
            (
                with(base(), |m| {
                    m.insert("terminal".into(), json!({}));
                }),
                "FM-E001",
                "/terminal",
            ),
            (
                with(base(), |m| {
                    m.insert("terminal".into(), json!({"tv.gordon": {"growth": "0.02", "first_flow": "normalised_next_flow"}}));
                }),
                "FM-E001",
                "/terminal/tv.gordon/normalised_next_flow",
            ),
            (
                with(base(), |m| {
                    m.insert("terminal".into(), json!({"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown", "normalised_next_flow": "5"}}));
                }),
                "FM-E001",
                "/terminal/tv.gordon/normalised_next_flow",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "terminal".into(),
                        json!({"tv.gordon": {"growth": "-1", "first_flow": "last_flow_grown"}}),
                    );
                }),
                "FM-E001",
                "/terminal/tv.gordon/growth",
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
                        "non_operating_assets".into(),
                        json!([{"label": "Land", "value": "-1"}]),
                    );
                }),
                "FM-E001",
                "/non_operating_assets/0/value",
            ),
            (
                with(base(), |m| {
                    m.insert(
                        "non_operating_assets".into(),
                        json!([{"label": " ", "value": "1"}]),
                    );
                }),
                "FM-E001",
                "/non_operating_assets/0/label",
            ),
        ];
        // R7 (#38): the five refusals of a stage with build-up lines.
        let line = json!({"ebit": "100", "depreciation_and_amortisation": "10",
                          "capital_expenditure": "12", "change_in_net_working_capital": "3"});
        let lines_stage = |extra: Value| {
            let mut stage = json!({"periods": 2, "projection": "given",
                                   "lines": [line.clone(), line.clone()]});
            stage
                .as_object_mut()
                .unwrap()
                .extend(extra.as_object().unwrap().clone());
            stage
        };
        let with_lines = |stage: Value, taxes: bool| {
            with(base(), |m| {
                m.insert("stages".into(), json!([stage]));
                if taxes {
                    m.insert("notional_tax_rate".into(), json!("0.24"));
                    m.insert("negative_ebit".into(), json!("credit"));
                }
            })
        };
        let mut financing = line.clone();
        financing["interest_expense"] = json!("5");
        financing["net_borrowing"] = json!("0");
        let r7: Vec<(Value, &str, &str)> = vec![
            (
                with_lines(lines_stage(json!({"flows": ["1", "2"]})), true),
                "FM-E001",
                "/stages/0/flows",
            ),
            (
                with_lines(
                    json!({"periods": 3, "projection": "given",
                           "lines": [line.clone(), line.clone()]}),
                    true,
                ),
                "FM-E025",
                "/stages/0/lines",
            ),
            (
                with_lines(
                    json!({"periods": 2, "projection": "constant_growth", "growth": "0.02",
                           "lines": [line.clone(), line.clone()]}),
                    true,
                ),
                "FM-E001",
                "/stages/0/lines",
            ),
            (
                with(base(), |m| {
                    m.insert("notional_tax_rate".into(), json!("0.24"));
                }),
                "FM-E001",
                "/notional_tax_rate",
            ),
            (
                with_lines(
                    json!({"periods": 2, "projection": "given",
                           "lines": [line.clone(), financing]}),
                    true,
                ),
                "FM-E001",
                "/stages/0/lines/1/interest_expense",
            ),
        ];
        for (input, code, pointer) in cases.into_iter().chain(r7) {
            let e = err(&input);
            assert_eq!(e.code(), code, "{e}");
            assert!(
                e.to_string().contains(&format!("`{pointer}")),
                "{pointer} not in: {e}"
            );
        }
    }

    /// AC8: different terms are refused both ways.
    #[test]
    fn different_terms_are_refused_both_ways() {
        for (flows, rates) in [("real", "nominal"), ("nominal", "real")] {
            let v = with(base(), |m| {
                m.insert("cash_flow_terms".into(), json!(flows));
                m.insert("discount_rate_terms".into(), json!(rates));
            });
            let e = err(&v);
            assert_eq!(e.code(), "FM-E024");
            assert!(e.to_string().contains("/cash_flow_terms"));
        }
    }

    /// R29: every choice is required; a missing choice is refused, never defaulted.
    #[test]
    fn every_choice_is_required_or_documented_default() {
        // R29 as amended by Decision 24: `stages` has no default; the terms, the timing and
        // the terminal option take their documented default, reported in meta.
        let v = with(base(), |m| {
            m.remove("stages");
        });
        assert_eq!(err(&v).code(), "FM-E001");
        for field in [
            "cash_flow_terms",
            "discount_rate_terms",
            "timing",
            "terminal",
        ] {
            let v = with(base(), |m| {
                m.remove(field);
            });
            let out = run("dcf.fcff", &v.to_string()).unwrap();
            assert_eq!(
                out["meta"]["defaults_applied"][0]["field"],
                format!("/{field}"),
                "{field}"
            );
        }
    }

    /// AC10 (R11): growth on a negative flow and a growth discontinuity are warnings, not
    /// errors.
    #[test]
    fn warnings_are_reported_not_refused() {
        let v = with(base(), |m| {
            m.insert(
                "stages".into(),
                json!([
                    {"periods": 1, "projection": "given", "flows": ["-10"]},
                    {"periods": 2, "projection": "constant_growth", "growth": "0.05"}
                ]),
            );
        });
        let c = conventions(&v);
        let warnings: Vec<&String> = c.iter().filter(|c| c.starts_with("warning: ")).collect();
        // Two negative-flow warnings, the growth discontinuity, the net financial position not
        // given (Decision 25) and (R26) the negative equity value that these flows produce.
        assert_eq!(warnings.len(), 5, "{c:?}");
        assert!(warnings[3].contains("no net financial position given"));
        assert!(warnings[4].contains("equity value is negative"));
        assert!(warnings[0].contains("period 2") && warnings[0].contains("negative flow"));
        assert!(warnings[2].contains("differs from the perpetual growth"));
    }

    /// AC14 (R28): every convention that changes a number is reported.
    #[test]
    fn every_convention_is_reported() {
        let v = with(base(), |m| {
            m.insert("timing".into(), json!("mid_period"));
            m.insert("terminal_discount_time".into(), json!("mid_period"));
            m.insert(
                "net_financial_position".into(),
                json!({"gross_debt": "10", "cash": "2"}),
            );
        });
        let c = conventions(&v).join("\n");
        for needle in [
            "basis: enterprise",
            "terms: cash flows and growth rates nominal, discount rates nominal",
            "stage 1: periods 1 to 3, given flows",
            "explicit horizon N = 3 periods",
            "discount rate: one rate",
            "timing: mid period",
            "terminal technique: tv.gordon version 1 on the enterprise basis",
            "first terminal flow: the last explicit flow grown",
            "terminal discounting time: mid_period, t = N − 0.5",
            "bridge items measured at the valuation date",
            "non-operating assets: none given",
            "net financial position = gross debt − cash + other items",
        ] {
            assert!(c.contains(needle), "missing '{needle}' in:\n{c}");
        }
        let c = conventions(&base()).join("\n");
        assert!(c.contains(WARNING_NO_CLAIMS));
        assert!(c.contains("default: `/net_financial_position` not given"));
        assert!(c.contains("terminal value discounted at the horizon, t = N"));
    }

    /// AC12 (R21): mid-period terminal discounting at N − 0.5 is DF_N × (1 + r_T)^0.5, and the
    /// discounting time is reported.
    #[test]
    fn mid_period_terminal_discounting_time() {
        let v = with(base(), |m| {
            m.insert("timing".into(), json!("mid_period"));
            m.insert("terminal_discount_time".into(), json!("mid_period"));
        });
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        assert_eq!(out["value"]["terminal"]["discount_time"], "2.5");
        let h = with(v.clone(), |m| {
            m.insert("terminal_discount_time".into(), json!("horizon"));
        });
        let out_h = run("dcf.fcff", &h.to_string()).unwrap();
        assert_eq!(out_h["value"]["terminal"]["discount_time"], "3");
        let factor = |o: &Value| -> rust_decimal::Decimal {
            o["value"]["terminal"]["discount_factor"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap()
        };
        assert!(factor(&out) > factor(&out_h));
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
        walk(&input_schema("dcf.fcff").unwrap(), "");
    }

    /// R26: a negative equity value is a warning, reported and never floored at zero.
    #[test]
    fn negative_equity_value_is_a_warning() {
        let v = with(base(), |m| {
            m.insert(
                "net_financial_position".into(),
                json!({"gross_debt": "5000", "cash": "0"}),
            );
        });
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        let equity: rust_decimal::Decimal = out["value"]["equity_value"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(equity.is_sign_negative());
        let c = out["explain"]["conventions"].to_string();
        assert!(c.contains("warning: the equity value is negative"), "{c}");
        assert!(!conventions(&base())
            .join("\n")
            .contains("equity value is negative"));
    }

    /// Decision 25 (#261), R23: without a net financial position the DCF is computed with the
    /// documented default of zero, listed in `meta.defaults_applied` and reported first as a
    /// default, with a warning that the equity value equals the enterprise value.
    #[test]
    fn net_financial_position_not_given_is_a_reported_default() {
        let out = run("dcf.fcff", &base().to_string()).unwrap();
        let defaults = out["meta"]["defaults_applied"].as_array().unwrap();
        assert_eq!(
            defaults.as_slice(),
            [json!({
                "field": "/net_financial_position",
                "value": "0",
                "reason": "net financial position not given"
            })]
        );
        let c: Vec<String> = serde_json::from_value(out["explain"]["conventions"].clone()).unwrap();
        assert_eq!(
            c[0],
            "default: `/net_financial_position` not given, set to \"0\": net financial position \
             not given"
        );
        assert!(
            c.contains(&WARNING_NO_CLAIMS.to_owned()),
            "missing the warning in {c:?}"
        );
        assert!(!c.iter().any(|c| c == "net financial position: none given"));
        // No number changes: equity value equals enterprise value; the bridge echoes the input.
        assert_eq!(
            out["value"]["equity_value"],
            out["value"]["enterprise_value"]
        );
        assert_eq!(
            out["value"]["bridge"]["equity_value"],
            out["value"]["enterprise_value"]
        );
        assert!(out["value"]["bridge"]["net_financial_position"].is_null());
        let steps = out["explain"]["steps"].as_array().unwrap();
        let equity = steps.iter().find(|s| s["label"] == "Equity value").unwrap();
        assert!(
            equity["formula"].as_str().unwrap().contains(" − 0 − "),
            "{equity}"
        );
    }

    /// Decision 25 (#261): with debt-like items or non-controlling interests and no net
    /// financial position, the warning says that only those are subtracted.
    #[test]
    fn net_financial_position_not_given_with_claims_names_them() {
        let v = with(bridge_input(), |m| {
            m.remove("net_financial_position");
        });
        let c = conventions(&v);
        assert!(c.contains(&WARNING_WITH_CLAIMS.to_owned()), "{c:?}");
        assert!(!c.contains(&WARNING_NO_CLAIMS.to_owned()), "{c:?}");
        // Only the lists given are named (code review of #261).
        let only = |drop: &str| {
            let v = with(v.clone(), |m| {
                m.remove(drop);
            });
            conventions(&v)
                .into_iter()
                .find(|c| c.starts_with("warning: no net financial position given"))
                .unwrap()
        };
        assert!(
            only("non_controlling_interests")
                .contains("equals the enterprise value less the debt-like items given; give"),
            "{}",
            only("non_controlling_interests")
        );
        assert!(
            only("debt_like_items").contains(
                "equals the enterprise value less the non-controlling interests given; give"
            ),
            "{}",
            only("debt_like_items")
        );
    }

    /// Decision 25 (#261): a net financial position given as zero is a true zero: the same
    /// value, no default entry and no warning.
    #[test]
    fn net_financial_position_given_as_zero_is_a_true_zero() {
        let zero = with(base(), |m| {
            m.insert(
                "net_financial_position".into(),
                json!({"gross_debt": "0", "cash": "0"}),
            );
        });
        let given = run("dcf.fcff", &zero.to_string()).unwrap();
        let absent = run("dcf.fcff", &base().to_string()).unwrap();
        assert!(given["meta"].get("defaults_applied").is_none());
        assert_eq!(
            given["value"]["equity_value"],
            absent["value"]["equity_value"]
        );
        assert_eq!(given["value"]["bridge"]["net_financial_position"], "0");
        let c = conventions(&zero);
        assert!(!c.iter().any(|c| c.contains("net financial position given")
            || c.contains("no net financial position")));
        assert_ne!(given["meta"]["input_hash"], absent["meta"]["input_hash"]);
    }

    const WARNING_NO_CLAIMS: &str = "warning: no net financial position given \
        (`/net_financial_position`): zero is subtracted, so the equity value equals the \
        enterprise value; give the net financial position to value the equity";

    const WARNING_WITH_CLAIMS: &str = "warning: no net financial position given \
        (`/net_financial_position`): zero is subtracted, so the equity value equals the \
        enterprise value less the debt-like items and non-controlling interests given; give the \
        net financial position to value the equity";

    /// Under mid-period timing, a Gordon value discounted at the horizon carries a warning.
    #[test]
    fn mid_period_gordon_at_the_horizon_is_a_warning() {
        let v = with(base(), |m| {
            m.insert("timing".into(), json!("mid_period"));
            m.insert("terminal_discount_time".into(), json!("horizon"));
        });
        assert!(conventions(&v)
            .iter()
            .any(|c| c.starts_with("warning: under mid-period timing")));
        let m = with(v, |m| {
            m.insert("terminal_discount_time".into(), json!("mid_period"));
        });
        let c = conventions(&m);
        assert!(!c
            .iter()
            .any(|c| c.starts_with("warning: under mid-period timing")));
        assert!(
            c.iter()
                .any(|c| c.contains("r_T = 0.10 (R21, Decision 16)")),
            "{c:?}"
        );
    }

    /// R23, R25: enterprise value minus value of operations is the sum of the non-operating
    /// assets, and the terminal value weight is computed on the value of operations.
    #[test]
    fn bridge_properties() {
        let v = with(base(), |m| {
            m.insert(
                "non_operating_assets".into(),
                json!([{"label": "Land", "value": "60"}, {"label": "Shares", "value": "40.5"}]),
            );
        });
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        let d = |p: &str| -> rust_decimal::Decimal {
            out["value"]
                .pointer(p)
                .unwrap()
                .as_str()
                .unwrap()
                .parse()
                .unwrap()
        };
        assert_eq!(
            d("/enterprise_value") - d("/value_of_operations"),
            "100.5".parse().unwrap()
        );
        assert_eq!(
            d("/terminal_value_weight"),
            d("/terminal/present_value") / d("/value_of_operations")
        );
        assert_eq!(d("/equity_value"), d("/enterprise_value"));
        assert_eq!(
            out["value"]["bridge"]["non_operating_assets"][1]["label"],
            "Shares"
        );
    }

    fn bridge_input() -> Value {
        with(base(), |m| {
            m.insert(
                "net_financial_position".into(),
                json!({"gross_debt": "300", "cash": "40",
                       "other_items": [{"name": "lease liabilities", "amount": "25"}]}),
            );
            m.insert(
                "non_operating_assets".into(),
                json!([{"label": "Land", "value": "60"}]),
            );
            m.insert(
                "debt_like_items".into(),
                json!([
                    {"label": "Unfunded pension obligation", "value": "70",
                     "measurement_basis": "present_value", "basis_note": "net of tax"},
                    {"label": "Restructuring provision", "value": "12.5",
                     "measurement_basis": "book_value"}
                ]),
            );
            m.insert(
                "non_controlling_interests".into(),
                json!([{"label": "20% of Subsidiary A", "value": "45",
                        "measurement_basis": "fair_value"}]),
            );
        })
    }

    /// R23, R25 (#44): equity value = enterprise value − net financial position − Σ debt-like
    /// items − Σ non-controlling interests, every line echoed with its label and basis.
    #[test]
    fn bridge_subtracts_debt_like_items_and_non_controlling_interests() {
        let out = run("dcf.fcff", &bridge_input().to_string()).unwrap();
        let d = |p: &str| -> rust_decimal::Decimal {
            out["value"]
                .pointer(p)
                .unwrap()
                .as_str()
                .unwrap()
                .parse()
                .unwrap()
        };
        assert_eq!(d("/bridge/net_financial_position"), "285".parse().unwrap());
        assert_eq!(d("/bridge/debt_like_items_total"), "82.5".parse().unwrap());
        assert_eq!(
            d("/bridge/non_controlling_interests_total"),
            "45".parse().unwrap()
        );
        assert_eq!(
            d("/equity_value"),
            d("/enterprise_value")
                - d("/bridge/net_financial_position")
                - d("/bridge/debt_like_items_total")
                - d("/bridge/non_controlling_interests_total")
        );
        assert_eq!(d("/bridge/equity_value"), d("/equity_value"));
        assert_eq!(
            out["value"]["bridge"]["debt_like_items"][1]["measurement_basis"],
            "book_value"
        );
        assert_eq!(
            out["value"]["bridge"]["debt_like_items"][0]["basis_note"],
            "net of tax"
        );
        assert!(out["value"]["bridge"]["debt_like_items"][1]
            .get("basis_note")
            .is_none());
        assert_eq!(
            out["value"]["bridge"]["non_controlling_interests"][0]["label"],
            "20% of Subsidiary A"
        );
        let steps = out["explain"]["steps"].as_array().unwrap();
        let step = |label: &str| {
            steps
                .iter()
                .find(|s| s["label"] == label)
                .unwrap_or_else(|| panic!("no step '{label}' in {steps:?}"))
                .clone()
        };
        assert_eq!(
            step("Debt-like item: Unfunded pension obligation (present_value: net of tax)")
                ["value"],
            "70"
        );
        assert_eq!(step("Debt-like items, total")["formula"], "70 + 12.5");
        assert_eq!(
            step("Non-controlling interest: 20% of Subsidiary A (fair_value)")["value"],
            "45"
        );
        assert_eq!(step("Non-controlling interests, total")["formula"], "45");
        let equity = step("Equity value");
        let formula = equity["formula"].as_str().unwrap();
        assert!(
            formula.ends_with(" − 285 − 82.5 − 45"),
            "equity value formula without every term substituted: {formula}"
        );
    }

    /// R25, R28 (#44): signs, the stated measurement basis of each claim, and "none given"
    /// for empty lists are reported.
    #[test]
    fn bridge_conventions_are_reported() {
        let c = conventions(&bridge_input()).join("\n");
        for needle in [
            "bridge items measured at the valuation date, as given",
            "bridge signs: non-operating assets added to the value of operations; net financial \
             position (positive for net debt), debt-like items and non-controlling interests \
             subtracted from enterprise value; every debt-like item and non-controlling \
             interest is zero or more",
            "debt-like item 'Unfunded pension obligation': measurement basis present_value \
             (present value of the obligation), stated by the analyst; note: net of tax",
            "debt-like item 'Restructuring provision': measurement basis book_value (book \
             value per the accounts), stated by the analyst",
            "non-controlling interest '20% of Subsidiary A': measurement basis fair_value (fair \
             value as defined by IFRS 13), stated by the analyst",
        ] {
            assert!(c.contains(needle), "missing '{needle}' in:\n{c}");
        }
        assert!(!c.contains("debt-like items: none given"));
        let c = conventions(&base()).join("\n");
        assert!(c.contains("debt-like items: none given"), "{c}");
        assert!(c.contains("non-controlling interests: none given"), "{c}");
    }

    /// R26 (#44): claims above the enterprise value give a negative equity value, reported
    /// with the warning and not floored.
    #[test]
    fn claims_above_enterprise_value_are_a_warning() {
        let v = with(base(), |m| {
            m.insert(
                "debt_like_items".into(),
                json!([{"label": "Pension", "value": "1000", "measurement_basis": "book_value"}]),
            );
            m.insert(
                "non_controlling_interests".into(),
                json!([{"label": "Minority", "value": "500", "measurement_basis": "book_value"}]),
            );
        });
        let out = run("dcf.fcff", &v.to_string()).unwrap();
        let equity: rust_decimal::Decimal = out["value"]["equity_value"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert!(equity.is_sign_negative());
        let c = out["explain"]["conventions"].to_string();
        assert!(c.contains("warning: the equity value is negative"), "{c}");
    }

    /// R25, R27 (#44): every refusal of a bridge claim names its field by JSON pointer and
    /// states the fix.
    #[test]
    fn bridge_claim_errors_name_their_field() {
        let claim = |label: &str, value: &str, basis: &str| json!([{"label": label, "value": value, "measurement_basis": basis}]);
        let noted = |basis: &str, note: &str| {
            json!([{"label": "Pension", "value": "1", "measurement_basis": basis,
                    "basis_note": note}])
        };
        let cases = [
            (
                "debt_like_items",
                claim("Pension", "-1", "book_value"),
                "`/debt_like_items/0/value`",
                "non_operating_assets",
            ),
            (
                "non_controlling_interests",
                claim("Minority", "-1", "book_value"),
                "`/non_controlling_interests/0/value`",
                "a non-controlling interest is zero or more (SPEC-0001 R25); give zero or a \
                 positive amount",
            ),
            (
                "debt_like_items",
                claim(" ", "1", "market_value"),
                "`/debt_like_items/0/label`",
                "name the item",
            ),
            (
                "non_controlling_interests",
                claim("", "1", "fair_value"),
                "`/non_controlling_interests/0/label`",
                "name the item",
            ),
            (
                "debt_like_items",
                claim("Pension", "1", "other"),
                "`/debt_like_items/0/basis_note`",
                "with `measurement_basis` `other`, describe in `basis_note` how the amount was \
                 measured",
            ),
            (
                "non_controlling_interests",
                noted("other", "  "),
                "`/non_controlling_interests/0/basis_note`",
                "with `measurement_basis` `other`, describe in `basis_note` how the amount was \
                 measured",
            ),
            (
                "debt_like_items",
                noted("book_value", ""),
                "`/debt_like_items/0/basis_note`",
                "write the note or remove the field",
            ),
        ];
        for (field, items, pointer, fix) in cases {
            let v = with(base(), |m| {
                m.insert(field.into(), items);
            });
            let e = err(&v);
            assert_eq!(e.code(), "FM-E001", "{e}");
            let text = e.to_string();
            assert!(text.contains(pointer), "{pointer} not in: {text}");
            assert!(text.contains(fix), "'{fix}' not in: {text}");
        }
        // A missing measurement basis, or one that is not an option, is refused, never
        // defaulted (R29, Decision 21).
        for items in [
            json!([{"label": "Pension", "value": "1"}]),
            json!([{"label": "Pension", "value": "1", "measurement_basis": "book value"}]),
            json!([{"label": "Pension", "value": "1", "measurement_basis": ""}]),
        ] {
            let v = with(base(), |m| {
                m.insert("debt_like_items".into(), items);
            });
            assert_eq!(err(&v).code(), "FM-E001");
        }
    }

    /// Decision 21: `other` with a note is accepted and the note is echoed in the step, the
    /// convention and the output; every option of the enumeration is accepted.
    #[test]
    fn every_measurement_basis_is_accepted_and_echoed() {
        for basis in [
            "book_value",
            "market_value",
            "fair_value",
            "present_value",
            "other",
        ] {
            let v = with(base(), |m| {
                m.insert(
                    "non_controlling_interests".into(),
                    json!([{"label": "Minority", "value": "10", "measurement_basis": basis,
                            "basis_note": "valued by a multiple of the subsidiary's earnings"}]),
                );
            });
            let out = run("dcf.fcff", &v.to_string()).unwrap();
            let claim = &out["value"]["bridge"]["non_controlling_interests"][0];
            assert_eq!(claim["measurement_basis"], basis);
            assert_eq!(
                claim["basis_note"],
                "valued by a multiple of the subsidiary's earnings"
            );
            let steps = out["explain"]["steps"].to_string();
            assert!(
                steps.contains(&format!(
                    "Non-controlling interest: Minority ({basis}: valued by a multiple of the \
                     subsidiary's earnings)"
                )),
                "{steps}"
            );
            let c = out["explain"]["conventions"].to_string();
            assert!(
                c.contains(&format!(
                    "non-controlling interest 'Minority': measurement basis {basis}"
                )) && c.contains("note: valued by a multiple of the subsidiary's earnings"),
                "{c}"
            );
        }
    }

    /// Decision 21: no measurement basis on the non-operating assets yet (pending the
    /// maintainer's decision under the schema of version 3); the input is unchanged.
    #[test]
    fn non_operating_assets_keep_their_shape() {
        let v = with(base(), |m| {
            m.insert(
                "non_operating_assets".into(),
                json!([{"label": "Land", "value": "1", "measurement_basis": "book_value"}]),
            );
        });
        assert_eq!(err(&v).code(), "FM-E001");
    }
}
