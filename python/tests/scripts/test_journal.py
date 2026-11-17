"""scripts/journal.py: the journal indexed by issue, area, date and text (issue #327)."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
from conftest import load_script, write

journal = load_script("journal")

ROADMAP = """labels: []
milestones:
  - title: "M0"
    epics:
      - title: "Governance"
        issue: 6
        labels: [epic, area:infra, area:agents]
        tasks:
          - { issue: 333, title: "Shared state", status: done }
      - title: "MCP"
        issue: 52
        labels: [epic, area:mcp]
        tasks:
          - { issue: 331, title: "Inspector", status: done }
"""
STATE = """# 2026-11-16 — fix/333-shared-sprint-state

- Issue: #333 Sprint runtime state shared by every worktree
- Pull request: #338 (ready)
- Spec: none

## Done
- The state directory is shared by every worktree; the ledger of #320 follows it.
- Second item.

## Next steps
1. #334: lanes declare their paths.
"""
INSPECTOR = """# 2026-11-16 — ci/331-mcp-inspector

- Issue: #331 MCP Inspector in CI
- Pull request: none

## Done
- The protocol check drives the server over stdio, see sprint state of #333.
"""
NO_ISSUE = """# 2026-11-07 — session-roadmap-analysis

- Issue: none

Notes on the merge order of generated files.
"""


@pytest.fixture
def entries(tmp_path: Path) -> tuple[Path, list[object]]:
    write(
        tmp_path,
        {
            "journal/2026-11-16-fix-333-shared-sprint-state.md": STATE,
            "journal/2026-11-16-ci-331-mcp-inspector.md": INSPECTOR,
            "journal/2026-11-07-session-roadmap-analysis.md": NO_ISSUE,
            "journal/README.md": "# Session journal\n",
            "journal/_template.md": "# <YYYY-MM-DD> — <branch>\n",
            "roadmap.yaml": ROADMAP,
        },
    )
    return tmp_path / "journal", journal.build(tmp_path / "journal", tmp_path / "roadmap.yaml")


def test_index_reads_every_entry_and_nothing_else(entries: tuple[Path, list]) -> None:
    _, built = entries
    assert [e.file for e in built] == [
        "2026-11-07-session-roadmap-analysis.md",
        "2026-11-16-ci-331-mcp-inspector.md",
        "2026-11-16-fix-333-shared-sprint-state.md",
    ]
    state = built[2]
    assert (state.date, state.branch, state.issue) == (
        "2026-11-16",
        "fix/333-shared-sprint-state",
        333,
    )
    assert state.mentions == [320, 334, 338]
    assert state.areas == ["agents", "infra"]
    assert state.summary.startswith("The state directory is shared by every worktree")
    assert built[0].issue is None and built[0].summary == ""


def test_search_by_issue_puts_its_own_entry_before_mentions(entries: tuple[Path, list]) -> None:
    folder, built = entries
    found = journal.search(built, "#333", folder)
    assert [(e.issue, why) for e, why in found] == [(333, "issue"), (331, "mentions")]
    assert journal.search(built, "333", folder) == found


def test_search_by_area_and_by_date(entries: tuple[Path, list]) -> None:
    folder, built = entries
    assert [e.issue for e, _ in journal.search(built, "area:mcp", folder)] == [331]
    # The entry of #331 cites #333 but is not about its areas.
    assert built[1].areas == ["mcp"]
    assert [e.issue for e, _ in journal.search(built, "agents", folder)] == [333]
    assert [e.file for e, _ in journal.search(built, "2026-11-07", folder)] == [
        "2026-11-07-session-roadmap-analysis.md"
    ]
    assert len(journal.search(built, "2026-11", folder)) == 3


def test_text_search_returns_the_matching_line(entries: tuple[Path, list]) -> None:
    folder, built = entries
    [(entry, line)] = journal.search(built, "MERGE ORDER", folder)
    assert entry.issue is None and line == "Notes on the merge order of generated files."


def test_long_summaries_are_cut_on_one_line() -> None:
    text = "## Done\n- " + "word " * 40 + "\n  continued\n"
    out = journal.summary(text)
    assert len(out) == journal.SUMMARY_LENGTH and out.endswith("...") and "\n" not in out


def test_cli_json_and_an_unknown_area(
    entries: tuple[Path, list],
    monkeypatch: pytest.MonkeyPatch,
    capsys: pytest.CaptureFixture[str],
) -> None:
    folder, _ = entries
    monkeypatch.setattr(journal, "JOURNAL", folder)
    monkeypatch.setattr(journal, "ROADMAP", folder.parent / "roadmap.yaml")
    assert journal.main(["search", "333", "--json"]) == 0
    payload = json.loads(capsys.readouterr().out)
    assert [(p["issue"], p["match"]) for p in payload] == [(333, "issue"), (331, "mentions")]
    assert journal.main(["search", "area:nope"]) == 1
    assert "Areas in the journal: agents, infra, mcp." in capsys.readouterr().out


def test_the_real_journal_is_indexed() -> None:
    built = journal.build()
    assert built and all(e.date and e.branch for e in built)
    assert any(e.issue == 333 for e in built)
