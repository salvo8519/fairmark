"""The shared DCF engine (SPEC-0001, issue #110) against the independent reference
scripts/reference_dcf.py.

The golden cases pin single points; these tests check that the golden values are those of the
reference, that the engine and the 50-digit reference agree across a grid of stages, rates and
timings, and the properties AC5 and AC7.
"""

from __future__ import annotations

import importlib.util
from decimal import Decimal as D
from itertools import product
from pathlib import Path
from typing import Any

import pytest
import yaml

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("ref_dcf", ROOT / "scripts/reference_dcf.py")
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ref)

REFERENCE = "Independent 50-digit reference, scripts/reference_dcf.py"
GOLDEN = sorted((ROOT / "tests/golden/dcf.fcff").glob("*.yaml"))
# Relative agreement of engine and reference: about 1e-24 at end of period (exact powers and
# divisions in 28 digits), about 1e-18 at mid period (square roots in 28 digits).
REL = {"end_of_period": D("1e-22"), "mid_period": D("1e-17")}


def pointer(value: Any, path: str) -> Any:
    for part in path.strip("/").split("/"):
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def reference_cases() -> list[Path]:
    out = []
    for path in GOLDEN:
        case = yaml.safe_load(path.read_text(encoding="utf-8"))
        if "expect" in case and str(case.get("source", "")).startswith(REFERENCE):
            out.append(path)
    return out


@pytest.mark.parametrize("path", reference_cases(), ids=lambda p: p.stem)
def test_golden_values_are_the_reference_values(path: Path) -> None:
    """Each expected value of a reference-sourced golden case is the reference value, rounded
    to the digits written in the file."""
    case = yaml.safe_load(path.read_text(encoding="utf-8"))
    values = ref.dcf_fcff(case["input"])
    for key, expected in case["expect"].items():
        want = D(str(expected))
        got = values[key if key.startswith("/") else f"/{key}"]
        assert abs(got - want) <= D(1).scaleb(want.as_tuple().exponent), key  # type: ignore[arg-type]


def run(payload: dict[str, Any]) -> dict[str, Any]:
    return fm.run_json("dcf.fcff", payload)["value"]


def close(got: str | D, want: D, timing: str) -> bool:
    return abs(D(str(got)) - want) <= REL[timing] * max(D(1), abs(want))


STAGES = [
    [{"periods": 2, "projection": "given", "flows": ["100", "-20"]}],
    [
        {"periods": 2, "projection": "given", "flows": ["50", "60"]},
        {"periods": 3, "projection": "constant_growth", "growth": "0.07"},
    ],
    [
        {"periods": 1, "projection": "given", "flows": ["75.5"]},
        {"periods": 4, "projection": "linear_decline", "from_growth": "0.15", "to_growth": "0.02"},
        {"periods": 2, "projection": "constant_growth", "growth": "0.02"},
    ],
]
RATES = ["0.065", "0.1", "0.14"]
TIMINGS = [("end_of_period", None), ("mid_period", "horizon"), ("mid_period", "mid_period")]


@pytest.mark.parametrize(
    ("stages", "rate", "timing"), list(product(STAGES, RATES, TIMINGS)), ids=str
)
def test_engine_matches_reference(
    stages: list[dict[str, Any]], rate: str, timing: tuple[str, str | None]
) -> None:
    payload: dict[str, Any] = {
        "stages": stages,
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": rate,
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": "0.015", "first_flow": "last_flow_grown"}},
        "net_financial_position": {"gross_debt": "120", "cash": "30"},
        "non_operating_assets": [{"label": "Land", "value": "15"}],
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got, want = run(payload), ref.dcf_fcff(payload)
    for key, value in want.items():
        assert close(pointer(got, key), value, timing[0]), key


@pytest.mark.parametrize("timing", TIMINGS, ids=str)
def test_engine_matches_reference_with_rates_per_period(timing: tuple[str, str | None]) -> None:
    payload: dict[str, Any] = {
        "stages": [
            {"periods": 3, "projection": "constant_growth", "growth": "0.09"},
            {
                "periods": 2,
                "projection": "linear_decline",
                "from_growth": "0.09",
                "to_growth": "0.03",
            },
        ],
        "base_flow": "40",
        "cash_flow_terms": "real",
        "discount_rate_terms": "real",
        "wacc_per_period": ["0.12", "0.11", "-0.01", "0.095", "0.08"],
        "terminal_wacc": "0.075",
        "timing": timing[0],
        "terminal": {
            "tv.gordon": {
                "growth": "0.03",
                "first_flow": "normalised_next_flow",
                "normalised_next_flow": "61",
            }
        },
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got, want = run(payload), ref.dcf_fcff(payload)
    for key, value in want.items():
        assert close(pointer(got, key), value, timing[0]), key


@pytest.mark.parametrize("timing", TIMINGS, ids=str)
def test_ac5_generated_stages_equal_given_flows(timing: tuple[str, str | None]) -> None:
    """AC5: a constant growth stage equals a given stage with the same flows; a linear decline
    with equal growths equals constant growth at that rate."""
    common: dict[str, Any] = {
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": "0.085",
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
    }
    if timing[1]:
        common["terminal_discount_time"] = timing[1]
    constant = run(
        {
            **common,
            "base_flow": "100",
            "stages": [{"periods": 4, "projection": "constant_growth", "growth": "0.06"}],
        }
    )
    flows = [row["flow"] for row in constant["periods"]]
    as_given = run({**common, "stages": [{"periods": 4, "projection": "given", "flows": flows}]})
    decline = run(
        {
            **common,
            "base_flow": "100",
            "stages": [
                {
                    "periods": 4,
                    "projection": "linear_decline",
                    "from_growth": "0.06",
                    "to_growth": "0.06",
                }
            ],
        }
    )
    for other in (as_given, decline):
        assert other["value_of_operations"] == constant["value_of_operations"]
        assert other["terminal"]["terminal_value"] == constant["terminal"]["terminal_value"]
        assert [r["present_value"] for r in other["periods"]] == [
            r["present_value"] for r in constant["periods"]
        ]


@pytest.mark.parametrize("timing", ["end_of_period", "mid_period"])
def test_ac7_discount_factors_equal_those_of_tvm_present_value(timing: str) -> None:
    rates = ["0.12", "0.1", "-0.02", "0.07"]
    flows = ["10", "20", "30", "40"]
    payload: dict[str, Any] = {
        "stages": [{"periods": 4, "projection": "given", "flows": flows}],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc_per_period": rates,
        "terminal_wacc": "0.07",
        "timing": timing,
        "terminal": {"tv.gordon": {"growth": "0.01", "first_flow": "last_flow_grown"}},
    }
    if timing == "mid_period":
        payload["terminal_discount_time"] = "horizon"
    dcf = run(payload)
    pv = fm.run_json(
        "tvm.present_value", {"cash_flows": flows, "period_rates": rates, "timing": timing}
    )["value"]
    assert [r["discount_factor"] for r in dcf["periods"]] == pv["discount_factors"]
    # Present values divide by the compounding factor instead of multiplying by the discount
    # factor (precision at long horizons): equal to the last digits, not bit for bit.
    for ours, theirs in zip(dcf["periods"], pv["present_values"], strict=True):
        assert abs(D(ours["present_value"]) - D(theirs)) <= D("1e-26")


@pytest.mark.parametrize(
    ("periods", "rate", "growth", "terminal_growth", "timing"),
    [
        (400, "0.10", "0.095", "0.095", ("end_of_period", None)),
        (300, "0.20", "0.195", "0.19", ("mid_period", "mid_period")),
        (300, "0.20", "0.195", "0.19", ("mid_period", "horizon")),
        (1000, "0.05", "0.045", "0.04", ("end_of_period", None)),
        (250, "0.12", "0.11", "0.11", ("mid_period", "horizon")),
    ],
    ids=str,
)
def test_long_horizons_keep_full_precision(
    periods: int, rate: str, growth: str, terminal_growth: str, timing: tuple[str, str | None]
) -> None:
    """Audit finding 1: at long horizons the value of operations matches the 50-digit
    reference to 1e-24 relative, because present values divide by the compounding factor."""
    payload: dict[str, Any] = {
        "stages": [{"periods": periods, "projection": "constant_growth", "growth": growth}],
        "base_flow": "100",
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": rate,
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": terminal_growth, "first_flow": "last_flow_grown"}},
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got = D(run(payload)["value_of_operations"])
    want = ref.dcf_fcff(payload)["/value_of_operations"]
    assert abs(got - want) <= D("1e-24") * abs(want)
