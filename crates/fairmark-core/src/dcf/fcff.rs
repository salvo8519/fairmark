//! `dcf.fcff` version 3: enterprise DCF, free cash flows to the firm discounted at the WACC
//! over explicit stages and a terminal value, then value of operations, enterprise value and
//! equity value (SPEC-0001 R1, R6 to R15, R19 to R23, R25).

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::engine::{
    self, EngineInput, PeriodRow, RateFields, RateInput, Stage, Terminal, TerminalDiscountTime,
    TerminalOutput, Terms,
};
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
    /// `nominal` or `real`: the terms of every amount and growth rate of the input.
    pub cash_flow_terms: Terms,
    /// `nominal` or `real`: the terms of the discount rates. They must equal
    /// `cash_flow_terms`.
    pub discount_rate_terms: Terms,
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
    /// The terminal technique, an object with exactly one key, the method id:
    /// `{"tv.gordon": {"growth", "first_flow", ...}}`, on the enterprise basis.
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
    /// Enterprise value minus the net financial position.
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
    /// Enterprise value minus the net financial position.
    pub equity_value: Decimal,
    /// Present value of the explicit FCFF, in currency units.
    pub pv_explicit_period: Decimal,
    /// Present value of the terminal value divided by the value of operations; `null` when
    /// the value of operations is zero.
    pub terminal_value_weight: Option<Decimal>,
    /// One row per period of the explicit horizon.
    pub periods: Vec<PeriodRow>,
    /// The terminal value, its technique and its discounting.
    pub terminal: TerminalOutput,
    /// Every line from value of operations to equity value.
    pub bridge: FcffBridge,
}

const FORMULA: &str = "Value of operations = Σ FCFF_k × DF_k + TV_N × DF_TV; Enterprise value = \
    value of operations + Σ non-operating assets; Equity value = enterprise value − net \
    financial position";

pub fn fcff(input: &DcfFcffInput) -> Result<(DcfFcffOutput, Explain)> {
    let mut ex = Explain::new("dcf.fcff", FORMULA);
    ex.convention("basis: enterprise, free cash flows to the firm (FCFF) discounted at the WACC");
    let engine_input = EngineInput {
        basis: TerminalBasis::Enterprise,
        stages: &input.stages,
        base_flow: input.base_flow,
        cash_flow_terms: input.cash_flow_terms,
        discount_rate_terms: input.discount_rate_terms,
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
        if a.value.is_sign_negative() && !a.value.is_zero() {
            return Err(FairmarkError::InvalidInput(format!(
                "`/non_operating_assets/{i}/value` is {}; a non-operating asset is zero or \
                 more (obligations belong in the net financial position)",
                a.value
            )));
        }
        if a.label.trim().is_empty() {
            return Err(FairmarkError::InvalidInput(format!(
                "`/non_operating_assets/{i}/label` is empty; name the asset"
            )));
        }
    }
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
            terminal: out.terminal,
            bridge,
        },
        ex,
    ))
}

/// R23, R25: value of operations + non-operating assets = enterprise value; enterprise value −
/// net financial position = equity value. Every line echoed in `explain`.
fn bridge(
    ex: &mut Explain,
    input: &DcfFcffInput,
    value_of_operations: Decimal,
) -> Result<FcffBridge> {
    ex.convention("bridge items measured at the valuation date, as given");
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
    let equity_value = sub(enterprise_value, nfp.unwrap_or_default())?;
    if equity_value.is_sign_negative() && !equity_value.is_zero() {
        // R26: reported, never floored at zero and never refused.
        ex.convention(format!(
            "warning: the equity value is negative ({equity_value}): the claims subtracted \
             exceed the enterprise value; it is reported as computed, not floored at zero"
        ));
    }
    ex.step(
        "Equity value",
        format!("{enterprise_value} − {}", nfp.unwrap_or_default()),
        equity_value,
    );
    Ok(FcffBridge {
        value_of_operations,
        non_operating_assets: input.non_operating_assets.clone(),
        non_operating_assets_total: total,
        enterprise_value,
        net_financial_position: nfp,
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
        of growth) and a terminal value. Returns every period and step for audit.";
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
        for (input, code, pointer) in cases {
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
        assert!(c.iter().any(|c| c.contains("r_3 = 0.10 (R21)")), "{c:?}");
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
}
