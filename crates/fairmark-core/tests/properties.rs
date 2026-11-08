//! Property tests (issue #33): identities that must hold for every input of a family, checked
//! on generated inputs with `proptest` (SPEC-0002 time value foundations, SPEC-0009 loans and
//! future value).
//!
//! Determinism of the suite itself. Every property runs through [`check`], which builds a
//! `TestRunner` directly from [`config`]: a fixed number of cases, a fixed seed, the ChaCha
//! generator and no failure persistence. The `proptest!` macro is not used, because it passes
//! its configuration through `contextualize_config`, where environment variables such as
//! `PROPTEST_CASES` or `PROPTEST_RNG_SEED` override it; `TestRunner::new` reads no environment
//! variable, and [`config`] sets every field that one could change. So two runs generate the
//! same inputs and the suite writes no files; `case_count_ignores_environment` checks it.
//!
//! Tolerances. The engine computes with `rust_decimal` (96-bit mantissa, 28 to 29 significant
//! digits), so one operation has a relative rounding error of at most about 1e-28. Each
//! tolerance below is derived from that precision, from the solver tolerance (1e-18 on the
//! rate) or from a documented engine tolerance, with the derivation next to the constant. All
//! arithmetic in this file is decimal as well: no binary floating point is involved.

use std::fmt::Debug;
use std::str::FromStr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Mutex;

use fairmark_core::dates::{Date, DayCount};
use fairmark_core::loans::{self, InstalmentInput, NumberOfInstalmentsInput};
use fairmark_core::rates::{self, Compounding, ConvertInput, FisherDirection, RealNominalInput};
use fairmark_core::tvm::{
    self, AnnuityInput, AnnuityTiming, DatedCashFlow, FutureValueInput, IrrInput, MirrInput,
    NpvInput, XirrInput, XnpvInput,
};
use fairmark_core::FairmarkError;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed, TestCaseResult, TestError, TestRunner};
use rust_decimal::{Decimal, MathematicalOps};
use serde_json::{Map, Value};

// ---------------------------------------------------------------- runner

/// Runner configuration: `cases` generated inputs from the fixed `seed`. Every field that a
/// `PROPTEST_*` environment variable can set is given here, so the environment values that
/// `Config::default()` picks up are all replaced (the `fork` and `timeout` features, which
/// have variables of their own, are not enabled).
fn config(cases: u32, seed: u64) -> Config {
    Config {
        cases,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        max_local_rejects: 65_536,
        max_global_rejects: 1_024,
        max_flat_map_regens: 1_000_000,
        max_shrink_iters: 1_024,
        max_shrink_time: 0,
        max_default_size_range: 100,
        verbose: 0,
        ..Config::default()
    }
}

/// Inputs accepted and rejected (by `prop_assume!`) in one run of a property.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Stats {
    passed: u32,
    rejected: u32,
}

/// Runs `body` on `cases` inputs drawn from `strategy` with the fixed `seed`; panics with the
/// minimal failing input if the property fails, or if the run is aborted (too many rejects).
fn check<S>(
    name: &str,
    cases: u32,
    seed: u64,
    strategy: S,
    body: impl Fn(S::Value) -> TestCaseResult,
) -> Stats
where
    S: Strategy,
    S::Value: Debug,
{
    let (passed, rejected) = (AtomicU32::new(0), AtomicU32::new(0));
    let mut runner = TestRunner::new(config(cases, seed));
    let result = runner.run(&strategy, |value| {
        let r = body(value);
        match &r {
            Ok(()) => passed.fetch_add(1, Ordering::Relaxed),
            Err(TestCaseError::Reject(_)) => rejected.fetch_add(1, Ordering::Relaxed),
            Err(TestCaseError::Fail(_)) => 0,
        };
        r
    });
    match result {
        Ok(()) => {}
        Err(TestError::Fail(reason, input)) => {
            panic!("property {name} failed: {reason}\nminimal failing input: {input:#?}")
        }
        Err(TestError::Abort(reason)) => panic!("property {name} aborted: {reason}"),
    }
    let stats = Stats {
        passed: passed.into_inner(),
        rejected: rejected.into_inner(),
    };
    eprintln!(
        "property {name}: {} cases passed, {} rejected",
        stats.passed, stats.rejected
    );
    stats
}

/// The values drawn and the stats of a small probe run: 40 cases, a quarter of the draws
/// rejected.
fn probe() -> (Vec<u64>, Stats) {
    let seen = Mutex::new(Vec::new());
    let stats = check("probe", 40, 0x3300, any::<u64>(), |v| {
        prop_assume!(v % 4 != 0);
        seen.lock().unwrap().push(v);
        Ok(())
    });
    (seen.into_inner().unwrap(), stats)
}

/// The case count, the seed and the reject limits do not come from the environment: the same
/// probe run in a child process with hostile `PROPTEST_*` variables draws the same values.
#[test]
fn case_count_ignores_environment() {
    let (values, stats) = probe();
    let line = format!("probe-result {} {:?} {:?}", stats.passed, stats, values);
    if std::env::var_os("FAIRMARK_PROPERTY_PROBE").is_some() {
        println!("{line}");
        return;
    }
    assert_eq!(stats.passed, 40);
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "case_count_ignores_environment",
            "--exact",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("FAIRMARK_PROPERTY_PROBE", "1")
        .env("PROPTEST_CASES", "3")
        .env("PROPTEST_RNG_SEED", "7")
        .env("PROPTEST_RNG_ALGORITHM", "xs")
        .env("PROPTEST_MAX_GLOBAL_REJECTS", "0")
        .env("PROPTEST_MAX_LOCAL_REJECTS", "0")
        .env("PROPTEST_MAX_FLAT_MAP_REGENS", "0")
        .env("PROPTEST_MAX_SHRINK_ITERS", "0")
        .env("PROPTEST_MAX_DEFAULT_SIZE_RANGE", "1")
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(out.status.success(), "child run failed:\n{stdout}");
    // The test harness may print its own text on the same line, before the result.
    let child = stdout
        .lines()
        .find_map(|l| l.find("probe-result ").map(|i| &l[i..]))
        .unwrap_or_else(|| panic!("no probe result in:\n{stdout}"));
    assert_eq!(child, line, "the environment changed the probe run");
}

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

/// Decimal from an integer mantissa and a scale: `dec(5, 2)` is 0.05.
fn dec(mantissa: i64, scale: u32) -> Decimal {
    Decimal::new(mantissa, scale)
}

// ---------------------------------------------------------------- tolerances

/// |NPV(IRR)| and |XNPV(XIRR)| relative to Σ |CF_t| × DF(IRR, t), the size of the discounted
/// terms that are summed.
///
/// The solver stops when its last Newton step, or its bisection interval, is at most 1e-18 on
/// the rate, so the rate is within about 1e-18 of the exact root. The generated flows keep the
/// root above -0.5 and have at most 28 periods (or 23 years for dated flows), so
/// |dNPV/dr| ≤ Σ t |CF_t| DF_t / (1 + r) ≤ 28 × 2 × scale = 56 × scale, and the rate error moves
/// NPV by at most 5.6e-17 × scale. Rounding adds at most (number of operations) × 1e-28 ×
/// scale, below 1e-24 × scale. 1e-16 × scale bounds the sum with a margin of about two.
const ROOT_RESIDUAL: &str = "1e-16";

/// Round trip of `rates.convert`: acceptance criterion AC3 of SPEC-0002, 1e-20 absolute.
const CONVERT_ROUND_TRIP: &str = "1e-20";

/// Round trip of `rates.real_nominal` (to real, then to nominal with the same inflation):
/// acceptance criterion AC3 of SPEC-0002, 1e-20 absolute. The round trip is one division and
/// one product, each rounded to 28 significant digits: (1 + n) / (1 + i) is at most 2 / 1e-6
/// here, so its rounding is below 1e-21, and multiplying by 1 + i (at most 2) keeps the error
/// far below the bound.
const FISHER_ROUND_TRIP: &str = "1e-20";

/// Stricter guard on the same round trip, so that a loss of precision well inside the AC3 bound
/// is noticed. The round trip is at most two growth factors ((1 + r/m)^m by squaring, m ≤ 12,
/// or `exp`), two `ln` and two `exp` (`rust_decimal` series, which stop when a term no longer
/// changes the 28-digit sum, so each is good to a few units of 1e-28 for arguments of order
/// one) and a few products: a few tens of roundings of 1e-28 on values of order one, amplified
/// at most by m = 12 in m × (root − 1). 1e-24 leaves a margin of about ten above that.
const CONVERT_ROUND_TRIP_STRICT: &str = "1e-24";

/// Relative tolerance for identities against an explicit sum (annuity, future value). The
/// engine uses g = (1 + r)^n or its reciprocal 1 / g, computed by exponentiation by squaring:
/// a rounding of relative size u in the partial power x^(2^k) reaches the result multiplied
/// by n / 2^k, so the power carries a relative error up to about n × u (not u); the test's own
/// sum by repeated multiplication has n roundings as well. u is 1e-28 for values of order one
/// or more and 1e-28 / v for a value of order v < 1 (a decimal has at most 28 decimal places).
/// Hence 2 × (n + 50) × 1e-28 × max(g, 1 / g), see `closed_form_tolerance`: about 1.3e-25 for
/// growing amounts over 600 periods, at most 1.3e-19 for the most discounted ones (the
/// generated problems keep g in [1e-6, 1e6]).
const ROUNDING: &str = "1e-28";

/// Absolute floor added to the relative tolerances above. A decimal has at most 28 decimal
/// places, so a small intermediate value (an amount of 0.01 discounted by 1e-6) is resolved
/// only to 1e-28 in absolute terms, not to 28 significant digits; the generated problems grow
/// or discount by at most 1e6, which amplifies that resolution to 1e-22. 1e-21 keeps a margin
/// of ten.
const ABSOLUTE_FLOOR: &str = "1e-21";

/// MIRR definition (1 + MIRR)^n × (−PV) = FV, relative. MIRR is computed as
/// exp(ln(FV / −PV) / n) − 1: one quotient, one `ln` and one `exp` (each good to a few units of
/// 1e-28 relative for the arguments here, ratios between about 0.003 and 3e7), so 1 + MIRR is
/// within about 1e-27 relative. Raising it to the power n ≤ 27 multiplies that error by n and
/// adds n roundings: about 27 × 1e-27 + 27 × 1e-28 ≈ 3e-26. 1e-24 keeps a margin of about
/// thirty.
const MIRR_RELATIVE: &str = "1e-24";

/// MIRR at finance rate = reinvestment rate = IRR equals IRR: the IRR is known to about 1e-18
/// (solver tolerance); this error passes to MIRR with a factor of order one (at most 2 for the
/// generated flows), and the MIRR computation adds 1e-24. 1e-17 bounds the sum.
const MIRR_AT_IRR: &str = "1e-17";

// ---------------------------------------------------------------- generators

/// Periodic flows with exactly one sign change: one to three outflows at t = 0, 1, 2, then one
/// to twenty-five inflows, every amount non-zero with two decimals. The outflows are scaled so
/// that their sum is u × (sum of the inflows), u between 0.10 and 1.90. Then NPV(-0.5) > 0
/// (each inflow is worth at least twice as much relative to the outflows, and u < 2) and
/// NPV(10) < 0 (u > 1/11), so the unique root lies in (-0.5, 10), inside the IRR grid.
fn conventional_flows() -> impl Strategy<Value = Vec<Decimal>> {
    (
        prop::collection::vec(1i64..=100_000_000, 1..=3),
        prop::collection::vec(1i64..=100_000_000, 1..=25),
        10i64..=190,
    )
        .prop_map(|(neg, pos, u)| {
            let pos: Vec<Decimal> = pos.into_iter().map(|c| dec(c, 2)).collect();
            let sum_pos: Decimal = pos.iter().sum();
            let raw: Vec<Decimal> = neg.into_iter().map(Decimal::from).collect();
            let sum_raw: Decimal = raw.iter().sum();
            let target = sum_pos * dec(u, 2);
            let mut flows: Vec<Decimal> = raw
                .iter()
                .map(|n| -(n * target / sum_raw).round_dp(2).max(dec(1, 2)))
                .collect();
            flows.extend(pos);
            flows
        })
}

/// Flows with an outflow at t = 0 and non-negative later flows, at least one of them positive:
/// NPV is then strictly decreasing in the rate on (-1, ∞).
fn outflow_then_inflows() -> impl Strategy<Value = Vec<Decimal>> {
    (
        -100_000_000i64..=0,
        prop::collection::vec(0i64..=100_000_000, 1..=30),
        1i64..=100_000_000,
    )
        .prop_map(|(c0, later, last)| {
            let mut flows = vec![dec(c0, 2)];
            flows.extend(later.into_iter().map(|c| dec(c, 2)));
            flows.push(dec(last, 2));
            flows
        })
}

/// A rate per period from -0.9000 to 1.0000 in steps of 0.0001, with tiny rates (scale 12)
/// mixed in so that the branches near zero are exercised.
fn rate() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        4 => (-9_000i64..=10_000).prop_map(|m| dec(m, 4)),
        1 => (-1_000i64..=1_000).prop_map(|m| dec(m, 12)),
    ]
}

/// A rate per period from -0.5000 to 1.0000, or tiny: with at most 32 periods the growth
/// (1 + r)^t stays below 2^32, so NPV of amounts up to 1e6 cannot overflow. Below -0.5 and over
/// long horizons the discount factor exceeds the decimal range and NPV returns `FM-E008`, a
/// typed error by design, which is not what these properties test.
fn moderate_rate() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        4 => (-5_000i64..=10_000).prop_map(|m| dec(m, 4)),
        1 => (-1_000i64..=1_000).prop_map(|m| dec(m, 12)),
    ]
}

/// A non-negative rate from 0 to 1, in steps of 0.0001 or tiny (scale 12).
fn positive_rate() -> impl Strategy<Value = Decimal> {
    prop_oneof![
        4 => (0i64..=10_000).prop_map(|m| dec(m, 4)),
        1 => (0i64..=1_000).prop_map(|m| dec(m, 12)),
    ]
}

/// The largest n ≤ `cap` such that (1 + r)^n stays within [1e-6, 1e6]: beyond that the
/// problems below are ill-conditioned at 28 digits (an error of 1e-28 in one input grows by
/// (1 + r)^n), and the identities would test the arithmetic rather than the engine.
fn well_conditioned_periods(r: Decimal, cap: u32) -> u32 {
    let (lo, hi) = (d("0.000001"), d("1000000"));
    let one_r = Decimal::ONE + r;
    let mut g = Decimal::ONE;
    for n in 1..=cap {
        g *= one_r;
        if g > hi || g < lo {
            return (n - 1).max(1);
        }
    }
    cap
}

/// A rate and a number of periods with (1 + r)^n within [1e-6, 1e6].
fn rate_and_periods(cap: u32) -> impl Strategy<Value = (Decimal, u32)> {
    rate().prop_flat_map(move |r| {
        let max = well_conditioned_periods(r, cap);
        (Just(r), 1..=max)
    })
}

/// A positive payment, 0.01 to 1,000,000.00.
fn payment() -> impl Strategy<Value = Decimal> {
    (1i64..=100_000_000).prop_map(|m| dec(m, 2))
}

/// Two rates and a number of periods with (1 + r)^n within [1e-6, 1e6] for both rates.
fn two_rates_and_periods(cap: u32) -> impl Strategy<Value = (Decimal, Decimal, u32)> {
    (rate(), rate()).prop_flat_map(move |(a, b)| {
        let max = well_conditioned_periods(a, cap).min(well_conditioned_periods(b, cap));
        (Just(a), Just(b), 1..=max)
    })
}

/// A rate and two numbers of periods n < m ≤ n + `extra`, with (1 + r)^m within [1e-6, 1e6].
/// Every rate from `rate()` allows at least six periods ((0.1)^6 = 1e-6), so n can be drawn.
fn rate_and_two_periods(cap: u32, extra: u32) -> impl Strategy<Value = (Decimal, u32, u32)> {
    rate()
        .prop_flat_map(move |r| {
            let max = well_conditioned_periods(r, cap);
            (Just(r), 1..max, Just(max))
        })
        .prop_flat_map(move |(r, n, max)| (Just(r), Just(n), n + 1..=(n + extra).min(max)))
}

const COMPOUNDINGS: [Compounding; 5] = [
    Compounding::Annual,
    Compounding::SemiAnnual,
    Compounding::Quarterly,
    Compounding::Monthly,
    Compounding::Continuous,
];

fn timing() -> impl Strategy<Value = AnnuityTiming> {
    prop_oneof![Just(AnnuityTiming::Ordinary), Just(AnnuityTiming::Due)]
}

fn compounding() -> impl Strategy<Value = Compounding> {
    prop_oneof![
        Just(Compounding::Annual),
        Just(Compounding::SemiAnnual),
        Just(Compounding::Quarterly),
        Just(Compounding::Monthly),
        Just(Compounding::Continuous),
    ]
}

fn day_count() -> impl Strategy<Value = DayCount> {
    prop_oneof![
        Just(DayCount::Act365Fixed),
        Just(DayCount::Act360),
        Just(DayCount::ActActIsda),
        Just(DayCount::Thirty360),
        Just(DayCount::ThirtyE360),
    ]
}

/// Dated flows with one sign change whose exact XNPV root is a chosen rate r*: inflows on
/// strictly increasing dates (day of month at most 28, one to eighteen months apart, so that
/// no two dates share a year fraction under any day count), and one outflow on the first date
/// equal to the inflows discounted at r*, rounded to cents. XNPV is then strictly decreasing
/// in the rate with its unique root next to r* in [-0.30, 2.00].
#[derive(Debug, Clone)]
struct DatedCase {
    flows: Vec<DatedCashFlow>,
    day_count: DayCount,
    compounding: Compounding,
}

fn dated_case() -> impl Strategy<Value = DatedCase> {
    (
        (1990i32..=2100, 1u32..=12, 1u32..=28),
        prop::collection::vec((1u32..=18, 1u32..=28, 1i64..=100_000_000), 1..=15),
        -3_000i64..=20_000,
        day_count(),
        compounding(),
    )
        .prop_map(|((y, m, day), later, r_star, dc, c)| {
            let start = Date::new(y, m, day).unwrap();
            let r_star = dec(r_star, 4);
            let mut months = (y * 12) + (m as i32 - 1);
            let mut flows = Vec::with_capacity(later.len() + 1);
            let mut outflow = Decimal::ZERO;
            for (gap, day, amount) in later {
                months += gap as i32;
                let date = Date::new(months / 12, (months % 12) as u32 + 1, day).unwrap();
                let amount = dec(amount, 2);
                let (t, _) = dc.year_fraction(start, date).unwrap();
                outflow += amount * rates::discount_factor(r_star, c, t).unwrap();
                flows.push(DatedCashFlow { date, amount });
            }
            let outflow = outflow.round_dp(2).max(dec(1, 2));
            flows.insert(
                0,
                DatedCashFlow {
                    date: start,
                    amount: -outflow,
                },
            );
            DatedCase {
                flows,
                day_count: dc,
                compounding: c,
            }
        })
}

// ---------------------------------------------------------------- helpers

fn npv(rate: Decimal, flows: &[Decimal]) -> Decimal {
    tvm::npv(&NpvInput {
        rate,
        cash_flows: flows.to_vec(),
    })
    .unwrap()
    .0
    .npv
}

/// Σ |CF_t| × DF(r, t): the size of the terms summed in NPV at `r`.
fn discounted_scale(rate: Decimal, flows: &[Decimal]) -> Decimal {
    flows
        .iter()
        .enumerate()
        .map(|(t, cf)| cf.abs() * tvm::discount_factor(rate, Decimal::from(t as u64)).unwrap())
        .sum()
}

fn xnpv(rate: Decimal, case: &DatedCase) -> Decimal {
    tvm::xnpv(&XnpvInput {
        rate,
        cash_flows: case.flows.clone(),
        valuation_date: None,
        day_count: Some(case.day_count),
        compounding: Some(case.compounding),
    })
    .unwrap()
    .0
    .xnpv
}

/// 2 × (n + 50) × `ROUNDING` × max(g, 1 / g), g = (1 + r)^n.
fn closed_form_tolerance(r: Decimal, n: u32) -> Decimal {
    let g = (Decimal::ONE + r).powi(i64::from(n));
    Decimal::TWO * Decimal::from(n + 50) * d(ROUNDING) * g.max(Decimal::ONE / g)
}

/// |a − b| ≤ tol × size + `ABSOLUTE_FLOOR`, `size` the magnitude of the terms summed.
fn close_scaled(a: Decimal, b: Decimal, tol: Decimal, size: Decimal) -> bool {
    (a - b).abs() <= tol * size.abs() + d(ABSOLUTE_FLOOR)
}

/// |a − b| ≤ tol × max(|a|, |b|) + `ABSOLUTE_FLOOR`.
fn close_relative(a: Decimal, b: Decimal, tol: Decimal) -> bool {
    let size = a.abs().max(b.abs());
    (a - b).abs() <= tol * size + d(ABSOLUTE_FLOOR)
}

// ---------------------------------------------------------------- IRR, XIRR and MIRR

/// NPV at the IRR is zero within the residual tolerance, the root is reported as unique, NPV
/// changes sign across it, and `npv_at_irr` is NPV evaluated at the IRR.
#[test]
fn npv_at_irr_is_zero() {
    check(
        "npv_at_irr_is_zero",
        192,
        0x3301,
        conventional_flows(),
        |flows| {
            let (out, _) = tvm::irr(&IrrInput {
                cash_flows: flows.clone(),
                guess: None,
            })
            .map_err(|e| TestCaseError::fail(format!("irr failed: {e}")))?;
            prop_assert!(
                out.uniqueness_verified,
                "one sign change must prove uniqueness"
            );
            prop_assert!(
                out.irr > d("-0.5") && out.irr < d("10"),
                "irr {} outside (-0.5, 10)",
                out.irr
            );
            let at = npv(out.irr, &flows);
            prop_assert_eq!(at, out.npv_at_irr);
            let tol = discounted_scale(out.irr, &flows) * d(ROOT_RESIDUAL);
            prop_assert!(at.abs() <= tol, "NPV(IRR) = {} exceeds {}", at, tol);
            let h = d("0.000000001");
            prop_assert!(npv(out.irr - h, &flows) > Decimal::ZERO);
            prop_assert!(npv(out.irr + h, &flows) < Decimal::ZERO);
            Ok(())
        },
    );
}

/// MIRR satisfies its definition (1 + MIRR)^n × (−PV of outflows) = FV of inflows, and equals
/// the IRR when both the finance and the reinvestment rate are the IRR.
#[test]
fn mirr_is_consistent() {
    let rates = (-5_000i64..=10_000).prop_map(|m| dec(m, 4));
    let strategy = (conventional_flows(), rates.clone(), rates);
    check(
        "mirr_is_consistent",
        192,
        0x3302,
        strategy,
        |(flows, finance, reinvest)| {
            let n = (flows.len() - 1) as i64;
            let mirr = |f: Decimal, r: Decimal| {
                tvm::mirr(&MirrInput {
                    cash_flows: flows.clone(),
                    finance_rate: f,
                    reinvest_rate: r,
                })
            };
            let (out, _) = mirr(finance, reinvest)
                .map_err(|e| TestCaseError::fail(format!("mirr failed: {e}")))?;
            let grown = -out.pv_negative_flows * (Decimal::ONE + out.mirr).powi(n);
            prop_assert!(
                close_relative(grown, out.fv_positive_flows, d(MIRR_RELATIVE)),
                "(1 + MIRR)^n × PV = {} but FV = {}",
                grown,
                out.fv_positive_flows
            );

            let irr = tvm::irr(&IrrInput {
                cash_flows: flows.clone(),
                guess: None,
            })
            .unwrap()
            .0
            .irr;
            let (at_irr, _) = mirr(irr, irr).unwrap();
            prop_assert!(
                (at_irr.mirr - irr).abs() <= d(MIRR_AT_IRR),
                "MIRR at IRR = {} but IRR = {}",
                at_irr.mirr,
                irr
            );
            Ok(())
        },
    );
}

/// Intervals and exact roots named in an FM-E011 message: "[a, b]" and "{x}".
fn named_roots(message: &str) -> Vec<(Decimal, Decimal)> {
    let mut out = Vec::new();
    let mut rest = message;
    while let Some(i) = rest.find(['[', '{']) {
        let close = if rest.as_bytes()[i] == b'[' { ']' } else { '}' };
        let j = i + rest[i..].find(close).expect("closing bracket");
        let parts: Vec<Decimal> = rest[i + 1..j].split(", ").map(d).collect();
        out.push((parts[0], *parts.last().unwrap()));
        rest = &rest[j + 1..];
    }
    out
}

/// Two distinct roots in the grid range: IRR refuses with FM-E011 and never picks one. Flows
/// -1, y1 + y2, -y1 y2 (scaled), with y = 1 + r: NPV × (1 + r)² = -(y - y1)(y - y2) exactly, so
/// the roots are r1 and r2 (the AC8 case is r1 = 0.10, r2 = 0.20). When a grid point lies
/// strictly between the roots, the message names two intervals (or exact grid roots), each
/// containing one of them.
#[test]
fn two_roots_are_refused() {
    let grid = tvm::solver_grid();
    let strategy = (
        (-8_000i64..=9_000).prop_map(|m| dec(m, 4)),
        1i64..=1_000,
        1i64..=1_000_000,
    );
    check(
        "two_roots_are_refused",
        192,
        0x3303,
        strategy,
        |(r1, gap, scale)| {
            let r2 = r1 + dec(gap, 4);
            let (y1, y2) = (Decimal::ONE + r1, Decimal::ONE + r2);
            let s = Decimal::from(scale);
            let flows = vec![-s, (y1 + y2) * s, -(y1 * y2) * s];
            let err = tvm::irr(&IrrInput {
                cash_flows: flows,
                guess: None,
            })
            .unwrap_err();
            prop_assert_eq!(err.code(), "FM-E011");
            let FairmarkError::MultipleRates(message) = &err else {
                return Err(TestCaseError::fail(format!("got {err:?}")));
            };
            if grid.iter().any(|g| *g > r1 && *g < r2) {
                let named = named_roots(message);
                prop_assert_eq!(named.len(), 2, "message: {}", message);
                for (root, (lo, hi)) in [r1, r2].iter().zip(&named) {
                    prop_assert!(
                        lo <= root && root <= hi,
                        "root {} not in [{}, {}]: {}",
                        root,
                        lo,
                        hi,
                        message
                    );
                }
            }
            Ok(())
        },
    );
}

/// Three roots: flows -1, s1, -s2, s3 with y = 1 + r give NPV × (1 + r)³ =
/// -(y - y1)(y - y2)(y - y3), three sign changes. Whatever the solver does, it never reports a
/// unique rate: FM-E011 when it sees more than one root (or an odd number missing), otherwise
/// one of the three roots with `uniqueness_verified` false.
#[test]
fn three_roots_are_never_unique() {
    let root = (-8_000i64..=60_000).prop_map(|m| dec(m, 4));
    let strategy = (root.clone(), root.clone(), root);
    check(
        "three_roots_are_never_unique",
        128,
        0x3304,
        strategy,
        |(a, b, c)| {
            prop_assume!(a != b && b != c && a != c);
            let ys = [Decimal::ONE + a, Decimal::ONE + b, Decimal::ONE + c];
            let s1 = ys[0] + ys[1] + ys[2];
            let s2 = ys[0] * ys[1] + ys[0] * ys[2] + ys[1] * ys[2];
            let s3 = ys[0] * ys[1] * ys[2];
            let flows = vec![-Decimal::ONE, s1, -s2, s3];
            match tvm::irr(&IrrInput {
                cash_flows: flows,
                guess: None,
            }) {
                Ok((out, _)) => {
                    prop_assert!(!out.uniqueness_verified, "unique rate {} reported", out.irr);
                    let nearest = [a, b, c].iter().map(|r| (out.irr - r).abs()).min().unwrap();
                    prop_assert!(nearest <= d("1e-12"), "irr {} is not a root", out.irr);
                }
                Err(e) => prop_assert!(
                    matches!(e, FairmarkError::MultipleRates(_)),
                    "unexpected error {:?}",
                    e
                ),
            }
            Ok(())
        },
    );
}

/// XNPV at the XIRR is zero within the residual tolerance, under every day count and
/// compounding; the root is reported as unique and `xnpv_at_xirr` is XNPV at the XIRR.
#[test]
fn xnpv_at_xirr_is_zero() {
    check("xnpv_at_xirr_is_zero", 96, 0x3305, dated_case(), |case| {
        let (out, _) = tvm::xirr(&XirrInput {
            cash_flows: case.flows.clone(),
            day_count: Some(case.day_count),
            compounding: Some(case.compounding),
            guess: None,
        })
        .map_err(|e| TestCaseError::fail(format!("xirr failed: {e}")))?;
        prop_assert!(out.uniqueness_verified);
        let (xn, _) = tvm::xnpv(&XnpvInput {
            rate: out.xirr,
            cash_flows: case.flows.clone(),
            valuation_date: None,
            day_count: Some(case.day_count),
            compounding: Some(case.compounding),
        })
        .unwrap();
        prop_assert_eq!(xn.xnpv, out.xnpv_at_xirr);
        let scale: Decimal = xn.flows.iter().map(|f| f.present_value.abs()).sum();
        let tol = scale * d(ROOT_RESIDUAL);
        prop_assert!(
            xn.xnpv.abs() <= tol,
            "XNPV(XIRR) = {} exceeds {}",
            xn.xnpv,
            tol
        );
        let h = d("0.000000001");
        prop_assert!(xnpv(out.xirr - h, &case) > Decimal::ZERO);
        prop_assert!(xnpv(out.xirr + h, &case) < Decimal::ZERO);
        Ok(())
    });
}

// ---------------------------------------------------------------- monotonicity

/// NPV is strictly decreasing in the rate for an outflow followed by inflows.
#[test]
fn npv_decreases_in_rate() {
    let strategy = (outflow_then_inflows(), moderate_rate(), moderate_rate());
    check(
        "npv_decreases_in_rate",
        256,
        0x3306,
        strategy,
        |(flows, a, b)| {
            prop_assume!(a != b);
            let (lo, hi) = (a.min(b), a.max(b));
            let (v_lo, v_hi) = (npv(lo, &flows), npv(hi, &flows));
            prop_assert!(
                v_lo > v_hi,
                "NPV({}) = {} is not above NPV({}) = {}",
                lo,
                v_lo,
                hi,
                v_hi
            );
            Ok(())
        },
    );
}

fn annuity_pv(payment: Decimal, rate: Decimal, periods: u32, timing: AnnuityTiming) -> Decimal {
    tvm::annuity(&AnnuityInput {
        payment,
        rate,
        periods,
        growth: None,
        timing,
    })
    .unwrap()
    .0
    .present_value
}

/// The present value of a level annuity with a positive payment is strictly decreasing in the
/// rate, except an annuity due of one payment, which is paid at t = 0 and equals the payment.
#[test]
fn annuity_decreases_in_rate() {
    let strategy = (payment(), two_rates_and_periods(360), timing());
    check(
        "annuity_decreases_in_rate",
        256,
        0x3307,
        strategy,
        |(p, (a, b, n), timing)| {
            prop_assume!(a != b);
            let (lo, hi) = (a.min(b), a.max(b));
            let (v_lo, v_hi) = (annuity_pv(p, lo, n, timing), annuity_pv(p, hi, n, timing));
            if timing == AnnuityTiming::Due && n == 1 {
                // The factor is (1 / (1 + r)) × (1 + r): one rounding from 1.
                prop_assert!(close_relative(v_lo, p, d("1e-27")), "{}", v_lo);
                prop_assert!(close_relative(v_hi, p, d("1e-27")), "{}", v_hi);
            } else {
                prop_assert!(
                    v_lo > v_hi,
                    "PV not decreasing: r {} and {} at n {}",
                    lo,
                    hi,
                    n
                );
            }
            Ok(())
        },
    );
}

/// The present value of a level annuity with a positive payment is strictly increasing in the
/// number of payments: the added payments are worth at least payment × 1e-6, far above the
/// precision.
#[test]
fn annuity_increases_in_periods() {
    let strategy = (payment(), rate_and_two_periods(420, 60), timing());
    check(
        "annuity_increases_in_periods",
        256,
        0x3308,
        strategy,
        |(p, (r, n, m), timing)| {
            let (v_n, v_m) = (annuity_pv(p, r, n, timing), annuity_pv(p, r, m, timing));
            prop_assert!(v_m > v_n, "PV not increasing: r {} at n {} and {}", r, n, m);
            Ok(())
        },
    );
}

/// For a non-negative rate the discount factor is in (0, 1], equal to 1 at t = 0 or r = 0,
/// and non-increasing in t; with a positive rate strictly decreasing. Horizons are kept where
/// (1 + r)^t ≤ 2^90 < 1e28 (t ≤ 89.5, and 90 for the next half period): beyond that the factor
/// underflows to 0 by design. The dated discount factor is in (0, 1] and non-increasing in t
/// under every compounding.
#[test]
fn discount_factor_bounds() {
    let strategy = (positive_rate(), 0u32..=179);
    check(
        "discount_factor_bounds",
        256,
        0x3309,
        strategy,
        |(r, half_periods)| {
            let t = Decimal::from(half_periods) / Decimal::TWO;
            let next = t + d("0.5");
            let df = tvm::discount_factor(r, t).unwrap();
            prop_assert!(
                df > Decimal::ZERO && df <= Decimal::ONE,
                "DF({}, {}) = {}",
                r,
                t,
                df
            );
            if t.is_zero() || r.is_zero() {
                prop_assert_eq!(df, Decimal::ONE);
            }
            let later = tvm::discount_factor(r, next).unwrap();
            if r.is_zero() {
                prop_assert_eq!(later, df);
            } else {
                prop_assert!(
                    later < df,
                    "DF not decreasing in t at r {}: {} then {}",
                    r,
                    df,
                    later
                );
            }
            // Dated: t / 2 years, so up to 45 years.
            for c in COMPOUNDINGS {
                let now = rates::discount_factor(r, c, t / Decimal::TWO).unwrap();
                let after = rates::discount_factor(r, c, next / Decimal::TWO).unwrap();
                prop_assert!(
                    now > Decimal::ZERO && now <= Decimal::ONE,
                    "{:?}: {}",
                    c,
                    now
                );
                prop_assert!(after <= now, "{:?} at r {}: {} then {}", c, r, now, after);
            }
            Ok(())
        },
    );
}

// ---------------------------------------------------------------- round trips

/// rates.convert from any compounding to any other and back returns the rate: within the AC3
/// bound, and within the stricter guard.
#[test]
fn convert_round_trip() {
    let strategy = (
        (-5_000i64..=10_000).prop_map(|m| dec(m, 4)),
        compounding(),
        compounding(),
    );
    check(
        "convert_round_trip",
        256,
        0x330a,
        strategy,
        |(r, from, to)| {
            let there = rates::convert(&ConvertInput { rate: r, from, to })
                .unwrap()
                .0;
            let back = rates::convert(&ConvertInput {
                rate: there.rate,
                from: to,
                to: from,
            })
            .unwrap()
            .0;
            let error = (back.rate - r).abs();
            prop_assert!(
                error <= d(CONVERT_ROUND_TRIP),
                "AC3: {} {:?} -> {} {:?} -> {}",
                r,
                from,
                there.rate,
                to,
                back.rate
            );
            prop_assert!(
                error <= d(CONVERT_ROUND_TRIP_STRICT),
                "{} {:?} -> {} {:?} -> {} (error {})",
                r,
                from,
                there.rate,
                to,
                back.rate,
                error
            );
            Ok(())
        },
    );
}

/// rates.real_nominal to_real then to_nominal with the same inflation returns the rate within
/// 1e-20 (AC3), for rate and inflation in (-1, 1] (issue #176, finding 2).
#[test]
fn fisher_round_trip() {
    let unit = || (-999_999i64..=1_000_000).prop_map(|m| dec(m, 6));
    check(
        "fisher_round_trip",
        256,
        0x330e,
        (unit(), unit()),
        |(rate, inflation)| {
            let real = rates::real_nominal(&RealNominalInput {
                rate,
                inflation,
                direction: FisherDirection::ToReal,
            })
            .unwrap()
            .0
            .rate;
            let back = rates::real_nominal(&RealNominalInput {
                rate: real,
                inflation,
                direction: FisherDirection::ToNominal,
            })
            .unwrap()
            .0
            .rate;
            prop_assert!(
                (back - rate).abs() <= d(FISHER_ROUND_TRIP),
                "AC3: {} with inflation {} -> real {} -> nominal {}",
                rate,
                inflation,
                real,
                back
            );
            Ok(())
        },
    );
}

/// loans.instalment then loans.number_of_instalments returns the number of instalments, with a
/// final instalment equal to the instalment within the engine's balance tolerance (1e-20 ×
/// principal, documented in the method), at positive, zero, tiny and small negative periodic
/// rates (SPEC-0009).
#[test]
fn instalment_round_trip() {
    let rate = prop_oneof![
        4 => (1i64..=500).prop_map(|m| dec(m, 4)),
        1 => (-50i64..=-1).prop_map(|m| dec(m, 4)),
        1 => (-1_000i64..=1_000).prop_map(|m| dec(m, 12)),
    ];
    let rate_and_n = rate.prop_flat_map(|i| (Just(i), 1..=well_conditioned_periods(i, 600)));
    let strategy = (
        (100_000i64..=1_000_000_000).prop_map(|m| dec(m, 2)),
        rate_and_n,
        timing(),
    );
    check(
        "instalment_round_trip",
        256,
        0x330b,
        strategy,
        |(principal, (i, n), timing)| {
            let (a, _) = loans::instalment(&InstalmentInput {
                principal,
                rate: None,
                compounding: None,
                payment_frequency: None,
                periodic_rate: Some(i),
                number_of_instalments: n,
                timing,
                balloon_amount: None,
                rounding: None,
            })
            .unwrap();
            let (back, _) = loans::number_of_instalments(&NumberOfInstalmentsInput {
                principal,
                rate: None,
                compounding: None,
                payment_frequency: None,
                periodic_rate: Some(i),
                instalment: a.instalment,
                timing,
            })
            .map_err(|e| TestCaseError::fail(format!("number_of_instalments failed: {e}")))?;
            prop_assert_eq!(
                back.number_of_instalments,
                n,
                "i {} instalment {}",
                i,
                a.instalment
            );
            let balance_tol = principal * d("1e-20");
            prop_assert!(
                (back.final_instalment - a.instalment).abs() <= balance_tol,
                "final {} vs instalment {}",
                back.final_instalment,
                a.instalment
            );
            Ok(())
        },
    );
}

/// Value at t = n of `pv` and of n payments, built period by period: ordinary
/// B_k = B_(k−1) (1 + r) + A, due B_k = (B_(k−1) + A)(1 + r), B_0 = pv. Also returns the sum of
/// the magnitudes of the accumulated terms, the size against which the rounding is measured.
fn accumulate(
    pv: Decimal,
    a: Decimal,
    r: Decimal,
    n: u32,
    timing: AnnuityTiming,
) -> (Decimal, Decimal) {
    let one_r = Decimal::ONE + r;
    let (mut b, mut size) = (pv, pv.abs());
    for _ in 0..n {
        (b, size) = match timing {
            AnnuityTiming::Ordinary => (b * one_r + a, size * one_r + a.abs()),
            AnnuityTiming::Due => ((b + a) * one_r, (size + a.abs()) * one_r),
        };
    }
    (b, size)
}

/// Present value at t = 0 of n payments, discounted period by period from the last one:
/// ordinary S_k = (S_(k+1) + A) / (1 + r), due the same with the first payment undiscounted.
fn discount_payments(a: Decimal, r: Decimal, n: u32, timing: AnnuityTiming) -> Decimal {
    let one_r = Decimal::ONE + r;
    let mut s = Decimal::ZERO;
    for _ in 0..n {
        s = (s + a) / one_r;
    }
    match timing {
        AnnuityTiming::Ordinary => s,
        AnnuityTiming::Due => s * one_r,
    }
}

/// tvm.future_value and tvm.annuity against explicit sums built in the test by repeated
/// multiplication and division, ordinary and due (SPEC-0009 future value, SPEC-0002 R11).
#[test]
fn future_and_present_value_match_explicit_sums() {
    let strategy = (
        (-100_000_000i64..=100_000_000).prop_map(|m| dec(m, 2)),
        payment(),
        rate_and_periods(600),
        timing(),
    );
    check(
        "future_and_present_value_match_explicit_sums",
        256,
        0x330c,
        strategy,
        |(amount, pmt, (r, n), timing)| {
            let tol = closed_form_tolerance(r, n);
            let fv = tvm::future_value(&FutureValueInput {
                present_value: Some(amount),
                payment: Some(pmt),
                rate: r,
                periods: n,
                timing,
            })
            .unwrap()
            .0
            .future_value;
            let (expected, size) = accumulate(amount, pmt, r, n, timing);
            prop_assert!(
                close_scaled(fv, expected, tol, size),
                "future_value {} vs explicit {}",
                fv,
                expected
            );

            let ann = tvm::annuity(&AnnuityInput {
                payment: pmt,
                rate: r,
                periods: n,
                growth: None,
                timing,
            })
            .unwrap()
            .0;
            let (fv_payments, size) = accumulate(Decimal::ZERO, pmt, r, n, timing);
            prop_assert!(
                close_scaled(ann.future_value, fv_payments, tol, size),
                "annuity future value {} vs explicit {}",
                ann.future_value,
                fv_payments
            );
            let pv_payments = discount_payments(pmt, r, n, timing);
            prop_assert!(
                close_scaled(ann.present_value, pv_payments, tol, pv_payments),
                "annuity present value {} vs explicit {}",
                ann.present_value,
                pv_payments
            );
            Ok(())
        },
    );
}

// ---------------------------------------------------------------- determinism through the registry

/// A method input as (key, value) pairs, in the order they are written.
type Pairs = Vec<(String, Value)>;

fn text(v: Decimal) -> Value {
    Value::String(v.to_string())
}

fn enum_value<T: serde::Serialize>(v: T) -> Value {
    serde_json::to_value(v).unwrap()
}

/// A method id and its input as (key, value) pairs, decimals written as JSON strings, for six
/// methods across the themes.
fn registry_case() -> impl Strategy<Value = (&'static str, Pairs)> {
    let npv = (rate(), conventional_flows()).prop_map(|(r, f)| {
        let cf = f.into_iter().map(text).collect();
        (
            "tvm.npv",
            vec![
                ("rate".into(), text(r)),
                ("cash_flows".into(), Value::Array(cf)),
            ],
        )
    });
    let irr = conventional_flows().prop_map(|f| {
        let cf = f.into_iter().map(text).collect();
        (
            "tvm.irr",
            vec![
                ("cash_flows".into(), Value::Array(cf)),
                ("guess".into(), text(d("0.1"))),
            ],
        )
    });
    let convert = ((-5_000i64..=10_000), compounding(), compounding()).prop_map(|(m, from, to)| {
        (
            "rates.convert",
            vec![
                ("rate".into(), text(dec(m, 4))),
                ("from".into(), enum_value(from)),
                ("to".into(), enum_value(to)),
            ],
        )
    });
    let annuity = (rate_and_periods(360), timing()).prop_map(|((r, n), t)| {
        (
            "tvm.annuity",
            vec![
                ("payment".into(), text(d("1000"))),
                ("rate".into(), text(r)),
                ("periods".into(), Value::from(n)),
                ("growth".into(), text(d("0.01"))),
                ("timing".into(), enum_value(t)),
            ],
        )
    });
    let instalment = (
        (100_000i64..=1_000_000_000),
        (1i64..=500),
        1u32..=240,
        timing(),
    )
        .prop_map(|(p, i, n, t)| {
            (
                "loans.instalment",
                vec![
                    ("principal".into(), text(dec(p, 2))),
                    ("periodic_rate".into(), text(dec(i, 4))),
                    ("number_of_instalments".into(), Value::from(n)),
                    ("timing".into(), enum_value(t)),
                ],
            )
        });
    let xnpv = (dated_case(), rate()).prop_map(|(c, r)| {
        let flows = c
            .flows
            .iter()
            .map(|f| {
                let mut o = Map::new();
                o.insert("date".into(), Value::String(f.date.to_string()));
                o.insert("amount".into(), text(f.amount));
                Value::Object(o)
            })
            .collect();
        (
            "tvm.xnpv",
            vec![
                ("rate".into(), text(r)),
                ("cash_flows".into(), Value::Array(flows)),
                ("day_count".into(), enum_value(c.day_count)),
                ("compounding".into(), enum_value(c.compounding)),
            ],
        )
    });
    prop_oneof![npv, irr, convert, annuity, instalment, xnpv]
}

/// The same value with every decimal string written as a JSON number instead (dates and
/// enumeration names do not parse as decimals and stay strings).
fn as_numbers(v: &Value) -> Value {
    match v {
        Value::String(s) if Decimal::from_str(s).is_ok() => serde_json::from_str(s).unwrap(),
        Value::Array(a) => Value::Array(a.iter().map(as_numbers).collect()),
        Value::Object(o) => {
            Value::Object(o.iter().map(|(k, v)| (k.clone(), as_numbers(v))).collect())
        }
        other => other.clone(),
    }
}

fn to_json(pairs: &[(String, Value)]) -> String {
    let mut o = Map::new();
    for (k, v) in pairs {
        o.insert(k.clone(), v.clone());
    }
    Value::Object(o).to_string()
}

/// The whole envelope (value, explain, meta) as text, or the error code and message.
fn run_text(id: &str, input: &str) -> String {
    match fairmark_core::run(id, input) {
        Ok(v) => serde_json::to_string(&v).unwrap(),
        Err(e) => format!("{}: {e}", e.code()),
    }
}

/// `meta.input_hash` of an envelope given as text, if the run succeeded.
fn input_hash(envelope: &str) -> Option<String> {
    let v: Value = serde_json::from_str(envelope).ok()?;
    v["meta"]["input_hash"].as_str().map(str::to_owned)
}

/// Determinism through the registry, in-process: the same input twice gives byte-identical
/// envelopes (value, explain and meta), and so does the same input with its keys in another
/// order or with its decimals written as JSON numbers instead of strings; `meta.input_hash`
/// is a 64-digit hexadecimal SHA-256 that depends on none of these. Bit-identical results
/// across platforms rest on the golden suite, which runs on every platform in CI.
#[test]
fn registry_is_deterministic() {
    let strategy = registry_case().prop_flat_map(|(id, pairs)| {
        let shuffled = Just(pairs.clone()).prop_shuffle();
        (Just(id), Just(pairs), shuffled)
    });
    check(
        "registry_is_deterministic",
        128,
        0x330d,
        strategy,
        |(id, pairs, shuffled)| {
            let input = to_json(&pairs);
            let first = run_text(id, &input);
            let second = run_text(id, &input);
            prop_assert_eq!(&first, &second, "two runs of {} differ", id);
            let reordered = run_text(id, &to_json(&shuffled));
            prop_assert_eq!(&first, &reordered, "key order changed the result of {}", id);
            let numbers: Pairs = pairs
                .iter()
                .map(|(k, v)| (k.clone(), as_numbers(v)))
                .collect();
            prop_assert_ne!(to_json(&numbers), input.clone());
            let as_numbers = run_text(id, &to_json(&numbers));
            prop_assert_eq!(
                &first,
                &as_numbers,
                "JSON numbers changed the result of {}",
                id
            );
            if let Some(hash) = input_hash(&first) {
                prop_assert_eq!(hash.len(), 64);
                prop_assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()));
                prop_assert_eq!(input_hash(&reordered), Some(hash.clone()));
                prop_assert_eq!(input_hash(&as_numbers), Some(hash));
            }
            Ok(())
        },
    );
}
