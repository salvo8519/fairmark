"""Analysis over any method: goal seek, sensitivity tables, scenarios and tornado.

The inner method is named by its id and receives ``base_input`` exactly as it would be passed
to :func:`fairmark.run`. Variables and outputs are RFC 6901 JSON Pointers, for example
``"/discount_rate"`` or ``"/enterprise_value"``. The keyword arguments that hold pointers have
the names of the input fields of the analyses (version 2, SPEC-0012): ``variable_pointer``,
``output_pointer`` and ``output_pointers``.
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
    variable_pointer: str,
    output_pointer: str,
    target: Number,
    bracket: tuple[Number, Number],
    tolerance: Number | None = None,
    max_iterations: int | None = None,
) -> Result:
    """Value of ``variable_pointer`` at which ``output_pointer`` equals ``target`` (Brent's method).

    The output minus the target must change sign between the two ends of ``bracket``;
    otherwise ``FairmarkError`` with code ``FM-E013`` is raised. The tolerance is absolute, in
    the output's units (default 0.000000001); ``max_iterations`` defaults to 100.

    Example: the WACC at which the DCF value of operations equals an offer of 1300::

        fm.analysis.goal_seek(
            "dcf.fcff", dcf_input,
            variable_pointer="/wacc", output_pointer="/value_of_operations",
            target="1300", bracket=("0.05", "0.5"),
        )
    """
    payload: dict[str, Any] = {
        "method": method,
        "base_input": dict(base_input),
        "variable_pointer": variable_pointer,
        "output_pointer": output_pointer,
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
    output_pointer: str,
    rows: tuple[str, Sequence[Number]],
    columns: tuple[str, Sequence[Number]] | None = None,
) -> Result:
    """One output of ``method`` across a one-way or two-way table of inputs.

    ``rows`` and ``columns`` are ``(pointer, values)`` pairs. A combination the method refuses
    (for example growth not below the discount rate) holds its error in its cell; the table
    goes on. Example, the table of a DCF by WACC and growth::

        fm.analysis.sensitivity(
            "dcf.fcff", dcf_input, output_pointer="/value_of_operations",
            rows=("/wacc", ["0.08", "0.09", "0.10"]),
            columns=("/terminal/tv.gordon/growth", ["0.01", "0.02", "0.03"]),
        )
    """
    payload: dict[str, Any] = {
        "method": method,
        "base_input": dict(base_input),
        "output_pointer": output_pointer,
        "rows": {"variable_pointer": rows[0], "values": list(rows[1])},
    }
    if columns is not None:
        payload["columns"] = {"variable_pointer": columns[0], "values": list(columns[1])}
    return run("analysis.sensitivity", payload)


def scenarios(
    method: str,
    base_input: Mapping[str, Any],
    *,
    output_pointers: Sequence[str],
    cases: Mapping[str, Mapping[str, Number]] | Sequence[tuple[str, Mapping[str, Number]]],
) -> Result:
    """The ``output_pointers`` of ``method`` under named cases, with deltas against the base case.

    ``cases`` maps each scenario name to the inputs it changes, as ``{pointer: value}``; it may
    also be a list of ``(name, changes)`` pairs. A repeated key in a Python dict literal
    silently replaces the earlier one before Fairmark sees it: use the list form when names
    or pointers are generated, so that repeats are refused (``FM-E001``). A case the method
    refuses holds its error; the others are returned. Example::

        fm.analysis.scenarios(
            "dcf.fcff", dcf_input, output_pointers=["/value_of_operations", "/equity_value"],
            cases={"downside": {"/wacc": "0.10", "/terminal/tv.gordon/growth": "0.01"},
                   "upside": {"/wacc": "0.08"}},
        )
    """
    payload = {
        "method": method,
        "base_input": dict(base_input),
        "output_pointers": list(output_pointers),
        "scenarios": [
            {"name": name, "overrides": [{"pointer": p, "value": v} for p, v in changes.items()]}
            for name, changes in (cases.items() if isinstance(cases, Mapping) else cases)
        ],
    }
    return run("analysis.scenarios", payload)


def tornado(
    method: str,
    base_input: Mapping[str, Any],
    *,
    output_pointer: str,
    variables: Mapping[str, tuple[Number, Number]] | Sequence[tuple[str, tuple[Number, Number]]],
) -> Result:
    """Which inputs move ``output_pointer`` the most: each varied alone between low and high.

    ``variables`` maps each pointer to its ``(low, high)`` values; it may also be a list of
    ``(pointer, (low, high))`` pairs, which keeps repeats visible to the engine (``FM-E001``).
    The result ranks the inputs by the absolute swing of the output; equal swings keep the
    order given. Example::

        fm.analysis.tornado(
            "dcf.fcff", dcf_input, output_pointer="/equity_value",
            variables={"/wacc": ("0.08", "0.10"), "/terminal/tv.gordon/growth": ("0.01", "0.03")},
        )
    """
    payload = {
        "method": method,
        "base_input": dict(base_input),
        "output_pointer": output_pointer,
        "variables": [
            {"variable_pointer": p, "low": low, "high": high}
            for p, (low, high) in (
                variables.items() if isinstance(variables, Mapping) else variables
            )
        ],
    }
    return run("analysis.tornado", payload)
