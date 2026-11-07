# Cost of debt (`coc.cost_of_debt`)

Returns the pre-tax and the after-tax cost of debt. The pre-tax rate is either given directly
(for example the rate on the company's current borrowing) or built as a risk-free rate plus a
credit spread. The after-tax rate applies the tax shield at a tax rate given explicitly.

## Formula

$$
k_d = r_f + s \quad\text{(when built from a spread)}
\qquad
k_d^{\text{after tax}} = k_d \,(1 - t)
$$

where $r_f$ is the risk-free rate, $s$ the credit spread and $t$ the tax rate of the tax
shield. All rates are annual effective rates, decimal fractions.

## Example

```python
from decimal import Decimal

import fairmark as fm

# Risk-free rate 3% plus a spread of 2.5%, tax shield at 24%.
r = fm.coc.cost_of_debt("0.24", risk_free_rate="0.03", credit_spread="0.025")
assert r.pre_tax_cost_of_debt == Decimal("0.055")
assert r.after_tax_cost_of_debt == Decimal("0.0418")  # 0.055 × (1 − 0.24)
print(r.conventions[-1])  # tax rate of the tax shield 0.24

# The same pre-tax rate given directly.
direct = fm.coc.cost_of_debt("0.24", pre_tax_cost_of_debt="0.055")
assert direct.after_tax_cost_of_debt == Decimal("0.0418")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `tax_rate` | Tax rate of the tax shield, decimal fraction from 0 to below 1 | required |
| `pre_tax_cost_of_debt` | Pre-tax cost of debt, annual decimal fraction; give either this, or `risk_free_rate` and `credit_spread` | none |
| `risk_free_rate` | Risk-free rate, annual decimal fraction, with `credit_spread` | none |
| `credit_spread` | Credit spread over the risk-free rate, annual decimal fraction, with `risk_free_rate` | none |

The tax rate is the one the analyst considers deductible for the tax shield; the engine applies
no tax-specific logic (for Italian companies, whether IRAP is included is decided in the rate
passed). The rate is echoed in `explain.conventions`.

## Outputs

| Field | Meaning |
| --- | --- |
| `pre_tax_cost_of_debt` | Pre-tax cost of debt, annual decimal fraction |
| `after_tax_cost_of_debt` | $k_d (1 - t)$, annual decimal fraction |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Rates | annual effective, decimal fractions | fixed |
| Pre-tax cost of debt | given, or risk-free rate plus credit spread | none: one must be given |
| Tax rate of the tax shield | input | none: must be given |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Neither or both ways of giving the pre-tax rate are used, only one of `risk_free_rate` and `credit_spread` is given, the tax rate is outside 0 to below 1, or a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

The formulas use only additions and multiplications: results are exact up to the 28
significant digits of the decimal type.

## Verified against

- Golden case in `tests/golden/coc.cost_of_debt/`, whose expected values are closed-form
  computations written out in the file.
