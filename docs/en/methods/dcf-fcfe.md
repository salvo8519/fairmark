# DCF — free cash flow to equity (`dcf.fcfe`)

Values the equity of a company directly by discounting free cash flows to equity (FCFE) at the
cost of equity over one to three explicit stages, adds a terminal value on the equity basis,
then adds the non-operating assets and subtracts the debt-like items and the non-controlling
interests. Debt is already in the flows, through interest and net borrowing, so no net
financial position is subtracted. Every choice that changes the value is an explicit input, as
in [`dcf.fcff`](dcf-fcff.md), which shares the same engine.

Method version: 1. Status: preview.

## Formula

$$
FCFE_t = FCFF_t - (\text{interest}_t - S_t) + \text{net borrowing}_t
\qquad
S_t = t \times D_t
$$

$$
V = \sum_{k=1}^{N} FCFE_k \, DF_k + TV_N \, DF_{TV}
\qquad
DF_k = \prod_{i=1}^{k} (1 + k_{e,i})^{-1}
$$

$$
Equity = V + \sum \text{non-operating assets} - \sum \text{debt-like items} - \sum
\text{non-controlling interests}
$$

$k_{e,i}$ is the cost of equity of period i: one rate for every period and the terminal value
(`cost_of_equity`), or one rate per period (`cost_of_equity_per_period`) with the rate of the
terminal phase (`terminal_cost_of_equity`). The cost of equity, not the WACC, discounts the
FCFE, because the FCFE is the cash left to the shareholders after the lenders have been paid
(CFA Institute, Free Cash Flow Valuation; Damodaran, *Investment Valuation*, chapter 14).

The flows are obtained exactly as in [`dcf.fcff`](dcf-fcff.md): the explicit horizon N is the
sum of the periods of the stages, and each stage is `given`, `constant_growth` or
`linear_decline`, with the same formulas, the same timing (end of period or mid-period, with
the terminal discounting time) and the same nominal and real terms with expected inflation.
The amounts are FCFE: given directly, generated from the base flow $CF_0$ (the FCFE of the base
period), or built up from line items.

## FCFE from line items

A `given` stage can take its flows as line items in `lines`, one object per period, with
`ebit`, `depreciation_and_amortisation`, `capital_expenditure`,
`change_in_net_working_capital`, optional `other_adjustments` and, in the amounts form,
`operating_taxes`, plus the financing lines `interest_expense` and `net_borrowing` (positive
for new borrowing) in every line. The FCFE of each period is built up as in
[`dcf.free_cash_flows`](dcf-free-cash-flows.md):

- with `notional_tax_rate` and `negative_ebit`, the taxes on EBIT come from the rate and the
  interest tax shield $S_t$ uses the same rate. Under `credit` the deductible interest $D_t$ is
  the whole interest expense. Under `zero` no tax credit arises on a pre-tax loss, so the
  deductible interest is limited to the taxable income available,
  $D_t = \min(\text{interest}_t, \max(EBIT_t, 0))$ (SPEC-0001 Decision 22): with a negative EBIT
  there is no shield and the interest is subtracted in full; with an EBIT above zero but below
  the interest the shield is t × EBIT; the FCFE then equals the net-income route in every
  period;
- with `operating_taxes` in every line, the rate of the interest tax shield is the required
  `interest_tax_rate` and the whole interest expense is deductible.

`dcf.fcfe` applies this rule from version 1, through `dcf.free_cash_flows` version 2.

Lines without the financing lines are refused: the equity basis needs them in every period.
With `cash_flow_terms` real, the interest expense must be real interest: the part of nominal
interest that compensates for inflation is, in real terms, a repayment of principal and
belongs in the net borrowing. This is a documentation note: the engine does not check it.
Every step of the build-up is in the explain, prefixed "Build-up:", and the period table says
the flow was built up from line items.

## Terminal techniques on the equity basis

The `terminal` object names exactly one technique by its method id, as in
[`dcf.fcff`](dcf-fcff.md#terminal-techniques), computed by its own method at the terminal cost
of equity. The technique must be on the equity basis:

| Technique | On the equity basis |
| --- | --- |
| [`tv.gordon`](tv-gordon.md), [`tv.h_model`](tv-h-model.md), [`tv.limited_horizon`](tv-limited-horizon.md) | Accepted: they take the basis of the flows they are given, here the FCFE |
| [`tv.exit_multiple`](tv-exit-multiple.md) | `earnings` (P/E) or `book_value` (P/BV); `ebitda`, `ebit` and `revenue` are enterprise multiples and are refused with `FM-E019` |
| [`tv.normalised_income`](tv-normalised-income.md) | A normalised net income with `basis` `equity`; `enterprise` is refused with `FM-E019` |
| [`tv.liquidation`](tv-liquidation.md) | `basis` `equity`, with the financial debt among the liabilities; `enterprise` is refused with `FM-E019` |
| [`tv.value_driver`](tv-value-driver.md) | Refused with `FM-E019`: its terms, NOPAT and the return on new invested capital, are firm-level (enterprise) quantities, so the value driver formula is not accepted practice on the equity basis (SPEC-0001 Decision 23); use `tv.gordon` with a normalised next flow, or an exit multiple on earnings or book value |

A Gordon terminal value on the last FCFE grown carries that flow's net borrowing into
perpetuity. In stable growth the net borrowing should be the one that keeps the debt ratio
of the company constant, as in the stable-growth FCFE of Damodaran, *Investment Valuation*,
chapter 14; when the last explicit year has unusual borrowing or repayment, give
a normalised FCFE at N + 1 (`first_flow` = `normalised_next_flow`). This is a documentation
note: no warning is reported.

## From the present value of the FCFE to equity value

The present value of the explicit FCFE and of the terminal value is the value of the equity
claim on the operating flows. Three kinds of lines complete the equity value:

| Line | Input | Sign |
| --- | --- | --- |
| Non-operating assets | `non_operating_assets`: `{"label", "value"}`, values zero or more | added |
| Debt-like items | `debt_like_items`: `{"label", "value", "measurement_basis", "basis_note"}`, values zero or more | subtracted |
| Non-controlling interests | `non_controlling_interests`: `{"label", "value", "measurement_basis", "basis_note"}`, values zero or more | subtracted |

The lines and their measurement bases work as in [`dcf.fcff`](dcf-fcff.md#from-value-of-operations-to-enterprise-value-and-equity-value):
every amount is measured at the valuation date and taken as given, each debt-like item and
non-controlling interest states its `measurement_basis`, and lists not given are reported as
"none given". The result has no value of operations and no enterprise value.

How each item is treated on the equity basis:

- **Debt and the net financial position.** The FCFE already pays the interest and records the
  borrowing and the repayments, so the debt is in the flows. A `net_financial_position` is
  refused with `FM-E019`: subtracting it would count the debt twice.
- **Cash.** The net financial position also nets the cash, so cash is not subtracted or added
  through it. Cash not needed for operations whose interest income is not in the FCFE is a
  non-operating asset: give it in `non_operating_assets`. Cash whose interest income is in the
  FCFE, or that the business needs to operate, is already valued by the flows and is not added.
- **Debt-like items.** Subtract only obligations whose cash flows are not in the FCFE, for
  example an unfunded pension obligation whose deficit contributions are not in the plan. An
  obligation whose payments the FCFE already includes must not be subtracted again; the engine
  does not check for double counting.
- **Non-controlling interests.** Subtract them when the FCFE are those of the consolidated
  group, which include 100% of the flows of subsidiaries that are not wholly owned.

A negative equity value is reported as computed, with a warning, and never floored at zero.

## Example

A plan with three given years of FCFE, then three years in which growth declines in linear
increments from 7% to 2.5%, then stable growth of 2.5%, at a cost of equity of 11.5%:

```python
import fairmark as fm

r = fm.dcf.fcfe(
    [
        fm.dcf.given([180, 195, 210]),
        fm.dcf.linear_decline(3, from_growth="0.07", to_growth="0.025"),
    ],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    cost_of_equity="0.115",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.025", first_flow="last_flow_grown"),
    non_operating_assets=[{"label": "Excess cash, interest income not in the FCFE", "value": 140}],
    non_controlling_interests=[
        fm.dcf.bridge_claim("20% of Subsidiary A", 85, measurement_basis="fair_value")
    ],
)
print([row["growth"] for row in r.periods][3:])  # 0.055, 0.040, 0.025
print(r.bridge["pv_explicit_and_terminal"])      # 2269.51242269056513224936544...
print(r.equity_value)                            # 2269.51... + 140 − 85 = 2324.51...
print(r.terminal_value_weight)                   # 0.61677753424248815718841...
assert r.equity_value - r.bridge["pv_explicit_and_terminal"] == 140 - 85
```

FCFE built up from line items with interest and net borrowing, and a P/E exit multiple:

```python
r = fm.dcf.fcfe(
    [
        fm.dcf.given_lines(
            [
                {"ebit": 400, "depreciation_and_amortisation": 60, "capital_expenditure": 80,
                 "change_in_net_working_capital": 10, "interest_expense": 30,
                 "net_borrowing": 20},
                {"ebit": 430, "depreciation_and_amortisation": 64, "capital_expenditure": 85,
                 "change_in_net_working_capital": 11, "interest_expense": 31,
                 "net_borrowing": 15},
            ]
        )
    ],
    notional_tax_rate="0.24",
    negative_ebit="zero",
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    cost_of_equity="0.11",
    timing="end_of_period",
    terminal=fm.dcf.exit_multiple("12", "earnings", 300),
)
print([row["flow"] for row in r.periods])  # 274 − 30 × 0.76 + 20 = 271.20; 286.24
print(r.terminal["terminal_value"])        # 12 × 300 = 3600
```

With no debt, the FCFE are the FCFF and `dcf.fcfe` at a cost of equity equal to the WACC gives
the equity value of `dcf.fcff` without a net financial position, to the last digit.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `stages` | One to three stages of FCFE, in order: `{"periods", "projection", ...}` with `flows` or `lines` (`given`), `growth` (`constant_growth`) or `from_growth` and `to_growth` (`linear_decline`) | required |
| `base_flow` | FCFE of the base period $CF_0$, when the first stage is generated | required for a generated first stage; refused otherwise |
| `notional_tax_rate` | Tax rate on EBIT for the stages with `lines`, also the rate of the interest tax shield; give this or `operating_taxes` in every line | none; refused without `lines` |
| `negative_ebit` | `credit` or `zero`, for the stages with `lines` and `notional_tax_rate` | required with `notional_tax_rate`; refused otherwise |
| `interest_tax_rate` | Rate of the interest tax shield for the stages with `lines` and `operating_taxes` | required in that case; refused otherwise |
| `cash_flow_terms` | `nominal` or `real`: terms of every amount and growth rate | required |
| `discount_rate_terms` | `nominal` or `real`: terms of the costs of equity; when different from `cash_flow_terms`, expected inflation is required and the rates are converted to the terms of the cash flows | required |
| `inflation`, `inflation_per_period`, `terminal_inflation` | Expected inflation, as in [`dcf.fcff`](dcf-fcff.md#nominal-and-real-terms) | none |
| `cost_of_equity` | One cost of equity for every period and for the terminal value, decimal fraction | give this or `cost_of_equity_per_period` |
| `cost_of_equity_per_period` | One cost of equity per period of the explicit horizon | give this or `cost_of_equity` |
| `terminal_cost_of_equity` | Cost of equity of the terminal phase | required with `cost_of_equity_per_period`; refused with `cost_of_equity` |
| `timing` | `end_of_period` or `mid_period` | required |
| `terminal_discount_time` | `horizon` (t = N) or `mid_period` (t = N − 0.5) | required with `mid_period` timing; refused with `end_of_period` |
| `terminal` | One technique on the equity basis, written with its method id | required |
| `net_financial_position` | Refused with `FM-E019`: debt is in the FCFE | not accepted |
| `non_operating_assets` | List of `{"label", "value"}`, values zero or more, added | none, reported |
| `debt_like_items` | List of `{"label", "value", "measurement_basis", "basis_note"}`, values zero or more, subtracted | none, reported |
| `non_controlling_interests` | List of `{"label", "value", "measurement_basis", "basis_note"}`, values zero or more, subtracted | none, reported |

## Outputs

| Field | Meaning |
| --- | --- |
| `basis` | `equity` |
| `equity_value` | Present value of the explicit FCFE and of the terminal value, plus the non-operating assets, minus the debt-like items and the non-controlling interests |
| `pv_explicit_period` | Present value of the explicit FCFE |
| `terminal_value_weight` | Present value of the terminal value divided by the present value of the explicit FCFE plus the present value of the terminal value; `null` when that sum is zero |
| `periods` | Per period, as in `dcf.fcff`: stage, projection, growth applied, flow, discount rate given, rate used, inflation used, discount factor, present value |
| `terms` | Terms of the flows and of the rates, the conversion, terminal rate given and used, perpetual growth in both terms |
| `terminal` | Technique and its method version, terminal rate, value at N, discounting time, factor, present value, the technique's own output |
| `bridge` | `pv_explicit_and_terminal`, every non-operating asset, debt-like item and non-controlling interest with its label and basis, the total of each list, and `equity_value` |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Basis | equity: FCFE discounted at the cost of equity; equity value directly, no net financial position | fixed by the method |
| FCFE of a `given` stage | amounts (`flows`) or line items with interest and net borrowing (`lines`) | one of the two required |
| Interest tax shield | t × deductible interest, t the tax rate on EBIT or `interest_tax_rate`; the deductible interest is the whole interest under `credit` and in the amounts form, and min(interest, max(EBIT, 0)) under `zero` (Decision 22) | fixed |
| Stages, discount rate, terms, inflation, timing, terminal discounting time | as in [`dcf.fcff`](dcf-fcff.md), with the cost of equity as the discount rate | required |
| Terminal technique | on the equity basis, at the terminal cost of equity | required |
| Bridge items | measured at the valuation date as given; none given means none | none |
| Bridge signs | non-operating assets added; debt-like items and non-controlling interests subtracted | fixed |
| Measurement basis of debt-like items and non-controlling interests | `book_value`, `market_value`, `fair_value`, `present_value`, `other` (with `basis_note`); reported, not used in the calculation | required per item |

The warnings of `dcf.fcff` apply unchanged (growth on a negative flow, growth discontinuity,
terminal discounting time under mid-period timing, perpetual growth below inflation), and a
negative equity value is reported as a warning, never floored at zero.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | As in [`dcf.fcff`](dcf-fcff.md#errors), with the cost of equity fields in place of the WACC fields (for example `/terminal_cost_of_equity` with `cost_of_equity`); a field of `dcf.fcff` that this method does not have, such as `wacc`; `lines` without `interest_expense` and `net_borrowing`; `interest_tax_rate` without `lines` |
| `FM-E003` | A cost of equity, given or converted, or an expected inflation is −100% or lower (for example `/cost_of_equity_per_period/2`) |
| `FM-E004` | The perpetual growth is equal to or greater than the terminal cost of equity in the terms of the cash flows |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |
| `FM-E018` | A term of the terminal technique outside its domain, as in `dcf.fcff` |
| `FM-E019` | A `net_financial_position` is given; or the terminal technique is on the enterprise basis: an exit multiple on `ebitda`, `ebit` or `revenue`, a normalised income or a liquidation with `basis` enterprise, or `tv.value_driver`, whose NOPAT and RONIC are firm-level quantities (Decision 23) |
| `FM-E024` | `cash_flow_terms` and `discount_rate_terms` differ and no expected inflation is given |
| `FM-E025` | A count does not match the stages: flows or lines, costs of equity or inflation per period, or a misplaced base flow |

Every error names the field by its JSON pointer, for example `/cost_of_equity_per_period/2`,
and states the fix, including errors raised while reading the input: a missing or unknown field
(`/terminal/tv.gordon/first_flow`), a wrong type or an unknown choice (`/stages/0/projection`,
with the values expected).

## Accuracy

The engine is the one of `dcf.fcff`: discount factors by exact divisions (end of period) or a
decimal square root (mid period), present values by division by the compounding factor. The
golden cases from the reference are checked to $10^{-20}$.

## Verified against

- An independent 50-digit implementation, `scripts/reference_dcf.py` (`dcf_fcfe`), for every
  reference golden case in `tests/golden/dcf.fcfe/`: three stages with a cost of equity per
  period and the bridge, a P/E exit multiple, a normalised net income, real flows with
  inflation under mid-period timing, a negative equity value; and on a grid of stages, rates
  and timings.
- Stages with `lines`: the 50-digit build-up of `scripts/reference_free_cash_flows.py`
  (`equity_stages_with_flows`) composed with `scripts/reference_dcf.py`, in both tax forms and
  with the `zero` treatment of a negative EBIT and of an EBIT above zero but below the interest
  (`stage-with-build-up-lines-*.yaml`).
- Consistency with `dcf.fcff` (seeded, parametrised tests): with no debt and no net borrowing, the equity
  value of `dcf.fcfe` equals the equity value of `dcf.fcff` on the same flows, at a WACC equal
  to the cost of equity and with the same bridge lines, to the last digit; also on build-up
  lines with zero interest and zero net borrowing.
- The R24 identity on random bridges: equity value = present value of the FCFE and the terminal
  value + non-operating assets − debt-like items − non-controlling interests.
