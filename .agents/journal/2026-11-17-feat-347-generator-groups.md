# 2026-11-17 — feat-347-generator-groups

- Issue: #347 Generated artifacts grouped by generator, one lane per group
- Pull request: opens from this branch after the gate
- Spec: none (tooling)

## Done
- `generates` in a sprint lane is now a list of generator groups: `registry` (schema
  snapshots, methods reference and indexes, README blocks, tool examples) and `agents`
  (`.claude/`). One lane per group; a lane may not cover the artifacts of a group it does
  not generate; `check-diff` allows the artifacts of the lane's own groups only. `true` is
  refused, so every sprint file names the group.
- Found while planning S01: the product lane changes methods and the process lane changes
  skills; with one generating lane for everything they could not run in parallel.
- `sprints/README.md` (table of the groups, merge order per group), the template, the
  sprint-planner role and the plan-sprint skill; `.claude/` regenerated. Six new tests.

## State at the end of the session
- Nothing half done.

## Decisions taken
- none beyond the issue

## Open questions for the maintainer
- none

## Next steps
1. #345: tests for every process script, moved to `python/tests/scripts/`; then sprint S01.
