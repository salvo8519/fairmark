//! The warning catalogue (SPEC-0012 R16, AC14): every code the engine declares is documented in
//! `docs/en/reference/warnings.md` and emitted by production code, every documented code is
//! declared, and no production source writes a convention starting with `warning` other than
//! through the warning API of the core (`src/diagnostics.rs`).
//!
//! The check reads the sources as text, so that a planted code or literal can be tested without
//! compiling it. A code is declared in `src/diagnostics.rs` as `Variant = "FM-Wnnn"` and
//! emitted as `WarningCode::Variant`.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// The file that holds the warning API: the only one that may write a `warning` text.
const API: &str = "diagnostics.rs";

/// `(file name, production text)` of every source file of the core: the text before its unit
/// tests module.
fn sources() -> Vec<(String, String)> {
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        for entry in std::fs::read_dir(dir).unwrap() {
            let path = entry.unwrap().path();
            if path.is_dir() {
                walk(&path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                out.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut files = Vec::new();
    walk(&root, &mut files);
    files.sort();
    files
        .into_iter()
        .map(|f| {
            let text = std::fs::read_to_string(&f).unwrap();
            let name = f.strip_prefix(&root).unwrap().display().to_string();
            (name, production(&text).to_owned())
        })
        .collect()
}

/// The text of a source file before its unit tests module.
fn production(text: &str) -> &str {
    text.find("#[cfg(test)]\nmod tests")
        .map_or(text, |end| &text[..end])
}

/// Codes listed in the first column of the tables of the catalogue page.
fn documented(catalogue: &str) -> BTreeSet<String> {
    catalogue
        .lines()
        .filter_map(|l| l.strip_prefix("| "))
        .filter_map(|l| l.split(" |").next())
        .map(|c| c.trim().trim_matches('`').to_owned())
        .filter(|c| is_code(c))
        .collect()
}

fn is_code(c: &str) -> bool {
    c.len() == 7 && c.starts_with("FM-W") && c[4..].bytes().all(|b| b.is_ascii_digit())
}

/// `variant -> code` of the declarations `Variant = "FM-Wnnn"` of the warning API.
fn declared(api: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    for line in api.lines() {
        let line = line.trim().trim_end_matches(',');
        if let Some((variant, code)) = line.split_once(" = ") {
            let code = code.trim_matches('"');
            if is_code(code) && variant.bytes().all(|b| b.is_ascii_alphanumeric()) {
                out.insert(variant.to_owned(), code.to_owned());
            }
        }
    }
    out
}

/// Variants named as `WarningCode::Variant` in `text`.
fn emitted(text: &str) -> BTreeSet<String> {
    text.match_indices("WarningCode::")
        .map(|(i, m)| {
            text[i + m.len()..]
                .chars()
                .take_while(char::is_ascii_alphanumeric)
                .collect::<String>()
        })
        .filter(|v| v.starts_with(|c: char| c.is_ascii_uppercase()))
        .collect()
}

/// Lines of `text` with a string literal starting with `warning` followed by a space or a
/// colon, in any case.
fn warning_literals(text: &str) -> Vec<String> {
    text.lines()
        .filter(|l| {
            let lower = l.to_ascii_lowercase();
            lower.contains("\"warning ") || lower.contains("\"warning:")
        })
        .map(|l| l.trim().to_owned())
        .collect()
}

/// Every breach of R16 in `sources` against `catalogue`.
fn breaches(sources: &[(String, String)], catalogue: &str) -> Vec<String> {
    let mut out = Vec::new();
    let api = sources
        .iter()
        .find(|(name, _)| name == API)
        .map(|(_, t)| t.as_str());
    let Some(api) = api else {
        return vec![format!("no warning API: src/{API} is missing")];
    };
    let declared = declared(api);
    if declared.is_empty() {
        out.push(format!("src/{API} declares no warning code"));
    }
    let documented = documented(catalogue);
    let mut used = BTreeSet::new();
    for (name, text) in sources {
        if name == API {
            continue;
        }
        used.extend(emitted(text));
        for line in warning_literals(text) {
            out.push(format!(
                "src/{name}: a warning written outside the API: {line}"
            ));
        }
    }
    let codes: BTreeSet<&String> = declared.values().collect();
    if codes.len() != declared.len() {
        out.push("a code is declared twice".to_owned());
    }
    for (variant, code) in &declared {
        if !documented.contains(code) {
            out.push(format!("{code} ({variant}) is not in the catalogue"));
        }
        if !used.contains(variant) {
            out.push(format!("{code} ({variant}) has no emission site"));
        }
    }
    for code in &documented {
        if !codes.contains(code) {
            out.push(format!("{code} is in the catalogue but not declared"));
        }
    }
    for variant in &used {
        if !declared.contains_key(variant) {
            out.push(format!(
                "WarningCode::{variant} is emitted but not declared"
            ));
        }
    }
    out
}

fn catalogue() -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/en/reference/warnings.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
}

/// R16: the engine and its catalogue agree, and no warning is written outside the API.
#[test]
#[ignore = "SPEC-0012: enabled by T20, which writes the catalogue page"]
fn every_warning_code_is_documented_and_emitted() {
    let found = breaches(&sources(), &catalogue());
    assert!(found.is_empty(), "{}", found.join("\n"));
    let declared = declared(&sources().into_iter().find(|(n, _)| n == API).unwrap().1);
    let codes: Vec<&str> = declared.values().map(String::as_str).collect();
    let expected: Vec<String> = (1..=16).map(|n| format!("FM-W{n:03}")).collect();
    let mut sorted = codes.clone();
    sorted.sort_unstable();
    assert_eq!(sorted, expected);
}

/// AC14: the check fails on a code that is not documented and on a `warning:` literal written
/// outside the API.
#[test]
fn the_check_fails_on_a_planted_code_or_literal() {
    let api = "pub enum WarningCode {\n    First = \"FM-W001\",\n    Second = \"FM-W002\",\n}\n";
    let user = "fn f(ex: &mut Explain) { ex.warn(WarningCode::First, None, \"x\"); \
                ex.warn(WarningCode::Second, None, \"y\"); }\n";
    let catalogue = "| Code | Condition |\n| --- | --- |\n| `FM-W001` | a |\n| `FM-W002` | b |\n";
    let files = |user: &str| {
        vec![
            (API.to_owned(), api.to_owned()),
            ("user.rs".to_owned(), user.to_owned()),
        ]
    };
    assert_eq!(breaches(&files(user), catalogue), Vec::<String>::new());

    // A planted code that the catalogue does not list.
    let planted = api.replace("}\n", "    Third = \"FM-W003\",\n}\n");
    let found = breaches(
        &[
            (API.to_owned(), planted),
            (
                "user.rs".to_owned(),
                format!("{user}fn g() {{ WarningCode::Third; }}\n"),
            ),
        ],
        catalogue,
    );
    assert_eq!(found, ["FM-W003 (Third) is not in the catalogue"]);

    // A planted `warning:` literal outside the API.
    let found = breaches(
        &files(&format!(
            "{user}fn h(ex: &mut Explain) {{ ex.convention(\"warning: x\"); }}\n"
        )),
        catalogue,
    );
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("a warning written outside the API"));

    // A documented code without emission site, and a catalogue code not declared.
    let found = breaches(
        &files("fn f(ex: &mut Explain) { ex.warn(WarningCode::First, None, \"x\"); }\n"),
        &format!("{catalogue}| `FM-W009` | c |\n"),
    );
    assert_eq!(
        found,
        [
            "FM-W002 (Second) has no emission site",
            "FM-W009 is in the catalogue but not declared"
        ]
    );
    // Literals in a unit tests module are not production code.
    assert_eq!(
        production("fn a() {}\n#[cfg(test)]\nmod tests { \"warning: x\" }"),
        "fn a() {}\n"
    );
}
