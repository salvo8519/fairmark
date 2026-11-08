"""SPEC-0010 `series.project` against the independent 50-digit reference
(scripts/reference_series.py).

Seeded random series, deterministic and dependency-free: every growth rule, both base periods
and the three growth bases for period lengths that divide 12 or are multiples of 12. The golden
cases pin selected values; this test covers the space between them.
"""

from __future__ import annotations

import importlib.util
import random
import sys
from decimal import Decimal as D
from pathlib import Path

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("ref_series", ROOT / "scripts/reference_series.py")
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
sys.modules["ref_series"] = ref
_spec.loader.exec_module(ref)

RNG = random.Random(20261108)
# The engine carries 28 significant digits; each period adds at most a few units in the last
# place, and the compound conversion a relative error near 1e-27: 1e-22 relative is a wide
# margin for 60 periods.
REL = D("1e-22")
LENGTHS = [1, 2, 3, 4, 6, 12, 24, 36]


def test_values_match_the_reference() -> None:
    for _ in range(300):
        n = RNG.randrange(1, 61)
        base_period = RNG.choice(["period_0", "period_1"])
        steps = ref.growth_steps(n, base_period)
        months = RNG.choice(LENGTHS)
        basis = RNG.choice(["per_period", "annual_compound", "annual_simple"])
        base = D(RNG.randrange(-1_000_000, 1_000_000)) / 100
        kind = RNG.choice(["constant", "linear", "per_period"])
        rand_rate = lambda: D(RNG.randrange(-3000, 3000)) / 10_000  # noqa: E731
        if kind == "constant":
            g = rand_rate()
            growth, stated = {"constant": {"growth": str(g)}}, [g] * steps
        elif kind == "linear":
            a, b = rand_rate(), rand_rate()
            growth, stated = (
                {"linear": {"from_growth": str(a), "to_growth": str(b)}},
                ref.linear(a, b, steps),
            )
        else:
            stated = [rand_rate() for _ in range(steps)]
            growth = {"per_period": {"growth_rates": [str(r) for r in stated]}}
        got = fm.run(
            "series.project",
            {
                "base_value": str(base),
                "base_period": base_period,
                "growth": growth,
                "growth_basis": basis,
                "timeline": {"periods": {"number_of_periods": n, "period_months": months}},
            },
        )
        want, rates = ref.project(base, stated, base_period, basis, months)
        assert len(got.values) == n
        for k, (x, y) in enumerate(zip(got.values, want, strict=True)):
            scale = max(abs(y), D(1))
            assert abs(x - y) <= REL * scale, f"{kind} {basis} p={months} period {k + 1}"
        applied = [p["growth_per_period"] for p in got.periods if "growth_per_period" in p]
        for x, y in zip(applied, rates, strict=True):
            assert abs(x - y) <= D("1e-26"), f"{basis} p={months}: {x} vs {y}"
