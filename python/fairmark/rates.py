"""Rate conventions: compounding conversion and real versus nominal rates."""

from __future__ import annotations

from decimal import Decimal
from typing import Literal

from ._core import Result, run

Number = Decimal | int | str | float
Compounding = Literal[
    "annual", "semi_annual", "four_monthly", "quarterly", "bimonthly", "monthly", "continuous"
]


def convert(rate: Number, from_: Compounding, to: Compounding) -> Result:
    """Equivalent annual rate under another compounding convention.

    >>> convert("0.12", "monthly", "annual").rate   # (1 + 0.12/12)^12 - 1
    Decimal('0.1268250301319697...')
    """
    return run("rates.convert", {"rate": rate, "from": from_, "to": to})


def real_nominal(
    rate: Number, inflation: Number, direction: Literal["to_real", "to_nominal"]
) -> Result:
    """Real rate from a nominal one, or the reverse, with the exact Fisher relation."""
    return run("rates.real_nominal", {"rate": rate, "inflation": inflation, "direction": direction})
