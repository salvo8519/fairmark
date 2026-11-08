# SPEC-0007 — Tasks

Status: active (SPEC-0007 approved on 2026-11-07). Issue #114.

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes.

## Tests (write first, they must fail)
- [x] T1 [P] Reference functions in `scripts/reference_ratios.py` (Python `decimal`, 50 digits,
  local context, written from the formulas) [AC1 to AC4]
- [x] T2 [P] Golden cases `tests/golden/ratios.profitability/` [R1, R2, R3; AC1, AC2, AC3]
- [x] T3 [P] Golden cases `tests/golden/ratios.capital_structure/` [R1, R2, R3, R6; AC1, AC2, AC3]
- [x] T4 [P] Golden cases `tests/golden/ratios.liquidity/` [R1, R3; AC1, AC3]
- [x] T5 [P] Golden cases `tests/golden/ratios.working_capital/` [R1, R2, R3; AC1, AC2, AC3]
- [x] T6 [P] Golden cases `tests/golden/ratios.debt_service/` [R1, R3, R4, R6; AC1, AC3, AC4]
- [x] T7 [P] Contract cases `tests/contracts/ratios.*.yaml` (extreme inputs, conventions)

## Implementation
- [x] T8 `ratios/mod.rs`: `Balances`, `DayBasis`, `RatioValue`, `Ratio` trait with `Quotient`
  and `DayRatio`, balance resolver, `NetFinancialPosition`, with unit tests [R2, R3, R6]
- [x] T9 `ratios.profitability` with explain and conventions [R1, R2, R3]
- [x] T10 `ratios.capital_structure` [R1, R2, R3, R6]
- [x] T11 `ratios.liquidity` [R1, R2, R3]
- [x] T12 `ratios.working_capital` [R1, R2, R3]
- [x] T13 `ratios.debt_service` [R1, R2, R3, R4, R6]
- [x] T14 Registry entries, `uv run just generate` (schemas v1, index, methods reference)

## Adapters and docs
- [x] T15 [P] Python helpers `python/fairmark/ratios.py` with docstrings
- [x] T16 [P] Methodology pages `docs/en/methods/ratios-*.md`, nav entries, index rows
- [x] T17 [P] Release note fragment `changes/114.feature.md`
- [x] T18 Italian edition (`docs/it/methods/ratios-*.md`, index) with the glossary, stamped
- [x] T19 `uv run just check` green

## Verification
- [ ] T20 `numerical-auditor` review, `code-reviewer` review, AC ticked in spec.md by the
  maintainer

## Not started: open questions in `plan.md`
- [ ] T21 `ratios.covenants` [R5; AC5] (open question 1)
- [ ] T22 ROE decomposition, leverage form (open question 2)
- [ ] T23 ROA (open question 3)
