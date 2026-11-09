"""SPEC-0007 R7 to R11 against the independent reference (scripts/reference_ratios_partial.py).

Seeded random inputs for every `ratios.*` method, with random gaps (any amount, the opening
balance sheet, the definitions, the leverage block, the debt schedule), zero and negative
amounts that make ratios not defined, and both balance conventions: every output value, every
`not_computed` entry (output and missing pointers) and every documented default applied must
equal the reference, and an input below the floor must be refused with `FM-E027` and the
alternative sets of the reference. The golden cases pin selected inputs; this test covers the
space between them.
"""

from __future__ import annotations

import importlib.util
import random
import sys
from decimal import Decimal as D
from pathlib import Path
from typing import Any

import pytest

from fairmark._core import FairmarkError, run_json

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "ref_ratios_partial", ROOT / "scripts/reference_ratios_partial.py"
)
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
sys.modules["ref_ratios_partial"] = ref  # dataclasses resolve annotations through sys.modules
_spec.loader.exec_module(ref)

CASES = 300
# Outputs that are amounts; every other output of the ratio groups is a ratio object.
AMOUNTS = {
    "/net_financial_position",
    "/primary_structure_margin",
    "/secondary_structure_margin",
    "/net_working_capital",
    "/treasury_margin",
    "/pv_cfads",
}


def amount(rng: random.Random, *, signed: bool = False) -> str:
    """A random amount: usually positive, sometimes zero, negative when `signed`."""
    roll = rng.random()
    if roll < 0.06:
        return "0"
    value = D(rng.randint(1, 9_000_000)) / D(rng.choice([1, 10, 100]))
    if signed and roll < 0.2:
        value = -value
    return str(value)


def sparse(rng: random.Random, block: dict[str, Any], p: float = 0.2) -> dict[str, Any]:
    return {k: v for k, v in block.items() if rng.random() >= p}


def balances(rng: random.Random, data: dict[str, Any], sheet: Any) -> None:
    data["balances"] = rng.choice(["closing", "average"])
    data["closing"] = sheet()
    if rng.random() < 0.8:
        data["opening"] = sheet()


def profitability(rng: random.Random) -> dict[str, Any]:
    data: dict[str, Any] = {
        "operating_return_base": rng.choice(["invested_capital", "total_assets"]),
        "income_statement": sparse(
            rng,
            {k: amount(rng, signed=True) for k in ("revenue", "ebitda", "ebit", "net_income")},
        ),
    }
    if rng.random() < 0.8:
        data["ebitda_definition"] = "reported EBITDA"
    keys = ("equity", "total_assets", "invested_capital", "debt")
    balances(rng, data, lambda: sparse(rng, {k: amount(rng, signed=k == "equity") for k in keys}))
    if rng.random() < 0.7:
        lev = {"debt_basis": rng.choice(["gross_financial_debt", "net_financial_position"])}
        lev |= sparse(
            rng, {k: amount(rng) for k in ("financial_charges", "taxes", "pre_tax_income")}
        )
        data["leverage"] = lev
    return data


def nfp_block(rng: random.Random) -> dict[str, Any]:
    block: dict[str, Any] = sparse(rng, {"gross_debt": amount(rng), "cash": amount(rng)}, p=0.15)
    if rng.random() < 0.4:
        item: dict[str, Any] = {"name": "lease liabilities"}
        if rng.random() < 0.85:
            item["amount"] = amount(rng, signed=True)
        block["other_items"] = [item]
    return block


def capital_structure(rng: random.Random) -> dict[str, Any]:
    data: dict[str, Any] = {}

    def sheet() -> dict[str, Any]:
        keys = ("equity", "total_assets", "fixed_assets", "non_current_liabilities")
        out = sparse(rng, {k: amount(rng, signed=k == "equity") for k in keys})
        if rng.random() < 0.85:
            out["net_financial_position"] = nfp_block(rng)
        return out

    balances(rng, data, sheet)
    return data


def liquidity(rng: random.Random) -> dict[str, Any]:
    data: dict[str, Any] = {}
    keys = ("current_assets", "inventories", "cash", "current_liabilities")
    balances(rng, data, lambda: sparse(rng, {k: amount(rng) for k in keys}))
    return data


def working_capital(rng: random.Random) -> dict[str, Any]:
    data: dict[str, Any] = {
        "day_basis": rng.choice(["360", "365"]),
        "income_statement": sparse(
            rng, {k: amount(rng) for k in ("revenue", "cost_of_sales", "purchases")}
        ),
    }
    keys = ("trade_receivables", "inventories", "trade_payables")
    balances(rng, data, lambda: sparse(rng, {k: amount(rng) for k in keys}))
    return data


def debt_service(rng: random.Random) -> dict[str, Any]:
    data: dict[str, Any] = {
        "income_statement": sparse(
            rng, {k: amount(rng, signed=True) for k in ("ebitda", "ebit", "interest")}
        ),
    }
    for definition in ("ebitda_definition", "interest_definition"):
        if rng.random() < 0.8:
            data[definition] = "as reported"

    def sheet() -> dict[str, Any]:
        return {"net_financial_position": nfp_block(rng)} if rng.random() < 0.9 else {}

    balances(rng, data, sheet)
    if rng.random() < 0.8:
        schedule: dict[str, Any] = sparse(
            rng,
            {
                "discount_rate": str(D(rng.randint(0, 150)) / D(1000)),
                "debt_outstanding": amount(rng, signed=True),
            },
            p=0.15,
        )
        if rng.random() < 0.4:
            schedule["reserve_balance"] = amount(rng)
        schedule["periods"] = [
            sparse(rng, {"cfads": amount(rng), "debt_service": amount(rng, signed=True)}, p=0.1)
            for _ in range(rng.randint(1, 4))
        ]
        data["debt_schedule"] = schedule
    return data


def covenants(rng: random.Random) -> dict[str, Any]:
    items = []
    for i in range(rng.randint(1, 4)):
        numerator: dict[str, Any] = {"definition": "net debt"}
        denominator: dict[str, Any] = {"definition": "EBITDA"}
        if rng.random() < 0.8:
            numerator["amount"] = amount(rng, signed=True)
        if rng.random() < 0.8:
            denominator["amount"] = amount(rng, signed=True)
        items.append(
            {
                "name": f"covenant {i}",
                "test_date": "2026-12-31",
                "numerator": numerator,
                "denominator": denominator,
                "operator": rng.choice(["at_most", "less_than", "at_least", "greater_than"]),
                "threshold": str(D(rng.randint(1, 600)) / D(100)),
                "if_denominator_not_positive": rng.choice(["breached", "passed", "not_tested"]),
            }
        )
    return {"covenants": items}


GENERATORS = {
    "ratios.profitability": (profitability, ref.profitability),
    "ratios.capital_structure": (capital_structure, ref.capital_structure),
    "ratios.liquidity": (liquidity, ref.liquidity),
    "ratios.working_capital": (working_capital, ref.working_capital),
    "ratios.debt_service": (debt_service, ref.debt_service),
    "ratios.covenants": (covenants, ref.covenants),
}


def close(got: Any, want: Any) -> bool:
    if want is None or isinstance(want, str):
        return bool(got == want)
    if got is None:
        return False
    got, want = D(got), D(want)
    return abs(got - want) <= D("1e-22") * max(D(1), abs(want))


@pytest.mark.parametrize("method", sorted(GENERATORS))
def test_engine_matches_the_reference_on_random_partial_inputs(method: str) -> None:
    make, reference = GENERATORS[method]
    rng = random.Random(f"{method}-20261109")
    refused = 0
    for case in range(CASES):
        data = make(rng)
        expected = reference(data)
        context = f"{method} case {case}: {data}"
        if "error" in expected:
            refused += 1
            with pytest.raises(FairmarkError) as err:
                run_json(method, data)
            assert err.value.code == expected["error"], context
            sets = " or ".join("[" + ", ".join(f"`{p}`" for p in s) + "]" for s in expected["sets"])
            assert f"give at least one of these sets of inputs: {sets};" in str(err.value), (
                f"{context}: {err.value}"
            )
            continue
        out = run_json(method, data)
        value = out["value"]
        applied = [d["field"] for d in out["meta"].get("defaults_applied", [])]
        assert applied == expected["defaults"], context
        for pointer, want in expected["values"].items():
            path = pointer.strip("/").split("/")
            if method != "ratios.covenants" and pointer not in AMOUNTS:
                path.append("value")
            got: Any = value
            for key in path:
                got = got[int(key)] if isinstance(got, list) else got[key]
            assert close(got, want), f"{context}: {pointer} = {got}, reference {want}"
        listed = [(e["output"], e["missing"]) for e in value["not_computed"]]
        assert listed == expected["not_computed"], context
    # The generators reach both sides of the floor.
    assert 0 < refused < CASES, f"{method}: {refused} inputs below the floor"
