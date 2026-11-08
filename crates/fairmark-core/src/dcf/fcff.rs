//! `dcf.fcff` version 3: enterprise DCF, free cash flows to the firm discounted at the WACC
//! over explicit stages and a terminal value, then value of operations, enterprise value and
//! equity value (SPEC-0001 R1, R6 to R15, R19 to R23, R25, R26).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::engine::{
    self, EngineInput, InflationInput, PeriodRow, RateFields, RateInput, Stage,
    TerminalDiscountTime, Terms, TermsOutput,
};
use super::free_cash_flows::{NegativeEbit, TaxChoice};
use super::terminal::{Terminal, TerminalOutput};
use crate::error::{FairmarkError, Result};
use crate::explain::Explain;
use crate::method::Method;
use crate::num::{add, div, sub};
use crate::ratios::{NetFinancialPosition, NFP_CONVENTION};
use crate::tv::{Line, TerminalBasis};
use crate::types::Timing;

/// Input of `dcf.fcff` (version 3). Every choice is required, with no default (R29).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DcfFcffInput {
    /// One to three explicit stages, in order; the explicit horizon N is the sum of their
    /// periods. Each stage is `{"periods", "projection", ...}` with the fields of its
    /// projection.
    #[schemars(length(min = 1))]
    pub stages: Vec<Stage>,
    /// FCFF of the base period CF_0, in currency units: required when the first stage is
    /// `constant_growth` or `linear_decline`, refused when it is `given`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_flow: Option<Decimal>,
    /// For stages with `lines`: tax rate applied to EBIT, decimal fraction from 0 to below 1
    /// (0.24 = 24%). Give this, or `operating_taxes` in every line, not both; refused when no
    /// stage has `lines`. Default: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notional_tax_rate: Option<Decimal>,
    /// For stages with `lines` and `notional_tax_rate`: treatment of a negative EBIT, `credit`
    /// (taxes = rate × EBIT) or `zero` (no tax); required with `notional_tax_rate`, no default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub negative_ebit: Option<NegativeEbit>,
    /// `nominal` or `real`: the terms of every amount and growth rate of the input.
    pub cash_flow_terms: Terms,
    /// `nominal` or `real`: the terms of the discount rates. When they differ from
    /// `cash_flow_terms`, expected inflation is required (`inflation`, or
    /// `inflation_per_period` with `terminal_inflation`) and every rate is converted to the
    /// terms of the cash flows; the cash flows are never converted.
    pub discount_rate_terms: Terms,
    /// Expected inflation, decimal fraction per period (0.02 = 2%), greater than -1, for every
    /// explicit period and for the terminal phase. Give this or `inflation_per_period`, or
    /// neither. With different terms it converts the discount rates by the exact Fisher
    /// relation; with equal terms it only restates the perpetual growth in the other terms.
    /// Default: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inflation: Option<Decimal>,
    /// Expected inflation per period of the explicit horizon, decimal fractions, exactly N of
    /// them, each greater than -1; requires `terminal_inflation`. Give this or `inflation`,
    /// not both. Default: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inflation_per_period: Option<Vec<Decimal>>,
    /// Expected inflation of the terminal phase, decimal fraction per period, greater than -1;
    /// required with `inflation_per_period`, refused otherwise.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_inflation: Option<Decimal>,
    /// One WACC for every period and for the terminal value, decimal fraction per period
    /// (0.09 = 9%), greater than -1. Give this or `wacc_per_period`, not both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wacc: Option<Decimal>,
    /// One WACC per period of the explicit horizon, decimal fractions, exactly N of them;
    /// requires `terminal_wacc`. Give this or `wacc`, not both.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wacc_per_period: Option<Vec<Decimal>>,
    /// WACC of the terminal phase, decimal fraction per period; required with
    /// `wacc_per_period`, refused with `wacc`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_wacc: Option<Decimal>,
    /// `end_of_period` (flow k at t = k) or `mid_period` (t = k − 0.5); no default.
    pub timing: Timing,
    /// With `mid_period` timing, required: `horizon` (terminal value at t = N) or
    /// `mid_period` (t = N − 0.5). Refused with `end_of_period`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub terminal_discount_time: Option<TerminalDiscountTime>,
    /// The terminal technique, an object with exactly one key, the method id, for example
    /// `{"tv.gordon": {"growth", "first_flow", ...}}`: `tv.gordon`, `tv.h_model`,
    /// `tv.value_driver`, `tv.exit_multiple`, `tv.limited_horizon`, `tv.normalised_income` or
    /// `tv.liquidation`, on the enterprise basis.
    pub terminal: Terminal,
    /// Net financial position at the valuation date (SPEC-0007): gross debt − cash + signed
    /// other items, positive for net debt, subtracted from enterprise value. Default: none,
    /// reported.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net_financial_position: Option<NetFinancialPosition>,
    /// Non-operating assets at the valuation date, each a label and a value in currency units
    /// (zero or more), added to the value of operations. Default: none, reported.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub non_operating_assets: Vec<Line>,
    /// Debt-like items (debt equivalents) at the valuation date, for example unfunded pension
    /// obligations or provisions, each a label, a value in currency units (zero or more) and
    /// its stated measurement basis, subtracted from enterprise value. Do not repeat an item
    /// already among the `other_items` of the net financial position. Default: none, reported.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub debt_like_items: Vec<BridgeClaim>,
    /// Non-controlling interests (minority interests) at the valuation date, each a label, a
    /// value in currency units (zero or more) and its stated measurement basis (for example
    /// book value or fair value), subtracted from enterprise value. Default: none, reported.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub non_controlling_interests: Vec<BridgeClaim>,
}

/// A claim on the value of the company other than the net financial position, subtracted from
/// enterprise value to reach equity value: a debt-like item or a non-controlling interest
/// (R25). Fairmark subtracts the amount as given and does not choose how it is measured.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BridgeClaim {
    /// What the claim is, for example "Unfunded pension obligation" or "30% of Subsidiary A".
    /// Not empty.
    pub label: String,
    /// Amount at the valuation date, in currency units, zero or more; subtracted.
    pub value: Decimal,
    /// How the amount was measured, stated by the analyst, for example "book value", "fair
    /// value" or "present value net of tax". Not empty. Reported with the amount, never used
    /// in the calculation.
    pub measurement_basis: String,
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
    /// Net financial position subtracted, positive for net debt; `null` when none is given.
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
    /// Present value of the explicit FCFF, in currency units.
    pub pv_explicit_period: Decimal,
    /// Present value of the terminal value divided by the value of operations; `null` when
    /// the value of operations is zero.
    pub terminal_value_weight: Option<Decimal>,
    /// One row per period of the explicit horizon.
    pub periods: Vec<PeriodRow>,
    /// The terms of the flows and of the rates, the conversion of the rates and the perpetual
    /// growth in the other terms (R14 to R18).
    pub terms: TermsOutput,
    /// The terminal value, its technique and its discounting.
    pub terminal: TerminalOutput,
    /// Every line from value of operations to equity value.
    pub bridge: FcffBridge,
}

const FORMULA: &str = "Value of operations = Σ FCFF_k × DF_k + TV_N × DF_TV; Enterprise value = \
    value of operations + Σ non-operating assets; Equity value = enterprise value − net \
    financial position − Σ debt-like items − Σ non-controlling interests";

pub fn fcff(input: &DcfFcffInput) -> Result<(DcfFcffOutput, Explain)> {
    let mut ex = Explain::new("dcf.fcff", FORMULA);
    ex.convention("basis: enterprise, free cash flows to the firm (FCFF) discounted at the WACC");
    let engine_input = EngineInput {
        taxes: TaxChoice {
            notional_tax_rate: input.notional_tax_rate,
            negative_ebit: input.negative_ebit,
            interest_tax_rate: None,
        },
        basis: TerminalBasis::Enterprise,
        stages: &input.stages,
        base_flow: input.base_flow,
        cash_flow_terms: input.cash_flow_terms,
        discount_rate_terms: input.discount_rate_terms,
        inflation: InflationInput {
            single: input.inflation,
            per_period: input.inflation_per_period.as_deref(),
            terminal: input.terminal_inflation,
        },
        rates: RateInput {
            single: input.wacc,
            per_period: input.wacc_per_period.as_deref(),
            terminal: input.terminal_wacc,
            fields: RateFields {
                single: "wacc",
                per_period: "wacc_per_period",
                terminal: "terminal_wacc",
            },
        },
        timing: input.timing,
        terminal_discount_time: input.terminal_discount_time,
        terminal: &input.terminal,
    };
    // Bridge items are checked before any calculation, like every other input.
    for (i, a) in input.non_operating_assets.iter().enumerate() {
        check_labelled_amount(
            &format!("/non_operating_assets/{i}"),
            &a.label,
            a.value,
            "name the asset",
            "a non-operating asset is zero or more (obligations belong in the net financial \
             position)",
        )?;
    }
    check_claims(
        &input.debt_like_items,
        "debt_like_items",
        "a debt-like item is zero or more (an asset belongs in `non_operating_assets`)",
    )?;
    check_claims(
        &input.non_controlling_interests,
        "non_controlling_interests",
        "a non-controlling interest is zero or more (SPEC-0001 R25); give zero or a positive \
         amount",
    )?;
    let out = engine::run(&engine_input, &mut ex)?;

    let pv_tv = out.terminal.present_value;
    let value_of_operations = add(out.pv_explicit, pv_tv)?;
    ex.step(
        "Value of operations",
        format!("{} + {pv_tv}", out.pv_explicit),
        value_of_operations,
    );
    let bridge = bridge(&mut ex, input, value_of_operations)?;
    let weight = if value_of_operations.is_zero() {
        ex.convention("terminal value weight not defined: the value of operations is zero");
        None
    } else {
        let w = div(pv_tv, value_of_operations)?;
        ex.step(
            "Terminal value weight",
            format!("{pv_tv} / {value_of_operations}"),
            w,
        );
        Some(w)
    };
    Ok((
        DcfFcffOutput {
            basis: TerminalBasis::Enterprise,
            value_of_operations,
            enterprise_value: bridge.enterprise_value,
            equity_value: bridge.equity_value,
            pv_explicit_period: out.pv_explicit,
            terminal_value_weight: weight,
            periods: out.periods,
            terms: out.terms,
            terminal: out.terminal,
            bridge,
        },
        ex,
    ))
}

/// R25, R27: a bridge line at `pointer` has a value of zero or more and a label that is not
/// empty; each refusal names the field by its JSON pointer and states the fix. Shared by the
/// non-operating assets, the debt-like items and the non-controlling interests.
fn check_labelled_amount(
    pointer: &str,
    label: &str,
    value: Decimal,
    label_fix: &str,
    sign_rule: &str,
) -> Result<()> {
    if value.is_sign_negative() && !value.is_zero() {
        return Err(FairmarkError::InvalidInput(format!(
            "`{pointer}/value` is {value}; {sign_rule}"
        )));
    }
    if label.trim().is_empty() {
        return Err(FairmarkError::InvalidInput(format!(
            "`{pointer}/label` is empty; {label_fix}"
        )));
    }
    Ok(())
}

/// R25, R27: every claim has a label, a stated measurement basis and a value of zero or more;
/// each refusal names the field by its JSON pointer and states the fix.
fn check_claims(claims: &[BridgeClaim], field: &str, sign_rule: &str) -> Result<()> {
    for (i, c) in claims.iter().enumerate() {
        check_labelled_amount(
            &format!("/{field}/{i}"),
            &c.label,
            c.value,
            "name the item",
            sign_rule,
        )?;
        if c.measurement_basis.trim().is_empty() {
            return Err(FairmarkError::InvalidInput(format!(
                "`/{field}/{i}/measurement_basis` is empty; state how the amount was measured, \
                 for example book value or fair value"
            )));
        }
    }
    Ok(())
}

/// R25: each claim as an explain step labelled with its basis, its stated basis as a
/// convention, and the total with the values substituted.
fn claims_total(
    ex: &mut Explain,
    claims: &[BridgeClaim],
    singular: &str,
    plural: &str,
) -> Result<Decimal> {
    if claims.is_empty() {
        ex.convention(format!("{}: none given", plural.to_lowercase()));
        return Ok(Decimal::ZERO);
    }
    let mut total = Decimal::ZERO;
    let mut terms = Vec::with_capacity(claims.len());
    for c in claims {
        ex.step(
            format!("{singular}: {} ({})", c.label, c.measurement_basis),
            "as given",
            c.value,
        );
        ex.convention(format!(
            "{} '{}': measurement basis stated by the analyst: {}",
            singular.to_lowercase(),
            c.label,
            c.measurement_basis
        ));
        total = add(total, c.value)?;
        terms.push(c.value.to_string());
    }
    ex.step(format!("{plural}, total"), terms.join(" + "), total);
    Ok(total)
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
    let mut total = Decimal::ZERO;
    if input.non_operating_assets.is_empty() {
        ex.convention("non-operating assets: none given");
    }
    for a in &input.non_operating_assets {
        ex.step(
            format!("Non-operating asset: {}", a.label),
            "as given",
            a.value,
        );
        total = add(total, a.value)?;
    }
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
        None => {
            ex.convention("net financial position: none given");
            None
        }
    };
    let debt_like = claims_total(
        ex,
        &input.debt_like_items,
        "Debt-like item",
        "Debt-like items",
    )?;
    let non_controlling = claims_total(
        ex,
        &input.non_controlling_interests,
        "Non-controlling interest",
        "Non-controlling interests",
    )?;
    let equity_value = sub(
        sub(sub(enterprise_value, nfp.unwrap_or_default())?, debt_like)?,
        non_controlling,
    )?;
    if equity_value.is_sign_negative() && !equity_value.is_zero() {
        // R26: reported, never floored at zero and never refused.
        ex.convention(format!(
            "warning: the equity value is negative ({equity_value}): the claims subtracted \
             exceed the enterprise value; it is reported as computed, not floored at zero"
        ));
    }
    ex.step(
        "Equity value",
        format!(
            "{enterprise_value} − {} − {debt_like} − {non_controlling}",
            nfp.unwrap_or_default()
        ),
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
        liquidation). Returns every period and step for audit.";
    const VERSION: &'static str = "3";
    type Input = DcfFcffInput;
    type Output = DcfFcffOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        fcff(input)
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
            "free cash flow build-up (dcf.free_cash_flows v1): tax form: rate on EBIT"
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
    fn every_choice_is_required() {
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
        // Two negative-flow warnings, the growth discontinuity, and (R26) the negative
        // equity value that these flows produce.
        assert_eq!(warnings.len(), 4, "{c:?}");
        assert!(warnings[3].contains("equity value is negative"));
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
        assert!(c.contains("net financial position: none given"));
        assert!(c.contains("terminal value discounted at the horizon, t = N"));
    }

    /// AC12 (R21): mid-period terminal discounting at N − 0.5 is DF_N × (1 + r_N)^0.5, and the
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
                     "measurement_basis": "present value net of tax"},
                    {"label": "Restructuring provision", "value": "12.5",
                     "measurement_basis": "book value"}
                ]),
            );
            m.insert(
                "non_controlling_interests".into(),
                json!([{"label": "20% of Subsidiary A", "value": "45",
                        "measurement_basis": "fair value"}]),
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
            "book value"
        );
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
            step("Debt-like item: Unfunded pension obligation (present value net of tax)")["value"],
            "70"
        );
        assert_eq!(step("Debt-like items, total")["formula"], "70 + 12.5");
        assert_eq!(
            step("Non-controlling interest: 20% of Subsidiary A (fair value)")["value"],
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
            "debt-like item 'Unfunded pension obligation': measurement basis stated by the \
             analyst: present value net of tax",
            "non-controlling interest '20% of Subsidiary A': measurement basis stated by the \
             analyst: fair value",
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
                json!([{"label": "Pension", "value": "1000", "measurement_basis": "book value"}]),
            );
            m.insert(
                "non_controlling_interests".into(),
                json!([{"label": "Minority", "value": "500", "measurement_basis": "book value"}]),
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
        let cases = [
            (
                "debt_like_items",
                claim("Pension", "-1", "book value"),
                "`/debt_like_items/0/value`",
                "non_operating_assets",
            ),
            (
                "non_controlling_interests",
                claim("Minority", "-1", "book value"),
                "`/non_controlling_interests/0/value`",
                "a non-controlling interest is zero or more (SPEC-0001 R25); give zero or a \
                 positive amount",
            ),
            (
                "debt_like_items",
                claim(" ", "1", "book value"),
                "`/debt_like_items/0/label`",
                "name the item",
            ),
            (
                "non_controlling_interests",
                claim("", "1", "book value"),
                "`/non_controlling_interests/0/label`",
                "name the item",
            ),
            (
                "debt_like_items",
                claim("Pension", "1", "  "),
                "`/debt_like_items/0/measurement_basis`",
                "state how the amount was measured",
            ),
            (
                "non_controlling_interests",
                claim("Minority", "1", ""),
                "`/non_controlling_interests/0/measurement_basis`",
                "for example book value or fair value",
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
        // A missing measurement basis is refused, never defaulted (R29).
        let v = with(base(), |m| {
            m.insert(
                "debt_like_items".into(),
                json!([{"label": "Pension", "value": "1"}]),
            );
        });
        assert_eq!(err(&v).code(), "FM-E001");
    }
}
