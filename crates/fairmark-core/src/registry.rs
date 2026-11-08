//! The method registry: the single source of truth for every adapter.
//!
//! Python, the CLI, the MCP server, function-calling tool definitions and the
//! documentation are all generated from this catalogue and its JSON Schemas.

use schemars::{schema_for, JsonSchema};
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::error::{FairmarkError, Result};
use crate::explain::{Meta, Valuation};
use crate::method::Method;
use crate::{
    analysis, coc, dates, dcf, loans, rates, ratios, series, statements, tv, tvm, ENGINE_VERSION,
};

/// Public description of a method, used to build agent tools.
#[derive(Debug, Clone, Serialize)]
pub struct MethodInfo {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub version: &'static str,
    /// `available` or `preview`; see [`Status`].
    pub status: Status,
}

/// How far a method has been verified (ADR 0012). Defined, with the promotion rule, in
/// `docs/en/project/releases.md`; both statuses follow the versioning rules of ADR 0009.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    /// Meets the promotion rule: approved spec, independent audit and review recorded.
    Available,
    /// Implemented, tested and documented, not yet promoted: tell users so.
    Preview,
}

/// Methods promoted to available (ADR 0012). Every other method is preview, so a new method
/// is preview until it is listed here; `scripts/method_status.py` checks each entry against
/// the evidence in `specs/method-evidence.yaml`.
const AVAILABLE: &[&str] = &[
    "dates.year_fraction",
    "rates.convert",
    "rates.real_nominal",
    "tvm.npv",
    "tvm.present_value",
    "tvm.xnpv",
    "tvm.irr",
    "tvm.xirr",
    "tvm.mirr",
    "tvm.annuity",
    "tvm.perpetuity",
    "tvm.future_value",
    "loans.instalment",
    "loans.number_of_instalments",
    "loans.amortisation_schedule",
    "coc.cost_of_equity",
    "coc.beta_relever",
    "coc.beta_peers",
    "coc.cost_of_debt",
    "coc.wacc",
    "tv.gordon",
    "tv.value_driver",
    "tv.h_model",
    "tv.exit_multiple",
    "tv.limited_horizon",
    "tv.normalised_income",
    "tv.liquidation",
    "tv.compare",
    "analysis.goal_seek",
    "analysis.sensitivity",
    "analysis.scenarios",
    "analysis.tornado",
];

/// Byte-wise string equality usable in constant evaluation.
const fn same(a: &str, b: &str) -> bool {
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut i = 0;
    while i < a.len() {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    true
}

const fn is_listed(id: &str, list: &[&str]) -> bool {
    let mut i = 0;
    while i < list.len() {
        if same(id, list[i]) {
            return true;
        }
        i += 1;
    }
    false
}

const fn status_of(id: &str) -> Status {
    if is_listed(id, AVAILABLE) {
        Status::Available
    } else {
        Status::Preview
    }
}

/// Builds the catalogue, the schema lookup and the dispatch from one list of [`Method`] types.
/// Adding a method to the engine means implementing [`Method`] in its theme module and adding
/// one line to the list below; nothing else can drift.
macro_rules! registry {
    ($($m:ty),+ $(,)?) => {
        $(const _: () = assert!(
            crate::method::is_valid_version(<$m as Method>::VERSION),
            "method VERSION must be a positive integer such as \"1\""
        );)+

        /// Every method exposed by the engine, in catalogue order.
        pub const METHODS: &[MethodInfo] = &[$(
            MethodInfo {
                id: <$m as Method>::ID,
                title: <$m as Method>::TITLE,
                description: <$m as Method>::DESCRIPTION,
                version: <$m as Method>::VERSION,
                status: status_of(<$m as Method>::ID),
            },
        )+];

        /// Every id promoted in `AVAILABLE` is a registered method (ADR 0012).
        const _: () = {
            let ids: &[&str] = &[$(<$m as Method>::ID),+];
            let mut i = 0;
            while i < AVAILABLE.len() {
                assert!(is_listed(AVAILABLE[i], ids), "AVAILABLE names an unregistered method");
                i += 1;
            }
        };

        /// `(id, Rust type path)` of every method, to check that each lives in its theme module.
        /// `type_name` is best-effort by documentation but stable in practice for plain paths;
        /// it is used only in this test, never at run time.
        #[cfg(test)]
        fn type_paths() -> Vec<(&'static str, &'static str)> {
            vec![$((<$m as Method>::ID, std::any::type_name::<$m>())),+]
        }

        fn schema_of(id: &str) -> Option<schemars::Schema> {
            $(if id == <$m as Method>::ID {
                return Some(schema_for!(<$m as Method>::Input));
            })+
            None
        }

        fn dispatch_to(id: &str, input_json: &str) -> Option<Result<Value>> {
            $(if id == <$m as Method>::ID {
                return Some(execute::<$m>(input_json));
            })+
            None
        }
    };
}

registry! {
    // Grouped by theme, in the order of the documentation: dates, series, rates, time value of
    // money, loans, cost of capital, terminal value, discounted cash flow, financial statements,
    // ratios, analysis.
    dates::YearFraction,
    dates::Schedule,
    series::Project,
    rates::Convert,
    rates::RealNominal,
    tvm::Npv,
    tvm::PresentValue,
    tvm::Xnpv,
    tvm::Irr,
    tvm::Xirr,
    tvm::Mirr,
    tvm::Annuity,
    tvm::Perpetuity,
    tvm::FutureValue,
    loans::Instalment,
    loans::NumberOfInstalments,
    loans::AmortisationSchedule,
    coc::CostOfEquity,
    coc::BetaRelever,
    coc::BetaPeers,
    coc::CostOfDebt,
    coc::Wacc,
    tv::Gordon,
    tv::ValueDriver,
    tv::HModel,
    tv::ExitMultiple,
    tv::LimitedHorizon,
    tv::NormalisedIncome,
    tv::Liquidation,
    tv::Compare,
    dcf::Fcff,
    dcf::Fcfe,
    dcf::FreeCashFlows,
    statements::Reorganise,
    ratios::profitability::Profitability,
    ratios::capital_structure::CapitalStructure,
    ratios::liquidity::Liquidity,
    ratios::working_capital::WorkingCapital,
    ratios::debt_service::DebtService,
    ratios::covenants::Covenants,
    analysis::goal_seek::GoalSeek,
    analysis::sensitivity::Sensitivity,
    analysis::scenarios::Scenarios,
    analysis::tornado::Tornado,
}

/// Evaluates a method on a JSON value from inside the core: the same path as [`run`] (parsing,
/// validation, computation, envelope), without a second panic guard, because it is only
/// called from within a `run` (ADR 0010).
pub(crate) fn evaluate(id: &str, input: &Value) -> Result<Value> {
    dispatch(id, &input.to_string())
}

pub fn method(id: &str) -> Result<&'static MethodInfo> {
    METHODS
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))
}

/// JSON Schema (2020-12) of a method's input.
pub fn input_schema(id: &str) -> Result<Value> {
    let schema = schema_of(id).ok_or_else(|| FairmarkError::UnknownMethod(id.to_owned()))?;
    serde_json::to_value(schema).map_err(|e| FairmarkError::InvalidInput(e.to_string()))
}

/// Run a method on a JSON input and return the JSON result envelope.
///
/// Numbers in the input may be JSON numbers or strings; both are parsed
/// exactly into decimals (no binary floating point is ever involved).
pub fn run(id: &str, input_json: &str) -> Result<Value> {
    // Last line of defence: all arithmetic is checked, so a panic is a defect. It must never
    // cross an adapter boundary (Python, CLI, MCP): it becomes `FM-E012` instead. This guard
    // has no effect where panics abort (for example WebAssembly targets); checked arithmetic
    // and the extreme-input contract cases (`tests/contracts/`) remain the primary defence there. Panic messages in
    // the core never contain input values.
    std::panic::catch_unwind(|| dispatch(id, input_json)).unwrap_or(Err(FairmarkError::Internal(
        "unexpected panic in the engine",
    )))
}

fn dispatch(id: &str, input_json: &str) -> Result<Value> {
    dispatch_to(id, input_json).unwrap_or_else(|| Err(FairmarkError::UnknownMethod(id.to_owned())))
}

fn execute<M: Method>(input_json: &str) -> Result<Value> {
    let mut input: M::Input = parse(input_json)?;
    // SPEC-0001 R31: the defaults are filled in once, in `run_typed`; the hash covers the input
    // with its defaults, so that a minimal input and its explicit equivalent hash identically.
    let (value, explain, defaults) = M::run_typed(&mut input)?;
    let canonical =
        serde_json::to_string(&input).map_err(|e| FairmarkError::InvalidInput(e.to_string()))?;
    let envelope = Valuation {
        method: M::ID.to_owned(),
        value,
        explain,
        meta: Meta {
            engine_version: ENGINE_VERSION.to_owned(),
            method_version: M::VERSION.to_owned(),
            input_hash: sha256_hex(canonical.as_bytes()),
            arithmetic: "decimal, 96-bit mantissa (28-29 significant digits)".to_owned(),
            method_status: status_of(M::ID),
            evaluated_method_status: M::evaluated_method(&input).map(status_of),
            defaults_applied: defaults,
        },
    };
    serde_json::to_value(envelope).map_err(|e| FairmarkError::InvalidInput(e.to_string()))
}

/// Reads a method input. Every failure names the field by its JSON pointer (issue #174,
/// SPEC-0001 R27): `FM-E001`, or `FM-E009` for an invalid date. Only a malformed JSON text
/// (a syntax error, a truncated input, trailing characters) keeps serde's line and column,
/// because it has no field to name.
fn parse<T: serde::de::DeserializeOwned>(input_json: &str) -> Result<T> {
    let mut de = serde_json::Deserializer::from_str(input_json);
    let input = serde_path_to_error::deserialize(&mut de).map_err(input_error)?;
    // `serde_json::from_str` does the same: nothing but whitespace may follow the value.
    de.end()
        .map_err(|e| FairmarkError::InvalidInput(e.to_string()))?;
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

fn input_error(e: serde_path_to_error::Error<serde_json::Error>) -> FairmarkError {
    let mut tokens = path_tokens(e.path());
    let inner = e.into_inner();
    if !inner.is_data() {
        return FairmarkError::InvalidInput(inner.to_string());
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
    let detail = if let Some(field) = quoted_field(msg, "missing field `") {
        // Reported at the object that lacks the field: the pointer is completed with it.
        tokens.push(field.to_owned());
        format!("missing required field `{field}`; add it")
    } else if let Some(field) = quoted_field(msg, "duplicate field `") {
        // serde refuses a repeated field of a struct at the object that holds it.
        tokens.push(field.to_owned());
        format!("field `{field}` appears more than once; keep one")
    } else if quoted_field(msg, "unknown field `").is_some() {
        // The path already ends with the unknown key.
        format!("{msg}; remove or rename it")
    } else if msg == OBJECT_FOR_NUMBER {
        // The path ends with the first key of the object; the decimal is its parent.
        tokens.pop();
        format!("invalid type: map, expected {DECIMAL_EXPECTED}")
    } else {
        without_type_names(&msg.replace(RUST_DECIMAL_EXPECTED, DECIMAL_EXPECTED))
    };
    match pointer(&tokens) {
        Some(pointer) => FairmarkError::InvalidInputAt { pointer, detail },
        None => FairmarkError::InvalidInput(detail),
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

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// ADR 0009: ids are unique `<theme>.<name>`, versions are integers, and every method type
    /// is defined in the module of its theme (`fairmark_core::<theme>::<Type>`).
    #[test]
    fn method_ids_are_unique_and_live_in_their_theme_module() {
        let mut seen = std::collections::BTreeSet::new();
        for m in METHODS {
            assert!(seen.insert(m.id), "duplicate method id {}", m.id);
            let (_, name) = m.id.split_once('.').expect("id is <theme>.<name>");
            assert!(
                !name.is_empty()
                    && name
                        .chars()
                        .all(|c| c.is_ascii_lowercase() || c == '_' || c.is_ascii_digit()),
                "{}: name must be snake_case",
                m.id
            );
            assert!(
                m.version.parse::<u32>().is_ok(),
                "{}: version must be an integer",
                m.id
            );
        }
        for (id, path) in type_paths() {
            let theme = id.split('.').next().unwrap();
            assert!(
                path.starts_with(&format!("fairmark_core::{theme}::")),
                "{id} is implemented by {path}, outside the module of its theme"
            );
        }
    }

    /// ADR 0012: methods not promoted in `AVAILABLE` are preview, and the status is part of
    /// the catalogue every adapter serialises.
    #[test]
    fn status_comes_from_the_promotion_list() {
        for m in METHODS {
            let expected = if AVAILABLE.contains(&m.id) {
                Status::Available
            } else {
                Status::Preview
            };
            assert_eq!(m.status, expected, "{}", m.id);
        }
        assert_eq!(method("dcf.fcff").unwrap().status, Status::Preview);
        assert_eq!(method("tvm.npv").unwrap().status, Status::Available);
        assert_eq!(method("tv.gordon").unwrap().status, Status::Available);
        let json = serde_json::to_value(method("dcf.fcff").unwrap()).unwrap();
        assert_eq!(json["status"], "preview");
        let json = serde_json::to_value(method("tv.gordon").unwrap()).unwrap();
        assert_eq!(json["status"], "available");
    }

    /// A two-period dcf.fcff version 3 input, valued with a Gordon terminal value.
    const DCF_V3: &str = r#"{"stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}], "cash_flow_terms": "nominal", "discount_rate_terms": "nominal", "wacc": "0.09", "timing": "end_of_period", "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}, "net_financial_position": {"gross_debt": 0, "cash": 0}}"#;

    /// ADR 0012: every result states the status of the method that produced it.
    #[test]
    fn results_report_the_method_status() {
        let tv = run(
            "tv.gordon",
            r#"{"rate": "0.1", "growth": "0.02", "next_flow": "50"}"#,
        )
        .unwrap();
        assert_eq!(tv["meta"]["method_status"], "available");
        assert!(tv["meta"].get("evaluated_method_status").is_none());
        let npv = run("tvm.npv", r#"{"rate": 0.1, "cash_flows": [-100, 60, 60]}"#).unwrap();
        assert_eq!(npv["meta"]["method_status"], "available");
        // dcf.fcff version 3 input (SPEC-0001).
        let dcf = run("dcf.fcff", DCF_V3).unwrap();
        assert_eq!(dcf["meta"]["method_status"], "preview");
    }

    /// ADR 0012: an analysis reports the status of the method it evaluates, in `meta` and, when
    /// that method is preview, as a convention.
    #[test]
    fn analysis_reports_the_status_of_the_evaluated_method() {
        let table = |method: &str, base: &str, variable: &str, output: &str| {
            run(
                "analysis.sensitivity",
                &format!(
                    r#"{{"method": "{method}", "base_input": {base}, "output": "{output}",
                        "rows": {{"variable": "{variable}", "values": ["0.08", "0.09"]}}}}"#
                ),
            )
            .unwrap()
        };
        let preview = table("dcf.fcff", DCF_V3, "/wacc", "/enterprise_value");
        assert_eq!(preview["meta"]["method_status"], "available");
        assert_eq!(preview["meta"]["evaluated_method_status"], "preview");
        let conventions = preview["explain"]["conventions"].as_array().unwrap();
        assert!(conventions.iter().any(|c| c
            .as_str()
            .unwrap()
            .starts_with("Evaluated method dcf.fcff is preview")));

        let available = table(
            "tv.gordon",
            r#"{"rate": "0.09", "growth": "0.02", "next_flow": "50"}"#,
            "/rate",
            "/terminal_value",
        );
        assert_eq!(available["meta"]["evaluated_method_status"], "available");
        let conventions = available["explain"]["conventions"].as_array().unwrap();
        assert!(!conventions
            .iter()
            .any(|c| c.as_str().unwrap().contains("is preview")));
    }

    #[test]
    fn promotion_list_names_registered_methods_once() {
        let mut seen = std::collections::BTreeSet::new();
        for id in AVAILABLE {
            assert!(seen.insert(id), "{id} is listed twice");
            assert!(method(id).is_ok(), "{id} is not registered");
        }
        assert!(is_listed("tv.gordon", AVAILABLE));
        assert!(!is_listed("tv.gordo", AVAILABLE));
        assert!(!is_listed("tv.gordonx", AVAILABLE));
    }

    #[test]
    fn every_method_has_a_schema() {
        for m in METHODS {
            assert!(input_schema(m.id).is_ok(), "{}", m.id);
        }
    }

    #[test]
    fn same_input_same_hash_and_value() {
        let input = r#"{"rate": 0.1, "cash_flows": [-100, 60, 60]}"#;
        assert_eq!(
            run("tvm.npv", input).unwrap(),
            run("tvm.npv", input).unwrap()
        );
    }

    #[test]
    fn json_numbers_are_parsed_exactly() {
        let out = run("tvm.npv", r#"{"rate": 0, "cash_flows": [0.1, 0.2]}"#).unwrap();
        assert_eq!(out["value"]["npv"], "0.3");
    }

    #[test]
    fn a_field_named_like_the_date_marker_is_not_a_date_error() {
        let err = run(
            "tvm.npv",
            r#"{"rate": 0.1, "cash_flows": [1], "invalid date": 1}"#,
        )
        .unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }

    #[test]
    fn unknown_fields_are_rejected() {
        let err = run("tvm.npv", r#"{"rate": 0.1, "cash_flows": [1], "typo": 1}"#).unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }
}
