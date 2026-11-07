# SPEC-0008 — Tasks

Status: active (SPEC-0008 approved on 2026-11-07).

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes.

## Decision and tooling
- [ ] T1 ADR 0010: composition through the registry on JSON values, analysis theme only, one
  level deep (`architect`; maintainer accepts) [R1, R2]
- [ ] T2 Golden runner: accept JSON Pointer keys in `expect`, existing cases unchanged
  (Rust and Python runners) [AC6, AC9, AC10]

## Tests (write first, they must fail)
- [ ] T3 [P] Reference functions in `scripts/reference_values.py` and
  `scripts/cross_check_analysis.py` (scipy, numpy-financial) [AC1, AC2, AC6, AC9, AC10]
- [ ] T4 [P] Golden cases `tests/golden/analysis.goal_seek/` [R10, R13; AC1, AC2, AC5]
- [ ] T5 [P] Golden cases `tests/golden/analysis.sensitivity/`, including the hand-built
  spreadsheet reference [R15, R16, R17; AC6, AC7]
- [ ] T6 [P] Golden cases `tests/golden/analysis.scenarios/` [R18, R19; AC9]
- [ ] T7 [P] Golden cases `tests/golden/analysis.tornado/` [R20; AC10]
- [ ] T8 [P] Contract cases `tests/contracts/analysis.*.yaml` [R2, R5, R8, R9, R14, R21;
  AC12, AC13, AC16, AC18, AC19]
- [ ] T9 [P] Budget test `crates/fairmark-core/tests/analysis_budget.rs` [R22; AC17]

## Implementation
- [ ] T10 Error variants `FM-E013` to `FM-E017` with code tests and
  `docs/en/reference/errors.md` [R11, R12, R3, R5, R2, R23; AC21]
- [ ] T11 `registry::evaluate` internal entry point; test that it equals `run` on every
  existing golden case [R1, R6]
- [ ] T12 `analysis/pointer.rs` with its unit tests first [R3, R4; AC11, AC15]
- [ ] T13 `solver.rs` Brent's method with its unit tests first [R10, R11, R12; AC3, AC4]
- [ ] T14 `analysis/mod.rs`: `BaseInput` (canonical serialisation, object schema),
  `InnerMethod`, nested check, evaluation cap, base evaluation [R1, R2, R5, R7, R8, R9]
- [ ] T15 `analysis.goal_seek` with explain and conventions [R10 to R14]
- [ ] T16 `analysis.sensitivity` [R15, R16, R17]
- [ ] T17 `analysis.scenarios` [R18, R19]
- [ ] T18 `analysis.tornado` [R20, R21]
- [ ] T19 Registry entries, `uv run just generate` (schemas v1, index, methods reference) [R8]
- [ ] T20 Property tests: cell equals direct run; byte-identical reruns; key order and number
  spelling do not change hash or value [AC8, AC14, AC15]

## Adapters and docs
- [ ] T21 [P] Python helpers `python/fairmark/analysis.py` with docstrings
- [ ] T22 [P] Methodology pages for the four methods and `docs/en/concepts/analysis.md`, nav
  entries, index rows
- [ ] T23 [P] Release note fragment in `changes/`
- [ ] T24 `uv run just docs`, docs coverage green, `/translate` for the Italian edition

## Verification
- [ ] T25 `numerical-auditor` review (Brent path, golden sources), `code-reviewer` review, all
  AC ticked in spec.md
