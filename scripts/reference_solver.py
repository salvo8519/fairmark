"""Independent 50-digit reference for the documented rate solver of `tvm.irr` and `tvm.xirr`.

Written on 2026-11-09 for issue #282 (missed mutants of the solver). It shares no code with
the engine and uses Python's `decimal` module only. It follows the solver as the methodology
pages document it (docs/en/methods/tvm-irr.md, "Solver"; tvm-xirr.md):

  Newton's method from the guess, r_(i+1) = r_i - f(r_i) / f'(r_i). Stopping rule, as
  tvm-irr.md states it: "Iteration stops when the step on the rate is at most 10^-18, at most
  200 iterations per stage." The iteration count is the number of Newton steps taken, the
  returned rate is the last iterate. The grid scan and the acceptance of Newton's result inside
  the closed interval [lower rate, upper rate], ends included (tvm-irr.md step 3), are not
  reproduced here: the cases are chosen so that Newton's result lies inside the interval.

  IRR:  f(r) = sum CF_t (1 + r)^-t,          f'(r) = sum -t CF_t (1 + r)^(-t-1)
  XIRR: f(r) = sum A_i (1 + r/m)^(-m t_i),   f'(r) = sum -t_i A_i (1 + r/m)^(-m t_i - 1)
        or, continuously compounded, f(r) = sum A_i e^(-r t_i), f'(r) = sum -t_i A_i e^(-r t_i)
        with t_i = days / 365 (Actual/365 Fixed, the ECMA-376 default)

Flows built from chosen roots (products of (x - x_j) with x = 1 + r) give cases whose rates
are known exactly, independently of any solver.

    python scripts/reference_solver.py
"""

from __future__ import annotations

from datetime import date
from decimal import Decimal as D
from decimal import localcontext

PREC = 50
TOLERANCE = D("1e-18")
MAX_ITERATIONS = 200


def newton(f, df, guess: D) -> tuple[D, int] | None:
    """Newton's method with the documented stopping rule; None when it does not converge."""
    with localcontext() as ctx:
        ctx.prec = PREC
        r = guess
        for it in range(1, MAX_ITERATIONS + 1):
            d = df(r)
            if d == 0:
                return None
            nxt = r - f(r) / d
            if nxt <= -1:
                return None
            if abs(nxt - r) <= TOLERANCE:
                return nxt, it
            r = nxt
    return None


def irr_newton(flows: list[D], guess: D = D("0.10")) -> tuple[D, int] | None:
    def f(r: D) -> D:
        return sum((cf / (1 + r) ** t for t, cf in enumerate(flows)), D(0))

    def df(r: D) -> D:
        return sum((-t * cf / (1 + r) ** (t + 1) for t, cf in enumerate(flows)), D(0))

    return newton(f, df, guess)


def xirr_newton(
    flows: list[tuple[str, D]], periods_per_year: int | None, guess: D = D("0.10")
) -> tuple[D, int] | None:
    start = min(date.fromisoformat(d) for d, _ in flows)
    with localcontext() as ctx:
        ctx.prec = PREC
        times = [(D((date.fromisoformat(d) - start).days) / D(365), a) for d, a in flows]

    if periods_per_year is None:

        def f(r: D) -> D:
            return sum((a * (-r * t).exp() for t, a in times), D(0))

        def df(r: D) -> D:
            return sum((-t * a * (-r * t).exp() for t, a in times), D(0))

    else:
        m = D(periods_per_year)

        def f(r: D) -> D:
            return sum((a * (1 + r / m) ** (-m * t) for t, a in times), D(0))

        def df(r: D) -> D:
            return sum((-t * a * (1 + r / m) ** (-m * t - 1) for t, a in times), D(0))

    return newton(f, df, guess)


def flows_from_roots(lead: D, roots: list[D]) -> list[D]:
    """Coefficients CF_0..CF_n of NPV(r) (1 + r)^n = lead * prod (x - x_j), x = 1 + r.

    NPV(r) = sum CF_t x^-t, so NPV(r) x^n = sum CF_t x^(n - t): CF_t is the coefficient of
    x^(n - t), highest power first.
    """
    poly = [lead]
    for x in roots:
        poly = [a - x * b for a, b in zip([*poly, D(0)], [D(0), *poly], strict=True)]
    return poly


XIRR_FLOWS = [
    ("2008-01-01", D(-10000)),
    ("2008-03-01", D(2750)),
    ("2008-10-30", D(4250)),
    ("2009-02-15", D(3250)),
    ("2009-04-01", D(2750)),
]


if __name__ == "__main__":
    print("irr newton [-1000, 300, 400, 500]", irr_newton([D(-1000), D(300), D(400), D(500)]))
    print("irr newton [-1, 20]", irr_newton([D(-1), D(20)]))
    print("xirr newton annual", xirr_newton(XIRR_FLOWS, 1))
    print("xirr newton monthly", xirr_newton(XIRR_FLOWS, 12))
    print("xirr newton continuous", xirr_newton(XIRR_FLOWS, None))
    below = flows_from_roots(D(-1), [D("1.11"), D("1.12"), D("1.53")])
    print("roots 0.11, 0.12, 0.53", below, "newton from 0.10:", irr_newton(below))
    above = flows_from_roots(D(-1), [D("1.21"), D("1.61"), D("1.62")])
    print("roots 0.21, 0.61, 0.62", above, "newton from 0.60:", irr_newton(above, D("0.60")))
