"""Independent reference for SPEC-0002 (time value foundations).

Written from the published definitions with Python's `decimal` module at 50 digits and
`datetime.date` for calendar arithmetic. It shares no code with the Rust engine and is used
to produce or confirm golden expectations:

    uv run python scripts/reference_time.py

Sources: 2006 ISDA Definitions section 4.16 (day counts); ISDA "EMU and market conventions"
(1998) for the Actual/Actual example; ECMA-376 SpreadsheetML (XNPV, XIRR, MIRR); closed forms
for annuities, perpetuities and rate conversions.
"""

from __future__ import annotations

from datetime import date
from decimal import Decimal as D
from decimal import getcontext

getcontext().prec = 50
Q = D("1e-20")


# ---------------------------------------------------------------- day counts (ISDA 4.16)
def yf(start: date, end: date, basis: str) -> D:
    days = (end - start).days
    if basis == "act_365_fixed":
        return D(days) / 365
    if basis == "act_360":
        return D(days) / 360
    if basis == "act_act_isda":
        if start.year == end.year:
            return D(days) / (366 if _leap(start.year) else 365)
        first = (date(start.year + 1, 1, 1) - start).days
        last = (end - date(end.year, 1, 1)).days
        return (
            D(first) / (366 if _leap(start.year) else 365)
            + (end.year - start.year - 1)
            + D(last) / (366 if _leap(end.year) else 365)
        )
    y1, m1, d1 = start.year, start.month, start.day
    y2, m2, d2 = end.year, end.month, end.day
    if basis == "thirty_360":  # 4.16(f) 30/360, Bond Basis
        d1 = 30 if d1 == 31 else d1
        d2 = 30 if d2 == 31 and d1 > 29 else d2
    elif basis == "thirty_e_360":  # 4.16(g) 30E/360, Eurobond Basis
        d1 = 30 if d1 == 31 else d1
        d2 = 30 if d2 == 31 else d2
    else:
        raise ValueError(basis)
    return D(360 * (y2 - y1) + 30 * (m2 - m1) + (d2 - d1)) / 360


def _leap(y: int) -> bool:
    return y % 4 == 0 and (y % 100 != 0 or y % 400 == 0)


# ---------------------------------------------------------------- rates
M = {
    "annual": 1,
    "semi_annual": 2,
    "four_monthly": 3,
    "quarterly": 4,
    "bimonthly": 6,
    "monthly": 12,
}


def growth(rate: D, comp: str, t: D) -> D:
    if comp == "continuous":
        return (rate * t).exp()
    m = M[comp]
    return ((1 + rate / m).ln() * m * t).exp()


def convert(rate: D, src: str, dst: str) -> D:
    g = growth(rate, src, D(1))
    if dst == "continuous":
        return g.ln()
    m = M[dst]
    return m * ((g.ln() / m).exp() - 1)


# ---------------------------------------------------------------- discounting
def present_value(cfs: list[D], rates: list[D], timing: str) -> tuple[list[D], D]:
    cum, dfs = D(1), []
    for r in rates[: len(cfs)]:
        prev, cum = cum, cum / (1 + r)
        if timing == "end_of_period":
            dfs.append(cum)
        elif timing == "mid_period":
            dfs.append(prev / (1 + r).sqrt())
        else:
            dfs.append(prev)
    pvs = [c * d for c, d in zip(cfs, dfs, strict=True)]
    return pvs, sum(pvs)


def xnpv(rate, flows, valuation=None, basis="act_365_fixed", comp="annual"):
    v = valuation or min(d for d, _ in flows)
    return sum(a / growth(rate, comp, yf(v, d, basis)) for d, a in flows)


def xirr(flows, lo=D("-0.99"), hi=D("10")):
    f_lo = xnpv(lo, flows)
    for _ in range(300):
        mid = (lo + hi) / 2
        f_mid = xnpv(mid, flows)
        if (f_mid > 0) == (f_lo > 0):
            lo, f_lo = mid, f_mid
        else:
            hi = mid
    return (lo + hi) / 2


def mirr(cfs, finance, reinvest):
    n = len(cfs) - 1
    pv_neg = sum(c / (1 + finance) ** t for t, c in enumerate(cfs) if c < 0)
    fv_pos = sum(c * (1 + reinvest) ** (n - t) for t, c in enumerate(cfs) if c > 0)
    return ((fv_pos / -pv_neg).ln() / n).exp() - 1


def annuity(p, r, n, g=D(0), due=False):
    pv = sum(p * (1 + g) ** (k - 1) / (1 + r) ** k for k in range(1, n + 1))
    return pv * (1 + r) if due else pv


def perpetuity(p, r, g=D(0), due=False):
    pv = p / (r - g)
    return pv * (1 + r) if due else pv


if __name__ == "__main__":
    ecma = [
        (date(2008, 1, 1), D(-10000)),
        (date(2008, 3, 1), D(2750)),
        (date(2008, 10, 30), D(4250)),
        (date(2009, 2, 15), D(3250)),
        (date(2009, 4, 1), D(2750)),
    ]
    out = {
        "yf isda 1998": yf(date(2003, 11, 1), date(2004, 5, 1), "act_act_isda"),
        "yf act365 2024": yf(date(2024, 1, 1), date(2025, 1, 1), "act_365_fixed"),
        "yf act360": yf(date(2024, 1, 15), date(2024, 7, 15), "act_360"),
        "yf 30/360 31->31": yf(date(2024, 1, 31), date(2024, 3, 31), "thirty_360"),
        "yf 30/360 15->31": yf(date(2024, 2, 15), date(2024, 3, 31), "thirty_360"),
        "yf 30E/360 15->31": yf(date(2024, 2, 15), date(2024, 3, 31), "thirty_e_360"),
        "yf isda multi-year": yf(date(2023, 7, 1), date(2026, 3, 15), "act_act_isda"),
        "conv 12% monthly->annual": convert(D("0.12"), "monthly", "annual"),
        "conv 10% annual->continuous": convert(D("0.10"), "annual", "continuous"),
        "conv 5% continuous->quarterly": convert(D("0.05"), "continuous", "quarterly"),
        # SPEC-0010 AC14: the two compounding frequencies added by R20.
        "conv 7.35% annual->bimonthly": convert(D("0.0735"), "annual", "bimonthly"),
        "conv 7.35% annual->four_monthly": convert(D("0.0735"), "annual", "four_monthly"),
        "real 8% nominal, 3% inflation": D("1.08") / D("1.03") - 1,
        "nominal 2% real, 3% inflation": D("1.02") * D("1.03") - 1,
        "xnpv ecma": xnpv(D("0.09"), ecma),
        "xnpv ecma valued 2007-07-01": xnpv(D("0.09"), ecma, date(2007, 7, 1)),
        "xnpv ecma act_act_isda quarterly": xnpv(
            D("0.09"), ecma, None, "act_act_isda", "quarterly"
        ),
        "xnpv ecma four_monthly": xnpv(D("0.09"), ecma, None, "act_365_fixed", "four_monthly"),
        "xirr ecma": xirr(ecma),
        "mirr 5y 10/12": mirr(
            [D(x) for x in (-120000, 39000, 30000, 21000, 37000, 46000)], D("0.10"), D("0.12")
        ),
        "mirr 3y 10/12": mirr([D(x) for x in (-120000, 39000, 30000, 21000)], D("0.10"), D("0.12")),
        "mirr 5y 10/14": mirr(
            [D(x) for x in (-120000, 39000, 30000, 21000, 37000, 46000)], D("0.10"), D("0.14")
        ),
        "annuity 1000 5% 10": annuity(D(1000), D("0.05"), 10),
        "annuity due 1000 5% 10": annuity(D(1000), D("0.05"), 10, due=True),
        "annuity growing 1000 8% 3% 20": annuity(D(1000), D("0.08"), 20, D("0.03")),
        "annuity r=g 1000 6% 6% 15": annuity(D(1000), D("0.06"), 15, D("0.06")),
        "perpetuity 100 8% 2%": perpetuity(D(100), D("0.08"), D("0.02")),
        "perpetuity due 100 8%": perpetuity(D(100), D("0.08"), due=True),
    }
    pv_c, tot_c = present_value([D(100), D(110), D(121)], [D("0.10")] * 3, "end_of_period")
    pv_m, tot_m = present_value([D(100)] * 3, [D("0.05"), D("0.06"), D("0.07")], "mid_period")
    pv_b, tot_b = present_value(
        [D(100)] * 3, [D("0.05"), D("0.06"), D("0.07")], "beginning_of_period"
    )
    pv_e, tot_e = present_value([D(100)] * 3, [D("0.05"), D("0.06"), D("0.07")], "end_of_period")
    out |= {
        "pv flat end": tot_c,
        "pv curve mid": tot_m,
        "pv curve begin": tot_b,
        "pv curve end": tot_e,
    }
    for k, v in out.items():
        print(f"{k}: {v.quantize(Q)}")
