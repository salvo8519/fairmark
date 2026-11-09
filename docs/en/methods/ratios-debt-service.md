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
\text{LLCR} = \frac{\sum_{t=1}^{n} \text{CFADS}_t \,/\, (1 + r)^t + \text{reserve balance}}{\text{debt outstanding}}
$$

where CFADS is the cash flow available for debt service, $r$ the discount rate per period of
the schedule (the cost of debt per period chosen by the analyst), the reserve balance the debt
service reserve account at the valuation date, added undiscounted (zero when not given), $n$ the number of periods up to the final maturity and the debt outstanding is
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

# Without a debt schedule, DSCR and LLCR are not defined; the other ratios are computed.
s = fm.ratios.debt_service(
    balances="closing",
    ebitda_definition="reported EBITDA, no adjustments",
    interest_definition="gross interest expense",
    income_statement={"ebitda": 800, "ebit": 500, "interest": 100},
    closing={"net_financial_position": {"gross_debt": 2200, "cash": 300}},
)
assert s.llcr["not_defined"] == "no debt schedule given"
assert s.interest_cover_ebitda["value"] == Decimal("8")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `balances` | `closing`, or `average` of the opening and closing net financial position | required |
| `ebitda_definition` | How EBITDA was measured, in words, not empty when given; echoed in the conventions | optional: without it the ratios on EBITDA are not computed |
| `interest_definition` | How interest was measured (gross, or net of interest income), in words, not empty when given; echoed in the conventions | optional: without it the interest covers are not computed |
| `income_statement` | `ebitda`, `ebit`, `interest` of the period, currency units | each amount optional |
| `closing` | `net_financial_position` with `gross_debt`, `cash` and `other_items` | each amount optional |
| `opening` | Same field at the start of the period | used with average balances, not used otherwise; each amount optional |
| `debt_schedule` | `discount_rate`: the cost of debt per period chosen by the analyst (decimal fraction); `debt_outstanding` at the start of the first period; optional `reserve_balance` (debt service reserve account at the valuation date); `periods`: a list of `{"cfads", "debt_service"}` up to the final maturity, at least one | optional: without it DSCR and LLCR are not defined; every amount in it optional |

## Outputs

| Field | Meaning |
| --- | --- |
| `net_financial_position` | NFP used by the ratios, currency units |
| `interest_cover_ebit` | EBIT / interest, multiple |
| `interest_cover_ebitda` | EBITDA / interest, multiple |
| `nfp_to_ebitda` | NFP / EBITDA, multiple |
| `dscr` | CFADS / debt service of each period, multiple, in the order of the schedule; empty without a debt schedule |
| `pv_cfads` | Present value of the CFADS at the start of the first period, currency units; null without a debt schedule |
| `llcr` | (Present value of CFADS + reserve balance) / debt outstanding, multiple |
| `not_computed` | Outputs not computed because an input is missing, in the order of the outputs: `{"output", "missing", "reason"}`, with `output` the JSON pointer of the output in the result value and `missing` the sorted JSON pointers of the inputs it lacks; empty when every output is computed |

Ratios are objects `{"value": ..., "not_defined": ...}`, as in every ratio group.

## Partial input

Every amount is optional (version 2, SPEC-0007 R7). Give what the statements show: each output
is computed when its own amounts are given; otherwise it is null and listed in `not_computed`
with the JSON pointers of the inputs it lacks. A missing amount is never taken as zero and is
not an error by itself. The floor: at least one output must have every input it needs (a ratio
not defined by its own inputs counts); below it the input is refused with `FM-E027`, whose
message lists the alternative sets of inputs that would lift it, so a result made only of nulls
is never returned. With average balances an amount needs both `/closing/<field>`
and `/opening/<field>`. A given amount that already makes a ratio not defined (a zero
denominator, for example) decides first: the ratio is then not defined, with its reason, and not
listed.

| Output | Inputs it needs |
| --- | --- |
| `net_financial_position` | gross debt, cash, the amount of each other item given |
| `interest_cover_ebit` | EBIT, interest, `interest_definition` |
| `interest_cover_ebitda` | EBITDA, `ebitda_definition`, interest, `interest_definition` |
| `nfp_to_ebitda` | the net financial position, EBITDA, `ebitda_definition` |
| `dscr` (each period) | the CFADS and the debt service of the period |
| `pv_cfads` | `discount_rate` and the CFADS of every period |
| `llcr` | the present value of CFADS, `debt_outstanding` |

Without a debt schedule `dscr` is empty and `pv_cfads` and `llcr` are listed with
`/debt_schedule`; `llcr` keeps its version 1 reason, "no debt schedule given".

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Balances | closing, average of opening and closing (net financial position) | required |
| EBITDA definition | as given by the analyst, non-empty | optional; without it the ratios on EBITDA are not computed |
| Interest definition | as given by the analyst (gross, or net of interest income), non-empty | optional; without it the interest covers are not computed |
| Debt service | as given | fixed |
| LLCR discounting | end of each period, at `discount_rate` per period, the cost of debt chosen by the analyst | fixed |
| Reserve balance | added undiscounted to the LLCR numerator; when absent and the LLCR has its other inputs, none: reported in `meta.defaults_applied` (`/debt_schedule/reserve_balance`, `"0"`) with a warning, and not written into the input, so `meta.input_hash` differs from a reserve given as `0` | none (documented default) |
| Other items of the net financial position | none when `other_items` is absent and gross debt and cash are given: reported in `meta.defaults_applied` with a warning | none (documented default) |
| Debt schedule | optional; without it DSCR and LLCR are not defined ("no debt schedule given") | none |
| Not defined | zero denominator; interest covers when interest or the numerator (EBIT or EBITDA) is zero or negative; NFP / EBITDA when EBITDA is zero or negative; DSCR when debt service is zero or negative; LLCR when debt outstanding is zero or negative | fixed |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `balances` missing; an empty or blank `ebitda_definition` or `interest_definition`; an unknown field; an amount that is not a decimal |
| `FM-E002` | A debt schedule is given with no periods |
| `FM-E003` | Discount rate at or below −100% per period; the message names `/debt_schedule/discount_rate` |
| `FM-E027` | Below the minimum input: no output has every input it needs; the message lists the alternative sets of inputs (any one is enough) |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Ratios are one decimal division each; discount factors are integer powers of $1 + r$. Results
are exact up to the 28 significant digits of the decimal type.

## Verified against

- Golden cases in `tests/golden/ratios.debt_service/`, whose expected values come from
  `scripts/reference_ratios.py` (Python decimal at 50 digits); the LLCR of a level CFADS equals
  the annuity closed form $C\,(1 - (1 + r)^{-n}) / r$ divided by the debt outstanding. Further
  cases cover the reserve balance, a missing schedule and each not-meaningful sign.
- Golden cases with partial input (interest without its definition, a period without debt
  service, a schedule without the discount rate) and a seeded random comparison on partial
  inputs, against `scripts/reference_ratios_partial.py`.
