"""The Python helper of statements.reorganise (SPEC-0005): its docstring example runs, and the
outputs reach the methods that take them through the Python API (AC9)."""

from __future__ import annotations

import doctest
from decimal import Decimal

import fairmark as fm
from fairmark import statements


def test_the_docstring_example_runs() -> None:
    failures, tried = doctest.testmod(statements, raise_on_error=False)
    assert tried > 0
    assert failures == 0


def test_the_net_financial_position_reaches_dcf_fcff_unchanged() -> None:
    r = statements.reorganise(
        [
            {
                "statutory": {
                    "balance_sheet": {
                        "cash": "200",
                        "current_financial_assets": "0",
                        "financial_fixed_assets": "0",
                        "payables": {
                            "total": "600",
                            "financial_debt": {"total": "600"},
                            "shareholder_loans": {"total": "0"},
                        },
                    }
                }
            }
        ]
    )
    position = r.years[0]["balance_sheet"]["sources"]["net_financial_position"]["position"]
    v = fm.dcf.fcff([fm.dcf.given(["100", "100"])], wacc="0.10", net_financial_position=position)
    assert v.bridge["net_financial_position"] == Decimal("400")
    nc = r.years[0]["not_computed"]["balance_sheet"]
    assert any(e["output"].endswith("/invested_capital") for e in nc)


def test_a_label_that_looks_like_a_number_stays_text() -> None:
    """Review of PR 1 (item 6): labels name things; "2024" is not the amount 2024."""
    r = statements.reorganise(
        [{"label": "2024", "statutory": {"balance_sheet": {"cash": "100"}}}],
        adjustments=[
            {
                "kind": "operating_cash",
                "label": "1000",
                "reason": "minimum cash",
                "amounts": ["10"],
            }
        ],
    )
    assert r.years[0]["label"] == "2024"
    assert r.years[0]["adjustments_applied"][0]["label"] == "1000"
    assert r.years[0]["adjustments_applied"][0]["amount"] == Decimal("10")
