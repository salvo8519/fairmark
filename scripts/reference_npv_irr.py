"""Independent 50-digit recomputation of three golden cases of `tvm.npv` and `tvm.irr`.

Written on 2026-11-08 to give these cases, which predate the `source` field, an independent
source (maintainer decision on #149). It shares no code with the engine or with
scripts/reference_values.py and uses Python's `decimal` module only:

  NPV = sum CF_t / (1 + r)^t, t = 0, 1, ..., n (first flow at time 0, undiscounted)
  IRR = the rate r with NPV(r) = 0, by bisection on [0, 1] to 1e-45 (each project has one
        sign change in its flows, so the root in that bracket is unique), and, for a single
        payoff, the closed form (CF_n / -CF_0)^(1/n) - 1 as a second check.

    python scripts/reference_npv_irr.py          # print reference, expected and difference
    python scripts/reference_npv_irr.py --check  # exit 1 if any case is outside its tolerance
"""

from __future__ import annotations

import sys
from decimal import Decimal as D
from decimal import localcontext
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
PREC = 50


def npv(rate: D, flows: list[D]) -> D:
    with localcontext() as ctx:
        ctx.prec = PREC
        one_plus = 1 + rate
        return sum((cf / one_plus**t for t, cf in enumerate(flows)), D(0))


def irr_bisection(flows: list[D], low: D = D(0), high: D = D(1)) -> D:
    with localcontext() as ctx:
        ctx.prec = PREC
        f_low = npv(low, flows)
        if f_low * npv(high, flows) >= 0:
            raise ValueError("no sign change of NPV on the bracket")
        while high - low > D("1e-45"):
            mid = (low + high) / 2
            f_mid = npv(mid, flows)
            if (f_mid < 0) == (f_low < 0):
                low, f_low = mid, f_mid
            else:
                high = mid
        return (low + high) / 2


def irr_single_payoff(flows: list[D]) -> D:
    with localcontext() as ctx:
        ctx.prec = PREC
        n = len(flows) - 1
        ratio = flows[-1] / -flows[0]
        return (ratio.ln() / n).exp() - 1


def cases() -> list[tuple[str, D, D, D]]:
    """(case, reference, expected, tolerance) for each of the three cases."""
    out = []
    golden = ROOT / "tests/golden"
    npv_case = yaml.safe_load((golden / "tvm.npv/five-flows.yaml").read_text(encoding="utf-8"))
    flows = [D(str(x)) for x in npv_case["input"]["cash_flows"]]
    out.append(
        (
            "tvm.npv/five-flows",
            npv(D(npv_case["input"]["rate"]), flows),
            D(npv_case["expect"]["npv"]),
            D(npv_case["tolerance"]),
        )
    )
    for name in ("conventional", "zero-coupon"):
        case = yaml.safe_load((golden / f"tvm.irr/{name}.yaml").read_text(encoding="utf-8"))
        flows = [D(str(x)) for x in case["input"]["cash_flows"]]
        ref = irr_bisection(flows)
        if name == "zero-coupon":
            closed = irr_single_payoff(flows)
            assert abs(closed - ref) < D("1e-40"), (closed, ref)
        out.append((f"tvm.irr/{name}", ref, D(case["expect"]["irr"]), D(case["tolerance"])))
    return out


def main() -> None:
    failed = False
    for name, ref, expected, tol in cases():
        diff = abs(ref - expected)
        ok = diff <= tol
        failed |= not ok
        print(f"{name}: reference {ref} expected {expected} |diff| {diff:.3E} ok={ok}")
    if "--check" in sys.argv and failed:
        sys.exit(1)


if __name__ == "__main__":
    main()
