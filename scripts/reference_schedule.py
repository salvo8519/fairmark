"""Independent reference for the schedules of SPEC-0010 (`dates.schedule`).

Each schedule is built twice, by two libraries that share no code with the Rust engine nor with
each other, and the generator fails when they disagree:

- python-dateutil: date k is `start + relativedelta(months=k * p, day=31)` when the start date is
  the last day of its month, `start + relativedelta(months=k * p)` otherwise (the day is clamped
  to the length of the month);
- QuantLib `Schedule`: effective date the start date, termination date the last generated date,
  tenor `Period(p, Months)`, `NullCalendar`, `Unadjusted` for every date, forward generation,
  `endOfMonth = True`. The termination date is the only date QuantLib does not generate, so the
  last date is confirmed by a second schedule one period longer whenever it stays in range.

Year fractions are computed with the QuantLib day counters (`Actual365Fixed`, `Actual360`,
`ActualActual(ISDA)`, `Thirty360(BondBasis)`, `Thirty360(European)`), and with Python's
`decimal` at 50 digits from the ISDA 2006 §4.16 definitions of `scripts/reference_time.py`, for
the exact digits.

Both libraries are in the `oracles` dependency group (`pyproject.toml`): QuantLib pinned
exactly, because its end-of-month behaviour has changed across releases, python-dateutil with a
lower bound. The versions are printed with every run, and a golden file records the versions it
was generated with (QuantLib 1.44 and python-dateutil 3.0.0; python-dateutil 4.0.0 gives the
same dates).

    uv run --group oracles python scripts/reference_schedule.py
"""

from __future__ import annotations

import calendar
import importlib.util
import itertools
import sys
from datetime import date
from decimal import Decimal as D
from decimal import localcontext
from pathlib import Path

import QuantLib as ql
from dateutil import __version__ as DATEUTIL_VERSION
from dateutil.relativedelta import relativedelta

ROOT = Path(__file__).resolve().parents[1]
_spec = importlib.util.spec_from_file_location("ref_time", ROOT / "scripts/reference_time.py")
assert _spec and _spec.loader
ref_time = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ref_time)

# Named frequencies and their tenors in months (SPEC-0010 R3), as the QuantLib `Frequency`
# values: Monthly 12, Bimonthly 6, Quarterly 4, EveryFourthMonth 3, Semiannual 2, Annual 1.
TENOR = {
    "monthly": 1,
    "bimonthly": 2,
    "quarterly": 3,
    "four_monthly": 4,
    "semi_annual": 6,
    "annual": 12,
}
QL_FREQUENCY = {
    "monthly": ql.Monthly,
    "bimonthly": ql.Bimonthly,
    "quarterly": ql.Quarterly,
    "four_monthly": ql.EveryFourthMonth,
    "semi_annual": ql.Semiannual,
    "annual": ql.Annual,
}
QL_DAY_COUNTERS = {
    "act_365_fixed": ql.Actual365Fixed(),
    "act_360": ql.Actual360(),
    "act_act_isda": ql.ActualActual(ql.ActualActual.ISDA),
    "thirty_360": ql.Thirty360(ql.Thirty360.BondBasis),
    "thirty_e_360": ql.Thirty360(ql.Thirty360.European),
}
LAST = date(2199, 12, 31)


def versions() -> str:
    return f"QuantLib {ql.__version__}, python-dateutil {DATEUTIL_VERSION}"


def is_month_end(d: date) -> bool:
    return d.day == calendar.monthrange(d.year, d.month)[1]


def dateutil_dates(start: date, months: int, n: int) -> list[date]:
    """Dates d_0 = start, ..., d_n, each from the start date (no drift)."""
    if is_month_end(start):
        return [start + relativedelta(months=k * months, day=31) for k in range(n + 1)]
    return [start + relativedelta(months=k * months) for k in range(n + 1)]


def _ql(d: date) -> ql.Date:
    return ql.Date(d.day, d.month, d.year)


def _py(d: ql.Date) -> date:
    return date(d.year(), d.month(), d.dayOfMonth())


def quantlib_dates(start: date, termination: date, months: int) -> list[date]:
    schedule = ql.Schedule(
        _ql(start),
        _ql(termination),
        ql.Period(months, ql.Months),
        ql.NullCalendar(),
        ql.Unadjusted,
        ql.Unadjusted,
        ql.DateGeneration.Forward,
        True,
    )
    return [_py(d) for d in schedule]


def schedule(start: date, months: int, n: int) -> list[date]:
    """The n + 1 dates of the schedule; raises when the two libraries disagree."""
    ours = dateutil_dates(start, months, n)
    theirs = quantlib_dates(start, ours[-1], months)
    if ours != theirs:
        raise AssertionError(f"{start} p={months} n={n}: dateutil {ours} != QuantLib {theirs}")
    longer = dateutil_dates(start, months, n + 1)
    if longer[-1] <= LAST:
        theirs = quantlib_dates(start, longer[-1], months)
        if longer != theirs:
            raise AssertionError(
                f"{start} p={months} n={n + 1}: dateutil {longer} != QuantLib {theirs}"
            )
    return ours


def year_fractions(dates: list[date], day_count: str) -> list[D]:
    """Year fraction of each period [d_{k-1}, d_k): the 50-digit ISDA reference, confirmed by the
    QuantLib day counter to 1e-12 (QuantLib computes in binary floating point)."""
    counter = QL_DAY_COUNTERS[day_count]
    out = []
    with localcontext() as ctx:
        ctx.prec = 50
        for a, b in itertools.pairwise(dates):
            exact = ref_time.yf(a, b, day_count)
            theirs = counter.yearFraction(_ql(a), _ql(b))
            if abs(float(exact) - theirs) > 1e-12:
                raise AssertionError(f"{day_count} {a} {b}: {exact} != QuantLib {theirs}")
            out.append(+exact)
    return out


# The schedules of SPEC-0010 AC1 and AC3: (start, months per period, number of periods).
AC1 = [
    ("2024-01-31", "monthly", 4),
    ("2025-01-30", "monthly", 3),
    ("2025-06-30", "quarterly", 4),
    ("2023-02-28", "monthly", 3),
    ("2024-02-29", "annual", 4),
    ("2025-12-31", "quarterly", 4),
    ("2026-01-01", "quarterly", 4),
    ("2025-01-31", "four_monthly", 3),
    ("2025-12-31", "bimonthly", 6),
    ("2025-03-15", "semi_annual", 3),
    ("2025-12-31", 3, 4),
    ("2025-01-31", 5, 3),
    ("2025-03-15", 18, 2),
    ("2024-02-29", 24, 3),
]
AC3 = [
    ("2025-12-31", "quarterly", 4, "act_365_fixed"),
    ("2024-01-31", "monthly", 4, "act_360"),
    ("2023-06-30", "semi_annual", 4, "act_act_isda"),
    ("2024-01-31", "monthly", 4, "thirty_360"),
    ("2024-01-31", "monthly", 4, "thirty_e_360"),
]


def months_of(length: str | int) -> int:
    return TENOR[length] if isinstance(length, str) else length


def main() -> int:
    print(versions())
    for start, length, n in AC1:
        dates = schedule(date.fromisoformat(start), months_of(length), n)
        print(f"{start} {length} x{n}: end dates {', '.join(map(str, dates[1:]))}")
    for start, length, n, day_count in AC3:
        dates = schedule(date.fromisoformat(start), months_of(length), n)
        fractions = year_fractions(dates, day_count)
        print(f"{start} {length} x{n} {day_count}:")
        print(f"  end dates {', '.join(map(str, dates[1:]))}")
        print(f"  year fractions {', '.join(str(f) for f in fractions)}")
        with localcontext() as ctx:
            ctx.prec = 50
            print(f"  sum {sum(fractions, D(0))}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
