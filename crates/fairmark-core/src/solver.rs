//! Brent's root-finding method on decimals (SPEC-0008 R10 to R12).
//!
//! Generic and independent of methods: it finds `x` in an explicit bracket such that
//! `|f(x)| <= tolerance`. It combines inverse quadratic interpolation, the secant step and
//! bisection in the usual formulation (Brent 1973, as in `scipy.optimize.brentq`). Every
//! interpolation uses checked arithmetic; any arithmetic failure inside an interpolation falls
//! back to bisection, so the path is deterministic and never fails for arithmetic reasons
//! inside the bracket. Every new point lies strictly inside the current bracket, which always
//! lies inside the caller's; when the bracket shrinks below the minimum step without meeting
//! the tolerance (a jump across the target, or a target below decimal resolution) the search
//! stops with `FM-E014` and reports the collapsed interval, as in the textbook method.

use rust_decimal::Decimal;
use schemars::JsonSchema;
use serde::Serialize;

use crate::error::{FairmarkError, Result};
use crate::num::{add, div, mul, sub};

/// How a point was obtained.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum StepKind {
    /// One end of the bracket given by the caller.
    Bracket,
    /// Midpoint of the current bracket.
    Bisection,
    /// Secant through the two latest points.
    Secant,
    /// Inverse quadratic interpolation through three points.
    InverseQuadratic,
}

impl StepKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Bracket => "bracket end",
            Self::Bisection => "bisection",
            Self::Secant => "secant",
            Self::InverseQuadratic => "inverse quadratic interpolation",
        }
    }
}

/// One evaluation of `f`.
#[derive(Debug, Clone, PartialEq)]
pub struct Point {
    pub x: Decimal,
    pub fx: Decimal,
    pub kind: StepKind,
}

/// Result of a successful search.
#[derive(Debug, Clone, PartialEq)]
pub struct Root {
    pub x: Decimal,
    pub fx: Decimal,
    /// Evaluations after the two bracket ends.
    pub iterations: u32,
    /// Every evaluation in order, bracket ends first.
    pub trace: Vec<Point>,
}

/// `b − a`, saturated at the largest decimal: used only to compare step sizes, so a bracket
/// wider than the decimal range does not overflow.
fn span(a: Decimal, b: Decimal) -> Decimal {
    sub(b, a).unwrap_or(if b > a { Decimal::MAX } else { Decimal::MIN })
}

/// `(c − b) / 2` without overflow.
fn half_span(b: Decimal, c: Decimal) -> Decimal {
    sub(c, b)
        .map(|v| v / Decimal::TWO)
        .or_else(|_| sub(c / Decimal::TWO, b / Decimal::TWO))
        .unwrap_or(if c > b {
            Decimal::MAX / Decimal::TWO
        } else {
            Decimal::MIN / Decimal::TWO
        })
}

/// Smallest step on the variable: 10^-24 × max(1, |x|), above decimal resolution.
pub fn min_step(x: Decimal) -> Decimal {
    let scale = Decimal::new(1, 24);
    let magnitude = x.abs().max(Decimal::ONE);
    magnitude.checked_mul(scale).unwrap_or(scale)
}

fn same_sign(a: Decimal, b: Decimal) -> bool {
    (a.is_sign_negative() && !a.is_zero()) == (b.is_sign_negative() && !b.is_zero())
        && !a.is_zero()
        && !b.is_zero()
}

/// Interpolated step `p / q` as in Brent's method, or `None` when bisection must be used.
#[allow(clippy::too_many_arguments)]
fn interpolate(
    a: Decimal,
    b: Decimal,
    c: Decimal,
    fa: Decimal,
    fb: Decimal,
    fc: Decimal,
    xm: Decimal,
    e: Decimal,
    tol1: Decimal,
) -> Option<(Decimal, StepKind)> {
    let two = Decimal::TWO;
    let three = Decimal::from(3);
    let step = || -> Result<(Decimal, Decimal, StepKind)> {
        let s = div(fb, fa)?;
        let (p, q, kind) = if a == c {
            let p = mul(mul(two, xm)?, s)?;
            let q = sub(Decimal::ONE, s)?;
            (p, q, StepKind::Secant)
        } else {
            let q = div(fa, fc)?;
            let r = div(fb, fc)?;
            let left = mul(mul(mul(two, xm)?, q)?, sub(q, r)?)?;
            let right = mul(sub(b, a)?, sub(r, Decimal::ONE)?)?;
            let p = mul(s, sub(left, right)?)?;
            let q = mul(
                mul(sub(q, Decimal::ONE)?, sub(r, Decimal::ONE)?)?,
                sub(s, Decimal::ONE)?,
            )?;
            (p, q, StepKind::InverseQuadratic)
        };
        Ok((p, q, kind))
    };
    let (mut p, mut q, kind) = step().ok()?;
    if p > Decimal::ZERO {
        q = -q;
    }
    p = p.abs();
    let accept = || -> Result<bool> {
        let min1 = sub(mul(mul(three, xm)?, q)?, mul(tol1, q)?.abs())?;
        let min2 = mul(e, q)?.abs();
        Ok(mul(two, p)? < min1.min(min2))
    };
    if accept().ok()? && !q.is_zero() {
        div(p, q).ok().map(|d| (d, kind))
    } else {
        None
    }
}

/// Finds `x` in `[low, high]` with `|f(x)| <= tolerance`.
///
/// Errors: `FM-E013` when `f(low)` and `f(high)` have the same sign and neither meets the
/// tolerance; `FM-E014` when `max_iterations` new evaluations do not meet it; any error of `f`
/// unchanged.
pub fn brent<F>(
    mut f: F,
    low: Decimal,
    high: Decimal,
    tolerance: Decimal,
    max_iterations: u32,
) -> Result<Root>
where
    F: FnMut(Decimal) -> Result<Decimal>,
{
    let mut trace = Vec::new();
    let (mut a, mut b) = (low, high);
    let mut fa = f(a)?;
    trace.push(Point {
        x: a,
        fx: fa,
        kind: StepKind::Bracket,
    });
    if fa.abs() <= tolerance {
        return Ok(Root {
            x: a,
            fx: fa,
            iterations: 0,
            trace,
        });
    }
    let mut fb = f(b)?;
    trace.push(Point {
        x: b,
        fx: fb,
        kind: StepKind::Bracket,
    });
    if fb.abs() <= tolerance {
        return Ok(Root {
            x: b,
            fx: fb,
            iterations: 0,
            trace,
        });
    }
    if same_sign(fa, fb) {
        return Err(FairmarkError::TargetNotBracketed(format!(
            "output minus target is {fa} at {a} and {fb} at {b}, with the same sign"
        )));
    }

    let (mut c, mut fc) = (a, fa);
    let mut d = span(a, b);
    let mut e = d;
    for iteration in 1..=max_iterations {
        if same_sign(fb, fc) {
            c = a;
            fc = fa;
            d = span(a, b);
            e = d;
        }
        if fc.abs() < fb.abs() {
            a = b;
            b = c;
            c = a;
            fa = fb;
            fb = fc;
            fc = fa;
        }
        let tol1 = min_step(b);
        // (c − b) / 2; at brackets wider than the decimal range the difference overflows and
        // the halves are used, saturated at half the largest decimal.
        let xm = half_span(b, c);
        if xm.abs() <= tol1 {
            let (lo, hi) = (b.min(c), b.max(c));
            return Err(FairmarkError::GoalSeekNoConvergence {
                iterations: iteration - 1,
                detail: format!(
                    "the bracket collapsed to [{lo}, {hi}] (output minus target {} and {}) \
                    without meeting the tolerance; the output jumps across the target there, or \
                    the tolerance is below what the output can resolve",
                    if lo == b { fb } else { fc },
                    if hi == b { fb } else { fc },
                ),
            });
        }

        let mut kind = StepKind::Bisection;
        if e.abs() >= tol1 && fa.abs() > fb.abs() {
            match interpolate(a, b, c, fa, fb, fc, xm, e, tol1) {
                Some((step, k)) => {
                    e = d;
                    d = step;
                    kind = k;
                }
                None => {
                    d = xm;
                    e = d;
                }
            }
        } else {
            d = xm;
            e = d;
        }
        a = b;
        fa = fb;
        let delta = if d.abs() > tol1 {
            d
        } else if xm.is_sign_negative() {
            -tol1
        } else {
            tol1
        };
        // An overflowing step falls back to bisection, like a failed interpolation.
        let mut next = add(b, delta).unwrap_or(b);
        // Safety net: a new point must lie strictly between b and c. Brent's acceptance test
        // guarantees it for interpolated steps; rounding could break it at decimal resolution.
        if next <= b.min(c) || next >= b.max(c) {
            next = add(b, xm)?;
            kind = StepKind::Bisection;
        }
        b = next;
        fb = f(b)?;
        trace.push(Point { x: b, fx: fb, kind });
        if fb.abs() <= tolerance {
            return Ok(Root {
                x: b,
                fx: fb,
                iterations: iteration,
                trace,
            });
        }
    }
    // The evaluation closest to the target; ties keep the earliest, so the report is stable.
    let best = trace
        .iter()
        .min_by_key(|p| p.fx.abs())
        .map(|p| (p.x, p.fx))
        .unwrap_or((b, fb));
    // The point bracketing the target with b: c is refreshed only at the start of an
    // iteration, so after the last evaluation it may share the sign of f(b).
    let (other, f_other) = if same_sign(fb, fc) { (a, fa) } else { (c, fc) };
    let (lo, hi) = (b.min(other), b.max(other));
    let (f_lo, f_hi) = if lo == b {
        (fb, f_other)
    } else {
        (f_other, fb)
    };
    Err(FairmarkError::GoalSeekNoConvergence {
        iterations: max_iterations,
        detail: format!(
            "the closest evaluation was at {} with output minus target {}; the target lies in \
            [{lo}, {hi}] (output minus target {f_lo} and {f_hi})",
            best.0, best.1
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    fn d(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn linear_root_is_found_in_one_secant_step() {
        let r = brent(
            |x| sub(mul(d("2"), x)?, d("1")),
            d("0"),
            d("1"),
            d("1e-20"),
            50,
        )
        .unwrap();
        assert_eq!(r.x, d("0.5"));
        assert!(r.iterations <= 2);
    }

    #[test]
    fn square_root_of_two_to_high_precision() {
        let r = brent(|x| sub(mul(x, x)?, d("2")), d("1"), d("2"), d("1e-25"), 100).unwrap();
        let sqrt2 = d("1.4142135623730950488016887242");
        assert!((r.x - sqrt2).abs() < d("1e-24"), "{}", r.x);
        assert!(r.fx.abs() <= d("1e-25"));
        assert_eq!(r.trace.len() as u32, r.iterations + 2);
        assert!(r.trace[2..].iter().all(|p| p.kind != StepKind::Bracket));
    }

    #[test]
    fn decreasing_function_and_reversed_signs() {
        let r = brent(|x| sub(d("3"), x), d("0"), d("10"), d("1e-20"), 50).unwrap();
        assert!((r.x - d("3")).abs() <= d("1e-20"), "{}", r.x);
    }

    #[test]
    fn bracket_end_meeting_the_target_needs_no_iteration() {
        let r = brent(|x| sub(x, d("1")), d("1"), d("5"), d("1e-9"), 10).unwrap();
        assert_eq!((r.x, r.iterations, r.trace.len()), (d("1"), 0, 1));
        let r = brent(|x| sub(x, d("5")), d("1"), d("5"), d("1e-9"), 10).unwrap();
        assert_eq!((r.x, r.iterations, r.trace.len()), (d("5"), 0, 2));
    }

    #[test]
    fn target_outside_the_bracket_is_e013() {
        let e = brent(|x| sub(mul(x, x)?, d("2")), d("2"), d("3"), d("1e-9"), 10).unwrap_err();
        assert_eq!(e.code(), "FM-E013");
    }

    #[test]
    fn a_jump_across_the_target_never_yields_a_false_root() {
        // Step function: -1 below 0.3, +1 from 0.3 on. No x meets |f| <= 0.5.
        let step = |x: Decimal| Ok(if x < d("0.3") { d("-1") } else { d("1") });
        let e = brent(step, d("0"), d("1"), d("0.5"), 200).unwrap_err();
        assert_eq!(e.code(), "FM-E014");
        assert!(
            e.to_string().contains("0.3"),
            "the message names the jump: {e}"
        );
    }

    /// The interval reported when the cap is reached always brackets the target.
    #[test]
    fn the_interval_reported_at_the_cap_brackets_the_target() {
        let f = |x: Decimal| sub(mul(mul(x, x)?, x)?, d("7"));
        for cap in 1..8 {
            let e = brent(f, d("0"), d("3"), d("1e-27"), cap).unwrap_err();
            let msg = e.to_string();
            let tail = msg.split("(output minus target ").last().unwrap();
            let values: Vec<Decimal> = tail
                .trim_end_matches(')')
                .split(" and ")
                .map(|v| Decimal::from_str(v).unwrap())
                .collect();
            assert!(
                values[0].is_sign_negative() != values[1].is_sign_negative(),
                "{msg}"
            );
        }
    }

    #[test]
    fn the_iteration_cap_is_respected() {
        let e = brent(|x| sub(mul(x, x)?, d("2")), d("1"), d("2"), d("1e-27"), 1).unwrap_err();
        assert_eq!(e.code(), "FM-E014");
    }

    #[test]
    fn errors_of_the_function_are_returned_unchanged() {
        let e = brent(
            |x| {
                if x > d("0.5") {
                    Err(FairmarkError::EmptyCashFlows)
                } else {
                    Ok(x)
                }
            },
            d("-1"),
            d("1"),
            d("1e-9"),
            10,
        )
        .unwrap_err();
        assert_eq!(e.code(), "FM-E002");
    }

    #[test]
    fn brackets_wider_than_the_decimal_range_work() {
        let max = Decimal::MAX;
        let r = brent(Ok, -max, max, d("1e-9"), 1000).unwrap();
        assert!(r.x.abs() <= d("1e-9"), "{}", r.x);
    }

    /// Every evaluated point lies in the caller's bracket, and a jump across the target ends in
    /// FM-E014 naming the collapsed interval, never in a point outside or a false root.
    #[test]
    fn evaluations_never_leave_the_bracket() {
        let cases: [(Decimal, Decimal, Decimal); 3] = [
            (d("0"), d("1"), d("1")),
            (d("0"), d("1"), d("0.3")),
            (d("-5"), d("7"), d("7")),
        ];
        for (low, high, jump) in cases {
            let mut seen = Vec::new();
            let step = |x: Decimal| {
                seen.push(x);
                Ok(if x < jump {
                    d("-1")
                } else if x == jump {
                    d("1")
                } else {
                    d("0.9")
                })
            };
            let e = brent(step, low, high, d("0.5"), 1000).unwrap_err();
            assert_eq!(e.code(), "FM-E014");
            assert!(e.to_string().contains("collapsed"), "{e}");
            assert!(seen.iter().all(|x| *x >= low && *x <= high), "{seen:?}");
            assert!(seen.len() < 200, "no progress: {} evaluations", seen.len());
        }
    }

    #[test]
    fn a_narrow_bracket_near_resolution_stays_inside() {
        let (low, high) = (
            d("0.09999999999999999999999997"),
            d("0.09999999999999999999999999"),
        );
        let mut seen = Vec::new();
        let r = brent(
            |x| {
                seen.push(x);
                sub(x, d("0.0999999999999999999999999812"))
            },
            low,
            high,
            d("1e-28"),
            100,
        );
        assert!(seen.iter().all(|x| *x >= low && *x <= high), "{seen:?}");
        assert!(r.is_ok() || r.unwrap_err().code() == "FM-E014");
    }

    #[test]
    fn same_inputs_same_trace() {
        let f = |x: Decimal| sub(mul(mul(x, x)?, x)?, d("7"));
        let a = brent(f, d("0"), d("3"), d("1e-20"), 100).unwrap();
        let b = brent(f, d("0"), d("3"), d("1e-20"), 100).unwrap();
        assert_eq!(a, b);
    }
}
