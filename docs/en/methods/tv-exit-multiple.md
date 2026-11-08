# Terminal value, exit multiple (`tv.exit_multiple`)

Returns the terminal value at the horizon N as a market multiple of a metric of year N, as
if the business were sold at N. The metric sets the basis of the value: EV/EBITDA, EV/EBIT and
EV/Sales give an enterprise value; P/E and P/BV (price to book value of equity) give an
equity value. The two bases are never mixed.
Given the rate and the next flow on the same basis, the method also reports the perpetual
growth that the multiple implies, which makes a market assumption comparable with a growth
assumption.

## Formula

$$
TV_N = \text{multiple} \times \text{metric}_N,
\qquad
g_{\text{implied}} = r - \frac{CF_{N+1}}{TV_N}
$$

The implied growth is the growth at which [`tv.gordon`](tv-gordon.md) on $CF_{N+1}$ and $r$
returns the same terminal value. When the implied growth is not below the rate (a next flow
of zero or below), the method still returns the value and states in `explain.conventions`
that the multiple is inconsistent with a stable-growth value.

The multiple applies to the metric of year N (a trailing multiple), stated in
`explain.conventions`. The `book_value` metric is the book value of equity, with the multiple
P/BV on the equity basis (maintainer decision of 2026-11-08).

## Example

```python
from decimal import Decimal

import fairmark as fm

# 8 × EBITDA 125, WACC 10%, FCFF at N + 1 of 60: the multiple implies 4% growth forever
r = fm.tv.exit_multiple(8, "ebitda", 125, rate="0.10", next_flow=60, growth_cap="0.03")
assert r.terminal_value == Decimal("1000")
assert r.basis == "enterprise"
assert r.implied_growth == Decimal("0.04")
assert r.growth_cap["status"] == "exceeded"

# Fed back into the Gordon model, the implied growth returns the same value
g = fm.tv.gordon("0.10", r.implied_growth, next_flow=60)
assert g.terminal_value == r.terminal_value

# P/E: an equity value
pe = fm.tv.exit_multiple(12, "earnings", 50)
assert (pe.terminal_value, pe.basis) == (Decimal("600"), "equity")

# P/BV: 1.4 × book value of equity 750, cost of equity 9%, FCFE 63 at N + 1
pbv = fm.tv.exit_multiple("1.4", "book_value", 750, rate="0.09", next_flow=63)
assert (pbv.terminal_value, pbv.implied_growth) == (Decimal("1050"), Decimal("0.03"))
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `multiple` | The multiple, above 0 | required |
| `metric` | `ebitda`, `ebit`, `revenue` (enterprise basis), `earnings` (P/E) or `book_value` (P/BV, book value of equity), both equity basis | required |
| `metric_value` | The metric at N, in currency units, above 0 | required |
| `rate` | Discount rate on the basis of the metric (WACC or cost of equity), decimal fraction per period; with `next_flow` | none |
| `next_flow` | Flow at N + 1 on the basis of the metric (FCFF or FCFE), in currency units; with `rate` | none |
| `growth_cap` | Upper bound for the implied growth, decimal fraction per period; needs `rate` and `next_flow` | none, reported as absent |

## Outputs

| Field | Meaning |
| --- | --- |
| `terminal_value` | Terminal value at N, not discounted |
| `basis` | `enterprise` or `equity` |
| `implied_growth` | $r - CF_{N+1} / TV_N$; `null` without `rate` and `next_flow` |
| `growth_cap` | The implied growth against the cap; `null` without a cap |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Metric and basis | ebitda, ebit, revenue (enterprise); earnings, book_value (equity) | none: must be chosen |
| Timing of the metric | metric at N (trailing multiple) | fixed |
| Implied growth not below the rate | reported in `explain.conventions`, not refused | fixed |
| Rate and next flow of the implied growth | on the basis of the metric | not computed when absent |
| Growth cap | value or none | none, reported as absent |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Only one of `rate` and `next_flow` is given, `growth_cap` is given without them, the metric is unknown, or a field is missing or unknown |
| `FM-E018` | The multiple or the metric is zero or negative |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

One multiplication for the value, one division and one subtraction for the implied growth:
exact up to the 28 significant digits of the decimal type. Fed back into `tv.gordon`, the
implied growth returns the same terminal value to within $10^{-18}$ (the rounding of one
division).

## Verified against

- Golden cases in `tests/golden/tv.exit_multiple/`, from closed forms and an independent
  50-digit computation (`scripts/reference_tv.py`).
- Unit and Python tests feed the implied growth back into `tv.gordon` on several inputs.
