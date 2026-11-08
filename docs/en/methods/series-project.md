# Value series projected on a timeline (`series.project`)

Grows a base value period by period with a stated growth rule and returns the value of every
period: revenues of 1,000 grown at 12% a year over twelve monthly periods, a cost line rising
by a different rate each year, a flow whose growth fades linearly to a long-run rate. Use it
instead of computing the row yourself, so that the base period, the conversion of an annual
rate to the period length and the dating of each value are stated, not assumed. With a dated
timeline the result feeds [`tvm.xnpv`](tvm-xnpv.md) and [`tvm.xirr`](tvm-xirr.md) directly.

## Formula

With base value $V_0$ and $g_k$ the growth per period into period $k$:

$$
V_k = V_{k-1} \times (1 + g_k)
$$

computed period by period, never by a closed-form power. The base period says which value the
base is:

| `base_period` | The base value is | Period 1 | Growth steps $s$ |
| --- | --- | --- | --- |
| `period_0` | the value before period 1, for example the last actual year | $V_1 = V_0 (1 + g_1)$ | $n$ |
| `period_1` | the value of period 1 | $V_1 = V_0$ | $n - 1$, into periods 2 to $n$ |

The base period is required: the two readings differ by a full period of growth on every
value. `period_0` is the convention of the DCF stages ([`dcf.fcff`](dcf-fcff.md)).

### Growth rules

| `growth` | Rate of growth step $j = 1, \dots, s$ |
| --- | --- |
| `{"constant": {"growth": g}}` | $g$ |
| `{"linear": {"from_growth": a, "to_growth": b}}` | $a + (b - a) \times j / s$: $a$ is the growth before the first step, $b$ is reached in the last step |
| `{"per_period": {"growth_rates": [...]}}` | the listed rates in order, exactly $s$ of them |

The linear rule is that of the `linear_decline` stage of [`dcf.fcff`](dcf-fcff.md) (Damodaran,
*Investment Valuation*, 2nd edition, Illustration 15.3, "linear increments"), so a series and a
DCF stage with the same base and rates give the same values, bit for bit.

### Growth basis: annual rates and the period length

With $p$ months per period, the rates of the rule are read according to `growth_basis`:

| `growth_basis` | $p$ divides 12 ($m = 12 / p$ periods a year) | $p$ a multiple of 12 ($k = p / 12$ years) |
| --- | --- | --- |
| `per_period` | the rate per period, as given | as given |
| `annual_compound` | $(1 + g)^{1/m} - 1$: compounds to $g$ over a year (the equivalent rate of [`rates.convert`](rates-convert.md), divided by $m$) | $(1 + g)^{k} - 1$, an exact integer power |
| `annual_simple` | $g / m$: $m$ periods compound to $(1 + g/m)^m - 1$, more than $g$ | $g \times k$ |

Twelve monthly periods at 12% a year, from 1,000:

| Basis | Rate per month | Growth over the year | $V_{12}$ |
| --- | --- | --- | --- |
| `annual_compound` | 0.9489% (rounded) | 12.00% | 1,120.00 |
| `annual_simple` | 1.00% | 12.68% | 1,126.83 |

The basis is required when the periods are not 12 months long. With 12-month periods
(`annual` or `period_months` 12) the three bases coincide and it may be omitted; without a
period length the rates are per period by definition. For a period length that neither
divides 12 nor is a multiple of 12 (5, 7 or 18 months) an annual rate would need a fractional
power of a year with no settled convention, so only `per_period` is accepted: give the rates
per period.

## Timeline and dating of values

The timeline is either:

- `{"periods": {"number_of_periods": n}}`, with an optional `frequency` or `period_months`:
  values without dates, for example the columns of a business plan;
- `{"schedule": {...}}`, the input of [`dates.schedule`](dates-schedule.md): the periods are
  built by the same function, so their dates are identical to those of `dates.schedule`.

With a schedule, `timing` is required and dates each value: `end_of_period` at the end date of
its period, `beginning_of_period` at its start date. Periods run from and including the start
date to but excluding the end date, and the end date is the first day of the next period. So
end-of-period values of a quarterly schedule from **1 January** are dated **1 April, 1 July, 1
October and 1 January**; from **31 December** they are dated **31 March, 30 June, 30 September
and 31 December**. Both are exact; the start date decides which reading the model uses.

## Example: schedule, series, XNPV

An outlay of 1,000 on 1 January 2026, then quarterly receipts growing 2% a quarter from a base
of 300, valued at 10% a year:

```python
from decimal import Decimal

import fairmark as fm

timeline = fm.series.schedule(fm.dates.schedule_input("2026-01-01", 4, frequency="quarterly"))
r = fm.series.project(
    300,
    base_period="period_0",
    growth=fm.series.constant("0.02"),
    growth_basis="per_period",
    timeline=timeline,
    timing="end_of_period",
)
print(r.values)  # 306, 312.12, 318.3624, 324.729648
print([f["date"] for f in r.dated_values])  # 2026-04-01, 2026-07-01, 2026-10-01, 2027-01-01

flows = [("2026-01-01", "-1000")] + [(f["date"], f["amount"]) for f in r.dated_values]
v = fm.tvm.xnpv("0.10", flows)
assert round(v.xnpv, 20) == Decimal("188.27000581025293752184")

# 12% a year over twelve monthly periods: compound reaches 1120, simple 1126.83.
compound = fm.series.project(
    1000,
    base_period="period_0",
    growth=fm.series.constant("0.12"),
    growth_basis="annual_compound",
    timeline=fm.series.periods(12, frequency="monthly"),
)
assert abs(compound.values[-1] - 1120) < Decimal("1e-17")
simple = fm.series.project(
    1000,
    base_period="period_0",
    growth=fm.series.constant("0.12"),
    growth_basis="annual_simple",
    timeline=fm.series.periods(12, frequency="monthly"),
)
assert simple.values[-1] == 1000 * Decimal("1.01") ** 12

# Damodaran, Illustration 15.3: growth fading from 13.08% to 5% over five years.
fade = fm.series.project(
    1000, base_period="period_0", growth=fm.series.linear("0.1308", "0.05"),
    timeline=fm.series.periods(5),
)
print([p["stated_growth"] for p in fade.periods])  # 0.11464, 0.09848, 0.08232, 0.06616, 0.05
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `base_value` | Base value $V_0$, in the units of the series; may be negative | required |
| `base_period` | `period_0` or `period_1` | required |
| `growth` | `constant`, `linear` or `per_period`, rates as decimal fractions greater than −1 | required |
| `growth_basis` | `per_period`, `annual_compound` or `annual_simple` | required unless the periods are 12 months long or have no length |
| `timeline` | `periods` (number of periods, optional `frequency` or `period_months`) or `schedule` (a `dates.schedule` input) | required |
| `timing` | `end_of_period` or `beginning_of_period`, with a `schedule` timeline only | required with a `schedule` timeline |

## Outputs

| Field | Meaning |
| --- | --- |
| `periods` | One row per period: `period`, `start_date`, `end_date` and `year_fraction` (with a schedule), `stated_growth` (the rate of the rule), `growth_per_period` (after conversion), `effective_annual_growth` (when the months per period divide 12), `value` |
| `values` | The values of periods 1 to $n$, the shape of `cash_flows` in [`tvm.present_value`](tvm-present-value.md) |
| `dated_values` | With a schedule: one `{date, amount}` per period, the shape of `cash_flows` in [`tvm.xnpv`](tvm-xnpv.md) and [`tvm.xirr`](tvm-xirr.md) |
| `months_per_period` | Months per period, when the timeline has a period length |
| `periods_per_year` | Periods a year, when the months per period divide 12 |

Under `period_1` the growth fields of period 1 are absent: no growth step leads into it.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Base period | `period_0`, `period_1` | none: required |
| Growth rule | `constant`, `linear`, `per_period` | none: required |
| Linear step | $g_j = g_\text{from} + (g_\text{to} - g_\text{from}) \times j / s$ | fixed |
| Growth basis | `per_period`, `annual_compound`, `annual_simple` | required with periods other than 12 months |
| Value timing | `end_of_period`, `beginning_of_period` | required with a schedule |
| Projection | $V_k = V_{k-1}(1 + g_k)$, period by period, no rounding | fixed |

A negative base value is projected, with a warning: a positive growth makes the values more
negative. `explain` states the recurrence, the base value and base period, the growth rule
with its parameters, the basis and the effective annual growth it implies, the conversion of
the first rate with the numbers substituted, the first and the last value and, with a schedule,
the schedule conventions of [`dates.schedule`](dates-schedule.md). The per-period numbers are in
the rows, so the number of steps does not grow with the number of periods.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | A growth rate of −1 or lower, as given or after conversion, named by its pointer (`/growth/constant/growth`, `/growth/per_period/growth_rates/1`); a `per_period` list without exactly $s$ rates, with the expected and actual count and the base period; `growth_basis` missing with periods other than 12 months, an annual basis without a period length or with a length that neither divides 12 nor is a multiple of 12; `timing` with a `periods` timeline or missing with a `schedule`; both `frequency` and `period_months`; `number_of_periods` outside 1 to 10000 |
| `FM-E008` | A value beyond the decimal range |
| `FM-E009` | A date of the `schedule` timeline out of range, named at `/timeline/schedule/...` as in [`dates.schedule`](dates-schedule.md) |

## Accuracy

The recurrence uses exact decimal multiplication: with decimal inputs the values are exact until
the 28 significant digits of the decimal type are used up. The linear rule divides once per
step. `annual_simple` is one division or multiplication; `annual_compound` with periods of whole
years is an exact integer power; with $m$ periods a year it goes through the logarithm and
exponential of [`rates.convert`](rates-convert.md), with a relative error near $10^{-27}$, so
twelve monthly periods at 12% reach 1,120 within $10^{-20}$ relative.

## Verified against

- Golden cases in `tests/golden/series.project/`, from an independent 50-digit computation
  written from the specification (`scripts/reference_series.py`): the recurrence, both base
  periods, the linear rule, the three bases for every period length that divides 12 and for 24
  months, and the end-to-end XNPV of a dated series.
- The linear rates of Damodaran, *Investment Valuation*, 2nd edition, Illustration 15.3
  (Amgen): 11.46%, 9.85%, 8.23%, 6.62%, 5.00%.
- Property tests: `constant` equals `per_period` with the rate repeated, `linear` with equal
  ends equals `constant`, and the values equal the flows of the `constant_growth` and
  `linear_decline` stages of `dcf.fcff`, bit for bit.

## Sources

- Equivalent periodic rates under compound interest, $i_m = (1 + i)^{1/m} - 1$, and under
  simple interest, $i_m = i / m$ (verified on 2026-11-08): Università di Trieste,
  A. R. Bacinello, *Matematica finanziaria*, "Tassi equivalenti nel regime dell'interesse
  semplice e della capitalizzazione composta"; M. Sandri, *Matematica finanziaria*,
  "Capitalizzazione frazionata", "Tassi equivalenti". The compound equivalent rate is the one of
  [`rates.convert`](rates-convert.md).
- Damodaran, *Investment Valuation*, 2nd edition, chapter 15, Illustration 15.3.
- Period definition and end-of-month rule: see [`dates.schedule`](dates-schedule.md).
