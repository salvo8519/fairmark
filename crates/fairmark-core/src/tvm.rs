//! Time value of money in exact decimal arithmetic.
//!
//! Conventions:
//! * rates are decimal fractions per period (`0.10` = 10%);
//! * `cash_flows[0]` happens at t = 0, `cash_flows[i]` at t = i.

use rust_decimal::prelude::ToPrimitive;
use rust_decimal::{Decimal, MathematicalOps};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::error::{FairmarkError, Result};
use crate::explain::Explain;

/// Absolute tolerance on the rate for the IRR solver.
pub const IRR_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 18); // 1e-18
/// Maximum iterations for each solver stage.
pub const IRR_MAX_ITERATIONS: u32 = 200;

/// `1 / (1 + rate)^t`. Integer `t` uses exact repeated multiplication;
/// fractional `t` (mid-period) uses decimal `powd`.
pub fn discount_factor(rate: Decimal, t: Decimal) -> Result<Decimal> {
    if rate <= -Decimal::ONE {
        return Err(FairmarkError::RateOutOfRange(rate));
    }
    let base = Decimal::ONE + rate;
    let growth = if t.fract().is_zero() {
        let exp = t
            .to_i64()
            .ok_or(FairmarkError::Overflow("discount factor exponent"))?;
        base.checked_powi(exp)
    } else {
        base.checked_powd(t)
    }
    .ok_or(FairmarkError::Overflow("discount factor"))?;
    Decimal::ONE
        .checked_div(growth)
        .ok_or(FairmarkError::Overflow("discount factor"))
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
    pub npv: Decimal,
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
        let df = discount_factor(input.rate, Decimal::from(i as u64))?;
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
    /// Starting point for Newton's method (default 0.10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub guess: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, JsonSchema)]
pub struct IrrOutput {
    /// Internal rate of return per period as a decimal fraction.
    pub irr: Decimal,
    /// NPV evaluated at the returned IRR (should be ~0).
    pub npv_at_irr: Decimal,
    /// "newton" or "bisection".
    pub solver: String,
    pub iterations: u32,
    pub tolerance: Decimal,
}

pub fn irr(input: &IrrInput) -> Result<(IrrOutput, Explain)> {
    let cfs = &input.cash_flows;
    if cfs.is_empty() {
        return Err(FairmarkError::EmptyCashFlows);
    }
    let has_pos = cfs.iter().any(|c| c.is_sign_positive() && !c.is_zero());
    let has_neg = cfs.iter().any(|c| c.is_sign_negative() && !c.is_zero());
    if !(has_pos && has_neg) {
        return Err(FairmarkError::NoSignChange);
    }

    let mut ex = Explain::new("tvm.irr", "find r such that Σ CF_t / (1 + r)^t = 0");
    ex.convention("Newton's method from the guess; bisection on [-0.99, 10] as fallback");
    ex.convention(format!("tolerance on the rate: {IRR_TOLERANCE}"));

    let (irr, solver, iterations) = match newton(cfs, input.guess.unwrap_or(Decimal::new(10, 2))) {
        Some((r, it)) => (r, "newton", it),
        None => {
            let (r, it) = bisection(cfs)?;
            (r, "bisection", it)
        }
    };
    let npv_at = npv_raw(irr, cfs)?;
    ex.step(
        "Internal rate of return",
        format!("{solver}, {iterations} iterations"),
        irr,
    );
    ex.step("NPV at IRR (check)", format!("NPV({irr})"), npv_at);

    Ok((
        IrrOutput {
            irr,
            npv_at_irr: npv_at,
            solver: solver.to_owned(),
            iterations,
            tolerance: IRR_TOLERANCE,
        },
        ex,
    ))
}

fn newton(cfs: &[Decimal], guess: Decimal) -> Option<(Decimal, u32)> {
    let mut r = guess;
    for it in 1..=IRR_MAX_ITERATIONS {
        let f = npv_raw(r, cfs).ok()?;
        let d = npv_derivative(r, cfs).ok()?;
        if d.is_zero() {
            return None;
        }
        let next = r.checked_sub(f.checked_div(d)?)?;
        if next <= -Decimal::ONE {
            return None;
        }
        if (next - r).abs() <= IRR_TOLERANCE {
            return Some((next, it));
        }
        r = next;
    }
    None
}

fn bisection(cfs: &[Decimal]) -> Result<(Decimal, u32)> {
    let mut lo = Decimal::new(-99, 2);
    let mut hi = Decimal::from(10);
    let mut f_lo = npv_raw(lo, cfs)?;
    let f_hi = npv_raw(hi, cfs)?;
    if f_lo.is_sign_negative() == f_hi.is_sign_negative() {
        return Err(FairmarkError::NoConvergence { iterations: 0 });
    }
    for it in 1..=IRR_MAX_ITERATIONS {
        let mid = (lo + hi) / Decimal::TWO;
        let f_mid = npv_raw(mid, cfs)?;
        if (hi - lo) <= IRR_TOLERANCE || f_mid.is_zero() {
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
        iterations: IRR_MAX_ITERATIONS,
    })
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
}
