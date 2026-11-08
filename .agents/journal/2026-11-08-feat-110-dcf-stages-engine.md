# 2026-11-08 — feat/110-dcf-stages-engine

- Issue: #110 Parametric stages on the shared DCF engine (dcf.fcff v3)
- Pull request: none (local commits; the coordinator opens it)
- Spec: SPEC-0001 R1, R6 to R15, R19 to R23, R25 to R30

## Done
- 43aa0fe plan.md and tasks.md for #110, with the extension points for #38, #165, #166, #43,
  #44 and #167.
- 8cc4d25 tests first: `scripts/reference_dcf.py` (50 digits), golden cases in
  `tests/golden/dcf.fcff/` including Damodaran Illustration 15.3 (Amgen), property tests and
  a numpy-financial cross-check.
- a734111 shared engine `dcf/engine.rs` and `dcf.fcff` version 3 `dcf/fcff.rs`; FM-E024,
  FM-E025; schema v3.
- a45724c migration of the version 2 golden cases (`Numerical-change: dcf.fcff v2 -> v3`),
  including the analysis cases whose inner method is `dcf.fcff`.
- dda00b7, 914a416 methodology page and examples, English and Italian.
- a6b59f1, ccc6317, 7bf6382, ffd3482, 6353784 findings of the code review and the numerical
  audit: present values by division by the compounding factor (golden X12, X13), warnings,
  R26, schema bounds of `terminal`, explain substitutions, docs, plan.

## State at the end of the session
- `uv run just check` passes locally.
- `scripts/check_golden_changes.py` requires the `numerical-change` label: two version 2
  `dcf.fcff` files retired, seven analysis cases with re-expressed inputs.

## Decisions taken
- Choices are flat objects with an enumeration and optional fields, not tagged unions, so that
  new options stay additive under `scripts/schema_snapshots.py` (plan.md).
- Terms of flows and rates are required in version 3 now and a mismatch is refused (FM-E024),
  so that #166 adds only optional fields.
- Present values divide by the compounding factor inside the DCF engine only;
  `tvm::period_discount_factors` is unchanged (coordinator, separate issue).

## Open questions for the maintainer
- Approval of the re-expressed analysis golden cases (expected numbers unchanged; the tornado
  ranking names its variables by the new pointers).
- Acceptance of the Amgen published-precision tolerances (2, and 20 for the terminal value).
- Whether the R11 discontinuity warning should use the growth implied by a given last stage.
- Whether the flat-object schema shape deserves an ADR.

## Next steps
1. Maintainer review of the migration commit a45724c with the `numerical-change` label, then
   the pull request `feat(dcf)!: shared engine with stages for dcf.fcff v3` closing #110.
