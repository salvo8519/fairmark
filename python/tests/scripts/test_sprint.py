"""scripts/sprint.py: sprint file validation and the per-issue state files (issue #318)."""

from __future__ import annotations

import importlib.util
import re
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[3]
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
    paths: ["crates/", "python/tests/test_core_*.py"]
    generates: [registry]
  - name: docs
    issues: [40]
    paths: ["docs/en/guides/", "docs/it/guides/"]
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
        "blocked_since": None,
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


# ------------------------------------------------------------------------------------------
# Lane paths (issue #334)


def lanes(*specs: tuple[str, list[str], list[str]]) -> list[dict[str, object]]:
    return [
        {"name": n, "issues": [i + 1], "paths": paths, "generates": gen}
        for i, (n, paths, gen) in enumerate(specs)
    ]


@pytest.mark.parametrize(
    ("path", "pattern", "expected"),
    [
        ("scripts/sprint.py", "scripts/", True),
        ("scripts/sub/x.py", "scripts/*.py", False),
        ("scripts/sub/x.py", "scripts/**/*.py", True),
        ("scripts/x.py", "scripts/**/*.py", True),
        ("docs/it/reference/methods.md", "docs/*/reference/methods.md", True),
        ("README.md", "**/*.md", True),
        ("python/tests/test_sprint.py", "python/tests/test_s?rint.py", True),
        ("scripts", "scripts/", False),
    ],
)
def test_pattern_grammar(path: str, pattern: str, expected: bool) -> None:
    assert sp.matches(path, pattern) is expected


@pytest.mark.parametrize(
    ("a", "b"),
    [
        ("docs/", "docs/*/reference/methods.md"),
        ("scripts/sprint.py", "scripts/*.py"),
        ("python/tests/test_s*.py", "python/tests/test_*.py"),
        (".agents/", "**/*.md"),
        ("schemas/**", "schemas/"),
        ("**", "x/y"),
    ],
)
def test_overlap_example_is_matched_by_both_patterns(a: str, b: str) -> None:
    example = sp.overlap_example(a, b)
    assert example is not None
    assert sp.matches(example, a) and sp.matches(example, b)


@pytest.mark.parametrize(
    ("a", "b"),
    [
        ("docs/en/guides/**", "docs/*/reference/methods.md"),
        ("scripts/*.md", "scripts/sprint.py"),
        ("crates/", "python/"),
        ("a/*.py", "a/*.md"),
        ("scripts/*.py", "scripts/sub/x.py"),
    ],
)
def test_disjoint_patterns_have_no_overlap(a: str, b: str) -> None:
    assert sp.overlap_example(a, b) is None


def test_disjoint_lanes_are_valid() -> None:
    plan = lanes(
        ("process", ["scripts/sprint.py", "sprints/"], []), ("core", ["crates/"], ["registry"])
    )
    assert sp.validate_paths(plan) == []


def test_a_lane_without_paths_is_rejected() -> None:
    plan = [{"name": "core", "issues": [1]}]
    assert sp.validate_paths(plan) == [
        "lane core: 'paths' must be a non-empty list of glob patterns"
    ]


def test_overlapping_lanes_are_rejected_with_an_example() -> None:
    plan = lanes(("a", ["scripts/"], []), ("b", ["scripts/*.py"], []))
    [error] = sp.validate_paths(plan)
    assert error.startswith("lanes a and b overlap: 'scripts/' and 'scripts/*.py' both cover")


def test_two_generating_lanes_are_rejected() -> None:
    plan = lanes(("a", ["crates/"], ["registry"]), ("b", ["python/fairmark/"], ["registry"]))
    assert "only one lane may generate the registry artifacts" in sp.validate_paths(plan)[0]


def test_a_lane_that_does_not_generate_cannot_cover_generated_artifacts() -> None:
    plan = lanes(("docs", ["docs/"], []))
    errors = sp.validate_paths(plan)
    assert any("generated artifact 'docs/x/reference/methods.md'" in e for e in errors)
    assert sp.validate_paths(lanes(("docs", ["docs/"], ["registry"]))) == []


def test_the_sprint_template_is_valid() -> None:
    template = ROOT / "sprints" / "_template.yaml"
    assert sp.validate_paths(sp.load(template)["lanes"]) == []


@pytest.fixture
def sprint_repo(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    """A repository on a sprint branch of issue 40 (lane docs of S01)."""
    repo = tmp_path / "repo"
    (repo / "sprints").mkdir(parents=True)
    _git(repo, "init", "-q", "-b", "main")
    sprint_file(repo / "sprints")
    _git(repo, "add", ".")
    _git(repo, "commit", "-q", "-m", "plan")
    _git(repo, "switch", "-q", "-c", "sprint/S01/40-guide")
    monkeypatch.chdir(repo)
    return repo


def commit_files(repo: Path, *paths: str) -> None:
    for path in paths:
        (repo / path).parent.mkdir(parents=True, exist_ok=True)
        (repo / path).write_text("x\n", encoding="utf-8")
    _git(repo, "add", ".")
    _git(repo, "commit", "-q", "-m", "work")


def check_diff(repo: Path, branch: str = "sprint/S01/40-guide") -> int:
    return sp.main(
        ["check-diff", "--branch", branch, "--base", "main", "--sprints", str(repo / "sprints")]
    )


def test_check_diff_accepts_files_inside_the_lane(sprint_repo: Path) -> None:
    commit_files(sprint_repo, "docs/en/guides/start.md", ".agents/journal/2026-11-16-x.md")
    assert check_diff(sprint_repo) == 0


def test_check_diff_accepts_the_navigation_of_a_new_page(sprint_repo: Path) -> None:
    commit_files(sprint_repo, "docs/en/guides/start.md", "zensical.toml")
    assert check_diff(sprint_repo) == 0


def test_check_diff_rejects_files_outside_the_lane(sprint_repo: Path) -> None:
    commit_files(sprint_repo, "docs/en/guides/start.md", "scripts/sprint.py")
    with pytest.raises(SystemExit, match=re.escape("Files outside the lane:\n  scripts/sprint.py")):
        check_diff(sprint_repo)


def test_check_diff_rejects_generated_files_in_a_lane_that_does_not_generate(
    sprint_repo: Path,
) -> None:
    commit_files(sprint_repo, "schemas/index.json")
    with pytest.raises(SystemExit, match=re.escape("schemas/index.json")):
        check_diff(sprint_repo)


def test_check_diff_allows_generated_files_in_the_generating_lane(sprint_repo: Path) -> None:
    _git(sprint_repo, "switch", "-q", "-c", "sprint/S01/12-core")
    commit_files(sprint_repo, "crates/fairmark-core/src/x.rs", "schemas/index.json")
    assert check_diff(sprint_repo, "sprint/S01/12-core") == 0


def test_check_diff_ignores_branches_outside_a_sprint(
    sprint_repo: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    assert check_diff(sprint_repo, "fix/334-lane-paths") == 0
    assert "not a sprint branch" in capsys.readouterr().out


# ------------------------------------------------------------------------------------------
# Generator groups (issue #347)


def test_two_lanes_may_generate_different_groups() -> None:
    plan = lanes(
        ("product", ["crates/", "schemas/"], ["registry"]),
        ("process", [".agents/", ".claude/"], ["agents"]),
    )
    assert sp.validate_paths(plan) == []


def test_two_lanes_may_not_generate_the_same_group() -> None:
    plan = lanes(("a", [".agents/skills/"], ["agents"]), ("b", [".agents/agents/"], ["agents"]))
    assert sp.validate_paths(plan) == [
        "only one lane may generate the agents artifacts; lanes: ['a', 'b']"
    ]


def test_a_lane_covering_the_artifacts_of_another_group_is_rejected() -> None:
    plan = lanes(("process", [".claude/", "schemas/"], ["agents"]))
    [error] = sp.validate_paths(plan)
    assert "'schemas/x' of the registry group" in error


@pytest.mark.parametrize("value", [True, "registry", ["registry", "docs"], [1]])
def test_generates_must_name_known_groups(value: object) -> None:
    plan = [{"name": "a", "issues": [1], "paths": ["crates/"], "generates": value}]
    assert sp.validate_paths(plan)[0].startswith("lane a: 'generates' must be a list of groups")


def test_check_diff_allows_only_the_artifacts_of_the_lane_groups(sprint_repo: Path) -> None:
    _git(sprint_repo, "switch", "-q", "-c", "sprint/S01/12-core")
    commit_files(sprint_repo, "schemas/index.json", ".claude/skills/x/SKILL.md")
    with pytest.raises(SystemExit, match=re.escape(".claude/skills/x/SKILL.md")):
        check_diff(sprint_repo, "sprint/S01/12-core")


# ------------------------------------------------------------------------------------------
# Event ledger, burn-down and loop metrics (issue #320)

import datetime as dt  # noqa: E402
import json  # noqa: E402


def test_every_checkpoint_appends_one_ledger_event(tmp_path: Path, fixed_now: None) -> None:
    f = sprint_file(tmp_path)
    sp.write_state(f, 12, branch="feat/12-irr-sign", step="spec")
    sp.write_state(f, 12, step="tests", blocker=["needs the spec approved"])
    sp.write_state(f, 40, step="docs")
    events = sp.read_ledger(f)
    assert sp.ledger_path(f) == tmp_path / "S01" / "ledger.jsonl"
    assert [(e["issue"], e["step"], e["blockers"], e["agent_steps"]) for e in events] == [
        (12, "spec", 0, 1),
        (12, "tests", 1, 2),
        (40, "docs", 0, 1),
    ]
    assert events[0]["lane"] == "core" and events[0]["at"] == "2026-11-14T22:30:00+00:00"


def test_blocked_since_is_set_once_and_cleared_with_the_blockers(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    f = sprint_file(tmp_path)
    clock = iter(dt.datetime(2026, 11, 14, 9, m, tzinfo=dt.UTC) for m in range(60))
    monkeypatch.setattr(sp, "_now", lambda: next(clock))
    assert sp.write_state(f, 12, step="spec")["blocked_since"] is None
    blocked = sp.write_state(f, 12, blocker=["a"])
    assert blocked["blocked_since"] == blocked["updated"]
    assert sp.write_state(f, 12, blocker=["b"])["blocked_since"] == blocked["updated"]
    assert sp.write_state(f, 12, clear_blockers=True)["blocked_since"] is None


def test_the_ledger_skips_a_damaged_line(tmp_path: Path, fixed_now: None) -> None:
    f = sprint_file(tmp_path)
    sp.write_state(f, 12, step="spec")
    with sp.ledger_path(f).open("a", encoding="utf-8") as ledger:
        ledger.write('{"issue": 12, "at": "2026-11-14T23:\n')
    sp.write_state(f, 13, step="spec")
    assert [e["issue"] for e in sp.read_ledger(f)] == [12, 13]


def test_appends_from_many_writers_keep_whole_lines(tmp_path: Path) -> None:
    f = sprint_file(tmp_path)
    for i in range(200):
        sp.append_ledger(
            f, {"at": f"2026-11-14T10:00:{i % 60:02d}+00:00", "issue": i, "x": "y" * 300}
        )
    lines = sp.ledger_path(f).read_text(encoding="utf-8").splitlines()
    assert len(lines) == 200 and all(json.loads(line)["x"] == "y" * 300 for line in lines)


def at(day: int, hour: int = 9) -> str:
    return dt.datetime(2026, 11, day, hour, tzinfo=dt.UTC).isoformat()


SPRINT = {
    "id": "S01",
    "start": dt.date(2026, 11, 10),
    "end": dt.date(2026, 11, 20),
    "lanes": [{"name": "core", "issues": [12, 13]}, {"name": "docs", "issues": [40, 41]}],
}
EVENTS = [
    {"at": at(10), "issue": 12, "blockers": 0, "agent_steps": 1},
    {"at": at(11), "issue": 12, "blockers": 0, "agent_steps": 4},
    {"at": at(11), "issue": 13, "blockers": 1, "agent_steps": 2},
    {"at": at(12), "issue": 99, "blockers": 0, "agent_steps": 9},  # not in the sprint
]
FACTS = {
    12: {
        "closed_at": at(12, 21),
        "pull_requests": [{"number": 1, "created_at": at(11), "closed_at": at(12, 21)}],
        "failed_checks": 2,
    },
    # A pull request of #13 closed while the issue stays open (as #244 for #243): the issue
    # leaves review when the pull request closes.
    13: {
        "closed_at": None,
        "pull_requests": [{"number": 3, "created_at": at(10), "closed_at": at(11, 18)}],
    },
    40: {
        "closed_at": None,
        "pull_requests": [{"number": 2, "created_at": at(11), "closed_at": None}],
        "failed_checks": 1,
    },
    41: {"closed_at": at(13)},
}


def test_burndown_puts_each_issue_in_one_state_per_day() -> None:
    report = sp.burndown(SPRINT, EVENTS, FACTS, dt.date(2026, 11, 13))
    assert report["days"] == [
        {"day": "2026-11-10", "open": 2, "in_progress": 1, "in_review": 1, "done": 0},
        {"day": "2026-11-11", "open": 1, "in_progress": 1, "in_review": 2, "done": 0},
        {"day": "2026-11-12", "open": 1, "in_progress": 1, "in_review": 1, "done": 1},
        {"day": "2026-11-13", "open": 0, "in_progress": 1, "in_review": 1, "done": 2},
    ]


def test_cycle_time_is_measured_only_from_a_checkpoint() -> None:
    report = sp.burndown(SPRINT, EVENTS, FACTS, dt.date(2026, 11, 13))
    assert report["cycle_time_hours"] == {"12": 60.0}
    assert report["cycle_time_median_hours"] == 60.0
    assert report["closed_without_checkpoint"] == [41]


def test_failed_gates_steps_and_blocked_issues() -> None:
    report = sp.burndown(SPRINT, EVENTS, FACTS, dt.date(2026, 11, 13))
    assert report["failed_gates_per_lane"] == {"core": 2, "docs": 1}
    assert report["agent_steps"] == {"12": 4, "13": 2}
    assert report["blocked"] == {"13": at(11)}
    text = sp.render_burndown(report)
    assert "median 60.0 h over 1 issues" in text and "#41" in text and "not tokens" in text


def test_a_pull_request_belongs_to_an_issue_only_from_its_branch(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    pulls = {
        244: {"number": 244, "head": {"ref": "chore/243-roadmap-wording"}},  # its branch
        356: {"number": 356, "head": {"ref": "chore/6-sprint-s01"}},  # only mentions it
        357: {"number": 357, "head": {"ref": "sprint/S01/243-fix"}},
    }

    def fake_gh(path: str) -> object:
        if path.endswith("/timeline?per_page=100"):
            return [
                {
                    "event": "cross-referenced",
                    "source": {"issue": {"number": n, "pull_request": {"url": "u"}}},
                }
                for n in pulls
            ] + [{"event": "labeled"}]
        return pulls[int(path.rsplit("/", 1)[1])]

    monkeypatch.setattr(sp, "_gh", fake_gh)
    assert [p["number"] for p in sp.pull_requests_of("o/r", 243, None)] == [244, 357]
    assert sp.issue_branch(24).match("feat/24-x") and not sp.issue_branch(24).match("feat/243-x")


def test_github_facts_count_shared_commits_once_and_skip_not_planned(
    monkeypatch: pytest.MonkeyPatch,
) -> None:
    def fake_gh(path: str) -> object:
        if path == "repos/o/r/issues/12":
            return {"title": "a", "closed_at": at(12), "state_reason": "completed"}
        if path == "repos/o/r/issues/40":
            return {"title": "b", "closed_at": at(12), "state_reason": "not_planned"}
        if path.endswith("/timeline?per_page=100"):
            n = 12 if "/12/" in path else 40
            return [
                {
                    "event": "cross-referenced",
                    "source": {"issue": {"number": n * 10 + k, "pull_request": {"url": "u"}}},
                }
                for k in (1, 2)
            ]
        if "/commits?" in path:
            return [{"sha": "same"}, {"sha": path.split("/")[-2]}]
        if "/check-runs" in path:
            return {"check_runs": [{"conclusion": "failure"}, {"conclusion": "success"}]}
        number = int(path.rsplit("/", 1)[1])
        return {
            "number": number,
            "head": {"ref": f"feat/{number // 10}-x"},
            "created_at": at(11),
            "closed_at": at(12),
        }

    monkeypatch.setattr(sp, "_gh", fake_gh)
    sprint = {"lanes": [{"name": "core", "issues": [12, 40]}]}
    facts = sp.github_facts("o/r", sprint, [])
    assert facts[12]["failed_checks"] == 3  # commits "same", "121", "122": one failure each
    assert [p["number"] for p in facts[12]["pull_requests"]] == [121, 122]
    assert facts[40]["closed_at"] is None  # closed as not planned is not done
