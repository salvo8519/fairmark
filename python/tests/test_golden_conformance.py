"""Golden results conform to the committed schema snapshots (SPEC-0013 R7, R8; AC6 to AC8).

Golden cases assert values; this module asserts shape. Every golden result, success or error,
is validated against the snapshots committed under `schemas/` for the versions the result
declares (never against schemas the engine builds), in closed mode: an object that declares
`properties` accepts no other key. The snapshots themselves stay open, since a new output field
is additive (ADR 0014 G8).
"""

from __future__ import annotations

import copy
import functools
import importlib.util
import json
import sys
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import jsonschema
import pytest
import yaml

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
SCHEMAS = ROOT / "schemas"
GOLDEN = ROOT / "tests" / "golden"
CASES = sorted(GOLDEN.rglob("*.yaml"))

_spec = importlib.util.spec_from_file_location(
    "golden_values_migration", ROOT / "scripts" / "golden_values_migration.py"
)
assert _spec and _spec.loader
gvm = importlib.util.module_from_spec(_spec)
sys.modules["golden_values_migration"] = gvm
_spec.loader.exec_module(gvm)

APPLICATORS = ("anyOf", "oneOf", "allOf")
# The `$defs` of the envelope snapshot that describe one entry of each semantic section.
ENTRY_DEFS = {
    "expect_defaults": "DefaultApplied",
    "expect_not_computed": "NotComputed",
    "expect_warnings": "Warning",
    "expect_checks": "Check",
    "expect_errors": "FairmarkErrorEntry",
}
# Where the other sections read: a part of the result, by its `$defs` in the envelope.
PART_DEFS = {
    "expect_meta": "Meta",
    "expect_diagnostics": "Diagnostics",
    "expect_diagnostics_length": "Diagnostics",
    "expect_error_report": "FairmarkErrorReport",
}


# ---------------------------------------------------------------- snapshots


@functools.cache
def history() -> dict[str, Any]:
    """The schema history, read the way ADR 0017 requires (`schema_history()` once present)."""
    return gvm.schema_history(ROOT)


def read(relative: str) -> dict[str, Any]:
    data: dict[str, Any] = json.loads((SCHEMAS / relative).read_text(encoding="utf-8"))
    return data


@functools.cache
def output_snapshot(method: str, version: str) -> dict[str, Any] | None:
    """The committed output snapshot of `method` at `version`, or None if it has none."""
    entry = history()["methods"].get(method, {}).get("versions", {}).get(str(version))
    if not entry or "output_file" not in entry:
        return None
    return read(entry["output_file"])


@functools.cache
def input_snapshot(method: str, version: str) -> dict[str, Any] | None:
    entry = history()["methods"].get(method, {}).get("versions", {}).get(str(version))
    return read(entry["file"]) if entry else None


@functools.cache
def envelope_snapshot(version: str) -> dict[str, Any] | None:
    entry = history()["envelope"]["versions"].get(str(version))
    return read(entry["file"]) if entry else None


def closed(node: Any, branch: bool = False) -> Any:
    """`node` with `unevaluatedProperties: false` on every object schema that declares
    `properties` and is not a direct branch of `anyOf`, `oneOf` or `allOf` (its parent closes
    it, so the fields a branch shares with its parent stay allowed)."""
    if isinstance(node, list):
        return [closed(x, branch) for x in node]
    if not isinstance(node, dict):
        return node
    out = {k: closed(v, k in APPLICATORS) for k, v in node.items()}
    if (
        "properties" in out
        and not branch
        and "additionalProperties" not in out
        and "unevaluatedProperties" not in out
    ):
        out["unevaluatedProperties"] = False
    return out


@functools.cache
def _validator(kind: str, method: str, version: str) -> jsonschema.Draft202012Validator | None:
    if kind == "output":
        schema = output_snapshot(method, version)
    elif kind == "input":
        schema = input_snapshot(method, version)
    elif kind == "envelope":
        schema = envelope_snapshot(version)
    else:  # the error document named by the envelope (ADR 0014 G5)
        envelope = envelope_snapshot(version)
        if envelope is None:
            return None
        schema = {
            "$schema": envelope["$schema"],
            "$defs": envelope["$defs"],
            "$ref": envelope["x-fairmark-error"],
        }
    if schema is None:
        return None
    return jsonschema.Draft202012Validator(
        closed(schema), format_checker=jsonschema.Draft202012Validator.FORMAT_CHECKER
    )


def problems_of(kind: str, method: str, version: str, document: Any) -> list[str]:
    validator = _validator(kind, method, str(version))
    if validator is None:
        return [f"no committed {kind} snapshot for {method} v{version}"]
    return [
        f"{kind} {method} v{version} /{'/'.join(map(str, e.absolute_path))}: {e.message}"
        for e in validator.iter_errors(document)
    ]


# ---------------------------------------------------------------- inner values of analyses


def inner_values(root: dict[str, Any], node: Any, data: Any, holder: Any) -> Iterator[tuple]:
    """(kind, method, version, value) of every value of `data` whose schema node carries
    `x-fairmark-output-of` or `x-fairmark-input-of`: a pointer, relative to the object holding
    the value, to the inner method id; the version is the `version` beside that id."""
    nodes = gvm._branches(root, node)
    for marker, kind in (("x-fairmark-output-of", "output"), ("x-fairmark-input-of", "input")):
        pointer = next((n[marker] for n in nodes if marker in n), None)
        if pointer is not None:
            parts = gvm.tokens(pointer)
            method = gvm.resolve(holder, parts)
            version = gvm.resolve(holder, [*parts[:-1], "version"])
            yield kind, method, str(version), data
            return
    if isinstance(data, dict):
        for n in nodes:
            for key, sub in n.get("properties", {}).items():
                if key in data:
                    yield from inner_values(root, sub, data[key], data)
    elif isinstance(data, list):
        for n in nodes:
            if "items" in n:
                for item in data:
                    yield from inner_values(root, n["items"], item, holder)


def result_problems(method: str, raw: dict[str, Any]) -> list[str]:
    """Every way a success envelope departs from the snapshots of the versions it declares."""
    meta = raw.get("meta") if isinstance(raw.get("meta"), dict) else {}
    envelope_version = str(meta.get("envelope_version"))
    method_version = str(meta.get("method_version"))
    found = problems_of("envelope", method, envelope_version, raw)
    if raw.get("method") != method:
        found.append(f"envelope names {raw.get('method')!r}, not {method}")
    value = raw.get("value")
    found += problems_of("output", method, method_version, value)
    root = output_snapshot(method, method_version)
    if root is not None and isinstance(value, dict):
        for kind, inner, version, data in inner_values(root, root, value, value):
            found += problems_of(kind, inner, version, data)
    return found


def error_problems(report: dict[str, Any], envelope_version: str) -> list[str]:
    return problems_of("error", "", envelope_version, {"error": report})


# ---------------------------------------------------------------- declared keys (R8)


def nodes_at(root: dict[str, Any], parts: list[str], data: Any) -> list[dict[str, Any]]:
    """The schema nodes at `parts` from `root`, switching to the inner method's output
    snapshot where a node carries `x-fairmark-output-of` (resolved against `data`)."""
    nodes = gvm._branches(root, root)
    holder: Any = None
    for t in parts:
        marker = next(
            (n["x-fairmark-output-of"] for n in nodes if "x-fairmark-output-of" in n), None
        )
        if marker is not None and holder is not None:
            ptr = gvm.tokens(marker)
            inner = output_snapshot(
                gvm.resolve(holder, ptr), str(gvm.resolve(holder, [*ptr[:-1], "version"]))
            )
            if inner is None:
                return []
            root, nodes = inner, gvm._branches(inner, inner)
        nxt: list[dict[str, Any]] = []
        for n in nodes:
            if t.isdigit() and "items" in n:
                nxt.extend(gvm._branches(root, n["items"]))
            props = n.get("properties", {})
            if t in props:
                nxt.extend(gvm._branches(root, props[t]))
            extra = n.get("additionalProperties")
            if isinstance(extra, dict) and t not in props and not t.isdigit():
                nxt.extend(gvm._branches(root, extra))
        nodes = nxt
        holder = data
        try:
            data = data[int(t)] if isinstance(data, list) else data[t]
        except (KeyError, IndexError, TypeError, ValueError):
            data = None
    return nodes


def undeclared_keys(case: dict[str, Any], value: Any, envelope_version: str) -> list[str]:
    """Assertion keys of `case` the schemas do not declare: the guard against a typo once a
    `null` expectation passes on an absent key (SPEC-0013 R4, R8)."""
    method = case["method"]
    found = []
    version = str(history()["methods"][method]["current"])
    output = output_snapshot(method, version) or {}
    envelope = envelope_snapshot(envelope_version) or {"$defs": {}}

    def defs_root(name: str) -> dict[str, Any]:
        return {"$defs": envelope["$defs"], "$ref": f"#/$defs/{name}"}

    def check(section: str, root: dict[str, Any], key: str, data: Any) -> None:
        parts = gvm.tokens(key.removesuffix("#count"))
        if not nodes_at(root, parts, data):
            found.append(f"{section} {key}: not declared by the schema")

    value_keys = list(case.get("expect", {})) + list(case.get("expect_length", {}))
    if "expect_error" not in case:
        for key in value_keys:
            check("expect", output, key, value)
        value_counts = [k for k in case.get("expect_counts", {}) if k.startswith("/")]
        for key in value_counts:
            check("expect_counts", output, key, value)
    for section, name in PART_DEFS.items():
        for key in case.get(section, {}):
            check(section, defs_root(name), key, None)
    if "expect_error" in case:
        for key in case.get("expect_length", {}):
            check("expect_length", defs_root("FairmarkErrorReport"), key, None)
    for section, name in ENTRY_DEFS.items():
        for entry in (case.get(section) or {}).values():
            for key in entry or {}:
                check(section, defs_root(name), key, None)
    return found


# ---------------------------------------------------------------- the golden cases


def run_case(case: dict[str, Any]) -> tuple[dict[str, Any] | None, dict[str, Any] | None]:
    """(success envelope, error report) of a golden input, as the engine returns them."""
    try:
        return dict(fm.run_json(case["method"], case["input"])), None
    except fm.FairmarkError as err:
        return None, err.report


@pytest.mark.parametrize("path", CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_golden_result_conforms_to_the_committed_snapshots(path: Path) -> None:
    case = yaml.safe_load(path.read_text(encoding="utf-8"))
    raw, report = run_case(case)
    envelope_version = str(history()["envelope"]["current"])
    if raw is not None:
        found = result_problems(case["method"], raw)
        envelope_version = str(raw.get("meta", {}).get("envelope_version", envelope_version))
        found += undeclared_keys(case, raw.get("value"), envelope_version)
    else:
        assert report is not None
        found = error_problems(report, envelope_version)
        found += undeclared_keys(case, None, envelope_version)
    assert not found, found[:5]


def test_every_golden_case_was_checked() -> None:
    assert len(CASES) > 600


# ---------------------------------------------------------------- the test catches defects


def golden_result(folder: str, name: str | None = None) -> tuple[str, dict[str, Any]]:
    paths = sorted((GOLDEN / folder).glob("*.yaml"))
    for path in paths:
        if name is not None and path.stem != name:
            continue
        case = yaml.safe_load(path.read_text(encoding="utf-8"))
        raw, _ = run_case(case)
        if raw is not None:
            return case["method"], raw
    raise AssertionError(f"no value case in {folder}")


def npv() -> tuple[str, dict[str, Any]]:
    return golden_result("tvm.npv")


def test_a_conforming_result_has_no_problem() -> None:
    method, raw = npv()
    assert result_problems(method, raw) == []


def mutated(change: str) -> tuple[str, dict[str, Any]]:
    method, raw = npv()
    raw = copy.deepcopy(raw)
    if change == "field added to value":
        raw["value"]["npv_rounded"] = "1"
    elif change == "field renamed":
        raw["value"]["net_present_value"] = raw["value"].pop("npv")
    elif change == "envelope key missing":
        del raw["diagnostics"]
    elif change == "decimal as a JSON number":
        raw["value"]["npv"] = 4.13
    elif change == "enumerated value not in the schema":
        raw["diagnostics"]["checks"] = [
            {"id": "x", "status": "maybe", "pointer": "/value", "pointers": []}
        ]
    elif change == "version without a snapshot":
        raw["meta"]["method_version"] = "99"
    return method, raw


@pytest.mark.parametrize(
    "change",
    [
        "field added to value",
        "field renamed",
        "envelope key missing",
        "decimal as a JSON number",
        "enumerated value not in the schema",
        "version without a snapshot",
    ],
)
def test_a_layout_defect_is_caught(change: str) -> None:
    method, raw = mutated(change)
    assert result_problems(method, raw), change


def test_a_value_checked_against_another_version_is_caught() -> None:
    method, raw = golden_result("ratios.profitability")
    assert result_problems(method, raw) == []
    raw["meta"]["method_version"] = "2"  # the snapshot of v2, which v3 changed
    assert result_problems(method, raw)


def test_a_date_that_is_not_a_date_is_caught() -> None:
    method, raw = golden_result("dates.schedule")
    assert result_problems(method, raw) == []
    raw["value"]["start_dates"][0] = "2024-02-30"
    assert any("is not a 'date'" in p for p in result_problems(method, raw))


def test_an_error_report_with_an_extra_field_is_caught() -> None:
    case = yaml.safe_load((GOLDEN / "tvm.npv" / "error-report-misspelt-fields.yaml").read_text())
    _, report = run_case(case)
    assert report is not None
    assert error_problems(report, "1") == []
    report = copy.deepcopy(report)
    report["errors"][0]["hint"] = "x"
    assert error_problems(report, "1")


def test_an_inner_value_with_an_unknown_field_is_caught() -> None:
    method, raw = golden_result("analysis.goal_seek")
    assert result_problems(method, raw) == []
    raw["value"]["inner_value"]["surprise"] = "1"
    assert any("surprise" in p for p in result_problems(method, raw))


def test_no_schema_is_read_from_the_engine(monkeypatch: pytest.MonkeyPatch) -> None:
    # AC7: the snapshots on disk are the reference, not the schemas the engine builds.
    for name in ("result_schema", "output_schema", "input_schema"):
        monkeypatch.setattr(fm, name, lambda *a, **k: pytest.fail("schema read from the engine"))
    _validator.cache_clear()
    method, raw = golden_result("analysis.goal_seek")
    assert result_problems(method, raw) == []


@pytest.mark.parametrize(("key", "declared"), [("/npvv", False), ("/npv", True)])
def test_a_misspelt_key_is_caught_even_when_null_is_expected(key: str, declared: bool) -> None:
    # AC8: a `null` expectation passes on an absent key (R4); the schema catches the typo.
    method, raw = npv()
    case = {"method": method, "expect": {key: "null"}}
    assert (undeclared_keys(case, raw["value"], "1") == []) is declared
