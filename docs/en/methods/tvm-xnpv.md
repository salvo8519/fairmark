# Net present value of dated flows (`tvm.xnpv`)

Discounts cash flows that occur on calendar dates, such as a closing, a distribution or an
earn-out, to a valuation date. The time of each flow is its year fraction from the valuation
date under the chosen day count, and the rate is an annual rate with an explicit compounding.
With the defaults, the result is that of the spreadsheet `XNPV` function.

## Formula

$$
XNPV = \sum_{j} CF_j \cdot DF(r, t_j),
\qquad t_j = \text{year fraction}(D_0, D_j)
$$

$$
DF(r, t) = \left(1 + \frac{r}{m}\right)^{-m t}
\qquad\text{or}\qquad
DF(r, t) = e^{-r t}\ \text{(continuous compounding)}
$$

where $D_0$ is the valuation date, $D_j$ the date of flow $j$, $m$ the compounding periods per
year and the year fraction is computed as in [`dates.year_fraction`](dates-year-fraction.md).
With the defaults (Actual/365 (Fixed), annual compounding, valuation at the earliest flow):

$$
XNPV = \sum_{j} \frac{CF_j}{(1 + r)^{(D_j - D_1)/365}}
$$

## Example

Rate 9%, five flows from 1 January 2008 to 1 April 2009. The published value is 2086.65,
rounded to two decimals.

```python
from decimal import Decimal

import fairmark as fm

flows = [
    ("2008-01-01", -10000),
    ("2008-03-01", 2750),
    ("2008-10-30", 4250),
    ("2009-02-15", 3250),
    ("2009-04-01", 2750),
]
r = fm.tvm.xnpv("0.09", flows)
assert round(r.xnpv, 2) == Decimal("2086.65")
assert abs(r.xnpv - Decimal("2086.64760203153662166361")) < Decimal("1e-15")
print(r.valuation_date)  # 2008-01-01, the earliest flow date
print(r.conventions[0])  # day count: Actual/365 (Fixed), ... (default, ECMA-376)

# The same flows valued six months earlier.
early = fm.tvm.xnpv("0.09", flows, valuation_date="2007-07-01")
assert abs(early.xnpv - Decimal("1997.93821626701985989269")) < Decimal("1e-15")
```

Dates are ISO 8601 strings or `datetime.date` objects; the flows may be in any order.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `rate` | Annual discount rate, decimal fraction (0.09 = 9%), quoted with `compounding` | required |
| `cash_flows` | Dated flows `{date, amount}`; negative amounts are payments, positive are receipts | required |
| `valuation_date` | Date at which the value is computed; no flow may precede it | earliest flow date |
| `day_count` | `act_365_fixed`, `act_360`, `act_act_isda`, `thirty_360` or `thirty_e_360` | `act_365_fixed` |
| `compounding` | `annual`, `semi_annual`, `four_monthly` (3 a year), `quarterly`, `bimonthly` (every two months, 6 a year), `monthly` or `continuous` | `annual` |

## Outputs

| Field | Meaning |
| --- | --- |
| `xnpv` | Net present value at the valuation date |
| `valuation_date` | The valuation date used, given or defaulted |
| `flows` | For each flow: `date`, `year_fraction` (years from the valuation date), `discount_factor`, `present_value` |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Day count | five day counts, see [`dates.year_fraction`](dates-year-fraction.md) | Actual/365 (Fixed), the ECMA-376 spreadsheet default |
| Compounding | annual, semi-annual, four-monthly, quarterly, bimonthly, monthly, continuous | annual, the ECMA-376 spreadsheet default |
| Valuation date | any date not later than the earliest flow | earliest flow date |

Each convention is reported with whether it was given or is the default. ECMA-376 values at
the first listed flow and rejects earlier ones; defaulting to the earliest flow gives the same
value whenever the spreadsheet accepts the input.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A field is missing or unknown, or a convention is not one of the listed options |
| `FM-E002` | `cash_flows` is empty |
| `FM-E003` | The rate per compounding period, $r/m$, is −100% or lower; the message names `/rate` and, with more than one compounding period a year, the bound $-m$ |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |
| `FM-E009` | A date is malformed, does not exist or is outside 1900-01-01 to 2199-12-31 |
| `FM-E010` | A flow is dated before the valuation date |

## Accuracy

Year fractions are exact ratios, and flows at a whole number of compounding periods use exact
integer powers; measured against an independent 60-digit computation, these agree to about
$10^{-24}$ relative or better. Fractional exponents use the decimal power function and
continuous compounding the decimal exponential; these agree to about $10^{-18}$ relative or
better on typical inputs. Negative rates are discounted through the reciprocal of $1 + r/m$,
keeping full relative precision. The full-precision golden cases are checked to $10^{-19}$.

## Verified against

- Published example: ECMA-376 / Excel `XNPV` example, 2086.65.
- Golden cases in `tests/golden/tvm.xnpv/` from an independent 50-digit computation
  (`scripts/reference_time.py`), including the same flows discounted with `four_monthly`
  compounding.
- Agrees with the `XNPV` function of Excel and LibreOffice and with `pyxirr.xnpv`.
- Day counts: definitions of the 2006 ISDA Definitions, section 4.16.
