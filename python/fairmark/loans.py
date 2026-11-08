"""Loans: instalment, number of instalments and amortisation schedules.

Give the rate in exactly one form: ``rate`` with ``compounding`` and ``payment_frequency``
(an annual borrowing rate, converted to the rate per payment period), or ``periodic_rate``
(the rate per payment period). Rates are decimal fractions, amounts are in currency units.
"""

from __future__ import annotations

from decimal import Decimal
from typing import Any, Literal

from ._core import Result, run
from .dates import Compounding

Number = Decimal | int | str | float
PaymentFrequency = Literal[
    "annual", "semi_annual", "four_monthly", "quarterly", "bimonthly", "monthly"
]
Timing = Literal["ordinary", "due"]
RoundingMode = Literal["half_up", "half_even"]


def _rate_form(
    rate: Number | None,
    compounding: Compounding | None,
    payment_frequency: PaymentFrequency | None,
    periodic_rate: Number | None,
) -> dict[str, Any]:
    fields = {
        "rate": rate,
        "compounding": compounding,
        "payment_frequency": payment_frequency,
        "periodic_rate": periodic_rate,
    }
    return {k: v for k, v in fields.items() if v is not None}


def _rounding(decimal_places: int | None, mode: RoundingMode | None) -> dict[str, Any]:
    if decimal_places is None and mode is None:
        return {}
    rounding: dict[str, Any] = {}
    if decimal_places is not None:
        rounding["decimal_places"] = decimal_places
    if mode is not None:
        rounding["mode"] = mode
    return {"rounding": rounding}


def instalment(
    principal: Number,
    number_of_instalments: int,
    *,
    timing: Timing,
    rate: Number | None = None,
    compounding: Compounding | None = None,
    payment_frequency: PaymentFrequency | None = None,
    periodic_rate: Number | None = None,
    balloon_amount: Number | None = None,
    decimal_places: int | None = None,
    rounding_mode: RoundingMode | None = None,
) -> Result:
    """Constant instalment of a loan, with an optional balloon amount and rounding.

    Rounding needs both ``decimal_places`` and ``rounding_mode``; without them the instalment
    is not rounded.

    >>> instalment(10000, 10, rate="0.08", compounding="monthly",
    ...            payment_frequency="monthly", timing="ordinary").instalment
    Decimal('1037.032089359152175652094834')
    """
    payload: dict[str, Any] = {
        "principal": principal,
        "number_of_instalments": number_of_instalments,
        "timing": timing,
        **_rate_form(rate, compounding, payment_frequency, periodic_rate),
        **_rounding(decimal_places, rounding_mode),
    }
    if balloon_amount is not None:
        payload["balloon_amount"] = balloon_amount
    return run("loans.instalment", payload)


def number_of_instalments(
    principal: Number,
    instalment: Number,
    *,
    timing: Timing,
    rate: Number | None = None,
    compounding: Compounding | None = None,
    payment_frequency: PaymentFrequency | None = None,
    periodic_rate: Number | None = None,
) -> Result:
    """Number of instalments that repays a loan, the exact number of periods and the final
    instalment.

    >>> r = number_of_instalments(3000, 800, periodic_rate="0.02", timing="ordinary")
    >>> r.number_of_instalments, r.final_instalment
    (4, Decimal('750.01008000'))
    """
    payload: dict[str, Any] = {
        "principal": principal,
        "instalment": instalment,
        "timing": timing,
        **_rate_form(rate, compounding, payment_frequency, periodic_rate),
    }
    return run("loans.number_of_instalments", payload)


def amortisation_schedule(
    principal: Number,
    number_of_instalments: int,
    *,
    amortisation_type: Literal["french", "fixed_amortisation_schedule", "bullet"],
    timing: Timing,
    rate: Number | None = None,
    compounding: Compounding | None = None,
    payment_frequency: PaymentFrequency | None = None,
    periodic_rate: Number | None = None,
    principal_grace_periods: int | None = None,
    balloon_amount: Number | None = None,
    decimal_places: int | None = None,
    rounding_mode: RoundingMode | None = None,
) -> Result:
    """Amortisation schedule: one row per period with instalment, interest portion, principal
    portion and outstanding balance, the totals and the final-period adjustment.

    ``amortisation_type`` is ``french`` (constant instalment), ``fixed_amortisation_schedule``
    (constant principal portion) or ``bullet`` (principal repaid with the last instalment).
    Version 1 supports payments in arrears only: ``timing="ordinary"`` (``"due"`` is refused
    with ``FM-E001``).

    >>> s = amortisation_schedule(120000, 8, periodic_rate="0.015", timing="ordinary",
    ...                           amortisation_type="fixed_amortisation_schedule")
    >>> s.rows[0]["instalment"], s.totals["interest"]
    (Decimal('16800.000'), Decimal('8100.000'))
    """
    payload: dict[str, Any] = {
        "principal": principal,
        "number_of_instalments": number_of_instalments,
        "timing": timing,
        "amortisation_type": amortisation_type,
        **_rate_form(rate, compounding, payment_frequency, periodic_rate),
        **_rounding(decimal_places, rounding_mode),
    }
    if principal_grace_periods is not None:
        payload["principal_grace_periods"] = principal_grace_periods
    if balloon_amount is not None:
        payload["balloon_amount"] = balloon_amount
    return run("loans.amortisation_schedule", payload)
