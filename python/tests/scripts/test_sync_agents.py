"""scripts/sync_agents.py: `.claude/` mirrors `.agents/`, with the notice after the front matter."""

from __future__ import annotations

import re
import sys
import tomllib
from pathlib import Path

import pytest
import yaml
from conftest import load_script, write

sync_agents = load_script("sync_agents")
SKILL = "---\nname: x\ndescription: y\n---\n\nBody.\n"
R_ROLE = "---\nname: r\ndescription: d\ntools: Read\n---\n\nRole.\n"


@pytest.fixture
def tree(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> Path:
    write(tmp_path, {".agents/skills/x/SKILL.md": SKILL, ".agents/agents/r.md": R_ROLE})
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
    role = (tree / ".claude/agents/r.md").read_text(encoding="utf-8")
    assert role.startswith("---\nname: r\n") and "\n---\n\n<!-- Generated from " in role
    run(monkeypatch, "--check")


def test_check_fails_on_a_stale_mirror(tree: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    run(monkeypatch)
    (tree / ".agents/agents/r.md").write_text(R_ROLE + "Changed.\n", encoding="utf-8")
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


ROLE = (
    "---\nname: {name}\ndescription: Reviews things.\n  Read-only.\ntools: {tools}\n---\n\n"
    "Body with '''quotes''' and a \"double\" one.\n"
)


@pytest.fixture
def roles(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> Path:
    write(
        tmp_path,
        {
            ".agents/skills/x/SKILL.md": SKILL,
            ".agents/agents/reviewer.md": ROLE.format(name="reviewer", tools="Read, Grep, Bash"),
            ".agents/agents/writer.md": "---\nname: writer\ndescription: Writes.\n"
            "tools: Read, Glob, Write, Edit\n---\n\nWrite code.\n",
        },
    )
    monkeypatch.setattr(sync_agents, "ROOT", tmp_path)
    monkeypatch.setattr(sync_agents, "SRC", tmp_path / ".agents")
    monkeypatch.setattr(sync_agents, "DST", tmp_path / ".claude")
    return tmp_path


def front(text: str) -> dict[str, object]:
    head, _ = sync_agents.split_front_matter(text)
    return yaml.safe_load(head)


def test_every_agent_system_gets_its_role_format(
    roles: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    run(monkeypatch)
    codex = tomllib.loads((roles / ".codex/agents/reviewer.toml").read_text(encoding="utf-8"))
    assert codex["name"] == "reviewer"
    assert codex["description"] == "Reviews things. Read-only."
    assert codex["sandbox_mode"] == "read-only"
    assert codex["developer_instructions"] == "Body with '''quotes''' and a \"double\" one.\n"
    writer = tomllib.loads((roles / ".codex/agents/writer.toml").read_text(encoding="utf-8"))
    assert "sandbox_mode" not in writer and writer["developer_instructions"] == "Write code.\n"

    copilot = (roles / ".github/agents/reviewer.agent.md").read_text(encoding="utf-8")
    assert front(copilot) == {
        "name": "reviewer",
        "description": "Reviews things. Read-only.",
        "tools": ["read", "search", "execute"],
    }
    assert front((roles / ".github/agents/writer.agent.md").read_text(encoding="utf-8"))[
        "tools"
    ] == ["read", "search", "edit"]
    assert "<!-- Generated from .agents/agents/reviewer.md" in copilot

    assert front((roles / ".cursor/agents/reviewer.md").read_text(encoding="utf-8"))["readonly"]
    assert not front((roles / ".cursor/agents/writer.md").read_text(encoding="utf-8"))["readonly"]
    assert (roles / ".claude/agents/reviewer.md").exists()
    assert not (roles / ".codex/skills").exists()  # skills are read natively from .agents/
    run(monkeypatch, "--check")


@pytest.mark.parametrize(
    ("source", "message"),
    [
        ("No front matter.\n", "front matter"),
        ("---\nname: reviewer\ntools: Read\n---\nx\n", "'description' is required"),
        ("---\nname: other\ndescription: d\ntools: Read\n---\nx\n", "differs from the file"),
        ("---\nname: reviewer\ndescription: d\ntools: Read, WebFetch\n---\nx\n", "WebFetch"),
    ],
)
def test_a_role_that_cannot_be_translated_fails(
    roles: Path, monkeypatch: pytest.MonkeyPatch, source: str, message: str
) -> None:
    (roles / ".agents/agents/reviewer.md").write_text(source, encoding="utf-8")
    with pytest.raises(SystemExit, match=re.escape(message)):
        run(monkeypatch)


def test_a_file_added_by_hand_to_a_role_mirror_fails_the_check(
    roles: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    run(monkeypatch)
    (roles / ".github/agents/manual.agent.md").write_text("x\n", encoding="utf-8")
    with pytest.raises(SystemExit, match=re.escape("manual.agent.md")):
        run(monkeypatch, "--check")
    run(monkeypatch)
    assert not (roles / ".github/agents/manual.agent.md").exists()


def test_the_repository_roles_translate() -> None:
    files = {p.relative_to(sync_agents.ROOT).as_posix() for p in sync_agents.expected()}
    names = {p.stem for p in (sync_agents.SRC / "agents").glob("*.md")}
    for name in names:
        assert f".codex/agents/{name}.toml" in files
        assert f".cursor/agents/{name}.md" in files
        assert f".github/agents/{name}.agent.md" in files


def test_every_role_mirror_belongs_to_the_agents_lane_group() -> None:
    sprint = load_script("sprint")
    group = sprint.GENERATED_GROUPS["agents"]
    for folder, _, _ in sync_agents.ROLE_MIRRORS:
        assert f"{folder}/**" in group, folder
    assert ".claude/**" in group
