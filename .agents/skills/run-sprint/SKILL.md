---
name: run-sprint
description: Orchestrate an approved sprint — delegate every issue to the specialised agents lane by lane, ending each issue as a pull request. Use when the maintainer asks to run or continue a sprint.
---

The main session is the orchestrator: subagents cannot start other subagents, so you
delegate and keep the overview.

## Preconditions
- `sprints/SNN.yaml` exists with `status: active` and passes
  `uv run python scripts/sprint.py validate`.
- `main` is green.

## Lanes (subagents, parallel)
For each lane, in parallel where the agents tool supports isolated worktrees, otherwise one
lane after the other. For each issue of the lane, in order:
1. Read the issue (`gh issue view N`) and its spec. Create branch `sprint/SNN/N-<slug>`.
   Change only the files of the lane's `paths`; before pushing, run
   `uv run python scripts/sprint.py check-diff --branch sprint/SNN/N-<slug> --base origin/main`
   (CI runs the same check on the pull request).
2. No approved spec for method work: delegate to `spec-writer`, then stop this lane and ask
   the maintainer to approve.
3. `test-engineer` writes failing tests for the issue's acceptance criteria.
4. `core-engineer` (Rust core) or `docs-writer` (documentation-only issues) implements.
5. Review by tier ("Effort proportional to risk" in AGENTS.md): none for small pull requests,
   one `code-reviewer` pass for substantial ones, plus `numerical-auditor` for numerical
   changes and `security-reviewer` when dependencies or workflows changed. Fix every finding;
   a second pass only for a blocking logic defect, then escalate to the maintainer. Run at
   most three lanes at a time.
6. `uv run just check` green, docs in sync (the Stop hook enforces it).
7. Push and open the PR: `gh pr create --title "<type>(<scope>): ... [SPEC-NNNN]" --body
   "Closes #N ..."`. Never merge.
Checkpoint state at every step: after each numbered step of this loop, run
`uv run python scripts/sprint.py state sprints/SNN.yaml N --step <step> [--pr <n> |
--blocker "<why>" | --clear-blockers]` (the running table persisted, sprints/README.md,
Sprint state). At every task switch append one line to
`.agents/journal/<date>-sprint-SNN-wip.md`: issue, step, pull request, blockers, last commit
of the lane. Both are local recovery state, never committed: a crash costs minutes, not a
session of context. The state files live in the git directory shared by every worktree, so a
checkpoint written from a lane worktree is the same file the orchestrator reads.
Keep a running table: issue, branch, PR, status, blockers. Each lane works in its own
worktree (one worktree per session, AGENTS.md):

- a lane other than the generate lane never runs `uv run just generate`;
- when a lane needs generated artifacts, it becomes the generate lane of the sprint, the
  only one (sprints/README.md).

## End of run
Propose the merge order to the maintainer (sprints/README.md, Generated artifacts and the merge
order): pull requests carrying regenerated artifacts merge strictly one at a time; after each
of them merges, rebase the next on `main` and run `uv run just generate` in it again. Report
per lane: PRs opened, issues blocked and why, what the maintainer must review or decide.
Do not change `status` to `done`: that is the maintainer's call. Propose the removal of the
worktree of every merged pull request: the Retirement candidates of
`uv run just workspace-report`, never deleted without the maintainer's approval.
