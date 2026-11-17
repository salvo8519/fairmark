# 2026-11-17 — sprint/S01/327-journal-index

- Issue: #327 Journal index and search of the agent session memory
- Pull request: opens from this branch after the gate (sprint S01, process lane)
- Spec: none (tooling)

## Done
- `scripts/journal.py`: `index` and `search <issue|area:name|date|text>`, with `--json` for
  agents. Per entry: date, branch, the issue of the entry, the issues it mentions, the areas
  (from the epics of the roadmap) and a one-line summary from `## Done`. An issue search lists
  the entry's own entries first, then those that mention it. `uv run just journal <query>`.
- Design change from the issue, recorded as an amendment of ADR 0015: the index is built from
  the entries each time and never committed. A committed index would be rewritten by every
  pull request (every session adds an entry) and make parallel lanes collide, the problem
  #335 removes for the schema index.
- `session-start` searches the journal for the branch issue and for every issue it proposes,
  and cites the entries; the journal README, the agentic development page (both languages)
  and ADR 0015 describe it.
- Defect found by planning S01: `scripts/conventions.py` rejected `sprint/SNN/<issue>-<slug>`,
  the branches `run-sprint` creates, so every sprint pull request would have failed PR
  hygiene. Fixed with a failing test first; conventions page updated. The file joined the
  process lane in the sprint file first (#356), as the lane rules require.
- Tests: 7 for the journal (index, issue, area, date, text, summary, CLI) and 1 for sprint
  branches. Full suite 4231 passed.

## State at the end of the session
- Nothing half done.

## Decisions taken
- Areas of an entry come from its own issue; mentions count only when it has none.

## Open questions for the maintainer
- none

## Next steps
1. #320: burn-down and loop metrics, with the ledger beside the shared sprint state.
