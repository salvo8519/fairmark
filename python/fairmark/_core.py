"""Bridge to the Rust engine. Decimals cross the boundary as strings: exact end to end."""

from __future__ import annotations

import copy
import functools
import json
import re
from collections.abc import Mapping
from dataclasses import dataclass, field
from datetime import date, datetime
from decimal import Decimal
from typing import Any

from . import _engine


class FairmarkError(ValueError):
    """Raised when the engine rejects an input. `code` is stable, e.g. ``FM-E004``."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


def _encode(obj: Any) -> Any:
    if isinstance(obj, Decimal):
        return str(obj)
    if isinstance(obj, datetime):
        raise TypeError("pass a datetime.date (calendar date), not a datetime: dates carry no time")
    if isinstance(obj, date):
        return obj.isoformat()
    if isinstance(obj, float):
        # repr() is the shortest exact round-trip: 0.1 -> "0.1".
        return repr(obj)
    raise TypeError(f"Object of type {type(obj).__name__} is not JSON serializable")


# A decimal in a schema: a string with one of these patterns. The first is how the engine
# writes every decimal of a result (ADR 0014 G7: decimals are returned as strings); the second
# is the pattern of a decimal in an input schema, used for the input of another method held in
# a result (`analysis.goal_seek` `solved_input`). A test checks both against the engine.
_DECIMAL_PATTERNS = frozenset({r"^-?\d+(\.\d+)?$", r"^-?\d+(\.\d+)?([eE]\d+)?$"})

# Keywords naming, by a JSON pointer into the same value, the method whose input or output a
# field holds (`analysis.goal_seek`): the field is decoded with that method's schema.
_INPUT_OF = "x-fairmark-input-of"
_OUTPUT_OF = "x-fairmark-output-of"

_JSON_TYPES: dict[str, tuple[type, ...]] = {
    "null": (type(None),),
    "boolean": (bool,),
    "string": (str,),
    "integer": (int,),
    "number": (int, float, Decimal),
    "object": (dict,),
    "array": (list,),
}


@functools.cache
def _schema(kind: str, method: str) -> dict[str, Any]:
    """The engine's schema of a method, parsed once: `input`, `output` or `result`."""
    text = {
        "input": _engine.input_schema,
        "output": _engine.output_schema,
        "result": _engine.result_schema,
    }[kind]
    try:
        schema: dict[str, Any] = json.loads(text(method))
    except _engine.EngineError as err:
        _raise(err)
        raise  # unreachable
    return schema


def _of_type(node: Any, name: str) -> bool:
    if isinstance(node, bool):
        return name == "boolean"
    return isinstance(node, _JSON_TYPES.get(name, ()))


def _types(schema: dict[str, Any]) -> list[str]:
    t = schema.get("type")
    return [] if t is None else t if isinstance(t, list) else [t]


def _pointer(value: Any, pointer: str) -> Any:
    """The node of `value` at an RFC 6901 pointer, or None when it does not resolve."""
    for token in pointer.split("/")[1:]:
        token = token.replace("~1", "/").replace("~0", "~")
        if isinstance(value, dict):
            value = value.get(token)
        elif isinstance(value, list) and token.isdigit() and int(token) < len(value):
            value = value[int(token)]
        else:
            return None
    return value


class _Decoder:
    """Decodes a value by its JSON Schema: a decimal becomes `Decimal`, everything else keeps
    its JSON type. Nothing is guessed from the text of a value: a label, an id or a pointer
    that looks like a number stays text, because its schema says it is text.
    """

    def __init__(self, schema: dict[str, Any], root: Any) -> None:
        self.schema = schema
        self.root = root

    def resolve(self, schema: Any) -> Any:
        seen = 0
        while isinstance(schema, dict) and "$ref" in schema:
            ref = schema["$ref"]
            if not ref.startswith("#/") or seen > 64:
                raise ValueError(f"unsupported schema reference {ref!r}")
            target: Any = self.schema
            for token in ref[2:].split("/"):
                target = target[token.replace("~1", "/").replace("~0", "~")]
            schema, seen = target, seen + 1
        return schema

    def is_decimal(self, schema: dict[str, Any]) -> bool:
        return "string" in _types(schema) and schema.get("pattern") in _DECIMAL_PATTERNS

    def properties(
        self, schema: Any, node: dict[str, Any], inherited: frozenset[str]
    ) -> dict[str, Any] | None:
        """The schema of each key of the object `node` under `schema`, or None when the object
        does not fit it: a required key is missing, or a key is not one the schema can emit.
        Branches of `anyOf`/`oneOf` (an untagged or flattened enum) are tried in order.
        """
        schema = self.resolve(schema)
        if not isinstance(schema, dict):
            return {} if schema is not False else None
        if _types(schema) and "object" not in _types(schema):
            return None
        props: dict[str, Any] = dict(schema.get("properties", {}))
        if any(key not in node for key in schema.get("required", [])):
            return None
        for member in schema.get("allOf", []):
            sub = self.properties(member, node, inherited | props.keys())
            if sub is None:
                return None
            props.update(sub)
        for keyword in ("anyOf", "oneOf"):
            if keyword not in schema:
                continue
            chosen = self.branch(schema[keyword], node, inherited | props.keys())
            if chosen is None:
                return None
            sub = self.properties(chosen, node, inherited | props.keys())
            assert sub is not None
            props.update(sub)
        # An output schema lists every key the engine can write (serialisation contract), so
        # a key it does not list means the object belongs to another branch.
        if (
            "properties" in schema
            and "additionalProperties" not in schema
            and any(key not in props and key not in inherited for key in node)
        ):
            return None
        return props

    def branch(self, members: list[Any], node: Any, inherited: frozenset[str] = frozenset()) -> Any:
        """The first member of an `anyOf`/`oneOf` that `node` fits, or None."""
        for member in members:
            if isinstance(node, dict):
                if self.properties(member, node, inherited) is not None:
                    return member
            elif self.fits(member, node):
                return member
        return None

    def fits(self, schema: Any, node: Any) -> bool:
        schema = self.resolve(schema)
        if not isinstance(schema, dict):
            return schema is not False
        if isinstance(node, dict):
            return self.properties(schema, node, frozenset()) is not None
        if "const" in schema and node != schema["const"]:
            return False
        if "enum" in schema and node not in schema["enum"]:
            return False
        types = _types(schema)
        if types and not any(_of_type(node, t) for t in types):
            return False
        if isinstance(node, str) and "pattern" in schema and not re.search(schema["pattern"], node):
            return False
        for keyword in ("anyOf", "oneOf"):
            if keyword in schema and not any(self.fits(m, node) for m in schema[keyword]):
                return False
        return True

    def decode(self, schema: Any, node: Any) -> Any:
        if node is None:
            return None
        schema = self.resolve(schema)
        if not isinstance(schema, dict):
            return node
        for keyword, kind in ((_INPUT_OF, "input"), (_OUTPUT_OF, "output")):
            if keyword in schema:
                method = _pointer(self.root, schema[keyword])
                if not isinstance(method, str):
                    return node
                return _decode(kind, method, node)
        if isinstance(node, dict):
            return self.decode_object(schema, node)
        for keyword in ("anyOf", "oneOf"):
            if keyword in schema:
                chosen = self.branch(schema[keyword], node)
                return node if chosen is None else self.decode(chosen, node)
        if isinstance(node, list):
            prefix = schema.get("prefixItems", [])
            items = schema.get("items", True)
            return [
                self.decode(prefix[i] if i < len(prefix) else items, v) for i, v in enumerate(node)
            ]
        if isinstance(node, bool) or not self.is_decimal(schema):
            return node
        if isinstance(node, str) and re.search(schema["pattern"], node):
            return Decimal(node)
        if isinstance(node, int):
            return Decimal(node)
        return node

    def decode_object(self, schema: dict[str, Any], node: dict[str, Any]) -> dict[str, Any]:
        props = self.properties(schema, node, frozenset())
        if props is None:
            return node
        extra = schema.get("additionalProperties")
        out = {}
        for key, value in node.items():
            if key in props:
                out[key] = self.decode(props[key], value)
            elif isinstance(extra, dict):
                out[key] = self.decode(extra, value)
            else:
                out[key] = value
        return out


def _decode(kind: str, method: str, node: Any) -> Any:
    """`node` decoded by the `input` or `output` schema of `method`."""
    schema = _schema(kind, method)
    return _Decoder(schema, node).decode(schema, node)


@dataclass(frozen=True)
class Step:
    label: str
    formula: str
    value: Decimal


@dataclass(frozen=True)
class Result:
    """A valuation result: exact values, the audit trail and reproducibility metadata.

    Value fields are reachable as attributes: ``result.enterprise_value``.
    """

    method: str
    value: dict[str, Any]
    steps: list[Step]
    formula: str
    conventions: list[str]
    meta: dict[str, Any]
    raw: dict[str, Any] = field(repr=False)

    @property
    def method_version(self) -> str:
        """Version of the method that produced the result, for example ``"3"``. It changes on
        any numerical change and on any breaking change of the method's input or output
        (ADR 0014 G8); the schemas of each version are kept in ``schemas/<method>/``."""
        return str(self.meta["method_version"])

    def __getattr__(self, name: str) -> Any:
        value = object.__getattribute__(self, "value")
        if name in value:
            return value[name]
        raise AttributeError(name)

    def to_json(self) -> str:
        """The exact JSON envelope returned by the engine (decimals as strings)."""
        return json.dumps(self.raw, ensure_ascii=False)

    def explain_markdown(self) -> str:
        """A methodology note ready to paste into a report."""
        lines = [f"### {self.method}", "", f"Formula: `{self.formula}`", ""]
        if self.conventions:
            lines += ["Conventions:", *[f"- {c}" for c in self.conventions], ""]
        lines += ["| Step | Calculation | Value |", "| --- | --- | ---: |"]
        lines += [f"| {s.label} | `{s.formula}` | {s.value} |" for s in self.steps]
        lines += [
            "",
            f"Engine {self.meta['engine_version']}, method v{self.meta['method_version']}, "
            f"input hash `{self.meta['input_hash'][:16]}…`",
        ]
        return "\n".join(lines)


def _raise(err: Exception) -> None:
    try:
        report = json.loads(str(err))
        code, message = str(report["code"]), str(report["message"])
    except (ValueError, KeyError, TypeError):
        code, message = "FM-E001", str(err)
    raise FairmarkError(code, message) from None


def _run_text(method: str, inputs: Mapping[str, Any] | str) -> str:
    payload = inputs if isinstance(inputs, str) else json.dumps(dict(inputs), default=_encode)
    try:
        return _engine.run(method, payload)
    except _engine.EngineError as err:
        _raise(err)
        raise  # unreachable


def run_json(method: str, inputs: Mapping[str, Any] | str) -> dict[str, Any]:
    """Run a method and return the raw JSON envelope as a dict (decimals as strings)."""
    out: dict[str, Any] = json.loads(_run_text(method, inputs))
    return out


def run(method: str, inputs: Mapping[str, Any] | str) -> Result:
    """Run any engine method by id, e.g. ``run("dcf.fcff", {...})``.

    ``value`` is decoded by the method's output schema (:func:`output_schema`): every decimal
    is a ``Decimal``, every text (labels, ids, pointers, conventions) stays ``str``, integers
    stay ``int``.
    """
    text = _run_text(method, inputs)
    raw = json.loads(text)
    # A JSON number inside a value (an input echoed in a result) is read exactly, never as a
    # binary float.
    exact = json.loads(text, parse_float=Decimal)
    explain = raw["explain"]
    return Result(
        method=raw["method"],
        value=_decode("output", raw["method"], exact["value"]),
        steps=[Step(s["label"], s["formula"], Decimal(s["value"])) for s in explain["steps"]],
        formula=explain["formula"],
        conventions=list(explain["conventions"]),
        meta=dict(raw["meta"]),
        raw=raw,
    )


def methods() -> list[dict[str, str]]:
    """Catalogue of every method: id, title, description, version and status.

    ``status`` is ``"available"`` or ``"preview"`` (ADR 0012; definitions and promotion rule
    in the project's release policy). Tell users when a result comes from a preview method.
    """
    result: list[dict[str, str]] = json.loads(_engine.methods())
    return result


def input_schema(method: str) -> dict[str, Any]:
    """JSON Schema (2020-12) of a method's input."""
    return copy.deepcopy(_schema("input", method))


def output_schema(method: str) -> dict[str, Any]:
    """JSON Schema (2020-12) of a method's output: the ``value`` of its result.

    Decimals are strings matching ``^-?\\d+(\\.\\d+)?$``; a field that may be absent is not
    ``required``; a field that may be ``null`` says so. Snapshotted for each method version as
    ``schemas/<method>/v<N>.output.json``. :func:`run` decodes results with it.
    """
    return copy.deepcopy(_schema("output", method))


def result_schema(method: str) -> dict[str, Any]:
    """JSON Schema (2020-12) of a method's whole result envelope (method, value, explain,
    meta), with ``value`` described by :func:`output_schema`."""
    return copy.deepcopy(_schema("result", method))
