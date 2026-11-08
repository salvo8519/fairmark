# Schedule of accrual periods (`dates.schedule`)

Builds the periods of a model from a start date, a period length and a number of periods: the
start date and end date of every period and, under a day count, its year fraction. Use it
instead of typing dates by hand, so that month ends, February and leap years are handled once,
by a stated rule, and the dates feed [`tvm.xnpv`](tvm-xnpv.md), [`tvm.xirr`](tvm-xirr.md) and
[`series.project`](series-project.md) without retyping.

## What a period is: [start, end)

Every period runs **from and including its start date to but excluding its end date**, as the
calculation periods of the 2006 ISDA Definitions. The end date of a period is the first day of
the next period: it belongs to the next period, not to this one. There is no "last day of the
period" field, so an end date is never counted twice.

```text
start date                                                         end date of period 3
    |                                                                         |
    [ period 1 ................)[ period 2 ................)[ period 3 ......)
 2025-12-31              2026-03-31                  2026-06-30          2026-09-30
    ^ included                  ^ start of period 2, not part of period 1
```

A quarterly schedule that starts on 1 January has its end dates on 1 April, 1 July, 1 October
and 1 January: its periods are the calendar quarters (1 January to 31 March inclusive, 1 April
to 30 June inclusive, and so on), each end date being the first day of the next quarter. A
quarterly schedule that starts on 31 December has its end dates on 31 March, 30 June, 30
September and 31 December: the 31 December schedule covers 31 December to 30 March inclusive
(90 days), one day earlier than the calendar quarter, and so on for every period. Both are
exact. Start on 1 January when the periods must be the calendar quarters; start on 31 December
when end-of-period values must be dated on month ends (31 March, 30 June, ...).

## Formula

With start date $d_0$, $p$ months per period and $n$ periods:

$$
d_k = d_0 + k \times p \text{ months}, \qquad
\text{period } k = [d_{k-1},\ d_k), \qquad k = 1, \dots, n
$$

Every date is generated from the start date, never from the previous date, so a day shortened
in one month is restored in the next: from 31 January, monthly, the dates are 29 February
(2024) and then 31 March, not 29 March. The day of each date follows one fixed rule, the
end-of-month convention of market practice (the `endOfMonth` rule of QuantLib `Schedule`):

- when the start date is the **last day of its month**, every date is the last day of its
  month (30 June, quarterly: 30 September, 31 December, 31 March, 30 June);
- otherwise every date keeps the **day of the start date**, or the last day of the month when
  that month is shorter (30 January, monthly: 28 February, 30 March, 30 April).

With a day count, the year fraction of period $k$ is that of
[`dates.year_fraction`](dates-year-fraction.md) from $d_{k-1}$ to $d_k$, the same function,
not a second implementation.

## Period length

Give the period length in exactly one of two forms. A named frequency and the equal number of
months give identical results.

| `frequency` | Months per period | Periods a year |
| --- | --- | --- |
| `monthly` | 1 | 12 |
| `bimonthly` (every two months, not twice a month) | 2 | 6 |
| `quarterly` | 3 | 4 |
| `four_monthly` (every four months) | 4 | 3 |
| `semi_annual` | 6 | 2 |
| `annual` | 12 | 1 |

`period_months` is the tenor as a positive whole number of months: 1, 2, 3, 4, 6, 12, and any
other such as 5, 18 or 24. It states months per period, not periods a year, so it cannot be
confused with a compounding frequency. When the months per period divide 12 the result also
reports `periods_per_year`; for 5, 18 or 24 months it is absent.

## Examples

End dates of the end-of-month rule, each checked against QuantLib and python-dateutil:

| Start | Period length | End dates |
| --- | --- | --- |
| 2024-01-31 | `monthly` | 2024-02-29, 2024-03-31, 2024-04-30, 2024-05-31 |
| 2025-01-30 | `monthly` | 2025-02-28, 2025-03-30, 2025-04-30 (not a month end: day 30 kept) |
| 2023-02-28 | `monthly` | 2023-03-31, 2023-04-30, 2023-05-31 (28 February 2023 is a month end) |
| 2024-02-29 | `annual` | 2025-02-28, 2026-02-28, 2027-02-28, 2028-02-29 |
| 2025-01-31 | `four_monthly` | 2025-05-31, 2025-09-30, 2026-01-31 |
| 2025-01-31 | `period_months` 5 | 2025-06-30, 2025-11-30, 2026-04-30 |
| 2025-03-15 | `period_months` 18 | 2026-09-15, 2028-03-15 |

```python
from decimal import Decimal

import fairmark as fm

q = fm.dates.schedule("2025-12-31", 4, frequency="quarterly", day_count="act_365_fixed")
print(q.end_dates)       # ['2026-03-31', '2026-06-30', '2026-09-30', '2026-12-31']
print(q.start_dates[1])  # 2026-03-31: the end date of period 1 starts period 2
print(q.year_fractions)  # 90/365, 91/365, 92/365, 92/365
assert q.start_dates[1:] == q.end_dates[:-1]
print(q.steps[-1].formula)  # 365 / 365: the total year fraction, numerators added first
assert q.steps[-1].value == Decimal(365) / 365  # total days / 365 exactly

jan = fm.dates.schedule("2026-01-01", 4, frequency="quarterly")
print(jan.end_dates)     # ['2026-04-01', '2026-07-01', '2026-10-01', '2027-01-01']

# A tenor without a name, and the named frequency equal to a number of months.
assert fm.dates.schedule("2025-03-15", 2, period_months=18).end_dates == [
    "2026-09-15",
    "2028-03-15",
]
assert (
    fm.dates.schedule("2025-12-31", 4, period_months=3).end_dates == q.end_dates
)
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `start_date` | Start date of the first period, ISO 8601 `YYYY-MM-DD` | required |
| `frequency` | Named frequency: `monthly`, `bimonthly`, `quarterly`, `four_monthly`, `semi_annual` or `annual` | exactly one of `frequency` and `period_months` |
| `period_months` | Months per period, a positive whole number | exactly one of `frequency` and `period_months` |
| `number_of_periods` | Number of periods, from 1 to 10000 | required |
| `day_count` | `act_365_fixed`, `act_360`, `act_act_isda`, `thirty_360` or `thirty_e_360` | none: no year fraction is computed |

## Outputs

| Field | Meaning |
| --- | --- |
| `start_dates` | Start date of each period, included in the period |
| `end_dates` | End date of each period, the first day of the next period, excluded from the period |
| `year_fractions` | Year fraction of each period on [start, end), only with a day count |
| `months_per_period` | Months per period |
| `periods_per_year` | 12 / months per period, only when the months per period divide 12 |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Period definition | [start, end): from and including the start date, to but excluding the end date | fixed |
| Date generation | from the start date, $k \times p$ months (forward, no drift) | fixed |
| End-of-month rule | a month-end start keeps month ends; otherwise the start day, clamped to the month length; `explain` states whether the start is a month end and how many end dates fall on another day | fixed |
| Business days | none: dates are unadjusted | fixed in version 1 |
| Day count | the five day counts of [`dates.year_fraction`](dates-year-fraction.md) | none: without it no year fraction is computed |

An end date, a termination date, a calendar, a business-day convention or a stub period are not
part of version 1 and are refused as unknown fields.

`explain` lists the number of periods, the months per period, the periods a year when defined,
the actual days of the whole schedule and, with a day count, the year fraction of period 1 with
its calculation and the total year fraction of the schedule, with its exact terms
substituted (for example `121 / 365`, or `a / 365 + b / 366 + w` under Actual/Actual (ISDA)).
The per-period numbers are in the output
vectors, so the number of steps does not grow with the number of periods.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | Both `frequency` and `period_months`, or neither (the message names both pointers); `period_months` of 0; `number_of_periods` outside 1 to 10000; an unknown field such as `end_date`, `calendar`, `business_day_convention` or `stub` |
| `FM-E009` | `start_date` is malformed, does not exist (`2023-02-29`) or is out of range; a generated end date falls after 2199-12-31: the message names `/number_of_periods`, the first date out of range and the last period that fits |

## Accuracy

Dates are integer arithmetic on months and days: exact. Year fractions are those of
[`dates.year_fraction`](dates-year-fraction.md), ratios of integers rounded once. The total year
fraction of the schedule in `explain` adds the integer numerators before dividing, so under
Actual/365 (Fixed) it equals the total days of the schedule / 365 exactly. Because each
returned `year_fractions` value is rounded once to 28 significant digits, the total can differ
from the sum of the returned values by up to $n$ half-units of their last digit.

## Verified against

- QuantLib 1.44 `Schedule` (null calendar, `Unadjusted`, forward generation, `endOfMonth`
  true, tenor `Period(p, Months)`), and python-dateutil 3.0.0 (`start + relativedelta(months=k
  × p, day=31)` for a month-end start, `start + relativedelta(months=k × p)` otherwise): every
  golden case in `tests/golden/dates.schedule/` is built by both libraries, which agree
  (`scripts/reference_schedule.py`), and the oracle tests compare random schedules with both.
  The day clamping of python-dateutil was read in its source (`relativedelta.__add__`: the day
  is the smaller of the given day and the length of the month).
- Year fractions: the 50-digit ISDA reference of `scripts/reference_time.py`, confirmed by the
  QuantLib day counters `Actual365Fixed`, `Actual360`, `ActualActual(ISDA)`,
  `Thirty360(BondBasis)` and `Thirty360(European)`.

## Sources

- 2006 ISDA Definitions: "Calculation Period" (from and including one Period End Date to but
  excluding the next), "Period End Date", "Effective Date"; section numbers to verify. The day
  counts, section 4.16, are verified for [`dates.year_fraction`](dates-year-fraction.md).
- End-of-month convention: the rule applied is the one stated above, the behaviour of QuantLib
  `Schedule` with `endOfMonth` true; its statement in the 2006 ISDA Definitions is to verify.
- QuantLib `Frequency` values (Monthly 12, Bimonthly 6, Quarterly 4, EveryFourthMonth 3,
  Semiannual 2, Annual 1) and `Period(n, Months)`; python-dateutil `relativedelta`.
