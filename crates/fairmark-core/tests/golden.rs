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
    /// list such as `/not_computed` has no entry beyond those asserted; optional. In an error
    /// case, the pointers are into the error report, for example `/errors`.
    #[serde(default)]
    expect_length: std::collections::BTreeMap<String, String>,
    /// Expected fields of `diagnostics` (keys are JSON Pointers into `diagnostics`, for example
    /// `/not_computed/0/reason_code` or `/warnings/1/code`), compared like `expect`; optional,
    /// for cases of SPEC-0012.
    #[serde(default)]
    expect_diagnostics: std::collections::BTreeMap<String, String>,
    /// Expected lengths of arrays of `diagnostics` (keys are JSON Pointers into `diagnostics`,
    /// for example `/warnings`, values counts), so that a list has no entry beyond those
    /// asserted; optional, for cases of SPEC-0012.
    #[serde(default)]
    expect_diagnostics_length: std::collections::BTreeMap<String, String>,
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
            if !case.expect_diagnostics.is_empty() || !case.expect_diagnostics_length.is_empty() {
                failures.push(format!(
                    "{}: expect_diagnostics and expect_diagnostics_length need expect, not \
                     expect_error",
                    file.display()
                ));
            }
            match result {
                Err(e) if e.code() == code => {
                    let report = serde_json::to_value(e.to_report()).unwrap();
                    for (field, count) in &case.expect_length {
                        let got = report
                            .pointer(field)
                            .and_then(|v| v.as_array())
                            .map(Vec::len);
                        if got.map(|n| n.to_string()).as_deref() != Some(count.as_str()) {
                            failures.push(format!(
                                "{} [{}] error report {field}: length {got:?}, want {count}",
                                file.display(),
                                case.description
                            ));
                        }
                    }
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
    let lengths = case
        .expect_length
        .iter()
        .map(|(f, c)| ("value", f, c))
        .chain(
            case.expect_diagnostics_length
                .iter()
                .map(|(f, c)| ("diagnostics", f, c)),
        );
    for (part, field, count) in lengths {
        let got = out[part]
            .pointer(field)
            .and_then(|v| v.as_array())
            .map(Vec::len);
        if got.map(|n| n.to_string()).as_deref() != Some(count.as_str()) {
            failures.push(format!(
                "{} [{}] {field}: length {got:?}, want {count}",
                file, case.description
            ));
        }
    }
    let tolerance = case.tolerance.as_deref().expect("tolerance");
    let expectations = case
        .expect
        .iter()
        .map(|(f, e)| ("value", f, e))
        .chain(case.expect_meta.iter().map(|(f, e)| ("meta", f, e)))
        .chain(
            case.expect_diagnostics
                .iter()
                .map(|(f, e)| ("diagnostics", f, e)),
        );
    for (part, field, expected) in expectations {
        // A key starting with '/' is an RFC 6901 pointer into `value` (`meta` for
        // `expect_meta`, `diagnostics` for `expect_diagnostics`); any other key is a top-level
        // field.
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
                file, case.description
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
                    file, case.description
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
                file, case.description
            ));
            continue;
        };
        if !within(&got.to_string(), expected, tolerance) {
            failures.push(format!(
                "{} [{}] {field}: got {got}, want {expected}",
                file, case.description
            ));
        }
    }
    failures
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

/// A temporary case with the diagnostics keys of SPEC-0012, checked against a result written
/// here (the engine does not write `diagnostics` before task T11).
fn temporary_case(extra: &str) -> Case {
    serde_norway::from_str(&format!(
        "method: tvm.npv\ndescription: temporary\ninput: {{}}\ntolerance: \"0.01\"\n\
         expect:\n  /npv: \"4.13\"\n{extra}"
    ))
    .unwrap()
}

fn temporary_result() -> serde_json::Value {
    serde_json::json!({
        "method": "tvm.npv",
        "value": {"npv": "4.132"},
        "explain": {"formula": "NPV", "steps": [], "conventions": ["end of period"]},
        "diagnostics": {
            "not_computed": [{"pointer": "/value/roe", "reason_code": "not_defined"}],
            "warnings": [
                {"code": "FM-W012", "pointer": "/net_financial_position", "message": "m"},
                {"code": "FM-W005", "pointer": null, "message": "m"}
            ],
            "checks": []
        },
        "meta": {"envelope_version": 1}
    })
}

#[test]
fn diagnostics_expectations_pass_on_a_matching_result() {
    let case = temporary_case(
        "expect_diagnostics:\n  /warnings/0/code: FM-W012\n  \
         /warnings/0/pointer: /net_financial_position\n  /warnings/1/pointer: \"null\"\n  \
         /not_computed/0/reason_code: not_defined\n\
         expect_diagnostics_length:\n  /warnings: \"2\"\n  /checks: \"0\"\n",
    );
    assert_eq!(case.expect_diagnostics.len(), 4);
    assert_eq!(case.expect_diagnostics_length.len(), 2);
    let failures = check_result("temporary.yaml", &case, &temporary_result());
    assert!(failures.is_empty(), "{failures:?}");
}

#[test]
fn diagnostics_expectations_fail_on_a_wrong_value_a_missing_key_or_a_wrong_length() {
    let case = temporary_case(
        "expect_diagnostics:\n  /warnings/0/code: FM-W013\n  /warnings/2/code: FM-W005\n  \
         /warnings/0/pointer: \"null\"\n\
         expect_diagnostics_length:\n  /warnings: \"1\"\n  /not_computed/0: \"1\"\n",
    );
    let failures = check_result("temporary.yaml", &case, &temporary_result());
    let text = failures.join("\n");
    assert_eq!(failures.len(), 5, "{text}");
    assert!(
        text.contains("/warnings/0/code: got \"FM-W012\", want FM-W013"),
        "{text}"
    );
    assert!(
        text.contains("/warnings/2/code: not in the result"),
        "{text}"
    );
    assert!(text.contains("/warnings/0/pointer: got \"/net_financial_position\", want null"));
    assert!(text.contains("/warnings: length Some(2), want 1"), "{text}");
    assert!(
        text.contains("/not_computed/0: length None, want 1"),
        "{text}"
    );
}

#[test]
fn diagnostics_expectations_read_diagnostics_not_value() {
    // The same pointer in `value` must not satisfy an expectation on `diagnostics`.
    let case = temporary_case("expect_diagnostics:\n  /npv: \"4.13\"\n");
    let failures = check_result("temporary.yaml", &case, &temporary_result());
    assert_eq!(failures.len(), 1, "{failures:?}");
    assert!(failures[0].contains("/npv: not in the result"));
    // A result without `diagnostics` fails every expectation on it.
    let mut result = temporary_result();
    result.as_object_mut().unwrap().remove("diagnostics");
    let case = temporary_case(
        "expect_diagnostics:\n  /warnings/0/code: FM-W012\n\
         expect_diagnostics_length:\n  /warnings: \"2\"\n",
    );
    assert_eq!(check_result("temporary.yaml", &case, &result).len(), 2);
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
        checked > 300 && errors > 50,
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
    let (mut checked, mut failures) = (0, Vec::new());
    let mut methods = std::collections::BTreeSet::new();
    for file in &files {
        let case: Case = serde_norway::from_str(&std::fs::read_to_string(file).unwrap()).unwrap();
        if case.expect_error.is_some() {
            continue;
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
    assert!(checked > 30, "{checked}");
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
