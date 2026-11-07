//! The method registry: the single source of truth for every adapter.
//!
//! Python, the CLI, the MCP server, function-calling tool definitions and the
//! documentation are all generated from this catalogue and its JSON Schemas.

use schemars::schema_for;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::{FairmarkError, Result};
use crate::explain::{Meta, Valuation};
use crate::method::Method;
use crate::{analysis, coc, dates, dcf, rates, tvm, ENGINE_VERSION};

/// Public description of a method, used to build agent tools.
#[derive(Debug, Clone, Serialize)]
pub struct MethodInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub version: &'static str,
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
            },
        )+];

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

        fn dispatch_to(id: &str, input_json: &str) -> Option<Result<Value>> {
            $(if id == <$m as Method>::ID {
                return Some(execute::<$m>(input_json));
            })+
            None
        }
    };
}

registry! {
    // Grouped by theme, in the order of the documentation: dates, rates, time value of money,
    // cost of capital, discounted cash flow, analysis.
    dates::YearFraction,
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
    coc::CostOfEquity,
    coc::BetaRelever,
    coc::BetaPeers,
    coc::CostOfDebt,
    coc::Wacc,
    dcf::Fcff,
    analysis::goal_seek::GoalSeek,
}

/// Evaluates a method on a JSON value from inside the core: the same path as [`run`] (parsing,
/// validation, computation, envelope), without a second panic guard, because it is only
/// called from within a `run` (ADR 0010).
pub(crate) fn evaluate(id: &str, input: &Value) -> Result<Value> {
    dispatch(id, &input.to_string())
}

pub fn method(id: &str) -> Result<&'static MethodInfo> {
    METHODS
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))
}

/// JSON Schema (2020-12) of a method's input.
pub fn input_schema(id: &str) -> Result<Value> {
    let schema = schema_of(id).ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))?;
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
    let input: M::Input = serde_json::from_str(input_json).map_err(input_error)?;
    let canonical =
        serde_json::to_string(&input).map_err(|e| FairmarkError::InvalidInput(e.to_string()))?;
    let (value, explain) = M::compute(&input)?;
    let envelope = Valuation {
        method: M::ID.to_owned(),
        value,
        explain,
        meta: Meta {
            engine_version: ENGINE_VERSION.to_owned(),
            method_version: M::VERSION.to_owned(),
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
