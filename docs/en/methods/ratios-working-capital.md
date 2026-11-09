# Working capital ratios (`ratios.working_capital`)

Answers the question "how much does working capital tie up?". Returns days sales outstanding,
days inventory outstanding, days payable outstanding and the cash conversion cycle, on a day
basis chosen by the analyst.

## Formula

$$
\text{DSO} = \frac{\text{trade receivables} \times \text{days}}{\text{revenue}}
\qquad
\text{DIO} = \frac{\text{inventories} \times \text{days}}{\text{cost of sales}}
\qquad
\text{DPO} = \frac{\text{trade payables} \times \text{days}}{\text{purchases}}
$$

$$
\text{cash conversion cycle} = \text{DSO} + \text{DIO} - \text{DPO}
$$

where days is 360 or 365, as chosen in `day_basis`. The denominators follow the CFA Institute
curriculum (receivables on revenue, inventories on cost of sales, payables on purchases), which
is also common Italian practice.

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.working_capital(
    balances="closing",
    day_basis=365,
    income_statement={"revenue": 5000, "cost_of_sales": 3000, "purchases": 2800},
    closing={"trade_receivables": 1200, "inventories": 600, "trade_payables": 700},
)
assert r.days_sales_outstanding["value"] == Decimal("87.6")  # 1200 × 365 / 5000
assert r.days_inventory_outstanding["value"] == Decimal("73")  # 600 × 365 / 3000
assert r.days_payable_outstanding["value"] == Decimal("91.25")  # 700 × 365 / 2800
assert r.cash_conversion_cycle["value"] == Decimal("69.35")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `balances` | `closing`, or `average` of the opening and closing balance sheets | required |
| `day_basis` | `"360"` or `"365"` | required |
| `income_statement` | `revenue`, `cost_of_sales`, `purchases` of the year, currency units | each amount optional |
| `closing` | `trade_receivables`, `inventories`, `trade_payables`, currency units | each amount optional |
| `opening` | Same fields at the start of the period | used with average balances, not used otherwise; each amount optional |

Amounts are used as given: state trade receivables and revenue on the same VAT basis, and trade
payables and purchases likewise.

## Outputs

| Field | Meaning |
| --- | --- |
| `days_sales_outstanding` | Trade receivables × days / revenue, days |
| `days_inventory_outstanding` | Inventories × days / cost of sales, days |
| `days_payable_outstanding` | Trade payables × days / purchases, days |
| `cash_conversion_cycle` | DSO + DIO − DPO, days; not defined when one of the three is not, not computed when one of the three is not computed |
| `not_computed` | Outputs not computed because an input is missing, in the order of the outputs: `{"output", "missing", "reason"}`, with `output` the JSON pointer of the output in the result value and `missing` the sorted JSON pointers of the inputs it lacks; empty when every output is computed |

Ratios are objects `{"value": ..., "not_defined": ...}`, as in every ratio group.

## Partial input

Every amount is optional (version 2, SPEC-0007 R7). Give what the statements show: each output
is computed when its own amounts are given; otherwise it is null and listed in `not_computed`
with the JSON pointers of the inputs it lacks. A missing amount is never taken as zero and is
not an error by itself. The floor: at least one output must have every input it needs (a ratio
not defined by its own inputs counts); below it the input is refused with `FM-E027`, whose
message lists the alternative sets of inputs that would lift it, so a result made only of nulls
is never returned. With average balances an amount needs both `/closing/<field>`
and `/opening/<field>`. A given amount that already makes a ratio not defined (a zero
denominator, for example) decides first: the ratio is then not defined, with its reason, and not
listed.

| Output | Inputs it needs |
| --- | --- |
| `days_sales_outstanding` | trade receivables, revenue |
| `days_inventory_outstanding` | inventories, cost of sales |
| `days_payable_outstanding` | trade payables, purchases |
| `cash_conversion_cycle` | the three above |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing | required |
| Day basis | 360, 365 | required |
| DSO, DIO, DPO denominators | revenue, cost of sales, purchases | fixed |
| VAT | amounts used as given | fixed |
| Not defined | zero denominator | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `balances` or `day_basis` missing; a day basis other than 360 or 365; an unknown field; an amount that is not a decimal |
| `FM-E027` | Below the minimum input: no output has every input it needs; the message lists the alternative sets of inputs (any one is enough) |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Each ratio multiplies by the days before dividing once, exact up to the 28 significant digits of
the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.working_capital/` (closing balances on 365 and on 360
  days, average balances on 360 days, zero purchases), whose expected values come from
  `scripts/reference_ratios.py` (Python decimal at 50 digits), with the closed form written in
  each file.
- A golden case without purchases and a seeded random comparison on partial inputs, against
  `scripts/reference_ratios_partial.py`.
