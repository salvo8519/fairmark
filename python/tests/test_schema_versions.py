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
            if "output_file" in record:
                assert record["output_file"] == f"{method}/v{version}.output.json"
                path = ROOT / "schemas" / record["output_file"]
                data = json.loads(path.read_text(encoding="utf-8"))
                assert data["$id"] == f"urn:fairmark:schema:{method}:v{version}:output"
                assert data["x-fairmark-method-version"] == version
        # ADR 0014 G8: the current version of every method has its output snapshot.
        assert "output_file" in entry["versions"][entry["current"]], method


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
    engine["tvm.npv"] = ("1", (head / "tvm.npv/v1.json").read_text(encoding="utf-8"), "{}\n")
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
    for method, (_, text, _output) in snap.current().items():
        (flat / f"{method}.json").write_text(text, encoding="utf-8", newline="\n")
    assert snap.diff_against(flat) == 0
    edit(flat / "tvm.npv.json", lambda d: d["properties"].__setitem__("gone", {"type": "string"}))
    assert snap.diff_against(flat) == 3


def test_version_keys_sort_numerically() -> None:
    index = {"format": 1, "methods": {"m.x": {"current": "10", "versions": {"10": {}, "9": {}}}}}
    text = snap.dump_index(index)
    assert text.index('"9": {') < text.index('"10": {')


# ---- output snapshots (issue #265, ADR 0014 G8)


def test_removing_an_output_field_is_breaking(repo: tuple[Path, Path]) -> None:
    base, _ = repo
    # The base had an output field that the engine no longer writes (removed or renamed).
    edit(
        base / "tvm.npv/v2.output.json",
        lambda d: d["properties"].__setitem__("legacy", {"type": "string"}),
    )
    assert snap.diff_against(base) == 3


def test_an_output_field_made_nullable_or_retyped_is_breaking() -> None:
    old = json.loads((ROOT / "schemas/tvm.npv/v2.output.json").read_text(encoding="utf-8"))
    nullable = json.loads(json.dumps(old))
    nullable["properties"]["npv"]["type"] = ["string", "null"]
    assert snap.classify("tvm.npv", old, nullable, output=True) == ["tvm.npv.npv: 'type' changed"]
    retyped = json.loads(json.dumps(old))
    retyped["properties"]["npv"] = {"type": "number"}
    assert snap.classify("tvm.npv", old, retyped, output=True)


def test_an_output_field_that_may_be_absent_is_breaking() -> None:
    old = json.loads((ROOT / "schemas/tvm.npv/v2.output.json").read_text(encoding="utf-8"))
    new = json.loads(json.dumps(old))
    new["required"].remove("npv")
    assert snap.classify("tvm.npv", old, new, output=True) == ["tvm.npv.npv: may now be absent"]
    # The same change to an input is additive: the field became optional.
    assert snap.classify("tvm.npv", old, new, output=False) == []


def test_a_new_output_field_is_additive(repo: tuple[Path, Path]) -> None:
    base, _ = repo

    # The base lacked an output field the engine now writes, even a required one.
    def drop(d: dict[str, Any]) -> None:
        field = next(iter(d["properties"]))
        d["properties"].pop(field)
        d["required"].remove(field)

    edit(base / "tvm.irr/v3.output.json", drop)
    assert snap.diff_against(base) == 0


def test_earlier_output_snapshots_are_frozen(
    repo: tuple[Path, Path], monkeypatch: pytest.MonkeyPatch
) -> None:
    base, head = repo
    # Pretend tvm.npv moved to v3: its v2 output snapshot is now an earlier version.
    engine = snap.current()
    _version, text, output = engine["tvm.npv"]
    engine["tvm.npv"] = ("3", text.replace(":v2", ":v3"), output.replace(":v2", ":v3"))
    monkeypatch.setattr(snap, "current", lambda: engine)
    assert snap.write() == 0
    assert snap.check() == 0
    assert snap.diff_against(base) == 0
    edit(head / "tvm.npv/v2.output.json", lambda d: d.__setitem__("description", "rewritten"))
    assert snap.diff_against(base) == 4
    assert snap.check() == 1


def test_a_stale_or_unindexed_output_snapshot_fails_the_check(repo: tuple[Path, Path]) -> None:
    _, head = repo
    edit(head / "tvm.npv/v2.output.json", lambda d: d.__setitem__("description", "stale"))
    assert snap.check() == 1
    (head / "tvm.npv/v2.output.json").unlink()
    assert snap.check() == 1
    assert snap.write() == 0 and snap.check() == 0
    (head / "tvm.npv/v9.output.json").write_text("{}\n", encoding="utf-8", newline="\n")
    assert snap.check() == 1


def test_versions_without_an_output_snapshot_stay_valid(repo: tuple[Path, Path]) -> None:
    base, _ = repo
    # tvm.npv v1 was published before output snapshots existed: it has none, and needs none.
    index = json.loads((base / "index.json").read_text(encoding="utf-8"))
    assert "output_file" not in index["methods"]["tvm.npv"]["versions"]["1"]
    assert snap.check() == 0
    # A base without any output snapshot (main before #265) compares cleanly.
    for path in base.glob("*/v*.output.json"):
        path.unlink()
    for entry in index["methods"].values():
        for record in entry["versions"].values():
            record.pop("output_file", None)
            record.pop("output_sha256", None)
    (base / "index.json").write_text(snap.dump_index(index), encoding="utf-8", newline="\n")
    assert snap.diff_against(base) == 0


def test_output_enumerations_may_widen_but_not_narrow() -> None:
    # Maintainer decision of 2026-11-09 (ADR 0014 G8): a value added to an output enumeration
    # is additive, clients accept unknown values; a value removed is breaking.
    old = {"type": "object", "properties": {"s": {"type": "string", "enum": ["a", "b"]}}}
    wider = {"type": "object", "properties": {"s": {"type": "string", "enum": ["a", "b", "c"]}}}
    assert snap.classify("m output", old, wider, output=True) == []
    assert snap.classify("m output", wider, old, output=True) == ["m output.s: 'enum' changed"]
    documented = {"oneOf": [{"const": "a", "type": "string"}, {"const": "b", "type": "string"}]}
    added = {"oneOf": [*documented["oneOf"], {"const": "c", "type": "string"}]}
    assert snap.classify("m output", documented, added, output=True) == []
    assert snap.classify("m output", added, documented, output=True) == [
        "m output: value '\"c\"' removed"
    ]


def test_a_renamed_output_file_is_reported_and_the_input_still_checked(
    repo: tuple[Path, Path], capsys: pytest.CaptureFixture[str]
) -> None:
    base, head = repo

    def rewrite(d: dict[str, Any]) -> None:
        record = d["methods"]["tvm.npv"]["versions"]["2"]
        record["output_file"] = "tvm.npv/v2.out.json"
        record["sha256"] = "0" * 64  # the input record of the same version, also rewritten

    edit(head / "index.json", rewrite)
    assert snap.diff_against(base) == 4
    out = capsys.readouterr().out
    assert "tvm.npv/v2.output.json: file name changed in the index" in out
    assert "tvm.npv/v2.json: hash changed in the index" in out
