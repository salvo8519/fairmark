"""Cost of capital: CAPM with explicit premia, beta relevering, peer betas, cost of debt, WACC.

All rates are annual decimal fractions (``"0.05"`` = 5%). Optional premia default to zero and
every default is listed in ``result.conventions``.
"""

from __future__ import annotations

from collections.abc import Iterable, Mapping
from decimal import Decimal
from typing import Any, Literal

from ._core import Result, run

Number = Decimal | int | str | float
Formula = Literal["hamada", "harris_pringle"]


def _put(payload: dict[str, Any], **optional: Any) -> dict[str, Any]:
    payload.update({k: v for k, v in optional.items() if v is not None})
    return payload


def cost_of_equity(
    risk_free_rate: Number,
    equity_risk_premium: Number,
    *,
    levered_beta: Number | None = None,
    relever: Mapping[str, Any] | None = None,
    country_risk_premium: Number | None = None,
    country_exposure: Number | None = None,
    size_premium: Number | None = None,
    specific_risk_premium: Number | None = None,
    specific_risk_justification: str | None = None,
) -> Result:
    """Cost of equity by CAPM: rf + beta * ERP + CRP * lambda + size + specific premium.

    Give either ``levered_beta`` or ``relever`` (a mapping with ``unlevered_beta``,
    ``debt_to_equity``, ``tax_rate``, ``formula`` and optionally ``debt_beta``). A non-zero
    specific premium requires a written justification.

    >>> cost_of_equity("0.03", "0.05", levered_beta="1.2").cost_of_equity
    Decimal('0.090')
    """
    payload = _put(
        {"risk_free_rate": risk_free_rate, "equity_risk_premium": equity_risk_premium},
        levered_beta=levered_beta,
        relever=dict(relever) if relever is not None else None,
        country_risk_premium=country_risk_premium,
        country_exposure=country_exposure,
        size_premium=size_premium,
        specific_risk_premium=specific_risk_premium,
        specific_risk_justification=specific_risk_justification,
    )
    return run("coc.cost_of_equity", payload)


def beta_relever(
    beta: Number,
    *,
    direction: Literal["relever", "unlever"],
    debt_to_equity: Number,
    tax_rate: Number,
    formula: Formula,
    debt_beta: Number | None = None,
) -> Result:
    """Relever an unlevered beta at a target D/E, or unlever a levered beta."""
    payload = _put(
        {
            "beta": beta,
            "direction": direction,
            "debt_to_equity": debt_to_equity,
            "tax_rate": tax_rate,
            "formula": formula,
        },
        debt_beta=debt_beta,
    )
    return run("coc.beta_relever", payload)


def beta_peers(
    peers: Iterable[Mapping[str, Any]],
    *,
    formula: Formula,
    statistic: Literal["mean", "median"],
    target_debt_to_equity: Number,
    target_tax_rate: Number,
    debt_beta: Number | None = None,
) -> Result:
    """Unlever comparable betas, take the mean or median, relever to the target structure.

    Each peer is a mapping with ``name``, ``levered_beta``, ``debt_to_equity`` and ``tax_rate``.
    """
    payload = _put(
        {
            "peers": [dict(p) for p in peers],
            "formula": formula,
            "statistic": statistic,
            "target_debt_to_equity": target_debt_to_equity,
            "target_tax_rate": target_tax_rate,
        },
        debt_beta=debt_beta,
    )
    return run("coc.beta_peers", payload)


def cost_of_debt(
    tax_rate: Number,
    *,
    pre_tax_cost_of_debt: Number | None = None,
    risk_free_rate: Number | None = None,
    credit_spread: Number | None = None,
) -> Result:
    """Pre-tax and after-tax cost of debt; give ``pre_tax_cost_of_debt`` or rate plus spread."""
    payload = _put(
        {"tax_rate": tax_rate},
        pre_tax_cost_of_debt=pre_tax_cost_of_debt,
        risk_free_rate=risk_free_rate,
        credit_spread=credit_spread,
    )
    return run("coc.cost_of_debt", payload)


def wacc(
    cost_of_equity: Number,
    pre_tax_cost_of_debt: Number,
    tax_rate: Number,
    *,
    debt_to_equity: Number | None = None,
    debt_weight: Number | None = None,
) -> Result:
    """WACC = E/V * ke + D/V * kd * (1 - t); give ``debt_to_equity`` or ``debt_weight``.

    >>> wacc("0.10", "0.06", "0.25", debt_weight="0.3").wacc
    Decimal('0.08350')
    """
    payload = _put(
        {
            "cost_of_equity": cost_of_equity,
            "pre_tax_cost_of_debt": pre_tax_cost_of_debt,
            "tax_rate": tax_rate,
        },
        debt_to_equity=debt_to_equity,
        debt_weight=debt_weight,
    )
    return run("coc.wacc", payload)
