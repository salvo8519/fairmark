# 2026-11-14 — docs-329-agent-loop-planning

- Issue: #329 Agent loop planning update of 2026-11-14: ADR 0015
- Pull request: none (opens from this branch after the gate)
- Spec: none (architecture record and planning)

## Done
- ADR 0015 "Agent memory: one canonical source, tiered memory, generated mirrors", status
  `proposed`: the four questions of the session answered — a lone session works in the main
  checkout (the worktree is the tool of parallel work), no database (tiered memory: runtime
  state and JSONL ledger local and git-ignored, journal and plan committed, resumption
  through the pushed branch), runtime state never committed, context retrieved through a
  generated index with a small reader. Navigation entry in `zensical.toml`.
- Task #327 "Journal index and search of the agent session memory" and task #328 "Generated
  mirrors of .agents/ for other agent systems" created with `scripts/roadmap.py new` in the
  Repository and governance epic (M0), both `parallel`, and annotated with the ADR.
- Task #329 (this update) created and marked done in the same change.
- Issue #320 extended with the ledger design: `sprints/SNN/ledger.jsonl`, one JSONL line per
  checkpoint of `sprint.py state` (issue, step, blockers, agent steps, UTC timestamp), local
  and git-ignored like the state directory; the burn-down and loop metrics of #320 are
  derived from it, with a comment on the issue linking the ADR.

## State at the end of the session
- Branch `docs/329-agent-loop-planning` from an up-to-date `main`; all work in the main
  checkout, alone in it (the rule of #319: a lone session needs no worktree).
- Checks run: `roadmap.py check --pr-body` consistent, `check_yaml.py`, `check_style.py`;
  the full gate runs before the push.

## Decisions taken
- ADR 0015 keeps status `proposed` and asks the maintainer to accept it; #327, #328 and the
  ledger part of #320 wait for that acceptance (decided by the orchestrator, adr-new skill).
- The ADR was written by the orchestrator with the full session context, not by the
  architecture agent: the four answers were already drafted in the conversation, and the
  maintainer reviews the proposed ADR anyway (decided by the orchestrator, agent budget).
- The event ledger is JSONL appended beside the state files by the command that already
  writes the state: no second write path (decided by the orchestrator).

## Open questions for the maintainer
- Accept ADR 0015 (status `proposed` → `accepted`), which unblocks #327, #328 and the ledger
  part of #320.

## Next steps
1. Gate, commit, push and open the pull request (Closes #329, label no-release-note), wait
   for CI and merge per the standing instruction of the session.
2. On acceptance: `status: accepted` in ADR 0015, then the sprint of the agent loop
   (ADR mirror tasks, #320 with the ledger, #327, #328).
