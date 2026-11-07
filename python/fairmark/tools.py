"""Function-calling adapters: give any LLM deterministic valuation tools.

Tool definitions are generated from the engine's JSON Schemas, so they never
drift from the implementation.

    from fairmark import tools

    response = client.messages.create(..., tools=tools.anthropic())
    for block in response.content:
        if block.type == "tool_use":
            result = tools.dispatch(block.name, block.input)

The LLM extracts parameters; Fairmark computes; the LLM writes the report
citing ``explain``. The model never does the arithmetic.
"""

from __future__ import annotations

import json
from collections.abc import Mapping
from typing import Any

from ._core import FairmarkError, input_schema, methods, run_json

PREFIX = "fairmark_"

_AGENT_RULE = (
    " Always use this tool instead of computing the numbers yourself. When reporting, cite"
    " the returned `explain.steps` and never alter the returned values."
)


def tool_name(method_id: str) -> str:
    """`dcf.fcff` -> `fairmark_dcf_fcff` (valid for every major provider)."""
    return PREFIX + method_id.replace(".", "_")


def method_id(name: str) -> str:
    """`fairmark_dcf_fcff` -> `dcf.fcff`."""
    if not name.startswith(PREFIX):
        raise FairmarkError("FM-E007", f"not a fairmark tool: {name}")
    by_name = {tool_name(m["id"]): m["id"] for m in methods()}
    try:
        return by_name[name]
    except KeyError:
        raise FairmarkError("FM-E007", f"unknown tool: {name}") from None


def _inline(schema: dict[str, Any]) -> dict[str, Any]:
    """Inline $refs and turn const-oneOf into enum for broad provider compatibility."""
    defs = schema.get("$defs", {})

    def walk(node: Any) -> Any:
        if isinstance(node, dict):
            if "$ref" in node:
                target = walk(defs[node["$ref"].split("/")[-1]])
                merged = {**target, **{k: v for k, v in node.items() if k != "$ref"}}
                return walk(merged)
            if "oneOf" in node and all("const" in o for o in node["oneOf"]):
                out = {k: v for k, v in node.items() if k != "oneOf"}
                out["type"] = "string"
                out["enum"] = [o["const"] for o in node["oneOf"]]
                return out
            return {k: walk(v) for k, v in node.items() if k not in ("$defs", "$schema", "title")}
        if isinstance(node, list):
            return [walk(v) for v in node]
        return node

    result: dict[str, Any] = walk(schema)
    return result


def definitions() -> list[dict[str, Any]]:
    """Provider-neutral tool definitions: name, description, JSON Schema parameters."""
    return [
        {
            "name": tool_name(m["id"]),
            "description": f"{m['title']}. {m['description']}{_AGENT_RULE}",
            "parameters": _inline(input_schema(m["id"])),
        }
        for m in methods()
    ]


def anthropic() -> list[dict[str, Any]]:
    """Tool definitions for the Anthropic Messages API."""
    return [
        {"name": d["name"], "description": d["description"], "input_schema": d["parameters"]}
        for d in definitions()
    ]


def openai() -> list[dict[str, Any]]:
    """Tool definitions for OpenAI-compatible Chat Completions APIs."""
    return [{"type": "function", "function": d} for d in definitions()]


def dispatch(name: str, arguments: Mapping[str, Any] | str) -> dict[str, Any]:
    """Execute a tool call coming from an LLM.

    Never raises on bad input: returns ``{"error": {"code", "message"}}`` so the
    model can read the error and correct its parameters.
    """
    try:
        args = json.loads(arguments) if isinstance(arguments, str) else dict(arguments)
        return run_json(method_id(name), args)
    except FairmarkError as err:
        return {"error": {"code": err.code, "message": err.message}}
    except json.JSONDecodeError as err:
        return {"error": {"code": "FM-E001", "message": f"arguments are not valid JSON: {err}"}}
