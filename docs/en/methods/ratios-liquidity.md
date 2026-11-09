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
| `closing` | `current_assets` (inventories and cash included), `inventories`, `cash`, `current_liabilities`, currency units | each amount optional |
| `opening` | Same fields at the start of the period | used with average balances, not used otherwise; each amount optional |

## Outputs

| Field | Meaning |
| --- | --- |
| `current_ratio` | Current assets / current liabilities, multiple |
| `quick_ratio` | (Current assets − inventories) / current liabilities, multiple |
| `cash_ratio` | Cash / current liabilities, multiple |
| `net_working_capital` | Current assets − current liabilities, currency units |
| `treasury_margin` | Current assets − inventories − current liabilities, currency units |
| `not_computed` | Outputs not computed because an input is missing, in the order of the outputs: `{"output", "missing", "reason"}`, with `output` the JSON pointer of the output in the result value and `missing` the sorted JSON pointers of the inputs it lacks; empty when every output is computed |

Ratios are objects `{"value": ..., "not_defined": ...}`, as in every ratio group; the margins
are amounts, null when not computed.

## Partial input

Every amount is optional (version 2, SPEC-0007 R7). Give what the statements show: each output
is computed when its own amounts are given; otherwise it is null and listed in `not_computed`
with the JSON pointers of the inputs it lacks. A missing amount is never taken as zero, and
missing data is never an error. With average balances an amount needs both `/closing/<field>`
and `/opening/<field>`. A given amount that already makes a ratio not defined (a zero
denominator, for example) decides first: the ratio is then not defined, with its reason, and not
listed.

| Output | Inputs it needs |
| --- | --- |
| `current_ratio`, `net_working_capital` | current assets, current liabilities |
| `quick_ratio`, `treasury_margin` | current assets, inventories, current liabilities |
| `cash_ratio` | cash, current liabilities |

```python
r = fm.ratios.liquidity(
    balances="closing",
    closing={"current_assets": 2500, "cash": 200, "current_liabilities": 2000},
)
assert r.current_ratio["value"] == Decimal("1.25")
assert r.quick_ratio["value"] is None  # inventories not given
assert r.not_computed[0] == {
    "output": "/quick_ratio",
    "missing": ["/closing/inventories"],
    "reason": "an input it needs is not given; a missing input is never taken as zero",
}
```

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing | required |
| Not defined | zero current liabilities | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `balances` missing or not `closing` or `average`; an unknown field; an amount that is not a decimal |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Margins are exact; each ratio is one decimal division, exact up to the 28 significant digits of
the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.liquidity/` (closing and average balances, zero
  current liabilities), whose expected values come from `scripts/reference_ratios.py` (Python decimal
  at 50 digits), with the closed form written in each file.
- Golden cases with partial input (inventories not given, an opening amount not given), and a
  seeded random comparison on partial inputs, against `scripts/reference_ratios_partial.py`.
