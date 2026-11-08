# SPEC-0001 — Discounted cash flow: free cash flow build-up, parametric stages, nominal and real terms, equity bridge

| | |
| --- | --- |
| Status | Approved — 2026-11-08, maintainer decisions recorded below |
| Milestone | M2 (the previous header said M1; the roadmap places this spec in M2) |
| Owner | @salvo8519 |
| Formula basis | Commonly adopted enterprise DCF (FCFF at the WACC) and equity DCF (FCFE at the cost of equity) with an explicit forecast period, a transition stage and a terminal value; exact Fisher relation for nominal and real rates; verified against an independent 50-digit reference and the published worked examples of Damodaran, *Investment Valuation*, 2nd edition, chapter 15, Illustrations 15.3 (Amgen) and 15.7 (Boeing) (Decision 12) |
| Related | ADR 0008, ADR 0009 (typed calls between methods), ADR 0010 (evaluation inside the core); SPEC-0002 (time model, `rates.real_nominal`), SPEC-0003 (`coc.*`), SPEC-0006 (`tv.*`), SPEC-0007 (net financial position), SPEC-0008 (`analysis.sensitivity`); statements reclassification in SPEC-0005 |

Changes in this revision are marked "Revised 2026-11-08" in each section heading. Requirements
are renumbered; the table in "Analysis" maps the previous numbers to the new ones. The
maintainer's decisions of 2026-11-08 are recorded in "Decisions (maintainer, 2026-11-08)";
references to "Decision N" point there.

## Analysis (Revised 2026-11-08)

### What is on main today
`dcf.fcff` is registered at **method version 2** (version 1 is the schema snapshot before the
negative-rate change of SPEC-0002; both are unreleased). It takes a list of FCFF for years 1..n,
one discount rate, one terminal growth, `timing` (default `end_of_period`), `net_debt`
(default 0) and `non_operating_assets` (default 0). It returns enterprise value, equity value,
the present values, the Gordon terminal value on FCFF_n × (1 + g), its present value and the
terminal value weight. The terminal value is always discounted at t = n, also under mid-period
timing. Its methodology page and the README quickstart use it; its status is Preview.

The building blocks it does not use yet are on main: `tv.*` (SPEC-0006, approved, all seven
techniques plus `tv.compare`, with basis checks `FM-E019` and growth-cap reporting),
`tvm.present_value` (one rate or one rate per period, end, mid or beginning of period),
`rates.real_nominal` (exact Fisher), `coc.*` (cost of equity, WACC), the net financial position
type of SPEC-0007 (`gross_debt`, `cash`, signed `other_items`) and `analysis.sensitivity`
(generic tables over any decimal input, SPEC-0008).

### Previous requirements against main

| Previous | Content | State on main | In this revision |
| --- | --- | --- | --- |
| R1 | One-stage Gordon value | Not in `dcf.*`; equals `tv.gordon` at N = 0 and `tvm.perpetuity` | Dropped from this spec (Decision 11) |
| R2 | Two-stage value | Partly: explicit flows plus Gordon only | R6, R7, R19 |
| R3 | Three-stage with linear decline of growth | Not implemented | R6 to R11 (#110) |
| R4 | Terminal value technique as explicit input | Gordon only, implicit | R19, R20 (#43, reusing `tv.*`) |
| R5 | One rate or one rate per period; explicit terminal rate | One rate only | R12, R13 (#110) |
| R6 | Explicit timing; terminal discounting time under mid-period | Timing with default; terminal at n fixed | R21 (#43) |
| R7 | Valuation date, dated periods, stub period | Not implemented; no issue | Deferred (Decision 11) |
| R8 | Equity bridge with explicit items | `net_debt` and `non_operating_assets` only | R23 to R26 (#44) |
| R9 | r × g sensitivity grid | Covered by `analysis.sensitivity` (#45, #116) | Dropped from this spec |
| R10 | Typed errors; terminal value weight in every result | g ≥ r error and weight exist | R22, R27, R28 |

### What each issue needs
- **#38 FCFF and FCFE build-up.** A method that turns per-period operating line items (EBIT,
  taxes on EBIT, depreciation and amortisation, capital expenditure, change in net working
  capital) into FCFF, and adds the financing items (interest after tax, net borrowing) for
  FCFE; the same lines accepted directly as the flows of an explicit stage. Statutory
  reclassification stays in SPEC-0005: this spec starts from the reclassified lines. Tax form:
  Decision 7; negative EBIT: Decision 8; new equity-basis method: Decision 1.
- **#110 Parametric stages.** An ordered list of stages with an explicit number of periods,
  each with a required projection choice (given flows, constant growth, linear decline);
  per-period discount rates; required declaration of nominal or real terms for flows and for
  rates, refusal of a mismatch without inflation, conversion of the rates by the exact Fisher
  relation when inflation is given. Decisions 2 to 5.
- **#44 EV to equity.** Value of operations, enterprise value and equity value, each with one
  meaning; the net financial position of SPEC-0007, labelled debt-like items, labelled
  non-operating assets and non-controlling interests, every line in `explain`; rules for the
  equity basis (Decision 9, terminology decision c).
- **#43 Exit-multiple terminal value; configurable terminal value timing.** See below.

### Is #43 already covered by SPEC-0006?
Partly. The exit-multiple computation is delivered: `tv.exit_multiple` (SPEC-0006 R4, issue
#115) returns TV_N = multiple × metric_N on a declared enterprise or equity basis, with the
implied perpetual growth and the growth cap. Two things are not covered, because SPEC-0006
places discounting out of its scope: (1) using an exit multiple, or any `tv.*` technique,
inside the DCF, and (2) the time at which the terminal value is discounted under mid-period
timing. Both are in this spec (R19 to R21). Recommendation (a roadmap change, not part of the
spec decisions): keep #43 open, reduce it to "Terminal value technique and discounting time in
the DCF" and link it to SPEC-0001 instead of SPEC-0006; or close it and fold R19 to R21 into
the #110 implementation. A leading exit
multiple (metric at N + 1) is not covered by `tv.exit_multiple` (trailing only) and is not
requested; it stays out of scope.

## Problem
Discounted cash flow is the reference method for going concerns. Analysts build free cash flows
from a business plan, split the forecast into stages of explicit length, choose how flows grow
in each stage, state whether the plan is in nominal or real terms, choose a terminal value and
its timing, and move from the value of operations to enterprise value and equity value. Each
choice changes the value materially, and the most frequent errors (a real plan discounted at a nominal rate, a flow list
shorter than the stated stage, a terminal value discounted one period too many, net debt
subtracted from an equity value) are silent in spreadsheets and in the output of an AI agent.
The engine must make every choice an explicit input, refuse inconsistent combinations with a
typed error that names the field and the fix, and reuse the building blocks already on main.

## Scope (Revised 2026-11-08)
**In scope:** free cash flow build-up from reclassified line items (FCFF; FCFE with financing
items); enterprise DCF (FCFF at the WACC) and equity DCF (FCFE at the cost of equity) on one
engine; one to three explicit stages of explicit length with given flows, constant growth or
linear decline of growth; one discount rate or one per period; declared nominal or real terms with
conversion of rates by the Fisher relation; terminal value by any `tv.*` technique, with a
basis check; end or mid-period timing and an explicit terminal discounting time; equity bridge.

**Out of scope:** reclassification of statutory statements (SPEC-0005); estimating inputs
(WACC: `coc.*`; terminal inputs: `tv.*`); sensitivity tables (`analysis.sensitivity`);
probability-weighted scenarios; real options; sub-annual periods.

**Deferred, input designed to accept them later as additive options:** valuation date with
dated period ends and a stub first period (previous R7); dividend discount basis; linear decline
of the discount rate (expressible today with per-period rates); inflating real line items to
nominal before taxes; a leading exit multiple.

## User stories
- As an analyst, I want to state that stage 1 has 5 periods of given flows and stage 2 has
  5 periods with growth declining in linear increments from 8% to 2%, so that the model
  matches my plan without hidden steps.
- As an analyst with a plan at constant prices, I want to declare it real and give expected
  inflation, so that my nominal WACC is converted instead of mixed.
- As an AI agent, I want a real flow discounted at a nominal rate to be refused with the field
  names and the fix, so that I correct my call instead of reporting a wrong value.
- As an AI agent, I want the per-period table, the terminal technique and every bridge line in
  the result, so that I can cite each number in the report.

## Requirements (EARS) (Revised 2026-11-08)

### Methods and basis
- **R1** — THE ENGINE SHALL provide an enterprise DCF (`dcf.fcff` version 3) that discounts
  FCFF at the WACC and returns value of operations, enterprise value and equity value, an
  equity DCF (`dcf.fcfe` version 1) that discounts FCFE at the cost of equity and returns
  equity value, and the build-up method `dcf.free_cash_flows` version 1, all computed by one
  shared engine; the basis SHALL be reported in `explain.conventions` (Decision 1).

### Free cash flow build-up (#38)
- **R2** — WHEN given, for each period, EBIT, depreciation and amortisation, capital
  expenditure, change in net working capital, optional labelled other adjustments and the taxes
  on EBIT, given as exactly one of a tax rate applied to EBIT or per-period tax amounts
  (Decision 7), THE ENGINE SHALL return FCFF_t = EBIT_t − taxes on EBIT_t +
  D&A_t − capex_t − ΔNWC_t + Σ adjustments_t, with every component and the substituted formula
  in `explain`.
- **R3** — WHEN additionally given, for each period, interest expense and net borrowing, THE
  ENGINE SHALL return FCFE_t = FCFF_t − interest expense_t × (1 − t) + net borrowing_t, with t
  the same tax rate as in R2, and SHALL report that the interest tax shield uses that rate.
  WHEN the taxes on EBIT are given as amounts, the tax rate t of the interest tax shield SHALL
  be a required separate input. WHEN the `zero` treatment of R4 applies in a period (EBIT
  negative, taxes from a rate), the interest tax shield of that period SHALL be zero
  (FCFE_t = FCFF_t − interest expense_t + net borrowing_t) and SHALL be reported (amendment 14).
- **R4** — WHEN EBIT is negative and the taxes are computed from a rate, THE ENGINE SHALL apply
  the required treatment chosen in the input, `credit` (taxes = rate × EBIT, a saving) or
  `zero` (no tax), with no default, and report it; it SHALL NOT model tax loss carry-forwards
  (Decision 8).
- **R5** — IF depreciation and amortisation, capital expenditure or interest expense is
  negative, THEN THE ENGINE SHALL return `FM-E001` naming the field and the period and stating
  the fix (disposals and other inflows go in the labelled adjustments with their sign). The
  sign of ΔNWC (positive for an increase, which absorbs cash), of net borrowing (positive for
  new borrowing) and of each adjustment (positive for an inflow) SHALL be stated in the schema.

### Stages (#110)
- **R6** — THE ENGINE SHALL take an ordered list of one to three explicit stages, each with a
  required number of periods (an integer of at least 1) and a required projection choice with
  no default; the explicit horizon N SHALL be the sum of the periods, and one period SHALL be
  one year (rates are annual effective, SPEC-0003 R7).
- **R7** — WHEN a stage's projection is `given`, THE ENGINE SHALL take its flows either as
  amounts or as build-up lines (R2, R3); IF the number of flows or lines differs from the
  stage's periods, THEN it SHALL return `FM-E025` naming the stage, the expected and the
  actual count.
- **R8** — WHEN a stage's projection is `constant_growth` with growth g, THE ENGINE SHALL
  project CF_t = CF_{t−1} × (1 + g), starting from the last flow of the previous stage or, for
  the first stage, from a required base flow CF_0; IF the base flow is missing for a generated
  first stage, or given when the first stage is `given`, THEN it SHALL return `FM-E025`.
- **R9** — WHEN a stage's projection is `linear_decline` over m periods from growth g_from to
  growth g_to, THE ENGINE SHALL apply g_k = g_from + (g_to − g_from) × k / m in period k = 1..m
  of the stage, so that g_to is reached in the last period (Decision 4), with the same starting
  flow rule as R8.
- **R10** — THE ENGINE SHALL return, for every period, the stage, the projection, the growth
  applied (when generated), the flow, the discount rate given, the rate used, the discount
  factor and the present value, and SHALL record each in `explain`.
- **R11** — IF a growth is −1 or lower, THEN THE ENGINE SHALL return `FM-E001`. WHEN growth is
  applied to a negative flow, or the growth of the last explicit period differs from the
  perpetual growth of the terminal technique, THE ENGINE SHALL report it in
  `explain.conventions` as a warning, not refuse it.

### Discount rates (#110)
- **R12** — THE ENGINE SHALL take either one discount rate for all explicit periods or one rate
  per period (a list of length N, otherwise `FM-E025`), and SHALL compute discount factors as in
  SPEC-0002 R5 through the `tvm` building block. WHEN one rate is given, the terminal technique
  SHALL use the same rate and the input SHALL NOT carry a separate terminal rate; WHEN rates
  per period are given, a terminal rate SHALL be required (`FM-E001` otherwise).
- **R13** — IF any discount rate, given or converted, is −1 or lower, THEN THE ENGINE SHALL
  return `FM-E003` naming the period.

### Nominal and real terms (#110)
- **R14** — THE ENGINE SHALL require the terms of the cash flows (`nominal` or `real`), which
  apply to every amount and every growth rate of the input, and the terms of the discount rates
  (`nominal` or `real`), with no default; both SHALL be reported in `explain.conventions`.
- **R15** — IF the terms of the cash flows and of the discount rates differ and no inflation is
  given, THEN THE ENGINE SHALL return `FM-E024` naming both fields and stating the two fixes
  (give expected inflation, or restate the rates or the flows in the same terms).
- **R16** — WHEN the terms differ and expected inflation is given (one rate, or one per period
  of length N with a required terminal inflation), THE ENGINE SHALL convert every discount rate,
  including the terminal rate, to the terms of the cash flows by the exact Fisher relation
  (1 + nominal) = (1 + real) × (1 + inflation), through the `rates.real_nominal` building block,
  report each converted rate, and SHALL NOT convert any cash flow or growth rate.
- **R17** — WHEN expected inflation is given, THE ENGINE SHALL report the perpetual growth of
  the terminal technique in the other terms (real growth implied by a nominal growth, nominal
  growth implied by a real growth) and, for nominal flows, SHALL report as a warning a perpetual
  growth below expected inflation (a real decline).
- **R18** — WHEN the terms are equal and inflation is given, THE ENGINE SHALL use it only for
  R17 and report that no conversion was made.

### Terminal value (#43, #110)
- **R19** — THE ENGINE SHALL require exactly one terminal technique named by its `tv.*` method
  id with its terms, compute it through the typed `tv` function (ADR 0009) at the terminal rate
  in the terms of the cash flows, and record the technique's method version. IF the technique's
  basis differs from the basis of the DCF (enterprise for FCFF, equity for FCFE), THEN it SHALL
  return `FM-E019`.
- **R20** — WHEN the technique uses the last explicit flow (`tv.gordon`, `tv.h_model`), THE
  ENGINE SHALL require an explicit choice of the first terminal flow: for `tv.gordon`, the last
  explicit flow grown by the perpetual growth, or a given normalised flow at N + 1; for
  `tv.h_model`, the last explicit flow, or a given normalised base flow at N that replaces it,
  with the standard formula and no conversion from N + 1 (Decision 15); amounts in the terminal
  terms SHALL be in the terms of the cash flows. (Amended 2026-11-08, Decision 15.)
- **R21** — THE ENGINE SHALL require the timing (`end_of_period` or `mid_period`, no default).
  WHEN the timing is `mid_period`, it SHALL require the terminal discounting time, `horizon`
  (t = N, factor DF_N) or `mid_period` (t = N − 0.5, factor DF_N × (1 + r_T)^0.5, with r_T the
  terminal rate; identical to (1 + r_N)^0.5 when one rate is given, Decision 16), and report it;
  WHEN `mid_period` is chosen for `tv.exit_multiple`, `tv.liquidation` or `tv.limited_horizon`
  (a price at a date; for the limited horizon, its residual value, Decision 17), it SHALL report
  a warning. WHEN the timing is `end_of_period`, the terminal discounting time SHALL NOT be given
  (`FM-E001`). (Amended 2026-11-08, Decisions 16 and 17.)
- **R22** — THE ENGINE SHALL return the terminal value at N, its present value and, in every
  result, the terminal value weight: the present value of the terminal value divided by the
  present value of the explicit flows plus the present value of the terminal value (the value of
  operations on the FCFF basis); it SHALL pass an optional growth cap to the technique
  (SPEC-0006 R8).

### From value of operations to equity value (#44) (Revised 2026-11-08, terminology decision c)
- **R23** — WHEN the basis is FCFF, THE ENGINE SHALL return, each quantity with this one meaning:
  `value_of_operations` = present value of the explicit FCFF + present value of the terminal
  value; `enterprise_value` = `value_of_operations` + Σ non-operating assets;
  `equity_value` = `enterprise_value` − net financial position − Σ debt-like items −
  non-controlling interests; with the net financial position in the structure of SPEC-0007
  (gross debt − cash + signed other items, positive for net debt) and every line labelled in
  `explain`.
- **R24** — WHEN the basis is FCFE, THE ENGINE SHALL return `equity_value` = present value of
  the explicit FCFE + present value of the terminal value + Σ non-operating assets − Σ
  debt-like items − non-controlling interests, and SHALL NOT return `value_of_operations` or
  `enterprise_value`; IF a net financial position is given, THEN it SHALL return `FM-E019`
  (debt is already in the flows) (Decision 9).
- **R25** — Debt-like items, non-operating assets and non-controlling interests SHALL be
  labelled amounts of zero or more, measured at the valuation date as given; the engine SHALL
  NOT choose which items are included and SHALL echo each label and amount. No item given
  means none, reported in `explain.conventions`.
- **R26** — WHEN equity value is negative, THE ENGINE SHALL report it in
  `explain.conventions` as a warning and SHALL NOT floor it at zero (as decided for SPEC-0006 on
  2026-11-08: inconsistent inputs are reported, not refused).

### Errors, outputs and stability
- **R27** — Every error SHALL name the offending field by its JSON pointer and state the fix in
  the message.
- **R28** — THE ENGINE SHALL report in `explain.conventions` every convention that changes a
  number: basis, terms, inflation and conversions, timing, terminal discounting time, terminal
  technique and first terminal flow, tax treatment, sign conventions of the build-up, bridge
  measurement date.
- **R29** — Multi-way choices SHALL be enumerations or tagged objects, never booleans, so that a
  later option is an additive change; every choice of R6 to R21 SHALL be required, with no
  default.
- **R30** — WHEN given one `given` stage of n flows, one rate, nominal terms on both sides,
  `tv.gordon` on the last flow grown by g, end-of-period timing (or mid-period with terminal
  discounting at the horizon), a net financial position equal to the previous `net_debt` and
  one non-operating asset equal to the previous `non_operating_assets`, THE ENGINE SHALL
  reproduce the expected values of every `dcf.fcff` version 2 golden case: the version 2
  `enterprise_value` (present values only) as the version 3 `value_of_operations`, and the
  version 2 `equity_value` as the version 3 `equity_value`.

## Conventions and parameters (Revised 2026-11-08)
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Basis | FCFF at WACC (enterprise), FCFE at cost of equity (equity) | required (method) | CFA Institute, Free Cash Flow Valuation (Terminology [S1]) |
| Tax on EBIT | rate on EBIT, or per-period amounts; exactly one | required (Decision 7) | Practice varies |
| Negative EBIT (rate form) | `credit` (taxes = rate × EBIT), `zero` | required (Decision 8) | Practice varies |
| Stage projection | `given`, `constant_growth`, `linear_decline` | required per stage | Damodaran, three-stage models (Terminology [S2], [S3]) |
| Linear decline step | g_k = g_from + (g_to − g_from) × k / m, g_to reached in the last period | fixed (Decision 4) | Damodaran [S3] Illustration 15.3 (growth 11.46% to 5.00% "in linear increments") |
| Discount rate | one rate, or one per period with an explicit terminal rate | required | SPEC-0002 R5 |
| Terms of flows and rates | `nominal`, `real` | required, each | Fisher relation, SPEC-0002 R4 |
| Conversion when terms differ | rates converted to the terms of the flows; flows never converted | fixed (Decision 5) | Fisher relation |
| Timing | `end_of_period`, `mid_period` | required (was `end_of_period` in version 2) | Practice varies |
| Terminal discounting under mid-period | `horizon` (N), `mid_period` (N − 0.5, half-period shift at the terminal rate) | required | Practice varies (Decisions 6, 16) |
| Terminal technique | any `tv.*` technique on the basis of the DCF | required | SPEC-0006 |
| First terminal flow (Gordon, H-model) | last flow grown, or normalised flow (Gordon: at N + 1; H-model: base flow at N, Decision 15) | required | PIV (normalised flow); to verify |
| From value of operations to equity value | non-operating assets added to reach enterprise value (FCFF); net financial position (FCFF only), debt-like items and non-controlling interests subtracted to reach equity value | none given means none, reported | Koller et al., ch. 16; IVS 200 130 (Terminology [S12], [S5]) |

## Acceptance criteria (Revised 2026-11-08)
- **AC1** (R2, R3, R5) — Build-up golden cases: FCFF and FCFE per period match the 50-digit
  reference to the 28th significant digit; Damodaran, *Investment Valuation*, 2nd edition,
  Illustration 15.7 (Boeing) matches at its published precision, starting from the published
  EBIT (1 − tax rate) of 1,138 and recording in the golden file that 1,753 × 0.65 = 1,139.45
  differs from the printed 1,138 (Decision 12).
- **AC2** (R3) — Property: with zero interest and zero net borrowing, FCFE equals FCFF.
- **AC3** (R4, R5) — Negative EBIT under each treatment matches the reference; each negative
  D&A, capex or interest returns `FM-E001` with the field and the period in the message.
- **AC4** (R6, R7, R8, R9, R10, R12) — A three-stage case (given, linear decline, Gordon
  terminal) and a two-stage case generated from a base flow match the reference: every
  projected flow, growth, discount factor and present value. Damodaran, *Investment Valuation*,
  2nd edition, Illustration 15.3 (Amgen, Table 15.6) matches at its published precision
  (given flows, rates per period, Gordon terminal value; Decision 12).
- **AC5** (R8, R9) — Properties: a `constant_growth` stage equals a `given` stage with the same
  flows; a `linear_decline` with g_from = g_to equals `constant_growth` at that rate.
- **AC6** (R7, R8, R12) — Count mismatches (flows, lines, rates, inflation) and a misplaced base
  flow return `FM-E025` naming the stage or field, the expected and the actual count.
- **AC7** (R12, R13) — With rates per period, discount factors equal those of
  `tvm.present_value` for the same rates and timing; a rate of −1 or lower returns `FM-E003`.
- [x] **AC8** (R14, R15) — Real flows with nominal rates and no inflation return `FM-E024`; so do
  nominal flows with real rates.
- [x] **AC9** (R16) — Real flows at nominal rates with inflation give the same result as the same
  flows at real rates computed by `rates.real_nominal` (exact equality); with end-of-period
  timing they equal the independently inflated nominal flows at nominal rates within 1e-20
  relative (50-digit reference).
- **AC10** (R17, R18, R11, R26) — Warnings are reported, never errors: growth below inflation,
  growth on a negative flow, growth discontinuity, negative equity value.
- **AC11** (R19, R20) — For each `tv.*` technique, the terminal value equals the standalone
  method on the same inputs (property); P/E or P/BV with FCFF, and EV/EBITDA with FCFE, return
  `FM-E019`.
- **AC12** (R21) — Mid-period with terminal discounting at N − 0.5 equals TV × DF_N × (1 + r_T)^0.5,
  r_T the terminal rate (reference, also with rates per period; Decision 16);
  `terminal_discount_time` with end-of-period timing returns `FM-E001`; the warning is reported
  for an exit multiple, a liquidation and a limited horizon (Decision 17).
- **AC13** (R22, R23, R24, R25) — An FCFF golden case with every item type matches the
  reference for `value_of_operations`, `enterprise_value` and `equity_value` and lists every
  label and amount in `explain`; property: `enterprise_value` − `value_of_operations` equals the
  sum of the non-operating assets, and the terminal value weight is computed on
  `value_of_operations`; an FCFE case returns `equity_value` only; a net financial position with
  FCFE returns `FM-E019`.
- **AC14** (R27, R28, R29) — Unit tests: every error message contains a JSON pointer; every
  convention of the table appears in `explain.conventions`; the schema has no boolean choice.
- **AC15** (R30) — Every `dcf.fcff` version 2 golden case, re-expressed in the new input, gives
  identical expected values (version 2 `enterprise_value` compared with version 3
  `value_of_operations`).

## Interfaces (Revised 2026-11-08)
- Method ids (Decision 1): `dcf.fcff` version 3 (enterprise basis, breaking
  input change), `dcf.fcfe` version 1 (equity basis, new), `dcf.free_cash_flows` version 1
  (build-up, new). The ids `dcf.value`, `dcf.gordon`, `dcf.terminal_value` and
  `dcf.sensitivity` of the previous draft are withdrawn: they duplicate `dcf.fcff` / `dcf.fcfe`,
  `tv.gordon`, `tv.*` and `analysis.sensitivity`.
- Input schema changes: `dcf.fcff` version 3 is **breaking** (`timing` becomes required;
  `net_debt` becomes the net financial position structure; `non_operating_assets` becomes a
  list of labelled amounts; new required stages, terms and terminal technique). Output change:
  the version 2 `enterprise_value` (present values only) is the version 3
  `value_of_operations`; the version 3 `enterprise_value` includes the non-operating assets.
  The two new methods are **additive**. Within version 3, new projection kinds, terminal
  techniques, a dated time axis or more than three stages can be added as
  additive options (R29).
- Compatibility plan: `schemas/dcf.fcff/v1.json` and `v2.json` stay unchanged; `v3.json` is
  generated. The version 2 golden cases are re-expressed as new version 3 cases with identical
  expected values (AC15) and the version 2 files are migrated to version 3 or retired in the
  implementation pull request, as approved by the maintainer on 2026-11-08 (see "Recorded
  approval of the numerical change"). The Python helper, the README quickstart and the
  methodology page change in the same release; the CHANGELOG records a `breaking-change` for `dcf.fcff` v2 -> v3.
- Building blocks reused, not duplicated: `tvm` discount factors (R12), `rates.real_nominal`
  (R16), `tv.*` functions and `TerminalBasis` (R19 to R22), `NetFinancialPosition` of SPEC-0007
  (R23), `tv::Line` for labelled amounts (R25). The WACC and the cost of equity come from
  `coc.*` as inputs; the interest and net borrowing of FCFE can come from
  `loans.amortisation_schedule`.
- New error codes (numbers are the next free ones at implementation): `FM-E024` terms of flows
  and rates differ without inflation; `FM-E025` stage, rate or inflation list inconsistent with
  the number of periods, or a misplaced base flow. `FM-E019` is widened to the bridge (R24).
- Output fields: `periods` table (R10), `terminal` block with the technique output and its
  version (R19 to R22), `bridge` block with every line (R23 to R25), `value_of_operations` and
  `enterprise_value` (FCFF only, R23), `equity_value` (both bases), `terminal_value_weight`
  (R22), converted rates (R16). Each output field has the single meaning stated in R22 to R24.

## Terminology and sources (Revised 2026-11-08)
The terms below were checked on 2026-11-08 against the sources listed after the table (keys
[S1] to [S14]). Terms already in `i18n/glossary.yaml` are marked "glossary". "Verified" names
the source and the passage; "to verify" stays where no recognised source could be reached, with
the reason; where a recognised source uses a different term, the replacement is proposed and the
previous term is kept in the note. Names in the interface were not changed by this review; the
maintainer's terminology decisions a to c of 2026-11-08 (see "Decisions") are applied in the
rows below.

| Term | Name in the interface | Italian equivalent | Candidate source | Status |
| --- | --- | --- | --- | --- |
| Free cash flow to the firm (FCFF) | `dcf.fcff` | flusso di cassa operativo disponibile (FCFF) | CFA Institute curriculum (equity valuation); Koller et al., *Valuation* | English verified: [S1] formulas and Summary (FCFF discounted at the weighted average cost of capital); [S3] definition. Italian: glossary; the OIV text [S11] 21.3 says "flussi di cassa di tipo asset side (o unlevered)", the glossary term itself is not found there: to verify on the final PIV |
| Free cash flow to equity (FCFE) | `dcf.fcfe` | flusso di cassa disponibile per gli azionisti (FCFE) | CFA Institute curriculum; Damodaran | English verified: [S1] formulas (FCFE discounted at the required rate of return on equity). Italian: glossary; the OIV text [S11] 21.2 says "flussi di cassa disponibili per i soci": proposed for the maintainer, glossary unchanged |
| Explicit forecast period | `stages` | periodo di previsione esplicita | Koller et al.; OIV, Principi Italiani di Valutazione (PIV) | Verified: [S5] IVS 103 A20.09 "explicit forecast period"; Italian [S11] annex to 20.2 "periodo di previsione esplicita" |
| Stage (high growth, transition, stable growth) | `stages[].periods` | fase (di crescita elevata, di transizione, di crescita stabile) | Damodaran, multi-stage DCF models | English verified: [S2] "three stage FCFE model", [S3] Illustration 15.3, [S4] "high growth", "transition period", "stable growth"; [S1] "Three-stage models". Italian to verify: the OIV text [S11] uses "fasi" only for the stages of the valuation process |
| Constant growth | `constant_growth` | crescita costante | Damodaran | English verified: [S1] "a constant growth rate in each stage"; [S5] IVS 103 A20.22(a) "Gordon growth model/constant growth model". Italian to verify: not found in [S11], which speaks of "tasso di crescita dei flussi oltre l'orizzonte di piano" (16.9) |
| Growth declining in linear increments (transition period) | `linear_decline`, `from_growth`, `to_growth` | riduzione lineare della crescita | Damodaran (transition period) | Verified: [S2] growth "drops in linear increments" (Illustration 14.5); [S3] Illustration 15.3 adjusts growth "to stable growth levels in linear increments"; [S4] uses the same wording for the cost of capital ("drops in linear increments"). Note: previous term "linear fade of growth" (`linear_fade`); "fade" is used by [S4] for excess returns, not for growth. Renamed to `linear_decline` and Italian "riduzione lineare della crescita" by terminology decision a |
| Base-year flow | `base_flow` | flusso dell'anno base | Damodaran | English verified: [S4] chapter 15, Gerdau example, "Base year numbers"; [S2] uses "most recent year". Italian to verify |
| Nominal terms, real terms | `cash_flow_terms`, `discount_rate_terms`: `nominal`, `real` | in termini nominali, in termini reali (a prezzi correnti, a prezzi costanti) | Eurostat (current and constant prices); Koller et al. (inflation) | Verified: [S5] IVS 103 A20.05(c) "real cash flow does not consider inflation whereas nominal cash flows include expectations regarding inflation"; Italian [S11] 16.7 "in termini nominali oppure in termini reali". The parenthesis (prezzi correnti, prezzi costanti) is to verify: no Eurostat glossary page could be reached in this session |
| Expected inflation | `inflation`, `terminal_inflation` | inflazione attesa | European Central Bank | Verified: [S14] "inflation expectations", "expected inflation"; Italian [S13] QEF 578 "l'inflazione attesa"; Banca d'Italia also uses "aspettative di inflazione" [S13] |
| Fisher relation | (conversion) | relazione di Fisher | SPEC-0002 R4 | verified in SPEC-0002 |
| Operating taxes (taxes on EBIT) | `notional_tax_rate`, `operating_taxes` | imposte figurative (teoriche) sul reddito operativo | Koller et al. ("operating taxes"); PIV | To verify. The chapter of Koller et al. that defines operating taxes (ch. 11) is behind a paywall; [S1] and [S3] write the quantity as EBIT × (1 − tax rate) without naming it; "imposte figurative" and "imposte teoriche" are not found in [S11] |
| EBIT | `ebit` | reddito operativo (EBIT) | IFRS practice; CFA Institute curriculum | English verified: [S1] "earnings before interest and taxes (EBIT)". Italian to verify: the closest statutory subtotal is "Differenza tra valore e costi della produzione (A-B)" [S9] para. 41, which is not identical to EBIT |
| Depreciation and amortisation | `depreciation_and_amortisation` | ammortamenti | IAS 16, IAS 38 | Verified: [S8] IAS 16 and IAS 38 amendment "Clarification of Acceptable Methods of Depreciation and Amortisation", Italian "Chiarimento sui metodi di ammortamento accettabili"; [S9] item B10 "Ammortamenti e svalutazioni" (para. 72 to 74) |
| Capital expenditure | `capital_expenditure` | investimenti in immobilizzazioni (capex) | CFA Institute curriculum | English verified: [S3] "FCFF = EBIT (1 - tax rate) + Depreciation - Capital Expenditure - Working Capital"; [S1] uses FCInv (fixed capital investment) for the same quantity. Italian verified: [S10] Appendix A, Schema 1, investing activities "Immobilizzazioni materiali (Investimenti)" |
| Change in net working capital | `change_in_net_working_capital` | variazione del capitale circolante netto | CFA Institute curriculum | glossary (net working capital). English verified: [S3] Illustration 15.7 "Change in Working Capital"; [S1] uses WCInv. Italian verified: [S10] Schema 1 heading "Variazioni del capitale circolante netto" |
| Interest expense | `interest_expense` | oneri finanziari | glossary (financial charges) | Italian verified: [S9] item C17 "Interessi e altri oneri finanziari" (para. 93, 98). English to verify: [S1] shows only the symbol "Int" in "FCFE = FCFF – Int(1 – Tax rate) + Net borrowing"; the spelled-out term is in the full reading, not reachable |
| Net borrowing | `net_borrowing` | variazione netta dei debiti finanziari | CFA Institute curriculum (FCFE) | English verified: [S1] "Net borrowing". Italian to verify: [S10] lists "Accensione finanziamenti" and "Rimborso finanziamenti" separately; no single term found |
| Debt-like items (debt equivalents) | `debt_like_items` | debt-like items (passività assimilabili al debito) | Koller et al. ("debt equivalents") | English verified: [S12] "debt equivalents like unfunded pension liabilities"; [S5] IVS 200 130.10 "debt equivalent securities", 130.13 "debt-like securities". Italian to verify |
| Non-operating assets | `non_operating_assets` | attività non operative (surplus asset) | Koller et al. | glossary. English verified: [S1] Summary "Non-operating assets, such as excess cash and marketable securities"; [S5] IVS 200 120.04; [S12] "nonoperating assets". Italian to verify: not found in [S11] |
| Non-controlling interests (minority interests) | `non_controlling_interests` | partecipazioni di minoranza | IFRS 10, Appendix A | Verified: [S6] IFRS 10 Appendix A "Equity in a subsidiary not attributable, directly or indirectly, to a parent"; Italian [S7] IAS 27 para. 4 (same definition) "La partecipazione di minoranza è il patrimonio netto di una controllata non attribuibile, direttamente o indirettamente, ad una controllante". Note: previous Italian "interessenze di minoranza (patrimonio netto di terzi)"; "patrimonio netto di terzi" is common practice but was not found on an official source. Italian "partecipazioni di minoranza" confirmed by terminology decision b |
| From enterprise value to equity value | `bridge` | passaggio dall'enterprise value all'equity value | Koller et al. | Replacement proposed. [S12] ch. 16 "Moving from Enterprise Value to Value per Share": "subtract all nonequity claims from enterprise value"; [S5] IVS 200 130.01 and 130.05. Note: previous term "equity bridge", not found on a recognised source. In [S12] enterprise value already includes nonoperating assets ("add nonoperating assets to the value of core operations"); by terminology decision c this spec follows [S12]: `value_of_operations` is the value of core operations and `enterprise_value` = `value_of_operations` + non-operating assets (R23). Italian: [S11] 21.3 "capitale investito lordo (enterprise value)" and 16.8 "capitale netto (equity value)"; the phrase itself is to verify |
| Value of operations (value of core operations) | `value_of_operations` | to verify | Koller et al. | English: [S12] "add nonoperating assets to the value of core operations" (terminology decision c). Italian to verify. Note for the translator: `i18n/glossary.yaml` renders "enterprise value" as "enterprise value (valore del capitale operativo)", which after decision c describes the value of operations rather than enterprise value; the glossary entry needs the maintainer's review before the Italian methodology page is written |
| Terminal discounting time | `terminal_discount_time`: `horizon`, `mid_period` | momento di attualizzazione del valore terminale | Practice (mid-year convention) | To verify. [S5] mentions a "mid-period adjustment" only for intangible assets (IVS 210 100.06(g)); no recognised source reached states when the terminal value is discounted under mid-period timing |
| Normalised flow | `normalised_next_flow` | flusso normalizzato | PIV | Partly verified. Italian [S11] 16.3 "risultato economico normalizzato" and 16.4 "normalizzazione dei risultati aziendali attesi" (income method). English: [S5] IVS 103 A20.10 uses "stabilised level of growth and profits", not "normalised". To verify on the final PIV |

Sources consulted on 2026-11-08:
- [S1] CFA Institute, "Free Cash Flow Valuation", Refresher Reading, 2025 curriculum: formulas
  for FCFF and FCFE, discount rates, single-, two- and three-stage models, Summary.
  https://www.cfainstitute.org/insights/professional-learning/refresher-readings/2025/free-cash-flow-valuation
- [S2] A. Damodaran, *Investment Valuation*, 2nd edition, chapter 14 (free cash flow to equity
  models), section "III. The E-Model - A Three Stage FCFE Model", Illustration 14.5 (Tsingtao
  Breweries). https://pages.stern.nyu.edu/~adamodar/pdfiles/valn2ed/ch14d.pdf
- [S3] A. Damodaran, *Investment Valuation*, 2nd edition, chapter 15 "Firm Valuation: Cost of
  Capital and APV Approaches": FCFF definition, Illustration 15.3 (Amgen, Table 15.6),
  Illustration 15.7 (Boeing). https://pages.stern.nyu.edu/~adamodar/pdfiles/valn2ed/ch15.pdf
- [S4] A. Damodaran, *Investment Valuation*, 3rd edition, chapter 15 "Firm Valuation: Cost of
  Capital and Adjusted Present Value Approaches", Illustration 15.3 (Amgen, 2009) and base year
  of Gerdau; and "Closure in Valuation: Estimating Terminal Value" (p. 17 and 18).
  https://pages.stern.nyu.edu/~adamodar/pdfiles/val3ed/c15.pdf,
  https://pages.stern.nyu.edu/~adamodar/pdfiles/papers/termvalue.pdf
- [S5] IVSC, International Valuation Standards effective 31 January 2025 (redline edition
  distributed by ICAEW): IVS 103 Appendix A20 (A20.05, A20.09, A20.10, A20.22, A20.23), IVS 200
  (120.04, 130.01, 130.05, 130.10, 130.13), IVS 210 (100.06).
  https://icaew.com/-/media/corporate/files/technical/corporate-finance/valuation/ivs-effective-31-january-2025-redline-edition.ashx
- [S6] IFRS 10 *Consolidated Financial Statements*, Appendix A, as adopted by Commission
  Regulation (EU) No 1254/2012. https://legislation.gov.uk/eur/2012/1254/annex/2020-01-31
- [S7] Commission Regulation (EC) No 494/2009 (IAS 27 as amended in 2008), Italian text, OJ L 149,
  12.6.2009, paragraph 4. https://fondazioneoic.eu/wp-content/uploads/2024/09/Reg_494_2009.pdf
- [S8] Commission Regulation (EU) 2015/2231 (amendments to IAS 16 and IAS 38), Italian text,
  OJ L 317, 3.12.2015, https://fondazioneoic.eu/wp-content/uploads/2024/09/Reg_2231_20151.pdf;
  IFRS Foundation, project "Clarification of Acceptable Methods of Depreciation and
  Amortisation",
  https://www.ifrs.org/projects/completed-projects/2014/clarification-of-acceptable-methods-of-depreciation-and-amortisation/
- [S9] OIC 12 "Composizione e schemi del bilancio d'esercizio" (December 2016, amended up to
  18 March 2024), paragraphs 41, 72 to 74, 93, 98.
  https://www.fondazioneoic.eu/wp-content/uploads/2026/01/2016-12-OIC-12-Composizione-e-schemi-del-bilancio.pdf
- [S10] OIC 10 "Rendiconto finanziario" (August 2014), Appendix A, Schema 1 (indirect method).
  https://www.fondazioneoic.eu/wp-content/uploads/2024/09/2014-08-05-OIC-10-Rendiconto-finanziario.pdf
- [S11] OIV, Principi Italiani di Valutazione, Parte Prima, exposure draft of 20 March 2013,
  paragraphs 16.3, 16.4, 16.7, 16.8, 16.9, 21.2, 21.3 and annex to 20.2. The final PIV text
  (2015, 2026 edition) is not publicly accessible; terms marked as verified on [S11] should be
  rechecked on the final text.
  https://www.dirittobancario.it/wp-content/uploads/sites/default/files/allegati/principi_italiani_di_valutazione_piv_-_exposure_draft.pdf
- [S12] T. Koller, M. Goedhart, D. Wessels (McKinsey & Company), *Valuation*, 7th edition, Wiley,
  2020, chapter 16 "Moving from Enterprise Value to Value per Share" (publisher preview).
  https://www.oreilly.com/library/view/valuation-7th-edition/9781119610885/c16.xhtml
- [S13] Banca d'Italia, Questioni di Economia e Finanza n. 578, A. Di Iorio and M. Fanari,
  "Tassi di inflazione di break-even: il caso italiano", September 2020,
  https://www.bancaditalia.it/pubblicazioni/qef/2020-0578/index.html; Banca d'Italia, "Indagine
  sulle aspettative di inflazione e crescita, 4° trimestre 2025", 14 January 2026,
  https://www.bancaditalia.it/pubblicazioni/indagine-inflazione/2025-indagine-inflazione/12/index.html
- [S14] ECB Blog, U. Baumann et al., "How euro area firms' inflation expectations affect their
  business decisions", 4 July 2024.
  https://www.ecb.europa.eu/press/blog/date/2024/html/ecb.blog240704~a2363df64d.en.html

## Decisions (maintainer, 2026-11-08)
The maintainer approved this spec on 2026-11-08 and resolved every open question of the draft
with the recommended answer, as recorded below, together with three terminology decisions from
the source verification.

- [x] **1. Method ids and versions.** `dcf.fcff` version 3 (enterprise basis), new `dcf.fcfe`
      version 1 (equity basis) and new `dcf.free_cash_flows` version 1 (build-up), on one shared
      engine. The polymorphic `dcf.value` is not introduced. The `dcf.fcff` version 2 golden
      cases are migrated to version 3 or retired as an approved numerical change (see "Recorded
      approval of the numerical change"). Reasons: one tool per basis gives field names that
      cannot be confused (`wacc` versus `cost_of_equity`) and simpler schemas for agents;
      nothing is released and `dcf.fcff` is Preview.
- [x] **2. Stages.** One to three explicit stages of given length, always followed by a
      terminal value; a finite last phase uses `tv.limited_horizon`.
- [x] **3. Growth choice.** A required `projection` per stage (`given`, `constant_growth`,
      `linear_decline`), no default, never a boolean.
- [x] **4. Linear decline step.** g_k = g_from + (g_to − g_from) × k / m: g_from is the growth
      before the stage and g_to is reached in the last period ("from 10% to 3% over 5 years":
      8.6, 7.2, 5.8, 4.4, 3.0), as in [S3] Illustration 15.3.
- [x] **5. Nominal and real.** A mismatch without inflation is refused (`FM-E024`); with
      inflation only the discount rates are converted to the terms of the flows, by the exact
      Fisher relation; flows are never converted.
- [x] **6. Timing.** `timing` is required; under mid-period the terminal discounting time
      (`horizon` or `mid_period`) is required, with a warning for an exit multiple or a
      liquidation discounted at N − 0.5.
- [x] **7. Taxes on EBIT.** Exactly one of a tax rate on EBIT or per-period tax amounts.
- [x] **8. Negative EBIT.** Required choice: `credit` (taxes = rate × EBIT) or `zero` (no tax);
      no default; tax loss carry-forwards out of scope.
- [x] **9. Equity basis.** `dcf.fcfe` refuses a net financial position (`FM-E019`) and accepts
      non-operating assets, debt-like items and non-controlling interests; the bridge stays a
      reusable type inside the core, and a standalone method is decided with the market
      multiples spec (M4).
- [x] **10. Error codes.** New `FM-E024` (terms of flows and rates differ without inflation) and
      `FM-E025` (counts and base flow); `FM-E019` widened to the equity basis of the bridge.
- [x] **11. Scope.** The one-stage method and the sensitivity grid are dropped (covered by
      `tv.gordon`, `tvm.perpetuity` and `analysis.sensitivity`); dated periods with a stub, the
      dividend basis and the linear decline of the discount rate are deferred as additive
      options of version 3; the stub needs its own roadmap task.
- [x] **12. Golden sources.** Every case uses the 50-digit reference; in addition: Damodaran,
      *Investment Valuation*, 2nd edition, chapter 15, Illustration 15.3 (Amgen, three-stage,
      compared at the published precision) and Illustration 15.7 (Boeing, FCFF build-up,
      starting from the published EBIT (1 − tax rate) and recording the 1,139.45 versus 1,138
      discrepancy in the golden file). The source analysis that supported the choice follows.
      Tolerances accepted by the maintainer on 2026-11-08: the Amgen case is compared within 2
      on the published present value of the explicit period, value of the operating assets and
      equity value (`damodaran-amgen-published-values.yaml`) and within 20 on the published
      terminal value (`damodaran-amgen-published-terminal-value.yaml`), because the published
      figures come from rounded printed inputs; the golden files decompose the differences.
- [x] **13. Terminal rate with one discount rate.** The single discount rate is used for the
      terminal value and no separate terminal rate is accepted; with rates per period, a
      terminal rate is required.
- [x] **14. Amendment to R3 and R4, approved by the maintainer on 2026-11-08 (issue #38).**
      (a) In a period where the `zero` treatment applies (EBIT negative, taxes computed from a
      rate), the interest tax shield is zero: the interest expense is subtracted in full,
      FCFE_t = FCFF_t − interest expense_t + net borrowing_t. This is the net-income route of
      the CFA Institute curriculum and Damodaran (FCFE = net income + D&A − capex − ΔNWC + net
      borrowing, with no tax on a negative pre-tax income), so in those periods the FCFE does
      not depend on the route (clarification of wording, 2026-11-08). With `credit`, and in periods with a positive or zero EBIT, R3 is unchanged: the
      shield is interest expense × t, also when EBIT is lower than the interest expense.
      (b) R29 interpretation: the choice between a tax rate (`notional_tax_rate`) and tax
      amounts (`operating_taxes` in every period) is expressed by which fields are present, as
      the rate forms of `loans.*` and the flow choice of `tv.gordon`. It satisfies R29: the
      choice is explicit (exactly one form, `FM-E001` otherwise), has no default, is not a
      boolean, and a later form can be added as a new field without breaking existing inputs.
- [x] **15. H-model with a normalised flow** (approved by the maintainer on 2026-11-08). The
      normalised flow of `tv.h_model` is given at N: a normalised base flow CF_N replaces the
      last explicit flow and the standard Fuller-Hsia formula applies, with no conversion from
      N + 1. The first terminal flow of the H-model is therefore the last explicit flow
      (`last_flow_grown`) or a normalised base flow at N (`normalised_base_flow`), stated in
      `explain.conventions` and in the schema. Amends R20.
- [x] **16. Rate of the half-period shift** (approved by the maintainer on 2026-11-08). A
      terminal value discounted at N − 0.5 uses the terminal rate r_T for the half period:
      PV_TV = TV × DF_N × (1 + r_T)^0.5. With one discount rate r_T = r_N and nothing changes;
      with rates per period the terminal rate (`terminal_wacc`) applies. Amends R21 and AC12.
- [x] **17. Limited horizon discounted at N − 0.5** (approved by the maintainer on
      2026-11-08). Under mid-period timing, `tv.limited_horizon` discounted at N − 0.5 carries
      the same warning as an exit multiple and a liquidation: its residual value is a price at a
      date. Amends R21.
- [x] **18. Terminal inflation with one inflation** (approved by the maintainer on
      2026-11-08). `terminal_inflation` next to a single `inflation` is refused with `FM-E001`
      naming both fields: one inflation applies to every period and to the terminal phase.
      Consistent with Decision 13 for the terminal discount rate. Applies to R16.
- [x] **19. Inflation at or below -1** (approved by the maintainer on 2026-11-08). An expected
      inflation at or below -1 is `FM-E003` (rate out of range), naming its JSON pointer: the
      same code that `rates.real_nominal` returns for the same value, so that one condition has
      one code across the library. Applies to R16 and R27.
- [x] **20. Readings of R17 confirmed** (approved by the maintainer on 2026-11-08). (a) No
      growth is restated for the terminal techniques that state no perpetual growth
      (`tv.exit_multiple`, `tv.limited_horizon`, `tv.liquidation`); the growth implied by an
      exit multiple is an output of the technique, not restated; a convention says that none is
      restated. (b) The warning for a perpetual growth below inflation uses the inflation of the
      terminal phase (`inflation`, or `terminal_inflation`), since the perpetual growth applies
      after N. (c) No warning is reported with real flows: the restated nominal growth is
      reported only.

Terminology decisions from the source verification:
- [x] **a.** The projection option `linear_fade` is renamed `linear_decline` everywhere
      (Damodaran: growth declining "in linear increments"); Italian "riduzione lineare della
      crescita".
- [x] **b.** Italian term for non-controlling interests: "partecipazioni di minoranza".
- [x] **c.** Outputs of the FCFF basis: `value_of_operations` (present value of the explicit
      FCFF plus the terminal value), `enterprise_value` = `value_of_operations` + non-operating
      assets (as in Koller et al. [S12]), `equity_value` = `enterprise_value` − net financial
      position − debt-like items − non-controlling interests. FCFE basis: `equity_value` =
      present value of the FCFE and the terminal value + non-operating assets − debt-like items
      − non-controlling interests. Applied in R22 to R24, AC13, AC15 and "Interfaces".

### Recorded approval of the numerical change
The maintainer approved on 2026-11-08 the `numerical-change` and `breaking-change` of `dcf.fcff`
version 2 to version 3: the version 2 golden cases in `tests/golden/dcf.fcff/` are migrated to
the version 3 input (with identical expected values, the version 2 `enterprise_value` compared
with the version 3 `value_of_operations`, AC15) or retired, in the implementation pull request.
Expected values of other golden cases are not changed.

On 2026-11-08 the maintainer also approved the migration of the seven `analysis.*` golden cases
whose inner method is `dcf.fcff`, with every expected number identical:

| File | What changed |
| --- | --- |
| `tests/golden/analysis.goal_seek/dcf-implied-discount-rate.yaml` | inner input in version 3 form; variable `/discount_rate` → `/wacc` |
| `tests/golden/analysis.goal_seek/no-convergence.yaml` | inner input in version 3 form; variable `/discount_rate` → `/wacc` |
| `tests/golden/analysis.goal_seek/target-not-bracketed.yaml` | inner input in version 3 form; variable `/discount_rate` → `/wacc` |
| `tests/golden/analysis.scenarios/dcf-three-cases.yaml` | inner input in version 3 form; override pointers renamed to the version 3 fields |
| `tests/golden/analysis.sensitivity/wacc-g-5x5.yaml` | inner input in version 3 form; rows `/wacc`, columns `/terminal/tv.gordon/growth` |
| `tests/golden/analysis.sensitivity/wacc-one-way.yaml` | inner input in version 3 form; rows `/wacc` |
| `tests/golden/analysis.tornado/dcf-four-drivers.yaml` | inner input in version 3 form; the four variables, and the four expected ranking variables, renamed to `/wacc`, `/terminal/tv.gordon/growth`, `/net_financial_position/gross_debt`, `/stages/0/flows/0` |

In every file the inner input is one given stage, one rate (`wacc`), nominal terms, end-of-period
timing, `tv.gordon` on the last flow grown and a net financial position equal to the previous
`net_debt`; the output pointers are unchanged.

### Source analysis for Decision 12
Candidates found on 2026-11-08 and approved as recorded in Decision 12:
- Three-stage model: Damodaran, *Investment Valuation*, 2nd edition, chapter 15,
  Illustration 15.3 (Amgen, May 2001), Table 15.6,
  https://pages.stern.nyu.edu/~adamodar/pdfiles/valn2ed/ch15.pdf. Inputs ($ million):
  adjusted after-tax EBIT 1,454; years 1 to 5 growth 13.08%, reinvestment rate 56.27%, cost
  of capital 10.76%; years 6 to 10 growth 11.46%, 9.85%, 8.23%, 6.62%, 5.00% ("linear
  increments"), reinvestment rate 50.01% down to 25.00%, cost of capital 10.38%, 10.00%,
  9.62%, 9.24%, 8.86%; stable growth 5%, reinvestment rate 25%, cost of capital 8.86%.
  Published results: FCFF years 1 to 10 of 719, 813, 919, 1,040, 1,176, 1,498, 1,851, 2,226,
  2,611, 2,991; FCFF in year 11 of 3,140; terminal value in year 10 of 81,364; value of the
  operating assets 39,161; plus cash and marketable securities 2,029, minus debt 323, equity
  value 40,867. An independent recomputation from the printed rates gives 8,328 for the
  explicit period (published 8,327), 81,368 for the terminal value and 39,168 for the
  operating assets: the differences come from the rounding of the printed rates, so the golden
  case matches at the published precision only. The transition growth rates follow Decision 4.
  The FCFF comes from a reinvestment rate, not from a line build-up, and the cost of capital
  changes every period, so the case exercises `given` flows with rates per period and a Gordon
  terminal value. A second candidate on the equity basis, not required by Decision 12:
  chapter 14, Illustration 14.5 (Tsingtao Breweries, three-stage FCFE, value per share
  CY 7.04), https://pages.stern.nyu.edu/~adamodar/pdfiles/valn2ed/ch14d.pdf.
- FCFF build-up from EBIT: Damodaran, *Investment Valuation*, 2nd edition, chapter 15,
  Illustration 15.7 (Boeing), same URL. Published figures ($ million): EBIT 1,753, tax rate
  35%, EBIT (1 − tax rate) 1,138, plus depreciation and amortisation 1,517, minus capital
  expenditures 1,584, minus change in working capital (105), FCFF 1,176. The arithmetic from
  EBIT (1 − tax rate) to FCFF is exact; 1,753 × 0.65 = 1,139.45, so the printed 1,138 implies
  an EBIT of about 1,751: the golden case starts from the published EBIT (1 − tax rate) and
  records the discrepancy (Decision 12). No CFA Institute worked example with numbers could be
  reached: the public refresher reading [S1] contains formulas only.

## Open questions
None: every question of the draft was resolved by the maintainer on 2026-11-08 (see
"Decisions (maintainer, 2026-11-08)").
