"""Bridge to the Rust engine. Decimals cross the boundary as strings: exact end to end."""

from __future__ import annotations

import json
import re
from collections.abc import Mapping
from dataclasses import dataclass, field
from datetime import date, datetime
from decimal import Decimal
from typing import Any

from . import _engine

_NUMBER = re.compile(r"^-?\d+(\.\d+)?([eE][-+]?\d+)?$")


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


# Fields that name things rather than measure them: kept as text even when they look like
# numbers (a method version "2", a scenario named "2025", an error code). Text and lists under
# such a key stay as they are; only an object under it (the `output` of a `tv.compare` row,
# which holds a whole method output) is converted like any other value.
_TEXT_KEYS = frozenset(
    {
        "id",
        "version",
        "method",
        "method_version",
        "name",
        "output",
        "variable",
        "pointer",
        "code",
        "message",
    }
)


def _decimalize(node: Any) -> Any:
    if isinstance(node, str) and _NUMBER.match(node):
        return Decimal(node)
    if isinstance(node, list):
        return [_decimalize(v) for v in node]
    if isinstance(node, dict):
        return {
            k: v if k in _TEXT_KEYS and not isinstance(v, dict) else _decimalize(v)
            for k, v in node.items()
        }
    return node


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
    meta: dict[str, str]
    raw: dict[str, Any] = field(repr=False)

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


def run_json(method: str, inputs: Mapping[str, Any] | str) -> dict[str, Any]:
    """Run a method and return the raw JSON envelope as a dict (decimals as strings)."""
    payload = inputs if isinstance(inputs, str) else json.dumps(dict(inputs), default=_encode)
    try:
        out: dict[str, Any] = json.loads(_engine.run(method, payload))
    except _engine.EngineError as err:
        _raise(err)
        raise  # unreachable
    return out


def run(method: str, inputs: Mapping[str, Any] | str) -> Result:
    """Run any engine method by id, e.g. ``run("dcf.fcff", {...})``."""
    raw = run_json(method, inputs)
    explain = raw["explain"]
    return Result(
        method=raw["method"],
        value=_decimalize(raw["value"]),
        steps=[Step(s["label"], s["formula"], Decimal(s["value"])) for s in explain["steps"]],
        formula=explain["formula"],
        conventions=list(explain["conventions"]),
        meta=dict(raw["meta"]),
        raw=raw,
    )


def methods() -> list[dict[str, str]]:
    """Catalogue of available methods (id, title, description, version)."""
    result: list[dict[str, str]] = json.loads(_engine.methods())
    return result


def input_schema(method: str) -> dict[str, Any]:
    """JSON Schema (2020-12) of a method's input."""
    try:
        schema: dict[str, Any] = json.loads(_engine.input_schema(method))
    except _engine.EngineError as err:
        _raise(err)
        raise  # unreachable
    return schema
