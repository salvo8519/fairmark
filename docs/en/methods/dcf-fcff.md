# DCF — unlevered free cash flow (`dcf.fcff`)

Values the operating business by discounting free cash flows to the firm (FCFF) at the weighted
average cost of capital (WACC) over one to three explicit stages, adds a terminal value, and
moves from the value of operations to enterprise value and equity value. Every choice that
changes the value is an explicit input: the stages and how their flows are obtained, the terms
of the flows and of the rates, the timing, the terminal technique and when it is discounted.

Method version: 3. Version 3 replaced the single list of flows of version 2 with stages, made
the timing and the terminal technique required inputs, and renamed the version 2
`enterprise_value` (present values only) to `value_of_operations`; the version 2 examples give
the same numbers in version 3.

## Formula

$$
V_{op} = \sum_{k=1}^{N} FCFF_k \, DF_k + TV_N \, DF_{TV}
\qquad
DF_k = \prod_{i=1}^{k} (1 + r_i)^{-1}
$$

$$
EV = V_{op} + \sum \text{non-operating assets}
\qquad
Equity = EV - \text{net financial position}
$$

The explicit horizon N is the sum of the periods of the stages; one period is one year. The
flows of each stage are:

| Projection | Flow of period k of the stage (m periods) |
| --- | --- |
| `given` | as given, one per period |
| `constant_growth` | $CF_k = CF_{k-1} (1 + g)$ |
| `linear_decline` | $CF_k = CF_{k-1} (1 + g_k)$, with $g_k = g_{from} + (g_{to} - g_{from}) \, k / m$ |

A generated stage starts from the last flow of the previous stage, or from the base flow
$CF_0$ when it is the first stage. In a linear decline, $g_{from}$ is the growth before the
stage and $g_{to}$ is reached in the last period: "from 10% to 3% over 5 years" gives 8.6%,
7.2%, 5.8%, 4.4% and 3.0%.

Under mid-period timing the flow of period k is discounted half a period earlier,
$DF_k = DF_{k-1} (1 + r_k)^{-0.5}$. The terminal value
$TV_N = CF_{N+1} / (r_{TV} - g)$ is computed by [`tv.gordon`](tv-gordon.md) and discounted
at the horizon, $DF_{TV} = DF_N$, or, under mid-period timing when chosen, half a period
earlier, $DF_{TV} = DF_N (1 + r_N)^{0.5}$; the half-period shift uses the rate of period N,
$r_N$, not the terminal rate.

Present values are computed by dividing each amount by its compounding factor
$\prod_{i \le k} (1 + r_i)$ (under mid-period timing, $\prod_{i < k} (1 + r_i) \,
(1 + r_k)^{0.5}$), which is the same quantity as multiplying by $DF_k$ but keeps every
significant digit at long horizons; the discount factors are reported as computed by
[`tvm.present_value`](tvm-present-value.md).

## Example

A plan with three given years, then four years in which growth declines in linear increments
from 12% to 3%, then stable growth of 3%:

```python
import fairmark as fm

r = fm.dcf.fcff(
    [
        fm.dcf.given([100, 112, 125]),
        fm.dcf.linear_decline(4, from_growth="0.12", to_growth="0.03"),
    ],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    wacc="0.095",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.03", first_flow="last_flow_grown"),
    net_financial_position={"gross_debt": 500, "cash": 120},
    non_operating_assets=[{"label": "Land held for sale", "value": 60}],
)
print([row["growth"] for row in r.periods][3:])  # 0.0975, 0.075, 0.0525, 0.03
print(r.value_of_operations)    # 1985.96014516242602644247722...
print(r.enterprise_value)       # value of operations + 60
print(r.equity_value)           # enterprise value − (500 − 120)
print(r.terminal_value_weight)  # share of the value of operations from the terminal value
assert r.enterprise_value - r.value_of_operations == 60
```

With rates per period, give `wacc_per_period` (one rate per period of the explicit horizon)
and `terminal_wacc` instead of `wacc`. With mid-period timing, state when the terminal value
is discounted:

```python
r = fm.dcf.fcff(
    [fm.dcf.constant_growth(3, "0.10"), fm.dcf.linear_decline(3, "0.10", "0.025")],
    base_flow=80,
    cash_flow_terms="real",
    discount_rate_terms="real",
    wacc_per_period=["0.11", "0.11", "0.105", "0.10", "0.095", "0.09"],
    terminal_wacc="0.09",
    timing="mid_period",
    terminal_discount_time="horizon",
    terminal=fm.dcf.gordon("0.025", first_flow="last_flow_grown"),
)
print(r.terminal["discount_time"])  # 6
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `stages` | One to three stages, in order: `{"periods", "projection", ...}` with `flows` (`given`), `growth` (`constant_growth`) or `from_growth` and `to_growth` (`linear_decline`) | required |
| `base_flow` | FCFF of the base period $CF_0$, when the first stage is generated | required for a generated first stage; refused otherwise |
| `cash_flow_terms` | `nominal` or `real`: terms of every amount and growth rate | required |
| `discount_rate_terms` | `nominal` or `real`: terms of the discount rates; must equal `cash_flow_terms` | required |
| `wacc` | One WACC for every period and for the terminal value, decimal fraction | give this or `wacc_per_period` |
| `wacc_per_period` | One WACC per period of the explicit horizon, decimal fractions | give this or `wacc` |
| `terminal_wacc` | WACC of the terminal phase | required with `wacc_per_period`; refused with `wacc` |
| `timing` | `end_of_period` or `mid_period` | required |
| `terminal_discount_time` | `horizon` (t = N) or `mid_period` (t = N − 0.5) | required with `mid_period` timing; refused with `end_of_period` |
| `terminal` | `{"tv.gordon": {"growth", "first_flow", "normalised_next_flow", "growth_cap"}}`; `first_flow` is `last_flow_grown` or `normalised_next_flow` | required |
| `net_financial_position` | `{"gross_debt", "cash", "other_items"}` as in [`ratios.debt_service`](ratios-debt-service.md), positive for net debt | none, reported |
| `non_operating_assets` | List of `{"label", "value"}`, values zero or more | none, reported |

## Outputs

| Field | Meaning |
| --- | --- |
| `value_of_operations` | Present value of the explicit FCFF plus present value of the terminal value |
| `enterprise_value` | Value of operations plus the non-operating assets |
| `equity_value` | Enterprise value minus the net financial position |
| `pv_explicit_period` | Present value of the explicit FCFF |
| `terminal_value_weight` | Present value of the terminal value divided by the value of operations |
| `periods` | Per period: stage, projection, growth applied, flow, discount rate given, rate used, discount factor, present value |
| `terminal` | Technique and its version, terminal rate, value at N, discounting time, factor, present value, the technique's own output |
| `bridge` | Every line from value of operations to equity value, with labels |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Basis | enterprise: FCFF discounted at the WACC | fixed by the method |
| Stage projection | `given`, `constant_growth`, `linear_decline` | required per stage |
| Linear decline step | $g_k = g_{from} + (g_{to} - g_{from}) \, k / m$, $g_{to}$ reached in the last period | fixed |
| Discount rate | one rate (also used for the terminal value), or one per period with a terminal rate | required |
| Terms of flows and rates | `nominal`, `real`; different terms are refused | required, each |
| Timing | `end_of_period`, `mid_period` | required |
| Terminal discounting time | `horizon`, `mid_period` (with mid-period timing only) | required |
| Terminal technique | `tv.gordon` at the terminal rate, on the enterprise basis | required |
| First terminal flow | last flow grown by g, or a normalised flow at N + 1 | required |
| Bridge items | measured at the valuation date as given; none given means none | none |

Warnings are reported in `explain.conventions` and never refuse the input: a growth applied to
a negative flow; a growth of the last explicit period that differs from the perpetual growth
of the terminal value; under mid-period timing, a Gordon terminal value discounted at the
horizon N, where practice commonly discounts it at N − 0.5; a negative equity value, which is
reported as computed and never floored at zero. A high `terminal_value_weight` signals that the valuation depends mostly
on long-term assumptions.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing, unknown or not used by the chosen option; no stage or more than three; stages adding up to more than 10,000 periods; a growth of −100% or lower; a terminal rate with one rate, or none with rates per period; a terminal discounting time missing under mid-period timing or given at end of period; a negative non-operating asset, or one with an empty label |
| `FM-E003` | A discount rate is −100% or lower; the message names the field and the period |
| `FM-E004` | The perpetual growth is equal to or greater than the terminal rate |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |
| `FM-E024` | `cash_flow_terms` and `discount_rate_terms` differ |
| `FM-E025` | A `given` stage with more or fewer flows than its periods, rates per period not one per period, a base flow missing for a generated first stage or given for a `given` one |

Validation errors name the field by its JSON pointer, for example `/stages/1/flows`, and state
the fix; errors raised while reading the input (a missing or unknown field, a wrong type) report
the line and column of the input instead.

## Accuracy

Discount factors are computed by `tvm.present_value` (`tvm::period_discount_factors`): at end
of period by exact divisions, at mid period with a decimal square root. Present values divide by
the compounding factor, so they keep their significant digits at long horizons: measured
against an independent 50-digit computation, the value of operations agrees to about
$10^{-24}$ relative or better, at end and at mid period, also over 300 to 1,000 periods. The
golden cases from the reference are checked to $10^{-20}$.

## Verified against

- Damodaran, *Investment Valuation*, 2nd edition, chapter 15, Illustration 15.3 (Amgen,
  three-stage FCFF with costs of capital changing every year): present value of the explicit
  period 8,327, value of the operating assets 39,161, equity value 40,867 and terminal value
  81,364 are matched within 2 (terminal value: 20) of the published figures, which come from
  rounded inputs; the golden files decompose the differences. These tolerances await the
  maintainer's acceptance.
- An independent 50-digit implementation, `scripts/reference_dcf.py`, for every golden case in
  `tests/golden/dcf.fcff/`, and properties: a constant growth stage equals the same flows
  given; a linear decline with equal growths equals constant growth; discount factors equal
  those of [`tvm.present_value`](tvm-present-value.md).
- numpy-financial NPV of the explicit flows plus the Gordon value, in binary floating point
  (agreement of conventions, not of digits).
