//! `fairmark schema`: the input schema by default, the output schema with `--output` and the
//! result envelope schema with `--envelope` (issue #265, ADR 0014 G8).

use std::process::Command;

const BIN: &str = env!("CARGO_BIN_EXE_fairmark");

fn schema(args: &[&str]) -> serde_json::Value {
    let output = Command::new(BIN)
        .arg("schema")
        .args(args)
        .output()
        .expect("run fairmark");
    assert!(output.status.success(), "status: {:?}", output.status);
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

#[test]
fn schema_prints_the_input_schema_by_default() {
    let input = schema(&["tvm.npv"]);
    assert_eq!(input["title"], "NpvInput");
    assert!(input["properties"]["rate"].is_object());
}

#[test]
fn schema_output_prints_the_output_schema() {
    let output = schema(&["tvm.npv", "--output"]);
    assert_eq!(output["title"], "NpvOutput");
    assert_eq!(output["properties"]["npv"]["type"], "string");
}

#[test]
fn schema_envelope_prints_the_result_schema() {
    let envelope = schema(&["tvm.npv", "--envelope"]);
    for key in ["method", "value", "explain", "meta"] {
        assert!(envelope["properties"][key].is_object(), "{key}");
    }
}

#[test]
fn schema_output_and_envelope_are_exclusive() {
    let output = Command::new(BIN)
        .args(["schema", "tvm.npv", "--output", "--envelope"])
        .output()
        .expect("run fairmark");
    assert!(!output.status.success());
}
