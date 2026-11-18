# SPEC-0013 — Tasks

Status: active (SPEC-0013 approved on 2026-11-17). Issue #351.

Rules: test first (inside each task the test is written and seen to fail before the change that
makes it pass); one task per commit; `[P]` tasks touch disjoint files and can run in parallel;
tick a task only when `just check` passes. No golden file is touched before the maintainer has
approved, by name, the list in `golden-migration.md` (T9).

## Preparation
- [x] T1 Design, inventory, migration script and dry-run list (`design.md`,
  `golden-migration.md`, `golden-migration.json`); spec, plan and tasks [decisions 1 to 10]

## Tests and runners
- [x] T2 [P] Tests of `scripts/golden_values_migration.py` on temporary trees and synthetic
  results (`python/tests/test_golden_values_migration.py`) [R13; AC13]
- [x] T3 [P] Rust runner: semantic sections, `expect_counts`, `#count`, null or absent; tests
  on temporary cases (`crates/fairmark-core/tests/golden.rs`) [R1 to R4; AC1 to AC3]
- [x] T4 [P] Python runner: the same, and `NEW_CASE_KEYS` (`python/tests/test_golden.py`)
  [R1 to R4; AC1 to AC3]
- [x] T5 [P] `jsonschema` declared in the `dev` group; lock unchanged (`pyproject.toml`,
  `uv.lock`) [R10; AC10]

## Shape
- [x] T6 Conformance test against the committed snapshots of the declared versions, success
  and error, closed mode, formats, inner analysis values, declared keys; old test removed from
  `test_output_schemas.py` (`python/tests/test_golden_conformance.py`) [R7, R8; AC6 to AC8]
- [ ] T7 Order test of every envelope list on every golden result and on synthetic results
  (same module) [R9; AC9]

## Rule
- [ ] T8 Rule in `.agents/skills/golden-case/SKILL.md` and new `tests/golden/README.md`;
  `uv run just generate`; new-case rules in `test_golden_case_shape` [R11, R12; AC11, AC12]

## Migration (after the maintainer's named approval)
- [ ] T9 `list` again on `main`; if the list differs from the approved one, stop for a new
  approval; `apply` on the approved files; `verify` against `origin/main` [R14; AC14]
- [ ] T10 Retired keys, and `/errors/...` keys of `expect_error_report`, refused by both
  runners and by `test_golden_case_shape` [R5, R6; AC4, AC5]. The refusal of R5 waits for
  the migration: before T9, five golden files still hold `/errors/N` keys.

## Verification
- [ ] T11 `code-reviewer` pass on the branch and `numerical-auditor` on the migration (golden
  files change, values do not); every finding fixed; all AC ticked in `spec.md`
