"""Tool definitions for agents (issue #264): enum value meanings, examples, compact profile,
tool subsets by area. Adapter output only: the method input schemas stay unchanged."""

from __future__ import annotations

import importlib.util
import json
import re
from pathlib import Path
from typing import Any

import jsonschema
import pytest
import yaml

import fairmark as fm
from fairmark import tools

ROOT = Path(__file__).resolve().parents[2]
GOLDEN = ROOT / "tests" / "golden"
METHODS = sorted(m["id"] for m in fm.methods())
VARIANTS = [
    pytest.param({}, id="default"),
    pytest.param({"portable": True}, id="portable"),
    pytest.param({"compact": True}, id="compact"),
    pytest.param({"portable": True, "compact": True}, id="portable-compact"),
]


def _spec(name: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / f"{name}.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


def by_method(**options: Any) -> dict[str, dict[str, Any]]:
    return {tools.method_id(d["name"]): d for d in tools.definitions(**options)}


def text(node: Any) -> str:
    """Every description in a definition, whitespace normalised, one per line."""
    found: list[str] = []

    def walk(value: Any) -> None:
        if isinstance(value, dict):
            if isinstance(value.get("description"), str):
                found.append(" ".join(value["description"].split()))
            for item in value.values():
                walk(item)
        elif isinstance(value, list):
            for item in value:
                walk(item)

    walk(node)
    return "\n".join(found)


def documented_values(schema: dict[str, Any]) -> list[tuple[str, str]]:
    """(value, meaning) of every documented enum value (schemars `oneOf` of `const`)."""
    found: list[tuple[str, str]] = []

    def walk(node: Any) -> None:
        if isinstance(node, dict):
            variants = node.get("oneOf")
            if isinstance(variants, list) and variants and all("const" in v for v in variants):
                for v in variants:
                    if v.get("description"):
                        found.append((str(v["const"]), " ".join(v["description"].split())))
            for value in node.values():
                walk(value)
        elif isinstance(node, list):
            for value in node:
                walk(value)

    walk(schema)
    return found


def golden_inputs(method: str) -> list[Any]:
    return [
        yaml.safe_load(p.read_text(encoding="utf-8"))["input"]
        for p in sorted((GOLDEN / method).glob("*.yaml"))
    ]


def plain(value: Any) -> Any:
    """YAML dates as ISO strings, as they travel in JSON."""
    return json.loads(json.dumps(value, default=str))


# 1. Enum value meanings -------------------------------------------------------------------


@pytest.mark.parametrize("options", VARIANTS)
def test_every_enum_value_keeps_its_meaning(options: dict[str, Any]) -> None:
    defs = by_method(**options)
    for method in METHODS:
        values = documented_values(fm.input_schema(method))
        definition = text(defs[method]["parameters"])
        for value, meaning in values:
            entry = f"`{value}` ({meaning.rstrip('.')})"
            assert entry in definition, f"{method}: {entry} missing"


def test_documented_enums_exist_and_collapse_to_enum() -> None:
    schema = by_method()["statements.reorganise"]["parameters"]
    assert documented_values(fm.input_schema("statements.reorganise"))
    assert '"oneOf": [{"const"' not in json.dumps(schema)
    assert "Values: `" in json.dumps(schema)


# 2. Examples ------------------------------------------------------------------------------

EXAMPLES = json.loads((ROOT / "python" / "fairmark" / "tool_examples.json").read_text())


def test_examples_file_is_current() -> None:
    assert _spec("tool_examples").main(["--check"]) == 0


def test_every_method_has_a_minimal_example_from_a_golden_case() -> None:
    assert sorted(EXAMPLES) == METHODS
    table = _spec("tool_examples").TABLE
    for method in METHODS:
        minimal, maximal = table[method]
        assert EXAMPLES[method]["minimal"]["source"] == f"tests/golden/{method}/{minimal}.yaml"
        if maximal is None:
            assert "maximal" not in EXAMPLES[method]
        else:
            assert EXAMPLES[method]["maximal"]["source"].endswith(f"/{maximal}.yaml")


@pytest.mark.parametrize("method", METHODS)
def test_examples_validate_against_the_method_schema_and_run(method: str) -> None:
    schema = fm.input_schema(method)
    for kind, example in EXAMPLES[method].items():
        source = yaml.safe_load((ROOT / example["source"]).read_text(encoding="utf-8"))
        assert "expect_error" not in source, f"{method} {kind}: source is an error case"
        assert json.loads(json.dumps(example["input"])) is not None
        jsonschema.validate(example["input"], schema)
        result = fm.run(method, example["input"])
        assert result.value is not None, f"{method} {kind}"


@pytest.mark.parametrize("options", [{}, {"compact": True}], ids=["default", "compact"])
def test_definitions_carry_the_examples(options: dict[str, Any]) -> None:
    defs = by_method(**options)
    for method in METHODS:
        examples = defs[method]["parameters"]["examples"]
        assert examples[0] == EXAMPLES[method]["minimal"]["input"]
        if "maximal" in EXAMPLES[method] and not options.get("compact"):
            assert examples[1] == EXAMPLES[method]["maximal"]["input"]
        else:
            assert len(examples) == 1
        for example in examples:
            jsonschema.validate(example, defs[method]["parameters"])


def test_portable_definitions_give_the_examples_in_the_description() -> None:
    defs = by_method(portable=True)
    for method in METHODS:
        assert "examples" not in defs[method]["parameters"]
        minimal = json.dumps(
            EXAMPLES[method]["minimal"]["input"], ensure_ascii=False, separators=(",", ":")
        )
        assert f"Minimal input example: {minimal}" in defs[method]["description"]


# 3. Compact profile -----------------------------------------------------------------------


@pytest.mark.parametrize("method", METHODS)
def test_compact_schema_accepts_what_the_full_schema_accepts(method: str) -> None:
    full = by_method()[method]["parameters"]
    compact = by_method(compact=True)[method]["parameters"]
    validator = jsonschema.Draft202012Validator(compact)
    checked = 0
    for case_input in golden_inputs(method):
        value = plain(case_input)
        if jsonschema.Draft202012Validator(full).is_valid(value):
            errors = [e.message for e in validator.iter_errors(value)]
            assert not errors, f"{method}: {errors[:3]}"
            checked += 1
    assert checked, f"{method}: no golden input checked"


def test_compact_is_smaller_and_drops_repetition() -> None:
    def size(node: Any) -> int:
        return len(json.dumps(node, ensure_ascii=False, separators=(",", ":")))

    full, compact = by_method(), by_method(compact=True)
    for method in METHODS:
        assert size(compact[method]) <= size(full[method]), method
    big = "statements.reorganise"
    assert size(compact[big]) < 0.8 * size(full[big])
    schema = json.dumps(compact[big]["parameters"])
    assert '"default": null' not in schema
    assert re.search(r'"format": "u?int\d*"', schema) is None
    assert "Decimals are" in compact[big]["description"]


def test_compact_default_is_unchanged() -> None:
    assert tools.definitions() == tools.definitions(compact=False)
    assert "Decimals are" not in by_method()["tvm.npv"]["description"]


@pytest.mark.parametrize("provider", sorted(tools.PROVIDERS))
def test_every_provider_accepts_compact_and_areas(provider: str) -> None:
    defs = tools.for_provider(provider, compact=True, areas=["tvm"])
    assert len(defs) == sum(1 for m in METHODS if m.startswith("tvm."))


# 4. Tool subsets by area ------------------------------------------------------------------


def test_areas_are_the_method_prefixes() -> None:
    assert tools.areas() == sorted({m.split(".")[0] for m in METHODS})


def test_definitions_by_area() -> None:
    names = [tools.method_id(d["name"]) for d in tools.definitions(areas=["dcf", "tvm"])]
    assert names and all(n.split(".")[0] in {"dcf", "tvm"} for n in names)
    assert len(names) == sum(1 for m in METHODS if m.split(".")[0] in {"dcf", "tvm"})
    assert len(tools.definitions(areas=None)) == len(METHODS)


def test_unknown_area_is_rejected() -> None:
    with pytest.raises(ValueError, match="Known areas"):
        tools.definitions(areas=["dfc"])
