"""A decimal rounded as it is read is reported alike by every adapter (ADR 0014 G7).

The rule is applied once, in the core: the Python SDK (string and ``Decimal``), function calling
and the MCP server return the same warning and the same value as the CLI and the core
(``crates/fairmark-cli/tests/decimal_precision.rs``, which pins the same strings).
"""

from __future__ import annotations

import json
from decimal import Decimal

import pytest

import fairmark as fm
from fairmark import mcp_server, tools

LONG = "0.009166666666666666666666666667"
WARNING = (
    "warning: `/rate` has more digits than a decimal holds: given "
    "0.009166666666666666666666666667, used 0.0091666666666666666666666667, rounded half away "
    "from zero at the first digit that does not fit (28 decimal places, or 28 to 29 significant "
    "digits), as the input is read"
)
FUTURE_VALUE = "82846.24637189952008238315640"


def _inputs(rate: object) -> dict[str, object]:
    return {"payment": "2000", "rate": rate, "periods": 35, "timing": "due"}


def _warnings(conventions: list[str]) -> list[str]:
    return [c for c in conventions if c.startswith("warning: ")]


@pytest.mark.parametrize("rate", [LONG, Decimal(LONG)])
def test_the_sdk_reports_the_rounding(rate: object) -> None:
    result = fm.run("tvm.future_value", _inputs(rate))
    assert _warnings(result.conventions) == [WARNING]
    assert result.value["future_value"] == Decimal(FUTURE_VALUE)


def test_function_calling_and_mcp_report_the_rounding() -> None:
    name = tools.tool_name("tvm.future_value")
    out = tools.dispatch(name, _inputs(LONG))
    assert _warnings(out["explain"]["conventions"]) == [WARNING]
    assert out["value"]["future_value"] == FUTURE_VALUE
    result = mcp_server.call_tool(name, _inputs(LONG))
    assert not result.is_error
    assert result.structured_content == out


def test_a_json_number_in_tool_arguments_is_read_exactly() -> None:
    # Arguments given as JSON text: the number reaches the engine as written, never as a float.
    name = tools.tool_name("tvm.future_value")
    text = '{"payment": "2000", "rate": ' + LONG + ', "periods": 35, "timing": "due"}'
    out = tools.dispatch(name, text)
    assert _warnings(out["explain"]["conventions"]) == [WARNING]
    assert out["value"]["future_value"] == FUTURE_VALUE


# Final review of #279, finding 6: tool schemas declare decimals as strings (ADR 0014 G7).
G7_PATTERN = r"^-?\d+(\.\d+)?([eE][+-]?\d+)?$"


def _decimal_nodes(node: object, out: list[dict[str, object]]) -> None:
    if isinstance(node, dict):
        if node.get("pattern") == G7_PATTERN or (
            isinstance(node.get("type"), list) and "number" in node["type"]
        ):
            out.append(node)
        for value in node.values():
            _decimal_nodes(value, out)
    elif isinstance(node, list):
        for value in node:
            _decimal_nodes(value, out)


@pytest.mark.parametrize("compact", [False, True])
def test_tool_schemas_declare_decimals_as_strings(compact: bool) -> None:
    for d in tools.definitions(compact=compact):
        nodes: list[dict[str, object]] = []
        _decimal_nodes(d["parameters"], nodes)
        for node in nodes:
            kinds = node["type"] if isinstance(node["type"], list) else [node["type"]]
            assert "number" not in kinds, (d["name"], node)
            assert "string" in kinds and node["pattern"] == G7_PATTERN, (d["name"], node)
    rate = next(d for d in tools.definitions(compact=compact) if d["name"] == "fairmark_tvm_npv")[
        "parameters"
    ]["properties"]["rate"]
    assert rate["type"] == "string" and rate["pattern"] == G7_PATTERN


def test_mcp_list_tools_declares_decimals_as_strings() -> None:
    import asyncio

    listed = asyncio.run(mcp_server.list_tools())
    npv = next(t for t in listed if t.name == "fairmark_tvm_npv")
    assert npv.input_schema["properties"]["rate"]["type"] == "string"
    assert npv.input_schema["properties"]["rate"]["pattern"] == G7_PATTERN


def test_tool_examples_follow_the_string_schema() -> None:
    for d in tools.definitions():
        for sample in d["parameters"].get("examples", []):
            assert '"rate": 0' not in json.dumps(sample), d["name"]


# Finding 9: a float argument reaching the MCP server is reported (finding 6 (c)).
def test_mcp_reports_a_float_argument() -> None:
    name = tools.tool_name("tvm.future_value")
    arguments = {"payment": "2000", "rate": float(LONG), "periods": 35, "timing": "due"}
    result = mcp_server.call_tool(name, arguments)
    assert not result.is_error
    warnings = _warnings(result.structured_content["explain"]["conventions"])
    assert warnings == [
        "warning: `/rate` arrived as a binary floating-point number (0.009166666666666667): a "
        "JSON number may have lost digits before Fairmark read it; send decimals as strings, "
        'for example "0.095", to keep every digit'
    ]
