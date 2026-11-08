# SPEC-0001 — Tasks

Status: active (SPEC-0001 approved on 2026-11-08). This list covers issue #110 (shared engine,
`dcf.fcff` version 3 with stages); the other issues of the spec (#38, #165, #166, #43, #44)
add their own sections.

Rules: test first; one task per commit; `[P]` tasks touch disjoint files and can run in
parallel; tick a task only when `just check` passes.

## Issue #110 — Parametric stages on the shared DCF engine

### Tests (write first, they must fail)
- [ ] T1 [P] Independent reference `scripts/reference_dcf.py` (50 digits, local context) and
  `python/tests/test_reference_dcf.py` [R6 to R13, R21 to R23; AC4, AC5, AC7, AC12]
- [ ] T2 [P] Golden cases `tests/golden/dcf.fcff/`: three-stage, two-stage from a base flow,
  mid-period terminal, bridge, Damodaran Illustration 15.3 (Amgen) at published and reference
  precision, errors [R6 to R15, R19 to R23; AC4, AC6, AC7, AC8, AC12]
- [ ] T3 [P] Cross-check against numpy-financial in `python/tests/test_cross_check.py`
  (agreement of conventions only)

### Implementation
- [ ] T4 Error variants `FM-E024`, `FM-E025` and pointer-carrying variants of `FM-E003` and
  `FM-E004`, with `docs/en/reference/errors.md` [R7, R8, R12, R13, R15, R27]
- [ ] T5 `dcf/engine.rs`: stages and projection, rate schedule, terms check, timing and
  terminal discounting time, discount factors through `tvm`, `tv.gordon` hookup, period table
  and explain [R6 to R15, R19 to R22, R27 to R29]
- [ ] T6 `dcf/fcff.rs`: version 3 input and output, value of operations, enterprise value and
  equity value with the net financial position and non-operating assets [R1, R22, R23, R25]
- [ ] T7 Registry at version 3, contract cases, `uv run just generate` (schema `v3.json`,
  index, methods reference)
- [ ] T8 Version 2 golden cases re-expressed in version 3 with identical expected values,
  version 2 files retired; analysis golden cases with `dcf.fcff` inner inputs re-expressed;
  commit with `Numerical-change: dcf.fcff v2 -> v3` [R30; AC15]

### Adapters and docs
- [ ] T9 [P] Python helper `python/fairmark/dcf.py` for version 3
- [ ] T10 [P] Methodology page `docs/en/methods/dcf-fcff.md`, examples in guides, concepts and
  analysis pages, README quickstart
- [ ] T11 [P] Release notes `changes/110.breaking.md`, `changes/110.feature.md`
- [ ] T12 Italian edition (`docs/it/`, `README.it.md`), stamped with `scripts/i18n.py`;
  roadmap #110 done; `uv run just check`

### Verification
- [ ] T13 `numerical-auditor` review, `code-reviewer` review, acceptance criteria of #110
  ticked in spec.md (maintainer)
