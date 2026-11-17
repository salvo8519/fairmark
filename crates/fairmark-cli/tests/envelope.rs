//! `fairmark run` prints the envelope of the core unchanged (SPEC-0012 R15, AC13): the same
//! JSON, diagnostics included, for one input per theme that has diagnostics to report.

use std::io::Write;
use std::process::{Command, Stdio};

const BIN: &str = env!("CARGO_BIN_EXE_fairmark");

fn cli(method: &str, input: &str) -> serde_json::Value {
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
    assert!(output.status.success(), "{method}");
    serde_json::from_slice(&output.stdout).expect("stdout is JSON")
}

const CASES: &[(&str, &str)] = &[
    (
        "tvm.npv",
        r#"{"rate": "0.1", "cash_flows": [-100, 60, 60]}"#,
    ),
    (
        "dcf.fcff",
        r#"{"stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}],
            "cash_flow_terms": "nominal", "discount_rate_terms": "nominal", "wacc": "0.09",
            "timing": "end_of_period",
            "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}}}"#,
    ),
    (
        "ratios.debt_service",
        r#"{"balances": "closing", "interest_definition": "gross interest expense",
            "income_statement": {"ebit": "500", "interest": "100"}}"#,
    ),
    (
        "statements.reorganise",
        r#"{"years": [{"label": "2024", "reclassified": {"balance_sheet": {"equity": "900",
            "net_financial_position": {"total": "100"}, "total_sources": "1000"}}}]}"#,
    ),
    (
        "analysis.goal_seek",
        r#"{"method": "ratios.debt_service", "base_input": {"balances": "closing",
            "interest_definition": "gross interest expense",
            "income_statement": {"ebit": "500", "interest": "100"}},
            "variable_pointer": "/income_statement/ebit",
            "output_pointer": "/interest_cover_ebit/value", "target": "3",
            "bracket": ["100", "1000"]}"#,
    ),
];

#[test]
fn the_cli_prints_the_envelope_of_the_core() {
    for (method, input) in CASES {
        let from_cli = cli(method, input);
        let from_core = fairmark_core::run(method, input).unwrap();
        assert_eq!(from_cli, from_core, "{method}");
        let keys: Vec<&str> = from_cli
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            ["method", "value", "explain", "diagnostics", "meta"],
            "{method}"
        );
    }
    // The cases report something in each list.
    let reported = |key: &str| {
        CASES.iter().any(|(m, i)| {
            !fairmark_core::run(m, i).unwrap()["diagnostics"][key]
                .as_array()
                .unwrap()
                .is_empty()
        })
    };
    assert!(reported("not_computed") && reported("warnings") && reported("checks"));
}
