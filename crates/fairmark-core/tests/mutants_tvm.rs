//! Behaviour tests for `tvm` and `rates` written to kill the missed mutants of the first
//! mutation-testing baseline (issue #282, `tests/mutants/triage.md`). Each test cites the
//! requirement or the documented behaviour it checks (SPEC-0002, the methodology pages).
//!
//! Expected values come from sources independent of the engine: the solver iterations and
//! rates from `scripts/reference_solver.py` (Python `decimal`, 50 digits, the solver as the
//! methodology pages document it), rates chosen as the roots of a polynomial (exact by
//! construction), or the documented rules themselves. Explain formulas are checked against the
//! values the same result reports, so that the audit trail substitutes the numbers that were
//! actually used (constitution, principle II).

use fairmark_core::run;
use rust_decimal::Decimal;
use serde_json::{json, Value};

fn ok(method: &str, input: &Value) -> Value {
    run(method, &input.to_string()).unwrap_or_else(|e| panic!("{method}: {e}"))
}

fn err(method: &str, input: &Value) -> (String, String) {
    let e = run(method, &input.to_string()).expect_err("the input must be refused");
    (e.code().to_owned(), e.to_string())
}

fn dec(v: &Value) -> Decimal {
    v.as_str()
        .unwrap_or_else(|| panic!("not a decimal string: {v}"))
        .parse()
        .unwrap()
}

fn steps(out: &Value) -> Vec<(String, String, String)> {
    out["explain"]["steps"]
        .as_array()
        .unwrap()
        .iter()
        .map(|s| {
            (
                s["label"].as_str().unwrap().to_owned(),
                s["formula"].as_str().unwrap().to_owned(),
                s["value"].as_str().unwrap().to_owned(),
            )
        })
        .collect()
}

/// The formula of the step with this exact label.
fn formula(out: &Value, label: &str) -> String {
    steps(out)
        .into_iter()
        .find(|(l, _, _)| l == label)
        .unwrap_or_else(|| panic!("no step {label:?} in {:?}", steps(out)))
        .1
}

fn labels(out: &Value) -> Vec<String> {
    steps(out).into_iter().map(|(l, _, _)| l).collect()
}

fn assert_close(actual: Decimal, expected: &str, tolerance: &str) {
    let expected: Decimal = expected.parse().unwrap();
    let tolerance: Decimal = tolerance.parse().unwrap();
    assert!(
        (actual - expected).abs() <= tolerance,
        "{actual} is not within {tolerance} of {expected}"
    );
}

// ------------------------------------------------------------- IRR solver (SPEC-0002 R8, R9)

/// SPEC-0002 R9, tvm-irr.md "Solver" step 3 and its example (`newton 5`): one root interval,
/// Newton's method from the default guess 0.10 converges inside it. Newton's method converges
/// quadratically, so the documented stopping rule (step at most 1e-18) is met after 5 steps;
/// `scripts/reference_solver.py` gives the same count and rate at 50 digits. A wrong
/// derivative or a rejected Newton result shows as more iterations or as `bisection`.
#[test]
fn irr_documented_example_converges_by_newton_in_five_iterations() {
    let out = ok(
        "tvm.irr",
        &json!({"cash_flows": ["-1000", "300", "400", "500"]}),
    );
    let v = &out["value"];
    assert_eq!(v["solver"], "newton");
    assert_eq!(v["iterations"], 5);
    assert_eq!(v["uniqueness_verified"], true);
    assert_close(
        dec(&v["irr"]),
        "0.088963394693349935317765679686868942760827791027568",
        "1e-26",
    );
    assert_eq!(
        formula(&out, "Internal rate of return"),
        "newton, 5 iterations"
    );
}

/// SPEC-0002 R9, tvm-irr.md "Solver" step 3: with no root interval on the grid and one sign
/// change, Newton's method from the guess is accepted when it converges. The flows -1, 20 have
/// the single rate 19 (1 + r = 20), outside the grid [-0.99, 10]: no root is isolated, and the
/// rate is still returned. Reference: 11 Newton iterations from 0.10
/// (`scripts/reference_solver.py`).
#[test]
fn irr_outside_the_grid_is_found_by_newton_from_the_guess() {
    let out = ok("tvm.irr", &json!({"cash_flows": ["-1", "20"]}));
    let v = &out["value"];
    assert_eq!(v["solver"], "newton");
    assert_eq!(v["iterations"], 11);
    assert_eq!(v["uniqueness_verified"], true);
    assert_close(dec(&v["irr"]), "19", "1e-24");
    // No root interval or grid root is reported: the grid isolated none.
    assert!(
        !labels(&out).iter().any(|l| l.starts_with("Root")),
        "{:?}",
        labels(&out)
    );
}

/// SPEC-0002 R9, tvm-irr.md "Solver" step 3: Newton's result is accepted only inside the root
/// interval. The flows are the coefficients of -(x − 1.11)(x − 1.12)(x − 1.53), x = 1 + r, so
/// the rates are 0.11, 0.12 and 0.53 exactly. The grid isolates 0.53 in [0.50, 0.55] (the pair
/// 0.11, 0.12 lies inside one grid step); Newton's method from 0.10 converges to 0.11, below
/// the interval (`scripts/reference_solver.py`), so bisection inside the interval gives 0.53.
#[test]
fn irr_newton_result_below_the_interval_falls_back_to_bisection() {
    let out = ok(
        "tvm.irr",
        &json!({"cash_flows": ["-1", "3.76", "-4.6551", "1.902096"]}),
    );
    let v = &out["value"];
    assert_eq!(v["solver"], "bisection");
    assert_eq!(v["uniqueness_verified"], false);
    assert_close(dec(&v["irr"]), "0.53", "1e-17");
}

/// As above with the pair above the interval: rates 0.21, 0.61 and 0.62 exactly (flows of
/// -(x − 1.21)(x − 1.61)(x − 1.62)). The grid isolates 0.21 in [0.20, 0.25]; Newton's method
/// from the guess 0.60 converges to 0.61, above the interval, so bisection gives 0.21.
#[test]
fn irr_newton_result_above_the_interval_falls_back_to_bisection() {
    let out = ok(
        "tvm.irr",
        &json!({"cash_flows": ["-1", "4.44", "-6.5165", "3.155922"], "guess": "0.60"}),
    );
    let v = &out["value"];
    assert_eq!(v["solver"], "bisection");
    assert_eq!(v["uniqueness_verified"], false);
    assert_close(dec(&v["irr"]), "0.21", "1e-17");
}

/// The root interval is closed: a Newton result equal to its lower end is inside it and
/// accepted (tvm-irr.md "Solver" step 3). The rate of -5,000,000 then
/// 7,500,000.0000000000000000000001 is 0.5 + 2 × 10^-29, which is 0.5 to the 28 decimal places
/// of the engine; the grid sees NPV(0.50) > 0 and NPV(0.55) < 0.
#[test]
fn irr_newton_result_on_the_lower_end_of_the_interval_is_accepted() {
    let out = ok(
        "tvm.irr",
        &json!({"cash_flows": ["-5000000", "7500000.0000000000000000000001"]}),
    );
    let v = &out["value"];
    assert_eq!(
        dec(&out["explain"]["steps"][0]["value"]),
        Decimal::new(50, 2)
    );
    assert_eq!(
        out["explain"]["steps"][0]["label"],
        "Root interval on the grid, lower rate"
    );
    assert_eq!(v["solver"], "newton");
    assert_eq!(dec(&v["irr"]), Decimal::new(5, 1));
}

/// As above at the upper end: the rate of -1,000,000 then 1,549,999.9999999999999999999998 is
/// 0.55 − 2 × 10^-28, which the engine's Newton iteration returns as 0.55, the upper end of
/// the interval [0.50, 0.55]; it is accepted.
#[test]
fn irr_newton_result_on_the_upper_end_of_the_interval_is_accepted() {
    let out = ok(
        "tvm.irr",
        &json!({"cash_flows": ["-1000000", "1549999.9999999999999999999998"]}),
    );
    let v = &out["value"];
    assert_eq!(
        out["explain"]["steps"][1]["label"],
        "Root interval on the grid, upper rate"
    );
    assert_eq!(
        dec(&out["explain"]["steps"][1]["value"]),
        Decimal::new(55, 2)
    );
    assert_eq!(v["solver"], "newton");
    assert_eq!(dec(&v["irr"]), Decimal::new(55, 2));
}

/// SPEC-0002 R10 (MIRR) and the IRR input rule: the flows need a positive and a negative
/// amount; a zero is neither, so flows of one sign and zeros are refused with `FM-E006`.
#[test]
fn zero_flows_are_neither_positive_nor_negative() {
    for flows in [json!(["-100", "0", "0"]), json!(["100", "0", "0"])] {
        let (code, _) = err(
            "tvm.mirr",
            &json!({"cash_flows": flows, "finance_rate": "0.10", "reinvest_rate": "0.12"}),
        );
        assert_eq!(code, "FM-E006", "MIRR {flows}");
        let (code, _) = err("tvm.irr", &json!({"cash_flows": flows}));
        assert_eq!(code, "FM-E006", "IRR {flows}");
    }
}

// ------------------------------------------------------------- XIRR solver (SPEC-0002 R8)

fn xirr_flows() -> Value {
    json!([
        {"date": "2008-01-01", "amount": "-10000"},
        {"date": "2008-03-01", "amount": "2750"},
        {"date": "2008-10-30", "amount": "4250"},
        {"date": "2009-02-15", "amount": "3250"},
        {"date": "2009-04-01", "amount": "2750"}
    ])
}

/// SPEC-0002 R8 and the example of tvm-xirr.md (`newton 6 1E-18`): the same deterministic
/// solver as IRR, reporting solver, iterations and tolerance. Reference at 50 digits:
/// 0.37336253351883151030845541191 in 6 Newton iterations (`scripts/reference_solver.py`,
/// Actual/365 Fixed, annual compounding).
#[test]
fn xirr_documented_example_converges_by_newton_in_six_iterations() {
    let out = ok("tvm.xirr", &json!({"cash_flows": xirr_flows()}));
    let v = &out["value"];
    assert_eq!(v["solver"], "newton");
    assert_eq!(v["iterations"], 6);
    assert_eq!(dec(&v["tolerance"]), "1e-18".parse::<Decimal>().unwrap());
    assert_close(dec(&v["xirr"]), "0.37336253351883151030845541191", "1e-18");
}

/// SPEC-0002 R3, R8: XIRR with continuous compounding solves Σ A_i e^(−r t_i) = 0 with the same
/// solver. Reference at 50 digits: 0.31726213673828026851126103301 in 6 Newton iterations
/// (`scripts/reference_solver.py`); it equals ln(1 + 0.37336...), the annual rate above.
#[test]
fn xirr_continuous_compounding_converges_by_newton_in_six_iterations() {
    let out = ok(
        "tvm.xirr",
        &json!({"cash_flows": xirr_flows(), "compounding": "continuous"}),
    );
    let v = &out["value"];
    assert_eq!(v["solver"], "newton");
    assert_eq!(v["iterations"], 6);
    assert_close(dec(&v["xirr"]), "0.31726213673828026851126103301", "1e-18");
}

/// As above with monthly compounding: Newton's method on (1 + r/12)^(−12 t) needs the
/// derivative −t (1 + r/12)^(−12 t − 1). Reference at 50 digits: 0.32149331240979578460749799185
/// in 6 Newton iterations (`scripts/reference_solver.py`), the annual rate above restated
/// monthly, 12 × ((1.37336...)^(1/12) − 1).
#[test]
fn xirr_monthly_compounding_converges_by_newton_in_six_iterations() {
    let out = ok(
        "tvm.xirr",
        &json!({"cash_flows": xirr_flows(), "compounding": "monthly"}),
    );
    let v = &out["value"];
    assert_eq!(v["solver"], "newton");
    assert_eq!(v["iterations"], 6);
    assert_close(dec(&v["xirr"]), "0.32149331240979578460749799185", "1e-18");
}

// ------------------------------------------------------------- present value (SPEC-0002 R5)

/// SPEC-0002 R5: the discount factor of period k is the product of (1 + r_i)^-1 for i ≤ k, and
/// the explain shows it as DF_(k−1) / (1 + r_k) with the factor of the previous period
/// substituted (constitution, principle II).
#[test]
fn present_value_explain_chains_the_discount_factors() {
    let out = ok(
        "tvm.present_value",
        &json!({
            "cash_flows": ["100", "100", "100"],
            "period_rates": ["0.05", "0.06", "0.07"],
            "timing": "end_of_period"
        }),
    );
    let dfs: Vec<String> = out["value"]["discount_factors"]
        .as_array()
        .unwrap()
        .iter()
        .map(|d| d.as_str().unwrap().to_owned())
        .collect();
    assert_eq!(formula(&out, "Period 1: discount factor"), "1 / (1 + 0.05)");
    assert_eq!(
        formula(&out, "Period 2: discount factor"),
        format!("{} / (1 + 0.06)", dfs[0])
    );
    assert_eq!(
        formula(&out, "Period 3: discount factor"),
        format!("{} / (1 + 0.07)", dfs[1])
    );
    // DF_2 = 1 / (1.05 × 1.06), the value of the methodology page.
    assert_eq!(dfs[1], "0.8984725965858041329739442947");
}

/// SPEC-0002 R5: one rate per period; a curve of another length is refused and the message
/// states the count of rates and of flows.
#[test]
fn present_value_period_rates_of_the_wrong_length_name_both_counts() {
    let (code, message) = err(
        "tvm.present_value",
        &json!({
            "cash_flows": ["100", "100", "100"],
            "period_rates": ["0.05", "0.06"],
            "timing": "end_of_period"
        }),
    );
    assert_eq!(code, "FM-E001");
    assert!(
        message.contains("period_rates has 2 rates for 3 cash flows"),
        "{message}"
    );
}

// ------------------------------------------------------------- XNPV explain (SPEC-0002 R6)

/// SPEC-0002 R6, R13: each discount factor of XNPV is written out with the rate, the
/// compounding and the year fraction substituted: 1 / (1 + r)^t, 1 / (1 + r / m)^(m × t) or
/// e^(−r × t).
#[test]
fn xnpv_explain_writes_out_each_discount_factor() {
    let flows = json!([
        {"date": "2024-01-01", "amount": "-100"},
        {"date": "2024-12-31", "amount": "110"}
    ]);
    for (compounding, rate, expected) in [
        ("annual", "0.10", "1 / (1 + 0.10)^{t}"),
        ("monthly", "0.12", "1 / (1 + 0.12 / 12)^(12 × {t})"),
        ("continuous", "0.05", "e^(−0.05 × {t})"),
    ] {
        let out = ok(
            "tvm.xnpv",
            &json!({"rate": rate, "cash_flows": flows, "compounding": compounding}),
        );
        let t = out["value"]["flows"][1]["year_fraction"].as_str().unwrap();
        assert_eq!(
            dec(&out["value"]["flows"][1]["year_fraction"]),
            Decimal::ONE
        );
        assert_eq!(
            formula(&out, "Discount factor, 2024-12-31"),
            expected.replace("{t}", t),
            "{compounding}"
        );
    }
}

// ------------------------------------------------------------- annuity factor (SPEC-0002 R11)

/// tvm-annuity.md: the direct summation applies when |r − g| < 10^-9; at |r − g| = 10^-9
/// exactly the closed form is used.
#[test]
fn annuity_closed_form_applies_at_a_rate_gap_of_exactly_one_billionth() {
    let at = ok(
        "tvm.annuity",
        &json!({"payment": "1", "rate": "0.05", "growth": "0.049999999", "periods": 10,
                "timing": "ordinary"}),
    );
    assert_eq!(
        formula(&at, "Annuity factor"),
        "[1 − ((1 + 0.049999999) / (1 + 0.05))^10] / (0.05 − 0.049999999)"
    );
    let below = ok(
        "tvm.annuity",
        &json!({"payment": "1", "rate": "0.05", "growth": "0.0499999991", "periods": 10,
                "timing": "ordinary"}),
    );
    assert!(
        labels(&below).contains(&"Annuity factor (r close to g, direct sum)".to_owned()),
        "{:?}",
        labels(&below)
    );
}

/// tv-limited-horizon.md and tvm-annuity.md "Long horizons": a growth-to-discount ratio above 1
/// raised to a long horizon leaves the decimal range; that is an overflow (`FM-E008`), never a
/// value taken as 0 (only a ratio below 1 tends to 0).
#[test]
fn limited_horizon_growth_above_rate_overflows_at_a_long_horizon() {
    let (code, _) = err(
        "tv.limited_horizon",
        &json!({"flow": "1", "rate": "0", "growth": "1", "periods": 10000}),
    );
    assert_eq!(code, "FM-E008");
}

/// tvm-annuity.md "Long horizons": `tvm.annuity` computes the future value too, so a long
/// horizon at 50% per period is `FM-E008` there.
#[test]
fn annuity_long_horizon_is_an_overflow() {
    let (code, _) = err(
        "tvm.annuity",
        &json!({"payment": "1", "rate": "0.5", "periods": 10000, "timing": "ordinary"}),
    );
    assert_eq!(code, "FM-E008");
}

// ------------------------------------------------------------- future value (SPEC-0009)

/// tvm-future-value.md: `periods` from 1 to 10,000; both ends are accepted, 0 and 10,001 are
/// refused with `FM-E001`. With the rate zero the value is PV + n × A whatever n is.
#[test]
fn future_value_accepts_one_to_ten_thousand_periods() {
    for (periods, expected) in [(1u32, "100"), (10_000, "100")] {
        let out = ok(
            "tvm.future_value",
            &json!({"present_value": "100", "rate": "0", "periods": periods,
                    "timing": "ordinary"}),
        );
        assert_eq!(dec(&out["value"]["future_value"]), dec(&json!(expected)));
    }
    for periods in [0u32, 10_001] {
        let (code, message) = err(
            "tvm.future_value",
            &json!({"present_value": "100", "rate": "0", "periods": periods,
                    "timing": "ordinary"}),
        );
        assert_eq!(code, "FM-E001", "{periods}");
        assert!(message.contains("between 1 and 10000"), "{message}");
    }
}

/// tvm-future-value.md: the payments accumulate with the level annuity factor, written with the
/// level formulas: [1 − (1 + r)^−n] / r, and Σ_(k=1..n) (1 + r)^−k by direct summation when the
/// rate is within 10^-9 of 0.
#[test]
fn future_value_reports_the_level_annuity_factor() {
    let out = ok(
        "tvm.future_value",
        &json!({"payment": "100", "rate": "0.05", "periods": 10, "timing": "ordinary"}),
    );
    assert_eq!(
        formula(&out, "Annuity factor (level payments)"),
        "[1 − (1 + 0.05)^−10] / 0.05"
    );
    let near_zero = ok(
        "tvm.future_value",
        &json!({"payment": "100", "rate": "0.0000000001", "periods": 10, "timing": "ordinary"}),
    );
    assert_eq!(
        formula(
            &near_zero,
            "Annuity factor (level payments, rate close to 0, direct sum)"
        ),
        "Σ_(k=1..10) (1 + 0.0000000001)^−k"
    );
}

// ------------------------------------------------------------- rates.convert (SPEC-0002 R3)

/// SPEC-0002 R3: the conversion shows the growth over one year with the given rate and its
/// compounding substituted, then the equivalent rate written from that growth factor.
#[test]
fn rates_convert_explain_writes_out_both_conversions() {
    for (from, to, growth, inverse) in [
        ("monthly", "annual", "(1 + 0.12 / 12)^12", "{g} − 1"),
        ("annual", "quarterly", "1 + 0.12", "4 × ({g}^(1/4) − 1)"),
        ("annual", "continuous", "1 + 0.12", "ln({g})"),
        ("continuous", "annual", "e^0.12", "{g} − 1"),
    ] {
        let out = ok(
            "rates.convert",
            &json!({"rate": "0.12", "from": from, "to": to}),
        );
        let g = out["value"]["annual_growth_factor"].as_str().unwrap();
        assert_eq!(formula(&out, "Growth over one year"), growth, "{from}");
        assert_eq!(
            formula(&out, "Equivalent rate"),
            inverse.replace("{g}", g),
            "{from} to {to}"
        );
    }
}

// ------------------------------------------------------------- defects found by the triage

/// tv-limited-horizon.md: when ((1 + g)/(1 + r))^n falls below the decimal precision "it is
/// taken as 0, its limit, and an explain step says so". At 50% per period over 10,000 periods
/// the ratio (1/1.5)^10000 is far below 10^-28: the factor is 1 / 0.5 = 2 and the step is
/// written with the numbers substituted. At 10 periods the power is well inside the precision
/// and no such step is written. Defect found by the triage of issue #282.
#[test]
fn limited_horizon_reports_the_power_taken_as_zero() {
    let out = ok(
        "tv.limited_horizon",
        &json!({"flow": "1", "rate": "0.5", "periods": 10000}),
    );
    assert_eq!(dec(&out["value"]["annuity_factor"]), Decimal::TWO);
    let label = "Growth-to-discount ratio to the power n (below the decimal precision, taken as 0)";
    assert_eq!(formula(&out, label), "((1 + 0) / (1 + 0.5))^10000");
    let short = ok(
        "tv.limited_horizon",
        &json!({"flow": "1", "rate": "0.5", "periods": 10}),
    );
    assert!(
        !labels(&short).contains(&label.to_owned()),
        "{:?}",
        labels(&short)
    );
}

/// `tvm::discount_factor` (public Rust API) is documented as `1 / (1 + rate)^t`. For t = −163 at
/// 50% the value is 1.5^163 = 5.0451632789815171509443825809 × 10^28 (Python `decimal`, 50
/// digits), inside the decimal range; SPEC-0002 does not restrict t, so the value is computed,
/// never 0. Beyond the decimal range (t = −200) it is `FM-E008`. Defect found by the triage of
/// issue #282.
#[test]
fn discount_factor_at_a_negative_time_is_the_compounding_factor() {
    let df = fairmark_core::tvm::discount_factor(Decimal::new(5, 1), Decimal::from(-163))
        .expect("inside the decimal range");
    assert_close(
        df,
        "50451632789815171509443825808.995501445114117692757",
        "1e10",
    );
    let e = fairmark_core::tvm::discount_factor(Decimal::new(5, 1), Decimal::from(-200))
        .expect_err("beyond the decimal range");
    assert_eq!(e.code(), "FM-E008");
    // At a negative rate the compounding factor is below 1: (1 − 0.5)^2 = 0.25.
    assert_eq!(
        fairmark_core::tvm::discount_factor(Decimal::new(-5, 1), Decimal::from(-2)).unwrap(),
        Decimal::new(25, 2)
    );
}
