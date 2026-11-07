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


def test_the_page_is_generated_from_the_file() -> None:
    d = data()
    d["milestones"][0]["gate"] = "A different gate."
    assert any("roadmap.md is not generated" in p for p in roadmap.problems_of(d))
