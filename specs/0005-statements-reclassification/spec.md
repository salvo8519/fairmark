# SPEC-0005 — Financial statements: statutory layouts, reclassified balance sheet and income statement, net financial position

| | |
| --- | --- |
| Status | Draft — ready for the maintainer's methodology review (rewritten 2026-11-08) |
| Milestone | M2 (release 0.1.0); epic #34; issues #35 and #36 |
| Owner | @salvo8519 |
| Formula basis | Statutory layouts of the Italian Civil Code (art. 2424, 2425; abridged art. 2435-bis) as the input structure; balance sheet reclassified by the financial (liquidity) criterion and the functional (operating pertinence) criterion, income statement by the value added and the cost of sales layouts, as commonly adopted in Italian bank credit assessment and corporate finance textbooks; verified against an exact decimal reference script and against published worked examples and practitioner cases still to be named (Open question 22) |
| Related | SPEC-0001 (owns the FCFF and FCFE build-up; consumes the net financial position, non-operating assets and debt-like items), SPEC-0003 (owns tax rates; no IRAP logic), SPEC-0007 (owns `NetFinancialPosition` and every ratio and margin); ADR 0001 (practice as explicit profiles), ADR 0009 (typed calls), ADR 0012 (preview status) |

This revision replaces the previous draft. It narrows the scope to the statements and their
reclassification, moves the free cash flow build-up and the taxes to the specs that already own
them, and turns every methodology choice that the previous draft left implicit into a numbered
open question with options and a recommendation.

## Analysis

### What is on main today
- No `statements` theme. No method reads statutory statements.
- `ratios.*` (SPEC-0007, approved) take named aggregates per balance sheet date (`closing`,
  `opening`): `equity`, `total_assets`, `fixed_assets`, `non_current_liabilities`,
  `current_assets`, `inventories`, `cash`, `current_liabilities` and the
  `NetFinancialPosition` type (`gross_debt`, `cash`, `other_items` of `{name, amount}`;
  NFP = gross debt − cash + Σ other items, positive for net debt). SPEC-0007 R6: "Fairmark
  does not choose which items it includes". The ratios and the margins (net working capital,
  treasury margin, structure margins) are defined there.
- `dcf.free_cash_flows` (SPEC-0001 R2 to R5, issue #38, done) builds FCFF and FCFE from
  reclassified lines (`ebit`, `operating_taxes` or `notional_tax_rate`,
  `depreciation_and_amortisation`, `capital_expenditure`, `change_in_net_working_capital`,
  `other_adjustments`, `interest_expense`, `net_borrowing`); the same lines are accepted by a
  `given` stage of `dcf.fcff` (`FreeCashFlowLines`, issue #38, pull request #180). `dcf.fcfe` is planned under
  SPEC-0001 R1 and R24 (issue #165).
- `dcf.fcff` version 3 takes `net_financial_position` in the SPEC-0007 structure and
  `non_operating_assets` as labelled amounts; debt-like items and non-controlling interests are
  added by issue #44 (SPEC-0001 R23 to R25).
- `coc.*` (SPEC-0003) takes the tax rate of the tax shield as an input; maintainer decision of
  2026-11-07: "the engine has no IRAP logic".

### The previous draft against the approved specs

| Previous | Content | Owner now | In this revision |
| --- | --- | --- | --- |
| R1 | Reclassified P&L and functional balance sheet from statutory items | SPEC-0005 | R13 to R35 |
| R2 | FCFF = EBIT × (1 − t) + D&A − capex − ΔNWC with IRES and IRAP inputs | SPEC-0001 R2, R4, R5 (`dcf.free_cash_flows`, #38, done); tax rates: SPEC-0003 decision of 2026-11-07 | Out of scope |
| R3 | FCFE from FCFF, interest after tax and net borrowing | SPEC-0001 R3 (build-up, done), R1 and R24 (`dcf.fcfe`, #165) | Out of scope |
| Open question | PFN definition, TFR, leases, IRAP base | — | Open questions 6, 8, 10, 18; IRAP: SPEC-0003 |
| Open question | Practitioner cases | — | Open question 22 |
| Open question | First layout: ordinary or abridged | — | Both, through one structure (R4, R8) |

### Design principles (maintainer requirements)
1. **Layers.** Every detailed layer is optional. A reclassified statement given by its totals
   is a complete input; the statutory line items of art. 2424 and 2425 are a richer layer that
   adds traceability, not a precondition.
2. **One source of truth.** The net financial position is the SPEC-0007 type; ratios and
   margins stay in SPEC-0007; free cash flows and taxes on EBIT stay in SPEC-0001; tax rates stay
   in SPEC-0003. This spec produces the inputs those methods already accept, under the same
   field names.
3. **No silent defaults.** A statutory line goes where the published mapping (Appendix A)
   sends it and the explain says so for every line; where practice disagrees, the analyst's
   choice is required and has no default.
4. **Practice, not compliance.** The layouts are aligned with practice. Fairmark does not check
   compliance with the Civil Code or with the OIC standards, nor eligibility for the abridged
   layout.

## Problem
Analysts and lenders start from the statutory accounts filed with the business register. The
statutory balance sheet classifies assets by their destination (art. 2424-bis, paragraph 1:
assets for durable use are fixed assets) and debts by their nature, with the amount due beyond
the next financial year shown separately; the statutory income statement classifies costs by
nature. Credit assessment needs a balance sheet by liquidity and maturity; valuation and
performance analysis need invested capital, the net financial position and an income statement
that separates operating, non-operating, non-recurring and financial results. Every
reclassification involves judgements (where the employee severance indemnity goes, whether
shareholder loans are debt, which receivables are financial) that change the net financial
position, EBITDA and every ratio built on them. In spreadsheets these judgements are invisible;
an AI agent that extracts statements from a PDF places lines inconsistently and cannot show why.
The engine must check that the statements reconcile, apply a published mapping, require the
analyst's choice wherever practice disagrees, and return outputs that `ratios.*` and `dcf.*`
accept as they are.

## Scope
**In scope:**
- Statutory statements as input (#35): balance sheet (art. 2424) and income statement
  (art. 2425) in the ordinary layout, the abridged layout (art. 2435-bis) and the micro-entity
  layout (art. 2435-ter, which uses the abridged layouts); labelled additional items
  (art. 2423-ter, paragraph 3); reconciliation checks.
- Reclassified balance sheet (#36): financial criterion (liquidity and maturity) and functional
  criterion (operating pertinence), from statutory statements or given directly by totals.
- Reclassified income statement (#36): value added layout and cost of sales layout, from
  statutory statements or given directly by totals.
- Net financial position (#36) in the structure of SPEC-0007 R6, with its composition by line
  and its current and non-current parts.

**Out of scope, owned elsewhere:**
- Free cash flow build-up, taxes on EBIT, interest tax shield, FCFE: SPEC-0001
  (`dcf.free_cash_flows`, `dcf.fcfe`). This spec returns `ebit` and
  `depreciation_and_amortisation` under the names of the build-up lines; capital expenditure and
  the change in net working capital are not derived here (Open question 21).
- IRES, IRAP, deferred taxes, tax rates: SPEC-0003 decision of 2026-11-07. Income taxes
  (art. 2425, item 20) are taken as given.
- Ratios and margins (net working capital, treasury margin, structure margins, coverage,
  leverage): SPEC-0007.

**Out of scope, not planned here:** XBRL or PDF parsing (an adapter or agent concern);
consolidated accounts; layouts of banks, insurers and other regulated entities; IFRS layouts
(IAS 1, IFRS 18) as a statutory input (IFRS preparers use the reclassified input layer, R27 and
R34); the cash flow statement (OIC 10); credit scoring models; the contribution margin layout
(it needs a fixed and variable cost split that statutory statements do not give).

## User stories
- As a credit analyst with abridged accounts, I want to enter the balance sheet at the level the
  filing shows and be asked only for the splits the reclassification really needs, so that I do
  not invent detail.
- As a valuation analyst, I want the net financial position, the non-operating assets and the
  debt-like items in the shapes that `dcf.fcff` accepts, so that the bridge from enterprise value
  to equity value uses the same numbers as the reclassified balance sheet.
- As an AI agent extracting statements from a PDF, I want a check that names the line by its
  JSON pointer when totals do not add up, so that I correct the extraction instead of reporting
  a wrong ratio.
- As an AI agent, I want every statutory line's destination in the explain, so that I can cite
  in the report where each amount went and who decided it.
- As an analyst who already has a bank's reclassified statement, I want to pass its totals
  directly and get the same outputs, so that the statutory layer is never a precondition.

## Requirements (EARS)

### Methods and input layers
- **R1** — THE ENGINE SHALL provide three methods in a new theme `statements`, each at version 1
  with status preview (ADR 0012): `statements.statutory_accounts` (checks and totals of the
  statutory statements), `statements.reclassify_balance_sheet` (financial criterion, functional
  criterion and net financial position) and `statements.reclassify_income_statement` (value
  added layout, cost of sales layout) (Open questions 1 and 2).
- **R2** — THE ENGINE SHALL take one or more financial years per call, each identified by its
  balance sheet date (ISO 8601; `FM-E009` when invalid), with distinct dates (`FM-E001`
  otherwise), reclassify each year independently with the method-level choices of the call, and
  return the years in input order. All amounts of a call SHALL be in the currency units of the
  statements, in one unit, never scaled by the engine.
- **R3** — THE ENGINE SHALL take, for each year of a reclassification method, exactly one input
  layer: `statutory` (R5 to R11) or `reclassified` (R27, R34); IF neither or both are given,
  THEN it SHALL return `FM-E001` naming both fields.
- **R4** — Every node of the statutory structure and of the reclassified structures SHALL accept
  a total, its items, or both. WHEN only items are given, THE ENGINE SHALL compute the total;
  WHEN both are given and the sum of the items differs from the total, THE ENGINE SHALL return
  `FM-E026` naming the node by its JSON pointer and stating the total, the sum and the
  difference.

### Statutory statements (#35)
- **R5** — THE ENGINE SHALL accept the balance sheet in the structure of art. 2424 (Appendix A,
  assets A to D, liabilities A to E, down to the items numbered with arabic numerals and their
  lettered sub-items), each field named in English with its statutory reference in the schema
  description. For every node of assets C.II and liabilities D it SHALL require the amount due
  beyond the next financial year, and for every node of assets B.III.2 the amount due within
  it, as the statutory layout shows them separately; IF that amount is missing or exceeds the
  node, THEN it SHALL return `FM-E001` naming the node.
- **R6** — THE ENGINE SHALL accept the income statement in the structure of art. 2425
  (Appendix A): items 1 to 5 (A), 6 to 14 (B), 15 to 17-bis (C), 18 and 19 (D), 20 and 21.
- **R7** — Amounts SHALL be entered as presented in the statutory layouts: assets, liabilities,
  provisions, revenue and cost items as amounts of zero or more (`FM-E001` naming the field
  otherwise); the signed items SHALL be A2 and A3 (positive for an increase in inventories),
  B11 (positive for a decrease in inventories, which is a cost), 17-bis (positive for a net
  gain), 20 (positive for a tax charge), 21, and the equity items A.VI, A.VII, A.VIII and A.IX
  (positive for a reserve, a gain or a profit), with A.X (negative reserve for own shares) zero
  or negative. The schema SHALL state the sign of every signed item (Open question 5).
- **R8** — THE ENGINE SHALL require the declared layout of each year, `ordinary`, `abridged` or
  `micro_entity`, echo it in `explain.conventions`, and not check eligibility thresholds. No
  number SHALL depend on it. WHEN the layout is `abridged` or `micro_entity`, THE ENGINE SHALL
  accept the grouped income statement nodes permitted by art. 2435-bis (Appendix A, column
  "Abridged"); WHEN it is `ordinary`, a grouped node SHALL be refused with `FM-E001` stating
  the items to give instead (Open question 3).
- **R9** — THE ENGINE SHALL accept, for intangible assets (B.I) and tangible assets (B.II), the
  optional gross amount and accumulated amortisation, depreciation and write-downs next to the
  net amount (the abridged layout shows them explicitly); WHEN given, IF net ≠ gross −
  accumulated, THEN it SHALL return `FM-E026`.
- **R10** — THE ENGINE SHALL accept labelled additional items (art. 2423-ter, paragraph 3)
  under any node of the statutory structure, each with a label and an amount under the sign
  rule of its node (and its maturity part under R5). A reclassification method SHALL require a
  classification for every additional item with a non-zero amount (R13), with no default.
- **R11** — THE ENGINE SHALL check that total assets equal total liabilities and equity exactly,
  with no tolerance; IF they differ, THEN it SHALL return `FM-E026` stating both totals, the
  difference and the fix (statements in euro units already carry their rounding difference in
  equity and in the income statement; enter it as given) (Open question 4). WHEN the profit or
  loss for the year is available from the balance sheet (liabilities A.IX) and from the income
  statement (item 21, given or computed), IF the two differ, THEN it SHALL return `FM-E026`
  naming both pointers.
- **R12** — `statements.statutory_accounts` SHALL accept a balance sheet, an income statement or
  both, and SHALL return per year every total of the statutory structure (each letter and
  roman-numeral node; total assets; total liabilities and equity; the amounts due beyond the
  next financial year of C.II and D; value of production A; costs of production B; the
  difference A − B; total C; total D; profit before tax; profit or loss for the year), with
  every total in `explain` with its items substituted.

### Classification of statutory lines (#36)
- **R13** — WHEN a reclassification method receives statutory input, THE ENGINE SHALL assign
  every amount given (every line at the deepest level given, and every maturity part) to exactly
  one destination of each requested criterion or layout, by the mapping of Appendix A: a line
  marked `fixed` SHALL take the mapping destination; a line marked `default` SHALL take the
  mapping destination unless the analyst classifies it; a line marked `required` SHALL be
  classified by the analyst whenever its amount is not zero; IF a required classification is
  missing, THEN THE ENGINE SHALL return `FM-E001` naming the line by its JSON pointer and
  listing the permitted destinations. A line whose amount is zero SHALL need no classification.
- **R14** — A classification SHALL reference the line by a JSON pointer relative to the year's
  `statutory` object and give either one destination or a split into labelled parts, each with
  a destination and an amount (and, for receivables and payables, the maturity of the part);
  the destination SHALL be one permitted for that line in Appendix A (`FM-E001` otherwise); IF
  the parts do not sum exactly to the line, THEN `FM-E026`; IF the pointer does not resolve to a
  line given in the input, or the same line is classified twice, THEN `FM-E001`.
- **R15** — WHEN a destination needs a split that the input does not give (for example trade
  receivables when receivables C.II are given only as a total in the abridged layout, under the
  functional criterion), THE ENGINE SHALL treat the node given as a `required` line with the
  union of the destinations of its items, and SHALL NOT infer the split.
- **R16** — THE ENGINE SHALL report every assignment in `explain` as one step per line and part:
  statutory reference, JSON pointer, amount, destination in each requested criterion or layout,
  and origin (`mapping` or `analyst`), so that every reclassified amount traces back to
  statutory lines.
- **R17** — The criteria and layouts to compute SHALL be a required, non-empty input
  (`criteria`: `financial`, `functional`; `layouts`: `value_added`, `cost_of_sales`). A
  classification SHALL be required only where a requested criterion or layout, or the net
  financial position, depends on it (Appendix A states which).

### Reclassified balance sheet, financial criterion (#36)
- **R18** — WHEN `financial` is requested, THE ENGINE SHALL return per year the structure of
  Appendix B.1 (non-current assets as `fixed_assets`; `inventories`; current receivables;
  current financial assets; current prepayments and accrued income; `cash`; `current_assets`;
  `total_assets`; `equity`; `non_current_liabilities`; `current_liabilities`; total liabilities
  and equity), each total in `explain` with its components substituted.
- **R19** — Under the financial criterion THE ENGINE SHALL place receivables and payables by the
  statutory maturity split, reporting the convention "the next financial year is read as the
  twelve months after the balance sheet date"; lines without a statutory maturity split SHALL
  follow Appendix A, where a split given by the analyst overrides the mapping (Open questions
  6, 7, 9, 11, 20).
- **R20** — The output of the financial criterion SHALL carry the field names and meanings of
  the inputs of `ratios.liquidity` and `ratios.capital_structure` (`fixed_assets`,
  `current_assets`, `inventories`, `cash`, `current_liabilities`, `non_current_liabilities`,
  `equity`, `total_assets`, `net_financial_position`), so that they can be passed unchanged;
  THE ENGINE SHALL NOT compute the ratios or margins of SPEC-0007.

### Reclassified balance sheet, functional criterion (#36)
- **R21** — WHEN `functional` is requested, THE ENGINE SHALL return per year the structure of
  Appendix B.2: operating fixed assets; trade working capital; operating working capital;
  operating provisions; `invested_capital`; labelled non-operating assets;
  `total_funds_invested`; `equity`; labelled quasi-equity items; `net_financial_position`;
  labelled debt-like items; total sources; each total in `explain` with its components
  substituted.
- **R22** — THE ENGINE SHALL check that total funds invested equal equity + Σ quasi-equity
  items + net financial position + Σ debt-like items, exactly. With statutory input the identity
  follows from R11 and is an internal invariant (`FM-E012` if it ever fails); with reclassified
  input, IF it fails, THEN THE ENGINE SHALL return `FM-E026` stating both sides and the
  difference.

### Net financial position (#36)
- **R23** — THE ENGINE SHALL return, per year and whatever the criteria requested, the net
  financial position in the `NetFinancialPosition` type of SPEC-0007 R6, positive for net debt:
  `gross_debt` = Σ lines classified as financial debt (borrowings); `cash` = assets C.IV;
  `other_items` = each other line classified as a financial asset (negative amount) or as an
  other financial liability (positive amount), named with its label and statutory reference.
  It SHALL also return the composition by line with each JSON pointer, and the current and
  non-current parts from the maturity split; its total SHALL be computed by the SPEC-0007
  function, not restated (Open question 18).
- **R24** — THE ENGINE SHALL return non-operating assets, debt-like items and quasi-equity items
  as labelled amounts with their statutory references, in the labelled-amount shape accepted by
  `dcf.fcff` (SPEC-0001 R23, R25), so that they can be passed to the DCF unchanged. A line SHALL
  become non-operating, debt-like or quasi-equity only through Appendix A or the analyst's
  classification.

### Balance sheet adjustments
- **R25** — THE ENGINE SHALL deduct subscribed capital unpaid (assets A) from equity in both
  criteria and report it (Open question 16). WHEN dividends declared out of the profit or the
  reserves of the year are given (a labelled amount of zero or more per year), THE ENGINE SHALL
  move them from equity to current liabilities (financial criterion) and to debt-like items
  (functional criterion), and report them; none given SHALL mean none, reported (Open
  question 17).
- **R26** — (Subject to Open question 10.) WHEN a lease restatement by the financial method is
  given (per lease or in total: carrying amount of the asset, lease liability with its current
  part, maxi-instalment prepayment to remove, and for the income statement the lease payments
  of the year, depreciation and interest), THE ENGINE SHALL add the asset to the tangible fixed
  assets, the liability to the net financial position as an other financial liability, remove
  the prepayment, report the difference as an equity adjustment with its formula, and SHALL NOT
  compute deferred taxes on it; in the income statement it SHALL remove the lease payments from
  B8 and add depreciation and interest. None given SHALL mean the leases as reported, stated as
  a convention.

### Reclassified input by totals (balance sheet)
- **R27** — WHEN the input layer is `reclassified`, THE ENGINE SHALL accept the structure of
  Appendix B.1 or B.2 (one of them per call, named by its block) with every node as a total or
  items (R4), the net financial position in the SPEC-0007 structure and labelled non-operating
  assets, debt-like items and quasi-equity items; it SHALL require the totals that the identity
  check needs (`FM-E001` naming the missing node), compute the totals that are not given, check
  the identity (`FM-E026`), return components that are neither given nor derivable as null with
  the reason "not given", and report that the classification was made before the call.

### Reclassified income statement (#36)
- **R28** — WHEN `value_added` is requested, THE ENGINE SHALL return per year the structure of
  Appendix B.3: value of production, external costs, value added, personnel costs, `ebitda`,
  `depreciation_and_amortisation`, write-downs, provisions, `ebit`, non-operating result,
  non-recurring result, net financial income and expense, value adjustments of financial assets,
  profit before tax, income taxes and `net_income`, each in `explain` with its components
  substituted.
- **R29** — WHEN `cost_of_sales` is requested, THE ENGINE SHALL return per year the structure of
  Appendix B.4 (revenue, cost of sales, gross profit, distribution costs, administrative
  expenses, other expenses, other operating income, `ebit`, and the lines below EBIT of R28). It
  SHALL require, for every cost line by nature marked for allocation in Appendix A whose amount
  is not zero, a split by function (`cost_of_sales`, `distribution_costs`,
  `administrative_expenses`, `other_expenses`), with no default (`FM-E001` naming the line),
  and parts summing exactly to the line (`FM-E026`) (Open question 19).
- **R30** — THE ENGINE SHALL compute EBITDA with one meaning in both layouts: EBIT +
  depreciation and amortisation + write-downs + provisions, the components of Appendix B.3
  (Open question 13); it SHALL report the definition in `explain.conventions` in words usable
  as `ebitda_definition` in SPEC-0007.
- **R31** — THE ENGINE SHALL require, whenever the amount is not zero, the classification of
  other revenue and income (A5) and of other operating expenses (B14) as `operating`,
  `non_operating` or `non_recurring`, or a split among them; it SHALL accept for any other
  income statement line an optional labelled part classified `non_recurring`, none given meaning
  none (reported). Non-recurring and non-operating results SHALL be presented below EBIT, so
  that EBITDA and EBIT exclude them (Open question 12).
- **R32** — THE ENGINE SHALL present items 15, 16, 17 and 17-bis as net financial income and
  expense, items 18 and 19 as value adjustments of financial assets, and item 20 as income
  taxes as given, with optional labelled components echoed; it SHALL NOT compute IRES, IRAP,
  deferred taxes or taxes on EBIT. The net income of every layout SHALL equal item 21 exactly,
  and profit before tax SHALL equal the statutory A − B ± C ± D exactly (Open question 14).
- **R33** — THE ENGINE SHALL return a reconciliation of EBIT with the statutory difference
  A − B: EBIT = (A − B) − Σ non-operating parts of A5 and B14 − Σ non-recurring parts moved
  out of A and B (+ the lease restatement effect when R26 applies), each part listed with its
  pointer.

### Reclassified input by totals (income statement)
- **R34** — WHEN the input layer is `reclassified`, THE ENGINE SHALL accept the structure of
  Appendix B.3 or B.4 with every node as a total or items (R4); subtotals such as `ebitda` and
  `ebit` MAY be given as totals without their components; THE ENGINE SHALL compute the totals
  that can be derived, check every subtotal that is both given and derivable (`FM-E026`), and
  return the others as null with the reason "not given".

### Interoperability
- **R35** — Output fields that correspond to inputs of other methods SHALL carry the same name
  and meaning: `ebit` and `depreciation_and_amortisation` (SPEC-0001 build-up lines);
  `revenue`, `ebitda`, `ebit`, `net_income`, `cost_of_sales` (cost of sales layout only) and
  `purchases` (item B6, value added layout) for SPEC-0007; `interest_and_other_financial_charges`
  (item 17) documented as the statutory amount, not as a covenant definition. A field SHALL NOT
  be returned under a shared name when its meaning differs.

### Explain, errors and stability
- **R36** — THE ENGINE SHALL report in `explain` every total and subtotal with its components
  substituted, and in `explain.conventions`: the layout, the input layer, the criteria and
  layouts, the sign conventions, the exact balance check, the maturity convention, every
  mapping destination used and every analyst classification, every required choice, the EBITDA
  definition, the treatment of financial income, of taxes and of leases, and the adjustments
  given or "none given".
- **R37** — Every error SHALL name the offending field by its JSON pointer and state the fix.
  Errors SHALL reuse `FM-E001` (invalid, missing or misplaced input, missing classification),
  `FM-E008` (overflow), `FM-E009` (invalid date) and `FM-E012` (broken internal invariant); one
  new code, `FM-E026` "statements do not reconcile", SHALL cover every difference between a
  total and its items, between assets and liabilities and equity, between the two profits,
  between a split and its line, and between the two sides of the functional identity.
- **R38** — Multi-way choices SHALL be enumerations or labelled objects, never booleans; every
  choice marked `required` SHALL have no default; the arithmetic SHALL be additions and
  subtractions of decimals only, with no rounding, so that equal inputs give equal outputs
  digit for digit.

## Conventions and parameters

| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Input layer | `statutory`, `reclassified` | required, one per year | Maintainer requirement (layers) |
| Statutory layout | `ordinary`, `abridged`, `micro_entity` | required; echoed; no number depends on it | Art. 2424, 2425, 2435-bis, 2435-ter |
| Sign of statutory inputs | as presented in the layouts; signed items listed in R7 | fixed | Art. 2425; OIC 12 (to verify, V12) |
| Balance check | exact equality | fixed (Open question 4) | Art. 2423, euro units (to verify, V1) |
| Maturity | statutory split within / beyond the next financial year, read as twelve months | fixed, reported; analyst split overrides | Art. 2424 (C.II, D, B.III.2); IAS 1 paragraphs 66 to 76 (to verify, V20) |
| Criteria, layouts | `financial`, `functional`; `value_added`, `cost_of_sales` | required, non-empty | Italian practice |
| Line destinations | Appendix A, `fixed` / `default` / `required` | per line, reported | Proposed here (Open question 15) |
| Employee severance indemnity (TFR), functional | `operating_provision`, `debt_like`, `other_financial_liability` | required when not zero | Practice varies (Open question 6) |
| Provisions B.1 and B.4, functional | `operating_provision`, `debt_like` | required when not zero | Practice varies (Open question 7) |
| TFR and provisions, financial | non-current, or analyst split | default non-current, reported | Practice (Open questions 6, 7) |
| Shareholder loans D.3 | `financial_debt`, `quasi_equity`, or a split | required when not zero | Practice varies; art. 2427 n. 19-bis, art. 2467 (to verify, V9, V10) (Open question 8) |
| Prepayments and accruals | current and operating, or analyst split | default, reported | Practice (Open question 9) |
| Leases | as reported; financial-method restatement when given | as reported, stated | Art. 2427 n. 22; IFRS 16 (to verify) (Open question 10) |
| Subscribed capital unpaid | deducted from equity | fixed, reported | Bank practice (Open question 16) |
| Dividends declared | labelled amount | none given means none, reported | Practice (Open question 17) |
| Net financial position | SPEC-0007 structure; cash = C.IV only | fixed | SPEC-0007 R6 (Open question 18) |
| EBITDA | EBIT + D&A + write-downs (B10c, B10d) + provisions (B12, B13) | fixed, reported | Italian practice (Open question 13) |
| Financial income | net financial income and expense below EBIT; area D on its own line | fixed, reported | Practice varies (Open question 14) |
| A5 and B14 | `operating`, `non_operating`, `non_recurring`, or a split | required when not zero | D.Lgs. 139/2015 removed area E (Open question 12) |
| Non-recurring parts of other lines | labelled parts | none given means none, reported | Art. 2427 n. 13 (to verify, V9) |
| Taxes | item 20 as given | fixed; no IRES or IRAP logic | SPEC-0003 decision of 2026-11-07 |
| Cost of sales allocation | split of each cost line by function | required when not zero | IAS 1 paragraphs 99 to 105; Directive 2013/34/EU Annex VI (to verify) (Open question 19) |

## Acceptance criteria
Every golden case compares exactly (tolerance 0): the methods only add and subtract decimals.

- **AC1** (R5, R6, R7, R11, R12) — Golden `statements.statutory_accounts/ordinary-complete.yaml`
  (every item of Appendix A non-zero, signed items of both signs) matches the reference script
  for every total.
- **AC2** (R4, R8, R9, R12) — Golden `abridged.yaml`: the statements of AC1 entered at the
  abridged level, with the grouped nodes and the gross and accumulated amounts of B.I and B.II,
  give the same totals as AC1. Property: any statutory input aggregated node by node gives the
  same totals as its detail.
- **AC3** (R4, R9, R11, R37) — One golden error case per `FM-E026` condition: items differing
  from their total, net ≠ gross − accumulated, assets ≠ liabilities and equity (one unit of
  difference), profit A.IX ≠ item 21. Each message contains the JSON pointer and the difference.
- **AC4** (R2, R3, R5, R7, R8, R10, R37) — `FM-E001` for: both layers, no layer, a negative
  unsigned amount, a missing or excessive maturity part, a grouped node under `ordinary`, an
  unclassified non-zero additional item, two years with the same date; `FM-E009` for an invalid
  date. Each message contains the JSON pointer and the fix.
- **AC5** (R13, R14, R15, R16, R17) — Unit and golden tests: a `required` line with a non-zero
  amount and no classification returns `FM-E001` listing the permitted destinations; with a zero
  amount it passes; a split that differs from its line returns `FM-E026`; an unresolved pointer
  and a duplicate classification return `FM-E001`; an abridged C.II total under `functional`
  without a split returns `FM-E001`. Property: for every line, the parts assigned in the explain
  sum to the line amount, and every given line appears exactly once.
- **AC6** (R18, R19, R20) — Golden `statements.reclassify_balance_sheet/financial-*.yaml` match
  the reference script. Property: total assets = equity + non-current liabilities + current
  liabilities. Property: passing the output to `ratios.liquidity` and `ratios.capital_structure`
  gives the same values as the reference applied to the reclassified totals.
- **AC7** (R21, R22, R23, R24) — Golden `functional-*.yaml` match the reference, including one
  case per choice for the TFR (three), for shareholder loans (financial debt, quasi-equity,
  split) and for provisions B.4. Property (random valid statutory inputs): total funds invested
  = equity + Σ quasi-equity + net financial position + Σ debt-like items, exactly. Contract test:
  the `net_financial_position`, `non_operating_assets` and debt-like items of the output are
  accepted by `dcf.fcff` version 3 unchanged, and the net financial position total equals the
  SPEC-0007 total of the same object.
- **AC8** (R25) — Golden cases with subscribed capital unpaid (called and uncalled) and with
  dividends declared, under both criteria; without them the conventions state "none given".
- **AC9** (R26) — (If Open question 10 accepts the restatement.) Golden `lease-restatement.yaml`
  from a constructed case: asset, liability, removed prepayment and equity adjustment in the
  balance sheet; B8, depreciation and interest in the income statement; EBITDA increases by the
  lease payments removed; the identity of R22 holds.
- **AC10** (R27) — Golden `reclassified-financial.yaml` and `reclassified-functional.yaml`:
  outputs equal those of AC6 and AC7 when the totals given equal the reclassified totals of
  those cases (property); a missing required total returns `FM-E001`; a failed identity returns
  `FM-E026`; components not given are null with the reason.
- **AC11** (R28, R30, R32, R33) — Golden `statements.reclassify_income_statement/value-added-*.yaml`
  match the reference for every subtotal. Properties: net income = item 21; profit before tax =
  statutory A − B ± C ± D; the EBIT reconciliation adds up to A − B.
- **AC12** (R29, R30) — Golden `cost-of-sales-*.yaml` match the reference. Property: for the
  same input with every allocation given, EBIT and EBITDA are equal in both layouts. A missing
  allocation returns `FM-E001`; a split that differs from its line returns `FM-E026`.
- **AC13** (R31) — Golden `non-recurring.yaml`: A5 and B14 split among the three classes and a
  non-recurring part moved out of B7; EBITDA and EBIT exclude both; a moved part larger than its
  line returns `FM-E026`; unclassified non-zero A5 returns `FM-E001`.
- **AC14** (R34) — Golden `reclassified-ebitda-ebit-only.yaml`: EBITDA and EBIT given as totals
  with no components return the derivable lines and null with reasons for the others; a given
  EBIT that differs from EBITDA minus given components returns `FM-E026`.
- **AC15** (R35) — Contract tests: `ebit` and `depreciation_and_amortisation` of the output are
  accepted as `FreeCashFlowLines` fields; `revenue`, `ebitda`, `ebit`, `net_income`,
  `cost_of_sales` and `purchases` are accepted by the SPEC-0007 groups that take them.
- **AC16** (R36, R37, R38) — Unit tests: every convention of the table appears in
  `explain.conventions` when it applies; every error message contains a JSON pointer; the
  schemas contain no boolean choice; two runs of every golden case give identical output.
- **AC17** (R18, R21, R28, R29) — At least one published worked example per criterion and per
  layout, re-derived from its printed statutory figures, matches at its published precision; the
  golden file records the source, the edition and the page (Open question 22).

## Interfaces
- **Method ids** (Open questions 1 and 2): `statements.statutory_accounts` version 1,
  `statements.reclassify_balance_sheet` version 1, `statements.reclassify_income_statement`
  version 1. All new, all preview, listed under `"0005"` in `specs/method-evidence.yaml`.
- **Input (both reclassification methods):** `criteria` or `layouts` (required, non-empty);
  `years`: list of `{balance_sheet_date, statutory | reclassified, classifications?,
  adjustments?}`. `statutory` holds `layout`, `balance_sheet`, `income_statement` (each optional
  for the method that does not need it). `classifications` is a list of
  `{line, destination}` or `{line, split: [{label, destination, amount, maturity?}]}`, with
  `line` a JSON pointer relative to `statutory`. Final type names are set in `plan.md`.
- **Output:** per year, the requested structures of Appendix B, `net_financial_position`
  (SPEC-0007 type) with `composition` and `current` / `non_current` parts,
  `non_operating_assets`, `debt_like_items`, `quasi_equity_items` (labelled amounts),
  `line_assignments` (R16), and for the income statement `ebit_reconciliation` (R33).
- **Schema changes:** additive only: three new methods, `schemas/statements.*/v1.json`, three
  entries in `schemas/index.json`. No existing method changes; no numerical change; no golden
  case of another method changes.
- **Building blocks reused, not duplicated:** `ratios::NetFinancialPosition` and `NfpItem`
  (SPEC-0007) for the net financial position and its total; the labelled-amount type accepted by
  `dcf.fcff` for non-operating assets and debt-like items (SPEC-0001 R25); the field names of
  `dcf::free_cash_flows::FreeCashFlowLines` and of the SPEC-0007 inputs (R35); `Explain`;
  checked arithmetic in `crate::num`; the registry. Note for the plan: SPEC-0007 names
  labelled items `{name, amount}` and the DCF `{label, value}`; the plan should reuse each where
  it is consumed and decide with the architect whether `NetFinancialPosition` moves from
  `ratios` to a shared module (no schema change either way).
- **Errors:** `FM-E001`, `FM-E008`, `FM-E009`, `FM-E012` reused; new `FM-E026` "statements do
  not reconcile" (next free code at implementation), documented in
  `docs/en/reference/errors.md`.
- **Theme and naming:** a new theme `statements` needs a commit scope in
  `docs/en/project/conventions.md` (`scripts/conventions.py`).

## Documentation
- Methodology pages: `docs/en/methods/statements-statutory-accounts.md`,
  `docs/en/methods/statements-reclassify-balance-sheet.md` (both criteria, the net financial
  position, Appendix A as a table, every required choice with its options),
  `docs/en/methods/statements-reclassify-income-statement.md` (both layouts, EBITDA definition,
  EBIT reconciliation); each with formula, conventions, example and a "Verified against"
  section; nav entries and method index rows (status preview).
- Concept page `docs/en/concepts/financial-statements.md`: the two input layers, how the outputs
  feed `ratios.*`, `dcf.free_cash_flows` and the bridge of `dcf.fcff`, and what is out of scope.
- Reference: `docs/en/reference/errors.md` (`FM-E026`), regenerated `methods.md` and schemas.
- Italian editions of every page through the glossary; new glossary entries for the terms
  below once verified.
- Python helpers in `python/fairmark/statements.py` (translate only); release note fragments
  for #35 and #36.

## Terminology and sources
Identifiers use the terms of international practice: IFRS and the CFA Institute curriculum
first, the English text of Directive 2013/34/EU (whose layouts the Civil Code transposes)
where no IFRS term exists. "glossary" marks terms already in `i18n/glossary.yaml`. Every other
row is "to verify" until checked on the source named.

| Term | Identifier | Italian equivalent | Candidate source | Status |
| --- | --- | --- | --- | --- |
| Statutory financial statements | `statutory` | bilancio d'esercizio | Civil Code art. 2423 | glossary (financial statements) |
| Balance sheet; income statement | `balance_sheet`, `income_statement` | stato patrimoniale; conto economico | Art. 2424, 2425; IAS 1 | to verify (V20) |
| Abridged layout; micro-entity | `abridged`, `micro_entity` | bilancio in forma abbreviata; micro-impresa | Art. 2435-bis, 2435-ter; Directive 2013/34/EU art. 3 | to verify (V7, V8, V19) |
| Reclassification | `reclassify_*` | riclassificazione | Italian practice | to verify (V27) |
| Financial criterion | `financial` | criterio finanziario (liquidità ed esigibilità) | Italian textbooks | to verify (V27) |
| Functional criterion | `functional` | criterio funzionale (pertinenza gestionale) | Italian textbooks | to verify (V27) |
| Fixed assets; current assets; non-current and current liabilities | `fixed_assets`, `current_assets`, `non_current_liabilities`, `current_liabilities` | immobilizzazioni (attivo immobilizzato); attivo corrente; passività consolidate; passività correnti | IAS 1 paragraphs 66 to 76; SPEC-0007 | glossary |
| Invested capital | `invested_capital` | capitale investito (netto) | Koller et al. (SPEC-0001 [S12]) | glossary; "netto" to verify |
| Total funds invested | `total_funds_invested` | to verify | Koller et al. | to verify (V25) |
| Operating working capital; trade working capital | `operating_working_capital`, `trade_working_capital` | capitale circolante netto operativo; capitale circolante commerciale | Koller et al.; bank practice | to verify (V25, V28) |
| Debt-like items; non-operating assets | `debt_like_items`, `non_operating_assets` | as in SPEC-0001 | SPEC-0001 | SPEC-0001; glossary |
| Quasi-equity | `quasi_equity_items` | to verify; keep English if none verified | Bank practice | to verify (V28) |
| Employee severance indemnity (TFR) | `employee_severance_indemnity` | trattamento di fine rapporto (TFR) | IFRS statements of Italian issuers; IAS 19 | to verify (V22) |
| Shareholder loans | `shareholder_loans` | finanziamenti dei soci | Art. 2424 D.3; art. 2467 | to verify (V10) |
| Prepayments and accrued income; accruals and deferred income | `prepayments_and_accrued_income`, `accruals_and_deferred_income` | ratei e risconti attivi; ratei e risconti passivi | Directive 2013/34/EU Annex III | to verify (V19) |
| Subscribed capital unpaid | `subscribed_capital_unpaid` | crediti verso soci per versamenti ancora dovuti | Directive 2013/34/EU Annex III | to verify (V19) |
| Value of production; value added | `value_of_production`, `value_added` | valore della produzione; valore aggiunto | Art. 2425 (A); Eurostat structural business statistics | to verify (V24) |
| Personnel costs | `personnel_costs` | costi per il personale | Art. 2425 B9; Eurostat | to verify (V24) |
| EBITDA | `ebitda` | margine operativo lordo (MOL) | CFA Institute; ESMA APM guidelines | to verify (V17, V26) |
| EBIT | `ebit` | reddito operativo | SPEC-0001 | SPEC-0001 (Italian to verify) |
| Cost of sales; gross profit; distribution costs; administrative expenses; other expenses | `cost_of_sales`, `gross_profit`, `distribution_costs`, `administrative_expenses`, `other_expenses` | costo del venduto; margine lordo; costi commerciali; costi amministrativi; altri costi | IAS 1 paragraph 103; Directive 2013/34/EU Annex VI | glossary (cost of sales); others to verify (V19, V20) |
| Non-recurring items | `non_recurring` | componenti non ricorrenti | Consob; ESMA APM guidelines | to verify (V17, V18) |
| Net financial position | `net_financial_position` | posizione finanziaria netta (PFN); "indebitamento finanziario netto" in the ESMA and Consob texts | SPEC-0007; ESMA32-382-1138 | glossary; ESMA term to verify (V15) |
| Write-downs; provisions | `write_downs`, `provisions` | svalutazioni; accantonamenti | Art. 2425 B10, B12, B13 | to verify |
| Dividends declared | `dividends_declared` | dividendi deliberati | Practice | to verify |

## Sources to verify
Cited in this spec and to be checked by the maintainer before approval. Items 1 to 11 are the
legal texts on Normattiva; items 12 to 14 the OIC standards on the OIC Foundation site (OIC 12
and OIC 10 are already referenced in SPEC-0001 as [S9] and [S10]).

1. **V1** Civil Code art. 2423: the paragraph requiring statements in euro units without
   decimals, and the treatment of rounding differences (equity reserve; income statement A5 or
   B14).
2. **V2** Art. 2423-ter: paragraph numbers for further subdivision, grouping of arabic-numeral
   items, additional items (paragraph 3), adaptation, comparatives and the prohibition of
   offsetting.
3. **V3** Art. 2424: the complete item list of Appendix A as in force after D.Lgs. 139/2015,
   in particular assets C.III.7 (financial assets for centralised treasury management), the
   "due within the next financial year" indication of B.III.2, liabilities A.VII and A.X, and
   provisions B.3.
4. **V4** Art. 2424-bis: paragraph 1 (assets for durable use are fixed assets) and the paragraphs
   on provisions, TFR and prepayments and accruals.
5. **V5** Art. 2425: the item list of Appendix A, including 17-bis, 18 d and 19 d (derivatives),
   the wording of item 20, and the sign presentation of A2, A3 and B11.
6. **V6** Art. 2425-bis: paragraph on sale and leaseback gains (relevant to Open question 10).
7. **V7** Art. 2435-bis. Thresholds verified on 2026-11-08: as amended by D.Lgs. 6 September
   2024, n. 125, art. 16, paragraph 1, letter a), not exceeding two of: total assets
   EUR 5,500,000; revenue EUR 11,000,000; 50 average employees. Still to verify: the balance
   sheet content of the abridged layout, the explicit deduction of amortisation and
   depreciation from B.I and B.II, and the income statement items that may be grouped.
8. **V8** Art. 2435-ter. Thresholds verified on 2026-11-08: as amended by D.Lgs. 125/2024,
   art. 16, paragraph 1, letter b), not exceeding two of: total assets EUR 220,000; revenue
   EUR 440,000; 5 average employees. Still to verify: the layouts used by micro-entities.
9. **V9** Art. 2427, paragraph 1: n. 6 (debts beyond five years, secured debts), n. 13 (amount
   and nature of items of exceptional size or incidence), n. 19-bis (shareholder loans by
   maturity, with the postponement clause shown separately), n. 22 (financial leases with the
   effects of the financial method).
10. **V10** Art. 2467 (postponement of shareholder loans) and its application to the company
    types concerned.
11. **V11** D.Lgs. 18 August 2015, n. 139: removal of area E from art. 2425 and its first
    application (financial years starting on or after 1 January 2016).
12. **V12** OIC 12 (SPEC-0001 [S9]): content of each item, sign of A2, A3 and B11, where former
    extraordinary items are classified after 2016, paragraph numbers.
13. **V13** OIC 10 (SPEC-0001 [S10]): definition of cash (disponibilità liquide) used for
    `cash`.
14. **V14** OIC 14 (cash), 15 (receivables), 18 (prepayments and accruals), 19 (payables), 25
    (income taxes), 28 (equity), 31 (provisions and TFR), 32 (derivatives): subject and the
    paragraphs that bear on Appendix A.
15. **V15** ESMA, Guidelines on disclosure requirements under the Prospectus Regulation,
    ESMA32-382-1138 of 4 March 2021, guideline 39: the net financial indebtedness table (rows
    A to M, total financial indebtedness H + L), adopted by Consob, Richiamo di attenzione
    n. 5/21 of 29 April 2021, which supersedes the references to the former CESR scheme
    (Comunicazione DEM/6064293/2006). Table structure and reference verified on 2026-11-08.
    Still to verify: whether it includes lease liabilities, derivatives and non-current trade
    payables (row K covers non-current trade and other payables).
16. **V16** CESR recommendation CESR/05-054b: superseded by V15; confirm before citing.
17. **V17** ESMA, Guidelines on Alternative Performance Measures, ESMA/2015/1415: EBITDA and the
    labelling of items as non-recurring.
18. **V18** Consob, Comunicazione DEM/6064293 of 28 July 2006: definition of significant
    non-recurring events and transactions.
19. **V19** Directive 2013/34/EU: Annexes III and IV (balance sheet layouts), V (profit and loss
    by nature), VI (by function), article 3 (size categories), and the English item names used
    as identifiers.
20. **V20** IAS 1 paragraphs 66 to 76 (current and non-current classification) and 99 to 105
    (analysis of expenses by nature or by function); IFRS 18 (issued April 2024, effective
    1 January 2027), which replaces IAS 1.
21. **V21** IFRS 16, lessee accounting: right-of-use asset and lease liability.
22. **V22** IAS 19 and the classification of TFR (defined benefit for the amounts accrued before
    the reform of D.Lgs. 252/2005) in IFRS statements of Italian companies.
23. **V23** EBA, Guidelines on loan origination and monitoring, EBA/GL/2020/06: Annex 3
    (metrics for credit assessment) as the English source for bank practice terms.
24. **V24** Eurostat structural business statistics: definitions of production value, value
    added at factor cost, personnel costs and gross operating surplus (legal basis and variable
    codes), and how they differ from the Italian value added layout (indirect taxes in B14).
25. **V25** Koller, Goedhart, Wessels, *Valuation*, 7th edition, chapter on reorganising the
    financial statements (invested capital, operating working capital, total funds invested,
    debt equivalents); behind a paywall in the SPEC-0001 review.
26. **V26** CFA Institute curriculum, financial reporting and analysis readings: EBITDA, working
    capital, current and non-current classification.
27. **V27** Italian corporate finance and financial statement analysis textbooks that publish
    the reclassification schemes and worked examples (edition and page to be named by the
    maintainer). Candidates: G. Ferrero, F. Dezzani, P. Pisoni, L. Puddu, *Le analisi di
    bilancio. Indici e flussi* (Giuffrè); C. Caramiello, F. Di Lazzaro, G. Fiori, *Indici di
    bilancio* (Giuffrè); C. Teodori, *L'analisi di bilancio* (Giappichelli).
28. **V28** Reclassification schemes used by Italian banks in credit assessment (for example the
    Centrale dei Bilanci scheme, now Cerved), in particular for TFR, shareholder loans, leases,
    tax debts in instalments and quasi-equity.

## Open questions
Each question lists the options and a recommendation. Nothing is decided until the maintainer
answers.

- [ ] **1. Method granularity.** (a) Three methods as in R1, each computing one or more criteria
      or layouts requested in the call (R17); (b) one method per criterion and per layout (four
      reclassification ids plus the statutory one); (c) one method `statements.reclassify`
      returning everything. Recommendation: (a). The classifications are shared within a
      statement, so one call gives consistent outputs; requesting only the needed criteria keeps
      the required choices to the minimum; ids stay few for agents.
- [ ] **2. `statements.statutory_accounts` as a method.** (a) A method; (b) only a shared type
      used by the two reclassification methods. Recommendation: (a). An agent that extracts a
      PDF needs a cheap check with JSON pointers before any judgement is involved.
- [ ] **3. Layout declaration.** (a) Required, echoed, governs only the grouped nodes (R8);
      (b) optional, informative; (c) no declaration, any depth accepted. And: (d) keep
      `micro_entity` as a value although it uses the abridged layouts, or (e) fold it into
      `abridged`. Recommendation: (a) and (d). The declaration explains missing detail in the
      audit trail at no cost and the engine never checks thresholds.
- [ ] **4. Balance check tolerance.** (a) Exact equality, `FM-E026` otherwise; (b) an optional
      explicit tolerance in currency units, the difference reported; (c) a labelled
      "unreconciled difference" line that absorbs it. Recommendation: (a). Statements in euro
      units balance exactly because the rounding difference is already an item; an extraction
      error must be fixed, not absorbed. A user who needs (c) enters a labelled additional item,
      which is visible and classified.
- [ ] **5. Sign conventions.** Inputs: (a) as presented in the statutory layouts, with the
      signed items of R7; (b) every amount signed by its effect on the result. Outputs: (c)
      magnitudes, with subtraction in the formulas and signed subtotals; (d) signed amounts.
      Recommendation: (a) and (c), as the statutory documents and as SPEC-0001 and SPEC-0007
      (D&A, capex, debts positive and subtracted). The B11 sign (positive is a decrease, a cost)
      is the most frequent trap and is stated in the schema and the methodology page.
- [ ] **6. Employee severance indemnity (TFR).** Functional criterion: (a) operating provision,
      deducted from invested capital (textbook functional layout); (b) debt-like item outside
      the net financial position (valuation practice; passes to the DCF bridge); (c) other
      financial liability inside the net financial position. Financial criterion: (d) all
      non-current unless split by the analyst; (e) required split. Recommendation: required
      choice among (a), (b), (c) with no default, and (d). The bank and ESMA definitions of the
      net financial position do not include the TFR (to verify, V15, V28), so (c) is offered
      but not suggested in examples.
- [ ] **7. Provisions for risks and charges.** B.1 (pensions) and B.4 (other): (a) required
      choice operating provision or debt-like; (b) default operating provision. B.2 (taxes,
      including deferred): default operating provision. B.3 (derivative liabilities): required,
      other financial liability or operating. Financial criterion: non-current unless split.
      Recommendation: (a) for B.1 and B.4, defaults for B.2, required for B.3. Litigation and
      restructuring provisions are debt-like in valuation practice and operating in most
      textbook layouts; the analyst knows which.
- [ ] **8. Shareholder loans (D.3).** (a) Financial debt in the net financial position;
      (b) quasi-equity, a separate source line not in the net financial position nor in
      statutory equity; (c) a split (for example the postponed part under art. 2467, disclosed
      by art. 2427 n. 19-bis). Financial criterion: by statutory maturity in every case.
      Recommendation: required choice (a), (b) or (c) with no default; statutory equity is never
      changed, so `ratios.*` keep their meaning and the quasi-equity effect is visible.
- [ ] **9. Prepayments and accrued income, accruals and deferred income.** (a) Default current
      and operating, reported, with an analyst split for multi-year parts (for example a
      leasing maxi-instalment) and for accrued interest (other financial liability); (b) a
      required classification. Recommendation: (a). The statutory layout gives no split; the
      amounts are usually small; the default is visible in the explain line by line.
- [ ] **10. Leases.** (a) As reported only (OIC: payments in B8, no asset or debt; IFRS 16
      preparers use the reclassified layer); (b) optional restatement by the financial method
      from the art. 2427 n. 22 disclosures (R26), no deferred taxes unless given; (c) defer (b)
      to a later additive version. Recommendation: (b) in version 1. Bank practice restates
      leases for comparability of EBITDA and net financial position between OIC and IFRS 16
      preparers. If the maintainer prefers a smaller first version, (c): R26 and AC9 move to a
      new roadmap task without changing the rest.
- [ ] **11. Maturity of receivables and payables.** (a) The statutory split "within / beyond the
      next financial year" read as twelve months, reported, with analyst splits for overdue or
      doubtful receivables and for tax debts in instalments; (b) a required maturity
      classification per line. Deferred tax assets (C.II.5-ter): (c) follow the statutory split;
      (d) always non-current. Recommendation: (a) and (c). The statutory split is the only
      maturity data in the filing; the override covers the cases banks adjust.
- [ ] **12. Extraordinary and non-recurring items.** After D.Lgs. 139/2015 removed area E, former
      extraordinary items sit mainly in A5 and B14. (a) No separate area; (b) A5 and B14
      classified as operating, non-operating or non-recurring (required when not zero) and
      optional non-recurring parts of any other line, both presented below EBIT (R31).
      Recommendation: (b). It restores a normalised EBITDA explicitly, with every amount traced;
      the methodology page warns, as the ESMA APM guidelines do (to verify, V17), against
      labelling recurring items as non-recurring.
- [ ] **13. EBITDA scope.** (a) Fixed: write-downs of fixed assets (B10c), of receivables and cash
      (B10d) and provisions (B12, B13) below EBITDA; (b) a required choice for B10d and for
      B12, B13. Recommendation: (a), reported in words usable as `ebitda_definition` in
      SPEC-0007. It is the common Italian definition of MOL (to verify, V27, V28); an analyst
      who wants (b) adjusts the EBITDA in the ratio input and states it there.
- [ ] **14. Financial income and area D.** (a) Items 15, 16, 17, 17-bis as one net financial
      line below EBIT, area D on its own line; (b) financial income (15, 16) in an "operating
      result of the company" above interest expense (some Italian textbooks); (c) exchange
      differences on trade items in operating. Recommendation: (a), aligned with the
      international EBIT (before interest income and expense); (b) and (c) can be added later
      as additive layout options.
- [ ] **15. Mapping table.** Confirm Appendix A row by row, in particular: intragroup receivables
      and payables (C.II.2 to C.II.5, D.9 to D.11-bis) required when not zero; equity
      investments (B.III.1, C.III.1 to C.III.4) required; financial receivables and other
      securities (B.III.2, B.III.3) required; derivatives (B.III.4, C.III.5, B.3) required;
      current securities and treasury management assets (C.III.6, C.III.7) default financial
      asset; advances from customers (D.6) default trade; bills payable (D.8) default trade;
      tax and social security payables (D.12, D.13) default operating; intangible and tangible
      assets default operating with the non-operating option (for example property not used in
      the business). Recommendation: as in Appendix A. "Required" is reserved for lines where
      the statutory item does not reveal the nature and practice disagrees; it applies only when
      the amount is not zero.
- [ ] **16. Subscribed capital unpaid (assets A).** (a) Deducted from equity in full; (b) uncalled
      part deducted, called part as a current receivable. Recommendation: (a), reported, as in
      bank practice (to verify, V28): unpaid capital does not fund the company until paid.
- [ ] **17. Dividends declared.** (a) Optional labelled amount moved from equity to current
      liabilities (financial) and to debt-like items (functional); (b) not handled.
      Recommendation: (a). Statements are approved with the proposal to distribute; the analyst
      states the amount; none given means none, reported.
- [ ] **18. Net financial position presentation.** (a) The SPEC-0007 structure with the
      composition by line and current and non-current parts, cash = C.IV only (C.III in other
      items); (b) also the ESMA table rows A to M. Recommendation: (a) in version 1; (b) as a
      later additive output once V15 is verified, because the current and non-current parts
      already give its subtotals.
- [ ] **19. Cost of sales layout.** Functions: (a) cost of sales, distribution costs,
      administrative expenses, other expenses (IAS 1 paragraph 103; Directive Annex VI); (b) add
      research and development. Allocation: (c) required for B7, B8, B9, B10, B12, B13 and the
      operating part of B14 when not zero, with materials consumed (B6 + B11) default to cost of
      sales, A2 and A3 reducing cost of sales, A4 (own work capitalised) required split by
      function as a reduction; (d) all lines required. Recommendation: (a) and (c). The statutory
      income statement is by nature only; the functional split comes from management accounts
      and must be stated by the analyst.
- [ ] **20. Inventories under the financial criterion.** (a) All current by default, with an
      analyst split for slow-moving or safety stock; (b) all current, fixed. Recommendation: (a).
- [ ] **21. Changes between years.** Should the balance sheet method return, for consecutive
      years, the change in operating working capital and in fixed assets for the SPEC-0001
      build-up? (a) No, version 1 returns the levels only; the build-up stays the single owner
      of ΔNWC and capex; (b) yes, as additive outputs. Recommendation: (a), and a follow-up
      roadmap task under SPEC-0001 for a build-up from two reclassified balance sheets, where
      capex needs disposals that statements do not give.
- [ ] **22. Golden sources.** (a) The exact reference script `scripts/reference_statements.py`,
      written from Appendix A and B, for every case; (b) at least one published worked example
      per criterion and per layout from a textbook named by the maintainer (V27); (c) two or
      three anonymised practitioner cases from the maintainer, one abridged. Recommendation:
      all three; (b) and (c) are required for AC17 and for promotion to available.
- [ ] **23. Identifier terminology.** (a) IFRS and CFA Institute terms first, Directive
      2013/34/EU terms where IFRS has none (as in the terminology table); (b) Directive terms
      throughout, closest to the Civil Code. Recommendation: (a), consistent with SPEC-0001 and
      SPEC-0007 and with how agents name these quantities.

## Appendix A — Statutory lines and their destinations (proposed)
Choice: **F** fixed; **D** default, reported, overridable by the analyst; **R** required when
the amount is not zero. "NFP" means a component of the net financial position (R23). Under the
financial criterion every receivable and payable follows its statutory maturity split; the
column states only the class. "Abridged" marks the nodes at which the abridged layout stops or
groups.

### Balance sheet, assets (art. 2424)

| Reference | Proposed identifier | Financial criterion | Functional criterion | Choice | Abridged |
| --- | --- | --- | --- | --- | --- |
| A | `subscribed_capital_unpaid` | deducted from equity | deducted from equity | F (OQ 16) | may be in C.II |
| B.I.1 to B.I.7 | `intangible_assets`: `formation_and_expansion_costs`, `development_costs`, `patents_and_intellectual_property_rights`, `concessions_licences_and_trademarks`, `goodwill`, `in_progress_and_advances`, `other` | fixed assets | operating fixed assets, or non-operating | D | B.I total, gross and accumulated |
| B.II.1 to B.II.5 | `tangible_assets`: `land_and_buildings`, `plant_and_machinery`, `industrial_and_commercial_equipment`, `other`, `in_progress_and_advances` | fixed assets | operating fixed assets, or non-operating | D | B.II total, gross and accumulated |
| B.III.1 a to d-bis | `financial_fixed_assets.equity_investments`: `subsidiaries`, `associates`, `parent_companies`, `sister_companies`, `other_companies` | fixed assets | operating fixed assets, or non-operating | R | B.III total |
| B.III.2 a to d-bis | `financial_fixed_assets.receivables` (same counterparties, `other`), with `within_one_year` | within: current receivables; beyond: fixed assets | NFP financial asset, non-operating, or operating | R | B.III total |
| B.III.3 | `financial_fixed_assets.other_securities` | fixed assets | NFP financial asset, or non-operating | R | B.III total |
| B.III.4 | `financial_fixed_assets.derivative_assets` | fixed assets | NFP financial asset, operating, or non-operating | R | B.III total |
| C.I.1 to C.I.5 | `inventories`: `raw_materials_and_consumables`, `work_in_progress_and_semi_finished_goods`, `contract_work_in_progress`, `finished_goods_and_goods_for_resale`, `advances_to_suppliers` | inventories (OQ 20) | trade working capital | D | C.I total |
| C.II.1 | `receivables.trade` | receivables | trade working capital | F | C.II total, with beyond |
| C.II.2 to C.II.5 | `receivables`: `subsidiaries`, `associates`, `parent_companies`, `sister_companies` | receivables | trade working capital, NFP financial asset, or non-operating | R | C.II total |
| C.II.5-bis | `receivables.tax` | receivables | other operating assets | D | C.II total |
| C.II.5-ter | `receivables.deferred_tax_assets` | receivables (OQ 11) | other operating assets | D | C.II total |
| C.II.5-quater | `receivables.other` | receivables | other operating assets, NFP financial asset, or non-operating | D | C.II total |
| C.III.1 to C.III.4 | `current_financial_assets.equity_investments`: `subsidiaries`, `associates`, `parent_companies`, `sister_companies`, `other` | current financial assets | NFP financial asset, or non-operating | R | C.III total |
| C.III.5 | `current_financial_assets.derivative_assets` | current financial assets | NFP financial asset, operating, or non-operating | R | C.III total |
| C.III.6 | `current_financial_assets.other_securities` | current financial assets | NFP financial asset | D | C.III total |
| C.III.7 | `current_financial_assets.treasury_management_assets` | current financial assets | NFP financial asset | D | C.III total |
| C.IV.1 to C.IV.3 | `cash`: `bank_and_postal_deposits`, `cheques`, `cash_on_hand` | cash | NFP cash | F | C.IV total |
| D | `prepayments_and_accrued_income` | current, or split (OQ 9) | other operating assets, or split | D | may be in C.II |

### Balance sheet, liabilities and equity (art. 2424)

| Reference | Proposed identifier | Financial criterion | Functional criterion | Choice | Abridged |
| --- | --- | --- | --- | --- | --- |
| A.I to A.X | `equity`: `share_capital`, `share_premium_reserve`, `revaluation_reserves`, `legal_reserve`, `statutory_reserves`, `other_reserves`, `cash_flow_hedge_reserve`, `retained_earnings`, `profit_or_loss_for_the_year`, `negative_reserve_for_own_shares` | equity | equity | F | roman numerals |
| B.1 | `provisions.pensions_and_similar_obligations` | non-current, or split | operating provision, or debt-like | R (OQ 7) | B total |
| B.2 | `provisions.taxes` | non-current, or split | operating provision, or debt-like | D | B total |
| B.3 | `provisions.derivative_liabilities` | non-current, or split | NFP other financial liability, or operating | R | B total |
| B.4 | `provisions.other` | non-current, or split | operating provision, or debt-like | R (OQ 7) | B total |
| C | `employee_severance_indemnity` | non-current, or split | operating provision, debt-like, or NFP other financial liability | R (OQ 6) | C |
| D.1, D.2 | `payables.bonds`, `payables.convertible_bonds` | payables | NFP gross debt | F | D total, with beyond |
| D.3 | `payables.shareholder_loans` | payables | NFP gross debt, or quasi-equity | R (OQ 8) | D total |
| D.4 | `payables.banks` | payables | NFP gross debt | F | D total |
| D.5 | `payables.other_lenders` | payables | NFP gross debt | F | D total |
| D.6 | `payables.advances_from_customers` | payables | trade working capital | D | D total |
| D.7 | `payables.trade` | payables | trade working capital | F | D total |
| D.8 | `payables.bills_payable` | payables | trade working capital, or NFP gross debt | D | D total |
| D.9 to D.11-bis | `payables`: `subsidiaries`, `associates`, `parent_companies`, `sister_companies` | payables | trade working capital, NFP gross debt, or other operating liabilities | R | D total |
| D.12, D.13 | `payables.tax`, `payables.social_security` | payables | other operating liabilities, or debt-like | D (OQ 11) | D total |
| D.14 | `payables.other` | payables | other operating liabilities, debt-like, or NFP gross debt | D | D total |
| E | `accruals_and_deferred_income` | current, or split (OQ 9) | other operating liabilities, or split (accrued interest: NFP other financial liability) | D | may be in D |

### Income statement (art. 2425)

| Reference | Proposed identifier | Value added layout | Cost of sales layout | Choice | Abridged |
| --- | --- | --- | --- | --- | --- |
| A1 | `revenue` | value of production | revenue | F | |
| A2, A3 | `change_in_product_inventories`, `change_in_contract_work_in_progress` (signed) | value of production | reduce cost of sales | F | may be grouped |
| A4 | `own_work_capitalised` | value of production | reduces the functions, split | F / R (cost of sales) | |
| A5 | `other_revenue_and_income`, with `operating_grants` | operating, non-operating or non-recurring | other operating income, non-operating or non-recurring | R (OQ 12) | |
| B6 | `raw_materials_and_goods` | external costs (materials consumed) | cost of sales | F / D (cost of sales) | |
| B7 | `services` | external costs | split by function | F / R | |
| B8 | `use_of_third_party_assets` | external costs | split by function | F / R | |
| B9 a to e | `personnel_costs`: `wages_and_salaries`, `social_security_contributions`, `severance_indemnity`, `pension_costs`, `other` | personnel costs | split by function | F / R | c, d, e may be grouped |
| B10 a, b | `amortisation_of_intangible_assets`, `depreciation_of_tangible_assets` | depreciation and amortisation | split by function | F / R | a, b, c may be grouped |
| B10 c, d | `other_write_downs_of_fixed_assets`, `write_downs_of_receivables_and_cash` | write-downs | split by function | F / R | |
| B11 | `change_in_raw_material_inventories` (signed, positive is a decrease) | external costs (materials consumed) | cost of sales | F / D | |
| B12, B13 | `provisions_for_risks`, `other_provisions` | provisions | split by function | F / R | |
| B14 | `other_operating_expenses` | operating, non-operating or non-recurring | operating part split by function | R (OQ 12) | |
| C15, C16 a to d | `income_from_equity_investments`, `other_financial_income` | net financial income and expense | same | F | 16 b, c may be grouped |
| C17 | `interest_and_other_financial_charges` | net financial income and expense | same | F | |
| C17-bis | `exchange_gains_and_losses` (signed) | net financial income and expense | same | F (OQ 14) | |
| D18, D19 a to d | `revaluations`, `write_downs_of_financial_assets` | value adjustments of financial assets | same | F | a to d may be grouped |
| 20 | `income_taxes` (optional components) | income taxes | same | F | |
| 21 | `profit_or_loss_for_the_year` (signed) | check of net income | check | F | |

Any line not marked R in the income statement accepts an optional labelled non-recurring part
(R31). The grouped nodes in the "Abridged" column are to be checked against art. 2435-bis (V7).

## Appendix B — Reclassified structures (proposed)

### B.1 Financial criterion (one balance sheet date)
- `fixed_assets` = intangible + tangible + financial fixed assets (beyond one year) + receivables
  beyond one year + non-current prepayments and accrued income + non-current inventories
  (analyst split).
- `current_assets` = `inventories` + current receivables (within one year) + current financial
  assets + current prepayments and accrued income + `cash`.
- `total_assets` = `fixed_assets` + `current_assets` (statutory total assets − subscribed
  capital unpaid ± lease restatement, each reported).
- `equity` = statutory equity − subscribed capital unpaid − dividends declared ± lease equity
  adjustment.
- `non_current_liabilities` = payables beyond one year + TFR and provisions (non-current part) +
  non-current accruals and deferred income + non-current lease liability.
- `current_liabilities` = payables within one year + TFR and provisions (current part) + current
  accruals and deferred income + dividends declared + current lease liability.
- Check: `total_assets` = `equity` + `non_current_liabilities` + `current_liabilities`.

### B.2 Functional criterion (one balance sheet date)
- Operating fixed assets = intangible + tangible + financial fixed assets classified operating.
- `trade_working_capital` = trade receivables + inventories + intragroup trade receivables −
  trade payables − advances from customers − bills payable (trade) − intragroup trade payables.
- `operating_working_capital` = `trade_working_capital` + other operating assets (including
  operating prepayments) − other operating liabilities (including operating accruals).
- Operating provisions = TFR and provisions classified operating.
- `invested_capital` = operating fixed assets + `operating_working_capital` − operating
  provisions.
- `total_funds_invested` = `invested_capital` + Σ non-operating assets.
- Sources = `equity` + Σ quasi-equity items + `net_financial_position` + Σ debt-like items.
- Check: `total_funds_invested` = sources.

### B.3 Value added layout (one financial year)
- `value_of_production` = A1 + A2 + A3 + A4 + A5 (operating part).
- External costs = (B6 + B11) + B7 + B8 + B14 (operating part).
- `value_added` = `value_of_production` − external costs.
- `ebitda` = `value_added` − B9.
- `ebit` = `ebitda` − (B10a + B10b) − (B10c + B10d) − (B12 + B13).
- Non-operating result = A5 (non-operating part) − B14 (non-operating part).
- Non-recurring result = Σ non-recurring income parts − Σ non-recurring expense parts.
- Net financial income and expense = C15 + C16 − C17 + C17-bis.
- Value adjustments of financial assets = D18 − D19.
- Profit before tax = `ebit` + non-operating result + non-recurring result + net financial
  income and expense + value adjustments of financial assets.
- `net_income` = profit before tax − item 20; check: equals item 21.
- Each line from which a non-recurring part was moved enters its subtotal net of that part.

### B.4 Cost of sales layout (one financial year)
- `cost_of_sales` = Σ parts allocated to cost of sales of (B6 + B11), B7, B8, B9, B10, B12, B13,
  B14 (operating part) − (A2 + A3) − A4 (part allocated to cost of sales).
- `gross_profit` = A1 − `cost_of_sales`.
- `distribution_costs`, `administrative_expenses`, `other_expenses` = Σ parts allocated to each
  function − A4 (part allocated to it).
- Other operating income = A5 (operating part).
- `ebit` = `gross_profit` − `distribution_costs` − `administrative_expenses` − `other_expenses`
  + other operating income; equal to the `ebit` of B.3 when every allocation is complete.
- `ebitda` = `ebit` + depreciation and amortisation + write-downs + provisions (R30).
- Below EBIT: as B.3.
