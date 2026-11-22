//! Runs every case in `tests/golden/**.yaml` against the engine.
//! The same files are executed by the Python test-suite.

use std::path::{Path, PathBuf};
use std::str::FromStr;

use rust_decimal::Decimal;
use serde::Deserialize;

// The test profile (issue #368): the sweeps below check a sample per method under mutation
// testing. Shared with the unit tests of the library.
#[allow(dead_code)]
#[path = "../src/test_profile.rs"]
mod test_profile;

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
    /// `expect_error` with a stable error code. The envelope lists are asserted by the
    /// semantic sections below (SPEC-0013); `expect_meta`, `expect_length`,
    /// `expect_diagnostics` and `expect_diagnostics_length` are retired and refused.
    #[serde(default)]
    expect: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    expect_error: Option<String>,
    /// Expected top-level fields of the error report, with `expect_error` only (keys are JSON
    /// Pointers into the report, for example `/pointer` or `/reason`; an entry of `errors` is
    /// asserted in `expect_errors`, SPEC-0013 R5; values compared exactly,
    /// `null` for a null field); optional, for cases added with issue #258.
    #[serde(default)]
    expect_error_report: std::collections::BTreeMap<String, String>,
    #[serde(default)]
    tolerance: Option<String>,
    /// SPEC-0013 R3: expected counts, keyed by a list name (`errors` in an error case;
    /// `not_computed`, `warnings`, `checks` in a value case) or by a pointer into `value`.
    #[serde(default)]
    expect_counts: std::collections::BTreeMap<String, String>,
    /// SPEC-0013 R1: entries of `meta.defaults_applied`, keyed by `pointer`.
    #[serde(default)]
    expect_defaults: Entries,
    /// SPEC-0013 R1: entries of `diagnostics.not_computed`, keyed by `pointer`.
    #[serde(default)]
    expect_not_computed: Entries,
    /// SPEC-0013 R1: entries of `diagnostics.warnings`, keyed by `code` and `pointer`.
    #[serde(default)]
    expect_warnings: Entries,
    /// SPEC-0013 R1: entries of `diagnostics.checks`, keyed by `pointer` and `id`.
    #[serde(default)]
    expect_checks: Entries,
    /// SPEC-0013 R1: entries of the `errors` of the error report, keyed by `code` and `pointer`.
    #[serde(default)]
    expect_errors: Entries,
}

/// A semantic section: the key of an entry, then pointers relative to the entry and their
/// expected values (SPEC-0013 R1).
type Entries = std::collections::BTreeMap<String, std::collections::BTreeMap<String, String>>;

/// SPEC-0013 R1: each semantic section, the part of the result and the list it reads, and the
/// fields whose values, joined by one space (`-` for null), name an entry. When an envelope
/// version moves a list, this table changes and the golden files do not.
const SECTIONS: [(&str, &str, &str, &[&str]); 5] = [
    ("expect_defaults", "meta", "defaults_applied", &["pointer"]),
    (
        "expect_not_computed",
        "diagnostics",
        "not_computed",
        &["pointer"],
    ),
    (
        "expect_warnings",
        "diagnostics",
        "warnings",
        &["code", "pointer"],
    ),
    ("expect_checks", "diagnostics", "checks", &["pointer", "id"]),
    ("expect_errors", "error", "errors", &["code", "pointer"]),
];

impl Case {
    fn section(&self, name: &str) -> &Entries {
        match name {
            "expect_defaults" => &self.expect_defaults,
            "expect_not_computed" => &self.expect_not_computed,
            "expect_warnings" => &self.expect_warnings,
            "expect_checks" => &self.expect_checks,
            "expect_errors" => &self.expect_errors,
            other => unreachable!("no semantic section named {other}"),
        }
    }
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
                    failures.extend(check_error(&file.display().to_string(), &case, &report));
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
        failures.extend(check_result(&file.display().to_string(), &case, &out));
    }
    assert!(
        failures.is_empty(),
        "golden failures:\n{}",
        failures.join("\n")
    );
}

/// The failures of a successful result `out` against the expectations of `case`; `file` names
/// the case in the messages.
fn check_result(file: &str, case: &Case, out: &serde_json::Value) -> Vec<String> {
    let mut failures = Vec::new();
    if out["explain"]["conventions"]
        .as_array()
        .is_none_or(|c| c.is_empty())
    {
        failures.push(format!("{file}: no conventions reported"));
    }
    let tolerance = case.tolerance.as_deref().expect("tolerance");
    for (field, expected) in &case.expect {
        if let Some(problem) = compare_at(&out["value"], field, expected, Some(tolerance)) {
            failures.push(format!(
                "{} [{}] {field}: {problem}",
                file, case.description
            ));
        }
    }
    let prefix = format!("{} [{}]", file, case.description);
    failures.extend(check_sections(&prefix, case, out, Some(tolerance)));
    for (name, count) in &case.expect_counts {
        let list = if name.starts_with('/') {
            out["value"].pointer(name)
        } else if ["not_computed", "warnings", "checks"].contains(&name.as_str()) {
            out["diagnostics"].get(name)
        } else {
            None
        };
        failures.extend(count_named(&prefix, name, list, count));
    }
    failures
}

/// The failures of an error report `report` against the expectations of the error case
/// `case`; `file` names the case in the messages (SPEC-0013 R1 to R5).
fn check_error(file: &str, case: &Case, report: &serde_json::Value) -> Vec<String> {
    let prefix = format!("{} [{}]", file, case.description);
    let mut failures = Vec::new();
    for (field, expected) in &case.expect_error_report {
        // R5: an entry of `errors` is named by its code and pointer, never by its position.
        if field.starts_with("/errors/") {
            failures.push(format!(
                "{prefix} expect_error_report {field}: name the entry in expect_errors"
            ));
            continue;
        }
        if let Some(problem) = compare_at(report, field, expected, None) {
            failures.push(format!(
                "{} [{}] error report {field}: {problem}",
                file, case.description
            ));
        }
    }
    let doc = serde_json::json!({ "error": report });
    failures.extend(check_sections(&prefix, case, &doc, None));
    for (name, count) in &case.expect_counts {
        let list = (name == "errors").then(|| &report["errors"]);
        failures.extend(count_named(&prefix, name, list, count));
    }
    failures
}

/// The failures of the semantic sections of `case` against `doc`: a result envelope, or
/// `{"error": <report>}` (SPEC-0013 R1, R2).
fn check_sections(
    prefix: &str,
    case: &Case,
    doc: &serde_json::Value,
    tolerance: Option<&str>,
) -> Vec<String> {
    let mut failures = Vec::new();
    for (section, part, list, names) in SECTIONS {
        let entries = case.section(section);
        if entries.is_empty() {
            continue;
        }
        let Some(items) = doc
            .get(part)
            .and_then(|p| p.get(list))
            .and_then(|l| l.as_array())
        else {
            let needs = if part == "error" {
                "needs expect_error, not expect"
            } else {
                "needs expect, not expect_error"
            };
            failures.push(format!("{prefix} {section}: {needs}"));
            continue;
        };
        for (key, fields) in entries {
            let named: Vec<&serde_json::Value> = items
                .iter()
                .filter(|e| entry_key(e, names) == *key)
                .collect();
            let [entry] = named[..] else {
                failures.push(format!(
                    "{prefix} {section} {key}: {} entries named it",
                    named.len()
                ));
                continue;
            };
            for (field, expected) in fields {
                let problem = match field.strip_suffix("#count") {
                    Some(list) => count_at(entry, list, expected),
                    None => compare_at(entry, field, expected, tolerance),
                };
                if let Some(problem) = problem {
                    failures.push(format!("{prefix} {section} {key} {field}: {problem}"));
                }
            }
        }
    }
    failures
}

/// The name of an entry: the values of `names`, joined by one space, `-` for a null or absent
/// value (SPEC-0013 R1).
fn entry_key(entry: &serde_json::Value, names: &[&str]) -> String {
    names
        .iter()
        .map(|n| match entry.get(*n) {
            None | Some(serde_json::Value::Null) => "-".to_owned(),
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(other) => other.to_string(),
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// A failure when `list` (absent when the name is not a list of this result) does not hold
/// `count` entries.
fn count_named(
    prefix: &str,
    name: &str,
    list: Option<&serde_json::Value>,
    count: &str,
) -> Option<String> {
    if name.is_empty() {
        return Some(format!(
            "{prefix} expect_counts: an empty key names nothing (empty key)"
        ));
    }
    let Some(list) = list else {
        return Some(format!(
            "{prefix} expect_counts {name}: not a list of this result"
        ));
    };
    let got = list.as_array().map(Vec::len);
    (got.map(|n| n.to_string()).as_deref() != Some(count)).then(|| {
        format!(
            "{prefix} expect_counts {name}: length {}, want {count}",
            shown(got)
        )
    })
}

/// A failure when the array at `field` of `container` does not hold `count` entries.
fn count_at(container: &serde_json::Value, field: &str, count: &str) -> Option<String> {
    let got = container
        .pointer(field)
        .and_then(|v| v.as_array())
        .map(Vec::len);
    (got.map(|n| n.to_string()).as_deref() != Some(count))
        .then(|| format!("length {}, want {count}", shown(got)))
}

/// A length as both runners write it: the number, or `none` when there is no list.
fn shown(got: Option<usize>) -> String {
    got.map_or_else(|| "none".to_owned(), |n| n.to_string())
}

/// The failure of one expectation: `field` (an RFC 6901 pointer, or a top-level field) of
/// `container` against `expected`. A decimal is compared within `tolerance` (exactly without
/// one); other text exactly; `null` passes on a null value or on an absent last token under
/// an existing object (SPEC-0013 R4). Any other missing field fails: a typo in a key must
/// never pass.
fn compare_at(
    container: &serde_json::Value,
    field: &str,
    expected: &str,
    tolerance: Option<&str>,
) -> Option<String> {
    if field.is_empty() {
        return Some("an empty key names the whole part: assert its fields (empty key)".to_owned());
    }
    let found = if field.starts_with('/') {
        container.pointer(field)
    } else {
        container.get(field)
    };
    let Some(raw) = found else {
        let parent = match field.rfind('/') {
            Some(at) if field.starts_with('/') => container.pointer(&field[..at]),
            _ => Some(container),
        };
        let absent_ok = expected == "null" && parent.is_some_and(serde_json::Value::is_object);
        return (!absent_ok).then(|| "not in the result".to_owned());
    };
    // Expectations that are not decimals are compared exactly: `null` (an undefined value,
    // such as a relative delta on a zero base), error codes, names.
    let want = Decimal::from_str(expected)
        .ok()
        .filter(|_| tolerance.is_some());
    let Some(want) = want else {
        let matches = if expected == "null" {
            raw.is_null()
        } else {
            raw.as_str() == Some(expected)
        };
        return (!matches).then(|| format!("got {raw}, want {expected}"));
    };
    let got = raw
        .as_str()
        .and_then(|v| Decimal::from_str(v).ok())
        .or_else(|| raw.as_u64().map(Decimal::from))
        .or_else(|| raw.as_i64().map(Decimal::from));
    let Some(got) = got else {
        return Some(format!("got {raw}, want the number {want}"));
    };
    let tolerance = tolerance.expect("checked above");
    (!within(&got.to_string(), expected, tolerance)).then(|| format!("got {got}, want {expected}"))
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

/// A temporary value case of `tvm.npv` with the sections `extra`, checked against a result
/// written in the test.
fn temporary_case(extra: &str) -> Case {
    serde_norway::from_str(&format!(
        "method: tvm.npv\ndescription: temporary\ninput: {{}}\ntolerance: \"0.01\"\n\
         expect:\n  /npv: \"4.13\"\n{extra}"
    ))
    .unwrap()
}

/// SPEC-0013 AC5 (R6): the sections retired by the migration are refused as unknown fields.
#[test]
fn retired_sections_are_refused() {
    for extra in [
        "expect_meta:\n  /defaults_applied/0/pointer: /timing\n",
        "expect_diagnostics:\n  /warnings/0/code: FM-W001\n",
        "expect_diagnostics_length:\n  /warnings: \"1\"\n",
        "expect_length:\n  /present_values: \"3\"\n",
    ] {
        let text = format!(
            "method: tvm.npv\ndescription: t\ninput: {{}}\ntolerance: \"0\"\nexpect:\n  /npv: \"1\"\n{extra}"
        );
        let refused = serde_norway::from_str::<Case>(&text)
            .err()
            .map(|e| e.to_string());
        assert!(
            refused
                .as_deref()
                .is_some_and(|e| e.contains("unknown field")),
            "{extra}: {refused:?}"
        );
    }
}

/// SPEC-0013 AC4 (R5): an entry of the error report is named by its code and pointer, never by
/// its position; the top-level fields are still read.
#[test]
fn a_positional_error_entry_is_refused() {
    let case = error_case(
        "expect_error_report:\n  /pointer: /cash_flows\n  /errors/1/reason: unknown_field\n",
    );
    let failures = check_error("temporary.yaml", &case, &semantic_report());
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(
        failures[0]
            .contains("expect_error_report /errors/1/reason: name the entry in expect_errors"),
        "{failures:?}"
    );
}

/// Code review of #394, N11 and N13: a count of a missing list reads `length none`, as in the
/// Python runner; an empty key, which would name the whole part, is refused everywhere.
#[test]
fn a_missing_list_and_an_empty_key_fail_with_the_python_messages() {
    let mut case = temporary_case(
        "expect_not_computed:\n  /value/roe: {\"/nope#count\": \"1\", \"\": x}\n\
         expect_counts:\n  \"\": \"1\"\n",
    );
    case.expect.insert(String::new(), "1".to_owned());
    let failures = check_result("temporary.yaml", &case, &semantic_result());
    let text = failures.join("\n");
    assert_eq!(failures.len(), 4, "{text}");
    assert!(text.contains("/nope#count: length none, want 1"), "{text}");
    let empty = failures
        .iter()
        .filter(|f| f.ends_with("(empty key)"))
        .count();
    assert_eq!(empty, 3, "{text}");
}

/// SPEC-0013: a result with every envelope list filled, for the semantic sections.
fn semantic_result() -> serde_json::Value {
    serde_json::json!({
        "method": "tvm.npv",
        "value": {"npv": "4.132", "present_values": ["1", "2", "3"], "roe": null},
        "explain": {"formula": "NPV", "steps": [], "conventions": ["end of period"]},
        "diagnostics": {
            "not_computed": [
                {"pointer": "/value/roe", "reason_code": "not_defined",
                 "missing_any_of": [], "permitted": []},
                {"pointer": "/value/ros", "reason_code": "missing_input",
                 "missing_any_of": [["/revenue"], ["/sales", "/margin"]], "permitted": []}
            ],
            "warnings": [
                {"code": "FM-W012", "pointer": "/net_financial_position", "message": "m"},
                {"code": "FM-W005", "pointer": null, "message": "m"}
            ],
            "checks": [
                {"id": "a_total", "status": "reconciled", "pointer": "/value/years/0", "pointers": []},
                {"id": "a_total", "status": "not_checked", "pointer": "/value/years/1", "pointers": []}
            ]
        },
        "meta": {"envelope_version": 1, "defaults_applied": [
            {"pointer": "/timing", "value": "end_of_period", "reason": "r"},
            {"pointer": "/periods", "value": 5, "reason": "r"}
        ]}
    })
}

fn semantic_report() -> serde_json::Value {
    serde_json::json!({
        "code": "FM-E001", "pointer": "/cash_flows", "reason": "missing",
        "errors": [
            {"code": "FM-E001", "pointer": "/cash_flows", "reason": "missing"},
            {"code": "FM-E001", "pointer": "/rat", "reason": "unknown_field", "expected": ["rate"]},
            {"code": "FM-E027", "pointer": null, "reason": "requires"}
        ]
    })
}

fn error_case(extra: &str) -> Case {
    serde_norway::from_str(&format!(
        "method: tvm.npv\ndescription: temporary\ninput: {{}}\nexpect_error: FM-E001\n{extra}"
    ))
    .unwrap()
}

/// SPEC-0013 AC1 to AC3: every semantic section, `expect_counts`, `#count` and `null` passes
/// on a matching result.
#[test]
fn semantic_sections_pass_on_a_matching_result() {
    let case = temporary_case(
        "  /roe: \"null\"\n  /irr: \"null\"\n\
         expect_defaults:\n  /timing: {/value: end_of_period}\n  /periods: {/value: \"5\"}\n\
         expect_not_computed:\n  /value/roe: {/reason_code: not_defined}\n  \
         /value/ros: {/missing_any_of/1/0: /sales, \"/missing_any_of#count\": \"2\"}\n\
         expect_warnings:\n  FM-W012 /net_financial_position: {}\n  FM-W005 -: {/message: m}\n\
         expect_checks:\n  /value/years/1 a_total: {/status: not_checked}\n\
         expect_counts:\n  not_computed: \"2\"\n  warnings: \"2\"\n  checks: \"2\"\n  \
         /present_values: \"3\"\n",
    );
    let failures = check_result("temporary.yaml", &case, &semantic_result());
    assert!(failures.is_empty(), "{failures:?}");
}

/// SPEC-0013 AC1 and AC2: a wrong value, a key naming no entry or two entries, a key of
/// another list and a wrong count fail, each with its own message.
#[test]
fn semantic_sections_fail_on_a_wrong_value_a_wrong_key_or_a_wrong_count() {
    let mut result = semantic_result();
    result["diagnostics"]["warnings"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!({"code": "FM-W005", "pointer": null, "message": "m"}));
    let case = temporary_case(
        "expect_defaults:\n  /timing: {/value: mid_period}\n\
         expect_not_computed:\n  /value/roi: {}\n  \
         /value/ros: {\"/missing_any_of#count\": \"1\"}\n\
         expect_warnings:\n  FM-W005 -: {}\n  /value/roe: {}\n\
         expect_checks:\n  /value/years/0 b_total: {}\n  \
         /value/years/1 a_total: {/status: reconciled}\n\
         expect_counts:\n  warnings: \"2\"\n  /present_values: \"4\"\n  errors: \"0\"\n",
    );
    let failures = check_result("temporary.yaml", &case, &result);
    let text = failures.join("\n");
    assert_eq!(failures.len(), 10, "{text}");
    for want in [
        "expect_checks /value/years/0 b_total: 0 entries named it",
        "expect_checks /value/years/1 a_total /status: got \"not_checked\", want reconciled",
        "expect_defaults /timing /value: got \"end_of_period\", want mid_period",
        "expect_not_computed /value/roi: 0 entries named it",
        "expect_not_computed /value/ros /missing_any_of#count: length 2, want 1",
        "expect_warnings FM-W005 -: 2 entries named it",
        "expect_warnings /value/roe: 0 entries named it",
        "expect_counts warnings: length 3, want 2",
        "expect_counts /present_values: length 3, want 4",
        "expect_counts errors: not a list of this result",
    ] {
        assert!(text.contains(want), "missing {want:?} in\n{text}");
    }
}

/// SPEC-0013 AC3 (R4): `null` passes on a null value and on an absent last token, fails on an
/// absent parent; any other expectation fails on an absent key.
#[test]
fn null_means_null_or_absent_under_an_existing_parent() {
    let case = temporary_case(
        "  /roe: \"null\"\n  /absent: \"null\"\n  /missing/deeper: \"null\"\n  \
         /present_values/7: \"null\"\n  /other: \"1\"\n",
    );
    let failures = check_result("temporary.yaml", &case, &semantic_result());
    let text = failures.join("\n");
    assert_eq!(failures.len(), 3, "{text}");
    assert!(
        text.contains("/missing/deeper: not in the result"),
        "{text}"
    );
    assert!(
        text.contains("/present_values/7: not in the result"),
        "{text}"
    );
    assert!(text.contains("/other: not in the result"), "{text}");
}

/// SPEC-0013 AC1 and AC2: error entries by code and pointer, the count of the report, the
/// top-level fields still read, and sections of a value case refused in an error case.
#[test]
fn error_sections_name_entries_and_refuse_positional_keys() {
    let ok = error_case(
        "expect_error_report:\n  /pointer: /cash_flows\n  /reason: missing\n\
         expect_errors:\n  FM-E001 /rat: {/reason: unknown_field, /expected/0: rate}\n  \
         FM-E027 -: {}\n\
         expect_counts:\n  errors: \"3\"\n",
    );
    let failures = check_error("temporary.yaml", &ok, &semantic_report());
    assert!(failures.is_empty(), "{failures:?}");
    let bad = error_case(
        "expect_errors:\n  FM-E001 /rat: {/reason: missing}\n  FM-E001 /rate: {}\n\
         expect_warnings:\n  FM-W005 -: {}\n\
         expect_counts:\n  errors: \"2\"\n  warnings: \"0\"\n",
    );
    let failures = check_error("temporary.yaml", &bad, &semantic_report());
    let text = failures.join("\n");
    assert_eq!(failures.len(), 5, "{text}");
    for want in [
        "expect_errors FM-E001 /rat /reason: got \"unknown_field\", want missing",
        "expect_errors FM-E001 /rate: 0 entries named it",
        "expect_warnings: needs expect, not expect_error",
        "expect_counts errors: length 3, want 2",
        "expect_counts warnings: not a list of this result",
    ] {
        assert!(text.contains(want), "missing {want:?} in\n{text}");
    }
    // A value case cannot name error entries.
    let case = temporary_case("expect_errors:\n  FM-E001 /rat: {}\n");
    let failures = check_result("temporary.yaml", &case, &semantic_result());
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("expect_errors: needs expect_error, not expect"));
}

/// `v` with the keys of every object, at every depth, put in the order `order` gives for the
/// number of keys and the depth-first position of the object; arrays keep their order.
fn permuted_keys(
    v: &serde_json::Value,
    order: &mut dyn FnMut(usize) -> Vec<usize>,
) -> serde_json::Value {
    match v {
        serde_json::Value::Object(map) => {
            let entries: Vec<_> = map.iter().collect();
            let positions = order(entries.len());
            serde_json::Value::Object(
                positions
                    .into_iter()
                    .map(|i| (entries[i].0.clone(), permuted_keys(entries[i].1, order)))
                    .collect(),
            )
        }
        serde_json::Value::Array(items) => {
            serde_json::Value::Array(items.iter().map(|x| permuted_keys(x, order)).collect())
        }
        other => other.clone(),
    }
}

/// The text a client receives for `input`: the envelope, or `{"error": <report>}`. For an
/// error, the code, the message and the pointer of the error itself (the typed API, the
/// compact report of an analysis cell) must be those of the first entry of the report (code
/// review of #371, S6); a mismatch is returned as the text instead.
fn received(method: &str, input: &serde_json::Value) -> String {
    match fairmark_core::run(method, &input.to_string()) {
        Ok(envelope) => envelope.to_string(),
        Err(e) => {
            let report = e.to_report();
            let first = &report.errors[0];
            if e.code() != first.code
                || e.to_string() != first.message
                || e.details().pointer != first.pointer
            {
                return format!(
                    "the error starts from {} {} {:?}, the report from {} {} {:?}",
                    e.code(),
                    e,
                    e.details().pointer,
                    first.code,
                    first.message,
                    first.pointer
                );
            }
            serde_json::json!({ "error": report }).to_string()
        }
    }
}

/// Issue #370 (principle I): the order of the keys of the input changes nothing a client
/// receives. For every golden input, results and errors alike, the keys of every object are
/// reversed, and shuffled with a fixed seed, at every depth: the envelope or the error report
/// is byte-identical to that of the input as written.
#[test]
fn key_order_changes_neither_envelope_nor_error_report() {
    // SplitMix64 with a fixed seed: the same shuffles on every run and platform.
    let mut state: u64 = 0x5EED_0370;
    let mut next = move || {
        state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        z ^ (z >> 31)
    };
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut files = Vec::new();
    golden_files(&root, &mut files);
    files.sort();
    let files = test_profile::sample(files, |f| f.parent().map(Path::to_path_buf));
    let (mut checked, mut errors) = (0, 0);
    let mut failures = Vec::new();
    for file in &files {
        let case: Case = serde_norway::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        let written = received(&case.method, &case.input);
        let reversed = permuted_keys(&case.input, &mut |n| (0..n).rev().collect());
        let shuffled = permuted_keys(&case.input, &mut |n| {
            // Fisher-Yates.
            let mut p: Vec<usize> = (0..n).collect();
            for i in (1..n).rev() {
                let j = usize::try_from(next() % (i as u64 + 1)).unwrap();
                p.swap(i, j);
            }
            p
        });
        for (how, input) in [("reversed", &reversed), ("shuffled", &shuffled)] {
            if received(&case.method, input) != written {
                failures.push(format!("{} ({how})", file.display()));
            }
        }
        checked += 1;
        errors += usize::from(case.expect_error.is_some());
    }
    assert!(
        checked > test_profile::pick(300, 80) && errors > test_profile::pick(50, 20),
        "{checked} cases, {errors} errors"
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The key sets of every choice written as an object with one key in `schema`: an object with
/// `maxProperties` 1, or a `oneOf` / `anyOf` whose branches are objects of one property.
fn single_key_choices(schema: &serde_json::Value) -> Vec<Vec<String>> {
    fn resolve<'a>(root: &'a serde_json::Value, n: &'a serde_json::Value) -> &'a serde_json::Value {
        match n["$ref"].as_str().and_then(|r| r.strip_prefix("#/$defs/")) {
            Some(name) => &root["$defs"][name],
            None => n,
        }
    }
    fn walk(root: &serde_json::Value, n: &serde_json::Value, out: &mut Vec<Vec<String>>) {
        match n {
            serde_json::Value::Object(map) => {
                if map.get("maxProperties").and_then(serde_json::Value::as_u64) == Some(1) {
                    if let Some(props) = map.get("properties").and_then(|p| p.as_object()) {
                        out.push(props.keys().cloned().collect());
                    }
                }
                for k in ["oneOf", "anyOf"] {
                    let Some(branches) = map.get(k).and_then(|b| b.as_array()) else {
                        continue;
                    };
                    let keys: Vec<String> = branches
                        .iter()
                        .map(|b| resolve(root, b))
                        .filter_map(|b| {
                            let props = b["properties"].as_object()?;
                            (props.len() == 1).then(|| props.keys().next().unwrap().clone())
                        })
                        .collect();
                    if keys.len() > 1 && keys.len() == branches.len() {
                        out.push(keys);
                    }
                }
                map.values().for_each(|v| walk(root, v, out));
            }
            serde_json::Value::Array(items) => items.iter().for_each(|v| walk(root, v, out)),
            _ => {}
        }
    }
    let mut out = Vec::new();
    walk(schema, schema, &mut out);
    out
}

/// The pointers of the objects of `v` written with one key that is a key of a choice, with the
/// key and another key of the same choice.
fn choice_objects(
    v: &serde_json::Value,
    at: String,
    choices: &[Vec<String>],
    out: &mut Vec<(String, String, String)>,
) {
    match v {
        serde_json::Value::Object(map) => {
            if map.len() == 1 {
                let key = map.keys().next().unwrap();
                if let Some(other) = choices
                    .iter()
                    .filter(|c| c.contains(key))
                    .find_map(|c| c.iter().find(|k| *k != key))
                {
                    out.push((at.clone(), key.clone(), other.clone()));
                }
            }
            for (k, x) in map {
                let token = k.replace('~', "~0").replace('/', "~1");
                choice_objects(x, format!("{at}/{token}"), choices, out);
            }
        }
        serde_json::Value::Array(items) => {
            for (i, x) in items.iter().enumerate() {
                choice_objects(x, format!("{at}/{i}"), choices, out);
            }
        }
        _ => {}
    }
}

/// ADR 0014 G5 (declared reads; found with issue #370): a choice written as an object with one
/// key and given two is reported the same way in every method, whatever its keys contain: one
/// entry, `length` at the choice, whose message names the keys found; nothing within the
/// choice is reported, since the value read in its place decides nothing. On every golden input
/// that succeeds, each such choice (the DCF terminal, the techniques of `tv.compare`, growth and
/// timeline of `series.project`, destination and split of `statements.reorganise`; for an
/// analysis, those of its `base_input`) is given a second key of the same choice, holding an
/// empty object, the terms of the first key or a number: the report is that entry alone, and
/// the same with the keys of every object reversed. With an unknown key beside the two keys,
/// before or after them, the unknown key is one more entry, at the key, with the message of any
/// unknown field.
#[test]
fn a_choice_with_two_keys_is_one_length_entry() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut files = Vec::new();
    golden_files(&root, &mut files);
    files.sort();
    // Inputs that succeed and hold a choice, with their choice objects; in the mutants
    // profile, a sample of them per method.
    let cases: Vec<_> = files
        .into_iter()
        .filter_map(|file| {
            let case: Case =
                serde_norway::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            if case.expect_error.is_some() {
                return None;
            }
            let (inner, base) = if case.method.starts_with("analysis.") {
                (case.input["method"].as_str().unwrap(), "/base_input")
            } else {
                (case.method.as_str(), "")
            };
            let choices = single_key_choices(&fairmark_core::input_schema(inner).unwrap());
            let mut objects = Vec::new();
            choice_objects(
                case.input.pointer(base).unwrap(),
                base.to_owned(),
                &choices,
                &mut objects,
            );
            let inner = inner.to_owned();
            (!objects.is_empty()).then_some((file, case, inner, objects))
        })
        .collect();
    let cases = test_profile::sample(cases, |(_, case, _, _)| case.method.clone());
    let (mut checked, mut failures) = (0, Vec::new());
    let mut methods = std::collections::BTreeSet::new();
    for (file, case, inner, objects) in cases {
        let inner = inner.as_str();
        let base = if case.method.starts_with("analysis.") {
            "/base_input"
        } else {
            ""
        };
        for (at, key, other) in objects {
            let terms = case.input.pointer(&at).unwrap()[key.as_str()].clone();
            let variants = [
                (serde_json::json!({}), false),
                (terms, false),
                (serde_json::json!(1), false),
                (serde_json::json!({}), true),
            ];
            for (content, unknown) in variants {
                let mut input = case.input.clone();
                let choice = input.pointer_mut(&at).unwrap();
                if unknown {
                    // An unknown key before the two keys of the choice (code review of #371,
                    // B1); the reversed order below puts it after them.
                    let mut with: serde_json::Map<String, serde_json::Value> =
                        serde_json::Map::new();
                    with.insert("bogus".to_owned(), serde_json::json!(1));
                    with.extend(choice.as_object().unwrap().clone());
                    *choice = serde_json::Value::Object(with);
                }
                choice[other.as_str()] = content.clone();
                let entries = match fairmark_core::run(&case.method, &input.to_string()) {
                    Ok(_) => vec!["accepted".to_owned()],
                    Err(e) => e
                        .to_report()
                        .errors
                        .iter()
                        .map(|x| format!("{:?} {:?} {}", x.pointer, x.reason, x.message))
                        .collect(),
                };
                checked += 1;
                methods.insert(inner.to_owned());
                // The order of the keys, of the choice and of every other object, changes
                // nothing (principle I).
                let reversed = permuted_keys(&input, &mut |n| (0..n).rev().collect());
                if received(&case.method, &reversed) != received(&case.method, &input) {
                    failures.push(format!("{} {at}: depends on the key order", file.display()));
                }
                let mut keys = [key.as_str(), other.as_str()];
                keys.sort_unstable();
                let one = entries.len() == 1 + usize::from(unknown)
                    && entries[0].starts_with(&format!("Some({at:?}) Some(Length) "))
                    && entries[0].contains(&format!("(got 2 keys: {})", keys.join(", ")))
                    && (!unknown
                        || entries[1].starts_with(&format!(
                            "Some(\"{at}/bogus\") Some(UnknownField) invalid input at \
                             `{inner_at}/bogus`: unknown field `bogus`",
                            inner_at = at.strip_prefix(base).unwrap_or(&at)
                        )));
                if !one {
                    failures.push(format!(
                        "{} {at} ({key} + {other}: {content}):\n  {}",
                        file.display(),
                        entries.join("\n  ")
                    ));
                }
            }
        }
    }
    for m in [
        "dcf.fcff",
        "dcf.fcfe",
        "tv.compare",
        "series.project",
        "statements.reorganise",
    ] {
        assert!(methods.contains(m), "no choice of {m} checked: {methods:?}");
    }
    assert!(checked > test_profile::pick(30, 80), "{checked}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Code review of #371, S6: a cell of an analysis whose inner input has two errors records the
/// first entry of the inner method's report, whatever the order of the keys of the request.
#[test]
fn an_analysis_cell_records_the_first_entry_of_its_report() {
    let request = serde_json::json!({
        "method": "tvm.annuity",
        "base_input": {"payment": "100", "rate": "0.05", "periods": 3, "growth": "0.01",
                       "timing": "ordinary"},
        "output_pointers": ["/present_value"],
        "scenarios": [{"name": "both refused", "overrides": [
            {"pointer": "/rate", "value": "-1"},
            {"pointer": "/growth", "value": "-1"}
        ]}]
    });
    let inner = serde_json::json!({"payment": "100", "rate": "-1", "periods": 3,
                                   "growth": "-1", "timing": "ordinary"});
    let report = fairmark_core::run("tvm.annuity", &inner.to_string())
        .unwrap_err()
        .to_report();
    assert!(report.errors.len() >= 2, "{:?}", report.errors);
    let written = received("analysis.scenarios", &request);
    let out: serde_json::Value = serde_json::from_str(&written).expect(&written);
    let cell = &out["value"]["scenarios"][0]["error"];
    assert_eq!(cell["code"], report.errors[0].code, "{out:#}");
    assert_eq!(
        cell["message"],
        report.errors[0].message.as_str(),
        "{out:#}"
    );
    let reversed = permuted_keys(&request, &mut |n| (0..n).rev().collect());
    assert_eq!(received("analysis.scenarios", &reversed), written);
}
