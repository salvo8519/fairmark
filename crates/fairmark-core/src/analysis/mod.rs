//! Analysis methods: generic tools that evaluate another method repeatedly (SPEC-0008).
//!
//! The inner method is chosen at run time and evaluated through the registry on JSON values,
//! one level deep, with a bounded number of evaluations (ADR 0010).

pub mod goal_seek;
pub mod pointer;
pub mod scenarios;
pub mod sensitivity;
pub mod tornado;

use std::borrow::Cow;

use rust_decimal::Decimal;
use schemars::{JsonSchema, Schema, SchemaGenerator};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};

use crate::error::{CompactErrorReport, FairmarkError, Reason, Result};
use crate::registry;

/// Most inner evaluations one analysis request may perform (ADR 0010).
pub const EVALUATION_LIMIT: u64 = 10_000;

/// Most work one analysis request may perform: evaluations × values in the base input. Each
/// evaluation parses, validates and computes the whole inner input, so the cost grows with
/// both; this bound keeps the cost of a request predictable whatever the input size.
pub const WORK_LIMIT: u64 = 1_000_000;

/// Size of a JSON document: every value counts one, containers included, so that empty
/// arrays and objects are not free.
pub(crate) fn size_of(value: &Value) -> u64 {
    match value {
        Value::Array(items) => 1 + items.iter().map(size_of).sum::<u64>(),
        Value::Object(map) => 1 + map.values().map(size_of).sum::<u64>(),
        _ => 1,
    }
}

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
    status: registry::Status,
    evaluations: u64,
    /// Evaluations reserved by `reserve` before the first one; exceeding it is a defect.
    reserved: u64,
}

impl Inner {
    /// `FM-E007` for an unknown id, `FM-E017` for an analysis method.
    pub(crate) fn new(id: &str) -> Result<Self> {
        let info = registry::method(id).map_err(|e| e.at("/method", Reason::Enum))?;
        if info.id.starts_with(THEME) {
            return Err(FairmarkError::NestedAnalysis(info.id.to_owned())
                .at("/method", Reason::Conflict)
                .fix("run the inner analysis directly; analyses do not nest"));
        }
        Ok(Self {
            id: info.id,
            version: info.version,
            status: info.status,
            evaluations: 0,
            reserved: 0,
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

    /// The convention an analysis reports when its inner method is preview (ADR 0012).
    pub(crate) fn status_convention(&self) -> Option<String> {
        (self.status == registry::Status::Preview).then(|| {
            format!(
                "Evaluated method {} is preview: its results have not been independently \
                verified (docs/en/project/releases.md, Method status)",
                self.id
            )
        })
    }

    /// Fails with `FM-E016` before any evaluation when the request needs more than
    /// `EVALUATION_LIMIT` evaluations, or more than `WORK_LIMIT` evaluations × input values.
    pub(crate) fn reserve(&mut self, needed: u64, base: &Value) -> Result<()> {
        if needed > EVALUATION_LIMIT {
            return Err(FairmarkError::EvaluationLimit(format!(
                "the request needs {needed} evaluations, more than {EVALUATION_LIMIT}"
            )));
        }
        let size = size_of(base).max(1);
        let work = needed.saturating_mul(size);
        if work > WORK_LIMIT {
            return Err(FairmarkError::EvaluationLimit(format!(
                "{needed} evaluations of an input of {size} values need {work} units of work, \
                more than {WORK_LIMIT}; use fewer evaluations or a shorter input"
            )));
        }
        self.reserved = needed;
        Ok(())
    }

    /// Runs the inner method; returns the `value` of its result.
    pub(crate) fn evaluate(&mut self, input: &Value) -> Result<Value> {
        self.evaluations += 1;
        if self.evaluations > self.reserved {
            return Err(FairmarkError::Internal(
                "an analysis exceeded its reserved evaluations",
            ));
        }
        // The inner input is `base_input` (or a variant of it): its errors are located under
        // `/base_input`, from the root of the analysis input (ADR 0014 G5).
        let mut envelope =
            registry::evaluate(self.id, input).map_err(|e| e.under("/base_input"))?;
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

/// `base` with each `(pointer, value)` written as a decimal string (R4).
pub(crate) fn with_values(base: &Value, values: &[(&str, Decimal)]) -> Result<Value> {
    let mut input = base.clone();
    for (pointer, value) in values {
        pointer::write_decimal(&mut input, pointer, *value)?;
    }
    Ok(input)
}

/// R9: the base case, `base_input` unchanged, and the selected outputs of its result. Any error
/// is returned unchanged; an output pointer that does not resolve is `FM-E015`.
///
/// `fields` gives the JSON pointer, in the analysis input, of each output pointer: `/output`,
/// or `/outputs/k`.
pub(crate) fn base_case(
    inner: &mut Inner,
    base: &Value,
    outputs: &[&str],
    fields: impl Fn(usize) -> String,
) -> Result<Vec<Decimal>> {
    let value = inner.evaluate(base)?;
    outputs
        .iter()
        .enumerate()
        .map(|(k, o)| output(&value, o).map_err(|e| e.located(fields(k))))
        .collect()
}

/// Outcome of one evaluation in a table or a scenario: a value, or the inner error (R16).
#[derive(Debug, Clone, Serialize, JsonSchema)]
#[serde(untagged)]
pub enum Outcome<T: Serialize> {
    /// The evaluation succeeded.
    Value {
        /// The output read from the inner result, in its units.
        value: T,
    },
    /// The inner method returned this error; the rest of the request went on.
    Error {
        /// Code and message of the inner method's error.
        error: CompactErrorReport,
    },
}

/// R16: an error of the inner method is recorded and the request goes on, except `FM-E012`
/// (a defect in Fairmark), which aborts the request.
pub(crate) fn recorded<T: Serialize>(result: Result<T>) -> Result<Outcome<T>> {
    match result {
        Ok(value) => Ok(Outcome::Value { value }),
        Err(e) if is_internal(&e) => Err(e),
        Err(e) => Ok(Outcome::Error {
            error: e.to_compact_report(),
        }),
    }
}

/// A defect in Fairmark (`FM-E012`), by its code, so that a defect carried with details still
/// aborts the request.
fn is_internal(e: &FairmarkError) -> bool {
    matches!(e.kind(), FairmarkError::Internal(_))
}

/// `FM-E001` when `items` repeat, naming the first repeated one, located at `at(k)` for the
/// k-th item.
pub(crate) fn unique<'a>(
    what: &str,
    items: impl IntoIterator<Item = &'a str>,
    at: impl Fn(usize) -> String,
) -> Result<()> {
    let mut seen = std::collections::BTreeSet::new();
    for (k, item) in items.into_iter().enumerate() {
        if !seen.insert(item) {
            return Err(
                FairmarkError::InvalidInput(format!("{what} '{item}' appears twice"))
                    .at(at(k), Reason::Duplicate),
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_defect_aborts_and_other_errors_are_recorded() {
        let defect: Result<Decimal> = Err(FairmarkError::Internal("injected"));
        assert_eq!(recorded(defect).unwrap_err().code(), "FM-E012");
        let domain: Result<Decimal> = Err(FairmarkError::GrowthNotBelowRate {
            growth: Decimal::ONE,
            rate: Decimal::ZERO,
        });
        let cell = serde_json::to_value(recorded(domain).unwrap()).unwrap();
        assert_eq!(cell["error"]["code"], "FM-E004");
        let ok = serde_json::to_value(recorded(Ok(Decimal::TWO)).unwrap()).unwrap();
        assert_eq!(ok, serde_json::json!({"value": "2"}));
    }
}
