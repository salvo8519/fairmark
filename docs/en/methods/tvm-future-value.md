# Future value (`tvm.future_value`)

Returns the value at the end of $n$ periods of a present amount and of a level series of
periodic payments, at a rate per period, with payments at the end of each period (ordinary) or
at the beginning (due). Use it for savings plans, sinking funds or the balance of a deposit
with regular withdrawals.

## Formula

$$
FV = PV\,(1 + r)^{n} + A\,\frac{(1 + r)^{n} - 1}{r}\,\times
\begin{cases}
1 & \text{ordinary} \\
(1 + r) & \text{due}
\end{cases}
\qquad
FV = PV + n\,A \text{ when } r = 0
$$

where $PV$ is the present amount and $A$ the level payment of every period.
The engine accumulates the annuity factor of [`tvm.annuity`](tvm-annuity.md) to $t = n$, so the
two methods share one implementation.

**Sign convention.** Amounts of the same sign accumulate in the same direction; a negative
amount is a withdrawal. This differs from the cash-flow sign convention of spreadsheets, where
a deposit is negative and the future value comes out positive.

The rate is per period, as in [`tvm.annuity`](tvm-annuity.md). For an annual rate, convert it
first with [`rates.convert`](rates-convert.md) and divide by the periods per year.

## Example

The ECMA-376 worked examples, with the signs of this convention:

```python
from decimal import Decimal

import fairmark as fm

savings = fm.tvm.future_value("0.01", 12, payment=1000, timing="ordinary")
assert round(savings.future_value, 2) == Decimal("12682.50")

plan = fm.tvm.future_value("0.005", 10, present_value=500, payment=200, timing="due")
assert round(plan.future_value, 2) == Decimal("2581.40")

# 10,000 deposited, 24 withdrawals of 500 at 0.4% a period: the deposit is overdrawn.
overdrawn = fm.tvm.future_value("0.004", 24, present_value=10000, payment=-500, timing="ordinary")
assert overdrawn.future_value < 0
print(overdrawn.conventions[1])  # sign convention: amounts of the same sign accumulate ...
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `present_value` | Amount at $t = 0$, currency units | `0` (at least one of the two amounts) |
| `payment` | Level payment of every period, currency units | `0` (at least one of the two amounts) |
| `rate` | Rate per period, decimal fraction, greater than −1 | required |
| `periods` | Number of periods, from 1 to 10000 | required |
| `timing` | `ordinary` (end of each period) or `due` (beginning of each period) | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `future_value` | Value at $t = n$ of the present amount and of the payments |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Timing | ordinary, due | none: must be chosen |
| Sign | same sign accumulates, negative is a withdrawal | fixed |
| Present value, payment | any amount | 0 each, reported as default; not both omitted |
| Rate zero | limit $PV + n\,A$, shown as a separate step | applied when $r = 0$ |
| Date of the future value | $t = n$ for both timings | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Both `present_value` and `payment` omitted, `periods` 0 or above 10000, or a field missing or unknown |
| `FM-E003` | The rate is −100% or lower |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Integer powers, multiplication and division only; the annuity factor divides by $1 + r$, so
values that are terminating decimals come out to the last of the engine's 28 significant
digits rather than exactly. The golden cases are checked to $10^{-20}$ against an independent
50-digit reference.

## Verified against

- Golden cases in `tests/golden/tvm.future_value/`: the ECMA-376 worked examples of the future
  value function and an independent 50-digit reference (`scripts/reference_loans.py`).
- The future value function of `numpy_financial` on random inputs, both timings, with the
  signs reversed.
