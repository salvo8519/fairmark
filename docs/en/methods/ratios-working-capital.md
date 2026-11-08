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
| `income_statement` | `revenue`, `cost_of_sales`, `purchases` of the year, currency units | required |
| `closing` | `trade_receivables`, `inventories`, `trade_payables`, currency units | required |
| `opening` | Same fields at the start of the period | required with average balances, not used otherwise |

Amounts are used as given: state trade receivables and revenue on the same VAT basis, and trade
payables and purchases likewise.

## Outputs

| Field | Meaning |
| --- | --- |
| `days_sales_outstanding` | Trade receivables × days / revenue, days |
| `days_inventory_outstanding` | Inventories × days / cost of sales, days |
| `days_payable_outstanding` | Trade payables × days / purchases, days |
| `cash_conversion_cycle` | DSO + DIO − DPO, days; not defined when one of the three is not |

Ratios are objects `{"value": ..., "not_defined": ...}`, as in every ratio group.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing | required |
| Day basis | 360, 365 | required |
| VAT | amounts used as given | fixed |
| Not defined | zero denominator | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Average balances without `opening`; a day basis other than 360 or 365; a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Each ratio multiplies by the days before dividing once, exact up to the 28 significant digits of
the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.working_capital/` (closing balances on 365 days, average
  balances on 360 days, zero purchases), whose expected values come from
  `scripts/reference_ratios.py` (Python decimal at 50 digits), with the closed form written in
  each file.
