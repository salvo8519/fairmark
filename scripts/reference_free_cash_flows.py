"""Independent reference for SPEC-0001 R2 to R4 (free cash flow build-up, `dcf.free_cash_flows`).

Written from the formulas of the spec, in Python's `decimal` module at 50 digits in a local
context (importing this module never changes the caller's precision). It shares no code with
the Rust engine.

    taxes_t = t x EBIT_t                      (rate form; `zero`: 0 when EBIT_t < 0)
    FCFF_t  = EBIT_t - taxes_t + D&A_t - capex_t - change in NWC_t + sum adjustments_t
    FCFE_t  = FCFF_t - interest_t x (1 - t) + net borrowing_t
              (t = 0 in a period where the `zero` treatment applies, SPEC-0001 amendment 14)

`net_income_route` computes the FCFE by the other route (net income + D&A - capex - change in
NWC + adjustments + net borrowing) as an independent check of amendment 14.

Usage:

    uv run python scripts/reference_free_cash_flows.py

prints the expected values of the golden cases in `tests/golden/dcf.free_cash_flows/`.
"""

from __future__ import annotations

from collections.abc import Callable, Mapping, Sequence
from dataclasses import dataclass, field
from decimal import Context, localcontext
from decimal import Decimal as D
from functools import wraps
from typing import Any, TypeVar

T = TypeVar("T", bound=Callable[..., Any])

CONTEXT = Context(prec=50)


def at_50_digits(func: T) -> T:
    @wraps(func)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        with localcontext(CONTEXT):
            return func(*args, **kwargs)

    return wrapper  # type: ignore[return-value]


@dataclass(frozen=True)
class Lines:
    """Line items of one period, in currency units."""

    ebit: D
    depreciation_and_amortisation: D
    capital_expenditure: D
    change_in_net_working_capital: D
    adjustments: Sequence[D] = field(default_factory=tuple)
    operating_taxes: D | None = None
    interest_expense: D | None = None
    net_borrowing: D | None = None


@dataclass(frozen=True)
class Period:
    taxes: D
    ebit_after_taxes: D
    fcff: D
    interest_after_taxes: D | None
    fcfe: D | None


@at_50_digits
def taxes_on_ebit(ebit: D, rate: D, negative_ebit: str) -> D:
    """R2, R4: rate x EBIT; with `zero`, no tax on a negative EBIT."""
    assert negative_ebit in ("credit", "zero")
    if ebit < 0 and negative_ebit == "zero":
        return D(0)
    return rate * ebit


@at_50_digits
def build_up(
    periods: Sequence[Lines],
    *,
    notional_tax_rate: D | None = None,
    negative_ebit: str | None = None,
    interest_tax_rate: D | None = None,
) -> list[Period]:
    """R2, R3, R4 for every period."""
    rate_form = notional_tax_rate is not None
    shield = notional_tax_rate if rate_form else interest_tax_rate
    out = []
    for p in periods:
        if rate_form:
            assert p.operating_taxes is None and negative_ebit is not None
            taxes = taxes_on_ebit(p.ebit, notional_tax_rate, negative_ebit)  # type: ignore[arg-type]
        else:
            assert p.operating_taxes is not None
            taxes = p.operating_taxes
        after = p.ebit - taxes
        fcff = (
            after
            + p.depreciation_and_amortisation
            - p.capital_expenditure
            - p.change_in_net_working_capital
            + sum(p.adjustments, D(0))
        )
        interest_after = fcfe = None
        if p.interest_expense is not None:
            assert p.net_borrowing is not None and shield is not None
            # Amendment 14: no interest tax shield where the `zero` treatment applies.
            no_shield = rate_form and negative_ebit == "zero" and p.ebit < 0
            rate = D(0) if no_shield else shield
            interest_after = p.interest_expense * (1 - rate)
            fcfe = fcff - interest_after + p.net_borrowing
        out.append(Period(taxes, after, fcff, interest_after, fcfe))
    return out


@at_50_digits
def net_income_route(p: Lines, rate: D, negative_ebit: str) -> D:
    """FCFE from net income: taxes on pre-tax income EBIT - interest at `rate`, with no tax on a
    negative pre-tax income under `zero` (CFA Institute; Damodaran). Valid where it agrees with
    the treatments: `credit` in any period, `zero` in periods with a negative EBIT."""
    assert p.interest_expense is not None and p.net_borrowing is not None
    pre_tax = p.ebit - p.interest_expense
    if negative_ebit == "zero":
        assert p.ebit < 0
        tax = D(0)
    else:
        tax = rate * pre_tax
    net_income = pre_tax - tax
    return (
        net_income
        + p.depreciation_and_amortisation
        - p.capital_expenditure
        - p.change_in_net_working_capital
        + sum(p.adjustments, D(0))
        + p.net_borrowing
    )


def lines_from_json(period: Mapping[str, Any]) -> Lines:
    """The JSON shape of a golden case input, read into the reference structure."""

    def opt(key: str) -> D | None:
        return D(str(period[key])) if key in period else None

    return Lines(
        ebit=D(str(period["ebit"])),
        depreciation_and_amortisation=D(str(period["depreciation_and_amortisation"])),
        capital_expenditure=D(str(period["capital_expenditure"])),
        change_in_net_working_capital=D(str(period["change_in_net_working_capital"])),
        adjustments=tuple(D(str(a["value"])) for a in period.get("other_adjustments", [])),
        operating_taxes=opt("operating_taxes"),
        interest_expense=opt("interest_expense"),
        net_borrowing=opt("net_borrowing"),
    )


def build_up_json(case_input: Mapping[str, Any]) -> list[Period]:
    def opt(key: str) -> D | None:
        return D(str(case_input[key])) if key in case_input else None

    return build_up(
        [lines_from_json(p) for p in case_input["periods"]],
        notional_tax_rate=opt("notional_tax_rate"),
        negative_ebit=case_input.get("negative_ebit"),
        interest_tax_rate=opt("interest_tax_rate"),
    )


# Inputs of the golden cases computed by this reference (the files repeat them).
CASES: dict[str, dict[str, Any]] = {
    "rate-form-fcff-and-fcfe": {
        "notional_tax_rate": "0.24",
        "negative_ebit": "credit",
        "periods": [
            {
                "ebit": "1200",
                "depreciation_and_amortisation": "350",
                "capital_expenditure": "420",
                "change_in_net_working_capital": "65",
                "interest_expense": "80",
                "net_borrowing": "-150",
            },
            {
                "ebit": "1315.5",
                "depreciation_and_amortisation": "372.25",
                "capital_expenditure": "460",
                "change_in_net_working_capital": "-12.4",
                "interest_expense": "72.6",
                "net_borrowing": "40",
            },
            {
                "ebit": "1401.37",
                "depreciation_and_amortisation": "390",
                "capital_expenditure": "395.8",
                "change_in_net_working_capital": "48.03",
                "interest_expense": "69.15",
                "net_borrowing": "0",
            },
        ],
    },
    "amounts-form-with-adjustments": {
        "interest_tax_rate": "0.279",
        "periods": [
            {
                "ebit": "845.7",
                "operating_taxes": "236.0",
                "depreciation_and_amortisation": "120.4",
                "capital_expenditure": "180",
                "change_in_net_working_capital": "22.15",
                "other_adjustments": [
                    {"label": "Disposal of a warehouse", "value": "60"},
                    {"label": "Provision utilisation", "value": "-14.3"},
                ],
                "interest_expense": "41.2",
                "net_borrowing": "25",
            },
            {
                "ebit": "910.25",
                "operating_taxes": "253.96",
                "depreciation_and_amortisation": "128.1",
                "capital_expenditure": "205.5",
                "change_in_net_working_capital": "18.4",
                "other_adjustments": [{"label": "Provision utilisation", "value": "-9.85"}],
                "interest_expense": "38.75",
                "net_borrowing": "-30",
            },
        ],
    },
    "negative-ebit-credit": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "credit",
        "periods": [
            {
                "ebit": "-250",
                "depreciation_and_amortisation": "90",
                "capital_expenditure": "60",
                "change_in_net_working_capital": "-35",
                "interest_expense": "30",
                "net_borrowing": "100",
            },
            {
                "ebit": "180",
                "depreciation_and_amortisation": "95",
                "capital_expenditure": "70",
                "change_in_net_working_capital": "12",
                "interest_expense": "33",
                "net_borrowing": "-20",
            },
        ],
    },
    "negative-ebit-zero": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "-250",
                "depreciation_and_amortisation": "90",
                "capital_expenditure": "60",
                "change_in_net_working_capital": "-35",
                "interest_expense": "30",
                "net_borrowing": "100",
            },
            {
                "ebit": "180",
                "depreciation_and_amortisation": "95",
                "capital_expenditure": "70",
                "change_in_net_working_capital": "12",
                "interest_expense": "33",
                "net_borrowing": "-20",
            },
        ],
    },
    "boeing-rate-form-recomputed": {
        "notional_tax_rate": "0.35",
        "negative_ebit": "credit",
        "periods": [
            {
                "ebit": "1753",
                "depreciation_and_amortisation": "1517",
                "capital_expenditure": "1584",
                "change_in_net_working_capital": "-105",
            }
        ],
    },
    "negative-ebit-zero-net-income-route": {
        "notional_tax_rate": "0.24",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "-120",
                "depreciation_and_amortisation": "45",
                "capital_expenditure": "30",
                "change_in_net_working_capital": "-8",
                "other_adjustments": [{"label": "Grant received", "value": "12.5"}],
                "interest_expense": "50",
                "net_borrowing": "20",
            }
        ],
    },
    "positive-ebit-below-interest": {
        "notional_tax_rate": "0.24",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "30",
                "depreciation_and_amortisation": "15",
                "capital_expenditure": "10",
                "change_in_net_working_capital": "2",
                "interest_expense": "50",
                "net_borrowing": "0",
            }
        ],
    },
    "full-precision-rate-form": {
        "notional_tax_rate": "0.2793456789123",
        "negative_ebit": "credit",
        "periods": [
            {
                "ebit": "98765432.12345678901",
                "depreciation_and_amortisation": "98765.4321",
                "capital_expenditure": "123456.789",
                "change_in_net_working_capital": "-4321.0987",
                "other_adjustments": [{"label": "Grant received", "value": "777.777"}],
            }
        ],
    },
}


def main() -> None:
    for name, case in CASES.items():
        print(name)
        for k, p in enumerate(build_up_json(case)):
            print(f"  period {k + 1}: taxes {p.taxes}, after taxes {p.ebit_after_taxes}")
            print(f"    FCFF {p.fcff}")
            if p.fcfe is not None:
                print(f"    interest after taxes {p.interest_after_taxes}, FCFE {p.fcfe}")
    case = CASES["negative-ebit-zero-net-income-route"]
    p = lines_from_json(case["periods"][0])
    print("net-income route:", net_income_route(p, D(case["notional_tax_rate"]), "zero"))
    with localcontext(CONTEXT):
        boeing = D(1753) * (1 - D("0.35"))
    print(f"Boeing: 1,753 x (1 - 0.35) = {boeing}; published EBIT (1 - t) 1,138")
    print(
        "Boeing from the published 1,138:",
        build_up(
            [
                Lines(
                    ebit=D(1753),
                    operating_taxes=D(615),
                    depreciation_and_amortisation=D(1517),
                    capital_expenditure=D(1584),
                    change_in_net_working_capital=D(-105),
                )
            ]
        )[0].fcff,
    )


if __name__ == "__main__":
    main()
