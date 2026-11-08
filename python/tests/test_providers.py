"""Provider-agnostic adapters: every supported format is well formed and executable."""

import json
import logging
from pathlib import Path

import pytest
import yaml

import fairmark as fm
from fairmark._logging import JsonFormatter, log_event

ROOT = Path(__file__).resolve().parents[2]


def walk(node, visit):
    if isinstance(node, dict):
        visit(node)
        for v in node.values():
            walk(v, visit)
    elif isinstance(node, list):
        for v in node:
            walk(v, visit)


@pytest.mark.parametrize("provider", sorted(fm.tools.PROVIDERS))
def test_every_provider_has_one_tool_per_method(provider: str) -> None:
    defs = fm.tools.for_provider(provider)
    assert len(defs) == len(fm.methods())


def test_unknown_provider_is_rejected() -> None:
    with pytest.raises(ValueError, match="Known"):
        fm.tools.for_provider("nope")


def test_portable_schema_has_only_simple_types() -> None:
    def visit(node: dict) -> None:
        assert not isinstance(node.get("type"), list)
        assert "pattern" not in node
        assert "additionalProperties" not in node
        assert "$ref" not in node

    for tool in fm.tools.for_provider("ollama"):
        walk(tool["function"]["parameters"], visit)


def test_gemini_uses_json_schema_declarations() -> None:
    decl = fm.tools.gemini()[0]
    assert set(decl) == {"name", "description", "parametersJsonSchema"}
    assert decl["parametersJsonSchema"]["type"] == "object"


def test_openai_responses_tools_are_flat() -> None:
    tool = fm.tools.openai_responses()[0]
    assert tool["type"] == "function" and "parameters" in tool and "function" not in tool


def test_string_decimals_from_portable_models_are_exact() -> None:
    args = json.dumps({"cash_flows": ["-1000", "300", "400", "500"]})
    out = fm.tools.dispatch("fairmark_tvm_irr", args)
    assert out["value"]["irr"].startswith("0.08896339469")


def test_langchain_tools_execute() -> None:
    pytest.importorskip("langchain_core")
    tool = {t.name: t for t in fm.tools.langchain()}["fairmark_tvm_npv"]
    out = json.loads(tool.invoke({"rate": "0", "cash_flows": ["0.1", "0.2"]}))
    assert out["value"]["npv"] == "0.3"


def test_logging_refuses_financial_fields() -> None:
    with pytest.raises(ValueError, match="not allowed"):
        log_event(logging.getLogger("fairmark.test"), "tool_call", inputs={"rate": "0.1"})


def test_json_log_lines_are_parseable() -> None:
    record = logging.LogRecord("fairmark.mcp", logging.INFO, "", 0, "tool_call", None, None)
    record.fields = {"tool": "fairmark_tvm_npv", "outcome": "ok"}
    line = json.loads(JsonFormatter().format(record))
    assert line["event"] == "tool_call" and line["tool"] == "fairmark_tvm_npv"


def test_methods_report_their_status() -> None:
    """ADR 0012: the registry reports the status; `fm.methods()` passes it through."""
    status = {m["id"]: m["status"] for m in fm.methods()}
    assert set(status.values()) == {"available", "preview"}
    assert status["dcf.fcff"] == "preview"
    assert status["tvm.npv"] == "preview"
    assert status["tv.gordon"] == "available"


@pytest.mark.parametrize("provider", sorted(fm.tools.PROVIDERS))
def test_tool_descriptions_flag_preview_methods(provider: str) -> None:
    preview = {fm.tools.tool_name(m["id"]) for m in fm.methods() if m["status"] == "preview"}
    for tool in fm.tools.for_provider(provider):
        body = tool.get("function", tool)
        flagged = fm.tools.PREVIEW_NOTE in body["description"]
        assert flagged == (body["name"] in preview), body["name"]
        assert "status" not in body, "provider payloads carry no extra keys"


def test_mcp_tools_carry_the_status() -> None:
    pytest.importorskip("mcp")
    import anyio

    from fairmark import mcp_server

    assert "preview" in mcp_server.INSTRUCTIONS
    status = {fm.tools.tool_name(m["id"]): m["status"] for m in fm.methods()}
    tools = anyio.run(mcp_server.list_tools)
    assert {t.name for t in tools} == set(status)
    for t in tools:
        assert t.meta == {"fairmark/status": status[t.name]}
        assert (fm.tools.PREVIEW_NOTE in (t.description or "")) == (status[t.name] == "preview")


def test_available_descriptions_are_unchanged_and_preview_titles_are_marked() -> None:
    """Available tools keep the exact description they had before ADR 0012."""
    by_name = {fm.tools.tool_name(m["id"]): m for m in fm.methods()}
    for d in fm.tools.definitions():
        m = by_name[d["name"]]
        if m["status"] == "available":
            assert d["description"] == f"{m['title']}. {m['description']}{fm.tools._AGENT_RULE}"
        else:
            title = d["description"].split(". ", 1)[0]
            assert title == f"{m['title']} (preview)"
            assert d["description"].endswith(fm.tools.PREVIEW_NOTE + fm.tools._AGENT_RULE)


def test_unknown_status_is_treated_as_preview(monkeypatch: pytest.MonkeyPatch) -> None:
    real = fm.tools.methods()
    odd = [{**m, "status": "experimental"} if m["id"] == "tv.gordon" else m for m in real]
    monkeypatch.setattr(fm.tools, "methods", lambda: odd)
    tv = next(d for d in fm.tools.definitions() if d["name"] == "fairmark_tv_gordon")
    assert fm.tools.PREVIEW_NOTE in tv["description"]


def test_results_report_the_method_status() -> None:
    assert (
        fm.run("tv.gordon", {"rate": "0.1", "growth": "0.02", "next_flow": "50"}).meta[
            "method_status"
        ]
        == "available"
    )
    # The inner input of the golden case, dcf.fcff version 3.
    case = yaml.safe_load(
        (ROOT / "tests/golden/analysis.sensitivity/wacc-one-way.yaml").read_text()
    )
    out = fm.tools.dispatch("fairmark_analysis_sensitivity", case["input"])
    assert "error" not in out, out
    assert out["meta"]["method_status"] == "available"
    assert out["meta"]["evaluated_method_status"] == "preview"
    assert "Evaluated method dcf.fcff is preview" in " ".join(out["explain"]["conventions"])
    inner = fm.run("dcf.fcff", case["input"]["base_input"])
    assert inner.meta["method_status"] == "preview" and inner.meta["method_version"] == "3"
