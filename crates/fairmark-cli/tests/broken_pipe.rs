//! Issue #191: when the reader of `fairmark`'s output closes the pipe early
//! (`fairmark methods | head -1`), the binary must exit quietly with status 0:
//! no panic, nothing on stderr.
//!
//! Each test hands the child the write end of a pipe whose read end is already
//! closed, so every write to stdout fails with `BrokenPipe` deterministically,
//! whatever the size of the output and the timing of the processes.

use std::io::Write;
use std::process::{Command, Output, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_fairmark");

/// Input of the `dcf.fcff` example in `docs/en/guides/cli.md`.
const DCF_INPUT: &str = r#"{
  "stages": [
    {"periods": 5, "projection": "given",
     "flows": ["1200000", "1350000", "1500000", "1600000", "1700000"]}
  ],
  "cash_flow_terms": "nominal",
  "discount_rate_terms": "nominal",
  "wacc": "0.095",
  "timing": "mid_period",
  "terminal_discount_time": "horizon",
  "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
  "net_financial_position": {"gross_debt": "3100000", "cash": "0"}
}"#;

/// Runs `fairmark` with `args`, `stdin` as standard input and a standard
/// output whose reader is closed before the process starts.
fn run_with_closed_stdout(args: &[&str], stdin: &str) -> Output {
    let (reader, writer) = std::io::pipe().expect("create pipe");
    drop(reader);
    let mut child = Command::new(BIN)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(writer)
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn fairmark");
    {
        let mut input = child.stdin.take().expect("stdin is piped");
        input.write_all(stdin.as_bytes()).expect("write stdin");
    }
    child.wait_with_output().expect("wait for fairmark")
}

fn assert_quiet_success(output: &Output) {
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "",
        "stderr must stay empty on a closed output pipe"
    );
    assert_eq!(output.status.code(), Some(0), "status: {:?}", output.status);
}

fn temp_file(name: &str, contents: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!("fairmark-191-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join(name);
    std::fs::write(&path, contents).expect("write temp file");
    path
}

#[test]
fn methods_exits_quietly_when_stdout_is_closed() {
    assert_quiet_success(&run_with_closed_stdout(&["methods"], ""));
}

#[test]
fn schema_exits_quietly_when_stdout_is_closed() {
    assert_quiet_success(&run_with_closed_stdout(&["schema", "dcf.fcff"], ""));
}

#[test]
fn run_exits_quietly_when_stdout_is_closed() {
    assert_quiet_success(&run_with_closed_stdout(&["run", "dcf.fcff"], DCF_INPUT));
}

#[test]
fn run_value_only_exits_quietly_when_stdout_is_closed() {
    assert_quiet_success(&run_with_closed_stdout(
        &["run", "dcf.fcff", "--value-only"],
        DCF_INPUT,
    ));
}

#[test]
fn verify_exits_quietly_when_stdout_is_closed() {
    let run = Command::new(BIN)
        .args(["run", "dcf.fcff"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            child
                .stdin
                .take()
                .expect("stdin is piped")
                .write_all(DCF_INPUT.as_bytes())?;
            child.wait_with_output()
        })
        .expect("run fairmark");
    assert!(run.status.success(), "run failed: {run:?}");
    let input = temp_file("input.json", DCF_INPUT);
    let result = temp_file("result.json", &String::from_utf8(run.stdout).unwrap());

    let output = run_with_closed_stdout(
        &[
            "verify",
            result.to_str().unwrap(),
            "--input",
            input.to_str().unwrap(),
        ],
        "",
    );
    std::fs::remove_dir_all(input.parent().unwrap()).ok();
    assert_quiet_success(&output);
}

#[test]
fn errors_still_go_to_stderr_with_failure_status_when_stdout_is_closed() {
    let output = run_with_closed_stdout(&["schema", "no.such.method"], "");
    assert_eq!(output.status.code(), Some(1), "status: {:?}", output.status);
    let report: serde_json::Value =
        serde_json::from_slice(&output.stderr).expect("stderr is a JSON error report");
    assert!(report["error"]["code"].is_string(), "report: {report}");
}
