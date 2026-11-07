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
        "base_input": {"free_cash_flows": [100, 105, 110, 116, 122, 128, 134, 141, 148, 155],
                       "discount_rate": "0.09", "terminal_growth": "0.02", "net_debt": 300},
        "output": "/enterprise_value",
        "rows": {"variable": "/discount_rate", "values": rates},
        "columns": {"variable": "/terminal_growth", "values": growths}
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
