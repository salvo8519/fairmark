"""Function-calling adapters: give any LLM deterministic valuation tools.

Provider-agnostic by design. Tool definitions are generated from the engine's JSON Schemas,
so they never drift from the implementation; the same ``dispatch`` executes calls from any
provider.

    from fairmark import tools

    tools.for_provider("anthropic")      # Anthropic Messages API
    tools.for_provider("openai")         # OpenAI Chat Completions and every compatible API:
                                         # Ollama, OpenRouter, LiteLLM, vLLM, LM Studio, ...
    tools.for_provider("openai-responses")
    tools.for_provider("gemini")         # Google Gemini (function declarations)

    result = tools.dispatch(name, arguments)   # arguments: dict or JSON string

The LLM extracts parameters; Fairmark computes; the LLM writes the report citing
``explain``. The model never does the arithmetic.
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


_DECIMAL_HINT = ' Decimal number written as a string, for example "0.095".'


def _portable(node: Any) -> Any:
    """Simplify a schema for providers and open-weight models with limited JSON Schema support.

    Decimal fields (number or string) become plain strings with an explicit hint: strings are
    also the exact representation, so nothing is lost. Validation keywords some providers
    reject (`pattern`, `additionalProperties`) are removed; the engine still validates.
    """
    if isinstance(node, list):
        return [_portable(v) for v in node]
    if not isinstance(node, dict):
        return node
    out = {k: _portable(v) for k, v in node.items() if k not in ("pattern", "additionalProperties")}
    kind = node.get("type")
    if isinstance(kind, list) and "string" in kind and "number" in kind:
        out["type"] = "string"
        if "description" in out:
            out["description"] = str(out["description"]) + _DECIMAL_HINT
    items = node.get("items")
    if kind == "array" and isinstance(items, dict) and isinstance(items.get("type"), list):
        out["description"] = str(out.get("description", "")) + " Each item:" + _DECIMAL_HINT
    return out


def definitions(portable: bool = False) -> list[dict[str, Any]]:
    """Provider-neutral tool definitions: name, description, JSON Schema parameters.

    Args:
        portable: simplify the schemas for providers or models with limited JSON Schema
            support (decimals as strings, no `pattern`/`additionalProperties`).
    """
    out = []
    for m in methods():
        schema = _inline(input_schema(m["id"]))
        out.append(
            {
                "name": tool_name(m["id"]),
                "description": f"{m['title']}. {m['description']}{_AGENT_RULE}",
                "parameters": _portable(schema) if portable else schema,
            }
        )
    return out


def anthropic() -> list[dict[str, Any]]:
    """Tool definitions for the Anthropic Messages API."""
    return [
        {"name": d["name"], "description": d["description"], "input_schema": d["parameters"]}
        for d in definitions()
    ]


def openai(portable: bool = False) -> list[dict[str, Any]]:
    """Tool definitions for OpenAI Chat Completions and every OpenAI-compatible API.

    Works with OpenAI, Azure OpenAI, Ollama (`/v1`), OpenRouter, LiteLLM, vLLM, LM Studio,
    Mistral, Groq and others. Use ``portable=True`` with open-weight models.
    """
    return [{"type": "function", "function": d} for d in definitions(portable)]


def openai_responses(portable: bool = False) -> list[dict[str, Any]]:
    """Tool definitions for the OpenAI Responses API (flat function tools)."""
    return [{"type": "function", **d} for d in definitions(portable)]


def gemini() -> list[dict[str, Any]]:
    """Function declarations for Google Gemini (google-genai SDK or REST).

    Uses ``parametersJsonSchema`` with the portable schema. Wrap them as
    ``{"function_declarations": fm.tools.gemini()}`` in the request ``tools``.
    """
    return [
        {
            "name": d["name"],
            "description": d["description"],
            "parametersJsonSchema": d["parameters"],
        }
        for d in definitions(portable=True)
    ]


PROVIDERS: dict[str, str] = {
    "anthropic": "anthropic",
    "openai": "openai",
    "azure-openai": "openai",
    "ollama": "openai-portable",
    "openrouter": "openai",
    "litellm": "openai",
    "vllm": "openai-portable",
    "lmstudio": "openai-portable",
    "mistral": "openai",
    "groq": "openai",
    "openai-responses": "openai-responses",
    "gemini": "gemini",
}


def for_provider(provider: str) -> list[dict[str, Any]]:
    """Tool definitions in the format a provider expects, by provider name.

    Known names: anthropic, openai, azure-openai, ollama, openrouter, litellm, vllm, lmstudio,
    mistral, groq, openai-responses, gemini. Local open-weight runtimes get the portable schema.
    """
    try:
        fmt = PROVIDERS[provider.lower()]
    except KeyError:
        known = ", ".join(sorted(PROVIDERS))
        raise ValueError(f"unknown provider '{provider}'. Known: {known}") from None
    if fmt == "anthropic":
        return anthropic()
    if fmt == "openai-responses":
        return openai_responses()
    if fmt == "gemini":
        return gemini()
    return openai(portable=fmt == "openai-portable")


def langchain() -> list[Any]:
    """LangChain / LangGraph tools (``pip install "fairmark[langchain]"``)."""
    try:
        from langchain_core.tools import StructuredTool
    except ImportError as err:  # pragma: no cover
        raise ImportError(
            'LangChain tools need the extra: pip install "fairmark[langchain]"'
        ) from err

    def make(name: str) -> Any:
        def call(**kwargs: Any) -> str:
            return json.dumps(dispatch(name, kwargs), ensure_ascii=False)

        return call

    return [
        StructuredTool.from_function(
            func=make(d["name"]),
            name=d["name"],
            description=d["description"],
            args_schema=d["parameters"],
        )
        for d in definitions()
    ]


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
