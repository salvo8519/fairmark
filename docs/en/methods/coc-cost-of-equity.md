# Cost of equity (`coc.cost_of_equity`)

Returns the cost of equity by the capital asset pricing model (CAPM), with the premia that
practice adds for private companies: country risk, size and company-specific risk. Every
premium is an explicit input, and every premium, given or defaulted, is reported in the audit
trail. The levered beta is either given or relevered inside the method from an unlevered beta,
as in [`coc.beta_relever`](coc-beta-relever.md).

## Formula

$$
k_e = r_f + \beta_L \cdot ERP + CRP \cdot \lambda + SP + SRP
$$

where $r_f$ is the risk-free rate, $\beta_L$ the levered beta, $ERP$ the equity risk premium
of the market, $CRP$ the country risk premium, $\lambda$ the exposure of the company to the
country risk, $SP$ the size premium and $SRP$ the company-specific risk premium. All rates are
annual effective rates, decimal fractions.

When `relever` is given, the levered beta is

$$
\beta_L = \beta_U + (\beta_U - \beta_D)\, k,
\qquad
k = (1 - t)\,\frac{D}{E}\ \text{(Hamada)}
\quad\text{or}\quad
k = \frac{D}{E}\ \text{(Harris-Pringle)}
$$

## When to use each premium

- **Country risk premium** and **exposure** $\lambda$: when the equity risk premium refers to
  a mature market and the company earns its cash flows in a riskier one. $\lambda$ scales the
  premium to the share of the business exposed (for example the share of revenue earned in
  that country); the default of 1 means full exposure.
- **Size premium**: when the analyst holds that small companies earn returns above those
  explained by beta, typically taken from a published size study.
- **Specific risk premium**: for risks of the individual company not captured elsewhere, such
  as dependence on a few customers or on a key person. Because it is a judgement, a non-zero
  value requires a written justification, which is echoed in `explain.conventions`.

Each premium appears in the conventions as given or as the default 0, so a reader of the result
can see which premia were applied and which were left out.

## Example

Risk-free rate 3.5%, beta 1.1, equity risk premium 5.5%, country premium 2% with exposure 0.5,
size premium 2% and a justified specific premium of 1%; then an unlevered beta of 0.8 relevered
by Hamada at D/E 0.5 and tax rate 24%.

```python
from decimal import Decimal

import fairmark as fm

r = fm.coc.cost_of_equity(
    "0.035",
    "0.055",
    levered_beta="1.1",
    country_risk_premium="0.02",
    country_exposure="0.5",
    size_premium="0.02",
    specific_risk_premium="0.01",
    specific_risk_justification="Dependence on two customers for 60% of revenue",
)
# 0.035 + 1.1 × 0.055 + 0.02 × 0.5 + 0.02 + 0.01
assert r.cost_of_equity == Decimal("0.1355")
print(r.conventions[-1])  # specific risk premium justification: Dependence on two ...

relevered = fm.coc.cost_of_equity(
    "0.03",
    "0.05",
    relever={
        "unlevered_beta": "0.8",
        "debt_to_equity": "0.5",
        "tax_rate": "0.24",
        "formula": "hamada",
    },
)
# beta 0.8 × (1 + 0.76 × 0.5) = 1.104; ke 0.03 + 1.104 × 0.05 = 0.0852
assert relevered.levered_beta == Decimal("1.104")
assert relevered.cost_of_equity == Decimal("0.0852")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `risk_free_rate` | Risk-free rate, annual decimal fraction | required |
| `equity_risk_premium` | Equity risk premium of the market, annual decimal fraction | required |
| `levered_beta` | Levered beta, dimensionless; give exactly one of `levered_beta` and `relever` | none |
| `relever` | `{unlevered_beta, debt_to_equity, tax_rate, formula, debt_beta}` to relever an unlevered beta; `formula` is `hamada` or `harris_pringle`, `debt_beta` defaults to 0 | none |
| `country_risk_premium` | Country risk premium, annual decimal fraction, not negative | `0` |
| `country_exposure` | Exposure to the country risk ($\lambda$), dimensionless, not negative | `1` |
| `size_premium` | Size premium, annual decimal fraction | `0` |
| `specific_risk_premium` | Company-specific risk premium, annual decimal fraction | `0` |
| `specific_risk_justification` | Written reason for the specific premium; required when the premium is not zero, recorded in the conventions when given with a zero premium | none |

## Outputs

| Field | Meaning |
| --- | --- |
| `cost_of_equity` | Cost of equity, annual decimal fraction |
| `levered_beta` | Levered beta used, given or relevered |

The steps show the relevered beta (when computed), the market risk component
$\beta_L \cdot ERP$, the country risk component $CRP \cdot \lambda$ and the sum.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Rates | annual effective, decimal fractions | fixed |
| Beta | given, or relevered by Hamada or Harris-Pringle | none: one must be given |
| Debt beta (when relevering) | any value | 0, risk-free debt |
| Country risk premium, exposure | any value | 0 and 1 |
| Size premium | any value | 0 |
| Specific risk premium | any value, with a written justification when not zero | 0 |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Both or neither of `levered_beta` and `relever` are given; a non-zero specific premium has no justification; the country risk premium or the country exposure is negative; in `relever`, D/E is negative or the tax rate is outside 0 to below 1; a field is missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

The formula, relevering included, uses only additions and multiplications: results are exact
up to the 28 significant digits of the decimal type.

## Verified against

- Golden cases in `tests/golden/coc.cost_of_equity/`, whose expected values are closed-form
  computations written out in each file and checkable by hand.
- CAPM with additive premia and the relevering formulas in their commonly published form.
