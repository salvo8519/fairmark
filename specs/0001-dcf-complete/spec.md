# SPEC-0001 — Discounted cash flow: one-, two- and three-stage models

| | |
| --- | --- |
| Status | Draft — proposed for approval |
| Milestone | M1 |
| Owner | @salvo8519 |
| Formula basis | Commonly adopted one-, two- and three-stage DCF, Gordon growth, value driver and exit multiple terminal values; cross-checked against widely used models and published worked examples |
| Related | ADR 0008; SPEC-0002, SPEC-0003; statements reclassification moved to SPEC-0005 |

## Problem
Discounted cash flow is the reference method for going concerns. Analysts choose the number of
stages, the terminal value formula and the timing conventions, and each choice changes the
value materially. The engine must support the standard model families with every convention
explicit, and reuse the time model of SPEC-0002 so that rates can vary over time and flows can
be dated.

## Scope
**In scope:** a single engine for three cash-flow bases (FCFF at WACC, FCFE at the cost of
equity, dividends at the cost of equity); one-stage (Gordon), two-stage and three-stage models;
transition stage with linear fade of growth (and of the discount rate, optionally); terminal
value by Gordon growth, by value driver (growth and return on new invested capital) or by exit
multiple; end or mid-period timing; stub first period from a valuation date; equity bridge;
sensitivity grid.
**Out of scope:** building FCFF from statements (SPEC-0005); probability-weighted scenarios
(later); real options.

## Requirements (EARS)
- **R1** — WHEN given a base cash flow, a discount rate r and a growth g < r, THE ENGINE SHALL
  return the one-stage value V0 = CF1 / (r − g), with CF1 either given or computed as CF0 × (1 + g)
  as declared by the input.
- **R2** — WHEN given explicit forecasts for periods 1..n and a terminal specification, THE
  ENGINE SHALL return the two-stage value: the present value of the forecasts plus the present
  value of the terminal value at n.
- **R3** — WHEN additionally given a transition stage of m periods with growth fading linearly
  from g_high to g_stable, THE ENGINE SHALL project each transition flow from the last explicit
  flow and return the three-stage value, with every projected flow and growth rate in `explain`.
- **R4** — THE ENGINE SHALL support terminal values by (a) Gordon growth on the next flow,
  (b) value driver NOPAT_{n+1} × (1 − g / RONIC) / (r − g), (c) exit multiple × metric; the
  choice SHALL be an explicit input.
- **R5** — Discount rates SHALL be either one rate or one rate per period (SPEC-0002 R5); the
  terminal rate SHALL be explicit when the rates vary.
- **R6** — Timing SHALL be explicit (end or mid-period); the discounting time of the terminal
  value under mid-period timing SHALL be an explicit input (n or n − 0.5).
- **R7** — WHEN given a valuation date and dated period ends, THE ENGINE SHALL discount on the
  time axis of SPEC-0002 and pro-rate a stub first period, reporting the stub fraction.
- **R8** — WHEN the basis is FCFF, THE ENGINE SHALL bridge enterprise value to equity value with
  explicit items: net financial position, debt-like items, minorities, non-operating assets.
- **R9** — WHEN requested, THE ENGINE SHALL return an r × g sensitivity grid in one call, each
  cell identical to a single run with those parameters.
- **R10** — IF g ≥ r in any Gordon-type formula, RONIC ≤ 0, or a stage is inconsistent, THEN
  THE ENGINE SHALL return a typed error; it SHALL report the share of value in the terminal
  value in every result.

## Conventions and parameters
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Basis | fcff, fcfe, dividends | required | Damodaran |
| Timing | end_of_period, mid_period | required | practice varies |
| Terminal discounting under mid-period | n, n − 0.5 | required | practice varies |
| Terminal value | gordon, value_driver, exit_multiple | required | Koller; practice |
| Fade in transition stage | linear growth; optional linear rate | linear growth | Damodaran |
| First terminal flow | CF_n × (1 + g) or given normalised flow | required | PIV (normalised flow) |

## Acceptance criteria
- **AC1** (R1) — Gordon model textbook example (source in the golden file) within 1e-12.
- **AC2** (R2, R4, R6) — Two-stage FCFF example with Gordon, value driver and exit multiple
  terminal values match the 50-digit reference.
- **AC3** (R3) — Three-stage example: projected transition flows and value match the reference.
- **AC4** (R5, R7) — With a time-varying rate and a stub period, discount factors equal those of
  `tvm.present_value` and `dates.year_fraction`.
- **AC5** (R9) — Every grid cell equals a direct run (property test).
- **AC6** (R10) — Each invalid input returns its documented error code.
- **AC7** — The existing `dcf.fcff` golden cases are reproduced by the new engine.

## Interfaces
- Method ids: `dcf.value` (one, two or three stages, any basis), `dcf.gordon` (one stage),
  `dcf.terminal_value`, `dcf.sensitivity`. `dcf.fcff` (version 2) stays available.

## Open questions
- [ ] Mid-period timing: default discounting of the terminal value at n or n − 0.5 when the
      analyst does not say? Proposal: required input, no default.
- [ ] Normalised terminal flow (PIV recommends normalising the last flow): accept an explicit
      normalised flow as a separate input. Confirm.
