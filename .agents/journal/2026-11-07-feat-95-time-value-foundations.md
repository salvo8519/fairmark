# 2026-11-07 — feat/95-time-value-foundations

- Issue: #95 Time value foundations: dates, day counts, rates, XNPV, XIRR, MIRR, annuities
- Pull request: opened at the end of this session
- Spec: SPEC-0002 R1–R13 (implemented); SPEC-0001, 0003, 0004, 0005 proposed as drafts

## Done
- Modules dates.rs, rates.rs and the extended tvm.rs; nine new methods; tvm.irr v2.
- Golden cases from ECMA-376 and ISDA plus a 50-digit reference; property tests.
- Independent numerical audit: eight findings, all fixed with regression tests.
- Cross-check tests against pyxirr and numpy-financial (1,500+ random cases).
- Methodology pages in English and Italian; ADR 0008.

## State at the end of the session
- tvm.npv and dcf.fcff bumped to method version 2: only last digits change, and only for
  negative rates (discount factor computed from the reciprocal of the base).

## Decisions taken
- Maintainer: documentation stays light; formulas are the commonly adopted ones, defended by
  golden cases and cross-checks against widely used implementations, not by citations.
- Maintainer: small sessions, one pull request each; Claude may merge when review and CI are clean.
- IRR uniqueness: Descartes parity rule; ambiguous inputs are refused, unproven uniqueness is flagged.

## Open questions for the maintainer
- Approve or amend the drafts SPEC-0003 (cost of capital), SPEC-0001 (multi-stage DCF),
  SPEC-0004 (Italian methods); answer their open questions.

## Next steps
1. SPEC-0003 cost of capital once approved, with the same test-first and audit loop.
