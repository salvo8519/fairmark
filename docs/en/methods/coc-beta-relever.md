# Beta relevering (`coc.beta_relever`)

Converts a beta between its unlevered (asset) form and its levered (equity) form at a given
capital structure. Use it to relever an industry or peer beta to the target debt-to-equity
ratio of the company being valued, or to unlever an observed beta.

## Formula

$$
\beta_L = \beta_U + (\beta_U - \beta_D)\, k
\qquad
\beta_U = \frac{\beta_L + \beta_D\, k}{1 + k}
$$

$$
k = (1 - t)\,\frac{D}{E}\ \text{(Hamada)}
\qquad
k = \frac{D}{E}\ \text{(Harris-Pringle)}
$$

where $\beta_L$ is the levered beta, $\beta_U$ the unlevered beta, $\beta_D$ the debt beta,
$D/E$ the debt-to-equity ratio and $t$ the tax rate of the tax shield. With $\beta_D = 0$ these
are the familiar $\beta_L = \beta_U [1 + (1 - t) D/E]$ and $\beta_L = \beta_U (1 + D/E)$.

## Choosing the formula

Hamada assumes debt fixed in amount, so the tax shield is as risky as the debt and reduces the
leverage effect by $(1 - t)$. Harris-Pringle assumes debt rebalanced to a target ratio, so the
tax shield carries the risk of the operating assets and the tax rate drops out. A positive
debt beta recognises that the debt of a leveraged company bears part of the business risk; it
lowers the levered beta for a given unlevered beta, and the default of 0 treats debt as
risk-free.

## Example

```python
from decimal import Decimal

import fairmark as fm

# Hamada, risk-free debt: 0.8 × (1 + (1 − 0.24) × 0.5)
r = fm.coc.beta_relever(
    "0.8", direction="relever", debt_to_equity="0.5", tax_rate="0.24", formula="hamada"
)
assert r.beta == Decimal("1.104")
print(r.conventions[1])  # debt beta 0 (default: risk-free debt)

# Harris-Pringle with a debt beta of 0.2: 0.8 + (0.8 − 0.2) × 0.5
hp = fm.coc.beta_relever(
    "0.8",
    direction="relever",
    debt_to_equity="0.5",
    tax_rate="0.24",
    formula="harris_pringle",
    debt_beta="0.2",
)
assert hp.beta == Decimal("1.1")

# Hamada unlevering with a debt beta of 0.1: 1.245 / 1.45
u = fm.coc.beta_relever(
    "1.2",
    direction="unlever",
    debt_to_equity="0.6",
    tax_rate="0.25",
    formula="hamada",
    debt_beta="0.1",
)
assert u.beta == Decimal("0.8586206896551724137931034483")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `beta` | Beta to convert: unlevered for `relever`, levered for `unlever`; dimensionless | required |
| `direction` | `relever` or `unlever` | required |
| `debt_to_equity` | Debt-to-equity ratio D/E, decimal (0.5 = debt equal to half of equity), not negative | required |
| `tax_rate` | Tax rate of the tax shield, decimal fraction from 0 to below 1 | required |
| `formula` | `hamada` or `harris_pringle` | required |
| `debt_beta` | Beta of debt, dimensionless | `0` (risk-free debt) |

The tax rate is the one the analyst considers deductible for the tax shield; the engine applies
no tax-specific logic (for Italian companies, whether IRAP is included is decided in the rate
passed). Harris-Pringle does not use it, but it is still validated and reported.

## Outputs

| Field | Meaning |
| --- | --- |
| `beta` | The converted beta: levered for `relever`, unlevered for `unlever` |

The steps show the leverage factor $k$ and the converted beta.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Formula | Hamada, Harris-Pringle | none: must be chosen |
| Debt beta | any value | 0, risk-free debt |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | D/E is negative, the tax rate is outside 0 to below 1, the formula or direction is not one of the listed options, or a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

The formulas are rational: results are exact up to the 28 significant digits of the decimal
type. Unlevering and then relevering at the same structure returns the original beta to
within $10^{-26}$ (property test, both formulas, with and without debt beta).

## Verified against

- Published worked examples (Damodaran, public NYU Stern lecture slides): Tata Motors, unlevered
  beta 0.8601 relevered at D/E 41.41% and tax 32.45% gives 1.1007; a median beta of 1.24 unlevered
  at gross D/E 27.06% and tax 40% gives 1.0668. Both are golden cases. The same slides state the
  version with a debt beta, algebraically identical to the one used here.

- Golden cases in `tests/golden/coc.beta_relever/`, whose expected values are closed-form
  computations written out in each file.
- The Hamada and Harris-Pringle formulas, with and without debt beta, in their commonly
  published form.
