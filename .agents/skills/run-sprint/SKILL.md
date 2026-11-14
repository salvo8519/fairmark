---
name: run-sprint
description: Orchestrate an approved sprint — delegate every issue to the specialised agents lane by lane (locally) or dispatch issues to Claude in GitHub Actions (remotely), ending each issue as a pull request. Use when the maintainer asks to run or continue a sprint.
---

The main session is the orchestrator: subagents cannot start other subagents, so you
delegate and keep the overview. Ask the maintainer which mode to use if not stated.

## Preconditions
- `sprints/SNN.yaml` exists with `status: active` and passes
  `uv run python scripts/sprint.py validate`.
- `main` is green.

## Local mode (subagents, parallel lanes)
For each lane, in parallel where the agents tool supports isolated worktrees, otherwise one
lane after the other. For each issue of the lane, in order:
1. Read the issue (`gh issue view N`) and its spec. Create branch `sprint/SNN/N-<slug>`.
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
Keep a running table: issue, branch, PR, status, blockers.

## Remote mode (Claude in GitHub Actions)
For each issue: `gh issue comment N --body "@claude Implement this issue following
.agents/skills/implement-spec/SKILL.md and AGENTS.md. Work test-first, run uv run just check,
and open a pull request that closes this issue."` Issues of different lanes run in parallel
on separate runners. Then track the resulting PRs with `scripts/sprint.py status`.

## End of run
Report per lane: PRs opened, issues blocked and why, what the maintainer must review or
decide. Do not change `status` to `done`: that is the maintainer's call.
