//! Reading a method input (ADR 0013, ADR 0014 G5): the typed input, or every input error the
//! reading finds, each named by its JSON pointer with its sub-kind, the accepted values and a
//! fix. `serde` stops at the first error; the text is then read as JSON and walked against the
//! schema of the input type to find every unknown key, repeated key and positional array.

use schemars::{schema_for, JsonSchema};
use serde_json::Value;

use crate::dates;
use crate::error::{CustomRefusal, FairmarkError, Reason, Result};

/// Reads a method input. Every failure names the field by its JSON pointer (issue #174,
/// SPEC-0001 R27, ADR 0014 G5): `FM-E001`, or `FM-E009` for an invalid date. Only a text that
/// is not JSON (a syntax error, a truncated input, trailing characters) keeps serde's line and
/// column, because it has no field to name. An input that `serde` reads is still refused when
/// it repeats a key or gives an object as a positional array: `serde` would read it in another
/// way than written.
///
/// `analysis`: the input is that of an analysis, whose `base_input` is the input of the method
/// named by `method`. When the analysis input cannot be read, `base_input` is still checked
/// by that method, and its input errors are reported under `/base_input` with the others.
pub(crate) fn parse<T: serde::de::DeserializeOwned + JsonSchema>(
    input_json: &str,
    analysis: bool,
) -> Result<T> {
    let mut de = serde_json::Deserializer::from_str(input_json);
    match serde_path_to_error::deserialize(&mut de) {
        Err(e) => {
            let first = input_error(e);
            Err(
                match reading_errors(Some(first), &schema_for!(T), input_json, analysis) {
                    Err(e) => e,
                    Ok(()) => FairmarkError::Internal("a refused input recorded no error"),
                },
            )
        }
        Ok(input) => {
            // `serde_json::from_str` does the same: nothing but whitespace may follow the value.
            de.end().map_err(syntax_error)?;
            reading_errors(None, &schema_for!(T), input_json, false)?;
            Ok(input)
        }
    }
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

/// The first error `serde` raised, with the reference tokens of its pointer.
fn input_error(e: serde_path_to_error::Error<serde_json::Error>) -> (FairmarkError, Vec<String>) {
    let mut tokens = path_tokens(e.path());
    let inner = e.into_inner();
    if !inner.is_data() {
        return (syntax_error(inner), tokens);
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
        let error = match pointer(&tokens) {
            Some(pointer) => FairmarkError::InvalidDateAt { pointer, detail },
            None => FairmarkError::InvalidDate(detail),
        };
        return (error, tokens);
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
        // The whole input: its pointer is the empty pointer, the root.
        None => FairmarkError::InvalidInput(detail).located(""),
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
    let error = match fix {
        Some(fix) => error.fix(fix),
        None => error,
    };
    (error, tokens)
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
            refusal.expected(),
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
        Some((_, form)) => {
            let form = plain(form);
            // A negative number for a count is a value outside the domain, not a wrong type.
            let reason = if detail.starts_with("invalid value: integer `-") && form == UNSIGNED {
                Reason::Range
            } else {
                reason
            };
            (
                Some(reason),
                vec![form.clone()],
                Some(format!("give {form}")),
            )
        }
        None => (Some(reason), Vec::new(), None),
    }
}

/// What a count of the input accepts, in the words of an analyst.
const UNSIGNED: &str = "a whole number of 0 or more";

/// The form `serde` expects, in plain words: no Rust type names (ADR 0014 G5).
fn plain(form: &str) -> String {
    match form {
        "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => return UNSIGNED.to_owned(),
        "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => return "a whole number".to_owned(),
        "f32" | "f64" => return "a number".to_owned(),
        _ => {}
    }
    let words: Vec<&str> = form.split(' ').collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < words.len() {
        match words[i] {
            "sequence" => out.push("list".to_owned()),
            "map" => out.push("object".to_owned()),
            "tuple" if words.get(i + 1) == Some(&"of") && words.get(i + 2) == Some(&"size") => {
                out.push(format!(
                    "list of {} values",
                    words.get(i + 3).unwrap_or(&"")
                ));
                i += 3;
            }
            w => out.push(w.to_owned()),
        }
        i += 1;
    }
    out.join(" ").replace("a object", "an object")
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

/// Every input error the reading finds (ADR 0014 G5). `first` is the error `serde` raised, if
/// it raised one; the input is then read again as JSON and walked against the schema of its
/// type, to report every unknown key, every repeated key and every object given as a
/// positional array, each like the first. The first error stays first. A choice (`oneOf`,
/// `anyOf`) is followed only where the branch is certain, as `serde` chooses it (the only
/// object branch, its `const` tag, the only branch the object fits, or the single key of an
/// externally tagged choice); elsewhere the walk stops, so it never reports a key that `serde`
/// would accept. A text that is not JSON is reported alone, as a syntax error.
fn reading_errors(
    first: Option<(FairmarkError, Vec<String>)>,
    schema: &schemars::Schema,
    text: &str,
    analysis: bool,
) -> Result<()> {
    let alone = |first: Option<(FairmarkError, Vec<String>)>| first.map_or(Ok(()), |(e, _)| Err(e));
    let (Ok(value), Ok(schema)) = (
        serde_json::from_str::<Value>(text),
        serde_json::to_value(schema),
    ) else {
        return alone(first);
    };
    // A positional input: nothing else can be located in it.
    if value.is_array() {
        return Err(positional_error(&[]));
    }
    let mut walk = UnknownKeys {
        root: &schema,
        found: Vec::new(),
    };
    walk.node(&schema, &value, &mut Vec::new());
    let duplicates = duplicate_keys(text);
    let mut found = crate::error::Found::new();
    let mut reported: Vec<(Option<String>, Option<Reason>)> = Vec::new();
    if let Some((error, tokens)) = first {
        let error = refine(error, &tokens, &value, &walk, &duplicates);
        let d = error.details();
        reported.push((d.pointer, d.reason));
        found.push(error);
    }
    let mut extra = Vec::new();
    for finding in std::mem::take(&mut walk.found) {
        extra.push(match finding {
            Finding::Unknown {
                tokens,
                allowed,
                listed,
            } => {
                let key = tokens.last().cloned().unwrap_or_default();
                unknown_field_error(pointer(&tokens).unwrap_or_default(), &key, allowed, &listed)
            }
            Finding::Positional { tokens } => positional_error(&tokens),
        });
    }
    extra.extend(duplicates.iter().map(|t| duplicate_error(t)));
    if analysis {
        extra.extend(base_input_errors(&value));
    }
    for error in extra {
        let d = error.details();
        let key = (d.pointer, d.reason);
        if !reported.contains(&key) {
            reported.push(key);
            found.push(error);
        }
    }
    found.finish()
}

/// The input errors of `base_input` of an analysis input that cannot be read, checked by the
/// method named in `method` and located under `/base_input`; none when the method is unknown or
/// is itself an analysis (refused by the analysis). Errors of the computation are left out.
fn base_input_errors(value: &Value) -> Vec<FairmarkError> {
    let (Some(id), Some(base)) = (
        value.get("method").and_then(Value::as_str),
        value.get("base_input").filter(|b| b.is_object()),
    ) else {
        return Vec::new();
    };
    if crate::registry::method(id).is_err() || id.starts_with("analysis.") {
        return Vec::new();
    }
    match crate::registry::evaluate(id, base) {
        Ok(_) => Vec::new(),
        Err(e) => e
            .under("/base_input")
            .into_errors()
            .into_iter()
            .filter(FairmarkError::is_input_error)
            .collect(),
    }
}

/// The first error of `serde`, completed with what the JSON and the schema say about it: a
/// value of the wrong kind for a choice (which `serde_json` reports as a syntax error), an
/// unknown key of a single-key choice, an extra key, the key a repeated key is, the fields an
/// object accepts after an unknown one, and the values a closed field accepts.
fn refine(
    error: FairmarkError,
    tokens: &[String],
    value: &Value,
    walk: &UnknownKeys<'_>,
    duplicates: &[Vec<String>],
) -> FairmarkError {
    let d = error.details();
    if d.reason == Some(Reason::Syntax) {
        // A repeated key where the reader stopped is what it met (the JSON value keeps one).
        let near = &tokens[..tokens.len().saturating_sub(1)];
        if let Some(t) = duplicates.iter().find(|t| t.starts_with(near)) {
            return duplicate_error(t);
        }
        return retype(tokens, value, walk);
    }
    let at = d.pointer.clone().unwrap_or_default();
    let at_tokens = tokens_of(&at);
    let node = value.pointer(&at);
    let message = error.to_string();
    match d.reason {
        // A key of a single-key choice that is not one of its keys is an unknown field at the
        // key, like any other unknown key (audit 9).
        Some(Reason::Enum) => {
            if let Some(variant) = quoted_field_in(&message, "unknown variant `") {
                let parent = at_tokens.split_last().map(|(_, p)| p).unwrap_or(&[]);
                let key_at = match node {
                    Some(Value::Object(map)) if map.contains_key(variant) => {
                        Some(join(&at_tokens, variant))
                    }
                    _ if at_tokens.last().map(String::as_str) == Some(variant)
                        && value
                            .pointer(&pointer(parent).unwrap_or_default())
                            .is_some_and(Value::is_object) =>
                    {
                        Some(at_tokens.clone())
                    }
                    _ => None,
                };
                if let Some(key_at) = key_at {
                    let fix = unknown_field_fix(variant, &d.expected);
                    return error
                        .at(pointer(&key_at).unwrap_or_default(), Reason::UnknownField)
                        .fix(fix);
                }
            }
            if d.expected.is_empty() {
                let given = node.and_then(Value::as_str);
                let values: Vec<String> = walk
                    .schema_at(&at_tokens, value)
                    .and_then(|s| walk.choice_values(&s))
                    .unwrap_or_default()
                    .into_iter()
                    .filter(|v| Some(v.as_str()) != given)
                    .collect();
                if !values.is_empty() {
                    return error.expected(values);
                }
            }
            error
        }
        // Two keys or more where one is expected: the extra key is named.
        Some(Reason::Length) => match node {
            Some(Value::Object(map)) if map.len() >= 2 => {
                let second = map.keys().nth(1).cloned().unwrap_or_default();
                error.at(
                    pointer(&join(&at_tokens, &second)).unwrap_or_default(),
                    Reason::Length,
                )
            }
            _ => error,
        },
        // A repeated key reported at its object is named at the key.
        Some(Reason::Duplicate) => match duplicates.iter().find(|t| {
            t.split_last()
                .is_some_and(|(_, p)| p == at_tokens.as_slice())
        }) {
            Some(t) => error.at(pointer(t).unwrap_or_default(), Reason::Duplicate),
            None => error,
        },
        // The fields the object accepts, from the schema: every field, its tag included.
        Some(Reason::UnknownField) => {
            let key = at_tokens.last().cloned().unwrap_or_default();
            let parent = &at_tokens[..at_tokens.len().saturating_sub(1)];
            match walk.object_fields(parent, value) {
                Some((allowed, _)) if !allowed.is_empty() => {
                    let fix = unknown_field_fix(&key, &allowed);
                    error.expected(allowed).fix(fix)
                }
                _ => error,
            }
        }
        _ => error,
    }
}

/// A value of the wrong kind where the reader expected a choice, which `serde_json` reports as
/// a syntax error with a line and column although the text is JSON: the error at the field,
/// with the values of the choice, or a plain wrong type.
fn retype(tokens: &[String], value: &Value, walk: &UnknownKeys<'_>) -> FairmarkError {
    for depth in [tokens.len(), tokens.len().saturating_sub(1)] {
        let at = &tokens[..depth];
        let (Some(node), Some(schema)) = (
            value.pointer(&pointer(at).unwrap_or_default()),
            walk.schema_at(at, value),
        ) else {
            continue;
        };
        if let Some(choices) = walk.choice_values(&schema) {
            return choice_error(at, node, choices);
        }
    }
    let node = value
        .pointer(&pointer(tokens).unwrap_or_default())
        .unwrap_or(&Value::Null);
    at_error(tokens, format!("invalid type: {}", unexpected(node))).because(Reason::Type)
}

/// The error of a choice given the wrong value: a value of another kind, or an object that does
/// not name exactly one of its keys.
fn choice_error(at: &[String], node: &Value, choices: Vec<String>) -> FairmarkError {
    let list = one_of(&choices);
    match node {
        Value::Object(map) => {
            if map.is_empty() {
                return at_error(
                    at,
                    format!("an empty object is given where one key is expected, {list}"),
                )
                .because(Reason::Length)
                .expected(choices)
                .fix("give exactly one of the keys in `expected`");
            }
            if let Some(key) = map.keys().find(|k| !choices.contains(k)) {
                return unknown_field_error(
                    pointer(&join(at, key)).unwrap_or_default(),
                    key,
                    choices.clone(),
                    &choices,
                );
            }
            let second = map.keys().nth(1).cloned().unwrap_or_default();
            at_error(
                &join(at, &second),
                format!(
                    "{} keys are given where one key is expected, {list}; keep one",
                    map.len()
                ),
            )
            .because(Reason::Length)
            .expected(choices)
            .fix("keep one key")
        }
        other => at_error(at, format!("invalid type: {}, {list}", unexpected(other)))
            .because(Reason::Enum)
            .expected(choices)
            .fix("use one of the values in `expected`"),
    }
}

/// "expected `a`", "expected `a` or `b`", "expected one of `a`, `b`, `c`", as `serde` words it.
fn one_of(values: &[String]) -> String {
    let names: Vec<String> = values.iter().map(|a| format!("`{a}`")).collect();
    match names.as_slice() {
        [] => "there are no fields".to_owned(),
        [one] => format!("expected {one}"),
        [a, b] => format!("expected {a} or {b}"),
        many => format!("expected one of {}", many.join(", ")),
    }
}

/// A JSON value as the messages name it.
fn unexpected(node: &Value) -> String {
    match node {
        Value::Null => "null".to_owned(),
        Value::Bool(b) => format!("boolean `{b}`"),
        Value::Number(n) if n.is_i64() || n.is_u64() => format!("integer `{n}`"),
        Value::Number(n) => format!("number `{n}`"),
        Value::String(s) => format!("string {s:?}"),
        Value::Array(_) => "a list".to_owned(),
        Value::Object(_) => "an object".to_owned(),
    }
}

/// `FM-E001` at the pointer of `tokens`: `invalid input at …`, or at the root `invalid input:`.
fn at_error(tokens: &[String], detail: String) -> FairmarkError {
    match pointer(tokens) {
        Some(pointer) => FairmarkError::InvalidInputAt { pointer, detail },
        None => FairmarkError::InvalidInput(detail).located(""),
    }
}

/// An object given as a positional array: `serde` reads its fields by position, which the
/// contract does not offer (audit 15, issue #201).
fn positional_error(tokens: &[String]) -> FairmarkError {
    at_error(
        tokens,
        "an array is given where an object is expected; give the fields by name".to_owned(),
    )
    .because(Reason::Type)
    .expected(["an object with its fields by name"])
    .fix("give the fields by name in an object")
}

/// A key given twice in one object, never read with its last value (audit 14).
fn duplicate_error(tokens: &[String]) -> FairmarkError {
    let key = tokens.last().cloned().unwrap_or_default();
    at_error(
        tokens,
        format!("field `{key}` appears more than once; keep one"),
    )
    .because(Reason::Duplicate)
    .fix(format!("keep one `{key}`"))
}

/// `tokens` with `key` appended.
fn join(tokens: &[String], key: &str) -> Vec<String> {
    let mut t = tokens.to_vec();
    t.push(key.to_owned());
    t
}

/// The reference tokens of an RFC 6901 pointer, unescaped.
fn tokens_of(pointer: &str) -> Vec<String> {
    pointer.split('/').skip(1).map(unescape).collect()
}

/// The quoted name after `prefix` anywhere in `msg`.
fn quoted_field_in<'a>(msg: &'a str, prefix: &str) -> Option<&'a str> {
    let at = msg.find(prefix)?;
    quoted_field(&msg[at..], prefix)
}

/// The pointers of every key repeated in an object of the text, at any depth.
fn duplicate_keys(text: &str) -> Vec<Vec<String>> {
    use serde::de::DeserializeSeed;
    let mut found = Vec::new();
    let mut de = serde_json::Deserializer::from_str(text);
    let _ = DuplicateScan {
        path: Vec::new(),
        found: &mut found,
    }
    .deserialize(&mut de);
    found
}

/// Reads any JSON value and records the keys repeated in its objects.
struct DuplicateScan<'a> {
    path: Vec<String>,
    found: &'a mut Vec<Vec<String>>,
}

impl<'de> serde::de::DeserializeSeed<'de> for DuplicateScan<'_> {
    type Value = ();

    fn deserialize<D: serde::Deserializer<'de>>(self, d: D) -> std::result::Result<(), D::Error> {
        d.deserialize_any(self)
    }
}

impl<'de> serde::de::Visitor<'de> for DuplicateScan<'_> {
    type Value = ();

    fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str("any JSON value")
    }

    fn visit_bool<E>(self, _: bool) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_i64<E>(self, _: i64) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_u64<E>(self, _: u64) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_f64<E>(self, _: f64) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_str<E>(self, _: &str) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_unit<E>(self) -> std::result::Result<(), E> {
        Ok(())
    }
    fn visit_none<E>(self) -> std::result::Result<(), E> {
        Ok(())
    }

    fn visit_seq<A: serde::de::SeqAccess<'de>>(
        self,
        mut seq: A,
    ) -> std::result::Result<(), A::Error> {
        let found = self.found;
        let mut i = 0usize;
        loop {
            let seed = DuplicateScan {
                path: join(&self.path, &i.to_string()),
                found: &mut *found,
            };
            if seq.next_element_seed(seed)?.is_none() {
                return Ok(());
            }
            i += 1;
        }
    }

    fn visit_map<A: serde::de::MapAccess<'de>>(
        self,
        mut map: A,
    ) -> std::result::Result<(), A::Error> {
        let found = self.found;
        let mut seen = std::collections::BTreeSet::new();
        while let Some(key) = map.next_key::<String>()? {
            if key == NUMBER_TOKEN {
                map.next_value::<serde::de::IgnoredAny>()?;
                continue;
            }
            let at = join(&self.path, &key);
            if !seen.insert(key) {
                found.push(at.clone());
            }
            map.next_value_seed(DuplicateScan {
                path: at,
                found: &mut *found,
            })?;
        }
        Ok(())
    }
}

/// The `FM-E001` of an unknown key found by the walk, worded as `serde` words it.
/// The message lists `listed`, the fields `serde` names (an internally tagged object leaves out
/// its tag); `expected` holds every field the object accepts.
fn unknown_field_error(
    pointer: String,
    key: &str,
    allowed: Vec<String>,
    listed: &[String],
) -> FairmarkError {
    let list = one_of(listed);
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
        .map(|a| (distance(key, a), a))
        .filter(|(d, _)| *d <= 2 && *d < key.chars().count())
        .min_by_key(|(d, _)| *d);
    match closest {
        Some((_, a)) => format!("rename `{key}` to `{a}`"),
        None => format!("remove `{key}`, or rename it to one of the fields in `expected`"),
    }
}

/// True when the field at `pointer` of `value` takes a whole number under `schema` (its type,
/// or the type of every branch but `null`, is `integer`).
pub(crate) fn takes_whole_number(schema: &Value, value: &Value, pointer: &str) -> bool {
    let walk = UnknownKeys {
        root: schema,
        found: Vec::new(),
    };
    let Some(field) = walk.schema_at(&tokens_of(pointer), value) else {
        return false;
    };
    if has_type(&field, "integer") {
        return true;
    }
    let branches = walk.branches(&field);
    branches.iter().any(|b| has_type(b, "integer"))
        && branches
            .iter()
            .all(|b| has_type(b, "integer") || has_type(b, "null"))
}

/// True when `key` is a likely typing error for `field`: within the distance of a suggestion.
pub(crate) fn is_likely_typo(key: &str, field: &str) -> bool {
    let d = distance(key, field);
    d > 0 && d <= 2 && d < key.chars().count()
}

/// The distance of a suggestion: the Levenshtein distance ignoring case, so that `Rate` is one
/// rename from `rate` (a difference of case alone counts 1).
fn distance(a: &str, b: &str) -> usize {
    let d = levenshtein(&a.to_lowercase(), &b.to_lowercase());
    if d == 0 && a != b {
        1
    } else {
        d
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
    found: Vec<Finding>,
}

/// What the walk finds.
enum Finding {
    /// A key its object does not define, with every field the object accepts and the fields
    /// `serde` lists (without the tag of an internally tagged object).
    Unknown {
        tokens: Vec<String>,
        allowed: Vec<String>,
        listed: Vec<String>,
    },
    /// An object given as an array.
    Positional { tokens: Vec<String> },
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
                let Some(list) = self.list_schema(&schema) else {
                    if self.object_like(&schema) {
                        self.found.push(Finding::Positional { tokens: at.clone() });
                    }
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

    /// The shape of the object `map` under `schema`, with the branch of a choice when it is
    /// certain; `None` when it is not.
    fn object_shape(&self, schema: &Value, map: &serde_json::Map<String, Value>) -> Option<Shape> {
        let mut shape = self.shape(schema);
        let branches = self.branches(schema);
        if !branches.is_empty() {
            let branch = self.choose(&branches, map)?;
            let sub = self.shape(&branch);
            shape.properties.extend(sub.properties);
            shape.closed |= sub.closed;
            shape.extra = shape.extra.or(sub.extra);
        }
        Some(shape)
    }

    fn object(
        &mut self,
        schema: &Value,
        map: &serde_json::Map<String, Value>,
        at: &mut Vec<String>,
    ) {
        let Some(shape) = self.object_shape(schema, map) else {
            return;
        };
        let (allowed, listed) = fields(&shape);
        for (key, value) in map {
            at.push(key.clone());
            if let Some(sub) = shape.properties.get(key) {
                self.node(sub, value, at);
            } else if shape.closed {
                self.found.push(Finding::Unknown {
                    tokens: at.clone(),
                    allowed: allowed.clone(),
                    listed: listed.clone(),
                });
            } else if let Some(sub) = &shape.extra {
                self.node(sub, value, at);
            }
            at.pop();
        }
    }

    /// The schema of the value at `tokens`, following the input where a choice must be made.
    fn schema_at(&self, tokens: &[String], value: &Value) -> Option<Value> {
        let mut schema = self.resolve(self.root);
        let mut node = value;
        for t in tokens {
            match node {
                Value::Object(map) => {
                    let shape = self.object_shape(&schema, map).or_else(|| {
                        // An uncertain choice: the branch that names the key.
                        self.branches(&schema)
                            .into_iter()
                            .map(|b| self.shape(&b))
                            .find(|sh| sh.properties.contains_key(t))
                    })?;
                    let sub = shape.properties.get(t).cloned().or(shape.extra)?;
                    schema = self.resolve(&sub);
                    node = map.get(t)?;
                }
                Value::Array(items) => {
                    let i: usize = t.parse().ok()?;
                    let list = self.list_schema(&schema)?;
                    let sub = list
                        .get("prefixItems")
                        .and_then(|p| p.get(i))
                        .or_else(|| list.get("items"))?
                        .clone();
                    schema = self.resolve(&sub);
                    node = items.get(i)?;
                }
                _ => return None,
            }
        }
        Some(schema)
    }

    /// Every field the object at `tokens` accepts, and the fields `serde` lists for it.
    fn object_fields(
        &self,
        tokens: &[String],
        value: &Value,
    ) -> Option<(Vec<String>, Vec<String>)> {
        let schema = self.schema_at(tokens, value)?;
        let map = value
            .pointer(&pointer(tokens).unwrap_or_default())?
            .as_object()?;
        let shape = self.object_shape(&schema, map)?;
        Some(fields(&shape))
    }

    /// The values of a closed choice: an enumeration, constants, or the keys of an externally
    /// tagged choice; `None` when the schema is not such a choice.
    fn choice_values(&self, schema: &Value) -> Option<Vec<String>> {
        let schema = self.resolve(schema);
        if let Some(values) = schema.get("enum").and_then(Value::as_array) {
            let values: Vec<String> = values
                .iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect();
            return (!values.is_empty()).then_some(values);
        }
        if let Some(c) = schema.get("const").and_then(Value::as_str) {
            return Some(vec![c.to_owned()]);
        }
        let branches = self.branches(&schema);
        if branches.is_empty() {
            return None;
        }
        let mut values = Vec::new();
        for b in branches {
            if has_type(&b, "null") && b.get("type").is_some_and(Value::is_string) {
                continue;
            }
            let required: Vec<&str> = b
                .get("required")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
                .collect();
            if let [key] = required.as_slice() {
                if b.get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(|p| p.len() == 1 && p.contains_key(*key))
                {
                    values.push((*key).to_owned());
                    continue;
                }
            }
            values.extend(self.choice_values(&b)?);
        }
        (!values.is_empty()).then_some(values)
    }

    /// The schema of a list: the schema itself, or the branch of an optional list.
    fn list_schema(&self, schema: &Value) -> Option<Value> {
        if has_type(schema, "array") {
            return Some(schema.clone());
        }
        self.branches(schema)
            .into_iter()
            .find(|b| has_type(b, "array"))
    }

    /// True for a schema that takes an object and never a list.
    fn object_like(&self, schema: &Value) -> bool {
        let schema = self.resolve(schema);
        if self.list_schema(&schema).is_some() {
            return false;
        }
        if schema.get("properties").is_some() || has_type(&schema, "object") {
            return true;
        }
        let branches = self.branches(&schema);
        !branches.is_empty()
            && branches
                .iter()
                .all(|b| has_type(b, "null") || self.object_like(b))
            && branches.iter().any(|b| self.object_like(b))
    }
}

/// Every field a shape accepts, and the fields `serde` lists: without the `const` tags of an
/// internally tagged object, which `serde` reads before the other fields.
fn fields(shape: &Shape) -> (Vec<String>, Vec<String>) {
    let allowed: Vec<String> = shape.properties.keys().cloned().collect();
    let listed = shape
        .properties
        .iter()
        .filter(|(_, v)| v.get("const").is_none())
        .map(|(k, _)| k.clone())
        .collect();
    (allowed, listed)
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

    /// The pointers (as tokens) of every finding of the walk: unknown keys and positional
    /// arrays.
    fn unknown_keys(schema: &Value, value: &Value) -> Vec<(Vec<String>, ())> {
        let mut walk = UnknownKeys {
            root: schema,
            found: Vec::new(),
        };
        walk.node(schema, value, &mut Vec::new());
        walk.found
            .into_iter()
            .map(|f| match f {
                Finding::Unknown { tokens, .. } | Finding::Positional { tokens } => (tokens, ()),
            })
            .collect()
    }

    /// Review of #279: each custom refusal is recognised from the text its producer writes,
    /// and only by its own marker.
    #[test]
    fn custom_refusals_round_trip() {
        for r in CustomRefusal::ALL {
            let written = format!("`x` {}", r.text());
            let (reason, expected, fix) = classify(&written);
            assert_eq!(reason, Some(r.reason()), "{r:?}");
            assert_eq!(expected, r.expected());
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
