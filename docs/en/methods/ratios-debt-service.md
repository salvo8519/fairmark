# Debt service ratios (`ratios.debt_service`)

Answers the question "can the company service its debt?". Returns the interest cover on EBIT
and on EBITDA, the net financial position to EBITDA, the debt service cover ratio (DSCR) of each
period of a loan schedule and the loan life cover ratio (LLCR).

## Formula

$$
\text{interest cover} = \frac{\text{EBIT}}{\text{interest}} \;\text{and}\; \frac{\text{EBITDA}}{\text{interest}}
\qquad
\frac{\text{NFP}}{\text{EBITDA}}
$$

$$
\text{DSCR}_t = \frac{\text{CFADS}_t}{\text{debt service}_t}
\qquad
\text{LLCR} = \frac{\sum_{t=1}^{n} \text{CFADS}_t \,/\, (1 + r)^t}{\text{debt outstanding}}
$$

where CFADS is the cash flow available for debt service, $r$ the discount rate per period of
the schedule, $n$ the number of periods up to the final maturity and the debt outstanding is
measured at the start of the first period. Discounting is the one of
[`tvm.present_value`](tvm-present-value.md), at the end of each period.

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.ratios.debt_service(
    balances="closing",
    ebitda_definition="reported EBITDA, no adjustments",
    interest_definition="gross interest expense",
    income_statement={"ebitda": 800, "ebit": 500, "interest": 100},
    closing={"net_financial_position": {"gross_debt": 2200, "cash": 300}},
    debt_schedule={
        "discount_rate": "0.06",
        "debt_outstanding": 1000,
        "periods": [{"cfads": 250, "debt_service": 200}] * 5,
    },
)
assert r.interest_cover_ebit["value"] == Decimal("5")  # 500 / 100
assert r.nfp_to_ebitda["value"] == Decimal("2.375")  # 1900 / 800
assert r.dscr[0]["value"] == Decimal("1.25")  # 250 / 200
# Level CFADS: LLCR = 250 × (1 − 1.06^−5) / 0.06 / 1000
assert abs(r.llcr["value"] - Decimal("1.05309094639142847003375")) < Decimal("1e-20")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `balances` | `closing`, or `average` of the opening and closing net financial position | required |
| `ebitda_definition` | How EBITDA was measured, in words, not empty; echoed in the conventions | required |
| `interest_definition` | How interest was measured (gross, or net of interest income), in words, not empty; echoed in the conventions | required |
| `income_statement` | `ebitda`, `ebit`, `interest` of the period, currency units | required |
| `closing` | `net_financial_position` with `gross_debt`, `cash` and `other_items` | required |
| `opening` | Same field at the start of the period | required with average balances, not used otherwise |
| `debt_schedule` | `discount_rate` per period (decimal fraction), `debt_outstanding` at the start of the first period, `periods`: a list of `{"cfads", "debt_service"}` up to the final maturity | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `net_financial_position` | NFP used by the ratios, currency units |
| `interest_cover_ebit` | EBIT / interest, multiple |
| `interest_cover_ebitda` | EBITDA / interest, multiple |
| `nfp_to_ebitda` | NFP / EBITDA, multiple |
| `dscr` | CFADS / debt service of each period, multiple, in the order of the schedule |
| `pv_cfads` | Present value of the CFADS at the start of the first period, currency units |
| `llcr` | Present value of CFADS / debt outstanding, multiple |

Ratios are objects `{"value": ..., "not_defined": ...}`, as in every ratio group.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing (net financial position) | required |
| EBITDA definition | as given by the analyst, non-empty | required |
| Interest definition | as given by the analyst (gross, or net of interest income), non-empty | required |
| Debt service | as given | fixed |
| LLCR discounting | end of each period, at `discount_rate` per period | fixed |
| Not defined | zero denominator (interest, EBITDA, debt service of a period, debt outstanding) | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Average balances without `opening`; an empty or blank `ebitda_definition` or `interest_definition`; a field is missing or unknown |
| `FM-E002` | The debt schedule has no periods |
| `FM-E003` | Discount rate at or below −100% per period |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Ratios are one decimal division each; discount factors are integer powers of $1 + r$. Results
are exact up to the 28 significant digits of the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.debt_service/`, whose expected values come from
  `scripts/reference_ratios.py` (Python decimal at 50 digits); the LLCR of a level CFADS equals
  the annuity closed form $C\,(1 - (1 + r)^{-n}) / r$ divided by the debt outstanding.
