# Annuity (`tvm.annuity`)

Returns the present value, the future value and the annuity factor of a series of $n$ periodic
payments, either level or growing at a constant rate, paid at the end of each period
(ordinary annuity) or at the beginning (annuity due). Use it for leases, instalment plans,
level royalties or any payment stream with a fixed number of periods.

## Formula

The payment of period $k$ is $P (1 + g)^{k - 1}$. For an ordinary annuity (payments at
$t = 1, \ldots, n$):

$$
PV_{\text{ordinary}} =
\begin{cases}
P \, \dfrac{1 - \left(\dfrac{1 + g}{1 + r}\right)^{n}}{r - g} & r \ne g \\[2ex]
P \, \dfrac{n}{1 + r} & r = g
\end{cases}
$$

An annuity due (payments at $t = 0, \ldots, n - 1$) is worth one period more:

$$
PV_{\text{due}} = PV_{\text{ordinary}} \,(1 + r)
\qquad
FV = PV \,(1 + r)^{n}
$$

The factor is the present value of a first payment of 1. With $g = 0$ the ordinary formula is
the familiar $P \left[1 - (1 + r)^{-n}\right] / r$.

When $r$ and $g$ differ by less than $10^{-9}$, the closed form loses digits to cancellation
in both numerator and denominator. The factor is then computed by summing the series directly:

$$
PV_{\text{ordinary}} = \frac{P}{1 + r} \sum_{k=1}^{n} \left(\frac{1 + g}{1 + r}\right)^{k - 1}
$$

## Example

A payment of 1,000 for 10 periods at 5%, ordinary and due, and a growing annuity of
20 payments starting at 1,000, growing at 3%, discounted at 8%:

```python
from decimal import Decimal

import fairmark as fm

ordinary = fm.tvm.annuity(1000, "0.05", 10, timing="ordinary")
due = fm.tvm.annuity(1000, "0.05", 10, timing="due")
growing = fm.tvm.annuity(1000, "0.08", 20, timing="ordinary", growth="0.03")

assert abs(ordinary.present_value - Decimal("7721.73492918481251282906")) < Decimal("1e-15")
assert abs(due.present_value - Decimal("8107.82167564405313847052")) < Decimal("1e-15")
assert abs(growing.present_value - Decimal("12250.04144458699209117333")) < Decimal("1e-15")
print(ordinary.conventions[-1])  # payment grows by 0 per period (default: level payments)

# Limit case rate = growth: n P / (1 + r).
limit = fm.tvm.annuity(1000, "0.06", 15, timing="ordinary", growth="0.06")
assert round(limit.present_value, 20) == Decimal("14150.94339622641509433962")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `payment` | First payment | required |
| `rate` | Discount rate per period, decimal fraction | required |
| `periods` | Number of payments, from 1 to 10000 | required |
| `growth` | Growth of the payment per period, decimal fraction, greater than −1 | `0` (level annuity) |
| `timing` | `ordinary` (end of each period) or `due` (beginning of each period) | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `present_value` | Present value at $t = 0$ |
| `future_value` | Value at $t = n$ of the same payments: present value $\times (1 + r)^n$ |
| `factor` | Present value of a first payment of 1 |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Timing | ordinary, due | none: must be chosen |
| Growth | any rate above −100% | 0, reported as default |
| Rate equal to growth | limit $n / (1 + r)$, shown as a separate step | applied when $r = g$ |
| Rate close to growth | direct summation, shown as a separate step | applied when $\lvert r - g \rvert < 10^{-9}$ |
| Future value date | $t = n$ for both timings | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `periods` is 0 or above 10000, `growth` is −100% or lower, or a field is missing or unknown |
| `FM-E003` | The rate is −100% or lower |
| `FM-E008` | Arithmetic overflow: inputs far outside a realistic range, or a future value beyond the decimal range at a long horizon (see [Long horizons](#long-horizons)) |

## Accuracy

The closed form and the direct sum use only integer powers, multiplication and division.
Measured against an independent 60-digit computation, they agree to about $10^{-24}$ relative
or better. The golden cases are checked to $10^{-19}$.

The annuity factor (the present value of a flow of 1 per period, growing at $g$) is shared
with [`tv.limited_horizon`](tv-limited-horizon.md), which multiplies it by the flow and does
not compute the future value: its range of periods is therefore not limited by the future
value at $t = n$.

### Long horizons

Every call computes the future value $PV \,(1 + r)^n$, and the result must fit the decimal range
(about $7.9 \times 10^{28}$). At long horizons it does not: a payment of 100 at 5% per period,
for example, returns `FM-E008` above 1,208 periods with ordinary timing and above 1,207 periods
with due timing. The present value is affected too: the call returns no result, so the present
value, which is finite, is not returned either. For the present value of a long ordinary
annuity, use [`tv.limited_horizon`](tv-limited-horizon.md), which computes the same factor and
never the future value (for the due timing, multiply its result by $1 + r$). Separating the two
values would change the outputs of the method and needs a new method version; it is recorded as
a follow-up in the plan of SPEC-0002.

## Verified against

- Golden cases in `tests/golden/tvm.annuity/`, from an independent 50-digit computation
  (`scripts/reference_time.py`).
- Level annuities agree with the spreadsheet `PV` and `FV` functions (Excel, LibreOffice) and
  with `numpy_financial.pv` and `numpy_financial.fv`, ordinary and due.
