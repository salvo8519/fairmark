"""Analysis over any method: goal seek, sensitivity tables, scenarios and tornado.

The inner method is named by its id and receives ``base_input`` exactly as it would be passed
to :func:`fairmark.run`. Variables and outputs are RFC 6901 JSON Pointers, for example
``"/discount_rate"`` or ``"/enterprise_value"``.
"""

from __future__ import annotations

from collections.abc import Mapping, Sequence
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


def sensitivity(
    method: str,
    base_input: Mapping[str, Any],
    *,
    output: str,
    rows: tuple[str, Sequence[Number]],
    columns: tuple[str, Sequence[Number]] | None = None,
) -> Result:
    """One output of ``method`` across a one-way or two-way table of inputs.

    ``rows`` and ``columns`` are ``(pointer, values)`` pairs. A combination the method refuses
    (for example growth not below the discount rate) holds its error in its cell; the table
    goes on. Example, the table of a DCF by WACC and growth::

        fm.analysis.sensitivity(
            "dcf.fcff", dcf_input, output="/enterprise_value",
            rows=("/discount_rate", ["0.08", "0.09", "0.10"]),
            columns=("/terminal_growth", ["0.01", "0.02", "0.03"]),
        )
    """
    payload: dict[str, Any] = {
        "method": method,
        "base_input": dict(base_input),
        "output": output,
        "rows": {"variable": rows[0], "values": list(rows[1])},
    }
    if columns is not None:
        payload["columns"] = {"variable": columns[0], "values": list(columns[1])}
    return run("analysis.sensitivity", payload)


def scenarios(
    method: str,
    base_input: Mapping[str, Any],
    *,
    outputs: Sequence[str],
    cases: Mapping[str, Mapping[str, Number]],
) -> Result:
    """The ``outputs`` of ``method`` under named cases, with deltas against the base case.

    ``cases`` maps each scenario name to the inputs it changes, as ``{pointer: value}``. A case
    the method refuses holds its error; the others are returned. Example::

        fm.analysis.scenarios(
            "dcf.fcff", dcf_input, outputs=["/enterprise_value", "/equity_value"],
            cases={"downside": {"/discount_rate": "0.10", "/terminal_growth": "0.01"},
                   "upside": {"/discount_rate": "0.08"}},
        )
    """
    payload = {
        "method": method,
        "base_input": dict(base_input),
        "outputs": list(outputs),
        "scenarios": [
            {"name": name, "overrides": [{"pointer": p, "value": v} for p, v in changes.items()]}
            for name, changes in cases.items()
        ],
    }
    return run("analysis.scenarios", payload)


def tornado(
    method: str,
    base_input: Mapping[str, Any],
    *,
    output: str,
    variables: Mapping[str, tuple[Number, Number]],
) -> Result:
    """Which inputs move ``output`` the most: each varied alone between low and high.

    ``variables`` maps each pointer to its ``(low, high)`` values. The result ranks the inputs
    by the absolute swing of the output; equal swings keep the order given. Example::

        fm.analysis.tornado(
            "dcf.fcff", dcf_input, output="/equity_value",
            variables={"/discount_rate": ("0.08", "0.10"), "/terminal_growth": ("0.01", "0.03")},
        )
    """
    payload = {
        "method": method,
        "base_input": dict(base_input),
        "output": output,
        "variables": [
            {"variable": p, "low": low, "high": high} for p, (low, high) in variables.items()
        ],
    }
    return run("analysis.tornado", payload)
