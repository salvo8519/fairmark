# SPEC-0005 — Financial statements: reorganising statutory or reclassified totals into an operating and financing view

| | |
| --- | --- |
| Status | Implementing — approved 2026-11-08, revision 5 (best-effort input, optional dates, reclassified input form, IFRS 18 subtotals); maintainer decisions 1 to 18 recorded below |
| Milestone | M2 (release 0.1.0); epic #34; issues #35 and #36 |
| Owner | @salvo8519 |
| Formula basis | The corporate-finance reorganisation of the financial statements into operating, investing and financing items (Koller, Goedhart, Wessels, *Valuation*, "Reorganizing the financial statements"; CFA Institute curriculum), with the subtotals of IFRS 18 (operating profit; profit before financing and income taxes) and of Italian credit analysis (MOL, reddito operativo, EBIT; OIC 12; art. 2427 n. 13; Banca d'Italia; EBA/GL/2020/06), applied to the aggregates of the statutory layouts (art. 2424, 2425, abridged art. 2435-bis) or to totals already reclassified; net financial position in the SPEC-0007 structure with ESMA32-382-1138 as reference; verified against an exact decimal reference script, published worked examples and practitioner cases (Decision 16) |
| Related | SPEC-0001 (free cash flow build-up, NOPAT taxes, bridge types), SPEC-0003 (tax rates; no IRAP logic), SPEC-0007 (`NetFinancialPosition`, every ratio and margin); ADR 0001, ADR 0009, ADR 0012 |

Approved by the maintainer on 2026-11-08.

## History
- Revision 1: statutory line items, two balance sheet criteria, two income statement layouts.
- Revision 2: statutory totals only; one operating and financing view; labelled adjustments.
- Revision 3: core and non-operating areas of the income statement; write-down rule;
  authoritative sources.
- Revision 4: consecutive financial years; choices stated once; exact changes between years.
- **Revision 5 (this one)**, from the maintainer's guidance of 2026-11-08 (simple, scalable,
  intuitive for agents and humans):
  - Best effort. Every input node is optional. An output is computed when its inputs are
    present; otherwise it is null and listed in `not_computed` with the missing inputs. Missing
    data is never an error and never a silent zero. Checks run only when both sides are given.
    A missing treatment is also missing data, not an error.
  - A second input form, `reclassified`, takes totals that are already reclassified; the output
    structure is the same.
  - Dates and labels of years are optional; order is oldest first; dates, when given, are
    checked.
  - Income statement subtotals renamed and aligned with IFRS 18 and Italian practice:
    `operating_profit` (reddito operativo, MON) and `profit_before_financing_and_taxes`
    (reddito operativo aziendale, EBIT in Italian practice). C15, C16, D18 and D19 form the
    investing area; C17 and C17-bis the financing area. Open question 18 is resolved; Open
    question 17 stays open, now with an IFRS 18 source.
  - Later on 2026-11-08 the maintainer decided Open question 17 (Decision 17): every write-down
    and revaluation is extra-characteristic by default; B10 d may be treated as operating.
  - Removed as redundant: the requirement that C items be given separately (now best effort);
    the C16 split adjustment (C16 is investing as a whole); the ordering of reconciliation
    before classification errors (a missing classification is no longer an error); the
    standalone statutory check method from the future extensions.
- **Approval.** Approved by the maintainer on 2026-11-08: revision 5 as written, with decisions
  1 to 18. Plan and tasks: `plan.md` and `tasks.md` in this folder.
- **Clarifications of 2026-11-08 (maintainer, after approval).** R2 clarified on 2026-11-08,
  end-of-month closings: two month-end dates one year apart are consecutive (28 February 2023
  followed by 29 February 2024), consistent with the end-of-month rule of SPEC-0010; acceptance
  case in AC6. R8 clarified on 2026-11-08: `default` destinations of Appendix A applied to a
  given aggregate are also listed in `meta.defaults_applied` (pointer, destination, reason),
  consistent with SPEC-0001 Decision 24; acceptance in AC13. `FM-E026` confirmed free on main.

## Analysis

### What is on main today
- No `statements` theme.
- `ratios.*` (SPEC-0007) take: `equity`, `total_assets`, `fixed_assets`,
  `non_current_liabilities`, `net_financial_position` (`ratios.capital_structure`);
  `current_assets`, `inventories`, `cash`, `current_liabilities` (`ratios.liquidity`);
  `trade_receivables`, `inventories`, `trade_payables`, `revenue`, `cost_of_sales`, `purchases`
  (`ratios.working_capital`); `equity`, `total_assets`, `invested_capital`, `revenue`, `ebitda`,
  `ebit`, `net_income` (`ratios.profitability`); `net_financial_position`, `ebitda`, `ebit`,
  `interest` (`ratios.debt_service`).
- `dcf.free_cash_flows` (SPEC-0001 R2 to R5) builds FCFF from `ebit`, taxes on EBIT,
  `depreciation_and_amortisation`, `capital_expenditure` and `change_in_net_working_capital`;
  `FreeCashFlowLines` (issue #180) carries the same lines into `dcf.fcff`. NOPAT is computed
  there.
- `dcf.fcff` version 3 takes `net_financial_position` (SPEC-0007 type) and
  `non_operating_assets` (`tv::Line`). Debt-like items and the measurement basis of bridge items
  come with issue #44 and SPEC-0001 Decision 21 (`MeasurementBasis`). **Dependency:** Decision 21
  and `MeasurementBasis` are not on the branch of this worktree; the plan takes them from main.

### Presentations that existing ratios need
- `ratios.liquidity` and `ratios.capital_structure` need current and non-current amounts. They
  come from the statutory split of C.II and D, so a minimal block `current_non_current` with
  the SPEC-0007 field names is kept (R15); the full liquidity criterion is not.
- `ratios.working_capital` needs `cost_of_sales`, which an income statement by nature does not
  contain. The method returns `purchases` (B6) and `materials_consumed` (B6 + B11) under their
  own names and never as `cost_of_sales` (Decision 13).

### Banca d'Italia and EBA indicators against `ratios.*` on main
Checked on `crates/fairmark-core/src/ratios/` in this worktree.

| Indicator | Source | On main |
| --- | --- | --- |
| Leverage = financial debt / (financial debt + equity) | Banca d'Italia (V30) | No: `debt_to_equity`, `nfp_to_equity` and `equity_ratio` exist |
| Financial charges / MOL | Banca d'Italia (V30) | Inverse only: `interest_cover_ebitda` |
| Financial debt / MOL | Banca d'Italia (V30) | Net form only: `nfp_to_ebitda` |
| EBITDA | EBA/GL/2020/06 Annex 3 (V23) | Input; produced by this method |
| Debt to equity; debt service coverage | EBA/GL/2020/06 Annex 3 | Yes: `debt_to_equity`; `dscr`, `llcr` |
| Interest cover; net debt / EBITDA | Credit practice | Yes: `interest_cover_ebit`, `interest_cover_ebitda`, `nfp_to_ebitda` |
| Debt to be restructured / annual free flows serving the debt (test pratico) | Ministry of Justice decree of 23 April 2026 (V31) | No |

The missing forms are future extensions 6 and 7. This method returns every input they need.

## Problem
Capital budgeting, valuation and credit analysis need the statements split into what the
business uses and earns (operating), what it holds outside its business (investing, or
accessory) and how it is financed. Statutory aggregates mix them: payables D hold bank loans
next to trade payables; provisions and the employee severance indemnity may be operating or
debt-like; financial fixed assets may be strategic, surplus or financial; C and D of the income
statement mix investment income and financing costs. Users rarely have everything: an agent
may read a few totals from a text, an analyst may hold a bank's reclassified statement. The
engine must take whatever totals are given, compute what they allow, say exactly what could
not be computed and why, check consistency wherever both sides are given, record every choice
and adjustment, and return outputs that `ratios.*` and `dcf.*` accept as they are.

## Scope
**In scope (version 1):** one method, `statements.reorganise`, that takes 1 to n financial years
(typically the two or three of a filing with its comparative column, art. 2423-ter), each in the
`statutory` form (aggregates of art. 2424 and 2425 in the ordinary, abridged or micro-entity
layout) or in the `reclassified` form (totals already reclassified), any subset of the totals,
and returns per year:
- the reorganised balance sheet: operating block up to invested capital, non-operating assets
  as a separate block, equity, quasi-equity, net financial position, debt-like items;
- the reorganised income statement: value of production, value added, EBITDA, operating profit,
  investing result, profit before financing and taxes, non-recurring result, financing result,
  profit before tax, taxes as given, net income;
- the current and non-current block for SPEC-0007;
- and, between consecutive years, the exact changes in operating working capital, net
  financial position and equity.

**Out of scope, owned elsewhere:** NOPAT taxes, FCFF and FCFE (SPEC-0001); tax rates, IRES and
IRAP (SPEC-0003); ratios and margins (SPEC-0007).

**Future extensions (each additive):**
1. Statutory line items (arabic numerals of art. 2424 and 2425) as a richer input layer.
2. The financial (liquidity) criterion as a full presentation; the cost of sales layout.
3. The full ESMA32-382-1138 net financial indebtedness table (rows A to M).
4. An estimate of capital expenditure from two balance sheets, labelled as an estimate in its
   name and in `explain` (disposals, write-downs and revaluations are not in the aggregates).
5. IFRS layouts (IFRS 18) as a statutory input form; consolidated accounts.
6. A debt sustainability method computing the ratio of the practical test of the negotiated
   settlement of the business crisis (decree of the Ministry of Justice of 23 April 2026, V31),
   with the deferred `ratios.crisis_indicators` of SPEC-0007 (CNDCEC indicators).
7. Banca d'Italia indicators in SPEC-0007: financial debt / (financial debt + equity), financial
   charges / EBITDA, gross financial debt / EBITDA.

**Not planned:** XBRL or PDF parsing; layouts of banks and insurers; credit scoring.

## User stories
- As an AI agent, I want to pass the few totals I read in a text and get every output they
  allow, with the list of what is missing for the rest, so that I ask the user only for that.
- As an analyst with a bank's reclassified statement, I want to pass its totals in the same
  tool and get the same outputs and checks.
- As an analyst, I want to pass the two or three years of a filing, state each choice once and
  read the exact changes between years.
- As a valuation analyst, I want the net financial position, non-operating assets and debt-like
  items in the shapes `dcf.fcff` accepts.
- As a reviewer, I want every adjustment with its amount, reason, source and destination in the
  explain.

## Requirements (EARS)

### Input
- **R1** — THE ENGINE SHALL provide the method `statements.reorganise` version 1, status preview
  (ADR 0012), taking either input form and returning one output structure (Decision 1).
- **R2** — THE ENGINE SHALL take a list `years` of one or more entries, oldest first, each with an
  optional `label` (for example "2024"), an optional `date` (ISO 8601 balance sheet date;
  `FM-E009` when invalid) and exactly one form, `statutory` or `reclassified` (`FM-E001` when
  both). WHEN two consecutive entries both have a date, the later SHALL be exactly one calendar
  year after the earlier (same month and day; 28 February follows 29 February) or, when both
  are the last day of their month, the last day of the same month one calendar year later
  (end-of-month convention, as in SPEC-0010: 29 February 2024 follows 28 February 2023),
  otherwise THE ENGINE SHALL return `FM-E001` naming the entry and stating the fix (list the years oldest
  first, one financial year apart; reorganise a year of different length in a separate call).
  WHEN a date is missing, the convention SHALL state that the order given is taken as oldest
  first. One entry, with or without a date, SHALL be valid.
- **R3** — Every input node SHALL be optional. Amounts SHALL be in the currency units of the
  statements, one unit per call, never scaled, and entered as presented in the statutory
  layouts: zero or more, except the signed items A2, A3 (positive for an increase in
  inventories), B11 (positive for a decrease, a cost), C17-bis (positive for a net gain), 20
  (positive for a tax charge), 21, the equity items A.VI to A.IX and A.X (zero or negative), and
  the signed totals of the reclassified form listed in Appendix B; a negative amount elsewhere
  SHALL be `FM-E001` naming the field (Decision 3).
- **R4** — The `statutory` form SHALL accept the aggregates of Appendix A: the letters and Roman
  numerals of art. 2424, the aggregates of art. 2425 with the groupings of art. 2435-bis, and
  the "of which" parts of Appendix A (amounts beyond the next financial year of C.II and D; the
  financial debt, shareholder loans and trade parts of D; the derivative part of B; B10 c within
  B10; assets A and D within C.II and liabilities E within D). Every node SHALL accept a total,
  its components or both.
- **R5** — The `reclassified` form SHALL accept the output fields of Appendix B as inputs, each
  as a total or with its components, and the net financial position as a SPEC-0007 object or a
  total. THE ENGINE SHALL derive every total that its formula in Appendix B allows from what is
  given. Treatments and adjustments SHALL NOT apply to a reclassified entry; an adjustment amount
  for it SHALL be `FM-E001` stating the fix (give the totals already adjusted).

### Best effort and checks
- **R6** — WHEN an output cannot be computed because an input it needs is missing, THE ENGINE
  SHALL return it as null and add to `not_computed` an entry with the output's pointer and the
  pointers of the missing inputs (for example `/years/0/statutory/balance_sheet/payables/
  financial_debt` for the net financial position); it SHALL NOT return an error and SHALL NOT
  use zero for a missing input. Every output SHALL state in `explain` the form it came from.
- **R7** — WHEN both sides of a relation are given, THE ENGINE SHALL check it exactly, with no
  tolerance (Decision 2), and return `FM-E026` with both amounts, the difference, the pointers
  and the fix: a total and its components; net = gross − accumulated (B.I, B.II); total assets
  = total liabilities and equity; the profit of A.IX and item 21; a given total of the
  reclassified form and its value derived by Appendix B; a split and its aggregate. A relation
  with a side missing SHALL be reported as not checked.

### Choices
- **R8** — THE ENGINE SHALL assign every statutory aggregate to its destination in Appendix A:
  `fixed` and `default` destinations are applied and reported; a `required` treatment is needed
  only when the aggregate is given and not zero. WHEN it is needed and missing, the outputs that
  depend on the aggregate SHALL be not computed (R6), with the treatment's pointer and the
  permitted destinations; WHEN it is not needed, the convention SHALL say so. A destination not
  permitted for the aggregate SHALL be `FM-E001`. WHEN a `default` destination is applied to a
  given aggregate, THE ENGINE SHALL also list it in `meta.defaults_applied` with the pointer of
  the input item, the destination as value and the reason, so that every default is read in one
  place (as SPEC-0001 Decision 24), besides the assignment and the convention; the same holds for
  the default `investing` of R17.
- **R9** — Treatments SHALL be stated once for the call and apply to every year. A per-year
  `treatment_overrides` entry SHALL be accepted only for financial fixed assets (B.III) and
  shareholder loans, as another destination or a split by amounts summing to that year's
  aggregate (`FM-E026` otherwise); an override of any other aggregate SHALL be `FM-E001`
  (Decisions 6, 10).
- **R10** — Adjustments SHALL be stated once for the call, each with a kind of Appendix C, a
  label, a reason (not blank) and `amounts`, a list with one entry per year in the order of
  `years` (null where it does not apply, reported). A list of another length, a blank reason or
  a source not permitted for the kind SHALL be `FM-E001`; an amount larger than what remains in
  its source SHALL be `FM-E026`.
- **R11** — THE ENGINE SHALL report in `explain` one step per aggregate (reference, pointer,
  amount, destination, origin `mapping` or `analyst`), one step per adjustment and year (kind,
  label, amount, reason, source, target), and every total with its components substituted.

### Balance sheet
- **R12** — THE ENGINE SHALL return the reorganised balance sheet of Appendix B.1 in three blocks:
  the operating block (operating fixed assets, operating working capital, operating provisions,
  `invested_capital`), the non-operating block (`non_operating_assets` listed one by one, the
  same items and labels returned for the bridge, never part of `invested_capital`), and the
  sources (`equity`, quasi-equity items, `net_financial_position`, `debt_like_items`); and SHALL
  hold `total_funds_invested` = sources exactly (`FM-E026` in the reclassified form when both
  sides are given; `FM-E012` in the statutory form, where it follows from R7).
- **R13** — THE ENGINE SHALL return the net financial position in the `NetFinancialPosition` type
  of SPEC-0007 R6 (positive for net debt), with its composition by source and its current and
  non-current parts, its total computed by the SPEC-0007 function; C.III and all of C.IV are in
  it by default and operating cash leaves it only by adjustment (Decision 11).
- **R14** — THE ENGINE SHALL return non-operating assets and debt-like items in the types of the
  `dcf.fcff` bridge, with the `MeasurementBasis` of SPEC-0001 Decision 21 set to the carrying
  amount (Decision 15).
- **R15** — THE ENGINE SHALL return the block `current_non_current` with the field names of the
  SPEC-0007 inputs, from the statutory split of C.II and D read as twelve months, provisions and
  the employee severance indemnity non-current and prepayments and accruals current unless their
  non-current part is given, each default reported (Decision 12). It SHALL NOT compute ratios or
  margins.

### Income statement
- **R16** — THE ENGINE SHALL return the layers of Appendix B.2, in order: `value_of_production`,
  `value_added`, `ebitda` (MOL), `operating_profit` (reddito operativo of the core management,
  MON; IFRS 18 "operating profit or loss"), `investing_result` (extra-characteristic area: the
  investing parts of A5 and B14, C15, C16, D18 − D19, and the write-downs B10 c and B10 d under
  R17), `profit_before_financing_and_taxes` (reddito operativo
  aziendale, EBIT in Italian practice; IFRS 18 "profit or loss before financing and income
  taxes") = `operating_profit` + `investing_result`, `non_recurring_result`, `financing_result`
  (C17-bis − C17, with C17 also as `interest_and_other_financial_charges`), `profit_before_tax`,
  `income_taxes` (item 20 as given) and `net_income`. `operating_profit` and
  `profit_before_financing_and_taxes` exclude the labelled non-recurring items; with none given
  they have the content of the IFRS 18 subtotals. IFRS 18 applies to IFRS preparers and art. 2425
  to OIC preparers; the reorganisation maps both to these subtotals.
- **R17** — Write-downs and revaluations SHALL be extra-characteristic (Decision 17), reported in
  `explain.conventions`: D18 and D19 (financial assets), B10 c (fixed assets) and B10 d (current
  receivables and cash) SHALL be in `investing_result`, below `operating_profit` and above
  `profit_before_financing_and_taxes`, and no adjustment may move D18 or D19. For B10 d only, an
  optional treatment stated once for the call, `write_downs_of_receivables: operating`, SHALL
  place it in `operating_profit`, below EBITDA; without it the default `investing` applies and the
  convention says so. EBITDA SHALL be the same either way. B10 a to c given as one group with no
  "of which" for B10 c SHALL be depreciation and amortisation, reported; giving the B10 c part
  places it in the investing result (Decision 8).
- **R18** — A5 and B14 SHALL be core by default, reported; `investing` and `non_recurring`
  adjustments SHALL move labelled parts out of them, and `non_recurring` adjustments out of any
  aggregate except D18 and D19 (Decision 7). EBITDA SHALL be value added − personnel costs, with
  B10, B12 and B13 below it, reported in words usable as `ebitda_definition` in SPEC-0007.
- **R19** — THE ENGINE SHALL NOT compute IRES, IRAP, deferred taxes or NOPAT. Profit before tax
  SHALL equal the statutory A − B ± C ± D and net income item 21, exactly, when given; THE
  ENGINE SHALL return the reconciliation of `operating_profit` and of
  `profit_before_financing_and_taxes` with A − B, listing every amount moved.
- **R20** — Outputs SHALL feed other methods under their documented meaning (Decision 18):
  `operating_profit` as `ebit` of `dcf.free_cash_flows` and of ROI on invested capital in
  `ratios.profitability`; `profit_before_financing_and_taxes` as `ebit` of the interest covers in
  `ratios.debt_service` and of ROA on total assets; `depreciation_and_amortisation`, `revenue`,
  `ebitda`, `net_income`, `purchases`, `invested_capital` under their own names. `cost_of_sales`
  SHALL NOT be returned.

### Adjustments affecting equity
- **R21** — A `lease` adjustment (financial method, Decision 9) SHALL add the asset to operating
  fixed assets and the liability to the net financial position, remove the prepayment, report
  the difference as an equity effect with its formula, move the lease payments out of B8, add
  depreciation and interest, and compute no deferred taxes.
- **R22** — Subscribed capital unpaid SHALL be deducted from equity; a `dividends_declared`
  adjustment SHALL move its amount from equity to debt-like items (Decision 14).

### Changes between years
- **R23** — For each pair of consecutive entries, THE ENGINE SHALL return, labelled with their
  labels or dates, the exact changes from the earlier to the later entry:
  `change_in_operating_working_capital` (positive for an increase, the sign of
  `change_in_net_working_capital` in SPEC-0001), `change_in_net_financial_position` (positive
  for more net debt) and `change_in_equity`, each the difference of the reorganised amounts with
  both substituted in `explain`; a change whose amount is missing in either entry SHALL be not
  computed (R6). With one entry, `changes` SHALL be empty and the convention SHALL say so.
  Capital expenditure SHALL NOT be estimated.

### Errors and stability
- **R24** — Every error SHALL name the field by its JSON pointer and state the fix, using
  `FM-E001`, `FM-E008`, `FM-E009`, `FM-E012` and one new code, `FM-E026` "statements do not
  reconcile". Choices SHALL be enumerations or labelled objects, never booleans. The arithmetic
  SHALL be additions and subtractions of decimals, with no rounding.

## Conventions and parameters

| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Years | oldest first; dates optional, checked when consecutive entries have them; month-end closings one year apart are consecutive | order assumed and reported when a date is missing | Art. 2423-ter (V2); end-of-month rule of SPEC-0010 |
| Defaults | every `default` destination and the default of R17 listed in `meta.defaults_applied` | fixed | R8; SPEC-0001 Decision 24 |
| Input form per year | `statutory`, `reclassified` | one per year | Decision 1, revision 5 |
| Missing input | output null, listed in `not_computed` | never zero, never an error | Revision 5 |
| Checks | exact, only when both sides are given | not checked otherwise, reported | Decision 2 |
| Layout (statutory) | `ordinary`, `abridged`, `micro_entity` | optional, echoed; no number depends on it | Art. 2424, 2425, 2435-bis, 2435-ter |
| Signs | as presented in the statutory layouts | fixed | Decision 3; OIC 12 (V12) |
| Treatments; adjustments | stated once; adjustment amounts per year; overrides for B.III and shareholder loans | fixed | Revision 4 |
| Provisions B, TFR C | `operating_provision`; `debt_like` by adjustment | default | Decision 5; ESMA32-382-1138 guideline 39 (V15) |
| B.III; shareholder loans; derivatives in B | required treatment when given and not zero | none | Decisions 5, 6, 10 |
| Net financial position | SPEC-0007 structure; C.III and C.IV inside; operating cash by adjustment | default | Decision 11; ESMA (V15) |
| A5, B14 | core; investing and non-recurring parts by adjustment | default | Decision 7; OIC 12; art. 2427 n. 13 (V9) |
| C15, C16, D18, D19 | investing result | fixed | IFRS 18 paragraphs 53(a), 54(a) (V32); Italian textbooks (V33) |
| C17, C17-bis | financing result | fixed | IFRS 18 (V32) |
| EBITDA | value added − personnel costs | fixed | Decision 8; Banca d'Italia MOL (V30) |
| Write-downs and revaluations | D18, D19, B10 c, B10 d in the investing result; for B10 d only, the optional treatment `operating` (below EBITDA) | `investing` | Decision 17; OIC 12 (V12); IFRS 18 practice for the `operating` option (V32) |
| Leases | as reported; financial method by adjustment | as reported | Decision 9; art. 2427 n. 22 |
| Taxes | item 20 as given | no IRES, IRAP or NOPAT | SPEC-0003; SPEC-0001 |
| Bridge items | carrying amount | fixed | Decision 15; SPEC-0001 Decision 21 |

## Acceptance criteria
Every golden case compares exactly (tolerance 0).

- **AC1** (R4) — Golden `ordinary-aggregates.yaml` and `abridged-aggregates.yaml` (same company,
  the abridged one with grouped nodes and "of which" parts) give identical outputs.
- **AC2** (R6, R8) — Golden `few-totals.yaml` (A1, B9, B10 and C.IV only) returns the outputs
  they allow and lists every other output in `not_computed` with its missing pointers; no output
  is zero for lack of input. Golden `missing-treatment.yaml`: B.III given without a treatment
  leaves `invested_capital` and `net_financial_position` not computed, with the treatment
  pointer and destinations; the other outputs are computed.
- **AC3** (R5) — Golden `reclassified.yaml`: the reclassified totals of the AC1 company give the
  same outputs as AC1, and each output states its form; a mix of a statutory and a reclassified
  year returns the changes between them.
- **AC4** (R7) — One golden error case per `FM-E026` relation of R7; the same inputs with one
  side removed pass and report the relation as not checked.
- **AC5** (R2, R3, R5, R9, R10, R24) — `FM-E001` for: both forms in one entry; consecutive dates
  not one year apart or newest first; a negative unsigned amount; an override of provisions B;
  an `amounts` list of another length; a blank reason; an adjustment on a reclassified entry;
  `FM-E009` for an invalid date. Each message has a JSON pointer and the fix.
- **AC6** (R2, R23) — Golden `three-years.yaml` returns two sets of changes, each the exact
  difference of its two entries; property: their sum equals last minus first. Golden
  `one-year-no-date.yaml` returns no changes and both conventions (order assumed, one year).
  Entries 29 February 2024 and 28 February 2025 are consecutive; golden
  `end-of-month-dates.yaml`: entries 28 February 2023 and 29 February 2024 are consecutive
  (end-of-month closings), while 27 February 2023 and 29 February 2024 return `FM-E001`.
- **AC7** (R9, R10, R11) — One golden case per adjustment kind of Appendix C; a treatment stated
  once applies to every year; a B.III override applies to its year only; an amount larger than
  its source returns `FM-E026`. Property: assigned amounts sum to their aggregate.
- **AC8** (R12, R13) — Properties (random valid inputs): `total_funds_invested` = sources;
  no non-operating asset is in `invested_capital`; the net financial position total equals the
  SPEC-0007 total of the returned object.
- **AC9** (R14, R15, R20) — Contract tests: the net financial position, non-operating assets and
  debt-like items are accepted by `dcf.fcff` unchanged; `operating_profit` and
  `depreciation_and_amortisation` are accepted as `FreeCashFlowLines` fields; the
  `current_non_current` block and the SPEC-0007 fields are accepted by the groups that take them.
- **AC10** (R16, R18, R19) — Golden income statement cases match the reference for every layer.
  Properties: `profit_before_financing_and_taxes` = `operating_profit` + `investing_result`; net
  income = item 21; profit before tax = A − B ± C ± D; the reconciliations add up to A − B.
- **AC11** (R17) — Golden `write-downs.yaml` (D18, D19, B10 c, B10 d non-zero): all four are in
  the investing result and none lowers `operating_profit` or EBITDA. With the treatment
  `write_downs_of_receivables: operating`, B10 d lowers `operating_profit` and not EBITDA,
  `profit_before_financing_and_taxes` is unchanged, and EBITDA equals that of the default case. A
  `non_recurring` adjustment of D19 returns `FM-E001`.
- **AC12** (R21, R22) — Golden `lease-restatement.yaml`, and cases with subscribed capital unpaid
  and dividends declared; the identity of R12 holds in each.
- **AC13** (R11, R24) — Unit tests: every convention of the table appears when it applies; every
  error has a JSON pointer; no boolean in the schema; two runs give identical output; every
  `default` destination applied to a given aggregate, and the default of R17, appears in
  `meta.defaults_applied` with pointer, destination and reason (R8).
- **AC14** (R12, R16) — At least one published worked example of the balance sheet and one of the
  income statement, re-derived from printed aggregates, match at their published precision
  (Decision 16).

## Interfaces
- **Method id:** `statements.reorganise` version 1, new, preview, listed under `"0005"` in
  `specs/method-evidence.yaml`.
- **Input:** `years`: list, oldest first, of `{label?, date?, statutory? | reclassified?,
  treatment_overrides?}`, with `statutory = {layout?, balance_sheet?, income_statement?}`;
  `treatments` (once): one destination per aggregate of R9, and optionally
  `write_downs_of_receivables` (`investing` by default, or `operating`; R17); `adjustments` (once):
  `{kind, label, reason, amounts: [one per year or null], ...}`.
- **Output:** per year, the blocks of Appendix B, `net_financial_position` with composition and
  current and non-current parts, `non_operating_assets`, `debt_like_items`, `quasi_equity_items`,
  `current_non_current`, the reconciliations, the assignments and adjustments applied, and
  `not_computed`; once, `changes` per pair of consecutive entries.
- **Schema changes:** additive: one new method, `schemas/statements.reorganise/v1.json`, one entry
  in `schemas/index.json`. No existing method, golden case or number changes.
- **Building blocks reused:** `ratios::NetFinancialPosition`, `NfpItem`; `tv::Line`; the
  debt-like item type and `MeasurementBasis` of SPEC-0001 (#44, Decision 21); field names of
  `FreeCashFlowLines` and of the SPEC-0007 inputs; `Explain`; `crate::num`; the registry.
- **Errors:** `FM-E001`, `FM-E008`, `FM-E009`, `FM-E012` reused; new `FM-E026`, documented in
  `docs/en/reference/errors.md`.
- **Theme:** new commit scope `statements` in `docs/en/project/conventions.md`.

## Documentation
- Methodology page `docs/en/methods/statements-reorganise.md`: the two forms, best-effort rules,
  Appendix A to C as tables, the write-down rule, which output feeds which method (R20), an
  example from a filed abridged statement, "Verified against".
- Concept page `docs/en/concepts/financial-statements.md`: from a filing or a few totals to
  `ratios.*`, `dcf.free_cash_flows` and the `dcf.fcff` bridge.
- `docs/en/reference/errors.md` (`FM-E026`); regenerated reference and schema; Italian editions;
  glossary entries; Python helper `python/fairmark/statements.py`; release note fragments for
  #35 and #36.

## Terminology and sources
"glossary" marks terms in `i18n/glossary.yaml`.

| Term | Identifier | Italian equivalent | Source | Status |
| --- | --- | --- | --- | --- |
| Operating profit | `operating_profit` | reddito operativo della gestione caratteristica (margine operativo netto, MON) | IFRS 18 paragraph 69(a) (V32); Italian textbooks (V33) | verified |
| Profit before financing and income taxes | `profit_before_financing_and_taxes` | reddito operativo aziendale (EBIT nella prassi italiana) | IFRS 18 (V32); Italian textbooks (V33) | verified; IFRS 18 paragraph to verify |
| Investing result (accessory area) | `investing_result` | risultato della gestione accessoria (extra-caratteristica) | IFRS 18 investing category, paragraphs 53(a), 54(a) (V32); V33 | verified for equity-accounted investments; other items to verify |
| Financing result | `financing_result` | risultato della gestione finanziaria | IFRS 18 financing category (V32) | paragraphs to verify |
| EBITDA | `ebitda` | margine operativo lordo (MOL) | Banca d'Italia (V30); EBA/GL/2020/06 Annex 3 (V23) | verified |
| Value of production; value added | `value_of_production`, `value_added` | valore della produzione; valore aggiunto | Art. 2425 A; V33 | to verify |
| Invested capital; total funds invested | `invested_capital`, `total_funds_invested` | capitale investito netto; keep English | Koller et al. (V25); SPEC-0007 | glossary for the first; to verify |
| Operating working capital | `operating_working_capital` | capitale circolante netto operativo | Koller et al. (V25) | to verify |
| Net financial position | `net_financial_position` | posizione finanziaria netta (PFN); "indebitamento finanziario netto" (ESMA, Consob) | SPEC-0007; V15 | glossary; verified |
| Non-operating assets; debt-like items | `non_operating_assets`, `debt_like_items` | as in SPEC-0001 | SPEC-0001 | glossary |
| Quasi-equity | `quasi_equity_items` | keep English | Bank practice (V28) | to verify |
| Employee severance indemnity | `employee_severance_indemnity` | trattamento di fine rapporto (TFR) | IAS 19 (V22) | to verify |
| Non-recurring items | `non_recurring_result` | componenti non ricorrenti | Art. 2427 n. 13 (V9); OIC 12 (V12) | verified |
| Write-downs | `write_downs` | svalutazioni | Art. 2425 B10, D19; OIC 12 (V12) | verified |

## Sources verified (2026-11-08)
Recorded as verified by the coordinator on 2026-11-08; numbers not stated are still to verify.

- **V7, V29** Civil Code art. 2435-bis and D.Lgs. 125/2024 (thresholds of the abridged layout).
- **V8** Civil Code art. 2435-ter (micro-entities).
- **V9** (part) Art. 2427, paragraph 1, n. 13 (items of exceptional size or incidence).
- **V12** OIC 12, revised December 2016 after D.Lgs. 139/2015: former extraordinary items by
  nature in A5 and B14; gains from debt restructuring in C16 d; D18 and D19 for financial assets.
- **V15** ESMA32-382-1138, guideline 39; Consob, Richiamo di attenzione n. 5/21.
- **V23** EBA/GL/2020/06, Annex 3: metrics for credit granting and monitoring (for firms, EBITDA,
  debt to equity, debt service coverage).
- **V30** Banca d'Italia, financial stability analysis of firms: leverage = financial debt /
  (financial debt + equity); financial charges / MOL; financial debt / MOL.
- **V31** Ministry of Justice, decreto dirigenziale of 23 April 2026 (practical test of the
  negotiated settlement; replaces the decree of 21 March 2023).
- **V32** IFRS 18 *Presentation and Disclosure in Financial Statements* (effective 2027):
  required subtotals "operating profit or loss" (paragraph 69(a); operating category as the
  residual) and "profit or loss before financing and income taxes"; equity-accounted
  investments in the investing category (paragraphs 53(a), 54(a)); returns from investments and
  from cash in investing; in IFRS 18 practice, impairment of trade receivables is operating and
  impairment of investments (for example bonds) is investing. Other paragraph numbers to verify.
- **V33** Italian textbooks, Modugno, Trieste: MOL, minus depreciation, amortisation and
  provisions, gives the reddito operativo; ± non-operating components (dividends, interest income
  on loans granted, revaluations and write-downs of financial assets) gives the EBIT, which "does
  not correspond to the margine operativo netto". Title, edition and page to complete.

## Sources to verify
None of these blocks the arithmetic: Appendices A to C and decisions 1 to 18 define every
number, and the identifiers are fixed by this approved spec. The column "Blocks" names the only
tasks of `tasks.md` that wait for a source; every other item must be verified before the
methodology page and the glossary entries are final (task T39), and cannot block a merge before
that.

| # | Source | Supports | Blocks |
| --- | --- | --- | --- |
| 1 | **V1** Art. 2423 (euro units, rounding differences); **V2** Art. 2423-ter (comparatives) | R2, R3 conventions | nothing |
| 2 | **V3, V5** Art. 2424 and 2425: the aggregates of Appendix A, the signs of A2, A3, B11 | Appendix A; Decision 3 | nothing |
| 3 | **V9** Art. 2427 n. 19-bis and n. 22, and which disclosures abridged filers give; **V10** Art. 2467 | shareholder loans, leases | nothing |
| 4 | **V12** OIC 12 paragraph numbers; **V13** OIC 10 (cash); **V14** OIC 18, 19, 31 | citations on the methodology page | nothing |
| 5 | **V17** ESMA/2015/1415 (APM); **V18** Consob DEM/6064293 | citations | nothing |
| 6 | **V19** Directive 2013/34/EU Annex III (English names); **V21** IFRS 16; **V22** IAS 19 | terminology; R21 | nothing |
| 7 | **V24** Eurostat SBS definitions; **V25** Koller et al., 7th edition; **V26** CFA Institute | terminology (value added, invested capital, operating working capital) | nothing |
| 8 | **V27** Italian textbooks with worked examples, edition and page to be named by the maintainer (candidates: Ferrero, Dezzani, Pisoni, Puddu; Caramiello, Di Lazzaro, Fiori; Teodori) | AC14; golden sources (Decision 16) | **T34** (published worked examples, AC14) |
| 9 | **V28** Bank reclassification schemes (Centrale dei Bilanci, now Cerved) | term "quasi-equity" | nothing |
| 10 | **V30** (part) The Banca d'Italia publication and page | citations | nothing |
| 11 | IFRS 18 paragraph numbers still marked "to verify" in "Terminology and sources" (V32) | terminology | nothing |

Golden sources (Decision 16), all independent of the implementation: the exact decimal
reference script `scripts/reference_statements.py`, 50 significant digits, written from
Appendices A to C and the requirements without reading the Rust code (tasks T3, T20, T32); at
least one published worked example of the balance sheet and one of the income statement, named
by the maintainer (V27, task T34); and at least one anonymised practitioner case provided by the
maintainer (task T35, which waits for that case). T34 and T35 block only the completion of the
third implementation pull request and the numerical audit (T41), not the first two.

## Decisions (maintainer, 2026-11-08)
Approved by the maintainer on 2026-11-08: decisions 1 to 18, all of them. Decisions 1 to 3 and
12 to 16 were first taken under the maintainer's delegation of 2026-11-08 (decide as a bank
credit analyst and an asset management analyst would, aligned with CFA, IFRS and EBA practice)
and were confirmed by the maintainer with the approval of the spec; the others he took or
approved directly.

1. One method, `statements.reorganise`; revision 5 adds the reclassified form to the same tool.
2. Checks are exact, with no tolerance.
3. Inputs as presented in the statutory layouts (B11 positive is a decrease).
4. Payables D: the financial debt and shareholder loans parts are needed for the net financial
   position; under revision 5, when missing, the outputs that need them are not computed and
   listed.
5. Provisions B and the TFR: default `operating_provision`, never in the net financial position;
   `debt_like` by adjustment with a reason; the derivative part of B needs a treatment.
6. Shareholder loans: treatment `financial_debt` or `quasi_equity`, or a split.
7. A5 and B14 core by default; normalisations by labelled adjustments.
8. EBITDA: B10, B12, B13 below it; a grouped B10 a to c is depreciation and amortisation.
9. Leases: optional financial-method adjustment, no deferred taxes.
10. B.III: treatment needed when given and not zero.
11. Net financial position: SPEC-0007 structure; all cash inside; operating cash by adjustment.
12. The `current_non_current` block is kept for SPEC-0007.
13. `cost_of_sales` is not returned.
14. Subscribed capital unpaid deducted from equity; dividends declared by adjustment.
15. Bridge items at carrying amount with `MeasurementBasis`.
16. Golden sources: reference script, published examples, anonymised practitioner cases.
17. Write-downs and revaluations (decided by the maintainer on 2026-11-08): they are
    extra-characteristic, not operating. D18 and D19 (financial assets), B10 c (fixed assets)
    and B10 d (current receivables) go to the investing (extra-characteristic) area by default,
    below `operating_profit` and above `profit_before_financing_and_taxes`. Reasoning: a
    write-down or revaluation measures a change in the value of an asset, not the result of the
    period's core operations; keeping all of them out of `operating_profit` gives one rule,
    consistent across the statutory items. For B10 d the analyst may choose the optional treatment
    `operating`, stated once for the call, which reflects the IFRS 18 practice that impairment of
    trade receivables is operating (V32); the default is the maintainer's. EBITDA is unaffected
    either way, because every write-down stays below it. The `core_write_down` adjustment of
    revision 5 is withdrawn.
18. Resolved by revision 5 (IFRS 18, V32; Italian textbooks, V33): `operating_profit` feeds the
    free cash flows and ROI on invested capital, because non-operating assets enter the bridge
    at their value and are excluded from invested capital; `profit_before_financing_and_taxes`
    feeds the interest covers (and ROA on total assets).

## Open questions
None: every question was decided on 2026-11-08 (see "Decisions").

## Appendix A — Statutory aggregates and their destinations
Choice: **F** fixed; **D** default, reported, changed by an adjustment; **R** treatment needed
when given and not zero. "NFP": component of the net financial position.

### A.1 Balance sheet (art. 2424; abridged art. 2435-bis)

| Reference | Identifier | Destination | Choice | Adjustments | Current / non-current |
| --- | --- | --- | --- | --- | --- |
| Assets A | `subscribed_capital_unpaid` (or "of which" of C.II) | deducted from equity | F | none | deducted |
| B.I | `intangible_assets` (gross and accumulated optional) | operating fixed assets | D | `non_operating_asset` | non-current |
| B.II | `tangible_assets` (gross and accumulated optional) | operating fixed assets | D | `non_operating_asset`, `lease` | non-current |
| B.III | `financial_fixed_assets` | operating, non-operating, or NFP financial asset | R | none | non-current |
| C.I | `inventories` | operating working capital | F | none | current |
| C.II | `receivables` (beyond one year; trade optional) | operating working capital | D | `financial_receivable`, `non_operating_asset` | statutory split |
| C.III | `current_financial_assets` | NFP other financial asset | D | `non_operating_asset` | current |
| C.IV | `cash` | NFP cash | D | `operating_cash` | current |
| Assets D | `prepayments_and_accrued_income` (or "of which" of C.II) | operating working capital | D | `lease` (prepayment) | current, or part given |
| Liabilities A | `equity` (total or Roman numerals; A.IX) | equity | F | `dividends_declared`, `lease` | equity |
| B, other | `provisions` | operating provision | D | `debt_like_provision` | non-current, or part given |
| B, derivatives | `provisions.of_which_derivative_liabilities` | NFP other financial liability, or operating provision | R | none | non-current, or part given |
| C | `employee_severance_indemnity` | operating provision | D | `debt_like_provision` | non-current, or part given |
| D, financial debt | `payables.financial_debt` (beyond one year) | NFP gross debt | F | none | its split |
| D, shareholder loans | `payables.shareholder_loans` (beyond one year) | NFP gross debt, or quasi-equity | R | none | its split |
| D, remainder | `payables` less the parts above (trade optional) | operating working capital | D | `tax_instalments`, `non_current_trade_payables` | statutory split |
| E | `accruals_and_deferred_income` (or "of which" of D) | operating working capital | D | `accrued_interest` | current, or part given |

### A.2 Income statement (art. 2425; groupings of art. 2435-bis)

| Reference | Identifier | Layer | Choice | Adjustments |
| --- | --- | --- | --- | --- |
| A1 | `revenue` | value of production | F | `non_recurring` |
| A2, A3 | `change_in_product_inventories` | value of production | F | none |
| A4 | `own_work_capitalised` | value of production | F | none |
| A5 | `other_revenue_and_income` | value of production | D | `investing`, `non_recurring` |
| B6, B11 | `purchases`, `change_in_raw_material_inventories` | external costs (`materials_consumed`) | F | `non_recurring` (B6) |
| B7, B8 | `services`, `use_of_third_party_assets` | external costs | F | `non_recurring`; `lease` (B8) |
| B9 | `personnel_costs` | below value added | F | `non_recurring` |
| B10 a, b (and c when grouped) | `depreciation_and_amortisation` | below EBITDA | F | `non_recurring` |
| B10 c | `write_downs_of_fixed_assets` | investing result | F | `non_recurring` |
| B10 d | `write_downs_of_receivables` | investing result; `operating_profit` below EBITDA with the optional treatment `operating` | D | `non_recurring` |
| B12, B13 | `provisions_for_risks`, `other_provisions` | below EBITDA | F | `non_recurring` |
| B14 | `other_operating_expenses` | external costs | D | `investing`, `non_recurring` |
| C15, C16 | `income_from_equity_investments`, `other_financial_income` | investing result | F | `non_recurring` |
| C17, C17-bis | `interest_and_other_financial_charges`, `exchange_gains_and_losses` | financing result | F | `non_recurring`; `lease` (C17) |
| D18, D19 | `revaluations_of_financial_assets`, `write_downs_of_financial_assets` | investing result | F | none |
| 20, 21 | `income_taxes`, `profit_or_loss_for_the_year` | taxes as given; check | F | none |

## Appendix B — Reorganised structures (outputs, and inputs of the reclassified form)

### B.1 Balance sheet
- `operating_fixed_assets` = B.I + B.II + B.III (operating) − non-operating parts + lease assets.
- `operating_working_capital` = C.I + C.II (operating) + assets D + operating cash − payables D
  (operating) − E (operating); `trade_working_capital` = trade receivables + inventories − trade
  payables, when given.
- `operating_provisions` = B and C treated as operating provisions.
- `invested_capital` = `operating_fixed_assets` + `operating_working_capital` −
  `operating_provisions`.
- `non_operating_assets` = B.III (non-operating) + parts moved by `non_operating_asset`.
- `total_funds_invested` = `invested_capital` + Σ `non_operating_assets`.
- `equity` = liabilities A − subscribed capital unpaid − dividends declared ± lease effect
  (signed).
- `net_financial_position` = gross debt − cash + Σ other items (SPEC-0007, signed);
  `excess_cash` = C.IV − operating cash.
- Sources = `equity` + Σ `quasi_equity_items` + `net_financial_position` + Σ `debt_like_items`;
  `total_funds_invested` = sources.

### B.2 Income statement
- `value_of_production` = A1 + A2 + A3 + A4 + A5 (core).
- `value_added` = `value_of_production` − (B6 + B11 + B7 + B8 + B14 (core)).
- `ebitda` = `value_added` − B9 (signed).
- `operating_profit` = `ebitda` − `depreciation_and_amortisation` − B12 − B13 (− B10 d with the
  treatment `operating`) (signed).
- `investing_result` = A5 (investing) − B14 (investing) + C15 + C16 + D18 − D19 − B10 c − B10 d
  (unless `operating`) (signed).
- `profit_before_financing_and_taxes` = `operating_profit` + `investing_result`.
- `non_recurring_result` = Σ non-recurring income − Σ non-recurring expense.
- `financing_result` = C17-bis − C17 (signed).
- `profit_before_tax` = `profit_before_financing_and_taxes` + `non_recurring_result` +
  `financing_result`; equals A − B ± C ± D.
- `net_income` = `profit_before_tax` − `income_taxes`; equals item 21.
- Lease adjustment: B8 less the lease payments; depreciation added; interest added to C17.

## Appendix C — Adjustment kinds
Each adjustment is stated once: `kind`, `label`, `reason`, the fields listed and `amounts`, one
entry per year (null where it does not apply; for `lease`, its amount fields).

| Kind | Source | Target | Fields |
| --- | --- | --- | --- |
| `non_operating_asset` | B.I, B.II, C.II or C.III | non-operating assets | aggregate |
| `financial_receivable` | C.II | NFP other financial asset | maturity |
| `operating_cash` | NFP cash | operating working capital | none |
| `accrued_interest` | E | NFP other financial liability | none |
| `tax_instalments` | payables D (remainder) | debt-like items | maturity |
| `non_current_trade_payables` | payables D (remainder) | NFP other financial liability | none |
| `debt_like_provision` | B or C (operating provision) | debt-like items | aggregate |
| `dividends_declared` | equity | debt-like items | none |
| `lease` | double entry (R21) | operating fixed assets, NFP, equity; B8, B10, C17 | asset, liability with current part, prepayment, payments, depreciation, interest |
| `investing` | A5, B14 | investing result | aggregate; optional related non-operating asset |
| `non_recurring` | any income statement aggregate except D18, D19 | non-recurring result | aggregate |
