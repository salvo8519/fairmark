"""SPEC-0001 R2 to R4 against the independent 50-digit reference
(scripts/reference_free_cash_flows.py).

Seeded random business plans, deterministic and dependency-free: both tax forms, both
treatments of a negative EBIT, labelled adjustments and financing lines. The golden cases pin
selected values; this test covers the space between them (AC1) and the property of AC2.
"""

from __future__ import annotations

import importlib.util
import random
import sys
from decimal import Decimal as D
from pathlib import Path
from typing import Any

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "ref_free_cash_flows", ROOT / "scripts/reference_free_cash_flows.py"
)
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
sys.modules["ref_free_cash_flows"] = ref  # dataclasses resolve annotations through sys.modules
_spec.loader.exec_module(ref)

RNG = random.Random(20261108)


def amount(low: int, high: int, places: int = 4) -> str:
    """A random amount with up to `places` decimals, as an exact string."""
    scale = 10**places
    return str(D(RNG.randrange(low * scale, high * scale + 1)) / scale)


def random_plan(*, financing: bool, rate_form: bool) -> dict[str, Any]:
    periods = []
    for _ in range(RNG.randrange(1, 11)):
        p: dict[str, Any] = {
            "ebit": amount(-5_000_000, 50_000_000),
            "depreciation_and_amortisation": amount(0, 5_000_000),
            "capital_expenditure": amount(0, 8_000_000),
            "change_in_net_working_capital": amount(-2_000_000, 2_000_000),
        }
        if RNG.random() < 0.5:
            p["other_adjustments"] = [
                {"label": f"Item {k}", "value": amount(-500_000, 500_000)}
                for k in range(RNG.randrange(1, 4))
            ]
        if not rate_form:
            p["operating_taxes"] = amount(-1_000_000, 15_000_000)
        if financing:
            p["interest_expense"] = amount(0, 3_000_000)
            p["net_borrowing"] = amount(-10_000_000, 10_000_000)
        periods.append(p)
    plan: dict[str, Any] = {"periods": periods}
    rate = str(D(RNG.randrange(0, 9_999)) / 10_000 + D(RNG.randrange(0, 10**12)) / 10**16)
    if rate_form:
        plan["notional_tax_rate"] = rate
        plan["negative_ebit"] = RNG.choice(["credit", "zero"])
    elif financing:
        plan["interest_tax_rate"] = rate
    return plan


def significant_digits_agree(got: D, want: D, digits: int = 28) -> bool:
    """|got - want| within half a unit of the 28th significant digit of `want`."""
    if want == 0:
        return got == 0
    unit = D(10) ** (want.adjusted() - digits + 1)
    return abs(got - want) <= unit / 2


def test_flows_match_the_reference_to_28_significant_digits() -> None:
    """AC1: FCFF and FCFE per period, in both tax forms and both negative EBIT treatments."""
    for _ in range(300):
        plan = random_plan(financing=RNG.random() < 0.6, rate_form=RNG.random() < 0.6)
        got = fm.run("dcf.free_cash_flows", plan)
        want = ref.build_up_json(plan)
        assert len(got.periods) == len(want)
        for g, w in zip(got.periods, want, strict=True):
            assert significant_digits_agree(g["operating_taxes"], w.taxes), (plan, g, w)
            assert significant_digits_agree(g["free_cash_flow_to_firm"], w.fcff), (plan, g, w)
            if w.fcfe is None:
                assert g["free_cash_flow_to_equity"] is None
                assert g["interest_tax_shield"] is None
            else:
                assert significant_digits_agree(g["interest_tax_shield"], w.interest_tax_shield), (
                    plan,
                    g,
                    w,
                )
                assert significant_digits_agree(g["free_cash_flow_to_equity"], w.fcfe), (
                    plan,
                    g,
                    w,
                )


def test_fcfe_equals_fcff_without_interest_and_borrowing() -> None:
    """AC2: with zero interest and zero net borrowing, FCFE equals FCFF exactly."""
    for _ in range(200):
        plan = random_plan(financing=True, rate_form=RNG.random() < 0.5)
        for p in plan["periods"]:
            p["interest_expense"] = "0"
            p["net_borrowing"] = "0"
        got = fm.run("dcf.free_cash_flows", plan)
        assert got.free_cash_flows_to_equity == got.free_cash_flows_to_firm


def test_golden_inputs_of_the_reference_agree_with_the_files() -> None:
    """Inputs and expected values of the golden files agree with the reference."""
    import yaml

    folder = ROOT / "tests/golden/dcf.free_cash_flows"
    for name, case in ref.CASES.items():
        data = yaml.safe_load((folder / f"{name}.yaml").read_text(encoding="utf-8"))
        assert data["input"] == case, name
        want = ref.build_up_json(case)
        fields = {
            "operating_taxes": "taxes",
            "ebit_after_taxes": "ebit_after_taxes",
            "free_cash_flow_to_firm": "fcff",
            "interest_tax_shield": "interest_tax_shield",
            "interest_after_taxes": "interest_after_taxes",
            "free_cash_flow_to_equity": "fcfe",
        }
        checked = 0
        for key, expected in data["expect"].items():
            parts = key.strip("/").split("/")
            if expected == "null":
                continue
            if len(parts) == 3 and parts[0] == "periods" and parts[2] in fields:
                value = getattr(want[int(parts[1])], fields[parts[2]])
                assert D(expected) == value, (name, key, expected, value)
                checked += 1
        assert checked, name


def test_fcfe_equals_the_net_income_route() -> None:
    """Decision 22: under both treatments every period's FCFE equals the net-income route; under
    `zero` no tax arises on a pre-tax loss, so the interest tax shield is t x min(interest,
    max(EBIT, 0)) and the total taxes are never below zero."""
    checked = limited = 0
    for _ in range(200):
        plan = random_plan(financing=True, rate_form=True)
        # Interest of the order of EBIT, so that EBIT falls below the interest in many periods.
        for p in plan["periods"]:
            p["interest_expense"] = amount(0, 30_000_000)
        got = fm.run("dcf.free_cash_flows", plan)
        rate, treatment = D(plan["notional_tax_rate"]), plan["negative_ebit"]
        for p, row in zip(plan["periods"], got.periods, strict=True):
            lines = ref.lines_from_json(p)
            want = ref.net_income_route(lines, rate, treatment)
            assert significant_digits_agree(row["free_cash_flow_to_equity"], want), (p, row)
            if treatment == "zero":
                total_taxes = row["operating_taxes"] - row["interest_tax_shield"]
                assert total_taxes >= 0, (p, row)
                if lines.ebit < 0:
                    assert row["interest_tax_rate"] == 0
                elif lines.ebit < lines.interest_expense:  # type: ignore[operator]
                    assert total_taxes == 0, (p, row)
                    limited += 1
            checked += 1
    assert checked > 300
    assert limited > 50


def test_given_lines_through_fcff_equal_the_built_up_flows() -> None:
    """SPEC-0001 R7: a stage built with `given_lines` values like the flows the reference
    builds up from the same lines."""
    lines = ref.CASES["negative-ebit-zero"]["periods"]
    lines = [
        {k: v for k, v in p.items() if k not in ("interest_expense", "net_borrowing")}
        for p in lines
    ]
    common = {
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": "0.09",
        "timing": "end_of_period",
        "terminal": fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
    }
    built = fm.dcf.fcff(
        [fm.dcf.given_lines(lines)], notional_tax_rate="0.279", negative_ebit="zero", **common
    )
    flows = [
        p.fcff
        for p in ref.build_up(
            [ref.lines_from_json(p) for p in lines],
            notional_tax_rate=D("0.279"),
            negative_ebit="zero",
        )
    ]
    given = fm.dcf.fcff([fm.dcf.given(flows)], **common)
    assert built.value_of_operations == given.value_of_operations
    assert [p["flow"] for p in built.periods] == flows
