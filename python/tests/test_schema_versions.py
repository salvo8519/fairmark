"""Rules of the versioned schema snapshots (scripts/schema_snapshots.py, ADR 0009)."""

from __future__ import annotations

import importlib.util
import json
import shutil
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "schema_snapshots", ROOT / "scripts/schema_snapshots.py"
)
assert _spec and _spec.loader
snap = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(snap)


@pytest.fixture
def repo(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> tuple[Path, Path]:
    """A copy of the committed schemas as 'base' and as 'head', with the engine as built."""
    base = tmp_path / "base"
    head = tmp_path / "head"
    shutil.copytree(ROOT / "schemas", base)
    shutil.copytree(ROOT / "schemas", head)
    monkeypatch.setattr(snap, "SCHEMAS", head)
    monkeypatch.setattr(snap, "INDEX", head / "index.json")
    return base, head


def edit(path: Path, change: Any) -> None:
    data = json.loads(path.read_text(encoding="utf-8"))
    change(data)
    path.write_text(snap.dump(data), encoding="utf-8", newline="\n")


def test_committed_snapshots_are_current() -> None:
    assert snap.check() == 0


def test_unchanged_schemas_pass(repo: tuple[Path, Path]) -> None:
    base, _ = repo
    assert snap.diff_against(base) == 0


def test_removing_a_field_from_an_existing_version_is_breaking(repo: tuple[Path, Path]) -> None:
    base, _ = repo
    # The base had an optional field that the engine no longer accepts.
    edit(
        base / "tvm.mirr/v1.json",
        lambda d: d["properties"].__setitem__("legacy_field", {"type": "string"}),
    )
    assert snap.diff_against(base) == 3


def test_earlier_versions_are_frozen(repo: tuple[Path, Path]) -> None:
    base, head = repo
    edit(head / "tvm.npv/v1.json", lambda d: d.__setitem__("description", "rewritten"))
    assert snap.diff_against(base) == 4


def test_removing_a_version_file_is_refused(repo: tuple[Path, Path]) -> None:
    base, head = repo
    (head / "tvm.irr/v1.json").unlink()
    assert snap.diff_against(base) == 4


def test_a_new_optional_field_in_the_current_version_is_additive(
    repo: tuple[Path, Path],
) -> None:
    base, _ = repo
    # The base lacked a field the engine now has as optional: additive, allowed.
    edit(base / "tvm.xnpv/v1.json", lambda d: d["properties"].pop("compounding"))
    assert snap.diff_against(base) == 0


def test_every_snapshot_names_its_method_and_version() -> None:
    index = json.loads((ROOT / "schemas/index.json").read_text(encoding="utf-8"))
    for method, entry in index["methods"].items():
        for version, record in entry["versions"].items():
            data = json.loads((ROOT / "schemas" / record["file"]).read_text(encoding="utf-8"))
            assert data["$id"] == f"urn:fairmark:schema:{method}:v{version}"
            assert data["x-fairmark-method-version"] == version


def test_a_removed_definition_is_breaking(repo: tuple[Path, Path]) -> None:
    base, _ = repo
    edit(
        base / "coc.wacc/v1.json",
        lambda d: d.setdefault("$defs", {}).__setitem__("Legacy", {"type": "string"}),
    )
    assert snap.diff_against(base) == 3


def test_documented_enumerations_may_widen_but_not_narrow() -> None:
    # schemars generates documented enums as oneOf of const members, as in rates.convert.
    real = json.loads((ROOT / "schemas/rates.convert/v1.json").read_text(encoding="utf-8"))
    name = next(n for n, d in real["$defs"].items() if snap.const_members(d.get("oneOf")))
    widened = json.loads(json.dumps(real))
    widened["$defs"][name]["oneOf"].append({"const": "new_value", "type": "string"})
    assert snap.classify("m", real, widened) == []
    assert snap.classify("m", widened, real)  # the reverse removes a value


@pytest.mark.parametrize(
    ("key", "value"),
    [
        ("additionalProperties", False),
        ("minimum", "0"),
        ("maximum", "1"),
        ("exclusiveMinimum", "0"),
        ("maxItems", 3),
        ("minLength", 1),
        ("multipleOf", "0.01"),
    ],
)
def test_new_constraints_are_breaking(key: str, value: Any) -> None:
    old = {"properties": {"x": {"type": "string"}}}
    new = {"properties": {"x": {"type": "string", key: value}}}
    assert snap.classify("m", old, new)


def test_introduced_in_is_stamped_only_with_a_release(repo: tuple[Path, Path]) -> None:
    base, head = repo
    edit(
        head / "index.json",
        lambda d: d["methods"]["tvm.irr"]["versions"]["1"].__setitem__("introduced_in", "soon"),
    )
    assert snap.diff_against(base) == 4


def test_a_narrowed_enumeration_inside_a_definition_is_breaking() -> None:
    old = {"$defs": {"Basis": {"enum": ["a", "b"]}}}
    assert snap.classify("m", old, {"$defs": {"Basis": {"enum": ["a"]}}})
    assert not snap.classify("m", old, {"$defs": {"Basis": {"enum": ["a", "b", "c"]}}})


def test_withdrawing_a_method_is_breaking(
    repo: tuple[Path, Path], monkeypatch: pytest.MonkeyPatch
) -> None:
    base, _ = repo
    engine = snap.current()
    engine.pop("tvm.mirr")
    monkeypatch.setattr(snap, "current", lambda: engine)
    assert snap.diff_against(base) == 3


def test_a_current_version_cannot_go_back(
    repo: tuple[Path, Path], monkeypatch: pytest.MonkeyPatch
) -> None:
    base, head = repo
    engine = snap.current()
    engine["tvm.npv"] = ("1", (head / "tvm.npv/v1.json").read_text(encoding="utf-8"))
    monkeypatch.setattr(snap, "current", lambda: engine)
    edit(head / "index.json", lambda d: d["methods"]["tvm.npv"].__setitem__("current", "1"))
    assert snap.diff_against(base) == 4
    # Generating snapshots from such an engine is refused too, before anything is written.
    shutil.copy(base / "index.json", head / "index.json")
    assert snap.write() == 4
    assert (head / "index.json").read_text(encoding="utf-8") == (base / "index.json").read_text(
        encoding="utf-8"
    )


def test_a_released_entry_of_the_index_is_immutable(repo: tuple[Path, Path]) -> None:
    base, head = repo
    released = lambda d: d["methods"]["tvm.irr"]["versions"]["1"].__setitem__(  # noqa: E731
        "introduced_in", "0.1.0"
    )
    edit(base / "index.json", released)
    assert snap.diff_against(base) == 4  # head says "unreleased": the history was altered
    edit(head / "index.json", released)
    assert snap.diff_against(base) == 0


def test_stamping_a_release_is_allowed(repo: tuple[Path, Path]) -> None:
    base, _ = repo
    assert snap.stamp_release("0.1.0") == sum(
        len(e["versions"])
        for e in json.loads((base / "index.json").read_text(encoding="utf-8"))["methods"].values()
    )
    assert snap.diff_against(base) == 0


def test_a_hash_rewritten_in_the_index_is_refused(repo: tuple[Path, Path]) -> None:
    base, head = repo
    edit(
        head / "index.json",
        lambda d: d["methods"]["tvm.irr"]["versions"]["1"].__setitem__("sha256", "0" * 64),
    )
    assert snap.diff_against(base) == 4


def test_stray_and_unindexed_files_fail_the_check(repo: tuple[Path, Path]) -> None:
    _, head = repo
    assert snap.check() == 0
    (head / "tvm.npv/v9.json").write_text("{}\n", encoding="utf-8", newline="\n")
    assert snap.check() == 1
    (head / "tvm.npv/v9.json").unlink()
    (head / "tvm.npv.json").write_text("{}\n", encoding="utf-8", newline="\n")
    assert snap.check() == 1


def test_flat_base_layout(repo: tuple[Path, Path], tmp_path: Path) -> None:
    flat = tmp_path / "flat"
    flat.mkdir()
    for method, (_, text) in snap.current().items():
        (flat / f"{method}.json").write_text(text, encoding="utf-8", newline="\n")
    assert snap.diff_against(flat) == 0
    edit(flat / "tvm.npv.json", lambda d: d["properties"].__setitem__("gone", {"type": "string"}))
    assert snap.diff_against(flat) == 3


def test_version_keys_sort_numerically() -> None:
    index = {"format": 1, "methods": {"m.x": {"current": "10", "versions": {"10": {}, "9": {}}}}}
    text = snap.dump_index(index)
    assert text.index('"9": {') < text.index('"10": {')
