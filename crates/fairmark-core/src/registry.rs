//! The method registry: the single source of truth for every adapter.
//!
//! Python, the CLI, the MCP server, function-calling tool definitions and the
//! documentation are all generated from this catalogue and its JSON Schemas.

use schemars::{schema_for, JsonSchema};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::{FairmarkError, Result};
use crate::explain::{Meta, Valuation};
use crate::method::Method;
pub(crate) use crate::reading::{nested_error, nested_error_text};
use crate::{
    analysis, coc, dates, dcf, loans, rates, ratios, series, statements, tv, tvm, ENGINE_VERSION,
};

/// Public description of a method, used to build agent tools.
#[derive(Debug, Clone, Serialize)]
pub struct MethodInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    /// `available` or `preview`; see [`Status`].
    pub status: Status,
}

/// How far a method has been verified (ADR 0012). Defined, with the promotion rule, in
/// `docs/en/project/releases.md`; both statuses follow the versioning rules of ADR 0009.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Meets the promotion rule: approved spec, independent audit and review recorded.
    Available,
    /// Implemented, tested and documented, not yet promoted: tell users so.
    Preview,
}

/// Methods promoted to available (ADR 0012). Every other method is preview, so a new method
/// is preview until it is listed here; `scripts/method_status.py` checks each entry against
/// the evidence in `specs/method-evidence.yaml`.
const AVAILABLE: &[&str] = &[
    "dates.year_fraction",
    "rates.convert",
    "rates.real_nominal",
    "tvm.npv",
    "tvm.present_value",
    "tvm.xnpv",
    "tvm.irr",
    "tvm.xirr",
    "tvm.mirr",
    "tvm.annuity",
    "tvm.perpetuity",
    "tvm.future_value",
    "loans.instalment",
    "loans.number_of_instalments",
    "loans.amortisation_schedule",
    "coc.cost_of_equity",
    "coc.beta_relever",
    "coc.beta_peers",
    "coc.cost_of_debt",
    "coc.wacc",
    "tv.gordon",
    "tv.value_driver",
    "tv.h_model",
    "tv.exit_multiple",
    "tv.limited_horizon",
    "tv.normalised_income",
    "tv.liquidation",
    "tv.compare",
    "analysis.goal_seek",
    "analysis.sensitivity",
    "analysis.scenarios",
    "analysis.tornado",
];

/// Byte-wise string equality usable in constant evaluation.
const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn is_listed(id: &str, list: &[&str]) -> bool {
    let mut i = 0;
    while i < list.len() {
        if same(id, list[i]) {
            return true;
        }
        i += 1;
    }
    false
}

const fn status_of(id: &str) -> Status {
    if is_listed(id, AVAILABLE) {
        Status::Available
    } else {
        Status::Preview
    }
}

/// Builds the catalogue, the schema lookup and the dispatch from one list of [`Method`] types.
/// Adding a method to the engine means implementing [`Method`] in its theme module and adding
/// one line to the list below; nothing else can drift.
macro_rules! registry {
    ($($m:ty),+ $(,)?) => {
        $(const _: () = assert!(
            crate::method::is_valid_version(<$m as Method>::VERSION),
            "method VERSION must be a positive integer such as \"1\""
        );)+

        /// Every method exposed by the engine, in catalogue order.
        pub const METHODS: &[MethodInfo] = &[$(
            MethodInfo {
                id: <$m as Method>::ID,
                title: <$m as Method>::TITLE,
                description: <$m as Method>::DESCRIPTION,
                version: <$m as Method>::VERSION,
                status: status_of(<$m as Method>::ID),
            },
        )+];

        /// Every id promoted in `AVAILABLE` is a registered method (ADR 0012).
        const _: () = {
            let ids: &[&str] = &[$(<$m as Method>::ID),+];
            let mut i = 0;
            while i < AVAILABLE.len() {
                assert!(is_listed(AVAILABLE[i], ids), "AVAILABLE names an unregistered method");
                i += 1;
            }
        };

        /// `(id, Rust type path)` of every method, to check that each lives in its theme module.
        /// `type_name` is best-effort by documentation but stable in practice for plain paths;
        /// it is used only in this test, never at run time.
        #[cfg(test)]
        fn type_paths() -> Vec<(&'static str, &'static str)> {
            vec![$((<$m as Method>::ID, std::any::type_name::<$m>())),+]
        }

        fn schema_of(id: &str) -> Option<schemars::Schema> {
            $(if id == <$m as Method>::ID {
                return Some(schema_for!(<$m as Method>::Input));
            })+
            None
        }

        fn output_schema_of(id: &str) -> Option<schemars::Schema> {
            $(if id == <$m as Method>::ID {
                return Some(serialize_schema::<<$m as Method>::Output>());
            })+
            None
        }

        fn result_schema_of(id: &str) -> Option<schemars::Schema> {
            $(if id == <$m as Method>::ID {
                return Some(serialize_schema::<Valuation<<$m as Method>::Output>>());
            })+
            None
        }

        fn validate_to(id: &str, input_json: &str) -> Option<Result<()>> {
            $(if id == <$m as Method>::ID {
                return Some(validate_only::<$m>(input_json));
            })+
            None
        }

        fn dispatch_to(id: &str, input_json: &str) -> Option<Result<Value>> {
            $(if id == <$m as Method>::ID {
                return Some(execute::<$m>(input_json));
            })+
            None
        }
    };
}

registry! {
    // Grouped by theme, in the order of the documentation: dates, series, rates, time value of
    // money, loans, cost of capital, terminal value, discounted cash flow, financial statements,
    // ratios, analysis.
    dates::YearFraction,
    dates::Schedule,
    series::Project,
    rates::Convert,
    rates::RealNominal,
    tvm::Npv,
    tvm::PresentValue,
    tvm::Xnpv,
    tvm::Irr,
    tvm::Xirr,
    tvm::Mirr,
    tvm::Annuity,
    tvm::Perpetuity,
    tvm::FutureValue,
    loans::Instalment,
    loans::NumberOfInstalments,
    loans::AmortisationSchedule,
    coc::CostOfEquity,
    coc::BetaRelever,
    coc::BetaPeers,
    coc::CostOfDebt,
    coc::Wacc,
    tv::Gordon,
    tv::ValueDriver,
    tv::HModel,
    tv::ExitMultiple,
    tv::LimitedHorizon,
    tv::NormalisedIncome,
    tv::Liquidation,
    tv::Compare,
    dcf::Fcff,
    dcf::Fcfe,
    dcf::FreeCashFlows,
    statements::Reorganise,
    ratios::profitability::Profitability,
    ratios::capital_structure::CapitalStructure,
    ratios::liquidity::Liquidity,
    ratios::working_capital::WorkingCapital,
    ratios::debt_service::DebtService,
    ratios::covenants::Covenants,
    analysis::goal_seek::GoalSeek,
    analysis::sensitivity::Sensitivity,
    analysis::scenarios::Scenarios,
    analysis::tornado::Tornado,
}

/// Evaluates a method on a JSON value from inside the core: the same path as [`run`] (parsing,
/// validation, computation, envelope), without a second panic guard, because it is only
/// called from within a `run` (ADR 0010).
pub(crate) fn evaluate(id: &str, input: &Value) -> Result<Value> {
    dispatch(id, &input.to_string())
}

/// Every input error of method `id` on `input`, without computing anything (ADR 0014 G5): the
/// input is read, its defaults filled in and [`Method::validate`] run. Used for the
/// `base_input` of an analysis, which is checked in the same pass as the analysis's own fields
/// without a hidden evaluation.
pub(crate) fn validate(id: &str, input: &Value) -> Result<()> {
    validate_to(id, &input.to_string())
        .unwrap_or_else(|| Err(FairmarkError::UnknownMethod(id.to_owned())))
}

fn validate_only<M: Method>(input_json: &str) -> Result<()> {
    let mut input: M::Input = crate::reading::parse(input_json, M::ID.starts_with("analysis."))?;
    let defaults = M::apply_defaults(&mut input);
    M::validate(&input, &defaults)
}

#[cfg(test)]
thread_local! {
    /// Computations of the methods other than analyses on this thread, for the tests that
    /// check that a validation computes nothing.
    pub(crate) static COMPUTED: std::cell::Cell<u64> = const { std::cell::Cell::new(0) };
}

pub fn method(id: &str) -> Result<&'static MethodInfo> {
    METHODS
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))
}

/// True when `pointer` names, in the input `input` of method `id`, a field that takes a whole
/// number (an integer of the schema), not a decimal.
pub(crate) fn takes_whole_number(id: &str, input: &Value, pointer: &str) -> bool {
    schema_of(id)
        .and_then(|s| serde_json::to_value(s).ok())
        .is_some_and(|s| crate::reading::takes_whole_number(&s, input, pointer))
}

/// JSON Schema (2020-12) of a method's input.
pub fn input_schema(id: &str) -> Result<Value> {
    let schema = schema_of(id).ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))?;
    serde_json::to_value(schema).map_err(|e| FairmarkError::InvalidInput(e.to_string()))
}

/// JSON Schema (2020-12) of a method's output: the `value` of its result envelope, as the
/// engine serialises it (ADR 0014 G8). Decimals are strings matching `^-?\d+(\.\d+)?$`;
/// a field that may be absent is not `required`; a field that may be `null` says so. A field
/// holding the input or the output of another method carries `x-fairmark-input-of` or
/// `x-fairmark-output-of`: an RFC 6901 pointer to that method's id, relative to the object
/// that holds the field (for `analysis.goal_seek`, `/inner_method/id` in the same object).
/// Clients accept enumeration values they do not know: a value added to an output enumeration
/// is additive (ADR 0014 G8).
pub fn output_schema(id: &str) -> Result<Value> {
    let schema = output_schema_of(id).ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))?;
    serde_json::to_value(schema).map_err(|e| FairmarkError::InvalidInput(e.to_string()))
}

/// JSON Schema (2020-12) of a method's whole result envelope (`method`, `value`, `explain`,
/// `meta`), with `value` described by [`output_schema`]. The envelope follows its own
/// version (ADR 0014 G4), not the method version.
///
/// The schema also describes what the method returns when it fails (ADR 0014 G5):
/// `$defs/FairmarkErrorResponse`, the `{"error": <report>}` document of every adapter, named at
/// the root by `x-fairmark-error` (an RFC 6901 fragment of this schema). A client validates an
/// error against `<this schema>#/$defs/FairmarkErrorResponse`.
pub fn result_schema(id: &str) -> Result<Value> {
    let schema = result_schema_of(id).ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))?;
    let mut result =
        serde_json::to_value(schema).map_err(|e| FairmarkError::InvalidInput(e.to_string()))?;
    let mut error = error_schema();
    let error_defs = error
        .as_object_mut()
        .and_then(|e| e.remove("$defs"))
        .unwrap_or_default();
    if let Some(e) = error.as_object_mut() {
        e.remove("$schema");
    }
    let root = result
        .as_object_mut()
        .ok_or(FairmarkError::Internal("result schema is not an object"))?;
    let defs = root
        .entry("$defs")
        .or_insert_with(|| Value::Object(serde_json::Map::new()))
        .as_object_mut()
        .ok_or(FairmarkError::Internal(
            "result schema $defs is not an object",
        ))?;
    for (name, def) in error_defs.as_object().into_iter().flatten() {
        if defs.insert(name.clone(), def.clone()).is_some() {
            return Err(FairmarkError::Internal("error report schema name collides"));
        }
    }
    if defs.insert(ERROR_RESPONSE.to_owned(), error).is_some() {
        return Err(FairmarkError::Internal("error report schema name collides"));
    }
    root.insert(
        "x-fairmark-error".to_owned(),
        Value::from(format!("#/$defs/{ERROR_RESPONSE}")),
    );
    Ok(result)
}

/// Name, in the `$defs` of a result schema, of the schema of an error document.
const ERROR_RESPONSE: &str = "FairmarkErrorResponse";

/// JSON Schema (2020-12) of what every adapter returns when a method fails (ADR 0014 G5):
/// `{"error": {code, message, pointer, pointers, reason, expected, fix, missing_any_of,
/// errors}}`. The same for every method; also embedded in each [`result_schema`].
pub fn error_schema() -> Value {
    serde_json::to_value(serialize_schema::<crate::error::ErrorResponse>()).unwrap_or_default()
}

/// The schema of a type as it is serialised (an output), not as it is read (an input):
/// decimals are strings, fields skipped when empty are optional.
fn serialize_schema<T: JsonSchema>() -> schemars::Schema {
    schemars::generate::SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<T>()
}

/// Run a method on a JSON input and return the JSON result envelope.
///
/// Numbers in the input may be JSON numbers or strings; both are parsed
/// exactly into decimals (no binary floating point is ever involved).
pub fn run(id: &str, input_json: &str) -> Result<Value> {
    // Last line of defence: all arithmetic is checked, so a panic is a defect. It must never
    // cross an adapter boundary (Python, CLI, MCP): it becomes `FM-E012` instead. This guard
    // has no effect where panics abort (for example WebAssembly targets); checked arithmetic
    // and the extreme-input contract cases (`tests/contracts/`) remain the primary defence there. Panic messages in
    // the core never contain input values.
    std::panic::catch_unwind(|| dispatch(id, input_json)).unwrap_or(Err(FairmarkError::Internal(
        "unexpected panic in the engine",
    )))
}

fn dispatch(id: &str, input_json: &str) -> Result<Value> {
    dispatch_to(id, input_json).unwrap_or_else(|| Err(FairmarkError::UnknownMethod(id.to_owned())))
}

fn execute<M: Method>(input_json: &str) -> Result<Value> {
    let mut input: M::Input = crate::reading::parse(input_json, M::ID.starts_with("analysis."))?;
    #[cfg(test)]
    if !M::ID.starts_with("analysis.") {
        COMPUTED.with(|c| c.set(c.get() + 1));
    }
    // SPEC-0001 R31: the defaults are filled in once, in `run_typed`; the hash covers the input
    // with its defaults, so that a minimal input and its explicit equivalent hash identically.
    let (value, explain, defaults) = M::run_typed(&mut input)?;
    let canonical =
        serde_json::to_string(&input).map_err(|e| FairmarkError::InvalidInput(e.to_string()))?;
    let envelope = Valuation {
        method: M::ID.to_owned(),
        value,
        explain,
        meta: Meta {
            engine_version: ENGINE_VERSION.to_owned(),
            method_version: M::VERSION.to_owned(),
            input_hash: sha256_hex(canonical.as_bytes()),
            arithmetic: "decimal, 96-bit mantissa (28-29 significant digits)".to_owned(),
            method_status: status_of(M::ID),
            evaluated_method_status: M::evaluated_method(&input).map(status_of),
            defaults_applied: defaults,
        },
    };
    serde_json::to_value(envelope).map_err(|e| FairmarkError::InvalidInput(e.to_string()))
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

    /// ADR 0009: ids are unique `<theme>.<name>`, versions are integers, and every method type
    /// is defined in the module of its theme (`fairmark_core::<theme>::<Type>`).
    #[test]
    fn method_ids_are_unique_and_live_in_their_theme_module() {
        let mut seen = std::collections::BTreeSet::new();
        for m in METHODS {
            assert!(seen.insert(m.id), "duplicate method id {}", m.id);
            let (_, name) = m.id.split_once('.').expect("id is <theme>.<name>");
            assert!(
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "{}: name must be snake_case",
                m.id
            );
            assert!(
                m.version.parse::<u32>().is_ok(),
                "{}: version must be an integer",
                m.id
            );
        }
        for (id, path) in type_paths() {
            let theme = id.split('.').next().unwrap();
            assert!(
                path.starts_with(&format!("fairmark_core::{theme}::")),
                "{id} is implemented by {path}, outside the module of its theme"
            );
        }
    }

    /// ADR 0012: methods not promoted in `AVAILABLE` are preview, and the status is part of
    /// the catalogue every adapter serialises.
    #[test]
    fn status_comes_from_the_promotion_list() {
        for m in METHODS {
            let expected = if AVAILABLE.contains(&m.id) {
                Status::Available
            } else {
                Status::Preview
            };
            assert_eq!(m.status, expected, "{}", m.id);
        }
        assert_eq!(method("dcf.fcff").unwrap().status, Status::Preview);
        assert_eq!(method("tvm.npv").unwrap().status, Status::Available);
        assert_eq!(method("tv.gordon").unwrap().status, Status::Available);
        let json = serde_json::to_value(method("dcf.fcff").unwrap()).unwrap();
        assert_eq!(json["status"], "preview");
        let json = serde_json::to_value(method("tv.gordon").unwrap()).unwrap();
        assert_eq!(json["status"], "available");
    }

    /// A two-period dcf.fcff version 3 input, valued with a Gordon terminal value.
    const DCF_V3: &str = r#"{"stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}], "cash_flow_terms": "nominal", "discount_rate_terms": "nominal", "wacc": "0.09", "timing": "end_of_period", "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}, "net_financial_position": {"gross_debt": 0, "cash": 0}}"#;

    /// ADR 0012: every result states the status of the method that produced it.
    #[test]
    fn results_report_the_method_status() {
        let tv = run(
            "tv.gordon",
            r#"{"rate": "0.1", "growth": "0.02", "next_flow": "50"}"#,
        )
        .unwrap();
        assert_eq!(tv["meta"]["method_status"], "available");
        assert!(tv["meta"].get("evaluated_method_status").is_none());
        let npv = run("tvm.npv", r#"{"rate": 0.1, "cash_flows": [-100, 60, 60]}"#).unwrap();
        assert_eq!(npv["meta"]["method_status"], "available");
        // dcf.fcff version 3 input (SPEC-0001).
        let dcf = run("dcf.fcff", DCF_V3).unwrap();
        assert_eq!(dcf["meta"]["method_status"], "preview");
    }

    /// ADR 0012: an analysis reports the status of the method it evaluates, in `meta` and, when
    /// that method is preview, as a convention.
    #[test]
    fn analysis_reports_the_status_of_the_evaluated_method() {
        let table = |method: &str, base: &str, variable: &str, output: &str| {
            run(
                "analysis.sensitivity",
                &format!(
                    r#"{{"method": "{method}", "base_input": {base}, "output": "{output}",
                        "rows": {{"variable": "{variable}", "values": ["0.08", "0.09"]}}}}"#
                ),
            )
            .unwrap()
        };
        let preview = table("dcf.fcff", DCF_V3, "/wacc", "/enterprise_value");
        assert_eq!(preview["meta"]["method_status"], "available");
        assert_eq!(preview["meta"]["evaluated_method_status"], "preview");
        let conventions = preview["explain"]["conventions"].as_array().unwrap();
        assert!(conventions.iter().any(|c| c
            .as_str()
            .unwrap()
            .starts_with("Evaluated method dcf.fcff is preview")));

        let available = table(
            "tv.gordon",
            r#"{"rate": "0.09", "growth": "0.02", "next_flow": "50"}"#,
            "/rate",
            "/terminal_value",
        );
        assert_eq!(available["meta"]["evaluated_method_status"], "available");
        let conventions = available["explain"]["conventions"].as_array().unwrap();
        assert!(!conventions
            .iter()
            .any(|c| c.as_str().unwrap().contains("is preview")));
    }

    #[test]
    fn promotion_list_names_registered_methods_once() {
        let mut seen = std::collections::BTreeSet::new();
        for id in AVAILABLE {
            assert!(seen.insert(id), "{id} is listed twice");
            assert!(method(id).is_ok(), "{id} is not registered");
        }
        assert!(is_listed("tv.gordon", AVAILABLE));
        assert!(!is_listed("tv.gordo", AVAILABLE));
        assert!(!is_listed("tv.gordonx", AVAILABLE));
    }

    #[test]
    fn every_method_has_a_schema() {
        for m in METHODS {
            assert!(input_schema(m.id).is_ok(), "{}", m.id);
        }
    }

    /// ADR 0014 G8 (issue #265): every method publishes the schema of its output and of its
    /// result envelope, each documented at the root.
    #[test]
    fn every_method_has_output_and_result_schemas() {
        for m in METHODS {
            let output = output_schema(m.id).unwrap();
            assert_eq!(output["type"], "object", "{}", m.id);
            assert!(
                output["description"].is_string(),
                "{}: document the output type",
                m.id
            );
            let result = result_schema(m.id).unwrap();
            let required = result["required"].as_array().unwrap();
            for key in ["method", "value", "explain", "meta"] {
                assert!(required.contains(&Value::from(key)), "{}: {key}", m.id);
            }
        }
        assert_eq!(
            output_schema("tvm.nope").unwrap_err().code(),
            FairmarkError::UnknownMethod(String::new()).code()
        );
    }

    /// ADR 0014 G5 (issue #258): the result schema describes the error document too, under a
    /// name no output type uses, and every error report validates against it.
    #[test]
    fn result_schemas_describe_the_error_report() {
        for m in METHODS {
            let result = result_schema(m.id).unwrap();
            assert_eq!(result["x-fairmark-error"], "#/$defs/FairmarkErrorResponse");
            let response = &result["$defs"]["FairmarkErrorResponse"];
            assert_eq!(
                response["required"],
                serde_json::json!(["error"]),
                "{}",
                m.id
            );
            let report = &result["$defs"]["FairmarkErrorReport"];
            for key in [
                "code", "message", "pointer", "pointers", "reason", "expected", "fix",
            ] {
                assert!(
                    report["required"]
                        .as_array()
                        .unwrap()
                        .contains(&Value::from(key)),
                    "{key}"
                );
            }
            assert!(result["$defs"]["FairmarkErrorReason"]["oneOf"].is_array());
            assert_eq!(report["properties"]["errors"]["minItems"], 1, "{}", m.id);
            assert!(report["required"]
                .as_array()
                .unwrap()
                .contains(&Value::from("errors")));
        }
        // The compact report of an analysis cell keeps its name and shape.
        let sensitivity = result_schema("analysis.sensitivity").unwrap();
        let cell = &sensitivity["$defs"]["ErrorReport"]["properties"];
        assert_eq!(
            cell.as_object().unwrap().keys().collect::<Vec<_>>(),
            ["code", "message"]
        );
    }

    /// ADR 0014 G7: decimals are returned as strings, and the output schema says so; a field
    /// skipped when empty is not required.
    #[test]
    fn output_schemas_describe_what_is_serialised() {
        let npv = output_schema("tvm.npv").unwrap();
        assert_eq!(npv["properties"]["npv"]["type"], "string");
        assert_eq!(npv["properties"]["npv"]["pattern"], r"^-?\d+(\.\d+)?$");
        let result = result_schema("tv.gordon").unwrap();
        let meta = &result["$defs"]["Meta"];
        let required = meta["required"].as_array().unwrap();
        assert!(required.contains(&Value::from("method_version")));
        assert!(required.contains(&Value::from("method_status")));
        assert!(!required.contains(&Value::from("defaults_applied")));
        let seek = output_schema("analysis.goal_seek").unwrap();
        assert_eq!(
            seek["properties"]["inner_value"]["x-fairmark-output-of"],
            "/inner_method/id"
        );
        assert_eq!(
            seek["properties"]["solved_input"]["x-fairmark-input-of"],
            "/inner_method/id"
        );
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

    /// The input conditions that only the computation can find, as `errors.md` lists them: the
    /// reconciliation of the statements, the rounding of a schedule, an instalment that never
    /// repays the loan and the count it solves over the limit, a generated date after
    /// 2199-12-31, and the minimum input of the ratios.
    const FOUND_BY_COMPUTATION: &[(&str, &str)] = &[
        ("statements.reorganise", "FM-E026"),
        ("loans.amortisation_schedule", "FM-E023"),
        ("loans.number_of_instalments", "FM-E021"),
        ("loans.number_of_instalments", "FM-E001"),
        ("dates.schedule", "FM-E009"),
        ("ratios.liquidity", "FM-E027"),
        ("ratios.working_capital", "FM-E027"),
        ("ratios.capital_structure", "FM-E027"),
        ("ratios.profitability", "FM-E027"),
        ("ratios.debt_service", "FM-E027"),
        ("ratios.covenants", "FM-E027"),
    ];

    /// The leaves of `value` that hold a number or a decimal string, as JSON pointers.
    fn numeric_leaves(value: &Value, at: String, out: &mut Vec<String>) {
        match value {
            Value::Object(map) => {
                for (k, v) in map {
                    numeric_leaves(
                        v,
                        format!("{at}/{}", k.replace('~', "~0").replace('/', "~1")),
                        out,
                    );
                }
            }
            Value::Array(items) => {
                for (i, v) in items.iter().enumerate() {
                    numeric_leaves(v, format!("{at}/{i}"), out);
                }
            }
            Value::Number(_) => out.push(at),
            Value::String(s) if crate::reading::is_decimal_text(s) => out.push(at),
            _ => {}
        }
    }

    /// The messages of every error in `e`, in order.
    fn messages(e: FairmarkError) -> Vec<(String, String)> {
        e.into_errors()
            .into_iter()
            .map(|e| (e.code().to_owned(), e.to_string()))
            .collect()
    }

    /// The validate-only path agrees with the computation (ADR 0014 G5): on every golden input,
    /// and on each with one number changed to a value often out of range, `validate` refuses
    /// exactly the inputs whose run reports input errors, with the same errors in the same
    /// order; an input it accepts fails, if at all, only with a computation error.
    #[test]
    fn validation_agrees_with_the_computation_on_every_golden_input() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
        let mut checked = 0u32;
        let mut failures: Vec<String> = Vec::new();
        for dir in std::fs::read_dir(root).unwrap() {
            let dir = dir.unwrap().path();
            let mut files: Vec<_> = std::fs::read_dir(&dir)
                .unwrap()
                .map(|f| f.unwrap().path())
                .collect();
            files.sort();
            for (position, file) in files.into_iter().enumerate() {
                let case: Value =
                    serde_norway::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
                let (Some(id), Some(input)) = (case["method"].as_str(), case.get("input")) else {
                    continue;
                };
                let (id, input) = if id.starts_with("analysis.") {
                    match (input["method"].as_str(), input.get("base_input")) {
                        (Some(inner), Some(base)) if method(inner).is_ok() => (inner, base),
                        _ => continue,
                    }
                } else {
                    (id, input)
                };
                if id.starts_with("analysis.") || method(id).is_err() {
                    continue;
                }
                let mut leaves = Vec::new();
                numeric_leaves(input, String::new(), &mut leaves);
                let mut variants = vec![input.clone()];
                // Every case as given; the first cases of each method also changed, one
                // number at a time, which keeps the test within seconds without optimisation.
                let sampled = if position < 12 {
                    leaves.as_slice()
                } else {
                    &[]
                };
                for leaf in sampled.iter().step_by(sampled.len().div_ceil(6).max(1)) {
                    for value in ["-2", "0", "2"] {
                        let mut changed = input.clone();
                        *changed.pointer_mut(leaf).unwrap() = Value::String(value.to_owned());
                        variants.push(changed);
                    }
                }
                for variant in variants {
                    let validated = validate(id, &variant);
                    let ran = run(id, &variant.to_string());
                    let at = format!("{} {id} {variant}", file.display());
                    match (validated, ran) {
                        (Ok(()), Ok(_)) => {}
                        (Ok(()), Err(e)) => {
                            let codes: Vec<String> = e
                                .into_errors()
                                .iter()
                                .filter(|e| e.is_input_error())
                                .filter(|e| !FOUND_BY_COMPUTATION.contains(&(id, e.code())))
                                .map(|e| e.code().to_owned())
                                .collect();
                            if !codes.is_empty() {
                                failures.push(format!("accepted {id} {codes:?}: {at}"));
                            }
                        }
                        (Err(v), Err(r)) => {
                            if messages(v.clone()) != messages(r.clone()) {
                                failures.push(format!(
                                    "differ {id}: {:?} != {:?}: {at}",
                                    messages(v),
                                    messages(r)
                                ));
                            }
                        }
                        (Err(v), Ok(_)) => failures.push(format!("refused {id} {v}: {at}")),
                    }
                    checked += 1;
                }
            }
        }
        assert!(checked > 1500, "{checked}");
        assert!(
            failures.is_empty(),
            "{}\n{}",
            failures.len(),
            failures.join("\n")
        );
    }
}
