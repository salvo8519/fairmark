# SPEC-0001 — Tasks

Status: active (SPEC-0001 approved on 2026-11-08). This list covers issue #110 (shared engine,
`dcf.fcff` version 3 with stages); the other issues of the spec (#38, #165, #166, #43, #44)
add their own sections.

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes.

## Issue #110 — Parametric stages on the shared DCF engine

### Tests (write first, they must fail)
- [x] T1 [P] Independent reference `scripts/reference_dcf.py` (50 digits, local context) and
  `python/tests/test_reference_dcf.py` [R6 to R13, R21 to R23; AC4, AC5, AC7, AC12]
- [x] T2 [P] Golden cases `tests/golden/dcf.fcff/`: three-stage, two-stage from a base flow,
  mid-period terminal, bridge, Damodaran Illustration 15.3 (Amgen) at published and reference
  precision, errors [R6 to R15, R19 to R23; AC4, AC6, AC7, AC8, AC12]
- [x] T3 [P] Cross-check against numpy-financial in `python/tests/test_cross_check.py`
  (agreement of conventions only)

### Implementation
- [x] T4 Error variants `FM-E024`, `FM-E025` and pointer-carrying variants of `FM-E003` and
  `FM-E004`, with `docs/en/reference/errors.md` [R7, R8, R12, R13, R15, R27]
- [x] T5 `dcf/engine.rs`: stages and projection, rate schedule, terms check, timing and
  terminal discounting time, discount factors through `tvm`, `tv.gordon` hookup, period table
  and explain [R6 to R15, R19 to R22, R27 to R29]
- [x] T6 `dcf/fcff.rs`: version 3 input and output, value of operations, enterprise value and
  equity value with the net financial position and non-operating assets [R1, R22, R23, R25]
- [x] T7 Registry at version 3, contract cases, `uv run just generate` (schema `v3.json`,
  index, methods reference)
- [x] T8 Version 2 golden cases re-expressed in version 3 with identical expected values,
  version 2 files retired; analysis golden cases with `dcf.fcff` inner inputs re-expressed;
  commit with `Numerical-change: dcf.fcff v2 -> v3` [R30; AC15]. Analysis cases approved by
  the maintainer on 2026-11-08 and recorded in spec.md

### Adapters and docs
- [x] T9 [P] Python helper `python/fairmark/dcf.py` for version 3
- [x] T10 [P] Methodology page `docs/en/methods/dcf-fcff.md`, examples in guides, concepts and
  analysis pages, README quickstart
- [x] T11 [P] Release notes `changes/110.breaking.md`, `changes/110.feature.md`
- [x] T12 Italian edition (`docs/it/`, `README.it.md`), stamped with `scripts/i18n.py`;
  roadmap #110 done; `uv run just check`

### Verification
- [x] T14 Findings of the code review and the numerical audit applied: present values by
  division at long horizons (golden cases X12, X13), mid-period Gordon warning, R26 warning,
  schema bounds of `terminal`, explain substitutions, docs; open question on R11 in plan.md
- [ ] T13 `numerical-auditor` review, `code-reviewer` review, acceptance criteria of #110
  ticked in spec.md (maintainer)

## Issue #38: free cash flow build-up (`dcf.free_cash_flows`)

### Tests (write first, they must fail)
- [x] F1 Reference functions `scripts/reference_free_cash_flows.py` (50 digits, local
  context): taxes on EBIT in both forms, negative EBIT under `credit` and `zero`, FCFF, FCFE
  [R2, R3, R4]
- [x] F2 [P] Golden cases `tests/golden/dcf.free_cash_flows/`: Damodaran Illustration 15.7
  (Boeing) with the recorded discrepancy, multi-period FCFF and FCFE in both tax forms,
  negative EBIT under each treatment, adjustments, errors [AC1, AC3]
- [x] F3 [P] Contract cases `tests/contracts/dcf.free_cash_flows.yaml` [R28]

### Implementation
- [x] F4 `src/dcf/free_cash_flows.rs`: input and output types, tax-form validation, build-up
  with explain and conventions, unit type `FreeCashFlows`; `build_up` with a `Location` for
  reuse by the shared engine; unit tests for AC2, AC14 (JSON pointers in the validation errors
  of the build-up, conventions) and every error [R2 to R5, R27 for validation errors, R28,
  R29]. Deserialisation errors (an unknown or missing field) come from the shared serde path
  of the registry and carry no JSON pointer: follow-up issue for `serde_path_to_error`
- [x] F5 Registry line `dcf::FreeCashFlows`, `uv run just generate` [R1]

### Adapters and docs
- [x] F6 [P] Python helper `fm.dcf.free_cash_flows`; seeded comparison with the reference and
  the AC2 property [AC1, AC2]
- [x] F7 [P] Methodology page `dcf-free-cash-flows.md`, nav, index (preview); Italian edition;
  release note `changes/38.feature.md`
- [x] F8 Roadmap: issue #38 done

- [x] F10 Amendment 14 (maintainer, 2026-11-08): no interest tax shield in a period where the
  `zero` treatment applies; reference, golden cases (net-income route, positive EBIT below the
  interest), unit and property tests, conventions, docs in English and Italian; R29
  interpretation of the presence-based tax form recorded in the spec [R3, R4, R29]

### Verification
- [ ] F9 `uv run just check` green (done); `numerical-auditor` and `code-reviewer` reviews
  (pending, to be run by the orchestrating session); AC1, AC2 and AC3 ticked in spec.md (by
  the maintainer: spec.md is not changed on this branch)
