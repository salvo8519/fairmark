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
    /// Expected fields of `meta` (keys are JSON Pointers into `meta`, for example
    /// `/defaults_applied/0/field`), compared like `expect`; optional.
    #[serde(default)]
    expect_meta: std::collections::BTreeMap<String, String>,
    /// Expected lengths of arrays of `value` (keys are JSON Pointers, values counts), so that a
    /// list such as `/not_computed` has no entry beyond those asserted; optional.
    #[serde(default)]
    expect_length: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    expect_error: Option<String>,
    /// Expected fields of the error report, with `expect_error` only (keys are JSON Pointers
    /// into the report, for example `/pointer` or `/errors/1/reason`; values compared exactly,
    /// `null` for a null field); optional, for cases added with issue #258.
    #[serde(default)]
    expect_error_report: std::collections::BTreeMap<String, String>,
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
                Err(e) if e.code() == code => {
                    let report = serde_json::to_value(e.to_report()).unwrap();
                    for (field, expected) in &case.expect_error_report {
                        let got = report.pointer(field);
                        let matches = match got {
                            Some(v) if expected == "null" => v.is_null(),
                            Some(v) => v.as_str() == Some(expected.as_str()),
                            None => false,
                        };
                        if !matches {
                            failures.push(format!(
                                "{} [{}] error report {field}: got {got:?}, want {expected}",
                                file.display(),
                                case.description
                            ));
                        }
                    }
                }
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
        for (field, count) in &case.expect_length {
            let got = out["value"]
                .pointer(field)
                .and_then(|v| v.as_array())
                .map(Vec::len);
            if got.map(|n| n.to_string()).as_deref() != Some(count.as_str()) {
                failures.push(format!(
                    "{} [{}] {field}: length {got:?}, want {count}",
                    file.display(),
                    case.description
                ));
            }
        }
        let tolerance = case.tolerance.as_deref().expect("tolerance");
        let expectations = case
            .expect
            .iter()
            .map(|(f, e)| ("value", f, e))
            .chain(case.expect_meta.iter().map(|(f, e)| ("meta", f, e)));
        for (part, field, expected) in expectations {
            // A key starting with '/' is an RFC 6901 pointer into `value` (or `meta` for
            // `expect_meta`); any other key is a top-level field.
            let found = if field.starts_with('/') {
                out[part].pointer(field)
            } else {
                out[part].get(field.as_str())
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
            if !within(&got.to_string(), expected, tolerance) {
                failures.push(format!(
                    "{} [{}] {field}: got {got}, want {expected}",
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

/// Exact decimal comparison `|got - want| <= tol` on the decimal strings themselves.
///
/// `Decimal::from_str` rounds a string with more than 28 significant digits, so the expected
/// value is never parsed into a `Decimal`: it is compared digit by digit, as Python's `decimal`
/// does in the Python runner (SPEC: issue 148). Inputs are plain decimals (`-12.345`), no
/// exponent.
fn within(got: &str, want: &str, tol: &str) -> bool {
    // (negative, digits of the integer value, scale): value = (-1)^neg * digits / 10^scale.
    fn parse(s: &str) -> (bool, Vec<u8>, usize) {
        let s = s.trim();
        let (neg, s) = match s.strip_prefix('-') {
            Some(rest) => (true, rest),
            None => (false, s.strip_prefix('+').unwrap_or(s)),
        };
        assert!(
            !s.contains(['e', 'E']),
            "golden values are written without exponent: {s}"
        );
        let (int, frac) = s.split_once('.').unwrap_or((s, ""));
        let digits: Vec<u8> = int.bytes().chain(frac.bytes()).map(|b| b - b'0').collect();
        assert!(digits.iter().all(|d| *d < 10), "not a decimal: {s}");
        (neg, digits, frac.len())
    }
    fn scaled(mut d: Vec<u8>, from: usize, to: usize) -> Vec<u8> {
        d.extend(std::iter::repeat_n(0, to - from));
        let first = d.iter().position(|x| *x != 0).unwrap_or(d.len());
        d.split_off(first)
    }
    fn cmp(a: &[u8], b: &[u8]) -> std::cmp::Ordering {
        a.len().cmp(&b.len()).then_with(|| a.cmp(b))
    }
    fn add(a: &[u8], b: &[u8]) -> Vec<u8> {
        let (mut out, mut carry) = (Vec::new(), 0u8);
        let (mut i, mut j) = (a.len(), b.len());
        while i > 0 || j > 0 || carry > 0 {
            let x = if i > 0 {
                i -= 1;
                a[i]
            } else {
                0
            };
            let y = if j > 0 {
                j -= 1;
                b[j]
            } else {
                0
            };
            let t = x + y + carry;
            out.push(t % 10);
            carry = t / 10;
        }
        out.reverse();
        scaled(out, 0, 0)
    }
    // a - b with a >= b.
    fn sub(a: &[u8], b: &[u8]) -> Vec<u8> {
        let (mut out, mut borrow) = (Vec::new(), 0i8);
        let mut j = b.len();
        for i in (0..a.len()).rev() {
            let y = if j > 0 {
                j -= 1;
                b[j] as i8
            } else {
                0
            };
            let mut t = a[i] as i8 - y - borrow;
            borrow = i8::from(t < 0);
            if t < 0 {
                t += 10;
            }
            out.push(t as u8);
        }
        out.reverse();
        scaled(out, 0, 0)
    }
    let (gn, g, gs) = parse(got);
    let (wn, w, ws) = parse(want);
    let (tn, t, ts) = parse(tol);
    assert!(!tn, "tolerance must not be negative: {tol}");
    let scale = gs.max(ws).max(ts);
    let (g, w, t) = (
        scaled(g, gs, scale),
        scaled(w, ws, scale),
        scaled(t, ts, scale),
    );
    let diff = if gn == wn {
        if cmp(&g, &w).is_ge() {
            sub(&g, &w)
        } else {
            sub(&w, &g)
        }
    } else {
        add(&g, &w)
    };
    cmp(&diff, &t).is_le()
}

#[test]
fn exact_comparison_does_not_round_the_expected_value() {
    // 32 significant digits: Decimal::from_str would round the expected value to 28.
    let want = "9999999999.9999999999999985398788";
    assert!(within(
        "9999999999.999999999999998540",
        want,
        "0.000000000000000001"
    ));
    assert!(!within(
        "9999999999.999999999999998540",
        want,
        "0.0000000000000000001"
    ));
    assert!(within("-1.5", "-1.25", "0.25"));
    assert!(!within("-1.5", "1.25", "2.7"));
    assert!(within("1.5", "-1.25", "2.75"));
    assert!(within("0", "0.000", "0"));
    assert!(within("100", "100.0", "0"));
}
