"""Discounted cash flow on the shared engine (SPEC-0001).

``fcff`` values the operating business from free cash flows to the firm discounted at the
WACC, over one to three explicit stages followed by a terminal value. Every choice is explicit:
the stages and their projection, the terms of the flows and of the rates, the timing, and the
terminal technique. The helpers ``given``, ``constant_growth``, ``linear_decline`` and the
terminal techniques ``gordon``, ``h_model``, ``value_driver``, ``exit_multiple``,
``limited_horizon`` and ``liquidation`` only build the input objects; the engine computes
everything.

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
NegativeEbit = Literal["credit", "zero"]
Metric = Literal["ebitda", "ebit", "revenue", "earnings", "book_value"]
Basis = Literal["enterprise", "equity"]


def given(flows: Sequence[Number]) -> dict[str, Any]:
    """A stage of given flows, one per period."""
    return {"periods": len(flows), "projection": "given", "flows": list(flows)}


def given_lines(lines: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """A stage of given flows built up from line items, one mapping per period, as in
    :func:`free_cash_flows` (``ebit``, ``depreciation_and_amortisation``,
    ``capital_expenditure``, ``change_in_net_working_capital``, optional
    ``other_adjustments`` and ``operating_taxes``). The tax fields go to :func:`fcff`."""
    return {"periods": len(lines), "projection": "given", "lines": [dict(x) for x in lines]}


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


def h_model(
    short_term_growth: Number,
    long_term_growth: Number,
    half_life: Number,
    *,
    first_flow: Literal["last_flow_grown"],
    growth_cap: Number | None = None,
) -> dict[str, Any]:
    """The terminal technique ``tv.h_model`` on the last explicit flow, written
    ``{"tv.h_model": {...}}``: growth declining linearly from ``short_term_growth`` to
    ``long_term_growth`` over 2 x ``half_life`` periods, at the terminal rate of the DCF."""
    terms: dict[str, Any] = {
        "short_term_growth": short_term_growth,
        "long_term_growth": long_term_growth,
        "half_life": half_life,
        "first_flow": first_flow,
    }
    if growth_cap is not None:
        terms["growth_cap"] = growth_cap
    return {"tv.h_model": terms}


def value_driver(
    nopat_next: Number,
    growth: Number,
    ronic: Number,
    *,
    growth_cap: Number | None = None,
) -> dict[str, Any]:
    """The terminal technique ``tv.value_driver``, written ``{"tv.value_driver": {...}}``:
    NOPAT at N + 1, perpetual growth and return on new invested capital, at the terminal rate
    of the DCF."""
    terms: dict[str, Any] = {"nopat_next": nopat_next, "growth": growth, "ronic": ronic}
    if growth_cap is not None:
        terms["growth_cap"] = growth_cap
    return {"tv.value_driver": terms}


def exit_multiple(
    multiple: Number,
    metric: Metric,
    metric_value: Number,
    *,
    next_flow: Number | None = None,
    growth_cap: Number | None = None,
) -> dict[str, Any]:
    """The terminal technique ``tv.exit_multiple``, written ``{"tv.exit_multiple": {...}}``:
    ``multiple`` x the metric at N. The metric must be on the basis of the DCF (``ebitda``,
    ``ebit`` or ``revenue`` for ``fcff``). With ``next_flow`` the growth implied by the
    multiple is computed at the terminal rate; ``growth_cap`` needs ``next_flow``."""
    terms: dict[str, Any] = {"multiple": multiple, "metric": metric, "metric_value": metric_value}
    if next_flow is not None:
        terms["next_flow"] = next_flow
    if growth_cap is not None:
        terms["growth_cap"] = growth_cap
    return {"tv.exit_multiple": terms}


def limited_horizon(
    flow: Number,
    periods: int,
    *,
    growth: Number,
    residual_value: Number,
    growth_cap: Number | None = None,
) -> dict[str, Any]:
    """The terminal technique ``tv.limited_horizon``, written
    ``{"tv.limited_horizon": {...}}``: ``flow`` at the end of period N + 1, growing by
    ``growth`` for ``periods`` periods, then ``residual_value`` at N + ``periods``, at the
    terminal rate of the DCF. ``growth`` and ``residual_value`` are required (give 0 for
    none)."""
    terms: dict[str, Any] = {
        "flow": flow,
        "periods": periods,
        "growth": growth,
        "residual_value": residual_value,
    }
    if growth_cap is not None:
        terms["growth_cap"] = growth_cap
    return {"tv.limited_horizon": terms}


def liquidation(
    assets: Iterable[Mapping[str, Any]],
    *,
    liabilities: Iterable[Mapping[str, Any]],
    liquidation_costs: Iterable[Mapping[str, Any]],
    tax_on_gains: Number,
    basis: Basis,
) -> dict[str, Any]:
    """The terminal technique ``tv.liquidation``, written ``{"tv.liquidation": {...}}``: each
    line is ``{"label", "value"}``; ``basis`` must be the basis of the DCF (``"enterprise"``
    for ``fcff``: the liabilities listed exclude financial debt)."""
    return {
        "tv.liquidation": {
            "assets": [dict(a) for a in assets],
            "liabilities": [dict(x) for x in liabilities],
            "liquidation_costs": [dict(c) for c in liquidation_costs],
            "tax_on_gains": tax_on_gains,
            "basis": basis,
        }
    }


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
    notional_tax_rate: Number | None = None,
    negative_ebit: NegativeEbit | None = None,
) -> Result:
    """Enterprise DCF (``dcf.fcff`` version 3): value of operations, enterprise value and
    equity value from FCFF discounted at the WACC.

    Give ``wacc`` (one rate, also used for the terminal value) or ``wacc_per_period`` with
    ``terminal_wacc``. Under ``timing="mid_period"`` give ``terminal_discount_time``.
    Stages built with :func:`given_lines` take ``notional_tax_rate`` with ``negative_ebit``,
    or ``operating_taxes`` in every line.
    ``terminal`` is built by ``gordon``, ``h_model``, ``value_driver``, ``exit_multiple``,
    ``limited_horizon`` or ``liquidation``.

    >>> r = fcff(
    ...     [given([100, 110, 120])],
    ...     cash_flow_terms="nominal",
    ...     discount_rate_terms="nominal",
    ...     wacc="0.10",
    ...     timing="end_of_period",
    ...     terminal=gordon("0.02", first_flow="last_flow_grown"),
    ...     net_financial_position={"gross_debt": 200, "cash": 0},
    ... )
    >>> r.value_of_operations
    Decimal('1421.4876033057851239669421487')
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
        "notional_tax_rate": notional_tax_rate,
        "negative_ebit": negative_ebit,
    }
    payload.update({k: v for k, v in optional.items() if v is not None})
    return run("dcf.fcff", payload)


def free_cash_flows(
    periods: Sequence[Mapping[str, Any]],
    *,
    notional_tax_rate: Number | None = None,
    negative_ebit: NegativeEbit | None = None,
    interest_tax_rate: Number | None = None,
) -> Result:
    """Free cash flow build-up: FCFF per period and, with the financing lines, FCFE.

    Each period is a mapping with ``ebit``, ``depreciation_and_amortisation``,
    ``capital_expenditure``, ``change_in_net_working_capital`` (positive for an increase) and
    optionally ``other_adjustments`` (``[{"label": ..., "value": ...}]``, positive for an
    inflow), ``interest_expense`` and ``net_borrowing`` (positive for new borrowing).

    Taxes on EBIT: give ``notional_tax_rate`` with ``negative_ebit`` (``"credit"`` or
    ``"zero"``), or ``operating_taxes`` in every period. With ``operating_taxes`` and the
    financing lines, ``interest_tax_rate`` is required. Amounts are not discounted.

    >>> r = free_cash_flows(
    ...     [{"ebit": 100, "depreciation_and_amortisation": 10, "capital_expenditure": 12,
    ...       "change_in_net_working_capital": 3}],
    ...     notional_tax_rate="0.24",
    ...     negative_ebit="credit",
    ... )
    >>> r.free_cash_flows_to_firm
    [Decimal('71.00')]
    """
    payload: dict[str, Any] = {"periods": [dict(p) for p in periods]}
    for key, value in (
        ("notional_tax_rate", notional_tax_rate),
        ("negative_ebit", negative_ebit),
        ("interest_tax_rate", interest_tax_rate),
    ):
        if value is not None:
            payload[key] = value
    return run("dcf.free_cash_flows", payload)
