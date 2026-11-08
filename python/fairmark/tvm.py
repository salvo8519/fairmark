"""Time value of money: present values, NPV and XNPV, IRR and XIRR, MIRR, annuities, perpetuities
and future value.

In ``npv``, ``irr`` and ``mirr`` the first cash flow occurs at t = 0. In ``present_value``
the flows are those of periods 1..n. Dated methods take ``datetime.date`` objects or ISO
8601 strings.
"""

from __future__ import annotations

from collections.abc import Iterable, Sequence
from datetime import date
from decimal import Decimal
from typing import Any, Literal

from ._core import Result, run

Number = Decimal | int | str | float
DateLike = date | str
DayCount = Literal["act_365_fixed", "act_360", "act_act_isda", "thirty_360", "thirty_e_360"]
Compounding = Literal[
    "annual", "semi_annual", "four_monthly", "quarterly", "bimonthly", "monthly", "continuous"
]
PeriodTiming = Literal["end_of_period", "mid_period", "beginning_of_period"]
AnnuityTiming = Literal["ordinary", "due"]
DatedFlows = Iterable[tuple[DateLike, Number]]


def npv(rate: Number, cash_flows: Sequence[Number]) -> Result:
    """Net present value. The first cash flow is at t = 0 and is not discounted."""
    return run("tvm.npv", {"rate": rate, "cash_flows": list(cash_flows)})


def irr(cash_flows: Sequence[Number], *, guess: Number | None = None) -> Result:
    """Internal rate of return, solved deterministically.

    Raises ``FairmarkError`` with code ``FM-E011`` when more than one rate solves the equation.
    """
    payload: dict[str, Any] = {"cash_flows": list(cash_flows)}
    if guess is not None:
        payload["guess"] = guess
    return run("tvm.irr", payload)


def _dated(cash_flows: DatedFlows) -> list[dict[str, Any]]:
    return [{"date": d, "amount": a} for d, a in cash_flows]


def present_value(
    cash_flows: Sequence[Number],
    *,
    timing: PeriodTiming,
    rate: Number | None = None,
    period_rates: Sequence[Number] | None = None,
) -> Result:
    """Present value of the flows of periods 1..n, with one rate or one rate per period.

    >>> present_value([100, 100, 100], period_rates=["0.05", "0.06", "0.07"],
    ...               timing="end_of_period").present_value
    Decimal('269.05475644675080400702...')
    """
    payload: dict[str, Any] = {"cash_flows": list(cash_flows), "timing": timing}
    if rate is not None:
        payload["rate"] = rate
    if period_rates is not None:
        payload["period_rates"] = list(period_rates)
    return run("tvm.present_value", payload)


def xnpv(
    rate: Number,
    cash_flows: DatedFlows,
    *,
    valuation_date: DateLike | None = None,
    day_count: DayCount | None = None,
    compounding: Compounding | None = None,
) -> Result:
    """Net present value of dated flows ``[(date, amount), ...]`` at the valuation date.

    When omitted, the engine applies the ECMA-376 XNPV defaults (valuation at the earliest
    flow, Actual/365 (Fixed), annual compounding) and lists them in ``result.conventions``.
    """
    payload: dict[str, Any] = {"rate": rate, "cash_flows": _dated(cash_flows)}
    if day_count is not None:
        payload["day_count"] = day_count
    if compounding is not None:
        payload["compounding"] = compounding
    if valuation_date is not None:
        payload["valuation_date"] = valuation_date
    return run("tvm.xnpv", payload)


def xirr(
    cash_flows: DatedFlows,
    *,
    day_count: DayCount | None = None,
    compounding: Compounding | None = None,
    guess: Number | None = None,
) -> Result:
    """Annual rate at which the XNPV of dated flows is zero.

    Raises ``FairmarkError`` ``FM-E011`` when the rate is ambiguous; ``uniqueness_verified``
    tells whether other rates are excluded. Omitted options take the ECMA-376 defaults.
    """
    payload: dict[str, Any] = {"cash_flows": _dated(cash_flows)}
    if day_count is not None:
        payload["day_count"] = day_count
    if compounding is not None:
        payload["compounding"] = compounding
    if guess is not None:
        payload["guess"] = guess
    return run("tvm.xirr", payload)


def mirr(cash_flows: Sequence[Number], finance_rate: Number, reinvest_rate: Number) -> Result:
    """Modified internal rate of return (ECMA-376 MIRR). The first flow is at t = 0."""
    return run(
        "tvm.mirr",
        {
            "cash_flows": list(cash_flows),
            "finance_rate": finance_rate,
            "reinvest_rate": reinvest_rate,
        },
    )


def annuity(
    payment: Number,
    rate: Number,
    periods: int,
    *,
    timing: AnnuityTiming,
    growth: Number | None = None,
) -> Result:
    """Present and future value of a level (default) or growing annuity, ordinary or due."""
    payload: dict[str, Any] = {
        "payment": payment,
        "rate": rate,
        "periods": periods,
        "timing": timing,
    }
    if growth is not None:
        payload["growth"] = growth
    return run("tvm.annuity", payload)


def perpetuity(
    payment: Number, rate: Number, *, timing: AnnuityTiming, growth: Number | None = None
) -> Result:
    """Present value of a level (default) or growing perpetuity (Gordon), ordinary or due."""
    payload: dict[str, Any] = {"payment": payment, "rate": rate, "timing": timing}
    if growth is not None:
        payload["growth"] = growth
    return run("tvm.perpetuity", payload)


def future_value(
    rate: Number,
    periods: int,
    *,
    timing: AnnuityTiming,
    present_value: Number | None = None,
    payment: Number | None = None,
) -> Result:
    """Value at t = n of a present amount and level payments at a rate per period.

    Amounts of the same sign accumulate; a negative amount is a withdrawal. Give at least one
    of ``present_value`` and ``payment``.

    >>> future_value("0.01", 12, payment=1000, timing="ordinary").future_value
    Decimal('12682.50301319697206612010...')
    """
    payload: dict[str, Any] = {"rate": rate, "periods": periods, "timing": timing}
    if present_value is not None:
        payload["present_value"] = present_value
    if payment is not None:
        payload["payment"] = payment
    return run("tvm.future_value", payload)
