# Terminal value, value driver (`tv.value_driver`)

Returns the terminal value at the horizon N when growth is earned by reinvestment: to grow at
g, a business that earns a return on new invested capital (RONIC) must reinvest g / RONIC of
its net operating profit after tax (NOPAT). The free cash flow is what remains. The method
makes value creation explicit: growth adds value only when RONIC exceeds the rate, and when
RONIC equals the rate the terminal value does not depend on g.

## Formula

For $-1 < g < r$ and $RONIC > 0$:

$$
\text{reinvestment rate} = \frac{g}{RONIC},
\qquad
TV_N = \frac{NOPAT_{N+1}\,\left(1 - \dfrac{g}{RONIC}\right)}{r - g}
$$

When $RONIC = r$ the formula reduces to $TV_N = NOPAT_{N+1} / r$; the method computes it
directly in that form, so the result is exactly independent of g. When RONIC is below g the
reinvestment rate exceeds 100% and the free cash flow at N + 1 is negative; the method returns
the value and states it in `explain.conventions`.

## Example

The table "The Big Assumption" in Damodaran's valuation lectures values a NOPAT of 100 at a
cost of capital of 10%; with growth 2% and a return on capital of 6% it shows 833:

```python
from decimal import Decimal

import fairmark as fm

r = fm.tv.value_driver(100, "0.02", "0.06", "0.10")
assert round(r.terminal_value) == 833
assert r.value_of_growth == "destroys"

# RONIC equal to the rate: growth creates no value, TV = NOPAT / r for any g
flat = fm.tv.value_driver(100, "0.03", "0.10", "0.10")
assert flat.terminal_value == Decimal("1000")
assert flat.value_of_growth == "none"

# NOPAT 150, growth 2%, RONIC 12%: one sixth of NOPAT is reinvested
up = fm.tv.value_driver(150, "0.02", "0.12", "0.10")
assert round(up.reinvestment_rate, 6) == Decimal("0.166667")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `nopat_next` | NOPAT of period N + 1, in currency units | required |
| `growth` | Perpetual growth, decimal fraction per period, above −1 and below `rate` | required |
| `ronic` | Return on new invested capital, decimal fraction per period, above 0 | required |
| `rate` | Discount rate of the terminal phase, decimal fraction per period | required |
| `growth_cap` | Upper bound for the growth, decimal fraction per period | none, reported as absent |

## Outputs

| Field | Meaning |
| --- | --- |
| `terminal_value` | Terminal value at N, not discounted |
| `reinvestment_rate` | g / RONIC |
| `next_free_cash_flow` | NOPAT_{N+1} × (1 − g / RONIC) |
| `value_of_growth` | `creates` (RONIC above the rate), `none` (equal) or `destroys` (below) |
| `growth_cap` | `{growth, cap, status}`; `null` without a cap |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Growth from reinvestment | reinvestment rate g / RONIC of NOPAT_{N+1} | fixed |
| Reinvestment rate above 100% | reported in `explain.conventions`, not refused | fixed |
| Rate of the terminal phase | given, may differ from the explicit phase | none: must be given |
| Growth cap | value or none | none, reported as absent |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | The growth is −100% or lower, or a field is missing or unknown |
| `FM-E004` | The growth is equal to or greater than the rate |
| `FM-E018` | RONIC is zero or negative |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Rational operations only: exact up to the 28 significant digits of the decimal type. The
reinvestment rate is rounded at the 28th digit when it is not a finite decimal (such as 1/6).

## Verified against

- Four cells of the table "The Big Assumption" (Damodaran, NYU Stern valuation lecture slides:
  NOPAT 100 in year n+1, cost of capital 10%) match at their published precision, including the
  column where the return on capital equals the cost of capital and the value is 1,000 at every
  growth (`tests/golden/tv.value_driver/`).
- An independent 50-digit computation (`scripts/reference_tv.py`).
