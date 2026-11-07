# SPEC-0007 — Financial ratios and covenants, organised by analytical question

| | |
| --- | --- |
| Status | Draft — proposed for approval |
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

## Open questions
- [ ] Italian crisis indicators (CNDCEC: negative equity, six-month DSCR, sector indices). After
      Legislative Decree 83/2022 the Code of business crisis no longer makes the CNDCEC sector
      indices mandatory; they remain a professional reference. Before implementing them, verify
      the current official text (Normattiva) and the current CNDCEC document, and decide whether
      to expose them as a separate method `ratios.crisis_indicators`.
- [ ] ROE decomposition: which Italian form to adopt as the reference (the classic leverage
      formula with ROI, cost of debt and D/E, or the three-factor decomposition: margin, turnover, equity multiplier).
