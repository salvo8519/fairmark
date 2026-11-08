# SPEC-0008 — Tasks

Status: active (SPEC-0008 approved on 2026-11-07).

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes.

## Decision and tooling
- [x] T1 ADR 0010: composition through the registry on JSON values, analysis theme only, one
  level deep (`architect`; maintainer accepts) [R1, R2]
- [x] T2 Golden runner: accept JSON Pointer keys in `expect`, existing cases unchanged
  (Rust and Python runners) [AC6, AC9, AC10]

## Tests (write first, they must fail)
- [x] T3 [P] Reference functions in `scripts/reference_values.py` and
  `scripts/cross_check_analysis.py` (scipy, numpy-financial) [AC1, AC2, AC6, AC9, AC10]
- [x] T4 [P] Golden cases `tests/golden/analysis.goal_seek/` [R10, R13; AC1, AC2, AC5]
- [x] T5 [P] Golden cases `tests/golden/analysis.sensitivity/`, including the hand-built
  spreadsheet reference [R15, R16, R17; AC6, AC7]
- [x] T6 [P] Golden cases `tests/golden/analysis.scenarios/` [R18, R19; AC9]
- [x] T7 [P] Golden cases `tests/golden/analysis.tornado/` [R20; AC10]
- [x] T8 [P] Contract cases `tests/contracts/analysis.*.yaml` [R2, R5, R8, R9, R14, R21;
  AC12, AC13, AC16, AC18, AC19]
- [x] T9 [P] Budget test `crates/fairmark-core/tests/analysis_budget.rs` [R22; AC17]

## Implementation
- [x] T10 Error variants `FM-E013` to `FM-E017` with code tests and
  `docs/en/reference/errors.md` [R11, R12, R3, R5, R2, R23; AC21]
- [x] T11 `registry::evaluate` internal entry point; test that it equals `run` on every
  existing golden case [R1, R6]
- [x] T12 `analysis/pointer.rs` with its unit tests first [R3, R4; AC11, AC15]
- [x] T13 `solver.rs` Brent's method with its unit tests first [R10, R11, R12; AC3, AC4]
- [x] T14 `analysis/mod.rs`: `BaseInput` (canonical serialisation, object schema),
  `InnerMethod`, nested check, evaluation cap, base evaluation [R1, R2, R5, R7, R8, R9]
- [x] T15 `analysis.goal_seek` with explain and conventions [R10 to R14]
- [x] T16 `analysis.sensitivity` [R15, R16, R17]
- [x] T17 `analysis.scenarios` [R18, R19]
- [x] T18 `analysis.tornado` [R20, R21]
- [x] T19 Registry entries, `uv run just generate` (schemas v1, index, methods reference) [R8]
- [x] T20 Property tests: cell equals direct run; byte-identical reruns; key order and number
  spelling do not change hash or value [AC8, AC14, AC15]

## Adapters and docs
- [x] T21 [P] Python helpers `python/fairmark/analysis.py` with docstrings
- [ ] T22 [P] Methodology pages for the four methods and `docs/en/concepts/analysis.md`, nav
  entries, index rows
- [x] T23 [P] Release note fragment in `changes/`
- [ ] T24 `uv run just docs`, docs coverage green, `/translate` for the Italian edition

## Verification
- [x] T25 `numerical-auditor` review (Brent path, golden sources) and `code-reviewer` review
  done, every blocking and should-fix finding resolved, all AC ticked in spec.md; recorded on
  pull request #129 (https://github.com/salvo8519/fairmark/pull/129#issuecomment-6714463167);
  ticked by maintainer decision of 2026-11-08
