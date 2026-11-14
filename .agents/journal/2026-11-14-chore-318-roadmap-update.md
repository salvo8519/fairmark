# 2026-11-14 — chore/318-roadmap-update

- Issue: #318 Persistent sprint state and per-task checkpoints (planning; stays open)
- Pull request: #321 (merged, squash bddbacf)
- Spec: none (planning and session protocol)

## Done
- Reviewed the agent and skill architecture with the maintainer and assessed it for
  long-running agent loops (six weaknesses identified: state in conversation, journal only
  at session end, collisions on generated artifacts, worktree lifecycle, no queue and
  flaky feedback, no loop metrics).
- Created the three planning issues with `scripts/roadmap.py new`, confirmed by the
  maintainer: #318 persistent sprint state and per-task checkpoints, #319 generated
  artifacts and worktree policy for long-running sprints, #320 sprint burn-down, loop
  metrics and pull request queue automation; all in M0 "Repository and governance",
  parallel, area:agents.
- Synced #282 to done in roadmap/roadmap.yaml, fixing the Roadmap sync divergence on
  main (commit on the branch, squash bddbacf after merge).
- Opened, completed and merged PR #321: conventions branch chore/318-roadmap-update,
  full template body, label no-release-note, all required checks green
  (Roadmap and GitHub agree after the merge).
- Coordinated with the parallel #210 session: its residue in the shared working tree was
  preserved on origin (becb448, merged as #322); discarded locally only after verifying
  the pushed version was newer; the main checkout was freed and the work moved to a
  dedicated worktree during the session, removed after the merge.

## State at the end of the session
- Nothing half done. main is clean and consistent: roadmap check passes, Roadmap and
  GitHub agree, Roadmap sync workflow green again on main.
- #318, #319, #320 are open and triaged: planned work for the next sprints.
- Gate on main: `uv run just check` all green at session close.

## Decisions taken
- Three thematic issues rather than six granular ones, covering all six weaknesses
  (maintainer, confirmed in session).
- Tooling milestone M0 "Repository and governance" because all agent-loop tooling lives
  there (#93, #123, #173); no spec needed: no method code.
- No release note: planning only (no-release-note label on PR #321).

## Open questions for the maintainer
- none

## Next steps
1. Plan the sprint for #318, #319 and #320 (three parallel lanes: state and checkpoints,
   artifacts and worktrees, burn-down and queue automation) with `/plan-sprint`.
