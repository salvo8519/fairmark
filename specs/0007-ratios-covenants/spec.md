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
