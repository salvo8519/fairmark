//! `fairmark run` reports a decimal rounded as it is read with the same warning and the same
//! value as the core (and as the Python SDK, function calling and MCP, see
//! `python/tests/test_decimal_precision.py`): the rule is applied once, in the core.

use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_fairmark");

/// The expected warning and value, shared with the Python test.
const WARNING: &str = "warning: `/rate` has more digits than a decimal holds: given \
    0.009166666666666666666666666667, used 0.0091666666666666666666666667, rounded half away \
    from zero at the first digit that does not fit (28 decimal places, or 28 to 29 significant \
    digits), as the input is read";
const FUTURE_VALUE: &str = "82846.24637189952008238315640";

fn cli(input: &str) -> serde_json::Value {
    let mut child = Command::new(BIN)
        .args(["run", "tvm.future_value"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("run fairmark");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

#[test]
fn the_cli_reports_the_rounding_as_the_core_does() {
    for rate in [
        "\"0.009166666666666666666666666667\"",
        "0.009166666666666666666666666667",
    ] {
        let input =
            format!(r#"{{"payment": "2000", "rate": {rate}, "periods": 35, "timing": "due"}}"#);
        let from_cli = cli(&input);
        let from_core = fairmark_core::run("tvm.future_value", &input).unwrap();
        assert_eq!(from_cli, from_core);
        assert_eq!(from_cli["value"]["future_value"], FUTURE_VALUE);
        let warnings: Vec<&str> = from_cli["explain"]["conventions"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|c| c.as_str())
            .filter(|c| c.starts_with("warning: "))
            .collect();
        assert_eq!(warnings, vec![WARNING]);
    }
}
