# SPEC-0009 — Tasks

Status: active (SPEC-0009 approved on 2026-11-08).

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes.

## Phase A: instalment, number of instalments, future value (issue #136)

### Tests (write first, they must fail)
- [x] T1 Reference functions `scripts/reference_loans.py` (50 digits, local context):
  periodic rate, instalment, number of instalments, future value, schedules [R1, R6, R8, R10,
  R12 to R20]
- [x] T2 [P] Golden cases `tests/golden/loans.instalment/`: ECMA-376 (AC1), balloon and due
  against the reference, form (b), rate zero, errors [AC1, AC2, AC11, AC12, AC12a]
- [x] T3 [P] Golden cases `tests/golden/loans.number_of_instalments/`: whole and non-integer
  `n*`, final instalment, due, rate zero, errors [AC3, AC11]
- [x] T4 [P] Golden cases `tests/golden/tvm.future_value/`: ECMA-376 (AC4), reference, rate
  zero, errors [AC4, AC11]
- [x] T5 [P] Contract cases for the three methods [R5, R11; AC13]

### Implementation
- [x] T6 Error variants `FM-E020` and `FM-E021`, code tests,
  `docs/en/reference/errors.md` [R7, R9]
- [x] T7 `tvm::annuity_factor` extracted from `tvm.annuity` (refactoring, numbers unchanged)
- [x] T8 `loans.rs`: rate-form helper, `loans.instalment`, `loans.number_of_instalments`
  with explain and conventions; unit tests [R1 to R9]
- [x] T9 `tvm.future_value` with explain and conventions; unit tests [R10, R11]
- [x] T10 Registry entries, `uv run just generate` [R1 to R11]

### Adapters and docs
- [x] T11 [P] Python helpers `fm.loans.instalment`, `fm.loans.number_of_instalments`,
  `fm.tvm.future_value`; cross-check with `numpy_financial` (`pmt`, `nper`, `fv`) [AC3]
- [x] T12 [P] Methodology pages `loans-instalment.md`, `loans-number-of-instalments.md`,
  `tvm-future-value.md`, nav, index; Italian edition; release note `changes/136.feature.md`

## Phase B: amortisation schedules (issue #137)

### Tests (write first, they must fail)
- [x] T13 [P] Golden cases `tests/golden/loans.amortisation_schedule/`: ECMA-376 principal
  portions (AC5), French, fixed amortisation schedule, bullet, grace and balloon, rounding
  `half_up` and `half_even`, textbook worked examples, errors [AC5 to AC10, AC15]
- [x] T14 [P] Contract cases `tests/contracts/loans.amortisation_schedule.yaml` [R5, R19, R22]

### Implementation
- [x] T15 Error variants `FM-E022` and `FM-E023`, code tests, error reference [R15, R16, R21]
- [x] T16 `loans.amortisation_schedule` with rows, totals, final adjustment, bounded explain
  and conventions [R12 to R22]
- [x] T17 Property and unit tests: sums, monotone balances, present value of instalments,
  time k, form (a) equals form (b), explain size [AC8, AC12a, AC13, AC14]
- [x] T18 Registry entry, `uv run just generate`

### Adapters and docs
- [x] T19 [P] Python helper `fm.loans.amortisation_schedule`; reference comparison on random
  loans; cross-check with `numpy_financial` (`ipmt`, `ppmt`)
- [x] T20 [P] Methodology page `loans-amortisation-schedule.md`, nav, index; Italian
  edition; glossary terms; release note `changes/137.feature.md`

## Project
- [x] T21 Conventional Commit scope `loans` (one scope per theme module) in
  `scripts/conventions.py` and `docs/en/project/conventions.md`
- [x] T22 Roadmap: issues #136 and #137 done, page regenerated with `scripts/roadmap.py render`

## Verification
- [x] T23 `uv run just check` green; `numerical-auditor` and `code-reviewer` reviews done,
  findings applied, all AC ticked in spec.md; recorded in the description of pull request #156
  (https://github.com/salvo8519/fairmark/pull/156, section Verification); ticked by maintainer
  decision of 2026-11-08
