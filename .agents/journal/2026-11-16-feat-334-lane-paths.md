# 2026-11-16 — feat-334-lane-paths

- Issue: #334 Sprint lanes declare their paths; overlaps fail validation and CI
- Pull request: opens from this branch after the gate
- Spec: none (tooling)

## Done
- Sprint lanes declare `paths` (glob patterns, required) and `generates` (one lane at most).
  `scripts/sprint.py validate` rejects a lane without paths, two lanes that some file could
  match both (with the lanes and an example path), two generating lanes, and a lane without
  `generates` that covers a generated artifact.
- The overlap check compares patterns segment by segment, `**` as any number of segments,
  and returns a witness path that a test proves both patterns match.
- `scripts/sprint.py check-diff --branch sprint/SNN/<issue>-<slug> --base <rev>` fails when a
  changed file is outside the lane of the issue; the roadmap, the journal and `changes/` are
  always allowed, the generated artifacts only for the generating lane. CI runs it on pull
  requests from sprint branches.
- The sprint validation in CI and in `just lint` no longer ends with `; true`: in the
  justfile the loop could never fail, so an invalid sprint file passed the local gate.
- `sprints/README.md` (new section Lane paths), `sprints/_template.yaml`, the
  `sprint-planner` role and the `plan-sprint` and `run-sprint` skills; `.claude/` regenerated.
- 30 new tests in `python/tests/test_sprint.py`; the full Python suite passes (4037).

## State at the end of the session
- Nothing half done.

## Decisions taken
- `README.md`, `README.it.md` and the methods indexes count as generated artifacts as whole
  files: their generated block cannot be separated from the rest in a diff.
- A file needed outside a lane's paths is a planning change, made in the sprint file first.

## Open questions for the maintainer
- none

## Next steps
1. #331: MCP protocol check with the Inspector in CI.
