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
$$

$$
Equity = EV - \text{net financial position} - \sum \text{debt-like items} - \sum
\text{non-controlling interests}
$$

The explicit horizon N is the sum of the periods of the stages; one period is one year. The
flows of each stage are:

| Projection | Flow of period k of the stage (m periods) |
| --- | --- |
| `given` | as given in `flows`, one per period, or built up from the line items in `lines` as in [`dcf.free_cash_flows`](dcf-free-cash-flows.md) |
| `constant_growth` | $CF_k = CF_{k-1} (1 + g)$ |
| `linear_decline` | $CF_k = CF_{k-1} (1 + g_k)$, with $g_k = g_{from} + (g_{to} - g_{from}) \, k / m$ |

A generated stage starts from the last flow of the previous stage, or from the base flow
$CF_0$ when it is the first stage. In a linear decline, $g_{from}$ is the growth before the
stage and $g_{to}$ is reached in the last period: "from 10% to 3% over 5 years" gives 8.6%,
7.2%, 5.8%, 4.4% and 3.0%.

A `given` stage can take its flows as line items instead of amounts: `lines` holds one object
per period with `ebit`, `depreciation_and_amortisation`, `capital_expenditure`,
`change_in_net_working_capital`, optional `other_adjustments` and, in the amounts form,
`operating_taxes`. The FCFF of each period is built up as in
[`dcf.free_cash_flows`](dcf-free-cash-flows.md), with the taxes on EBIT from
`notional_tax_rate` and `negative_ebit`, or from `operating_taxes` in every line. Interest
expense and net borrowing are refused: FCFF is before financing. Every step of the build-up is
in the explain, prefixed "Build-up:".

Under mid-period timing the flow of period k is discounted half a period earlier,
$DF_k = DF_{k-1} (1 + r_k)^{-0.5}$. The terminal value $TV_N$ is computed by the terminal
technique chosen (see "Terminal techniques" below), for example
$TV_N = CF_{N+1} / (r_{TV} - g)$ with [`tv.gordon`](tv-gordon.md), and discounted at the
horizon, $DF_{TV} = DF_N$, or, under mid-period timing when chosen, half a period earlier,
$DF_{TV} = DF_N (1 + r_{TV})^{0.5}$; the half-period shift uses the terminal rate $r_{TV}$,
which with one discount rate is the rate of every period.

Present values are computed by dividing each amount by its compounding factor
$\prod_{i \le k} (1 + r_i)$ (under mid-period timing, $\prod_{i < k} (1 + r_i) \,
(1 + r_k)^{0.5}$), which is the same quantity as multiplying by $DF_k$ but keeps every
significant digit at long horizons; the discount factors are reported as computed by
[`tvm.present_value`](tvm-present-value.md).

## Terminal techniques

The `terminal` object names exactly one technique by its method id. Each technique is computed
by its own method, on the same terms, at the terminal rate $r_{TV}$ (`wacc`, or
`terminal_wacc` with rates per period), and the result reports the method id, its version and
its own output in `terminal`. Amounts are in the terms of the cash flows.

| Technique | Terms inside the DCF | Terminal value at N |
| --- | --- | --- |
| [`tv.gordon`](tv-gordon.md) | `growth`, `first_flow`, `normalised_next_flow`, `growth_cap` | $CF_{N+1} / (r_{TV} - g)$, with $CF_{N+1} = CF_N (1 + g)$ or a normalised flow |
| [`tv.h_model`](tv-h-model.md) | `short_term_growth`, `long_term_growth`, `half_life`, `first_flow`, `normalised_base_flow`, `growth_cap` | $CF_N [(1 + g_l) + H (g_s - g_l)] / (r_{TV} - g_l)$ on the last explicit flow or a normalised base flow at N |
| [`tv.value_driver`](tv-value-driver.md) | `nopat_next`, `growth`, `ronic`, `growth_cap` | $NOPAT_{N+1} (1 - g / RONIC) / (r_{TV} - g)$ |
| [`tv.exit_multiple`](tv-exit-multiple.md) | `multiple`, `metric` (`ebitda`, `ebit`, `revenue`), `metric_value`, `next_flow`, `growth_cap` | multiple × metric at N; with `next_flow`, the implied growth $r_{TV} - CF_{N+1} / TV_N$ |
| [`tv.limited_horizon`](tv-limited-horizon.md) | `flow`, `periods`, `growth`, `residual_value`, `growth_cap` | the flows of periods N + 1 to N + n and the residual value at N + n, discounted to N at $r_{TV}$ |
| [`tv.normalised_income`](tv-normalised-income.md) | `income`, `basis` (`enterprise`), `growth`, `growth_cap` | $R / (r_{TV} - g)$: the capitalisation rate is the terminal rate |
| [`tv.liquidation`](tv-liquidation.md) | `assets`, `liabilities`, `liquidation_costs`, `tax_on_gains`, `basis` (`enterprise`) | realisable values − liabilities − liquidation costs − tax on gains |

The technique must be on the enterprise basis of `dcf.fcff`: an exit multiple on earnings
(P/E) or book value (P/BV), or a normalised income or a liquidation declared on the equity
basis, is refused with `FM-E019`. `tv.gordon` and `tv.h_model` grow the last explicit flow, so the first terminal flow
is a required choice: for `tv.gordon`, `last_flow_grown` or a normalised flow at N + 1
(`normalised_next_flow`); for `tv.h_model`, `last_flow_grown` or a normalised base flow at N
(`normalised_base_flow`) that replaces the last explicit flow in the standard formula. Inside the DCF the
`growth` and `residual_value` of `tv.limited_horizon` and the `growth` of
`tv.normalised_income` are required (give 0 for none). `tv.normalised_income` is capitalised at
the terminal rate, as every technique that takes a rate: a separate `capitalisation_rate` is
not accepted inside the DCF.

## Nominal and real terms

`cash_flow_terms` states whether every amount and growth rate of the input is nominal (at
current prices, expected inflation included) or real (at constant prices); `discount_rate_terms`
states the same for the discount rates. Both are required. When they are equal, the rates are
used as given. When they differ:

- without expected inflation the input is refused with `FM-E024`: discounting real flows at a
  nominal rate, or nominal flows at a real rate, misstates the value;
- with expected inflation every discount rate, the terminal rate included, is converted to the
  terms of the cash flows by the exact Fisher relation, computed by
  [`rates.real_nominal`](rates-real-nominal.md):

$$
(1 + r_{nominal}) = (1 + r_{real}) \, (1 + i)
\qquad
r_{real} = \frac{1 + r_{nominal}}{1 + i} - 1
\qquad
r_{nominal} = (1 + r_{real}) (1 + i) - 1
$$

The cash flows and the growth rates are never converted: only the rates move to the terms of
the flows. Expected inflation is one rate for every period and the terminal phase
(`inflation`), or one rate per period of the explicit horizon (`inflation_per_period`) with
the inflation of the terminal phase (`terminal_inflation`); the rate of period k is converted
with the inflation of period k, the terminal rate with the inflation of the terminal phase.
Each period reports the rate given (`discount_rate`), the inflation and the rate used
(`rate_used`), and the explain shows every conversion with its numbers.

Whenever inflation is given, also with equal terms (where it converts nothing), the perpetual
growth of the terminal technique (`growth`, or `long_term_growth` for `tv.h_model`) is restated
in the other terms with the inflation of the terminal phase, in `terms`; with nominal flows, a
perpetual growth below that inflation is reported as a warning, because the flows then decline
in real terms after the horizon. An exit multiple, a limited horizon and a liquidation state no
perpetual growth, and the conventions say that none is restated.

A real plan discounted at a nominal WACC of 10% with expected inflation of 2%:

```python
import fairmark as fm

r = fm.dcf.fcff(
    [fm.dcf.given([100, 104, 108, 112])],
    cash_flow_terms="real",
    discount_rate_terms="nominal",
    wacc="0.10",
    inflation="0.02",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.01", first_flow="last_flow_grown"),
)
print(r.periods[0]["rate_used"])  # (1 + 0.10) / (1 + 0.02) − 1 = 0.07843137...
print(r.terminal["rate"])  # the same real rate for the terminal value
print(r.value["terms"]["perpetual_growth_other_terms"])  # 1.01 × 1.02 − 1 = 0.0302 nominal
print(r.value_of_operations)  # 1573.18265943050768644554905...
```

The same value follows from the real rate given directly with `discount_rate_terms="real"`.
Real flows are expressed at the prices of the valuation date (t = 0). Under end-of-period
timing the value also equals the flows restated in nominal terms, each real flow times the
cumulated inflation index $\prod_{i \le k} (1 + i_i)$, discounted at the nominal rate with the
nominal growth; under mid-period timing the equivalence holds with the index taken at
t = k − 0.5, $\prod_{i < k} (1 + i_i) \, (1 + i_k)^{0.5}$. When the terminal rate was converted,
the rate convention of the terminal technique says "converted" instead of "given".

Points of practice that the engine does not check:

- A real WACC must be the nominal after-tax WACC deflated by the Fisher relation: the interest
  tax shield is earned on nominal interest, so compute the WACC in nominal terms and let the
  engine convert it (or deflate it yourself), rather than applying the tax rate to a real cost
  of debt.
- With real build-up lines, taxes computed on real EBIT understate the tax actually paid,
  because tax depreciation is on historical cost and is not indexed to inflation; when
  inflation matters, prefer a plan in nominal terms (as Damodaran recommends). This is a
  documentation note: no warning is reported.
- A constant growth perpetuity (`tv.gordon`) gives the same value in real and in nominal
  terms. The H-model approximates a declining growth and the value driver subtracts a
  reinvestment g / RONIC, so a real and a nominal valuation of the same business differ
  slightly with `tv.h_model` and `tv.value_driver`: this is a property of the techniques, not
  of the conversion. Their terms, RONIC included, are in the terms of the cash flows
  (Decision 5).

## From value of operations to enterprise value and equity value

The value of operations is the value of the core operations: the present value of the
explicit FCFF and of the terminal value. Non-operating assets (for example excess cash
invested outside the operations, land held for sale, investments in non-consolidated
companies) are added to reach enterprise value. The claims on the company other than equity
are subtracted to reach equity value:

| Line | Input | Sign |
| --- | --- | --- |
| Non-operating assets | `non_operating_assets`: `{"label", "value"}`, values zero or more | added |
| Net financial position | `net_financial_position`: gross debt − cash + signed other items, as in SPEC-0007 | subtracted; positive for net debt |
| Debt-like items | `debt_like_items`: `{"label", "value", "measurement_basis"}`, values zero or more; for example unfunded pension obligations or provisions | subtracted |
| Non-controlling interests | `non_controlling_interests`: `{"label", "value", "measurement_basis"}`, values zero or more; the share of consolidated subsidiaries that belongs to other shareholders | subtracted |

Every amount is measured at the valuation date and taken as given: Fairmark does not choose
which items are included or how they are measured. How a debt-like item is measured (for
example before or after tax) and how the non-controlling interests are measured (for example at
book value or at fair value) is a choice of the analyst, stated in `measurement_basis`; the
basis is reported next to the amount in `explain` and is never used in the calculation. An item
given among the `other_items` of the net financial position (for example lease liabilities) must
not be repeated in `debt_like_items`: the engine does not check for double counting. Lists that
are not given are reported as "none given". Each line is an `explain` step with its label, each
list has a total step, and the equity value step shows every term subtracted.

```python
import fairmark as fm

r = fm.dcf.fcff(
    [fm.dcf.given([250, 270, 285, 300])],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    wacc="0.09",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
    net_financial_position={"gross_debt": 900, "cash": 150},
    non_operating_assets=[{"label": "Land held for sale", "value": 80}],
    debt_like_items=[
        {
            "label": "Unfunded pension obligation",
            "value": 95,
            "measurement_basis": "present value net of plan assets, after tax",
        }
    ],
    non_controlling_interests=[
        {"label": "30% of Subsidiary A", "value": 210, "measurement_basis": "fair value"}
    ],
)
print(r.bridge["debt_like_items_total"], r.bridge["non_controlling_interests_total"])  # 95 210
assert r.enterprise_value - r.equity_value == 750 + 95 + 210
```

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

With an exit multiple, the terminal value is a price at the horizon; giving the flow at N + 1
also reports the perpetual growth that the multiple implies:

```python
r = fm.dcf.fcff(
    [fm.dcf.given([150, 165, 180, 190])],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    wacc="0.10",
    timing="end_of_period",
    terminal=fm.dcf.exit_multiple("7.5", "ebitda", 410, next_flow=260),
)
print(r.terminal["technique"], r.terminal["terminal_value"])  # tv.exit_multiple 3075.0
print(r.terminal["output"]["implied_growth"])  # 0.10 − 260 / 3075 = 0.01544...
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `stages` | One to three stages, in order: `{"periods", "projection", ...}` with `flows` or `lines` (`given`), `growth` (`constant_growth`) or `from_growth` and `to_growth` (`linear_decline`) | required |
| `base_flow` | FCFF of the base period $CF_0$, when the first stage is generated | required for a generated first stage; refused otherwise |
| `notional_tax_rate` | Tax rate on EBIT for the stages with `lines`, decimal fraction from 0 to below 1; give this or `operating_taxes` in every line | none; refused without `lines` |
| `negative_ebit` | `credit` or `zero`, for the stages with `lines` and `notional_tax_rate` | required with `notional_tax_rate`; refused otherwise |
| `cash_flow_terms` | `nominal` or `real`: terms of every amount and growth rate | required |
| `discount_rate_terms` | `nominal` or `real`: terms of the discount rates; when different from `cash_flow_terms`, expected inflation is required and the rates are converted to the terms of the cash flows | required |
| `inflation` | Expected inflation for every period and for the terminal phase, decimal fraction, greater than −1 | none; give this or `inflation_per_period` |
| `inflation_per_period` | Expected inflation per period of the explicit horizon, decimal fractions | none; requires `terminal_inflation` |
| `terminal_inflation` | Expected inflation of the terminal phase | required with `inflation_per_period`; refused otherwise |
| `wacc` | One WACC for every period and for the terminal value, decimal fraction | give this or `wacc_per_period` |
| `wacc_per_period` | One WACC per period of the explicit horizon, decimal fractions | give this or `wacc` |
| `terminal_wacc` | WACC of the terminal phase | required with `wacc_per_period`; refused with `wacc` |
| `timing` | `end_of_period` or `mid_period` | required |
| `terminal_discount_time` | `horizon` (t = N) or `mid_period` (t = N − 0.5) | required with `mid_period` timing; refused with `end_of_period` |
| `terminal` | One technique written with its method id, for example `{"tv.gordon": {"growth", "first_flow", "normalised_next_flow", "growth_cap"}}`; see "Terminal techniques" | required |
| `net_financial_position` | `{"gross_debt", "cash", "other_items"}` as in [`ratios.debt_service`](ratios-debt-service.md), positive for net debt | none, reported |
| `non_operating_assets` | List of `{"label", "value"}`, values zero or more | none, reported |
| `debt_like_items` | List of `{"label", "value", "measurement_basis"}`, values zero or more, subtracted from enterprise value | none, reported |
| `non_controlling_interests` | List of `{"label", "value", "measurement_basis"}`, values zero or more, subtracted from enterprise value | none, reported |

## Outputs

| Field | Meaning |
| --- | --- |
| `value_of_operations` | Present value of the explicit FCFF plus present value of the terminal value |
| `enterprise_value` | Value of operations plus the non-operating assets |
| `equity_value` | Enterprise value minus the net financial position, the debt-like items and the non-controlling interests |
| `pv_explicit_period` | Present value of the explicit FCFF |
| `terminal_value_weight` | Present value of the terminal value divided by the value of operations |
| `periods` | Per period: stage, projection, growth applied, flow, discount rate given, rate used (in the terms of the cash flows), inflation used to convert it (`null` without conversion), discount factor, present value |
| `terms` | Terms of the flows and of the rates, the conversion (`none`, `to_real`, `to_nominal`), terminal rate given and used, inflation of the terminal phase, perpetual growth in the terms of the flows and in the other terms |
| `terminal` | Technique and its method version, terminal rate, value at N, discounting time, factor, present value, the technique's own output (for example the implied growth of an exit multiple) |
| `bridge` | Every line from value of operations to equity value, with labels and measurement bases, and the total of each list |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Basis | enterprise: FCFF discounted at the WACC | fixed by the method |
| Stage projection | `given`, `constant_growth`, `linear_decline` | required per stage |
| Flows of a `given` stage | amounts (`flows`) or line items (`lines`), with the tax conventions of `dcf.free_cash_flows` | one of the two required |
| Linear decline step | $g_k = g_{from} + (g_{to} - g_{from}) \, k / m$, $g_{to}$ reached in the last period | fixed |
| Discount rate | one rate (also used for the terminal value), or one per period with a terminal rate | required |
| Terms of flows and rates | `nominal`, `real` | required, each |
| Different terms | refused without expected inflation; with it, every discount rate converted to the terms of the cash flows by the exact Fisher relation; flows and growth rates never converted | fixed |
| Expected inflation | one rate, or one per period with a terminal inflation | none |
| Timing | `end_of_period`, `mid_period` | required |
| Terminal discounting time | `horizon`, `mid_period` (with mid-period timing only; the half period at the terminal rate) | required |
| Terminal technique | `tv.gordon`, `tv.h_model`, `tv.value_driver`, `tv.exit_multiple`, `tv.limited_horizon`, `tv.normalised_income`, `tv.liquidation`, at the terminal rate, on the enterprise basis | required |
| First terminal flow (`tv.gordon`, `tv.h_model`) | last flow grown; or a normalised flow at N + 1 (`tv.gordon`), a normalised base flow at N (`tv.h_model`) | required |
| Bridge items | measured at the valuation date as given; none given means none | none |
| Bridge signs | non-operating assets added; net financial position (positive for net debt), debt-like items and non-controlling interests subtracted | fixed |
| Measurement basis of debt-like items and non-controlling interests | stated by the analyst, reported, not used in the calculation | required per item |

Warnings are reported in `explain.conventions` and never refuse the input: a growth applied to
a negative flow; a growth of the last explicit period that differs from the perpetual growth
of the terminal value (`growth`, or `long_term_growth` for `tv.h_model`; the growth implied
by an exit multiple is reported, not compared); under mid-period timing, a perpetual-growth
terminal value (`tv.gordon`, `tv.h_model`, `tv.value_driver`, `tv.normalised_income`)
discounted at the horizon N, where practice commonly discounts it at N − 0.5; under mid-period
timing, an exit multiple, a liquidation or a limited horizon discounted at N − 0.5, although
it is or includes a price at a date (for the limited horizon, its residual value); with
nominal flows and expected inflation, a perpetual growth below the inflation of the terminal
phase (a real decline); a negative equity value, which is reported as computed and never floored at zero. A high `terminal_value_weight` signals that the valuation depends mostly
on long-term assumptions.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing, unknown or not used by the chosen option; both `inflation` and `inflation_per_period`, `inflation_per_period` without `terminal_inflation`, `terminal_inflation` without `inflation_per_period`, `terminal_inflation` with `inflation`; no stage or more than three; stages adding up to more than 10,000 periods; a growth of −100% or lower; a terminal rate with one rate, or none with rates per period; a terminal discounting time missing under mid-period timing or given at end of period; no terminal technique or more than one; a `normalised_base_flow` of `tv.h_model` missing with `first_flow` = `normalised_base_flow` or given otherwise; a growth cap of an exit multiple without `next_flow`; a liquidation without assets or with an unlabelled line; a negative non-operating asset, or one with an empty label; a negative debt-like item or non-controlling interest, or one with an empty label or measurement basis (for example `/non_controlling_interests/0/measurement_basis`); a `given` stage with both or neither of `flows` and `lines`, `lines` on a generated stage, tax fields without `lines`, interest expense or net borrowing in `lines`, and every refusal of [`dcf.free_cash_flows`](dcf-free-cash-flows.md) for the lines, named by pointer (for example `/stages/0/lines/2/capital_expenditure`) |
| `FM-E003` | A discount rate, given or converted, or an expected inflation is −100% or lower; the message names the field and the period (the same code as [`rates.real_nominal`](rates-real-nominal.md) for the same value) |
| `FM-E004` | The perpetual growth (`growth`, or `long_term_growth` for `tv.h_model`) is equal to or greater than the terminal rate in the terms of the cash flows (after conversion, when the terms differ) |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |
| `FM-E018` | A term of the terminal technique outside its domain: a return on new invested capital, multiple or metric of zero or below, a terminal rate of zero or below with `tv.normalised_income`, a finite horizon outside 1 to 10,000 periods, a negative half-life, a negative liquidation line or tax on gains |
| `FM-E019` | The terminal technique is on the equity basis: an exit multiple on `earnings` (P/E) or `book_value` (P/BV), or a normalised income or a liquidation with `basis` equity |
| `FM-E024` | `cash_flow_terms` and `discount_rate_terms` differ and no expected inflation is given; the message states both fixes |
| `FM-E025` | A `given` stage with more or fewer flows or lines than its periods, rates or expected inflation per period not one per period, a base flow missing for a generated first stage or given for a `given` one |

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
  rounded inputs; the golden files decompose the differences.
- Damodaran, *Investment Valuation*, 2nd edition, chapter 16, Illustration 16.6 (Segovia
  Entertainment, consolidated approach): value of the operating assets 2,205, plus the 15%
  holding in LatinWorks at book value 22.5, minus consolidated debt 500, minus the minority
  interests in Seville at book value 122.5, equity value 1,605, matched exactly
  (`damodaran-segovia-consolidated.yaml`).
- An independent 50-digit implementation, `scripts/reference_dcf.py`, for every golden case in
  `tests/golden/dcf.fcff/`, and properties: a constant growth stage equals the same flows
  given; a linear decline with equal growths equals constant growth; discount factors equal
  those of [`tvm.present_value`](tvm-present-value.md).
- Nominal and real terms: the conversion restated in the reference with the Fisher relation
  written there (`terms-*.yaml`); real flows at nominal rates with inflation equal exactly the
  same flows at the real rates computed by `rates.real_nominal` and given directly, and under
  end-of-period timing the independently inflated nominal flows at the nominal rates within
  $10^{-20}$ relative (property tests, one inflation and inflation per period).
- Stages with `lines`: the 50-digit build-up of `scripts/reference_free_cash_flows.py`
  composed with `scripts/reference_dcf.py` (`stage-with-build-up-lines*.yaml`).
- Every terminal technique: the terminal value and the technique's whole output inside the
  DCF equal those of the `tv.*` method run alone on the same terms and the terminal rate, and
  its present value equals that value discounted at the terminal discounting time (property
  tests, every technique, rate form and timing).
- numpy-financial NPV of the explicit flows plus the Gordon value, in binary floating point
  (agreement of conventions, not of digits).
