"""Analysis over any method: goal seek.

The inner method is named by its id and receives ``base_input`` exactly as it would be passed
to :func:`fairmark.run`. Variables and outputs are RFC 6901 JSON Pointers, for example
``"/discount_rate"`` or ``"/enterprise_value"``.
"""

from __future__ import annotations

from collections.abc import Mapping
from decimal import Decimal
from typing import Any

from ._core import Result, run

Number = Decimal | int | str | float


def goal_seek(
    method: str,
    base_input: Mapping[str, Any],
    *,
    variable: str,
    output: str,
    target: Number,
    bracket: tuple[Number, Number],
    tolerance: Number | None = None,
    max_iterations: int | None = None,
) -> Result:
    """Value of ``variable`` at which ``output`` equals ``target`` (Brent's method).

    The output minus the target must change sign between the two ends of ``bracket``;
    otherwise ``FairmarkError`` with code ``FM-E013`` is raised. The tolerance is absolute, in
    the output's units (default 0.000000001); ``max_iterations`` defaults to 100.

    Example: the discount rate at which the DCF enterprise value equals an offer of 1300::

        fm.analysis.goal_seek(
            "dcf.fcff",
            {"free_cash_flows": [100, 110, 120], "discount_rate": "0.10",
             "terminal_growth": "0.02", "net_debt": 200},
            variable="/discount_rate", output="/enterprise_value",
            target="1300", bracket=("0.05", "0.5"),
        )
    """
    payload: dict[str, Any] = {
        "method": method,
        "base_input": dict(base_input),
        "variable": variable,
        "output": output,
        "target": target,
        "bracket": list(bracket),
    }
    if tolerance is not None:
        payload["tolerance"] = tolerance
    if max_iterations is not None:
        payload["max_iterations"] = max_iterations
    return run("analysis.goal_seek", payload)
