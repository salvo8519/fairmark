# SPEC-0006 — Terminal value: a family of explicit methods

| | |
| --- | --- |
| Status | Approved — 2026-11-07, maintainer decisions recorded below |
| Milestone | M1 |
| Owner | @salvo8519 |
| Formula basis | The three families used in practice (stable growth, multiples, liquidation) and the Italian finite-horizon and normalised-income forms; value driver tied to reinvestment and return on new capital; verified on published worked examples and closed forms |
| Related | SPEC-0001 (DCF consumes these methods), SPEC-0002 (time model), SPEC-0003 (rates), SPEC-0004 (income-based methods) |

## Problem
The terminal value is often 60 to 80 percent of a DCF value, and it is where most valuation
disputes are decided. Spreadsheets hide its assumptions in one cell. Practice offers several
techniques that rest on different economic assumptions: perpetual growth, growth tied to the
return on new investment, market multiples, liquidation, and the Italian forms with a finite
horizon or a normalised income. An analyst must be able to choose one, see its assumptions, and
compare it with the others on the same inputs.

## Scope
**In scope:** one method per technique, each returning the terminal value at the horizon N
(not discounted) with its implied assumptions; a comparison method that runs several techniques
on consistent inputs; cross-checks that make hidden assumptions visible (implied growth of an
exit multiple, reinvestment rate implied by a growth rate, growth cap).
**Out of scope:** discounting to the valuation date (done by SPEC-0001 with the time model of
SPEC-0002); estimating the inputs (normalised flows, multiples from comparables).

## Taxonomy

| Family | Method id | Economic assumption | Typical use |
| --- | --- | --- | --- |
| Stable growth | `tv.gordon` | Cash flow grows at g forever, g < r | Mature going concern |
| Stable growth, value driver | `tv.value_driver` | Growth is earned by reinvesting g / RONIC of NOPAT | Going concern; makes value creation explicit |
| Declining growth | `tv.h_model` | Growth declines linearly from g_s to g_l over 2H periods | Transition already under way at N |
| Market | `tv.exit_multiple` | The business is sold at a multiple of a metric at N | Transactions, private equity; a relative check |
| Finite horizon (Italian practice) | `tv.limited_horizon` | The flow lasts n more periods, then a residual value | Limited competitive advantage, concessions, licences |
| Normalised income (Italian practice) | `tv.normalised_income` | A normalised income is capitalised, with or without growth | Income-based reasoning at the horizon |
| Liquidation | `tv.liquidation` | Assets are realised and liabilities settled at N | Ventures without continuity, separable assets |
| Comparison | `tv.compare` | Runs several techniques on the same inputs | Sensitivity of the conclusion to the technique |

## Requirements (EARS)
- **R1** — `tv.gordon`: WHEN given r, g < r and either the next flow CF_{N+1} or the last flow
  CF_N, THE ENGINE SHALL return TV_N = CF_{N+1} / (r − g), with CF_{N+1} = CF_N × (1 + g) when
  the last flow is given; the choice SHALL be reported.
- **R2** — `tv.value_driver`: WHEN given NOPAT_{N+1}, g, RONIC and r, THE ENGINE SHALL return
  TV_N = NOPAT_{N+1} × (1 − g / RONIC) / (r − g) and report the reinvestment rate g / RONIC;
  WHEN RONIC equals r it SHALL report that growth creates no value (TV_N = NOPAT_{N+1} / r).
- **R3** — `tv.h_model`: WHEN given CF_N, short-term growth g_s, long-term growth g_l < r and the
  half-life H, THE ENGINE SHALL return TV_N = CF_N × [(1 + g_l) + H × (g_s − g_l)] / (r − g_l)
  and state that it is an approximation of linearly declining growth.
- **R4** — `tv.exit_multiple`: WHEN given a multiple and the metric it applies to (EBITDA, EBIT,
  revenue, earnings, book value), THE ENGINE SHALL return TV_N = multiple × metric_N; WHEN also
  given r and the next flow, it SHALL report the implied perpetual growth g = r − CF_{N+1} / TV_N.
- **R5** — `tv.limited_horizon`: WHEN given a flow, r, a number of periods n (and optionally a
  growth and a residual value at N + n), THE ENGINE SHALL return the present value at N of the
  n flows plus the discounted residual value, reusing `tvm.annuity`.
- **R6** — `tv.normalised_income`: WHEN given a normalised income and a capitalisation rate (and
  optionally a growth), THE ENGINE SHALL return R / i or R / (i − g).
- **R7** — `tv.liquidation`: WHEN given labelled realisable values of assets, labelled
  liabilities, liquidation costs and the tax on gains, THE ENGINE SHALL return the net
  liquidation value with every line in `explain`.
- **R8** — WHEN given a growth cap (for example the risk-free rate or the nominal growth of the
  economy), every growth-based method SHALL report whether g respects it; exceeding the cap SHALL
  be reported, not silently accepted.
- **R9** — `tv.compare` SHALL run the requested techniques on one consistent set of inputs and
  return each terminal value with its implied assumptions side by side.
- **R10** — IF g ≥ r, RONIC ≤ 0, a multiple or a metric is negative where it makes no sense, or
  n < 1, THEN THE ENGINE SHALL return a typed error.

## Conventions and parameters
| Convention | Options | Default |
| --- | --- | --- |
| Flow at N + 1 | given (normalised) or CF_N × (1 + g) | required choice |
| Discount rate in the terminal phase | given, may differ from the explicit phase | required |
| Growth cap | value or none | none (reported as absent) |
| Multiple metric | ebitda, ebit, revenue, earnings, book_value | required |

## Acceptance criteria
- **AC1** (R1, R2) — Published worked examples (Damodaran's public lecture material on stable
  growth and on reinvestment g / ROC) match at their published precision.
- **AC2** (R2) — Property: with RONIC = r, TV does not change with g.
- **AC3** (R3) — With g_s = g_l the H-model equals the Gordon model (property).
- **AC4** (R4) — Implied growth of an exit multiple fed back into `tv.gordon` returns the same TV.
- **AC5** (R5, R6) — Limited horizon with n large converges to the perpetuity; closed forms in
  the golden files.
- **AC6** (R7, R8, R10) — Liquidation example and every invalid input return the documented
  value or error.

## Interfaces
- Method ids: as in the taxonomy. SPEC-0001 `dcf.value` takes a terminal specification that
  names one of these methods and its inputs; the DCF discounts the result.

## Decisions (maintainer, 2026-11-07)
- [x] Finite horizon: the constant flow for n periods first, with n an explicit input; the
      flow declining to zero and the goodwill decay follow as further options.
- [x] Exit multiples: EV/EBITDA, EV/EBIT and EV/Sales on an enterprise basis; P/E is accepted
      only on an equity basis, never mixed with enterprise measures.
- [x] Stage lengths, growth and inflation in multi-stage models are tracked separately in the
      issue on parametric stages (explicit periods per stage, optional growth and inflation
      switches, nominal and real conversions through `rates.real_nominal`).
