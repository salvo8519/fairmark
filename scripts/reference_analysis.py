"""Independent reference for SPEC-0008 (analysis: goal seek, tables, scenarios, tornado).

Two references in Python's `decimal` module at 50 digits, sharing no code with the Rust engine:

- `bisect`, to 1e-30 on the variable: the expected solutions of the golden cases;
- `brent_textbook`, Brent's method written from its textbook formulation (Numerical Recipes,
  section 9.3, `zbrent`), with the engine's stopping rule on the output and minimum step: the
  engine must reproduce its iterates, so a change in the solver path cannot go unnoticed.

Usage:

    uv run python scripts/reference_analysis.py

The DCF function restates `dcf.fcff` from its documented formula: end-of-period discounting,
Gordon terminal value at the end of year n.
"""

from __future__ import annotations

from collections.abc import Callable
from decimal import Context, localcontext
from decimal import Decimal as D
from functools import wraps
from typing import Any

# 50 digits in a local context: importing this module never changes the caller's precision.
CONTEXT = Context(prec=50)


def at_50_digits[T: Callable[..., Any]](func: T) -> T:
    @wraps(func)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        with localcontext(CONTEXT):
            return func(*args, **kwargs)

    return wrapper  # type: ignore[return-value]


@at_50_digits
def bisect(f: Callable[[D], D], low: D, high: D, xtol: D = D("1e-30")) -> D:
    flow, fhigh = f(low), f(high)
    assert flow * fhigh < 0, "target not bracketed"
    while high - low > xtol:
        mid = (low + high) / 2
        fmid = f(mid)
        if fmid == 0:
            return mid
        if (fmid < 0) == (flow < 0):
            low, flow = mid, fmid
        else:
            high = mid
    return (low + high) / 2


@at_50_digits
def brent_textbook(
    f: Callable[[D], D], low: D, high: D, ftol: D, itmax: int = 100
) -> tuple[D, int]:
    """Root and number of iterations after the two bracket ends."""
    a, b = low, high
    fa, fb = f(a), f(b)
    if abs(fa) <= ftol:
        return a, 0
    if abs(fb) <= ftol:
        return b, 0
    c, fc = b, fb
    d = e = b - a
    for iteration in range(1, itmax + 1):
        if (fb > 0 and fc > 0) or (fb < 0 and fc < 0):
            c, fc = a, fa
            d = e = b - a
        if abs(fc) < abs(fb):
            a, b, c = b, c, b
            fa, fb, fc = fb, fc, fb
        tol1 = D("1e-24") * max(D(1), abs(b))
        xm = (c - b) / 2
        if abs(e) >= tol1 and abs(fa) > abs(fb):
            s = fb / fa
            if a == c:
                p, q = 2 * xm * s, 1 - s
            else:
                q, r = fa / fc, fb / fc
                p = s * (2 * xm * q * (q - r) - (b - a) * (r - 1))
                q = (q - 1) * (r - 1) * (s - 1)
            if p > 0:
                q = -q
            p = abs(p)
            if 2 * p < min(3 * xm * q - abs(tol1 * q), abs(e * q)):
                e, d = d, p / q
            else:
                d = e = xm
        else:
            d = e = xm
        a, fa = b, fb
        b = b + d if abs(d) > tol1 else b + (tol1 if xm >= 0 else -tol1)
        fb = f(b)
        if abs(fb) <= ftol:
            return b, iteration
    raise RuntimeError("no convergence")


@at_50_digits
def npv(rate: D, flows: list[D]) -> D:
    return sum((cf / (1 + rate) ** t for t, cf in enumerate(flows)), D(0))


@at_50_digits
def enterprise_value(rate: D, flows: list[D], growth: D) -> D:
    n = len(flows)
    explicit = sum((cf / (1 + rate) ** (t + 1) for t, cf in enumerate(flows)), D(0))
    terminal = flows[-1] * (1 + growth) / (rate - growth)
    return explicit + terminal / (1 + rate) ** n


@at_50_digits
def equity_value(rate: D, flows: list[D], growth: D, net_debt: D) -> D:
    return enterprise_value(rate, flows, growth) - net_debt


@at_50_digits
def sensitivity_table(rates: list[D], growths: list[D], flows: list[D]) -> list[list[D | None]]:
    """Enterprise value per (rate, growth); None where growth >= rate (no Gordon value)."""
    return [[enterprise_value(r, flows, g) if g < r else None for g in growths] for r in rates]


@at_50_digits
def tornado_ranking(
    base: dict[str, D], flows: list[D], shocks: list[tuple[str, D, D]]
) -> list[tuple[str, D, D, D]]:
    """(name, equity at low, equity at high, swing) by decreasing |swing|, stable on ties.

    Names: discount_rate, terminal_growth, net_debt, or flow:<index>.
    """

    def equity(changes: dict[str, D]) -> D:
        values = {**base, **changes}
        f = list(flows)
        for k, v in changes.items():
            if k.startswith("flow:"):
                f[int(k[5:])] = v
        return equity_value(
            values["discount_rate"], f, values["terminal_growth"], values["net_debt"]
        )

    bars = []
    for name, low, high in shocks:
        lo, hi = equity({name: low}), equity({name: high})
        bars.append((name, lo, hi, hi - lo))
    return sorted(bars, key=lambda b: -abs(b[3]))


def irr_via_npv() -> D:
    flows = [D(-1000), D(300), D(400), D(500)]
    return bisect(lambda r: npv(r, flows), D(0), D(1))


def implied_discount_rate() -> D:
    flows = [D(100), D(110), D(120)]
    return bisect(lambda r: enterprise_value(r, flows, D("0.02")) - D(1300), D("0.05"), D("0.5"))


if __name__ == "__main__":
    print("IRR of -1000, 300, 400, 500:", irr_via_npv())
    print("Discount rate giving EV 1300 (FCF 100, 110, 120; g 2%):", implied_discount_rate())
