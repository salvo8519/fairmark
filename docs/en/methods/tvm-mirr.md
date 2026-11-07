# Modified internal rate of return (`tvm.mirr`)

Returns the modified internal rate of return of a periodic cash flow series: the rate that
grows the present value of the outflows, financed at a finance rate, into the future value of
the inflows, reinvested at a reinvestment rate. Unlike the IRR, it states explicitly the rates
at which money is borrowed and reinvested, and it always has a single value. The definition is
that of the spreadsheet `MIRR` function.

## Formula

For flows $CF_0, \ldots, CF_n$ with the first at $t = 0$:

$$
PV^{-} = \sum_{CF_t < 0} \frac{CF_t}{(1 + r_{\text{fin}})^{t}}
\qquad
FV^{+} = \sum_{CF_t > 0} CF_t \,(1 + r_{\text{re}})^{\,n - t}
$$

$$
MIRR = \left(\frac{FV^{+}}{-PV^{-}}\right)^{1/n} - 1
$$

where $r_{\text{fin}}$ is the finance rate and $r_{\text{re}}$ the reinvestment rate, both per
period.

## Example

An investment of 120,000 followed by five years of income, financed at 10% and reinvested at
12%. The published values are 12.6094%; for the first three years only, −4.8045%; with
reinvestment at 14%, 13.4759%.

```python
from decimal import Decimal

import fairmark as fm

flows = [-120000, 39000, 30000, 21000, 37000, 46000]

r = fm.tvm.mirr(flows, finance_rate="0.10", reinvest_rate="0.12")
assert round(r.mirr, 6) == Decimal("0.126094")
assert abs(r.mirr - Decimal("0.12609413036590514583")) < Decimal("1e-15")
print(r.pv_negative_flows)  # -120000
print(r.fv_positive_flows)  # 217297.49504000

three_years = fm.tvm.mirr(flows[:4], finance_rate="0.10", reinvest_rate="0.12")
assert round(three_years.mirr, 6) == Decimal("-0.048045")

reinvest_14 = fm.tvm.mirr(flows, finance_rate="0.10", reinvest_rate="0.14")
assert round(reinvest_14.mirr, 6) == Decimal("0.134759")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `cash_flows` | Periodic cash flows, the first at $t = 0$; at least two, with at least one positive and one negative | required |
| `finance_rate` | Rate paid on the money invested (applied to negative flows), decimal fraction per period | required |
| `reinvest_rate` | Rate earned on reinvested positive flows, decimal fraction per period | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `mirr` | Modified internal rate of return per period, decimal fraction |
| `pv_negative_flows` | Present value at $t = 0$ of the negative flows, at the finance rate |
| `fv_positive_flows` | Value at $t = n$ of the positive flows, at the reinvestment rate |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Time origin | `cash_flows[0]` at $t = 0$, as in [`tvm.npv`](tvm-npv.md) | fixed |
| Negative flows | discounted to $t = 0$ at the finance rate | fixed |
| Positive flows | compounded to $t = n$ at the reinvestment rate | fixed |
| Finance and reinvestment rates | required inputs | none |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Fewer than two cash flows, or a field is missing or unknown |
| `FM-E003` | The finance rate or the reinvestment rate is −100% or lower; both are checked before any calculation |
| `FM-E006` | The flows have no positive or no negative value |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

The present and future values use exact integer powers; measured against an independent
60-digit computation, these agree to about $10^{-24}$ relative or better. The $n$-th root is
computed as $\exp(\ln(x)/n)$ with the decimal logarithm and exponential, which agree to about
$10^{-18}$ relative or better on typical inputs. The golden cases are checked to $10^{-19}$.

## Verified against

- Published examples: ECMA-376 / Excel `MIRR` examples, 12.6094%, −4.8045% and 13.4759%.
- Agrees with the `MIRR` function of Excel and LibreOffice, `numpy_financial.mirr` and
  `pyxirr.mirr`.
