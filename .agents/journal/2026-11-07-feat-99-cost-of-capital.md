# 2026-11-07 — feat/99-cost-of-capital

- Issue: #99 Cost of capital: CAPM, beta relevering, peer betas, cost of debt and WACC
- Pull request: opened at the end of this session
- Spec: SPEC-0003 R1–R7 (approved by the maintainer on 2026-11-07)

## Done
- Merged #96 (time value foundations) and #98 (draft specs) after a combined-state check.
- coc.rs with five methods; closed-form golden cases; property tests; Python helpers.

## State at the end of the session
- #94 (conventions) waits only for the Dependency graph setting.

## Decisions taken
- Maintainer: tax rate of the shield is an explicit input; specific premium needs a justification.
- Maintainer: target product score 5; merge only after a deep mergeability and stability check.

## Next steps
1. SPEC-0001 multi-stage DCF on top of SPEC-0002 and SPEC-0003.
