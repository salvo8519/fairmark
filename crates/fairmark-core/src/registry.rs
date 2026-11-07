//! The method registry: the single source of truth for every adapter.
//!
//! Python, the CLI, the MCP server, function-calling tool definitions and the
//! documentation are all generated from this catalogue and its JSON Schemas.

use schemars::{schema_for, JsonSchema};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::{FairmarkError, Result};
use crate::explain::{Explain, Meta, Valuation};
use crate::{coc, dates, dcf, rates, tvm, ENGINE_VERSION};

/// Public description of a method, used to build agent tools.
#[derive(Debug, Clone, Serialize)]
pub struct MethodInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub version: &'static str,
}

/// Every method exposed by the engine.
pub const METHODS: &[MethodInfo] = &[
    MethodInfo {
        id: "dcf.fcff",
        title: "DCF — unlevered free cash flow",
        description:
            "Enterprise and equity value from explicit FCFF forecasts discounted at WACC, \
                      with a Gordon growth terminal value. Returns every step for audit.",
        version: "2",
    },
    MethodInfo {
        id: "tvm.npv",
        title: "Net present value",
        description: "Net present value of a cash flow series; the first flow is at t = 0.",
        version: "2",
    },
    MethodInfo {
        id: "tvm.irr",
        title: "Internal rate of return",
        description: "Rate at which the NPV of a periodic cash flow series is zero, solved \
                      deterministically; refuses inputs with more than one IRR.",
        version: "2",
    },
    MethodInfo {
        id: "dates.year_fraction",
        title: "Year fraction (day count)",
        description: "Fraction of a year between two dates under an ISDA 2006 day-count \
                      convention: Actual/365 Fixed, Actual/360, Actual/Actual ISDA, 30/360, 30E/360.",
        version: "1",
    },
    MethodInfo {
        id: "rates.convert",
        title: "Rate conversion between compounding conventions",
        description: "Equivalent annual rate under another compounding (annual, semi-annual, \
                      quarterly, monthly, continuous).",
        version: "1",
    },
    MethodInfo {
        id: "rates.real_nominal",
        title: "Real and nominal rates (Fisher)",
        description: "Converts a nominal rate to a real rate, or the reverse, with the exact \
                      Fisher relation.",
        version: "1",
    },
    MethodInfo {
        id: "tvm.present_value",
        title: "Present value with a constant or time-varying rate",
        description: "Present value of periodic flows for periods 1..n with one rate or one rate \
                      per period, at end, mid or beginning of period. Returns every discount factor.",
        version: "1",
    },
    MethodInfo {
        id: "tvm.xnpv",
        title: "Net present value of dated cash flows (XNPV)",
        description: "Present value of dated flows at a valuation date, with explicit day count \
                      and compounding (defaults as in ECMA-376 XNPV).",
        version: "1",
    },
    MethodInfo {
        id: "tvm.xirr",
        title: "Internal rate of return of dated cash flows (XIRR)",
        description: "Annual rate at which the XNPV of dated flows is zero, solved \
                      deterministically; refuses inputs with more than one rate.",
        version: "1",
    },
    MethodInfo {
        id: "tvm.mirr",
        title: "Modified internal rate of return (MIRR)",
        description: "Rate of return with negative flows financed at a finance rate and positive \
                      flows reinvested at a reinvestment rate (ECMA-376 MIRR).",
        version: "1",
    },
    MethodInfo {
        id: "tvm.annuity",
        title: "Annuity (level or growing, ordinary or due)",
        description: "Present and future value of a finite series of level or growing payments.",
        version: "1",
    },
    MethodInfo {
        id: "tvm.perpetuity",
        title: "Perpetuity (level or growing)",
        description: "Present value of a level or growing perpetuity (Gordon), ordinary or due.",
        version: "1",
    },
    MethodInfo {
        id: "coc.cost_of_equity",
        title: "Cost of equity (CAPM with explicit premia)",
        description: "Cost of equity from risk-free rate, levered or relevered beta and equity \
                      risk premium, plus optional country, size and specific premia, each reported.",
        version: "1",
    },
    MethodInfo {
        id: "coc.beta_relever",
        title: "Beta unlevering and relevering",
        description: "Converts an unlevered beta to a levered beta at a target D/E, or the \
                      reverse, with the Hamada or Harris-Pringle formula and an optional debt beta.",
        version: "1",
    },
    MethodInfo {
        id: "coc.beta_peers",
        title: "Beta from comparable companies",
        description: "Unlevers the betas of comparable companies, takes the mean or median and \
                      relevers it to the target capital structure.",
        version: "1",
    },
    MethodInfo {
        id: "coc.cost_of_debt",
        title: "Cost of debt",
        description: "Pre-tax cost of debt, given or as risk-free rate plus credit spread, and \
                      the after-tax cost of debt.",
        version: "1",
    },
    MethodInfo {
        id: "coc.wacc",
        title: "Weighted average cost of capital (WACC)",
        description: "WACC from cost of equity, cost of debt, tax rate and target weights \
                      (D/E or D/(D+E)).",
        version: "1",
    },
];

pub fn method(id: &str) -> Result<&'static MethodInfo> {
    METHODS
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))
}

/// JSON Schema (2020-12) of a method's input.
pub fn input_schema(id: &str) -> Result<Value> {
    let schema = match method(id)?.id {
        "dcf.fcff" => schema_for!(dcf::DcfFcffInput),
        "tvm.npv" => schema_for!(tvm::NpvInput),
        "tvm.irr" => schema_for!(tvm::IrrInput),
        "dates.year_fraction" => schema_for!(dates::YearFractionInput),
        "rates.convert" => schema_for!(rates::ConvertInput),
        "rates.real_nominal" => schema_for!(rates::RealNominalInput),
        "tvm.present_value" => schema_for!(tvm::PresentValueInput),
        "tvm.xnpv" => schema_for!(tvm::XnpvInput),
        "tvm.xirr" => schema_for!(tvm::XirrInput),
        "tvm.mirr" => schema_for!(tvm::MirrInput),
        "tvm.annuity" => schema_for!(tvm::AnnuityInput),
        "tvm.perpetuity" => schema_for!(tvm::PerpetuityInput),
        "coc.cost_of_equity" => schema_for!(coc::CostOfEquityInput),
        "coc.beta_relever" => schema_for!(coc::BetaReleverInput),
        "coc.beta_peers" => schema_for!(coc::BetaPeersInput),
        "coc.cost_of_debt" => schema_for!(coc::CostOfDebtInput),
        "coc.wacc" => schema_for!(coc::WaccInput),
        other => return Err(FairmarkError::UnknownMethod(other.to_owned())),
    };
    serde_json::to_value(schema).map_err(|e| FairmarkError::InvalidInput(e.to_string()))
}

/// Run a method on a JSON input and return the JSON result envelope.
///
/// Numbers in the input may be JSON numbers or strings; both are parsed
/// exactly into decimals (no binary floating point is ever involved).
pub fn run(id: &str, input_json: &str) -> Result<Value> {
    // Last line of defence: all arithmetic is checked, so a panic is a defect. It must never
    // cross an adapter boundary (Python, CLI, MCP): it becomes `FM-E012` instead. This guard
    // has no effect where panics abort (for example WebAssembly targets); checked arithmetic
    // and the extreme-input test below remain the primary defence there. Panic messages in
    // the core never contain input values.
    std::panic::catch_unwind(|| dispatch(id, input_json)).unwrap_or(Err(FairmarkError::Internal(
        "unexpected panic in the engine",
    )))
}

fn dispatch(id: &str, input_json: &str) -> Result<Value> {
    let info = method(id)?;
    match info.id {
        "dcf.fcff" => execute(info, input_json, dcf::fcff),
        "tvm.npv" => execute(info, input_json, tvm::npv),
        "tvm.irr" => execute(info, input_json, tvm::irr),
        "dates.year_fraction" => execute(info, input_json, dates::year_fraction),
        "rates.convert" => execute(info, input_json, rates::convert),
        "rates.real_nominal" => execute(info, input_json, rates::real_nominal),
        "tvm.present_value" => execute(info, input_json, tvm::present_value),
        "tvm.xnpv" => execute(info, input_json, tvm::xnpv),
        "tvm.xirr" => execute(info, input_json, tvm::xirr),
        "tvm.mirr" => execute(info, input_json, tvm::mirr),
        "tvm.annuity" => execute(info, input_json, tvm::annuity),
        "tvm.perpetuity" => execute(info, input_json, tvm::perpetuity),
        "coc.cost_of_equity" => execute(info, input_json, coc::cost_of_equity),
        "coc.beta_relever" => execute(info, input_json, coc::beta_relever),
        "coc.beta_peers" => execute(info, input_json, coc::beta_peers),
        "coc.cost_of_debt" => execute(info, input_json, coc::cost_of_debt),
        "coc.wacc" => execute(info, input_json, coc::wacc),
        other => Err(FairmarkError::UnknownMethod(other.to_owned())),
    }
}

fn execute<I, O>(
    info: &MethodInfo,
    input_json: &str,
    f: fn(&I) -> Result<(O, Explain)>,
) -> Result<Value>
where
    I: DeserializeOwned + Serialize + JsonSchema,
    O: Serialize,
{
    let input: I = serde_json::from_str(input_json).map_err(input_error)?;
    let canonical =
        serde_json::to_string(&input).map_err(|e| FairmarkError::InvalidInput(e.to_string()))?;
    let (value, explain) = f(&input)?;
    let envelope = Valuation {
        method: info.id.to_owned(),
        value,
        explain,
        meta: Meta {
            engine_version: ENGINE_VERSION.to_owned(),
            method_version: info.version.to_owned(),
            input_hash: sha256_hex(canonical.as_bytes()),
            arithmetic: "decimal, 96-bit mantissa (28-29 significant digits)".to_owned(),
        },
    };
    serde_json::to_value(envelope).map_err(|e| FairmarkError::InvalidInput(e.to_string()))
}

/// Deserialisation failures are `FM-E001`, except invalid dates, which are `FM-E009`.
fn input_error(e: serde_json::Error) -> FairmarkError {
    let msg = e.to_string();
    // serde_json puts the text of a custom error first; other errors start differently
    // ("unknown field", "missing field", "invalid type"), so a prefix match is exact.
    match msg.strip_prefix(dates::INVALID_DATE) {
        Some(rest) => FairmarkError::InvalidDate(rest.trim_start_matches(": ").to_owned()),
        None => FairmarkError::InvalidInput(msg),
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_method_has_a_schema() {
        for m in METHODS {
            assert!(input_schema(m.id).is_ok(), "{}", m.id);
        }
    }

    #[test]
    fn same_input_same_hash_and_value() {
        let input = r#"{"rate": 0.1, "cash_flows": [-100, 60, 60]}"#;
        assert_eq!(
            run("tvm.npv", input).unwrap(),
            run("tvm.npv", input).unwrap()
        );
    }

    #[test]
    fn json_numbers_are_parsed_exactly() {
        let out = run("tvm.npv", r#"{"rate": 0, "cash_flows": [0.1, 0.2]}"#).unwrap();
        assert_eq!(out["value"]["npv"], "0.3");
    }

    #[test]
    fn extreme_inputs_return_errors_not_panics() {
        let max = "79228162514264337593543950335";
        let min = "-79228162514264337593543950335";
        let cases: Vec<(&str, String)> = vec![
            (
                "tvm.npv",
                format!(r#"{{"rate": "{max}", "cash_flows": ["{max}", "{max}"]}}"#),
            ),
            (
                "tvm.irr",
                format!(r#"{{"cash_flows": ["{min}", "{max}", "{max}"]}}"#),
            ),
            (
                "tvm.mirr",
                format!(
                    r#"{{"cash_flows": ["{min}", "{max}"], "finance_rate": "{max}", "reinvest_rate": "{max}"}}"#
                ),
            ),
            (
                "tvm.present_value",
                format!(r#"{{"cash_flows": ["{max}"], "rate": "{max}", "timing": "mid_period"}}"#),
            ),
            (
                "tvm.annuity",
                format!(
                    r#"{{"payment": "{max}", "rate": "{max}", "growth": "{min}", "periods": 3, "timing": "due"}}"#
                ),
            ),
            (
                "tvm.annuity",
                format!(
                    r#"{{"payment": "{max}", "rate": "0.05", "growth": "0.05", "periods": 3, "timing": "due"}}"#
                ),
            ),
            (
                "tvm.perpetuity",
                format!(
                    r#"{{"payment": "{max}", "rate": "{max}", "growth": "{min}", "timing": "due"}}"#
                ),
            ),
            (
                "rates.convert",
                format!(r#"{{"rate": "{max}", "from": "monthly", "to": "continuous"}}"#),
            ),
            (
                "rates.real_nominal",
                format!(r#"{{"rate": "{max}", "inflation": "{max}", "direction": "to_nominal"}}"#),
            ),
            (
                "tvm.xnpv",
                format!(
                    r#"{{"rate": "{max}", "compounding": "semi_annual", "cash_flows": [{{"date": "2020-01-01", "amount": "{max}"}}, {{"date": "2199-12-31", "amount": "{max}"}}]}}"#
                ),
            ),
            (
                "tvm.xirr",
                format!(
                    r#"{{"cash_flows": [{{"date": "2020-01-01", "amount": "{min}"}}, {{"date": "2020-01-01", "amount": "{min}"}}, {{"date": "2021-01-01", "amount": "{max}"}}]}}"#
                ),
            ),
            (
                "dcf.fcff",
                format!(
                    r#"{{"free_cash_flows": ["{max}", "{max}"], "discount_rate": "{max}", "terminal_growth": "0.01"}}"#
                ),
            ),
        ];
        let cases = cases.into_iter().chain([
            ("coc.cost_of_equity", format!(r#"{{"risk_free_rate": "{max}", "equity_risk_premium": "{max}", "levered_beta": "{max}", "country_risk_premium": "{max}", "size_premium": "{max}"}}"#)),
            ("coc.cost_of_equity", format!(r#"{{"risk_free_rate": "0.03", "equity_risk_premium": "0.05", "relever": {{"unlevered_beta": "{max}", "debt_to_equity": "{max}", "tax_rate": "0.5", "formula": "hamada", "debt_beta": "{min}"}}}}"#)),
            ("coc.beta_relever", format!(r#"{{"beta": "{max}", "direction": "unlever", "debt_to_equity": "{max}", "tax_rate": "0", "formula": "harris_pringle", "debt_beta": "{max}"}}"#)),
            ("coc.beta_peers", format!(r#"{{"formula": "hamada", "statistic": "mean", "target_debt_to_equity": "{max}", "target_tax_rate": "0", "peers": [{{"name": "a", "levered_beta": "{max}", "debt_to_equity": "0", "tax_rate": "0"}}, {{"name": "b", "levered_beta": "{max}", "debt_to_equity": "0", "tax_rate": "0"}}]}}"#)),
            ("coc.cost_of_debt", format!(r#"{{"risk_free_rate": "{max}", "credit_spread": "{max}", "tax_rate": "0"}}"#)),
            ("coc.wacc", format!(r#"{{"cost_of_equity": "{max}", "pre_tax_cost_of_debt": "{max}", "tax_rate": "0", "debt_to_equity": "{max}"}}"#)),
        ]);
        let mut covered = std::collections::BTreeSet::new();
        for (id, input) in cases {
            covered.insert(id);
            let out = std::panic::catch_unwind(|| dispatch(id, &input));
            assert!(out.is_ok(), "{id} panicked on {input}");
        }
        let missing: Vec<_> = METHODS
            .iter()
            .map(|m| m.id)
            .filter(|id| !covered.contains(id) && *id != "dates.year_fraction")
            .collect();
        assert!(missing.is_empty(), "no extreme-input case for {missing:?}");
    }

    /// AC11: each method reports the conventions that change its numbers, including defaults.
    #[test]
    fn conventions_name_every_choice_and_default() {
        let flows = r#"[{"date": "2024-01-01", "amount": "-100"}, {"date": "2025-01-01", "amount": "110"}]"#;
        let cases: Vec<(&str, String, Vec<&str>)> = vec![
            ("dates.year_fraction", r#"{"start": "2024-01-01", "end": "2024-07-01", "day_count": "act_360"}"#.into(), vec!["Actual/360"]),
            ("rates.convert", r#"{"rate": "0.1", "from": "annual", "to": "monthly"}"#.into(), vec!["from: annual", "to: monthly"]),
            ("rates.real_nominal", r#"{"rate": "0.05", "inflation": "0.02", "direction": "to_real"}"#.into(), vec!["exact Fisher"]),
            ("tvm.present_value", r#"{"cash_flows": ["1"], "rate": "0.1", "timing": "mid_period"}"#.into(), vec!["mid period", "one rate for every period"]),
            ("tvm.xnpv", format!(r#"{{"rate": "0.1", "cash_flows": {flows}}}"#), vec!["Actual/365 (Fixed)", "(default, ECMA-376)", "annual compounding", "earliest flow date (default"]),
            ("tvm.xnpv", format!(r#"{{"rate": "0.1", "day_count": "act_360", "compounding": "monthly", "valuation_date": "2023-12-01", "cash_flows": {flows}}}"#), vec!["Actual/360", "(given)", "monthly compounding"]),
            ("tvm.xirr", format!(r#"{{"cash_flows": {flows}}}"#), vec!["(default, ECMA-376)", "Newton starting point: 0.10 (default)", "uniqueness proven"]),
            ("tvm.irr", r#"{"cash_flows": ["-100", "110"], "guess": "0.2"}"#.into(), vec!["cash_flows[0] occurs at t = 0", "Newton starting point: 0.2 (given)", "uniqueness proven"]),
            ("tvm.mirr", r#"{"cash_flows": ["-100", "60", "60"], "finance_rate": "0.1", "reinvest_rate": "0.12"}"#.into(), vec!["cash_flows[0] occurs at t = 0", "finance rate", "reinvestment rate"]),
            ("tvm.annuity", r#"{"payment": "1", "rate": "0.05", "periods": 3, "timing": "due"}"#.into(), vec!["annuity due", "grows by 0 per period (default"]),
            ("tvm.perpetuity", r#"{"payment": "1", "rate": "0.05", "growth": "0.01", "timing": "ordinary"}"#.into(), vec!["end of the first period", "grows by 0.01 per period (given)"]),
            ("coc.cost_of_equity", r#"{"risk_free_rate": "0.03", "equity_risk_premium": "0.05", "levered_beta": "1"}"#.into(), vec!["annual effective", "levered beta 1 (given)", "size premium 0 (default)", "country exposure (lambda) 1 (default)"]),
            ("coc.beta_relever", r#"{"beta": "1", "direction": "relever", "debt_to_equity": "0.5", "tax_rate": "0.2", "formula": "hamada"}"#.into(), vec!["Hamada", "debt beta 0 (default"]),
            ("coc.beta_peers", r#"{"formula": "harris_pringle", "statistic": "median", "target_debt_to_equity": "0.5", "target_tax_rate": "0.2", "peers": [{"name": "a", "levered_beta": "1", "debt_to_equity": "0.1", "tax_rate": "0.2"}]}"#.into(), vec!["Harris-Pringle", "median", "does not use the tax rate", "one debt beta applies"]),
            ("coc.cost_of_debt", r#"{"pre_tax_cost_of_debt": "0.05", "tax_rate": "0.24"}"#.into(), vec!["pre-tax cost of debt given", "tax rate of the tax shield 0.24"]),
            ("coc.wacc", r#"{"cost_of_equity": "0.1", "pre_tax_cost_of_debt": "0.05", "tax_rate": "0.24", "debt_weight": "0.3"}"#.into(), vec!["annual effective", "target weights from D/(D + E) 0.3", "tax rate of the tax shield 0.24"]),
        ];
        for (id, input, expected) in cases {
            let out = run(id, &input).unwrap_or_else(|e| panic!("{id}: {e}"));
            let conventions = out["explain"]["conventions"].to_string();
            for e in expected {
                assert!(
                    conventions.contains(e),
                    "{id}: missing '{e}' in {conventions}"
                );
            }
        }
    }

    #[test]
    fn a_field_named_like_the_date_marker_is_not_a_date_error() {
        let err = run(
            "tvm.npv",
            r#"{"rate": 0.1, "cash_flows": [1], "invalid date": 1}"#,
        )
        .unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let err = run("tvm.npv", r#"{"rate": 0.1, "cash_flows": [1], "typo": 1}"#).unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }
}
