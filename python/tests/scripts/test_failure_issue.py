"""scripts/failure_issue.py: one issue per failing workflow, updated and closed (#399).

The GitHub CLI is replaced by a fake that keeps the issues in memory: a forced failure on `main`
opens one issue, a second updates it, a green run closes it.
"""

from __future__ import annotations

import json
import re
from pathlib import Path
from typing import Any

import pytest
import yaml
from conftest import ROOT, load_script

failure_issue = load_script("failure_issue")
WORKFLOWS = ROOT / ".github" / "workflows"

RUN = "https://github.com/o/r/actions/runs/"


class FakeGh:
    """The subset of the GitHub CLI that the script calls, on an in-memory issue list."""

    def __init__(self, failed_jobs: list[str] | None = None) -> None:
        self.issues: dict[int, dict[str, Any]] = {}
        self.comments: dict[int, list[str]] = {}
        self.failed_jobs = failed_jobs or []
        self.calls: list[list[str]] = []

    @staticmethod
    def flag(args: list[str], name: str) -> str:
        return args[args.index(name) + 1]

    @staticmethod
    def body(args: list[str], stdin: str | None) -> str:
        assert FakeGh.flag(args, "--body-file") == "-"
        assert stdin is not None
        return stdin

    def __call__(self, args: list[str], stdin: str | None = None) -> str:
        self.calls.append(args)
        match args[:2]:
            case ["issue", "list"]:
                label = self.flag(args, "--label")
                return json.dumps(
                    [
                        {"number": n, "title": i["title"], "body": i["body"]}
                        for n, i in self.issues.items()
                        if i["state"] == "open"
                        and label in i["labels"]
                        and i.get("author", "app/github-actions") == self.flag(args, "--author")
                    ]
                )
            case ["issue", "create"]:
                number = len(self.issues) + 1
                self.issues[number] = {
                    "title": self.flag(args, "--title"),
                    "body": self.body(args, stdin),
                    "labels": [self.flag(args, "--label")],
                    "state": "open",
                }
                return f"https://github.com/o/r/issues/{number}"
            case ["issue", "edit"]:
                self.issues[int(args[2])]["body"] = self.body(args, stdin)
                return ""
            case ["issue", "comment"]:
                self.comments.setdefault(int(args[2]), []).append(self.body(args, stdin))
                return ""
            case ["issue", "close"]:
                self.issues[int(args[2])]["state"] = "closed"
                self.comments.setdefault(int(args[2]), []).append(self.flag(args, "--comment"))
                return ""
            case ["api", path] if path.endswith("/jobs?per_page=100"):
                return "\n".join(self.failed_jobs)
        pytest.fail(f"unexpected gh call: {args}")


def report(gh: FakeGh, conclusion: str, run_id: int, **kw: Any) -> str:
    options: dict[str, Any] = {
        "repo": "o/r",
        "workflow": "CI",
        "run_id": run_id,
        "run_url": f"{RUN}{run_id}",
        "conclusion": conclusion,
        "event": "push",
        "branch": "main",
        "label": "bug",
        "title": None,
        "jobs": None,
        "details": "",
    }
    options.update(kw)
    return str(failure_issue.report(gh, **options))


def open_issues(gh: FakeGh) -> list[dict[str, Any]]:
    return [i for i in gh.issues.values() if i["state"] == "open"]


def test_a_failure_opens_one_issue_with_the_jobs_and_the_log() -> None:
    gh = FakeGh(failed_jobs=["House style, translations and Rust"])
    assert report(gh, "failure", 11) == "opened #1"
    (issue,) = open_issues(gh)
    assert issue["labels"] == ["bug"]
    assert issue["title"] == "CI passes again on main"
    assert "House style, translations and Rust" in issue["body"]
    assert f"{RUN}11" in issue["body"]


def test_a_second_failure_updates_the_same_issue() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11)
    gh.failed_jobs = ["Test"]
    assert report(gh, "timed_out", 12) == "updated #1"
    (issue,) = open_issues(gh)
    assert "Test" in issue["body"] and f"{RUN}12" in issue["body"]
    assert f"{RUN}12" in gh.comments[1][0]


def test_a_success_closes_the_issue_and_a_later_failure_opens_a_new_one() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11)
    assert report(gh, "success", 13) == "closed #1"
    assert open_issues(gh) == []
    assert f"{RUN}13" in gh.comments[1][-1]
    assert report(gh, "failure", 14) == "opened #2"


def test_a_success_without_an_open_issue_changes_nothing() -> None:
    gh = FakeGh()
    assert report(gh, "success", 13) == "nothing to close"
    assert all(call[:2] == ["issue", "list"] for call in gh.calls)


def test_each_workflow_has_its_own_issue() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11)
    assert report(gh, "failure", 12, workflow="Security", event="schedule") == "opened #2"
    assert report(gh, "success", 13) == "closed #1"
    assert [i["title"] for i in open_issues(gh)] == ["Security passes again on main"]


@pytest.mark.parametrize(
    ("conclusion", "event", "branch"),
    [
        ("cancelled", "push", "main"),
        ("skipped", "schedule", "main"),
        ("failure", "pull_request", "main"),
        ("failure", "push", "feature/1-x"),
    ],
)
def test_runs_that_are_not_failures_on_main_are_ignored(
    conclusion: str, event: str, branch: str
) -> None:
    gh = FakeGh(failed_jobs=["Build"])
    assert report(gh, conclusion, 11, event=event, branch=branch).startswith("ignored")
    assert gh.calls == []


def test_explicit_jobs_and_details_and_a_legacy_issue_found_by_title() -> None:
    gh = FakeGh()
    gh.issues[1] = {
        "title": "Weekly maintenance report",
        "body": "older report",
        "labels": ["maintenance"],
        "state": "open",
    }
    result = report(
        gh,
        "failure",
        21,
        workflow="Weekly maintenance",
        event="schedule",
        label="maintenance",
        title="Weekly maintenance report",
        jobs=["mutants=failure"],
        details="Surviving mutants:\n- lib.rs:3: replace f with true",
    )
    assert result == "updated #1"
    body = gh.issues[1]["body"]
    assert "mutants=failure" in body and "replace f with true" in body
    assert not any(call[0] == "api" for call in gh.calls)


def test_long_details_are_cut_to_the_size_of_an_issue_body() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11, details="x" * 100_000)
    assert len(open_issues(gh)[0]["body"]) <= failure_issue.MAX_BODY


def test_main_reads_the_details_file(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    details = tmp_path / "missed.md"
    details.write_text("Surviving mutants: 2", encoding="utf-8")
    gh = FakeGh(failed_jobs=["Build"])
    monkeypatch.setattr(failure_issue, "gh_cli", gh)
    argv = [
        "report",
        "--repo=o/r",
        "--workflow=CI",
        "--run-id=5",
        f"--run-url={RUN}5",
        "--conclusion=failure",
        "--event=push",
        "--branch=main",
        f"--details={details}",
    ]
    assert failure_issue.main(argv) == 0
    assert "Surviving mutants: 2" in open_issues(gh)[0]["body"]


def test_issues_not_opened_by_the_workflow_are_never_touched() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    marker = failure_issue.marker("CI")
    gh.issues[1] = {
        "title": "CI passes again on main",
        "body": marker,
        "labels": ["bug"],
        "state": "open",
        "author": "someone",
    }
    assert report(gh, "failure", 11) == "opened #2"
    assert report(gh, "success", 12) == "closed #2"
    assert gh.issues[1]["state"] == "open" and gh.issues[1]["body"] == marker


def test_the_body_says_how_to_reference_the_issue() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11)
    assert "Refs #1" not in open_issues(gh)[0]["body"]  # the number is not known yet
    assert "`Refs #<this issue>`" in open_issues(gh)[0]["body"]


def test_the_same_run_reported_twice_changes_nothing() -> None:
    gh = FakeGh(failed_jobs=["Build"])
    report(gh, "failure", 11)
    assert report(gh, "failure", 11) == "already reported in #1"
    assert 1 not in gh.comments


def test_an_empty_job_list_is_explicit(monkeypatch: pytest.MonkeyPatch) -> None:
    gh = FakeGh(failed_jobs=["Build"])
    monkeypatch.setattr(failure_issue, "gh_cli", gh)
    argv = ["report", "--repo=o/r", "--workflow=CI", "--run-id=5", f"--run-url={RUN}5"]
    argv += ["--conclusion=success", "--event=push", "--branch=main", "--jobs="]
    assert failure_issue.main(argv) == 0
    assert not any(call[0] == "api" for call in gh.calls)


def test_the_open_maintenance_issue_is_found_by_its_title_and_closed_when_green() -> None:
    """The issue opened before the marker existed (#256) is the one the next runs update."""
    gh = FakeGh()
    gh.issues[1] = {
        "title": "Weekly maintenance report",
        "body": "Weekly maintenance found problems: links=failure.",
        "labels": ["maintenance"],
        "state": "open",
    }
    options = {
        "workflow": "Weekly maintenance",
        "event": "schedule",
        "label": "maintenance",
        "title": "Weekly maintenance report",
        "jobs": [],
    }
    assert report(gh, "success", 31, **options) == "closed #1"
    assert open_issues(gh) == []


def load_workflow(path: Path) -> dict[Any, Any]:
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    assert isinstance(data, dict), path
    return data


def triggers(workflow: dict[Any, Any]) -> dict[str, Any]:
    # PyYAML reads the key `on` as the boolean True (YAML 1.1).
    on = workflow[True]
    return on if isinstance(on, dict) else {name: None for name in on}


def runs_on_main_or_schedule(workflow: dict[Any, Any]) -> bool:
    on = triggers(workflow)
    push = on.get("push") or {}
    return "schedule" in on or "main" in (push.get("branches") or [])


def watched() -> list[str]:
    trigger = load_workflow(WORKFLOWS / "failure-issue.yml")
    names = triggers(trigger)["workflow_run"]["workflows"]
    assert isinstance(names, list)
    return [str(n) for n in names]


def all_workflows() -> dict[str, dict[Any, Any]]:
    return {p.name: load_workflow(p) for p in sorted(WORKFLOWS.glob("*.yml"))}


def reports_itself(path_name: str) -> bool:
    return "scripts/failure_issue.py" in (WORKFLOWS / path_name).read_text(encoding="utf-8")


def test_every_watched_workflow_exists_under_its_exact_name() -> None:
    names = {w["name"] for w in all_workflows().values()}
    assert watched() and set(watched()) <= names, set(watched()) - names


def test_every_watched_workflow_runs_on_main_or_on_a_schedule() -> None:
    by_name = {w["name"]: w for w in all_workflows().values()}
    assert [n for n in watched() if not runs_on_main_or_schedule(by_name[n])] == []


def test_every_workflow_on_main_or_on_a_schedule_reports_its_failures_once() -> None:
    """Watched by failure-issue.yml, or reporting through the script itself: never both."""
    names = set(watched())
    missing, twice = [], []
    for file, workflow in all_workflows().items():
        if file == "failure-issue.yml" or not runs_on_main_or_schedule(workflow):
            continue
        is_watched, itself = workflow["name"] in names, reports_itself(file)
        if not (is_watched or itself):
            missing.append(workflow["name"])
        if is_watched and itself:
            twice.append(workflow["name"])
    assert missing == [] and twice == [], (missing, twice)


def test_workflows_that_report_themselves_use_distinct_issues() -> None:
    """Each keeps its own title, so a green run of one never closes the issue of another."""
    titles: dict[str, str] = {}
    for file in sorted(f for f in all_workflows() if reports_itself(f)):
        if file == "failure-issue.yml":
            continue
        text = (WORKFLOWS / file).read_text(encoding="utf-8")
        found = re.findall(r'--title "([^"]+)"', text)
        assert len(found) == 1, (file, found)
        titles[file] = found[0]
    assert titles["maintenance.yml"] == "Weekly maintenance report"  # the open issue #256
    assert len(set(titles.values())) == len(titles), titles


def test_the_failure_workflow_runs_only_for_this_repository_with_minimal_permissions() -> None:
    workflow = load_workflow(WORKFLOWS / "failure-issue.yml")
    assert workflow["permissions"] == {}
    assert triggers(workflow)["workflow_run"]["branches"] == ["main"]
    (job,) = workflow["jobs"].values()
    condition = " ".join(str(job["if"]).split())
    assert "github.event.workflow_run.head_repository.full_name == github.repository" in condition
    assert "github.event.workflow_run.event == 'push'" in condition
    assert "github.event.workflow_run.event == 'schedule'" in condition
    assert job["permissions"] == {"actions": "read", "contents": "read", "issues": "write"}
