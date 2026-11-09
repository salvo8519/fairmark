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

    tools.definitions(compact=True)      # smaller schemas for large methods
    tools.definitions(areas=["dcf", "tvm"])   # only the tools of some areas

Every definition keeps the meaning of each enum value in the description, and carries input
examples taken from golden cases (``tool_examples.json``).

The LLM extracts parameters; Fairmark computes; the LLM writes the report citing
``explain``. The model never does the arithmetic.

A refused call returns ``{"error": <report>}`` (see ``dispatch``): the model corrects the
argument named by ``pointer`` as ``fix`` says, and every other entry of ``errors``, without
parsing the message.
"""

from __future__ import annotations

import json
from collections.abc import Iterable, Mapping
from functools import cache
from importlib import resources
from typing import Any

from ._core import FairmarkError, input_schema, methods, run_json

PREFIX = "fairmark_"

_AGENT_RULE = (
    " Always use this tool instead of computing the numbers yourself. When reporting, cite"
    " the returned `explain.steps` and never alter the returned values."
)

PREVIEW_NOTE = (
    " Status: preview. This method has not yet completed the promotion review (independent"
    " numerical audit and code review); a later method version may change its inputs or"
    " results. Results from preview methods should be reported as preview."
)
"""Sentences added to the description of every tool whose method is not available (ADR 0012)."""


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


# Private key carrying the sentence that lists the documented values of an enum while the
# schema is transformed; `_finish` moves it into the description and removes it.
_VALUES = "_fairmark_values"


def _values_sentence(variants: list[dict[str, Any]]) -> str:
    """The sentence "Values: `a` (meaning); `b` (meaning)." from schemars `oneOf` of `const`."""
    parts = []
    for variant in variants:
        meaning = " ".join(str(variant.get("description", "")).split()).rstrip(".")
        parts.append(f"`{variant['const']}` ({meaning})" if meaning else f"`{variant['const']}`")
    return "Values: " + "; ".join(parts) + "."


def _inline(schema: dict[str, Any]) -> dict[str, Any]:
    """Inline $refs and turn const-oneOf into enum for broad provider compatibility.

    The description of each enum value is kept: it is listed after the description of the
    field (`_VALUES`, finished by `_finish`), so no meaning is lost when `oneOf` collapses.
    The `title` keyword is dropped; a property named `title` is kept.
    """
    defs = schema.get("$defs", {})

    def walk(node: Any) -> Any:
        if isinstance(node, dict):
            properties = node.get("properties")
            if "$ref" in node:
                target = walk(defs[node["$ref"].split("/")[-1]])
                merged = {**target, **{k: v for k, v in node.items() if k != "$ref"}}
                return walk(merged)
            variants = node.get("oneOf")
            if isinstance(variants, list) and variants and all("const" in o for o in variants):
                out = {k: v for k, v in node.items() if k != "oneOf"}
                out["type"] = "string"
                out["enum"] = [o["const"] for o in variants]
                if any(o.get("description") for o in variants):
                    out[_VALUES] = _values_sentence(variants)
                return out
            out = {k: walk(v) for k, v in node.items() if k not in ("$defs", "$schema", "title")}
            if isinstance(properties, dict):
                # Property names are data, not keywords: none is dropped.
                out["properties"] = {name: walk(sub) for name, sub in properties.items()}
            return out
        if isinstance(node, list):
            return [walk(v) for v in node]
        return node

    result: dict[str, Any] = walk(schema)
    return result


def _finish(node: Any) -> Any:
    """Append the list of enum values to the description and drop the private key."""
    if isinstance(node, list):
        return [_finish(v) for v in node]
    if not isinstance(node, dict):
        return node
    out = {k: _finish(v) for k, v in node.items() if k != _VALUES}
    if _VALUES in node:
        description = str(node.get("description", "")).rstrip()
        out["description"] = f"{description} {node[_VALUES]}".strip()
    return out


_DECIMAL_HINT = ' Decimal number written as a string, for example "0.095".'


def _portable(node: Any) -> Any:
    """Simplify a schema for providers and open-weight models with limited JSON Schema support.

    Decimal fields (number or string) become plain strings with an explicit hint: strings are
    also the exact representation, so nothing is lost. Optional values (`X or null`) become
    `X`. Validation keywords some providers reject (`pattern`, `additionalProperties`) are
    removed; the engine still validates.
    """
    if isinstance(node, list):
        return [_portable(v) for v in node]
    if not isinstance(node, dict):
        return node
    variants = node.get("anyOf") or node.get("oneOf")
    if isinstance(variants, list):
        # Optional values (`X or null`): keep X; optional fields are simply not required.
        concrete = [v for v in variants if not (isinstance(v, dict) and v.get("type") == "null")]
        if len(concrete) == 1 and isinstance(concrete[0], dict):
            merged = {k: v for k, v in node.items() if k not in ("anyOf", "oneOf")}
            return _portable({**concrete[0], **merged})
    out = {k: _portable(v) for k, v in node.items() if k not in ("pattern", "additionalProperties")}
    kind = node.get("type")
    if isinstance(kind, list):
        kinds = [k for k in kind if k != "null"]
        if "string" in kinds and "number" in kinds:
            out["type"] = "string"
            if "description" in out:
                out["description"] = str(out["description"]) + _DECIMAL_HINT
        elif len(kinds) == 1:
            out["type"] = kinds[0]
        kind = out["type"]
    items = node.get("items")
    if kind == "array" and isinstance(items, dict) and isinstance(items.get("type"), list):
        out["description"] = str(out.get("description", "")) + " Each item:" + _DECIMAL_HINT
    return out


# Formats defined by JSON Schema 2020-12; others (schemars' `uint32`) are dropped when compact.
_STANDARD_FORMATS = frozenset(
    {
        "date-time", "date", "time", "duration", "email", "idn-email", "hostname",
        "idn-hostname", "ipv4", "ipv6", "uri", "uri-reference", "iri", "iri-reference",
        "uuid", "uri-template", "json-pointer", "relative-json-pointer", "regex",
    }
)  # fmt: skip
_REPEATED_MIN = 40
"""Descriptions at least this long are given once per tool in the compact profile: shorter
ones cost about as much as a reference to them would, so they are kept."""

DECIMAL_RULE = (
    ' Decimals are strings such as "0.095" or "1250000": decimal point, optional exponent'
    ' ("1e6"), no thousands separators, percent signs or currency symbols.'
)
"""Sentence added once to a portable compact tool description in place of the decimal hint
repeated on every decimal field."""


def _compact(schema: dict[str, Any]) -> tuple[dict[str, Any], bool]:
    """Smaller schema that accepts exactly the same inputs, with the same meanings.

    Removes only annotations, never a validation keyword: `"default": null` (absent and null
    mean the same), non-standard `format` values (`uint32`, an annotation; `minimum` still
    applies), the portable decimal hint (stated once in the tool description instead, where
    that is shorter), and every long description, or list of enum values, already given
    earlier in the same tool: a later occurrence carries no description. The decimal
    `pattern` is kept, so thousands separators and percent signs are refused before a call
    (ADR 0014 G7). Returns the schema and whether the decimal hint was removed.
    """
    seen: set[str] = set()
    decimals = False
    hints = (" Each item:" + _DECIMAL_HINT, _DECIMAL_HINT)
    # The hint is replaced by one sentence only where that is shorter: the saving is the
    # length of the hint on every field that carries it.
    repeated = json.dumps(schema).count(json.dumps(_DECIMAL_HINT)[1:-1])
    strip_hints = repeated * len(_DECIMAL_HINT) > len(DECIMAL_RULE)

    def walk(node: Any) -> Any:
        nonlocal decimals
        if isinstance(node, list):
            return [walk(v) for v in node]
        if not isinstance(node, dict):
            return node
        out: dict[str, Any] = {}
        for key, value in node.items():
            if key == "default" and value is None:
                continue
            if key == "format" and isinstance(value, str) and value not in _STANDARD_FORMATS:
                continue
            if key in ("description", _VALUES) and isinstance(value, str):
                for hint in hints:
                    if strip_hints and value.endswith(hint):
                        value, decimals = value[: -len(hint)], True
                if len(value) >= _REPEATED_MIN or key == _VALUES:
                    if value in seen:
                        continue
                    seen.add(value)
                if not value:
                    continue
                out[key] = value
                continue
            out[key] = walk(value)
        return out

    result: dict[str, Any] = walk(schema)
    return result, decimals


@cache
def _examples() -> dict[str, Any]:
    """Input examples per method, copied from golden cases by scripts/tool_examples.py."""
    data = resources.files("fairmark").joinpath("tool_examples.json").read_text("utf-8")
    loaded: dict[str, Any] = json.loads(data)
    return loaded


def examples(method: str) -> list[dict[str, Any]]:
    """Input examples of a method: the minimal one first, then the maximal one if any.

    Each is the input of a golden case (its path is in ``tool_examples.json``), validated
    against the method's schema and run through the engine by the test suite.
    """
    if method not in {m["id"] for m in methods()}:
        raise FairmarkError("FM-E007", f"unknown method: {method}")
    entry = _examples().get(method, {})
    return [entry[k]["input"] for k in ("minimal", "maximal") if k in entry]


def _strings_for_decimals(value: Any, schema: Any) -> Any:
    """An example as the portable schema expects it: numbers become strings where the schema
    asks for a string (decimals), integers stay where it asks for an integer."""
    if not isinstance(schema, dict):
        return value
    variants = [v for v in schema.get("anyOf", []) + schema.get("oneOf", []) if isinstance(v, dict)]
    if isinstance(value, dict):
        properties: dict[str, Any] = {}
        for candidate in [*variants, schema]:
            for name, sub in candidate.get("properties", {}).items():
                properties.setdefault(name, sub)
        return {k: _strings_for_decimals(v, properties.get(k)) for k, v in value.items()}
    if isinstance(value, list):
        items = schema.get("items")
        if items is None:
            items = next((v["items"] for v in variants if "items" in v), None)
        return [_strings_for_decimals(v, items) for v in value]
    if isinstance(value, (int, float)) and not isinstance(value, bool):
        kinds = {schema.get("type")} | {v.get("type") for v in variants}
        if "string" in kinds and not kinds & {"number", "integer"}:
            return str(value)
    return value


def areas() -> list[str]:
    """Areas of the tools: the part of each method id before the dot (`dcf`, `tvm`, ...)."""
    return sorted({m["id"].split(".")[0] for m in methods()})


def _selected(selection: Iterable[str] | str | None) -> set[str] | None:
    if selection is None:
        return None
    wanted = {selection} if isinstance(selection, str) else set(selection)
    wanted.discard("")
    if not wanted:
        raise ValueError(f"no area selected. Known areas: {', '.join(areas())}")
    unknown = sorted(wanted - set(areas()))
    if unknown:
        known = ", ".join(areas())
        raise ValueError(f"unknown area {', '.join(unknown)}. Known areas: {known}")
    return wanted


def _compact_json(value: Any) -> str:
    return json.dumps(value, ensure_ascii=False, separators=(",", ":"))


def definitions(
    portable: bool = False,
    *,
    compact: bool = False,
    areas: Iterable[str] | str | None = None,
) -> list[dict[str, Any]]:
    """Provider-neutral tool definitions: name, description, JSON Schema parameters.

    Args:
        portable: simplify the schemas for providers or models with limited JSON Schema
            support (decimals as strings, no `pattern`/`additionalProperties`). The input
            examples then go in the description instead of the `examples` keyword.
        compact: smaller schemas for large methods. Long descriptions repeated in a tool are
            given once (later occurrences carry no description), `"default": null` and
            non-standard `format` values are dropped, the portable decimal hint is stated
            once in the description, and only the minimal example is kept. Only annotations
            are removed: exactly the same inputs are accepted, and every enum value keeps
            its meaning.
        areas: only the tools of these areas (``areas()``: `dcf`, `statements`, `tvm`, ...).
            Default: every tool. An empty selection or an unknown area raises ValueError.

    Preview methods are flagged in the description (title suffix and ``PREVIEW_NOTE``), so a
    model sees the status before it calls the tool; ``fm.methods()`` reports it as data.
    Every enum value is listed with its meaning in the description ("Values: ..."), and the
    schema carries input examples taken from golden cases (``examples(method)``).
    """
    wanted = _selected(areas)
    out = []
    for m in methods():
        if wanted is not None and m["id"].split(".")[0] not in wanted:
            continue
        schema = _inline(input_schema(m["id"]))
        if portable:
            schema = _portable(schema)
        decimals = False
        if compact:
            schema, decimals = _compact(schema)
        schema = _finish(schema)
        samples = examples(m["id"])[:1] if compact else examples(m["id"])
        sample_text = ""
        if portable:
            samples = [_strings_for_decimals(sample, schema) for sample in samples]
            labels = ("Minimal input example", "Complete input example")
            sample_text = "".join(
                f" {label}: {_compact_json(s)}." for label, s in zip(labels, samples, strict=False)
            )
        elif samples:
            schema["examples"] = samples
        # Anything other than "available" is treated as preview: fail towards the warning.
        preview = m.get("status") != "available"
        title = f"{m['title']} (preview)" if preview else m["title"]
        note = PREVIEW_NOTE if preview else ""
        rule = DECIMAL_RULE if decimals else ""
        out.append(
            {
                "name": tool_name(m["id"]),
                "description": f"{title}. {m['description']}{note}{_AGENT_RULE}{rule}{sample_text}",
                "parameters": schema,
            }
        )
    return out


def anthropic(
    *, compact: bool = False, areas: Iterable[str] | str | None = None
) -> list[dict[str, Any]]:
    """Tool definitions for the Anthropic Messages API."""
    return [
        {"name": d["name"], "description": d["description"], "input_schema": d["parameters"]}
        for d in definitions(compact=compact, areas=areas)
    ]


def openai(
    portable: bool = False, *, compact: bool = False, areas: Iterable[str] | str | None = None
) -> list[dict[str, Any]]:
    """Tool definitions for OpenAI Chat Completions and every OpenAI-compatible API.

    Works with OpenAI, Azure OpenAI, Ollama (`/v1`), OpenRouter, LiteLLM, vLLM, LM Studio,
    Mistral, Groq and others. Use ``portable=True`` with open-weight models.
    """
    return [
        {"type": "function", "function": d}
        for d in definitions(portable, compact=compact, areas=areas)
    ]


def openai_responses(
    portable: bool = False, *, compact: bool = False, areas: Iterable[str] | str | None = None
) -> list[dict[str, Any]]:
    """Tool definitions for the OpenAI Responses API (flat function tools)."""
    return [{"type": "function", **d} for d in definitions(portable, compact=compact, areas=areas)]


def gemini(
    *, compact: bool = False, areas: Iterable[str] | str | None = None
) -> list[dict[str, Any]]:
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
        for d in definitions(portable=True, compact=compact, areas=areas)
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


def for_provider(
    provider: str, *, compact: bool = False, areas: Iterable[str] | str | None = None
) -> list[dict[str, Any]]:
    """Tool definitions in the format a provider expects, by provider name.

    Known names: anthropic, openai, azure-openai, ollama, openrouter, litellm, vllm, lmstudio,
    mistral, groq, openai-responses, gemini. Local open-weight runtimes get the portable schema.
    ``compact`` and ``areas`` are passed to ``definitions``.
    """
    try:
        fmt = PROVIDERS[provider.lower()]
    except KeyError:
        known = ", ".join(sorted(PROVIDERS))
        raise ValueError(f"unknown provider '{provider}'. Known: {known}") from None
    if fmt == "anthropic":
        return anthropic(compact=compact, areas=areas)
    if fmt == "openai-responses":
        return openai_responses(compact=compact, areas=areas)
    if fmt == "gemini":
        return gemini(compact=compact, areas=areas)
    return openai(portable=fmt == "openai-portable", compact=compact, areas=areas)


def langchain(*, compact: bool = False, areas: Iterable[str] | str | None = None) -> list[Any]:
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
        for d in definitions(compact=compact, areas=areas)
    ]


def dispatch(name: str, arguments: Mapping[str, Any] | str) -> dict[str, Any]:
    """Execute a tool call coming from an LLM.

    Never raises on bad input: returns ``{"error": <report>}`` so the model can read the error
    and correct its parameters. The report (ADR 0014 G5) carries ``code`` (stable),
    ``message``, ``pointer`` (JSON pointer of the argument to correct, or null), ``pointers``
    (every argument of a relation), ``reason`` (the sub-kind, for example ``missing``,
    ``unknown_field``, ``enum``, ``sign``), ``expected`` (the accepted values), ``fix`` (one
    sentence), ``missing_any_of`` (``FM-E027`` only: alternative sets of arguments, any one of
    which is enough) and ``errors`` (every input error found, the first equal to the top level).
    Correct every entry of ``errors`` before calling again.
    """
    try:
        args = json.loads(arguments) if isinstance(arguments, str) else dict(arguments)
        return run_json(method_id(name), args)
    except FairmarkError as err:
        return {"error": err.report}
    except json.JSONDecodeError as err:
        return {
            "error": FairmarkError(
                "FM-E001",
                f"arguments are not valid JSON: {err}",
                {
                    "code": "FM-E001",
                    "message": f"arguments are not valid JSON: {err}",
                    "reason": "syntax",
                    "fix": "send the arguments as one well-formed JSON object",
                },
            ).report
        }
