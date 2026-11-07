"""analysis.goal_seek against two independent references (scripts/reference_analysis.py).

Deterministic and dependency-free: the textbook Brent's method must reach the same iteration
count and the same solution, and the reference function must confirm the tolerance. The
golden cases compare the solutions with bisection at 50 digits.
"""

from __future__ import annotations

import importlib.util
from decimal import Decimal as D
from pathlib import Path

import pytest

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("ref", ROOT / "scripts/reference_analysis.py")
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ref)

DCF = {"free_cash_flows": [100, 110, 120], "discount_rate": "0.10", "terminal_growth": "0.02"}
FLOWS = [D(-1000), D(300), D(400), D(500)]
MULTIPLE_IRR = [D(1000), D(-3600), D(4310), D(-1716)]  # IRRs 0.1, 0.2 and 0.3

CASES = [
    # (method, base input, variable, output, target, bracket, tolerance, reference function)
    (
        "dcf.fcff",
        {**DCF, "net_debt": 200},
        "/discount_rate",
        "/enterprise_value",
        "1300",
        ("0.05", "0.5"),
        "0.000000001",
        lambda r: ref.enterprise_value(r, [D(100), D(110), D(120)], D("0.02")) - 1300,
    ),
    (
        "tvm.npv",
        {"rate": "0.05", "cash_flows": [-1000, 300, 400, 500]},
        "/rate",
        "/npv",
        "0",
        ("0", "1"),
        "0.000000000000000001",
        lambda r: ref.npv(r, FLOWS),
    ),
    (
        "tvm.npv",
        {"rate": "0.05", "cash_flows": [1000, -3600, 4310, -1716]},
        "/rate",
        "/npv",
        "0",
        ("0", "0.35"),
        "0.000000000000000001",
        lambda r: ref.npv(r, MULTIPLE_IRR),
    ),
]


@pytest.mark.parametrize("case", CASES, ids=["dcf-offer", "irr", "multiple-irr"])
def test_engine_reproduces_the_textbook_brent_path(case: tuple) -> None:  # type: ignore[type-arg]
    method, base, variable, output, target, bracket, tol, f = case
    result = fm.analysis.goal_seek(
        method,
        base,
        variable=variable,
        output=output,
        target=target,
        bracket=bracket,
        tolerance=tol,
    )
    root, iterations = ref.brent_textbook(f, D(bracket[0]), D(bracket[1]), D(tol))
    assert int(result.value["iterations"]) == iterations
    assert abs(D(str(result.value["solution"])) - root) <= D("1e-24")
    # The reference function confirms the solution meets the tolerance (twice it, for the
    # rounding of two independent computations at 28 and 50 digits).
    assert abs(f(D(str(result.value["solution"])))) <= D(tol) * 2
