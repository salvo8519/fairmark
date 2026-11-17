"""scripts/sync_agents.py: `.claude/` mirrors `.agents/`, with the notice after the front matter."""

from __future__ import annotations

import re
import sys
from pathlib import Path

import pytest
from conftest import load_script, write

sync_agents = load_script("sync_agents")
SKILL = "---\nname: x\ndescription: y\n---\n\nBody.\n"


@pytest.fixture
def tree(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> Path:
    write(tmp_path, {".agents/skills/x/SKILL.md": SKILL, ".agents/agents/r.md": "Role.\n"})
    monkeypatch.setattr(sync_agents, "ROOT", tmp_path)
    monkeypatch.setattr(sync_agents, "SRC", tmp_path / ".agents")
    monkeypatch.setattr(sync_agents, "DST", tmp_path / ".claude")
    return tmp_path


def run(monkeypatch: pytest.MonkeyPatch, *args: str) -> None:
    monkeypatch.setattr(sys, "argv", ["sync_agents.py", *args])
    sync_agents.main()


def test_generate_puts_the_notice_after_the_front_matter(
    tree: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    run(monkeypatch)
    text = (tree / ".claude/skills/x/SKILL.md").read_text(encoding="utf-8")
    assert text.startswith("---\nname: x\ndescription: y\n---\n\n<!-- Generated from ")
    assert (tree / ".claude/agents/r.md").read_text(encoding="utf-8").startswith("<!-- Generated")
    run(monkeypatch, "--check")


def test_check_fails_on_a_stale_mirror(tree: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    run(monkeypatch)
    (tree / ".agents/agents/r.md").write_text("Changed role.\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="out of sync"):
        run(monkeypatch, "--check")


def test_check_fails_on_a_file_edited_or_added_by_hand(
    tree: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    run(monkeypatch)
    (tree / ".claude/agents/extra.md").write_text("By hand.\n", encoding="utf-8")
    with pytest.raises(SystemExit, match=re.escape("extra.md")):
        run(monkeypatch, "--check")
    run(monkeypatch)  # regenerating removes it
    assert not (tree / ".claude/agents/extra.md").exists()
