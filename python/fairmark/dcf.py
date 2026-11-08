"""Discounted cash flow on the shared engine (SPEC-0001).

``fcff`` values the operating business from free cash flows to the firm discounted at the
WACC, over one to three explicit stages followed by a terminal value. Every choice is explicit:
the stages and their projection, the terms of the flows and of the rates, the timing, and the
terminal technique. The helpers ``given``, ``constant_growth``, ``linear_decline`` and
``gordon`` only build the input objects; the engine computes everything.

Rates and growths are decimal fractions per period (``"0.09"`` = 9%); one period is one year.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping, Sequence
from decimal import Decimal
from typing import Any, Literal

from ._core import Result, run

Number = Decimal | int | str | float
Timing = Literal["end_of_period", "mid_period"]
Terms = Literal["nominal", "real"]
TerminalDiscountTime = Literal["horizon", "mid_period"]
FirstFlow = Literal["last_flow_grown", "normalised_next_flow"]


def given(flows: Sequence[Number]) -> dict[str, Any]:
    """A stage of given flows, one per period."""
    return {"periods": len(flows), "projection": "given", "flows": list(flows)}


def constant_growth(periods: int, growth: Number) -> dict[str, Any]:
    """A stage of ``periods`` flows, each equal to the previous one times (1 + growth)."""
    return {"periods": periods, "projection": "constant_growth", "growth": growth}


def linear_decline(periods: int, from_growth: Number, to_growth: Number) -> dict[str, Any]:
    """A stage whose growth moves from ``from_growth`` (the growth before the stage) to
    ``to_growth`` in equal steps, reaching ``to_growth`` in its last period."""
    return {
        "periods": periods,
        "projection": "linear_decline",
        "from_growth": from_growth,
        "to_growth": to_growth,
    }


def gordon(
    growth: Number,
    *,
    first_flow: FirstFlow,
    normalised_next_flow: Number | None = None,
    growth_cap: Number | None = None,
) -> dict[str, Any]:
    """The terminal technique ``tv.gordon``, written ``{"tv.gordon": {...}}``.

    ``first_flow`` is ``"last_flow_grown"`` (CF_N x (1 + g)) or ``"normalised_next_flow"``
    (then give ``normalised_next_flow``, the flow at N + 1 in the terms of the cash flows).
    """
    terms: dict[str, Any] = {"growth": growth, "first_flow": first_flow}
    if normalised_next_flow is not None:
        terms["normalised_next_flow"] = normalised_next_flow
    if growth_cap is not None:
        terms["growth_cap"] = growth_cap
    return {"tv.gordon": terms}


def fcff(
    stages: Iterable[Mapping[str, Any]],
    *,
    cash_flow_terms: Terms,
    discount_rate_terms: Terms,
    timing: Timing,
    terminal: Mapping[str, Any],
    wacc: Number | None = None,
    wacc_per_period: Sequence[Number] | None = None,
    terminal_wacc: Number | None = None,
    terminal_discount_time: TerminalDiscountTime | None = None,
    base_flow: Number | None = None,
    net_financial_position: Mapping[str, Any] | None = None,
    non_operating_assets: Iterable[Mapping[str, Any]] | None = None,
) -> Result:
    """Enterprise DCF (``dcf.fcff`` version 3): value of operations, enterprise value and
    equity value from FCFF discounted at the WACC.

    Give ``wacc`` (one rate, also used for the terminal value) or ``wacc_per_period`` with
    ``terminal_wacc``. Under ``timing="mid_period"`` give ``terminal_discount_time``.

    >>> r = fcff(
    ...     [given([100, 110, 120])],
    ...     cash_flow_terms="nominal",
    ...     discount_rate_terms="nominal",
    ...     wacc="0.10",
    ...     timing="end_of_period",
    ...     terminal=gordon("0.02", first_flow="last_flow_grown"),
    ...     net_financial_position={"gross_debt": 200, "cash": 0},
    ... )
    >>> r.value["terminal"]["terminal_value"]
    Decimal('1530')
    """
    payload: dict[str, Any] = {
        "stages": [dict(s) for s in stages],
        "cash_flow_terms": cash_flow_terms,
        "discount_rate_terms": discount_rate_terms,
        "timing": timing,
        "terminal": dict(terminal),
    }
    optional: dict[str, Any] = {
        "wacc": wacc,
        "wacc_per_period": None if wacc_per_period is None else list(wacc_per_period),
        "terminal_wacc": terminal_wacc,
        "terminal_discount_time": terminal_discount_time,
        "base_flow": base_flow,
        "net_financial_position": (
            None if net_financial_position is None else dict(net_financial_position)
        ),
        "non_operating_assets": (
            None if non_operating_assets is None else [dict(a) for a in non_operating_assets]
        ),
    }
    payload.update({k: v for k, v in optional.items() if v is not None})
    return run("dcf.fcff", payload)
