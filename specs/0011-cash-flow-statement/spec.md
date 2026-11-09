# SPEC-0011 — Cash flow statement (OIC 10, indirect method) from the reorganised statements of consecutive financial years

| | |
| --- | --- |
| Status | Draft |
| Milestone | M2 (release 0.1.0); epic #34; issue #253 |
| Owner | @salvo8519 |
| Formula basis | The indirect method of OIC 10 (Appendix A, Schema 1) with the IAS 7 classification for the IFRS mapping, built from the changes between two reorganised balance sheets and the income statement of the later year; free cash flow to the firm as NOPAT − change in operating invested capital (Koller, Goedhart, Wessels, *Valuation*; SPEC-0001 R2). Verified against an independent exact decimal reference script and a synthetic two-year company built on the structure of OIC 10 Schema 1 and IAS 7 Illustrative Example A with changed numbers (maintainer decision of 2026-11-08 for #36) |
| Related | SPEC-0005 (`statements.reorganise`, R20, R23; issue #36), SPEC-0001 (`dcf.free_cash_flows`, `FreeCashFlowLines`), SPEC-0007 (`NetFinancialPosition`); ADR 0009 (typed calls), ADR 0012 (preview), ADR 0014 (G1 to G10); issues #36, #258, #259, #263, #266; practice audit of SPEC-0005 of 2026-11-09, section E |
| Commit scope | `statements` (existing in `scripts/conventions.py`) |

## Analysis

### What exists and what this spec needs
- `statements.reorganise` (SPEC-0005) returns, per year, the reorganised balance sheet
  (operating fixed assets, operating working capital, operating provisions, invested capital,
  non-operating assets, equity, quasi-equity, net financial position with composition, debt-like
  items) and the income statement with IFRS 18 aligned subtotals; R23 (issue #36, open) returns
  per pair of consecutive years `change_in_operating_working_capital`,
  `change_in_net_financial_position` and `change_in_equity`.
- The practice audit of 2026-11-09 (section E.2) recommends adding to R23
  `change_in_operating_fixed_assets`, `change_in_operating_provisions`,
  `change_in_invested_capital`, the changes of non-operating assets, debt-like items and
  quasi-equity, a funding check and an `implied_net_investment`. This spec consumes them; it does
  not define them (Open question 12).
- The statements v2 contract (issue #263; ADR 0014 maintainer decision 3) makes a closing `date`
  required for each year of a multi-year input and adds one amounts unit per call. This method
  takes the same `years`.
- `dcf.free_cash_flows` version 2 takes, per period, `FreeCashFlowLines`: `ebit`,
  `depreciation_and_amortisation` (zero or more), `capital_expenditure` (zero or more),
  `change_in_net_working_capital` (positive for an increase), `other_adjustments` (labelled,
  positive for an inflow), optional `operating_taxes`, `interest_expense`, `net_borrowing`; and
  once `notional_tax_rate` with `negative_ebit`, or `interest_tax_rate`.

### Option A: an extension of `statements.reorganise`
A `cash_flows` block next to `changes`, filled for every pair of consecutive years.
- For: one call; one source of every reorganised amount; no second schema.
- Against: every multi-year reorganisation carries the cash flow block, its conventions and its
  `not_computed` entries (several dozen lines per pair), also when the analyst asked only for
  ratios; the input grows by the cash flow choices (classification, movements, tax basis) that
  have no meaning for a reorganisation; a change of OIC 10 (the 2026 draft) would force a version
  of the reorganisation itself, already in the v2 batch of ADR 0014; the tool description for
  agents covers two intents.

### Option B: a new method `statements.cash_flows` on the same input (recommended)
A method that takes the `statements.reorganise` input unchanged plus three optional blocks
(classification, movements, tax basis for the free cash flows) and calls the reorganisation
engine through its typed function (ADR 0009).
- **Maintainability.** Separate versions: a change of OIC 10 bumps this method only; the
  reorganisation keeps its contract. Shared input types are reused, not copied (G9).
- **One envelope.** Each call returns one G4 envelope whose `diagnostics` speak only of its own
  outputs; numbers are identical by construction because the reorganised amounts come from the
  same engine (AC3).
- **Agents.** One tool per intent ("cash flow statement", "reorganise the statements"); the same
  `years` payload is passed verbatim to both; the cash flow schema stays small enough for strict
  tool modes.
- Cost: the reorganisation runs again inside the call (additions only; negligible).

Option C, a method that takes the *output* of `statements.reorganise` as input, is rejected:
error pointers would point into a derived document the user never wrote, and the income
statement items the cash flow needs (item 20, C15, C16, C17) would have to be carried twice.

## Problem
An analyst who values or lends to an Italian SME needs to see how the company generated and
used cash: OIC 10 requires the statement for ordinary filers, abridged and micro filers are
exempt (art. 2435-bis, art. 2435-ter), and banks rebuild it from the aggregates. A valuer needs
the historical free cash flows on the same basis as the forecast flows of `dcf.free_cash_flows`.
Both come from the same changes between consecutive balance sheets. The engine must build the
OIC 10 indirect statement from whatever totals are given, reconcile it with the change in cash
and with the change in the net financial position, derive the free cash flows consistently, state
every line it presents net for lack of detail, and never fill a missing amount with zero unless
a documented default applies.

## Scope
**In scope (version 1):** one method, `statements.cash_flows`, that takes two or more dated
consecutive financial years in the `statements.reorganise` input (statutory or reclassified form,
mixed allowed) and returns, for each pair of consecutive years:
- the OIC 10 indirect statement: A operating activities (profit-based reconciliation), B
  investing activities, C financing activities, increase or decrease in cash, cash at the start
  and at the end;
- the non-cash transactions excluded from it;
- the reconciliation with the change in the net financial position;
- the free cash flow lines in the `FreeCashFlowLines` type and, when a tax basis is given, the
  free cash flow to the firm and to equity computed by `dcf.free_cash_flows`.

**Out of scope:** the direct method (OIC 10 Schema 2); consolidated statements; the effect of
exchange rates on cash (not separable from aggregates; stated as a limitation); acquisitions and
disposals of business units (OIC 10 line to verify); forecast cash flows (SPEC-0001); NOPAT taxes
(SPEC-0001); the ratios built on cash flows (SPEC-0007, future); the DSCR of the CNDCEC
indicators (SPEC-0005 future extension 6).

**Future extensions (each additive):** the direct method; a `cash_equivalents` "of which" of
C.III for the IAS 7 definition; exchange rate effect when foreign currency cash is given; the
final text of the OIC 10 amendment if it differs from the draft (Open question 2).

## User stories
- As a credit analyst with two abridged filings, I want the operating, investing and financing
  flows from the totals I have, with the lines presented net named as such, so that I can judge
  the cash generation of a company that files no cash flow statement.
- As a valuer, I want the historical FCFF on the same lines that `dcf.free_cash_flows` takes, so
  that the base flow of my forecast is consistent with the history.
- As an AI agent, I want to pass the same `years` I passed to `statements.reorganise`, and read
  in `diagnostics` which lines are not computed and which input would complete them.
- As a reviewer, I want the change in cash and the change in the net financial position to
  reconcile line by line, with both balance sheet amounts substituted in `explain`.

## Requirements (EARS)

### Method and input
- **R1** — THE ENGINE SHALL provide the method `statements.cash_flows` version 1, status preview
  (ADR 0012), following ADR 0014 G1 to G10, computing every reorganised amount through the typed
  function of `statements.reorganise` on the same `years`, `treatments` and `adjustments`, so that
  each reorganised amount it uses equals the one `statements.reorganise` returns for the same
  input.
- **R2** — THE ENGINE SHALL take the `years`, `treatments` and `adjustments` of the
  `statements.reorganise` contract in force (version 2 after issue #263: a closing `date` for
  each year, one amounts unit per call), and three optional blocks: `classification` (R7),
  `movements` (R9) and `free_cash_flow_taxes` (R18). A year without its date, or an ambiguous
  amounts unit, SHALL be refused as in that contract (G10.4).
- **R3** — THE ENGINE SHALL sort the years by date and build one statement for each pair of
  consecutive years (consecutive as defined by SPEC-0005 R2, end-of-month rule included),
  labelled by the closing dates of both years; the flows SHALL be those of the financial year
  ending on the later date.
- **R4** — (Floor, ADR 0014 G10.) IF fewer than two years are given, or no pair has the cash
  (C.IV, or the cash of the reclassified net financial position) of both years and the profit or
  loss of the later year, THEN THE ENGINE SHALL refuse the input with the floor error code of
  G10 (Open question 9), listing the alternative minimum sets in `missing_any_of` and stating the
  fix.

### Classification
- **R5** — THE ENGINE SHALL assign the change of every balance sheet aggregate between the two
  years, and every income statement item of the later year, to exactly one line of Appendix A,
  by its OIC 10 nature: financial fixed assets (B.III) and current financial assets (C.III) to
  investing whatever their reorganisation treatment; financial debt and shareholder loans to
  financing whatever their treatment (`financial_debt` or `quasi_equity`); provisions and the
  employee severance indemnity to operating, also when treated as debt-like; dividends declared
  and not paid (SPEC-0005 R22) to financing when paid. The reorganisation treatments SHALL change
  only the free cash flow lines (R16), never the OIC 10 classification; the convention SHALL say
  so.
- **R6** — Cash SHALL be C.IV in full in both years (OIC 10 paragraph 11); an `operating_cash`
  adjustment of the reorganisation SHALL NOT move any amount out of cash in this statement, and
  the convention SHALL say so. C.III SHALL NOT be cash.
- **R7** — THE ENGINE SHALL take the convention `classification.interest_and_dividends`, an
  enumeration:
  - `operating`: interest paid, interest received and dividends received in operating
    activities (OIC 10, December 2016; one of the options of IAS 7 paragraphs 31 to 34, to
    verify);
  - `by_source`: interest paid in financing activities, interest and dividends received in
    investing activities (OIC 10 consultation draft of 2026, verified as a draft by the practice
    audit; IAS 7 as amended by IFRS 18, paragraphs to verify).
  Dividends paid SHALL be in financing activities under both. The default SHALL be decided by
  the maintainer (Open question 2); the value used SHALL be reported in `explain.conventions` and,
  when it is a default, in `meta.defaults_applied`.
- **R8** — THE ENGINE SHALL exclude non-cash transactions from the three sections and list them
  in a `non_cash_transactions` block (IAS 7 paragraph 43, to verify; OIC 10, paragraph to
  verify): right-of-use assets acquired under a restated lease (from the `lease` adjustment:
  change in the lease asset + lease depreciation of the later year), revaluations of fixed assets
  credited to equity when given (R9), write-downs and revaluations (B10 c, D18, D19), and
  conversions of shareholder loans into equity when given.

### Movements not visible in two balance sheets
- **R9** — THE ENGINE SHALL accept `movements`, a list keyed by the `date` of the later year of a
  pair (G6), each entry with optional amounts in the unit of the call: `interest_paid`,
  `interest_received`, `dividends_received`, `income_taxes_paid`, `use_of_provisions`,
  `disposals` (`proceeds`, `carrying_amount`), `revaluations` (credited to equity),
  `loans_raised`, `loans_repaid`, `capital_contributions`, `capital_repaid`, `dividends_paid`,
  `shareholder_loans_converted_to_equity`; all zero or more. A `date` that is not the later date
  of a pair, or a date given twice, SHALL be `FM-E001` with the pointer and the valid dates in
  `expected`; a negative amount SHALL be `FM-E001`.
- **R10** — WHEN a movement is given, THE ENGINE SHALL present the gross lines it allows
  (investments and disinvestments, loans raised and repaid, use of provisions, taxes and interest
  paid) and SHALL take its counterpart out of the net line where it would otherwise sit, so that
  every section total is unchanged by the detail. IF a given movement makes a derived gross
  amount negative (for example a `carrying_amount` of disposals that makes the gross investment
  in fixed assets negative, or `loans_repaid` that makes `loans_raised` implied negative), THEN
  THE ENGINE SHALL return `FM-E026` with both amounts, the pointers and the fix.
- **R11** — WHEN a movement is not given, THE ENGINE SHALL present the net line of Appendix A
  under its own name (for example `change_in_financial_debt` instead of `loans_raised` and
  `loans_repaid`; `change_in_provisions` instead of accruals and use), list the gross lines in
  `diagnostics.not_computed` with the movement pointer in `missing_any_of`, and SHALL NOT
  introduce a zero. The cash amounts of interest, dividends and taxes, and the results of
  disposals and revaluations, SHALL follow the computability table (documented default or not
  computed, Open questions 3 to 5).

### Operating activities (A)
- **R12** — THE ENGINE SHALL return, in the order of Appendix A.1: the profit or loss of the
  later year; income taxes, net interest, dividends received and results of disposals added back
  (line 1, profit before income taxes, interest, dividends and gains or losses on disposals); the
  non-cash adjustments with no counterpart in working capital (line 2); the changes in working
  capital (line 3); the other adjustments (cash amounts of interest, taxes, dividends, use of
  provisions); and `operating_activities` (A). Profit before tax SHALL also be returned as a line,
  so that the presentation of the 2026 draft (starting from profit before tax) can be read from
  the same output (Open question 16).
- **R13** — The changes in working capital SHALL be computed from the operating working capital
  of the reorganisation with cash (R6) and the counterparts of R10 removed, as: inventories;
  trade receivables and trade payables when their trade parts are given in both years;
  prepayments and accrued income; accruals and deferred income; and
  `other_changes_in_working_capital` with its composition listed in `explain`. A detail line
  whose parts are missing SHALL be not computed and its amount SHALL stay in the other line, as
  reported. Write-downs of receivables (B10 d) SHALL NOT be added back: they are within the
  change in receivables, net of the allowance.

### Investing activities (B)
- **R14** — THE ENGINE SHALL return the investing lines of Appendix A.2: net investment in
  tangible and intangible fixed assets = −(change in operating fixed assets + depreciation and
  amortisation + write-downs of fixed assets − revaluations given − right-of-use assets acquired),
  split into investments and disinvestments when `disposals` is given; change in financial fixed
  assets; change in current financial assets (C.III); change in non-operating assets other than
  financial; under `by_source`, interest and dividends received; and `investing_activities` (B).

### Financing activities (C)
- **R15** — THE ENGINE SHALL return the financing lines of Appendix A.3: change in financial debt
  (or loans raised and repaid when given), lease principal repaid (change in the lease liability −
  right-of-use assets acquired), change in shareholder loans, capital contributions and repaid,
  dividends paid (including the change in dividends declared and not paid), other equity
  movements = change in equity − profit of the later year − the equity lines given − non-cash
  equity movements; under `by_source`, interest paid; and `financing_activities` (C).

### Cash and the net financial position
- **R16** — THE ENGINE SHALL return `change_in_cash` = A + B + C, `cash_at_start` and
  `cash_at_end`, and the check `change_in_cash_equals_flows` in `diagnostics.checks`: `passed`
  when A, B, C and both cash amounts are computed, `not_checked` otherwise, with the pointers.
  Because every line is a change of a given aggregate, a difference SHALL be an internal invariant
  (`FM-E012`), not a user error.
- **R17** — THE ENGINE SHALL return the reconciliation with the net financial position:
  − change in net financial position = A + B excluding the changes of financial assets inside the
  net financial position + C excluding the principal movements of items inside the net financial
  position − non-cash changes of the net financial position (right-of-use liabilities, accrued
  interest moved by adjustment, write-downs of financial assets inside it), each line labelled,
  and the check `net_financial_position_reconciles` against `change_in_net_financial_position` of
  SPEC-0005 R23: `passed`, or `not_checked` when a side is missing; a difference is `FM-E012`.

### Free cash flows
- **R18** — THE ENGINE SHALL return, for each pair, `free_cash_flow_lines` in the
  `FreeCashFlowLines` type of `dcf.free_cash_flows` (G9, type reused):
  `ebit` = `operating_profit` (SPEC-0005 R20); `depreciation_and_amortisation` as reorganised
  (B10 c included when `operating`); `capital_expenditure` = implied investment in operating fixed
  assets = change in operating fixed assets + depreciation and amortisation + write-downs of
  fixed assets − revaluations given (right-of-use assets acquired stay in it, Open question 7),
  when zero or more; `change_in_net_working_capital` = `change_in_operating_working_capital`
  (SPEC-0005 R23); `other_adjustments`: the change in operating provisions (positive for an
  increase) and the net disinvestment as a positive inflow when the implied investment is
  negative (then `capital_expenditure` is zero, as SPEC-0001 R5 requires). B10 c with the
  treatment `investing` is not in `ebit` and SHALL NOT be added back. THE ENGINE SHALL hold the identity
  FCFF = NOPAT − (change in invested capital − non-cash changes in invested capital) exactly
  (property, AC10).
- **R19** — WHEN `free_cash_flow_taxes` is given (`kind` `notional_rate` with
  `notional_tax_rate` and `negative_ebit`, or `kind` `amounts` with `operating_taxes` keyed by
  date and `interest_tax_rate`), THE ENGINE SHALL compute `free_cash_flow_to_firm` and
  `free_cash_flow_to_equity` through the typed function of `dcf.free_cash_flows`, with
  `interest_expense` = `interest_and_other_financial_charges` of the later year and
  `net_borrowing` = change in `net_financial_position.gross_debt`; it SHALL record the method
  version of `dcf.free_cash_flows` used. WHEN it is not given, both SHALL be not computed with
  the pointer `/free_cash_flow_taxes`; no tax rate SHALL be assumed.

### Reporting, best effort, errors
- **R20** — THE ENGINE SHALL report in `explain` one step per line with the amounts of both years
  substituted (or the income statement item), the convention of every net line, and the method
  version of `statements.reorganise` used; every default in `meta.defaults_applied`
  (`{pointer, value, reason}`); every warning in `diagnostics.warnings` with a stable code
  (Interfaces).
- **R21** — WHEN an output cannot be computed because an input it needs is missing, THE ENGINE
  SHALL return it as `null` with a `diagnostics.not_computed` entry (`reason_code`,
  `missing_any_of`), compute every other output, and treat a missing amount as zero only where
  the computability table names a documented default. A pair with a reclassified year SHALL give
  the lines its totals allow (Appendix A, column "Reclassified") and list the detail lines as not
  computed.
- **R22** — The arithmetic SHALL be additions and subtractions of decimals, with no rounding and
  no tolerance; the same input SHALL give the same output bit for bit. Errors SHALL follow ADR
  0014 G5, reusing `FM-E001`, `FM-E012`, `FM-E026` and the floor code (R4).

## Conventions and parameters

| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Years and pairs | sorted by date; one statement per pair of consecutive years; flows of the later year | fixed | SPEC-0005 R2; ADR 0014 decision 3 |
| Amounts unit | one per call, never scaled | required as in statements v2 | ADR 0014 G10.4; issue #263 |
| Method | indirect, from the profit or loss of the later year | fixed | OIC 10 Schema 1 (V, practice audit); IAS 7 paragraphs 18(b), 20 (to verify) |
| Cash | C.IV in full; no cash equivalents; `operating_cash` adjustment ignored | fixed | OIC 10 paragraph 11 (V, practice audit) |
| Classification basis | OIC 10 nature of each aggregate; reorganisation treatments affect only the free cash flow lines | fixed | OIC 10 Schema 1; R5 |
| Interest and dividends | `operating`; `by_source` | to be decided (Open question 2; recommended `operating`) | OIC 10 (2016) and IAS 7 paragraphs 31 to 34 (to verify); OIC 10 consultation draft 2026 (V as a draft; final text not verified); IAS 7 as amended by IFRS 18 (to verify) |
| Dividends paid | financing | fixed | OIC 10 Schema 1; IAS 7 paragraph 34 (to verify) |
| Income taxes | operating | fixed | OIC 10 Schema 1; IAS 7 paragraph 35 (to verify) |
| Cash amounts of interest, dividends, taxes when not given | accrual amount of the later year, difference left in working capital; or not computed | to be decided (Open question 3) | to verify; no source found |
| Results of disposals when not given | none, investing net of disposals at carrying amount; or not computed | to be decided (Open question 4) | to verify; no source found |
| Revaluations credited to equity when not given | none, with a warning when the revaluation reserve (A.III) increased | to be decided (Open question 5) | Banca d'Italia note on DL 104/2020 revaluations (V, practice audit) for the distortion |
| Gross movements (loans, provisions, equity) when not given | net change line under its own name | fixed (no default: a computed net line) | R11 |
| Non-cash transactions | excluded and listed | fixed | IAS 7 paragraph 43 (to verify); OIC 10 (paragraph to verify) |
| Leases restated by adjustment | right-of-use additions non-cash in the statement; lease principal in financing; in the free cash flow lines, additions are investment | fixed in the statement; FCFF to be decided (Open question 7) | IAS 7 paragraph 43 (to verify); Koller et al. (leases as debt, to verify) |
| Exchange rate effect on cash | not separated; included in the flows | fixed, stated as a limitation | OIC 10 Schema 1 line (to verify) |
| FCFF basis | `ebit` = `operating_profit`; capital expenditure implied from the balance sheets; taxes only through `dcf.free_cash_flows` | fixed | SPEC-0001 R2; SPEC-0005 R20; practice audit E.2 |
| FCFE financing lines | interest expense = `interest_and_other_financial_charges`; net borrowing = change in gross debt | fixed | SPEC-0001 R3 |

## Computability, defaults and checks
Pointers: input `/years/<i>/...` as in the `statements.reorganise` contract in force;
`/movements/<k>/...`; output `/value/pairs/<p>/...`. "Both years" means the earlier and the later
year of the pair; "later" means the later year only. A documented default marked "pending" is
admitted only if the maintainer accepts its practice basis (ADR 0014 G3 test 1); until then the
row reads "not computed".

| Output | Inputs it needs | When an input is missing | Checks it takes part in |
| --- | --- | --- | --- |
| `cash_at_start`, `cash_at_end` | C.IV of each year (or reclassified `net_financial_position.cash`) | not computed | `change_in_cash_equals_flows` |
| `change_in_cash` | cash of both years | not computed | `change_in_cash_equals_flows` |
| `profit_or_loss` | item 21 (or reclassified `net_income`), later | not computed; below the floor when no pair has it (R4) | none |
| `income_taxes` (added back) | item 20, later | not computed; line 1 and A not computed | none |
| `net_interest` (added back) | C16, C17, C17-bis core and lease interest (the financing and investing items of SPEC-0005), later | not computed; line 1 and A not computed | none |
| `dividends_received` (added back) | C15, later | not computed; line 1 and A not computed | none |
| `gains_losses_on_disposals` | `disposals` | pending default: none, listed, warning `disposal_results_in_operating`; otherwise not computed with line 1, A and B (Open question 4) | `disposals_within_investment` (R10) |
| `profit_before_tax` | item 21 and item 20, or the reorganised `profit_before_tax`, later | not computed | none |
| line 1 | the four rows above | not computed | none |
| `depreciation_and_amortisation` | reorganised `depreciation_and_amortisation`, later | not computed; line 2, A and B not computed | none |
| `write_downs_and_revaluations` | B10 c (when not in D&A), D18, D19, later | not computed; line 2 and A not computed | none |
| `change_in_provisions` (net) | operating provisions and debt-like provisions, both years | not computed; line 2 and A not computed | none |
| `accruals_to_provisions`, `use_of_provisions` | `use_of_provisions` and the net change | not computed (net line used) | none |
| line 2 | line 1 and the three rows above | not computed | none |
| `change_in_inventories` | C.I, both years | not computed; amount stays in no line, line 3 not computed | `working_capital_lines_sum` |
| `change_in_trade_receivables`, `change_in_trade_payables` | trade parts of C.II and D, both years | not computed; the amount stays in `other_changes_in_working_capital`, reported | `working_capital_lines_sum` |
| `change_in_prepayments_and_accrued_income`, `change_in_accruals_and_deferred_income` | assets D and liabilities E (or their "of which" parts, default none, SPEC-0005 D2), both years | not computed | `working_capital_lines_sum` |
| `other_changes_in_working_capital` | reorganised operating working capital of both years, cash removed (R6) | not computed; line 3 and A not computed | `working_capital_lines_sum` |
| line 3 | line 2 and the change in operating working capital | not computed | none |
| `interest_paid`, `interest_received`, `dividends_received_in_cash`, `income_taxes_paid` | the movement, or the accrual amount | pending default: accrual amount of the later year, listed; warning `accrual_basis_cash_amounts` only under `by_source` for interest and dividends (it moves amounts between sections); otherwise not computed with A, and with B and C under `by_source` (Open question 3) | none |
| `operating_activities` (A) | line 3 and the other adjustments | not computed | `change_in_cash_equals_flows`, `net_financial_position_reconciles` |
| `net_investment_in_fixed_assets` | operating fixed assets of both years and of the non-operating fixed assets moved by adjustment; D&A and B10 c, later; `revaluations` | not computed; revaluations: pending default none, listed, warning `revaluation_reserve_increase` when A.III is given and increased (Open question 5) | `disposals_within_investment` |
| `investments_in_fixed_assets`, `disinvestments_of_fixed_assets` | the row above and `disposals` | not computed (net line used) | `disposals_within_investment` |
| `change_in_financial_fixed_assets` | B.III, both years | not computed; B not computed | none |
| `change_in_current_financial_assets` | C.III, both years | not computed; B not computed | none |
| `investing_activities` (B) | the rows above; under `by_source` interest and dividends received | not computed | `change_in_cash_equals_flows`, `net_financial_position_reconciles` |
| `change_in_financial_debt` or `loans_raised`, `loans_repaid` | financial debt part of D, both years; the movements | not computed (D not split, SPEC-0005 Decision 4); C not computed | `loans_within_change` (R10) |
| `lease_principal_repaid` | `lease` adjustment amounts, both years | zero with a convention when no lease adjustment is given (nothing restated, nothing to repay in the statement) | none |
| `change_in_shareholder_loans` | shareholder loans part of D, both years | not computed; C not computed | none |
| `capital_contributions`, `capital_repaid`, `dividends_paid` | the movements; dividends declared, both years | not computed (net line used) | none |
| `other_equity_movements` | equity of both years, profit or loss of the later year, the equity lines given, non-cash equity movements | not computed; C not computed | none |
| `financing_activities` (C) | the rows above; under `by_source` interest paid | not computed | `change_in_cash_equals_flows`, `net_financial_position_reconciles` |
| `non_cash_transactions` | `lease` adjustment, `revaluations`, B10 c, D18, D19, `shareholder_loans_converted_to_equity` | each item not computed when its input is missing; an empty list means none was given and is stated | none |
| `net_financial_position_reconciliation` | A, B, C, the NFP composition of both years | not computed | `net_financial_position_reconciles` (against SPEC-0005 R23) |
| `free_cash_flow_lines` | `operating_profit`, D&A, B10 c, revaluations, both years of operating fixed assets, R23 changes in operating working capital and operating provisions | each line not computed when its inputs are; the object is returned only when every required field of `FreeCashFlowLines` is computed | `fcff_identity` (property) |
| `free_cash_flow_to_firm` | `free_cash_flow_lines`, `free_cash_flow_taxes` | not computed with `/free_cash_flow_taxes`; no default (reported taxes are not taxes on EBIT) | none |
| `free_cash_flow_to_equity` | as above, `interest_and_other_financial_charges`, gross debt of both years | not computed | none |
| Convention `classification.interest_and_dividends` | none | per Open question 2: documented default `operating`, listed, warning `classification_default` (it determines A, B and C); or `choice_required` for A, B, C with the permitted values | none |

Rules: never zero for a missing input unless the row says it is a documented default; an item
the user gave in another position counts as given (SPEC-0005 R6); an error raised on a missing
input names only pointers the user can act on, and the fix it states resolves the error.

## Acceptance criteria
Every golden case compares exactly (tolerance 0). Expected values come from the reference script
`scripts/reference_cash_flows.py` (exact decimals, 50 significant digits), written by
`test-engineer` from this spec and Appendices A and B without reading the Rust code, unless
stated otherwise.

- **AC1** (R1, R12 to R17) — Golden `synthetic-ordinary.yaml`: a synthetic two-year company in
  the ordinary statutory layout, with every movement of R9 given, built on the structure of OIC
  10 Schema 1 and IAS 7 Illustrative Example A with every number changed and the balance sheet
  re-balanced, so that it is not a copy (maintainer decision of 2026-11-08 for #36); the file
  names both sources and states the changes. Every line of Appendix A matches the reference; A +
  B + C equals the change in C.IV.
- **AC2** (R7) — Golden `synthetic-by-source.yaml`: the AC1 company with `by_source`; A, B and C
  differ from AC1 exactly by interest paid, interest received and dividends received; the change
  in cash is identical. Property: for any valid input, `change_in_cash` does not depend on the
  classification.
- **AC3** (R1) — Contract test: for every golden input of this method, each reorganised amount in
  `explain` equals the output of `statements.reorganise` on the same `years`, `treatments` and
  `adjustments`.
- **AC4** (R11, R21) — Golden `totals-only-abridged.yaml`: the AC1 company in the abridged layout
  with no `movements`: A, B, C and the change in cash equal AC1 (under `operating`, and given the
  decisions of Open questions 3 to 5); gross lines are listed in `not_computed` with their
  movement pointers; every documented default is in `meta.defaults_applied`.
- **AC5** (R21) — Golden `reclassified-totals.yaml`: the AC1 company in the reclassified form
  gives the same A, B, C, change in cash and free cash flow lines; detail lines are not computed.
  A mix of a statutory and a reclassified year gives the lines the coarser year allows.
- **AC6** (R3) — Golden `three-years.yaml`: two statements; property: the sum of the changes in
  cash equals the cash of the last year minus the cash of the first.
- **AC7** (R8, R15, R18) — Golden `lease-restatement.yaml`: right-of-use additions are in
  `non_cash_transactions`, lease principal repaid in C; in `free_cash_flow_lines` the additions
  are in `capital_expenditure` and in `net_borrowing` (per Open question 7).
- **AC8** (R8, R9, R14) — Golden `revaluation.yaml`: a revaluation credited to A.III given as a
  movement is excluded from B and from the equity movements; the same input without the movement
  shows the warning `revaluation_reserve_increase` (or the not-computed rows, per Open question 5).
- **AC9** (R16, R17) — Properties on random valid inputs: `change_in_cash` = A + B + C; the
  reconciliation of R17 equals − `change_in_net_financial_position` of SPEC-0005 R23; both checks
  are `passed` when every side is computed and `not_checked` with pointers when a side is missing.
- **AC10** (R18) — Property: FCFF computed by `dcf.free_cash_flows` from `free_cash_flow_lines`
  with a notional rate equals operating profit × (1 − rate) − (change in invested capital −
  non-cash changes in invested capital) for non-negative operating profit, exactly.
- **AC11** (R19; ADR 0014 G9) — Golden chain `chain-dcf-free-cash-flows.yaml`: the
  `free_cash_flow_lines` of AC1 passed unchanged to `dcf.free_cash_flows` give the
  `free_cash_flow_to_firm` and `free_cash_flow_to_equity` of this method; without
  `free_cash_flow_taxes` both are `null` with `/free_cash_flow_taxes` in `missing_any_of`.
- **AC12** (R4) — Error cases: one year; two years without the cash of either year; no profit
  of the later year: the floor code with `missing_any_of` and the fix. Years without dates and an
  ambiguous unit are refused with the codes of the statements v2 contract.
- **AC13** (R9, R10) — Error cases: a movement `date` that is not the later date of a pair
  (`FM-E001`, `expected` lists the valid dates); a movement given twice for a date; a negative
  movement amount; disposals with a carrying amount larger than the gross investment allows
  (`FM-E026`, both pointers).
- **AC14** (R21; template AC4) — For every row of the computability table, one case with the
  input missing and, for every documented default admitted by the maintainer, one case that shows
  it in `meta.defaults_applied` and, where the table says so, its warning.
- **AC15** (R20, R22) — Unit tests: every line has an explain step with both amounts; every
  convention of the table appears when it applies; two runs give identical output; no boolean in
  the schema; every error carries the G5 fields.

## Interfaces
- Contract: follows ADR 0014 G1 to G10. Departures: none intended. Dependencies on the contract
  work: envelope with `diagnostics` (#259), structured errors (#258), null equals absent (#266),
  statements v2 years and unit (#263); and SPEC-0005 R23 with the audit additions (#36). This
  method is implemented after them (Open question 10).
- Minimum input (floor, ADR 0014 G10): two years with closing dates one financial year apart, the
  amounts unit, the cash of both years and the profit or loss of the later year. Headline output:
  the statement of the pair (A, B, C and the change in cash). Alternative minimum sets listed in
  the floor error: `[/years/<i>/statutory/balance_sheet/cash, /years/<j>/statutory/balance_sheet/cash,
  /years/<j>/statutory/income_statement/profit_or_loss_for_the_year]` or the reclassified
  equivalents (`net_financial_position/cash`, `income_statement/net_income`). With only the
  minimum, the result carries the change in cash and the profit, and lists every other line in
  `not_computed`; it is never a result made only of `null` values.
- Maximal input example (G1): the AC1 input (ordinary layout, every movement, `by_source`,
  `free_cash_flow_taxes`).
- Method id: `statements.cash_flows` version 1, new, preview; entry `"0011"` in
  `specs/method-evidence.yaml`.
- Input: the `statements.reorganise` input (types reused) plus
  `classification: {interest_and_dividends}`, `movements: [{date, interest_paid?, …}]`,
  `free_cash_flow_taxes: {kind, notional_tax_rate?, negative_ebit?, operating_taxes?: [{date,
  value}], interest_tax_rate?}`. At most four levels below the root (G6).
- Input schema changes: additive (one new method, `schemas/statements.cash_flows/v1.json` and
  `v1.output.json`, one entry in `schemas/index.json`). No change to `statements.reorganise`
  beyond R23 of issue #36, which SPEC-0005 owns.
- Output fields: `pairs[]`, each `{from_date, to_date, operating_activities: {lines…, total},
  investing_activities: {lines…, total}, financing_activities: {lines…, total}, change_in_cash,
  cash_at_start, cash_at_end, non_cash_transactions[], net_financial_position_reconciliation,
  free_cash_flow_lines, free_cash_flow_to_firm, free_cash_flow_to_equity}`. Every line is a
  `label` + `value` object with its OIC 10 reference (G6).
- Warning codes (proposed, stable once approved): `classification_default`,
  `accrual_basis_cash_amounts`, `disposal_results_in_operating`, `revaluation_reserve_increase`,
  `exchange_effect_not_separated`.
- Check ids: `change_in_cash_equals_flows`, `net_financial_position_reconciles`,
  `working_capital_lines_sum`, `disposals_within_investment`, `loans_within_change`.
- Errors: `FM-E001`, `FM-E012`, `FM-E026` reused; the floor code of ADR 0014 G10 (Open question 9).
  No other new code.
- Building blocks reused: the typed functions of `statements.reorganise` and
  `dcf.free_cash_flows` (ADR 0009); `FreeCashFlowLines`; `ratios::NetFinancialPosition`;
  `Explain`; `crate::num`; the registry. Python helper `cash_flows` in
  `python/fairmark/statements.py`; methodology page `docs/en/methods/statements-cash-flows.md`.

## Terminology and sources
| Term | Identifier | Italian equivalent | Source | Status |
| --- | --- | --- | --- | --- |
| Statement of cash flows | `statements.cash_flows` | rendiconto finanziario | IAS 7 title; OIC 10 title | to verify (IAS 7 text not read in this session) |
| Cash flows from operating activities | `operating_activities` | flusso finanziario dell'attività operativa (A) | IAS 7 paragraph 6; OIC 10 Schema 1 | OIC 10 Schema 1 verified by the practice audit; wording and IAS 7 paragraph to verify |
| Cash flows from investing activities | `investing_activities` | flusso finanziario dell'attività di investimento (B) | as above | as above |
| Cash flows from financing activities | `financing_activities` | flusso finanziario dell'attività di finanziamento (C) | as above | as above |
| Increase (decrease) in cash | `change_in_cash` | incremento (decremento) delle disponibilità liquide | OIC 10 Schema 1 | wording to verify |
| Cash | `cash_at_start`, `cash_at_end` | disponibilità liquide | OIC 10 paragraph 11 | verified (practice audit) |
| Non-cash transactions | `non_cash_transactions` | operazioni non monetarie | IAS 7 paragraph 43 | to verify |
| Free cash flow to the firm, to equity | `free_cash_flow_to_firm`, `free_cash_flow_to_equity` | as in SPEC-0001 | SPEC-0001 [S1] | glossary |

Sources:
- OIC 10 *Rendiconto finanziario*, December 2016, paragraph 11, paragraphs 41 to 43, Appendix A
  Schema 1: https://www.fondazioneoic.eu/wp-content/uploads/2026/01/2016-12-OIC-10-Rendiconto-finanziario.pdf
  (verified by the practice audit of 2026-11-09; the content of paragraphs 41 to 43 is assumed to
  be the classification of interest and dividends: to verify).
- OIC 10, consultation draft of amendments, 2026, comments by 31 July 2026, Proposals 1 to 5
  (start from profit before tax; interest paid in financing; interest and dividends received in
  investing; reverse factoring): https://www.dirittobancario.it/wp-content/uploads/2026/05/Bozza-in-consultazione-modifiche-al-pricnipio-contabile-OIC-10-Rendiconto-finanziario.pdf
  (verified as a draft by the practice audit; final text not verified).
- IAS 7 *Statement of Cash Flows*, paragraphs 6, 18 to 20, 31 to 36, 43, 44A to 44E, Illustrative
  Example A; and its amendments by IFRS 18 on interest and dividends: to verify (not read in this
  session).
- Koller, Goedhart, Wessels, *Valuation*, 7th edition: FCFF = NOPAT − increase in invested
  capital; leases as debt: to verify (not read in this session; SPEC-0001 [S12] covers chapter 16
  only).

## Open questions
Answered by the maintainer on 2026-11-09 (PR #278). The spec stays Draft until the maintainer
approves the full text; the answers below are binding for that text.

- [x] **1. Method form.** New method `statements.cash_flows` on the same input as
      `statements.reorganise` (Option B).
- [x] **2. Default classification of interest and dividends.** Documented default `operating`
      (OIC 10, 2016), reported in `meta.defaults_applied` with the warning
      `classification_default`; enum values `operating` and `by_source` name the content.
- [x] **3. Cash amounts of interest, dividends and taxes when not given.** No accrual default
      (ADR 0014 test 1 not met). The lines "interest paid", "dividends paid" and "taxes paid" are
      not computed until the analyst gives the cash amounts. Section totals that do not depend on
      them are computed: under `operating` the totals of A, B and C and the change in cash; under
      `by_source` the change in cash, while A and C are not computed without the cash amounts of
      interest and dividends.
- [x] **4. Results of disposals when not given.** Documented default: no disposals, reported in
      `meta.defaults_applied`, with the warning `disposal_results_in_operating` on A and B (the
      change in cash is exact). The maintainer accepts it as common credit-analysis practice
      when only two balance sheets are available.
- [x] **5. Revaluations credited to equity when not given.** None, listed, with the warning
      `revaluation_reserve_increase` when A.III is given in both years and increased.
- [x] **6. Free cash flows inside the method.** FCFF and FCFE through `dcf.free_cash_flows` only
      when `free_cash_flow_taxes` is given; no tax rate is ever assumed.
- [x] **7. Leases in the free cash flow lines.** Right-of-use additions are capital expenditure
      and the lease liability change is net borrowing, consistent with SPEC-0005 P3.
- [x] **8. Derivatives.** Version 1 excludes the change in derivative liabilities and in A.VII as a
      non-cash pair when both are given; otherwise as proposed, with a convention.
- [x] **9. Floor error code.** One shared code for input below the minimum, as for the ratios
      (FM-E027 on main since #276), if its meaning is the same; otherwise the next free code.
- [x] **10. Order of work.** After #36, #263, #258, #259 and #266, on the v2 statements input
      from the first version.
- [x] **11. Golden sources.** The synthetic company with changed numbers is the independent
      source (maintainer decision of 2026-11-08); whether OIC 10 Appendix A has a numeric
      example stays to verify.
- [x] **12. R23 additions.** They belong to SPEC-0005 and issue #36 (recorded on #36); this spec
      consumes them.
- [x] **13. Exchange rate effect.** Not separated in version 1; limitation with the warning
      `exchange_effect_not_separated` when C17-bis is not zero.
- [x] **14. Reverse factoring.** Financing only when the analyst gives it within
      `financial_debt`.
- [x] **15. Roadmap.** Task #253 references spec 0011 (this pull request).
- [x] **16. Starting point.** Both profit for the year and profit before tax are returned as
      lines; no `starting_point` choice.

## Appendix A — Lines of the statement
"Reclassified": computable when the year is in the reclassified form. Signs: positive for an
inflow. Δ X = X of the later year − X of the earlier year.

### A.1 Operating activities (A)
| Line | OIC 10 line (Italian, wording to verify) | Computed from | Reclassified |
| --- | --- | --- | --- |
| `profit_or_loss` | Utile (perdita) dell'esercizio | item 21, later | yes |
| `income_taxes` | Imposte sul reddito | + item 20 | yes |
| `net_interest` | Interessi passivi/(attivi) | + C17 + lease interest − C16 (under both classifications; C17-bis stays in profit, Open question 13) | yes (`interest_and_other_financial_charges` and the C16 part of `investing_result` when given) |
| `dividends_received` | (Dividendi) | − C15 | yes when given |
| `gains_losses_on_disposals` | (Plusvalenze)/minusvalenze da cessione | − gains + losses (`disposals`) | no |
| line 1 | 1. Utile prima d'imposte, interessi, dividendi e plus/minusvalenze | sum | partly |
| `change_in_provisions` | Accantonamenti ai fondi (net of use when use not given) | + Δ provisions B and C (operating and debt-like) | yes (`operating_provisions`) |
| `depreciation_and_amortisation` | Ammortamenti delle immobilizzazioni | + D&A | yes |
| `write_downs_and_revaluations` | Svalutazioni per perdite durevoli di valore; rettifiche di valore di attività finanziarie | + B10 c (when `investing`) + D19 − D18 | no |
| line 2 | 2. Flusso finanziario prima delle variazioni del ccn | sum | partly |
| working capital lines | Variazioni del capitale circolante netto | − Δ C.I, − Δ trade receivables, + Δ trade payables, − Δ assets D, + Δ liabilities E, other | `other_changes_in_working_capital` only |
| line 3 | 3. Flusso finanziario dopo le variazioni del ccn | sum | yes |
| other adjustments | Interessi incassati/(pagati); (Imposte pagate); Dividendi incassati; (Utilizzo dei fondi) | − interest paid + interest received (under `operating`), − taxes paid, + dividends received in cash (under `operating`), − use of provisions (when given) | per Open question 3 |
| `operating_activities` | Flusso finanziario dell'attività operativa (A) | sum | yes |

### A.2 Investing activities (B)
| Line | OIC 10 line | Computed from | Reclassified |
| --- | --- | --- | --- |
| `net_investment_in_fixed_assets` | Immobilizzazioni materiali e immateriali (investimenti) / disinvestimenti | −(Δ operating fixed assets + Δ non-operating fixed assets + D&A + B10 c − revaluations − right-of-use assets acquired) | yes |
| `change_in_financial_fixed_assets` | Immobilizzazioni finanziarie | − Δ B.III − D19 + D18 (D18 and D19 as a whole: the aggregates do not split them between B.III and C.III; the total B is exact, the split between the two lines is stated as a convention) | yes when `non_operating_assets` and NFP composition give it |
| `change_in_current_financial_assets` | Attività finanziarie non immobilizzate | − Δ C.III | yes (NFP composition) |
| `interest_and_dividends_received` | (only under `by_source`) | + interest received + dividends received | per Open question 3 |
| `investing_activities` | Flusso finanziario dell'attività di investimento (B) | sum | yes |

### A.3 Financing activities (C)
| Line | OIC 10 line | Computed from | Reclassified |
| --- | --- | --- | --- |
| `change_in_financial_debt` | Incremento (decremento) debiti a breve verso banche; accensione e rimborso finanziamenti | + Δ financial debt (gross debt of the NFP excluding lease liabilities and shareholder loans) | yes (NFP composition) |
| `lease_principal_repaid` | (restated leases) | + Δ lease liability − right-of-use additions | yes when given |
| `change_in_shareholder_loans` | Mezzi di terzi (soci) | + Δ shareholder loans, whatever the treatment | yes (quasi-equity and NFP composition) |
| equity lines | Aumento di capitale a pagamento; (Rimborso di capitale); (Dividendi pagati) | movements given; dividends paid + Δ dividends declared | no |
| `other_equity_movements` | (net, when not given) | Δ equity − profit − equity lines given − non-cash equity movements | yes |
| `interest_paid` | (only under `by_source`) | − interest paid | per Open question 3 |
| `financing_activities` | Flusso finanziario dell'attività di finanziamento (C) | sum | yes |

## Appendix B — Free cash flow identity
With IC = operating fixed assets + operating working capital − operating provisions (SPEC-0005
B.1) and NC = non-cash changes in invested capital outside operating profit = revaluations given
− write-downs of fixed assets with the treatment `investing` (B10 c with the treatment
`operating` is in D&A and needs no term):

capex = Δ operating fixed assets + D&A + B10 c (investing) − revaluations,

FCFF = NOPAT + D&A − capex − ΔOWC + Δ operating provisions
     = NOPAT − (ΔIC − NC).

Right-of-use assets acquired are in capex and in ΔIC (Open question 7). The identity holds on
book values for any input where every term is computed; AC10 tests it.
