# SPEC-0005 — Plan

Status: active (SPEC-0005 approved on 2026-11-08); changes to the spec are reflected here first.
One method, `statements.reorganise` version 1, status preview (ADR 0012), delivered in three
implementation pull requests (see "Delivery in three pull requests").

**Dependency on main.** This plan is written on a branch based on an older main. Three building
blocks it reuses are on main and not on this branch: `MeasurementBasis` and the debt-like item
type of the `dcf.fcff` bridge (SPEC-0001 Decision 21, issue #44), and the defaults mechanism
`Method::apply_defaults` with `meta.defaults_applied` (pull request #221). Implementation starts
after the rebase on main; the exact type paths are taken from main at that point, never copied.

## Design

### Module layout in `crates/fairmark-core`
| File | Content |
| --- | --- |
| `src/statements/mod.rs` (new, theme `statements`) | Unit type `Reorganise` implementing `Method` (`ID = "statements.reorganise"`, `VERSION = "1"`); the loop over `years`; assembly of the output; `apply_defaults` |
| `src/statements/input.rs` | Every input type of version 1: `ReorganiseInput`, `Year`, `Statutory`, `Reclassified`, `StatutoryBalanceSheet`, `StatutoryIncomeStatement`, the nodes of Appendix A, `Treatments`, `TreatmentOverride`, `Adjustment` (one variant per kind of Appendix C) |
| `src/statements/partial.rs` | `Partial`: an amount or the set of missing input pointers (R6); checked addition and subtraction that carry the missing pointers |
| `src/statements/validate.rs` | Signs (R3), forms (R2), dates (R2), adjustment lists (R10), overrides (R9), permitted destinations (R8) |
| `src/statements/checks.rs` | The relations of R7 and the identity of R12, each `reconciled` or `not_checked`, `FM-E026` on a difference |
| `src/statements/balance_sheet.rs` | Appendix A.1 and B.1: assignments, treatments, balance sheet adjustments, net financial position, bridge items, `current_non_current` |
| `src/statements/income_statement.rs` | Appendix A.2 and B.2: layers, write-downs (R17), reconciliations (R19), income statement adjustments, the income statement side of `lease` |
| `src/statements/changes.rs` | Changes between consecutive entries (R23) |
| `src/error.rs` | Variant `StatementsDoNotReconcile`, code `FM-E026` |
| `src/registry.rs` | One line `statements::Reorganise`, in a new `statements` group placed before `ratios` |
| `src/lib.rs` | `pub mod statements;` |

The type path `fairmark_core::statements::Reorganise` satisfies the ADR 0009 test.

### Reuse, no duplication
- **Net financial position (R13).** The output holds the SPEC-0007 type itself:
  `net_financial_position: { position: ratios::NetFinancialPosition, total, composition,
  current, non_current }`. `position` is built from the assignments (C.IV and financial debt
  into `cash` and `gross_debt`; C.III, financial receivables, derivatives, accrued interest,
  non-current trade payables and lease liabilities into `other_items` as `NfpItem` with their
  signs). `total` is computed only by `NetFinancialPosition::total`, whose steps are copied into
  the explain, so AC8 holds by construction. The reclassified form takes the same type:
  `net_financial_position: { total?, position? }`, where `position` is a
  `ratios::NetFinancialPosition`; nothing is redefined.
- **Bridge items (R14).** `non_operating_assets` is a list of `tv::Line`, the type `dcf.fcff`
  takes; `debt_like_items` uses the debt-like item type of the `dcf.fcff` bridge, with
  `MeasurementBasis` set to the carrying amount (Decision 15). The same values are returned once,
  in the shapes `dcf.fcff` accepts; the balance sheet block refers to them, it does not repeat
  them under other types.
- **Field names (R15, R20).** `current_non_current` uses the field names of the inputs of
  `ratios.liquidity` and `ratios.capital_structure`; income statement outputs use the names of
  `FreeCashFlowLines` (`depreciation_and_amortisation`) and of the SPEC-0007 inputs (`revenue`,
  `ebitda`, `net_income`, `purchases`, `invested_capital`). A Rust test feeds them to those
  methods through `registry::evaluate` (AC9).
- **Dates (R2).** The date type and parser of `crate::dates` (`FM-E009`); no new parser.
- **Arithmetic.** `crate::num` checked `add` and `sub` only (`FM-E008` on overflow); no
  multiplication, division or rounding anywhere in the method (R24).
- **Explain.** `Explain::step` and `Explain::convention`; no new explain type.

### Input
- `years`: list, oldest first, of `Year { label?, date?, statutory?, reclassified?,
  treatment_overrides? }`. Exactly one form; both is `FM-E001` (R2). An entry with neither form
  is valid and computes nothing: every output is listed in `not_computed`.
- **Nodes (R4).** A leaf aggregate is a decimal. An aggregate with components is an object with
  an optional `total` and optional component fields, for example `equity: { total?,
  share_capital?, ..., profit_or_loss_for_the_year? }` and `receivables: { total?,
  beyond_one_year?, trade? }`. One shape per node, never "a number or an object": untagged
  unions give poor error messages and do not combine with `deny_unknown_fields`.
- **Signs (R3).** A constant table lists the signed fields (A2, A3, B11, C17-bis, 20, 21, A.VI to
  A.X and the signed totals of Appendix B); every other amount below zero is `FM-E001` naming its
  pointer.
- **Choices.** `treatments` (once): one enumeration per aggregate with choice R
  (`financial_fixed_assets`, `shareholder_loans`, `derivative_liabilities`), each a destination
  or a split by labelled amounts; `write_downs_of_receivables`: `investing` or `operating`.
  `adjustments` (once): a list of objects tagged by `kind` (Appendix C), each with `label`,
  `reason`, `amounts` (one entry per year or null) and the fields of its kind. No booleans
  anywhere (R24, AC13).
- **The input schema is complete in the first pull request.** `schemas/statements.reorganise/
  v1.json` is generated once, with every field of the spec, so later pull requests change the
  computation and the output but never the input schema (its hash in `schemas/index.json` stays
  the same).

### Defaults (`Method::apply_defaults`, `meta.defaults_applied`)
Every default is read in one place, `meta.defaults_applied` (R8 as clarified on 2026-11-08,
consistent with SPEC-0001 Decision 24). Two kinds exist:
1. **Input default** documented in the schema: `treatments.write_downs_of_receivables =
   investing` (R17). `Reorganise::apply_defaults` fills it when it is missing; it is listed in
   `meta.defaults_applied`, enters the canonical input of `meta.input_hash` and is stated in
   `explain.conventions`.
2. **Mapping defaults** of Appendix A (choice D: provisions and the TFR as operating provisions,
   A5 and B14 core, C.III and C.IV in the net financial position, prepayments and accruals in
   operating working capital, B10 d in the investing result, and the other D rows). For each
   given aggregate whose `default` destination is applied, `apply_defaults` adds an entry to
   `meta.defaults_applied` with the pointer of the input item (for example
   `/years/1/statutory/balance_sheet/provisions`), the destination as value and the reason (the
   Appendix A row and its decision). The same assignment appears in `assignments` (origin
   `mapping`) and in `explain.conventions`. A default destination is listed only when the
   aggregate is given; a `fixed` destination is not a default and is not listed.
   Because the mapping defaults depend on which aggregates are given and on the adjustments,
   they are derived in `apply_defaults` from the parsed input, before `compute`; `compute` uses
   the same mapping table, so the two cannot drift (one constant table for Appendix A).
The conventions of R2 (order assumed when a date is missing) and R15 (twelve-month reading,
provisions non-current) remain conventions, not defaults.

### Output (per year, then once)
- `form`: `statutory` or `reclassified` (R6: every output states its form).
- `balance_sheet`: `operating` block (`operating_fixed_assets`, `operating_working_capital`,
  `trade_working_capital`, `operating_provisions`, `invested_capital`), `non_operating_assets`,
  `total_funds_invested`, `sources` (`equity`, `quasi_equity_items`, `net_financial_position`,
  `debt_like_items`, `total_sources`), `excess_cash`.
- `income_statement`: the layers of Appendix B.2 in order, `ebitda_definition` (words, R18),
  `materials_consumed`, `purchases`, `reconciliations` (R19).
- `current_non_current` (R15).
- `assignments` (aggregate, pointer, amount, destination, origin) and `adjustments_applied`
  (kind, label, amount, reason, source, target) (R11).
- `checks`: each relation of R7 and R12 with status `reconciled` or `not_checked` and its
  pointers. An enumeration, not a boolean.
- `not_computed`: an object with one list per block (`balance_sheet`, `income_statement`,
  `current_non_current`), each entry `{ output, missing, permitted? }` with JSON pointers,
  sorted by output pointer. Per-block lists keep the pointers of a block stable when another
  block gains outputs, which the golden cases rely on (see "Test strategy").
- Once: `changes`, one entry per pair of consecutive years, labelled with labels or dates,
  with the three changes of R23 and its own `not_computed` list.
- Every computed amount is a decimal; every amount that cannot be computed is null and listed
  (R6). A null is never replaced by zero.

### Error codes
| Code | Variant | Requirement |
| --- | --- | --- |
| `FM-E001` | `InvalidInput` (reused) | R2, R3, R5, R8, R9, R10, R18 |
| `FM-E008` | reused (overflow) | R24 |
| `FM-E009` | `InvalidDate` (reused) | R2 |
| `FM-E012` | `Internal` (reused): a broken identity of R12 in the statutory form is a defect | R12 |
| `FM-E026` | `StatementsDoNotReconcile` (new): both amounts, difference, pointers, fix | R7, R10, R12 |

`FM-E026` is the next free code, on this branch and on main (`FM-E025` is the last used,
confirmed by the coordinator on 2026-11-08).

## Schema changes
Additive: one new method, `statements.reorganise` version 1, preview; snapshot
`schemas/statements.reorganise/v1.json` and one entry in `schemas/index.json` with
`introduced_in: unreleased`. No existing method, schema, golden case or number changes.

| Part | Required | Optional (default) |
| --- | --- | --- |
| Top level | `years` (one or more) | `treatments` (`write_downs_of_receivables`: `investing`), `adjustments` (none) |
| `years[]` | none | `label`, `date`, `statutory` or `reclassified`, `treatment_overrides` |
| `statutory` | none | `layout` (echoed only), `balance_sheet`, `income_statement` |
| Every node | none | `total` and components |

## Numerical approach
1. **Validation, before any computation**: forms, dates (same month and day one calendar year
   later; 28 February after 29 February; or both month ends, one calendar year apart, as the
   end-of-month rule of SPEC-0010, so 29 February 2024 follows 28 February 2023), signs, adjustment lists and reasons, overrides,
   permitted destinations and sources. Every error names a JSON pointer and states the fix.
2. **Input checks (R7)**, per year: totals against components, net = gross − accumulated,
   total assets = total liabilities and equity, A.IX against item 21, splits against their
   aggregate. Exact comparison of decimals; a relation with a side missing is `not_checked`.
3. **Assignments**: each given aggregate to its destination (fixed, default, treatment or
   override), with origin `mapping` or `analyst`. A required treatment that is missing makes the
   dependent outputs `Partial::Missing` with the treatment pointer and the permitted
   destinations.
4. **Adjustments**, in the order given, per year: amount at most what remains in the source
   (`FM-E026` otherwise); source reduced, target increased, step recorded.
5. **Balance sheet (B.1)**, then the identity of R12, then `current_non_current`.
6. **Income statement (B.2)**: value of production, value added, EBITDA, operating profit,
   investing result, profit before financing and taxes, non-recurring result, financing result,
   profit before tax, net income; reconciliations with A − B; checks of profit before tax and
   net income against the statutory amounts.
7. **Reclassified form**: every total of Appendix B derivable from what is given; a given total
   that differs from its derived value is `FM-E026`.
8. **Changes (R23)**: later minus earlier for each pair; missing on either side is not computed.

Additions and subtractions of `rust_decimal` values are exact within 28 significant digits;
statutory amounts in currency units are far inside that range, and an overflow is `FM-E008`.
`Partial` keeps the missing pointers in a `BTreeSet`, so the order of `not_computed` is
deterministic (AC13).

## Test strategy
- **Reference** `scripts/reference_statements.py`: independent of the Rust code, written by the
  `test-engineer` from Appendices A to C and the requirements, 50 significant digits in a local
  `decimal` context (pattern of `scripts/reference_loans.py`). It builds its own best-effort
  logic from the requirements, so a defect in `Partial` is not mirrored. Extended in each pull
  request (balance sheet; income statement; changes).
- **Golden cases** `tests/golden/statements.reorganise/`, tolerance 0, each with its source
  (reference script, published example with edition and page, or anonymised practitioner case):
  - **Rule for the three pull requests**: a golden file is written once, in the pull request
    where every output it asserts exists; it asserts pointers of the blocks implemented there.
    Golden files are never edited afterwards (`tests/golden/` protection). The files named by
    AC1 to AC3, which assert both statements or changes, are written in the second and third
    pull requests; the first pull request uses its own files for the balance sheet.
  - Balance sheet (PR 1): AC1 (balance sheet part), AC2 `missing-treatment.yaml`, AC4 balance
    sheet relations, AC5, AC7 balance sheet kinds and overrides, AC12 subscribed capital unpaid
    and dividends declared.
  - Income statement (PR 2): AC1 `ordinary-aggregates.yaml` and `abridged-aggregates.yaml`, AC2
    `few-totals.yaml`, AC4 income statement relations, AC7 income statement kinds, AC10, AC11
    `write-downs.yaml` and the `operating` variant, AC12 `lease-restatement.yaml`.
  - Years and sources (PR 3): AC3 `reclassified.yaml`, AC6 `three-years.yaml` and
    `one-year-no-date.yaml`, AC14 published examples, the anonymised practitioner case.
- **Properties** (Rust, `proptest`, random valid inputs): AC7 (assigned amounts sum to their
  aggregate), AC8 (identity, no non-operating asset in invested capital, net financial position
  total), AC10 (layer identities), AC6 (sum of changes equals last minus first).
- **Unit tests**: AC13 (every convention when it applies, every error has a pointer, no boolean
  in the generated schema, two runs give identical output), exhaustiveness of `not_computed`
  for AC2 (golden files cannot assert the length of a list).
- **Contract tests** (Rust, through `registry::evaluate`): AC9; and
  `tests/contracts/statements.reorganise.yaml` for extreme amounts (`FM-E008`) and the date
  edge cases.
- **Python**: `python/tests/test_reference_statements.py` compares the engine with the reference
  on seeded random statements in both forms.

## Documentation
- Methodology page `docs/en/methods/statements-reorganise.md`: the two forms, best-effort rules,
  Appendices A to C as tables, the write-down rule and the `operating` option, which output
  feeds which method (R20), an example from a filed abridged statement, "Verified against".
  Grown in each pull request; nav entry in `zensical.toml` and row in `docs/en/methods/index.md`
  in the first.
- Concept page `docs/en/concepts/financial-statements.md` (PR 3).
- `docs/en/reference/errors.md` row for `FM-E026` (PR 1).
- `uv run just generate` for the schema snapshot and `docs/en/reference/methods.md`.
- Italian edition of every page through the `translator` agent, with the terms of the spec's
  "Terminology and sources" added to `i18n/glossary.yaml`.
- Python helper `python/fairmark/statements.py`: `reorganise(years, treatments=None,
  adjustments=None)`, passing amounts as strings; no default values beyond the engine's.
- Release note fragments: `changes/35.feature.md` (PR 1), one for #TBD (income statement), and
  `changes/36.feature.md` (PR 3).
- Commit scope `statements` in `scripts/conventions.py` and `docs/en/project/conventions.md`.

## Delivery in three pull requests
Each pull request leaves `uv run just check` green, keeps the method preview and is reviewed on
its own. The method is not released until the third is merged (`scripts/release.py ready`
checks the evidence of `specs/method-evidence.yaml`).

| Pull request | Content | Requirements | Closes | Tasks |
| --- | --- | --- | --- | --- |
| PR 1: input schema and balance sheet | Complete input schema; validation; best effort; checks; balance sheet in both forms; net financial position; bridge items; `current_non_current`; balance sheet adjustments; registry; Python helper; page and glossary for the balance sheet | R1 to R15, R22, R24 | #35 | T1 to T19 |
| PR 2: income statement | Layers of B.2; write-downs and the `operating` option; A5 and B14; non-recurring and investing adjustments; `lease` (both statements); reclassified income statement; reconciliations | R16 to R21 | #TBD (income statement) | T20 to T31 |
| PR 3: years, changes and independent sources | Changes between years; mixed forms; published and practitioner golden cases; concept page; final audit and review | R23; AC3, AC6, AC14 | #36 | T32 to T41 |

**Behaviour between pull requests (unreleased).** After PR 1 the income statement inputs are
parsed, validated and checked (R3, R7), and the output has no income statement block; a
convention states that the reorganisation of the income statement is not implemented in this
build. An adjustment of an income statement kind (`investing`, `non_recurring`, `lease`) is
refused with `FM-E001` stating the same, never ignored. PR 2 removes both. Both states exist only
on main between merges and never in a release.

## Risks and alternatives
- **Size of the change.** Splitting by statement rather than by layer keeps each pull request
  near one area of the spec. Alternative rejected: one pull request (too large to review);
  registering the method only in PR 3 (no golden cases through the registry until the end, and
  the documentation coverage check could not run on PR 1 and PR 2).
- **Golden files across pull requests.** Pointers into a single `not_computed` list would shift
  when PR 2 adds outputs, and golden files cannot be edited. Per-block lists and the rule
  "written once, where every asserted output exists" remove the risk.
- **Schema snapshot across pull requests.** The input schema is completed in PR 1, so the hash
  in `schemas/index.json` does not change in PR 2 and PR 3; T27 and T37 check it.
- **Building blocks on main.** `MeasurementBasis`, the debt-like item type and
  `apply_defaults` are taken after the rebase; if their shape differs from what this plan
  assumes, the plan is updated before T9.
- **Exact checks on filed statements.** Statutory statements in euro units carry a rounding
  reserve in equity (art. 2423, V1); with it given as part of A.VI the checks reconcile exactly.
  A filing that does not balance is reported as `FM-E026`, by design (Decision 2).
- **Defaults in `apply_defaults`.** Mapping defaults depend on the parsed input; if the
  signature of `Method::apply_defaults` on main cannot express a default that depends on other
  fields, the plan is updated before T9, keeping one constant table for Appendix A.

## Decisions on the plan (maintainer, 2026-11-08)
1. Mapping defaults of Appendix A are listed in `meta.defaults_applied` with pointer,
   destination and reason, besides `assignments` and conventions (R8 clarified).
2. Month-end closings one year apart are consecutive, including 28 February 2023 followed by
   29 February 2024 (R2 clarified; AC6 `end-of-month-dates.yaml`).
3. `FM-E026` is free on main and kept.
4. Roadmap: the coordinator adds the task for PR 2 and retitles #35 and #36; `tasks.md` refers
   to the new task as "#TBD (income statement)" until it is replaced.
