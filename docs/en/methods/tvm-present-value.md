# Present value of periodic flows (`tvm.present_value`)

Discounts the cash flows of periods 1 to $n$ of a plan, with either one rate for every period or
a different rate for each period, and with the flows placed at the end, the middle or the
beginning of their period. It returns the discount factor and the present value of each flow,
and their sum. Use it when the cost of capital changes over the plan, or when the timing of the
flows within each period must be explicit.

## Formula

With period rates $r_1, \ldots, r_n$ (all equal when a single `rate` is given), the end of
period discount factor of period $k$ is the product of the period factors up to $k$:

$$
DF_k^{\text{end}} = \prod_{i=1}^{k} \frac{1}{1 + r_i},
\qquad DF_0^{\text{end}} = 1
$$

The other timings start from the factor of the previous period:

$$
DF_k^{\text{mid}} = \frac{DF_{k-1}^{\text{end}}}{\sqrt{1 + r_k}}
\qquad
DF_k^{\text{beginning}} = DF_{k-1}^{\text{end}}
$$

$$
PV = \sum_{k=1}^{n} CF_k \cdot DF_k
$$

| Timing | `timing` | Flow $k$ occurs at |
| --- | --- | --- |
| End of period | `end_of_period` | $t = k$ |
| Mid period | `mid_period` | $t = k - 0.5$, half a period at the rate $r_k$ of that period |
| Beginning of period | `beginning_of_period` | $t = k - 1$ |

With a constant rate and end-of-period timing the result equals [`tvm.npv`](tvm-npv.md) of the
same flows preceded by a zero flow at $t = 0$.

## Example

Three flows of 100 with rates of 5%, 6% and 7% for periods 1 to 3, under each timing:

```python
from decimal import Decimal

import fairmark as fm

flows = [100, 100, 100]
curve = ["0.05", "0.06", "0.07"]

end = fm.tvm.present_value(flows, period_rates=curve, timing="end_of_period")
mid = fm.tvm.present_value(flows, period_rates=curve, timing="mid_period")
beg = fm.tvm.present_value(flows, period_rates=curve, timing="beginning_of_period")

assert round(end.present_value, 18) == Decimal("269.054756446750804007")
assert abs(mid.present_value - Decimal("276.95204711016002295632")) < Decimal("1e-15")
assert round(beg.present_value, 18) == Decimal("285.085354896675651393")
print(end.discount_factors[1])  # 1 / (1.05 × 1.06) = 0.8984725965858041329739442947
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `cash_flows` | Cash flows of periods 1, 2, ..., n | required |
| `rate` | One discount rate for every period, decimal fraction per period | give exactly one of `rate` and `period_rates` |
| `period_rates` | One discount rate per period (forward period rates), decimal fractions; exactly one per cash flow | give exactly one of `rate` and `period_rates` |
| `timing` | `end_of_period`, `mid_period` or `beginning_of_period` | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `present_value` | Sum of the present values |
| `discount_factors` | Discount factor applied to each flow |
| `present_values` | Present value of each flow |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Timing | end of period, mid period, beginning of period | none: must be chosen |
| Rates | one rate for every period, or one rate per period | none: give exactly one |
| Period rates | forward: $r_k$ applies to period $k$ only | fixed |
| Mid period | half a period at the rate of the same period, a division by $\sqrt{1 + r_k}$ | fixed |

Timing has no default because practice varies: business plans often use mid period, contracts
end of period. With a constant rate, mid-period discounting is the usual
$(1 + r)^{-(k - 0.5)}$.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Both or neither of `rate` and `period_rates` are given, `period_rates` does not have exactly one rate per cash flow, or a field is missing or unknown |
| `FM-E002` | `cash_flows` is empty |
| `FM-E003` | `rate`, or a rate in `period_rates`, is −100% or lower; the message names the field by its JSON pointer, for example `/period_rates/2` |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

End-of-period and beginning-of-period factors use only division; measured against an
independent 60-digit computation, they agree to about $10^{-24}$ relative or better.
Mid-period factors use the decimal square root, which agrees to about $10^{-18}$ relative or
better on typical inputs. The golden cases are checked to $10^{-18}$ for end and beginning
timing and $10^{-19}$ for mid-period timing.

## Verified against

- Golden cases in `tests/golden/tvm.present_value/`, from an independent 50-digit computation
  (`scripts/reference_time.py`).
- With a constant rate and end-of-period timing, agrees with the spreadsheet `NPV` function
  (Excel, LibreOffice), which discounts its first value by one period, and with
  `numpy_financial.npv` applied to the same flows preceded by a zero flow.
