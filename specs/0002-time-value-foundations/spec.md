# SPEC-0002 — Time value foundations: dates, day counts, rates and discounting

| | |
| --- | --- |
| Status | Approved (implementing) — approved by maintainer instruction of 2026-11-07 (published market standards only) |
| Milestone | M1 |
| Owner | @salvo8519 |
| Formula basis | Market definitions of day counts (ISDA 2006 §4.16) and spreadsheet functions (ECMA-376: XNPV, XIRR, MIRR); closed forms for annuities and rate conversion; cross-checked against widely used implementations |
| Related | ADR 0002, ADR 0008; SPEC-0001, SPEC-0003, SPEC-0004 build on it |

## Problem
Every valuation method discounts flows in time. Analysts need dated flows (closings,
distributions), periodic plans, rates that change over time and explicit conventions. Today
the engine handles only periodic flows at a single rate. Errors in day counts or compounding
are silent in spreadsheets and invisible when an LLM computes them.

## Scope
**In scope:** calendar dates; year fractions under ISDA day counts; compounding conventions and
rate conversion (including real and nominal rates); present value of periodic flows with a
constant or time-varying rate and end, mid or beginning-of-period timing; XNPV and XIRR for
dated flows; MIRR; level and growing annuities and perpetuities; uniqueness check for IRR-type
solutions.

**Out of scope:** business-day calendars and date rolling; coupon schedules; Actual/Actual
(ICMA) which needs a coupon schedule; yield curves from market instruments (bootstrapping).

## User stories
- As an analyst, I want to discount dated flows from a valuation date with the day count I
  choose, so that a mid-year closing is valued correctly.
- As an analyst, I want a different discount rate for each year, so that the cost of capital
  can change over the plan.
- As an analyst, I want XNPV, XIRR and MIRR that match the spreadsheet standard, so that I can
  reconcile with existing models.
- As an AI agent, I want the engine to refuse an IRR when several rates are valid, so that I
  never report an arbitrary one.

## Requirements (EARS)
- **R1** — THE ENGINE SHALL accept calendar dates as ISO 8601 `YYYY-MM-DD` strings from
  1900-01-01 to 2199-12-31; IF a date is malformed, impossible or out of range, THEN it SHALL
  return `FM-E009`.
- **R2** — WHEN given two dates and a day count (Actual/365 Fixed, Actual/360, Actual/Actual
  ISDA, 30/360 Bond Basis, 30E/360), THE ENGINE SHALL return the year fraction as defined in the
  2006 ISDA Definitions §4.16, with the day numerator and denominator in `explain`.
- **R3** — WHEN given a rate and its compounding (annual, semi-annual, quarterly, monthly,
  continuous) and a target compounding, THE ENGINE SHALL return the equivalent rate (same growth
  over one year).
- **R4** — WHEN given a nominal rate and an inflation rate, THE ENGINE SHALL return the real rate
  by the exact Fisher relation (1 + n) = (1 + r)(1 + π), and the inverse.
- **R5** — WHEN given periodic cash flows for periods 1..n, either one rate or one rate per
  period, and a timing (end, mid, beginning of period), THE ENGINE SHALL return each discount
  factor, each present value and their sum, where the discount factor of period k is the product
  of (1 + r_i)^-1 for i ≤ k, adjusted by half a period (mid) or one period (beginning) of r_k.
- **R6** — WHEN given dated cash flows, a rate, a day count and a compounding, THE ENGINE SHALL
  return XNPV at the valuation date; IF no valuation date is given, THEN the earliest flow date
  SHALL be used and reported (ECMA-376 convention).
- **R7** — IF a dated cash flow precedes the valuation date, THEN THE ENGINE SHALL return
  `FM-E010`.
- **R8** — WHEN asked for XIRR, THE ENGINE SHALL find the annual rate at which XNPV is zero with
  the same deterministic solver as IRR, and report solver, iterations and tolerance.
- **R9** — WHEN asked for IRR or XIRR, THE ENGINE SHALL scan a fixed, documented grid of
  effective rates in [-0.99, 10] and compare the roots found with the sign changes of the flows
  (Descartes' rule of signs); IF more than one root is found, or the parity shows that another
  rate exists, or no root is isolated while the flows change sign more than once, THEN it SHALL
  return `FM-E011` and never pick one; otherwise it SHALL report whether uniqueness is proven
  (`uniqueness_verified`).
- **R10** — WHEN given periodic flows, a finance rate and a reinvestment rate, THE ENGINE SHALL
  return MIRR as defined in ECMA-376; IF the flows have no positive or no negative value, THEN
  it SHALL return `FM-E006`.
- **R11** — WHEN given a payment, a rate, a number of periods, an optional growth rate and a
  timing (ordinary or due), THE ENGINE SHALL return the present value of the annuity and its
  factor, including the limit case rate = growth.
- **R12** — WHEN given a first payment, a rate, an optional growth and a timing, THE ENGINE SHALL
  return the present value of the perpetuity; IF growth ≥ rate, THEN it SHALL return `FM-E004`.
- **R13** — Every result SHALL list in `explain.conventions` the day count, compounding, timing
  and any default applied.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Day count (dated flows) | act_365_fixed, act_360, act_act_isda, thirty_360, thirty_e_360 | act_365_fixed for XNPV and XIRR; required elsewhere | ECMA-376; ISDA 2006 §4.16 |
| Compounding | annual, semi_annual, quarterly, monthly, continuous | annual for XNPV and XIRR | ECMA-376 |
| Valuation date (XNPV) | any date ≤ first flow | earliest flow date | ECMA-376 |
| Periodic timing | end_of_period, mid_period, beginning_of_period | required | practice varies |
| Annuity timing | ordinary (end), due (beginning) | required | textbook |
| IRR grid | -0.99, -0.95, -0.90 to 1.00 step 0.05, 1.25 to 10 step 0.25 | fixed | ADR 0008 |

## Acceptance criteria
- **AC1** (R1) — Malformed, impossible (2023-02-29) and out-of-range dates return `FM-E009`.
- **AC2** (R2) — ISDA 1998 example (2003-11-01 to 2004-05-01, Actual/Actual ISDA) gives
  61/365 + 121/366; 30/360 and 30E/360 end-of-month cases match the §4.16 rules; Actual/365F and
  Actual/360 match the day count over a leap year.
- **AC3** (R3, R4) — Conversions round-trip within 1e-20 and match the 50-digit reference.
- **AC4** (R5) — With a constant rate, present values equal `tvm.npv` shifted by one period;
  with a curve, they match the reference; mid and beginning timings match the reference.
- **AC5** (R6) — ECMA-376 XNPV example (rate 9%, five flows 2008-01-01 to 2009-04-01) gives
  2086.65 (published to two decimals).
- **AC6** (R7) — A flow dated before the valuation date returns `FM-E010`.
- **AC7** (R8) — ECMA-376 XIRR example gives 0.373362535 (published to nine decimals).
- **AC8** (R9) — Flows -100, 230, -132 (roots at 10% and 20%) return `FM-E011` for IRR.
- **AC9** (R10) — ECMA-376 MIRR examples give 0.126094, -0.048045 and 0.134759.
- **AC10** (R11, R12) — Annuity and perpetuity values match the closed forms in the 50-digit
  reference, including rate = growth and the due timing; growth ≥ rate returns `FM-E004`.
- **AC11** (R13) — Every new method reports its conventions (unit test per method).

## Interfaces
- New method ids: `dates.year_fraction`, `rates.convert`, `rates.real_nominal`,
  `tvm.present_value`, `tvm.xnpv`, `tvm.xirr`, `tvm.mirr`, `tvm.annuity`, `tvm.perpetuity`.
- `tvm.irr`: method version 2 (uniqueness rule, output `uniqueness_verified`, error `FM-E011`).
- `tvm.npv` and `dcf.fcff`: method version 2. Discount factors are computed from the reciprocal
  of the base: full relative precision at negative rates, and very long horizons underflow to
  zero instead of failing with `FM-E008`. Golden values unchanged; only the last digits of
  results at negative rates change. Requires the maintainer's approval (numerical change).
- New error codes: `FM-E009` invalid date, `FM-E010` flow before valuation date, `FM-E011`
  several rates solve the equation, `FM-E012` internal error (a defect, never an input problem).

## Open questions
- None: every convention above is a published market standard or an explicit input.
