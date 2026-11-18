"""The schema walk shared by the golden tooling and the golden tests (SPEC-0013).

RFC 6901 tokens and lookup, `$ref` and `anyOf`/`oneOf`/`allOf` branches, the schema nodes at a
path, and the schema history read the way ADR 0017 requires. Imported by
`scripts/golden_values_migration.py` and `python/tests/test_golden_conformance.py`, so both read
schemas one way.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path
from typing import Any


def tokens(pointer: str) -> list[str]:
    """RFC 6901 tokens of `pointer`; a key without a leading '/' is one top-level field."""
    if not pointer.startswith("/"):
        return [pointer]
    return [t.replace("~1", "/").replace("~0", "~") for t in pointer[1:].split("/")]


def pointer_of(parts: list[str | int]) -> str:
    return "".join("/" + str(p).replace("~", "~0").replace("/", "~1") for p in parts)


def resolve(data: Any, parts: list[str]) -> Any:
    """The node of `data` at `parts`; KeyError when it does not exist."""
    for t in parts:
        if isinstance(data, list):
            if not re.fullmatch(r"0|[1-9][0-9]*", t) or int(t) >= len(data):
                raise KeyError(t)
            data = data[int(t)]
        elif isinstance(data, dict):
            if t not in data:
                raise KeyError(t)
            data = data[t]
        else:
            raise KeyError(t)
    return data


def deref(root: dict[str, Any], node: Any) -> Any:
    seen = 0
    while isinstance(node, dict) and isinstance(node.get("$ref"), str) and seen < 20:
        name = node["$ref"].removeprefix("#/$defs/")
        node = root.get("$defs", {}).get(name, {})
        seen += 1
    return node


def branches(root: dict[str, Any], node: Any) -> list[Any]:
    node = deref(root, node)
    if not isinstance(node, dict):
        return []
    out = [node]
    for k in ("anyOf", "oneOf", "allOf"):
        for b in node.get(k, []):
            out.extend(branches(root, b))
    return out


def schema_at(root: dict[str, Any], parts: list[str]) -> list[dict[str, Any]]:
    """Every schema node that can describe the data at `parts` (all branches kept)."""
    nodes = branches(root, root)
    for t in parts:
        nxt: list[dict[str, Any]] = []
        for n in nodes:
            if t.isdigit() and "items" in n:
                nxt.extend(branches(root, n["items"]))
            props = n.get("properties", {})
            if t in props:
                nxt.extend(branches(root, props[t]))
            ap = n.get("additionalProperties")
            if isinstance(ap, dict) and t not in props and not t.isdigit():
                nxt.extend(branches(root, ap))
        nodes = nxt
    return nodes


def schema_history(repo: Path) -> dict[str, Any]:
    """The schema history: through `schema_history()` of `scripts/schema_snapshots.py` once
    ADR 0017 provides it (the only reader allowed then), else the committed index."""
    import importlib.util

    spec = importlib.util.spec_from_file_location(
        "schema_snapshots", repo / "scripts/schema_snapshots.py"
    )
    if spec and spec.loader:
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        if hasattr(module, "schema_history"):
            history: dict[str, Any] = module.schema_history()
            return history
    index: dict[str, Any] = json.loads((repo / "schemas/index.json").read_text(encoding="utf-8"))
    return index


INDEX = re.compile(r"0|[1-9][0-9]*")


def input_order_key(root: dict[str, Any], pointer: str) -> list[tuple[Any, ...]]:
    """The place of `pointer` in a depth-first walk of the input, objects in the property order
    of the input schema `root`, arrays by index (R9: input order, the input taken with its
    defaults; SPEC-0013 R9). A token the schema does not declare sorts after every declared
    one. The engine sorts `meta.defaults_applied` by the same rule (`registry::in_input_order`)."""
    nodes = branches(root, root)
    key: list[tuple[Any, ...]] = []
    for t in tokens(pointer):
        if INDEX.fullmatch(t) and any("items" in n for n in nodes):
            key.append((0, int(t)))
            nodes = [b for n in nodes if "items" in n for b in branches(root, n["items"])]
            continue
        position = next(
            (list(n["properties"]).index(t) for n in nodes if t in n.get("properties", {})), None
        )
        key.append((1, sys.maxsize if position is None else position, t))
        nodes = [
            b
            for n in nodes
            if t in n.get("properties", {})
            for b in branches(root, n["properties"][t])
        ]
    return key
