# Financial statements reorganised (`statements.reorganise`)

Reorganises the financial statements of one or more financial years into what the business
uses (operating), what it holds outside its business (non-operating) and how it is financed.
Each year is given either as the aggregates of the statutory layouts (art. 2424 and 2425 of the
Italian Civil Code, ordinary or abridged, art. 2435-bis) or as totals already reclassified, for
example by a bank. The result is in the shapes that [`dcf.fcff`](dcf-fcff.md) and the
`ratios.*` groups take, unchanged.

This build returns the reorganised **balance sheet**: invested capital, non-operating assets,
equity, quasi-equity, the net financial position with its composition and maturities, debt-like
items, and current and non-current amounts. The income statement inputs are read, validated and
checked against their totals; their reorganisation is not returned yet (see the
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
- gross debt = financial debt + shareholder loans treated as financial debt; cash = C.IV −
  operating cash; other items, each with its sign: C.III, financial receivables, B.III treated
  as a financial asset (negative), derivative liabilities, accrued interest and trade payables
  with extended terms (positive);
- excess cash = C.IV − operating cash, the cash of the net financial position.

Derivatives (decision D3): derivative liabilities within provisions B treated as
`financial_liability` are non-current in the net financial position. Derivative assets are not
given separately: those within B.III (B.III.4) follow the treatment of B.III and are
non-current; those within C.III (C.III.5) are current financial assets of the net financial
position.

A positive net financial position is net debt. Its total is computed by the function of the
ratios (SPEC-0007), so it equals the total of the object returned.

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
| `write_downs_of_receivables` | `investing`, `operating` (used by the income statement) | `investing` |

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

The kinds `lease`, `investing` and `non_recurring` adjust the income statement; this build
refuses them with `FM-E001` rather than ignore them.

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
| `assignments` | Each aggregate given: reference, pointer, amount, destination, origin (`mapping` or `analyst`) |
| `adjustments_applied` | Each adjustment: kind, label, amount, reason, source, target |
| `checks` | Each relation by id: `reconciled` or `not_checked`, with its pointers |
| `not_computed` | Per block, each missing output with the pointers of its missing inputs and, for a missing treatment, the permitted destinations |

| Output | Feeds |
| --- | --- |
| `net_financial_position.position` | `net_financial_position` of `dcf.fcff` and `ratios.capital_structure` |
| `non_operating_assets` | `non_operating_assets` of `dcf.fcff` |
| `debt_like_items` | `debt_like_items` of `dcf.fcff` |
| `current_non_current` | `ratios.liquidity` and `ratios.capital_structure` |
| `invested_capital` | `invested_capital` of `ratios.profitability` |

## Defaults

Every default is listed in `meta.defaults_applied` and in `explain.conventions`, in this
order:

1. each statutory aggregate given whose destination is a default of Appendix A (B.I, B.II,
   C.II, C.III, C.IV, assets D, provisions B, employee severance indemnity C, payables D,
   liabilities E): `field` is the pointer of the item, `value` the destination, and the
   convention reads "assigned to". An aggregate counts as given when its amount is: a node
   holding only a part of it (for example `receivables.trade`) does not;
2. `treatments.write_downs_of_receivables`, `investing` when not given;
3. each item taken as zero because it is not given ([Best effort](#best-effort)): `value` is
   `"0"` and the convention reads "not given, set to".

Fixed destinations are not defaults and are not listed. The default destinations of the income
statement (A5, B14, B10 d) are listed from the build that reorganises it.

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
B10, A − B, C, D, profit before tax, item 21). Reclassified form: operating working capital,
invested capital, total funds invested, the net financial position (against its position and
its maturities) and total sources against their components, and total funds invested = total
sources.

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
| `FM-E001` | Both forms in one entry; dates not one year apart or newest first; a negative amount where the layout has none; an override of another aggregate; `beyond_one_year` on a `quasi_equity` part of a split; an `amounts` list of another length; a blank label or reason; an adjustment on a reclassified entry; an income statement adjustment in this build; an unknown field or value |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |
| `FM-E009` | A date that does not exist or is outside 1900 to 2199 |
| `FM-E012` | Defect: total funds invested differ from the sources of a statutory year with every item given |
| `FM-E026` | A relation of the statements does not reconcile; the funds invested of a statutory year differ from its sources once the separate lines taken as zero are counted (named in the message); or an adjustment exceeds its source |

## Limitations and practice notes

- **Deferred tax assets** (C.II 5-ter) stay in receivables, and so in operating working capital.
  Practitioners often exclude them from working capital (Borsa Italiana, *Guida alla
  Valutazione*); to do so, move them with a `non_operating_asset` adjustment of `receivables`
  and state why in its reason.
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

## Accuracy

Additions and subtractions of decimals only, exact; no multiplication, division or rounding.

## Verified against

- Golden cases in `tests/golden/statements.reorganise/` (the ordinary and abridged layouts of
  one company with identical outputs, a missing treatment, treatments and overrides over several
  years, one case per balance sheet adjustment kind, subscribed capital unpaid, dividends
  declared, leap-year and month-end dates, the reclassified form of the same company, one
  `FM-E026` case per relation with its not-checked twin, and the input errors), whose expected
  values come from `scripts/reference_statements.py`, an exact decimal reference at 50 digits
  written from the appendices of SPEC-0005 without reading the engine.
- `python/tests/test_reference_statements.py`: the engine against that reference on 400 seeded
  random calls in both forms, every output, check, default and `not_computed` entry, including
  items taken as zero, splits of shareholder loans with maturities and reclassified net
  financial positions given by their parts.

## Sources

- Civil Code art. 2423-ter, 2424, 2425, 2435-bis and 2435-ter (layouts and comparatives).
- OIC 12 (presentation of the statements); D.Lgs. 139/2015 (own shares deducted from equity
  from 2016).
- Borsa Italiana, *Guida alla Valutazione* (treatment of deferred tax assets in practice).
- ESMA32-382-1138, guideline 39, and Consob, Richiamo di attenzione n. 5/21 (net financial
  indebtedness).
- Koller, Goedhart, Wessels, *Valuation*, "Reorganizing the financial statements".
- SPEC-0005, its decisions 1 to 18 and the maintainer's decisions D1 to D6 of 2026-11-08;
  SPEC-0007 (net financial position); SPEC-0001 Decision 21 (bridge items and
  `MeasurementBasis`).
