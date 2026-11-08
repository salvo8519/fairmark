# SPEC-0006 — Tasks

Status: active (SPEC-0006 approved on 2026-11-07). Issue #115.

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes.

## Tests (write first, they must fail)
- [x] T1 [P] Independent reference `scripts/reference_tv.py` (50 digits, local context) and
  `python/tests/test_reference_tv.py` [R1 to R7, R9]
- [x] T2 [P] Golden cases `tests/golden/tv.*/`, including the published Damodaran table cells
  [R1 to R7, R9, R10; AC1, AC2, AC5, AC6]
- [x] T3 [P] Contract cases `tests/contracts/tv.*.yaml` [R8, R10]

## Implementation
- [x] T4 Error variants `FM-E018`, `FM-E019` with `docs/en/reference/errors.md` [R10; AC6]
- [x] T5 `tv/mod.rs`: growth cap check, domain checks, shared types [R8, R10]
- [x] T6 `tv.gordon` and `tv.value_driver` with unit tests [R1, R2, R8; AC1, AC2]
- [x] T7 `tv.h_model` with unit tests [R3, R8; AC3]
- [x] T8 `tv.exit_multiple` with unit tests [R4, R8; AC4], including `book_value` (P/BV)
- [x] T8b Maintainer decisions of 2026-11-08: P/BV metric, required `basis` of
  `tv.normalised_income` checked by `tv.compare`, tightest golden tolerances [R4, R6, R9]
- [x] T8c Decided under the maintainer's instruction of 2026-11-08: required `basis` of
  `tv.liquidation` checked by `tv.compare`, own capitalisation rate of normalised income
  stated, AC1 source check recorded [R7, R9; AC1]
- [x] T9 `tv.limited_horizon` and `tv.normalised_income` with unit tests [R5, R6, R8; AC5]
- [x] T10 `tv.liquidation` with unit tests [R7; AC6]
- [x] T11 `tv.compare` with unit tests [R9]
- [x] T12 Registry entries, `uv run just generate` (schemas v1, index, methods reference)

## Adapters and docs
- [x] T13 [P] Python helpers `python/fairmark/tv.py` with docstrings
- [x] T14 [P] Methodology pages `docs/en/methods/tv-*.md`, nav entries, index rows
- [x] T15 [P] Release note fragment `changes/115.feature.md`
- [x] T16 Italian edition (`docs/it/`), stamped with `scripts/i18n.py`; `uv run just check`

## Verification
- [ ] T17 `numerical-auditor` review, `code-reviewer` review, all AC ticked in spec.md
  (maintainer)
