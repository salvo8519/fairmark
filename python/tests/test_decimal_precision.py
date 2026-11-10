"""A decimal rounded as it is read is reported alike by every adapter (ADR 0014 G7).

The rule is applied once, in the core: the Python SDK (string and ``Decimal``), function calling
and the MCP server return the same warning and the same value as the CLI and the core
(``crates/fairmark-cli/tests/decimal_precision.rs``, which pins the same strings).
"""

from __future__ import annotations

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
