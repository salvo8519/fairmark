"""scripts/golden_schema.py: the schema walk shared by the golden migration script and the
golden conformance test (code review of #394, S6)."""

from __future__ import annotations

from conftest import ROOT, load_script

gs = load_script("golden_schema")

SCHEMA = {
    "type": "object",
    "properties": {
        "rows": {"type": "array", "items": {"$ref": "#/$defs/Row"}},
        "choice": {"anyOf": [{"$ref": "#/$defs/Row"}, {"type": "null"}]},
    },
    "$defs": {"Row": {"type": "object", "properties": {"a/b": {"type": "string"}}}},
}


def test_tokens_unescape_rfc_6901() -> None:
    assert gs.tokens("/rows/0/a~1b") == ["rows", "0", "a/b"]
    assert gs.tokens("/x~0y") == ["x~y"]
    assert gs.tokens("field") == ["field"]


def test_resolve_follows_objects_and_arrays_and_refuses_the_rest() -> None:
    data = {"rows": [{"a/b": "1"}]}
    assert gs.resolve(data, ["rows", "0", "a/b"]) == "1"
    for parts in (["rows", "1"], ["rows", "01"], ["nope"], ["rows", "0", "a/b", "x"]):
        try:
            gs.resolve(data, parts)
        except KeyError:
            continue
        raise AssertionError(parts)


def test_branches_follow_refs_and_choices() -> None:
    nodes = gs.branches(SCHEMA, SCHEMA["properties"]["choice"])
    assert {"properties"} <= set(nodes[1]) and any(n.get("type") == "null" for n in nodes)
    assert gs.deref(SCHEMA, {"$ref": "#/$defs/Row"}) is SCHEMA["$defs"]["Row"]


def test_schema_at_reaches_through_items_and_refs() -> None:
    assert gs.schema_at(SCHEMA, ["rows", "0", "a/b"]) == [{"type": "string"}]
    assert gs.schema_at(SCHEMA, ["rows", "0", "missing"]) == []


def test_schema_history_names_every_method_and_the_envelope() -> None:
    history = gs.schema_history(ROOT)
    assert "envelope" in history and "tvm.npv" in history["methods"]
