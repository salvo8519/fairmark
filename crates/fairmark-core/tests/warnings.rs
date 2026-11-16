//! The warnings of the catalogue (SPEC-0012 R8, R9; AC7): one test per code, on a golden input
//! that already raised the warning as a convention before SPEC-0012, and the echo test over
//! every golden result. The catalogue is `docs/en/reference/warnings.md`.

use std::path::Path;

use serde_json::Value;

/// The input and method of the golden case `case` (`<folder>/<file>.yaml`).
fn golden(case: &str) -> (String, Value) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/golden")
        .join(case);
    let case: Value = serde_norway::from_str(&std::fs::read_to_string(path).unwrap()).unwrap();
    (
        case["method"].as_str().unwrap().to_owned(),
        case["input"].clone(),
    )
}

fn run(method: &str, input: &Value) -> Value {
    fairmark_core::run(method, &input.to_string()).unwrap()
}

fn conventions(out: &Value) -> Vec<&str> {
    out["explain"]["conventions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap())
        .collect()
}

/// One warning of the catalogue on `case`: an entry with `code` at `pointer` whose message
/// starts with the text the engine wrote before SPEC-0012 (without the `warning: ` prefix), and
/// its echo `warning <code>: <message>` exactly once in the conventions (R9).
fn assert_warning(case: &str, code: &str, pointer: Option<&str>, message: &str) {
    let (method, input) = golden(case);
    let out = run(&method, &input);
    let warnings = out["diagnostics"]["warnings"]
        .as_array()
        .unwrap_or_else(|| panic!("{case}: no diagnostics.warnings in {out}"));
    let entry = warnings
        .iter()
        .find(|w| w["code"] == code && w["pointer"].as_str() == pointer)
        .unwrap_or_else(|| panic!("{case}: no {code} at {pointer:?} in {warnings:?}"));
    let text = entry["message"].as_str().unwrap();
    assert!(
        text.starts_with(message),
        "{case}: {code} message {text:?} does not start with {message:?}"
    );
    let echo = format!("warning {code}: {text}");
    let conventions = conventions(&out);
    assert_eq!(
        conventions.iter().filter(|c| **c == echo).count(),
        1,
        "{case}: echo {echo:?} in {conventions:?}"
    );
    assert!(
        !conventions.iter().any(|c| c.starts_with("warning: ")),
        "{case}: a warning without its code in {conventions:?}"
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T11, which writes FM-W001"]
fn fm_w001_a_decimal_rounded_as_it_is_read() {
    assert_warning(
        "tvm.future_value/ecma-376-monthly-rate-due.yaml",
        "FM-W001",
        Some("/rate"),
        "`/rate` has more digits than a decimal holds: given 0.009166666666666666666666666667, \
         used 0.0091666666666666666666666667, rounded half away from zero",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w002_a_negative_base_value() {
    assert_warning(
        "series.project/negative-base-value.yaml",
        "FM-W002",
        Some("/base_value"),
        "the base value -200 is negative, so a positive growth makes the values more negative",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w003_a_growth_applied_to_a_negative_flow() {
    for period in ["period 4", "period 5"] {
        let (method, input) =
            golden("dcf.fcfe/stage-with-build-up-lines-ebit-below-interest-zero.yaml");
        let out = run(&method, &input);
        let found = out["diagnostics"]["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|w| {
                w["code"] == "FM-W003"
                    && w["pointer"] == "/stages/1"
                    && w["message"]
                        .as_str()
                        .unwrap()
                        .starts_with(&format!("{period}: growth 0.02 applied to a negative flow"))
            })
            .count();
        assert_eq!(found, 1, "{period}");
    }
    assert_warning(
        "dcf.fcfe/stage-with-build-up-lines-ebit-below-interest-zero.yaml",
        "FM-W003",
        Some("/stages/1"),
        "period 4: growth 0.02 applied to a negative flow",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w004_a_perpetual_growth_below_inflation() {
    assert_warning(
        "dcf.fcff/terms-equal-with-inflation-growth-in-other-terms.yaml",
        "FM-W004",
        Some("/terminal/tv.gordon/growth"),
        "the perpetual growth 0.02 (nominal) is below the expected inflation of the terminal \
         phase 0.03: the flows decline in real terms after the horizon",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w005_no_terminal_value() {
    assert_warning(
        "dcf.fcfe/flows-only-single-cost-of-equity.yaml",
        "FM-W005",
        Some("/terminal"),
        "no terminal value is included: the value covers the 5 explicit flows only",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w006_a_negative_given_terminal_value() {
    assert_warning(
        "dcf.fcff/terminal-given-negative-decommissioning.yaml",
        "FM-W006",
        Some("/terminal/given/value"),
        "the given terminal value -300 is negative (for example a decommissioning cost); it is \
         discounted as given",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w007_a_terminal_value_weight_not_meaningful() {
    assert_warning(
        "dcf.fcff/terminal-given-negative-decommissioning.yaml",
        "FM-W007",
        Some("/value/terminal_value_weight"),
        "the terminal value is negative, so the terminal value weight is not meaningful as a \
         share of the value",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w008_last_explicit_growth_differs_from_the_perpetual_growth() {
    assert_warning(
        "dcf.fcff/terms-converted-rate-exact.yaml",
        "FM-W008",
        Some("/terminal/tv.h_model/long_term_growth"),
        "the growth of the last explicit period (0.02) differs from the perpetual growth of the \
         terminal value (0.01)",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w009_a_price_at_a_date_discounted_at_n_minus_half() {
    assert_warning(
        "dcf.fcff/terminal-exit-multiple-mid-period-at-n-minus-half.yaml",
        "FM-W009",
        Some("/terminal_discount_time"),
        "tv.exit_multiple is a price at the horizon N (a sale or a liquidation at a date); \
         discounting it at N − 0.5 brings it forward half a period",
    );
    assert_warning(
        "dcf.fcff/terminal-given-mid-period-shift-rates-per-period.yaml",
        "FM-W009",
        Some("/terminal_discount_time"),
        "given: the analyst's terminal value at the horizon N; discounting it at N − 0.5",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w010_a_perpetuity_discounted_at_the_horizon_under_mid_period_timing() {
    assert_warning(
        "dcf.fcff/terminal-normalised-income-zero-growth.yaml",
        "FM-W010",
        Some("/terminal_discount_time"),
        "under mid-period timing a perpetuity-growth terminal value is commonly discounted at \
         N − 0.5",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w011_a_negative_equity_value() {
    assert_warning(
        "dcf.fcff/bridge-negative-equity-value.yaml",
        "FM-W011",
        Some("/value/equity_value"),
        "the equity value is negative (-78.51239669421487603",
    );
    assert_warning(
        "dcf.fcfe/bridge-negative-equity-value.yaml",
        "FM-W011",
        Some("/value/equity_value"),
        "the equity value is negative (-41.75438596491228070",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w012_no_net_financial_position() {
    assert_warning(
        "dcf.fcff/terms-equal-with-inflation-growth-in-other-terms.yaml",
        "FM-W012",
        Some("/net_financial_position"),
        "no net financial position given (`/net_financial_position`): zero is subtracted, so \
         the equity value equals the enterprise value",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w013_no_other_items_of_the_net_financial_position() {
    assert_warning(
        "ratios.debt_service/partial-schedule.yaml",
        "FM-W013",
        Some("/closing/net_financial_position/other_items"),
        "no other items of the net financial position given \
         (`/closing/net_financial_position/other_items`): the net financial position is gross \
         debt less cash",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w014_no_reserve_balance() {
    assert_warning(
        "ratios.debt_service/partial-schedule.yaml",
        "FM-W014",
        Some("/debt_schedule/reserve_balance"),
        "no reserve balance given (`/debt_schedule/reserve_balance`): nothing is added to the \
         present value of CFADS",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w015_a_write_down_below_ebitda_by_default() {
    assert_warning(
        "statements.reorganise/income-statement-loss.yaml",
        "FM-W015",
        Some("/treatments/write_downs_of_receivables"),
        "year 2024: B10 d (4000) is in other_charges_below_ebitda, below EBITDA, by the default \
         `operating` of `/treatments/write_downs_of_receivables`",
    );
    assert_warning(
        "statements.reorganise/abridged-aggregates.yaml",
        "FM-W015",
        Some("/treatments/write_downs_of_fixed_assets"),
        "year 2024: B10 c (20000) is in depreciation_and_amortisation, below EBITDA",
    );
}

#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn fm_w016_the_evaluated_method_is_preview() {
    assert_warning(
        "analysis.goal_seek/dcf-implied-discount-rate.yaml",
        "FM-W016",
        Some("/method"),
        "Evaluated method dcf.fcff is preview: its results have not been independently verified",
    );
}

/// R9: on every golden result, the conventions echo the warnings once each, as
/// `warning FM-Wnnn: <message>`, in the order of `diagnostics.warnings`, and no other convention
/// starts with `warning`.
#[test]
#[ignore = "SPEC-0012: enabled by T12, which writes FM-W002 to FM-W016"]
fn every_warning_is_echoed_once_in_order() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/golden");
    let mut failures = Vec::new();
    let mut warned = 0u32;
    let mut dirs: Vec<_> = std::fs::read_dir(&root)
        .unwrap()
        .map(|d| d.unwrap().path())
        .collect();
    dirs.sort();
    for dir in dirs {
        let mut files: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|f| f.unwrap().path())
            .collect();
        files.sort();
        for file in files {
            let case: Value =
                serde_norway::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
            if case.get("expect_error").is_some() {
                continue;
            }
            let name = file.display().to_string();
            let out = run(case["method"].as_str().unwrap(), &case["input"]);
            let Some(warnings) = out["diagnostics"]["warnings"].as_array() else {
                failures.push(format!("{name}: no diagnostics.warnings"));
                continue;
            };
            let echoes: Vec<String> = warnings
                .iter()
                .map(|w| {
                    format!(
                        "warning {}: {}",
                        w["code"].as_str().unwrap(),
                        w["message"].as_str().unwrap()
                    )
                })
                .collect();
            let written: Vec<String> = conventions(&out)
                .into_iter()
                .filter(|c| c.to_ascii_lowercase().starts_with("warning"))
                .map(str::to_owned)
                .collect();
            if echoes != written {
                failures.push(format!(
                    "{name}: warnings {echoes:?}, conventions {written:?}"
                ));
            }
            warned += u32::from(!warnings.is_empty());
        }
    }
    assert!(warned > 50, "{warned}");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
