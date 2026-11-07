"""Cross-check Fairmark against widely used independent implementations.

pyxirr (Rust) and numpy-financial compute in binary floating point; Fairmark in decimal. The
comparison therefore uses a relative tolerance of 1e-9: it proves that the conventions and
formulas agree, not that the digits do. Exact digits are proven by the golden cases.
Installed with the `oracles` dependency group; skipped when it is absent.
"""

from __future__ import annotations

import random
from datetime import date, timedelta
from decimal import Decimal

import pytest

import fairmark as fm

pyxirr = pytest.importorskip("pyxirr")
npf = pytest.importorskip("numpy_financial")

REL = 1e-9
RNG = random.Random(20261107)  # fixed seed: the same cases on every run


def close(ours: Decimal, theirs: float) -> bool:
    return abs(float(ours) - theirs) <= REL * max(1.0, abs(theirs))


def random_date(lo: date = date(1990, 1, 1), span_days: int = 15_000) -> date:
    return lo + timedelta(days=RNG.randrange(span_days))


DAY_COUNTS = {
    "act_365_fixed": "ACT_365F",
    "act_360": "ACT_360",
    "act_act_isda": "ACT_ACT_ISDA",
    "thirty_360": "THIRTY_360_ISDA",
    "thirty_e_360": "THIRTY_E_360",
}


@pytest.mark.parametrize("ours", sorted(DAY_COUNTS))
def test_year_fraction_agrees_with_pyxirr(ours: str) -> None:
    theirs = getattr(pyxirr.DayCount, DAY_COUNTS[ours])
    for _ in range(300):
        start = random_date()
        end = start + timedelta(days=RNG.randrange(1, 4000))
        if RNG.random() < 0.3:  # month ends exercise the 30/360 rules
            end = date(end.year, end.month, 28) + timedelta(days=4)
            end = end - timedelta(days=end.day)
        got = fm.dates.year_fraction(start, end, ours).year_fraction  # type: ignore[arg-type]
        want = pyxirr.year_fraction(start, end, theirs)
        assert close(got, want), f"{ours} {start} {end}: {got} vs {want}"


def random_dated_flows() -> list[tuple[date, Decimal]]:
    start = random_date()
    flows = [(start, Decimal(-RNG.randrange(1000, 100_000)))]
    day = start
    for _ in range(RNG.randrange(2, 12)):
        day += timedelta(days=RNG.randrange(20, 400))
        flows.append((day, Decimal(RNG.randrange(100, 30_000))))
    return flows


def test_xnpv_and_xirr_agree_with_pyxirr() -> None:
    for _ in range(200):
        flows = random_dated_flows()
        dates = [d for d, _ in flows]
        amounts = [float(a) for _, a in flows]
        rate = Decimal(RNG.randrange(-50, 300)) / 1000
        assert close(fm.tvm.xnpv(rate, flows).xnpv, pyxirr.xnpv(float(rate), dates, amounts))
        theirs = pyxirr.xirr(dates, amounts)
        try:
            ours = fm.tvm.xirr(flows)
        except fm.FairmarkError as err:  # Fairmark refuses ambiguous cases; pyxirr does not
            assert err.code == "FM-E011"
            continue
        if theirs is not None:
            assert close(ours.xirr, theirs), f"{flows}: {ours.xirr} vs {theirs}"


def test_periodic_functions_agree_with_numpy_financial() -> None:
    for _ in range(200):
        flows = [Decimal(-RNG.randrange(1000, 50_000))] + [
            Decimal(RNG.randrange(-2000, 20_000)) for _ in range(RNG.randrange(2, 30))
        ]
        values = [float(v) for v in flows]
        rate = Decimal(RNG.randrange(-50, 300)) / 1000
        assert close(fm.tvm.npv(rate, flows).npv, npf.npv(float(rate), values))
        finance, reinvest = Decimal("0.07"), Decimal("0.11")
        try:
            ours_mirr = fm.tvm.mirr(flows, finance, reinvest).mirr
        except fm.FairmarkError:
            continue
        assert close(ours_mirr, npf.mirr(values, 0.07, 0.11))
        try:
            ours_irr = fm.tvm.irr(flows).irr
        except fm.FairmarkError as err:
            assert err.code in ("FM-E011", "FM-E005")
            continue
        theirs = npf.irr(values)
        if theirs == theirs:  # numpy-financial returns NaN when it finds no root
            assert close(ours_irr, theirs), f"{flows}: {ours_irr} vs {theirs}"


def test_annuities_agree_with_numpy_financial() -> None:
    for _ in range(200):
        rate = Decimal(RNG.randrange(1, 200)) / 1000
        periods = RNG.randrange(1, 60)
        payment = Decimal(RNG.randrange(10, 10_000))
        for timing, when in (("ordinary", "end"), ("due", "begin")):
            ours = fm.tvm.annuity(payment, rate, periods, timing=timing).present_value  # type: ignore[arg-type]
            theirs = -npf.pv(float(rate), periods, float(payment), 0, when)
            assert close(ours, theirs)


def test_goal_seek_on_npv_agrees_with_irr_and_pyxirr() -> None:
    """SPEC-0008 AC1: the rate that sets the NPV to zero, found by goal seek."""
    flows = [-1000, 300, 400, 500]
    seek = fm.analysis.goal_seek(
        "tvm.npv",
        {"rate": "0.05", "cash_flows": flows},
        variable="/rate",
        output="/npv",
        target="0",
        bracket=("0", "1"),
        tolerance="0.000000000000000001",
    )
    solution = Decimal(str(seek.value["solution"]))
    irr = Decimal(str(fm.tvm.irr(flows).value["irr"]))
    assert abs(solution - irr) <= Decimal("1e-15")
    assert close(solution, pyxirr.irr(flows))
