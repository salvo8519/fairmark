"""Calendar dates, day-count conventions (ISDA 2006, section 4.16) and schedules of periods."""

from __future__ import annotations

from datetime import date
from typing import Any, Literal

from ._core import Result, run

DayCount = Literal["act_365_fixed", "act_360", "act_act_isda", "thirty_360", "thirty_e_360"]
Frequency = Literal["monthly", "bimonthly", "quarterly", "four_monthly", "semi_annual", "annual"]
# Compounding of a rate: the named frequencies (periods a year) and continuous compounding.
Compounding = Literal[
    "annual", "semi_annual", "four_monthly", "quarterly", "bimonthly", "monthly", "continuous"
]
DateLike = date | str


def year_fraction(start: DateLike, end: DateLike, day_count: DayCount) -> Result:
    """Fraction of a year between two dates under an ISDA day-count convention.

    Dates are ``datetime.date`` objects or ISO 8601 strings (``"2024-06-30"``).

    >>> year_fraction("2003-11-01", "2004-05-01", "act_act_isda").year_fraction
    Decimal('0.4977243805674077401026606...')
    """
    return run("dates.year_fraction", {"start": start, "end": end, "day_count": day_count})


def schedule_input(
    start_date: DateLike,
    number_of_periods: int,
    *,
    frequency: Frequency | None = None,
    period_months: int | None = None,
    day_count: DayCount | None = None,
) -> dict[str, Any]:
    """The input of ``dates.schedule`` as a dictionary, also the ``schedule`` timeline of
    ``fairmark.series.project``. Give exactly one of ``frequency`` and ``period_months``."""
    payload: dict[str, Any] = {"start_date": start_date, "number_of_periods": number_of_periods}
    if frequency is not None:
        payload["frequency"] = frequency
    if period_months is not None:
        payload["period_months"] = period_months
    if day_count is not None:
        payload["day_count"] = day_count
    return payload


def schedule(
    start_date: DateLike,
    number_of_periods: int,
    *,
    frequency: Frequency | None = None,
    period_months: int | None = None,
    day_count: DayCount | None = None,
) -> Result:
    """Start and end dates of contiguous accrual periods, each [start, end).

    Each period runs from and including its start date to but excluding its end date, which is
    the start date of the next period. Give the period length as exactly one of ``frequency``
    (``"monthly"`` 1 month, ``"bimonthly"`` 2, ``"quarterly"`` 3, ``"four_monthly"`` 4,
    ``"semi_annual"`` 6, ``"annual"`` 12) or ``period_months`` (any positive whole number of
    months, such as 18). Dates are unadjusted; a start on the last day of a month keeps month
    ends. With ``day_count`` the year fraction of each period is returned.

    >>> r = schedule("2025-12-31", 4, frequency="quarterly")
    >>> r.end_dates
    ['2026-03-31', '2026-06-30', '2026-09-30', '2026-12-31']
    """
    return run(
        "dates.schedule",
        schedule_input(
            start_date,
            number_of_periods,
            frequency=frequency,
            period_months=period_months,
            day_count=day_count,
        ),
    )
