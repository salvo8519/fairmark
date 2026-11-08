# Terminal value, stable growth (`tv.gordon`)

Returns the terminal value at the horizon N of a flow that grows at a constant rate g forever,
discounted at the rate of the terminal phase r. This is the Gordon growth model applied at the
end of the explicit forecast. The result is a value at N: discounting it to the valuation
date is the job of the DCF. The flow of the first period after the horizon is either given
(for example a normalised flow) or obtained by growing the last explicit flow by g; the choice
is required and reported.

## Formula

For $-1 < g < r$:

$$
TV_N = \frac{CF_{N+1}}{r - g},
\qquad
CF_{N+1} = CF_N\,(1 + g) \text{ when the last flow is given}
$$

When $g \ge r$ the series does not converge and the method returns an error instead of a
value.

## Example

```python
from decimal import Decimal

import fairmark as fm

# Next flow 50, rate 8%, growth 4%, checked against a 2% cap (the cap is reported, not refused)
r = fm.tv.gordon("0.08", "0.04", next_flow=50, growth_cap="0.02")
assert r.terminal_value == Decimal("1250")
assert r.flow_basis == "next_flow_given"
assert r.growth_cap["status"] == "exceeded"

# Last flow 100 grown by 2.5% to 102.5, rate 9%
last = fm.tv.gordon("0.09", "0.025", last_flow=100)
assert last.next_flow == Decimal("102.5")
print(last.conventions[1])  # flow at N + 1 is the last flow grown by g: CF_N × (1 + g)
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `rate` | Discount rate of the terminal phase, decimal fraction per period; may differ from the explicit phase | required |
| `growth` | Perpetual growth, decimal fraction per period, above −1 and below `rate` | required |
| `next_flow` | Flow at N + 1, in currency units; give this or `last_flow` | none |
| `last_flow` | Flow at N, in currency units, grown by `growth`; give this or `next_flow` | none |
| `growth_cap` | Upper bound for the growth, decimal fraction per period (for example the risk-free rate or the nominal growth of the economy) | none, reported as absent |

## Outputs

| Field | Meaning |
| --- | --- |
| `terminal_value` | Terminal value at N, not discounted |
| `next_flow` | Flow at N + 1 used |
| `flow_basis` | `next_flow_given` or `last_flow_grown` |
| `growth_cap` | `{growth, cap, status}` with status `respected` (g ≤ cap) or `exceeded`; `null` without a cap |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Flow at N + 1 | given, or CF_N × (1 + g) | none: one must be chosen |
| Rate of the terminal phase | given, may differ from the explicit phase | none: must be given |
| Growth cap | value or none | none, reported as absent |
| Timing | value at N of flows from N + 1 onwards, end of period | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Both or neither of `next_flow` and `last_flow` are given, the growth is −100% or lower, or a field is missing or unknown |
| `FM-E004` | The growth is equal to or greater than the rate |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

One multiplication and one division: exact up to the 28 significant digits of the decimal
type. The golden cases are checked to $10^{-22}$ against a 50-digit reference.

## Verified against

- `tests/golden/tv.gordon/damodaran-fcff-from-roc-table.yaml`: a cell of the table "The Big
  Assumption" in Damodaran's valuation lecture slides (NYU Stern), restated as a Gordon flow,
  matches at its published precision.
- The other golden cases come from an independent 50-digit computation
  (`scripts/reference_tv.py`). The formula is the one of
  [`tvm.perpetuity`](tvm-perpetuity.md) for an ordinary growing perpetuity.
