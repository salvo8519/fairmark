//! Time value of money: present values, NPV and XNPV, IRR and XIRR, MIRR, annuities and
//! perpetuities (SPEC-0002), future value (SPEC-0009).
//!
//! Conventions:
//! * rates are decimal fractions (`0.10` = 10%), per period for periodic methods and per year
//!   with an explicit compounding for dated methods;
//! * in `tvm.npv`, `tvm.irr` and `tvm.mirr`, `cash_flows[0]` happens at t = 0;
//! * in `tvm.present_value`, the flows are those of periods 1..n.

use rust_decimal::{Decimal, MathematicalOps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::dates::{Date, DayCount};
use crate::error::{FairmarkError, Result};
use crate::explain::{Explain, Step};
use crate::method::Method;
use crate::num;
use crate::rates::{self, Compounding};

/// Absolute tolerance on the rate for the IRR solver.
pub const IRR_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 18); // 1e-18
/// Maximum iterations for each solver stage.
pub const IRR_MAX_ITERATIONS: u32 = 200;

/// `1 / (1 + rate)^t`. Integer `t` uses exact repeated multiplication, fractional `t`
/// (mid-period) decimal `powd`; rates below zero raise the reciprocal of the base so that no
/// precision is lost, and very long horizons underflow to zero instead of overflowing.
pub fn discount_factor(rate: Decimal, t: Decimal) -> Result<Decimal> {
    if rate <= -Decimal::ONE {
        return Err(FairmarkError::RateOutOfRange(rate));
    }
    rates::inverse_pow(num::one_plus(rate)?, t)
}

/// Names the rate field of the method in an `FM-E003` raised by a shared helper (finding 4 of
/// the review of issue #176): the helpers only know the value, the method knows the field.
fn rate_field(field: &'static str) -> impl Fn(FairmarkError) -> FairmarkError {
    move |e| match e {
        FairmarkError::RateOutOfRange(rate) => FairmarkError::DiscountRateOutOfRange {
            field: field.to_owned(),
            rate,
        },
        other => other,
    }
}

/// `FM-E003` naming `field` when `rate` is −100% or lower.
fn check_rate(field: impl Into<String>, rate: Decimal) -> Result<()> {
    if rate <= -Decimal::ONE {
        return Err(FairmarkError::DiscountRateOutOfRange {
            field: field.into(),
            rate,
        });
    }
    Ok(())
}

fn npv_raw(rate: Decimal, cash_flows: &[Decimal]) -> Result<Decimal> {
    let mut total = Decimal::ZERO;
    for (i, cf) in cash_flows.iter().enumerate() {
        let df = discount_factor(rate, Decimal::from(i as u64))?;
        total = total
            .checked_add(cf.checked_mul(df).ok_or(FairmarkError::Overflow("npv"))?)
            .ok_or(FairmarkError::Overflow("npv"))?;
    }
    Ok(total)
}

/// d NPV / d rate, used by Newton's method.
fn npv_derivative(rate: Decimal, cash_flows: &[Decimal]) -> Result<Decimal> {
    let mut total = Decimal::ZERO;
    for (i, cf) in cash_flows.iter().enumerate().skip(1) {
        let t = Decimal::from(i as u64);
        let df = discount_factor(rate, t + Decimal::ONE)?;
        let term = (-t)
            .checked_mul(*cf)
            .and_then(|v| v.checked_mul(df))
            .ok_or(FairmarkError::Overflow("npv derivative"))?;
        total = total
            .checked_add(term)
            .ok_or(FairmarkError::Overflow("npv derivative"))?;
    }
    Ok(total)
}

// ---------------------------------------------------------------- NPV

/// Input of `tvm.npv`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NpvInput {
    /// Discount rate per period as a decimal fraction (0.10 = 10%).
    pub rate: Decimal,
    /// Cash flows; the first one occurs at t = 0 (no discounting).
    pub cash_flows: Vec<Decimal>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct NpvOutput {
    /// Net present value at t = 0, in the currency units of the cash flows.
    pub npv: Decimal,
    /// Present value of each cash flow at t = 0, in the order of `cash_flows`.
    pub present_values: Vec<Decimal>,
}

pub fn npv(input: &NpvInput) -> Result<(NpvOutput, Explain)> {
    if input.cash_flows.is_empty() {
        return Err(FairmarkError::EmptyCashFlows);
    }
    let mut ex = Explain::new("tvm.npv", "NPV = Σ CF_t / (1 + r)^t");
    ex.convention("cash_flows[0] occurs at t = 0 and is not discounted");

    let mut pvs = Vec::with_capacity(input.cash_flows.len());
    for (i, cf) in input.cash_flows.iter().enumerate() {
        let df =
            discount_factor(input.rate, Decimal::from(i as u64)).map_err(rate_field("/rate"))?;
        let pv = cf.checked_mul(df).ok_or(FairmarkError::Overflow("npv"))?;
        ex.step(
            format!("Present value, t = {i}"),
            format!("{cf} / (1 + {})^{i}", input.rate),
            pv,
        );
        pvs.push(pv);
    }
    let total = pvs
        .iter()
        .try_fold(Decimal::ZERO, |acc, v| acc.checked_add(*v))
        .ok_or(FairmarkError::Overflow("npv"))?;
    ex.step("Net present value", "Σ present values", total);
    Ok((
        NpvOutput {
            npv: total,
            present_values: pvs,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- IRR

/// Input of `tvm.irr`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IrrInput {
    /// Cash flows; the first one occurs at t = 0. Needs at least one sign change.
    pub cash_flows: Vec<Decimal>,
    /// Starting point for Newton's method, decimal fraction per period. Default: 0.10.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guess: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct IrrOutput {
    /// Internal rate of return per period as a decimal fraction.
    pub irr: Decimal,
    /// True when no other rate can solve the equation (see `explain.conventions`).
    pub uniqueness_verified: bool,
    /// NPV evaluated at the returned IRR (should be ~0).
    pub npv_at_irr: Decimal,
    /// "newton", "bisection" or "grid (exact root)".
    pub solver: String,
    /// Iterations of the solver that produced the rate (0 for an exact grid root).
    pub iterations: u32,
    /// Stopping tolerance on the rate step, decimal fraction (1e-18).
    pub tolerance: Decimal,
}

pub fn irr(input: &IrrInput) -> Result<(IrrOutput, Explain)> {
    let cfs = &input.cash_flows;
    if cfs.is_empty() {
        return Err(FairmarkError::EmptyCashFlows);
    }
    require_sign_change(cfs.iter())?;

    let mut ex = Explain::new("tvm.irr", "find r such that Σ CF_t / (1 + r)^t = 0");
    ex.convention("cash_flows[0] occurs at t = 0");
    solver_conventions(&mut ex, "rates per period (the period of the cash flows)");
    let guess = guess_convention(&mut ex, input.guess);
    let sol = solve_rate(
        "IRR",
        |r| npv_raw(r, cfs),
        |r| npv_derivative(r, cfs),
        guess,
        sign_changes(cfs.iter()),
        &solver_grid(),
    )?;
    ex.convention(sol.uniqueness.clone());
    let npv_at = npv_raw(sol.rate, cfs)?;
    sol.location_steps(&mut ex, "NPV");
    ex.step(
        "Internal rate of return",
        format!("{}, {} iterations", sol.solver, sol.iterations),
        sol.rate,
    );
    ex.step("NPV at IRR (check)", format!("NPV({})", sol.rate), npv_at);

    Ok((
        IrrOutput {
            irr: sol.rate,
            uniqueness_verified: sol.unique,
            npv_at_irr: npv_at,
            solver: sol.solver.to_owned(),
            iterations: sol.iterations,
            tolerance: IRR_TOLERANCE,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- rate solver

/// Rates scanned before solving (ADR 0008): -0.99, -0.95, -0.90 to 1.00 step 0.05,
/// 1.25 to 10 step 0.25.
pub fn solver_grid() -> Vec<Decimal> {
    let mut g = vec![Decimal::new(-99, 2), Decimal::new(-95, 2)];
    g.extend((-18..=20).map(|k| Decimal::new(k * 5, 2)));
    g.extend((5..=40).map(|k| Decimal::new(k * 25, 2)));
    g
}

/// The solver conventions. `rates_scanned` says what the grid points are: rates per period
/// for IRR, effective annual rates for XIRR.
pub(crate) fn solver_conventions(ex: &mut Explain, rates_scanned: &str) {
    ex.convention(format!(
        "roots located on a fixed grid of {rates_scanned}: -0.99, -0.95, -0.90 to 1.00 step \
         0.05, 1.25 to 10 step 0.25; an ambiguous rate is an error, never a choice"
    ));
    ex.convention("Newton's method from the guess inside the root interval; bisection as fallback");
    ex.convention(format!(
        "stopping rule: step on the rate below {IRR_TOLERANCE}, at most {IRR_MAX_ITERATIONS} \
         iterations (a step, not an error bound: accuracy is lower near a double root)"
    ));
}

/// The starting point of Newton's method, reported with whether it is the default.
pub(crate) fn guess_convention(ex: &mut Explain, guess: Option<Decimal>) -> Decimal {
    match guess {
        Some(g) => {
            ex.convention(format!("Newton starting point: {g} (given)"));
            g
        }
        None => {
            let g = Decimal::new(10, 2);
            ex.convention(format!("Newton starting point: {g} (default)"));
            g
        }
    }
}

/// [`solver_grid`] of effective annual rates expressed in the given compounding (audit D4),
/// so that the scanned range of economic rates does not depend on the quotation.
pub(crate) fn solver_grid_for(c: Compounding) -> Result<Vec<Decimal>> {
    solver_grid()
        .into_iter()
        .map(|r| rates::equivalent_rate(r, Compounding::Annual, c))
        .collect()
}

/// Number of sign changes in a sequence of flows, ignoring zeros (Descartes' rule of signs:
/// an upper bound on the number of positive roots in 1 + r).
pub(crate) fn sign_changes<'a>(flows: impl Iterator<Item = &'a Decimal>) -> usize {
    let mut last: Option<bool> = None;
    let mut n = 0;
    for c in flows.filter(|c| !c.is_zero()) {
        let neg = c.is_sign_negative();
        if last.is_some_and(|l| l != neg) {
            n += 1;
        }
        last = Some(neg);
    }
    n
}

pub(crate) fn require_sign_change<'a>(flows: impl Iterator<Item = &'a Decimal>) -> Result<()> {
    let (mut pos, mut neg) = (false, false);
    for c in flows {
        pos |= *c > Decimal::ZERO;
        neg |= *c < Decimal::ZERO;
    }
    if pos && neg {
        Ok(())
    } else {
        Err(FairmarkError::NoSignChange)
    }
}

pub(crate) struct Solution {
    pub rate: Decimal,
    pub solver: &'static str,
    pub iterations: u32,
    /// True when the root is proven unique on (-1, +inf): roots found = sign changes.
    pub unique: bool,
    /// Explanation of the uniqueness verdict, for `explain.conventions`.
    pub uniqueness: String,
    /// Where the grid located the root, for the audit trail: an exact grid point, or the grid
    /// interval with the value of the function at both ends; `None` when Newton's method ran
    /// from the guess without an interval.
    pub located: Option<Located>,
}

/// Where the grid scan located the root.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Located {
    Exact(Decimal),
    Between {
        lo: Decimal,
        f_lo: Decimal,
        hi: Decimal,
        f_hi: Decimal,
    },
}

impl Solution {
    /// The explain steps that show where the grid located the root (`value_name` is `NPV` or
    /// `XNPV`).
    pub(crate) fn location_steps(&self, ex: &mut Explain, value_name: &str) {
        match self.located {
            Some(Located::Exact(r)) => {
                ex.step(
                    "Root located on the grid (exact grid point)",
                    format!("{value_name}({r}) = 0"),
                    r,
                );
            }
            Some(Located::Between { lo, f_lo, hi, f_hi }) => {
                ex.step(
                    "Root interval on the grid, lower rate",
                    format!("{value_name}({lo}) = {f_lo}"),
                    lo,
                );
                ex.step(
                    "Root interval on the grid, upper rate",
                    format!("{value_name}({hi}) = {f_hi}"),
                    hi,
                );
            }
            None => {}
        }
    }
}

enum Root {
    Exact(Decimal),
    Between(Decimal, Decimal, Decimal, Decimal),
}

/// Deterministic root finder for a present-value function of the rate.
///
/// 1. Evaluate `f` on `grid`; points where `f` overflows are skipped.
/// 2. Count the roots found (exact grid points and sign changes) and compare them with the
///    number of sign changes `s` of the flows. By Descartes' rule of signs the number of
///    rates in (-1, +inf) is `s` minus an even number. Hence:
///    * more than one root found: `FM-E011`;
///    * `s - found` odd: at least one more rate exists, `FM-E011`;
///    * no root found and `s > 1`: zero or several rates, `FM-E011`;
///    * one root found and `s - found` even and positive: the result is returned with
///      `unique = false` (further rates cannot be excluded) and the reason is reported;
///    * one root found and `s = 1`: the rate is unique.
/// 3. One interval: Newton from `guess`, accepted only inside the interval; otherwise
///    bisection inside the interval. No interval and `s = 1`: Newton from `guess`, accepted
///    only if it converges, else `FM-E005` naming `solver` (`IRR` or `XIRR`).
pub(crate) fn solve_rate(
    solver: &'static str,
    f: impl Fn(Decimal) -> Result<Decimal>,
    df: impl Fn(Decimal) -> Result<Decimal>,
    guess: Decimal,
    sign_changes: usize,
    grid: &[Decimal],
) -> Result<Solution> {
    let mut roots = Vec::new();
    let mut prev: Option<(Decimal, Decimal)> = None;
    for &r in grid {
        let Ok(v) = f(r) else { continue };
        if v.is_zero() {
            roots.push(Root::Exact(r));
            prev = None;
            continue;
        }
        if let Some((pr, pv)) = prev {
            if pv.is_sign_negative() != v.is_sign_negative() {
                roots.push(Root::Between(pr, pv, r, v));
            }
        }
        prev = Some((r, v));
    }
    let describe = |rs: &[Root]| {
        rs.iter()
            .map(|r| match r {
                Root::Exact(x) => format!("{{{x}}}"),
                Root::Between(a, _, b, _) => format!("[{a}, {b}]"),
            })
            .collect::<Vec<_>>()
            .join(", ")
    };
    let found = roots.len();
    if found > 1 {
        return Err(FairmarkError::MultipleRates(format!(
            "the grid finds roots in {}",
            describe(&roots)
        )));
    }
    let gap = sign_changes.saturating_sub(found);
    if found == 1 && gap % 2 == 1 {
        return Err(FairmarkError::MultipleRates(format!(
            "the grid finds a root in {} and at least one more exists outside the grid or in a \
             pair inside one grid interval (the flows change sign {sign_changes} times)",
            describe(&roots)
        )));
    }
    if found == 0 && sign_changes > 1 {
        return Err(FairmarkError::MultipleRates(format!(
            "the grid isolates no root while the flows change sign {sign_changes} times, so \
             zero or several rates exist"
        )));
    }
    let (unique, uniqueness) = if sign_changes == 1 {
        (
            true,
            "uniqueness proven: one sign change in the flows (Descartes' rule of signs)".to_owned(),
        )
    } else {
        (
            false,
            format!(
                "uniqueness not verified: the flows change sign {sign_changes} times; one rate \
                 found on the grid, further rates in pairs cannot be excluded"
            ),
        )
    };
    let located = match roots.as_slice() {
        [Root::Exact(r)] => Some(Located::Exact(*r)),
        [Root::Between(lo, f_lo, hi, f_hi)] => Some(Located::Between {
            lo: *lo,
            f_lo: *f_lo,
            hi: *hi,
            f_hi: *f_hi,
        }),
        _ => None,
    };
    let solution = |rate, method, iterations| Solution {
        rate,
        solver: method,
        iterations,
        unique,
        uniqueness: uniqueness.clone(),
        located,
    };
    match roots.as_slice() {
        [] => newton(&f, &df, guess, None)
            .map(|(rate, it)| solution(rate, "newton", it))
            .ok_or(FairmarkError::NoConvergence {
                solver,
                iterations: IRR_MAX_ITERATIONS,
            }),
        [Root::Exact(r)] => Ok(solution(*r, "grid (exact root)", 0)),
        [Root::Between(lo, _, hi, _)] => {
            if let Some((rate, it)) = newton(&f, &df, guess, Some((*lo, *hi))) {
                return Ok(solution(rate, "newton", it));
            }
            let (rate, it) = bisection(solver, &f, *lo, *hi)?;
            Ok(solution(rate, "bisection", it))
        }
        _ => unreachable!("more than one root returned above"),
    }
}

fn newton(
    f: &impl Fn(Decimal) -> Result<Decimal>,
    df: &impl Fn(Decimal) -> Result<Decimal>,
    guess: Decimal,
    bracket: Option<(Decimal, Decimal)>,
) -> Option<(Decimal, u32)> {
    let mut r = guess;
    for it in 1..=IRR_MAX_ITERATIONS {
        let v = f(r).ok()?;
        let d = df(r).ok()?;
        if d.is_zero() {
            return None;
        }
        let next = r.checked_sub(v.checked_div(d)?)?;
        if next <= -Decimal::ONE {
            return None;
        }
        if num::sub(next, r).ok()?.abs() <= IRR_TOLERANCE {
            return match bracket {
                Some((lo, hi)) if next < lo || next > hi => None,
                _ => Some((next, it)),
            };
        }
        r = next;
    }
    None
}

fn bisection(
    solver: &'static str,
    f: &impl Fn(Decimal) -> Result<Decimal>,
    mut lo: Decimal,
    mut hi: Decimal,
) -> Result<(Decimal, u32)> {
    let mut f_lo = f(lo)?;
    for it in 1..=IRR_MAX_ITERATIONS {
        let mid = num::div(num::add(lo, hi)?, Decimal::TWO)?;
        let f_mid = f(mid)?;
        if num::sub(hi, lo)? <= IRR_TOLERANCE || f_mid.is_zero() {
            return Ok((mid, it));
        }
        if f_mid.is_sign_negative() == f_lo.is_sign_negative() {
            lo = mid;
            f_lo = f_mid;
        } else {
            hi = mid;
        }
    }
    Err(FairmarkError::NoConvergence {
        solver,
        iterations: IRR_MAX_ITERATIONS,
    })
}

// ---------------------------------------------------------------- present value

/// When each periodic flow occurs within its period.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PeriodTiming {
    /// At the end of the period: flow k at t = k.
    EndOfPeriod,
    /// Spread evenly: flow k at t = k − 0.5.
    MidPeriod,
    /// At the beginning of the period: flow k at t = k − 1.
    BeginningOfPeriod,
}

impl PeriodTiming {
    fn label(self) -> &'static str {
        match self {
            Self::EndOfPeriod => "end of period (flow k at t = k)",
            Self::MidPeriod => "mid period (flow k at t = k − 0.5, half a period at rate r_k)",
            Self::BeginningOfPeriod => "beginning of period (flow k at t = k − 1)",
        }
    }
}

/// Input of `tvm.present_value`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PresentValueInput {
    /// Cash flows of periods 1, 2, ..., n, in currency units.
    pub cash_flows: Vec<Decimal>,
    /// One discount rate for every period, decimal fraction per period. Give exactly one of
    /// `rate` and `period_rates`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rate: Option<Decimal>,
    /// One discount rate per period (forward period rates, decimal fractions), exactly one per
    /// cash flow.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_rates: Option<Vec<Decimal>>,
    /// When flows occur within each period.
    pub timing: PeriodTiming,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PresentValueOutput {
    /// Sum of the present values.
    pub present_value: Decimal,
    /// Discount factor applied to each flow.
    pub discount_factors: Vec<Decimal>,
    /// Present value of each flow.
    pub present_values: Vec<Decimal>,
}

/// Discount factors of periods 1..n for a sequence of period rates (SPEC-0002 R5).
pub fn period_discount_factors(rates: &[Decimal], timing: PeriodTiming) -> Result<Vec<Decimal>> {
    let mut cum = Decimal::ONE;
    let mut out = Vec::with_capacity(rates.len());
    for r in rates {
        if *r <= -Decimal::ONE {
            return Err(FairmarkError::RateOutOfRange(*r));
        }
        let base = num::one_plus(*r)?;
        let prev = cum;
        cum = cum
            .checked_div(base)
            .ok_or(FairmarkError::Overflow("discount factor"))?;
        out.push(match timing {
            PeriodTiming::EndOfPeriod => cum,
            PeriodTiming::BeginningOfPeriod => prev,
            PeriodTiming::MidPeriod => {
                let root = base.sqrt().ok_or(FairmarkError::Overflow("square root"))?;
                prev.checked_div(root)
                    .ok_or(FairmarkError::Overflow("discount factor"))?
            }
        });
    }
    Ok(out)
}

pub fn present_value(input: &PresentValueInput) -> Result<(PresentValueOutput, Explain)> {
    let n = input.cash_flows.len();
    if n == 0 {
        return Err(FairmarkError::EmptyCashFlows);
    }
    let rates: Vec<Decimal> = match (&input.rate, &input.period_rates) {
        (Some(r), None) => vec![*r; n],
        (None, Some(curve)) if curve.len() == n => curve.clone(),
        (None, Some(curve)) => {
            return Err(FairmarkError::InvalidInput(format!(
                "period_rates has {} rates for {n} cash flows; give exactly one rate per period",
                curve.len()
            )))
        }
        _ => {
            return Err(FairmarkError::InvalidInput(
                "give exactly one of `rate` and `period_rates`".to_owned(),
            ))
        }
    };
    let mut ex = Explain::new(
        "tvm.present_value",
        "PV = Σ CF_k × DF_k, DF_k = Π_{i ≤ k} (1 + r_i)^-1 (adjusted for timing)",
    );
    ex.convention(format!("timing: {}", input.timing.label()));
    ex.convention(if input.rate.is_some() {
        "one rate for every period"
    } else {
        "one rate per period (forward period rates)"
    });
    for (k, r) in rates.iter().enumerate() {
        match input.rate {
            Some(_) => check_rate("/rate", *r)?,
            None => check_rate(format!("/period_rates/{k}"), *r)?,
        }
    }
    let dfs = period_discount_factors(&rates, input.timing)?;
    let ends = period_discount_factors(&rates, PeriodTiming::EndOfPeriod)?;
    let mut pvs = Vec::with_capacity(n);
    for (k, (cf, df)) in input.cash_flows.iter().zip(&dfs).enumerate() {
        let pv = cf
            .checked_mul(*df)
            .ok_or(FairmarkError::Overflow("present value"))?;
        let start = if k == 0 { Decimal::ONE } else { ends[k - 1] };
        let formula = match input.timing {
            PeriodTiming::EndOfPeriod => format!("{start} / (1 + {})", rates[k]),
            PeriodTiming::MidPeriod => format!("{start} / (1 + {})^0.5", rates[k]),
            PeriodTiming::BeginningOfPeriod => {
                format!("{start} (discount factor at the start of the period)")
            }
        };
        ex.step(format!("Period {}: discount factor", k + 1), formula, *df);
        ex.step(
            format!("Period {}: present value", k + 1),
            format!("{cf} × {df}"),
            pv,
        );
        pvs.push(pv);
    }
    let total = sum(&pvs, "present value")?;
    ex.step("Present value", "Σ present values", total);
    Ok((
        PresentValueOutput {
            present_value: total,
            discount_factors: dfs,
            present_values: pvs,
        },
        ex,
    ))
}

fn sum(values: &[Decimal], what: &'static str) -> Result<Decimal> {
    values
        .iter()
        .try_fold(Decimal::ZERO, |acc, v| acc.checked_add(*v))
        .ok_or(FairmarkError::Overflow(what))
}

// ---------------------------------------------------------------- dated flows

/// An amount paid or received on a date.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatedCashFlow {
    /// Date of the flow (ISO 8601).
    pub date: Date,
    /// Amount in currency units; negative for payments, positive for receipts.
    pub amount: Decimal,
}

/// Input of `tvm.xnpv`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct XnpvInput {
    /// Annual discount rate, decimal fraction (0.09 = 9%), quoted with `compounding`.
    pub rate: Decimal,
    /// Dated cash flows, in any order.
    pub cash_flows: Vec<DatedCashFlow>,
    /// Date at which the value is computed. Default: the earliest flow date (ECMA-376 XNPV).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub valuation_date: Option<Date>,
    /// Day count for the time between dates. Default: `act_365_fixed` (ECMA-376 XNPV).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_count: Option<DayCount>,
    /// Compounding of `rate`. Default: `annual` (ECMA-376 XNPV).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compounding: Option<Compounding>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct DatedFlowValue {
    /// Date of the flow (ISO 8601).
    pub date: Date,
    /// Time from the valuation date in years.
    pub year_fraction: Decimal,
    /// Discount factor from the date of the flow to the valuation date.
    pub discount_factor: Decimal,
    /// Value of the flow at the valuation date, in currency units.
    pub present_value: Decimal,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct XnpvOutput {
    /// Net present value at the valuation date, in currency units.
    pub xnpv: Decimal,
    /// Valuation date used: the given one, or the earliest flow date (ISO 8601).
    pub valuation_date: Date,
    /// Year fraction, discount factor and present value of each flow, in input order.
    pub flows: Vec<DatedFlowValue>,
}

/// Year fractions of each flow from the valuation date (SPEC-0002 R6, R7), with the day-count
/// calculation of each one for the audit trail.
pub(crate) struct TimeAxis {
    pub valuation_date: Date,
    pub times: Vec<Decimal>,
    pub formulas: Vec<String>,
}

pub(crate) fn time_axis(
    flows: &[DatedCashFlow],
    valuation_date: Option<Date>,
    day_count: DayCount,
) -> Result<TimeAxis> {
    let earliest = flows
        .iter()
        .map(|f| f.date)
        .min()
        .ok_or(FairmarkError::EmptyCashFlows)?;
    let v = valuation_date.unwrap_or(earliest);
    let mut times = Vec::with_capacity(flows.len());
    let mut formulas = Vec::with_capacity(flows.len());
    for f in flows {
        if f.date < v {
            return Err(FairmarkError::FlowBeforeValuationDate {
                date: f.date.to_string(),
                valuation_date: v.to_string(),
            });
        }
        let (t, formula) = day_count.year_fraction(v, f.date)?;
        times.push(t);
        formulas.push(formula);
    }
    Ok(TimeAxis {
        valuation_date: v,
        times,
        formulas,
    })
}

fn dated_npv(
    rate: Decimal,
    c: Compounding,
    flows: &[DatedCashFlow],
    ts: &[Decimal],
) -> Result<Decimal> {
    let mut total = Decimal::ZERO;
    for (f, t) in flows.iter().zip(ts) {
        let pv = f
            .amount
            .checked_mul(rates::discount_factor(rate, c, *t)?)
            .ok_or(FairmarkError::Overflow("xnpv"))?;
        total = total
            .checked_add(pv)
            .ok_or(FairmarkError::Overflow("xnpv"))?;
    }
    Ok(total)
}

fn dated_npv_derivative(
    rate: Decimal,
    c: Compounding,
    flows: &[DatedCashFlow],
    ts: &[Decimal],
) -> Result<Decimal> {
    let mut total = Decimal::ZERO;
    for (f, t) in flows.iter().zip(ts) {
        let d = f
            .amount
            .checked_mul(rates::discount_factor_derivative(rate, c, *t)?)
            .ok_or(FairmarkError::Overflow("xnpv derivative"))?;
        total = total
            .checked_add(d)
            .ok_or(FairmarkError::Overflow("xnpv derivative"))?;
    }
    Ok(total)
}

/// Amounts in time order, netting flows that fall at the same time `t`. Under 30/360 two
/// different dates can share a year fraction, so netting is on `t`, not on the date (the
/// sign-change count must describe the function that is solved).
fn net_by_time(flows: &[DatedCashFlow], ts: &[Decimal]) -> Result<Vec<Decimal>> {
    let mut pairs: Vec<(Decimal, Decimal)> = ts
        .iter()
        .copied()
        .zip(flows.iter().map(|f| f.amount))
        .collect();
    pairs.sort_by_key(|p| p.0);
    let mut out: Vec<(Decimal, Decimal)> = Vec::new();
    for (t, amount) in pairs {
        match out.last_mut() {
            Some((last_t, last)) if *last_t == t => *last = num::add(*last, amount)?,
            _ => out.push((t, amount)),
        }
    }
    Ok(out.into_iter().map(|(_, a)| a).collect())
}

/// The discount factor written out with the numbers substituted.
fn discount_formula(rate: Decimal, c: Compounding, t: Decimal) -> String {
    match c.per_year() {
        Some(1) => format!("1 / (1 + {rate})^{t}"),
        Some(m) => format!("1 / (1 + {rate} / {m})^({m} × {t})"),
        None => format!("e^(−{rate} × {t})"),
    }
}

/// Reports day count and compounding, marking ECMA-376 defaults, and returns them.
fn dated_conventions(
    ex: &mut Explain,
    dc: Option<DayCount>,
    c: Option<Compounding>,
) -> (DayCount, Compounding) {
    let mark = |given: bool| if given { "given" } else { "default, ECMA-376" };
    let day_count = dc.unwrap_or(DayCount::Act365Fixed);
    let compounding = c.unwrap_or(Compounding::Annual);
    ex.convention(format!(
        "day count: {} ({})",
        day_count.label(),
        mark(dc.is_some())
    ));
    ex.convention(format!(
        "rate quoted with {} ({})",
        compounding.label(),
        mark(c.is_some())
    ));
    (day_count, compounding)
}

pub fn xnpv(input: &XnpvInput) -> Result<(XnpvOutput, Explain)> {
    let mut ex = Explain::new(
        "tvm.xnpv",
        "XNPV = Σ CF_j × DF(r, t_j), t_j = year fraction(valuation date, d_j)",
    );
    let (dc, c) = dated_conventions(&mut ex, input.day_count, input.compounding);
    let axis = time_axis(&input.cash_flows, input.valuation_date, dc)?;
    let (v, ts) = (axis.valuation_date, &axis.times);
    ex.convention(match input.valuation_date {
        Some(_) => format!("valuation date {v} (given)"),
        None => format!(
            "valuation date {v}: earliest flow date (default; ECMA-376 uses the first listed \
             flow and rejects earlier ones, which gives the same value whenever it is valid)"
        ),
    });
    let mut flows = Vec::with_capacity(ts.len());
    for ((f, t), yf_formula) in input.cash_flows.iter().zip(ts).zip(&axis.formulas) {
        ex.step(format!("Year fraction, {}", f.date), yf_formula.clone(), *t);
        let df = rates::discount_factor(input.rate, c, *t)?;
        ex.step(
            format!("Discount factor, {}", f.date),
            discount_formula(input.rate, c, *t),
            df,
        );
        let pv = num::mul(f.amount, df)?;
        ex.step(
            format!("Present value, {}", f.date),
            format!("{} × {df}", f.amount),
            pv,
        );
        flows.push(DatedFlowValue {
            date: f.date,
            year_fraction: *t,
            discount_factor: df,
            present_value: pv,
        });
    }
    let total = flows
        .iter()
        .try_fold(Decimal::ZERO, |acc, f| acc.checked_add(f.present_value))
        .ok_or(FairmarkError::Overflow("xnpv"))?;
    ex.step("Net present value", "Σ present values", total);
    Ok((
        XnpvOutput {
            xnpv: total,
            valuation_date: v,
            flows,
        },
        ex,
    ))
}

/// Input of `tvm.xirr`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct XirrInput {
    /// Dated cash flows, in any order; at least one positive and one negative amount.
    pub cash_flows: Vec<DatedCashFlow>,
    /// Day count for the time between dates. Default: `act_365_fixed` (ECMA-376 XIRR).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub day_count: Option<DayCount>,
    /// Compounding of the returned rate. Default: `annual` (ECMA-376 XIRR).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub compounding: Option<Compounding>,
    /// Starting point for Newton's method, annual decimal fraction. Default: 0.10 (ECMA-376 XIRR).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guess: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct XirrOutput {
    /// Annual rate at which XNPV is zero, decimal fraction, with the chosen compounding.
    pub xirr: Decimal,
    /// True when no other rate can solve the equation (see `explain.conventions`).
    pub uniqueness_verified: bool,
    /// XNPV at the returned rate (should be about zero).
    pub xnpv_at_xirr: Decimal,
    /// "newton", "bisection" or "grid (exact root)".
    pub solver: String,
    /// Iterations of the solver that produced the rate (0 for an exact grid root).
    pub iterations: u32,
    /// Stopping tolerance on the rate step, decimal fraction (1e-18).
    pub tolerance: Decimal,
}

pub fn xirr(input: &XirrInput) -> Result<(XirrOutput, Explain)> {
    let flows = &input.cash_flows;
    if flows.is_empty() {
        return Err(FairmarkError::EmptyCashFlows);
    }
    let mut ex = Explain::new("tvm.xirr", "find r such that Σ CF_j × DF(r, t_j) = 0");
    let (dc, c) = dated_conventions(&mut ex, input.day_count, input.compounding);
    let axis = time_axis(flows, None, dc)?;
    let (v, ts) = (axis.valuation_date, axis.times.clone());
    let netted = net_by_time(flows, &ts)?;
    require_sign_change(netted.iter())?;
    for ((f, t), formula) in flows.iter().zip(&ts).zip(&axis.formulas) {
        ex.step(format!("Year fraction, {}", f.date), formula.clone(), *t);
    }
    ex.convention(format!("times measured from the earliest flow date {v}"));
    solver_conventions(
        &mut ex,
        "effective annual rates, each expressed in the compounding of the result",
    );
    let guess = guess_convention(&mut ex, input.guess);
    let sol = solve_rate(
        "XIRR",
        |r| dated_npv(r, c, flows, &ts),
        |r| dated_npv_derivative(r, c, flows, &ts),
        guess,
        sign_changes(netted.iter()),
        &solver_grid_for(c)?,
    )?;
    ex.convention(sol.uniqueness.clone());
    let check = dated_npv(sol.rate, c, flows, &ts)?;
    sol.location_steps(&mut ex, "XNPV");
    ex.step(
        "Internal rate of return (dated flows)",
        format!("{}, {} iterations", sol.solver, sol.iterations),
        sol.rate,
    );
    ex.step("XNPV at XIRR (check)", format!("XNPV({})", sol.rate), check);
    Ok((
        XirrOutput {
            xirr: sol.rate,
            uniqueness_verified: sol.unique,
            xnpv_at_xirr: check,
            solver: sol.solver.to_owned(),
            iterations: sol.iterations,
            tolerance: IRR_TOLERANCE,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- MIRR

/// Input of `tvm.mirr`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct MirrInput {
    /// Periodic cash flows; the first one occurs at t = 0. At least one positive and one negative.
    pub cash_flows: Vec<Decimal>,
    /// Rate paid on the money invested (applied to negative flows), decimal fraction per period.
    pub finance_rate: Decimal,
    /// Rate earned on reinvested positive flows, decimal fraction per period.
    pub reinvest_rate: Decimal,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct MirrOutput {
    /// Modified internal rate of return per period.
    pub mirr: Decimal,
    /// Present value at t = 0 of the negative flows, at the finance rate.
    pub pv_negative_flows: Decimal,
    /// Value at t = n of the positive flows, at the reinvestment rate.
    pub fv_positive_flows: Decimal,
}

pub fn mirr(input: &MirrInput) -> Result<(MirrOutput, Explain)> {
    let cfs = &input.cash_flows;
    if cfs.len() < 2 {
        return Err(FairmarkError::InvalidInput(
            "MIRR needs at least two cash flows".to_owned(),
        ));
    }
    require_sign_change(cfs.iter())?;
    check_rate("/finance_rate", input.finance_rate)?;
    check_rate("/reinvest_rate", input.reinvest_rate)?;
    let n = (cfs.len() - 1) as i64;
    let mut ex = Explain::new(
        "tvm.mirr",
        "MIRR = (FV of positive flows / −PV of negative flows)^(1/n) − 1",
    );
    ex.convention("cash_flows[0] occurs at t = 0");
    ex.convention("negative flows discounted to t = 0 at the finance rate");
    ex.convention("positive flows compounded to t = n at the reinvestment rate (ECMA-376 MIRR)");
    let (mut pv_neg, mut fv_pos) = (Decimal::ZERO, Decimal::ZERO);
    let (mut neg_terms, mut pos_terms) = (Vec::new(), Vec::new());
    for (t, cf) in cfs.iter().enumerate() {
        let t = t as i64;
        let overflow = FairmarkError::Overflow("MIRR");
        if *cf < Decimal::ZERO {
            let g = rates::pow(num::one_plus(input.finance_rate)?, Decimal::from(t))?;
            let v = cf.checked_div(g).ok_or(overflow.clone())?;
            pv_neg = pv_neg.checked_add(v).ok_or(overflow)?;
            ex.step(
                format!("PV at t = 0 of the flow at t = {t} (finance rate)"),
                format!("{cf} / (1 + {})^{t}", input.finance_rate),
                v,
            );
            neg_terms.push(v);
        } else if *cf > Decimal::ZERO {
            let g = rates::pow(num::one_plus(input.reinvest_rate)?, Decimal::from(n - t))?;
            let v = cf.checked_mul(g).ok_or(overflow.clone())?;
            fv_pos = fv_pos.checked_add(v).ok_or(overflow)?;
            ex.step(
                format!("FV at t = {n} of the flow at t = {t} (reinvestment rate)"),
                format!("{cf} × (1 + {})^({n} − {t})", input.reinvest_rate),
                v,
            );
            pos_terms.push(v);
        }
    }
    let terms = |vs: &[Decimal]| {
        vs.iter()
            .map(|v| format!("({v})"))
            .collect::<Vec<_>>()
            .join(" + ")
    };
    ex.step(
        "PV of negative flows at the finance rate",
        format!(
            "Σ CF_t / (1 + {})^t = {}",
            input.finance_rate,
            terms(&neg_terms)
        ),
        pv_neg,
    );
    ex.step(
        "FV of positive flows at the reinvestment rate",
        format!(
            "Σ CF_t × (1 + {})^({n} − t) = {}",
            input.reinvest_rate,
            terms(&pos_terms)
        ),
        fv_pos,
    );
    let ratio = fv_pos
        .checked_div(-pv_neg)
        .ok_or(FairmarkError::Overflow("MIRR"))?;
    let root = ratio
        .checked_ln()
        .ok_or(FairmarkError::Overflow("MIRR root"))
        .and_then(|l| num::div(l, Decimal::from(n)))?
        .checked_exp()
        .ok_or(FairmarkError::Overflow("MIRR root"))?;
    let m = num::sub(root, Decimal::ONE)?;
    ex.step(
        "Modified internal rate of return",
        format!("({fv_pos} / {})^(1/{n}) − 1", -pv_neg),
        m,
    );
    Ok((
        MirrOutput {
            mirr: m,
            pv_negative_flows: pv_neg,
            fv_positive_flows: fv_pos,
        },
        ex,
    ))
}

// ---------------------------------------------------------------- annuities

/// Largest number of annuity payments accepted (bounds the direct summation).
pub const MAX_PERIODS: u32 = 10_000;

/// Below this distance between rate and growth, annuities are summed directly.
const NEAR_EQUAL: Decimal = Decimal::from_parts(1, 0, 0, false, 9); // 1e-9

fn growth_convention(growth: Option<Decimal>) -> String {
    match growth {
        Some(g) => format!("payment grows by {g} per period (given)"),
        None => "payment grows by 0 per period (default: level payments)".to_owned(),
    }
}

/// When the payments of an annuity or perpetuity occur.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum AnnuityTiming {
    /// Payments at the end of each period (first payment at t = 1).
    Ordinary,
    /// Payments at the beginning of each period (first payment at t = 0).
    Due,
}

/// Input of `tvm.annuity`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AnnuityInput {
    /// First payment, in currency units.
    pub payment: Decimal,
    /// Discount rate per period, decimal fraction.
    pub rate: Decimal,
    /// Number of payments, from 1 to 10000.
    pub periods: u32,
    /// Growth of the payment per period, decimal fraction. Default: 0 (level annuity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth: Option<Decimal>,
    /// `ordinary` (end of period) or `due` (beginning of period).
    pub timing: AnnuityTiming,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct AnnuityOutput {
    /// Present value at t = 0.
    pub present_value: Decimal,
    /// Value at t = n of the same payments (present value × (1 + r)^n).
    pub future_value: Decimal,
    /// Present value of a first payment of 1.
    pub factor: Decimal,
}

/// Present value at t = 0 of a first payment of 1 growing at `g` for `n` periods at rate `r`
/// (the annuity factor), with the explain steps that compute it. Validates `n` (1 to
/// [`MAX_PERIODS`], `FM-E001`), `r` (`FM-E003`) and `g` (`FM-E001`). Shared by `tvm.annuity`
/// and `tv.limited_horizon`; it never computes a future value, so it cannot overflow on the
/// growth factor (1 + r)^n.
///
/// For a long horizon ((1 + g)/(1 + r))^n can fall below the decimal precision (about
/// 10^−28); the power then underflows, and the factor takes it as 0, its correct limit, with an
/// explain step that says so. The result is the perpetuity factor to within that precision.
pub(crate) fn annuity_factor(
    r: Decimal,
    g: Decimal,
    n: u32,
    timing: AnnuityTiming,
) -> Result<(Decimal, Vec<Step>)> {
    factor_with(r, g, n, timing, true)
}

/// `underflow_to_zero: false` keeps the behaviour of `tvm.annuity` version 1 unchanged: an
/// underflowing power is `FM-E008` there.
fn factor_with(
    r: Decimal,
    g: Decimal,
    n: u32,
    timing: AnnuityTiming,
    underflow_to_zero: bool,
) -> Result<(Decimal, Vec<Step>)> {
    if n == 0 || n > MAX_PERIODS {
        return Err(FairmarkError::InvalidInput(format!(
            "periods must be between 1 and {MAX_PERIODS} (got {n})"
        )));
    }
    if r <= -Decimal::ONE {
        return Err(FairmarkError::RateOutOfRange(r));
    }
    if g <= -Decimal::ONE {
        return Err(FairmarkError::InvalidInput(format!(
            "growth must be greater than -1 (got {g})"
        )));
    }
    let one = Decimal::ONE;
    let periods = Decimal::from(n);
    let one_r = num::one_plus(r)?;
    let one_g = num::one_plus(g)?;
    let diff = num::sub(r, g)?;
    let mut ex = Explain::new("", "");
    let ordinary_factor = if r == g {
        let f = periods
            .checked_div(one_r)
            .ok_or(FairmarkError::Overflow("annuity factor"))?;
        ex.step("Annuity factor (r = g)", format!("{n} / (1 + {r})"), f);
        f
    } else if diff.abs() < NEAR_EQUAL {
        // Closed form cancels catastrophically when r and g almost coincide (audit D5):
        // sum the geometric series by Horner's rule instead.
        let ratio = one_g
            .checked_div(one_r)
            .ok_or(FairmarkError::Overflow("annuity factor"))?;
        let mut acc = Decimal::ZERO;
        for _ in 0..n {
            acc = acc
                .checked_mul(ratio)
                .and_then(|a| a.checked_add(one))
                .ok_or(FairmarkError::Overflow("annuity factor"))?;
        }
        let f = acc
            .checked_div(one_r)
            .ok_or(FairmarkError::Overflow("annuity factor"))?;
        ex.step(
            "Annuity factor (r close to g, direct sum)",
            format!("Σ_(k=1..{n}) ((1 + {g}) / (1 + {r}))^(k−1) / (1 + {r})"),
            f,
        );
        f
    } else {
        let ratio = one_g
            .checked_div(one_r)
            .ok_or(FairmarkError::Overflow("annuity factor"))?;
        let q = match rates::pow(ratio, periods) {
            // Only a ratio below 1 can underflow; its power is then below 10^−28.
            Err(FairmarkError::Overflow(_)) if underflow_to_zero && ratio < one => {
                ex.step(
                    "Growth-to-discount ratio to the power n (below the decimal precision, taken as 0)",
                    format!("((1 + {g}) / (1 + {r}))^{n}"),
                    Decimal::ZERO,
                );
                Decimal::ZERO
            }
            other => other?,
        };
        let f = num::sub(one, q)?
            .checked_div(diff)
            .ok_or(FairmarkError::Overflow("annuity factor"))?;
        ex.step(
            "Annuity factor",
            format!("[1 − ((1 + {g}) / (1 + {r}))^{n}] / ({r} − {g})"),
            f,
        );
        f
    };
    let factor = match timing {
        AnnuityTiming::Ordinary => ordinary_factor,
        AnnuityTiming::Due => {
            let f = ordinary_factor
                .checked_mul(one_r)
                .ok_or(FairmarkError::Overflow("annuity factor"))?;
            ex.step(
                "Annuity due factor",
                format!("{ordinary_factor} × (1 + {r})"),
                f,
            );
            f
        }
    };
    Ok((factor, ex.steps))
}

pub fn annuity(input: &AnnuityInput) -> Result<(AnnuityOutput, Explain)> {
    let (r, g, n) = (input.rate, input.growth.unwrap_or_default(), input.periods);
    let (factor, steps) = factor_with(r, g, n, input.timing, false).map_err(rate_field("/rate"))?;
    let periods = Decimal::from(n);
    let one_r = num::one_plus(r)?;
    let mut ex = Explain::new(
        "tvm.annuity",
        "PV = P × [1 − ((1 + g)/(1 + r))^n] / (r − g); P × n / (1 + r) when r = g; × (1 + r) if due",
    );
    ex.convention(match input.timing {
        AnnuityTiming::Ordinary => "ordinary annuity: payments at the end of each period",
        AnnuityTiming::Due => "annuity due: payments at the beginning of each period",
    });
    ex.convention(growth_convention(input.growth));
    ex.convention("future value at t = n for both timings");
    ex.steps.extend(steps);
    let pv = input
        .payment
        .checked_mul(factor)
        .ok_or(FairmarkError::Overflow("annuity"))?;
    ex.step("Present value", format!("{} × {factor}", input.payment), pv);
    let fv = pv
        .checked_mul(rates::pow(one_r, periods)?)
        .ok_or(FairmarkError::Overflow("annuity future value"))?;
    ex.step("Future value at t = n", format!("{pv} × (1 + {r})^{n}"), fv);
    Ok((
        AnnuityOutput {
            present_value: pv,
            future_value: fv,
            factor,
        },
        ex,
    ))
}

/// Input of `tvm.perpetuity`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PerpetuityInput {
    /// First payment, in currency units.
    pub payment: Decimal,
    /// Discount rate per period, decimal fraction; must exceed `growth`.
    pub rate: Decimal,
    /// Growth of the payment per period, decimal fraction, greater than -1. Default: 0 (level
    /// perpetuity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub growth: Option<Decimal>,
    /// `ordinary` (first payment at t = 1) or `due` (first payment at t = 0).
    pub timing: AnnuityTiming,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct PerpetuityOutput {
    /// Present value at t = 0.
    pub present_value: Decimal,
}

pub fn perpetuity(input: &PerpetuityInput) -> Result<(PerpetuityOutput, Explain)> {
    let (r, g) = (input.rate, input.growth.unwrap_or_default());
    check_rate("/rate", r)?;
    if g <= -Decimal::ONE {
        // With g ≤ -1 the payments alternate or vanish and the series diverges or is
        // meaningless (audit D6); aligned with `tvm.annuity`.
        return Err(FairmarkError::InvalidInput(format!(
            "growth must be greater than -1 (got {g})"
        )));
    }
    if g >= r {
        return Err(FairmarkError::GrowthNotBelowTerminalRate {
            growth_field: "/growth".to_owned(),
            growth: g,
            rate_field: "/rate".to_owned(),
            rate: r,
        });
    }
    let mut ex = Explain::new("tvm.perpetuity", "PV = P / (r − g); × (1 + r) if due");
    ex.convention(match input.timing {
        AnnuityTiming::Ordinary => "first payment at the end of the first period",
        AnnuityTiming::Due => "first payment immediately (perpetuity due)",
    });
    ex.convention(growth_convention(input.growth));
    let mut pv = input
        .payment
        .checked_div(num::sub(r, g)?)
        .ok_or(FairmarkError::Overflow("perpetuity"))?;
    ex.step(
        "Present value, ordinary",
        format!("{} / ({r} − {g})", input.payment),
        pv,
    );
    if input.timing == AnnuityTiming::Due {
        let due = pv
            .checked_mul(num::one_plus(r)?)
            .ok_or(FairmarkError::Overflow("perpetuity"))?;
        ex.step("Present value, due", format!("{pv} × (1 + {r})"), due);
        pv = due;
    }
    Ok((PerpetuityOutput { present_value: pv }, ex))
}

// ---------------------------------------------------------------- future value

/// The steps of [`annuity_factor`] for level payments (growth 0), relabelled with the level
/// formulas: same values, no `(1 + 0)` in the text. Used by `tvm.future_value` and the loan
/// methods; `tvm.annuity` keeps its own labels.
pub(crate) fn level_factor_steps(steps: Vec<Step>, r: Decimal, n: u32) -> Vec<Step> {
    steps
        .into_iter()
        .map(|step| {
            let (label, formula) = match step.label.as_str() {
                "Annuity factor" => (
                    "Annuity factor (level payments)".to_owned(),
                    format!("[1 − (1 + {r})^−{n}] / {r}"),
                ),
                "Annuity factor (r close to g, direct sum)" => (
                    "Annuity factor (level payments, rate close to 0, direct sum)".to_owned(),
                    format!("Σ_(k=1..{n}) (1 + {r})^−k"),
                ),
                "Annuity factor (r = g)" => (
                    "Annuity factor (level payments, rate 0)".to_owned(),
                    format!("{n} / (1 + {r})"),
                ),
                _ => (step.label, step.formula),
            };
            Step {
                label,
                formula,
                value: step.value,
            }
        })
        .collect()
}

/// Input of `tvm.future_value`.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FutureValueInput {
    /// Amount at t = 0, in currency units, accumulated to t = n. Default: 0. Give at least one
    /// of `present_value` and `payment`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub present_value: Option<Decimal>,
    /// Level payment of every period, in currency units. Default: 0. An amount of the same sign
    /// as the present value accumulates in the same direction; a negative amount is a
    /// withdrawal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payment: Option<Decimal>,
    /// Rate per period, decimal fraction (0.01 = 1% per period). No annual-rate conversion: for
    /// an annual rate, convert it first (`rates.convert`) and divide by the periods per year.
    pub rate: Decimal,
    /// Number of periods, from 1 to 10000.
    pub periods: u32,
    /// `ordinary` (payments at the end of each period) or `due` (at the beginning).
    pub timing: AnnuityTiming,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct FutureValueOutput {
    /// Value at t = n of the present amount and of the payments, in currency units.
    pub future_value: Decimal,
}

pub fn future_value(input: &FutureValueInput) -> Result<(FutureValueOutput, Explain)> {
    let (r, n) = (input.rate, input.periods);
    if input.present_value.is_none() && input.payment.is_none() {
        return Err(FairmarkError::InvalidInput(
            "give `present_value`, `payment` or both (each defaults to 0, but not both)".to_owned(),
        ));
    }
    if n == 0 || n > MAX_PERIODS {
        return Err(FairmarkError::InvalidInput(format!(
            "`periods` must be between 1 and {MAX_PERIODS} (got {n})"
        )));
    }
    check_rate("/rate", r)?;
    let pv = input.present_value.unwrap_or_default();
    let pmt = input.payment.unwrap_or_default();
    let mut ex = Explain::new(
        "tvm.future_value",
        "FV = PV (1 + r)^n + A ((1 + r)^n − 1) / r, A the level payment, the payment term \
         × (1 + r) if due; PV + n A when r = 0",
    );
    ex.convention(match input.timing {
        AnnuityTiming::Ordinary => "ordinary: payments at the end of each period",
        AnnuityTiming::Due => "due: payments at the beginning of each period",
    });
    ex.convention(
        "sign convention: amounts of the same sign accumulate in the same direction, a negative \
         amount is a withdrawal (not the cash-flow sign convention of spreadsheets)",
    );
    ex.convention("rate per period, no annual-rate conversion; value at t = n for both timings");
    ex.convention(match input.present_value {
        Some(v) => format!("present value: {v} (given)"),
        None => "present value: 0 (default)".to_owned(),
    });
    ex.convention(match input.payment {
        Some(v) => format!("payment: {v} (given)"),
        None => "payment: 0 (default)".to_owned(),
    });
    let periods = Decimal::from(n);
    let fv = if r.is_zero() {
        let fv = num::add(pv, num::mul(periods, pmt)?)?;
        ex.step(
            "Future value (rate zero, limit form)",
            format!("{pv} + {n} × {pmt}"),
            fv,
        );
        fv
    } else {
        let growth = rates::pow(num::one_plus(r)?, periods)?;
        ex.step("Growth factor", format!("(1 + {r})^{n}"), growth);
        let fv_pv = num::mul(pv, growth)?;
        ex.step(
            "Present value accumulated to t = n",
            format!("{pv} × {growth}"),
            fv_pv,
        );
        let (factor, steps) = annuity_factor(r, Decimal::ZERO, n, input.timing)?;
        ex.steps.extend(level_factor_steps(steps, r, n));
        let accumulation = num::mul(factor, growth)?;
        ex.step(
            "Accumulation factor of the payments",
            format!("{factor} × {growth} (= ((1 + {r})^{n} − 1) / {r}, × (1 + {r}) if due)"),
            accumulation,
        );
        let fv_pmt = num::mul(pmt, accumulation)?;
        ex.step(
            "Payments accumulated to t = n",
            format!("{pmt} × {accumulation}"),
            fv_pmt,
        );
        let fv = num::add(fv_pv, fv_pmt)?;
        ex.step("Future value", format!("{fv_pv} + {fv_pmt}"), fv);
        fv
    };
    Ok((FutureValueOutput { future_value: fv }, ex))
}

// ---------------------------------------------------------------- method registrations

/// `tvm.npv`: see the methodology page.
pub struct Npv;

impl Method for Npv {
    const ID: &'static str = "tvm.npv";
    const TITLE: &'static str = "Net present value";
    const DESCRIPTION: &'static str =
        "Net present value of a cash flow series; the first flow is at t = 0.";
    const VERSION: &'static str = "2";
    type Input = NpvInput;
    type Output = NpvOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        npv(input)
    }
}

/// `tvm.irr`: see the methodology page.
pub struct Irr;

impl Method for Irr {
    const ID: &'static str = "tvm.irr";
    const TITLE: &'static str = "Internal rate of return";
    const DESCRIPTION: &'static str =
        "Rate at which the NPV of a periodic cash flow series is zero, solved deterministically; \
        refuses inputs with more than one IRR.";
    const VERSION: &'static str = "2";
    type Input = IrrInput;
    type Output = IrrOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        irr(input)
    }
}

/// `tvm.present_value`: see the methodology page.
pub struct PresentValue;

impl Method for PresentValue {
    const ID: &'static str = "tvm.present_value";
    const TITLE: &'static str = "Present value with a constant or time-varying rate";
    const DESCRIPTION: &'static str =
        "Present value of periodic flows for periods 1..n with one rate or one rate per period, \
        at end, mid or beginning of period. Returns every discount factor.";
    const VERSION: &'static str = "1";
    type Input = PresentValueInput;
    type Output = PresentValueOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        present_value(input)
    }
}

/// `tvm.xnpv`: see the methodology page.
pub struct Xnpv;

impl Method for Xnpv {
    const ID: &'static str = "tvm.xnpv";
    const TITLE: &'static str = "Net present value of dated cash flows (XNPV)";
    const DESCRIPTION: &'static str =
        "Present value of dated flows at a valuation date, with explicit day count and \
        compounding (defaults as in ECMA-376 XNPV).";
    const VERSION: &'static str = "1";
    type Input = XnpvInput;
    type Output = XnpvOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        xnpv(input)
    }
}

/// `tvm.xirr`: see the methodology page.
pub struct Xirr;

impl Method for Xirr {
    const ID: &'static str = "tvm.xirr";
    const TITLE: &'static str = "Internal rate of return of dated cash flows (XIRR)";
    const DESCRIPTION: &'static str =
        "Annual rate at which the XNPV of dated flows is zero, solved deterministically; refuses \
        inputs with more than one rate.";
    const VERSION: &'static str = "1";
    type Input = XirrInput;
    type Output = XirrOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        xirr(input)
    }
}

/// `tvm.mirr`: see the methodology page.
pub struct Mirr;

impl Method for Mirr {
    const ID: &'static str = "tvm.mirr";
    const TITLE: &'static str = "Modified internal rate of return (MIRR)";
    const DESCRIPTION: &'static str =
        "Rate of return with negative flows financed at a finance rate and positive flows \
        reinvested at a reinvestment rate (ECMA-376 MIRR).";
    const VERSION: &'static str = "1";
    type Input = MirrInput;
    type Output = MirrOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        mirr(input)
    }
}

/// `tvm.annuity`: see the methodology page.
pub struct Annuity;

impl Method for Annuity {
    const ID: &'static str = "tvm.annuity";
    const TITLE: &'static str = "Annuity (level or growing, ordinary or due)";
    const DESCRIPTION: &'static str =
        "Present and future value of a finite series of level or growing payments.";
    const VERSION: &'static str = "1";
    type Input = AnnuityInput;
    type Output = AnnuityOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        annuity(input)
    }
}

/// `tvm.future_value`: see the methodology page.
pub struct FutureValue;

impl Method for FutureValue {
    const ID: &'static str = "tvm.future_value";
    const TITLE: &'static str = "Future value of a present amount and level payments";
    const DESCRIPTION: &'static str =
        "Value at the end of n periods of a present amount and a level series of payments, \
        ordinary or due, at a rate per period.";
    const VERSION: &'static str = "1";
    type Input = FutureValueInput;
    type Output = FutureValueOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        future_value(input)
    }
}

/// `tvm.perpetuity`: see the methodology page.
pub struct Perpetuity;

impl Method for Perpetuity {
    const ID: &'static str = "tvm.perpetuity";
    const TITLE: &'static str = "Perpetuity (level or growing)";
    const DESCRIPTION: &'static str =
        "Present value of a level or growing perpetuity (Gordon), ordinary or due.";
    const VERSION: &'static str = "1";
    type Input = PerpetuityInput;
    type Output = PerpetuityOutput;

    fn compute(input: &Self::Input) -> Result<(Self::Output, Explain)> {
        perpetuity(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::prelude::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn npv_at_zero_rate_is_sum_of_flows() {
        let input = NpvInput {
            rate: Decimal::ZERO,
            cash_flows: vec![d("-100"), d("30"), d("40"), d("50.5")],
        };
        assert_eq!(npv(&input).unwrap().0.npv, d("20.5"));
    }

    #[test]
    fn integer_discount_factor_is_exact() {
        assert_eq!(discount_factor(d("0.25"), d("2")).unwrap(), d("0.64"));
    }

    #[test]
    fn npv_at_irr_is_zero() {
        let cfs = vec![d("-1000"), d("300"), d("400"), d("500")];
        let (out, _) = irr(&IrrInput {
            cash_flows: cfs.clone(),
            guess: None,
        })
        .unwrap();
        assert!(npv_raw(out.irr, &cfs).unwrap().abs() < d("0.000000001"));
    }

    #[test]
    fn irr_requires_sign_change() {
        let r = irr(&IrrInput {
            cash_flows: vec![d("1"), d("2")],
            guess: None,
        });
        assert_eq!(r.unwrap_err(), FairmarkError::NoSignChange);
    }

    #[test]
    fn rate_must_exceed_minus_one() {
        assert!(matches!(
            discount_factor(d("-1"), d("1")),
            Err(FairmarkError::RateOutOfRange(_))
        ));
    }
    fn dated(pairs: &[(&str, &str)]) -> Vec<DatedCashFlow> {
        pairs
            .iter()
            .map(|(dt, a)| DatedCashFlow {
                date: dt.parse().unwrap(),
                amount: d(a),
            })
            .collect()
    }

    #[test]
    fn present_value_with_flat_rate_equals_npv_shifted() {
        let flows = vec![d("120"), d("-35"), d("410.5"), d("0"), d("77")];
        let (pv, _) = present_value(&PresentValueInput {
            cash_flows: flows.clone(),
            rate: Some(d("0.083")),
            period_rates: None,
            timing: PeriodTiming::EndOfPeriod,
        })
        .unwrap();
        let mut shifted = vec![Decimal::ZERO];
        shifted.extend(flows);
        let (n, _) = npv(&NpvInput {
            rate: d("0.083"),
            cash_flows: shifted,
        })
        .unwrap();
        assert!((pv.present_value - n.npv).abs() < d("1e-20"));
    }

    #[test]
    fn present_value_needs_exactly_one_rate_source() {
        let base = PresentValueInput {
            cash_flows: vec![d("1")],
            rate: None,
            period_rates: None,
            timing: PeriodTiming::EndOfPeriod,
        };
        assert!(present_value(&base).is_err());
        let both = PresentValueInput {
            rate: Some(d("0.1")),
            period_rates: Some(vec![d("0.1")]),
            ..base.clone()
        };
        assert!(present_value(&both).is_err());
        let short = PresentValueInput {
            cash_flows: vec![d("1"), d("2")],
            period_rates: Some(vec![d("0.1")]),
            ..base
        };
        assert!(present_value(&short).is_err());
    }

    #[test]
    fn xirr_on_whole_365_day_years_equals_irr() {
        let flows = dated(&[
            ("2021-01-01", "-1000"),
            ("2022-01-01", "300"),
            ("2023-01-01", "400"),
            ("2024-01-01", "500"),
        ]);
        // 2023 to 2024 is 365 days; 2024 is a leap year but the flows end on 1 January.
        let (x, _) = xirr(&XirrInput {
            cash_flows: flows,
            day_count: Some(DayCount::Act365Fixed),
            compounding: Some(Compounding::Annual),
            guess: None,
        })
        .unwrap();
        let (i, _) = irr(&IrrInput {
            cash_flows: vec![d("-1000"), d("300"), d("400"), d("500")],
            guess: None,
        })
        .unwrap();
        assert!(
            (x.xirr - i.irr).abs() < d("1e-17"),
            "{} vs {}",
            x.xirr,
            i.irr
        );
    }

    #[test]
    fn mirr_equals_irr_when_both_rates_equal_the_irr() {
        let cfs = vec![d("-1000"), d("300"), d("400"), d("500")];
        let (i, _) = irr(&IrrInput {
            cash_flows: cfs.clone(),
            guess: None,
        })
        .unwrap();
        let (m, _) = mirr(&MirrInput {
            cash_flows: cfs,
            finance_rate: i.irr,
            reinvest_rate: i.irr,
        })
        .unwrap();
        assert!((m.mirr - i.irr).abs() < d("1e-15"));
    }

    #[test]
    fn several_irrs_are_refused_with_their_intervals() {
        let err = irr(&IrrInput {
            cash_flows: vec![d("-100"), d("230"), d("-132")],
            guess: None,
        })
        .unwrap_err();
        assert_eq!(err.code(), "FM-E011");
        assert!(err.to_string().contains("0.10") && err.to_string().contains("0.20"));
    }

    #[test]
    fn long_series_do_not_fail_on_grid_overflow() {
        let mut cfs = vec![d("-1000")];
        cfs.extend(std::iter::repeat_n(d("80"), 60));
        let (out, _) = irr(&IrrInput {
            cash_flows: cfs.clone(),
            guess: None,
        })
        .unwrap();
        assert!(npv_raw(out.irr, &cfs).unwrap().abs() < d("1e-9"));
    }

    /// SPEC-0009 R10: the payment term of the future value equals the future value of
    /// `tvm.annuity`, and the present amount grows by (1 + r)^n.
    #[test]
    fn future_value_agrees_with_the_annuity() {
        for timing in [AnnuityTiming::Ordinary, AnnuityTiming::Due] {
            let (fv, _) = future_value(&FutureValueInput {
                present_value: Some(d("-300")),
                payment: Some(d("125")),
                rate: d("0.0125"),
                periods: 40,
                timing,
            })
            .unwrap();
            let (a, _) = annuity(&AnnuityInput {
                payment: d("125"),
                rate: d("0.0125"),
                periods: 40,
                growth: None,
                timing,
            })
            .unwrap();
            let expected = a.future_value - d("300") * d("1.0125").powi(40);
            assert!(
                (fv.future_value - expected).abs() < d("1e-20"),
                "{timing:?}"
            );
        }
    }

    /// SPEC-0009 R4, R11: rate zero is a separate step; no amount at all is refused.
    #[test]
    fn future_value_limit_and_missing_amounts() {
        let (fv, ex) = future_value(&FutureValueInput {
            present_value: None,
            payment: Some(d("10")),
            rate: Decimal::ZERO,
            periods: 7,
            timing: AnnuityTiming::Due,
        })
        .unwrap();
        assert_eq!(fv.future_value, d("70"));
        assert_eq!(ex.steps.len(), 1);
        assert!(ex.steps[0].label.contains("rate zero"));
        let err = future_value(&FutureValueInput {
            present_value: None,
            payment: None,
            rate: d("0.01"),
            periods: 7,
            timing: AnnuityTiming::Due,
        })
        .unwrap_err();
        assert_eq!(err.code(), "FM-E001");
    }

    #[test]
    fn annuity_matches_explicit_sum() {
        let (a, _) = annuity(&AnnuityInput {
            payment: d("250"),
            rate: d("0.045"),
            periods: 12,
            growth: Some(d("0.02")),
            timing: AnnuityTiming::Ordinary,
        })
        .unwrap();
        let mut total = Decimal::ZERO;
        for k in 1..=12i64 {
            total += d("250") * d("1.02").powi(k - 1) / d("1.045").powi(k);
        }
        assert!((a.present_value - total).abs() < d("1e-18"));
    }

    #[test]
    fn two_roots_inside_one_grid_interval_are_refused() {
        // Roots at 11% and 12%: both inside the grid interval [0.10, 0.15], NPV keeps its sign
        // at every grid point, so only the sign-change count reveals the ambiguity.
        let cfs = vec![d("-100"), d("223"), d("-124.32")];
        assert_eq!(sign_changes(cfs.iter()), 2);
        let err = irr(&IrrInput {
            cash_flows: cfs,
            guess: None,
        })
        .unwrap_err();
        assert_eq!(err.code(), "FM-E011");
    }

    #[test]
    fn negative_rates_keep_relative_precision() {
        // Audit D1: 1 / 1.01^-n computed via the reciprocal keeps all digits.
        let df = discount_factor(d("-0.99"), d("13")).unwrap();
        assert_eq!(df, d("100").powi(13));
    }

    #[test]
    fn very_long_series_still_find_the_rate() {
        // Audit D2: 700 periods overflowed every grid point above 0.10.
        let mut cfs = vec![d("-1000")];
        cfs.extend(std::iter::repeat_n(d("80"), 700));
        let (out, _) = irr(&IrrInput {
            cash_flows: cfs,
            guess: None,
        })
        .unwrap();
        assert!((out.irr - d("0.08")).abs() < d("1e-15"), "{}", out.irr);
        assert!(out.uniqueness_verified);
    }

    #[test]
    fn odd_parity_reveals_a_second_rate_outside_the_grid() {
        // Audit D3: roots 0.10 and 20; the grid sees only 0.10.
        let err = irr(&IrrInput {
            cash_flows: vec![d("-1"), d("22.1"), d("-23.1")],
            guess: None,
        })
        .unwrap_err();
        assert_eq!(err.code(), "FM-E011");
    }

    #[test]
    fn even_parity_returns_the_rate_flagged_as_not_unique() {
        // Roots 0.11, 0.12 and 0.50: the grid isolates only 0.50.
        let (out, ex) = irr(&IrrInput {
            cash_flows: vec![d("-1"), d("3.73"), d("-4.5882"), d("1.8648")],
            guess: None,
        })
        .unwrap();
        assert!(!out.uniqueness_verified);
        assert!(ex.conventions.iter().any(|c| c.contains("not verified")));
    }

    #[test]
    fn annuity_is_stable_when_rate_and_growth_almost_coincide() {
        // Audit D5.
        let (a, _) = annuity(&AnnuityInput {
            payment: d("1000"),
            rate: d("0.05"),
            periods: 50,
            growth: Some(d("0.0500000000000000000000000001")),
            timing: AnnuityTiming::Ordinary,
        })
        .unwrap();
        let exact = d("50000") / d("1.05");
        assert!(
            (a.present_value - exact).abs() < d("1e-15"),
            "{}",
            a.present_value
        );
    }

    #[test]
    fn perpetuity_rejects_growth_at_or_below_minus_one() {
        // Audit D6.
        let r = perpetuity(&PerpetuityInput {
            payment: d("100"),
            rate: d("0.05"),
            growth: Some(d("-3")),
            timing: AnnuityTiming::Ordinary,
        });
        assert_eq!(r.unwrap_err().code(), "FM-E001");
    }

    #[test]
    fn mirr_validates_rates_before_computing() {
        // Audit D7.
        let r = mirr(&MirrInput {
            cash_flows: vec![d("-1"), d("2")],
            finance_rate: d("-1"),
            reinvest_rate: d("0.1"),
        });
        assert_eq!(r.unwrap_err().code(), "FM-E003");
    }

    #[test]
    fn xirr_grid_does_not_depend_on_the_quotation() {
        // Audit D4: same flows, verdict identical for every compounding.
        let flows = dated(&[
            ("2021-01-01", "-1"),
            ("2022-01-01", "2.23"),
            ("2023-01-01", "-1.2432"),
        ]);
        for c in [
            Compounding::Annual,
            Compounding::Monthly,
            Compounding::Continuous,
        ] {
            let r = xirr(&XirrInput {
                cash_flows: flows.clone(),
                day_count: Some(DayCount::Act365Fixed),
                compounding: Some(c),
                guess: None,
            });
            assert_eq!(r.unwrap_err().code(), "FM-E011", "{c:?}");
        }
    }

    // Issue #176: error messages name the field and the fix; codes are unchanged.

    fn message(r: Result<impl std::fmt::Debug>) -> (String, &'static str) {
        let e = r.unwrap_err();
        (e.to_string(), e.code())
    }

    #[test]
    fn rate_errors_name_the_field() {
        let (m, code) = message(npv(&NpvInput {
            rate: d("-1"),
            cash_flows: vec![d("1")],
        }));
        assert_eq!(code, "FM-E003");
        assert!(m.starts_with("`/rate` is -1"), "{m}");
        let (m, code) = message(present_value(&PresentValueInput {
            cash_flows: vec![d("1"), d("2"), d("3")],
            rate: None,
            period_rates: Some(vec![d("0.1"), d("0.1"), d("-1.5")]),
            timing: PeriodTiming::EndOfPeriod,
        }));
        assert_eq!(code, "FM-E003");
        assert!(m.starts_with("`/period_rates/2` is -1.5"), "{m}");
        for (fin, re, field) in [
            ("-1", "0.1", "`/finance_rate`"),
            ("0.1", "-2", "`/reinvest_rate`"),
        ] {
            let (m, code) = message(mirr(&MirrInput {
                cash_flows: vec![d("-1"), d("2")],
                finance_rate: d(fin),
                reinvest_rate: d(re),
            }));
            assert_eq!(code, "FM-E003");
            assert!(m.starts_with(field) && m.contains("10% is 0.10"), "{m}");
        }
        let (m, code) = message(annuity(&AnnuityInput {
            payment: d("1"),
            rate: d("-1"),
            periods: 3,
            growth: None,
            timing: AnnuityTiming::Ordinary,
        }));
        assert_eq!(code, "FM-E003");
        assert!(m.starts_with("`/rate`"), "{m}");
        let (m, code) = message(perpetuity(&PerpetuityInput {
            payment: d("1"),
            rate: d("-1"),
            growth: None,
            timing: AnnuityTiming::Ordinary,
        }));
        assert_eq!(code, "FM-E003");
        assert!(m.starts_with("`/rate`"), "{m}");
    }

    #[test]
    fn annuity_keeps_the_periods_error_first() {
        // The field is named without changing which error comes first.
        let r = annuity(&AnnuityInput {
            payment: d("1"),
            rate: d("-1"),
            periods: 0,
            growth: None,
            timing: AnnuityTiming::Ordinary,
        });
        assert_eq!(r.unwrap_err().code(), "FM-E001");
    }

    #[test]
    fn perpetuity_growth_error_names_growth_and_rate() {
        let (m, code) = message(perpetuity(&PerpetuityInput {
            payment: d("100"),
            rate: d("0.05"),
            growth: Some(d("0.06")),
            timing: AnnuityTiming::Ordinary,
        }));
        assert_eq!(code, "FM-E004");
        assert!(m.starts_with("`/growth` (0.06)"), "{m}");
        assert!(
            m.contains("`/rate` (0.05)") && !m.contains("terminal"),
            "{m}"
        );
    }

    #[test]
    fn solver_errors_say_what_to_do() {
        let e = FairmarkError::NoConvergence {
            solver: "XIRR",
            iterations: IRR_MAX_ITERATIONS,
        };
        assert_eq!(e.code(), "FM-E005");
        let m = e.to_string();
        assert!(m.starts_with("the XIRR solver"), "{m}");
        assert!(m.contains("give a `guess` near the expected rate"), "{m}");
        let (m, code) = message(irr(&IrrInput {
            cash_flows: vec![d("1"), d("2")],
            guess: None,
        }));
        assert_eq!(code, "FM-E006");
        assert_eq!(
            m,
            "`cash_flows` needs at least one positive and one negative amount"
        );
        let (m, code) = message(irr(&IrrInput {
            cash_flows: vec![d("-100"), d("230"), d("-132")],
            guess: None,
        }));
        assert_eq!(code, "FM-E011");
        assert!(m.contains("repeated root"), "{m}");
        assert!(
            m.ends_with("use `tvm.npv` at the cost of capital or `tvm.mirr` with explicit rates"),
            "{m}"
        );
    }

    // Issue #176: explain detail.

    #[test]
    fn mirr_explain_lists_every_flow() {
        let (_, ex) = mirr(&MirrInput {
            cash_flows: vec![d("-100"), d("0"), d("-50"), d("80"), d("120")],
            finance_rate: d("0.10"),
            reinvest_rate: d("0.12"),
        })
        .unwrap();
        let labels: Vec<&str> = ex.steps.iter().map(|s| s.label.as_str()).collect();
        assert_eq!(
            labels[..4],
            [
                "PV at t = 0 of the flow at t = 0 (finance rate)",
                "PV at t = 0 of the flow at t = 2 (finance rate)",
                "FV at t = 4 of the flow at t = 3 (reinvestment rate)",
                "FV at t = 4 of the flow at t = 4 (reinvestment rate)",
            ]
        );
        assert_eq!(ex.steps[1].formula, "-50 / (1 + 0.10)^2");
        let pv = &ex.steps[4];
        assert_eq!(pv.value, ex.steps[0].value + ex.steps[1].value);
        assert!(pv.formula.ends_with(&format!(
            "= ({}) + ({})",
            ex.steps[0].value, ex.steps[1].value
        )));
        let fv = &ex.steps[5];
        assert_eq!(fv.value, ex.steps[2].value + ex.steps[3].value);
    }

    #[test]
    fn irr_explain_shows_where_the_grid_located_the_root() {
        let (out, ex) = irr(&IrrInput {
            cash_flows: vec![d("-1000"), d("300"), d("400"), d("500")],
            guess: None,
        })
        .unwrap();
        let lo = ex
            .steps
            .iter()
            .find(|s| s.label == "Root interval on the grid, lower rate")
            .unwrap();
        let hi = ex
            .steps
            .iter()
            .find(|s| s.label == "Root interval on the grid, upper rate")
            .unwrap();
        assert_eq!((lo.value, hi.value), (d("0.05"), d("0.10")));
        assert!(lo.value < out.irr && out.irr < hi.value);
        assert!(lo.formula.starts_with("NPV(0.05) = "), "{}", lo.formula);
        assert!(ex
            .conventions
            .iter()
            .any(|c| c.contains("grid of rates per period")));
        let (out, ex) = irr(&IrrInput {
            cash_flows: vec![d("-1"), d("2")],
            guess: None,
        })
        .unwrap();
        assert_eq!(out.solver, "grid (exact root)");
        assert!(
            ex.steps
                .iter()
                .any(|s| s.label == "Root located on the grid (exact grid point)"
                    && s.value == d("1"))
        );
    }

    #[test]
    fn annuity_reports_the_future_value_date() {
        for timing in [AnnuityTiming::Ordinary, AnnuityTiming::Due] {
            let (_, ex) = annuity(&AnnuityInput {
                payment: d("100"),
                rate: d("0.05"),
                periods: 10,
                growth: None,
                timing,
            })
            .unwrap();
            assert!(ex
                .conventions
                .iter()
                .any(|c| c == "future value at t = n for both timings"));
        }
    }
}
