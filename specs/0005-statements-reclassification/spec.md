# SPEC-0005 — Financial statements: reorganising the statutory aggregates into an operating and financing view

| | |
| --- | --- |
| Status | Draft — revision 3 (2026-11-08): core and non-operating areas of the income statement, write-down rule, authoritative sources |
| Milestone | M2 (release 0.1.0); epic #34; issues #35 and #36 |
| Owner | @salvo8519 |
| Formula basis | The corporate-finance reorganisation of the financial statements into operating and financing items (Koller, Goedhart, Wessels, *Valuation*, "Reorganizing the financial statements"; Damodaran; CFA Institute curriculum), applied to the aggregates of the Italian statutory layouts (art. 2424, 2425, abridged art. 2435-bis), with the net financial position in the SPEC-0007 structure and the ESMA32-382-1138 table as reference, and the income statement presented in the value added layout with the core (characteristic) management separated from the non-operating (accessory), non-recurring and financial areas, as in Italian credit analysis (OIC 12; art. 2427 n. 13; Banca d'Italia; EBA/GL/2020/06); verified against an exact decimal reference script and against published worked examples and practitioner cases still to be named (Open question 16) |
| Related | SPEC-0001 (owns the free cash flow build-up and NOPAT taxes; its bridge types receive the non-operating assets and debt-like items), SPEC-0003 (owns tax rates; no IRAP logic), SPEC-0007 (owns `NetFinancialPosition`, every ratio and margin); ADR 0001, ADR 0009, ADR 0012 |

## Revision 3 (2026-11-08)
After the maintainer's answers to Open questions 1 to 16 (commits a7b30ca, d65c45a):
- The income statement is presented in named layers that separate the core (characteristic)
  management from the non-operating (accessory) management, the non-recurring items and the
  financial management (R17, R17a, Appendix B.2). `core_ebit` (reddito operativo della gestione
  caratteristica) and `ebit` (reddito operativo aziendale) are distinct outputs.
- Write-downs follow an explicit rule (R18a): D19 always financial area; B10 c non-recurring by
  default; B10 d core, below EBITDA. New Open question 17 asks the maintainer to confirm it.
- Which EBIT feeds the free cash flows and ROI is Open question 18.
- The balance sheet shows non-operating (accessory) assets as a separate block, consistent with
  the bridge (R12).
- Decision 5 applied to R9, the conventions table and Appendix A: provisions B and the employee
  severance indemnity C take the documented default `operating_provision`; only the derivative
  part of B stays required.
- Items 15, 16, 17 and 17-bis are now required separately (R6), because C15 and part of C16
  belong to the non-operating area.
- Sources: authoritative sources verified on 2026-11-08 added (EBA, Banca d'Italia, OIC 12,
  art. 2427 n. 13, ESMA and Consob, D.Lgs. 125/2024, decree of the Ministry of Justice of
  23 April 2026). New future extensions: a debt sustainability method and the Banca d'Italia
  indicators missing from `ratios.*`.

## Revision 2 (2026-11-08)
The maintainer narrowed the scope to capital-budgeting use. Changes from revision 1:
- Input: statutory **totals** (letters and Roman numerals of art. 2424, the aggregates of
  art. 2425 as the abridged layout shows them), not the line items. Line items become a future
  extension.
- Output: one operating and financing view (invested capital, net financial position, equity,
  non-operating assets, debt-like items) and a value added income statement, instead of two
  balance sheet criteria and two income statement layouts.
- The reclassification choices of practice are **labelled adjustments** with an amount and a
  reason, or required treatments of an aggregate where its content is ambiguous.
- Dropped from version 1 and recorded as future extensions: the financial (liquidity) criterion
  as a presentation, the cost of sales layout, `statements.statutory_accounts` as a separate
  method. A minimal current and non-current block is kept because existing SPEC-0007 groups
  need it (see "Analysis").
- Requirements, acceptance criteria, open questions and Appendix A are rewritten; the ids of
  revision 1 do not carry over.

## Analysis

### What is on main today
- No `statements` theme.
- `ratios.*` (SPEC-0007) take, per balance sheet date: `equity`, `total_assets`,
  `fixed_assets`, `non_current_liabilities` and `net_financial_position`
  (`ratios.capital_structure`); `current_assets`, `inventories`, `cash`, `current_liabilities`
  (`ratios.liquidity`); `trade_receivables`, `inventories`, `trade_payables` with `revenue`,
  `cost_of_sales`, `purchases` (`ratios.working_capital`); `equity`, `total_assets`,
  `invested_capital` with `revenue`, `ebitda`, `ebit`, `net_income` (`ratios.profitability`);
  `net_financial_position` with `ebitda`, `ebit`, `interest` (`ratios.debt_service`).
- `dcf.free_cash_flows` (SPEC-0001 R2 to R5) builds FCFF from `ebit`, taxes on EBIT (a rate or
  amounts), `depreciation_and_amortisation`, `capital_expenditure`,
  `change_in_net_working_capital`; `FreeCashFlowLines` (issue #180) carries the same lines into
  a `given` stage of `dcf.fcff`. NOPAT = EBIT − taxes on EBIT is computed there.
- `dcf.fcff` version 3 takes `net_financial_position` (SPEC-0007 type) and
  `non_operating_assets` as `tv::Line` (`{label, value}`). Debt-like items, non-controlling
  interests and the measurement basis of bridge items come with issue #44 and SPEC-0001
  Decision 21 (`MeasurementBasis`). **Dependency:** Decision 21 and `MeasurementBasis` are not on
  the branch of this worktree; this spec reuses them as approved and the plan takes them from
  main.
- `coc.*` (SPEC-0003): the tax rate is an input; "the engine has no IRAP logic".

### Which dropped presentations existing ratios still need
- **Financial (liquidity) criterion.** `ratios.liquidity` and `ratios.capital_structure` need
  current and non-current amounts. They are derivable from the statutory aggregates, because
  art. 2424 and 2435-bis show the amounts of C.II and D due beyond the next financial year. Kept
  as a minimal block `current_non_current` with the SPEC-0007 field names (R16); the full
  presentation (liquidity layers, margins) is dropped, and the margins stay in SPEC-0007.
- **Cost of sales layout.** `ratios.working_capital` needs `cost_of_sales` for days inventory
  outstanding. A statutory income statement by nature does not contain it, and a cost of sales
  layout needs an allocation of costs by function from management accounts, so producing it
  would only move the judgement into an allocation input. Dropped: the method returns
  `purchases` (B6) and `materials_consumed` (B6 + B11) under their own names and never under
  the name `cost_of_sales` (Open question 13).

### Banca d'Italia and EBA indicators against `ratios.*` on main
Checked on the code of `crates/fairmark-core/src/ratios/` in this worktree.

| Indicator | Source | On main | Note |
| --- | --- | --- | --- |
| Leverage = financial debt / (financial debt + equity) | Banca d'Italia (V30) | No | `ratios.capital_structure` has `debt_to_equity` (gross debt / equity), `nfp_to_equity` and `equity_ratio` (equity / total assets); D / (D + E) is not returned |
| Financial charges / MOL (EBITDA) | Banca d'Italia (V30) | Inverse only | `ratios.debt_service` has `interest_cover_ebitda` = EBITDA / interest; the Banca d'Italia form and its not-defined rule are not returned |
| Financial debt / MOL (EBITDA) | Banca d'Italia (V30) | Net form only | `nfp_to_ebitda` uses the net financial position; gross financial debt / EBITDA is not returned |
| EBITDA | EBA/GL/2020/06 Annex 3 (V23) | Input | Taken as input with `ebitda_definition`; this spec produces it (R18) |
| Debt to equity | EBA/GL/2020/06 Annex 3 | Yes | `debt_to_equity` |
| Debt service coverage | EBA/GL/2020/06 Annex 3 | Yes | `dscr`, and `llcr` |
| Interest coverage, net debt / EBITDA | Credit practice | Yes | `interest_cover_ebit`, `interest_cover_ebitda`, `nfp_to_ebitda` |
| Debt to be restructured / annual free flows serving the debt (test pratico) | Ministry of Justice decree of 23 April 2026 (V31) | No | Future extension 8 |

The three missing Banca d'Italia forms are future extension 9 (a SPEC-0007 change). This
method returns every input they need: `ebitda`, `equity`, the gross debt of the net financial
position, and `interest_and_other_financial_charges`.

### Revision 1 and the previous draft
| Earlier content | Owner now |
| --- | --- |
| FCFF build-up with IRES and IRAP (first draft R2) | SPEC-0001 R2, R4, R5 (`dcf.free_cash_flows`, #38, done); tax rates: SPEC-0003 |
| FCFE (first draft R3) | SPEC-0001 R3 (build-up), R1 and R24 (`dcf.fcfe`, #165) |
| Statutory line items, financial criterion, cost of sales layout, statutory check method (revision 1) | Future extensions of this spec |

## Problem
Capital budgeting and valuation need the statements split into what the business uses
(invested capital and the operating income it earns) and how it is financed (equity, net
financial position, debt-like claims). Italian companies file statutory statements whose
aggregates mix the two: payables D hold bank loans next to trade payables; provisions B and the
employee severance indemnity C may be operating or debt-like; financial fixed assets B.III may
be operating, non-operating or financial. Every split changes the net financial position,
invested capital, EBITDA and every ratio and value built on them. In spreadsheets the splits are
invisible, and an AI agent that reads a filing splits them inconsistently. The engine must
accept the totals as filed, refuse statements that do not reconcile, apply documented defaults
only where practice is settled, require the analyst's statement where it is not, record every
adjustment with its amount and reason, and return outputs that `ratios.*` and `dcf.*` accept as
they are.

## Scope
**In scope (version 1):**
- Input: the aggregates of the statutory balance sheet (art. 2424 letters and Roman numerals,
  with the amounts beyond the next financial year of C.II and D) and income statement (art. 2425
  aggregates, as in the abridged layout of art. 2435-bis), in the ordinary, abridged and
  micro-entity layouts (art. 2435-ter); one or more financial years.
- Reconciliation checks of the totals.
- Reorganised balance sheet: operating fixed assets, operating working capital, operating
  provisions, invested capital, non-operating assets, equity, quasi-equity, net financial
  position, debt-like items.
- Reorganised income statement in the value added layout, by layers: value of production, value
  added, EBITDA, EBIT of core management, non-operating (accessory) result, EBIT of the company,
  non-recurring result, financial management, value adjustments of financial assets, profit
  before tax, taxes as given, net income.
- Labelled adjustments of corporate-finance practice: employee severance indemnity,
  provisions, shareholder loans, leases (financial method, optional), non-recurring items,
  prepayments and accruals, operating and excess cash, and the others of Appendix C.
- A minimal current and non-current block for SPEC-0007 (R16).

**Out of scope, owned elsewhere:** NOPAT taxes, FCFF and FCFE (SPEC-0001); tax rates, IRES and
IRAP (SPEC-0003); ratios and margins (SPEC-0007).

**Future extensions (not version 1; each additive unless stated):**
1. Statutory line items (arabic numerals of art. 2424 and 2425) as an optional richer layer,
   mapped to the same aggregates and destinations.
2. The financial (liquidity) criterion as a full presentation.
3. The cost of sales layout, with allocations by function.
4. The full ESMA32-382-1138 net financial indebtedness table (rows A to M).
5. Changes between consecutive years (operating working capital, fixed assets) for the
   SPEC-0001 build-up, as a SPEC-0001 task.
6. A standalone statutory check method, if agents need it before classification.
7. IFRS layouts (IAS 1, IFRS 18) as statutory input; consolidated accounts.
8. A debt sustainability method computing the ratio of the practical test (test pratico) of
   the negotiated settlement of the business crisis: debt to be restructured over the annual free
   flows serving the debt, as defined by the decree of the Ministry of Justice of 23 April 2026
   (V31). It needs its own spec, with the definitions of the decree as inputs; it belongs with
   the deferred `ratios.crisis_indicators` of SPEC-0007 (decision of 2026-11-07, CNDCEC
   indicators).
9. Banca d'Italia indicators in SPEC-0007: leverage as financial debt / (financial debt +
   equity), financial charges / EBITDA, gross financial debt / EBITDA (see "Analysis"; the
   other EBA and credit indicators already exist).

**Not planned:** XBRL or PDF parsing (adapters or agents); layouts of banks and insurers;
credit scoring.

## User stories
- As an analyst, I want to type the totals of a filed abridged statement and state only the
  splits the filing does not show, so that I never invent detail.
- As a valuation analyst, I want the net financial position, non-operating assets and
  debt-like items in the shapes `dcf.fcff` accepts, so that the bridge uses the numbers of the
  reorganised balance sheet.
- As an AI agent extracting a filing, I want reconciliation errors before classification
  errors, with JSON pointers, so that I fix the extraction first.
- As a reviewer, I want every adjustment with its amount, reason, source and destination in the
  explain, so that I can challenge each one.

## Requirements (EARS)

### Method and input
- **R1** — THE ENGINE SHALL provide the method `statements.reorganise` version 1, status
  preview (ADR 0012), which reorganises the statutory balance sheet, the income statement or
  both of one or more financial years into the operating and financing view (Open question 1).
- **R2** — THE ENGINE SHALL take a list of one or more years, each with a balance sheet date
  (ISO 8601, `FM-E009` when invalid; distinct dates, `FM-E001` otherwise), a declared layout
  (`ordinary`, `abridged`, `micro_entity`), a balance sheet, an income statement or both
  (`FM-E001` when neither), the treatments of R8 and the adjustments of R10. It SHALL reorganise
  each year independently and return the years in input order. The layout SHALL be echoed in
  `explain.conventions`; no number SHALL depend on it and eligibility thresholds SHALL NOT be
  checked. All amounts SHALL be in the currency units of the statements, one unit per call,
  never scaled.
- **R3** — THE ENGINE SHALL accept the balance sheet aggregates of Appendix A.1. For receivables
  C.II and payables D it SHALL require the amount due beyond the next financial year (`FM-E001`
  when missing, negative or larger than the total). For payables D it SHALL require the parts
  that are financial debt and shareholder loans, each with its amount beyond the next financial
  year, zero allowed, and accept optional trade payables; the remainder of D is operating
  (Open question 4). Aggregates that the abridged layout may present inside another (assets A
  and D inside C.II, liabilities E inside D) SHALL be given as their own node or as an "of
  which" of the node that contains them.
- **R4** — THE ENGINE SHALL accept the income statement aggregates of Appendix A.2, including
  the groupings permitted by art. 2435-bis (A2 with A3; B10 a, b and c; others as listed), under
  any declared layout.
- **R5** — Amounts SHALL be entered as presented in the statutory layouts: zero or more for
  assets, liabilities, provisions, revenue and cost aggregates (`FM-E001` naming the field
  otherwise); signed for A2 and A3 (positive for an increase in inventories), B11 (positive for a
  decrease, which is a cost), 17-bis (positive for a net gain; items 15, 16, 17, 18 and 19 are
  zero or more), 20 (positive for a tax charge), 21, and the equity aggregates that can be
  negative (A.VI, A.VII, A.VIII, A.IX; A.X zero or negative). The schema SHALL state each sign
  (Open question 3).
- **R6** — Every aggregate that has components in Appendix A (equity A.I to A.X; B.I and B.II
  gross and accumulated amortisation and depreciation; A1 to A5; B6 to B14; items 15 to 17-bis)
  SHALL accept a total, its components or both; WHEN both are given and they differ, THE ENGINE
  SHALL return `FM-E026` naming the node and stating the total, the sum and the difference.
  Items 15, 16, 17 and 17-bis SHALL be required separately (zero allowed; the abridged layout
  shows them), because C15 and part of C16 belong to the non-operating area (R17a); a total of C
  alone SHALL be refused with `FM-E001` naming the missing items. Item B10 c SHALL be accepted
  separately or as an "of which" of B10 a to c when they are grouped.
- **R7** — THE ENGINE SHALL check, before any classification check, that total assets equal
  total liabilities and equity exactly, with no tolerance, and that the profit or loss for the
  year of the balance sheet (A.IX) equals item 21 when both statements are given; IF a check
  fails, THEN it SHALL return `FM-E026` with both amounts, the difference, the JSON pointers and
  the fix (Open question 2).

### Destinations, treatments and adjustments
- **R8** — THE ENGINE SHALL assign every aggregate to the destination of Appendix A. An
  aggregate marked `fixed` or `default` SHALL take the documented destination, reported in
  `explain.conventions`; an aggregate marked `required` SHALL take the treatment stated by the
  analyst whenever its amount is not zero: one destination, or a split into labelled parts with
  a destination and an amount each, summing exactly to the aggregate (`FM-E026` otherwise). IF a
  required treatment is missing or names a destination not permitted for that aggregate, THEN
  THE ENGINE SHALL return `FM-E001` naming the aggregate and listing the permitted destinations.
- **R9** — The aggregates with a required treatment SHALL be: financial fixed assets (B.III), the
  shareholder loans part of D and the derivative liabilities part of provisions B (an optional
  "of which" of B) (Open questions 5, 6, 10). Provisions B (other than derivatives) and the
  employee severance indemnity C SHALL take the documented default `operating_provision`,
  reported, never inside the net financial position; an adjustment SHALL move all or part to
  debt-like items with a reason (Decision of Open question 5). No other aggregate SHALL require a
  treatment in version 1.
- **R10** — THE ENGINE SHALL accept a list of labelled adjustments, each with a kind from
  Appendix C, an amount of zero or more, a reason (not blank; `FM-E001` otherwise) and the fields
  of its kind. An adjustment SHALL move its amount from the source destination to the target
  destination of its kind; IF its source is not one permitted for its kind in Appendix C (for
  example a non-recurring adjustment of D19), THEN THE ENGINE SHALL return `FM-E001`; IF the
  amount exceeds what remains in the source after earlier adjustments, THEN it SHALL return
  `FM-E026` naming the adjustment. None given SHALL mean none, reported.
- **R11** — THE ENGINE SHALL report in `explain` one step per aggregate (reference, JSON pointer,
  amount, destination, origin `mapping` or `analyst`) and one step per adjustment (kind, label,
  amount, reason, source, target), so that every reorganised amount traces back to the filed
  aggregates.

### Reorganised balance sheet
- **R12** — WHEN a balance sheet is given, THE ENGINE SHALL return the structure of Appendix
  B.1 in two separate blocks of uses and one block of sources: the operating (core) block,
  operating fixed assets, operating working capital (with trade working capital when trade
  receivables and trade payables are given), operating provisions and `invested_capital`; the
  non-operating (accessory) block, `non_operating_assets` listed one by one with their source
  (for example property not used in the business, equity investments that are not strategic,
  surplus securities), with their total; `total_funds_invested`; and the sources, `equity`,
  quasi-equity items, `net_financial_position`, `debt_like_items` and total sources. Each total
  SHALL be in `explain` with its components substituted. The non-operating assets of this block
  SHALL be the same items, with the same labels, as those returned for the bridge (R15), and
  SHALL NOT be part of `invested_capital`.
- **R13** — THE ENGINE SHALL hold total funds invested = equity + Σ quasi-equity items + net
  financial position + Σ debt-like items exactly; since the inputs reconcile (R7) and every
  adjustment moves amounts between destinations or applies a documented double entry (R23,
  R24), a failure SHALL be `FM-E012`.
- **R14** — THE ENGINE SHALL return the net financial position in the `NetFinancialPosition`
  type of SPEC-0007 R6, positive for net debt: `gross_debt` = financial debt of D + shareholder
  loans treated as financial debt; `cash` = C.IV less the operating cash adjustment;
  `other_items` = each other financial asset (negative) or other financial liability
  (positive) with its label and source. It SHALL return the composition with sources and the
  current and non-current parts, and compute the total with the SPEC-0007 function. The
  ESMA32-382-1138 table is the reference for the composition and for the methodology page
  (Open question 11).
- **R15** — THE ENGINE SHALL return non-operating assets and debt-like items in the types of the
  `dcf.fcff` bridge (`tv::Line` for non-operating assets; the debt-like item type of issue #44),
  each with the measurement basis of SPEC-0001 Decision 21 set to the carrying amount of the
  statements, so that they pass to `dcf.fcff` unchanged and the analyst restates them to a
  market value there when needed (Open question 15). No new type SHALL be introduced for them.
- **R16** — WHEN a balance sheet is given, THE ENGINE SHALL return the block
  `current_non_current` with the field names and meanings of the SPEC-0007 inputs:
  `fixed_assets`, `current_assets`, `inventories`, `cash`, `current_liabilities`,
  `non_current_liabilities`, `equity`, `total_assets`, and `trade_receivables` and
  `trade_payables` when given. Current and non-current parts SHALL follow the statutory split of
  C.II and D read as twelve months; provisions B and the employee severance indemnity C SHALL be
  non-current and prepayments and accruals current unless the analyst gives their non-current
  part; inventories SHALL be current. Every such default SHALL be reported. THE ENGINE SHALL NOT
  compute ratios or margins (Open question 12).

### Reorganised income statement
- **R17** — WHEN an income statement is given, THE ENGINE SHALL return the value added layout of
  Appendix B.2 as named layers, in this order, each in `explain` with its components
  substituted:
  1. `value_of_production` (core): A1 + A2 + A3 + A4 + the core part of A5;
  2. `value_added`: value of production − external costs (B6 + B11, B7, B8, the core part of
     B14);
  3. `ebitda` (margine operativo lordo, MOL): value added − personnel costs (B9);
  4. `core_ebit` (reddito operativo della gestione caratteristica): EBITDA − depreciation and
     amortisation (B10 a, b) − write-downs of current receivables (B10 d) − provisions (B12,
     B13);
  5. `non_operating_result` (gestione accessoria, extra-characteristic): the non-operating parts
     of A5 and B14 (for example rents of property not used in the business, royalties, gains and
     losses on disposals of non-operating assets), income from equity investments (C15) and the
     income from non-operating financial assets moved out of C16;
  6. `ebit` (reddito operativo aziendale): `core_ebit` + `non_operating_result`;
  7. `non_recurring_result`: the labelled non-recurring adjustments (OIC 12; art. 2427 n. 13),
     including B10 c by default (R18a);
  8. `financial_result` (gestione finanziaria): C16 (financial part) − C17 + C17-bis, with item
     17 also returned as `interest_and_other_financial_charges`;
  9. `value_adjustments_of_financial_assets`: D18 − D19, always outside operating income;
  10. `profit_before_tax` = `ebit` + `non_recurring_result` + `financial_result` +
      `value_adjustments_of_financial_assets`;
  11. `income_taxes` (item 20, as given) and `net_income` = profit before tax − income taxes.
- **R17a** — Item C15 SHALL be non-operating by default and item C16 financial by default, both
  reported; an adjustment SHALL move the part of C16 earned on non-operating financial assets to
  the non-operating result, and MAY name the non-operating asset it relates to, so that the
  income of each accessory asset of R12 can be traced (Open question 18).
- **R18** — THE ENGINE SHALL compute EBITDA as value added − personnel costs, with B10 d and B12,
  B13 below EBITDA and above `core_ebit`; WHEN B10 a, b and c are given grouped with no "of
  which" for B10 c, the group SHALL be `depreciation_and_amortisation` and an impairment SHALL
  be entered as a non-recurring adjustment. The definition SHALL be reported in words usable as
  `ebitda_definition` in SPEC-0007 (Open question 8).
- **R18a** — Write-downs (svalutazioni) SHALL follow this rule, reported in
  `explain.conventions` (Open question 17): (a) write-downs of financial assets (D19) SHALL be
  in the value adjustments of financial assets, always in the financial area and never in
  operating income, with no adjustment able to move them; (b) write-downs of fixed assets
  (B10 c), when given, SHALL be classified by nature, by default in the non-recurring result
  below `core_ebit` and `ebit`, and the analyst MAY move them into the core management, below
  EBITDA, with an adjustment of kind `core_write_down` and a reason; (c) write-downs of current
  receivables and cash (B10 d) SHALL be in the core management, below EBITDA, because customer
  credit risk is part of core trade.
- **R19** — Other revenue and income (A5) and other operating expenses (B14) SHALL be core by
  default, reported; THE ENGINE SHALL accept adjustments that move labelled parts of A5, B14 or
  of any other income statement aggregate to the non-operating result (A5, B14, C16 only) or to
  the non-recurring result (any aggregate except D), so that EBITDA and `core_ebit` exclude them,
  and the non-recurring result SHALL be below `ebit`, so that `ebit` excludes it as well (Open
  question 7).
- **R20** — THE ENGINE SHALL present item 20 as income taxes as given and SHALL NOT compute IRES,
  IRAP, deferred taxes, taxes on EBIT or NOPAT. Profit before tax SHALL equal the statutory
  A − B ± C ± D and net income SHALL equal item 21, exactly.
- **R21** — THE ENGINE SHALL return a reconciliation of `core_ebit` and of `ebit` with the
  statutory difference A − B, listing every adjustment that moved an amount out of A or B, the
  amounts of C15 and C16 moved into the non-operating result, and the lease restatement effect
  when R23 applies.
- **R22** — Output fields that are inputs of other methods SHALL carry the same name and
  meaning: `depreciation_and_amortisation`, and an EBIT, for the SPEC-0001 build-up lines (NOPAT
  is computed there with the analyst's tax choice); `revenue`, `ebitda`, an EBIT, `net_income`,
  `purchases` (B6) and `invested_capital` for SPEC-0007. Both `core_ebit` and `ebit` SHALL be
  returned, and the methodology page SHALL state which of the two each consumer should receive
  as its `ebit` field (Open question 18; recommendation: `core_ebit` for the free cash flows
  and for ROI on invested capital, `ebit` for interest cover and ROA). `cost_of_sales` SHALL NOT
  be returned (Open question 13).

### Adjustments that change both statements or equity
- **R23** — WHEN a lease adjustment is given (per lease or in total: carrying amount of the
  asset, lease liability with its current part, prepayment to remove, and for the income
  statement the lease payments of the year, depreciation and interest), THE ENGINE SHALL add the
  asset to operating fixed assets and the liability to the net financial position as an other
  financial liability, remove the prepayment, report the difference as an equity adjustment with
  its formula, move the lease payments out of B8 and add depreciation and interest. It SHALL NOT
  compute deferred taxes. None given SHALL mean leases as reported, stated as a convention (Open
  question 9).
- **R24** — THE ENGINE SHALL deduct subscribed capital unpaid (assets A) from equity, reported;
  WHEN dividends declared are given as an adjustment, it SHALL move them from equity to
  debt-like items (Open question 14).

### Explain, errors and stability
- **R25** — THE ENGINE SHALL report in `explain.conventions`: the layout; the sign conventions;
  the exact balance check; every default destination used; every treatment and adjustment; the
  twelve-month reading of the statutory maturity split; the EBITDA definition; the layers of the
  income statement and the area of each aggregate; the write-down rule of R18a; the default areas
  of C15 and C16; the treatment of financial income, taxes and leases; the measurement basis of bridge items; and "none given"
  for each adjustment kind not used.
- **R26** — Every error SHALL name the field by its JSON pointer and state the fix. Errors SHALL
  reuse `FM-E001`, `FM-E008`, `FM-E009` and `FM-E012`; one new code, `FM-E026` "statements do
  not reconcile", SHALL cover every difference between a total and its components, between
  assets and liabilities and equity, between the two profits, between a split and its
  aggregate, and an adjustment larger than its source.
- **R27** — Multi-way choices SHALL be enumerations or labelled objects, never booleans; every
  required treatment SHALL have no default; the arithmetic SHALL be additions and subtractions
  of decimals only, with no rounding.

## Conventions and parameters

| Convention | Options | Default | Source |
| --- | --- | --- | --- |
| Layout | `ordinary`, `abridged`, `micro_entity` | required; echoed; no number depends on it | Art. 2424, 2425, 2435-bis, 2435-ter |
| Sign of inputs | as presented in the statutory layouts (R5) | fixed | Art. 2425; OIC 12 (V5, V12) |
| Balance check | exact equality | fixed | Art. 2423, euro units (V1); Open question 2 |
| Maturity | statutory split of C.II and D, read as twelve months | fixed, reported | Art. 2424, 2435-bis (V3, V7) |
| Payables D by nature | financial debt and shareholder loans stated; remainder operating | required parts | Open question 4 |
| Provisions B (other than derivatives) | `operating_provision`; `debt_like` by adjustment | default `operating_provision`, reported | Italian functional layout (V27, V28); ESMA32-382-1138 guideline 39 (V15); Decision of Open question 5 |
| Derivative liabilities in B | NFP other financial liability, or operating | required when not zero | Decision of Open question 5 |
| Employee severance indemnity C | `operating_provision`; `debt_like` by adjustment | default `operating_provision`, reported | As provisions B; Decision of Open question 5 |
| Financial fixed assets B.III | `operating`, `non_operating`, `financial_asset`, or a split | required when not zero | Open question 10 |
| Shareholder loans (part of D) | `financial_debt`, `quasi_equity`, or a split | required when not zero | Art. 2427 n. 19-bis, art. 2467 (V9, V10); Open question 6 |
| Receivables C.II | operating working capital; financial parts by adjustment | default, reported | Practice |
| Current financial assets C.III | net financial position (other financial asset) | default, reported | ESMA32-382-1138 (V15) |
| Cash C.IV | net financial position; operating cash by adjustment | default, reported | ESMA32-382-1138 (V15); Koller (V25); Open question 11 |
| Prepayments and accruals (assets D, liabilities E) | operating working capital, current; accrued interest by adjustment | default, reported | Practice |
| A5 and B14 | core; non-operating and non-recurring parts by adjustment | default, reported | OIC 12 (V12); art. 2427 n. 13 (V9); Open question 7 |
| C15 | non-operating result | default, reported | Italian credit analysis (V28, V30); Open question 18 |
| C16 | financial result; non-operating part by adjustment | default, reported | Italian credit analysis; Open question 18 |
| EBITDA | value added − personnel costs; B10 d, B12, B13 below EBITDA | fixed, reported | Italian practice (V27, V28); Banca d'Italia MOL (V30); Open question 8 |
| Write-downs | D19 financial area, fixed; B10 c non-recurring, core by adjustment; B10 d core below EBITDA | as stated, reported | OIC 12 (V12); Open question 17 |
| `core_ebit` and `ebit` | core management; core + non-operating | both returned | Italian credit analysis; Open question 18 |
| Leases | as reported; financial method by adjustment | as reported, stated | Art. 2427 n. 22 (V9); IFRS 16 (V21); Open question 9 |
| Taxes | item 20 as given | fixed; no IRES, IRAP or NOPAT | SPEC-0003 decision of 2026-11-07; SPEC-0001 |
| Bridge items measurement | carrying amount | fixed, reported | SPEC-0001 Decision 21; Open question 15 |

## Acceptance criteria
Every golden case compares exactly (tolerance 0).

- **AC1** (R2, R3, R4, R5, R6) — Golden `statements.reorganise/ordinary-aggregates.yaml` and
  `abridged-aggregates.yaml` (same company, the abridged one with grouped nodes and A, D, E
  given as "of which") give identical outputs.
- **AC2** (R6, R7, R26) — One golden error case per `FM-E026` condition of reconciliation: a
  component sum that differs from its total, assets ≠ liabilities and equity by one unit, A.IX ≠
  item 21. Property: a statement that fails both a reconciliation and a classification check
  returns the reconciliation error.
- **AC3** (R2, R3, R5, R26) — `FM-E001` for: no statement, a negative unsigned amount, a missing
  or excessive beyond-one-year amount, a missing financial debt or shareholder loans part of D,
  two years with the same date; `FM-E009` for an invalid date. Every message has a JSON pointer.
- **AC4** (R8, R9, R12) — Property: no non-operating asset is part of `invested_capital`, and the
  non-operating assets of the balance sheet block equal those returned for the bridge, label by
  label. For each required treatment: missing with a non-zero amount returns
  `FM-E001` listing the destinations; a zero amount passes without it; a split that differs from
  the aggregate returns `FM-E026`; one golden case per destination.
- **AC5** (R10, R11) — One golden case per adjustment kind of Appendix C; an adjustment larger
  than its source returns `FM-E026`; a blank reason returns `FM-E001`. Property: the amounts in
  the explain assignment table sum to their aggregate.
- **AC6** (R12, R13, R14) — Golden cases match the reference script for every subtotal.
  Property (random valid inputs and adjustments): total funds invested = equity + Σ quasi-equity
  + net financial position + Σ debt-like items, exactly; the net financial position total equals
  the SPEC-0007 total of the returned object.
- **AC7** (R14, R15, R22) — Contract tests: `net_financial_position`, `non_operating_assets` and
  `debt_like_items` are accepted by `dcf.fcff` version 3 unchanged; `core_ebit` (as `ebit`) and
  `depreciation_and_amortisation` are accepted as `FreeCashFlowLines` fields; the SPEC-0007
  fields are accepted by the groups that take them.
- **AC8** (R16) — Property: `total_assets` = `fixed_assets` + `current_assets` =
  `equity` + `non_current_liabilities` + `current_liabilities` (equity after the deductions of
  R24); passing the block to `ratios.liquidity` and `ratios.capital_structure` gives the values
  of the reference applied to the same totals.
- **AC9** (R17, R17a, R18, R19, R20, R21) — Golden value added cases match the reference for
  every layer. Properties: `ebit` = `core_ebit` + `non_operating_result`; net income = item 21;
  profit before tax = A − B ± C ± D; the reconciliations of `core_ebit` and `ebit` add up to
  A − B and to A − B plus the C15 and C16 amounts moved; moving an income to the non-recurring
  result lowers EBITDA and `core_ebit` by its amount, and moving a cost raises both by its amount;
  C15 never enters `core_ebit`.
- **AC9a** (R18a) — Golden `write-downs.yaml` with D19, B10 c and B10 d all non-zero: D19 is in
  `value_adjustments_of_financial_assets` and in neither EBIT; B10 c is in the non-recurring
  result; B10 d lowers `core_ebit` and not EBITDA. With a `core_write_down` adjustment and a
  reason, B10 c lowers `core_ebit` and not EBITDA. An adjustment whose source is D returns
  `FM-E001`. Item C given only as a total returns `FM-E001`.
- **AC10** (R23) — Golden `lease-restatement.yaml` (constructed case): asset, liability,
  prepayment and equity adjustment; EBITDA rises by the lease payments moved out of B8; the
  identity of R13 holds.
- **AC11** (R24) — Golden cases with subscribed capital unpaid and with dividends declared.
- **AC12** (R25, R26, R27) — Unit tests: every convention of the table appears when it applies;
  every error has a JSON pointer; no boolean in the schema; two runs of every golden case give
  identical output.
- **AC13** (R12, R17) — At least one published worked example of the reorganisation and one of
  the value added layout, re-derived from printed statutory aggregates, match at their published
  precision; the golden file records the source, edition and page (Open question 16).

## Interfaces
- **Method id:** `statements.reorganise` version 1, new, preview, listed under `"0005"` in
  `specs/method-evidence.yaml` (Open question 1 discusses the two-method alternative).
- **Input:** `years`: list of `{balance_sheet_date, layout, balance_sheet?, income_statement?,
  treatments?, adjustments?}`; treatments are keyed by the aggregate's JSON pointer relative to
  the year; adjustments are `{kind, label, amount, reason, ...}`. Type names are set in
  `plan.md`.
- **Output per year:** `reorganised_balance_sheet` (Appendix B.1), `net_financial_position`
  (SPEC-0007 type) with `composition`, `current` and `non_current`; `non_operating_assets`,
  `debt_like_items`, `quasi_equity_items`; `current_non_current` (R16);
  `reorganised_income_statement` (Appendix B.2, the layers of R17 including `core_ebit` and
  `ebit`) with `ebit_reconciliation`; `assignments` and
  `adjustments` as applied (R11).
- **Schema changes:** additive: one new method, `schemas/statements.reorganise/v1.json`, one
  entry in `schemas/index.json`. No existing method, golden case or number changes.
- **Building blocks reused:** `ratios::NetFinancialPosition` and `NfpItem`; `tv::Line`; the
  debt-like item type and `MeasurementBasis` of SPEC-0001 (#44, Decision 21); field names of
  `FreeCashFlowLines` and of the SPEC-0007 inputs; `Explain`; `crate::num`; the registry.
  Whether `NetFinancialPosition` moves from `ratios` to a shared module is a plan decision with
  the architect (no schema change either way).
- **Errors:** `FM-E001`, `FM-E008`, `FM-E009`, `FM-E012` reused; new `FM-E026` (next free code
  at implementation), documented in `docs/en/reference/errors.md`.
- **Theme:** new commit scope `statements` in `docs/en/project/conventions.md`.

## Documentation
- Methodology page `docs/en/methods/statements-reorganise.md`: formula, Appendix A and C as
  tables, every required treatment and adjustment with its effect, an example from a filed
  abridged statement, "Verified against".
- Concept page `docs/en/concepts/financial-statements.md`: what the method takes, what it
  returns, how the outputs feed `ratios.*`, `dcf.free_cash_flows` and the bridge of `dcf.fcff`.
- `docs/en/reference/errors.md` (`FM-E026`); regenerated methods reference and schema; Italian
  editions; glossary entries for the terms below once verified; Python helper
  `python/fairmark/statements.py`; release note fragments for #35 and #36.

## Terminology and sources
"glossary" marks terms in `i18n/glossary.yaml`; other rows are to verify on the source named.

| Term | Identifier | Italian equivalent | Candidate source | Status |
| --- | --- | --- | --- | --- |
| Reorganise (the financial statements) | `statements.reorganise` | riclassificazione (finanziaria e operativa) | Koller et al. (V25) | to verify |
| Invested capital | `invested_capital` | capitale investito netto | Koller et al.; SPEC-0007 | glossary ("capitale investito"); "netto" to verify |
| Total funds invested | `total_funds_invested` | keep English unless verified | Koller et al. | to verify (V25) |
| Operating working capital; trade working capital | `operating_working_capital`, `trade_working_capital` | capitale circolante netto operativo; capitale circolante commerciale | Koller et al.; bank practice | to verify (V25, V28) |
| Operating provisions | `operating_provisions` | fondi operativi | Koller et al. | to verify (V25) |
| Net financial position | `net_financial_position` | posizione finanziaria netta (PFN); "indebitamento finanziario netto" in ESMA and Consob texts | SPEC-0007; ESMA32-382-1138 | glossary; ESMA verified (V15) |
| Non-operating assets; debt-like items | `non_operating_assets`, `debt_like_items` | as in SPEC-0001 | SPEC-0001 | glossary; SPEC-0001 |
| Quasi-equity | `quasi_equity_items` | keep English unless verified | Bank practice | to verify (V28) |
| Excess cash; operating cash | `excess_cash`, `operating_cash` | liquidità in eccesso; liquidità operativa | Koller et al.; Damodaran | to verify (V25) |
| Employee severance indemnity | `employee_severance_indemnity` | trattamento di fine rapporto (TFR) | IFRS statements of Italian issuers; IAS 19 | to verify (V22) |
| Shareholder loans | `shareholder_loans` | finanziamenti dei soci | Art. 2424 D.3; art. 2467 | to verify (V10) |
| Prepayments and accrued income; accruals and deferred income | as named | ratei e risconti attivi; passivi | Directive 2013/34/EU Annex III | to verify (V19) |
| Value of production; value added | `value_of_production`, `value_added` | valore della produzione; valore aggiunto | Art. 2425 A; Eurostat SBS | to verify (V24) |
| EBITDA | `ebitda` | margine operativo lordo (MOL) | CFA Institute; Banca d'Italia; EBA/GL/2020/06 Annex 3 | Italian "MOL" verified in the Banca d'Italia indicators (V30); English verified in EBA Annex 3 (V23) |
| EBIT of core management | `core_ebit` | reddito operativo della gestione caratteristica | Italian credit analysis; "core operations" in Koller et al. | to verify (V25, V27) |
| EBIT (operating income of the company) | `ebit` | reddito operativo aziendale | CFA Institute; SPEC-0001 | to verify (V26, V27) |
| Non-operating (accessory) management | `non_operating_result` | gestione accessoria (extra-caratteristica) | Italian textbooks; Koller et al. (non-operating) | to verify (V25, V27) |
| Financial management | `financial_result` | gestione finanziaria | Italian textbooks | to verify (V27) |
| Write-downs | `write_downs` (B10 c, B10 d, D19) | svalutazioni | Art. 2425 B10, D19; OIC 12 | verified in OIC 12 (V12) for D18 and D19 |
| Non-recurring items | `non_recurring` | componenti non ricorrenti | ESMA APM guidelines; Consob | to verify (V17, V18) |
| Materials consumed; purchases | `materials_consumed`, `purchases` | consumi; acquisti | Art. 2425 B6, B11 | glossary (purchases) |

## Sources verified (2026-11-08)
Recorded as verified by the coordinator on 2026-11-08. Paragraph and page numbers not stated
below are still to verify.

- **V7** Civil Code art. 2435-bis (abridged layout) and **V29** D.Lgs. 125/2024: the current
  thresholds of the abridged layout and of micro-entities.
- **V8** Civil Code art. 2435-ter (micro-entities).
- **V9** (part) Civil Code art. 2427, paragraph 1, n. 13: amount and nature of items of income
  and expense of exceptional size or incidence; basis of the non-recurring adjustments (R17).
- **V12** OIC 12, revised December 2016 after D.Lgs. 139/2015: former extraordinary items are
  classified by nature in A5 and B14; gains from debt restructuring in C16 d; D18 and D19 for the
  value adjustments of financial assets. Paragraph numbers to verify.
- **V15** ESMA32-382-1138 (guideline 39, net financial indebtedness) and Consob, Richiamo di
  attenzione n. 5/21.
- **V23** EBA/GL/2020/06, Guidelines on loan origination and monitoring, Annex 3: metrics for
  credit granting and monitoring; for firms they include EBITDA, debt to equity and debt service
  coverage. Page numbers to verify.
- **V30** Banca d'Italia, definitions used in its financial stability analysis of firms:
  leverage = financial debt / (financial debt + equity); financial charges / MOL; financial
  debt / MOL. Publication and page to verify.
- **V31** Ministry of Justice, decreto dirigenziale of 23 April 2026, practical test (test
  pratico) of the negotiated settlement of the business crisis: debt to be restructured over
  the annual free flows serving the debt; it replaces the decree of 21 March 2023. Article and
  annex numbers to verify.

## Sources to verify
Revision 1's V16 (CESR) and the Annex VI part of V19 are no longer cited. CNDCEC documents on
crisis indicators are not cited in version 1; they belong to future extension 8 and to the
deferred `ratios.crisis_indicators` of SPEC-0007.

1. **V1** Art. 2423: statements in euro units without decimals; where rounding differences go.
2. **V2** Art. 2423-ter: further subdivision, grouping, additional items, comparatives.
3. **V3** Art. 2424: letters and Roman numerals of Appendix A.1, including A.VII and A.X, and the
   separate indication of amounts beyond the next financial year for C.II and D.
4. **V4** Art. 2424-bis, paragraph 1 (assets for durable use are fixed assets).
5. **V5** Art. 2425: aggregates of Appendix A.2, item 17-bis, and the sign presentation of A2,
   A3 and B11.
6. **V7** and **V8**: verified, see above.
7. **V9** Art. 2427, paragraph 1, n. 19-bis (shareholder loans) and n. 22 (financial leases), and
   which disclosures abridged filers must give (n. 13 verified, see above).
8. **V10** Art. 2467 (postponement of shareholder loans).
9. **V11** D.Lgs. 139/2015: removal of area E, first application (its effect on the layout is
   verified through OIC 12, V12).
10. **V12** OIC 12: the paragraph numbers of the verified content, and the signs of A2, A3, B11.
11. **V13** OIC 10: definition of cash used for `cash`.
12. **V14** OIC 18, 19, 31: prepayments and accruals, payables, provisions and TFR.
13. **V15**: verified, see above.
14. **V17** ESMA Guidelines on Alternative Performance Measures (ESMA/2015/1415): EBITDA,
    non-recurring labels.
15. **V18** Consob Comunicazione DEM/6064293 of 28 July 2006: non-recurring events.
16. **V19** Directive 2013/34/EU, Annex III: English names of the balance sheet aggregates.
17. **V20** IAS 1 paragraphs 66 to 76 (current and non-current); IFRS 18.
18. **V21** IFRS 16 (lessee accounting).
19. **V22** IAS 19 and the TFR in IFRS statements of Italian companies.
20. **V23**: verified, see above (page numbers to verify).
21. **V24** Eurostat structural business statistics: production value, value added at factor
    cost, personnel costs, gross operating surplus.
22. **V25** Koller, Goedhart, Wessels, *Valuation*, 7th edition, chapter "Reorganizing the
    financial statements": invested capital, operating working capital, operating provisions,
    total funds invested, excess cash, debt equivalents, core and non-operating income (behind
    a paywall in the SPEC-0001 review).
23. **V26** CFA Institute curriculum: EBITDA, working capital.
24. **V27** Italian textbooks with worked examples and the layers of R17 (gestione
    caratteristica, accessoria, finanziaria; reddito operativo caratteristico and aziendale),
    edition and page to be named by the maintainer. Candidates: Ferrero, Dezzani, Pisoni, Puddu,
    *Le analisi di bilancio. Indici e flussi* (Giuffrè); Caramiello, Di Lazzaro, Fiori, *Indici
    di bilancio* (Giuffrè); Teodori, *L'analisi di bilancio* (Giappichelli).
25. **V28** Bank reclassification schemes (for example Centrale dei Bilanci, now Cerved): TFR,
    shareholder loans, leases, tax debts in instalments, quasi-equity, and where they place C15
    and the write-downs of B10 c and B10 d.
26. **V30** (part) The Banca d'Italia publication in which the three indicator definitions
    appear, and whether its MOL deducts B10 d and provisions.

## Open questions
Questions 1 to 3 and 12 to 16 are resolved with the recommendation stated, under the delegation the maintainer gave on 2026-11-08 (decide as a bank credit analyst and an asset management analyst would, aligned with CFA, IFRS and EBA practice); they are listed for his confirmation when the spec is submitted for approval.
Each lists options and a recommendation. **[bank]** marks questions that need the maintainer's
bank-practice judgement. Questions 17 and 18 are new in revision 3 and open.

- [x] **1. Method ids.** (a) One method `statements.reorganise` with optional balance sheet and
      income statement; (b) two methods, `statements.reorganise_balance_sheet` and
      `statements.reorganise_income_statement`. Recommendation: (a). Leases and the profit
      cross-check touch both statements, so one call keeps them consistent and needs no
      duplicated adjustments; agents learn one tool and one schema; reconciliation errors come
      first in one place. The cost is one version for both statements, acceptable while the
      method is preview. Choose (b) if schema size for function calling becomes a problem.
- [x] **2. Balance check.** (a) Exact equality; (b) an explicit tolerance in currency units.
      Recommendation: (a). Filed statements in euro units balance exactly; a difference is an
      extraction error to fix.
- [x] **3. Signs.** (a) As presented in the statutory layouts (R5); (b) signed by effect on the
      result. Recommendation: (a): an agent copies the filing as printed. B11 is the trap
      (positive is a decrease); the schema and page say so.
- [x] **4. Payables D by nature. [bank]** Resolved 2026-11-08: option (a), approved by the maintainer. The abridged layout shows D as one total with the
      amount beyond the next financial year. (a) Require the financial debt part and the
      shareholder loans part, each with its beyond part; remainder operating; trade payables
      optional; (b) require the full split by nature; (c) default D to operating. Recommendation:
      (a). The net financial position cannot be built without them, and the notes or the filed
      detail give them; (c) would be a silent and material default.
- [x] **5. Provisions B and employee severance indemnity C. [bank]** Resolved 2026-11-08: the maintainer asked for the choice a bank or a financial intermediary would make. Credit practice and the Italian functional layout deduct the TFR and the provisions of operating risks from invested capital, as operating liabilities, and keep them out of the net financial position; provisions on financial assets go to the financial area. Decision: documented default `operating_provision` for C (TFR) and for B, reported in conventions; never inside the net financial position; the analyst may move all or part to `debt_like` with a reason (valuation practice), and B.3 (derivatives) keeps a required treatment when not zero. Sources: Italian functional layout as taught in analisi di bilancio (capitale investito operativo netto net of operating provisions and TFR); ESMA32-382-1138 guideline 39 keeps provisions out of financial indebtedness; to cite precisely at approval (V27, V28). (a) Required treatment when
      not zero: `operating_provision` (deducted from invested capital) or `debt_like` (a claim in
      the bridge), or a split; never inside the net financial position; (b) documented
      defaults: TFR debt-like, B operating. Recommendation: (a). Textbook functional layouts
      deduct the TFR from invested capital, valuation practice treats it as debt-like, and the
      ESMA and bank definitions keep it out of the net financial position; the analyst decides.
- [x] **6. Shareholder loans. [bank]** Resolved 2026-11-08: option (a), approved by the maintainer. (a) Required treatment `financial_debt` (in the net
      financial position) or `quasi_equity` (a separate source line, not in equity nor in the
      net financial position), or a split (for example the postponed part); (b) default
      financial debt. Recommendation: (a). Credit analysis often treats postponed loans as
      quasi-equity, valuation as debt; statutory equity is never changed.
- [x] **7. A5, B14 and non-recurring items. [bank]** Resolved 2026-11-08: option (a). The maintainer asked (2026-11-08) to decide as a credit analyst of a bank or a financial intermediary and as an analyst of an asset management firm would, aligned with CFA, IFRS and EBA practice, classifying items by financial pertinence and risk, because the output serves to judge the financial health of the company. Credit analysis (EBA/GL/2020/06, sustainable cash flows) and OIC 12 (former extraordinary items in A5 and B14, disclosure under art. 2427 n. 13) both support filed totals as operating with visible, reasoned normalisations. (a) Operating by default, with labelled
      adjustments for non-operating and non-recurring parts of any aggregate; (b) a required
      classification of A5 and B14 when not zero. Recommendation: (a). It keeps the input to the
      filed totals and puts every normalisation in a reasoned, visible adjustment; the page
      warns, as the ESMA APM guidelines do, against recurring items labelled non-recurring.
- [x] **8. EBITDA scope and grouped B10. [bank]** Resolved 2026-11-08: option (a), the definition used in Italian credit analysis and in bank covenants. The maintainer asked (2026-11-08) to decide as a credit analyst of a bank or a financial intermediary and as an analyst of an asset management firm would, aligned with CFA, IFRS and EBA practice, classifying items by financial pertinence and risk, because the output serves to judge the financial health of the company. (a) B10 d, B12, B13 below EBITDA; B10 a to c
      (grouped in the abridged layout) as depreciation and amortisation, impairments by
      adjustment; (b) required statement of the impairment part of B10 a to c and of the place
      of B10 d. Recommendation: (a), the usual Italian definition of MOL, reported in words.
- [x] **9. Leases. [bank]** Resolved 2026-11-08: option (b). The maintainer asked (2026-11-08) to decide as a credit analyst of a bank or a financial intermediary and as an analyst of an asset management firm would, aligned with CFA, IFRS and EBA practice, classifying items by financial pertinence and risk, because the output serves to judge the financial health of the company. Lease obligations are financial in pertinence and risk; the optional financial-method adjustment makes EBITDA and the net financial position comparable between OIC and IFRS 16 preparers, from the data of art. 2427 n. 22. (a) As reported only; (b) optional financial-method adjustment
      (R23) in version 1, no deferred taxes; (c) (b) deferred to a later version.
      Recommendation: (b). Bank practice restates leases so that EBITDA and the net financial
      position compare between OIC and IFRS 16 preparers; the adjustment is optional.
- [x] **10. Financial fixed assets B.III.** Resolved 2026-11-08: option (a), required treatment when not zero, because the aggregate mixes items of different pertinence and risk. The maintainer asked (2026-11-08) to decide as a credit analyst of a bank or a financial intermediary and as an analyst of an asset management firm would, aligned with CFA, IFRS and EBA practice, classifying items by financial pertinence and risk, because the output serves to judge the financial health of the company. (a) Required treatment when not zero: operating,
      non-operating or financial asset, or a split; (b) default non-operating. Recommendation:
      (a): the aggregate mixes strategic stakes, surplus investments and loans.
- [x] **11. Net financial position composition. [bank]** Resolved 2026-11-08: option (a), approved by the maintainer. (a) SPEC-0007 structure with
      composition and current and non-current parts; C.III and all of C.IV inside by default;
      operating cash, financial receivables in C.II, accrued interest, non-current trade
      payables and tax debts in instalments moved by adjustment; (b) operating cash deducted by
      default as a percentage of revenue (Koller practice). Recommendation: (a). Italian and
      ESMA practice deduct all cash; a percentage rule would be a hidden parameter; the analyst
      states operating cash as an amount with a reason, and `excess_cash` is reported.
- [x] **12. Current and non-current block.** (a) Keep it (R16) with the stated defaults, because
      `ratios.liquidity` and `ratios.capital_structure` need it; (b) drop it and let the analyst
      compute the inputs. Recommendation: (a). It costs a few documented defaults and avoids
      arithmetic in agents.
- [x] **13. Cost of sales for `ratios.working_capital`.** (a) Not returned; `purchases` and
      `materials_consumed` returned under their own names, and the analyst supplies cost of sales
      to the ratio; (b) return `materials_consumed` as `cost_of_sales`. Recommendation: (a): (b)
      would give one name two meanings.
- [x] **14. Subscribed capital unpaid and dividends declared.** (a) Capital unpaid deducted from
      equity; dividends declared by adjustment to debt-like items; (b) both by adjustment only.
      Recommendation: (a): unpaid capital does not fund the company until paid.
- [x] **15. Measurement basis of bridge items.** (a) Carrying amount, stated with the
      `MeasurementBasis` of SPEC-0001 Decision 21; (b) an optional market value per item in
      this method. Recommendation: (a): market values belong to the bridge, where the analyst
      restates them.
- [x] **16. Golden sources.** (a) Exact reference script `scripts/reference_statements.py`
      written from Appendix A to C; (b) published worked examples named by the maintainer (V27);
      (c) two or three anonymised practitioner cases, at least one abridged. Recommendation: all
      three; (b) and (c) are needed for AC13 and for promotion to available.
- [ ] **17. Write-downs (svalutazioni). [bank]** The maintainer's guidance (2026-11-08) was that
      write-downs are extra-characteristic. Options: (a) the rule of R18a: D19 always in the
      financial area; B10 c non-recurring by default below `core_ebit`, movable into the core
      management by an adjustment with a reason; B10 d in the core management, below EBITDA;
      (b) every write-down outside the core management, B10 d included; (c) B10 c core by
      default as well. Recommendation: (a). It keeps the guidance for D19 and B10 c, and keeps
      B10 d in the core for prudence: losses on trade receivables recur with sales on credit, so
      leaving them out would overstate the core result that a lender relies on. B10 d stays
      below EBITDA, as Open question 8 decided.
- [ ] **18. Which EBIT feeds the free cash flows and the operating return.** `core_ebit`
      excludes the non-operating result; `ebit` includes C15, the non-operating parts of A5 and
      B14 and the income moved out of C16. Options: (a) document that `dcf.free_cash_flows` and
      ROI on invested capital (`ratios.profitability` with `operating_return_base` =
      `invested_capital`) should receive `core_ebit`, and that interest cover and ROA on total
      assets should receive `ebit`; (b) feed `ebit` everywhere. Recommendation: (a). The
      non-operating assets are added in the `dcf.fcff` bridge at their value, so their income
      inside the free cash flows would be counted twice; and invested capital excludes those
      assets, so the numerator of ROI should exclude their income. The engine returns both; the
      methodology page and the concept page say which goes where. This needs no change in
      SPEC-0001 or SPEC-0007, whose `ebit` fields are documented as the operating result.

## Appendix A — Statutory aggregates and their destinations (proposed)
Choice: **F** fixed; **D** default, reported, changed only by an adjustment; **R** required
treatment when the amount is not zero. "NFP" is a component of the net financial position.

### A.1 Balance sheet (art. 2424; abridged art. 2435-bis)

| Reference | Proposed identifier | Destination | Choice | Adjustments (Appendix C) | Current / non-current (R16) |
| --- | --- | --- | --- | --- | --- |
| Assets A | `subscribed_capital_unpaid` (or "of which" of C.II) | deducted from equity | F | none | deducted |
| B.I | `intangible_assets` (net; gross and accumulated optional) | operating fixed assets | D | `non_operating_asset` | non-current |
| B.II | `tangible_assets` (net; gross and accumulated optional) | operating fixed assets | D | `non_operating_asset`, `lease` | non-current |
| B.III | `financial_fixed_assets` | operating fixed assets, non-operating asset, or NFP financial asset | R | none | non-current |
| C.I | `inventories` | operating working capital | F | none | current |
| C.II | `receivables` (total, beyond one year; trade optional) | operating working capital | D | `financial_receivable`, `non_operating_asset` | by statutory split |
| C.III | `current_financial_assets` | NFP other financial asset | D | `non_operating_asset` | current |
| C.IV | `cash` | NFP cash | D | `operating_cash` | current |
| Assets D | `prepayments_and_accrued_income` (or "of which" of C.II; non-current part optional) | operating working capital | D | `lease` (prepayment) | current, or the part given |
| Liabilities A.I to A.X | `equity` (total or Roman numerals; A.IX profit or loss) | equity | F | `dividends_declared`, `lease` (equity effect) | equity |
| B, other than derivatives | `provisions` (non-current part optional) | operating provision | D | `debt_like_provision` | non-current, or the part given |
| B, derivative liabilities | `provisions.of_which_derivative_liabilities` (optional) | NFP other financial liability, or operating provision | R | none | non-current, or the part given |
| C | `employee_severance_indemnity` (non-current part optional) | operating provision | D | `debt_like_provision` | non-current, or the part given |
| D, financial debt part | `payables.financial_debt` (with beyond) | NFP gross debt | F | none | by its split |
| D, shareholder loans part | `payables.shareholder_loans` (with beyond) | NFP gross debt, or quasi-equity | R | none | by its split |
| D, remainder | `payables` total less the parts above (trade optional) | operating working capital | D | `tax_instalments`, `non_current_trade_payables` | by statutory split |
| E | `accruals_and_deferred_income` (or "of which" of D; non-current part optional) | operating working capital | D | `accrued_interest` | current, or the part given |

### A.2 Income statement (art. 2425; groupings of art. 2435-bis)

| Reference | Proposed identifier | Area and layer (R17) | Choice | Adjustments |
| --- | --- | --- | --- | --- |
| A1 | `revenue` | core: value of production | F | `non_recurring` |
| A2, A3 (may be grouped) | `change_in_product_inventories` (signed) | core: value of production | F | none |
| A4 | `own_work_capitalised` | core: value of production | F | none |
| A5 | `other_revenue_and_income` | core: value of production | D | `non_operating`, `non_recurring` |
| B6 | `purchases` | core: external costs (materials consumed) | F | `non_recurring` |
| B7 | `services` | core: external costs | F | `non_recurring` |
| B8 | `use_of_third_party_assets` | core: external costs | F | `lease`, `non_recurring` |
| B9 (c, d, e may be grouped) | `personnel_costs` | core: below value added, above EBITDA | F | `non_recurring` |
| B10 a, b (with c when grouped and no "of which") | `depreciation_and_amortisation` | core: below EBITDA | F | `non_recurring` (impairment in a grouped B10) |
| B10 c (separate or "of which") | `write_downs_of_fixed_assets` | non-recurring result | D (R18a) | `core_write_down` |
| B10 d | `write_downs_of_receivables` | core: below EBITDA | F (R18a) | `non_recurring` |
| B11 | `change_in_raw_material_inventories` (signed) | core: external costs (materials consumed) | F | none |
| B12, B13 | `provisions_for_risks`, `other_provisions` | core: below EBITDA | F | `non_recurring` |
| B14 | `other_operating_expenses` | core: external costs | D | `non_operating`, `non_recurring` |
| C15 | `income_from_equity_investments` | non-operating result | D | `non_recurring` |
| C16 (b, c may be grouped) | `other_financial_income` | financial result | D | `non_operating`, `non_recurring` |
| C17 | `interest_and_other_financial_charges` | financial result | F | `lease` (interest), `non_recurring` |
| C17-bis | `exchange_gains_and_losses` (signed) | financial result | F | `non_recurring` |
| D18, D19 (a to d may be grouped) | `revaluations_of_financial_assets`, `write_downs_of_financial_assets` | value adjustments of financial assets (financial area) | F (R18a) | none |
| 20 | `income_taxes` | income taxes as given | F | none |
| 21 | `profit_or_loss_for_the_year` | check of net income | F | none |

## Appendix B — Reorganised structures (proposed)

### B.1 Balance sheet
Uses are presented in two blocks: the operating (core) block, up to `invested_capital`, and the
non-operating (accessory) block, `non_operating_assets`; their sum is `total_funds_invested`.
- Operating fixed assets = B.I + B.II + B.III (operating part) − non-operating adjustments +
  lease assets.
- `trade_working_capital` (when trade parts are given) = trade receivables + inventories − trade
  payables.
- `operating_working_capital` = C.I + C.II (operating) + assets D (operating) + operating cash
  − D remainder (operating) − E (operating).
- Operating provisions = B and C treated as operating provisions.
- `invested_capital` = operating fixed assets + `operating_working_capital` − operating
  provisions.
- `non_operating_assets` = B.III (non-operating part) + the parts of B.I, B.II, C.II and C.III
  moved by `non_operating_asset` adjustments, each listed with its label and source (the same
  list as for the bridge, R15).
- `total_funds_invested` = `invested_capital` + Σ `non_operating_assets`.
- `equity` = A − subscribed capital unpaid − dividends declared ± lease equity effect.
- `net_financial_position` = gross debt − cash + Σ other items (SPEC-0007), with `excess_cash` =
  C.IV − operating cash reported.
- Sources = `equity` + Σ quasi-equity items + `net_financial_position` + Σ `debt_like_items`.
- Identity: `total_funds_invested` = sources.

### B.2 Value added income statement, by area
Core (characteristic) management:
- `value_of_production` = A1 + A2 + A3 + A4 + A5 (core part).
- External costs = B6 + B11 (`materials_consumed`) + B7 + B8 + B14 (core part).
- `value_added` = `value_of_production` − external costs.
- `ebitda` (MOL) = `value_added` − B9.
- `core_ebit` = `ebitda` − B10 a, b (and c when grouped) − B10 d − B12 − B13 (core parts), and
  − B10 c moved into the core by `core_write_down`.

Non-operating (accessory) management:
- `non_operating_result` = A5 (non-operating part) − B14 (non-operating part) + C15 (unless moved
  to non-recurring) + C16 (non-operating part).
- `ebit` = `core_ebit` + `non_operating_result`.

Non-recurring items:
- `non_recurring_result` = Σ non-recurring income parts − Σ non-recurring expense parts − B10 c
  (default, R18a).

Financial management:
- `financial_result` = C16 (financial part) − C17 + C17-bis.
- `value_adjustments_of_financial_assets` = D18 − D19.

Result:
- `profit_before_tax` = `ebit` + `non_recurring_result` + `financial_result` +
  `value_adjustments_of_financial_assets`; equals A − B ± C ± D.
- `net_income` = `profit_before_tax` − item 20; equals item 21.
- With a lease adjustment: B8 less the lease payments; depreciation added to B10 a, b; interest
  added to C17.

## Appendix C — Adjustment kinds (proposed)
Each adjustment has `kind`, `label`, `amount` (zero or more), `reason` and the fields listed.

| Kind | Source | Target | Fields |
| --- | --- | --- | --- |
| `non_operating_asset` | operating fixed assets, C.II, or C.III | non-operating assets | aggregate pointer |
| `financial_receivable` | C.II (operating) | NFP other financial asset | maturity |
| `operating_cash` | NFP cash | operating working capital | none |
| `accrued_interest` | E (operating) | NFP other financial liability | none |
| `tax_instalments` | D remainder (operating) | debt-like items | maturity |
| `non_current_trade_payables` | D remainder (operating) | NFP other financial liability | none |
| `dividends_declared` | equity | debt-like items | none |
| `lease` | double entry (R23) | operating fixed assets, NFP, equity; B8, B10, C | asset, liability with current part, prepayment, payments, depreciation, interest |
| `debt_like_provision` | provisions B or employee severance indemnity C (operating provision) | debt-like items | aggregate pointer |
| `non_recurring` | any income statement aggregate except D18 and D19 | non-recurring result | aggregate pointer |
| `non_operating` | A5, B14, C16 | non-operating result | aggregate pointer; optional related non-operating asset label |
| `core_write_down` | B10 c (non-recurring result) | core management, below EBITDA | none |
