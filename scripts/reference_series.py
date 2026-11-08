"""Independent reference for `series.project` (SPEC-0010 R9 to R18).

Written from the spec with Python's `decimal` module at 50 significant digits in a local
context; it shares no code with the Rust engine and is used to produce or confirm golden
expectations:

    uv run python scripts/reference_series.py

Independence from the engine, by construction:
- the equivalent rate per period of `annual_compound` is the m-th root of the growth over one
  year, `exp(ln(1 + g) / m) - 1` (the engine goes through `rates.convert` to a nominal rate
  with compounding m and divides it by m);
- a period of k years compounds with `(1 + g) ** k`, the proportional rates are `g / m` and
  `g * k`;
- the linear rule is written as g_j = g_from + (g_to - g_from) * j / s;
- the values follow the recurrence V_k = V_{k-1} * (1 + g_k), and the end-to-end check of
  AC10 discounts the dated values with the XNPV of `scripts/reference_time.py`.
"""

from __future__ import annotations

import importlib.util
from collections.abc import Callable
from datetime import date
from decimal import Context, localcontext
from decimal import Decimal as D
from functools import wraps
from pathlib import Path
from typing import Any, TypeVar

T = TypeVar("T", bound=Callable[..., Any])
ROOT = Path(__file__).resolve().parents[1]


def at_50_digits(fn: T) -> T:
    @wraps(fn)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        with localcontext(Context(prec=50)):
            return fn(*args, **kwargs)

    return wrapper  # type: ignore[return-value]


def growth_steps(n: int, base_period: str) -> int:
    """Growth steps s: n under `period_0`, n - 1 under `period_1` (R10)."""
    return n if base_period == "period_0" else n - 1


@at_50_digits
def linear(g_from: D, g_to: D, s: int) -> list[D]:
    """g_j = g_from + (g_to - g_from) * j / s, j = 1..s (R11)."""
    return [g_from + (g_to - g_from) * j / s for j in range(1, s + 1)]


@at_50_digits
def per_period_rate(g: D, basis: str | None, months: int | None) -> D:
    """The growth per period of a stated rate (R13)."""
    if basis in (None, "per_period") or months == 12:
        return +g
    assert months is not None
    if 12 % months == 0:
        m = 12 // months
        if basis == "annual_compound":
            return ((1 + g).ln() / m).exp() - 1
        return g / m
    assert months % 12 == 0, "annual bases need p | 12 or 12 | p"
    k = months // 12
    if basis == "annual_compound":
        return (1 + g) ** k - 1
    return g * k


@at_50_digits
def effective_annual(rate_per_period: D, months: int) -> D:
    """(1 + g_p)^m - 1, m = 12 / p (R16)."""
    return (1 + rate_per_period) ** (12 // months) - 1


@at_50_digits
def project(base: D, stated: list[D], base_period: str, basis: str | None, months: int | None):
    """Values of periods 1..n and the rates per period applied (R9, R10)."""
    rates = [per_period_rate(g, basis, months) for g in stated]
    values = [] if base_period == "period_0" else [+base]
    v = +base
    for g in rates:
        v = v * (1 + g)
        values.append(v)
    return values, rates


def _reference_time():
    spec = importlib.util.spec_from_file_location("ref_time", ROOT / "scripts/reference_time.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def show(label: str, values: list[D]) -> None:
    print(f"{label}:")
    for k, v in enumerate(values, 1):
        print(f"  {k}: {v}")


@at_50_digits
def main() -> None:
    # AC6: base 100, constant 0.05, three periods.
    show("AC6 period_0", project(D(100), [D("0.05")] * 3, "period_0", None, None)[0])
    show("AC6 period_1", project(D(100), [D("0.05")] * 2, "period_1", None, None)[0])

    # AC7: Damodaran, Investment Valuation, 2nd ed., Illustration 15.3 (Amgen) and SPEC-0001.
    amgen = linear(D("0.1308"), D("0.05"), 5)
    show("AC7 Amgen rates", amgen)
    show("AC7 Amgen values (base 1000)", project(D(1000), amgen, "period_0", None, None)[0])
    decline = linear(D("0.10"), D("0.03"), 5)
    show("AC7 0.10 to 0.03 rates", decline)
    show("AC7 0.10 to 0.03 values (base 100)", project(D(100), decline, "period_0", None, None)[0])

    # AC8: period_1, four periods, three rates.
    rates = [D("0.10"), D("0.08"), D("-0.02")]
    show("AC8 period_1 values (base 500)", project(D(500), rates, "period_1", None, None)[0])

    # AC9: base 1000, 12% a year, one year of periods for every length dividing 12.
    for months in (1, 2, 3, 4, 6):
        n = 12 // months
        for basis in ("annual_compound", "annual_simple"):
            values, applied = project(D(1000), [D("0.12")] * n, "period_0", basis, months)
            print(f"AC9 p={months} {basis}: rate per period {applied[0]}")
            print(f"  effective annual {effective_annual(applied[0], months)}")
            print(f"  V_{n} {values[-1]}")
    for basis in ("annual_compound", "annual_simple"):
        values, applied = project(D(1000), [D("0.12")] * 2, "period_0", basis, 24)
        print(f"AC9 p=24 {basis}: rate per period {applied[0]}, values {values}")

    # AC10: quarterly schedule from 2026-01-01, base 300, 2% a quarter, period_0; the dated
    # values at the period end dates after an outlay of 1000 on 2026-01-01, XNPV at 10%.
    values, _ = project(D(300), [D("0.02")] * 4, "period_0", "per_period", 3)
    show("AC10 values", values)
    ends = [date(2026, 4, 1), date(2026, 7, 1), date(2026, 10, 1), date(2027, 1, 1)]
    flows = [(date(2026, 1, 1), D(-1000)), *zip(ends, values, strict=True)]
    print(f"AC10 XNPV at 0.10 (act_365_fixed, annual): {_reference_time().xnpv(D('0.10'), flows)}")

    # AC12: a negative base value grows more negative under a positive growth.
    show("AC12 negative base", project(D(-200), [D("0.10")] * 3, "period_0", None, None)[0])


if __name__ == "__main__":
    main()
