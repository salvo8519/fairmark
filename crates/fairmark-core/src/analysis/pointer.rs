//! RFC 6901 JSON Pointers into inputs and results (SPEC-0008 R3, R4).
//!
//! Resolution reuses `serde_json::Value::pointer`, which implements RFC 6901 (escapes `~0` and
//! `~1`, array indices without leading zeros). A leaf is a decimal when it deserialises as a
//! `Decimal`, with the same deserialiser the methods use, so accepted spellings cannot diverge.

use rust_decimal::Decimal;
use serde_json::Value;

use crate::error::{FairmarkError, Result};

fn invalid(pointer: &str, reason: impl Into<String>) -> FairmarkError {
    FairmarkError::InvalidPointer {
        pointer: pointer.to_owned(),
        reason: reason.into(),
    }
}

/// Checks the syntax: a pointer to a member, never the whole document, never `-`.
pub fn check_syntax(pointer: &str) -> Result<()> {
    if !pointer.starts_with('/') {
        return Err(invalid(pointer, "must start with '/'"));
    }
    if pointer.split('/').skip(1).any(|token| token == "-") {
        return Err(invalid(
            pointer,
            "the '-' token (past the end of an array) is not a member",
        ));
    }
    Ok(())
}

/// The decimal at `pointer` in `doc`.
pub fn read_decimal(doc: &Value, pointer: &str) -> Result<Decimal> {
    check_syntax(pointer)?;
    let leaf = doc
        .pointer(pointer)
        .ok_or_else(|| invalid(pointer, "no such member"))?;
    if !(leaf.is_number() || leaf.is_string()) {
        return Err(invalid(pointer, "the value is not a decimal"));
    }
    serde_json::from_value::<Decimal>(leaf.clone())
        .map_err(|_| invalid(pointer, "the value is not a decimal"))
}

/// Replaces the decimal at `pointer` with `value`, written as a JSON string to stay exact.
pub fn write_decimal(doc: &mut Value, pointer: &str, value: Decimal) -> Result<()> {
    read_decimal(doc, pointer)?;
    let leaf = doc
        .pointer_mut(pointer)
        .ok_or_else(|| invalid(pointer, "no such member"))?;
    *leaf = Value::String(value.to_string());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::str::FromStr;

    fn doc() -> Value {
        serde_json::from_str(
            r#"{"rate": 0.1, "flows": ["-100", 60, 60], "timing": "ordinary",
                "a/b": {"m~n": "2.5"}, "nested": {"x": [1, 2]}}"#,
        )
        .unwrap()
    }

    #[test]
    fn reads_numbers_and_strings_exactly() {
        let d = doc();
        assert_eq!(
            read_decimal(&d, "/rate").unwrap(),
            Decimal::from_str("0.1").unwrap()
        );
        assert_eq!(read_decimal(&d, "/flows/0").unwrap(), Decimal::from(-100));
        assert_eq!(
            read_decimal(&d, "/a~1b/m~0n").unwrap(),
            Decimal::from_str("2.5").unwrap()
        );
        assert_eq!(read_decimal(&d, "/nested/x/1").unwrap(), Decimal::TWO);
    }

    #[test]
    fn every_invalid_pointer_is_e015() {
        let d = doc();
        for p in [
            "/missing",
            "/flows/3",
            "/flows/-",
            "rate",
            "",
            "/timing",
            "/nested",
            "/flows/01",
        ] {
            let e = read_decimal(&d, p).unwrap_err();
            assert_eq!(e.code(), "FM-E015", "{p}");
        }
    }

    #[test]
    fn writes_a_string_and_leaves_the_rest_unchanged() {
        let mut d = doc();
        write_decimal(&mut d, "/flows/1", Decimal::from_str("61.25").unwrap()).unwrap();
        assert_eq!(d["flows"], json!(["-100", "61.25", 60]));
        assert_eq!(d["rate"], doc()["rate"]);
        assert_eq!(
            write_decimal(&mut d, "/nope", Decimal::ONE)
                .unwrap_err()
                .code(),
            "FM-E015"
        );
    }
}
