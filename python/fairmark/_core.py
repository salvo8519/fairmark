"""Bridge to the Rust engine. Decimals cross the boundary as strings: exact end to end."""

from __future__ import annotations

import copy
import functools
import json
import re
from collections.abc import Mapping, Sequence
from dataclasses import dataclass, field
from datetime import date, datetime
from decimal import Decimal
from typing import Any

from . import _engine


class FairmarkError(ValueError):
    """Raised when the engine rejects an input. `code` is stable, e.g. ``FM-E004``.

    The error report of the engine (ADR 0014 G5) is available as attributes, so that a caller
    corrects the input without reading the message:

    - ``pointer``: JSON pointer of the field to correct, or ``None`` (malformed JSON);
    - ``pointers``: every field of a relation (both sides of an ``FM-E026``, both fields of a
      conflict), empty when the error concerns one field;
    - ``reason``: the sub-kind, for example ``missing``, ``unknown_field``, ``enum``, ``sign``
      or ``reconcile``, or ``None``;
    - ``expected``: the accepted values, or a short description of the accepted form;
    - ``fix``: one sentence that resolves the error, or ``None``;
    - ``missing_any_of``: for ``FM-E027``, the alternative sets of input pointers;
    - ``errors``: every error the validation found, the first equal to this one, each a
      ``FairmarkError``;
    - ``report``: the report as the engine returned it (a dict).
    """

    def __init__(self, code: str, message: str, report: Mapping[str, Any] | None = None) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message
        data: dict[str, Any] = (
            dict(report) if report is not None else {"code": code, "message": message}
        )
        for key, default in _REPORT_DEFAULTS.items():
            data.setdefault(key, copy.copy(default))
        if not data.get("errors"):
            data["errors"] = [{k: v for k, v in data.items() if k != "errors"}]
        self.report: dict[str, Any] = data
        self.pointer: str | None = data.get("pointer")
        self.pointers: list[str] = list(data.get("pointers") or [])
        self.reason: str | None = data.get("reason")
        self.expected: list[str] = list(data.get("expected") or [])
        self.fix: str | None = data.get("fix")
        self.missing_any_of: list[list[str]] = [list(s) for s in data.get("missing_any_of") or []]
        self.errors: list[FairmarkError] = [self]
        self.errors += [
            FairmarkError(str(e["code"]), str(e["message"]), e) for e in data["errors"][1:]
        ]


# The fields of an error report that the engine always writes (ADR 0014 G5), for an error
# raised by the Python layer itself (an unknown tool, a result that does not fit its schema).
_REPORT_DEFAULTS: dict[str, Any] = {
    "pointer": None,
    "pointers": [],
    "reason": None,
    "expected": [],
    "fix": None,
}


def _encode(obj: Any) -> Any:
    if isinstance(obj, Decimal):
        # Fixed-point notation, exact: Decimal("1E-7") -> "0.0000001". The engine also reads
        # an exponent with a sign (ADR 0014 G7), but plain digits are what an analyst reads.
        return format(obj, "f")
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


class _Shape:
    """What an object schema allows: the schema of each named key, the schema of any other key
    (`extra`, None when the object is closed) and whether other keys are allowed at all."""

    def __init__(self) -> None:
        self.props: dict[str, Any] = {}
        self.extra: Any = None


class _Mismatch(Exception):
    """A result that does not fit its own schema: a defect, reported as `FM-E012`."""


class _Decoder:
    """Decodes a value by its JSON Schema: a decimal becomes `Decimal`, everything else keeps
    its JSON type. Nothing is guessed from the text of a value: a label, an id or a pointer
    that looks like a number stays text, because its schema says it is text. A value that does
    not fit its schema is a defect of the engine, raised as `FM-E012`.
    """

    def __init__(self, schema: dict[str, Any]) -> None:
        self.schema = schema

    def resolve(self, schema: Any) -> Any:
        seen = 0
        while isinstance(schema, dict) and "$ref" in schema:
            ref = schema["$ref"]
            if not ref.startswith("#/") or seen > 64:
                raise _Mismatch(f"unsupported schema reference {ref!r}")
            target: Any = self.schema
            for token in ref[2:].split("/"):
                target = target[token.replace("~1", "/").replace("~0", "~")]
            schema, seen = target, seen + 1
        return schema

    def is_decimal(self, schema: dict[str, Any]) -> bool:
        return "string" in _types(schema) and schema.get("pattern") in _DECIMAL_PATTERNS

    def shape(self, schema: Any, node: dict[str, Any], inherited: frozenset[str]) -> _Shape | None:
        """The shape of the object `node` under `schema`, or None when it does not fit: a
        required key is missing, or a key is neither named by the schema nor allowed by its
        `additionalProperties`. Branches of `anyOf`/`oneOf` (an untagged or flattened enum)
        are tried in order; their named keys and their `additionalProperties` are merged.
        """
        schema = self.resolve(schema)
        shape = _Shape()
        if not isinstance(schema, dict):
            shape.extra = True
            return shape if schema is not False else None
        if _types(schema) and "object" not in _types(schema):
            return None
        shape.props.update(schema.get("properties", {}))
        if any(key not in node for key in schema.get("required", [])):
            return None
        extra = schema.get("additionalProperties")
        if isinstance(extra, dict) or extra is True:
            shape.extra = extra
        members = list(schema.get("allOf", []))
        for keyword in ("anyOf", "oneOf"):
            if keyword in schema:
                chosen = self.branch(schema[keyword], node, inherited | shape.props.keys())
                if chosen is None:
                    return None
                members.append(chosen)
        for member in members:
            sub = self.shape(member, node, inherited | shape.props.keys())
            if sub is None:
                return None
            shape.props.update(sub.props)
            if shape.extra is None:
                shape.extra = sub.extra
        # An output schema lists every key the engine can write (serialisation contract), and
        # an input schema refuses unknown keys (`additionalProperties: false`): a key neither
        # names means the object belongs to another branch. Only a schema or `true` in
        # `additionalProperties` keeps the object open.
        closed = extra is False or (extra is None and "properties" in schema)
        if (
            closed
            and shape.extra is None
            and any(key not in shape.props and key not in inherited for key in node)
        ):
            return None
        return shape

    def branch(self, members: list[Any], node: Any, inherited: frozenset[str] = frozenset()) -> Any:
        """The first member of an `anyOf`/`oneOf` that `node` fits, or None."""
        for member in members:
            if isinstance(node, dict):
                if self.shape(member, node, inherited) is not None:
                    return member
            elif self.fits(member, node):
                return member
        return None

    def fits(self, schema: Any, node: Any) -> bool:
        schema = self.resolve(schema)
        if not isinstance(schema, dict):
            return schema is not False
        if isinstance(node, dict):
            return self.shape(schema, node, frozenset()) is not None
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

    def decode(self, schema: Any, node: Any, holder: Any = None, at: str = "") -> Any:
        """`node` decoded by `schema`; `holder` is the object that holds it (for the
        `x-fairmark-*-of` pointers), `at` its pointer within the decoded value."""
        if node is None:
            return None
        schema = self.resolve(schema)
        if not isinstance(schema, dict):
            return node
        for keyword, kind in ((_INPUT_OF, "input"), (_OUTPUT_OF, "output")):
            if keyword in schema:
                # The pointer is relative to the object that holds the field.
                method = _pointer(holder, schema[keyword])
                if not isinstance(method, str):
                    raise _Mismatch(f"{at}: {keyword} {schema[keyword]!r} names no method id")
                return _decode(kind, method, node)
        if isinstance(node, dict):
            return self.decode_object(schema, node, at)
        for keyword in ("anyOf", "oneOf"):
            if keyword in schema:
                chosen = self.branch(schema[keyword], node)
                if chosen is None:
                    raise _Mismatch(f"{at or '/'}: the value fits no alternative of its schema")
                return self.decode(chosen, node, holder, at)
        if isinstance(node, list):
            prefix = schema.get("prefixItems", [])
            items = schema.get("items", True)
            return [
                self.decode(prefix[i] if i < len(prefix) else items, v, node, f"{at}/{i}")
                for i, v in enumerate(node)
            ]
        if isinstance(node, bool) or not self.is_decimal(schema):
            return node
        if isinstance(node, Decimal):  # a JSON number read exactly (`parse_float=Decimal`)
            return node
        if isinstance(node, str) and re.search(schema["pattern"], node):
            return Decimal(node)
        if isinstance(node, int):
            return Decimal(node)
        raise _Mismatch(f"{at or '/'}: {node!r} is not a decimal")

    def decode_object(self, schema: dict[str, Any], node: dict[str, Any], at: str) -> Any:
        shape = self.shape(schema, node, frozenset())
        if shape is None:
            raise _Mismatch(f"{at or '/'}: the object does not fit its schema")
        out = {}
        for key, value in node.items():
            where = f"{at}/{key.replace('~', '~0').replace('/', '~1')}"
            sub = shape.props.get(key, shape.extra)
            out[key] = value if sub is None else self.decode(sub, value, node, where)
        return out


def _decode(kind: str, method: str, node: Any) -> Any:
    """`node` decoded by the `input` or `output` schema of `method`."""
    schema = _schema(kind, method)
    return _Decoder(schema).decode(schema, node)


def _decode_by(schema: dict[str, Any], node: Any, what: str) -> Any:
    """`node` decoded by `schema`. A node that does not fit the schema is a defect of the engine,
    reported as `FM-E012` like a panic in the engine, with the pointer of the node."""
    try:
        return _Decoder(schema).decode(schema, node)
    except _Mismatch as err:
        raise FairmarkError(
            "FM-E012",
            f"internal error: {what} does not match its schema at {err}; please report it",
        ) from None


def _decode_value(method: str, node: Any) -> Any:
    """The `value` of a result of `method`, decoded by its output schema."""
    return _decode_by(_schema("output", method), node, f"the result of {method}")


@dataclass(frozen=True)
class Step:
    label: str
    formula: str
    value: Decimal


@dataclass(frozen=True)
class Result:
    """A valuation result: exact values, the audit trail, the diagnostics and reproducibility
    metadata (the envelope of ADR 0014 G4).

    Value fields are reachable as attributes: ``result.enterprise_value``. The diagnostics are
    ``result.diagnostics`` and the lists ``result.not_computed``, ``result.warnings``,
    ``result.checks`` and ``result.defaults_applied``, which take precedence over a value field
    with the same name.
    """

    method: str
    value: dict[str, Any]
    steps: list[Step]
    formula: str
    conventions: list[str]
    meta: dict[str, Any]
    raw: dict[str, Any] = field(repr=False)
    diagnostics: dict[str, Any] = field(default_factory=dict)
    """What the result does not contain (``not_computed``), what the reader could miss
    (``warnings``, each with a stable code ``FM-Wnnn``) and what was verified (``checks``), as
    the engine writes them: pointers and reasons are ``str``."""

    @property
    def not_computed(self) -> list[dict[str, Any]]:
        """The outputs not computed: ``pointer`` (from the envelope root, ``/value/...``),
        ``reason_code`` (``missing_input``, ``choice_required``, ``not_defined``,
        ``not_applicable``), ``reason``, ``missing_any_of`` and ``permitted``."""
        return list(self.diagnostics.get("not_computed", []))

    @property
    def warnings(self) -> list[dict[str, Any]]:
        """The warnings: ``code`` (``FM-Wnnn``, see the warning catalogue), ``pointer`` and
        ``message``; each is echoed in ``conventions``."""
        return list(self.diagnostics.get("warnings", []))

    @property
    def checks(self) -> list[dict[str, Any]]:
        """The relations verified: ``id``, ``status`` (``reconciled`` or ``not_checked``),
        ``pointer`` (the result block) and ``pointers`` (the inputs read)."""
        return list(self.diagnostics.get("checks", []))

    @property
    def defaults_applied(self) -> list[dict[str, Any]]:
        """The documented defaults filled in: ``pointer``, ``value`` and ``reason``."""
        return list(self.meta.get("defaults_applied", []))

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


def _raise(err: Exception) -> None:
    try:
        report = json.loads(str(err))
        code, message = str(report["code"]), str(report["message"])
    except (ValueError, KeyError, TypeError):
        raise FairmarkError("FM-E001", str(err)) from None
    raise FairmarkError(code, message, report) from None


def _run_text(
    method: str, inputs: Mapping[str, Any] | str, binary_floats: Sequence[str] = ()
) -> str:
    payload = inputs if isinstance(inputs, str) else json.dumps(dict(inputs), default=_encode)
    try:
        return _engine.run(method, payload, list(binary_floats))
    except _engine.EngineError as err:
        _raise(err)
        raise  # unreachable


def run_json(
    method: str, inputs: Mapping[str, Any] | str, *, binary_floats: Sequence[str] = ()
) -> dict[str, Any]:
    """Run a method and return the raw JSON envelope as a dict (decimals as strings).

    ``binary_floats``: JSON pointers of the input values that reached the caller as binary
    floating-point numbers (a client library read a JSON number as a float). The engine
    reports each as warning ``FM-W017``; the envelope is the engine's, unchanged. Function
    calling and the MCP server pass them (:func:`fairmark.tools.dispatch`).

    A Python ``float`` in ``inputs`` is sent as its shortest decimal representation, the
    literal the analyst typed (``0.095`` stays ``0.095``). A float that is the result of a
    Python computation (for example ``0.1 + 0.2``, ``0.30000000000000004``) carries binary
    artifacts that Fairmark cannot detect: pass ``Decimal`` or strings for computed values.
    Warning ``FM-W017`` is emitted only where an outside library converts JSON numbers to
    floats (function calling with a dict, MCP, LangChain), never for the floats given here.
    """
    out: dict[str, Any] = json.loads(_run_text(method, inputs, binary_floats))
    return out


def run(method: str, inputs: Mapping[str, Any] | str) -> Result:
    """Run any engine method by id, e.g. ``run("dcf.fcff", {...})``.

    ``value`` is decoded by the method's output schema (:func:`output_schema`): every decimal
    is a ``Decimal``, every text (labels, ids, pointers, conventions) stays ``str``, integers
    stay ``int``.

    A Python ``float`` in ``inputs`` is sent as its shortest decimal representation, the
    literal the analyst typed (``0.095`` stays ``0.095``). A float that is the result of a
    Python computation (for example ``0.1 + 0.2``, ``0.30000000000000004``) carries binary
    artifacts that Fairmark cannot detect: pass ``Decimal`` or strings for computed values.
    Warning ``FM-W017`` is emitted only where an outside library converts JSON numbers to
    floats (function calling with a dict, MCP, LangChain), never for the floats given here.
    """
    text = _run_text(method, inputs)
    raw = json.loads(text)
    # A JSON number inside a value (an input echoed in a result) is read exactly, never as a
    # binary float.
    exact = json.loads(text, parse_float=Decimal)
    explain = raw["explain"]
    return Result(
        method=raw["method"],
        value=_decode_value(raw["method"], exact["value"]),
        steps=[Step(s["label"], s["formula"], Decimal(s["value"])) for s in explain["steps"]],
        formula=explain["formula"],
        conventions=list(explain["conventions"]),
        meta=dict(raw["meta"]),
        raw=raw,
        diagnostics=dict(raw["diagnostics"]),
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
