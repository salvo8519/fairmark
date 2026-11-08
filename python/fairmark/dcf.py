"""Discounted cash flow on the shared engine (SPEC-0001).

``fcff`` values the operating business from free cash flows to the firm discounted at the
WACC, and ``fcfe`` values the equity from free cash flows to equity discounted at the cost of
equity, each over one to three explicit stages followed by a terminal value. Every choice is
explicit: the stages and their projection, the terms of the flows and of the rates, the timing,
and the terminal technique. The helpers ``given``, ``constant_growth``, ``linear_decline`` and the
terminal techniques ``gordon``, ``h_model``, ``value_driver``, ``exit_multiple``,
``limited_horizon``, ``normalised_income`` and ``liquidation`` only build the input objects;
the engine computes everything.

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
MeasurementBasis = Literal["book_value", "market_value", "fair_value", "present_value", "other"]


def given(flows: Sequence[Number]) -> dict[str, Any]:
    """A stage of given flows, one per period."""
    return {"periods": len(flows), "projection": "given", "flows": list(flows)}


def given_lines(lines: Sequence[Mapping[str, Any]]) -> dict[str, Any]:
    """A stage of given flows built up from line items, one mapping per period, as in
    :func:`free_cash_flows` (``ebit``, ``depreciation_and_amortisation``,
    ``capital_expenditure``, ``change_in_net_working_capital``, optional
    ``other_adjustments`` and ``operating_taxes``; for :func:`fcfe` also ``interest_expense``
    and ``net_borrowing``). The tax fields go to :func:`fcff` or :func:`fcfe`."""
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
    first_flow: Literal["last_flow_grown", "normalised_base_flow"],
    normalised_base_flow: Number | None = None,
    growth_cap: Number | None = None,
) -> dict[str, Any]:
    """The terminal technique ``tv.h_model``, written ``{"tv.h_model": {...}}``: growth
    declining linearly from ``short_term_growth`` to ``long_term_growth`` over 2 x
    ``half_life`` periods, at the terminal rate of the DCF.

    ``first_flow`` is ``"last_flow_grown"`` (the H-model grows the last explicit flow CF_N) or
    ``"normalised_base_flow"`` (then give ``normalised_base_flow``, a normalised CF_N at the
    horizon N that replaces the last explicit flow).
    """
    terms: dict[str, Any] = {
        "short_term_growth": short_term_growth,
        "long_term_growth": long_term_growth,
        "half_life": half_life,
        "first_flow": first_flow,
    }
    if normalised_base_flow is not None:
        terms["normalised_base_flow"] = normalised_base_flow
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


def normalised_income(
    income: Number,
    *,
    basis: Basis,
    growth: Number,
    growth_cap: Number | None = None,
) -> dict[str, Any]:
    """The terminal technique ``tv.normalised_income``, written
    ``{"tv.normalised_income": {...}}``: the normalised income at N + 1 capitalised at the
    terminal rate of the DCF (no separate capitalisation rate), R / (r - g). ``basis`` must be
    the basis of the DCF (``"enterprise"`` for ``fcff``); ``growth`` is required (give 0 for
    none)."""
    terms: dict[str, Any] = {"income": income, "basis": basis, "growth": growth}
    if growth_cap is not None:
        terms["growth_cap"] = growth_cap
    return {"tv.normalised_income": terms}


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


def bridge_claim(
    label: str,
    value: Number,
    *,
    measurement_basis: MeasurementBasis,
    basis_note: str | None = None,
) -> dict[str, Any]:
    """A debt-like item or a non-controlling interest for :func:`fcff` or :func:`fcfe`: the
    amount at the valuation date (zero or more), how it was measured, and an optional note that
    qualifies the basis (required with ``measurement_basis="other"``)."""
    claim: dict[str, Any] = {
        "label": label,
        "value": value,
        "measurement_basis": measurement_basis,
    }
    if basis_note is not None:
        claim["basis_note"] = basis_note
    return claim


def _plain(value: Any) -> Any:
    """Copies mappings to dicts and other iterables (not strings) to lists, at every level, so
    that the payload is plain JSON-like data whatever the caller passed."""
    if isinstance(value, Mapping):
        return {k: _plain(v) for k, v in value.items()}
    if isinstance(value, Iterable) and not isinstance(value, str | bytes):
        return [_plain(v) for v in value]
    return value


def _dcf_payload(required: Mapping[str, Any], optional: Mapping[str, Any]) -> dict[str, Any]:
    """The input of a DCF method: every required field, and the optional fields that were
    given. An optional field left at None is omitted, so the engine applies its documented
    rule; the helper never fills in a value."""
    payload = {k: _plain(v) for k, v in required.items()}
    payload.update({k: _plain(v) for k, v in optional.items() if v is not None})
    return payload


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
    inflation: Number | None = None,
    inflation_per_period: Sequence[Number] | None = None,
    terminal_inflation: Number | None = None,
    terminal_discount_time: TerminalDiscountTime | None = None,
    base_flow: Number | None = None,
    net_financial_position: Mapping[str, Any] | None = None,
    non_operating_assets: Iterable[Mapping[str, Any]] | None = None,
    debt_like_items: Iterable[Mapping[str, Any]] | None = None,
    non_controlling_interests: Iterable[Mapping[str, Any]] | None = None,
    notional_tax_rate: Number | None = None,
    negative_ebit: NegativeEbit | None = None,
) -> Result:
    """Enterprise DCF (``dcf.fcff`` version 3): value of operations, enterprise value and
    equity value from FCFF discounted at the WACC.

    Give ``wacc`` (one rate, also used for the terminal value) or ``wacc_per_period`` with
    ``terminal_wacc``. Under ``timing="mid_period"`` give ``terminal_discount_time``.
    When ``cash_flow_terms`` and ``discount_rate_terms`` differ, give expected inflation
    (``inflation``, or ``inflation_per_period`` with ``terminal_inflation``): every discount
    rate is converted to the terms of the cash flows by the exact Fisher relation, the cash
    flows never are; with equal terms, inflation only restates the perpetual growth in the
    other terms (``result.value["terms"]``).
    Stages built with :func:`given_lines` take ``notional_tax_rate`` with ``negative_ebit``,
    or ``operating_taxes`` in every line.
    ``terminal`` is built by ``gordon``, ``h_model``, ``value_driver``, ``exit_multiple``,
    ``limited_horizon``, ``normalised_income`` or ``liquidation``.

    From value of operations to equity value: ``non_operating_assets`` (``{"label",
    "value"}``) are added; ``net_financial_position`` (``{"gross_debt", "cash",
    "other_items"}``), ``debt_like_items`` and ``non_controlling_interests`` (each
    built by :func:`bridge_claim`, values zero or more) are subtracted. The measurement basis
    (``book_value``, ``market_value``, ``fair_value``, ``present_value`` or ``other`` with a
    note) is stated by the analyst and reported, never used in the calculation.

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
    return run(
        "dcf.fcff",
        _dcf_payload(
            {
                "stages": stages,
                "cash_flow_terms": cash_flow_terms,
                "discount_rate_terms": discount_rate_terms,
                "timing": timing,
                "terminal": terminal,
            },
            {
                "wacc": wacc,
                "wacc_per_period": wacc_per_period,
                "terminal_wacc": terminal_wacc,
                "inflation": inflation,
                "inflation_per_period": inflation_per_period,
                "terminal_inflation": terminal_inflation,
                "terminal_discount_time": terminal_discount_time,
                "base_flow": base_flow,
                "net_financial_position": net_financial_position,
                "non_operating_assets": non_operating_assets,
                "debt_like_items": debt_like_items,
                "non_controlling_interests": non_controlling_interests,
                "notional_tax_rate": notional_tax_rate,
                "negative_ebit": negative_ebit,
            },
        ),
    )


def fcfe(
    stages: Iterable[Mapping[str, Any]],
    *,
    cash_flow_terms: Terms,
    discount_rate_terms: Terms,
    timing: Timing,
    terminal: Mapping[str, Any],
    cost_of_equity: Number | None = None,
    cost_of_equity_per_period: Sequence[Number] | None = None,
    terminal_cost_of_equity: Number | None = None,
    inflation: Number | None = None,
    inflation_per_period: Sequence[Number] | None = None,
    terminal_inflation: Number | None = None,
    terminal_discount_time: TerminalDiscountTime | None = None,
    base_flow: Number | None = None,
    net_financial_position: Mapping[str, Any] | None = None,
    non_operating_assets: Iterable[Mapping[str, Any]] | None = None,
    debt_like_items: Iterable[Mapping[str, Any]] | None = None,
    non_controlling_interests: Iterable[Mapping[str, Any]] | None = None,
    notional_tax_rate: Number | None = None,
    negative_ebit: NegativeEbit | None = None,
    interest_tax_rate: Number | None = None,
) -> Result:
    """Equity DCF (``dcf.fcfe`` version 1): equity value from FCFE discounted at the cost of
    equity.

    Give ``cost_of_equity`` (one rate, also used for the terminal value) or
    ``cost_of_equity_per_period`` with ``terminal_cost_of_equity``. Stages, terms, inflation,
    timing and ``terminal`` work as in :func:`fcff`; the terminal technique must be on the
    equity basis (``exit_multiple`` on ``earnings`` or ``book_value``, ``normalised_income``
    or ``liquidation`` with ``basis="equity"``); ``value_driver`` is not available on the
    equity basis in version 1. Stages built with :func:`given_lines` need
    ``interest_expense`` and ``net_borrowing`` in every line, and take ``notional_tax_rate``
    with ``negative_ebit``, or ``operating_taxes`` in every line with ``interest_tax_rate``.

    Equity value = present value of the FCFE and of the terminal value +
    ``non_operating_assets`` (for example cash not reflected in the FCFE) -
    ``debt_like_items`` - ``non_controlling_interests``. Debt is already in the FCFE, so a
    ``net_financial_position`` is refused (``FM-E019``); the parameter exists only so that the
    error names the fix. The result has no value of operations and no enterprise value.

    >>> r = fcfe(
    ...     [given([100, 110, 120])],
    ...     cash_flow_terms="nominal",
    ...     discount_rate_terms="nominal",
    ...     cost_of_equity="0.11",
    ...     timing="end_of_period",
    ...     terminal=gordon("0.02", first_flow="last_flow_grown"),
    ...     non_operating_assets=[{"label": "Excess cash", "value": 30}],
    ... )
    >>> r.equity_value
    Decimal('1291.5318020723426128831534237')
    """
    return run(
        "dcf.fcfe",
        _dcf_payload(
            {
                "stages": stages,
                "cash_flow_terms": cash_flow_terms,
                "discount_rate_terms": discount_rate_terms,
                "timing": timing,
                "terminal": terminal,
            },
            {
                "cost_of_equity": cost_of_equity,
                "cost_of_equity_per_period": cost_of_equity_per_period,
                "terminal_cost_of_equity": terminal_cost_of_equity,
                "inflation": inflation,
                "inflation_per_period": inflation_per_period,
                "terminal_inflation": terminal_inflation,
                "terminal_discount_time": terminal_discount_time,
                "base_flow": base_flow,
                "net_financial_position": net_financial_position,
                "non_operating_assets": non_operating_assets,
                "debt_like_items": debt_like_items,
                "non_controlling_interests": non_controlling_interests,
                "notional_tax_rate": notional_tax_rate,
                "negative_ebit": negative_ebit,
                "interest_tax_rate": interest_tax_rate,
            },
        ),
    )


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
    financing lines, ``interest_tax_rate`` is required. Under ``"zero"`` the interest tax
    shield is limited to the taxable income available, rate x min(interest, max(EBIT, 0)).
    Amounts are not discounted.

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
