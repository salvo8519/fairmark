//! SPEC-0008 R22, AC17: a 21 × 21 sensitivity table on a ten-year DCF stays within budget.
//!
//! The gating test runs in the test profile with a generous ceiling, so that CI cannot flake;
//! the target on the reference runner, in release mode, is recorded by an ignored test:
//! `cargo test --release -p fairmark-core --test analysis_budget -- --ignored`.

use std::time::{Duration, Instant};

use serde_json::{json, Value};

fn request() -> String {
    let rates: Vec<String> = (0..21).map(|i| format!("0.{:03}", 70 + i * 2)).collect();
    let growths: Vec<String> = (0..21).map(|i| format!("0.{:03}", i)).collect();
    json!({
        "method": "dcf.fcff",
        "base_input": {
            "stages": [{"periods": 10, "projection": "given",
                        "flows": [100, 105, 110, 116, 122, 128, 134, 141, 148, 155]}],
            "cash_flow_terms": "nominal", "discount_rate_terms": "nominal",
            "wacc": "0.09", "timing": "end_of_period",
            "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
            "net_financial_position": {"gross_debt": 300, "cash": 0}
        },
        "output": "/value_of_operations",
        "rows": {"variable": "/wacc", "values": rates},
        "columns": {"variable": "/terminal/tv.gordon/growth", "values": growths}
    })
    .to_string()
}

fn timed() -> (Value, Duration) {
    let start = Instant::now();
    let out = fairmark_core::run("analysis.sensitivity", &request()).expect("table");
    (out, start.elapsed())
}

#[test]
fn a_21_by_21_table_runs_442_evaluations_within_the_ceiling() {
    let (out, elapsed) = timed();
    let conventions = out["explain"]["conventions"].to_string();
    assert!(
        conventions.contains("442 evaluations of the inner method"),
        "{conventions}"
    );
    assert_eq!(out["value"]["cells"].as_array().unwrap().len(), 21);
    assert!(elapsed < Duration::from_secs(10), "took {elapsed:?}");
}

#[test]
#[ignore = "release-mode target, not gating: run with --release -- --ignored"]
fn target_in_release_mode() {
    let (_, elapsed) = timed();
    println!("21 × 21 table on a ten-year DCF: {elapsed:?}");
    assert!(elapsed < Duration::from_millis(250), "took {elapsed:?}");
}
