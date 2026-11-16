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
        report = err.value.report
        for field, count in case.get("expect_length", {}).items():
            items = resolve(report, field)
            assert isinstance(items, list) and len(items) == int(count), f"{field}: length"
        for field, expected in case.get("expect_error_report", {}).items():
            raw = resolve(report, field)
            if expected == "null":
                assert raw is None, f"{field}: {raw!r} is not null"
            else:
                assert raw == expected, f"{field}: {raw!r}"
        return
    result = fm.run(case["method"], case["input"])
    parts = {"value": result.value, "meta": result.meta}
    if "diagnostics" in result.raw:
        parts["diagnostics"] = result.raw["diagnostics"]
    check_result(case, parts)


# Where each kind of expectation reads: `expect` and `expect_length` into `value`,
# `expect_meta` into `meta`, `expect_diagnostics` and `expect_diagnostics_length` into
# `diagnostics` (SPEC-0012).
EXPECT_PARTS = {"expect": "value", "expect_meta": "meta", "expect_diagnostics": "diagnostics"}
LENGTH_PARTS = {"expect_length": "value", "expect_diagnostics_length": "diagnostics"}


def check_result(case: dict[str, Any], parts: dict[str, Any]) -> None:
    """Assert the expectations of a value case against the parts of its result envelope."""
    for key, part in LENGTH_PARTS.items():
        for field, count in case.get(key, {}).items():
            items = lookup(parts, part, field)
            assert isinstance(items, list) and len(items) == int(count), f"{field}: length"
    tolerance = Decimal(case["tolerance"])
    for key, part in EXPECT_PARTS.items():
        for field, expected in case.get(key, {}).items():
            compare(lookup(parts, part, field), field, expected, tolerance)


def lookup(parts: dict[str, Any], part: str, field: str) -> Any:
    """The field of a part of the result; a missing part or field always fails."""
    if part not in parts:
        raise AssertionError(f"{field}: no {part} in the result")
    try:
        return resolve(parts[part], field)
    except (KeyError, IndexError, TypeError):
        raise AssertionError(f"{field}: not in the result") from None


def compare(raw: Any, field: str, expected: Any, tolerance: Decimal) -> None:
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
        return
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
    "expect_length",
    "expect_diagnostics",
    "expect_diagnostics_length",
    "expect_error",
    "expect_error_report",
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
        for key in ("expect_meta", "expect_diagnostics", "expect_diagnostics_length"):
            assert key not in case, f"{key} needs expect"
    if "expect_error_report" in case:
        assert "expect_error" in case, "expect_error_report needs expect_error"


# The diagnostics keys of SPEC-0012, on a temporary case and a result written here (the engine
# does not write `diagnostics` before task T11 of SPEC-0012).
TEMPORARY_CASE: dict[str, Any] = {
    "method": "tvm.npv",
    "description": "temporary",
    "input": {},
    "tolerance": "0.01",
    "expect": {"/npv": "4.13"},
}


def temporary_parts() -> dict[str, Any]:
    return {
        "value": {"npv": Decimal("4.132")},
        "meta": {"envelope_version": 1},
        "diagnostics": {
            "not_computed": [{"pointer": "/value/roe", "reason_code": "not_defined"}],
            "warnings": [
                {"code": "FM-W012", "pointer": "/net_financial_position", "message": "m"},
                {"code": "FM-W005", "pointer": None, "message": "m"},
            ],
            "checks": [],
        },
    }


def test_diagnostics_expectations_pass_on_a_matching_result() -> None:
    case = TEMPORARY_CASE | {
        "expect_diagnostics": {
            "/warnings/0/code": "FM-W012",
            "/warnings/0/pointer": "/net_financial_position",
            "/warnings/1/pointer": "null",
            "/not_computed/0/reason_code": "not_defined",
        },
        "expect_diagnostics_length": {"/warnings": "2", "/checks": "0"},
    }
    check_result(case, temporary_parts())


@pytest.mark.parametrize(
    ("key", "field", "expected", "message"),
    [
        ("expect_diagnostics", "/warnings/0/code", "FM-W013", "'FM-W012'"),
        ("expect_diagnostics", "/warnings/2/code", "FM-W005", "not in the result"),
        ("expect_diagnostics", "/warnings/0/pointer", "null", "is not null"),
        ("expect_diagnostics", "/npv", "4.13", "not in the result"),
        ("expect_diagnostics_length", "/warnings", "1", "length"),
        ("expect_diagnostics_length", "/not_computed/0", "1", "length"),
    ],
)
def test_diagnostics_expectations_fail(key: str, field: str, expected: str, message: str) -> None:
    case = TEMPORARY_CASE | {key: {field: expected}}
    with pytest.raises(AssertionError, match=re.escape(message)):
        check_result(case, temporary_parts())


def test_diagnostics_expectations_fail_without_diagnostics() -> None:
    parts = temporary_parts()
    del parts["diagnostics"]
    for key in ("expect_diagnostics", "expect_diagnostics_length"):
        case = TEMPORARY_CASE | {key: {"/warnings": "2"}}
        with pytest.raises(AssertionError, match="no diagnostics in the result"):
            check_result(case, parts)


def test_case_shape_accepts_the_diagnostics_keys_with_expect_only(tmp_path: Path) -> None:
    case = TEMPORARY_CASE | {
        "source": "temporary",
        "expect_diagnostics": {"/warnings/0/code": "FM-W012"},
        "expect_diagnostics_length": {"/warnings": "1"},
    }
    path = tmp_path / "case.yaml"
    path.write_text(yaml.safe_dump(case), encoding="utf-8")
    test_golden_case_shape(path)
    error_case = {k: v for k, v in case.items() if k not in ("expect", "tolerance")}
    path.write_text(yaml.safe_dump(error_case | {"expect_error": "FM-E004"}), encoding="utf-8")
    with pytest.raises(AssertionError, match="needs expect"):
        test_golden_case_shape(path)
