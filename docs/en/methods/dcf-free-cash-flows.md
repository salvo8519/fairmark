# Free cash flow build-up (`dcf.free_cash_flows`)

Builds the free cash flow to the firm (FCFF) of every period of a business plan from its
reclassified line items: EBIT, taxes on EBIT, depreciation and amortisation, capital
expenditure, change in net working capital and labelled adjustments. When interest expense and
net borrowing are also given, it builds the free cash flow to equity (FCFE). The flows are
amounts per period, not discounted: discounting them is the job of the DCF,
[`dcf.fcff`](dcf-fcff.md) for the FCFF and [`dcf.fcfe`](dcf-fcfe.md) for the FCFE, both of
which also accept the same lines in a `given` stage. Reclassifying the statutory statements
into these lines is a separate step.

Method version: 2. Status: preview.

## Formula

For every period $t$:

$$
FCFF_t = EBIT_t - T_t + DA_t - CAPEX_t - \Delta NWC_t + \sum_k ADJ_{k,t}
$$

$$
FCFE_t = FCFF_t - (INT_t - S_t) + NB_t, \qquad S_t = \tau \times D_t
$$

with the deductible interest $D_t = INT_t$ under `credit` and in the amounts form, and
$D_t = \min\big(INT_t, \max(EBIT_t, 0)\big)$ under `zero` (see below).

The taxes on EBIT $T_t$ are given in exactly one of two forms:

- **rate on EBIT**: $T_t = \tau \times EBIT_t$, with the tax rate $\tau$ given once for all
  periods. When $EBIT_t < 0$ the required treatment decides: `credit` keeps
  $T_t = \tau \times EBIT_t$ (a tax saving), `zero` sets $T_t = 0$. Tax loss carry-forwards are
  not modelled;
- **amounts**: $T_t$ given for every period.

The rate $\tau$ of the interest tax shield $S_t$ in the FCFE is the tax rate on EBIT in the
first form, and a separate required input, `interest_tax_rate`, in the second.

Under the `zero` treatment no tax credit arises on a pre-tax loss, so the interest is deductible
only up to the taxable income available:

$$
S_t = \tau \times \min\big(INT_t, \max(EBIT_t, 0)\big)
$$

and the total taxes, $T_t - S_t$, never fall below zero. In a period with a negative EBIT there
is no shield: $\tau = 0$ and the interest is subtracted in full. When EBIT is positive but
below the interest, the shield is $\tau \times EBIT_t$: with EBIT 30, interest 50 and a tax
rate of 24%, the shield is 7.20, not 12. In every period the FCFE then equals the net-income
route (net income + D&A − capex − ΔNWC + adjustments + net borrowing, with no tax on a negative
pre-tax income). This limit applies from method version 2 (SPEC-0001 Decision 22); version 1
kept the full shield when EBIT was positive but below the interest.

With `credit` the full shield $\tau \times INT_t$ applies in every period, also when EBIT is
lower than the interest or negative: the loss gives a tax saving. In the amounts form the shield
stays $\tau \times INT_t$ at `interest_tax_rate` in every period, whatever the sign of EBIT and
whatever the given `operating_taxes`: the treatment of a negative EBIT belongs to the rate form
only. When the given operating taxes are lower than the shield, the shield implies a tax credit
(total taxes below zero); if no such credit is available, give a lower `interest_tax_rate`, or
use the rate form with `zero`.

**Consistency with the discount rate.** An after-tax cost of debt $K_d \times (1 - t)$ in the
WACC or in the cost of equity assumes that the full interest tax shield is earned every year.
Under `zero`, the years with a limited or no shield earn less: the tax rate behind the cost of
debt of those years should be the lower effective rate the limited shield implies, for example
through rates per period in the DCF, or the report should say why the full shield is assumed.
Fairmark does not adjust the discount rate.

## Example

Damodaran's Boeing illustration, from the published EBIT (1 − tax rate) of 1,138 ($ million),
and a two-period plan with FCFE:

```python
from decimal import Decimal

import fairmark as fm

boeing = fm.dcf.free_cash_flows(
    [
        {
            "ebit": 1753,
            "operating_taxes": 615,  # 1,753 − 1,138, as published
            "depreciation_and_amortisation": 1517,
            "capital_expenditure": 1584,
            "change_in_net_working_capital": -105,  # a decrease releases cash
        }
    ]
)
assert boeing.free_cash_flows_to_firm == [Decimal("1176")]

plan = fm.dcf.free_cash_flows(
    [
        {
            "ebit": "1200",
            "depreciation_and_amortisation": "350",
            "capital_expenditure": "420",
            "change_in_net_working_capital": "65",
            "interest_expense": "80",
            "net_borrowing": "-150",
        },
        {
            "ebit": "-250",
            "depreciation_and_amortisation": "90",
            "capital_expenditure": "60",
            "change_in_net_working_capital": "-35",
            "other_adjustments": [{"label": "Disposal of a warehouse", "value": "60"}],
            "interest_expense": "30",
            "net_borrowing": "100",
        },
    ],
    notional_tax_rate="0.24",
    negative_ebit="zero",
)
assert plan.free_cash_flows_to_firm == [Decimal("777.00"), Decimal("-125")]
assert plan.free_cash_flows_to_equity == [Decimal("566.20"), Decimal("-55")]
print(plan.conventions[-1])  # no interest tax shield in period 2 (negative EBIT, treatment zero)
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `periods` | Line items of periods 1..n, in order; at least one | required |
| `periods[].ebit` | EBIT of the period, currency units, any sign | required |
| `periods[].operating_taxes` | Taxes on EBIT as an amount, positive for a tax paid (amounts form) | none |
| `periods[].depreciation_and_amortisation` | Depreciation and amortisation, zero or more, added back | required |
| `periods[].capital_expenditure` | Capital expenditure, zero or more, subtracted | required |
| `periods[].change_in_net_working_capital` | Change in net working capital: positive for an increase, which absorbs cash | required |
| `periods[].other_adjustments` | Labelled amounts `{label, value}`, positive for an inflow | none |
| `periods[].interest_expense` | Interest expense, zero or more; with `net_borrowing`, for every period or for none | none |
| `periods[].net_borrowing` | Net borrowing: positive for new borrowing, negative for net repayment | none |
| `notional_tax_rate` | Tax rate on EBIT, decimal fraction from 0 to below 1 (rate form) | none |
| `negative_ebit` | `credit` or `zero`; required with `notional_tax_rate`, not accepted with `operating_taxes` | none: must be chosen |
| `interest_tax_rate` | Rate of the interest tax shield, decimal fraction from 0 to below 1; required with `operating_taxes` and the financing lines, not accepted otherwise | none |

Give either `notional_tax_rate` or `operating_taxes` in every period, never both.

## Outputs

| Field | Meaning |
| --- | --- |
| `periods` | One row per period: every line as given, `operating_taxes` used, `ebit_after_taxes`, `other_adjustments_total`, `free_cash_flow_to_firm` and, with the financing lines, `interest_tax_rate` applied (0 in a period with a negative EBIT under `zero`), `interest_tax_shield` (under `zero` limited to the taxable income available), `interest_after_taxes` (interest − shield) and `free_cash_flow_to_equity` (`null` otherwise) |
| `free_cash_flows_to_firm` | FCFF of periods 1..n |
| `free_cash_flows_to_equity` | FCFE of periods 1..n; `null` without the financing lines |
| `tax_form` | `rate_on_ebit` or `amounts` |
| `negative_ebit` | Treatment used in the rate form; `null` in the amounts form |
| `interest_tax_rate` | Rate of the interest tax shield of the tax form (the tax rate on EBIT, or the given `interest_tax_rate`); `null` without the financing lines. The rate applied in each period is `periods[].interest_tax_rate`, 0 where the `zero` treatment applies |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Taxes on EBIT | rate on EBIT, or amounts per period | none: exactly one must be given |
| Negative EBIT (rate form) | `credit`, `zero` | none: must be chosen |
| Tax loss carry-forward | not modelled | fixed |
| Rate of the interest tax shield | the tax rate on EBIT (rate form), or `interest_tax_rate` (amounts form); 0 in a period with a negative EBIT under `zero` | fixed by the tax form and the treatment |
| Interest tax shield under `zero` | limited to the taxable income available, rate × min(interest, max(EBIT, 0)); each period where the limit binds is reported with its values | fixed by the treatment |
| Sign of the taxes on EBIT (amounts form) | positive for a tax paid, subtracted; negative for a tax credit | fixed |
| Sign of the change in net working capital | positive for an increase, subtracted | fixed |
| Sign of net borrowing | positive for new borrowing, added | fixed |
| Sign of adjustments | positive for an inflow, added | fixed |
| Amounts | currency units per period, not discounted | fixed |

Disposals of fixed assets and other inflows go in `other_adjustments` with a positive sign:
capital expenditure and depreciation are never negative.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A negative depreciation and amortisation, capital expenditure or interest expense (the message names the field by its JSON pointer and the period); both or neither tax form; tax amounts missing in a period; `negative_ebit` missing with a tax rate or given with amounts; `interest_tax_rate` missing or not accepted; interest expense without net borrowing, or financing lines in some periods only; a tax rate outside 0 to below 1; no period; an empty adjustment label; a field missing or unknown |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Additions, subtractions and one multiplication per tax: exact up to the 28 significant digits
of the decimal type. The golden cases are compared exactly, or to the 28th significant digit
when a product needs more digits, against a 50-digit reference. The Boeing case of Damodaran's
illustration is compared at its published precision, in units (tolerance 0.5).

## Verified against

- `tests/golden/dcf.free_cash_flows/damodaran-boeing-illustration-15-7.yaml`: Damodaran,
  *Investment Valuation*, 2nd edition, Illustration 15.7 (Boeing), FCFF 1,176 at the published
  precision. The illustration prints EBIT (1 − tax rate) as 1,138, while
  1,753 × (1 − 0.35) = 1,139.45; the case starts from the printed 1,138, and
  `boeing-rate-form-recomputed.yaml` records the FCFF of 1,177.45 that the rate form gives.
- The other golden cases and a seeded comparison on random plans come from an independent
  50-digit computation (`scripts/reference_free_cash_flows.py`).
- The interest tax shield under both treatments (`shield-*.yaml`: EBIT negative, below, equal
  to and above the interest, EBIT zero, interest zero) is cross-checked by the net-income route
  (net income + D&A − capex − ΔNWC + adjustments + net borrowing), computed separately in the
  same reference: under `zero` with no tax on a pre-tax loss, under `credit` with a tax saving.
