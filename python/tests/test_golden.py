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
    result = fm.run(case["method"], case["input"])
    tolerance = Decimal(case["tolerance"])
    for field, expected in case["expect"].items():
        got = result.value[field]
        assert abs(got - Decimal(str(expected))) <= tolerance, f"{field}: {got} != {expected}"


def test_golden_cases_exist() -> None:
    assert CASES, "no golden cases found"
