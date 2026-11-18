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
import itertools
import json
import re
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
    "golden_schema", ROOT / "scripts" / "golden_schema.py"
)
assert _spec and _spec.loader
gs = importlib.util.module_from_spec(_spec)
sys.modules["golden_schema"] = gs
_spec.loader.exec_module(gs)

APPLICATORS = ("anyOf", "oneOf", "allOf")  # choices first: `closed` reads [:2]
# The `$defs` of the envelope snapshot that describe one entry of each semantic section.
ENTRY_DEFS = {
    "expect_defaults": "DefaultApplied",
    "expect_not_computed": "NotComputed",
    "expect_warnings": "Warning",
    "expect_checks": "Check",
    "expect_errors": "FairmarkErrorEntry",
}
# Where `expect_error_report` reads: the error report, by its `$defs` in the envelope.
PART_DEFS = {"expect_error_report": "FairmarkErrorReport"}


# ---------------------------------------------------------------- snapshots


@functools.cache
def history() -> dict[str, Any]:
    """The schema history, read the way ADR 0017 requires (`schema_history()` once present)."""
    return gs.schema_history(ROOT)


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


def closed(node: Any, branch: bool = False, root: Any = None) -> Any:
    """`node` with `unevaluatedProperties: false` on every object schema that is not a direct
    branch of `anyOf`, `oneOf` or `allOf` and either declares `properties` or is a choice
    whose branches declare them (its parent closes a branch, so the fields a branch shares
    with its parent stay allowed)."""
    root = node if root is None else root
    if isinstance(node, list):
        return [closed(x, branch, root) for x in node]
    if not isinstance(node, dict):
        return node
    out = {k: closed(v, k in APPLICATORS, root) for k, v in node.items()}
    branches = [b for k in APPLICATORS[:2] for b in node.get(k, [])]
    choice = any("properties" in gs.deref(root, b) for b in branches)
    if (
        ("properties" in out or choice)
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
    nodes = gs.branches(root, node)
    for marker, kind in (("x-fairmark-output-of", "output"), ("x-fairmark-input-of", "input")):
        pointer = next((n[marker] for n in nodes if marker in n), None)
        if pointer is not None:
            parts = gs.tokens(pointer)
            method = gs.resolve(holder, parts)
            version = gs.resolve(holder, [*parts[:-1], "version"])
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
    nodes = gs.branches(root, root)
    holder: Any = None
    for t in parts:
        marker = next(
            (n["x-fairmark-output-of"] for n in nodes if "x-fairmark-output-of" in n), None
        )
        if marker is not None and holder is not None:
            ptr = gs.tokens(marker)
            inner = output_snapshot(
                gs.resolve(holder, ptr), str(gs.resolve(holder, [*ptr[:-1], "version"]))
            )
            if inner is None:
                return []
            root, nodes = inner, gs.branches(inner, inner)
        nxt: list[dict[str, Any]] = []
        for n in nodes:
            if t.isdigit() and "items" in n:
                nxt.extend(gs.branches(root, n["items"]))
            props = n.get("properties", {})
            if t in props:
                nxt.extend(gs.branches(root, props[t]))
            extra = n.get("additionalProperties")
            if isinstance(extra, dict) and t not in props and not t.isdigit():
                nxt.extend(gs.branches(root, extra))
        nodes = nxt
        holder = data
        try:
            data = data[int(t)] if isinstance(data, list) else data[t]
        except (KeyError, IndexError, TypeError, ValueError):
            data = None
    return nodes


def undeclared_keys(
    case: dict[str, Any], value: Any, envelope_version: str, method_version: str
) -> list[str]:
    """Assertion keys of `case` the schemas do not declare, in the method version the result
    declares: the guard against a typo once a `null` expectation passes on an absent key
    (SPEC-0013 R4, R8)."""
    method = case["method"]
    found = []
    version = method_version
    output = output_snapshot(method, version) or {}
    envelope = envelope_snapshot(envelope_version) or {"$defs": {}}

    def defs_root(name: str) -> dict[str, Any]:
        return {"$defs": envelope["$defs"], "$ref": f"#/$defs/{name}"}

    def check(section: str, root: dict[str, Any], key: str, data: Any) -> None:
        parts = gs.tokens(key.removesuffix("#count"))
        if not nodes_at(root, parts, data):
            found.append(f"{section} {key}: not declared by the schema")

    value_keys = list(case.get("expect", {}))
    if "expect_error" not in case:
        for key in value_keys:
            check("expect", output, key, value)
        value_counts = [k for k in case.get("expect_counts", {}) if k.startswith("/")]
        for key in value_counts:
            check("expect_counts", output, key, value)
    for section, name in PART_DEFS.items():
        for key in case.get(section, {}):
            check(section, defs_root(name), key, None)
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
        meta = raw.get("meta", {})
        envelope_version = str(meta.get("envelope_version", envelope_version))
        found += undeclared_keys(
            case, raw.get("value"), envelope_version, str(meta.get("method_version"))
        )
    else:
        assert report is not None
        found = error_problems(report, envelope_version)
        current = str(history()["methods"][case["method"]]["current"])
        found += undeclared_keys(case, None, envelope_version, current)
    assert not found, found[:5]


def test_every_golden_file_is_a_parametrized_case() -> None:
    # Code review of #394, N9: the parametrized cases are exactly the golden files, each with
    # its own id (no file hidden by a duplicate id).
    for test in (
        test_golden_result_conforms_to_the_committed_snapshots,
        test_envelope_lists_follow_their_documented_order,
    ):
        mark = next(m for m in test.pytestmark if m.name == "parametrize")  # type: ignore[attr-defined]
        assert list(mark.args[1]) == sorted(GOLDEN.rglob("*.yaml")) == CASES
        ids = [mark.kwargs["ids"](p) for p in CASES]
        assert len(set(ids)) == len(CASES) > 600


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
    version = str(raw["meta"]["method_version"])
    assert (undeclared_keys(case, raw["value"], "1", version) == []) is declared


# ---------------------------------------------------------------- documented orders (R9)


def pointer_key(pointer: str) -> list[tuple[int, int, str]]:
    """The order of pointers in every list of the result: token by token, array indices
    (`0`, or digits without a leading zero) as numbers and before names (SPEC-0012 R13)."""
    out = []
    for t in pointer.split("/"):
        index = re.fullmatch(r"0|[1-9][0-9]*", t) is not None
        out.append((0, int(t), "") if index else (1, 0, t))
    return out


def unsorted(name: str, keys: list[Any]) -> list[str]:
    return [] if keys == sorted(keys) else [f"{name}: not in its documented order"]


@functools.cache
def input_order_schema(method: str) -> dict[str, Any]:
    """The input schema with its property order. The committed snapshots sort their keys
    (`scripts/schema_snapshots.py`), so the order of the input is read from the engine; the
    shape is still validated against the snapshots only (AC7)."""
    schema: dict[str, Any] = fm.input_schema(method)
    return schema


def default_order_problems(method: str, version: str, pointers: list[str]) -> list[str]:
    if str(history()["methods"][method]["current"]) != version:
        return [f"defaults_applied: {method} v{version} is not the version the engine runs"]
    root = input_order_schema(method)
    found = []
    for a, b in itertools.pairwise(pointers):
        if gs.input_order_key(root, a) > gs.input_order_key(root, b):
            found.append(f"defaults_applied: {a} before {b}, against the input order")
    return found


def warning_order_key(warning: dict[str, Any]) -> tuple[Any, ...]:
    """Method warnings first, in the order the method emits them; then every FM-W001 by
    pointer; then every FM-W017 by pointer (R9)."""
    rank = {"FM-W001": 1, "FM-W017": 2}.get(warning["code"], 0)
    return (rank, pointer_key(warning.get("pointer") or "") if rank else [])


def order_problems(raw: dict[str, Any]) -> list[str]:
    """The envelope lists of a result that depart from their documented order (SPEC-0013 R9):
    defaults applied in input order; warnings of the method first, then FM-W001 and FM-W017 by
    pointer; outputs not computed by pointer; checks by result block, then id."""
    diagnostics = raw["diagnostics"]
    meta = raw["meta"]
    found = default_order_problems(
        raw["method"],
        str(meta["method_version"]),
        [d["pointer"] for d in meta["defaults_applied"]],
    )
    found += unsorted("warnings", [warning_order_key(w) for w in diagnostics["warnings"]])
    found += unsorted(
        "not_computed", [pointer_key(n["pointer"]) for n in diagnostics["not_computed"]]
    )
    found += unsorted(
        "checks", [(pointer_key(c["pointer"]), c["id"]) for c in diagnostics["checks"]]
    )
    return found


def error_order_problems(report: dict[str, Any]) -> list[str]:
    """The entries of an error report out of the canonical order: by pointer (none first),
    then code, then message (issue #370)."""
    keys = [
        (e.get("pointer") is not None, pointer_key(e.get("pointer") or ""), e["code"], e["message"])
        for e in report.get("errors", [])
    ]
    return unsorted("errors", keys)


@pytest.mark.parametrize("path", CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_envelope_lists_follow_their_documented_order(path: Path) -> None:
    case = yaml.safe_load(path.read_text(encoding="utf-8"))
    raw, report = run_case(case)
    found = order_problems(raw) if raw is not None else error_order_problems(report or {})
    assert not found, found


def ordered_result() -> dict[str, Any]:
    """A `dcf.fcff` result whose envelope lists follow their documented orders (R9)."""
    version = str(history()["methods"]["dcf.fcff"]["current"])
    return {
        "method": "dcf.fcff",
        "meta": {
            "method_version": version,
            # Input schema order: `stages` (its `periods` before `projection`), then
            # `cash_flow_terms`, then `timing`.
            "defaults_applied": [
                {"pointer": "/stages/0/periods"},
                {"pointer": "/stages/0/projection"},
                {"pointer": "/stages/1/periods"},
                {"pointer": "/cash_flow_terms"},
                {"pointer": "/timing"},
            ],
        },
        "diagnostics": {
            "not_computed": [{"pointer": "/value/periods/2"}, {"pointer": "/value/periods/10"}],
            # Method warnings first, then FM-W001 by pointer, then FM-W017 by pointer.
            "warnings": [
                {"code": "FM-W012", "pointer": "/net_financial_position"},
                {"code": "FM-W005", "pointer": None},
                {"code": "FM-W001", "pointer": "/stages/2/flows"},
                {"code": "FM-W001", "pointer": "/stages/10/flows"},
                {"code": "FM-W017", "pointer": "/rate"},
                {"code": "FM-W017", "pointer": "/wacc"},
            ],
            "checks": [
                {"pointer": "/value/years/0", "id": "b"},
                {"pointer": "/value/years/1", "id": "a"},
            ],
        },
    }


def test_an_ordered_result_has_no_order_problem() -> None:
    assert order_problems(ordered_result()) == []


@pytest.mark.parametrize(
    ("part", "name"),
    [
        ("meta", "defaults_applied"),
        ("diagnostics", "not_computed"),
        ("diagnostics", "warnings"),
        ("diagnostics", "checks"),
    ],
)
def test_a_reversed_list_is_caught(part: str, name: str) -> None:
    raw = ordered_result()
    raw[part][name].reverse()
    assert any(p.startswith(name) for p in order_problems(raw)), name


def test_a_method_warning_after_the_reading_warnings_is_caught() -> None:
    raw = ordered_result()
    warnings = raw["diagnostics"]["warnings"]
    warnings.append(warnings.pop(0))  # FM-W012 after the FM-W017 entries
    assert any(p.startswith("warnings") for p in order_problems(raw))


def test_reading_warnings_out_of_pointer_order_are_caught() -> None:
    raw = ordered_result()
    warnings = raw["diagnostics"]["warnings"]
    warnings[2], warnings[3] = warnings[3], warnings[2]  # FM-W001 /stages/10 before /stages/2
    assert any(p.startswith("warnings") for p in order_problems(raw))


def test_a_default_out_of_input_schema_order_is_caught() -> None:
    raw = ordered_result()
    defaults = raw["meta"]["defaults_applied"]
    defaults[3], defaults[4] = defaults[4], defaults[3]  # `timing` before `cash_flow_terms`
    assert any(p.startswith("defaults_applied") for p in order_problems(raw))


def test_error_entries_follow_pointer_then_code_then_message() -> None:
    entries = [
        {"pointer": None, "code": "FM-E027", "message": "m"},
        {"pointer": "/a/2", "code": "FM-E001", "message": "m"},
        {"pointer": "/a/10", "code": "FM-E001", "message": "m"},
        {"pointer": "/a/10", "code": "FM-E004", "message": "a"},
        {"pointer": "/a/10", "code": "FM-E004", "message": "b"},
    ]
    assert error_order_problems({"errors": entries}) == []
    for i in range(len(entries) - 1):
        swapped = [*entries]
        swapped[i], swapped[i + 1] = swapped[i + 1], swapped[i]
        assert error_order_problems({"errors": swapped}), i


def test_a_sensitivity_cell_with_an_unknown_field_is_caught() -> None:
    # Code review of #394, S3: a cell is a choice (`anyOf` of `value` and `error`) whose node
    # declares no property of its own; closed mode closes it too.
    method, raw = golden_result("analysis.sensitivity")
    assert result_problems(method, raw) == []
    raw["value"]["cells"][0][0]["surprise"] = "1"
    assert any("surprise" in p for p in result_problems(method, raw))


def test_declared_keys_are_read_in_the_version_the_result_declares() -> None:
    # Code review of #394, N8: `/not_computed` is an output of ratios.profitability v2 only.
    case = {"method": "ratios.profitability", "expect": {"/not_computed": "null"}}
    assert undeclared_keys(case, {}, "1", "2") == []
    assert undeclared_keys(case, {}, "1", "3") != []
