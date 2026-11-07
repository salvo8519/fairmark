"""Discounted cash flow methods."""

from __future__ import annotations

from collections.abc import Sequence
from decimal import Decimal
from typing import Literal

from ._core import Result, run

Number = Decimal | int | str | float
Timing = Literal["end_of_period", "mid_period"]


def fcff(
    free_cash_flows: Sequence[Number],
    discount_rate: Number,
    terminal_growth: Number,
    *,
    timing: Timing = "end_of_period",
    net_debt: Number = 0,
    non_operating_assets: Number = 0,
) -> Result:
    """Unlevered DCF: enterprise and equity value with a Gordon growth terminal value.

    Rates are decimal fractions (``"0.09"`` = 9%). Pass numbers as ``Decimal`` or
    strings for exactness; floats are converted through their shortest repr.

    >>> r = fcff([100, 110, 120], "0.10", "0.02", net_debt=200)
    >>> r.terminal_value
    Decimal('1530')
    """
    return run(
        "dcf.fcff",
        {
            "free_cash_flows": list(free_cash_flows),
            "discount_rate": discount_rate,
            "terminal_growth": terminal_growth,
            "timing": timing,
            "net_debt": net_debt,
            "non_operating_assets": non_operating_assets,
        },
    )
