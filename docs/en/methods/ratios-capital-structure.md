# Capital structure ratios (`ratios.capital_structure`)

Answers the question "is the company financed soundly?". Returns debt to equity, net financial
position to equity, the equity ratio, the primary and secondary structure margins and the
primary and secondary fixed-asset coverage. The net financial position is given with its
composition, and every component is echoed in the audit trail.

## Formula

$$
\text{NFP} = \text{gross debt} - \text{cash} + \sum \text{other items}
$$

$$
\text{D/E} = \frac{\text{gross debt}}{\text{equity}}
\qquad
\text{NFP/E} = \frac{\text{NFP}}{\text{equity}}
\qquad
\text{equity ratio} = \frac{\text{equity}}{\text{total assets}}
$$

$$
\text{primary structure margin} = \text{equity} - \text{fixed assets}
$$

$$
\text{secondary structure margin} = \text{equity} + \text{non-current liabilities} - \text{fixed assets}
$$

$$
\text{primary coverage} = \frac{\text{equity}}{\text{fixed assets}}
\qquad
\text{secondary coverage} = \frac{\text{equity} + \text{non-current liabilities}}{\text{fixed assets}}
$$

A positive net financial position is net debt. Fairmark adds the items given and does not
decide which items belong to the net financial position.

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.capital_structure(
    balances="closing",
    closing={
        "equity": 2000,
        "total_assets": 6000,
        "fixed_assets": 3500,
        "non_current_liabilities": 1800,
        "net_financial_position": {
            "gross_debt": 2200,
            "cash": 300,
            "other_items": [{"name": "lease liabilities", "amount": 150}],
        },
    },
)
assert r.net_financial_position == Decimal("2050")  # 2200 − 300 + 150
assert r.debt_to_equity["value"] == Decimal("1.1")  # 2200 / 2000
assert r.nfp_to_equity["value"] == Decimal("1.025")  # 2050 / 2000
assert r.primary_structure_margin == Decimal("-1500")  # 2000 − 3500
assert r.secondary_structure_margin == Decimal("300")  # 2000 + 1800 − 3500
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `balances` | `closing`, or `average` of the opening and closing balance sheets | required |
| `closing` | `equity`, `total_assets`, `fixed_assets`, `non_current_liabilities`, `net_financial_position`, currency units | required |
| `opening` | Same fields at the start of the period | required with average balances, not used otherwise |

`net_financial_position` has `gross_debt`, `cash` and `other_items`, a list of
`{"name", "amount"}` added with their sign (positive for debt-like items, negative for financial
assets; default: none).

## Outputs

| Field | Meaning |
| --- | --- |
| `net_financial_position` | NFP used by the ratios, currency units |
| `debt_to_equity` | Gross debt / equity, multiple |
| `nfp_to_equity` | NFP / equity, multiple |
| `equity_ratio` | Equity / total assets, decimal fraction |
| `primary_structure_margin` | Equity − fixed assets, currency units |
| `secondary_structure_margin` | Equity + non-current liabilities − fixed assets, currency units |
| `primary_fixed_asset_coverage` | Equity / fixed assets, multiple |
| `secondary_fixed_asset_coverage` | (Equity + non-current liabilities) / fixed assets, multiple |

Ratios are objects `{"value": ..., "not_defined": ...}`, as in every ratio group.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing (applied to every balance, NFP included) | required |
| Net financial position | gross debt − cash + other items, positive for net debt | fixed |
| Not defined | zero denominator; equity zero or negative for D/E and NFP/E | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Average balances without `opening`; a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Margins are exact; each ratio is one decimal division, exact up to the 28 significant digits of
the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.capital_structure/` (closing and average balances,
  negative equity), whose expected values come from `scripts/reference_ratios.py` (Python
  decimal at 50 digits), with the closed form written in each file.
