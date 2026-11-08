//! The contract every valuation method fulfils (ADR 0009).
//!
//! A method is a unit type in the module of its theme (`tvm`, `coc`, `dcf`, ...) that declares,
//! in one place, its identity (id, version, title, description), its input and output types and
//! its computation. The registry is built from the list of these types, so metadata, JSON
//! Schema and dispatch can never drift apart.

use schemars::JsonSchema;
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::error::Result;
use crate::explain::{DefaultApplied, Explain};

/// A deterministic valuation method.
pub trait Method {
    /// Stable identifier `<theme>.<name>`, for example `tvm.npv`. Never reused.
    const ID: &'static str;
    /// Short human title.
    const TITLE: &'static str;
    /// One or two sentences for catalogues and agent tool descriptions.
    const DESCRIPTION: &'static str;
    /// Method version: a positive integer written as a string (checked at compile time by the
    /// registry). Bumped on any numerical change or breaking schema change; each version keeps
    /// its schema snapshot in `schemas/<id>/v<version>.json`.
    const VERSION: &'static str;

    /// Input: `deny_unknown_fields`, units on every field. `Serialize` is required because the
    /// registry re-serialises the parsed input to compute `meta.input_hash` on a canonical form
    /// (defaults applied, field order fixed), so equivalent requests hash identically.
    type Input: DeserializeOwned + Serialize + JsonSchema;
    /// Output: the `value` of the result envelope.
    type Output: Serialize;

    /// The calculation, with its audit trail.
    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)>;

    /// Fills in the documented defaults of the fields not given, in place, and returns one
    /// entry per default, in input order (SPEC-0001 R31). The registry calls it once, before
    /// `meta.input_hash` and [`Method::compute`]: the hash covers the input with its defaults,
    /// the defaults are reported first in `explain.conventions` and listed in
    /// `meta.defaults_applied`. A field given explicitly is never changed. Methods without
    /// defaults keep this implementation, which changes nothing.
    fn apply_defaults(_input: &mut Self::Input) -> Vec<DefaultApplied> {
        Vec::new()
    }

    /// [`Method::compute`] on an input whose defaults were filled in by
    /// [`Method::apply_defaults`], with the list of those defaults, for a method whose messages
    /// say when a value came from a default (SPEC-0001 Decision 24). The registry calls this
    /// one; the default implementation ignores the list.
    fn compute_with_defaults(
        input: &Self::Input,
        _defaults: &[DefaultApplied],
    ) -> Result<(Self::Output, Explain)> {
        Self::compute(input)
    }

    /// Id of the method this one evaluates through the registry (analysis methods, ADR 0010),
    /// so that the result reports its status in `meta.evaluated_method_status` (ADR 0012).
    /// `None` for every method that evaluates no other method.
    fn evaluated_method(_input: &Self::Input) -> Option<&str> {
        None
    }
}

/// True when `v` is a positive integer without leading zeros ("1", "12"; not "", "0", "01",
/// "1.0"). Used in a constant assertion by the registry, so an invalid version does not compile.
pub const fn is_valid_version(v: &str) -> bool {
    let b = v.as_bytes();
    if b.is_empty() || b[0] == b'0' {
        return false;
    }
    let mut i = 0;
    while i < b.len() {
        if !b[i].is_ascii_digit() {
            return false;
        }
        i += 1;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::is_valid_version;

    #[test]
    fn versions_are_positive_integers() {
        for ok in ["1", "2", "10", "123"] {
            assert!(is_valid_version(ok), "{ok}");
        }
        for bad in ["", "0", "01", "1.0", "v1", "-1", " 1"] {
            assert!(!is_valid_version(bad), "{bad:?}");
        }
    }
}
