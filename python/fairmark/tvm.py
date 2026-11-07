"""Time value of money: NPV and IRR. ``cash_flows[0]`` occurs at t = 0."""

from __future__ import annotations

from collections.abc import Sequence
from decimal import Decimal
from typing import Any

from ._core import Result, run

Number = Decimal | int | str | float


def npv(rate: Number, cash_flows: Sequence[Number]) -> Result:
    """Net present value. The first cash flow is at t = 0 and is not discounted."""
    return run("tvm.npv", {"rate": rate, "cash_flows": list(cash_flows)})


def irr(cash_flows: Sequence[Number], *, guess: Number | None = None) -> Result:
    """Internal rate of return, solved deterministically (Newton, bisection fallback)."""
    payload: dict[str, Any] = {"cash_flows": list(cash_flows)}
    if guess is not None:
        payload["guess"] = guess
    return run("tvm.irr", payload)
