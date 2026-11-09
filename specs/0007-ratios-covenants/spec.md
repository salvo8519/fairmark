# SPEC-0007 — Financial ratios and covenants, organised by analytical question

| | |
| --- | --- |
| Status | Approved — 2026-11-07, maintainer decisions recorded below |
| Milestone | M2 |
| Owner | @salvo8519 |
| Formula basis | Ratios commonly used in Italian and international credit and equity analysis; covenant definitions as written in loan agreements (each covenant states its own definition); verified on closed forms and published worked examples |
| Related | SPEC-0005 (reclassified statements feed the inputs), SPEC-0002 (LLCR discounting) |

## Problem
Analysts compute the same ratios in every engagement, each time in a slightly different way:
average or closing balances, 360 or 365 days, which debt is in the net financial position. A
lender reads a covenant breach from the same numbers. A library of ratios is useful only if
every definition is explicit and every ratio says how it was computed, and if the ratios are
organised by the question the analyst asks, not as a flat list of formulas.

## Scope
**In scope:** six method groups, each answering one question; explicit definitions and
conventions; ratios that cannot be computed (zero denominator) reported as not defined, with the
reason, without failing the whole group; covenant testing with headroom.
**Out of scope:** reclassification of statutory accounts (SPEC-0005 produces the inputs);
sector benchmarks; scoring models (for example Altman Z), possible later.

## Structure

| Question | Method id | Ratios |
| --- | --- | --- |
| Does it earn enough on what it uses? | `ratios.profitability` | ROE, ROI (operating), ROA, ROS, EBITDA margin, EBIT margin, capital turnover, ROE decomposition (ROI, leverage effect, tax and non-operating effects) |
| Is it financed soundly? | `ratios.capital_structure` | D/E, net financial position / equity, equity ratio, primary and secondary structure margin, fixed-asset coverage |
| Can it pay in the short term? | `ratios.liquidity` | current ratio, quick ratio, cash ratio, net working capital, treasury margin |
| How much does working capital tie up? | `ratios.working_capital` | days sales outstanding, days payable outstanding, days inventory outstanding, cash conversion cycle |
| Can it service its debt? | `ratios.debt_service` | interest cover (EBIT and EBITDA), net financial position / EBITDA, DSCR, LLCR |
| Does it respect its covenants? | `ratios.covenants` | any of the above against thresholds, with headroom |

## Requirements (EARS)
- **R1** — Each group SHALL take named aggregates (revenue, EBITDA, EBIT, net income, equity,
  invested capital, net financial position, current assets and liabilities, inventories,
  receivables, payables, interest, debt service, cash flow available for debt service) and return
  every ratio of the group with its formula and the numbers substituted in `explain`.
- **R2** — Balance-sheet denominators SHALL be computed on closing or average balances as an
  explicit choice; day-based ratios SHALL use an explicit day basis (360 or 365).
- **R3** — WHEN a denominator is zero or a ratio is not meaningful (for example negative equity
  in ROE), THE ENGINE SHALL report the ratio as not defined with the reason, and compute the
  others.
- **R4** — `ratios.debt_service` SHALL compute DSCR = CFADS / debt service per period and LLCR =
  present value of CFADS over the loan life / debt outstanding, with the discount rate explicit
  (SPEC-0002 for discounting).
- **R5** — `ratios.covenants` SHALL test a list of covenants, each with ratio, operator (≤, ≥),
  threshold and test date, and return for each the value, pass or breach, and headroom (absolute
  and as a percentage of the threshold).
- **R6** — The net financial position SHALL be an input with its composition stated (gross debt,
  cash, other items) and echoed in `explain`; Fairmark does not choose which items it includes.

## Conventions and parameters
| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing | required |
| Day basis | 360, 365 | required |
| Operating return base | invested capital (operating) or total assets | required for ROI |
| EBITDA definition | as given (the analyst states adjustments) | input |

## Acceptance criteria
- **AC1** (R1) — Each group on one closed-form example written out in the golden file.
- **AC2** (R2) — The same inputs with average and closing balances give the documented results.
- **AC3** (R3) — Zero denominators and negative equity produce not-defined ratios with reasons.
- **AC4** (R4) — DSCR and LLCR on a loan schedule match a closed-form computation.
- **AC5** (R5) — A covenant set with one breach reports the breach and the headroom of the others.

## Interfaces
- Method ids: as in the structure table.

## Decisions (maintainer, 2026-11-07)
- [x] Italian crisis indicators (CNDCEC: negative equity, six-month DSCR, sector indices):
      deferred. After Legislative Decree 83/2022 the sector indices are no longer mandatory and
      remain a professional reference; they are implemented only after verifying the current
      official text on Normattiva and the current CNDCEC document, as a separate method
      `ratios.crisis_indicators`.
- [x] ROE decomposition: both forms, as two outputs of the same method, so the analyst reads
      either without ambiguity: the leverage formula ROE = [ROI + (ROI − i) × D/E] × (1 − t)
      and the three-factor decomposition (margin × turnover × equity multiplier).

## Decisions (maintainer, 2026-11-08)
Recorded amendment, approved by the maintainer on 2026-11-08 on the open questions of
`plan.md`. Covenants remain a separate method and a separate issue.

- [x] Covenants (future `ratios.covenants`, not in this issue): the tested value comes from the
      numerator and denominator defined by the loan agreement; a ratio that cannot be computed
      is "not tested" with the reason, never a breach; the test date refers to the period ending
      on that date.
- [x] Leverage form of the ROE decomposition, a second output of `ratios.profitability`:
      ROE = [ROI + (ROI − i) × D/E] × (1 − t). The basis of D is a required input choice,
      `gross_financial_debt` or `net_financial_position`, with no default; i = financial
      charges / D; t = effective tax rate = taxes / pre-tax income. Its inputs (financial
      charges, taxes, pre-tax income, D on the chosen basis) form an optional block: without
      them the leverage form is not defined, with the reason. All conventions are stated.
- [x] ROA = operating income (EBIT) / total assets.
- [x] ROS = EBIT / revenue (confirmed).
- [x] Structure margins and fixed-asset coverage: primary and secondary, both (confirmed).
- [x] Average balances apply to every balance-sheet amount, numerator included (confirmed).
- [x] Not meaningful (value null with the reason): NFP / EBITDA when EBITDA ≤ 0; an interest
      cover when its numerator (EBITDA or EBIT) ≤ 0 or interest ≤ 0; ROI when invested capital
      ≤ 0; DSCR when debt service ≤ 0; LLCR when debt outstanding ≤ 0. Golden cases for each.
- [x] `debt_schedule` is optional in `ratios.debt_service`: without it DSCR and LLCR are not
      defined ("no debt schedule given") and the other ratios are computed.
- [x] LLCR: an optional `reserve_balance` (debt service reserve account) is added to the
      numerator, undiscounted at the valuation date: LLCR = (PV of CFADS + reserve balance) /
      debt outstanding. The discount rate is the cost of debt per period chosen by the analyst.
- [x] Golden tolerance: 1e-20 for every `ratios.*` golden case.

## Decisions (maintainer, 2026-11-08, covenant tests)
Recorded amendment, approved by the maintainer on 2026-11-08 on the open questions of the
covenant section of `plan.md` (issue #147). Principle: the engine never decides what the loan
agreement decides; such a choice is an explicit required input, so every outcome traces back to
a clause of the agreement (lender practice, LMA-style agreements).

- [x] Zero or negative denominator: a required per-covenant input `if_denominator_not_positive`
      with values `breached`, `passed` or `not_tested`, no default. The outcome follows it;
      value and headroom are null; when the outcome is `not_tested` the code is
      `denominator_not_positive` and the reason says whether the denominator is zero or
      negative. Explain and conventions state the rule applied and that it comes from the input.
- [x] Negative numerator: compared as computed (confirmed), stated as a convention.
- [x] Strict operators: `less_than` and `greater_than` are confirmed, an extension of R5 (which
      names ≤ and ≥) approved by the maintainer.
- [x] Contractual rounding: an optional per-covenant input `rounding` with `decimal_places`
      (0 to 6) and `mode` (`half_up` or `half_even`), both required when `rounding` is given,
      no default. When given, the tested value is rounded as the agreement states before the
      comparison and the outcome is decided exactly on the rounded value; explain shows the
      unrounded and the rounded value. Without it the outcome is decided exactly on the
      unrounded value.
- [x] Covenants on a single amount (minimum net worth, maximum capital expenditure): out of
      scope of `ratios.covenants`; a separate roadmap task.
- [x] Equity cure: out of scope in version 1. Step-downs are expressed as one covenant per test
      date, each with its own threshold.

## Amendment of 2026-11-09: ratios computed from partial input (issue #262)
Recorded amendment, approved by the maintainer on 2026-11-09 through the contract batch of
ADR 0014 (issue #262; decisions G, Q1 to Q5 of 2026-11-09). It applies guidelines G1 to G3 of
ADR 0014 to every `ratios.*` method and bumps each of them to version 2 (G8: output fields
become nullable and a `not_computed` list is added).

### Requirements
- **R7** — Every amount of every `ratios.*` input SHALL be optional. A ratio, margin or
  amount of the output SHALL be computed when every input it needs is given; otherwise it
  SHALL be null and listed in `not_computed` with the input pointers it lacks. A missing
  amount SHALL never be taken as zero.
- **R8** — Required inputs SHALL be only the method choices that change the meaning of the
  ratios: `balances` (every method), `operating_return_base` (`ratios.profitability`),
  `day_basis` (`ratios.working_capital`), `leverage.debt_basis` when `leverage` is given, and,
  for each covenant, `name`, `test_date`, `operator`, `threshold`,
  `if_denominator_not_positive` and the two `definition` texts.
- **R9** — `ebitda_definition` and `interest_definition` SHALL be optional. When an amount is
  given without its definition, the ratios that use it SHALL be not computed, with the pointer
  of the missing definition. A definition given blank SHALL remain `FM-E001`.
- **R10** — A ratio SHALL be reported as not defined (R3, `not_defined` text, unchanged) when
  the terms that are given already make it not defined, whether or not another term is
  missing; otherwise, when a term is missing, it SHALL be not computed. A ratio built from
  other ratios (cash conversion cycle, the ROE decompositions) SHALL be not defined when a
  part is not defined, otherwise not computed when a part is not computed, with the union of
  the missing pointers of those parts.
- **R11** — `ratios.covenants`: a covenant whose denominator amount is missing, or whose
  denominator is positive and numerator amount is missing, SHALL be `not_tested` with
  `not_tested_code` `missing_input`, never breached; a denominator given zero or negative
  SHALL give the outcome of `if_denominator_not_positive` whatever the numerator.

### Interfaces
- `not_computed`: a list at the root of each output value, in the order of the output
  fields, of `{output, missing, reason}`: `output` is the JSON pointer of the output in the
  result value (for a ratio, the ratio object, for example `/roe` or `/dscr/0`; for an amount,
  the amount); `missing` the JSON pointers of the inputs it lacks, sorted; `reason` one
  sentence. It is the shape `statements.reorganise` uses (shared type; `reason` is optional
  and omitted when empty, so the `statements.reorganise` output does not change). The later
  envelope task (#259) moves both to `diagnostics.not_computed`.
- `not_computed` lists only outputs null because an input is missing. Ratios not defined by
  their values keep the `not_defined` text of R3 (decision Q4); a ratio not computed has
  `value` and `not_defined` both null, except the two version 1 cases of a missing block
  (`leverage` absent, `debt_schedule` absent), which keep their `not_defined` text and are
  also listed in `not_computed` with the pointer of the block.
- Unchanged defaults, already documented in version 1: `other_items` of a net financial
  position absent means no other items; `reserve_balance` absent means no reserve account
  (LLCR on CFADS only); `opening` is not used with closing balances. They are reported in
  `explain.conventions` as before; their listing in `meta.defaults_applied` is left to the
  envelope task (#259).
- Departure from G1 (examples in the schema) and G4 (envelope): left to #259 and #264.
- Method versions: `ratios.profitability`, `ratios.capital_structure`, `ratios.liquidity`,
  `ratios.working_capital`, `ratios.debt_service` and `ratios.covenants` v1 -> v2. Inputs are
  relaxed (additive); outputs gain `not_computed`, and amounts that were always present become
  nullable (capital structure margins, liquidity margins, the net financial position).

### Computability
Balance-sheet amounts: with `balances` `closing` an amount `x` needs `/closing/x`; with
`average` it needs `/closing/x` and `/opening/x`. Below, "equity" stands for those pointers.
"Not computed" means null and listed in `not_computed`. No row has a default; no row raises an
error on a missing input.

`ratios.profitability`

| Output | Inputs it needs | When an input is missing |
| --- | --- | --- |
| `roe` | net income, equity | not computed |
| `roi` | EBIT, operating return base (invested capital or total assets) | not computed |
| `roa` | EBIT, total assets | not computed |
| `ros`, `ebit_margin` | EBIT, revenue | not computed |
| `ebitda_margin` | EBITDA, `ebitda_definition`, revenue | not computed |
| `capital_turnover` | revenue, operating return base | not computed |
| `roe_three_factor/net_margin` | net income, revenue | not computed |
| `roe_three_factor/asset_turnover` | revenue, total assets | not computed |
| `roe_three_factor/equity_multiplier` | total assets, equity | not computed |
| `roe_three_factor/roe` | the three factors | not computed (R10) |
| `roe_leverage/*` | `leverage` block | not computed, missing `/leverage`; `not_defined` "leverage inputs not given" as in v1 |
| `roe_leverage/roi` | as `roi` | not computed |
| `roe_leverage/cost_of_debt` | `leverage/financial_charges`, debt | not computed |
| `roe_leverage/debt_to_equity` | debt, equity | not computed |
| `roe_leverage/tax_rate` | `leverage/taxes`, `leverage/pre_tax_income` | not computed |
| `roe_leverage/leverage_effect`, `pre_tax_roe` | ROI, cost of debt, D/E | not computed (R10) |
| `roe_leverage/roe` | pre-tax ROE, tax rate | not computed (R10) |

`ratios.capital_structure` (gross debt and cash are those of `net_financial_position`)

| Output | Inputs it needs | When an input is missing |
| --- | --- | --- |
| `net_financial_position` | gross debt, cash (other items: none when absent) | not computed |
| `debt_to_equity` | gross debt, equity | not computed |
| `nfp_to_equity` | gross debt, cash, equity | not computed |
| `equity_ratio` | equity, total assets | not computed |
| `primary_structure_margin`, `primary_fixed_asset_coverage` | equity, fixed assets | not computed |
| `secondary_structure_margin`, `secondary_fixed_asset_coverage` | equity, non-current liabilities, fixed assets | not computed |

`ratios.liquidity`

| Output | Inputs it needs | When an input is missing |
| --- | --- | --- |
| `current_ratio`, `net_working_capital` | current assets, current liabilities | not computed |
| `quick_ratio`, `treasury_margin` | current assets, inventories, current liabilities | not computed |
| `cash_ratio` | cash, current liabilities | not computed |

`ratios.working_capital`

| Output | Inputs it needs | When an input is missing |
| --- | --- | --- |
| `days_sales_outstanding` | trade receivables, `income_statement/revenue` | not computed |
| `days_inventory_outstanding` | inventories, `income_statement/cost_of_sales` | not computed |
| `days_payable_outstanding` | trade payables, `income_statement/purchases` | not computed |
| `cash_conversion_cycle` | DSO, DIO, DPO | not computed (R10) |

`ratios.debt_service`

| Output | Inputs it needs | When an input is missing |
| --- | --- | --- |
| `net_financial_position` | gross debt, cash | not computed |
| `interest_cover_ebit` | EBIT, interest, `interest_definition` | not computed |
| `interest_cover_ebitda` | EBITDA, `ebitda_definition`, interest, `interest_definition` | not computed |
| `nfp_to_ebitda` | gross debt, cash, EBITDA, `ebitda_definition` | not computed |
| `dscr/t` | `debt_schedule/periods/t/cfads`, `/debt_service` | not computed; without `debt_schedule`, `dscr` is empty as in v1 |
| `pv_cfads` | `debt_schedule/discount_rate`, every `cfads` | not computed (missing `/debt_schedule` when no schedule is given) |
| `llcr` | as `pv_cfads`, `debt_schedule/debt_outstanding` (reserve balance: none when absent) | not computed; without `debt_schedule`, missing `/debt_schedule` and `not_defined` "no debt schedule given" as in v1 |

`ratios.covenants`

| Output | Inputs it needs | When an input is missing |
| --- | --- | --- |
| `covenants/i/value`, `headroom`, `headroom_relative` | `covenants/i/numerator/amount`, `covenants/i/denominator/amount` | not tested, code `missing_input` (R11), each output not computed |

### Acceptance criteria
- **AC6** (R7, R10) — For every row above, an engine test removes one input of a complete
  input and finds the row's output null and listed with that pointer, and every other output
  unchanged.
- **AC7** (R7) — Golden cases with partial inputs for each method, expected values from an
  independent reference (`scripts/reference_ratios_partial.py`).
- **AC8** (R7; decision G) — The three version 1 error cases
  `ratios.profitability/invested-capital-missing`, `leverage-without-debt` and
  `average-without-opening` return the partial results of the reference.
- **AC9** (R9) — EBITDA without `ebitda_definition` leaves the EBITDA ratios not computed with
  `/ebitda_definition`; a blank definition is `FM-E001`.
- **AC10** (R11) — A covenant without its numerator amount is not tested with code
  `missing_input`; the others are tested.
