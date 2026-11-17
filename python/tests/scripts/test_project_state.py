"""scripts/project_state.py: the session briefing renders, with the shared sprint state (#333)."""

from __future__ import annotations

import importlib.util
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "scripts" / "project_state.py"
_spec = importlib.util.spec_from_file_location("project_state", SCRIPT)
ps = importlib.util.module_from_spec(_spec)  # type: ignore[arg-type]
_spec.loader.exec_module(ps)  # type: ignore[union-attr]


def briefing(sprint: dict[str, Any] | None) -> dict[str, Any]:
    return {
        "repository": "owner/fairmark",
        "branch": "fix/333-shared-sprint-state",
        "branch_issue": 333,
        "ahead_of_main": 1,
        "behind_main": 0,
        "uncommitted_files": [],
        "recent_commits": ["abc1234 fix(scripts): shared sprint state"],
        "journal": [],
        "pull_requests": [],
        "active_sprint": sprint,
        "sprint_done": [12],
        "current_milestone": "M0 · Foundations",
        "in_progress": [],
        "blocked": [],
        "ready": [],
    }


def test_briefing_lists_the_per_issue_state_of_the_active_sprint() -> None:
    sprint = {
        "file": "S01.yaml",
        "goal": "Shared state",
        "end": "2026-11-22",
        "issues": [12, 13],
        "state": ["#13, branch fix/13-x, step tests, 0 blockers"],
    }
    text = ps.render(briefing(sprint))
    assert "S01.yaml: Shared state (ends 2026-11-22); 1/2 issues closed." in text
    assert "Per-issue state (sprints state files, local recovery):" in text
    assert "- #13, branch fix/13-x, step tests, 0 blockers" in text


def test_briefing_without_state_files_has_no_state_heading() -> None:
    sprint = {"file": "S01.yaml", "goal": "g", "end": "2026-11-22", "issues": [12], "state": []}
    assert "Per-issue state" not in ps.render(briefing(sprint))


def test_briefing_without_an_active_sprint_names_the_milestone() -> None:
    assert "No active sprint. Current milestone: M0 · Foundations." in ps.render(briefing(None))
