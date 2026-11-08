# Terminal value, H-model (`tv.h_model`)

Returns the terminal value at the horizon N when growth is still above its stable level at N
and declines linearly from a short-term growth $g_s$ to a long-term growth $g_l$ over 2H
periods, then stays at $g_l$ forever. The H-model (Fuller and Hsia) is a closed-form
approximation of that path: the value at stable growth plus the value of the extra growth
during the decline. Use it when a transition is already under way at N.

## Formula

For $g_l < r$ and $H \ge 0$:

$$
TV_N = \frac{CF_N\,(1 + g_l)}{r - g_l} + \frac{CF_N\,H\,(g_s - g_l)}{r - g_l}
= CF_N\,\frac{(1 + g_l) + H\,(g_s - g_l)}{r - g_l}
$$

With $g_s = g_l$, or $H = 0$, the second term is zero and the result equals
[`tv.gordon`](tv-gordon.md) on the last flow. The formula is an approximation, not the exact
value of a linearly declining growth path; the method states it in `explain.conventions`.

## Example

```python
from decimal import Decimal

import fairmark as fm

# CF_N 100, growth from 10% down to 3% over 2H = 10 periods, rate 9%
r = fm.tv.h_model(100, "0.10", "0.03", 5, "0.09")
assert r.terminal_value == Decimal("2300")
assert round(r.extra_growth_value, 10) == Decimal("583.3333333333")

# Equal growths: the H-model is the Gordon model on the last flow
same = fm.tv.h_model(100, "0.02", "0.02", 4, "0.09")
gordon = fm.tv.gordon("0.09", "0.02", last_flow=100)
assert same.terminal_value == gordon.terminal_value
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `last_flow` | Flow of period N, in currency units | required |
| `short_term_growth` | $g_s$, growth at the start of the decline, decimal fraction per period, above −1 | required |
| `long_term_growth` | $g_l$, stable growth, decimal fraction per period, above −1 and below `rate` | required |
| `half_life` | H, in periods, zero or positive, may be fractional; the decline lasts 2H periods | required |
| `rate` | Discount rate of the terminal phase, decimal fraction per period | required |
| `growth_cap` | Upper bound for the long-term growth, decimal fraction per period | none, reported as absent |

## Outputs

| Field | Meaning |
| --- | --- |
| `terminal_value` | Terminal value at N, not discounted |
| `stable_growth_value` | $CF_N (1 + g_l) / (r - g_l)$ |
| `extra_growth_value` | $CF_N H (g_s - g_l) / (r - g_l)$ |
| `growth_cap` | The long-term growth against the cap; `null` without a cap |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Growth path | linear decline over 2H periods, then $g_l$ (approximation) | fixed |
| Growth checked against the cap | the long-term growth $g_l$; $g_s$ is transient | fixed |
| Rate of the terminal phase | given | none: must be given |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A growth is −100% or lower, or a field is missing or unknown |
| `FM-E004` | The long-term growth is equal to or greater than the rate |
| `FM-E018` | The half-life is negative |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Rational operations only: exact up to the 28 significant digits of the decimal type. The
approximation error of the H-model itself, against an exact declining path, is a property of
the model and is not corrected.

## Verified against

- Golden cases in `tests/golden/tv.h_model/`, from an independent 50-digit computation
  (`scripts/reference_tv.py`) and the closed form written in each file.
- A unit test checks that equal growths give exactly the result of `tv.gordon`.
