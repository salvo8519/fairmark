"""The structured error report in every adapter (ADR 0014 G5, issue #258)."""

from __future__ import annotations

import json

import pytest

import fairmark as fm
from fairmark import mcp_server, tools

BAD_LOAN = {
    "principal": "0",
    "number_of_instalments": 0,
    "timing": "ordinary",
    "periodic_rate": "0.01",
}


def test_python_error_exposes_the_report() -> None:
    with pytest.raises(fm.FairmarkError) as caught:
        fm.run("loans.instalment", BAD_LOAN)
    err = caught.value
    assert err.code == "FM-E001"
    assert err.pointer == "/principal"
    assert err.reason == "range"
    assert err.expected == ["an amount greater than 0"]
    assert [e.pointer for e in err.errors] == ["/principal", "/number_of_instalments"]
    assert err.errors[0] is err
    assert err.errors[1].errors == [err.errors[1]]
    assert err.report["errors"][1]["reason"] == "range"


def test_missing_any_of_and_pointers() -> None:
    with pytest.raises(fm.FairmarkError) as caught:
        fm.run("ratios.liquidity", {"balances": "closing", "closing": {"current_liabilities": "0"}})
    assert caught.value.code == "FM-E027"
    assert caught.value.missing_any_of == [["/closing/cash"], ["/closing/current_assets"]]
    with pytest.raises(fm.FairmarkError) as caught:
        fm.run("tvm.npv", {"rate": "0.1", "period_rates": ["0.1"], "cash_flows": [1]})
    assert caught.value.code == "FM-E001"
    assert caught.value.reason == "unknown_field"
    assert caught.value.pointer == "/period_rates"
    assert caught.value.expected == ["rate", "cash_flows"]


def test_message_and_code_are_unchanged_for_old_clients() -> None:
    with pytest.raises(fm.FairmarkError) as caught:
        fm.run("tvm.npv", {"rate": "0.1"})
    assert str(caught.value) == (
        "FM-E001: invalid input at `/cash_flows`: missing required field `cash_flows`; add it"
    )
    assert caught.value.fix == "add the required field `cash_flows`"


def test_dispatch_returns_the_whole_report() -> None:
    out = tools.dispatch(tools.tool_name("loans.instalment"), BAD_LOAN)
    error = out["error"]
    for key in ("code", "message", "pointer", "pointers", "reason", "expected", "fix", "errors"):
        assert key in error, key
    assert len(error["errors"]) == 2


def test_dispatch_reports_python_side_errors_in_the_same_shape() -> None:
    out = tools.dispatch(tools.tool_name("tvm.npv"), "{not json")
    assert out["error"]["reason"] == "syntax"
    assert out["error"]["pointer"] is None
    assert out["error"]["errors"][0]["code"] == "FM-E001"
    unknown = tools.dispatch("other_tool", {})["error"]
    assert unknown["code"] == "FM-E007" and unknown["errors"][0]["code"] == "FM-E007"


def test_mcp_returns_the_report_as_structured_content() -> None:
    result = mcp_server.call_tool(tools.tool_name("loans.instalment"), BAD_LOAN)
    assert result.is_error
    error = result.structured_content["error"]
    assert error["pointer"] == "/principal"
    assert json.loads(result.content[0].text)["error"]["errors"][1]["pointer"] == (
        "/number_of_instalments"
    )


def test_result_schema_validates_the_error_document() -> None:
    jsonschema = pytest.importorskip("jsonschema")
    schema = fm.result_schema("loans.instalment")
    out = tools.dispatch(tools.tool_name("loans.instalment"), BAD_LOAN)
    validator = jsonschema.Draft202012Validator(
        {**schema, "$ref": schema["x-fairmark-error"]}
        | {"properties": {}, "required": [], "type": "object"}
    )
    errors = [e.message for e in validator.iter_errors(out)]
    assert not errors, errors
