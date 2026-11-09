"""Financial ratios organised by the question the analyst asks.

Every group takes the flows of the period in ``income_statement`` (where it needs flows), the
closing balance sheet in ``closing`` and, for average balances, the opening balance sheet in
``opening``. Amounts are in currency units; ratios come back as decimal fractions (``0.13`` =
13%), multiples or days, as each field states.

Every amount is optional (version 2): give what you have. A ratio whose amounts are not all
given has ``value`` ``None`` and is listed in ``not_computed`` with the JSON pointers of the
inputs it lacks; a missing amount is never taken as zero. A ratio that is not meaningful (a zero
denominator, negative equity) has ``value`` ``None`` and the reason in ``not_defined``. The
other ratios of the group are still computed. Only the method choices are required:
``balances``, and ``operating_return_base`` or ``day_basis`` where the group uses them. At
least one output must have every input it needs: otherwise the engine raises ``FM-E027``,
whose message lists the sets of inputs that would be enough.

``covenants`` tests covenants on the definitions of the loan agreement, with an explicit
outcome per covenant.

The net financial position is given with its composition::

    {"gross_debt": "2200", "cash": "300",
     "other_items": [{"name": "lease liabilities", "amount": "150"}]}
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
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
    ebitda_definition: str | None = None,
    income_statement: Mapping[str, Any] | None = None,
    closing: Mapping[str, Any] | None = None,
    opening: Mapping[str, Any] | None = None,
    leverage: Mapping[str, Any] | None = None,
) -> Result:
    """ROE, ROI, ROA, ROS, margins, capital turnover and the ROE decompositions.

    ``income_statement``: ``revenue``, ``ebitda``, ``ebit``, ``net_income``. Balance sheets:
    ``equity``, ``total_assets`` and, when ``operating_return_base`` is ``invested_capital``,
    ``invested_capital``. ``ebitda_definition`` states in words how EBITDA was measured; the
    EBITDA margin needs it. Every amount is optional.

    ``leverage`` (optional) enables the leverage form ROE = [ROI + (ROI - i) x D/E] x (1 - t):
    ``debt_basis`` (``"gross_financial_debt"`` or ``"net_financial_position"``),
    ``financial_charges``, ``taxes``, ``pre_tax_income``; D itself is ``debt`` in the balance
    sheets. Without it the leverage form is not defined.

    >>> r = profitability(balances="closing", operating_return_base="invested_capital",
    ...     ebitda_definition="reported", closing={"equity": 2000, "total_assets": 6000,
    ...     "invested_capital": 4000}, income_statement={"revenue": 5000, "ebitda": 800,
    ...     "ebit": 500, "net_income": 260})
    >>> r.roe["value"]
    Decimal('0.13')

    With net income and equity only, ROE is computed and the rest is listed as not computed:

    >>> r = profitability(balances="closing", operating_return_base="total_assets",
    ...     income_statement={"net_income": 260}, closing={"equity": 2000})
    >>> r.roe["value"], r.roa["value"], r.not_computed[1]["output"]
    (Decimal('0.13'), None, '/roa')
    >>> r.not_computed[1]["missing"]
    ['/closing/total_assets', '/income_statement/ebit']
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
            leverage=leverage,
        ),
    )


def capital_structure(
    *,
    balances: Balances,
    closing: Mapping[str, Any] | None = None,
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """D/E, NFP/E, equity ratio, primary and secondary structure margins and coverage.

    Balance sheets: ``equity``, ``total_assets``, ``fixed_assets``, ``non_current_liabilities``
    and ``net_financial_position`` (``gross_debt``, ``cash``, optional ``other_items``). Every
    amount is optional.
    """
    return run(
        "ratios.capital_structure",
        _payload(balances=balances, closing=closing, opening=opening),
    )


def liquidity(
    *,
    balances: Balances,
    closing: Mapping[str, Any] | None = None,
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """Current, quick and cash ratios, net working capital and treasury margin.

    Balance sheets: ``current_assets``, ``inventories``, ``cash``, ``current_liabilities``.
    Every amount is optional.
    """
    return run("ratios.liquidity", _payload(balances=balances, closing=closing, opening=opening))


def working_capital(
    *,
    balances: Balances,
    day_basis: Literal[360, 365, "360", "365"],
    income_statement: Mapping[str, Any] | None = None,
    closing: Mapping[str, Any] | None = None,
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """Days sales, inventory and payable outstanding and the cash conversion cycle.

    ``income_statement``: ``revenue``, ``cost_of_sales``, ``purchases``. Balance sheets:
    ``trade_receivables``, ``inventories``, ``trade_payables``. ``day_basis`` is 360 or 365.
    Every amount is optional.
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
    ebitda_definition: str | None = None,
    interest_definition: str | None = None,
    income_statement: Mapping[str, Any] | None = None,
    closing: Mapping[str, Any] | None = None,
    debt_schedule: Mapping[str, Any] | None = None,
    opening: Mapping[str, Any] | None = None,
) -> Result:
    """Interest cover, NFP / EBITDA, DSCR per period and LLCR.

    ``income_statement``: ``ebitda``, ``ebit``, ``interest``; ``ebitda_definition`` and
    ``interest_definition`` state in words how EBITDA and interest were measured; the ratios on
    EBITDA and the interest covers need them. Every amount is optional. Balance sheets:
    ``net_financial_position``. ``debt_schedule`` (optional; without it DSCR and LLCR are not
    defined): ``discount_rate`` (cost of debt per period), ``debt_outstanding`` (at the start of
    the first period), optional ``reserve_balance`` (added undiscounted to the LLCR numerator)
    and ``periods``, a list of ``{"cfads": ..., "debt_service": ...}`` up to the final maturity.
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


def covenants(*, covenants: Sequence[Mapping[str, Any]]) -> Result:
    """Test covenants on the numerator and denominator defined by the loan agreement.

    Each covenant: ``name``; ``test_date`` (``"YYYY-MM-DD"``, the amounts refer to the period
    ending on it); ``numerator`` and ``denominator``, each ``{"definition": <the words of the
    agreement>, "amount": ...}``; ``operator``, required: ``"at_most"`` (<=), ``"less_than"``
    (<), ``"at_least"`` (>=) or ``"greater_than"`` (>); ``threshold`` in the unit of the ratio
    (a multiple, or a decimal fraction such as ``"0.30"`` for 30%);
    ``if_denominator_not_positive``, required, no default: ``"breached"``, ``"passed"`` or
    ``"not_tested"``, the outcome the agreement gives when the denominator is zero or
    negative; ``rounding``, optional: ``{"decimal_places": 0 to 6, "mode": "half_up" or
    "half_even"}``, the rounding of the tested value the agreement applies before the
    comparison.

    Each result has ``outcome`` (``"passed"``, ``"breached"`` or ``"not_tested"``), decided
    exactly, ``outcome_basis`` (``"tested_value"`` or ``"if_denominator_not_positive"``),
    ``value`` (the value compared, rounded only when ``rounding`` is given),
    ``value_unrounded`` (with ``rounding``), ``headroom`` (positive on the compliant side,
    negative on the breach side), ``headroom_relative`` (headroom / threshold),
    ``not_tested_code`` (``"denominator_not_positive"``, ``"overflow"`` or
    ``"missing_input"``) and ``not_tested_reason``. An ``amount`` may be omitted: the covenant is
    then not tested (``"missing_input"``) and listed in ``not_computed``, unless a denominator
    given zero or negative decides it by its rule.

    >>> r = covenants(covenants=[{"name": "net leverage", "test_date": "2026-12-31",
    ...     "numerator": {"definition": "net debt", "amount": 3504},
    ...     "denominator": {"definition": "EBITDA, last 12 months", "amount": 1000},
    ...     "operator": "at_most", "threshold": "3.50",
    ...     "if_denominator_not_positive": "breached",
    ...     "rounding": {"decimal_places": 2, "mode": "half_up"}}])
    >>> from decimal import Decimal
    >>> c = r.covenants[0]
    >>> c["outcome"], c["value"] == Decimal("3.50"), c["value_unrounded"] == Decimal("3.504")
    ('passed', True, True)
    """
    return run("ratios.covenants", {"covenants": [dict(c) for c in covenants]})
