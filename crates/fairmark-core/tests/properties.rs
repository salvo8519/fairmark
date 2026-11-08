//! Property tests (issue #33): identities that must hold for every input of a family, checked
//! on generated inputs with `proptest`.
//!
//! Determinism of the suite itself: every property runs a fixed number of cases from a fixed
//! seed with the ChaCha generator, and failure persistence is off, so two runs generate the same
//! inputs on every platform and the suite never writes files. Environment variables such as
//! `PROPTEST_CASES` do not change the case counts below.
//!
//! Tolerances. The engine computes with `rust_decimal` (96-bit mantissa, 28 to 29 significant
//! digits), so one operation has a relative rounding error of at most about 1e-28. Each
//! tolerance below is derived from that precision, from the solver tolerance (1e-18 on the
//! rate) or from a documented engine tolerance, with the derivation next to the constant. All
//! arithmetic in this file is decimal as well: no binary floating point is involved.

use std::str::FromStr;

use fairmark_core::dates::{Date, DayCount};
use fairmark_core::loans::{self, InstalmentInput, NumberOfInstalmentsInput};
use fairmark_core::rates::{self, Compounding, ConvertInput};
use fairmark_core::tvm::{
    self, AnnuityInput, AnnuityTiming, DatedCashFlow, FutureValueInput, IrrInput, MirrInput,
    NpvInput, XirrInput, XnpvInput,
};
use fairmark_core::FairmarkError;
use proptest::prelude::*;
use proptest::test_runner::{Config, RngAlgorithm, RngSeed};
use rust_decimal::{Decimal, MathematicalOps};
use serde_json::{Map, Value};

// ---------------------------------------------------------------- configuration

/// Runner configuration: `cases` generated inputs from the fixed `seed`.
fn config(cases: u32, seed: u64) -> Config {
    Config {
        cases,
        rng_algorithm: RngAlgorithm::ChaCha,
        rng_seed: RngSeed::Fixed(seed),
        failure_persistence: None,
        max_local_rejects: 65_536,
        max_global_rejects: 1_024,
        max_shrink_iters: 1_024,
        ..Config::default()
    }
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

/// Round trip of `rates.convert`: acceptance criterion AC3 of SPEC-0002 (1e-20 absolute, for
/// rates of order one). `ln` and `exp` in `rust_decimal` are series approximations good to
/// about 1e-25 for arguments of order one; the round trip uses two of each, which leaves five
/// orders of magnitude of margin.
const CONVERT_ROUND_TRIP: &str = "1e-20";

/// Relative tolerance for identities between two closed forms (annuity, future value). Both
/// sides use g = (1 + r)^n or its reciprocal 1 / g, computed by exponentiation by squaring:
/// a rounding of relative size u in the partial power x^(2^k) reaches the result multiplied
/// by n / 2^k, so the power carries a relative error up to about n × u (not u), plus a few
/// roundings for the products and quotients around it. u is 1e-28 for values of order one or
/// more and 1e-28 / v for a value of order v < 1 (a decimal has at most 28 decimal places), and
/// both g and 1 / g appear. Hence 2 × (n + 50) × 1e-28 × max(g, 1 / g), see
/// `closed_form_tolerance`: about 1.3e-25 for growing amounts over 600 periods, at most
/// 1.3e-19 for the most discounted ones (the generated problems keep g in [1e-6, 1e6]).
const ROUNDING: &str = "1e-28";

/// Absolute floor added to the relative tolerances above. A decimal has at most 28 decimal
/// places, so a small intermediate value (an amount of 0.01 discounted by 1e-6) is resolved
/// only to 1e-28 in absolute terms, not to 28 significant digits; the generated problems grow
/// or discount by at most 1e6, which amplifies that resolution to 1e-22. 1e-21 keeps a margin
/// of ten.
const ABSOLUTE_FLOOR: &str = "1e-21";

/// MIRR identities: MIRR is computed as exp(ln(FV / PV) / n) − 1, and `ln` and `exp` in
/// `rust_decimal` are good to about 1e-25 relative for arguments of order one. Raised back to
/// the power n ≤ 28 that error grows to at most 28 × 1e-25 ≈ 3e-24. 1e-22 keeps a margin of
/// about thirty for arguments further from one.
const MIRR_RELATIVE: &str = "1e-22";

/// MIRR at finance rate = reinvestment rate = IRR equals IRR: the IRR is known to about 1e-18
/// (solver tolerance); this error passes to MIRR with a factor of order one (at most 2 for the
/// generated flows), and the MIRR identity adds 1e-22. 1e-17 bounds the sum.
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

/// |a − b| ≤ tol × max(|a|, |b|) + `ABSOLUTE_FLOOR`.
fn close_relative(a: Decimal, b: Decimal, tol: Decimal) -> bool {
    let size = a.abs().max(b.abs());
    (a - b).abs() <= tol * size + d(ABSOLUTE_FLOOR)
}

// ---------------------------------------------------------------- IRR, XIRR and MIRR

proptest! {
    #![proptest_config(config(192, 0x3301))]

    /// NPV at the IRR is zero within the residual tolerance, the root is reported as unique,
    /// NPV changes sign across it, and `npv_at_irr` is NPV evaluated at the IRR.
    #[test]
    fn npv_at_irr_is_zero(flows in conventional_flows()) {
        let (out, _) = tvm::irr(&IrrInput { cash_flows: flows.clone(), guess: None })
            .map_err(|e| TestCaseError::fail(format!("irr failed: {e}")))?;
        prop_assert!(out.uniqueness_verified, "one sign change must prove uniqueness");
        prop_assert!(out.irr > d("-0.5") && out.irr < d("10"), "irr {} outside (-0.5, 10)", out.irr);
        let at = npv(out.irr, &flows);
        prop_assert_eq!(at, out.npv_at_irr);
        let tol = discounted_scale(out.irr, &flows) * d(ROOT_RESIDUAL);
        prop_assert!(at.abs() <= tol, "NPV(IRR) = {} exceeds {}", at, tol);
        let h = d("0.000000001");
        prop_assert!(npv(out.irr - h, &flows) > Decimal::ZERO);
        prop_assert!(npv(out.irr + h, &flows) < Decimal::ZERO);
    }

    /// MIRR satisfies its definition (1 + MIRR)^n × (−PV of outflows) = FV of inflows, and
    /// equals the IRR when both the finance and the reinvestment rate are the IRR.
    #[test]
    fn mirr_is_consistent(
        flows in conventional_flows(),
        finance in (-5_000i64..=10_000).prop_map(|m| dec(m, 4)),
        reinvest in (-5_000i64..=10_000).prop_map(|m| dec(m, 4)),
    ) {
        let n = (flows.len() - 1) as i64;
        let mirr = |f: Decimal, r: Decimal| tvm::mirr(&MirrInput {
            cash_flows: flows.clone(),
            finance_rate: f,
            reinvest_rate: r,
        });
        let (out, _) = mirr(finance, reinvest)
            .map_err(|e| TestCaseError::fail(format!("mirr failed: {e}")))?;
        let grown = -out.pv_negative_flows * (Decimal::ONE + out.mirr).powi(n);
        prop_assert!(
            close_relative(grown, out.fv_positive_flows, d(MIRR_RELATIVE)),
            "(1 + MIRR)^n × PV = {} but FV = {}", grown, out.fv_positive_flows
        );

        let irr = tvm::irr(&IrrInput { cash_flows: flows.clone(), guess: None }).unwrap().0.irr;
        let (at_irr, _) = mirr(irr, irr).unwrap();
        prop_assert!(
            (at_irr.mirr - irr).abs() <= d(MIRR_AT_IRR),
            "MIRR at IRR = {} but IRR = {}", at_irr.mirr, irr
        );
    }

    /// Two distinct roots in the grid range: IRR refuses with FM-E011 and never picks one.
    /// Flows -1, y1 + y2, -y1 y2 (scaled), with y = 1 + r: NPV × (1 + r)² = -(y - y1)(y - y2),
    /// exactly, so the roots are r1 and r2 (the AC8 case is r1 = 0.10, r2 = 0.20).
    #[test]
    fn two_roots_are_refused(
        r1 in (-8_000i64..=9_000).prop_map(|m| dec(m, 4)),
        gap in 1i64..=1_000,
        scale in 1i64..=1_000_000,
    ) {
        let (y1, y2) = (Decimal::ONE + r1, Decimal::ONE + r1 + dec(gap, 4));
        let s = Decimal::from(scale);
        let flows = vec![-s, (y1 + y2) * s, -(y1 * y2) * s];
        let err = tvm::irr(&IrrInput { cash_flows: flows, guess: None }).unwrap_err();
        prop_assert!(matches!(err, FairmarkError::MultipleRates(_)), "got {err:?}");
        prop_assert_eq!(err.code(), "FM-E011");
    }
}

proptest! {
    #![proptest_config(config(96, 0x3302))]

    /// XNPV at the XIRR is zero within the residual tolerance, under every day count and
    /// compounding; the root is reported as unique and `xnpv_at_xirr` is XNPV at the XIRR.
    #[test]
    fn xnpv_at_xirr_is_zero(case in dated_case()) {
        let (out, _) = tvm::xirr(&XirrInput {
            cash_flows: case.flows.clone(),
            day_count: Some(case.day_count),
            compounding: Some(case.compounding),
            guess: None,
        })
        .map_err(|e| TestCaseError::fail(format!("xirr failed: {e}")))?;
        prop_assert!(out.uniqueness_verified);
        let at = xnpv(out.xirr, &case);
        prop_assert_eq!(at, out.xnpv_at_xirr);
        let (xn, _) = tvm::xnpv(&XnpvInput {
            rate: out.xirr,
            cash_flows: case.flows.clone(),
            valuation_date: None,
            day_count: Some(case.day_count),
            compounding: Some(case.compounding),
        })
        .unwrap();
        let scale: Decimal = xn.flows.iter().map(|f| f.present_value.abs()).sum();
        let tol = scale * d(ROOT_RESIDUAL);
        prop_assert!(at.abs() <= tol, "XNPV(XIRR) = {} exceeds {}", at, tol);
        let h = d("0.000000001");
        prop_assert!(xnpv(out.xirr - h, &case) > Decimal::ZERO);
        prop_assert!(xnpv(out.xirr + h, &case) < Decimal::ZERO);
    }
}

// ---------------------------------------------------------------- monotonicity

proptest! {
    #![proptest_config(config(256, 0x3303))]

    /// NPV is strictly decreasing in the rate for an outflow followed by inflows.
    #[test]
    fn npv_decreases_in_rate(
        flows in outflow_then_inflows(),
        a in moderate_rate(),
        b in moderate_rate(),
    ) {
        prop_assume!(a != b);
        let (lo, hi) = (a.min(b), a.max(b));
        let (v_lo, v_hi) = (npv(lo, &flows), npv(hi, &flows));
        prop_assert!(v_lo > v_hi, "NPV({}) = {} is not above NPV({}) = {}", lo, v_lo, hi, v_hi);
    }

    /// The present value of a level annuity with a positive payment is strictly decreasing in
    /// the rate (except an annuity due of one payment, which is paid at t = 0 and does not
    /// depend on the rate) and strictly increasing in the number of payments.
    #[test]
    fn annuity_monotonic(
        payment in (1i64..=100_000_000).prop_map(|m| dec(m, 2)),
        (a, n) in rate_and_periods(360),
        b in rate(),
        extra in 1u32..=60,
        timing in timing(),
    ) {
        let pv = |r: Decimal, n: u32| tvm::annuity(&AnnuityInput {
            payment, rate: r, periods: n, growth: None, timing,
        }).unwrap().0.present_value;
        // Rate: compare at the same n, with b inside the well-conditioned range for n.
        prop_assume!(a != b && n <= well_conditioned_periods(b, 360));
        let (lo, hi) = (a.min(b), a.max(b));
        if timing == AnnuityTiming::Due && n == 1 {
            // The factor is (1 / (1 + r)) × (1 + r): one rounding from 1.
            let tol = d("1e-27");
            prop_assert!(close_relative(pv(lo, n), payment, tol), "{}", pv(lo, n));
            prop_assert!(close_relative(pv(hi, n), payment, tol), "{}", pv(hi, n));
        } else {
            prop_assert!(
                pv(lo, n) > pv(hi, n),
                "PV not decreasing in rate: r {} and {} at n {}", lo, hi, n
            );
        }
        // Periods: the added payments are worth at least payment × 1e-6 × (1 + r)^-60, far
        // above the precision, so the increase is strict.
        let m = n + extra;
        prop_assume!(m <= well_conditioned_periods(a, 420));
        prop_assert!(pv(a, m) > pv(a, n), "PV not increasing in n: r {} at n {} and {}", a, n, m);
    }

    /// For a non-negative rate the discount factor is in (0, 1], equal to 1 at t = 0 or r = 0,
    /// and non-increasing in t; with a positive rate strictly decreasing. Horizons are kept
    /// where (1 + r)^t ≤ 2^90 < 1e28: beyond that the factor underflows to 0 by design.
    #[test]
    fn discount_factor_bounds(
        r in positive_rate(),
        half_periods in 0u32..=180,
        c in compounding(),
    ) {
        let t = Decimal::from(half_periods) / Decimal::TWO;
        let df = tvm::discount_factor(r, t).unwrap();
        prop_assert!(df > Decimal::ZERO && df <= Decimal::ONE, "DF({}, {}) = {}", r, t, df);
        if t.is_zero() || r.is_zero() {
            prop_assert_eq!(df, Decimal::ONE);
        }
        let later = tvm::discount_factor(r, t + d("0.5")).unwrap();
        if r.is_zero() {
            prop_assert_eq!(later, df);
        } else {
            prop_assert!(later < df, "DF not decreasing in t at r {}: {} then {}", r, df, later);
        }
        // The dated discount factor, any compounding: in (0, 1] over up to 45 years.
        let dated = rates::discount_factor(r, c, t / Decimal::TWO).unwrap();
        prop_assert!(dated > Decimal::ZERO && dated <= Decimal::ONE);
    }
}

// ---------------------------------------------------------------- round trips

proptest! {
    #![proptest_config(config(256, 0x3304))]

    /// rates.convert from any compounding to any other and back returns the rate (AC3).
    #[test]
    fn convert_round_trip(
        r in (-5_000i64..=10_000).prop_map(|m| dec(m, 4)),
        from in compounding(),
        to in compounding(),
    ) {
        let there = rates::convert(&ConvertInput { rate: r, from, to }).unwrap().0;
        let back = rates::convert(&ConvertInput { rate: there.rate, from: to, to: from }).unwrap().0;
        prop_assert!(
            (back.rate - r).abs() <= d(CONVERT_ROUND_TRIP),
            "{} {:?} -> {} {:?} -> {}", r, from, there.rate, to, back.rate
        );
    }

    /// loans.instalment then loans.number_of_instalments returns the number of instalments,
    /// with a final instalment equal to the instalment within the engine's balance tolerance
    /// (1e-20 × principal, documented in the method), at positive, zero, tiny and small
    /// negative periodic rates.
    #[test]
    fn instalment_round_trip(
        principal in (100_000i64..=1_000_000_000).prop_map(|m| dec(m, 2)),
        i in prop_oneof![
            4 => (1i64..=500).prop_map(|m| dec(m, 4)),
            1 => (-50i64..=-1).prop_map(|m| dec(m, 4)),
            1 => (-1_000i64..=1_000).prop_map(|m| dec(m, 12)),
        ],
        n in 1u32..=600,
        timing in timing(),
    ) {
        let n = n.min(well_conditioned_periods(i, 600));
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
        prop_assert_eq!(back.number_of_instalments, n, "i {} instalment {}", i, a.instalment);
        let balance_tol = principal * d("1e-20");
        prop_assert!(
            (back.final_instalment - a.instalment).abs() <= balance_tol,
            "final {} vs instalment {}", back.final_instalment, a.instalment
        );
    }

    /// tvm.future_value, tvm.annuity and the discount factor agree: an amount accumulated over
    /// n periods and discounted back is unchanged; the future value of the annuity's payments
    /// equals the annuity's future value; accumulating the annuity's present value gives it too.
    #[test]
    fn future_and_present_value_agree(
        amount in (-100_000_000i64..=100_000_000).prop_map(|m| dec(m, 2)),
        payment in (1i64..=100_000_000).prop_map(|m| dec(m, 2)),
        (r, n) in rate_and_periods(600),
        timing in timing(),
    ) {
        let tol = closed_form_tolerance(r, n);
        let fv = |pv: Option<Decimal>, pmt: Option<Decimal>| tvm::future_value(&FutureValueInput {
            present_value: pv, payment: pmt, rate: r, periods: n, timing,
        }).unwrap().0.future_value;

        let back = fv(Some(amount), None) * tvm::discount_factor(r, Decimal::from(n)).unwrap();
        prop_assert!(close_relative(back, amount, tol), "{} came back as {}", amount, back);

        let ann = tvm::annuity(&AnnuityInput {
            payment, rate: r, periods: n, growth: None, timing,
        }).unwrap().0;
        let of_payments = fv(None, Some(payment));
        prop_assert!(
            close_relative(of_payments, ann.future_value, tol),
            "future_value {} vs annuity future value {}", of_payments, ann.future_value
        );
        let of_pv = fv(Some(ann.present_value), None);
        prop_assert!(
            close_relative(of_pv, ann.future_value, tol),
            "accumulated PV {} vs annuity future value {}", of_pv, ann.future_value
        );
    }
}

// ---------------------------------------------------------------- determinism through the registry

/// A decimal written either as a JSON string or as a JSON number: both are parsed exactly.
fn json_decimal(v: Decimal, as_string: bool) -> Value {
    if as_string {
        Value::String(v.to_string())
    } else {
        serde_json::from_str(&v.to_string()).unwrap()
    }
}

/// A method input as (key, value) pairs, in the order they are written.
type Pairs = Vec<(String, Value)>;

/// A method id and its input as (key, value) pairs, for a few methods across the themes.
fn registry_case() -> impl Strategy<Value = (&'static str, Pairs)> {
    let s = any::<bool>;
    let npv = (rate(), conventional_flows(), s()).prop_map(|(r, f, s)| {
        let cf: Vec<Value> = f.into_iter().map(|v| json_decimal(v, s)).collect();
        (
            "tvm.npv",
            vec![
                ("rate".into(), json_decimal(r, s)),
                ("cash_flows".into(), Value::Array(cf)),
            ],
        )
    });
    let irr = (conventional_flows(), s()).prop_map(|(f, s)| {
        let cf: Vec<Value> = f.into_iter().map(|v| json_decimal(v, s)).collect();
        (
            "tvm.irr",
            vec![
                ("cash_flows".into(), Value::Array(cf)),
                ("guess".into(), json_decimal(d("0.1"), s)),
            ],
        )
    });
    let convert =
        ((-5_000i64..=10_000), compounding(), compounding(), s()).prop_map(|(m, from, to, s)| {
            (
                "rates.convert",
                vec![
                    ("rate".into(), json_decimal(dec(m, 4), s)),
                    ("from".into(), serde_json::to_value(from).unwrap()),
                    ("to".into(), serde_json::to_value(to).unwrap()),
                ],
            )
        });
    let annuity = (rate_and_periods(360), timing(), s()).prop_map(|((r, n), t, s)| {
        (
            "tvm.annuity",
            vec![
                ("payment".into(), json_decimal(d("1000"), s)),
                ("rate".into(), json_decimal(r, s)),
                ("periods".into(), Value::from(n)),
                ("growth".into(), json_decimal(d("0.01"), s)),
                ("timing".into(), serde_json::to_value(t).unwrap()),
            ],
        )
    });
    let instalment = (
        (100_000i64..=1_000_000_000),
        (1i64..=500),
        1u32..=240,
        timing(),
        s(),
    )
        .prop_map(|(p, i, n, t, s)| {
            (
                "loans.instalment",
                vec![
                    ("principal".into(), json_decimal(dec(p, 2), s)),
                    ("periodic_rate".into(), json_decimal(dec(i, 4), s)),
                    ("number_of_instalments".into(), Value::from(n)),
                    ("timing".into(), serde_json::to_value(t).unwrap()),
                ],
            )
        });
    let xnpv = (dated_case(), rate(), s()).prop_map(|(c, r, s)| {
        let flows: Vec<Value> = c
            .flows
            .iter()
            .map(|f| {
                let mut o = Map::new();
                o.insert("date".into(), Value::String(f.date.to_string()));
                o.insert("amount".into(), json_decimal(f.amount, s));
                Value::Object(o)
            })
            .collect();
        (
            "tvm.xnpv",
            vec![
                ("rate".into(), json_decimal(r, s)),
                ("cash_flows".into(), Value::Array(flows)),
                (
                    "day_count".into(),
                    serde_json::to_value(c.day_count).unwrap(),
                ),
                (
                    "compounding".into(),
                    serde_json::to_value(c.compounding).unwrap(),
                ),
            ],
        )
    });
    prop_oneof![npv, irr, convert, annuity, instalment, xnpv]
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

/// The same input twice gives byte-identical output, and so does the same input with its keys
/// in any order; `meta.input_hash` does not depend on the key order either.
fn determinism_case() -> impl Strategy<Value = (&'static str, Pairs, Pairs)> {
    registry_case().prop_flat_map(|(id, pairs)| {
        let shuffled = Just(pairs.clone()).prop_shuffle();
        (Just(id), Just(pairs), shuffled)
    })
}

proptest! {
    #![proptest_config(config(128, 0x3305))]

    #[test]
    fn registry_is_deterministic((id, pairs, shuffled) in determinism_case()) {
        let input = to_json(&pairs);
        let first = run_text(id, &input);
        let second = run_text(id, &input);
        prop_assert_eq!(&first, &second, "two runs of {} differ", id);
        let reordered = run_text(id, &to_json(&shuffled));
        prop_assert_eq!(&first, &reordered, "key order changed the result of {}", id);
        if let Ok(v) = fairmark_core::run(id, &input) {
            let w = fairmark_core::run(id, &to_json(&shuffled)).unwrap();
            prop_assert_eq!(&v["meta"]["input_hash"], &w["meta"]["input_hash"]);
            prop_assert_eq!(v["meta"]["input_hash"].as_str().map(str::len), Some(64));
        }
    }
}
