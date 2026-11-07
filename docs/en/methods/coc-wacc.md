# Weighted average cost of capital (`coc.wacc`)

Returns the weighted average cost of capital (WACC) from the cost of equity, the pre-tax cost
of debt, the tax rate of the tax shield and the target capital structure. The structure is
given either as a debt-to-equity ratio D/E or as a debt weight D/(D + E). The result is the
discount rate of [`dcf.fcff`](dcf-fcff.md).

## Formula

$$
WACC = \frac{E}{V}\, k_e + \frac{D}{V}\, k_d \,(1 - t),
\qquad
\frac{D}{V} = \frac{D/E}{1 + D/E},
\qquad
\frac{E}{V} = 1 - \frac{D}{V}
$$

where $V = D + E$, $k_e$ is the cost of equity (see
[`coc.cost_of_equity`](coc-cost-of-equity.md)), $k_d$ the pre-tax cost of debt (see
[`coc.cost_of_debt`](coc-cost-of-debt.md)) and $t$ the tax rate of the tax shield. All rates
are annual effective rates, decimal fractions.

## Example

```python
from decimal import Decimal

import fairmark as fm

# Target debt weight 30%: 0.7 × 0.10 + 0.3 × 0.06 × (1 − 0.25)
r = fm.coc.wacc("0.10", "0.06", "0.25", debt_weight="0.3")
assert r.wacc == Decimal("0.0835")
assert r.equity_weight == Decimal("0.7")

# Target D/E 0.5, so weights 2/3 and 1/3: 2/3 × 0.0852 + 1/3 × 0.055 × (1 − 0.24)
by_ratio = fm.coc.wacc("0.0852", "0.055", "0.24", debt_to_equity="0.5")
assert by_ratio.wacc == Decimal("0.0707333333333333333333333333")
assert by_ratio.debt_weight == Decimal("0.3333333333333333333333333333")
assert by_ratio.after_tax_cost_of_debt == Decimal("0.0418")
```

Use the same target D/E here and when relevering the beta of the cost of equity, so that the
two inputs describe the same capital structure.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `cost_of_equity` | Cost of equity, annual decimal fraction | required |
| `pre_tax_cost_of_debt` | Pre-tax cost of debt, annual decimal fraction | required |
| `tax_rate` | Tax rate of the tax shield, decimal fraction from 0 to below 1 | required |
| `debt_to_equity` | Target D/E, decimal, not negative; give exactly one of `debt_to_equity` and `debt_weight` | none |
| `debt_weight` | Target D/(D + E), decimal fraction from 0 to below 1; give exactly one of `debt_to_equity` and `debt_weight` | none |

## Outputs

| Field | Meaning |
| --- | --- |
| `wacc` | Weighted average cost of capital, annual decimal fraction |
| `equity_weight` | E / (D + E) |
| `debt_weight` | D / (D + E) |
| `debt_to_equity` | D/E implied by the weights; compare it with the D/E used to relever the beta |
| `after_tax_cost_of_debt` | $k_d (1 - t)$, annual decimal fraction |

The steps show the debt weight (derived from D/E when given that way), the equity weight, the
after-tax cost of debt and the weighted sum.

## Consistency with the beta

Use the same capital structure in the cost of equity and in the weights: relever the beta at
the D/E implied by the WACC weights (returned as `debt_to_equity`). Target weights assume a
constant D/V, which matches Harris-Pringle; Hamada assumes a constant amount of debt. Mixing a
Hamada beta with target weights is a common, tolerated approximation; state it when it matters.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Rates | annual effective, decimal fractions | fixed |
| Weights | target capital structure, as D/E or D/(D + E) | none: one must be given |
| Tax rate of the tax shield | input, no tax-specific logic (IRAP included or not in the rate passed) | none: must be given |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Both or neither of `debt_to_equity` and `debt_weight` are given, D/E is negative, the debt weight is outside 0 to below 1, D/E is so large that the equity weight rounds to zero, the tax rate is outside 0 to below 1, or a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

The formula is rational: results are exact up to the 28 significant digits of the decimal
type. Weights derived from D/E that are not finite decimals (such as 1/3) are rounded at the
28th significant digit.

## Verified against

- Golden cases in `tests/golden/coc.wacc/`, whose expected values are closed-form computations
  written out in each file.
