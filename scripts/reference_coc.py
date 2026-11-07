"""Independent reference for SPEC-0003 (cost of capital), 50-digit decimal.

uv run python scripts/reference_coc.py
"""

from decimal import Decimal as D
from decimal import getcontext
from statistics import median

getcontext().prec = 50

PEERS = [  # (levered beta, D/E, tax rate)
    (D("1.15"), D("0.40"), D("0.24")),
    (D("0.95"), D("0.20"), D("0.25")),
    (D("1.30"), D("0.75"), D("0.21")),
    (D("1.05"), D("0.35"), D("0.24")),
    (D("0.88"), D("0.10"), D("0.28")),
]


def unlever_hamada(bl: D, de: D, t: D, bd: D = D(0)) -> D:
    return (bl + bd * (1 - t) * de) / (1 + (1 - t) * de)


def relever_hamada(bu: D, de: D, t: D, bd: D = D(0)) -> D:
    return bu + (bu - bd) * (1 - t) * de


if __name__ == "__main__":
    unlevered = [unlever_hamada(*p) for p in PEERS]
    mean = sum(unlevered) / len(unlevered)
    med = D(median(unlevered))
    print("unlevered:", [str(u.quantize(D("1e-28"))) for u in unlevered])
    print("mean:", mean.quantize(D("1e-28")), "median:", med.quantize(D("1e-28")))
    relevered = relever_hamada(med, D("0.5"), D("0.24"))
    print("median relevered at D/E 0.5, t 0.24:", relevered.quantize(D("1e-28")))
