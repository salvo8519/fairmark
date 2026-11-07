//! Runs every case in `tests/golden/**.yaml` against the engine.
//! The same files are executed by the Python test-suite.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Case {
    method: String,
    description: String,
    /// Independent source of the expected values (required for new cases).
    #[serde(default)]
    source: Option<String>,
    input: serde_json::Value,
    /// Expected value fields (a key starting with `/` is a JSON Pointer into `value`), or
    /// `expect_error` with a stable error code.
    #[serde(default)]
    expect: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    expect_error: Option<String>,
    #[serde(default)]
    tolerance: Option<String>,
}

fn golden_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("golden dir") {
        let path = entry.unwrap().path();
        if path.is_dir() {
            golden_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "yaml") {
            out.push(path);
        }
    }
}

#[test]
fn golden_cases() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut files = Vec::new();
    golden_files(&root, &mut files);
    files.sort();
    assert!(!files.is_empty(), "no golden cases found");

    let mut failures = Vec::new();
    for file in &files {
        let case: Case = serde_norway::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        if case.source.as_deref().is_some_and(|s| s.trim().is_empty()) {
            failures.push(format!("{}: empty source", file.display()));
        }
        let result = fairmark_core::run(&case.method, &case.input.to_string());
        if let Some(code) = &case.expect_error {
            match result {
                Err(e) if e.code() == code => {}
                Err(e) => failures.push(format!(
                    "{}: got {} ({e}), want {code}",
                    file.display(),
                    e.code()
                )),
                Ok(_) => failures.push(format!("{}: succeeded, want error {code}", file.display())),
            }
            continue;
        }
        let out = match result {
            Ok(out) => out,
            Err(e) => {
                failures.push(format!("{}: {} {e}", file.display(), e.code()));
                continue;
            }
        };
        if out["explain"]["conventions"]
            .as_array()
            .is_none_or(|c| c.is_empty())
        {
            failures.push(format!("{}: no conventions reported", file.display()));
        }
        let tol = Decimal::from_str(case.tolerance.as_deref().expect("tolerance")).unwrap();
        for (field, expected) in &case.expect {
            // A key starting with '/' is an RFC 6901 pointer into `value` (tables, nested
            // results); any other key is a top-level field.
            let found = if field.starts_with('/') {
                out["value"].pointer(field)
            } else {
                out["value"].get(field.as_str())
            };
            // A missing field always fails, even when null is expected: a typo in a key must
            // never pass.
            let Some(raw) = found else {
                failures.push(format!(
                    "{} [{}] {field}: not in the result",
                    file.display(),
                    case.description
                ));
                continue;
            };
            // Expectations that are not decimals are compared exactly: `null` (an undefined
            // value, such as a relative delta on a zero base), error codes, names.
            let Ok(want) = Decimal::from_str(expected) else {
                let matches = if expected == "null" {
                    raw.is_null()
                } else {
                    raw.as_str() == Some(expected.as_str())
                };
                if !matches {
                    failures.push(format!(
                        "{} [{}] {field}: got {raw}, want {expected}",
                        file.display(),
                        case.description
                    ));
                }
                continue;
            };
            let got = raw
                .as_str()
                .and_then(|v| Decimal::from_str(v).ok())
                .or_else(|| raw.as_u64().map(Decimal::from))
                .or_else(|| raw.as_i64().map(Decimal::from));
            let Some(got) = got else {
                failures.push(format!(
                    "{} [{}] {field}: got {raw}, want the number {want}",
                    file.display(),
                    case.description
                ));
                continue;
            };
            if (got - want).abs() > tol {
                failures.push(format!(
                    "{} [{}] {field}: got {got}, want {want}",
                    file.display(),
                    case.description
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "golden failures:\n{}",
        failures.join("\n")
    );
}

/// SPEC-0008 AC14 and AC20 for every analysis golden case: two runs are byte-identical, the
/// order of the keys of `base_input` changes neither the value nor `meta.input_hash`, and the
/// result names the inner method at the version of the catalogue.
#[test]
fn analysis_results_are_deterministic_and_name_their_inner_method() {
    fn reversed(v: &serde_json::Value) -> serde_json::Value {
        match v {
            serde_json::Value::Object(map) => serde_json::Value::Object(
                map.iter()
                    .rev()
                    .map(|(k, v)| (k.clone(), reversed(v)))
                    .collect(),
            ),
            serde_json::Value::Array(items) => {
                serde_json::Value::Array(items.iter().map(reversed).collect())
            }
            other => other.clone(),
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut files = Vec::new();
    golden_files(&root, &mut files);
    let mut checked = 0;
    // The folder name, not the path text: separators differ on Windows.
    let in_analysis = |f: &&PathBuf| {
        f.parent()
            .and_then(|d| d.file_name())
            .is_some_and(|d| d.to_string_lossy().starts_with("analysis."))
    };
    for file in files.iter().filter(in_analysis) {
        let case: Case = serde_norway::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        if case.expect_error.is_some() {
            continue;
        }
        let input = case.input.to_string();
        let first = fairmark_core::run(&case.method, &input).unwrap();
        let second = fairmark_core::run(&case.method, &input).unwrap();
        assert_eq!(first.to_string(), second.to_string(), "{}", file.display());
        let mut permuted = case.input.clone();
        permuted["base_input"] = reversed(&case.input["base_input"]);
        let third = fairmark_core::run(&case.method, &permuted.to_string()).unwrap();
        assert_eq!(
            first["value"].to_string(),
            third["value"].to_string(),
            "{}",
            file.display()
        );
        assert_eq!(
            first["meta"]["input_hash"],
            third["meta"]["input_hash"],
            "{}",
            file.display()
        );
        let inner = fairmark_core::method(case.input["method"].as_str().unwrap()).unwrap();
        assert_eq!(
            first["value"]["inner_method"]["id"],
            inner.id,
            "{}",
            file.display()
        );
        assert_eq!(
            first["value"]["inner_method"]["version"],
            inner.version,
            "{}",
            file.display()
        );
        checked += 1;
    }
    assert!(checked >= 8, "only {checked} analysis golden cases checked");
}
