"""SPEC-0005 against the independent 50-digit reference (scripts/reference_statements.py).

Seeded random statements, deterministic and dependency-free, in the statutory form (with random
treatments, overrides and adjustments of every kind, the lease double entry included, and random
gaps in the inputs) and in the reclassified form: every balance sheet and income statement
output, every reconciliation, every current and non-current amount, every check status, every
`not_computed` entry and every default must equal the reference. The golden cases pin selected
companies; this test covers the space between them.
"""

from __future__ import annotations

import copy
import importlib.util
import random
import sys
from decimal import Decimal as D
from pathlib import Path
from typing import Any

import pytest

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "ref_statements", ROOT / "scripts/reference_statements.py"
)
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
sys.modules["ref_statements"] = ref  # dataclasses resolve annotations through sys.modules
_spec.loader.exec_module(ref)

RNG = random.Random(20261108)
OPTIONAL = [
    "subscribed_capital_unpaid",
    "intangible_assets",
    "tangible_assets",
    "financial_fixed_assets",
    "inventories",
    "receivables",
    "current_financial_assets",
    "cash",
    "prepayments_and_accrued_income",
    "provisions",
    "employee_severance_indemnity",
    "payables",
    "accruals_and_deferred_income",
]


def amount(hi: int = 2_000_000) -> int:
    return RNG.randrange(0, hi)


def statutory_year() -> dict[str, Any]:
    """A balanced statutory balance sheet; some aggregates and parts dropped at random."""
    b1, b2, b3, c1, c3, c4 = (amount() for _ in range(6))
    c2, d, a = amount(), amount(100_000), RNG.choice([0, 0, amount(50_000)])
    a_within = RNG.choice([0, amount(10_000)])
    c2 += a_within
    prov, tfr, fd, sl, other, e = (amount(800_000) for _ in range(6))
    deriv = RNG.choice([0, prov // 3])
    e_within = RNG.choice([0, amount(20_000)])
    payables = fd + sl + other + e_within
    assets = a + b1 + b2 + b3 + c1 + c2 + c3 + c4 + d
    equity = assets - prov - tfr - payables - e
    bs: dict[str, Any] = {
        "subscribed_capital_unpaid": str(a),
        "intangible_assets": {"gross": str(b1 + 1000), "accumulated": "1000"},
        "tangible_assets": {"total": str(b2)},
        "financial_fixed_assets": str(b3),
        "inventories": str(c1),
        "receivables": {
            "total": str(c2),
            "beyond_one_year": str(c2 // 4),
            "trade": str(c2 // 2),
            "subscribed_capital_unpaid": str(a_within),
        },
        "current_financial_assets": str(c3),
        "cash": str(c4),
        "prepayments_and_accrued_income": {"total": str(d), "beyond_one_year": str(d // 5)},
        "fixed_assets": str(b1 + b2 + b3),
        "current_assets": str(c1 + c2 + c3 + c4),
        "total_assets": str(assets),
        "equity": {"total": str(equity)},
        "provisions": {"total": str(prov), "of_which_derivative_liabilities": str(deriv)},
        "employee_severance_indemnity": {"total": str(tfr), "beyond_one_year": str(tfr // 2)},
        "payables": {
            "total": str(payables),
            "beyond_one_year": str(fd // 2),
            "financial_debt": {"total": str(fd), "beyond_one_year": str(fd // 2)},
            "shareholder_loans": {"total": str(sl), "beyond_one_year": str(sl // 3)},
            "trade": str(other // 2),
            "accruals_and_deferred_income": str(e_within),
        },
        "accruals_and_deferred_income": {"total": str(e)},
        "total_liabilities_and_equity": str(assets),
    }
    for key in OPTIONAL:
        if RNG.random() < 0.08:
            del bs[key]
            # The totals stay given half of the time: the R15 amounts read them, and an item
            # taken as zero (D1, D2) then meets a given total (FM-E026 or not checked).
            if RNG.random() < 0.5:
                for total in ("fixed_assets", "current_assets", "total_assets"):
                    bs.pop(total, None)
                bs.pop("total_liabilities_and_equity", None)
    for key, sub in (("payables", "shareholder_loans"), ("receivables", "beyond_one_year")):
        if key in bs and RNG.random() < 0.08:
            del bs[key][sub]
    return {"statutory": {"balance_sheet": bs, "income_statement": statutory_income_statement()}}


IS_TOTALS = [
    "value_of_production",
    "costs_of_production",
    "difference_between_value_and_costs_of_production",
    "financial_income_and_charges",
    "value_adjustments_of_financial_assets",
    "profit_before_tax",
]


def statutory_income_statement() -> dict[str, Any]:
    """An income statement by nature whose totals add up; B10 in the ordinary or the abridged
    reading, signed items of both signs; some items and totals dropped at random."""
    a1, a4, a5 = amount(), amount(50_000), amount(80_000)
    a23 = amount(100_000) - 50_000
    b6, b7, b8, b9 = (amount() for _ in range(4))
    ab, c10, d10 = amount(200_000), RNG.choice([0, amount(30_000)]), amount(20_000)
    b11 = amount(60_000) - 30_000
    b12, b13, b14 = amount(20_000), amount(20_000), amount(50_000)
    c15, c16, c17 = amount(20_000), amount(10_000), amount(90_000)
    c17b = amount(6_000) - 3_000
    d18, d19 = amount(5_000), amount(15_000)
    taxes = amount(200_000) - 40_000
    total_a = a1 + a23 + a4 + a5
    total_b = b6 + b7 + b8 + b9 + ab + c10 + d10 + b11 + b12 + b13 + b14
    total_c = c15 + c16 - c17 + c17b
    total_d = d18 - d19
    pbt = total_a - total_b + total_c + total_d
    b10: dict[str, Any] = {"write_downs_of_receivables": str(d10)}
    reading = RNG.random()
    if reading < 0.5:
        b10 |= {"depreciation_and_amortisation": str(ab), "write_downs_of_fixed_assets": str(c10)}
        if RNG.random() < 0.5:
            b10["total"] = str(ab + c10 + d10)
    elif reading < 0.8 or c10 != 0:
        # Abridged layout: B10 a to c as one amount with its B10 c part (Decision 8).
        b10 |= {
            "value_adjustments_of_fixed_assets": str(ab + c10),
            "write_downs_of_fixed_assets": str(c10),
        }
    else:
        b10["value_adjustments_of_fixed_assets"] = str(ab)
    inc: dict[str, Any] = {
        "revenue": str(a1),
        "change_in_product_inventories": str(a23),
        "own_work_capitalised": str(a4),
        "other_revenue_and_income": str(a5),
        "purchases": str(b6),
        "services": str(b7),
        "use_of_third_party_assets": str(b8),
        "personnel_costs": str(b9),
        "depreciation_and_write_downs": b10,
        "change_in_raw_material_inventories": str(b11),
        "provisions_for_risks": str(b12),
        "other_provisions": str(b13),
        "other_operating_expenses": str(b14),
        "income_from_equity_investments": str(c15),
        "other_financial_income": str(c16),
        "interest_and_other_financial_charges": str(c17),
        "exchange_gains_and_losses": str(c17b),
        "revaluations_of_financial_assets": str(d18),
        "write_downs_of_financial_assets": str(d19),
        "value_of_production": str(total_a),
        "costs_of_production": str(total_b),
        "difference_between_value_and_costs_of_production": str(total_a - total_b),
        "financial_income_and_charges": str(total_c),
        "value_adjustments_of_financial_assets": str(total_d),
        "profit_before_tax": str(pbt),
        "income_taxes": str(taxes),
        "profit_or_loss_for_the_year": str(pbt - taxes),
    }
    for key in IS_TOTALS:
        if RNG.random() < 0.4:
            del inc[key]
    for key in list(inc):
        if key not in IS_TOTALS and RNG.random() < 0.04:
            del inc[key]
    return inc


def adjustments(n: int, years: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Adjustments of every kind, each within a small share of its source."""
    kinds = [
        ("non_operating_asset", {"aggregate": "tangible_assets"}),
        ("financial_receivable", {"maturity": RNG.choice(["within_one_year", "beyond_one_year"])}),
        ("operating_cash", {}),
        ("accrued_interest", {}),
        ("tax_instalments", {"maturity": "beyond_one_year"}),
        ("non_current_trade_payables", {}),
        ("debt_like_provision", {"aggregate": "employee_severance_indemnity"}),
        ("dividends_declared", {}),
        (
            "investing",
            {"aggregate": RNG.choice(["other_revenue_and_income", "other_operating_expenses"])},
        ),
        (
            "investing",
            {"aggregate": "other_operating_expenses", "related_non_operating_asset": "Land"},
        ),
        ("non_recurring", {"aggregate": RNG.choice(NON_RECURRING)}),
        ("non_recurring", {"aggregate": RNG.choice(NON_RECURRING)}),
        ("lease", {}),
    ]
    out = []
    for kind, extra in RNG.sample(kinds, RNG.randrange(0, len(kinds) + 1)):
        amounts: list[Any] = [
            None if RNG.random() < 0.3 or "reclassified" in y else str(RNG.randrange(0, 5))
            for y in years
        ]
        if kind == "lease":
            amounts = [None if a is None else lease_amounts() for a in amounts]
        out.append(
            {"kind": kind, "label": f"{kind} item", "reason": "test", **extra, "amounts": amounts}
        )
    return out


NON_RECURRING = [
    "revenue",
    "other_revenue_and_income",
    "purchases",
    "services",
    "use_of_third_party_assets",
    "personnel_costs",
    "depreciation_and_amortisation",
    "write_downs_of_fixed_assets",
    "write_downs_of_receivables",
    "provisions_for_risks",
    "other_provisions",
    "other_operating_expenses",
    "income_from_equity_investments",
    "other_financial_income",
    "interest_and_other_financial_charges",
    "exchange_gains_and_losses",
]


def lease_amounts() -> dict[str, str]:
    liability = amount(300_000)
    return {
        "asset": str(amount(300_000)),
        "liability": str(liability),
        "liability_current": str(liability // RNG.choice([2, 3, 4])),
        "prepayment": str(RNG.choice([0, 0, amount(3_000)])),
        "payments": str(amount(5_000)),
        "depreciation": str(amount(40_000)),
        "interest": str(amount(10_000)),
    }


def reclassified_year() -> dict[str, Any]:
    ofa, prov = amount(), amount(500_000)
    parts = {
        k: amount(800_000) for k in ("trade_receivables", "inventories", "other_operating_assets")
    }
    parts |= {k: amount(400_000) for k in ("trade_payables", "other_operating_liabilities")}
    owc = sum(int(v) for k, v in parts.items() if "payables" not in k and "liabilities" not in k)
    owc -= parts["trade_payables"] + parts["other_operating_liabilities"]
    noa = amount(300_000)
    ic = ofa + owc - prov
    gross, cash, item = amount(), amount(), amount(100_000)
    nfp = gross - cash + item
    dl = amount(100_000)
    equity = ic + noa - nfp - dl
    bs: dict[str, Any] = {
        "operating_fixed_assets": str(ofa),
        "operating_working_capital": {"total": str(owc), **{k: str(v) for k, v in parts.items()}},
        "operating_provisions": str(prov),
        "invested_capital": str(ic),
        "non_operating_assets": [{"label": "Land", "value": str(noa)}],
        "total_funds_invested": str(ic + noa),
        "equity": str(equity),
        "quasi_equity_items": [],
        "net_financial_position": {
            "total": str(nfp),
            "position": {
                "gross_debt": str(gross),
                "cash": str(cash),
                "other_items": [{"name": "Leases", "amount": str(item)}],
            },
            "current": str(nfp - gross // 2),
            "non_current": str(gross // 2),
        },
        "debt_like_items": [
            {"label": "Pensions", "value": str(dl), "measurement_basis": "book_value"}
        ],
        "total_sources": str(ic + noa),
    }
    if RNG.random() < 0.2:
        # Only the parts of the net financial position: its total is their sum (R5).
        nfp_node = bs["net_financial_position"]
        bs["net_financial_position"] = {
            "current": nfp_node["current"],
            "non_current": nfp_node["non_current"],
        }
    for key in list(bs):
        if RNG.random() < 0.1:
            del bs[key]
    return {"reclassified": {"balance_sheet": bs, "income_statement": reclassified_is()}}


def reclassified_is() -> dict[str, Any]:
    """Reclassified income statement totals that add up by Appendix B.2; some dropped, and one
    total off by one now and then."""
    va, personnel, op, inv = amount(), amount(800_000), amount(500_000), amount(80_000) - 40_000
    nr, fin, taxes = amount(40_000) - 20_000, -amount(90_000), amount(150_000)
    ebitda = va - personnel
    vop, dna = amount(), amount(200_000)
    pbft = op + inv
    pbt = pbft + nr + fin
    inc = {
        "revenue": str(amount()),
        "value_of_production": str(vop),
        "external_costs": str(vop - va),
        "other_charges_below_ebitda": str(abs(ebitda - dna - op)),
        "purchases": str(amount()),
        "materials_consumed": str(amount() - 100_000),
        "value_added": str(va),
        "personnel_costs": str(personnel),
        "ebitda": str(ebitda + RNG.choice([0] * 30 + [1])),
        "depreciation_and_amortisation": str(dna),
        "operating_profit": str(op),
        "investing_result": str(inv),
        "profit_before_financing_and_taxes": str(pbft),
        "non_recurring_result": str(nr),
        "financing_result": str(fin),
        "interest_and_other_financial_charges": str(amount(90_000)),
        "profit_before_tax": str(pbt),
        "income_taxes": str(taxes),
        "net_income": str(pbt - taxes),
    }
    for key in list(inc):
        if RNG.random() < 0.15:
            del inc[key]
    return inc


def loan_split(loans: dict[str, Any]) -> list[dict[str, Any]]:
    """Two parts of the shareholder loans, with maturities that mostly reconcile with the loans'
    (review of PR 1, item 4): sometimes a quasi-equity part, sometimes a maturity left out or
    off by one."""
    total, beyond = int(loans["total"]), int(loans.get("beyond_one_year", "0"))
    first = total // 2
    first_beyond = min(first, beyond)
    parts: list[dict[str, Any]] = [
        {
            "destination": "financial_debt",
            "amount": str(first),
            "label": "Loan A",
            "beyond_one_year": str(first_beyond + RNG.choice([0, 0, 0, 1])),
        },
        {
            "destination": RNG.choice(["financial_debt", "quasi_equity"]),
            "amount": str(total - first),
            "label": "Loan B",
        },
    ]
    if parts[1]["destination"] == "financial_debt" and RNG.random() < 0.8:
        parts[1]["beyond_one_year"] = str(beyond - first_beyond)
    return parts


def call() -> dict[str, Any]:
    n = RNG.randrange(1, 4)
    years = [statutory_year() if RNG.random() < 0.75 else reclassified_year() for _ in range(n)]
    for i, y in enumerate(years):
        y["label"] = str(2020 + i)
        loans = (
            y["statutory"]["balance_sheet"].get("payables", {}).get("shareholder_loans")
            if "statutory" in y
            else None
        )
        if loans is not None and RNG.random() < 0.3:
            y["treatment_overrides"] = {"shareholder_loans": {"split": loan_split(loans)}}
        if "statutory" in y and RNG.random() < 0.2:
            b3 = y["statutory"]["balance_sheet"].get("financial_fixed_assets")
            if b3 is not None:
                half = int(b3) // 2
                y.setdefault("treatment_overrides", {})
                y["treatment_overrides"] |= {
                    "financial_fixed_assets": {
                        "split": [
                            {"destination": "non_operating", "amount": str(half), "label": "Stake"},
                            {
                                "destination": "financial_asset",
                                "amount": str(int(b3) - half),
                                "label": "Bonds",
                            },
                        ]
                    }
                }
    treatments: dict[str, Any] = {}
    for key, options in (
        ("financial_fixed_assets", ["operating", "non_operating", "financial_asset"]),
        ("shareholder_loans", ["financial_debt", "quasi_equity"]),
        ("derivative_liabilities", ["financial_liability", "operating_provision"]),
        ("write_downs_of_receivables", ["investing", "operating"]),
    ):
        if RNG.random() < 0.85:
            treatments[key] = RNG.choice(options)
    return {"years": years, "treatments": treatments, "adjustments": adjustments(n, years)}


def engine_view(value: dict[str, Any], meta: dict[str, Any]) -> dict[str, Any]:
    """The engine result reduced to what the reference computes."""
    years = []
    for y in value["years"]:
        years.append(
            {
                "form": y["form"],
                "balance_sheet": y["balance_sheet"],
                "income_statement": y["income_statement"],
                "current_non_current": y["current_non_current"],
                "checks": {k: c["status"] for k, c in y["checks"].items()},
                "not_computed": y["not_computed"],
            }
        )
    defaults = [(d["field"], d["value"]) for d in meta.get("defaults_applied", [])]
    return {"years": years, "defaults_applied": defaults}


def normalise(node: Any) -> Any:
    if isinstance(node, dict):
        return {k: normalise(v) for k, v in node.items()}
    if isinstance(node, list):
        return [normalise(v) for v in node]
    if isinstance(node, tuple):
        return tuple(normalise(v) for v in node)
    if isinstance(node, (D, int)) and not isinstance(node, bool):
        return D(node).normalize()
    if isinstance(node, str):
        try:
            return D(node).normalize() if node.lstrip("-").replace(".", "", 1).isdigit() else node
        except ArithmeticError:
            return node
    return node


@pytest.mark.parametrize("seed", range(400))
def test_statements_match_the_reference(seed: int) -> None:
    RNG.seed(20261108 + seed)
    inp = call()
    try:
        expected = ref.reorganise(copy.deepcopy(inp))
    except ref.RefError as e:
        with pytest.raises(fm.FairmarkError) as err:
            fm.run("statements.reorganise", inp)
        assert err.value.code == e.code, (e.detail, err.value)
        return
    result = fm.run("statements.reorganise", inp)
    got = engine_view(result.value, result.meta)
    assert normalise(got) == normalise(expected)


def test_the_random_calls_reach_every_outcome() -> None:
    """The generator is not degenerate: computed, partly computed and refused calls occur."""
    outcomes = set()
    for seed in range(400):
        RNG.seed(20261108 + seed)
        inp = call()
        try:
            r = ref.reorganise(copy.deepcopy(inp))
        except ref.RefError as e:
            outcomes.add(e.code)
            continue
        missing = any(y["not_computed"]["balance_sheet"] for y in r["years"])
        outcomes.add("partial" if missing else "complete")
        for y in r["years"]:
            is_ = y["income_statement"]
            outcomes.add("net income" if is_["net_income"] is not None else "no net income")
            if is_["reconciliations"]["operating_profit"] is not None:
                outcomes.add("reconciled")
    assert {"partial", "complete", "net income", "no net income", "reconciled"} <= outcomes, (
        outcomes
    )
