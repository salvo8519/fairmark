# 2026-11-17 — feat-259-envelope-diagnostics

- Issue: #259 One envelope: diagnostics block and envelope version
- Pull request: none (opened by the orchestrator after T24)
- Spec: SPEC-0012 (tasks T5 to T23 in this session)

## Done
- T5 envelope tests in `registry.rs`, ignored until T11 enables them.
- T6 warning tests FM-W001 to FM-W016 and the echo test, `tests/warnings.rs`, ignored until T11 and T12.

## State at the end of the session
- In progress: tasks T5 to T23 of `specs/0012-envelope-diagnostics/tasks.md`, one commit each.
- Test-first commits (T5 to T10) mark the new tests `#[ignore = "SPEC-0012: enabled by
  T<n>"]`, so that every commit passes `just check`; the implementation task removes the
  marker.

## Decisions taken
- none yet

## Open questions for the maintainer
- none

## Next steps
1. T7: not computed, checks, defaults key and decision 14 tests.
