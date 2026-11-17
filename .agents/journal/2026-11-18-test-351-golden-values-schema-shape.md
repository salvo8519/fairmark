# 2026-11-18 — test/351-golden-values-schema-shape

- Issue: #351 Golden cases assert values, schema snapshots assert shape
- Pull request: none yet
- Spec: SPEC-0013 (approved 2026-11-17); migration list approved 2026-11-18 on condition that
  the list regenerated on current `main` is identical (recorded on #351)

## Done
- Design, spec, plan, tasks and the dry-run list (119 files, 1,437 keys) committed earlier.
- Rebased on `origin/main` at `df4d11f`; `list` regenerated: identical to the approved list.
- T2 tests of `scripts/golden_values_migration.py` (`python/tests/scripts/`).

## State at the end of the session
- Working: T3 (Rust runner) next.

## Decisions taken
- The migration is applied only through `scripts/golden_values_migration.py apply` on the
  approved files, never by editing golden files by hand (maintainer approval of 2026-11-18).

## Open questions for the maintainer
- None.

## Next steps
1. T3: semantic sections in `crates/fairmark-core/tests/golden.rs`, tests on temporary cases.
