"""scripts/check_task_ids.py: a task id defined twice fails; references are not definitions."""

from __future__ import annotations

from pathlib import Path

import pytest
from conftest import load_script

check_task_ids = load_script("check_task_ids")


def run(monkeypatch: pytest.MonkeyPatch, tmp_path: Path, text: str) -> None:
    tasks = tmp_path / "specs" / "0001-x" / "tasks.md"
    tasks.parent.mkdir(parents=True)
    tasks.write_text(text, encoding="utf-8")
    monkeypatch.setattr(check_task_ids, "ROOT", tmp_path)
    monkeypatch.setattr(check_task_ids, "tracked_tasks_files", lambda: [tasks])
    check_task_ids.main()


def test_a_task_defined_twice_fails(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    with pytest.raises(SystemExit, match="T2 is defined on lines 2, 3"):
        run(monkeypatch, tmp_path, "- [ ] T1 one\n- [x] T2 two\n* [ ] T2 again\n")


def test_references_are_not_definitions(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    run(monkeypatch, tmp_path, "- [ ] T1 one\n- [ ] T2 after T1\nT1 and T2 are linked.\n")
