# Beta from comparable companies (`coc.beta_peers`)

Estimates the beta of a private company from listed comparables: unlevers the observed beta of
each comparable at its own capital structure and tax rate, takes the mean or the median of the
unlevered betas, and relevers the selected statistic to the target structure of the company
being valued. The relevering formulas are those of [`coc.beta_relever`](coc-beta-relever.md).

## Formula

$$
\beta_{U,i} = \frac{\beta_{L,i} + \beta_D\, k_i}{1 + k_i},
\qquad
\bar\beta_U = \operatorname{mean}\ \text{or}\ \operatorname{median}(\beta_{U,i}),
\qquad
\beta_L = \bar\beta_U + (\bar\beta_U - \beta_D)\, k^{*}
$$

where $k_i = (1 - t_i)\,(D/E)_i$ (Hamada) or $(D/E)_i$ (Harris-Pringle) for comparable $i$, and
$k^{*}$ is the same factor at the target D/E and tax rate. For an even number of comparables
the median is the mean of the two central values.

## Example

Five comparables unlevered by Hamada; the median is relevered to D/E 0.5 at a 24% tax rate.

```python
from decimal import Decimal

import fairmark as fm

peers = [
    {"name": "Peer A", "levered_beta": "1.15", "debt_to_equity": "0.40", "tax_rate": "0.24"},
    {"name": "Peer B", "levered_beta": "0.95", "debt_to_equity": "0.20", "tax_rate": "0.25"},
    {"name": "Peer C", "levered_beta": "1.30", "debt_to_equity": "0.75", "tax_rate": "0.21"},
    {"name": "Peer D", "levered_beta": "1.05", "debt_to_equity": "0.35", "tax_rate": "0.24"},
    {"name": "Peer E", "levered_beta": "0.88", "debt_to_equity": "0.10", "tax_rate": "0.28"},
]
r = fm.coc.beta_peers(
    peers,
    formula="hamada",
    statistic="median",
    target_debt_to_equity="0.5",
    target_tax_rate="0.24",
)
# Median is Peer B: 0.95 / (1 + 0.75 × 0.2) = 0.95 / 1.15; relevered × (1 + 0.76 × 0.5)
assert r.median_unlevered_beta == Decimal("0.8260869565217391304347826087")
assert r.mean_unlevered_beta == Decimal("0.8349189472537530893029774557")
assert r.relevered_beta == Decimal("1.14")
print(r.conventions[-1])  # selected statistic: median (mean of the two central values ...)
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `peers` | Comparables, at least one: `{name, levered_beta, debt_to_equity, tax_rate}`; D/E not negative, tax rate from 0 to below 1; `name` is used in the audit trail only | required |
| `formula` | `hamada` or `harris_pringle`, used both to unlever and to relever | required |
| `statistic` | `mean` or `median` of the unlevered betas | required |
| `target_debt_to_equity` | Target D/E of the company being valued, decimal, not negative | required |
| `target_tax_rate` | Tax rate of the company being valued, decimal fraction from 0 to below 1 | required |
| `debt_beta` | Beta of debt, dimensionless, applied to comparables and target | `0` (risk-free debt) |

## Outputs

| Field | Meaning |
| --- | --- |
| `unlevered_betas` | Unlevered beta of each comparable, in input order |
| `mean_unlevered_beta` | Arithmetic mean of the unlevered betas |
| `median_unlevered_beta` | Median of the unlevered betas |
| `selected_unlevered_beta` | The statistic chosen by `statistic` |
| `relevered_beta` | Selected unlevered beta relevered to the target structure |

Both statistics are always returned, so the effect of the choice is visible. The steps show
each comparable by name, both statistics and the relevered beta.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Formula | Hamada, Harris-Pringle | none: must be chosen |
| Statistic | mean, median (mean of the two central values for an even count) | none: must be chosen |
| Debt beta | any value | 0, risk-free debt |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `peers` is empty, a D/E is negative, a tax rate is outside 0 to below 1, an option is not one of the listed ones, or a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

The formulas are rational: results are exact up to the 28 significant digits of the decimal
type. The golden case is checked to $3 \times 10^{-28}$.

## Verified against

- Golden case in `tests/golden/coc.beta_peers/`: closed form per comparable written out in the
  file, confirmed by an independent 50-digit computation (`scripts/reference_coc.py`).
- The Hamada and Harris-Pringle formulas in their commonly published form.
