//! `fairmark run` prints the whole error report on stderr (ADR 0014 G5, issue #258): the
//! field to correct, the sub-kind, the fix and every input error found.

use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_fairmark");

fn run(method: &str, input: &str) -> serde_json::Value {
    let mut child = Command::new(BIN)
        .args(["run", method])
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
    assert!(!output.status.success());
    serde_json::from_slice(&output.stderr).expect("stderr is JSON")
}

#[test]
fn run_prints_the_structured_report() {
    let report = run(
        "loans.instalment",
        r#"{"principal": "0", "number_of_instalments": 0, "timing": "ordinary", "periodic_rate": "0.01"}"#,
    );
    let error = &report["error"];
    assert_eq!(error["code"], "FM-E001");
    assert_eq!(error["pointer"], "/principal");
    assert_eq!(error["reason"], "range");
    let errors = error["errors"].as_array().unwrap();
    assert_eq!(errors.len(), 2);
    assert_eq!(errors[1]["pointer"], "/number_of_instalments");
}

#[test]
fn malformed_json_has_reason_syntax() {
    let report = run("tvm.npv", "{");
    assert_eq!(report["error"]["reason"], "syntax");
    assert!(report["error"]["pointer"].is_null());
}
