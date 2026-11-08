"""Independent reference for SPEC-0001 R2 to R4 (free cash flow build-up, `dcf.free_cash_flows`).

Written from the formulas of the spec, in Python's `decimal` module at 50 digits in a local
context (importing this module never changes the caller's precision). It shares no code with
the Rust engine.

    taxes_t = t x EBIT_t                      (rate form; `zero`: 0 when EBIT_t < 0)
    FCFF_t  = EBIT_t - taxes_t + D&A_t - capex_t - change in NWC_t + sum adjustments_t
    FCFE_t  = FCFF_t - interest_t + shield_t + net borrowing_t
    shield_t = t x interest_t                          (`credit`, and the amounts form)
    shield_t = t x min(interest_t, max(EBIT_t, 0))     (`zero`, SPEC-0001 Decision 22)

Under `zero` the deduction of interest is limited to the taxable income available, so the
total taxes, t x max(EBIT_t, 0) - shield_t, are never below zero (Decision 22 amends
amendment 14, which removed the shield only when EBIT was negative).

`net_income_route` computes the FCFE by the other route (net income + D&A - capex - change in
NWC + adjustments + net borrowing, with taxes on the pre-tax income EBIT - interest) as an
independent check of Decision 22: under `zero` there is no tax on a pre-tax loss.

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
    interest_tax_shield: D | None
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
def interest_tax_shield(ebit: D, interest: D, rate: D, limited: bool) -> D:
    """R3 and Decision 22: t x interest; under `zero` the deductible interest is limited to the
    taxable income available, max(EBIT, 0)."""
    if not limited:
        return rate * interest
    deductible = min(interest, max(ebit, D(0)))
    return rate * deductible


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
        interest_after = fcfe = tax_shield = None
        if p.interest_expense is not None:
            assert p.net_borrowing is not None and shield is not None
            tax_shield = interest_tax_shield(
                p.ebit, p.interest_expense, shield, rate_form and negative_ebit == "zero"
            )
            interest_after = p.interest_expense - tax_shield
            fcfe = fcff - interest_after + p.net_borrowing
        out.append(Period(taxes, after, fcff, tax_shield, interest_after, fcfe))
    return out


@at_50_digits
def net_income_route(p: Lines, rate: D, negative_ebit: str) -> D:
    """FCFE from net income: taxes on the pre-tax income EBIT - interest at `rate`; under `zero`
    no tax on a pre-tax loss (no tax credit arises), under `credit` a loss gives a tax saving
    (CFA Institute; Damodaran, Investment Valuation, chapter 14). Agrees with the build-up of
    both treatments in every period (Decision 22)."""
    assert p.interest_expense is not None and p.net_borrowing is not None
    pre_tax = p.ebit - p.interest_expense
    no_tax_on_a_loss = negative_ebit == "zero" and pre_tax <= 0
    tax = D(0) if no_tax_on_a_loss else rate * pre_tax
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


def stages_with_flows(dcf_input: Mapping[str, Any]) -> dict[str, Any]:
    """SPEC-0001 R7: a `dcf.fcff` input whose `given` stages carry `lines`, with the lines
    replaced by the FCFF built up here, so that `scripts/reference_dcf.py` values it."""
    out = {k: v for k, v in dcf_input.items() if k not in ("notional_tax_rate", "negative_ebit")}
    stages = []
    for stage in dcf_input["stages"]:
        stage = dict(stage)
        if "lines" in stage:
            built = build_up(
                [lines_from_json(p) for p in stage.pop("lines")],
                notional_tax_rate=(
                    D(str(dcf_input["notional_tax_rate"]))
                    if "notional_tax_rate" in dcf_input
                    else None
                ),
                negative_ebit=dcf_input.get("negative_ebit"),
            )
            stage["flows"] = [str(p.fcff) for p in built]
        stages.append(stage)
    out["stages"] = stages
    return out


def equity_stages_with_flows(dcf_input: Mapping[str, Any]) -> dict[str, Any]:
    """SPEC-0001 R3, R7 on the equity basis: a `dcf.fcfe` input whose `given` stages carry
    `lines` with interest expense and net borrowing, with the lines replaced by the FCFE built
    up here, so that `scripts/reference_dcf.py` values it."""
    tax_fields = ("notional_tax_rate", "negative_ebit", "interest_tax_rate")
    out = {k: v for k, v in dcf_input.items() if k not in tax_fields}

    def opt(key: str) -> D | None:
        return D(str(dcf_input[key])) if key in dcf_input else None

    stages = []
    for stage in dcf_input["stages"]:
        stage = dict(stage)
        if "lines" in stage:
            built = build_up(
                [lines_from_json(p) for p in stage.pop("lines")],
                notional_tax_rate=opt("notional_tax_rate"),
                negative_ebit=dcf_input.get("negative_ebit"),
                interest_tax_rate=opt("interest_tax_rate"),
            )
            assert all(p.fcfe is not None for p in built), "FCFE needs the financing lines"
            stage["flows"] = [str(p.fcfe) for p in built]
        stages.append(stage)
    out["stages"] = stages
    return out


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
    # Decision 22: the interest tax shield under both treatments.
    "shield-ebit-below-interest-zero": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "42.5",
                "depreciation_and_amortisation": "18",
                "capital_expenditure": "25.4",
                "change_in_net_working_capital": "3.1",
                "interest_expense": "61.75",
                "net_borrowing": "12",
            },
        ],
    },
    "shield-ebit-below-interest-credit": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "credit",
        "periods": [
            {
                "ebit": "42.5",
                "depreciation_and_amortisation": "18",
                "capital_expenditure": "25.4",
                "change_in_net_working_capital": "3.1",
                "interest_expense": "61.75",
                "net_borrowing": "12",
            },
        ],
    },
    "shield-negative-ebit-zero": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "-37.6",
                "depreciation_and_amortisation": "14.2",
                "capital_expenditure": "9.5",
                "change_in_net_working_capital": "-4.05",
                "interest_expense": "22.4",
                "net_borrowing": "30",
            },
        ],
    },
    "shield-negative-ebit-credit": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "credit",
        "periods": [
            {
                "ebit": "-37.6",
                "depreciation_and_amortisation": "14.2",
                "capital_expenditure": "9.5",
                "change_in_net_working_capital": "-4.05",
                "interest_expense": "22.4",
                "net_borrowing": "30",
            },
        ],
    },
    "shield-ebit-above-interest-zero": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "310.8",
                "depreciation_and_amortisation": "64.3",
                "capital_expenditure": "88.9",
                "change_in_net_working_capital": "11.6",
                "interest_expense": "47.25",
                "net_borrowing": "-25",
            },
        ],
    },
    "shield-ebit-above-interest-credit": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "credit",
        "periods": [
            {
                "ebit": "310.8",
                "depreciation_and_amortisation": "64.3",
                "capital_expenditure": "88.9",
                "change_in_net_working_capital": "11.6",
                "interest_expense": "47.25",
                "net_borrowing": "-25",
            },
        ],
    },
    "shield-zero-interest-zero": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "55",
                "depreciation_and_amortisation": "21",
                "capital_expenditure": "30.5",
                "change_in_net_working_capital": "2.4",
                "interest_expense": "0",
                "net_borrowing": "-15",
            },
            {
                "ebit": "-20",
                "depreciation_and_amortisation": "19.5",
                "capital_expenditure": "12",
                "change_in_net_working_capital": "-1.6",
                "interest_expense": "0",
                "net_borrowing": "10",
            },
        ],
    },
    "shield-zero-interest-credit": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "credit",
        "periods": [
            {
                "ebit": "55",
                "depreciation_and_amortisation": "21",
                "capital_expenditure": "30.5",
                "change_in_net_working_capital": "2.4",
                "interest_expense": "0",
                "net_borrowing": "-15",
            },
            {
                "ebit": "-20",
                "depreciation_and_amortisation": "19.5",
                "capital_expenditure": "12",
                "change_in_net_working_capital": "-1.6",
                "interest_expense": "0",
                "net_borrowing": "10",
            },
        ],
    },
    "shield-ebit-equal-interest-zero": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "80",
                "depreciation_and_amortisation": "26",
                "capital_expenditure": "31",
                "change_in_net_working_capital": "5",
                "interest_expense": "80",
                "net_borrowing": "0",
            },
        ],
    },
    "shield-ebit-zero-zero": {
        "notional_tax_rate": "0.279",
        "negative_ebit": "zero",
        "periods": [
            {
                "ebit": "0",
                "depreciation_and_amortisation": "12",
                "capital_expenditure": "8",
                "change_in_net_working_capital": "1.5",
                "interest_expense": "35",
                "net_borrowing": "20",
            },
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
                print(
                    f"    interest tax shield {p.interest_tax_shield}, interest after taxes "
                    f"{p.interest_after_taxes}, FCFE {p.fcfe}"
                )
                if case.get("negative_ebit") is not None:
                    lines = lines_from_json(case["periods"][k])
                    route = net_income_route(
                        lines, D(case["notional_tax_rate"]), case["negative_ebit"]
                    )
                    print(f"    net-income route {route}")
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
