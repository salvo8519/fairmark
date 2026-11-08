"""Independent reference for SPEC-0006 (terminal value methods `tv.*`).

Written from the formulas of the spec, in Python's `decimal` module at 50 digits in a local
context (importing this module never changes the caller's precision). It shares no code with
the Rust engine: the finite-horizon value sums the discounted flows one by one instead of
using the annuity closed form, so it also checks the reuse of `tvm.annuity`.

Usage:

    uv run python scripts/reference_tv.py

prints the expected values of the golden cases in `tests/golden/tv.*/`.
"""

from __future__ import annotations

from collections.abc import Callable, Sequence
from decimal import Context, localcontext
from decimal import Decimal as D
from functools import wraps
from typing import Any, TypeVar

T = TypeVar("T", bound=Callable[..., Any])

CONTEXT = Context(prec=50)


def at_50_digits(func: T) -> T:
    @wraps(func)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        with localcontext(CONTEXT):
            return func(*args, **kwargs)

    return wrapper  # type: ignore[return-value]


@at_50_digits
def gordon(rate: D, growth: D, *, next_flow: D | None = None, last_flow: D | None = None) -> D:
    """R1: TV_N = CF_{N+1} / (r - g), with CF_{N+1} = CF_N (1 + g) when CF_N is given."""
    assert (next_flow is None) != (last_flow is None)
    assert growth < rate
    flow = next_flow if next_flow is not None else last_flow * (1 + growth)  # type: ignore[operator]
    return flow / (rate - growth)


@at_50_digits
def value_driver(nopat_next: D, growth: D, ronic: D, rate: D) -> D:
    """R2: TV_N = NOPAT_{N+1} (1 - g / RONIC) / (r - g)."""
    assert ronic > 0 and growth < rate
    return nopat_next * (1 - growth / ronic) / (rate - growth)


@at_50_digits
def h_model(last_flow: D, short_growth: D, long_growth: D, half_life: D, rate: D) -> D:
    """R3: TV_N = CF_N [(1 + g_l) + H (g_s - g_l)] / (r - g_l)."""
    assert long_growth < rate
    return (
        last_flow
        * ((1 + long_growth) + half_life * (short_growth - long_growth))
        / (rate - long_growth)
    )


@at_50_digits
def exit_multiple(multiple: D, metric: D) -> D:
    """R4: TV_N = multiple x metric_N."""
    return multiple * metric


@at_50_digits
def implied_growth(rate: D, next_flow: D, terminal_value: D) -> D:
    """R4: g = r - CF_{N+1} / TV_N."""
    return rate - next_flow / terminal_value


@at_50_digits
def limited_horizon(
    flow: D, rate: D, periods: int, growth: D = D(0), residual_value: D = D(0)
) -> tuple[D, D, D]:
    """R5: (value at N, value of the n flows, discounted residual value).

    Flows at the end of periods N + 1 ... N + n, the first equal to `flow`, then growing by
    `growth`; the residual value is at N + n. Summed term by term.
    """
    flows = sum(
        (flow * (1 + growth) ** (k - 1) / (1 + rate) ** k for k in range(1, periods + 1)),
        D(0),
    )
    residual = residual_value / (1 + rate) ** periods
    return flows + residual, flows, residual


@at_50_digits
def normalised_income(income: D, capitalisation_rate: D, growth: D | None = None) -> D:
    """R6: R / i, or R / (i - g) when a growth is given."""
    if growth is None:
        return income / capitalisation_rate
    assert growth < capitalisation_rate
    return income / (capitalisation_rate - growth)


@at_50_digits
def liquidation(
    assets: Sequence[D], liabilities: Sequence[D], costs: Sequence[D], tax_on_gains: D
) -> D:
    """R7: realisable assets - liabilities - liquidation costs - tax on gains."""
    return sum(assets, D(0)) - sum(liabilities, D(0)) - sum(costs, D(0)) - tax_on_gains


def show(label: str, value: D) -> None:
    with localcontext(CONTEXT):
        print(f"{label}: {value.quantize(D('1e-28')).normalize():f}")


if __name__ == "__main__":
    print("tv.gordon")
    show(
        "  Damodaran table cell as FCFF 75, r 10%, g 3%",
        gordon(D("0.10"), D("0.03"), next_flow=D(75)),
    )
    show("  last flow 100, r 9%, g 2.5%", gordon(D("0.09"), D("0.025"), last_flow=D(100)))
    show("  next flow 50, r 8%, g 4%", gordon(D("0.08"), D("0.04"), next_flow=D(50)))
    print("tv.value_driver (NOPAT 100, r 10%)")
    for g, roc in [("0.02", "0.06"), ("0.025", "0.08"), ("0.03", "0.14"), ("0.03", "0.10")]:
        show(f"  g {g}, RONIC {roc}", value_driver(D(100), D(g), D(roc), D("0.10")))
    show("  g 0.02, RONIC 0.12", value_driver(D(100), D("0.02"), D("0.12"), D("0.10")))
    print("tv.h_model")
    show(
        "  CF 100, g_s 10%, g_l 3%, H 5, r 9%",
        h_model(D(100), D("0.10"), D("0.03"), D(5), D("0.09")),
    )
    show(
        "  CF 100, g_s 8%, g_l 2%, H 2.5, r 7.5%",
        h_model(D(100), D("0.08"), D("0.02"), D("2.5"), D("0.075")),
    )
    print("tv.exit_multiple")
    tv = exit_multiple(D(8), D(125))
    show("  8 x EBITDA 125", tv)
    show("  implied growth, r 10%, next flow 60", implied_growth(D("0.10"), D(60), tv))
    show("  12 x earnings 50", exit_multiple(D(12), D(50)))
    show("  7.5 x EBITDA 180", exit_multiple(D("7.5"), D(180)))
    show(
        "  implied growth, r 9%, next flow 95",
        implied_growth(D("0.09"), D(95), exit_multiple(D("7.5"), D(180))),
    )
    print("tv.limited_horizon")
    for args in [
        (D(100), D("0.08"), 10, D(0), D(0)),
        (D(100), D("0.10"), 5, D("0.02"), D(500)),
        (D(100), D("0.10"), 10000, D(0), D(0)),
    ]:
        total, flows, residual = limited_horizon(*args)
        show(f"  {args}: value", total)
        show("    flows", flows)
        show("    residual", residual)
    print("tv.normalised_income")
    show("  R 120, i 8%", normalised_income(D(120), D("0.08")))
    show("  R 120, i 9%, g 1%", normalised_income(D(120), D("0.09"), D("0.01")))
    show("  R 95, i 11%, g 1.5%", normalised_income(D(95), D("0.11"), D("0.015")))
    print("tv.liquidation")
    show(
        "  example",
        liquidation([D(800), D(300), D(200)], [D(600), D(250)], [D(40), D(60)], D(30)),
    )
