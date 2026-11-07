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


GOLDEN = ROOT / "tests" / "golden"


def _expect(path: str) -> dict[str, str]:
    import yaml

    return yaml.safe_load((GOLDEN / path).read_text(encoding="utf-8"))["expect"]


def test_sensitivity_golden_is_reproduced_by_the_reference() -> None:
    """The 5 x 5 table of the golden case comes from sensitivity_table, nowhere else."""
    flows = [D(x) for x in (100, 110, 120, 130, 140)]
    rates = [D(x) for x in ("0.07", "0.08", "0.09", "0.10", "0.11")]
    growths = [D(x) for x in ("0", "0.01", "0.02", "0.03", "0.08")]
    table = ref.sensitivity_table(rates, growths, flows)
    expect = _expect("analysis.sensitivity/wacc-g-5x5.yaml")
    for i, row in enumerate(table):
        for j, value in enumerate(row):
            if value is None:
                assert expect[f"/cells/{i}/{j}/error/code"] == "FM-E004"
            else:
                assert abs(D(expect[f"/cells/{i}/{j}/value"]) - value) < D("1e-24")


def test_tornado_golden_is_reproduced_by_the_reference() -> None:
    flows = [D(x) for x in (100, 110, 120, 130, 140)]
    base = {"discount_rate": D("0.09"), "terminal_growth": D("0.02"), "net_debt": D(300)}
    shocks = [
        ("net_debt", D(250), D(350)),
        ("flow:0", D(90), D(110)),
        ("terminal_growth", D("0.01"), D("0.03")),
        ("discount_rate", D("0.08"), D("0.10")),
    ]
    ranking = ref.tornado_ranking(base, flows, shocks)
    expect = _expect("analysis.tornado/dcf-four-drivers.yaml")
    pointers = {
        "net_debt": "/net_debt",
        "flow:0": "/free_cash_flows/0",
        "terminal_growth": "/terminal_growth",
        "discount_rate": "/discount_rate",
    }
    for k, (name, low, high, swing) in enumerate(ranking):
        assert expect[f"/ranking/{k}/variable"] == pointers[name]
        assert abs(D(expect[f"/ranking/{k}/output_low"]) - low) < D("1e-24")
        assert abs(D(expect[f"/ranking/{k}/output_high"]) - high) < D("1e-24")
        assert abs(D(expect[f"/ranking/{k}/swing"]) - swing) < D("1e-24")
