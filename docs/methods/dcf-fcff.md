# DCF — unlevered free cash flow (`dcf.fcff`)

Values the operating business by discounting free cash flows to the firm (FCFF) at the weighted
average cost of capital (WACC), then moves from enterprise value to equity value.

## Formula

```latex
EV = \sum_{t=1}^{n} \frac{FCFF_t}{(1+WACC)^{\tau_t}} + \frac{TV}{(1+WACC)^{n}}
\qquad
TV = \frac{FCFF_n \,(1+g)}{WACC - g}
```

```latex
Equity = EV - \text{net debt} + \text{non-operating assets}
```

where `τ_t = t` (end of period) or `τ_t = t − 0.5` (mid period).

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `free_cash_flows` | FCFF for explicit years 1..n | required |
| `discount_rate` | WACC as a decimal fraction | required |
| `terminal_growth` | Perpetual growth after year n; must be < WACC | required |
| `timing` | `end_of_period` or `mid_period` | `end_of_period` |
| `net_debt` | Debt minus cash; negative for net cash | `0` |
| `non_operating_assets` | Surplus assets added to equity | `0` |

## Conventions (explicit, reported in `explain.conventions`)

- The terminal value is placed at the end of year n and discounted at `t = n`, also under the
  mid-period convention. Practice varies; a parameter to change this is planned.
- `terminal_value_weight` reports the share of EV coming from the terminal value: a high weight
  signals that the valuation depends mostly on long-term assumptions.

## Planned (M2)

FCFF build-up from statutory financial statements, WACC/CAPM with size and specific risk
premia, exit-multiple terminal value, sensitivity grids.
