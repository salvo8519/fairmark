"""Terminal value methods (SPEC-0006) against the independent reference scripts/reference_tv.py.

The golden cases pin single points; these grids check that the engine and the 50-digit
reference agree across a range of inputs, and the properties of AC2 to AC5.
"""

from __future__ import annotations

import importlib.util
from decimal import Decimal as D
from itertools import product
from pathlib import Path

import pytest

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("ref_tv", ROOT / "scripts/reference_tv.py")
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ref)

TOL = D("1e-20")
RATES = ["0.06", "0.085", "0.1", "0.13"]
GROWTHS = ["-0.01", "0", "0.015", "0.03"]


@pytest.mark.parametrize(("rate", "growth"), list(product(RATES, GROWTHS)))
def test_gordon_and_value_driver_match_the_reference(rate: str, growth: str) -> None:
    r, g = D(rate), D(growth)
    tv = fm.tv.gordon(rate, growth, last_flow="123.45").terminal_value
    assert abs(tv - ref.gordon(r, g, last_flow=D("123.45"))) <= TOL
    for ronic in ["0.05", "0.1", "0.2"]:
        tv = fm.tv.value_driver("80", growth, ronic, rate).terminal_value
        assert abs(tv - ref.value_driver(D(80), g, D(ronic), r)) <= TOL


@pytest.mark.parametrize("growth", GROWTHS)
def test_ac2_ronic_equal_to_rate_makes_growth_irrelevant(growth: str) -> None:
    assert fm.tv.value_driver("100", growth, "0.1", "0.1").terminal_value == D(1000)


@pytest.mark.parametrize(("rate", "growth"), list(product(RATES, GROWTHS)))
def test_ac3_h_model_with_equal_growths_is_gordon(rate: str, growth: str) -> None:
    h = fm.tv.h_model("100", growth, growth, "4", rate).terminal_value
    gordon = fm.tv.gordon(rate, growth, last_flow="100").terminal_value
    assert abs(h - gordon) <= TOL
    h = fm.tv.h_model("100", "0.09", growth, "3.5", rate).terminal_value
    assert abs(h - ref.h_model(D(100), D("0.09"), D(growth), D("3.5"), D(rate))) <= TOL


@pytest.mark.parametrize("rate", RATES)
def test_ac4_implied_growth_fed_back_into_gordon(rate: str) -> None:
    exit_tv = fm.tv.exit_multiple("7", "ebitda", "150", rate=rate, next_flow="64")
    assert abs(exit_tv.implied_growth - ref.implied_growth(D(rate), D(64), D(1050))) <= TOL
    gordon = fm.tv.gordon(rate, exit_tv.implied_growth, next_flow="64")
    assert abs(gordon.terminal_value - exit_tv.terminal_value) <= D("1e-18")


@pytest.mark.parametrize(("rate", "periods"), list(product(RATES, [1, 7, 40])))
def test_limited_horizon_matches_the_term_by_term_sum(rate: str, periods: int) -> None:
    result = fm.tv.limited_horizon("100", rate, periods, growth="0.02", residual_value="250")
    total, flows, residual = ref.limited_horizon(D(100), D(rate), periods, D("0.02"), D(250))
    assert abs(result.terminal_value - total) <= TOL
    assert abs(result.flows_value - flows) <= TOL
    assert abs(result.residual_value_discounted - residual) <= TOL


@pytest.mark.parametrize("rate", RATES)
def test_ac5_long_horizon_converges_to_the_perpetuity(rate: str) -> None:
    # The longest horizon with (1 + r)^n below e^57 (about 6e24): tvm.annuity, reused,
    # also computes the future value at N + n, which must stay within the decimal range.
    periods = int(D(57) / (1 + D(rate)).ln())
    finite = fm.tv.limited_horizon("100", rate, periods).terminal_value
    perpetual = fm.tv.normalised_income("100", rate).terminal_value
    assert abs(finite - perpetual) <= TOL


def test_compare_equals_each_technique_run_alone() -> None:
    result = fm.tv.compare(
        "0.1",
        "enterprise",
        [
            {"tv.gordon": {"growth": "0.02", "last_flow": "100"}},
            {"tv.normalised_income": {"income": "95", "capitalisation_rate": "0.11"}},
        ],
        growth_cap="0.03",
    )
    values = [D(r["terminal_value"]) for r in result.value["results"]]
    assert values[0] == fm.tv.gordon("0.1", "0.02", last_flow="100").terminal_value
    assert values[1] == fm.tv.normalised_income("95", "0.11").terminal_value
