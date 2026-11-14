# 2026-11-14 — feat-318-sprint-state

- Issue: #318 Persistent sprint state and per-task checkpoints
- Pull request: none (opens from this branch after the gate)
- Spec: none (tooling)

## Done
- `scripts/sprint.py` gained the per-issue state of the running table: `state <file> <issue>`
  writes `sprints/SNN/state/<issue>.yaml` — issue, lane, branch, pull request, current step
  (`spec`, `tests`, `implementation`, `review`, `pr`, `done`), blockers, `started`/`updated`
  UTC ISO 8601 and `agent_steps` (one per write: the agent cost of the issue). Every write
  preserves the fields no argument names; `#<issue> is not in the lanes` is refused.
- `sprint.py status` prints the state next to the GitHub state of its issue and marks the
  state file of a closed issue as stale. `main` now takes `argv` and uses subparsers
  (`validate`, `status`, `state`).
- `scripts/project_state.py` reads `sprints/SNN/state/*.yaml` of the active sprint: the
  session briefing lists the per-issue state under the Sprint section, so a new session
  resumes mid-issue from the recorded branch and step.
- `sprints/README.md` — new section "Sprint state: resume mid-issue": fields, the command,
  the resume protocol (state file + work-in-progress note, both local and git-ignored).
- `.agents/skills/run-sprint/SKILL.md` — checkpoint write after every numbered step of the
  lane loop and a work-in-progress line in `.agents/journal/<date>-sprint-SNN-wip.md` at
  every task switch.
- `.agents/skills/session-start/SKILL.md` — the briefing shows per-issue state; resume in
  the worktree the state names; a stale state file goes with the worktree.
- `.gitignore` — `/sprints/*/state/` (runtime state of the orchestrator, never committed).
- Marked task 318 done in `roadmap/roadmap.yaml` in the same change.

## State at the end of the session
- Work in the isolated worktree `../fairmark-318`. 11 new tests in
  `python/tests/test_sprint.py`, written first (the suite failed on the missing functions):
  state path next to the sprint file, first write and its fields, agent_steps increment with
  `started` kept, blocker append and clear, issue outside the sprint refused, non-integer
  pull request refused, summary lines for open and closed issues, command-line write and
  validation ordering. ruff, typos check of the style gate, mypy green.
- Full gate (`uv run just check`) runs before the push.

## Decisions taken
- `agent_steps` counts the state writes — the honest, measurable agent cost; no invented
  accounting (decided by the orchestrator).
- State files are git-ignored runtime state of the orchestrator's checkout, like
  `.generated/`; the committed record stays the journal entry and the pull request
  (decided by the orchestrator, following the issue text "on file", not "in history").
- The step names follow the numbered steps of the run-sprint lane loop (decided by the
  orchestrator).

## Open questions for the maintainer
- Whether `sprint.py status` should also verify the worktree of each branch exists.

## Next steps
1. Full gate, commit, push, PR (Closes #318, label no-release-note), CI green, merge when
   MERGEABLE per the maintainer's standing instruction of this session.
