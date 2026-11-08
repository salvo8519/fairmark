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


def test_sensitivity_table_agrees_with_numpy_financial() -> None:
    """SPEC-0008 AC6: each cell of a WACC by growth table recomputed independently."""
    flows = [100, 110, 120, 130, 140]
    rates = ["0.07", "0.08", "0.09", "0.10", "0.11"]
    growths = ["0", "0.01", "0.02", "0.03", "0.08"]
    table = fm.analysis.sensitivity(
        "dcf.fcff",
        {
            "stages": [{"periods": len(flows), "projection": "given", "flows": flows}],
            "cash_flow_terms": "nominal",
            "discount_rate_terms": "nominal",
            "wacc": "0.09",
            "timing": "end_of_period",
            "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
            "net_financial_position": {"gross_debt": 300, "cash": 0},
        },
        output="/value_of_operations",
        rows=("/wacc", rates),
        columns=("/terminal/tv.gordon/growth", growths),
    )
    for i, r in enumerate(rates):
        for j, g in enumerate(growths):
            cell = table.value["cells"][i][j]
            if float(g) >= float(r):
                assert cell["error"]["code"] == "FM-E004"
                continue
            rate, growth = float(r), float(g)
            explicit = npf.npv(rate, [0.0, *flows])  # npv discounts from t = 0
            terminal = flows[-1] * (1 + growth) / (rate - growth) / (1 + rate) ** len(flows)
            assert close(Decimal(str(cell["value"])), explicit + terminal)


DCF_RNG = random.Random(20261109)  # SPEC-0001: its own seed, independent of the cases above


def test_dcf_fcff_agrees_with_numpy_financial() -> None:
    """SPEC-0001 (#110), cross-check only: the explicit flows of dcf.fcff discounted by
    numpy-financial NPV plus the Gordon value at the horizon, in binary floating point."""
    for _ in range(200):
        flows = [DCF_RNG.randrange(-50, 500) for _ in range(DCF_RNG.randrange(1, 15))]
        rate = DCF_RNG.randrange(300, 1500) / 10_000
        growth = DCF_RNG.randrange(-200, 250) / 10_000
        r = fm.dcf.fcff(
            [fm.dcf.given(flows)],
            cash_flow_terms="nominal",
            discount_rate_terms="nominal",
            wacc=str(rate),
            timing="end_of_period",
            terminal=fm.dcf.gordon(str(growth), first_flow="last_flow_grown"),
        )
        explicit = npf.npv(rate, [0.0, *flows])  # npv discounts from t = 0
        terminal = flows[-1] * (1 + growth) / (rate - growth) / (1 + rate) ** len(flows)
        assert close(r.value_of_operations, explicit + terminal)


LOANS_RNG = random.Random(20261108)  # SPEC-0009: its own seed, independent of the cases above


def random_loan() -> tuple[Decimal, Decimal, int]:
    principal = Decimal(LOANS_RNG.randrange(1_000, 1_000_000))
    periodic_rate = Decimal(LOANS_RNG.randrange(1, 200)) / 10_000
    return principal, periodic_rate, LOANS_RNG.randrange(1, 361)


def test_loan_instalments_agree_with_numpy_financial() -> None:
    """SPEC-0009 R6: numpy_financial.pmt, with a balloon as its future value, both timings."""
    for _ in range(200):
        principal, i, n = random_loan()
        balloon = principal * Decimal(LOANS_RNG.randrange(0, 50)) / 100
        for timing, when in (("ordinary", "end"), ("due", "begin")):
            ours = fm.loans.instalment(
                principal, n, periodic_rate=i, balloon_amount=balloon, timing=timing
            ).instalment  # type: ignore[arg-type]
            theirs = -npf.pmt(float(i), n, float(principal), -float(balloon), when)
            assert close(ours, theirs), f"{principal} {i} {n} {balloon} {timing}"


def test_number_of_instalments_agrees_with_numpy_financial() -> None:
    """SPEC-0009 AC3: numpy_financial.nper on ten random loans, both timings."""
    for _ in range(10):
        principal, i, n = random_loan()
        instalment = fm.loans.instalment(principal, n, periodic_rate=i, timing="ordinary")
        payment = instalment.instalment * Decimal("1.05")
        for timing, when in (("ordinary", "end"), ("due", "begin")):
            ours = fm.loans.number_of_instalments(
                principal, payment, periodic_rate=i, timing=timing
            ).exact_number_of_periods  # type: ignore[arg-type]
            theirs = float(npf.nper(float(i), -float(payment), float(principal), 0, when))
            assert close(ours, theirs), f"{principal} {i} {payment} {timing}"


def test_future_value_agrees_with_numpy_financial() -> None:
    """SPEC-0009 R10: numpy_financial.fv with its cash-flow signs reversed."""
    for _ in range(200):
        rate = Decimal(LOANS_RNG.randrange(-50, 200)) / 10_000
        periods = LOANS_RNG.randrange(1, 400)
        pv = Decimal(LOANS_RNG.randrange(-100_000, 100_000))
        pmt = Decimal(LOANS_RNG.randrange(-5_000, 5_000))
        for timing, when in (("ordinary", "end"), ("due", "begin")):
            ours = fm.tvm.future_value(
                rate, periods, present_value=pv, payment=pmt, timing=timing
            ).future_value  # type: ignore[arg-type]
            theirs = -npf.fv(float(rate), periods, float(pmt), float(pv), when)
            assert close(ours, theirs), f"{rate} {periods} {pv} {pmt} {timing}"


def test_french_schedule_agrees_with_numpy_financial() -> None:
    """SPEC-0009 R13: interest and principal portions against numpy_financial.ipmt and ppmt."""
    for _ in range(50):
        principal, i, n = random_loan()
        schedule = fm.loans.amortisation_schedule(
            principal, n, periodic_rate=i, amortisation_type="french", timing="ordinary"
        )
        for row in schedule.rows:
            k = row["period"]
            interest = -float(npf.ipmt(float(i), k, n, float(principal)))
            principal_portion = -float(npf.ppmt(float(i), k, n, float(principal)))
            assert close(row["interest_portion"], interest), f"{principal} {i} {n} {k}"
            assert close(row["principal_portion"], principal_portion), f"{principal} {i} {n} {k}"
