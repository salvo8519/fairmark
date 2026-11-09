"""Independent reference for SPEC-0007 R7 to R11 (ratios from partial input), Python `decimal`.

Written from the formulas and the computability tables of `specs/0007-ratios-covenants/spec.md`
(amendment of 2026-11-09), sharing no code with the Rust engine or with
`scripts/reference_ratios.py`. Each function takes a method input as JSON-like data (amounts as
strings, absent keys meaning "not given") and returns the expected outputs, keyed by their JSON
pointer in the result value, and the list `not_computed` of `(output, missing)` pairs:

- an output is computed when every input it needs is given; otherwise it is `None` and listed
  with the sorted pointers of the inputs it lacks (R7);
- a ratio is not defined when the terms that are given already make it not defined (zero
  denominator, a sign rule); it is then `None` and not listed (R10);
- a ratio built from other ratios is not defined when a part is not defined, otherwise not
  computed when a part is not computed, with the union of their missing pointers (R10).

Usage (prints the `expect` block of every partial-input golden case):

    uv run python scripts/reference_ratios_partial.py

The arithmetic runs in a local 50-digit context.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from decimal import Context, localcontext
from decimal import Decimal as D
from typing import Any

CONTEXT = Context(prec=50)


@dataclass
class Val:
    """A computed amount, a value not defined, or a value not computed (with its pointers)."""

    value: D | None = None
    undefined: bool = False
    missing: list[str] = field(default_factory=list)

    @property
    def known(self) -> bool:
        return self.value is not None


def given(value: D) -> Val:
    return Val(value=value)


def lacking(*pointers: str) -> Val:
    return Val(missing=sorted(set(pointers)))


UNDEFINED = Val(undefined=True)


def get(data: dict[str, Any] | None, *path: str) -> Any:
    cur: Any = data
    for key in path:
        if not isinstance(cur, dict) or cur.get(key) is None:
            return None
        cur = cur[key]
    return cur


def amount(data: dict[str, Any], *path: str) -> Val:
    raw = get(data, *path)
    return lacking("/" + "/".join(path)) if raw is None else given(D(str(raw)))


def balance(data: dict[str, Any], *path: str) -> Val:
    """Closing, or (opening + closing) / 2 with average balances."""
    closing = amount(data, "closing", *path)
    if data["balances"] == "closing":
        return closing
    opening = amount(data, "opening", *path)
    if closing.known and opening.known:
        return given((opening.value + closing.value) / 2)  # type: ignore[operator]
    return lacking(*closing.missing, *opening.missing)


def combine(*parts: Val) -> list[str] | None:
    """Missing pointers of the parts, or None when every part is known."""
    missing = [p for part in parts for p in part.missing]
    return sorted(set(missing)) if any(not p.known for p in parts) else None


def ratio(
    num: Val, den: Val, *, positive_den: bool = False, positive_num: bool = False, days: int = 0
) -> Val:
    """num / den (times days): not defined first on the terms given, then not computed."""
    if den.known and (den.value == 0 or (positive_den and den.value < 0)):  # type: ignore[operator]
        return UNDEFINED
    if num.known and positive_num and num.value <= 0:  # type: ignore[operator]
        return UNDEFINED
    if not (num.known and den.known):
        return lacking(*num.missing, *den.missing)
    top = num.value * days if days else num.value  # type: ignore[operator]
    return given(D(top / den.value))  # type: ignore[operator]


def composite(parts: list[Val], fn: Any) -> Val:
    if any(p.undefined for p in parts):
        return UNDEFINED
    if any(not p.known for p in parts):
        return lacking(*(m for p in parts for m in p.missing))
    return given(fn(*(p.value for p in parts)))


def arith(fn: Any, *parts: Val) -> Val:
    """An amount (margin, net financial position): computed or not computed, never undefined."""
    if all(p.known for p in parts):
        return given(fn(*(p.value for p in parts)))
    return lacking(*(m for p in parts for m in p.missing))


def with_definition(value: Val, data: dict[str, Any], definition: str) -> Val:
    if data.get(definition) is None:
        return lacking(*value.missing, "/" + definition)
    return value


def report(outputs: dict[str, Val]) -> dict[str, Any]:
    values = {k: v.value for k, v in outputs.items()}
    nc = [(k, v.missing) for k, v in outputs.items() if not v.known and not v.undefined]
    return {"values": values, "not_computed": nc}


def nfp(data: dict[str, Any], which: str) -> Val:
    gross = amount(data, which, "net_financial_position", "gross_debt")
    cash = amount(data, which, "net_financial_position", "cash")
    items = get(data, which, "net_financial_position", "other_items") or []
    other = sum((D(str(i["amount"])) for i in items), D(0))
    return arith(lambda g, c: g - c + other, gross, cash)


def nfp_balance(data: dict[str, Any]) -> Val:
    closing = nfp(data, "closing")
    if data["balances"] == "closing":
        return closing
    opening = nfp(data, "opening")
    return arith(lambda o, c: (o + c) / 2, opening, closing)


def profitability(data: dict[str, Any]) -> dict[str, Any]:
    with localcontext(CONTEXT):

        def inc(k: str) -> Val:
            return amount(data, "income_statement", k)

        revenue, ebit, ni = inc("revenue"), inc("ebit"), inc("net_income")
        ebitda = with_definition(inc("ebitda"), data, "ebitda_definition")
        equity, assets = balance(data, "equity"), balance(data, "total_assets")
        base = balance(data, data["operating_return_base"])
        on_ic = data["operating_return_base"] == "invested_capital"
        roi = ratio(ebit, base, positive_den=on_ic)
        net_margin = ratio(ni, revenue)
        asset_turnover = ratio(revenue, assets)
        multiplier = ratio(assets, equity, positive_den=True)
        out = {
            "/roe": ratio(ni, equity, positive_den=True),
            "/roi": roi,
            "/roa": ratio(ebit, assets),
            "/ros": ratio(ebit, revenue),
            "/ebitda_margin": ratio(ebitda, revenue),
            "/ebit_margin": ratio(ebit, revenue),
            "/capital_turnover": ratio(revenue, base),
            "/roe_three_factor/net_margin": net_margin,
            "/roe_three_factor/asset_turnover": asset_turnover,
            "/roe_three_factor/equity_multiplier": multiplier,
            "/roe_three_factor/roe": composite(
                [net_margin, asset_turnover, multiplier], lambda a, b, c: a * b * c
            ),
        }
        names = (
            "roi",
            "cost_of_debt",
            "debt_to_equity",
            "leverage_effect",
            "pre_tax_roe",
            "tax_rate",
            "roe",
        )
        lev = data.get("leverage")
        if lev is None:
            out.update({f"/roe_leverage/{n}": lacking("/leverage") for n in names})
            return report(out)
        debt = balance(data, "debt")
        i = ratio(amount(data, "leverage", "financial_charges"), debt)
        de = ratio(debt, equity, positive_den=True)
        t = ratio(amount(data, "leverage", "taxes"), amount(data, "leverage", "pre_tax_income"))
        effect = composite([roi, i, de], lambda r, c, d: (r - c) * d)
        pre_tax = composite([roi, i, de], lambda r, c, d: r + (r - c) * d)
        roe = composite([pre_tax, t], lambda p, tax: p * (1 - tax))
        out.update(
            {
                "/roe_leverage/roi": roi,
                "/roe_leverage/cost_of_debt": i,
                "/roe_leverage/debt_to_equity": de,
                "/roe_leverage/leverage_effect": effect,
                "/roe_leverage/pre_tax_roe": pre_tax,
                "/roe_leverage/tax_rate": t,
                "/roe_leverage/roe": roe,
            }
        )
        return report(out)


def capital_structure(data: dict[str, Any]) -> dict[str, Any]:
    with localcontext(CONTEXT):
        equity, assets = balance(data, "equity"), balance(data, "total_assets")
        fixed = balance(data, "fixed_assets")
        ncl = balance(data, "non_current_liabilities")
        gross = balance(data, "net_financial_position", "gross_debt")
        position = nfp_balance(data)
        long_term = arith(lambda e, n: e + n, equity, ncl)
        return report(
            {
                "/net_financial_position": position,
                "/debt_to_equity": ratio(gross, equity, positive_den=True),
                "/nfp_to_equity": ratio(position, equity, positive_den=True),
                "/equity_ratio": ratio(equity, assets),
                "/primary_structure_margin": arith(lambda e, f: e - f, equity, fixed),
                "/secondary_structure_margin": arith(lambda lt, f: lt - f, long_term, fixed),
                "/primary_fixed_asset_coverage": ratio(equity, fixed),
                "/secondary_fixed_asset_coverage": ratio(long_term, fixed),
            }
        )


def liquidity(data: dict[str, Any]) -> dict[str, Any]:
    with localcontext(CONTEXT):
        ca, inv = balance(data, "current_assets"), balance(data, "inventories")
        cash, cl = balance(data, "cash"), balance(data, "current_liabilities")
        quick = arith(lambda a, b: a - b, ca, inv)
        return report(
            {
                "/current_ratio": ratio(ca, cl),
                "/quick_ratio": ratio(quick, cl),
                "/cash_ratio": ratio(cash, cl),
                "/net_working_capital": arith(lambda a, b: a - b, ca, cl),
                "/treasury_margin": arith(lambda q, b: q - b, quick, cl),
            }
        )


def working_capital(data: dict[str, Any]) -> dict[str, Any]:
    with localcontext(CONTEXT):
        days = int(data["day_basis"])

        def inc(k: str) -> Val:
            return amount(data, "income_statement", k)

        dso = ratio(balance(data, "trade_receivables"), inc("revenue"), days=days)
        dio = ratio(balance(data, "inventories"), inc("cost_of_sales"), days=days)
        dpo = ratio(balance(data, "trade_payables"), inc("purchases"), days=days)
        return report(
            {
                "/days_sales_outstanding": dso,
                "/days_inventory_outstanding": dio,
                "/days_payable_outstanding": dpo,
                "/cash_conversion_cycle": composite([dso, dio, dpo], lambda a, b, c: a + b - c),
            }
        )


def debt_service(data: dict[str, Any]) -> dict[str, Any]:
    with localcontext(CONTEXT):

        def inc(k: str) -> Val:
            return amount(data, "income_statement", k)

        interest = with_definition(inc("interest"), data, "interest_definition")
        ebitda = with_definition(inc("ebitda"), data, "ebitda_definition")
        position = nfp_balance(data)
        out = {
            "/net_financial_position": position,
            "/interest_cover_ebit": ratio(
                inc("ebit"), interest, positive_den=True, positive_num=True
            ),
            "/interest_cover_ebitda": ratio(ebitda, interest, positive_den=True, positive_num=True),
            "/nfp_to_ebitda": ratio(position, ebitda, positive_den=True),
        }
        schedule = data.get("debt_schedule")
        if schedule is None:
            out["/pv_cfads"] = lacking("/debt_schedule")
            out["/llcr"] = lacking("/debt_schedule")
            return report(out)
        periods = schedule["periods"]
        cfads = [
            given(D(str(p["cfads"])))
            if p.get("cfads") is not None
            else lacking(f"/debt_schedule/periods/{t}/cfads")
            for t, p in enumerate(periods)
        ]
        for t, p in enumerate(periods):
            service = (
                given(D(str(p["debt_service"])))
                if p.get("debt_service") is not None
                else lacking(f"/debt_schedule/periods/{t}/debt_service")
            )
            out[f"/dscr/{t}"] = ratio(cfads[t], service, positive_den=True)
        rate = amount(data, "debt_schedule", "discount_rate")

        def pv(r: D, *flows: D) -> D:
            return sum((f / (1 + r) ** (t + 1) for t, f in enumerate(flows)), D(0))

        present = arith(pv, rate, *cfads)
        reserve = D(str(schedule.get("reserve_balance") or 0))
        out["/pv_cfads"] = present
        out["/llcr"] = ratio(
            arith(lambda v: v + reserve, present),
            amount(data, "debt_schedule", "debt_outstanding"),
            positive_den=True,
        )
        return report(out)


def covenants(data: dict[str, Any]) -> dict[str, Any]:
    """Only what R11 adds: the outcome of each covenant and the outputs not computed."""
    with localcontext(CONTEXT):
        values: dict[str, Any] = {}
        nc: list[tuple[str, list[str]]] = []
        for i, c in enumerate(data["covenants"]):
            num = (
                given(D(str(c["numerator"]["amount"])))
                if c["numerator"].get("amount") is not None
                else lacking(f"/covenants/{i}/numerator/amount")
            )
            den = (
                given(D(str(c["denominator"]["amount"])))
                if c["denominator"].get("amount") is not None
                else lacking(f"/covenants/{i}/denominator/amount")
            )
            at = f"/covenants/{i}"
            if den.known and den.value <= 0:  # type: ignore[operator]
                rule = c["if_denominator_not_positive"]
                values[f"{at}/outcome"] = rule
                values[f"{at}/value"] = None
                continue
            if not (num.known and den.known):
                missing = sorted(set(num.missing + den.missing))
                values[f"{at}/outcome"] = "not_tested"
                values[f"{at}/not_tested_code"] = "missing_input"
                values[f"{at}/value"] = None
                for out in ("value", "headroom", "headroom_relative"):
                    nc.append((f"{at}/{out}", missing))
                continue
            value = num.value / den.value  # type: ignore[operator]
            threshold = D(str(c["threshold"]))
            maximum = c["operator"] in ("at_most", "less_than")
            headroom = threshold - value if maximum else value - threshold
            passes = {
                "at_most": value <= threshold,
                "less_than": value < threshold,
                "at_least": value >= threshold,
                "greater_than": value > threshold,
            }[c["operator"]]
            values[f"{at}/outcome"] = "passed" if passes else "breached"
            values[f"{at}/value"] = value
            values[f"{at}/headroom"] = headroom
        return {"values": values, "not_computed": nc}


# ----------------------------------------------------------------------------- cases

PROFIT = {
    "income_statement": {"revenue": "5000", "ebitda": "800", "ebit": "500", "net_income": "260"},
    "closing": {"equity": "2000", "total_assets": "6000", "invested_capital": "4000"},
}
DEFINITION = "reported EBITDA, no adjustments"
LEVERAGE = {
    "debt_basis": "net_financial_position",
    "financial_charges": "100",
    "taxes": "140",
    "pre_tax_income": "400",
}

CASES: dict[str, tuple[Any, dict[str, Any]]] = {
    "ratios.profitability/invested-capital-missing": (
        profitability,
        {
            "balances": "closing",
            "operating_return_base": "invested_capital",
            "ebitda_definition": DEFINITION,
            "income_statement": PROFIT["income_statement"],
            "closing": {"equity": "2000", "total_assets": "6000"},
        },
    ),
    "ratios.profitability/leverage-without-debt": (
        profitability,
        {
            "balances": "closing",
            "operating_return_base": "invested_capital",
            "ebitda_definition": DEFINITION,
            **PROFIT,
            "leverage": LEVERAGE,
        },
    ),
    "ratios.profitability/average-without-opening": (
        profitability,
        {
            "balances": "average",
            "operating_return_base": "total_assets",
            "ebitda_definition": DEFINITION,
            "income_statement": PROFIT["income_statement"],
            "closing": {"equity": "2000", "total_assets": "6000"},
        },
    ),
    "ratios.profitability/roe-only": (
        profitability,
        {
            "balances": "closing",
            "operating_return_base": "total_assets",
            "income_statement": {"net_income": "260"},
            "closing": {"equity": "2000"},
        },
    ),
    "ratios.profitability/ebitda-without-definition": (
        profitability,
        {
            "balances": "closing",
            "operating_return_base": "total_assets",
            "income_statement": PROFIT["income_statement"],
            "closing": {"equity": "2000", "total_assets": "6000"},
        },
    ),
    "ratios.capital_structure/partial-balance-sheet": (
        capital_structure,
        {
            "balances": "closing",
            "closing": {
                "equity": "2000",
                "fixed_assets": "3500",
                "net_financial_position": {"gross_debt": "2200"},
            },
        },
    ),
    "ratios.liquidity/without-inventories": (
        liquidity,
        {
            "balances": "closing",
            "closing": {"current_assets": "2500", "cash": "200", "current_liabilities": "2000"},
        },
    ),
    "ratios.liquidity/average-partial-opening": (
        liquidity,
        {
            "balances": "average",
            "closing": {
                "current_assets": "2500",
                "inventories": "900",
                "cash": "200",
                "current_liabilities": "2000",
            },
            "opening": {
                "current_assets": "2300",
                "inventories": "800",
                "current_liabilities": "1900",
            },
        },
    ),
    "ratios.working_capital/without-purchases": (
        working_capital,
        {
            "balances": "closing",
            "day_basis": "365",
            "income_statement": {"revenue": "5000", "cost_of_sales": "3000"},
            "closing": {"trade_receivables": "1200", "inventories": "600", "trade_payables": "700"},
        },
    ),
    "ratios.debt_service/partial-schedule": (
        debt_service,
        {
            "balances": "closing",
            "ebitda_definition": DEFINITION,
            "income_statement": {"ebitda": "800", "ebit": "500", "interest": "100"},
            "closing": {"net_financial_position": {"gross_debt": "2200", "cash": "300"}},
            "debt_schedule": {
                "discount_rate": "0.06",
                "debt_outstanding": "1000",
                "periods": [
                    {"cfads": "250", "debt_service": "230"},
                    {"cfads": "260"},
                    {"cfads": "270", "debt_service": "240"},
                ],
            },
        },
    ),
    "ratios.debt_service/schedule-without-rate": (
        debt_service,
        {
            "balances": "closing",
            "ebitda_definition": DEFINITION,
            "interest_definition": "gross interest expense",
            "income_statement": {"ebitda": "800", "interest": "100"},
            "closing": {"net_financial_position": {"gross_debt": "2200", "cash": "300"}},
            "debt_schedule": {
                "debt_outstanding": "1000",
                "periods": [
                    {"cfads": "250", "debt_service": "230"},
                    {"cfads": "260", "debt_service": "235"},
                ],
            },
        },
    ),
    "ratios.covenants/missing-amounts": (
        covenants,
        {
            "covenants": [
                {
                    "name": "net leverage",
                    "test_date": "2026-12-31",
                    "numerator": {"definition": "net financial debt at the test date"},
                    "denominator": {
                        "definition": "consolidated EBITDA, 12 months",
                        "amount": "1000",
                    },
                    "operator": "at_most",
                    "threshold": "3.5",
                    "if_denominator_not_positive": "breached",
                },
                {
                    "name": "net leverage, zero EBITDA",
                    "test_date": "2026-12-31",
                    "numerator": {"definition": "net financial debt at the test date"},
                    "denominator": {"definition": "consolidated EBITDA, 12 months", "amount": "0"},
                    "operator": "at_most",
                    "threshold": "3.5",
                    "if_denominator_not_positive": "breached",
                },
                {
                    "name": "interest cover",
                    "test_date": "2026-12-31",
                    "numerator": {"definition": "consolidated EBITDA, 12 months", "amount": "900"},
                    "denominator": {"definition": "net finance charges, 12 months"},
                    "operator": "at_least",
                    "threshold": "4",
                    "if_denominator_not_positive": "not_tested",
                },
                {
                    "name": "debt service cover",
                    "test_date": "2026-12-31",
                    "numerator": {
                        "definition": "cash flow available for debt service",
                        "amount": "1300",
                    },
                    "denominator": {"definition": "debt service", "amount": "1000"},
                    "operator": "at_least",
                    "threshold": "1.2",
                    "if_denominator_not_positive": "breached",
                },
            ]
        },
    ),
}


def text(v: Any) -> str:
    if v is None:
        return "null"
    return format(v, "f") if isinstance(v, D) else str(v)


def expect(name: str) -> dict[str, str]:
    """The `expect` block of a case: every value and every entry of `not_computed`."""
    fn, data = CASES[name]
    result = fn(data)
    out = {k: text(v) for k, v in result["values"].items()}
    for i, (output, missing) in enumerate(result["not_computed"]):
        out[f"/not_computed/{i}/output"] = output
        for j, pointer in enumerate(missing):
            out[f"/not_computed/{i}/missing/{j}"] = pointer
    return out


if __name__ == "__main__":
    for case in CASES:
        print(case)
        for key, value in expect(case).items():
            print(f"  {key}: '{value}'")
