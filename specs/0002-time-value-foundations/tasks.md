# SPEC-0002 — Tasks

- [x] T1 Reference implementation `scripts/reference_time.py` and golden cases with sources (AC1–AC10)
- [x] T2 Golden runners: `source`, `expect_error`, conventions present (AC11); guard requires a source on new cases
- [x] T3 `dates.rs`: `Date`, `DayCount`, `dates.year_fraction`, error `FM-E009` (R1, R2)
- [x] T4 `rates.rs`: compounding, discount factors, `rates.convert`, `rates.real_nominal` (R3, R4)
- [x] T5 `tvm.rs`: solver with uniqueness check, `tvm.irr` v2, error `FM-E011` (R9)
- [x] T6 `tvm.present_value` with constant or per-period rates and three timings (R5)
- [x] T7 `tvm.xnpv`, `tvm.xirr`, error `FM-E010` (R6, R7, R8)
- [x] T8 `tvm.mirr`, `tvm.annuity`, `tvm.perpetuity` (R10, R11, R12)
- [x] T9 Registry, schemas, generated reference; Python helpers `dates`, `rates`, `tvm`
- [x] T10 Portable tool schema handles optional values
- [ ] T11 Methodology pages, methods index, error catalogue (docs-writer)
- [ ] T12 Italian edition (translator)
- [x] T13 Independent numerical audit (numerical-auditor) and fixes: reciprocal discount factors,
      parity uniqueness rule with `uniqueness_verified`, effective-rate grid for XIRR, stable
      near-equal annuities, perpetuity growth bound, validation order, defaults in conventions
- [x] T14 Cross-check tests against pyxirr and numpy-financial (`oracles` group, CI matrix)
