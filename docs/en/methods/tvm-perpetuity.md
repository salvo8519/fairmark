# Perpetuity (`tvm.perpetuity`)

Returns the present value of a payment that continues indefinitely, either level or growing at
a constant rate below the discount rate. The growing perpetuity is the Gordon growth model,
used for terminal values and for the value of stable dividend streams. Payments start at the end
of the first period (ordinary) or immediately (due).

## Formula

The payment of period $k$ is $P (1 + g)^{k - 1}$. For $-1 < g < r$:

$$
PV_{\text{ordinary}} = \frac{P}{r - g}
\qquad
PV_{\text{due}} = \frac{P}{r - g}\,(1 + r)
$$

With $g = 0$ the ordinary formula is $P / r$. When $g \ge r$ the series does not converge and
the method returns an error instead of a value. A growth of −100% or lower makes the payments
vanish or alternate in sign, so it is rejected as in [`tvm.annuity`](tvm-annuity.md).

## Example

A first payment of 100, growing at 2%, discounted at 8% (Gordon), and a level perpetuity due
of 100 at 8%:

```python
from decimal import Decimal

import fairmark as fm

gordon = fm.tvm.perpetuity(100, "0.08", timing="ordinary", growth="0.02")
assert round(gordon.present_value, 20) == Decimal("1666.66666666666666666667")

due = fm.tvm.perpetuity(100, "0.08", timing="due")
assert due.present_value == Decimal("1350")
print(due.conventions[-1])  # payment grows by 0 per period (default: level payments)

try:
    fm.tvm.perpetuity(100, "0.05", timing="ordinary", growth="0.05")
except fm.FairmarkError as err:
    assert err.code == "FM-E004"  # growth must be below the rate
    print(err.message)
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `payment` | First payment | required |
| `rate` | Discount rate per period, decimal fraction; must exceed `growth` | required |
| `growth` | Growth of the payment per period, decimal fraction, greater than −1 | `0` (level perpetuity) |
| `timing` | `ordinary` (first payment at $t = 1$) or `due` (first payment at $t = 0$) | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `present_value` | Present value at $t = 0$ |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Timing | ordinary, due | none: must be chosen |
| Growth | above −100% and below the rate | 0, reported as default |

The rate and the growth are per period and must use the same period and compounding.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `growth` is −100% or lower, a field is missing or unknown, or `timing` is not `ordinary` or `due` |
| `FM-E003` | The rate is −100% or lower |
| `FM-E004` | The growth is equal to or greater than the rate |

## Accuracy

The formula uses one division and, for the due timing, one multiplication. Measured against an
independent 60-digit computation, it agrees to about $10^{-24}$ relative or better. The golden
cases are checked to $10^{-19}$.

## Verified against

- Golden cases in `tests/golden/tvm.perpetuity/`, from an independent 50-digit computation
  (`scripts/reference_time.py`).
- The closed form is the Gordon growth model, the same formula used for the terminal value in
  [`dcf.fcff`](dcf-fcff.md).
