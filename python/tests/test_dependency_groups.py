"""Development dependencies the tests import directly are declared, not borrowed from an extra
(SPEC-0013 R10, AC10)."""

from __future__ import annotations

import re
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]


def locked_version(lock: dict, name: str) -> tuple[int, ...]:
    package = next(p for p in lock["package"] if p["name"] == name)
    return tuple(int(x) for x in package["version"].split(".")[:3])


def test_jsonschema_is_a_declared_dev_dependency_satisfied_by_the_lock() -> None:
    project = tomllib.loads((ROOT / "pyproject.toml").read_text(encoding="utf-8"))
    dev = project["dependency-groups"]["dev"]
    declared = [d for d in dev if re.match(r"jsonschema\b(?!-)", d)]
    assert len(declared) == 1, dev
    floor = tuple(int(x) for x in re.search(r">=([\d.]+)", declared[0]).group(1).split("."))  # type: ignore[union-attr]
    lock = tomllib.loads((ROOT / "uv.lock").read_text(encoding="utf-8"))
    assert locked_version(lock, "jsonschema") >= floor
    fairmark = next(p for p in lock["package"] if p["name"] == "fairmark")
    dev_locked = {d["name"] for d in fairmark["dev-dependencies"]["dev"]}
    assert "jsonschema" in dev_locked
