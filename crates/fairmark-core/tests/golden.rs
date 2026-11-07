//! Runs every case in `tests/golden/**.yaml` against the engine.
//! The same files are executed by the Python test-suite.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;

#[derive(Deserialize)]
struct Case {
    method: String,
    description: String,
    input: serde_json::Value,
    expect: std::collections::BTreeMap<String, String>,
    tolerance: String,
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
        let out = fairmark_core::run(&case.method, &case.input.to_string())
            .unwrap_or_else(|e| panic!("{}: {e}", file.display()));
        let tol = Decimal::from_str(&case.tolerance).unwrap();
        for (field, expected) in &case.expect {
            let got = Decimal::from_str(out["value"][field].as_str().unwrap()).unwrap();
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
