//! Outputs a method could not compute, with the inputs they lack (ADR 0014, G2).
//!
//! One shape for every method that computes from partial input: `statements.reorganise`
//! (SPEC-0005 R6) and the `ratios.*` methods (SPEC-0007 R7). The envelope task (#259) moves the
//! lists into `diagnostics.not_computed`.

use schemars::JsonSchema;
use serde::Serialize;

/// A missing treatment and the destinations it permits (SPEC-0005 R8).
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct Permitted {
    /// JSON pointer of the treatment to give.
    pub treatment: String,
    /// Its permitted values.
    pub destinations: Vec<&'static str>,
}

/// An output not computed, with the pointers of its missing inputs.
#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NotComputedEntry {
    /// JSON pointer of the output in the result value.
    pub output: String,
    /// JSON pointers of the inputs it needs and lacks, sorted.
    pub missing: Vec<String>,
    /// For a missing treatment, its permitted destinations; omitted otherwise.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub permitted: Vec<Permitted>,
    /// Why the output is not computed, one sentence; omitted when the method gives none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}
