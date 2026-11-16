"""scripts/sprint.py: sprint file validation and the per-issue state files (issue #318)."""

from __future__ import annotations

import importlib.util
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts" / "sprint.py"
_spec = importlib.util.spec_from_file_location("sprint", SCRIPT)
sp = importlib.util.module_from_spec(_spec)
sys.modules["sprint"] = sp
_spec.loader.exec_module(sp)


def sprint_file(tmp_path: Path) -> Path:
    path = tmp_path / "S01.yaml"
    path.write_text(
        """\
id: S01
goal: Numerical core ready for the complete DCF
start: 2026-11-09
end: 2026-11-22
milestone: "M1 · Numerical core"
status: active
lanes:
  - name: core
    issues: [12, 13]
  - name: docs
    issues: [40]
""",
        encoding="utf-8",
    )
    return path


@pytest.fixture
def fixed_now(monkeypatch: pytest.MonkeyPatch) -> None:
    import datetime as dt

    when = dt.datetime(2026, 11, 14, 22, 30, 0, tzinfo=dt.UTC)
    monkeypatch.setattr(sp, "_now", lambda: when)


def test_state_file_lives_next_to_the_sprint_file(tmp_path: Path) -> None:
    f = sprint_file(tmp_path)
    assert sp.state_path(f, 42) == tmp_path / "S01" / "state" / "42.yaml"


def test_write_state_creates_the_file_with_the_first_step(tmp_path: Path, fixed_now: None) -> None:
    f = sprint_file(tmp_path)
    state = sp.write_state(f, 12, branch="feat/12-irr-sign", step="spec")
    assert state == {
        "issue": 12,
        "lane": "core",
        "branch": "feat/12-irr-sign",
        "pull_request": None,
        "step": "spec",
        "blockers": [],
        "started": "2026-11-14T22:30:00+00:00",
        "updated": "2026-11-14T22:30:00+00:00",
        "agent_steps": 1,
    }
    assert sp.state_path(f, 12).exists()


def test_state_updates_increment_agent_steps_and_keep_started(
    tmp_path: Path, fixed_now: None
) -> None:
    import datetime as dt

    f = sprint_file(tmp_path)
    sp.write_state(f, 12, branch="feat/12-irr-sign", step="spec")
    later = dt.datetime(2026, 11, 14, 23, 5, 0, tzinfo=dt.UTC)
    sp._now = lambda: later
    state = sp.write_state(f, 12, step="pr", pull_request=33, blocker=["review comment"])
    assert state["started"] == "2026-11-14T22:30:00+00:00"
    assert state["updated"] == "2026-11-14T23:05:00+00:00"
    assert state["agent_steps"] == 2
    assert state["step"] == "pr"
    assert state["pull_request"] == 33
    assert state["blockers"] == ["review comment"]


def test_clear_blockers_keeps_the_other_fields(tmp_path: Path, fixed_now: None) -> None:
    f = sprint_file(tmp_path)
    sp.write_state(f, 40, branch="docs/40-guide", step="review", blocker=["waiting"])
    state = sp.write_state(f, 40, clear_blockers=True, step="done")
    assert state["blockers"] == []
    assert state["step"] == "done"
    assert state["branch"] == "docs/40-guide"


def test_write_state_rejects_an_issue_outside_the_sprint(tmp_path: Path) -> None:
    f = sprint_file(tmp_path)
    with pytest.raises(SystemExit, match="#99 is not in the lanes"):
        sp.write_state(f, 99, step="spec")


def test_read_state_returns_none_without_a_file(tmp_path: Path) -> None:
    f = sprint_file(tmp_path)
    assert sp.read_state(f, 12) is None


def test_state_summary_for_the_sprint_output() -> None:
    state = {
        "issue": 12,
        "lane": "core",
        "branch": "feat/12-irr-sign",
        "pull_request": 33,
        "step": "pr",
        "blockers": ["review comment"],
        "started": "2026-11-14T22:30:00+00:00",
        "updated": "2026-11-14T23:05:00+00:00",
        "agent_steps": 2,
    }
    summary = sp.state_summary(state, closed=False)
    assert "step pr" in summary and "PR #33" in summary and "1 blocker" in summary


def test_state_summary_flags_the_stale_file_of_a_closed_issue() -> None:
    state = {"issue": 12, "step": "done", "blockers": [], "pull_request": 33}
    assert "stale" in sp.state_summary(state, closed=True)


def test_state_rejects_conflicting_pull_request_forms(tmp_path: Path, fixed_now: None) -> None:
    f = sprint_file(tmp_path)
    with pytest.raises(SystemExit, match="pull request must be an integer"):
        sp.write_state(f, 12, pull_request="#33")  # type: ignore[arg-type]


def test_command_line_state_writes_and_prints(
    tmp_path: Path, capsys: pytest.CaptureFixture[str], fixed_now: None
) -> None:
    f = sprint_file(tmp_path)
    assert sp.main(["state", str(f), "12", "--branch", "feat/12-irr-sign", "--step", "spec"]) == 0
    out = capsys.readouterr().out
    assert "sprints state" in out or "state" in out
    assert sp.read_state(f, 12) is not None


def test_command_line_state_validates_the_sprint_first(tmp_path: Path) -> None:
    broken = tmp_path / "S02.yaml"
    broken.write_text("id: S02\nstatus: maybe\n", encoding="utf-8")
    with pytest.raises(SystemExit, match="Invalid sprint file"):
        sp.main(["state", str(broken), "1", "--step", "spec"])


# ------------------------------------------------------------------------------------------
# Shared state across worktrees (issue #333)


def _git(cwd: Path, *args: str) -> str:
    import subprocess

    env = {
        "GIT_AUTHOR_NAME": "t",
        "GIT_AUTHOR_EMAIL": "t@example.invalid",
        "GIT_COMMITTER_NAME": "t",
        "GIT_COMMITTER_EMAIL": "t@example.invalid",
        "GIT_CONFIG_NOSYSTEM": "1",
        "HOME": str(cwd),
        "PATH": __import__("os").environ["PATH"],
    }
    return subprocess.run(
        ["git", *args], cwd=cwd, env=env, capture_output=True, text=True, check=True
    ).stdout


@pytest.fixture
def repo_with_worktree(tmp_path: Path) -> tuple[Path, Path]:
    """A repository with sprints/S01.yaml committed and a second worktree of it."""
    main = tmp_path / "main"
    (main / "sprints").mkdir(parents=True)
    _git(main, "init", "-q", "-b", "main")
    sprint_file(main / "sprints")
    _git(main, "add", ".")
    _git(main, "commit", "-q", "-m", "sprint")
    lane = tmp_path / "lane"
    _git(main, "worktree", "add", "-q", str(lane), "-b", "fix/12-lane")
    return main, lane


def test_state_written_from_a_worktree_is_read_from_the_main_checkout(
    repo_with_worktree: tuple[Path, Path], fixed_now: None
) -> None:
    main, lane = repo_with_worktree
    sp.write_state(lane / "sprints" / "S01.yaml", 12, step="tests")
    state = sp.read_state(main / "sprints" / "S01.yaml", 12)
    assert state is not None and state["step"] == "tests"


def test_shared_state_lives_in_the_git_common_dir_outside_the_working_tree(
    repo_with_worktree: tuple[Path, Path],
) -> None:
    main, lane = repo_with_worktree
    expected = (main / ".git").resolve() / "fairmark" / "sprints" / "S01" / "state"
    assert sp.state_dir(main / "sprints" / "S01.yaml") == expected
    assert sp.state_dir(lane / "sprints" / "S01.yaml") == expected
    assert sp.state_path(lane / "sprints" / "S01.yaml", 12) == expected / "12.yaml"


def test_legacy_state_of_a_checkout_is_moved_once(
    repo_with_worktree: tuple[Path, Path], fixed_now: None
) -> None:
    main, lane = repo_with_worktree
    legacy = lane / "sprints" / "S01" / "state" / "13.yaml"
    legacy.parent.mkdir(parents=True)
    legacy.write_text("issue: 13\nstep: review\nupdated: '2026-11-14T10:00:00+00:00'\n")
    files = sp.state_files(main / "sprints" / "S01.yaml")
    assert files == []  # the legacy file belongs to the lane checkout, found from there
    files = sp.state_files(lane / "sprints" / "S01.yaml")
    assert [p.name for p in files] == ["13.yaml"]
    assert not legacy.exists()
    assert sp.read_state(main / "sprints" / "S01.yaml", 13)["step"] == "review"


def test_legacy_state_older_than_the_shared_one_is_dropped(
    repo_with_worktree: tuple[Path, Path], fixed_now: None
) -> None:
    main, lane = repo_with_worktree
    sp.write_state(main / "sprints" / "S01.yaml", 12, step="pr")  # updated 2026-11-14T22:30
    legacy = lane / "sprints" / "S01" / "state" / "12.yaml"
    legacy.parent.mkdir(parents=True)
    legacy.write_text("issue: 12\nstep: tests\nupdated: '2026-11-14T09:00:00+00:00'\n")
    assert sp.read_state(lane / "sprints" / "S01.yaml", 12)["step"] == "pr"
    assert not legacy.exists()


def test_state_files_are_written_atomically(
    repo_with_worktree: tuple[Path, Path], fixed_now: None
) -> None:
    main, _ = repo_with_worktree
    sp.write_state(main / "sprints" / "S01.yaml", 12, step="spec")
    directory = sp.state_dir(main / "sprints" / "S01.yaml")
    assert [p.name for p in directory.iterdir()] == ["12.yaml"]  # no temporary file left


def test_state_files_are_ordered_by_issue_number(
    repo_with_worktree: tuple[Path, Path], fixed_now: None
) -> None:
    main, _ = repo_with_worktree
    f = main / "sprints" / "S01.yaml"
    for issue in (40, 12, 13):
        sp.write_state(f, issue, step="spec")
    assert [p.stem for p in sp.state_files(f)] == ["12", "13", "40"]
