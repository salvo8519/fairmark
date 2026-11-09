"""Output schemas and schema-driven decoding of results (issue #265, ADR 0014 G7 and G8)."""

from __future__ import annotations

import re
from decimal import Decimal
from pathlib import Path
from typing import Any

import pytest
import yaml

import fairmark as fm
from fairmark import _core

GOLDEN = Path(__file__).resolve().parents[2] / "tests" / "golden"
VALUE_CASES = [
    p
    for p in sorted(GOLDEN.rglob("*.yaml"))
    if "expect" in yaml.safe_load(p.read_text(encoding="utf-8"))
]
METHODS = [m["id"] for m in fm.methods()]
# Fields of the golden results whose text looks like a number.
TEXT_FIELDS = {"technique_version", "method_version", "label", "name", "id", "version"}
NUMERIC = re.compile(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?$")


def case_of(path: Path) -> dict[str, Any]:
    case: dict[str, Any] = yaml.safe_load(path.read_text(encoding="utf-8"))
    return case


@pytest.mark.parametrize("method", METHODS)
def test_every_method_publishes_its_output_and_result_schemas(method: str) -> None:
    output = fm.output_schema(method)
    assert output["$schema"] == "https://json-schema.org/draft/2020-12/schema"
    assert output["type"] == "object" and output["description"]
    result = fm.result_schema(method)
    assert set(result["required"]) == {"method", "value", "explain", "meta"}
    # The copies are independent: a caller cannot alter the schema used for decoding.
    output["type"] = "changed"
    assert fm.output_schema(method)["type"] == "object"


def test_unknown_method_schema_is_an_error() -> None:
    with pytest.raises(fm.FairmarkError) as err:
        fm.output_schema("tvm.nope")
    assert err.value.code == "FM-E007"


def test_decimal_patterns_are_the_engines() -> None:
    # The decoder recognises a decimal by the pattern of its schema: the output pattern and
    # the input pattern (for an input held in a result) must be the engine's.
    assert fm.output_schema("tvm.npv")["properties"]["npv"]["pattern"] in _core._DECIMAL_PATTERNS
    assert fm.input_schema("tvm.npv")["properties"]["rate"]["pattern"] in _core._DECIMAL_PATTERNS


@pytest.mark.parametrize("path", VALUE_CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_golden_results_match_their_result_schema(path: Path) -> None:
    jsonschema = pytest.importorskip("jsonschema")
    case = case_of(path)
    raw = fm.run_json(case["method"], case["input"])
    validator = jsonschema.Draft202012Validator(fm.result_schema(case["method"]))
    errors = [f"{list(e.absolute_path)}: {e.message}" for e in validator.iter_errors(raw)]
    assert not errors, errors[:3]


def same_leaves(raw: Any, decoded: Any, path: str, text: list[str]) -> None:
    """Decoding changes only the type of a decimal, exactly; every text stays as it was."""
    if isinstance(raw, dict):
        assert isinstance(decoded, dict) and raw.keys() == decoded.keys(), path
        for key in raw:
            same_leaves(raw[key], decoded[key], f"{path}/{key}", text)
    elif isinstance(raw, list):
        assert isinstance(decoded, list) and len(raw) == len(decoded), path
        for i, (r, d) in enumerate(zip(raw, decoded, strict=True)):
            same_leaves(r, d, f"{path}/{i}", text)
    elif isinstance(decoded, Decimal):
        # Exact: the same digits and exponent, no rounding, no normalisation.
        assert decoded.as_tuple() == Decimal(raw).as_tuple(), f"{path}: {raw!r} -> {decoded!r}"
    else:
        assert type(decoded) is type(raw) and decoded == raw, path
        if isinstance(raw, str) and NUMERIC.match(raw):
            text.append(path)


@pytest.mark.parametrize("path", VALUE_CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_decoding_of_golden_results_is_exact(path: Path) -> None:
    case = case_of(path)
    result = fm.run(case["method"], case["input"])
    text: list[str] = []
    same_leaves(result.raw["value"], result.value, "", text)
    # Text that looks like a number stays text only where the schema says so: versions,
    # labels, names and ids.
    for p in text:
        assert p.rsplit("/", 1)[-1] in TEXT_FIELDS, p
    assert all(isinstance(c, str) for c in result.conventions)
    assert all(isinstance(s.value, Decimal) for s in result.steps)


class StrictDecoder(_core._Decoder):
    """Fails when a value fits two branches of an `anyOf` that would decode it differently."""

    def branch(self, members: list[Any], node: Any, inherited: frozenset[str] = frozenset()) -> Any:
        fitting = [
            m
            for m in members
            if (
                self.properties(m, node, inherited) is not None
                if isinstance(node, dict)
                else self.fits(m, node)
            )
        ]
        decodings = {repr(self.decode(m, node)) for m in fitting}
        assert len(decodings) <= 1, f"ambiguous branches for {node!r}"
        return fitting[0] if fitting else None


@pytest.mark.parametrize("path", VALUE_CASES, ids=lambda p: f"{p.parent.name}/{p.stem}")
def test_no_golden_result_is_ambiguous(path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    case = case_of(path)
    expected = fm.run(case["method"], case["input"]).value
    monkeypatch.setattr(_core, "_Decoder", StrictDecoder)
    assert fm.run(case["method"], case["input"]).value == expected


def test_numeric_looking_text_stays_text() -> None:
    # Scenario names, output pointers and inner error messages are text.
    r = fm.analysis.scenarios(
        "tvm.npv",
        {"rate": "0.1", "cash_flows": [-100, 60, 60]},
        outputs=["/npv"],
        cases=[("2025", {"/rate": "0.05"}), ("2026", {"/rate": "-1"})],
    )
    first, failed = r.value["scenarios"]
    assert first["name"] == "2025" and first["value"][0]["output"] == "/npv"
    assert isinstance(first["value"][0]["delta"], Decimal)
    assert failed["name"] == "2026" and failed["error"]["code"] == "FM-E003"
    assert r.value["inner_method"] == {"id": "tvm.npv", "version": "2"}
    # A year label that is a number stays text; its amounts are decimals.
    statements = case_of(GOLDEN / "statements.reorganise" / "balance-sheet-ordinary.yaml")
    year = fm.run("statements.reorganise", statements["input"]).value["years"][0]
    assert year["label"] == "2024" and year["date"] == "2024-12-31"
    assert isinstance(year["balance_sheet"]["operating"]["invested_capital"], Decimal)
    # The technique version of a DCF terminal value is text (it was a Decimal before #265).
    dcf = case_of(GOLDEN / "dcf.fcff" / "bridge-every-line.yaml")
    terminal = fm.run("dcf.fcff", dcf["input"]).value["terminal"]
    assert terminal["technique_version"] == "1" and isinstance(terminal["technique"], str)


def test_inner_input_and_value_are_decoded_by_the_inner_methods_schemas() -> None:
    seek = fm.analysis.goal_seek(
        "tvm.npv",
        {"rate": "0.05", "cash_flows": [-1000, 300, 400, 500]},
        variable="/rate",
        output="/npv",
        target="0",
        bracket=("0", "1"),
    )
    assert seek.value["inner_method"]["version"] == "2"
    assert isinstance(seek.value["solution"], Decimal)
    assert isinstance(seek.value["inner_value"]["npv"], Decimal)
    assert seek.value["solved_input"]["cash_flows"][0] == Decimal("-1000")
    assert isinstance(seek.value["solved_input"]["rate"], Decimal)


def test_numbers_in_an_input_held_in_a_result_are_read_exactly() -> None:
    seek = fm.run(
        "analysis.goal_seek",
        '{"method": "tvm.npv", "base_input": {"rate": 0.05, "cash_flows": '
        "[-1000.10000000000000000001, 300, 400, 500]}, "
        '"variable": "/rate", "output": "/npv", "target": "0", "bracket": ["0", "1"]}',
    )
    flow = seek.value["solved_input"]["cash_flows"][0]
    assert flow == Decimal("-1000.10000000000000000001") and isinstance(flow, Decimal)


def test_result_exposes_the_method_version() -> None:
    r = fm.tvm.npv("0.1", [-100, 60, 60])
    assert r.method_version == "2" == r.meta["method_version"]
    versions = {m["id"]: m["version"] for m in fm.methods()}
    assert r.method_version == versions["tvm.npv"]
