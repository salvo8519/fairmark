"""Independent reference for SPEC-0009 (loans: instalment, number of instalments, future value
and amortisation schedules).

Written from the closed forms and the period recurrence of the spec with Python's `decimal`
module at 50 significant digits in a local context (importing this module never changes the
caller's precision). It shares no code with the Rust engine and is used to produce or confirm
golden expectations:

    uv run python scripts/reference_loans.py

Independence from the engine, by construction:
- the periodic rate is the p-th root of the growth over one year, through `exp` and `ln`
  (the engine converts to a nominal rate with `rates.convert` and divides);
- the instalment is the closed form of R6 written directly (the engine divides by the
  annuity factor of `tvm.annuity`);
- the future value uses `((1 + r)^n - 1) / r` (the engine accumulates the annuity factor);
- schedules follow the period recurrence of R12 to R20 row by row.
"""

from __future__ import annotations

from collections.abc import Callable
from dataclasses import dataclass, field
from decimal import ROUND_CEILING, ROUND_HALF_EVEN, ROUND_HALF_UP, Context, localcontext
from decimal import Decimal as D
from functools import wraps
from typing import Any, TypeVar

T = TypeVar("T", bound=Callable[..., Any])

CONTEXT = Context(prec=50)
PER_YEAR = {"annual": 1, "semi_annual": 2, "quarterly": 4, "monthly": 12}
MODES = {"half_up": ROUND_HALF_UP, "half_even": ROUND_HALF_EVEN}


def at_50_digits(func: T) -> T:
    @wraps(func)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        with localcontext(CONTEXT):
            return func(*args, **kwargs)

    return wrapper  # type: ignore[return-value]


# ---------------------------------------------------------------- rates (R1)
@at_50_digits
def periodic_rate(rate: D, compounding: str, payment_frequency: str) -> D:
    """Rate per payment period with the same growth over one year as `rate` (R1, form a)."""
    if compounding == "continuous":
        log_growth = rate
    else:
        m = PER_YEAR[compounding]
        log_growth = m * (1 + rate / m).ln()
    p = PER_YEAR[payment_frequency]
    return (log_growth / p).exp() - 1


def _round(x: D, rounding: tuple[int, str] | None) -> D:
    if rounding is None:
        return x
    places, mode = rounding
    return x.quantize(D(1).scaleb(-places), rounding=MODES[mode])


# ---------------------------------------------------------------- instalment (R6)
@at_50_digits
def instalment(
    principal: D,
    i: D,
    n: int,
    timing: str,
    balloon: D = D(0),
    rounding: tuple[int, str] | None = None,
) -> D:
    """A = (P - B (1 + i)^-n) * i / (1 - (1 + i)^-n), divided by (1 + i) when due."""
    if i == 0:
        a = (principal - balloon) / n
    else:
        v_n = (1 + i) ** -n
        a = (principal - balloon * v_n) * i / (1 - v_n)
        if timing == "due":
            a = a / (1 + i)
    return _round(a, rounding)


# ---------------------------------------------------------------- number of instalments (R8)
@dataclass
class Term:
    exact: D
    count: int
    final: D


@at_50_digits
def number_of_instalments(principal: D, i: D, a: D, timing: str) -> Term:
    """n* = -ln(1 - i P' / A) / ln(1 + i); count = ⌈n*⌉; final instalment by recurrence."""
    if i == 0:
        exact = principal / a
    else:
        p_eff = principal if timing == "ordinary" else principal / (1 + i)
        exact = -(1 - i * p_eff / a).ln() / (1 + i).ln()
    nearest = exact.to_integral_value()
    count = int(
        nearest if abs(exact - nearest) <= D("1e-15") else exact.to_integral_value(ROUND_CEILING)
    )
    # Amount owed on the date of payment k: ordinary at time k, due at time k - 1.
    owed = principal * (1 + i) if timing == "ordinary" else principal
    for _ in range(count - 1):
        owed = (owed - a) * (1 + i)
    return Term(exact, count, owed)


# ---------------------------------------------------------------- future value (R10)
@at_50_digits
def future_value(pv: D, pmt: D, r: D, n: int, timing: str) -> D:
    """FV = PV (1 + r)^n + PMT ((1 + r)^n - 1) / r, the payment term x (1 + r) when due."""
    if r == 0:
        return pv + n * pmt
    g = (1 + r) ** n
    payments = pmt * (g - 1) / r
    if timing == "due":
        payments *= 1 + r
    return pv * g + payments


# ---------------------------------------------------------------- schedules (R12 to R20)
@dataclass
class Row:
    period: int
    instalment: D
    interest: D
    principal: D
    balance: D
    balloon: D | None = None


@dataclass
class Schedule:
    rows: list[Row] = field(default_factory=list)
    final_adjustment: D = D(0)
    regular: D = D(0)
    error: str | None = None

    @property
    def total_instalments(self) -> D:
        return sum((r.instalment for r in self.rows), D(0))

    @property
    def total_interest(self) -> D:
        return sum((r.interest for r in self.rows), D(0))

    @property
    def total_principal(self) -> D:
        return sum((r.principal for r in self.rows), D(0))


@at_50_digits
def schedule(
    principal: D,
    i: D,
    n: int,
    kind: str,
    grace: int = 0,
    balloon: D = D(0),
    rounding: tuple[int, str] | None = None,
) -> Schedule:
    """One row per period; instalment k at time k (payments in arrears)."""
    out = Schedule()
    m = n - grace
    if kind == "french":
        regular = instalment(principal, i, m, "ordinary", balloon, rounding)
    elif kind == "fixed_amortisation_schedule":
        regular = _round((principal - balloon) / m, rounding)
    else:  # bullet
        regular = D(0)
    out.regular = regular
    balance = principal
    for k in range(1, n + 1):
        interest = i * balance
        if k == n:
            principal_k = balance
            pay = interest + principal_k
            if kind == "french":
                out.final_adjustment = pay - balloon - regular
            elif kind == "fixed_amortisation_schedule":
                out.final_adjustment = principal_k - balloon - regular
            if rounding is not None and (principal_k < 0 or pay < 0):
                out.error = "FM-E023"
            balance -= principal_k
            out.rows.append(Row(k, pay, interest, principal_k, balance, balloon))
            break
        if k <= grace or kind == "bullet":
            principal_k = D(0)
        elif kind == "french":
            principal_k = regular - interest
        else:
            principal_k = regular
        balance -= principal_k
        if rounding is not None and balance < 0:
            out.error = "FM-E023"
            return out
        out.rows.append(Row(k, interest + principal_k, interest, principal_k, balance))
    return out


@at_50_digits
def present_value_of_rows(rows: list[Row], i: D) -> D:
    return sum((r.instalment / (1 + i) ** r.period for r in rows), D(0))


if __name__ == "__main__":
    i = periodic_rate(D("0.08"), "monthly", "monthly")
    print("ECMA-376 instalment, arrears:", instalment(D(10000), i, 10, "ordinary"))
    print("ECMA-376 instalment, advance:", instalment(D(10000), i, 10, "due"))
    print("ECMA-376 future value:", future_value(D(0), D(1000), D("0.01"), 12, "ordinary"))
    j = periodic_rate(D("0.05"), "quarterly", "semi_annual")
    s = schedule(D(50000), j, 6, "french", rounding=(2, "half_up"))
    print("Watts 6.1, 50,000 loan: periodic rate", j)
    for row in s.rows:
        print(" ", row.period, row.instalment, row.interest, row.principal, row.balance)
    print("  final adjustment", s.final_adjustment, "total interest", s.total_interest)
