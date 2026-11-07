//! Contract cases per method, one file each in `tests/contracts/<method>.yaml` (ADR 0009).
//!
//! Every method must have a file with at least one extreme input (which must return a value or
//! a typed error, never reach the panic guard) and at least one conventions case (whose expected
//! texts must appear in `explain.conventions`). Data lives in files so that adding a method never
//! touches a shared test, and parallel work does not conflict.

use std::path::{Path, PathBuf};

use serde::Deserialize;
use serde_json::Value;

const MAX: &str = "79228162514264337593543950335";
const MIN: &str = "-79228162514264337593543950335";

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Contract {
    method: String,
    extreme_inputs: Vec<Value>,
    conventions: Vec<ConventionCase>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConventionCase {
    input: Value,
    expect: Vec<String>,
}

fn substitute(v: &Value) -> Value {
    match v {
        Value::String(s) if s == "MAX" => Value::String(MAX.to_owned()),
        Value::String(s) if s == "MIN" => Value::String(MIN.to_owned()),
        Value::Array(a) => Value::Array(a.iter().map(substitute).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, v)| (k.clone(), substitute(v))).collect())
        }
        other => other.clone(),
    }
}

fn contracts() -> Vec<(PathBuf, Contract)> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/contracts");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&dir)
        .expect("tests/contracts")
        .map(|e| e.unwrap().path())
        .filter(|p| p.extension().is_some_and(|e| e == "yaml"))
        .collect();
    files.sort();
    files
        .into_iter()
        .map(|p| {
            let c: Contract = serde_norway::from_str(&std::fs::read_to_string(&p).unwrap())
                .unwrap_or_else(|e| panic!("{}: {e}", p.display()));
            (p, c)
        })
        .collect()
}

#[test]
fn every_method_has_a_contract_file_and_every_file_a_method() {
    let found = contracts();
    for m in fairmark_core::METHODS {
        let file = found.iter().find(|(_, c)| c.method == m.id);
        let (path, c) = file.unwrap_or_else(|| panic!("missing tests/contracts/{}.yaml", m.id));
        assert_eq!(
            path.file_stem().unwrap(),
            m.id,
            "file name must be the method id"
        );
        assert!(!c.extreme_inputs.is_empty(), "{}: no extreme input", m.id);
        assert!(!c.conventions.is_empty(), "{}: no conventions case", m.id);
    }
    for (path, c) in &found {
        assert!(
            fairmark_core::METHODS.iter().any(|m| m.id == c.method),
            "{}: unknown method {}",
            path.display(),
            c.method
        );
    }
}

#[test]
fn extreme_inputs_never_reach_the_panic_guard() {
    for (path, c) in contracts() {
        for input in &c.extreme_inputs {
            let json = substitute(input).to_string();
            if let Err(e) = fairmark_core::run(&c.method, &json) {
                assert_ne!(e.code(), "FM-E012", "{}: panic on {json}", path.display());
            }
        }
    }
}

#[test]
fn conventions_name_every_choice_and_default() {
    for (path, c) in contracts() {
        for case in &c.conventions {
            let out = fairmark_core::run(&c.method, &case.input.to_string())
                .unwrap_or_else(|e| panic!("{}: {e}", path.display()));
            let conventions = out["explain"]["conventions"].to_string();
            for e in &case.expect {
                assert!(
                    conventions.contains(e.as_str()),
                    "{}: missing '{e}' in {conventions}",
                    path.display()
                );
            }
        }
    }
}
