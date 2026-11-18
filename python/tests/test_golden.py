"""Runs the shared golden cases (tests/golden) through the Python API."""

import json
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
        check_error(case, err.value.report)
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
# SPEC-0013 R1: each semantic section, the part of the result and the list it reads, and the
# fields whose values, joined by one space (`-` for null), name an entry. When an envelope
# version moves a list, this table changes and the golden files do not.
SECTIONS: dict[str, tuple[str, str, tuple[str, ...]]] = {
    "expect_defaults": ("meta", "defaults_applied", ("pointer",)),
    "expect_not_computed": ("diagnostics", "not_computed", ("pointer",)),
    "expect_warnings": ("diagnostics", "warnings", ("code", "pointer")),
    "expect_checks": ("diagnostics", "checks", ("pointer", "id")),
    "expect_errors": ("error", "errors", ("code", "pointer")),
}
# SPEC-0013 R3: the lists `expect_counts` may name in a value case.
COUNTED_LISTS = ("not_computed", "warnings", "checks")
ABSENT = object()


def check_result(case: dict[str, Any], parts: dict[str, Any]) -> None:
    """Assert the expectations of a value case against the parts of its result envelope."""
    for key, part in LENGTH_PARTS.items():
        for field, count in case.get(key, {}).items():
            items = lookup(parts, part, field)
            assert isinstance(items, list) and len(items) == int(count), f"{field}: length"
    tolerance = Decimal(case["tolerance"])
    for key, part in EXPECT_PARTS.items():
        for field, expected in case.get(key, {}).items():
            compare(lookup(parts, part, field, expected), field, expected, tolerance)
    check_sections(case, parts, tolerance)
    for name, count in case.get("expect_counts", {}).items():
        if name.startswith("/"):
            items = lookup(parts, "value", name)
        elif name in COUNTED_LISTS and "diagnostics" in parts:
            items = parts["diagnostics"][name]
        else:
            raise AssertionError(f"expect_counts {name}: not a list of this result")
        count_is(items, f"expect_counts {name}", count)


def check_error(case: dict[str, Any], report: dict[str, Any]) -> None:
    """Assert the expectations of an error case against its error report (SPEC-0013 R1 to R5)."""
    for field, count in case.get("expect_length", {}).items():
        items = resolve(report, field)
        assert isinstance(items, list) and len(items) == int(count), f"{field}: length"
    for field, expected in case.get("expect_error_report", {}).items():
        compare(value_at(report, field, expected), field, expected, None)
    check_sections(case, {"error": report}, None)
    for name, count in case.get("expect_counts", {}).items():
        if name != "errors":
            raise AssertionError(f"expect_counts {name}: not a list of this result")
        count_is(report["errors"], f"expect_counts {name}", count)


def check_sections(case: dict[str, Any], doc: dict[str, Any], tolerance: Decimal | None) -> None:
    """Assert the semantic sections of `case` against `doc`: the parts of a result envelope,
    or `{"error": <report>}` (SPEC-0013 R1, R2)."""
    for section, (part, name, fields) in SECTIONS.items():
        entries = case.get(section) or {}
        if not entries:
            continue
        items = (doc.get(part) or {}).get(name)
        if not isinstance(items, list):
            needs = "expect_error, not expect" if part == "error" else "expect, not expect_error"
            raise AssertionError(f"{section}: needs {needs}")
        for key, expectations in entries.items():
            named = [e for e in items if entry_key(e, fields) == key]
            if len(named) != 1:
                raise AssertionError(f"{section} {key}: {len(named)} entries named it")
            for field, expected in (expectations or {}).items():
                where = f"{section} {key} {field}"
                if field.endswith("#count"):
                    count_is(resolve(named[0], field.removesuffix("#count")), where, expected)
                else:
                    compare(value_at(named[0], field, expected), where, expected, tolerance)


def entry_key(entry: dict[str, Any], fields: tuple[str, ...]) -> str:
    """The name of an entry: its values of `fields`, joined by one space, `-` for null."""
    return " ".join("-" if entry.get(f) is None else str(entry[f]) for f in fields)


def count_is(items: Any, where: str, count: Any) -> None:
    got = len(items) if isinstance(items, list) else None
    assert got == int(count), f"{where}: length {got}, want {count}"


def lookup(parts: dict[str, Any], part: str, field: str, expected: Any = None) -> Any:
    """The field of a part of the result; a missing part or field always fails, except a
    `null` expectation on an absent last token (SPEC-0013 R4)."""
    if part not in parts:
        raise AssertionError(f"{field}: no {part} in the result")
    return value_at(parts[part], field, expected)


def value_at(container: Any, field: str, expected: Any = None) -> Any:
    """`field` of `container`; `ABSENT` when `expected` is `null` and only the last token is
    missing under an existing object; any other missing field fails."""
    try:
        return resolve(container, field)
    except (KeyError, IndexError, TypeError):
        pass
    if expected == "null":
        parent_field = field.rsplit("/", 1)[0] if field.startswith("/") else ""
        try:
            parent = resolve(container, parent_field) if parent_field else container
        except (KeyError, IndexError, TypeError):
            parent = None
        if isinstance(parent, dict):
            return ABSENT
    raise AssertionError(f"{field}: not in the result") from None


def compare(raw: Any, field: str, expected: Any, tolerance: Decimal | None) -> None:
    if raw is ABSENT:
        return  # a `null` expectation on an absent field (SPEC-0013 R4)
    try:
        want = Decimal(str(expected))
        if not want.is_finite() or tolerance is None:  # NaN, Infinity: not numbers
            raise ArithmeticError
    except ArithmeticError:
        # Not a decimal, or no tolerance (an error report): compared exactly.
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
    "expect_counts",
    "expect_defaults",
    "expect_not_computed",
    "expect_warnings",
    "expect_checks",
    "expect_errors",
}
# Sections that read a result (`expect` needed) or an error report (`expect_error` needed).
VALUE_ONLY = (
    "expect_meta",
    "expect_diagnostics",
    "expect_diagnostics_length",
    "expect_defaults",
    "expect_not_computed",
    "expect_warnings",
    "expect_checks",
)


@pytest.mark.parametrize("path", CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_golden_case_shape(path: Path) -> None:
    case = yaml.safe_load(path.read_text(encoding="utf-8"))
    assert set(case) <= NEW_CASE_KEYS, f"unknown keys: {set(case) - NEW_CASE_KEYS}"
    assert ("expect" in case) != ("expect_error" in case), "exactly one of expect, expect_error"
    if "expect" in case:
        assert "tolerance" in case, "value cases need a tolerance"
    else:
        for key in VALUE_ONLY:
            assert key not in case, f"{key} needs expect"
    for key in ("expect_error_report", "expect_errors"):
        if key in case:
            assert "expect_error" in case, f"{key} needs expect_error"
    if str(path.resolve().relative_to(GOLDEN.parents[1], walk_up=True)) not in MIGRATED:
        new_case_rules(case)


# SPEC-0013 R12: the cases of the approved migration list keep their assertions as they were;
# every other case follows the rules of decisions 2 and 4.
MIGRATED = set(
    json.loads(
        (
            GOLDEN.parents[1] / "specs/0013-golden-values-schema-shape/golden-migration.json"
        ).read_text(encoding="utf-8")
    )["files"]
)
# Lists whose entries, once asserted, need their count (decision 4).
COUNTED_SECTIONS = {
    "expect_errors": "errors",
    "expect_warnings": "warnings",
    "expect_not_computed": "not_computed",
}
WORDING = ("message", "fix")


def new_case_rules(case: dict[str, Any]) -> None:
    """Decision 4: asserted entries of errors, warnings and outputs not computed come with their
    count. Decision 2: no wording (`message`, `fix`, or `reason` outside an error)."""
    for section, name in COUNTED_SECTIONS.items():
        if case.get(section):
            counts = case.get("expect_counts", {})
            assert name in counts, f"{section} needs expect_counts {name}"
    keys: list[tuple[str, str]] = []
    for section in ("expect", "expect_meta", "expect_diagnostics", "expect_error_report"):
        keys += [(section, k) for k in case.get(section, {})]
    for section in SECTIONS:
        for entry in (case.get(section) or {}).values():
            keys += [(section, k) for k in entry or {}]
    for section, key in keys:
        last = key.rsplit("/", 1)[-1]
        error = section in ("expect_errors", "expect_error_report")
        assert last not in WORDING and (last != "reason" or error), (
            f"{section} {key}: wording is not asserted (SPEC-0013 decision 2)"
        )


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


# ---- SPEC-0013: semantic sections, counts and `null` (AC1 to AC3), on results written here


def semantic_parts() -> dict[str, Any]:
    return {
        "value": {"npv": Decimal("4.132"), "present_values": ["1", "2", "3"], "roe": None},
        "meta": {
            "envelope_version": 1,
            "defaults_applied": [
                {"pointer": "/timing", "value": "end_of_period", "reason": "r"},
                {"pointer": "/periods", "value": 5, "reason": "r"},
            ],
        },
        "diagnostics": {
            "not_computed": [
                {"pointer": "/value/roe", "reason_code": "not_defined", "missing_any_of": []},
                {
                    "pointer": "/value/ros",
                    "reason_code": "missing_input",
                    "missing_any_of": [["/revenue"], ["/sales", "/margin"]],
                },
            ],
            "warnings": [
                {"code": "FM-W012", "pointer": "/net_financial_position", "message": "m"},
                {"code": "FM-W005", "pointer": None, "message": "m"},
            ],
            "checks": [
                {"id": "a_total", "status": "reconciled", "pointer": "/value/years/0"},
                {"id": "a_total", "status": "not_checked", "pointer": "/value/years/1"},
            ],
        },
    }


SEMANTIC_REPORT: dict[str, Any] = {
    "code": "FM-E001",
    "pointer": "/cash_flows",
    "reason": "missing",
    "errors": [
        {"code": "FM-E001", "pointer": "/cash_flows", "reason": "missing"},
        {"code": "FM-E001", "pointer": "/rat", "reason": "unknown_field", "expected": ["rate"]},
        {"code": "FM-E027", "pointer": None, "reason": "requires"},
    ],
}
ERROR_CASE: dict[str, Any] = {
    "method": "tvm.npv",
    "description": "temporary",
    "input": {},
    "expect_error": "FM-E001",
}


def test_semantic_sections_pass_on_a_matching_result() -> None:
    case = TEMPORARY_CASE | {
        "expect": {"/npv": "4.13", "/roe": "null", "/irr": "null"},
        "expect_defaults": {"/timing": {"/value": "end_of_period"}, "/periods": {"/value": "5"}},
        "expect_not_computed": {
            "/value/roe": {"/reason_code": "not_defined"},
            "/value/ros": {"/missing_any_of/1/0": "/sales", "/missing_any_of#count": "2"},
        },
        "expect_warnings": {"FM-W012 /net_financial_position": {}, "FM-W005 -": {"/message": "m"}},
        "expect_checks": {"/value/years/1 a_total": {"/status": "not_checked"}},
        "expect_counts": {
            "not_computed": "2",
            "warnings": "2",
            "checks": "2",
            "/present_values": "3",
        },
    }
    check_result(case, semantic_parts())


@pytest.mark.parametrize(
    ("key", "content", "message"),
    [
        ("expect_defaults", {"/timing": {"/value": "mid_period"}}, "'end_of_period'"),
        ("expect_not_computed", {"/value/roi": {}}, "0 entries named it"),
        ("expect_not_computed", {"/value/ros": {"/missing_any_of#count": "1"}}, "length 2"),
        ("expect_warnings", {"/value/roe": {}}, "0 entries named it"),
        ("expect_checks", {"/value/years/0 b_total": {}}, "0 entries named it"),
        ("expect_counts", {"warnings": "3"}, "length 2"),
        ("expect_counts", {"/present_values": "4"}, "length 3"),
        ("expect_counts", {"errors": "0"}, "not a list of this result"),
        ("expect_errors", {"FM-E001 /rat": {}}, "needs expect_error, not expect"),
    ],
)
def test_semantic_sections_fail(key: str, content: dict[str, Any], message: str) -> None:
    case = TEMPORARY_CASE | {key: content}
    with pytest.raises(AssertionError, match=re.escape(message)):
        check_result(case, semantic_parts())


def test_a_key_naming_two_entries_fails() -> None:
    parts = semantic_parts()
    parts["diagnostics"]["warnings"].append({"code": "FM-W005", "pointer": None, "message": "m"})
    case = TEMPORARY_CASE | {"expect_warnings": {"FM-W005 -": {}}}
    with pytest.raises(AssertionError, match="2 entries named it"):
        check_result(case, parts)


@pytest.mark.parametrize(
    ("field", "expected", "passes"),
    [
        ("/roe", "null", True),
        ("/absent", "null", True),
        ("/missing/deeper", "null", False),
        ("/present_values/7", "null", False),
        ("/other", "1", False),
    ],
)
def test_null_means_null_or_absent_under_an_existing_parent(
    field: str, expected: str, passes: bool
) -> None:
    case = TEMPORARY_CASE | {"expect": {field: expected}}
    if passes:
        check_result(case, semantic_parts())
    else:
        with pytest.raises(AssertionError, match="not in the result"):
            check_result(case, semantic_parts())


def test_error_sections_name_entries_by_code_and_pointer() -> None:
    check_error(
        ERROR_CASE
        | {
            "expect_error_report": {"/pointer": "/cash_flows", "/reason": "missing"},
            "expect_errors": {
                "FM-E001 /rat": {"/reason": "unknown_field", "/expected/0": "rate"},
                "FM-E027 -": {},
            },
            "expect_counts": {"errors": "3"},
        },
        SEMANTIC_REPORT,
    )


@pytest.mark.parametrize(
    ("key", "content", "message"),
    [
        ("expect_errors", {"FM-E001 /rat": {"/reason": "missing"}}, "'unknown_field'"),
        ("expect_errors", {"FM-E001 /rate": {}}, "0 entries named it"),
        ("expect_warnings", {"FM-W005 -": {}}, "needs expect, not expect_error"),
        ("expect_counts", {"errors": "2"}, "length 3"),
        ("expect_counts", {"warnings": "0"}, "not a list of this result"),
    ],
)
def test_error_sections_fail(key: str, content: dict[str, Any], message: str) -> None:
    with pytest.raises(AssertionError, match=re.escape(message)):
        check_error(ERROR_CASE | {key: content}, SEMANTIC_REPORT)


def test_case_shape_accepts_the_semantic_sections(tmp_path: Path) -> None:
    case = TEMPORARY_CASE | {
        "source": "temporary",
        "expect_not_computed": {"/value/roe": {}},
        "expect_counts": {"not_computed": "1"},
    }
    path = tmp_path / "case.yaml"
    path.write_text(yaml.safe_dump(case), encoding="utf-8")
    test_golden_case_shape(path)
    error = ERROR_CASE | {
        "source": "temporary",
        "expect_errors": {"FM-E001 /rat": {}},
        "expect_counts": {"errors": "3"},
    }
    path.write_text(yaml.safe_dump(error), encoding="utf-8")
    test_golden_case_shape(path)
    path.write_text(yaml.safe_dump(error | {"expect_checks": {"/value a": {}}}), encoding="utf-8")
    with pytest.raises(AssertionError, match="needs expect"):
        test_golden_case_shape(path)


# ---- SPEC-0013 R12: rules for new cases (decisions 2 and 4), on temporary files


def write_case(tmp_path: Path, case: dict[str, Any]) -> Path:
    path = tmp_path / "case.yaml"
    path.write_text(yaml.safe_dump(case), encoding="utf-8")
    return path


@pytest.mark.parametrize(
    ("extra", "message"),
    [
        ({"expect_errors": {"FM-E001 /rat": {}}}, "needs expect_counts errors"),
        (
            {"expect_errors": {"FM-E001 /rat": {"/fix": "x"}}, "expect_counts": {"errors": "1"}},
            "/fix",
        ),
        ({"expect_error_report": {"/message": "m"}}, "/message"),
    ],
)
def test_a_new_error_case_follows_the_rules(
    tmp_path: Path, extra: dict[str, Any], message: str
) -> None:
    case = ERROR_CASE | {"source": "temporary"} | extra
    with pytest.raises(AssertionError, match=re.escape(message)):
        test_golden_case_shape(write_case(tmp_path, case))


@pytest.mark.parametrize(
    ("extra", "message"),
    [
        ({"expect_warnings": {"FM-W005 -": {}}}, "needs expect_counts warnings"),
        ({"expect_not_computed": {"/value/roe": {}}}, "needs expect_counts not_computed"),
        (
            {
                "expect_not_computed": {"/value/roe": {"/reason": "r"}},
                "expect_counts": {"not_computed": "1"},
            },
            "/reason",
        ),
        ({"expect": {"/npv": "4.13", "/note/reason": "r"}}, "/reason"),
    ],
)
def test_a_new_value_case_follows_the_rules(
    tmp_path: Path, extra: dict[str, Any], message: str
) -> None:
    case = TEMPORARY_CASE | {"source": "temporary"} | extra
    with pytest.raises(AssertionError, match=re.escape(message)):
        test_golden_case_shape(write_case(tmp_path, case))


def test_new_case_rules_spare_the_migrated_cases() -> None:
    # A case of the approved migration list keeps its wording assertions (decision 2).
    path = GOLDEN / "tvm.npv" / "error-report-misspelt-fields.yaml"
    assert str(path.relative_to(GOLDEN.parents[1])) in MIGRATED
    test_golden_case_shape(path)
