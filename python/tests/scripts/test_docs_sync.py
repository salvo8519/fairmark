"""scripts/docs_sync.py: code changes without their documentation fail; claims and coverage."""

from __future__ import annotations

import argparse
from pathlib import Path

import pytest
from conftest import git, load_script, write

docs_sync = load_script("docs_sync")


def impact(monkeypatch: pytest.MonkeyPatch, repo: Path, *, hook: bool = False) -> int:
    monkeypatch.setattr(docs_sync, "ROOT", repo)
    args = argparse.Namespace(base="main", staged=False, worktree=False, hook=hook)
    return int(docs_sync.impact(args))


@pytest.fixture
def branch(repo: Path) -> Path:
    write(repo, {"README.md": "x\n"})
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "base")
    git(repo, "switch", "-q", "-c", "work")
    return repo


def commit(repo: Path, files: dict[str, str], message: str = "change") -> None:
    write(repo, files)
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", message)


def test_mcp_change_without_its_guide_fails(
    branch: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    commit(branch, {"python/fairmark/mcp_server.py": "x = 1\n"})
    assert impact(monkeypatch, branch) == 1
    assert "update docs/en/guides/mcp.md" in capsys.readouterr().out


def test_a_method_area_needs_its_methodology_page(
    branch: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    commit(branch, {"crates/fairmark-core/src/tvm.rs": "// x\n"})
    assert impact(monkeypatch, branch) == 1
    assert "method area 'tvm' changed" in capsys.readouterr().out
    commit(branch, {"docs/en/methods/tvm-npv.md": "# NPV\n"})
    assert impact(monkeypatch, branch) == 0


def test_the_trailer_waives_the_check(branch: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    commit(
        branch,
        {"python/fairmark/mcp_server.py": "x = 1\n"},
        "chore(mcp): rename\n\nDocs-impact: none",
    )
    assert impact(monkeypatch, branch) == 0


def test_the_hook_mode_exits_2_on_stderr(
    branch: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    commit(branch, {"crates/fairmark-cli/src/main.rs": "// x\n"})
    assert impact(monkeypatch, branch, hook=True) == 2
    assert "docs-writer" in capsys.readouterr().err


def test_claims_reject_plans_and_unknown_methods(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    write(
        tmp_path,
        {
            "README.md": "Uses `tvm.npv`.\n",
            "README.it.md": "Usa `tvm.npv`.\n",
            "docs/en/guides/a.md": "This is planned for later. See `tvm.nope`.\n",
            "docs/en/project/vision.md": "Planned: everything.\n",
        },
    )
    monkeypatch.setattr(docs_sync, "ROOT", tmp_path)
    assert docs_sync.claims() == 1
    out = capsys.readouterr().out
    assert "docs/en/guides/a.md:1: plans belong to the roadmap" in out
    assert "`tvm.nope` is not a registered method" in out
    assert "vision.md" not in out


def test_coverage_of_the_repository_passes() -> None:
    assert docs_sync.coverage() == 0
