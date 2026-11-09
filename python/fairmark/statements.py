"""Financial statements reorganised into an operating and financing view (SPEC-0005).

``reorganise`` takes one or more financial years, oldest first, each in the ``statutory`` form
(the aggregates of art. 2424 and 2425 of the Italian Civil Code, ordinary or abridged layout)
or in the ``reclassified`` form (totals already reorganised), and returns per year the
reorganised balance sheet (invested capital, non-operating assets, equity, quasi-equity, the
net financial position with its composition and maturities, debt-like items, current and
non-current amounts for the ratios) and the reorganised income statement (value of production,
value added, EBITDA, operating profit, investing result, profit before financing and taxes,
non-recurring and financing results, profit before tax, income taxes and net income, with the
reconciliations of the two IFRS 18 aligned subtotals with A - B).

Every input is optional. An output is computed when its inputs are given; otherwise it is
``None`` and listed in ``not_computed`` with the JSON pointers of the missing inputs, so an
agent can ask only for those. Totals given are checked exactly against their components
(``FM-E026`` when they differ). Amounts are in the currency units of the statements, as
strings or ``Decimal``.

The outputs feed other methods unchanged: ``sources.net_financial_position.position``,
``non_operating_assets`` and ``sources.debt_like_items`` go to ``fm.dcf.fcff``;
``current_non_current`` to ``fm.ratios.liquidity`` and ``fm.ratios.capital_structure``;
``income_statement.operating_profit`` and ``depreciation_and_amortisation`` to
``fm.dcf.free_cash_flows``; ``revenue``, ``ebitda`` (with ``ebitda_definition``) and
``net_income`` to ``fm.ratios.profitability``; ``profit_before_financing_and_taxes`` and
``interest_and_other_financial_charges`` to ``fm.ratios.debt_service``.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
from typing import Any

from ._core import Result, run


def reorganise(
    years: Sequence[Mapping[str, Any]],
    treatments: Mapping[str, Any] | None = None,
    adjustments: Sequence[Mapping[str, Any]] | None = None,
) -> Result:
    """Reorganise one or more financial years (``statements.reorganise``).

    ``years``: oldest first, each ``{"label": ..., "date": ..., "statutory": {...}}`` or
    ``{"reclassified": {...}}``; label and date are optional. ``treatments`` (stated once):
    ``financial_fixed_assets`` (``operating``, ``non_operating`` or ``financial_asset``),
    ``shareholder_loans`` (``financial_debt`` or ``quasi_equity``) and
    ``derivative_liabilities``, each needed only when its aggregate is given and not zero, and
    ``write_downs_of_receivables`` and ``write_downs_of_fixed_assets`` (``operating`` by
    default, or ``investing``).
    ``adjustments``: labelled moves of Appendix C, each with ``kind``, ``label``, ``reason``
    and one amount per year (``None`` where it does not apply). No value is defaulted here
    beyond the engine's own defaults, listed in ``result.meta["defaults_applied"]``.

    >>> r = reorganise(
    ...     [{"label": "2024", "statutory": {"balance_sheet": {
    ...         "subscribed_capital_unpaid": "0",
    ...         "intangible_assets": {"total": "100"}, "tangible_assets": {"total": "900"},
    ...         "financial_fixed_assets": "0", "inventories": "300",
    ...         "receivables": {"total": "500", "beyond_one_year": "0"},
    ...         "current_financial_assets": "0", "cash": "200",
    ...         "prepayments_and_accrued_income": {"total": "0"},
    ...         "equity": {"total": "800"}, "provisions": {"total": "0"},
    ...         "employee_severance_indemnity": {"total": "100"},
    ...         "payables": {"total": "1100", "beyond_one_year": "400",
    ...             "financial_debt": {"total": "600", "beyond_one_year": "400"},
    ...             "shareholder_loans": {"total": "0", "beyond_one_year": "0"}},
    ...         "accruals_and_deferred_income": {"total": "0"}}}}])
    >>> bs = r.years[0]["balance_sheet"]
    >>> bs["operating"]["invested_capital"], bs["sources"]["net_financial_position"]["total"]
    (Decimal('1200'), Decimal('400'))
    >>> bs["total_funds_invested"] == bs["sources"]["total_sources"]
    True
    >>> r = reorganise([{"statutory": {"income_statement": {
    ...     "revenue": "1000", "change_in_product_inventories": "0", "own_work_capitalised": "0",
    ...     "other_revenue_and_income": "0", "purchases": "400",
    ...     "change_in_raw_material_inventories": "0", "services": "200",
    ...     "use_of_third_party_assets": "0", "other_operating_expenses": "0",
    ...     "personnel_costs": "250"}}}])
    >>> r.years[0]["income_statement"]["ebitda"]
    Decimal('150')
    """
    payload: dict[str, Any] = {"years": [dict(y) for y in years]}
    if treatments is not None:
        payload["treatments"] = dict(treatments)
    if adjustments is not None:
        payload["adjustments"] = [dict(a) for a in adjustments]
    return run("statements.reorganise", payload)
