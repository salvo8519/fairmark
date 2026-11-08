# SPEC-0001 — Discounted cash flow: free cash flow build-up, parametric stages, nominal and real terms, equity bridge

| | |
| --- | --- |
| Status | Draft — revised 2026-11-08 for issues #38, #110, #44 and #43; awaiting the maintainer's decisions (see "Open questions") |
| Milestone | M2 (the previous header said M1; the roadmap places this spec in M2) |
| Owner | @salvo8519 |
| Formula basis | Commonly adopted enterprise DCF (FCFF at the WACC) and equity DCF (FCFE at the cost of equity) with an explicit forecast period, a transition stage and a terminal value; exact Fisher relation for nominal and real rates; verified against an independent 50-digit reference and published worked examples to be named in the golden files (Open question 12) |
| Related | ADR 0008, ADR 0009 (typed calls between methods), ADR 0010 (evaluation inside the core); SPEC-0002 (time model, `rates.real_nominal`), SPEC-0003 (`coc.*`), SPEC-0006 (`tv.*`), SPEC-0007 (net financial position), SPEC-0008 (`analysis.sensitivity`); statements reclassification in SPEC-0005 |

Changes in this revision are marked "Revised 2026-11-08" in each section heading. Requirements
are renumbered; the table in "Analysis" maps the previous numbers to the new ones.

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
| R1 | One-stage Gordon value | Not in `dcf.*`; equals `tv.gordon` at N = 0 and `tvm.perpetuity` | Dropped from this spec (Open question 11) |
| R2 | Two-stage value | Partly: explicit flows plus Gordon only | R6, R7, R19 |
| R3 | Three-stage with linear fade | Not implemented | R6 to R11 (#110) |
| R4 | Terminal value technique as explicit input | Gordon only, implicit | R19, R20 (#43, reusing `tv.*`) |
| R5 | One rate or one rate per period; explicit terminal rate | One rate only | R12, R13 (#110) |
| R6 | Explicit timing; terminal discounting time under mid-period | Timing with default; terminal at n fixed | R21 (#43) |
| R7 | Valuation date, dated periods, stub period | Not implemented; no issue | Deferred (Open question 11) |
| R8 | Equity bridge with explicit items | `net_debt` and `non_operating_assets` only | R23 to R26 (#44) |
| R9 | r × g sensitivity grid | Covered by `analysis.sensitivity` (#45, #116) | Dropped from this spec |
| R10 | Typed errors; terminal value weight in every result | g ≥ r error and weight exist | R22, R27, R28 |

### What each issue needs
- **#38 FCFF and FCFE build-up.** A method that turns per-period operating line items (EBIT,
  taxes on EBIT, depreciation and amortisation, capital expenditure, change in net working
  capital) into FCFF, and adds the financing items (interest after tax, net borrowing) for
  FCFE; the same lines accepted directly as the flows of an explicit stage. Statutory
  reclassification stays in SPEC-0005: this spec starts from the reclassified lines. Needs the
  tax form (Open question 7) and the treatment of a negative EBIT (Open question 8). Requires
  a new equity-basis method (Open question 1).
- **#110 Parametric stages.** An ordered list of stages with an explicit number of periods,
  each with a required projection choice (given flows, constant growth, linear fade);
  per-period discount rates; required declaration of nominal or real terms for flows and for
  rates, refusal of a mismatch without inflation, conversion of the rates by the exact Fisher
  relation when inflation is given. Needs Open questions 2 to 5.
- **#44 EV to equity.** A bridge with the net financial position of SPEC-0007, labelled
  debt-like items, labelled non-operating assets and non-controlling interests, every line in
  `explain`; rules for the equity basis (Open question 9).
- **#43 Exit-multiple terminal value; configurable terminal value timing.** See below.

### Is #43 already covered by SPEC-0006?
Partly. The exit-multiple computation is delivered: `tv.exit_multiple` (SPEC-0006 R4, issue
#115) returns TV_N = multiple × metric_N on a declared enterprise or equity basis, with the
implied perpetual growth and the growth cap. Two things are not covered, because SPEC-0006
places discounting out of its scope: (1) using an exit multiple, or any `tv.*` technique,
inside the DCF, and (2) the time at which the terminal value is discounted under mid-period
timing. Both are in this spec (R19 to R21). Recommendation: keep #43 open, reduce it to "Terminal
value technique and discounting time in the DCF" and link it to SPEC-0001 instead of
SPEC-0006; or close it and fold R19 to R21 into the #110 implementation. A leading exit
multiple (metric at N + 1) is not covered by `tv.exit_multiple` (trailing only) and is not
requested; it stays out of scope.

## Problem
Discounted cash flow is the reference method for going concerns. Analysts build free cash flows
from a business plan, split the forecast into stages of explicit length, choose how flows grow
in each stage, state whether the plan is in nominal or real terms, choose a terminal value and
its timing, and bridge enterprise value to equity value. Each choice changes the value
materially, and the most frequent errors (a real plan discounted at a nominal rate, a flow list
shorter than the stated stage, a terminal value discounted one period too many, net debt
subtracted from an equity value) are silent in spreadsheets and in the output of an AI agent.
The engine must make every choice an explicit input, refuse inconsistent combinations with a
typed error that names the field and the fix, and reuse the building blocks already on main.

## Scope (Revised 2026-11-08)
**In scope:** free cash flow build-up from reclassified line items (FCFF; FCFE with financing
items); enterprise DCF (FCFF at the WACC) and equity DCF (FCFE at the cost of equity) on one
engine; one to three explicit stages of explicit length with given flows, constant growth or
linear fade; one discount rate or one per period; declared nominal or real terms with
conversion of rates by the Fisher relation; terminal value by any `tv.*` technique, with a
basis check; end or mid-period timing and an explicit terminal discounting time; equity bridge.

**Out of scope:** reclassification of statutory statements (SPEC-0005); estimating inputs
(WACC: `coc.*`; terminal inputs: `tv.*`); sensitivity tables (`analysis.sensitivity`);
probability-weighted scenarios; real options; sub-annual periods.

**Deferred, input designed to accept them later as additive options:** valuation date with
dated period ends and a stub first period (previous R7); dividend discount basis; linear fade of
the discount rate (expressible today with per-period rates); inflating real line items to
nominal before taxes; a leading exit multiple.

## User stories
- As an analyst, I want to state that stage 1 has 5 periods of given flows and stage 2 has
  5 periods fading from 8% to 2%, so that the model matches my plan without hidden steps.
- As an analyst with a plan at constant prices, I want to declare it real and give expected
  inflation, so that my nominal WACC is converted instead of mixed.
- As an AI agent, I want a real flow discounted at a nominal rate to be refused with the field
  names and the fix, so that I correct my call instead of reporting a wrong value.
- As an AI agent, I want the per-period table, the terminal technique and every bridge line in
  the result, so that I can cite each number in the report.

## Requirements (EARS) (Revised 2026-11-08)

### Methods and basis
- **R1** — THE ENGINE SHALL provide an enterprise DCF that discounts FCFF at the WACC and
  bridges enterprise value to equity value, and an equity DCF that discounts FCFE at the cost
  of equity and returns equity value, computed by one shared engine; the basis SHALL be
  reported in `explain.conventions`. (Method ids and versions: Open question 1.)

### Free cash flow build-up (#38)
- **R2** — WHEN given, for each period, EBIT, depreciation and amortisation, capital
  expenditure, change in net working capital, optional labelled other adjustments and the taxes
  on EBIT (Open question 7), THE ENGINE SHALL return FCFF_t = EBIT_t − taxes on EBIT_t +
  D&A_t − capex_t − ΔNWC_t + Σ adjustments_t, with every component and the substituted formula
  in `explain`.
- **R3** — WHEN additionally given, for each period, interest expense and net borrowing, THE
  ENGINE SHALL return FCFE_t = FCFF_t − interest expense_t × (1 − t) + net borrowing_t, with t
  the same tax rate as in R2, and SHALL report that the interest tax shield uses that rate.
- **R4** — WHEN EBIT is negative, THE ENGINE SHALL apply the treatment chosen in the input
  (Open question 8) and report it; it SHALL NOT model tax loss carry-forwards.
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
- **R9** — WHEN a stage's projection is `linear_fade` over m periods from growth g_from to
  growth g_to, THE ENGINE SHALL apply g_k = g_from + (g_to − g_from) × k / m in period k = 1..m
  of the stage (Open question 4), with the same starting flow rule as R8.
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
  ENGINE SHALL require an explicit choice of the first terminal flow: the last explicit flow
  grown by the perpetual growth, or a given normalised flow at N + 1; amounts in the terminal
  terms SHALL be in the terms of the cash flows.
- **R21** — THE ENGINE SHALL require the timing (`end_of_period` or `mid_period`, no default).
  WHEN the timing is `mid_period`, it SHALL require the terminal discounting time, `horizon`
  (t = N, factor DF_N) or `mid_period` (t = N − 0.5, factor DF_N × (1 + r_N)^0.5), and report it;
  WHEN `mid_period` is chosen for `tv.exit_multiple` or `tv.liquidation` (a price at a date), it
  SHALL report a warning. WHEN the timing is `end_of_period`, the terminal discounting time SHALL
  NOT be given (`FM-E001`).
- **R22** — THE ENGINE SHALL return the terminal value at N, its present value and the share of
  value in the terminal value in every result, and SHALL pass an optional growth cap to the
  technique (SPEC-0006 R8).

### Equity bridge (#44)
- **R23** — WHEN the basis is FCFF, THE ENGINE SHALL return equity value = enterprise value −
  net financial position − Σ debt-like items + Σ non-operating assets − non-controlling
  interests, with the net financial position in the structure of SPEC-0007 (gross debt − cash +
  signed other items, positive for net debt) and every line labelled in `explain`.
- **R24** — WHEN the basis is FCFE, THE ENGINE SHALL return equity value = present value of FCFE
  and terminal value + Σ non-operating assets − Σ debt-like items − non-controlling interests;
  IF a net financial position is given, THEN it SHALL return `FM-E019` (debt is already in the
  flows) (Open question 9).
- **R25** — Debt-like items, non-operating assets and non-controlling interests SHALL be
  labelled amounts of zero or more, measured at the valuation date as given; the engine SHALL
  NOT choose which items are included and SHALL echo each label and amount.
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
  discounting at the horizon) and a net financial position equal to the previous `net_debt`,
  THE ENGINE SHALL reproduce the expected values of every `dcf.fcff` version 2 golden case.

## Conventions and parameters (Revised 2026-11-08)
| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Basis | FCFF at WACC (enterprise), FCFE at cost of equity (equity) | required (method) | Commonly adopted; source to verify (Terminology) |
| Tax on EBIT | notional rate on EBIT, or per-period amounts | required (Open question 7) | Practice varies |
| Negative EBIT | taxes as a credit (symmetric), or zero tax | required (Open question 8) | Practice varies |
| Stage projection | `given`, `constant_growth`, `linear_fade` | required per stage | Damodaran (stages); to verify |
| Linear fade step | g_k = g_from + (g_to − g_from) × k / m | fixed (Open question 4) | To verify |
| Discount rate | one rate, or one per period with an explicit terminal rate | required | SPEC-0002 R5 |
| Terms of flows and rates | `nominal`, `real` | required, each | Fisher relation, SPEC-0002 R4 |
| Conversion when terms differ | rates converted to the terms of the flows; flows never converted | fixed (Open question 5) | Fisher relation |
| Timing | `end_of_period`, `mid_period` | required (was `end_of_period` in version 2) | Practice varies |
| Terminal discounting under mid-period | `horizon` (N), `mid_period` (N − 0.5) | required | Practice varies (Open question 6) |
| Terminal technique | any `tv.*` technique on the basis of the DCF | required | SPEC-0006 |
| First terminal flow (Gordon, H-model) | last flow grown, or normalised flow | required | PIV (normalised flow); to verify |
| Bridge items | net financial position (FCFF only), debt-like items, non-operating assets, non-controlling interests | none given means none, reported | Practice; to verify |

## Acceptance criteria (Revised 2026-11-08)
- **AC1** (R2, R3, R5) — Build-up golden cases: FCFF and FCFE per period match the 50-digit
  reference to the 28th significant digit; one published worked example per flow type matches
  at its published precision (source named in the golden file, Open question 12).
- **AC2** (R3) — Property: with zero interest and zero net borrowing, FCFE equals FCFF.
- **AC3** (R4, R5) — Negative EBIT under each treatment matches the reference; each negative
  D&A, capex or interest returns `FM-E001` with the field and the period in the message.
- **AC4** (R6, R7, R8, R9, R10) — A three-stage case (given, linear fade, Gordon terminal) and
  a two-stage case generated from a base flow match the reference: every projected flow,
  growth, discount factor and present value.
- **AC5** (R8, R9) — Properties: a `constant_growth` stage equals a `given` stage with the same
  flows; a `linear_fade` with g_from = g_to equals `constant_growth` at that rate.
- **AC6** (R7, R8, R12) — Count mismatches (flows, lines, rates, inflation) and a misplaced base
  flow return `FM-E025` naming the stage or field, the expected and the actual count.
- **AC7** (R12, R13) — With rates per period, discount factors equal those of
  `tvm.present_value` for the same rates and timing; a rate of −1 or lower returns `FM-E003`.
- **AC8** (R14, R15) — Real flows with nominal rates and no inflation return `FM-E024`; so do
  nominal flows with real rates.
- **AC9** (R16) — Real flows at nominal rates with inflation give the same result as the same
  flows at real rates computed by `rates.real_nominal` (exact equality); with end-of-period
  timing they equal the independently inflated nominal flows at nominal rates within 1e-20
  relative (50-digit reference).
- **AC10** (R17, R18, R11, R26) — Warnings are reported, never errors: growth below inflation,
  growth on a negative flow, growth discontinuity, negative equity value.
- **AC11** (R19, R20) — For each `tv.*` technique, the terminal value equals the standalone
  method on the same inputs (property); P/E or P/BV with FCFF, and EV/EBITDA with FCFE, return
  `FM-E019`.
- **AC12** (R21) — Mid-period with terminal discounting at N − 0.5 equals TV × DF_N × (1 + r_N)^0.5
  (reference); `terminal_discount_time` with end-of-period timing returns `FM-E001`; the exit
  multiple warning is reported.
- **AC13** (R23, R24, R25) — Bridge golden case with every item type matches the reference and
  lists every label and amount in `explain`; a net financial position with FCFE returns
  `FM-E019`.
- **AC14** (R27, R28, R29) — Unit tests: every error message contains a JSON pointer; every
  convention of the table appears in `explain.conventions`; the schema has no boolean choice.
- **AC15** (R30) — Every `dcf.fcff` version 2 golden case, re-expressed in the new input, gives
  identical expected values.

## Interfaces (Revised 2026-11-08)
- Method ids (recommended, Open question 1): `dcf.fcff` version 3 (enterprise basis, breaking
  input change), `dcf.fcfe` version 1 (equity basis, new), `dcf.free_cash_flows` version 1
  (build-up, new). The ids `dcf.value`, `dcf.gordon`, `dcf.terminal_value` and
  `dcf.sensitivity` of the previous draft are withdrawn: they duplicate `dcf.fcff` / `dcf.fcfe`,
  `tv.gordon`, `tv.*` and `analysis.sensitivity`.
- Input schema changes: `dcf.fcff` version 3 is **breaking** (`timing` becomes required;
  `net_debt` becomes the net financial position structure; new required stages, terms and
  terminal technique). The two new methods are **additive**. Within version 3, new projection
  kinds, terminal techniques, a dated time axis or more than three stages can be added as
  additive options (R29).
- Compatibility plan: `schemas/dcf.fcff/v1.json` and `v2.json` stay unchanged; `v3.json` is
  generated. The version 2 golden cases are re-expressed as new version 3 cases with identical
  expected values (AC15); the existing files are in a protected directory, so the maintainer
  moves or retires them. The Python helper, the README quickstart and the methodology page change
  in the same release; the CHANGELOG records a `breaking-change` for `dcf.fcff` v2 -> v3.
- Building blocks reused, not duplicated: `tvm` discount factors (R12), `rates.real_nominal`
  (R16), `tv.*` functions and `TerminalBasis` (R19 to R22), `NetFinancialPosition` of SPEC-0007
  (R23), `tv::Line` for labelled amounts (R25). The WACC and the cost of equity come from
  `coc.*` as inputs; the interest and net borrowing of FCFE can come from
  `loans.amortisation_schedule`.
- New error codes (numbers are the next free ones at implementation): `FM-E024` terms of flows
  and rates differ without inflation; `FM-E025` stage, rate or inflation list inconsistent with
  the number of periods, or a misplaced base flow. `FM-E019` is widened to the bridge (R24).
- Output fields: `periods` table (R10), `terminal` block with the technique output and its
  version (R19 to R22), `bridge` block with every line (R23 to R25), `enterprise_value`
  (FCFF only), `equity_value`, `terminal_value_weight`, converted rates (R16).

## Terminology and sources (Revised 2026-11-08)
No recognised source could be consulted while drafting (no network access in this session).
Terms already in `i18n/glossary.yaml` are marked "glossary"; every source below is a
candidate to check, and each row stays "to verify" until the maintainer or a session with
access confirms the term on that source.

| Term | Name in the interface | Italian equivalent | Candidate source | Status |
| --- | --- | --- | --- | --- |
| Free cash flow to the firm (FCFF) | `dcf.fcff` | flusso di cassa operativo disponibile (FCFF) | CFA Institute curriculum (equity valuation); Koller et al., *Valuation* | glossary; source to verify |
| Free cash flow to equity (FCFE) | `dcf.fcfe` | flusso di cassa disponibile per gli azionisti (FCFE) | CFA Institute curriculum; Damodaran | glossary; source to verify |
| Explicit forecast period | `stages` | periodo di previsione esplicita | Koller et al.; OIV, Principi Italiani di Valutazione (PIV) | to verify |
| Stage (high growth, transition, stable growth) | `stages[].periods` | fase (di crescita elevata, di transizione, di crescita stabile) | Damodaran, multi-stage DCF models | to verify |
| Constant growth | `constant_growth` | crescita costante | Damodaran | to verify |
| Linear fade of growth | `linear_fade`, `from_growth`, `to_growth` | riduzione lineare della crescita | Damodaran (transition period) | to verify |
| Base-year flow | `base_flow` | flusso dell'anno base | Damodaran | to verify |
| Nominal terms, real terms | `cash_flow_terms`, `discount_rate_terms`: `nominal`, `real` | in termini nominali, in termini reali (a prezzi correnti, a prezzi costanti) | Eurostat (current and constant prices); Koller et al. (inflation) | to verify |
| Expected inflation | `inflation`, `terminal_inflation` | inflazione attesa | European Central Bank | to verify |
| Fisher relation | (conversion) | relazione di Fisher | SPEC-0002 R4 | verified in SPEC-0002 |
| Operating taxes (taxes on EBIT) | `notional_tax_rate`, `operating_taxes` | imposte figurative (teoriche) sul reddito operativo | Koller et al. ("operating taxes"); PIV | to verify |
| EBIT | `ebit` | reddito operativo (EBIT) | IFRS practice; CFA Institute curriculum | to verify |
| Depreciation and amortisation | `depreciation_and_amortisation` | ammortamenti | IAS 16, IAS 38 | to verify |
| Capital expenditure | `capital_expenditure` | investimenti in immobilizzazioni (capex) | CFA Institute curriculum | to verify |
| Change in net working capital | `change_in_net_working_capital` | variazione del capitale circolante netto | CFA Institute curriculum | glossary (net working capital); to verify |
| Interest expense | `interest_expense` | oneri finanziari | glossary (financial charges) | to verify |
| Net borrowing | `net_borrowing` | variazione netta dei debiti finanziari | CFA Institute curriculum (FCFE) | to verify |
| Debt-like items (debt equivalents) | `debt_like_items` | debt-like items (passività assimilabili al debito) | Koller et al. ("debt equivalents") | to verify |
| Non-operating assets | `non_operating_assets` | attività non operative (surplus asset) | Koller et al. | glossary; to verify |
| Non-controlling interests (minority interests) | `non_controlling_interests` | interessenze di minoranza (patrimonio netto di terzi) | IFRS 10, Appendix A | to verify |
| Equity bridge (from enterprise value to equity value) | `bridge` | passaggio dall'enterprise value all'equity value | Koller et al. | to verify |
| Terminal discounting time | `terminal_discount_time`: `horizon`, `mid_period` | momento di attualizzazione del valore terminale | Practice (mid-year convention) | to verify |
| Normalised flow | `normalised_next_flow` | flusso normalizzato | PIV | to verify |

## Open questions (Revised 2026-11-08)
Each item has a recommendation; answer "ok" or pick an option.

1. **Method ids and versions.** (a) `dcf.fcff` version 3 + new `dcf.fcfe` + new
   `dcf.free_cash_flows`, shared engine; (b) keep `dcf.fcff` version 2 frozen and add one
   polymorphic `dcf.value`. **Recommended: (a).** Reasons: one tool per basis gives field names
   that cannot be confused (`wacc` versus `cost_of_equity`) and simpler schemas for agents; two
   overlapping FCFF tools would make agents choose between them; nothing is released and
   `dcf.fcff` is Preview, so the break costs only the quickstart, the methodology page and the
   golden migration (which needs you, the directory is protected).
2. **Meaning of "stage 3".** (a) One to three explicit stages of given length, always followed
   by a terminal technique; a finite last phase uses `tv.limited_horizon`; (b) stage 3 is the
   terminal stage itself. **Recommended: (a).** It keeps every terminal technique available and
   reuses SPEC-0006 instead of adding a second notion of terminal stage.
3. **Form of the growth choice.** (a) A required `projection` per stage: `given`,
   `constant_growth` or `linear_fade`; (b) a boolean `apply_growth` plus a pattern field.
   **Recommended: (a).** A required enumeration is the explicit choice you asked for, has no
   invalid combinations and can grow without a new version; a boolean leaves the pattern
   undefined when true.
4. **Linear fade step.** (a) g_k = g_from + (g_to − g_from) × k / m: g_from is the growth before
   the stage and g_to is reached in the last period, as in "from 10% to 3% over 5 years":
   8.6, 7.2, 5.8, 4.4, 3.0; (b) first and last period growth given, linear in between: 10, 8.25,
   6.5, 4.75, 3. **Recommended: (a).** It matches the textbook wording of the transition stage
   and ends at the stable growth, so the terminal value continues smoothly; it must be verified
   on the published model chosen in question 12.
5. **Nominal and real.** Refuse a mismatch without inflation (`FM-E024`); with inflation,
   convert only the discount rates to the terms of the flows by the exact Fisher relation;
   never convert flows. **Recommended: ok.** Converting flows is not neutral under mid-period
   timing and, in a build-up, taxes on non-indexed depreciation differ between real and
   nominal plans; inflating line items can be added later as an explicit option.
6. **Terminal discounting time under mid-period.** Required input, no default, with a warning
   for an exit multiple or a liquidation discounted at N − 0.5; `timing` also becomes required
   (it defaulted to `end_of_period` in version 2). **Recommended: ok.** Practice varies by
   technique, and a default would hide a choice that moves the value by about half a period of
   the rate.
7. **Taxes on EBIT.** (a) Exactly one of a notional tax rate on EBIT or per-period tax amounts;
   (b) the rate only. **Recommended: (a).** The rate covers the common case; the amounts let an
   analyst include IRAP or a computed tax without engine logic, consistent with the SPEC-0003
   decision of 2026-11-07 (no IRAP logic in the engine).
8. **Negative EBIT.** Required choice: `credit` (tax = rate × EBIT, a saving used elsewhere in
   the group) or `zero` (no tax, no saving). **Recommended: required, no default.** The two
   practices give different values; tax loss carry-forwards stay out of scope.
9. **Bridge on the equity basis.** For `dcf.fcfe`, refuse a net financial position
   (`FM-E019`), accept non-operating assets, debt-like items and non-controlling interests.
   The bridge stays a reusable type inside the core; a standalone bridge method is decided with
   the market multiples spec (M4). **Recommended: ok.** Subtracting debt from an equity value is
   the double count most likely from an agent; a standalone id now would need renaming when
   multiples use it.
10. **Error codes.** New `FM-E024` (terms mismatch) and `FM-E025` (counts and base flow); widen
    `FM-E019` to the bridge. **Recommended: ok.** Dedicated codes let agents fix the call
    without parsing messages; `FM-E019` already means "enterprise and equity measures mixed".
11. **Scope trims.** Drop the one-stage method and the sensitivity grid (covered by `tv.gordon`,
    `tvm.perpetuity` and `analysis.sensitivity`); defer dated periods with a stub, the dividend
    basis and the rate fade. **Recommended: ok.** Each deferred item can be added within
    version 3 as an additive option; the stub needs its own roadmap task.
12. **Published examples for the golden cases.** Every case uses the 50-digit reference; in
    addition, one published worked example for the FCFF and FCFE build-up and one for a
    three-stage model. Candidates to check: the CFA Institute curriculum examples on FCFF and
    FCFE, and Damodaran's public three-stage FCFF material. **Recommended: you pick or approve
    the two sources;** the agents will not cite a source they could not verify.
13. **Terminal rate with one discount rate.** The terminal technique uses the same rate and no
    separate terminal rate is accepted; with rates per period, a terminal rate is required.
    **Recommended: ok.** A sensitivity on `/discount_rate` then moves the explicit and the
    terminal phase together, which is what the analyst means.
