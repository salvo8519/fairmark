# 2026-11-18 — test/351-golden-values-schema-shape

- Issue: #351 Golden cases assert values, schema snapshots assert shape
- Pull request: #394 (ready)
- Spec: SPEC-0013 (approved 2026-11-17); migration list approved 2026-11-18 on condition that
  the list regenerated on current `main` is identical (recorded on #351)

## Done
- Design, spec, plan, tasks and the dry-run list (119 files, 1,437 keys) committed earlier.
- Rebased on `origin/main` at `df4d11f`; `list` regenerated: identical to the approved list.
- T2 tests of `scripts/golden_values_migration.py` (`python/tests/scripts/`).
- T3 Rust runner: semantic sections, counts, null or absent.
- T4 Python runner: the same rules; case shape accepts the new sections.
- T5 `jsonschema>=4.23` in the `dev` group; `uv.lock` gains two lines, no package (104).
- T6 `python/tests/test_golden_conformance.py`: 666 golden results conform, closed mode.
- T7 order test of every envelope list and error report on every golden result.
- T8 rule in the skill and `tests/README.md`; new-case rules; the new sections are
  protected by `scripts/check_golden_changes.py`, which now reads only YAML files.

## State at the end of the session
- T9 rebased on `origin/main` at `31dbefa`; `list` identical to the approved plan (119 files,
  1,437 keys); `apply` on the 119 files; `verify --base-root` (archive of `origin/main`):
  119 checked, 0 problems.
- T10 retired sections and `/errors/N` keys of `expect_error_report` refused by both runners
  and the case shape test; `scripts/mcp_protocol_check.py` reads `expect_defaults`.
- PR #394 opened. Code review (request changes) fixed in this PR: B2 order checks against
  the documented order (`7b57341`), S3 closed choices (`11a6e4f`), S5 exact text (`7df5c5c`),
  S6 shared schema walk `scripts/golden_schema.py` (`a1f81b8`), nits N8 to N13 (`5029771`),
  S4 tolerant-reader rule in the docs (`6497666`). Audit: sound.

## Decisions taken
- `check_golden_changes.py` protected only some sections: the diagnostics sections of
  SPEC-0012 and the new ones are added (found in T8, fixed in the same task).
- R9: the input order of `meta.defaults_applied` is checked as the order of their `default:`
  echo in `explain.conventions` (spec clarified in T7); it holds on all 111 results with defaults.
- `uv.lock` keeps `revision = 3`: the local uv (0.13.7) wrote 5; `uv lock --check` accepts 3.
- The refusal of `/errors/N` keys in `expect_error_report` (R5) moves from T3/T4 to T10:
  five golden files hold such keys until the migration (T9).
- The migration is applied only through `scripts/golden_values_migration.py apply` on the
  approved files, never by editing golden files by hand (maintainer approval of 2026-11-18).

## Open questions for the maintainer
- R9 (pending sign-off): the engine departs from the input order of defaults in eight places
  (`KNOWN_DEFAULT_ORDER_DEPARTURES` in `python/tests/test_golden_conformance.py`). Change the
  engine (its own issue) or restate the rule.

## Next steps
1. T11 on #394: `code-reviewer` pass and `numerical-auditor` on the migration; the maintainer
   adds the `numerical-change` label for `check_golden_changes.py` (no value changes).
