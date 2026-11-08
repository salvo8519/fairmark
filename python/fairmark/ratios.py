"""Financial ratios organised by the question the analyst asks.

Every group takes the flows of the period in ``income_statement`` (where it needs flows), the
closing balance sheet in ``closing`` and, for average balances, the opening balance sheet in
``opening``. Amounts are in currency units; ratios come back as decimal fractions (``0.13`` =
13%), multiples or days, as each field states. A ratio that cannot be computed has ``value``
``None`` and the reason in ``not_defined``; the other ratios of the group are still computed.

The net financial position is given with its composition::

    {"gross_debt": "2200", "cash": "300",
     "other_items": [{"name": "lease liabilities", "amount": "150"}]}
"""

from __future__ import annotations

from collections.abc import Mapping
from typing import Any, Literal

from ._core import Result, run

Balances = Literal["closing", "average"]
OperatingReturnBase = Literal["invested_capital", "total_assets"]


def _payload(**fields: Any) -> dict[str, Any]:
    return {k: dict(v) if isinstance(v, Mapping) else v for k, v in fields.items() if v is not None}


def profitability(
    *,
    balances: Balances,
    operating_return_base: OperatingReturnBase,
    ebitda_definition: str,
    income_statement: Mapping[str, Any],
    closing: Mapping[str, Any],
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """ROE, ROI, ROS, EBITDA and EBIT margins, capital turnover and the three-factor ROE.

    ``income_statement``: ``revenue``, ``ebitda``, ``ebit``, ``net_income``. Balance sheets:
    ``equity``, ``total_assets`` and, when ``operating_return_base`` is ``invested_capital``,
    ``invested_capital``. ``ebitda_definition`` states in words how EBITDA was measured.

    >>> r = profitability(balances="closing", operating_return_base="invested_capital",
    ...     ebitda_definition="reported", closing={"equity": 2000, "total_assets": 6000,
    ...     "invested_capital": 4000}, income_statement={"revenue": 5000, "ebitda": 800,
    ...     "ebit": 500, "net_income": 260})
    >>> r.roe["value"]
    Decimal('0.13')
    """
    return run(
        "ratios.profitability",
        _payload(
            balances=balances,
            operating_return_base=operating_return_base,
            ebitda_definition=ebitda_definition,
            income_statement=income_statement,
            closing=closing,
            opening=opening,
        ),
    )


def capital_structure(
    *,
    balances: Balances,
    closing: Mapping[str, Any],
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """D/E, NFP/E, equity ratio, primary and secondary structure margins and coverage.

    Balance sheets: ``equity``, ``total_assets``, ``fixed_assets``, ``non_current_liabilities``
    and ``net_financial_position`` (``gross_debt``, ``cash``, optional ``other_items``).
    """
    return run(
        "ratios.capital_structure",
        _payload(balances=balances, closing=closing, opening=opening),
    )


def liquidity(
    *,
    balances: Balances,
    closing: Mapping[str, Any],
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """Current, quick and cash ratios, net working capital and treasury margin.

    Balance sheets: ``current_assets``, ``inventories``, ``cash``, ``current_liabilities``.
    """
    return run("ratios.liquidity", _payload(balances=balances, closing=closing, opening=opening))


def working_capital(
    *,
    balances: Balances,
    day_basis: Literal[360, 365, "360", "365"],
    income_statement: Mapping[str, Any],
    closing: Mapping[str, Any],
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """Days sales, inventory and payable outstanding and the cash conversion cycle.

    ``income_statement``: ``revenue``, ``cost_of_sales``, ``purchases``. Balance sheets:
    ``trade_receivables``, ``inventories``, ``trade_payables``. ``day_basis`` is 360 or 365.
    """
    return run(
        "ratios.working_capital",
        _payload(
            balances=balances,
            day_basis=str(day_basis),
            income_statement=income_statement,
            closing=closing,
            opening=opening,
        ),
    )


def debt_service(
    *,
    balances: Balances,
    ebitda_definition: str,
    interest_definition: str,
    income_statement: Mapping[str, Any],
    closing: Mapping[str, Any],
    debt_schedule: Mapping[str, Any],
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """Interest cover, NFP / EBITDA, DSCR per period and LLCR.

    ``income_statement``: ``ebitda``, ``ebit``, ``interest``; ``ebitda_definition`` and
    ``interest_definition`` state in words how EBITDA and interest were measured. Balance sheets:
    ``net_financial_position``. ``debt_schedule``: ``discount_rate`` (per period),
    ``debt_outstanding`` (at the start of the first period) and ``periods``, a list of
    ``{"cfads": ..., "debt_service": ...}`` up to the final maturity.
    """
    return run(
        "ratios.debt_service",
        _payload(
            balances=balances,
            ebitda_definition=ebitda_definition,
            interest_definition=interest_definition,
            income_statement=income_statement,
            closing=closing,
            opening=opening,
            debt_schedule=debt_schedule,
        ),
    )
