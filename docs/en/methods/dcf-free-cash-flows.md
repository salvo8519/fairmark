# Free cash flow build-up (`dcf.free_cash_flows`)

Builds the free cash flow to the firm (FCFF) of every period of a business plan from its
reclassified line items: EBIT, taxes on EBIT, depreciation and amortisation, capital
expenditure, change in net working capital and labelled adjustments. When interest expense and
net borrowing are also given, it builds the free cash flow to equity (FCFE). The flows are
amounts per period, not discounted: discounting them is the job of the DCF. Reclassifying the
statutory statements into these lines is a separate step.

Method version: 1. Status: preview.

## Formula

For every period $t$:

$$
FCFF_t = EBIT_t - T_t + DA_t - CAPEX_t - \Delta NWC_t + \sum_k ADJ_{k,t}
$$

$$
FCFE_t = FCFF_t - INT_t \times (1 - \tau) + NB_t
$$

The taxes on EBIT $T_t$ are given in exactly one of two forms:

- **rate on EBIT**: $T_t = \tau \times EBIT_t$, with the tax rate $\tau$ given once for all
  periods. When $EBIT_t < 0$ the required treatment decides: `credit` keeps
  $T_t = \tau \times EBIT_t$ (a tax saving), `zero` sets $T_t = 0$. Tax loss carry-forwards are
  not modelled;
- **amounts**: $T_t$ given for every period.

The rate $\tau$ of the interest tax shield in the FCFE is the tax rate on EBIT in the first
form, and a separate required input, `interest_tax_rate`, in the second. It is applied to the
interest as given, whatever the sign of EBIT and the treatment of a negative EBIT.

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
assert plan.free_cash_flows_to_equity == [Decimal("566.20"), Decimal("-47.80")]
print(plan.conventions[-1])  # negative EBIT in period 2: treatment zero applied
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
| `periods` | One row per period: every line as given, `operating_taxes` used, `ebit_after_taxes`, `other_adjustments_total`, `free_cash_flow_to_firm` and, with the financing lines, `interest_after_taxes` and `free_cash_flow_to_equity` (`null` otherwise) |
| `free_cash_flows_to_firm` | FCFF of periods 1..n |
| `free_cash_flows_to_equity` | FCFE of periods 1..n; `null` without the financing lines |
| `tax_form` | `rate_on_ebit` or `amounts` |
| `negative_ebit` | Treatment used in the rate form; `null` in the amounts form |
| `interest_tax_rate` | Rate of the interest tax shield used; `null` without the financing lines |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Taxes on EBIT | rate on EBIT, or amounts per period | none: exactly one must be given |
| Negative EBIT (rate form) | `credit`, `zero` | none: must be chosen |
| Tax loss carry-forward | not modelled | fixed |
| Rate of the interest tax shield | the tax rate on EBIT (rate form), or `interest_tax_rate` (amounts form) | fixed by the tax form |
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
