# Financial statements reorganised (`statements.reorganise`)

Reorganises the financial statements of one or more financial years into what the business
uses (operating), what it holds outside its business (non-operating) and how it is financed.
Each year is given either as the aggregates of the statutory layouts (art. 2424 and 2425 of the
Italian Civil Code, ordinary or abridged, art. 2435-bis) or as totals already reclassified, for
example by a bank. The result is in the shapes that [`dcf.fcff`](dcf-fcff.md) and the
`ratios.*` groups take, unchanged.

This build returns, for each year, the reorganised **balance sheet** (invested capital,
non-operating assets, equity, quasi-equity, the net financial position with its composition and
maturities, debt-like items, and current and non-current amounts) and the reorganised **income
statement** by nature (value of production, value added, EBITDA, operating profit, investing
result, profit before financing and taxes, non-recurring and financing results, profit before
tax, income taxes and net income), with the reconciliation of its two IFRS 18 aligned subtotals
with A − B. The exact changes between consecutive years come with a later build (see the
[roadmap](../project/roadmap.md)).

## Best effort

Every input is optional. An output is computed when the inputs it needs are given; otherwise it
is `null` and listed in `not_computed` with the JSON pointers of the missing inputs. A missing
input is never taken as zero and never an error. An agent that reads a few totals from a text
can pass them, receive what they allow and ask the user only for the pointers listed.

A few items are taken as zero when they are not given, so that the statutory layouts can be
used as filed. Each is listed in `meta.defaults_applied` with its pointer and the value `"0"`,
and stated in the conventions:

- item A, subscribed capital unpaid (decision D1 of 2026-11-08): the same zero is deducted from
  equity and from the total assets of the current and non-current block;
- an "of which" part (decision D2): item A or assets D within receivables, liabilities E within
  payables, the derivative part of provisions, also when the parent aggregate is not given; in
  the ordinary layout each is a separate item;
- the separate line D or E, when assets D are given within C.II or liabilities E within D (the
  abridged layout, art. 2435-bis; maintainer approval of 2026-11-08).

Item A and the "of which" parts taken as zero never enter a check; a part presented within C.II
or D counts as given: see [Checks](#checks).

Financial debt and shareholder loans within payables are needed for the net financial position
and for operating working capital (Decision 4): without them both are not computed.

## Formula

$$
\text{invested capital} = \text{operating fixed assets} + \text{operating working capital} - \text{operating provisions}
$$

$$
\text{total funds invested} = \text{invested capital} + \sum \text{non-operating assets}
$$

$$
= \text{equity} + \sum \text{quasi-equity items} + \text{net financial position} + \sum \text{debt-like items}
$$

$$
\text{net financial position} = \text{gross debt} - \text{cash} + \sum \text{other items}
$$

with, for a statutory year (Appendix B.1 of SPEC-0005):

- operating fixed assets = B.I + B.II + B.III treated as operating − parts moved to
  non-operating assets;
- operating working capital = C.I + C.II (operating part) + assets D + operating cash − payables
  D (operating part) − liabilities E (operating part);
- operating provisions = provisions B and employee severance indemnity C treated as operating;
- equity = liabilities A − subscribed capital unpaid (zero when not given) − dividends declared;
- gross debt = financial debt + shareholder loans treated as financial debt + lease liabilities
  restated by a `lease` adjustment (decision P3 of 2026-11-09); cash = C.IV −
  operating cash; other items, each with its sign: C.III, financial receivables, B.III treated
  as a financial asset (negative), derivative liabilities, accrued interest and trade payables
  with extended terms (positive);
- excess cash = C.IV − operating cash, the cash of the net financial position.

**Against the ESMA table** (ESMA32-382-1138, guideline 39; decision P4 of 2026-11-09). The net
financial position of Fairmark is the analyst measure of SPEC-0007, not the ESMA "total
financial indebtedness". It differs where Fairmark deducts what the ESMA table does not:
non-current financial assets (B.III treated as `financial_asset`, financial receivables due
beyond one year, the derivative assets within B.III) and the hedging derivative assets within
C.III, which ESMA leaves out of the other current financial assets. The composition lists each
of these items, so the ESMA figure can be rebuilt by removing them. An ESMA view (rows A to M)
is planned as a separate issue.

Derivatives (decision D3): derivative liabilities within provisions B treated as
`financial_liability` are non-current in the net financial position. Derivative assets are not
given separately: those within B.III (B.III.4) follow the treatment of B.III and are
non-current; those within C.III (C.III.5) are current financial assets of the net financial
position.

A positive net financial position is net debt. Its total is computed by the function of the
ratios (SPEC-0007), so it equals the total of the object returned.

### Income statement (Appendix B.2)

$$
\text{EBITDA} = \text{value added} - \text{personnel costs}
$$

$$
\text{profit before financing and taxes} = \text{operating profit} + \text{investing result}
$$

$$
\text{profit before tax} = \text{profit before financing and taxes} + \text{non-recurring result} + \text{financing result}
$$

with, for a statutory year:

- value of production = A1 + A2 and A3 + A4 + A5, core (less the parts moved by `investing`
  and `non_recurring` adjustments); total A is used when given;
- external costs = B6 + B7 + B8 + B11 + B14, each core, B8 also net of lease payments (signed;
  decision E1 of 2026-11-09); value added = value of production − external costs;
  `materials_consumed` = B6 + B11;
- EBITDA (margine operativo lordo, MOL) = value added − B9; depreciation, amortisation and
  write-downs (B10), provisions (B12, B13), the investing and financing results and the
  non-recurring items are below it. The definition is returned in words as
  `ebitda_definition`, ready for the `ratios.*` groups;
- depreciation and amortisation = B10 a and b, + B10 c with the treatment
  `write_downs_of_fixed_assets: operating` (the default, P1), + lease depreciation;
- other charges below EBITDA = B12 + B13, + B10 d with the treatment
  `write_downs_of_receivables: operating` (the default, E3), each core (E2);
- operating profit (reddito operativo della gestione caratteristica, margine operativo netto)
  = EBITDA − depreciation and amortisation − other charges below EBITDA;
- investing result = investing parts of A5 − investing parts of B14 + C15 + C16 + (D18 − D19),
  − B10 c and − B10 d only with the treatment `investing`; total D is used when given;
- profit before financing and taxes (reddito operativo aziendale, EBIT in Italian practice) =
  operating profit + investing result;
- non-recurring result = Σ non-recurring income − Σ non-recurring charges, from the
  `non_recurring` adjustments: zero, with the convention "no item labelled non-recurring", when
  there are none; null, listed in `not_computed`, when the year gives no income statement (E6);
- financing result = C17-bis − C17 (C17 plus lease interest, returned as
  `interest_and_other_financial_charges`). C16, which includes the interest on cash and on the
  financial receivables of the net financial position, is in the investing result (the IFRS 18
  investing category), so the financing result is gross of that interest;
- net income = profit before tax − item 20, taxes as given.

The two subtotals correspond to the subtotals required by IFRS 18, "operating profit or loss"
and "profit or loss before financing and income taxes", under the conventions of this page
(decision P1 amends R16): items labelled non-recurring are excluded from `operating_profit`,
which IFRS 18 would keep in the operating category, and the write-downs follow their
treatments. IFRS 18 applies to IFRS preparers and art. 2425 to OIC preparers, and the
reorganisation maps both to the same subtotals. `operating_profit` excludes the investing area
(income from investments and from cash, D18 and D19) and the non-recurring items.

**Write-downs and revaluations** (Decision 17 of SPEC-0005, as amended by E3 and P1 of
2026-11-09). D18 and D19 (financial assets) are in the investing result, below operating profit
and above profit before financing and taxes; no adjustment moves them. B10 c (write-downs of
tangible and intangible fixed assets) is operating by default, below EBITDA with depreciation
and amortisation (IFRS 18; OIC 12 paragraph 75); the treatment
`write_downs_of_fixed_assets: investing` places it in the investing result. B10 d
(write-downs of current receivables and cash) is operating by default, in the other charges
below EBITDA, as IFRS 18 practice does for the impairment of trade receivables; the treatment
`write_downs_of_receivables: investing` places it in the investing result. EBITDA is the same
with either treatment. A one-off impairment is isolated with a `non_recurring` adjustment. With
B10 c operating, B10 a to c given as one amount (abridged layout, art. 2435-bis) gives the same
outputs with or without its B10 c part; with the treatment `investing`, a group given without
its B10 c part is read as depreciation and amortisation, and the conventions say so (Decision
8).

**Reconciliations** (R19). `reconciliations.operating_profit` and
`reconciliations.profit_before_financing_and_taxes` start from A − B of the statutory
statement, list every amount moved (write-downs, investing and non-recurring parts, lease
lines, C15, C16 and D) with the pointers actually read (`sources`: both D18 and D19 when both
are read; the B10 a to c group, with the fix, when B10 c is read as none), and end at the
subtotal, exactly. Profit before tax equals A − B ± C ± D and net income equals item 21, except
for the lease difference, payments − depreciation − interest (E4).

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.statements.reorganise(
    [
        {
            "label": "2024",
            "date": "2024-12-31",
            "statutory": {
                "balance_sheet": {
                    "subscribed_capital_unpaid": "0",
                    "intangible_assets": {"total": "120000"},
                    "tangible_assets": {"total": "900000"},
                    "financial_fixed_assets": "150000",
                    "inventories": "400000",
                    "receivables": {"total": "950000", "beyond_one_year": "50000", "trade": "800000"},
                    "current_financial_assets": "60000",
                    "cash": "240000",
                    "prepayments_and_accrued_income": {"total": "30000"},
                    "equity": {"total": "900000"},
                    "provisions": {"total": "70000", "of_which_derivative_liabilities": "10000"},
                    "employee_severance_indemnity": {"total": "180000"},
                    "payables": {
                        "total": "1650000",
                        "beyond_one_year": "600000",
                        "financial_debt": {"total": "900000", "beyond_one_year": "550000"},
                        "shareholder_loans": {"total": "100000", "beyond_one_year": "50000"},
                        "trade": "500000",
                    },
                    "accruals_and_deferred_income": {"total": "50000"},
                }
            },
        }
    ],
    treatments={
        "financial_fixed_assets": "non_operating",
        "shareholder_loans": "financial_debt",
        "derivative_liabilities": "financial_liability",
    },
)
bs = r.years[0]["balance_sheet"]
assert bs["operating"]["invested_capital"] == Decimal("1460000")  # 1020000 + 680000 − 240000
assert bs["non_operating_assets_total"] == Decimal("150000")
nfp = bs["sources"]["net_financial_position"]
assert nfp["total"] == Decimal("710000")  # 1000000 − 240000 − 60000 + 10000
assert (nfp["current"], nfp["non_current"]) == (Decimal("100000"), Decimal("610000"))
assert bs["total_funds_invested"] == bs["sources"]["total_sources"] == Decimal("1610000")
```

The income statement of the same company (ordinary layout, art. 2425):

```python
from decimal import Decimal

import fairmark as fm

r = fm.statements.reorganise(
    [
        {
            "label": "2024",
            "statutory": {
                "income_statement": {
                    "revenue": "5000000",
                    "change_in_product_inventories": "40000",
                    "own_work_capitalised": "10000",
                    "other_revenue_and_income": "50000",
                    "purchases": "2200000",
                    "services": "1244000",
                    "use_of_third_party_assets": "150000",
                    "personnel_costs": "1000000",
                    "depreciation_and_write_downs": {
                        "depreciation_and_amortisation": "180000",
                        "write_downs_of_fixed_assets": "20000",
                        "write_downs_of_receivables": "15000",
                    },
                    "change_in_raw_material_inventories": "-30000",
                    "provisions_for_risks": "10000",
                    "other_provisions": "5000",
                    "other_operating_expenses": "40000",
                    "income_from_equity_investments": "12000",
                    "other_financial_income": "3000",
                    "interest_and_other_financial_charges": "60000",
                    "exchange_gains_and_losses": "-1000",
                    "revaluations_of_financial_assets": "2000",
                    "write_downs_of_financial_assets": "8000",
                    "income_taxes": "64000",
                    "profit_or_loss_for_the_year": "150000",
                }
            },
        }
    ]
)
inc = r.years[0]["income_statement"]
assert inc["ebitda"] == Decimal("496000")  # 5100000 − 3604000 − 1000000
assert inc["operating_profit"] == Decimal("266000")  # 496000 − (180000 + 20000) − (10000 + 5000 + 15000)
assert inc["investing_result"] == Decimal("9000")  # 12000 + 3000 + 2000 − 8000
assert inc["profit_before_financing_and_taxes"] == Decimal("275000")
assert inc["net_income"] == Decimal("150000")  # item 21
assert inc["reconciliations"]["operating_profit"]["start"] == Decimal("266000")  # A − B
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `years` | One or more financial years, oldest first | required |
| `years[].label`, `years[].date` | Name of the year; balance sheet date (ISO 8601) | none; order taken as given, reported |
| `years[].statutory` | `layout` (echoed), `balance_sheet`, `income_statement` | give this or `reclassified` |
| `years[].reclassified` | `balance_sheet`, `income_statement` with the totals of Appendix B | give this or `statutory` |
| `years[].treatment_overrides` | Another destination, or a split by amounts, of B.III or shareholder loans for that year | none |
| `treatments` | Destinations stated once for the call | see [Treatments](#treatments) |
| `adjustments` | Labelled moves of Appendix C, one amount per year | none |

Amounts are in the currency units of the statements, never scaled, as presented in the
statutory layouts: zero or more, except the signed items (equity A.VI to A.IX and its total,
A.X zero or negative; A2 and A3, B11, C17-bis, items 20 and 21 and the totals that contain
them; the totals of the reclassified form defined as differences).

### Statutory balance sheet (Appendix A.1)

| Reference | Field | Destination | Choice |
| --- | --- | --- | --- |
| Assets A | `subscribed_capital_unpaid` (or within C.II); zero when not given | deducted from equity | fixed |
| B.I | `intangible_assets` (`total`, or `gross` and `accumulated`) | operating fixed assets | default |
| B.II | `tangible_assets` (`total`, or `gross` and `accumulated`) | operating fixed assets | default |
| B.III | `financial_fixed_assets` | operating, non-operating or financial asset | treatment |
| C.I | `inventories` | operating working capital | fixed |
| C.II | `receivables` (`beyond_one_year`, `trade`, and the parts within it) | operating working capital | default |
| C.III | `current_financial_assets` | net financial position, other financial asset | default |
| C.IV | `cash` | net financial position, cash | default |
| Assets D | `prepayments_and_accrued_income` (or within C.II) | operating working capital | default |
| Liabilities A | `equity` (`total` or A.I to A.X) | equity | fixed |
| B | `provisions` (`of_which_derivative_liabilities`, `beyond_one_year`) | operating provisions | default |
| B, derivatives | `provisions.of_which_derivative_liabilities`; none when not given | net financial position (non-current) or operating provisions | treatment |
| C | `employee_severance_indemnity` | operating provisions | default |
| D, financial debt | `payables.financial_debt` | net financial position, gross debt | fixed |
| D, shareholder loans | `payables.shareholder_loans` | gross debt or quasi-equity | treatment |
| D, remainder | `payables` less the parts above (`trade`) | operating working capital | default |
| E | `accruals_and_deferred_income` (or within D) | operating working capital | default |

The totals `fixed_assets` (B), `current_assets` (C), `total_assets` and
`total_liabilities_and_equity` are optional and checked; when given, the current and
non-current block uses them in place of components that are missing.

### Reclassified balance sheet (Appendix B.1)

`operating_fixed_assets`, `operating_working_capital` (`total` or `trade_receivables`,
`inventories`, `other_operating_assets`, `trade_payables`, `other_operating_liabilities`),
`operating_provisions`, `invested_capital`, `non_operating_assets` (labelled amounts),
`total_funds_invested`, `equity`, `quasi_equity_items`, `net_financial_position` (`total`,
`position` in the SPEC-0007 type, `current`, `non_current`), `debt_like_items` (the `dcf.fcff`
type, with `measurement_basis`) and `total_sources`. Every total its formula allows is derived
from what is given; a total given is checked against its derived value. The net financial
position total is, in this order, the total given, the total of `position`, or `current` +
`non_current`; in the last case the maturity relation is not checked, because it would compare
the total with itself. Treatments and adjustments do not apply to a reclassified year: give the
totals already adjusted.

The current and non-current block and the composition of the net financial position need the
statutory form: in a reclassified year they are listed in `not_computed` with no pointer to
give, and a convention says so. Inventories are taken from
`operating_working_capital.inventories`.

### Statutory income statement (Appendix A.2)

| Reference | Field | Layer | Choice |
| --- | --- | --- | --- |
| A1 | `revenue` | value of production | fixed |
| A2, A3 | `change_in_product_inventories` (signed, positive for an increase) | value of production | fixed |
| A4 | `own_work_capitalised` | value of production | fixed |
| A5 | `other_revenue_and_income` | value of production, core | default |
| B6, B11 | `purchases`, `change_in_raw_material_inventories` (signed, positive for a decrease) | value added (`materials_consumed`) | fixed |
| B7, B8 | `services`, `use_of_third_party_assets` | value added | fixed |
| B9 | `personnel_costs` | EBITDA | fixed |
| B10 a, b | `depreciation_and_write_downs.depreciation_and_amortisation` (or `value_adjustments_of_fixed_assets`, B10 a to c, less B10 c) | depreciation and amortisation | fixed |
| B10 c | `depreciation_and_write_downs.write_downs_of_fixed_assets` | depreciation and amortisation, or investing result with the treatment `investing` | default |
| B10 d | `depreciation_and_write_downs.write_downs_of_receivables` | other charges below EBITDA, or investing result with the treatment `investing` | default |
| B12, B13 | `provisions_for_risks`, `other_provisions` | operating profit | fixed |
| B14 | `other_operating_expenses` | value added, core | default |
| C15, C16 | `income_from_equity_investments`, `other_financial_income` | investing result | fixed |
| C17, C17-bis | `interest_and_other_financial_charges`, `exchange_gains_and_losses` (signed, positive for a net gain) | financing result | fixed |
| D18, D19 | `revaluations_of_financial_assets`, `write_downs_of_financial_assets` | investing result | fixed |
| 20, 21 | `income_taxes` (signed), `profit_or_loss_for_the_year` (signed) | taxes as given; check | fixed |

The totals `value_of_production` (A), `costs_of_production` (B), the B10 `total`,
`difference_between_value_and_costs_of_production` (A − B), `financial_income_and_charges`
(C), `value_adjustments_of_financial_assets` (D) and `profit_before_tax` are optional, signed
and checked. Total A stands in for its items in the value of production, and total D for D18
and D19 in the investing result, when their items are not given.

### Reclassified income statement (Appendix B.2)

`revenue`, `value_of_production`, `purchases`, `materials_consumed`, `value_added`,
`external_costs`, `personnel_costs`, `ebitda`, `depreciation_and_amortisation`,
`other_charges_below_ebitda`, `operating_profit`,
`investing_result`, `profit_before_financing_and_taxes`, `non_recurring_result`,
`financing_result`, `interest_and_other_financial_charges`, `profit_before_tax`,
`income_taxes` and `net_income`, all optional. EBITDA (value added − personnel costs), profit
before financing and taxes (operating profit + investing result), profit before tax and net
income are derived when not given and checked exactly when given, and so are value added
(value of production − `external_costs`) and operating profit (EBITDA − depreciation and
amortisation − `other_charges_below_ebitda`), with `external_costs` (signed) and
`other_charges_below_ebitda` (zero or more) defined as in the statutory form (E1, E2). The
reconciliations with A − B need the statutory form and are listed in `not_computed` with no
pointer to give.

## Treatments

Stated once in `treatments` and applied to every statutory year. A treatment is needed only
when its aggregate is given and not zero; when it is needed and missing, the outputs that depend
on the aggregate are not computed and `not_computed` names the treatment and its permitted
destinations.

| Treatment | Values | Default |
| --- | --- | --- |
| `financial_fixed_assets` | `operating`, `non_operating`, `financial_asset` | none |
| `shareholder_loans` | `financial_debt`, `quasi_equity` | none |
| `derivative_liabilities` | `financial_liability`, `operating_provision` | none |
| `write_downs_of_receivables` | `operating` (B10 d in the other charges below EBITDA), `investing` (B10 d in the investing result) | `operating`, listed when a statutory year gives B10 d |
| `write_downs_of_fixed_assets` | `operating` (B10 c with depreciation and amortisation), `investing` (B10 c in the investing result) | `operating`, listed when a statutory year gives B10 c |

`treatment_overrides` of a year accept only `financial_fixed_assets` and `shareholder_loans`,
as `{"destination": ...}` or `{"split": [{"destination", "amount", "label"}, ...]}` with parts
summing to that year's aggregate. A part of shareholder loans that is financial debt may give
its `beyond_one_year`, at most the part; a `quasi_equity` part cannot (`FM-E001`), because it
has no maturity in the net financial position. When every part is financial debt with its
`beyond_one_year`, the parts' amounts beyond one year equal the loans'; otherwise that relation
is not checked, but a sum already larger than the loans' is refused (`FM-E026`).

## Adjustments

Each adjustment is stated once with `kind`, `label`, `reason` (not blank) and `amounts`, one
entry per year in the order of `years` (`null` where it does not apply, reported). An amount
larger than what remains in its source is `FM-E026`.

| Kind | Source | Target | Fields |
| --- | --- | --- | --- |
| `non_operating_asset` | B.I, B.II, C.II or C.III | non-operating assets | `aggregate` |
| `financial_receivable` | C.II | net financial position, other financial asset | `maturity` |
| `operating_cash` | cash of the net financial position | operating working capital | none |
| `accrued_interest` | E | net financial position, other financial liability | none |
| `tax_instalments` | payables D (remainder) | debt-like items | `maturity` |
| `non_current_trade_payables` | payables D (remainder) | net financial position, other financial liability | none |
| `debt_like_provision` | B or C (operating) | debt-like items | `aggregate` |
| `dividends_declared` | equity | debt-like items | none |
| `investing` | A5 or B14 | investing result | `aggregate`; optional `related_non_operating_asset` (reported) |
| `non_recurring` | any income statement aggregate of A.2 except D18 and D19 | non-recurring result | `aggregate` |
| `lease` | double entry (below) | operating fixed assets, net financial position, equity; B8, B10, C17 | `amounts`: `asset`, `liability`, `liability_current`, `prepayment`, `payments`, `depreciation`, `interest` |

A `non_recurring` adjustment labels a part that is exceptional in size or incidence (art. 2427
n. 13, OIC 12): it leaves its aggregate and the subtotals above, and enters the non-recurring
result (componenti non ricorrenti). An `investing` adjustment moves income or charges of a
non-operating asset (rent of an investment property, for example) out of A5 or B14 into the
investing result.

A `lease` restates a lease by the financial method (OIC 12): the right-of-use asset
joins operating fixed assets; the liability joins gross debt of the net financial position
(decision P3; ESMA32-382-1138 paragraph 180), with its `liability_current` part current; the prepayment (maxicanone)
leaves assets D (the separate line when given, otherwise the part presented within C.II, which
then leaves C.II); asset − liability − prepayment is the equity effect, added to equity with
its formula in the explain. In the income statement the payments leave B8, the depreciation is
added to depreciation and amortisation and the interest to C17. No deferred taxes are computed,
so profit before tax and net income differ from the statutory amounts by payments −
depreciation − interest: payments and depreciation are lines of both reconciliations with A − B,
and interest is in the financing result. Each amount moved is listed in
`adjustments_applied` with its `line`.

## Outputs

Per year: `label`, `date`, `form`, `layout`, then:

| Field | Meaning |
| --- | --- |
| `balance_sheet.operating` | `operating_fixed_assets`, `operating_working_capital`, `trade_working_capital`, `operating_provisions`, `invested_capital` |
| `balance_sheet.non_operating_assets` | Labelled amounts in the `dcf.fcff` type, outside invested capital; with their total |
| `balance_sheet.total_funds_invested` | Invested capital + non-operating assets |
| `balance_sheet.sources` | `equity`, `quasi_equity_items`, `net_financial_position`, `debt_like_items` (at book value), `total_sources` |
| `balance_sheet.sources.net_financial_position` | `position` (SPEC-0007 type), `total`, `current`, `non_current`, `composition` by source |
| `balance_sheet.excess_cash` | C.IV less operating cash |
| `current_non_current` | `fixed_assets`, `current_assets`, `inventories`, `cash`, `total_assets` (less item A), `current_liabilities`, `non_current_liabilities`; from total B, total C and total assets when given |
| `income_statement` | The layers of Appendix B.2 (see [Formula](#income-statement-appendix-b2)), `ebitda_definition` and `reconciliations`; items named after a statutory item (`revenue`, `purchases`, `materials_consumed`, `personnel_costs`, `depreciation_and_amortisation`, `interest_and_other_financial_charges`) are core amounts, after the adjustments |
| `assignments` | Each aggregate given: reference, pointer, amount, destination, origin (`mapping` or `analyst`) |
| `adjustments_applied` | Each adjustment: kind, label, amount, reason, source, target; for a lease, one entry per amount with its `line` |
| `checks` | Each relation by id: `reconciled` or `not_checked`, with its pointers |
| `not_computed` | Per block, each missing output with the pointers of its missing inputs and, for a missing treatment, the permitted destinations; the entry has the same shape as in the `ratios.*` methods (`output`, `missing`, `permitted`) |

| Output | Feeds |
| --- | --- |
| `net_financial_position.position` | `net_financial_position` of `dcf.fcff` and `ratios.capital_structure` |
| `non_operating_assets` | `non_operating_assets` of `dcf.fcff` |
| `debt_like_items` | `debt_like_items` of `dcf.fcff` |
| `current_non_current` | `ratios.liquidity` and `ratios.capital_structure` |
| `invested_capital` | `invested_capital` of `ratios.profitability` |
| `operating_profit` | `ebit` of `dcf.free_cash_flows`; `ebit` of `ratios.profitability` for ROI on invested capital (non-operating assets enter the bridge at their value and are outside invested capital) |
| `profit_before_financing_and_taxes` | `ebit` of `ratios.debt_service` (interest covers) and of `ratios.profitability` for ROA on total assets |
| `depreciation_and_amortisation` | `depreciation_and_amortisation` of `dcf.free_cash_flows` |
| `revenue`, `ebitda` with `ebitda_definition`, `net_income` | `ratios.profitability`; `ebitda` also `ratios.debt_service` |
| `interest_and_other_financial_charges` | `interest` of `ratios.debt_service` |
| `purchases` | `purchases` of `ratios.working_capital`; `cost_of_sales` is not returned (Decision 13): an income statement by nature does not contain it |

## Defaults

Every default is listed in `meta.defaults_applied` and in `explain.conventions`, in this
order:

1. by year, each statutory aggregate given whose destination is a default of Appendix A (B.I,
   B.II, C.II, C.III, C.IV, assets D, provisions B, employee severance indemnity C, payables D,
   liabilities E; then A5 to `value_of_production`, B14 to `external_costs`, B10 c to
   `depreciation_and_amortisation` and B10 d to `other_charges_below_ebitda`, each while its
   treatment is not given): `field` is
   the pointer of the item, `value` the destination, and the convention reads "assigned to". An
   aggregate counts as given when its amount is: a node holding only a part of it (for example
   `receivables.trade`) does not;
2. `treatments.write_downs_of_receivables` and `treatments.write_downs_of_fixed_assets`,
   `operating` when not given, each listed only when a statutory year gives its aggregate (B10 d,
   B10 c; decision F1): otherwise no number depends on it;
3. each item taken as zero because it is not given ([Best effort](#best-effort)): `value` is
   `"0"` and the convention reads "not given, set to".

Fixed destinations are not defaults and are not listed.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Years | oldest first; dates checked when consecutive entries have them: the same month and day one calendar year later, 28 February after 29 February, or both month ends (28 February 2023, 29 February 2024) | order assumed and reported when a date is missing |
| Missing input | output null, listed in `not_computed` | never zero, never an error |
| Checks | exact, only when both sides are given | `not_checked` otherwise |
| Layout | `ordinary`, `abridged`, `micro_entity` | echoed; no number depends on it |
| Provisions B, employee severance indemnity C | operating provisions; debt-like by adjustment | default |
| Net financial position | C.III and all of C.IV inside; operating cash out by adjustment | default |
| Bridge items | carrying amount, `book_value` | fixed |
| Current and non-current | statutory amounts beyond one year of C.II and D, read as twelve months; provisions and the employee severance indemnity non-current, prepayments and accruals current, unless their part beyond one year is given; not changed by the adjustments | default, each reported |
| Item A not given | zero, the same amount for equity and total assets | default, listed (D1) |
| "Of which" parts not given | none within their aggregate, also when the aggregate is not given | default, listed (D2) |
| Derivatives | liabilities treated as financial: non-current in the net financial position; assets follow B.III (non-current) or C.III (current) | fixed, stated (D3) |
| A5, B14 | core; investing and non-recurring parts by adjustment | default, listed (Decision 7) |
| C15, C16, D18, D19 | investing result | fixed |
| C17, C17-bis | financing result | fixed |
| EBITDA | value added − personnel costs, returned as `ebitda_definition` | fixed (Decision 8) |
| Write-downs and revaluations | D18, D19 in the investing result; B10 c and B10 d `operating` (below EBITDA) or `investing` | `operating`, listed (Decision 17, E3, P1) |
| B10 a to c as one amount | with B10 c `investing`: depreciation and amortisation unless its B10 c part is given | fixed, stated (Decision 8) |
| Consistency of income and balance sheet | income and costs follow the classification of the related balance sheet item (decision P2) | not yet applied: see Limitations |
| Leases | as reported; financial method by adjustment, no deferred taxes | as reported (Decision 9) |
| Taxes | item 20 as given; no IRES, IRAP, deferred taxes or NOPAT | fixed |

## Checks

Each relation is checked exactly when both sides are given, and `FM-E026` gives both amounts,
the difference, the pointers and the fix when they differ. A side is built only from items the
user gave: item A or an "of which" part taken as zero never enters a check, and the message and
the `pointers` of a checked relation name only items given. A part presented within C.II or D
counts as given: its separate line is then zero, because the part is inside C or D.
Without total assets, for example, total assets rebuilt as A + B + C + D are compared when item
A is given and line D is given or presented within C.II.

Statutory form: B.I and B.II net = gross − accumulated; total B, total C, total equity, total
assets and total liabilities and equity against their items; total assets = total liabilities
and equity; parts beyond one year and "of which" parts within their aggregate; the profit of
A.IX = item 21; a split of an override = the year's aggregate; the maturities of a split of
shareholder loans (see [Treatments](#treatments)); the totals of the income statement (A, B,
B10 a to c, B10, A − B, C, D, profit before tax, item 21); B10 c within B10 a to c; the current
part of a lease liability
within the liability. Reclassified form: operating working capital, invested capital, total
funds invested, the net financial position (against its position and its maturities) and total
sources against their components, and total funds invested = total sources; EBITDA, profit
before financing and taxes, profit before tax and net income against their derivation by
Appendix B.2, and value added and operating profit against theirs.

The layers of the statutory income statement are built from the same items as its totals, so
profit before tax equals A − B ± C ± D and net income equals item 21, apart from the lease
restatements, and each reconciliation adds up exactly; a difference is a defect (`FM-E012`).

In the statutory form total funds invested equal the sources whenever every item is given,
because the identity follows from the relations above. When they differ, the items given do not
balance once the separate lines taken as zero are counted (item A, and lines D and E presented
within C.II or D): the result is `FM-E026` (maintainer approval of 2026-11-08), which names
only those lines and states the fix: an amount given differs, or one of these items taken as
zero is not zero. An "of which" part taken as none moves an amount within one side and is not named. With
every item given, a difference is a defect (`FM-E012`). This relation names no pointer in
`checks`: its sides are computed outputs, and its status counts the items taken as zero.

For statements filed in euro units, give the rounding reserve within other reserves (A.VI), so
that the totals reconcile exactly.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Both forms in one entry; dates not one year apart or newest first; a negative amount where the layout has none (a lease amount included); an override of another aggregate; `beyond_one_year` on a `quasi_equity` part of a split; an `amounts` list of another length; a blank label, reason or `related_non_operating_asset`; an adjustment on a reclassified entry; a `non_recurring` adjustment of D18 or D19; an `investing` adjustment of an aggregate other than A5 or B14; an unknown field or value |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |
| `FM-E009` | A date that does not exist or is outside 1900 to 2199 |
| `FM-E012` | Defect: total funds invested differ from the sources of a statutory year with every item given; a reconciliation with A − B does not add up; the reorganised profit differs from the statutory one other than by the lease restatements |
| `FM-E026` | A relation of the statements does not reconcile; the funds invested of a statutory year differ from its sources once the separate lines taken as zero are counted (named in the message); an adjustment exceeds what remains in its source; B10 c exceeds the B10 a to c group it belongs to; or the current part of a lease liability exceeds the liability |

## Limitations and practice notes

- **Deferred tax assets** (C.II 5-ter) stay in receivables, and so in operating working capital.
  To exclude them from working capital, move them with a `non_operating_asset` adjustment of
  `receivables` and state why in its reason.
- **Income of assets classified as operating** (decision P2 of 2026-11-09, not yet applied):
  income from financial fixed assets treated as `operating` (C15) stays in the investing result,
  and the revaluation of the employee severance indemnity treated as debt-like stays in
  personnel costs (B9) and EBITDA. The decision places them in operating profit and in the
  financing result; its implementation waits for the maintainer on four points.
- **Receivables within financial fixed assets** (B.III.2) follow the treatment of B.III and are
  always non-current in the current and non-current block, also for the part due within one
  year that art. 2424 asks to disclose.
- **Comparative years before 2016.** Before D.Lgs. 139/2015, own shares were presented as
  assets (B.III.4 or C.III.5) with a reserve in equity; from 2016 they are deducted from equity
  through the negative reserve A.X. Give a year before 2016 as restated in the comparative
  column of the following filing, or deduct the own shares from their asset aggregate and give
  them as A.X, so that the years compare.
- **Employee severance indemnity** (TFR, liabilities C) is an operating provision by default,
  as are provisions B; when the analyst treats it as debt, a `debt_like_provision` adjustment
  moves it, with its reason, to the debt-like items for the bridge.
- **Payables to group companies** (D.8 to D.11: to subsidiaries, associates, parents and sister
  companies) stay in the remainder of payables D, in operating working capital: this version
  does not separate their financial part (decision D6 of 2026-11-08; a separate issue). When
  part of them is financing, give it within `payables.financial_debt`, or move it with a
  `non_current_trade_payables` adjustment, and state why.
- **Exchange differences.** C17-bis is a net amount: a `non_recurring` adjustment of C17-bis
  moves a part of a net gain; a non-recurring exchange loss cannot be expressed in this version
  (decision E7; [issue #251](https://github.com/salvo8519/fairmark/issues/251)).
- **Leases** are restated without deferred taxes (Decision 9): the tax effect of the
  restatement stays in item 20 as reported.

## Accuracy

Additions and subtractions of decimals only, exact; no multiplication, division or rounding.

## Verified against

- Golden cases in `tests/golden/statements.reorganise/` (the ordinary and abridged layouts of
  one company with identical outputs, for the balance sheet and for both statements, a missing
  treatment, a few totals only, treatments and overrides over several years, one case per
  adjustment kind, subscribed capital unpaid, dividends declared, a lease restatement,
  write-downs with both treatments, a loss-making year, grouped depreciation, leap-year and
  month-end dates, the reclassified form of the same company, one `FM-E026` case per relation
  with its not-checked twin, and the input errors), whose expected values come from
  `scripts/reference_statements.py`, an exact decimal reference at 50 digits written from the
  appendices of SPEC-0005 without reading the engine; its income statement section was written
  before the engine's.
- `python/tests/test_reference_statements.py`: the engine against that reference on 400 seeded
  random calls in both forms, every output, reconciliation, check, default and `not_computed`
  entry, including items taken as zero, splits of shareholder loans with maturities,
  reclassified net financial positions given by their parts, every adjustment kind and the
  lease double entry.
- Rust properties: total funds invested equal the sources; profit before financing and taxes
  equals operating profit + investing result; profit before tax equals A − B ± C ± D; net
  income equals item 21; the reconciliations add up; the treatment `operating` changes neither
  EBITDA nor profit before financing and taxes.

## Sources

- Civil Code art. 2423-ter, 2424, 2425, 2435-bis and 2435-ter (layouts and comparatives);
  art. 2427, paragraph 1, n. 13 (items of exceptional size or incidence).
- IFRS 18 *Presentation and Disclosure in Financial Statements* (effective 2027): the subtotals
  "operating profit or loss" (paragraph 69(a)) and "profit or loss before financing and income
  taxes"; equity-accounted investments in the investing category (paragraphs 53(a), 54(a)).
- Banca d'Italia, financial stability analysis of firms, and EBA/GL/2020/06, Annex 3: they use
  MOL and EBITDA (the EBA lists EBITDA among the metrics for credit granting) but do not define
  them; the definition of EBITDA here is SPEC-0005 Decision 8.
- OIC 12 (presentation of the statements; leases and the financial method); D.Lgs. 139/2015
  (own shares deducted from equity from 2016).
- ESMA32-382-1138, guideline 39, paragraphs 175 to 186, and Consob, Richiamo di attenzione n.
  5/21 (net financial indebtedness; leases in financial debt, paragraph 180); see "Against the
  ESMA table" for where Fairmark differs.
- Koller, Goedhart, Wessels, *Valuation*, "Reorganizing the financial statements".
- SPEC-0005, its decisions 1 to 18, the maintainer's decisions D1 to D6 of 2026-11-08 and E1
  to E7, P1 to P5 of 2026-11-09;
  SPEC-0007 (net financial position); SPEC-0001 Decision 21 (bridge items and
  `MeasurementBasis`).
