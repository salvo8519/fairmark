"""SPEC-0010 `dates.schedule` against QuantLib `Schedule` and python-dateutil (AC2, AC3).

Random schedules from a fixed seed: start dates from 1900 to 2150, every named frequency and
`period_months` from 1 to 36. The dates must equal both libraries exactly; the year fractions
must equal the QuantLib day counters to 1e-12 (QuantLib computes in binary floating point and
its dates start in 1901, so schedules starting in 1900 are compared with dateutil only;
the exact digits are proven by the golden cases). Installed with the `oracles` dependency
group (versions pinned in pyproject.toml); skipped when it is absent.
"""

from __future__ import annotations

import calendar
import itertools
import random
from datetime import date

import pytest

import fairmark as fm

ql = pytest.importorskip("QuantLib")
relativedelta = pytest.importorskip("dateutil.relativedelta").relativedelta

RNG = random.Random(20261108)
QL_FIRST_YEAR = 1901  # the first year of a QuantLib date
NAMED = {
    "monthly": 1,
    "bimonthly": 2,
    "quarterly": 3,
    "four_monthly": 4,
    "semi_annual": 6,
    "annual": 12,
}
DAY_COUNTERS = {
    "act_365_fixed": ql.Actual365Fixed(),
    "act_360": ql.Actual360(),
    "act_act_isda": ql.ActualActual(ql.ActualActual.ISDA),
    "thirty_360": ql.Thirty360(ql.Thirty360.BondBasis),
    "thirty_e_360": ql.Thirty360(ql.Thirty360.European),
}


def random_start() -> date:
    year, month = RNG.randrange(1900, 2151), RNG.randrange(1, 13)
    last = calendar.monthrange(year, month)[1]
    # One start in three on a month end, so that the end-of-month rule is exercised.
    day = last if RNG.random() < 1 / 3 else RNG.randrange(1, last + 1)
    return date(year, month, day)


def dateutil_dates(start: date, months: int, n: int) -> list[date]:
    month_end = start.day == calendar.monthrange(start.year, start.month)[1]
    if month_end:
        return [start + relativedelta(months=k * months, day=31) for k in range(n + 1)]
    return [start + relativedelta(months=k * months) for k in range(n + 1)]


def quantlib_dates(start: date, end: date, months: int) -> list[date]:
    schedule = ql.Schedule(
        ql.Date(start.day, start.month, start.year),
        ql.Date(end.day, end.month, end.year),
        ql.Period(months, ql.Months),
        ql.NullCalendar(),
        ql.Unadjusted,
        ql.Unadjusted,
        ql.DateGeneration.Forward,
        True,
    )
    return [date(d.year(), d.month(), d.dayOfMonth()) for d in schedule]


def test_schedules_match_dateutil_and_quantlib() -> None:
    for _ in range(600):
        start = random_start()
        if RNG.random() < 0.5:
            name = RNG.choice(sorted(NAMED))
            months, form = NAMED[name], {"frequency": name}
        else:
            months = RNG.randrange(1, 37)
            form = {"period_months": months}
        n = RNG.randrange(1, 17)
        day_count = RNG.choice(sorted(DAY_COUNTERS))
        r = fm.dates.schedule(start, n, day_count=day_count, **form)  # type: ignore[arg-type]
        ours = [date.fromisoformat(r.start_dates[0])] + [date.fromisoformat(d) for d in r.end_dates]
        assert ours == dateutil_dates(start, months, n), f"{start} {form} x{n}"
        if start.year < QL_FIRST_YEAR:
            continue  # QuantLib dates start in 1901; dateutil covers 1900
        assert ours == quantlib_dates(start, ours[-1], months), f"{start} {form} x{n}"
        counter = DAY_COUNTERS[day_count]
        for (a, b), yf in zip(itertools.pairwise(ours), r.year_fractions, strict=True):
            theirs = counter.yearFraction(
                ql.Date(a.day, a.month, a.year), ql.Date(b.day, b.month, b.year)
            )
            assert abs(float(yf) - theirs) <= 1e-12, f"{day_count} {a} {b}: {yf} vs {theirs}"
