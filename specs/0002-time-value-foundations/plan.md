# SPEC-0002 — Plan

## Design
- **Modules.** `dates.rs` (calendar `Date`, `DayCount`, `dates.year_fraction`), `rates.rs`
  (`Compounding`, growth and discount factors and their derivative, equivalent rates,
  `rates.convert`, `rates.real_nominal`), `tvm.rs` (solver, `tvm.present_value`, dated flows,
  `tvm.xnpv`, `tvm.xirr`, `tvm.mirr`, `tvm.annuity`, `tvm.perpetuity`, `tvm.irr` v2).
- **Date type.** Implemented in the core (ADR 0008): strict ISO 8601 parsing, range
  1900–2199, day number by the civil-from-days algorithm, exhaustively tested over the range.
  Serde failures on dates carry a marker that the registry maps to `FM-E009`.
- **Solver.** One function for IRR and XIRR: grid scan, uniqueness check (`FM-E011`), Newton
  inside the root interval, bisection fallback. Grid points that overflow are skipped, so long
  series do not fail at rates close to -100%.
- **Time-varying rates.** `period_discount_factors` is the building block that DCF (SPEC-0001)
  will reuse: cumulative product of period factors, with mid-period as half a period at the
  period rate (square root, more accurate than a fractional power).
- **Defaults.** Only the ECMA-376 defaults for XNPV and XIRR (Actual/365 Fixed, annual
  compounding, earliest flow as valuation date, guess 0.10) and zero growth for annuities and
  perpetuities; all documented in the schema and reported in `explain.conventions`.

## Schema changes
Additive: nine new method schemas. `tvm.irr` schema unchanged; method version 2 because inputs
with several IRRs now return `FM-E011` instead of one of the roots.

## Risks
- Transcendental functions of `rust_decimal` are approximations: measured by the numerical audit
  and stated on the methodology pages.
- Two roots closer than the grid spacing are not distinguished: documented.

## Verification
Golden cases from published sources (ECMA-376, ISDA) plus a 50-digit reference
(`scripts/reference_time.py`); property tests; an independent audit by the
`numerical-auditor` with its own recomputation; a code review by the `code-reviewer`
(issue #176).

## Follow-ups
- **`tvm.annuity` version 2.** Version 1 always computes the future value $PV (1 + r)^n$,
  which leaves the decimal range at long horizons (a payment of 100 at 5% fails with
  `FM-E008` above 1,208 periods), and the present value is then lost with it. A version 2
  could return the present value and report the future value as unavailable, or drop it.
  This changes the outputs, so it needs a new method version and the maintainer's approval;
  out of scope of issue #176, documented on the methodology page.
