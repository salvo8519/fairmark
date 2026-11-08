# Terminal value, capitalised normalised income (`tv.normalised_income`)

Returns the terminal value at the horizon N as a normalised income capitalised at a rate i,
as in the income-based reasoning of Italian practice: the income R that the business can earn
on a sustainable basis after the horizon, divided by the capitalisation rate, with or without
a perpetual growth. Estimating R and i is outside the method.

## Formula

$$
TV_N = \frac{R}{i}
\qquad\text{or, with a growth } g < i,\qquad
TV_N = \frac{R}{i - g}
$$

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.tv.normalised_income(120, "0.08")
assert r.terminal_value == Decimal("1500")
print(r.conventions[2])  # no growth: R / i (default)

grown = fm.tv.normalised_income(120, "0.09", growth="0.01", growth_cap="0.02")
assert grown.terminal_value == Decimal("1500")
assert grown.growth_cap["status"] == "respected"
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `income` | Normalised income R of period N + 1, in currency units | required |
| `capitalisation_rate` | Capitalisation rate i, decimal fraction per period, above 0 and above the growth | required |
| `growth` | Perpetual growth of the income, decimal fraction per period, above −1 and below i | none (R / i) |
| `growth_cap` | Upper bound for the growth, decimal fraction per period | none, reported as absent |

## Outputs

| Field | Meaning |
| --- | --- |
| `terminal_value` | Terminal value at N, not discounted |
| `growth_cap` | The growth (0 when not given) against the cap; `null` without a cap |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Growth | none (R / i), or given (R / (i − g)) | none, reported as default |
| Capitalisation rate | given | none: must be given |
| Growth cap | value or none | none, reported as absent |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | The growth is −100% or lower, or a field is missing or unknown |
| `FM-E004` | The growth is equal to or greater than the capitalisation rate |
| `FM-E018` | The capitalisation rate is zero or negative (checked first) |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

One division (and one subtraction with growth): exact up to the 28 significant digits of the
decimal type.

## Verified against

- Golden cases in `tests/golden/tv.normalised_income/`, from closed forms written in each file
  and an independent 50-digit computation (`scripts/reference_tv.py`).
