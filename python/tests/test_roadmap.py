"""Rules of the roadmap file (scripts/roadmap.py), offline."""

from __future__ import annotations

import copy
import importlib.util
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("roadmap", ROOT / "scripts/roadmap.py")
assert _spec and _spec.loader
roadmap = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(roadmap)


def data() -> dict[str, Any]:
    return copy.deepcopy(roadmap.load())


def test_committed_roadmap_is_consistent() -> None:
    assert roadmap.problems_of(data()) == []


def test_every_task_needs_a_unique_issue() -> None:
    d = data()
    first = d["milestones"][0]["epics"][0]["tasks"]
    first[1]["issue"] = first[0]["issue"]
    assert any("appears twice" in p for p in roadmap.problems_of(d))
    del first[0]["issue"]
    assert any("needs an issue" in p for p in roadmap.problems_of(d))


def test_specs_must_exist_and_be_referenced() -> None:
    d = data()
    task = d["milestones"][0]["epics"][0]["tasks"][0]
    task["spec"] = "9999"
    assert any("spec 9999 does not exist" in p for p in roadmap.problems_of(d))
    for m in d["milestones"]:
        for e in m["epics"]:
            for t in e["tasks"]:
                if t.get("spec") == "0008":
                    del t["spec"]
    assert any("spec 0008 is not referenced" in p for p in roadmap.problems_of(d))


def test_unknown_status_and_keys_are_refused() -> None:
    d = data()
    task = d["milestones"][0]["epics"][0]["tasks"][0]
    task["status"] = "almost"
    task["owner"] = "someone"
    problems = roadmap.problems_of(d)
    assert any("status must be" in p for p in problems)
    assert any("unknown keys" in p for p in problems)


def test_a_pull_request_must_close_a_planned_issue() -> None:
    d = data()
    task = d["milestones"][0]["epics"][0]["tasks"][0]
    planned = task["issue"]
    task["status"] = "open"
    assert any("mark its task done" in p for p in roadmap.problems_of(d, f"Closes #{planned}."))
    task["status"] = "done"
    problems = roadmap.problems_of(d, f"Summary.\n\nCloses #{planned}.")
    assert not [p for p in problems if "closes" in p]
    assert any("#99999" in p for p in roadmap.problems_of(d, "Closes #99999"))


def test_dropped_work_stays_visible_and_is_not_counted() -> None:
    d = data()
    task = d["milestones"][0]["epics"][0]["tasks"][0]
    task["status"] = "dropped"
    block = roadmap.page_block(d)
    assert "| Dropped |" in block
    assert not [p for p in roadmap.problems_of(d) if "status" in p]


def test_the_page_includes_the_generated_tables() -> None:
    """The tables are generated at build time; a change to the plan needs no page change."""
    d = data()
    d["milestones"][0]["gate"] = "A different gate."
    assert not [p for p in roadmap.problems_of(d) if "roadmap.md" in p]
    assert roadmap.INCLUDE in roadmap.PAGE.read_text(encoding="utf-8")


def test_a_page_without_the_include_is_refused(monkeypatch, tmp_path) -> None:  # type: ignore[no-untyped-def]
    page = tmp_path / "roadmap.md"
    page.write_text("# Roadmap\n\nTables pasted by hand.\n", encoding="utf-8")
    monkeypatch.setattr(roadmap, "PAGE", page)
    assert any("must include the generated tables" in p for p in roadmap.problems_of(data()))


def test_write_page_produces_the_tables(monkeypatch, tmp_path) -> None:  # type: ignore[no-untyped-def]
    target = tmp_path / ".generated" / "ROADMAP.md"
    monkeypatch.setattr(roadmap, "GENERATED_PAGE", target)
    d = data()
    roadmap.write_page(d)
    assert target.read_text(encoding="utf-8") == roadmap.page_block(d) + "\n"


# ---- sync, offline: GitHub issues are given as data


def github_view(d: dict[str, Any]) -> dict[int, dict[str, Any]]:
    """Issues as GitHub would report them if it agreed with the roadmap `d`."""
    issues: dict[int, dict[str, Any]] = {}
    for m, _e, t in roadmap.tasks(d):
        status = t.get("status", "open")
        issues[t["issue"]] = {
            "number": t["issue"],
            "title": t["title"],
            "state": "open" if status == "open" else "closed",
            "state_reason": "not_planned" if status == "dropped" else "completed",
            "milestone": {"title": m["title"]},
        }
    return issues


def test_sync_agrees_with_a_matching_github() -> None:
    d = data()
    drift, untriaged = roadmap.differences(copy.deepcopy(d), github_view(d))
    assert drift == [] and untriaged == []


def test_new_issue_is_pending_while_its_pull_request_is_open() -> None:
    main = data()
    branch = copy.deepcopy(main)
    epic = branch["milestones"][0]["epics"][0]
    epic["tasks"].append({"issue": 99999, "title": "New task", "status": "open"})
    issues = github_view(branch)
    drift, _ = roadmap.differences(copy.deepcopy(main), issues)
    assert [n for n, _ in drift] == [99999]
    assert roadmap.pending_in(drift, {7: (branch, set())}, issues) == {99999: 7}
    # Without the pull request the difference stands.
    assert roadmap.pending_in(drift, {}, issues) == {}


def test_a_pull_request_that_does_not_resolve_leaves_the_difference() -> None:
    main = data()
    issues = github_view(main)
    task = main["milestones"][0]["epics"][0]["tasks"][0]
    issues[task["issue"]]["state"] = "open" if task.get("status") != "open" else "closed"
    drift, _ = roadmap.differences(copy.deepcopy(main), issues)
    assert [n for n, _ in drift] == [task["issue"]]
    unrelated = copy.deepcopy(main)
    assert roadmap.pending_in(drift, {8: (unrelated, set())}, issues) == {}


def test_issue_closed_by_an_open_pull_request_is_pending() -> None:
    # The pull request adds the task as done (check --pr-body requires it); the issue is still
    # open on GitHub until the merge closes it.
    main = data()
    branch = copy.deepcopy(main)
    branch["milestones"][0]["epics"][0]["tasks"].append(
        {"issue": 99998, "title": "Closed by the pull request", "status": "done"}
    )
    issues = github_view(branch)
    issues[99998]["state"] = "open"
    drift, _ = roadmap.differences(copy.deepcopy(main), issues)
    assert [n for n, _ in drift] == [99998]
    assert roadmap.pending_in(drift, {9: (branch, {99998})}, issues) == {99998: 9}
    # A pull request that does not close the issue leaves the difference.
    assert roadmap.pending_in(drift, {9: (branch, set())}, issues) == {}


def test_closed_by_reads_the_closing_keywords() -> None:
    body = "Closes #12\nfixes #7\n  Resolves #3\nSee #99\nRelated: closes #5 later"
    assert roadmap.closed_by(body) == {12, 7, 3}
