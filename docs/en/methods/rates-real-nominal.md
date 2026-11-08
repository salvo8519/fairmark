# Real and nominal rates (`rates.real_nominal`)

Converts a nominal rate into a real rate, or a real rate into a nominal rate, given the
inflation over the same period. Fairmark uses the exact Fisher relation, not the additive
approximation $r \approx n - \pi$, which overstates the real rate by about $r \cdot \pi$ and
becomes material when inflation is high. Use it to keep cash flows and discount rates
consistent: real flows with a real rate, nominal flows with a nominal rate.

## Formula

The Fisher relation:

$$
(1 + n) = (1 + r)(1 + \pi)
$$

$$
r = \frac{1 + n}{1 + \pi} - 1
\qquad
n = (1 + r)(1 + \pi) - 1
$$

where $n$ is the nominal rate, $r$ the real rate and $\pi$ the inflation rate, all decimal
fractions for the same period.

## Example

A nominal rate of 8% with inflation of 3% is a real rate of $1.08 / 1.03 - 1 = 4.8544\ldots\%$,
not 5%. A real rate of 2% with inflation of 3% is a nominal rate of exactly 5.06%.

```python
from decimal import Decimal

import fairmark as fm

real = fm.rates.real_nominal("0.08", "0.03", "to_real")
assert round(real.rate, 20) == Decimal("0.04854368932038834951")

nominal = fm.rates.real_nominal("0.02", "0.03", "to_nominal")
assert nominal.rate == Decimal("0.0506")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `rate` | Rate to convert, decimal fraction per period (0.08 = 8%) | required |
| `inflation` | Inflation over the same period, decimal fraction (0.03 = 3%) | required |
| `direction` | `to_real` (the given rate is nominal) or `to_nominal` (the given rate is real) | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `rate` | The converted rate, decimal fraction per period |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Relation | exact Fisher relation, not the additive approximation | fixed |
| Period | rate and inflation for the same period and compounding | required of the input |

If the rate and the inflation do not refer to the same period, convert one of them first with
[`rates.convert`](rates-convert.md).

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing or unknown, or `direction` is not `to_real` or `to_nominal` |
| `FM-E003` | `rate` or `inflation` is −100% or lower; the message names the field (`inflation` is checked first) |

## Accuracy

The relation uses only multiplication, division and subtraction. `to_nominal` is exact;
`to_real` rounds once, in the final division. Measured against an independent 60-digit
computation, results agree to about $10^{-24}$ relative or better. The golden cases are
checked to $10^{-19}$.

## Verified against

- Golden cases in `tests/golden/rates.real_nominal/`, from an independent 50-digit
  computation (`scripts/reference_time.py`).
