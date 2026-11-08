# Rate conversion (`rates.convert`)

Converts an annual rate quoted with one compounding convention into the equivalent annual rate
quoted with another. Two rates are equivalent when they give the same growth of one unit over
one year. Use it to reconcile a monthly-compounded loan rate with an effective annual cost of
capital, or to move between periodic and continuously compounded rates before discounting.

## Formula

The growth of one unit over one year at a rate $r$ with $m$ compounding periods per year is

$$
G = \left(1 + \frac{r}{m}\right)^{m}
\qquad\text{or}\qquad
G = e^{r}\ \text{(continuous compounding)}
$$

The equivalent rate with target compounding $m'$ is

$$
r' = m' \left(G^{1/m'} - 1\right)
\qquad\text{or}\qquad
r' = \ln G\ \text{(continuous compounding)}
$$

For an annual target ($m' = 1$) this is simply $r' = G - 1$.

| Compounding | `from` / `to` value | $m$ |
| --- | --- | --- |
| Annual (effective annual rate) | `annual` | 1 |
| Semi-annual | `semi_annual` | 2 |
| Four-monthly (every four months) | `four_monthly` | 3 |
| Quarterly | `quarterly` | 4 |
| Bimonthly (every two months, not twice a month) | `bimonthly` | 6 |
| Monthly | `monthly` | 12 |
| Continuous | `continuous` | none, growth $e^{r}$ |

The periodic values are the named frequencies shared with the loan payment frequency, defined
once in the engine.

## Example

A rate of 12% a year compounded monthly is an effective annual rate of
$(1 + 0.12/12)^{12} - 1 = 12.6825\ldots\%$. The calculation uses integer powers only, so the
result is exact to the 28 digits of the decimal type.

```python
from decimal import Decimal

import fairmark as fm

r = fm.rates.convert("0.12", "monthly", "annual")
print(r.rate)                  # 0.126825030131969720661201
print(r.annual_growth_factor)  # 1.126825030131969720661201
assert r.rate == Decimal("1.01") ** 12 - 1

# 10% effective annual as a continuously compounded rate: ln(1.1).
c = fm.rates.convert("0.10", "annual", "continuous")
assert abs(c.rate - Decimal("0.09531017980432486004")) < Decimal("1e-15")
```

The Python argument for the source compounding is `from_` (with a trailing underscore,
because `from` is a Python keyword); in JSON, the CLI and the MCP server the field is `from`.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `rate` | Annual rate as a decimal fraction (0.12 = 12% a year), quoted with `from` compounding | required |
| `from` | Compounding of the given rate | required |
| `to` | Compounding of the rate to return | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `rate` | Equivalent annual rate with `to` compounding, decimal fraction |
| `annual_growth_factor` | Growth of one unit over one year, identical for both rates |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Source compounding (`from`) | annual, semi-annual, four-monthly, quarterly, bimonthly, monthly, continuous | none: a rate without its compounding is ambiguous |
| Target compounding (`to`) | same options | none |
| Quotation | rates are always per year: a monthly-compounded rate of 0.12 means 1% a month | fixed |

When `from` and `to` are equal the rate is returned unchanged.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing or unknown, or a compounding is not one of the seven options |
| `FM-E003` | The rate per compounding period, $r/m$, is −100% or lower; the message names `/rate` and, with more than one compounding period a year, the bound $-m$ |
| `FM-E008` | The exponential or logarithm overflows (rates far outside a realistic range) |

## Accuracy

Conversion to an annual rate from a periodic rate uses exact integer powers; measured against
an independent 60-digit computation, it agrees to about $10^{-24}$ relative or better.
Conversions that involve continuous compounding or a non-annual target use the decimal
exponential and logarithm, which agree to about $10^{-18}$ relative or better on typical
inputs. A conversion followed by its inverse returns the original rate to within $10^{-20}$.
The golden cases are checked to $10^{-19}$.

## Verified against

- Golden cases in `tests/golden/rates.convert/`, from an independent 50-digit computation
  (`scripts/reference_time.py`), including 7.35% effective annual converted to `bimonthly` and
  to `four_monthly`, and the refusal of a rate of −3 compounded every four months ($r/m = -1$).
- Conversions between a periodic compounding and `annual` agree with the spreadsheet `EFFECT`
  and `NOMINAL` functions (Excel, LibreOffice).
