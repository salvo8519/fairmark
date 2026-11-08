"""Value series: a base value grown period by period on a timeline (SPEC-0010)."""

from __future__ import annotations

from decimal import Decimal
from typing import Any, Literal

from ._core import Result, run
from .dates import Frequency

Number = Decimal | int | str | float
BasePeriod = Literal["period_0", "period_1"]
GrowthBasis = Literal["per_period", "annual_compound", "annual_simple"]
Timing = Literal["end_of_period", "beginning_of_period"]


def constant(growth: Number) -> dict[str, Any]:
    """The same growth rate for every growth step."""
    return {"constant": {"growth": growth}}


def linear(from_growth: Number, to_growth: Number) -> dict[str, Any]:
    """Growth moving in equal steps from ``from_growth`` (the growth before the first step) to
    ``to_growth`` (reached in the last step)."""
    return {"linear": {"from_growth": from_growth, "to_growth": to_growth}}


def per_period(growth_rates: list[Number]) -> dict[str, Any]:
    """One growth rate per growth step: n rates under ``period_0``, n - 1 under ``period_1``."""
    return {"per_period": {"growth_rates": list(growth_rates)}}


def periods(
    number_of_periods: int,
    *,
    frequency: Frequency | None = None,
    period_months: int | None = None,
) -> dict[str, Any]:
    """A timeline of ``number_of_periods`` periods without dates, with an optional period
    length (at most one of ``frequency`` and ``period_months``)."""
    timeline: dict[str, Any] = {"number_of_periods": number_of_periods}
    if frequency is not None:
        timeline["frequency"] = frequency
    if period_months is not None:
        timeline["period_months"] = period_months
    return {"periods": timeline}


def schedule(schedule_input: dict[str, Any]) -> dict[str, Any]:
    """A timeline of dated periods, defined as the input of ``dates.schedule`` (for example
    ``fairmark.dates.schedule_input("2025-12-31", 4, frequency="quarterly")``)."""
    return {"schedule": schedule_input}


def project(
    base_value: Number,
    *,
    base_period: BasePeriod,
    growth: dict[str, Any],
    timeline: dict[str, Any],
    growth_basis: GrowthBasis | None = None,
    timing: Timing | None = None,
) -> Result:
    """Values of periods 1 to n grown from ``base_value``: V_k = V_(k-1) x (1 + g_k).

    ``base_period`` is required: ``"period_0"`` when the base value precedes period 1 (the
    last actual year), ``"period_1"`` when it is the value of period 1. ``growth`` is built by
    :func:`constant`, :func:`linear` or :func:`per_period`; ``timeline`` by :func:`periods` or
    :func:`schedule`. ``growth_basis`` says how the rates are read (required when the periods
    are not 12 months long); ``timing`` dates the values of a schedule timeline, and the result
    then carries ``dated_values`` in the shape of ``fairmark.tvm.xnpv`` cash flows.

    >>> r = project(100, base_period="period_0", growth=constant("0.05"), timeline=periods(3))
    >>> r.values
    [Decimal('105.00'), Decimal('110.2500'), Decimal('115.762500')]
    """
    payload: dict[str, Any] = {
        "base_value": base_value,
        "base_period": base_period,
        "growth": growth,
        "timeline": timeline,
    }
    if growth_basis is not None:
        payload["growth_basis"] = growth_basis
    if timing is not None:
        payload["timing"] = timing
    return run("series.project", payload)
