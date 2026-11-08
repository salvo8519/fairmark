# Terminal value, finite horizon (`tv.limited_horizon`)

Returns the terminal value at the horizon N of a flow that lasts a given number n of further
periods, followed by a residual value at N + n. Italian practice uses this form when the
competitive advantage, a concession or a licence has a limited life: instead of a perpetuity,
the flow is valued for n explicit periods. The flow is constant, or grows at a given rate; the
residual value is zero unless given. The value of the n flows is the flow times the annuity
factor of [`tvm.annuity`](tvm-annuity.md); only the factor is reused, so no value at N + n is
computed.

## Formula

With the first flow $CF_{N+1}$ at the end of period N + 1:

$$
TV_N = CF_{N+1}\,\frac{1 - \left(\dfrac{1 + g}{1 + r}\right)^{n}}{r - g}
+ \frac{RV_{N+n}}{(1 + r)^{n}}
$$

and $CF_{N+1}\,n / (1 + r)$ for the first term when $g = r$. As n grows the first term tends
to the perpetuity $CF_{N+1} / (r - g)$ when $g < r$.

## Example

```python
from decimal import Decimal

import fairmark as fm

# A licence: flow 100 for 10 more periods at 8%, nothing after
r = fm.tv.limited_horizon(100, "0.08", 10)
assert round(r.terminal_value, 10) == Decimal("671.0081398941")

# Growing flow, with a residual value of 500 at N + 5
g = fm.tv.limited_horizon(100, "0.10", 5, growth="0.02", residual_value=500)
assert round(g.residual_value_discounted, 10) == Decimal("310.4606615296")
print(g.conventions[3])  # residual value at N + 5: 500 (given)

# The longest horizon, 10,000 periods, equals the perpetuity 100 / 0.10
long = fm.tv.limited_horizon(100, "0.10", 10000)
assert long.terminal_value == Decimal("1000")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `flow` | First flow after the horizon, at the end of period N + 1, in currency units | required |
| `rate` | Discount rate of the terminal phase, decimal fraction per period, above −1 | required |
| `periods` | Number of further periods n, from 1 to 10,000 | required |
| `growth` | Growth of the flow per period, decimal fraction, above −1 | `0` (constant flow) |
| `residual_value` | Residual value at the end of period N + n, in currency units | `0` |
| `growth_cap` | Upper bound for the growth, decimal fraction per period | none, reported as absent |

## Outputs

| Field | Meaning |
| --- | --- |
| `terminal_value` | Terminal value at N, not discounted |
| `flows_value` | Value at N of the flows N + 1 to N + n |
| `annuity_factor` | Value at N of a first flow of 1 |
| `discount_factor` | $(1 + r)^{-n}$ |
| `residual_value_discounted` | Residual value brought to N |
| `growth_cap` | The growth (0 when not given) against the cap; `null` without a cap |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Flow profile | constant, or growing at `growth` | constant |
| Number of periods | explicit input | none: must be given |
| Timing | end of periods N + 1 to N + n; residual value at N + n | fixed |
| Residual value | given, or 0 | 0, reported as default |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | The growth is −100% or lower, or a field is missing or unknown |
| `FM-E003` | The rate is −100% or lower |
| `FM-E018` | `periods` is outside 1 to 10,000 (the same code for both ends of the range) |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

The value of the flows uses the annuity factor of [`tvm.annuity`](tvm-annuity.md) times the
flow; the residual value uses one discount factor. Measured against a term-by-term sum at 50
digits, the golden cases agree to $10^{-22}$, including a flow of $10^9$ over 600 periods. Every
n from 1 to 10,000 is accepted: no value at N + n is computed, so long horizons stay within the
decimal range. When $((1 + g)/(1 + r))^n$ falls below the decimal precision (about
$10^{-28}$), it is taken as 0, its limit, and an explain step says so; the value is then the
perpetuity to within that precision.

## Verified against

- Golden cases in `tests/golden/tv.limited_horizon/`, from an independent 50-digit
  computation that sums the discounted flows one by one (`scripts/reference_tv.py`), not the
  closed form used by the engine.
- Unit and Python tests check the convergence to the perpetuity for long horizons.
