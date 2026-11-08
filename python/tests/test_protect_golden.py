"""The golden case protection hook in .claude/hooks/protect_golden.py (issue #192)."""

import importlib.util
import json
import subprocess
import sys
from pathlib import Path

import pytest

HOOK = Path(__file__).resolve().parents[2] / ".claude" / "hooks" / "protect_golden.py"
_spec = importlib.util.spec_from_file_location("protect_golden", HOOK)
assert _spec and _spec.loader
protect_golden = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(protect_golden)


def _git(cwd: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=cwd, check=True, capture_output=True)


@pytest.fixture
def repo(tmp_path: Path) -> Path:
    """A repository whose `main` holds one golden case, plus a linked worktree."""
    origin = tmp_path / "origin"
    origin.mkdir()
    _git(origin, "init", "-q", "-b", "main")
    _git(origin, "config", "user.email", "test@example.com")
    _git(origin, "config", "user.name", "Test")
    case = origin / "tests" / "golden" / "m" / "existing.yaml"
    case.parent.mkdir(parents=True)
    case.write_text("expected: 1\n", encoding="utf-8")
    _git(origin, "add", ".")
    _git(origin, "commit", "-q", "-m", "golden")
    _git(origin, "worktree", "add", "-q", "-b", "feature", str(tmp_path / "lane"))
    return tmp_path


@pytest.mark.parametrize("checkout", ["origin", "lane"])
def test_existing_case_on_base_is_blocked_in_every_worktree(repo: Path, checkout: str) -> None:
    path = repo / checkout / "tests" / "golden" / "m" / "existing.yaml"
    assert protect_golden.reason_to_block(str(path), "main") == "exists on main"


def test_case_new_on_the_branch_stays_editable(repo: Path) -> None:
    path = repo / "lane" / "tests" / "golden" / "m" / "new.yaml"
    path.write_text("expected: 2\n", encoding="utf-8")
    assert protect_golden.reason_to_block(str(path), "main") is None


def test_files_outside_golden_are_not_checked(repo: Path) -> None:
    assert protect_golden.reason_to_block(str(repo / "lane" / "README.md"), "main") is None


def test_unknown_base_protects_existing_files(repo: Path) -> None:
    path = repo / "lane" / "tests" / "golden" / "m" / "existing.yaml"
    reason = protect_golden.reason_to_block(str(path), "origin/missing")
    assert reason is not None and "not available" in reason
    new = repo / "lane" / "tests" / "golden" / "m" / "other.yaml"
    assert protect_golden.reason_to_block(str(new), "origin/missing") is None


def test_outside_a_repository_existing_files_are_protected(tmp_path: Path) -> None:
    path = tmp_path / "tests" / "golden" / "case.yaml"
    path.parent.mkdir(parents=True)
    path.write_text("x: 1\n", encoding="utf-8")
    assert protect_golden.reason_to_block(str(path), "main") == "is an existing golden case"


def test_hook_exit_status_and_message(repo: Path) -> None:
    path = repo / "lane" / "tests" / "golden" / "m" / "existing.yaml"
    result = subprocess.run(
        [sys.executable, str(HOOK)],
        input=json.dumps({"tool_input": {"file_path": str(path)}}),
        capture_output=True,
        text=True,
        encoding="utf-8",
        env={"FAIRMARK_GOLDEN_BASE": "main", "PATH": "/usr/bin:/bin"},
        check=False,
    )
    assert result.returncode == 2
    assert "numerical-change" in result.stderr
