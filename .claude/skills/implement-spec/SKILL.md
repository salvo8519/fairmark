---
name: implement-spec
description: Execute an approved spec's tasks.md test-first, one task per commit, with review subagents. Use when the user asks to implement or continue a spec.
---

<!-- Generated from .agents/skills/implement-spec/SKILL.md by scripts/sync_agents.py. Edit the source, not this file. -->

Preconditions: the spec is `Approved` and has `plan.md` and `tasks.md`. If not, stop and
use `/spec-new`.

Loop over unticked tasks in order (respect `[P]` only when working in parallel worktrees):
1. Read the task, the requirements and acceptance criteria it serves.
2. **Tests first**: if the task has no test yet, delegate to `test-engineer`; run the tests
   and confirm they fail for the expected reason.
3. Implement the minimum to make them pass, following AGENTS.md hard rules.
4. `uv run just check` must pass.
5. Run `code-reviewer`; if numbers changed, also `numerical-auditor`. Fix blocking findings.
6. Tick the task in `tasks.md`, commit with a Conventional Commit message citing the spec:
   `feat(<scope>): <what> [SPEC-NNNN R<n>]`, signed off (`git commit -s`).

When all tasks are ticked, tick the acceptance criteria in `spec.md`, set status
`Implementing` → leave `Done` to the maintainer, and suggest `/pre-pr`.
