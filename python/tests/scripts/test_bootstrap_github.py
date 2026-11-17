"""scripts/bootstrap_github.py: labels, milestones, epics and tasks created once, from the plan."""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest
import yaml
from conftest import load_script

bootstrap = load_script("bootstrap_github")
PLAN = {
    "labels": [{"name": "epic", "color": "5319e7"}, {"name": "area:core", "color": "1d76db"}],
    "milestones": [
        {
            "title": "M0 · Foundations",
            "gate": "a package installs",
            "epics": [
                {
                    "title": "Repository",
                    "labels": ["area:core"],
                    "tasks": [
                        {"title": "Layout", "status": "done"},
                        {"title": "Existing task"},
                    ],
                }
            ],
        }
    ],
}


class FakeGitHub:
    """Records the writes; answers the reads with what already exists on GitHub."""

    def __init__(self, repo: str, dry_run: bool) -> None:
        self.writes: list[tuple[str, str, dict[str, Any]]] = []
        self.number = 100
        FakeGitHub.last = self

    def api(self, method: str, path: str, **fields: Any) -> Any:
        if method == "GET":
            return {
                "labels?per_page=100": [{"name": "epic"}],
                "milestones?state=all&per_page=100": [],
                "issues?state=all&per_page=100": [{"title": "Existing task", "number": 7}],
            }[path]
        self.writes.append((method, path, fields))
        self.number += 1
        return {"number": self.number, "id": self.number * 10}


def test_only_what_is_missing_is_created(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    plan = tmp_path / "roadmap.yaml"
    plan.write_text(yaml.safe_dump(PLAN, allow_unicode=True), encoding="utf-8")
    monkeypatch.setattr(bootstrap, "ROADMAP", plan)
    monkeypatch.setattr(bootstrap, "GitHub", FakeGitHub)
    monkeypatch.setattr(sys, "argv", ["bootstrap_github.py", "--repo", "o/r"])
    bootstrap.main()
    writes = [(m, p) for m, p, _ in FakeGitHub.last.writes]
    assert writes == [
        ("POST", "labels"),  # area:core only: epic exists
        ("POST", "milestones"),
        ("POST", "issues"),  # the epic
        ("POST", "issues"),  # the task "Layout"; "Existing task" is not created again
        ("POST", "issues/103/sub_issues"),  # the task under its epic
        ("PATCH", "issues/104"),  # done tasks are closed
    ]
    epic = FakeGitHub.last.writes[2][2]
    assert epic["title"] == "[Epic] Repository" and epic["labels"] == ["epic", "area:core"]


def test_dry_run_writes_nothing(capsys: pytest.CaptureFixture[str]) -> None:
    gh = bootstrap.GitHub("o/r", dry_run=True)
    assert gh.api("POST", "labels", name="x") == {"number": 0, "id": 0}
    assert capsys.readouterr().out.startswith("[dry-run] POST labels")
