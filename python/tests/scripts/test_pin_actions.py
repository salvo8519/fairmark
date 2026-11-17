"""scripts/pin_actions.py: actions pinned to the commit of their tag; refs moved to inputs."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest
from conftest import load_script

pin_actions = load_script("pin_actions")
SHA = "a" * 40
LS_REMOTE = f"{'b' * 40}\trefs/tags/v4\n{SHA}\trefs/tags/v4^{{}}\n{'c' * 40}\trefs/heads/stable\n"


@pytest.fixture
def remote(monkeypatch: pytest.MonkeyPatch) -> list[list[str]]:
    calls: list[list[str]] = []

    def fake_run(cmd: list[str], **_: Any) -> subprocess.CompletedProcess[str]:
        calls.append(cmd)
        return subprocess.CompletedProcess(cmd, 0, LS_REMOTE, "")

    monkeypatch.setattr(pin_actions.subprocess, "run", fake_run)
    return calls


def test_resolve_peels_an_annotated_tag_and_caches(remote: list[list[str]]) -> None:
    cache: dict[tuple[str, str], str] = {}
    assert pin_actions.resolve("actions/checkout", "v4", cache) == SHA
    assert pin_actions.resolve("actions/checkout/sub", "v4", cache) == SHA
    assert len(remote) == 1


def test_pin_file_keeps_the_tag_as_a_comment_and_moves_the_toolchain(
    remote: list[list[str]], tmp_path: Path
) -> None:
    workflow = tmp_path / "ci.yml"
    workflow.write_text(
        "steps:\n"
        "  - uses: actions/checkout@v4\n"
        "  - uses: dtolnay/rust-toolchain@stable\n"
        f"  - uses: actions/cache@{SHA} # v4\n"
        "  - uses: ./local-action\n",
        encoding="utf-8",
    )
    pin_actions.pin_file(workflow, {})
    lines = workflow.read_text(encoding="utf-8").splitlines()
    assert lines[1] == f"  - uses: actions/checkout@{SHA} # v4"
    assert lines[2] == f"  - uses: dtolnay/rust-toolchain@{'c' * 40} # stable"
    assert lines[3:5] == ["    with:", "      toolchain: stable"]
    assert lines[5] == f"  - uses: actions/cache@{SHA} # v4"


def test_resolve_fails_on_an_unknown_ref(monkeypatch: pytest.MonkeyPatch) -> None:
    monkeypatch.setattr(
        pin_actions.subprocess,
        "run",
        lambda cmd, **_: subprocess.CompletedProcess(cmd, 0, "", ""),
    )
    with pytest.raises(SystemExit, match="no such tag or branch"):
        pin_actions.resolve("actions/checkout", "v99", {})


def test_check_lists_unpinned_actions(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    workflow = tmp_path / "a.yml"
    workflow.write_text("steps:\n  - uses: actions/checkout@v4\n", encoding="utf-8")
    monkeypatch.setattr(pin_actions, "ROOT", tmp_path)
    monkeypatch.setattr(pin_actions, "WORKFLOWS", [workflow])
    assert pin_actions.unpinned() == ["a.yml:2: actions/checkout@v4"]
    monkeypatch.setattr(sys, "argv", ["pin_actions.py", "--check"])
    with pytest.raises(SystemExit):
        pin_actions.main()


def test_every_workflow_of_the_repository_is_pinned() -> None:
    assert pin_actions.unpinned() == []
