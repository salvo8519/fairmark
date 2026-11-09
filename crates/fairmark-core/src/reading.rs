//! Reading a method input (ADR 0013, ADR 0014 G5): the typed input, or every input error the
//! reading finds, each named by its JSON pointer with its sub-kind, the accepted values and a
//! fix. `serde` stops at the first error; the text is then read as JSON and walked against the
//! schema of the input type to find every unknown key, repeated key and positional array.

use schemars::{schema_for, JsonSchema};
use serde_json::Value;

use crate::dates;
use crate::error::{CustomRefusal, FairmarkError, Reason, Result};

/// Reads a method input. Every failure names the field by its JSON pointer (issue #174,
/// SPEC-0001 R27): `FM-E001`, or `FM-E009` for an invalid date. Only a malformed JSON text
/// (a syntax error, a truncated input, trailing characters) keeps serde's line and column,
/// because it has no field to name.
pub(crate) fn parse<T: serde::de::DeserializeOwned + JsonSchema>(input_json: &str) -> Result<T> {
    let mut de = serde_json::Deserializer::from_str(input_json);
    let input = serde_path_to_error::deserialize(&mut de).map_err(|e| {
        let first = input_error(e);
        if first.details().reason == Some(Reason::UnknownField) {
            every_unknown_field(first, &schema_for!(T), input_json)
        } else {
            first
        }
    })?;
    // `serde_json::from_str` does the same: nothing but whitespace may follow the value.
    de.end().map_err(syntax_error)?;
    Ok(input)
}

/// Key under which `serde_json` (with `arbitrary_precision`) hands an exact number to its
/// deserialiser; it is not part of the input, so it never appears in a pointer.
const NUMBER_TOKEN: &str = "$serde_json::private::Number";

/// Text of the error `serde_json` and `rust_decimal` raise when an object is given where a
/// decimal is expected: they read numbers through a map with one private key.
const OBJECT_FOR_NUMBER: &str = "expected field with custom name";

/// What `rust_decimal` says it expects, for a decimal and for an optional decimal.
const RUST_DECIMAL_EXPECTED: &str = "a Decimal type representing a fixed-point number";

/// What Fairmark says instead, in every error about a decimal.
const DECIMAL_EXPECTED: &str = "a decimal number (a JSON number or a string such as \"0.10\")";

/// Marks an error raised while reading a value that its type buffered before reading it
/// (`tv::compare::TechniqueEntry`): the text between the two markers is the pointer of the
/// error relative to that value, which [`input_error`] appends to the path. Built only by
/// [`nested_error`]; never shown.
const NESTED: &str = "\u{1f}fairmark-relative-pointer\u{1f}";

/// The text of a custom error that carries the relative pointer of an error raised while
/// reading a buffered value, so that the reported pointer still names the field (issue #174).
pub(crate) fn nested_error(
    relative: &[String],
    e: serde_path_to_error::Error<serde_json::Error>,
) -> String {
    let mut tokens = relative.to_vec();
    tokens.extend(path_tokens(e.path()));
    format!(
        "{NESTED}{}{NESTED}{}",
        pointer(&tokens).unwrap_or_default(),
        e.into_inner()
    )
}

/// The text of a custom error raised by a type that reads a buffered value itself, located at
/// `relative` within that value (for example the `kind` of an adjustment of
/// `statements.reorganise`): read by [`input_error`] like [`nested_error`].
pub(crate) fn nested_error_text(relative: &[String], message: &str) -> String {
    format!(
        "{NESTED}{}{NESTED}{message}",
        pointer(relative).unwrap_or_default()
    )
}

/// A text that is not JSON: no field to name, the line and column kept (ADR 0014 G5).
fn syntax_error(e: serde_json::Error) -> FairmarkError {
    FairmarkError::InvalidInput(e.to_string())
        .because(Reason::Syntax)
        .fix("send one well-formed JSON object")
}

fn input_error(e: serde_path_to_error::Error<serde_json::Error>) -> FairmarkError {
    let mut tokens = path_tokens(e.path());
    let inner = e.into_inner();
    if !inner.is_data() {
        return syntax_error(inner);
    }
    // The pointer replaces serde's position, which says where the reader stopped, not which
    // field is wrong.
    let full = inner.to_string();
    let suffix = format!(" at line {} column {}", inner.line(), inner.column());
    let mut msg = full.strip_suffix(&suffix).unwrap_or(&full);
    if let Some((relative, rest)) = msg.strip_prefix(NESTED).and_then(|m| m.split_once(NESTED)) {
        tokens.extend(relative.split('/').skip(1).map(unescape));
        msg = rest;
    }

    // serde_json puts the text of a custom error first; other errors start differently
    // ("unknown field", "missing field", "invalid type"), so a prefix match is exact.
    if let Some(rest) = msg.strip_prefix(dates::INVALID_DATE) {
        let detail = rest.trim_start_matches(": ").to_owned();
        return match pointer(&tokens) {
            Some(pointer) => FairmarkError::InvalidDateAt { pointer, detail },
            None => FairmarkError::InvalidDate(detail),
        };
    }
    // The sub-kind, the accepted values and the fix of the report (ADR 0014 G5); the message
    // is the one of ADR 0013.
    let (detail, reason, expected, fix) = if let Some(field) = quoted_field(msg, "missing field `")
    {
        // Reported at the object that lacks the field: the pointer is completed with it.
        tokens.push(field.to_owned());
        (
            format!("missing required field `{field}`; add it"),
            Some(Reason::Missing),
            Vec::new(),
            Some(format!("add the required field `{field}`")),
        )
    } else if let Some(field) = quoted_field(msg, "duplicate field `") {
        // serde refuses a repeated field of a struct at the object that holds it.
        tokens.push(field.to_owned());
        (
            format!("field `{field}` appears more than once; keep one"),
            Some(Reason::Duplicate),
            Vec::new(),
            Some(format!("keep one `{field}`")),
        )
    } else if let Some(field) = quoted_field(msg, "unknown field `") {
        // The path already ends with the unknown key.
        let allowed = expected_names(msg);
        let fix = unknown_field_fix(field, &allowed);
        (
            format!("{msg}; remove or rename it"),
            Some(Reason::UnknownField),
            allowed,
            Some(fix),
        )
    } else if msg == OBJECT_FOR_NUMBER {
        // The path ends with the first key of the object; the decimal is its parent.
        tokens.pop();
        (
            format!("invalid type: map, expected {DECIMAL_EXPECTED}"),
            Some(Reason::Type),
            vec![DECIMAL_EXPECTED.to_owned()],
            Some(format!("give {DECIMAL_EXPECTED}")),
        )
    } else {
        let detail = without_type_names(&msg.replace(RUST_DECIMAL_EXPECTED, DECIMAL_EXPECTED));
        let (reason, expected, fix) = classify(&detail);
        (detail, reason, expected, fix)
    };
    let error = match pointer(&tokens) {
        Some(pointer) => FairmarkError::InvalidInputAt { pointer, detail },
        None => FairmarkError::InvalidInput(detail),
    };
    let error = match reason {
        Some(reason) => error.because(reason),
        None => error,
    };
    let error = if expected.is_empty() {
        error
    } else {
        error.expected(expected)
    };
    match fix {
        Some(fix) => error.fix(fix),
        None => error,
    }
}

/// The sub-kind, accepted values and fix of a `serde` error other than a missing, repeated or
/// unknown field. An unknown enumeration value lists the values; a wrong type or value states
/// what is expected. A custom error raised by a type of the engine keeps its own text and has
/// no sub-kind here.
fn classify(detail: &str) -> (Option<Reason>, Vec<String>, Option<String>) {
    if detail.starts_with("unknown variant `") {
        return (
            Some(Reason::Enum),
            expected_names(detail),
            Some("use one of the values in `expected`".to_owned()),
        );
    }
    // Custom errors of the engine's own readers: their texts come from `CustomRefusal`.
    if let Some(refusal) = CustomRefusal::ALL
        .into_iter()
        .find(|r| detail.contains(r.text()))
    {
        return (
            Some(refusal.reason()),
            Vec::new(),
            Some(refusal.fix().to_owned()),
        );
    }
    let typed = ["invalid type: ", "invalid value: ", "invalid length "]
        .iter()
        .any(|p| detail.starts_with(p));
    if !typed {
        return (None, Vec::new(), None);
    }
    let reason = if detail.starts_with("invalid length ") {
        Reason::Length
    } else {
        Reason::Type
    };
    match detail.split_once(", expected ") {
        Some((_, form)) => (
            Some(reason),
            vec![form.to_owned()],
            Some(format!("give {form}")),
        ),
        None => (Some(reason), Vec::new(), None),
    }
}

/// The names `serde` lists after "expected": "expected `a`, `b` or `c`", "expected one of
/// `a`, `b`", "expected `a`". Empty when it lists none ("there are no fields").
fn expected_names(msg: &str) -> Vec<String> {
    let Some((_, list)) = msg.split_once("expected ") else {
        return Vec::new();
    };
    list.split('`')
        .skip(1)
        .step_by(2)
        .map(str::to_owned)
        .collect()
}

/// Every unknown key of the input, after `serde` refused the first (ADR 0014 G5, maintainer
/// instruction of 2026-11-09). `serde` stops at the first error, so the input is read again as
/// JSON and walked against the schema of its type: each key that an object closed by
/// `deny_unknown_fields` (`additionalProperties: false`) does not define is reported like the
/// first, with the message `serde` gives. The first error stays first. A choice (`oneOf`,
/// `anyOf`) is followed only where the branch is certain, as `serde` chooses it (the only
/// object branch, its `const` tag, the only branch the object fits, or the single key of an
/// externally tagged choice); elsewhere the walk stops, so it never reports a key that `serde`
/// would accept. Errors of other kinds (a wrong type, a missing field) are not looked for.
fn every_unknown_field(
    first: FairmarkError,
    schema: &schemars::Schema,
    text: &str,
) -> FairmarkError {
    let (Ok(value), Ok(schema)) = (
        serde_json::from_str::<Value>(text),
        serde_json::to_value(schema),
    ) else {
        return first;
    };
    let first_pointer = first.details().pointer;
    let mut found = crate::error::Found::new();
    found.push(first);
    for (tokens, allowed) in unknown_keys(&schema, &value) {
        let at = pointer(&tokens).unwrap_or_default();
        if Some(&at) == first_pointer.as_ref() {
            continue;
        }
        let key = tokens.last().cloned().unwrap_or_default();
        found.push(unknown_field_error(at, &key, allowed));
    }
    found.into_error()
}

/// The unknown keys of `value` under `schema`: the reference tokens of each pointer, with the
/// keys its object allows.
fn unknown_keys(schema: &Value, value: &Value) -> Vec<(Vec<String>, Vec<String>)> {
    let mut walk = UnknownKeys {
        root: schema,
        found: Vec::new(),
    };
    walk.node(schema, value, &mut Vec::new());
    walk.found
}

/// The `FM-E001` of an unknown key found by the walk, worded as `serde` words it.
fn unknown_field_error(pointer: String, key: &str, allowed: Vec<String>) -> FairmarkError {
    let names: Vec<String> = allowed.iter().map(|a| format!("`{a}`")).collect();
    let list = match names.as_slice() {
        [] => "there are no fields".to_owned(),
        [one] => format!("expected {one}"),
        [a, b] => format!("expected {a} or {b}"),
        many => format!("expected one of {}", many.join(", ")),
    };
    let fix = unknown_field_fix(key, &allowed);
    FairmarkError::InvalidInputAt {
        pointer,
        detail: format!("unknown field `{key}`, {list}; remove or rename it"),
    }
    .because(Reason::UnknownField)
    .expected(allowed)
    .fix(fix)
}

/// The fix of an unknown key: the closest allowed field when it is a likely typing error (an
/// edit distance of at most 2, and less than the length of the key), otherwise the choice.
fn unknown_field_fix(key: &str, allowed: &[String]) -> String {
    let closest = allowed
        .iter()
        .map(|a| (levenshtein(key, a), a))
        .filter(|(d, _)| *d <= 2 && *d < key.chars().count())
        .min_by_key(|(d, _)| *d);
    match closest {
        Some((_, a)) => format!("rename `{key}` to `{a}`"),
        None => format!("remove `{key}`, or rename it to one of the fields in `expected`"),
    }
}

/// The Levenshtein distance between two texts, by characters: insertions, deletions and
/// substitutions each count 1.
fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, cb) in b.iter().enumerate() {
            let above = row[j + 1];
            row[j + 1] = (above + 1)
                .min(row[j] + 1)
                .min(diagonal + usize::from(ca != *cb));
            diagonal = above;
        }
    }
    row[b.len()]
}

/// The walk of [`unknown_keys`].
struct UnknownKeys<'a> {
    root: &'a Value,
    found: Vec<(Vec<String>, Vec<String>)>,
}

/// What an object schema allows: its named properties, whether it is closed, and the schema of
/// any other key when it is open with one.
struct Shape {
    properties: serde_json::Map<String, Value>,
    closed: bool,
    extra: Option<Value>,
}

impl UnknownKeys<'_> {
    /// `schema` with its `$ref` (to `#/$defs/...`) followed.
    fn resolve(&self, schema: &Value) -> Value {
        let mut schema = schema;
        for _ in 0..64 {
            let Some(target) = schema
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|r| r.strip_prefix('#'))
                .and_then(|r| self.root.pointer(r))
            else {
                break;
            };
            schema = target;
        }
        schema.clone()
    }

    fn node(&mut self, schema: &Value, node: &Value, at: &mut Vec<String>) {
        let schema = self.resolve(schema);
        match node {
            Value::Object(map) => self.object(&schema, map, at),
            Value::Array(items) => {
                // The list itself, or the branch of an optional list that is an array.
                let list = if has_type(&schema, "array") {
                    Some(schema)
                } else {
                    self.branches(&schema)
                        .into_iter()
                        .find(|b| has_type(b, "array"))
                };
                let Some(list) = list else {
                    return;
                };
                let prefix = list.get("prefixItems").and_then(Value::as_array).cloned();
                let items_schema = list.get("items").cloned();
                for (i, item) in items.iter().enumerate() {
                    let sub = prefix
                        .as_ref()
                        .and_then(|p| p.get(i).cloned())
                        .or_else(|| items_schema.clone());
                    if let Some(sub) = sub {
                        at.push(i.to_string());
                        self.node(&sub, item, at);
                        at.pop();
                    }
                }
            }
            _ => {}
        }
    }

    /// The branches of `oneOf` and `anyOf`, resolved.
    fn branches(&self, schema: &Value) -> Vec<Value> {
        ["oneOf", "anyOf"]
            .iter()
            .filter_map(|k| schema.get(*k).and_then(Value::as_array))
            .flatten()
            .map(|b| self.resolve(b))
            .collect()
    }

    /// The shape of an object schema, with its `allOf` members merged.
    fn shape(&self, schema: &Value) -> Shape {
        let mut properties = schema
            .get("properties")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let additional = schema.get("additionalProperties");
        let mut closed = additional == Some(&Value::Bool(false));
        let mut extra = additional.filter(|a| a.is_object()).cloned();
        for member in schema
            .get("allOf")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let sub = self.shape(&self.resolve(member));
            properties.extend(sub.properties);
            closed |= sub.closed;
            extra = extra.or(sub.extra);
        }
        Shape {
            properties,
            closed,
            extra,
        }
    }

    /// The branch `serde` reads `map` with, when it is certain; `None` otherwise.
    fn choose(&self, branches: &[Value], map: &serde_json::Map<String, Value>) -> Option<Value> {
        let objects: Vec<&Value> = branches
            .iter()
            .filter(|b| has_type(b, "object") || b.get("properties").is_some())
            .collect();
        let only = |candidates: Vec<&Value>| match candidates.as_slice() {
            [one] => Some((*one).clone()),
            _ => None,
        };
        if objects.len() == 1 {
            return only(objects);
        }
        // An internally tagged choice: the branch whose `const` properties the object carries.
        let tagged: Vec<&Value> = objects
            .iter()
            .copied()
            .filter(|b| {
                let tags: Vec<(&String, &Value)> = b
                    .get("properties")
                    .and_then(Value::as_object)
                    .into_iter()
                    .flatten()
                    .filter_map(|(k, v)| v.get("const").map(|c| (k, c)))
                    .collect();
                !tags.is_empty() && tags.iter().all(|(k, c)| map.get(*k) == Some(*c))
            })
            .collect();
        if tagged.len() == 1 {
            return only(tagged);
        }
        // The only branch with required keys, all present, whose names cover every key.
        let fitting: Vec<&Value> = objects
            .iter()
            .copied()
            .filter(|b| {
                let required: Vec<&str> = b
                    .get("required")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .collect();
                let shape = self.shape(b);
                !required.is_empty()
                    && required.iter().all(|r| map.contains_key(*r))
                    && (!shape.closed || map.keys().all(|k| shape.properties.contains_key(k)))
            })
            .collect();
        if fitting.len() == 1 {
            return only(fitting);
        }
        // An externally tagged choice: each branch requires one key, and the object names one.
        let named: Vec<&Value> = objects
            .iter()
            .copied()
            .filter(|b| match b.get("required").and_then(Value::as_array) {
                Some(r) if r.len() == 1 => r[0].as_str().is_some_and(|k| map.contains_key(k)),
                _ => false,
            })
            .collect();
        only(named)
    }

    fn object(
        &mut self,
        schema: &Value,
        map: &serde_json::Map<String, Value>,
        at: &mut Vec<String>,
    ) {
        let mut shape = self.shape(schema);
        let branches = self.branches(schema);
        if !branches.is_empty() {
            let Some(branch) = self.choose(&branches, map) else {
                return;
            };
            let sub = self.shape(&branch);
            shape.properties.extend(sub.properties);
            shape.closed |= sub.closed;
            shape.extra = shape.extra.or(sub.extra);
        }
        let allowed: Vec<String> = shape.properties.keys().cloned().collect();
        for (key, value) in map {
            at.push(key.clone());
            if let Some(sub) = shape.properties.get(key) {
                self.node(sub, value, at);
            } else if shape.closed {
                self.found.push((at.clone(), allowed.clone()));
            } else if let Some(sub) = &shape.extra {
                self.node(sub, value, at);
            }
            at.pop();
        }
    }
}

/// True when the schema's `type` is or includes `name`.
fn has_type(schema: &Value, name: &str) -> bool {
    match schema.get("type") {
        Some(Value::String(t)) => t == name,
        Some(Value::Array(ts)) => ts.iter().any(|t| t == name),
        _ => false,
    }
}

/// serde names the Rust type it expected ("expected struct Stage"); the input knows objects,
/// not Rust types, so the name becomes "an object". "struct variant" is left as it is.
fn without_type_names(msg: &str) -> String {
    const STRUCT: &str = "expected struct ";
    let mut out = String::with_capacity(msg.len());
    let mut rest = msg;
    while let Some(at) = rest.find(STRUCT) {
        let after = &rest[at + STRUCT.len()..];
        let name_len = after
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
            .unwrap_or(after.len());
        out.push_str(&rest[..at]);
        if name_len == 0 || &after[..name_len] == "variant" {
            out.push_str(STRUCT);
        } else {
            out.push_str("expected an object");
        }
        rest = &after[name_len..];
    }
    out.push_str(rest);
    out
}

/// One reference token of an RFC 6901 pointer, unescaped.
fn unescape(token: &str) -> String {
    token.replace("~1", "/").replace("~0", "~")
}

/// The field name in a serde message that starts with `prefix` and quotes the name in
/// backticks, for example "missing field `rate`".
fn quoted_field<'a>(msg: &'a str, prefix: &str) -> Option<&'a str> {
    msg.strip_prefix(prefix)?
        .split_once('`')
        .map(|(field, _)| field)
}

/// Reference tokens of a serde path, unescaped. An unknown segment (a key that is not a
/// string, which JSON cannot produce) ends the path at its parent, still a valid location.
fn path_tokens(path: &serde_path_to_error::Path) -> Vec<String> {
    use serde_path_to_error::Segment;
    let mut tokens = Vec::new();
    for segment in path.iter() {
        match segment {
            Segment::Seq { index } => tokens.push(index.to_string()),
            Segment::Map { key } if key == NUMBER_TOKEN => {}
            Segment::Map { key } => tokens.push(key.clone()),
            Segment::Enum { variant } => tokens.push(variant.clone()),
            Segment::Unknown => break,
        }
    }
    tokens
}

/// RFC 6901 pointer of the tokens (`~` escaped as `~0`, `/` as `~1`); `None` for the whole
/// input, whose pointer is empty.
fn pointer(tokens: &[String]) -> Option<String> {
    if tokens.is_empty() {
        return None;
    }
    Some(
        tokens
            .iter()
            .map(|t| format!("/{}", t.replace('~', "~0").replace('/', "~1")))
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Review of #279: each custom refusal is recognised from the text its producer writes,
    /// and only by its own marker.
    #[test]
    fn custom_refusals_round_trip() {
        for r in CustomRefusal::ALL {
            let written = format!("`x` {}", r.text());
            let (reason, expected, fix) = classify(&written);
            assert_eq!(reason, Some(r.reason()), "{r:?}");
            assert!(expected.is_empty());
            assert_eq!(fix.as_deref(), Some(r.fix()));
            for other in CustomRefusal::ALL.into_iter().filter(|o| *o != r) {
                assert!(!r.text().contains(other.text()), "{r:?} contains {other:?}");
            }
        }
        let err = crate::registry::run(
            "dcf.fcff",
            r#"{"stages": [{"flows": [1]}], "wacc": "0.1", "timing": null}"#,
        )
        .unwrap_err();
        assert_eq!(err.details().reason, Some(Reason::Type));
    }

    /// Maintainer instruction of 2026-11-09 (item A): the walk for unknown keys never reports a
    /// key that `serde` accepts. On every golden input that is read without an unknown field,
    /// it finds none; with a key added at the root, it finds that key only.
    #[test]
    fn the_unknown_key_walk_has_no_false_positives() {
        fn files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    files(&path, out);
                } else if path.extension().is_some_and(|e| e == "yaml") {
                    out.push(path);
                }
            }
        }
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
        let mut all = Vec::new();
        files(&root, &mut all);
        let mut checked = 0;
        for file in all {
            let case: Value =
                serde_norway::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            let id = case["method"].as_str().unwrap();
            let input = &case["input"];
            if let Err(e) = crate::registry::run(id, &input.to_string()) {
                if e.details().reason == Some(Reason::UnknownField) {
                    continue;
                }
            }
            let schema = crate::registry::input_schema(id).unwrap();
            assert!(
                unknown_keys(&schema, input).is_empty(),
                "{}: {:?}",
                file.display(),
                unknown_keys(&schema, input)
            );
            let mut extra = input.clone();
            if let Some(map) = extra.as_object_mut() {
                map.insert("zz_unknown".to_owned(), Value::from(1));
                let keys: Vec<Vec<String>> = unknown_keys(&schema, &extra)
                    .into_iter()
                    .map(|(k, _)| k)
                    .collect();
                assert_eq!(keys, [vec!["zz_unknown".to_owned()]], "{}", file.display());
            }
            checked += 1;
        }
        assert!(checked > 600, "{checked}");
    }

    #[test]
    fn levenshtein_counts_edits() {
        assert_eq!(levenshtein("wac", "wacc"), 1);
        assert_eq!(levenshtein("growht", "growth"), 2);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("same", "same"), 0);
    }
}
