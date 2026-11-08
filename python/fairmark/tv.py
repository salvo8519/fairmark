"""Terminal value: stable growth, value driver, H-model, exit multiple, finite horizon,
normalised income, liquidation, and a comparison of techniques (SPEC-0006).

Every function returns the terminal value at the horizon N, not discounted to the valuation
date, in ``result.terminal_value``, together with the assumptions it implies. Rates and
growths are decimal fractions per period (``"0.02"`` = 2%). A ``growth_cap`` is checked and
reported in ``result.growth_cap``, never refused.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping
from decimal import Decimal
from typing import Any, Literal

from ._core import Result, run

Number = Decimal | int | str | float
Metric = Literal["ebitda", "ebit", "revenue", "earnings", "book_value"]
Basis = Literal["enterprise", "equity"]


def _put(payload: dict[str, Any], **optional: Any) -> dict[str, Any]:
    payload.update({k: v for k, v in optional.items() if v is not None})
    return payload


def gordon(
    rate: Number,
    growth: Number,
    *,
    next_flow: Number | None = None,
    last_flow: Number | None = None,
    growth_cap: Number | None = None,
) -> Result:
    """Stable growth: CF_{N+1} / (r - g).

    Give exactly one of ``next_flow`` (CF_{N+1}) and ``last_flow`` (CF_N, grown by g).

    >>> gordon("0.08", "0.04", next_flow=50).terminal_value
    Decimal('1250')
    """
    payload = _put(
        {"rate": rate, "growth": growth},
        next_flow=next_flow,
        last_flow=last_flow,
        growth_cap=growth_cap,
    )
    return run("tv.gordon", payload)


def value_driver(
    nopat_next: Number,
    growth: Number,
    ronic: Number,
    rate: Number,
    *,
    growth_cap: Number | None = None,
) -> Result:
    """Growth earned by reinvestment: NOPAT_{N+1} x (1 - g / RONIC) / (r - g).

    Reports ``reinvestment_rate`` (g / RONIC) and ``value_of_growth`` (``creates``, ``none``,
    ``destroys``). With RONIC equal to the rate, growth creates no value.

    >>> value_driver(100, "0.03", "0.10", "0.10").terminal_value
    Decimal('1000')
    """
    payload = _put(
        {"nopat_next": nopat_next, "growth": growth, "ronic": ronic, "rate": rate},
        growth_cap=growth_cap,
    )
    return run("tv.value_driver", payload)


def h_model(
    last_flow: Number,
    short_term_growth: Number,
    long_term_growth: Number,
    half_life: Number,
    rate: Number,
    *,
    growth_cap: Number | None = None,
) -> Result:
    """Growth declining linearly over 2H periods: CF_N x [(1 + g_l) + H (g_s - g_l)] / (r - g_l).

    >>> h_model(100, "0.10", "0.03", 5, "0.09").terminal_value == 2300
    True
    """
    payload = _put(
        {
            "last_flow": last_flow,
            "short_term_growth": short_term_growth,
            "long_term_growth": long_term_growth,
            "half_life": half_life,
            "rate": rate,
        },
        growth_cap=growth_cap,
    )
    return run("tv.h_model", payload)


def exit_multiple(
    multiple: Number,
    metric: Metric,
    metric_value: Number,
    *,
    rate: Number | None = None,
    next_flow: Number | None = None,
    growth_cap: Number | None = None,
) -> Result:
    """A multiple of a metric at N; ``earnings`` (P/E) and ``book_value`` (P/BV) give equity
    values, the others enterprise values. With ``rate`` and ``next_flow``, reports the
    implied growth.

    >>> exit_multiple(8, "ebitda", 125, rate="0.10", next_flow=60).implied_growth
    Decimal('0.04')
    """
    payload = _put(
        {"multiple": multiple, "metric": metric, "metric_value": metric_value},
        rate=rate,
        next_flow=next_flow,
        growth_cap=growth_cap,
    )
    return run("tv.exit_multiple", payload)


def limited_horizon(
    flow: Number,
    rate: Number,
    periods: int,
    *,
    growth: Number | None = None,
    residual_value: Number | None = None,
    growth_cap: Number | None = None,
) -> Result:
    """A flow lasting ``periods`` more periods (constant, or growing), plus a residual value
    at N + n, valued at N.

    >>> limited_horizon(100, "0.10", 2, residual_value=121).residual_value_discounted == 100
    True
    """
    payload = _put(
        {"flow": flow, "rate": rate, "periods": periods},
        growth=growth,
        residual_value=residual_value,
        growth_cap=growth_cap,
    )
    return run("tv.limited_horizon", payload)


def normalised_income(
    income: Number,
    capitalisation_rate: Number,
    basis: Basis,
    *,
    growth: Number | None = None,
    growth_cap: Number | None = None,
) -> Result:
    """A normalised income capitalised: R / i, or R / (i - g) with a growth.

    ``basis`` is required: ``"enterprise"`` for an operating income, ``"equity"`` for a net
    income.

    >>> normalised_income(120, "0.08", "equity").terminal_value
    Decimal('1500')
    """
    payload = _put(
        {"income": income, "capitalisation_rate": capitalisation_rate, "basis": basis},
        growth=growth,
        growth_cap=growth_cap,
    )
    return run("tv.normalised_income", payload)


Lines = Mapping[str, Number] | Iterable[tuple[str, Number]]


def liquidation(
    assets: Lines,
    liabilities: Lines,
    liquidation_costs: Lines,
    tax_on_gains: Number,
    basis: Basis,
) -> Result:
    """Net liquidation value: realisable assets less liabilities, costs and tax on gains.

    Each group is a mapping from a label to an amount, for example ``{"Land": 800}``, or a list
    of ``(label, amount)`` pairs, which allows the same label twice. At least one asset is
    required; a negative net value is returned as is. ``basis`` is required: ``"equity"`` when
    the liabilities include financial debt, ``"enterprise"`` when they exclude it.

    >>> r = liquidation({"Land": 800}, {"Bank loan": 600}, {"Legal fees": 40}, 30, "equity")
    >>> r.terminal_value
    Decimal('130')
    >>> liquidation([("Lot", 500), ("Lot", 300)], [], [], 0, "enterprise").terminal_value
    Decimal('800')
    """

    def lines(items: Lines) -> list[dict[str, Any]]:
        pairs = items.items() if isinstance(items, Mapping) else items
        return [{"label": label, "value": value} for label, value in pairs]

    payload = {
        "assets": lines(assets),
        "liabilities": lines(liabilities),
        "liquidation_costs": lines(liquidation_costs),
        "tax_on_gains": tax_on_gains,
        "basis": basis,
    }
    return run("tv.liquidation", payload)


def compare(
    rate: Number,
    basis: Basis,
    techniques: Iterable[Mapping[str, Mapping[str, Any]]],
    *,
    growth_cap: Number | None = None,
) -> Result:
    """Several techniques on one rate, growth cap and basis, side by side.

    Each technique is ``{"tv.<name>": {terms}}``, the terms being the method's own inputs
    without ``rate`` and ``growth_cap``. ``result.results`` holds one row per technique.

    >>> r = compare("0.10", "enterprise", [
    ...     {"tv.gordon": {"growth": "0.02", "last_flow": 100}},
    ...     {"tv.exit_multiple": {"multiple": 8, "metric": "ebitda", "metric_value": 160}},
    ... ])
    >>> [row["terminal_value"] for row in r.results]
    [Decimal('1275'), Decimal('1280')]
    """
    payload = _put(
        {
            "rate": rate,
            "basis": basis,
            "techniques": [{k: dict(v) for k, v in t.items()} for t in techniques],
        },
        growth_cap=growth_cap,
    )
    return run("tv.compare", payload)
