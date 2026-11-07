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
    /// Expected value fields, or `expect_error` with a stable error code.
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
            let raw = &out["value"][field];
            let got = raw
                .as_str()
                .map(|v| Decimal::from_str(v).unwrap())
                .or_else(|| raw.as_u64().map(Decimal::from))
                .or_else(|| raw.as_i64().map(Decimal::from))
                .unwrap_or_else(|| {
                    panic!("{}: field {field} missing or not a number", file.display())
                });
            let want = Decimal::from_str(expected).unwrap();
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
