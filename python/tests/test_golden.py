"""Runs the shared golden cases (tests/golden) through the Python API."""

import re
from decimal import Decimal
from pathlib import Path
from typing import Any

import pytest
import yaml

import fairmark as fm

GOLDEN = Path(__file__).resolve().parents[2] / "tests" / "golden"
CASES = sorted(GOLDEN.rglob("*.yaml"))


@pytest.mark.parametrize("path", CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_golden(path: Path) -> None:
    case = yaml.safe_load(path.read_text(encoding="utf-8"))
    if "expect_error" in case:
        with pytest.raises(fm.FairmarkError) as err:
            fm.run(case["method"], case["input"])
        assert err.value.code == case["expect_error"]
        return
    result = fm.run(case["method"], case["input"])
    tolerance = Decimal(case["tolerance"])
    expectations = [(result.value, f, e) for f, e in case["expect"].items()]
    expectations += [(result.meta, f, e) for f, e in case.get("expect_meta", {}).items()]
    for part, field, expected in expectations:
        raw = resolve(part, field)
        try:
            want = Decimal(str(expected))
            if not want.is_finite():  # NaN, Infinity: not numbers, as in the Rust runner
                raise ArithmeticError
        except ArithmeticError:
            # Not a decimal: compared exactly (null, error codes, names).
            if expected == "null":
                assert raw is None, f"{field}: {raw!r} is not null"
            else:
                assert raw is not None and str(raw) == str(expected), f"{field}: {raw!r}"
            continue
        got = Decimal(str(raw))
        assert abs(got - want) <= tolerance, f"{field}: {got} != {expected}"


def resolve(value: Any, field: str) -> Any:
    """A key starting with '/' is an RFC 6901 pointer into the value; other keys are fields."""
    if not field.startswith("/"):
        return value[field]
    for token in field[1:].split("/"):
        token = token.replace("~1", "/").replace("~0", "~")
        if isinstance(value, list):
            # RFC 6901 array index: digits, no leading zero, no sign.
            assert re.fullmatch(r"0|[1-9][0-9]*", token), f"{field}: bad index {token!r}"
            value = value[int(token)]
        else:
            value = value[token]
    return value


def test_golden_cases_exist() -> None:
    assert CASES, "no golden cases found"


NEW_CASE_KEYS = {
    "method",
    "description",
    "source",
    "input",
    "expect",
    "expect_meta",
    "expect_error",
    "tolerance",
}


@pytest.mark.parametrize("path", CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_golden_case_shape(path: Path) -> None:
    case = yaml.safe_load(path.read_text(encoding="utf-8"))
    assert set(case) <= NEW_CASE_KEYS, f"unknown keys: {set(case) - NEW_CASE_KEYS}"
    assert ("expect" in case) != ("expect_error" in case), "exactly one of expect, expect_error"
    if "expect" in case:
        assert "tolerance" in case, "value cases need a tolerance"
    else:
        assert "expect_meta" not in case, "expect_meta needs expect"
