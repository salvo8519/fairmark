# DCF — unlevered free cash flow (`dcf.fcff`)

Values the operating business by discounting free cash flows to the firm (FCFF) at the weighted
average cost of capital (WACC), then moves from enterprise value to equity value.

Method version: 2. Version 2 computes discount factors for negative rates from the reciprocal
of $1 + r$; only the last digits of results with negative rates differ from version 1.

## Formula

$$
EV = \sum_{t=1}^{n} \frac{FCFF_t}{(1+WACC)^{\tau_t}} + \frac{TV}{(1+WACC)^{n}}
\qquad
TV = \frac{FCFF_n \,(1+g)}{WACC - g}
$$

$$
Equity = EV - \text{net debt} + \text{non-operating assets}
$$

where $\tau_t = t$ (end of period) or $\tau_t = t - 0.5$ (mid period). The terminal value is
the Gordon growth model, as in [`tvm.perpetuity`](tvm-perpetuity.md).

## Example

```python
import fairmark as fm

r = fm.dcf.fcff(
    [250, 270, 290, 300, 310],
    discount_rate="0.085",
    terminal_growth="0.015",
    timing="mid_period",
    net_debt=-50,              # net cash
    non_operating_assets=30,
)
print(r.enterprise_value)       # 4145.01655907...
print(r.terminal_value_weight)  # share of EV from the terminal value
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `free_cash_flows` | FCFF for explicit years 1..n | required |
| `discount_rate` | WACC per year, decimal fraction | required |
| `terminal_growth` | Perpetual growth after year n, decimal fraction; must be below WACC | required |
| `timing` | `end_of_period` or `mid_period` | `end_of_period` |
| `net_debt` | Debt minus cash; negative for net cash | `0` |
| `non_operating_assets` | Surplus assets added to equity | `0` |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Timing of explicit flows | end of period, mid period | end of period |
| Terminal value | placed at the end of year n and discounted at $t = n$, also under mid period | fixed |

Practice varies on the terminal value placement; a parameter to change it is planned.
`terminal_value_weight` reports the share of EV coming from the terminal value: a high weight
signals that the valuation depends mostly on long-term assumptions.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing or unknown |
| `FM-E002` | `free_cash_flows` is empty |
| `FM-E003` | The discount rate is −100% or lower |
| `FM-E004` | The terminal growth is equal to or greater than the discount rate |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

End-of-period discounting uses exact integer powers; measured against an independent 60-digit
computation, such operations agree to about $10^{-24}$ relative or better. Mid-period
discounting uses fractional powers, which agree to about $10^{-18}$ relative or better on
typical inputs. The golden cases are checked to $10^{-9}$.

## Verified against

- Golden cases in `tests/golden/dcf.fcff/`, from an independent decimal implementation
  (`scripts/reference_values.py`).

## Planned (M2)

FCFF build-up from statutory financial statements, WACC/CAPM with size and specific risk
premia, exit-multiple terminal value, sensitivity grids.
