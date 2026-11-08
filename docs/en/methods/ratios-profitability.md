# Profitability ratios (`ratios.profitability`)

Answers the question "does the company earn enough on what it uses?". Returns ROE, ROI, ROS,
the EBITDA and EBIT margins, the capital turnover and the three-factor decomposition of ROE,
each with its formula and the numbers substituted in the audit trail. A ratio that cannot be
computed is reported as not defined with the reason, and the other ratios are still computed.

## Formula

$$
\text{ROE} = \frac{\text{net income}}{\text{equity}}
\qquad
\text{ROI} = \frac{\text{EBIT}}{\text{operating return base}}
\qquad
\text{ROS} = \frac{\text{EBIT}}{\text{revenue}}
$$

$$
\text{capital turnover} = \frac{\text{revenue}}{\text{operating return base}}
\qquad
\text{ROI} = \text{ROS} \times \text{capital turnover}
$$

$$
\text{EBITDA margin} = \frac{\text{EBITDA}}{\text{revenue}}
\qquad
\text{EBIT margin} = \frac{\text{EBIT}}{\text{revenue}}
$$

The three-factor (DuPont) decomposition:

$$
\text{ROE} = \underbrace{\frac{\text{net income}}{\text{revenue}}}_{\text{net margin}}
\times \underbrace{\frac{\text{revenue}}{\text{total assets}}}_{\text{asset turnover}}
\times \underbrace{\frac{\text{total assets}}{\text{equity}}}_{\text{equity multiplier}}
$$

The operating return base is the operating invested capital or the total assets, as chosen in
`operating_return_base`. Balance-sheet amounts are closing balances, or the average of opening
and closing balances, as chosen in `balances`.

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.profitability(
    balances="closing",
    operating_return_base="invested_capital",
    ebitda_definition="reported EBITDA, no adjustments",
    income_statement={"revenue": 5000, "ebitda": 800, "ebit": 500, "net_income": 260},
    closing={"equity": 2000, "total_assets": 6000, "invested_capital": 4000},
)
assert r.roe["value"] == Decimal("0.13")  # 260 / 2000
assert r.roi["value"] == Decimal("0.125")  # 500 / 4000
assert r.capital_turnover["value"] == Decimal("1.25")  # 5000 / 4000

# Negative equity: ROE is not defined, the other ratios are computed.
n = fm.ratios.profitability(
    balances="closing",
    operating_return_base="total_assets",
    ebitda_definition="reported EBITDA, no adjustments",
    income_statement={"revenue": 5000, "ebitda": 800, "ebit": 500, "net_income": 260},
    closing={"equity": -100, "total_assets": 6000},
)
assert n.roe["value"] is None
print(n.roe["not_defined"])  # equity is negative (-100)
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `balances` | `closing`, or `average` of the opening and closing balance sheets | required |
| `operating_return_base` | `invested_capital` or `total_assets`: the capital of ROI and of the capital turnover | required |
| `ebitda_definition` | How EBITDA was measured, in words, not empty; echoed in the conventions | required |
| `income_statement` | `revenue`, `ebitda`, `ebit`, `net_income` of the period, currency units | required |
| `closing` | `equity`, `total_assets` and, with an invested capital base, `invested_capital`, currency units | required |
| `opening` | Same fields at the start of the period | required with average balances, not used otherwise |

## Outputs

Every ratio is an object `{"value": ..., "not_defined": ...}`: `value` is null when the ratio
is not defined, and `not_defined` then gives the reason.

| Field | Meaning |
| --- | --- |
| `roe` | Net income / equity, decimal fraction |
| `roi` | EBIT / operating return base, decimal fraction |
| `ros` | EBIT / revenue, decimal fraction |
| `ebitda_margin` | EBITDA / revenue, decimal fraction |
| `ebit_margin` | EBIT / revenue, decimal fraction |
| `capital_turnover` | Revenue / operating return base, times per period |
| `roe_three_factor` | `net_margin`, `asset_turnover`, `equity_multiplier` and their product `roe` |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing | required |
| Operating return base | invested capital, total assets; an invested capital given with a total-assets base is reported as not used | required |
| EBITDA definition | as given by the analyst, non-empty | required |
| ROS | EBIT / revenue, equal to the EBIT margin | fixed |
| Three-factor ROE | on total assets (asset turnover and equity multiplier), whatever the operating return base | fixed |
| Not defined | zero denominator; equity zero or negative for ROE and the equity multiplier | fixed |

ROS is EBIT over revenue, so it equals the EBIT margin; both are returned so that the chain ROI
= ROS × capital turnover reads directly.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Average balances without `opening`; `invested_capital` missing with an invested capital base; an empty or blank `ebitda_definition`; a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Each ratio is one decimal division, exact up to the 28 significant digits of the decimal type.
The three-factor ROE is the product of the three rounded factors and can differ from the direct
ROE in the 28th digit.

## Verified against

- Golden cases in `tests/golden/ratios.profitability/` (closing and average balances on both
  operating return bases, negative equity, zero revenue), whose expected values come from `scripts/reference_ratios.py` (Python
  decimal at 50 digits), with the closed form written in each file.
