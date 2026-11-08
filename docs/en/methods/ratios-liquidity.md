# Liquidity ratios (`ratios.liquidity`)

Answers the question "can the company pay in the short term?". Returns the current, quick and
cash ratios, the net working capital and the treasury margin.

## Formula

$$
\text{current ratio} = \frac{\text{current assets}}{\text{current liabilities}}
\qquad
\text{quick ratio} = \frac{\text{current assets} - \text{inventories}}{\text{current liabilities}}
\qquad
\text{cash ratio} = \frac{\text{cash}}{\text{current liabilities}}
$$

$$
\text{net working capital} = \text{current assets} - \text{current liabilities}
$$

$$
\text{treasury margin} = \text{current assets} - \text{inventories} - \text{current liabilities}
$$

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.liquidity(
    balances="closing",
    closing={
        "current_assets": 2500,
        "inventories": 900,
        "cash": 200,
        "current_liabilities": 2000,
    },
)
assert r.current_ratio["value"] == Decimal("1.25")  # 2500 / 2000
assert r.quick_ratio["value"] == Decimal("0.8")  # (2500 − 900) / 2000
assert r.treasury_margin == Decimal("-400")  # 2500 − 900 − 2000
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `balances` | `closing`, or `average` of the opening and closing balance sheets | required |
| `closing` | `current_assets` (inventories and cash included), `inventories`, `cash`, `current_liabilities`, currency units | required |
| `opening` | Same fields at the start of the period | required with average balances, not used otherwise |

## Outputs

| Field | Meaning |
| --- | --- |
| `current_ratio` | Current assets / current liabilities, multiple |
| `quick_ratio` | (Current assets − inventories) / current liabilities, multiple |
| `cash_ratio` | Cash / current liabilities, multiple |
| `net_working_capital` | Current assets − current liabilities, currency units |
| `treasury_margin` | Current assets − inventories − current liabilities, currency units |

Ratios are objects `{"value": ..., "not_defined": ...}`, as in every ratio group.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing | required |
| Not defined | zero current liabilities | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Average balances without `opening`; a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Margins are exact; each ratio is one decimal division, exact up to the 28 significant digits of
the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.liquidity/` (closing and average balances, zero
  current liabilities), whose expected values come from `scripts/reference_ratios.py` (Python decimal
  at 50 digits), with the closed form written in each file.
