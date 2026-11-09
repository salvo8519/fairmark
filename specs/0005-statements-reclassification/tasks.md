# SPEC-0005 — Tasks

Status: active (SPEC-0005 approved on 2026-11-08). Implementation starts after the rebase on
main (see `plan.md`, "Dependency on main").

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes. Golden files are written once, in the pull
request where every output they assert exists (`plan.md`, "Test strategy").

## Project (before PR 1)
- [x] T1 Roadmap: the coordinator adds the task for PR 2 under epic #34 (referred to here as
  "#236", to be replaced with its number), retitles #35 and #36 and removes
  `needs-methodology` from #35; check that `python scripts/roadmap.py` passes
- [x] T2 Spec status `Implementing` when PR 1 opens

## PR 1: input schema and balance sheet (closes #35)

### Tests (write first, they must fail)
- [x] T3 Reference `scripts/reference_statements.py` (50 digits, local context), written by
  `test-engineer` from Appendices A.1, B.1, C and the requirements without reading the Rust
  code: validation, best effort, input checks, balance sheet, net financial position,
  `current_non_current`; self-tests [R2 to R15, R22]
- [x] T4 [P] Golden value cases `tests/golden/statements.reorganise/`:
  `balance-sheet-ordinary.yaml` and `balance-sheet-abridged.yaml` (same company, AC1 balance
  sheet part), `missing-treatment.yaml` (AC2), `treatment-once-three-years.yaml` and
  `override-financial-fixed-assets.yaml` (AC7), one file per balance sheet adjustment kind
  (`non_operating_asset`, `financial_receivable`, `operating_cash`, `accrued_interest`,
  `tax_instalments`, `non_current_trade_payables`, `debt_like_provision`, `dividends_declared`)
  (AC7), `subscribed-capital-unpaid.yaml` and `dividends-declared.yaml` (AC12),
  `leap-year-dates.yaml` (29 February 2024, 28 February 2025) and `end-of-month-dates.yaml`
  (28 February 2023, 29 February 2024 consecutive; AC6 date part), each asserting balance sheet
  outputs and the `meta.defaults_applied` entries of its defaults; source: T3
- [x] T5 [P] Golden error and not-checked cases: one `FM-E026` file per R7 relation of the
  statutory form and of the reclassified balance sheet, each with a passing twin that removes one
  side and asserts `not_checked` (AC4); every `FM-E001` and `FM-E009` case of AC5, and
  27 February 2023 followed by 29 February 2024 (`FM-E001`, AC6); an adjustment
  larger than its source (AC7) [R2, R3, R5, R7, R9, R10, R24]
- [x] T6 [P] Contract tests: Rust tests through `registry::evaluate` passing the returned net
  financial position, non-operating assets and debt-like items to `dcf.fcff`, and
  `current_non_current` to `ratios.liquidity` and `ratios.capital_structure` (AC9 balance sheet
  part); `tests/contracts/statements.reorganise.yaml` for extreme amounts (`FM-E008`)

### Implementation
- [x] T7 Error variant `FM-E026` `StatementsDoNotReconcile` (free on main), code test, row in
  `docs/en/reference/errors.md` [R7, R24]
- [x] T8 Commit scope `statements` in `scripts/conventions.py` and
  `docs/en/project/conventions.md`
- [x] T9 `statements/input.rs`: every input type of version 1, both forms and both statements,
  `Serialize + Deserialize + JsonSchema`, `deny_unknown_fields`, units on every field,
  enumerations only; one constant table for Appendix A; `Reorganise::apply_defaults` listing in
  `meta.defaults_applied` the default `write_downs_of_receivables` and every `default`
  destination applied to a given aggregate, with input pointer, destination and reason
  [R1 to R5, R8, R9, R10, R17 input, R24]
- [x] T10 `statements/partial.rs` and `statements/validate.rs`: `Partial` with missing pointers,
  `not_computed` per block, signs, forms, dates with the end-of-month rule, adjustment lists,
  overrides, permitted destinations [R2, R3, R6, R8 to R10]
- [x] T11 `statements/checks.rs`: the relations of R7 on the statutory inputs of both statements
  and on the reclassified balance sheet, `checks` output [R7]
- [x] T12 `statements/balance_sheet.rs`: assignments with origin, treatments, overrides, balance
  sheet adjustments, B.1 blocks, identity of R12, net financial position as
  `ratios::NetFinancialPosition` with composition and current and non-current parts, bridge items
  as `tv::Line` and the `dcf.fcff` debt-like type with `MeasurementBasis`, `current_non_current`,
  explain steps and conventions [R8 to R15, R22]
- [x] T13 Reclassified form for the balance sheet: totals derived by B.1, given against derived
  [R5]
- [x] T14 Property tests AC7 and AC8; unit tests AC13 for the balance sheet, including every
  balance sheet `default` destination in `meta.defaults_applied` and none for `fixed` ones;
  exhaustiveness of
  `not_computed` for `missing-treatment.yaml`; income statement kinds refused with `FM-E001`
  until PR 2 (`plan.md`, "Behaviour between pull requests")
- [x] T15 Registry line `statements::Reorganise` (preview); `uv run just generate` writes
  `schemas/statements.reorganise/v1.json`, `schemas/index.json`, `docs/en/reference/methods.md`
  [R1]

### Adapters and docs
- [x] T16 [P] Python helper `python/fairmark/statements.py` (`reorganise`), export, docstring
  with a runnable example; `python/tests/test_reference_statements.py` against T3 on seeded
  random balance sheets in both forms
- [x] T17 [P] Methodology page `docs/en/methods/statements-reorganise.md` (forms, best effort,
  A.1, B.1, balance sheet kinds of C, `FM-E026`), nav in `zensical.toml`, row in
  `docs/en/methods/index.md`; entry `"0005"` in `specs/method-evidence.yaml` (methods
  `[statements.reorganise]`, `numerical_audit: T41`, `code_review: T41`)
- [x] T18 [P] Glossary terms of the balance sheet in `i18n/glossary.yaml`; Italian edition by
  `translator`; release note `changes/35.feature.md`

### Verification
- [ ] T19 `code-reviewer` review of the PR 1 diff and `numerical-auditor` recomputation of the
  PR 1 golden cases from T3 and of determinism; findings applied

## PR 2: income statement (closes #236; part of #36)

### Tests (write first, they must fail)
- [x] T20 Reference: A.2, B.2, income statement adjustments, the `lease` double entry (R21),
  reconciliations (R19) and the reclassified income statement added to
  `scripts/reference_statements.py` [R16 to R21]
- [x] T21 [P] Golden cases: `ordinary-aggregates.yaml` and `abridged-aggregates.yaml` (AC1),
  `few-totals.yaml` (AC2), income statement relations with not-checked twins (AC4), one file per
  kind `investing`, `non_recurring`, `lease` (AC7), layer cases (AC10), `write-downs.yaml` and
  `write-downs-operating.yaml`, error `non_recurring` on D19 (AC11), `lease-restatement.yaml`
  (AC12); source: T20
- [x] T22 [P] Contract tests: `operating_profit` and `depreciation_and_amortisation` accepted as
  `FreeCashFlowLines` fields; `revenue`, `ebitda`, `net_income`, `purchases`,
  `invested_capital` accepted by the `ratios.*` groups; no `cost_of_sales` (AC9, R20)

### Implementation
- [x] T23 `statements/income_statement.rs` (R19 identities as internal invariants, `FM-E012`;
  plan step 6): assignments of A.2, layers of B.2, write-downs and the
  `operating` option, A5 and B14 core, `ebitda_definition`, reconciliations with A − B, checks
  of profit before tax and net income [R16 to R20]
- [x] T24 Adjustments `investing` and `non_recurring` (D18 and D19 refused); `lease` on both
  statements with its equity effect; removal of the PR 1 refusal [R18, R21]
- [x] T25 Reclassified form for the income statement [R5]
- [x] T26 Property tests AC10; unit tests AC11 and AC13 for the income statement, including the
  `default` destinations of A5, B14 and B10 d in `meta.defaults_applied`; exhaustiveness
  of `not_computed` for `few-totals.yaml`
- [x] T27 `uv run just generate`; `schemas/statements.reorganise/v1.json` changed only by the two
  additive fields deferred by D4 (`external_costs`, `other_charges_below_ebitda`), defined by E1
  and E2 of 2026-11-09
- [x] T27a Review round 1 (2026-11-09): E1, E2, E4 to E7; B10 c larger than its group is
  `FM-E026`; pointers of the items read; lease convention when B8 is missing;
  `related_non_operating_asset` reported; sources; reference and golden cases updated
- [ ] T27b E3 (default `operating` for B10 d) and audit F1 (input default listed only when B10 d
  is given), under the approval of 2026-11-09 for 18 named golden files on main; P1 (B10 c
  operating by default), P3 (lease liability in gross debt), P4 and P5 (documentation)
- [ ] T27c P2 (income and costs follow the classification of the related balance sheet item):
  waits for the maintainer on four implementation points

### Adapters and docs
- [x] T28 [P] `python/tests/test_reference_statements.py` extended to income statements
- [x] T29 [P] Methodology page: A.2, B.2, write-down rule, R20 table "which output feeds which
  method"
- [x] T30 [P] Glossary terms of the income statement; Italian edition by `translator`; release
  note for #236

### Verification
- [ ] T31 `code-reviewer` review of the PR 2 diff and `numerical-auditor` recomputation of the
  PR 2 golden cases; findings applied

## PR 3: years, changes and independent sources (closes #36)

### Tests (write first, they must fail)
- [ ] T32 Reference: changes between consecutive entries added to
  `scripts/reference_statements.py` [R23]
- [ ] T33 [P] Golden cases `three-years.yaml`, `one-year-no-date.yaml` (AC6), `reclassified.yaml`
  with the AC1 company and a statutory and reclassified mix (AC3); source: T32
- [ ] T34 [P] Golden cases from published worked examples, one balance sheet and one income
  statement, re-derived from printed aggregates, with edition and page (AC14). Waits for the
  maintainer to name the source (V27)
- [ ] T35 [P] Golden case from an anonymised practitioner statement (Decision 16). Waits for the
  maintainer to provide the case

### Implementation
- [ ] T36 `statements/changes.rs`: changes per pair with labels, explain and `not_computed`;
  empty `changes` and its convention for one entry [R23]
- [ ] T37 Property test AC6 (sum of changes equals last minus first); final AC13 unit tests;
  `uv run just generate` with the input schema hash unchanged

### Adapters and docs
- [ ] T38 [P] `python/tests/test_reference_statements.py` extended to several years and mixed
  forms
- [ ] T39 [P] Concept page `docs/en/concepts/financial-statements.md`; methodology page completed
  (example from a filed abridged statement, "Verified against", sources of "Sources to verify"
  resolved or marked); Italian edition by `translator`; release note `changes/36.feature.md`
- [ ] T40 Roadmap: #35, #36 and #236 done; `python scripts/roadmap.py render`

### Verification
- [ ] T41 `uv run just check` green; independent `numerical-auditor` audit of every golden case
  of `statements.reorganise` and of determinism, and full `code-reviewer` review of the method;
  findings applied; every AC ticked in spec.md; recorded in the description of the PR 3 pull
  request
