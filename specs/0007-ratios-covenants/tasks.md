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

## Maintainer decisions of 2026-11-08 (spec.md)
- [x] T21 Leverage form of the ROE decomposition, optional `leverage` block, `debt` in the
  balance sheets [decision 2]
- [x] T22 ROA = EBIT / total assets [decision 3]
- [x] T23 Not-meaningful rules with a golden case each [decision 7]
- [x] T24 Optional `debt_schedule`; `reserve_balance` in the LLCR [decisions 8, 9]
- [x] T25 Golden tolerance 1e-20 for every `ratios.*` case [decision 10]
- [x] T26 Reference script, docs (English and Italian), glossary, release note, schemas v1
- [x] T27 `ratios.covenants` [R5; AC5]: separate method and issue #147 (decision 1), broken
  down in T28 to T35

## Covenants, issue #147 (plan.md, "Covenants")
- [x] T28 Reference function `covenants` in `scripts/reference_ratios.py` [R5; AC5]
- [x] T29 Golden cases `tests/golden/ratios.covenants/` and contract cases
  `tests/contracts/ratios.covenants.yaml` [R5; AC5; decision 1]
- [x] T30 `ratios/covenants.rs`: input, outcome, headroom, not-tested rule, explain and
  conventions, unit tests [R5; decision 1]
- [x] T31 Registry entry, `uv run just generate` (schema v1, index, methods reference)
- [x] T32 [P] Python helper `fairmark.ratios.covenants`
- [x] T33 [P] Methodology page `docs/en/methods/ratios-covenants.md`, nav entry, index row;
  Italian edition with the glossary, stamped
- [x] T34 [P] Release note fragment `changes/147.feature.md`; roadmap #147 done
- [x] T35 `uv run just check` green

## Partial input, issue #262 (amendment of 2026-11-09)
- [x] T36 Reference `scripts/reference_ratios_partial.py` (Python `decimal`, written from the
  formulas, no shared code) [R7, R10, R11; AC7, AC8]
- [x] T37 Golden cases with partial inputs for each method; the three approved version 1
  error cases of `ratios.profitability` replaced by their partial results [AC7, AC8, AC9,
  AC10]
- [x] T38 Engine test removing each input of the computability table [AC6]
- [x] T39 Shared `NotComputedEntry` with an optional `reason`; `statements.reorganise` output
  unchanged
- [x] T40 Every `ratios.*` method: optional amounts, `not_computed`, version 2, schemas v2
  [R7 to R11]
- [ ] T41 Python helpers, methodology pages (English and Italian), release note
  `changes/262.feature.md`; roadmap #262 done

## Review of 2026-11-09 (issue #262)
- [x] T42 The floor (R12): `FM-E027` with the alternative sets, a golden case and an engine
  test per method; spec, errors page and method pages in English and Italian
- [x] T43 The two documented defaults (R13) in `meta.defaults_applied` with their warnings and
  their admission tests; `reserve_balance` reported, not written into the input
- [x] T44 The discount rate checked whenever given (R14)
- [x] T45 One net financial position formula and audit trail; no default value for a missing
  term; pointers sorted with array indices as numbers; covenant reason wording
- [x] T46 AC6 rows for the other items and the opening net financial position; `expect_length`
  in the new golden cases
- [x] T47 Minimal tool examples from the partial golden cases (`scripts/tool_examples.py`)
