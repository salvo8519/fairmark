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


def given_examples(definition: dict[str, Any]) -> list[Any]:
    """The examples a definition gives: the `examples` keyword, or those in the description."""
    if "examples" in definition["parameters"]:
        return list(definition["parameters"]["examples"])
    found = []
    decoder = json.JSONDecoder()
    for label in ("Minimal input example: ", "Complete input example: "):
        start = definition["description"].find(label)
        if start >= 0:
            found.append(decoder.raw_decode(definition["description"], start + len(label))[0])
    return found


def test_portable_definitions_give_the_examples_in_the_description() -> None:
    defs = by_method(portable=True)
    for method in METHODS:
        assert "examples" not in defs[method]["parameters"]
        assert "Minimal input example: {" in defs[method]["description"]
        assert len(given_examples(defs[method])) == len(EXAMPLES[method])


@pytest.mark.parametrize("options", VARIANTS)
def test_every_example_validates_against_every_variant(options: dict[str, Any]) -> None:
    defs = by_method(**options)
    for method in METHODS:
        given = given_examples(defs[method])
        assert given, method
        validator = jsonschema.Draft202012Validator(defs[method]["parameters"])
        for example in given:
            errors = [e.message for e in validator.iter_errors(example)]
            assert not errors, f"{method}: {errors[:3]}"
            assert fm.run(method, example).value is not None, method


def test_portable_examples_write_decimals_as_strings() -> None:
    defs = by_method(portable=True)
    for method in ("tvm.npv", "tvm.irr"):
        assert any(isinstance(v, int) for v in EXAMPLES[method]["minimal"]["input"]["cash_flows"])
        flows = given_examples(defs[method])[0]["cash_flows"]
        assert flows and all(isinstance(v, str) for v in flows), method
    # Integer fields (counts) stay integers.
    instalment = given_examples(defs["loans.instalment"])[0]
    assert isinstance(instalment["number_of_instalments"], int)


def test_examples_of_an_unknown_method_are_refused() -> None:
    with pytest.raises(fm.FairmarkError) as err:
        tools.examples("dcf.nope")
    assert err.value.code == "FM-E007"
    assert tools.examples("tvm.npv")


# 3. Compact profile -----------------------------------------------------------------------


DECIMAL = re.compile(r"^-?\d+(\.\d+)?([eE]\d+)?$")
PAIRS = [
    pytest.param({}, {"compact": True}, id="default-compact"),
    pytest.param({"portable": True}, {"portable": True, "compact": True}, id="portable"),
]


def decimal_pointers(value: Any, path: tuple[Any, ...] = ()) -> list[tuple[Any, ...]]:
    """Paths of the leaves that are decimals written as strings."""
    if isinstance(value, dict):
        return [p for k, v in value.items() for p in decimal_pointers(v, (*path, k))]
    if isinstance(value, list):
        return [p for i, v in enumerate(value) for p in decimal_pointers(v, (*path, i))]
    return [path] if isinstance(value, str) and DECIMAL.match(value) else []


def replaced(value: Any, path: tuple[Any, ...], new: Any) -> Any:
    copy = json.loads(json.dumps(value))
    node = copy
    for key in path[:-1]:
        node = node[key]
    node[path[-1]] = new
    return copy


def invalid_inputs(example: Any, portable: bool) -> list[Any]:
    """Inputs the core refuses: thousands separators, percent signs, a float in place of a
    decimal string (portable schemas require strings)."""
    out = []
    for path in decimal_pointers(example):
        if portable:
            out.append(replaced(example, path, 1200.5))
        else:
            out += [replaced(example, path, "1,200"), replaced(example, path, "5%")]
    return out


@pytest.mark.parametrize(("full_options", "compact_options"), PAIRS)
@pytest.mark.parametrize("method", METHODS)
def test_compact_and_full_schemas_accept_the_same_inputs(
    method: str, full_options: dict[str, Any], compact_options: dict[str, Any]
) -> None:
    full_def = by_method(**full_options)[method]
    full = jsonschema.Draft202012Validator(full_def["parameters"])
    compact = jsonschema.Draft202012Validator(by_method(**compact_options)[method]["parameters"])
    # Both directions, over every golden input, valid or not.
    for case_input in golden_inputs(method):
        value = plain(case_input)
        assert full.is_valid(value) == compact.is_valid(value), f"{method}: {value}"
    # Inputs the core refuses are refused by both, before any call.
    refused = 0
    for example in given_examples(full_def):
        for value in invalid_inputs(example, portable=bool(full_options)):
            assert full.is_valid(value) == compact.is_valid(value), f"{method}: {value}"
            refused += not full.is_valid(value)
    if decimal_pointers(given_examples(full_def)[0]):
        assert refused, f"{method}: no invalid input refused"


def test_thousands_separators_and_percent_signs_are_refused_up_front() -> None:
    for options in ({}, {"compact": True}):
        schema = by_method(**options)["tvm.npv"]["parameters"]
        for bad in ("1,200", "5%", "1.200,50", "€100"):
            value = {"rate": bad, "cash_flows": ["-100", "110"]}
            assert not jsonschema.Draft202012Validator(schema).is_valid(value), (options, bad)
            with pytest.raises(fm.FairmarkError):
                fm.run("tvm.npv", value)


def test_compact_is_smaller_and_drops_repetition() -> None:
    def size(node: Any) -> int:
        return len(json.dumps(node, ensure_ascii=False, separators=(",", ":")))

    full, compact = by_method(), by_method(compact=True)
    for method in METHODS:
        assert size(compact[method]) <= size(full[method]), method
    big = "statements.reorganise"
    assert size(compact[big]) < 0.9 * size(full[big])
    schema = json.dumps(compact[big]["parameters"])
    assert '"default": null' not in schema
    assert re.search(r'"format": "u?int\d*"', schema) is None
    assert '"pattern": "^-?' in schema
    assert "Decimals are" not in compact[big]["description"]
    portable = by_method(portable=True, compact=True)[big]
    assert "Decimals are" in portable["description"]


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


@pytest.mark.parametrize("empty", [[], "", set()])
def test_empty_area_selection_is_rejected(empty: Any) -> None:
    with pytest.raises(ValueError, match="no area"):
        tools.definitions(areas=empty)


# Transformations ----------------------------------------------------------------------------


def test_a_property_named_title_is_kept() -> None:
    schema = {
        "title": "Input",
        "type": "object",
        "properties": {"title": {"title": "Title", "type": "string"}, "x": {"type": "string"}},
    }
    out = tools._inline(schema)
    assert "title" not in out
    assert out["properties"]["title"] == {"type": "string"}
