"""SPEC-0009 against the independent 50-digit reference (scripts/reference_loans.py).

Seeded random loans, deterministic and dependency-free: instalments, numbers of instalments
and full schedules (every row) for the three amortisation types, with grace periods, balloon
amounts and rounding. The golden cases pin selected values; this test covers the space between
them.
"""

from __future__ import annotations

import importlib.util
import random
import sys
from decimal import Decimal as D
from pathlib import Path

import pytest

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("ref_loans", ROOT / "scripts/reference_loans.py")
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
sys.modules["ref_loans"] = ref  # dataclasses resolve annotations through sys.modules
_spec.loader.exec_module(ref)

RNG = random.Random(20261108)
TOL = D("1e-18")
# Schedule rows: the engine carries 28 significant digits and an error in the instalment grows
# by (1 + i) per period, so the bound is relative to the principal, on loans of realistic term.
REL = D("1e-22")
FREQUENCIES = ["annual", "semi_annual", "quarterly", "monthly"]
COMPOUNDINGS = [*FREQUENCIES, "continuous"]


PER_YEAR = {"annual": 1, "semi_annual": 2, "quarterly": 4, "monthly": 12}


def random_rate() -> tuple[dict[str, str], D, int]:
    """A rate in form (a) or (b), the periodic rate of the reference and the longest number of
    instalments of a realistic loan (40 years; 240 periods in form b)."""
    if RNG.random() < 0.5:
        i = D(RNG.randrange(0, 150)) / 10_000
        return {"periodic_rate": str(i)}, i, 240
    rate = D(RNG.randrange(0, 1200)) / 10_000
    compounding, frequency = RNG.choice(COMPOUNDINGS), RNG.choice(FREQUENCIES)
    form = {"rate": str(rate), "compounding": compounding, "payment_frequency": frequency}
    return form, ref.periodic_rate(rate, compounding, frequency), 40 * PER_YEAR[frequency]


def test_instalments_and_terms_match_the_reference() -> None:
    for _ in range(200):
        form, i, longest = random_rate()
        principal = D(RNG.randrange(1_000, 2_000_000))
        n = RNG.randrange(1, longest + 1)
        balloon = principal * D(RNG.randrange(0, 60)) / 100
        timing = RNG.choice(["ordinary", "due"])
        got = fm.run(
            "loans.instalment",
            {
                "principal": str(principal),
                "number_of_instalments": n,
                "timing": timing,
                "balloon_amount": str(balloon),
                **form,
            },
        )
        want = ref.instalment(principal, i, n, timing, balloon)
        assert abs(got.instalment - want) < TOL, (form, principal, n, balloon, timing)
        if balloon == 0 and i > 0:
            payment = (want * D("1.03")).quantize(D("0.01"))
            term = fm.run(
                "loans.number_of_instalments",
                {"principal": str(principal), "instalment": str(payment), "timing": timing, **form},
            )
            expected = ref.number_of_instalments(principal, i, payment, timing)
            assert term.number_of_instalments == expected.count
            assert abs(term.exact_number_of_periods - expected.exact) < TOL
            assert abs(term.final_instalment - expected.final) < D("1e-15")


@pytest.mark.parametrize("kind", ["french", "fixed_amortisation_schedule", "bullet"])
def test_schedules_match_the_reference_row_by_row(kind: str) -> None:
    for _ in range(60):
        form, i, longest = random_rate()
        principal = D(RNG.randrange(1_000, 1_000_000))
        n = RNG.randrange(1, longest + 1)
        payload: dict[str, object] = {
            "principal": str(principal),
            "number_of_instalments": n,
            "timing": "ordinary",
            "amortisation_type": kind,
            **form,
        }
        grace, balloon, rounding = 0, D(0), None
        if kind != "bullet":
            grace = RNG.randrange(0, min(n, 6))
            balloon = principal * D(RNG.randrange(0, 40)) / 100
            payload |= {"principal_grace_periods": grace, "balloon_amount": str(balloon)}
        if RNG.random() < 0.5:
            rounding = (RNG.randrange(0, 4), RNG.choice(["half_up", "half_even"]))
            payload["rounding"] = {"decimal_places": rounding[0], "mode": rounding[1]}
        want = ref.schedule(principal, i, n, kind, grace, balloon, rounding)
        try:
            got = fm.run("loans.amortisation_schedule", payload)
        except fm.FairmarkError as err:
            assert err.code == "FM-E023" and want.error == "FM-E023", payload
            continue
        assert want.error is None, payload
        assert len(got.rows) == len(want.rows)
        for row, expected in zip(got.rows, want.rows, strict=True):
            for field, value in (
                ("instalment", expected.instalment),
                ("interest_portion", expected.interest),
                ("principal_portion", expected.principal),
                ("outstanding_balance", expected.balance),
            ):
                assert abs(row[field] - value) < REL * principal, (payload, row["period"], field)
        assert abs(got.final_adjustment - want.final_adjustment) < REL * principal, payload
