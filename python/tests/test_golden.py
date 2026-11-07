"""Runs the shared golden cases (tests/golden) through the Python API."""

from decimal import Decimal
from pathlib import Path

import pytest
import yaml

import fairmark as fm

GOLDEN = Path(__file__).resolve().parents[2] / "tests" / "golden"
CASES = sorted(GOLDEN.rglob("*.yaml"))


@pytest.mark.parametrize("path", CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_golden(path: Path) -> None:
    case = yaml.safe_load(path.read_text())
    if "expect_error" in case:
        with pytest.raises(fm.FairmarkError) as err:
            fm.run(case["method"], case["input"])
        assert err.value.code == case["expect_error"]
        return
    result = fm.run(case["method"], case["input"])
    tolerance = Decimal(case["tolerance"])
    for field, expected in case["expect"].items():
        got = Decimal(str(result.value[field]))
        assert abs(got - Decimal(str(expected))) <= tolerance, f"{field}: {got} != {expected}"


def test_golden_cases_exist() -> None:
    assert CASES, "no golden cases found"


NEW_CASE_KEYS = {"method", "description", "source", "input", "expect", "expect_error", "tolerance"}


@pytest.mark.parametrize("path", CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_golden_case_shape(path: Path) -> None:
    case = yaml.safe_load(path.read_text())
    assert set(case) <= NEW_CASE_KEYS, f"unknown keys: {set(case) - NEW_CASE_KEYS}"
    assert ("expect" in case) != ("expect_error" in case), "exactly one of expect, expect_error"
    if "expect" in case:
        assert "tolerance" in case, "value cases need a tolerance"
