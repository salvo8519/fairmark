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
use crate::{dcf, tvm, ENGINE_VERSION};

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
        version: "1",
    },
    MethodInfo {
        id: "tvm.npv",
        title: "Net present value",
        description: "Net present value of a cash flow series; the first flow is at t = 0.",
        version: "1",
    },
    MethodInfo {
        id: "tvm.irr",
        title: "Internal rate of return",
        description:
            "Rate at which the NPV of a cash flow series is zero, solved deterministically.",
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
        other => return Err(FairmarkError::UnknownMethod(other.to_owned())),
    };
    serde_json::to_value(schema).map_err(|e| FairmarkError::InvalidInput(e.to_string()))
}

/// Run a method on a JSON input and return the JSON result envelope.
///
/// Numbers in the input may be JSON numbers or strings; both are parsed
/// exactly into decimals (no binary floating point is ever involved).
pub fn run(id: &str, input_json: &str) -> Result<Value> {
    let info = method(id)?;
    match info.id {
        "dcf.fcff" => execute(info, input_json, dcf::fcff),
        "tvm.npv" => execute(info, input_json, tvm::npv),
        "tvm.irr" => execute(info, input_json, tvm::irr),
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
    let input: I =
        serde_json::from_str(input_json).map_err(|e| FairmarkError::InvalidInput(e.to_string()))?;
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
    fn unknown_fields_are_rejected() {
        let err = run("tvm.npv", r#"{"rate": 0.1, "cash_flows": [1], "typo": 1}"#).unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }
}
