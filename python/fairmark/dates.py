"""Calendar dates and day-count conventions (ISDA 2006, section 4.16)."""

from __future__ import annotations

from datetime import date
from typing import Literal

from ._core import Result, run

DayCount = Literal["act_365_fixed", "act_360", "act_act_isda", "thirty_360", "thirty_e_360"]
DateLike = date | str


def year_fraction(start: DateLike, end: DateLike, day_count: DayCount) -> Result:
    """Fraction of a year between two dates under an ISDA day-count convention.

    Dates are ``datetime.date`` objects or ISO 8601 strings (``"2024-06-30"``).

    >>> year_fraction("2003-11-01", "2004-05-01", "act_act_isda").year_fraction
    Decimal('0.4977243805674077401026606...')
    """
    return run("dates.year_fraction", {"start": start, "end": end, "day_count": day_count})
