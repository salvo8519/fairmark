//! Analysis methods: generic tools that evaluate another method repeatedly (SPEC-0008).
//!
//! The inner method is chosen at run time and evaluated through the registry on JSON values,
//! one level deep, with a bounded number of evaluations (ADR 0010).

pub mod goal_seek;
pub mod pointer;

use std::borrow::Cow;

use rust_decimal::Decimal;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::error::{FairmarkError, Result};
use crate::registry;

/// Most inner evaluations one analysis request may perform (ADR 0010).
pub const EVALUATION_LIMIT: u64 = 10_000;

/// Theme prefix of analysis method ids.
const THEME: &str = "analysis.";

/// Input of the inner method, validated by that method at run time.
///
/// Serialised with keys sorted recursively, so that `meta.input_hash` does not depend on the
/// order in which a caller wrote the keys.
#[derive(Debug, Clone, Deserialize)]
pub struct BaseInput(pub Map<String, Value>);

/// `value` with object keys sorted recursively: the canonical form of inputs in results.
pub(crate) fn sorted(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys: Vec<&String> = map.keys().collect();
            keys.sort();
            Value::Object(
                keys.into_iter()
                    .map(|k| (k.clone(), sorted(&map[k])))
                    .collect(),
            )
        }
        Value::Array(items) => Value::Array(items.iter().map(sorted).collect()),
        other => other.clone(),
    }
}

impl Serialize for BaseInput {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        sorted(&Value::Object(self.0.clone())).serialize(serializer)
    }
}

impl JsonSchema for BaseInput {
    fn schema_name() -> Cow<'static, str> {
        "BaseInput".into()
    }

    fn json_schema(_: &mut SchemaGenerator) -> Schema {
        schemars::json_schema!({
            "type": "object",
            "description": "Input of the inner method, exactly as that method accepts it. It is \
                validated by the inner method at run time against its own schema; this schema \
                does not describe it."
        })
    }
}

/// The method an analysis evaluated, at the version actually run.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct InnerMethod {
    /// Method id, for example `dcf.fcff`.
    pub id: String,
    /// Method version run by this engine.
    pub version: String,
}

/// Evaluates one inner method and counts the evaluations against the limit.
pub(crate) struct Inner {
    id: &'static str,
    version: &'static str,
    evaluations: u64,
}

impl Inner {
    /// `FM-E007` for an unknown id, `FM-E017` for an analysis method.
    pub(crate) fn new(id: &str) -> Result<Self> {
        let info = registry::method(id)?;
        if info.id.starts_with(THEME) {
            return Err(FairmarkError::NestedAnalysis(info.id.to_owned()));
        }
        Ok(Self {
            id: info.id,
            version: info.version,
            evaluations: 0,
        })
    }

    pub(crate) fn info(&self) -> InnerMethod {
        InnerMethod {
            id: self.id.to_owned(),
            version: self.version.to_owned(),
        }
    }

    pub(crate) fn id(&self) -> &str {
        self.id
    }

    /// Fails with `FM-E016` before any evaluation when `needed` exceeds the limit.
    pub(crate) fn reserve(needed: u64) -> Result<()> {
        if needed > EVALUATION_LIMIT {
            return Err(FairmarkError::EvaluationLimit {
                needed,
                limit: EVALUATION_LIMIT,
            });
        }
        Ok(())
    }

    /// Runs the inner method; returns the `value` of its result.
    pub(crate) fn evaluate(&mut self, input: &Value) -> Result<Value> {
        self.evaluations += 1;
        Self::reserve(self.evaluations)?;
        let mut envelope = registry::evaluate(self.id, input)?;
        Ok(envelope["value"].take())
    }

    pub(crate) fn evaluations(&self) -> u64 {
        self.evaluations
    }
}

/// `base_input` as a JSON value.
pub(crate) fn base_value(base: &BaseInput) -> Value {
    Value::Object(base.0.clone())
}

/// The decimal at `pointer` in an inner result `value`.
pub(crate) fn output(value: &Value, pointer: &str) -> Result<Decimal> {
    pointer::read_decimal(value, pointer)
}
