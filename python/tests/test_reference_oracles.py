"""The independent reference scripts behind golden cases still reproduce them (issue #345).

`scripts/reference_coc.py`, `reference_npv_irr.py`, `reference_solver.py`, `reference_time.py`
and `reference_schedule.py` are the 50-digit oracles that golden files name as their source.
These tests check each oracle against closed forms, against another oracle, and against the
golden values it produced, so that a broken oracle cannot silently vouch for a wrong case.
"""

from __future__ import annotations

import importlib.util
import os
import sys
from collections.abc import Iterator
from datetime import date
from decimal import Decimal as D
from decimal import localcontext
from pathlib import Path
from types import ModuleType

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[2]


def load(name: str) -> ModuleType:
    """Import an oracle without its side effects: some set the global decimal precision to 50
    digits when imported, which would change the results of every test that runs after."""
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / f"{name}.py")
    module = importlib.util.module_from_spec(spec)  # type: ignore[arg-type]
    sys.modules[name] = module
    with localcontext():
        spec.loader.exec_module(module)  # type: ignore[union-attr]
    return module


@pytest.fixture(autouse=True)
def fifty_digits() -> Iterator[None]:
    """The oracles are written for 50 significant digits, in a context local to each test."""
    with localcontext() as ctx:
        ctx.prec = 50
        yield


coc = load("reference_coc")
npv_irr = load("reference_npv_irr")
solver = load("reference_solver")
ref_time = load("reference_time")
values = load("reference_values")
ratios = load("reference_ratios")
TIGHT = D("1e-30")


def test_hamada_unlever_and_relever_are_inverse() -> None:
    unlevered = coc.unlever_hamada(D("1.2"), D("0.5"), D("0.25"))
    assert abs(unlevered - D("1.2") / D("1.375")) < TIGHT
    assert abs(coc.relever_hamada(unlevered, D("0.5"), D("0.25")) - D("1.2")) < TIGHT
    with_debt_beta = coc.unlever_hamada(D("1.2"), D("0.5"), D("0.25"), D("0.2"))
    relevered = coc.relever_hamada(with_debt_beta, D("0.5"), D("0.25"), D("0.2"))
    assert abs(relevered - D("1.2")) < TIGHT


def test_npv_and_irr_oracle_reproduce_their_golden_cases() -> None:
    for name, reference, expected, tolerance in npv_irr.cases():
        assert abs(reference - expected) <= tolerance, name


def test_newton_finds_the_root_built_into_the_flows() -> None:
    flows = solver.flows_from_roots(D(-1), [D("1.11")])
    root, iterations = solver.irr_newton(flows)
    assert abs(root - D("0.11")) < D("1e-20") and iterations < 50
    # Three roots: from 0.10 Newton converges to the nearest one.
    three = solver.flows_from_roots(D(-1), [D("1.11"), D("1.12"), D("1.53")])
    assert abs(solver.irr_newton(three)[0] - D("0.11")) < D("1e-20")


def test_two_independent_xirr_oracles_agree() -> None:
    flows = [(date.fromisoformat(d), a) for d, a in solver.XIRR_FLOWS]
    newton, _ = solver.xirr_newton(solver.XIRR_FLOWS, 1)
    bisection = ref_time.xirr(flows)
    assert abs(newton - bisection) < D("1e-20")


def test_year_fractions_follow_isda() -> None:
    isda = ref_time.yf(date(2003, 11, 1), date(2004, 5, 1), "act_act_isda")
    assert isda == D(61) / D(365) + D(121) / D(366)
    assert ref_time.yf(date(2024, 1, 31), date(2024, 3, 31), "thirty_360") == D(60) / D(360)
    assert ref_time.yf(date(2024, 2, 15), date(2024, 3, 31), "thirty_360") == D(46) / D(360)
    assert ref_time.yf(date(2024, 2, 15), date(2024, 3, 31), "thirty_e_360") == D(45) / D(360)
    with pytest.raises(ValueError):
        ref_time.yf(date(2024, 1, 1), date(2024, 2, 1), "nonsense")


def test_annuities_and_present_values_match_closed_forms() -> None:
    assert abs(ref_time.annuity(D(1000), D("0.06"), 15, D("0.06")) - D(15000) / D("1.06")) < TIGHT
    assert ref_time.perpetuity(D(100), D("0.08"), D("0.02")) == D(100) / D("0.06")
    pvs, total = ref_time.present_value([D(100), D(110), D(121)], [D("0.10")] * 3, "end_of_period")
    assert abs(total - D(300) / D("1.1")) < TIGHT and len(pvs) == 3
    annual = ref_time.convert(D("0.12"), "monthly", "annual")
    assert abs(annual - ((1 + D("0.01")) ** 12 - 1)) < TIGHT


def test_xnpv_oracle_reproduces_its_golden_case() -> None:
    case = yaml.safe_load(
        (ROOT / "tests/golden/tvm.xnpv/act-act-quarterly.yaml").read_text(encoding="utf-8")
    )
    data = case["input"]
    flows = [(date.fromisoformat(f["date"]), D(f["amount"])) for f in data["cash_flows"]]
    value = ref_time.xnpv(D(data["rate"]), flows, None, data["day_count"], data["compounding"])
    assert abs(value - D(case["expect"]["xnpv"])) <= D(case["tolerance"])


def test_loading_an_oracle_leaves_the_precision_alone() -> None:
    import decimal

    with localcontext() as ctx:
        ctx.prec = 28
        load("reference_coc")  # sets getcontext().prec = 50 when imported
        assert decimal.getcontext().prec == 28


def test_schedule_oracle_rolls_month_ends_and_agrees_with_quantlib() -> None:
    # The schedule oracle needs the `oracles` group; in CI (FAIRMARK_REQUIRE_ORACLES=1) a missing
    # library is an import error, so this test cannot pass by being skipped.
    if os.environ.get("FAIRMARK_REQUIRE_ORACLES") != "1":
        pytest.importorskip("QuantLib")
        pytest.importorskip("dateutil")
    schedule = load("reference_schedule")
    dates = schedule.schedule(date(2025, 1, 31), 1, 3)
    assert dates == [date(2025, 1, 31), date(2025, 2, 28), date(2025, 3, 31), date(2025, 4, 30)]
    fractions = schedule.year_fractions(schedule.schedule(date(2024, 1, 31), 1, 2), "thirty_360")
    expected = [D(29) / D(360), D(32) / D(360)]  # 30/360: Jan 31 counts as day 30
    assert all(abs(f - e) < D("1e-45") for f, e in zip(fractions, expected, strict=True))
    assert schedule.main() == 0


def test_textbook_dcf_npv_and_irr() -> None:
    assert values.npv(D("0.1"), [D(-100), D(110)]) == 0
    assert abs(values.irr([D(-100), D(110)]) - D("0.1")) < D("1e-40")
    out = values.dcf([D(100)], D("0.1"), D(0), net_debt=D(50), noa=D(10))
    assert out["terminal_value"] == D(1000)
    assert abs(out["enterprise_value"] - (D(100) + D(1000)) / D("1.1")) < TIGHT
    assert out["equity_value"] == out["enterprise_value"] - 50 + 10
    mid = values.dcf([D(100)], D("0.1"), D(0), mid=True)
    assert mid["pv_explicit_period"] > out["pv_explicit_period"]


def test_ratio_oracle_definitions() -> None:
    assert ratios.q(D(1), D(4)) == D("0.25")
    assert ratios.q(D(1), D(0)) is None
    assert ratios.q(D(1), D(-2), positive=True) is None
    assert ratios.q_both(D(-1), D(2)) is None and ratios.q_both(D(3), D(2)) == D("1.5")
    assert ratios.avg(D(10), D(20), True) == 15 and ratios.avg(None, D(20), False) == 20
    assert ratios.nfp({"gross_debt": D(2200), "cash": D(300), "other_items": [D(150)]}) == 2050
    annuity = (1 - D("1.1") ** -3) / D("0.1")
    assert abs(ratios.level_llcr(D(100), D("0.1"), 3, D(200)) - 100 * annuity / 200) < TIGHT


def test_ratio_oracle_keeps_the_callers_precision() -> None:
    import decimal

    with localcontext() as ctx:
        ctx.prec = 10
        assert ratios.at_50_digits(lambda: decimal.getcontext().prec)() == 50
        assert decimal.getcontext().prec == 10
