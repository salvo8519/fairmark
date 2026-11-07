# Net present value (`tvm.npv`)

$$
NPV = \sum_{t=0}^{n} \frac{CF_t}{(1+r)^t}
$$

The first cash flow occurs at $t = 0$ and is not discounted. This differs from the
spreadsheet `NPV()` function, which starts discounting from the first value.

```python
import fairmark as fm

r = fm.tvm.npv("0.07", [-5000, 1200, 1500, 1800, 2100])
print(r.npv)   # 503.0695432159...
```

| Input | Meaning |
| --- | --- |
| `rate` | Discount rate per period, decimal fraction |
| `cash_flows` | Cash flows, the first at $t = 0$ |

Integer powers are computed by exact multiplication, so the result has no rounding error
beyond the final 28-digit division.
