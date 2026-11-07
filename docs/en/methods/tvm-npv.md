# Net present value (`tvm.npv`)

Discounts a periodic cash flow series to $t = 0$ at one rate per period. For flows on calendar
dates use [`tvm.xnpv`](tvm-xnpv.md); for a different rate in each period or mid-period timing
use [`tvm.present_value`](tvm-present-value.md).

Method version: 2. Version 2 computes discount factors for negative rates from the reciprocal
of $1 + r$; only the last digits of results with negative rates differ from version 1.

## Formula

$$
NPV = \sum_{t=0}^{n} \frac{CF_t}{(1+r)^t}
$$

The first cash flow occurs at $t = 0$ and is not discounted. This differs from the
spreadsheet `NPV` function, which starts discounting from the first value.

## Example

```python
import fairmark as fm

r = fm.tvm.npv("0.07", [-5000, 1200, 1500, 1800, 2100])
print(r.npv)   # 503.0695432159...
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `rate` | Discount rate per period, decimal fraction | required |
| `cash_flows` | Cash flows, the first at $t = 0$ | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `npv` | Net present value at $t = 0$ |
| `present_values` | Present value of each flow |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Time origin | `cash_flows[0]` at $t = 0$, not discounted | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing or unknown |
| `FM-E002` | `cash_flows` is empty |
| `FM-E003` | The rate is −100% or lower |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Integer powers are computed by exact multiplication. Measured against an independent 60-digit
computation, results agree to about $10^{-24}$ relative or better. Negative rates are
discounted through the reciprocal of $1 + r$, keeping full relative precision, and very long
horizons underflow to zero instead of overflowing. The golden case is checked to $10^{-9}$.

## Verified against

- Golden case in `tests/golden/tvm.npv/`.
- Agrees with `numpy_financial.npv` and `pyxirr.npv`, which use the same time origin, and
  with the spreadsheet expression `NPV(r, CF_1, ..., CF_n) + CF_0` (Excel, LibreOffice).
