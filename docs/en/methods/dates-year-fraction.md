# Year fraction (`dates.year_fraction`)

Returns the fraction of a year between two calendar dates under one of five market day-count
conventions. Every dated method in Fairmark (`tvm.xnpv`, `tvm.xirr`) measures time in years
with this function, so the choice of day count is visible and auditable. The result also
reports the actual calendar days between the two dates, and `explain` shows the day numerator
and denominator.

## Formula

For a start date $D_1 = (Y_1, M_1, d_1)$ and an end date $D_2 = (Y_2, M_2, d_2)$, with
$\text{days} = D_2 - D_1$ in actual calendar days:

| Day count | `day_count` | Year fraction |
| --- | --- | --- |
| Actual/365 (Fixed) | `act_365_fixed` | $\text{days} / 365$ |
| Actual/360 | `act_360` | $\text{days} / 360$ |
| Actual/Actual (ISDA) | `act_act_isda` | $\dfrac{\text{days in non-leap years}}{365} + \dfrac{\text{days in leap years}}{366}$ |
| 30/360 (Bond Basis) | `thirty_360` | $\dfrac{360(Y_2 - Y_1) + 30(M_2 - M_1) + (d_2 - d_1)}{360}$ |
| 30E/360 (Eurobond Basis) | `thirty_e_360` | same formula as 30/360 |

The 30/360 variants differ only in how day 31 is treated:

- **30/360 (Bond Basis):** if $d_1 = 31$, set $d_1 = 30$; if $d_2 = 31$ and $d_1 \ge 30$
  (after the first adjustment), set $d_2 = 30$.
- **30E/360 (Eurobond Basis):** every day 31 becomes 30, for both dates.

The formula step in `explain` states each adjustment made, and a day 31 kept at the end under
30/360 (Bond Basis) because the start day is below 30.

Under Actual/Actual (ISDA), when the two dates fall in different years the period is split at
each 1 January: the days to the end of the first year over that year's length, plus one for
each whole year in between, plus the days in the last year over that year's length. The start
date is included and the end date is excluded.

## Example

1 November 2003 to 1 May 2004 under Actual/Actual (ISDA) is 61 days in 2003 (a 365-day year)
and 121 days in 2004 (a 366-day year).

```python
from decimal import Decimal

import fairmark as fm

r = fm.dates.year_fraction("2003-11-01", "2004-05-01", "act_act_isda")
print(r.actual_days)        # 182
print(r.steps[-1].formula)  # 61 / 365 + 0 + 121 / 366
assert round(r.year_fraction, 20) == Decimal("0.49772438056740774010")

# The two 30/360 variants differ only on day 31.
bond = fm.dates.year_fraction("2024-02-15", "2024-03-31", "thirty_360")
euro = fm.dates.year_fraction("2024-02-15", "2024-03-31", "thirty_e_360")
assert bond.year_fraction == Decimal(46) / 360
assert euro.year_fraction == Decimal("0.125")  # 45 / 360
```

Dates are ISO 8601 strings (`"2024-06-30"`) or `datetime.date` objects.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `start` | Start date, ISO 8601 `YYYY-MM-DD` | required |
| `end` | End date, ISO 8601 `YYYY-MM-DD`, not earlier than `start` | required |
| `day_count` | `act_365_fixed`, `act_360`, `act_act_isda`, `thirty_360` or `thirty_e_360` | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `year_fraction` | Fraction of a year between the dates, in years |
| `actual_days` | Actual calendar days between the dates |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Calendar | proleptic Gregorian dates, no time of day or time zone, 1900-01-01 to 2199-12-31 | fixed |
| Day count | one of the five above | none: must be chosen |

Defaults exist only where a spreadsheet function defines one: Actual/365 (Fixed) for
`tvm.xnpv` and `tvm.xirr`. Actual/Actual (ICMA), which needs a coupon schedule, and
business-day adjustments are out of scope.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `end` precedes `start` (the message asks to swap them), a field is missing or unknown, or `day_count` is not one of the five options |
| `FM-E009` | A date is malformed (not exactly `YYYY-MM-DD`), does not exist (`2023-02-29`) or is outside 1900-01-01 to 2199-12-31 |

## Accuracy

Year fractions are ratios of integers, rounded once in the final division. Measured against an
independent 60-digit computation, they agree to about $10^{-24}$ relative or better. The
golden cases are checked to $10^{-19}$.

## Verified against

- Definitions: 2006 ISDA Definitions, section 4.16.
- Published example: the ISDA worked example for Actual/Actual (ISDA), $61/365 + 121/366$.
- The same conventions as the QuantLib day counters, and agrees with `pyxirr.year_fraction`
  for all five day counts.
