"""Independent reference implementation used to produce golden expectations.

Deliberately written from the textbook formulas with Python's `decimal`
module at 50 digits, sharing no code with the Rust engine.
"""

from decimal import Decimal as D
from decimal import getcontext

getcontext().prec = 50


def dcf(fcf, r, g, mid=False, net_debt=D(0), noa=D(0)):
    n = len(fcf)
    pv = sum(c / (1 + r) ** (D(i + 1) - (D("0.5") if mid else 0)) for i, c in enumerate(fcf))
    tv = fcf[-1] * (1 + g) / (r - g)
    pv_tv = tv / (1 + r) ** n
    ev = pv + pv_tv
    return {
        "enterprise_value": ev,
        "equity_value": ev - net_debt + noa,
        "terminal_value": tv,
        "pv_terminal_value": pv_tv,
        "pv_explicit_period": pv,
    }


def npv(r, cfs):
    return sum(c / (1 + r) ** i for i, c in enumerate(cfs))


def irr(cfs, lo=D("-0.99"), hi=D("10")):
    for _ in range(400):
        mid = (lo + hi) / 2
        if (npv(mid, cfs) > 0) == (npv(lo, cfs) > 0):
            lo = mid
        else:
            hi = mid
    return (lo + hi) / 2


if __name__ == "__main__":
    q = D("1e-12")
    print(
        "dcf end",
        {
            k: v.quantize(q)
            for k, v in dcf([D(100), D(110), D(120)], D("0.10"), D("0.02"), net_debt=D(200)).items()
        },
    )
    print(
        "dcf mid",
        {
            k: v.quantize(q)
            for k, v in dcf(
                [D(250), D(270), D(290), D(300), D(310)],
                D("0.085"),
                D("0.015"),
                mid=True,
                net_debt=D(-50),
                noa=D(30),
            ).items()
        },
    )
    print("npv", npv(D("0.07"), [D(-5000), D(1200), D(1500), D(1800), D(2100)]).quantize(q))
    print("irr", irr([D(-1000), D(300), D(400), D(500)]).quantize(q))
    print("irr2", irr([D(-2500), D(0), D(0), D(4000)]).quantize(q))
