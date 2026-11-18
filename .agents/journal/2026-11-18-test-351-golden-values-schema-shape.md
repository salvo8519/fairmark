# 2026-11-18 — test/351-golden-values-schema-shape

- Issue: #351 Golden cases assert values, schema snapshots assert shape
- Pull request: none yet
- Spec: SPEC-0013 (approved 2026-11-17); migration list approved 2026-11-18 on condition that
  the list regenerated on current `main` is identical (recorded on #351)

## Done
- Design, spec, plan, tasks and the dry-run list (119 files, 1,437 keys) committed earlier.
- Rebased on `origin/main` at `df4d11f`; `list` regenerated: identical to the approved list.
- T2 tests of `scripts/golden_values_migration.py` (`python/tests/scripts/`).
- T3 Rust runner: semantic sections, counts, null or absent.
- T4 Python runner: the same rules; case shape accepts the new sections.

## State at the end of the session
- Working: T5 (`jsonschema` in the dev group) next.

## Decisions taken
- The refusal of `/errors/N` keys in `expect_error_report` (R5) moves from T3/T4 to T10:
  five golden files hold such keys until the migration (T9).
- The migration is applied only through `scripts/golden_values_migration.py apply` on the
  approved files, never by editing golden files by hand (maintainer approval of 2026-11-18).

## Open questions for the maintainer
- None.

## Next steps
1. T5: declare `jsonschema` in the `dev` group; check that `uv.lock` gains no package.
