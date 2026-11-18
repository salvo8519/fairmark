"""scripts/mcp_protocol_check.py (issue #331): the checks fail when the protocol misbehaves.

The script itself runs the MCP Inspector (Node.js) in its own CI job. These tests replace the
Inspector with a stand-in that answers like the server would, then break one answer at a time.
"""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

import anyio
import pytest

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "scripts" / "mcp_protocol_check.py"
_spec = importlib.util.spec_from_file_location("mcp_protocol_check", SCRIPT)
check = importlib.util.module_from_spec(_spec)  # type: ignore[arg-type]
sys.modules["mcp_protocol_check"] = check
_spec.loader.exec_module(check)  # type: ignore[union-attr]

pytest.importorskip("mcp")
from fairmark import mcp_server  # noqa: E402


def in_process(name: str, arguments: dict[str, Any]) -> dict[str, Any]:
    """What the server answers, as the Inspector prints it."""
    result = mcp_server.call_tool(name, arguments)
    structured = json.loads(json.dumps(result.structured_content))
    return {
        "content": [{"type": "text", "text": json.dumps(structured)}],
        "structuredContent": structured,
        "isError": result.is_error,
    }


class StandIn:
    """Answers tools/call from the in-process server; `tamper` changes the answer."""

    def __init__(self, tamper: Any = None) -> None:
        self.tamper = tamper

    def run(self, *args: str, server_args: tuple[str, ...] = ()) -> tuple[int, str, str]:
        name = args[args.index("--tool-name") + 1]
        arguments = json.loads(args[args.index("--tool-args-json") + 1])
        answer = in_process(name, arguments)
        if self.tamper:
            answer = self.tamper(answer)
        code = 5 if answer.get("isError") else 0
        return code, json.dumps(answer, indent=2) + '\n{"error": {"code": "x"}}\n', ""


def test_calls_pass_when_the_protocol_returns_the_in_process_result() -> None:
    report: list[str] = []
    check.check_calls(StandIn(), report)
    assert len(report) == len(check.CASE_METHODS)


def test_calls_fail_when_the_protocol_result_differs() -> None:
    def changed(answer: dict[str, Any]) -> dict[str, Any]:
        answer["structuredContent"]["value"]["extra"] = "1"
        return answer

    with pytest.raises(check.Failure, match="differs from the in-process call"):
        check.check_calls(StandIn(changed), [])


def test_calls_fail_when_text_and_structured_content_disagree() -> None:
    def text_changed(answer: dict[str, Any]) -> dict[str, Any]:
        answer["content"][0]["text"] = "{}"
        return answer

    with pytest.raises(check.Failure, match="text content differs"):
        check.check_calls(StandIn(text_changed), [])


def test_error_check_passes_on_the_structured_error_report() -> None:
    report: list[str] = []
    check.check_error(StandIn(), report)
    assert report == ["invalid input: FM-E001 with pointers ['/cash_flows/0', '/rate']"]


def test_error_check_fails_on_a_protocol_error() -> None:
    class ProtocolError:
        def run(self, *args: str, server_args: tuple[str, ...] = ()) -> tuple[int, str, str]:
            return 1, '{"error": {"code": "connection_failed", "message": "x"}}', "boom"

    with pytest.raises(check.Failure, match="protocol error instead of a tool result"):
        check.check_error(ProtocolError(), [])


def test_unmet_expectations_use_the_golden_tolerance() -> None:
    case = {"expect": {"npv": "10.00"}, "tolerance": "0.01"}
    assert check.unmet_expectations(case, {"value": {"npv": "10.005"}}) == []
    assert check.unmet_expectations(case, {"value": {"npv": "10.02"}}) == [
        "npv: 10.02 != 10.00 (tolerance 0.01)"
    ]


def test_unmet_expectations_compare_names_and_null_exactly() -> None:
    case = {"expect": {"/a/0": "fcff", "b": "null"}, "tolerance": "0"}
    assert check.unmet_expectations(case, {"value": {"a": ["fcff"], "b": None}}) == []
    assert len(check.unmet_expectations(case, {"value": {"a": ["fcfe"], "b": 1}})) == 2


def test_first_json_ignores_what_follows() -> None:
    assert check.first_json('note\n{"a": [1]}\n{"error": 1}') == {"a": [1]}
    with pytest.raises(check.Failure, match="no JSON"):
        check.first_json("nothing here")


class Listing:
    """Answers tools/list like the Inspector with --strict, with a given stderr summary."""

    def __init__(self, stderr: str, code: int = 0) -> None:
        self.stderr, self.code = stderr, code

    def run(self, *args: str, server_args: tuple[str, ...] = ()) -> tuple[int, str, str]:
        assert "--strict" in args and "tools/list" in args, args
        listed = [
            {"name": t.name, "inputSchema": t.input_schema}
            for t in anyio.run(mcp_server.list_tools)
        ]
        return self.code, json.dumps({"tools": listed}), self.stderr


def test_list_passes_without_portability_problems() -> None:
    report: list[str] = []
    check.check_list(Listing(""), report)
    assert report[0].endswith("schema portability: no problem found")


def test_list_fails_on_a_single_portability_warning() -> None:
    with pytest.raises(check.Failure, match="0 errors, 1 warning"):
        check.check_list(Listing("Warning: ...\n0 errors, 1 warning across 1 tool."), [])


def test_list_fails_on_a_warning_line_without_a_summary() -> None:
    # A later Inspector version may word its summary differently: the warning still counts.
    with pytest.raises(check.Failure, match="1 problems"):
        check.check_list(Listing('Warning: tool "x"\n  Path: inputSchema\n'), [])


def test_list_fails_on_portability_errors() -> None:
    with pytest.raises(check.Failure, match="error-severity"):
        check.check_list(Listing("1 error, 0 warnings", code=6), [])


def test_unmet_expectations_read_defaults_by_pointer_and_null_as_absent() -> None:
    # SPEC-0013: defaults are named by their input pointer; `null` passes on an absent key.
    case = {
        "expect": {"b": "null", "/c/d": "null"},
        "expect_defaults": {"/timing": {"/value": "end_of_period"}},
        "tolerance": "0",
    }
    envelope = {
        "value": {"c": {}},
        "meta": {"defaults_applied": [{"pointer": "/timing", "value": "end_of_period"}]},
    }
    assert check.unmet_expectations(case, envelope) == []
    envelope["meta"]["defaults_applied"][0]["value"] = "mid_period"
    assert len(check.unmet_expectations(case, envelope)) == 1
    envelope["meta"]["defaults_applied"] = []
    assert check.unmet_expectations(case, envelope) == ["expect_defaults /timing: 0 entries"]
    # An absent parent is not an output not computed: the key is wrong.
    assert check.unmet_expectations({"expect": {"/x/y": "null"}, "tolerance": "0"}, envelope) == [
        "/x/y: not in the result"
    ]


def test_unmet_expectations_report_a_key_through_a_text_value() -> None:
    # Code review of #394, N12: indexing into a text raises TypeError; it is a missing key.
    case = {"expect": {"/a/b": "1"}, "tolerance": "0"}
    assert check.unmet_expectations(case, {"value": {"a": "text"}}) == ["/a/b: not in the result"]
