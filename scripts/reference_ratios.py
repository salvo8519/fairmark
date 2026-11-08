"""Independent reference for SPEC-0007 (financial ratios), Python `decimal` at 50 digits.

Written from the definitions in `specs/0007-ratios-covenants/plan.md` ("Definitions
implemented"), sharing no code with the Rust engine. A ratio whose denominator is zero, or a
ratio over equity when equity is not positive, is `None` (not defined, SPEC-0007 R3).

Usage (prints the expected values of every golden case in `tests/golden/ratios.*`):

    uv run python scripts/reference_ratios.py

The arithmetic runs in a local 50-digit context: importing this module never changes the
caller's precision.
"""

from __future__ import annotations

from collections.abc import Callable
from decimal import Context, localcontext
from decimal import Decimal as D
from fractions import Fraction
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


Opt = D | None


def q(num: D, den: D, *, positive: bool = False) -> Opt:
    """num / den; None when den is zero, or not positive when `positive`."""
    if den == 0 or (positive and den < 0):
        return None
    return num / den


def q_both(num: D, den: D) -> Opt:
    """num / den; None when either is zero or negative (interest covers)."""
    if den <= 0 or num <= 0:
        return None
    return num / den


def avg(opening: D | None, closing: D, average: bool) -> D:
    if not average:
        return closing
    assert opening is not None, "average balances need the opening balance sheet"
    return (opening + closing) / 2


def nfp(block: dict[str, Any]) -> D:
    """Net financial position: gross debt - cash + other items (positive = net debt)."""
    return block["gross_debt"] - block["cash"] + sum((i for i in block["other_items"]), D(0))


@at_50_digits
def profitability(
    income: dict[str, D],
    closing: dict[str, D],
    opening: dict[str, D] | None,
    *,
    average: bool,
    base: str,
    leverage: dict[str, D] | None = None,
) -> dict[str, Opt]:
    """`leverage`: financial_charges, taxes, pre_tax_income; D is `debt` in the balance sheets."""

    def bal(k: str) -> D:
        return avg(opening[k] if opening else None, closing[k], average)

    equity, assets = bal("equity"), bal("total_assets")
    base_value = bal(base)
    revenue, ebitda, ebit, ni = (income[k] for k in ("revenue", "ebitda", "ebit", "net_income"))
    net_margin = q(ni, revenue)
    asset_turnover = q(revenue, assets)
    multiplier = q(assets, equity, positive=True)
    factors = (net_margin, asset_turnover, multiplier)
    roi = q(ebit, base_value, positive=base == "invested_capital")
    lev: dict[str, Opt]
    if leverage is None:
        lev = dict.fromkeys(
            ("roi", "cost_of_debt", "debt_to_equity", "leverage_effect", "pre_tax_roe"), None
        )
        lev.update(tax_rate=None, roe=None)
    else:
        debt = bal("debt")
        i = q(leverage["financial_charges"], debt)
        de = q(debt, equity, positive=True)
        t = q(leverage["taxes"], leverage["pre_tax_income"])
        effect = None if None in (roi, i, de) else (roi - i) * de  # type: ignore[operator]
        pre_tax = None if effect is None else roi + effect  # type: ignore[operator]
        roe_lev = None if None in (pre_tax, t) else pre_tax * (1 - t)  # type: ignore[operator]
        lev = {
            "roi": roi,
            "cost_of_debt": i,
            "debt_to_equity": de,
            "leverage_effect": effect,
            "pre_tax_roe": pre_tax,
            "tax_rate": t,
            "roe": roe_lev,
        }
    return {
        "roe": q(ni, equity, positive=True),
        "roi": roi,
        "roa": q(ebit, assets),
        "ros": q(ebit, revenue),
        "ebitda_margin": q(ebitda, revenue),
        "ebit_margin": q(ebit, revenue),
        "capital_turnover": q(revenue, base_value),
        "roe_three_factor.net_margin": net_margin,
        "roe_three_factor.asset_turnover": asset_turnover,
        "roe_three_factor.equity_multiplier": multiplier,
        "roe_three_factor.roe": (
            None if None in factors else net_margin * asset_turnover * multiplier  # type: ignore[operator]
        ),
        **{f"roe_leverage.{k}": v for k, v in lev.items()},
    }


@at_50_digits
def capital_structure(
    closing: dict[str, Any], opening: dict[str, Any] | None, *, average: bool
) -> dict[str, Opt]:
    def bal(k: str) -> D:
        return avg(opening[k] if opening else None, closing[k], average)

    def debt(f: Callable[[dict[str, Any]], D]) -> D:
        o = f(opening["net_financial_position"]) if opening else None
        return avg(o, f(closing["net_financial_position"]), average)

    equity, assets = bal("equity"), bal("total_assets")
    fixed, ncl = bal("fixed_assets"), bal("non_current_liabilities")
    gross = debt(lambda b: b["gross_debt"])
    position = debt(nfp)
    return {
        "net_financial_position": position,
        "debt_to_equity": q(gross, equity, positive=True),
        "nfp_to_equity": q(position, equity, positive=True),
        "equity_ratio": q(equity, assets),
        "primary_structure_margin": equity - fixed,
        "secondary_structure_margin": equity + ncl - fixed,
        "primary_fixed_asset_coverage": q(equity, fixed),
        "secondary_fixed_asset_coverage": q(equity + ncl, fixed),
    }


@at_50_digits
def liquidity(
    closing: dict[str, D], opening: dict[str, D] | None, *, average: bool
) -> dict[str, Opt]:
    def bal(k: str) -> D:
        return avg(opening[k] if opening else None, closing[k], average)

    ca, inv, cash, cl = (
        bal(k) for k in ("current_assets", "inventories", "cash", "current_liabilities")
    )
    return {
        "current_ratio": q(ca, cl),
        "quick_ratio": q(ca - inv, cl),
        "cash_ratio": q(cash, cl),
        "net_working_capital": ca - cl,
        "treasury_margin": ca - inv - cl,
    }


@at_50_digits
def working_capital(
    income: dict[str, D],
    closing: dict[str, D],
    opening: dict[str, D] | None,
    *,
    average: bool,
    days: int,
) -> dict[str, Opt]:
    def bal(k: str) -> D:
        return avg(opening[k] if opening else None, closing[k], average)

    def day_ratio(num: D, den: D) -> Opt:
        return None if den == 0 else num * days / den

    dso = day_ratio(bal("trade_receivables"), income["revenue"])
    dio = day_ratio(bal("inventories"), income["cost_of_sales"])
    dpo = day_ratio(bal("trade_payables"), income["purchases"])
    ccc = None if None in (dso, dio, dpo) else dso + dio - dpo  # type: ignore[operator]
    return {
        "days_sales_outstanding": dso,
        "days_inventory_outstanding": dio,
        "days_payable_outstanding": dpo,
        "cash_conversion_cycle": ccc,
    }


@at_50_digits
def debt_service(
    income: dict[str, D],
    closing: dict[str, Any],
    opening: dict[str, Any] | None,
    schedule: dict[str, Any] | None,
    *,
    average: bool,
) -> dict[str, Opt]:
    """Without a schedule, DSCR and LLCR are not defined; `reserve_balance` is optional."""
    position = avg(
        nfp(opening["net_financial_position"]) if opening else None,
        nfp(closing["net_financial_position"]),
        average,
    )
    out: dict[str, Opt] = {
        "net_financial_position": position,
        "interest_cover_ebit": q_both(income["ebit"], income["interest"]),
        "interest_cover_ebitda": q_both(income["ebitda"], income["interest"]),
        "nfp_to_ebitda": q(position, income["ebitda"], positive=True),
    }
    if schedule is None:
        out.update(pv_cfads=None, llcr=None)
        return out
    rate: D = schedule["discount_rate"]
    pv = sum(
        (p["cfads"] / (1 + rate) ** t for t, p in enumerate(schedule["periods"], start=1)),
        D(0),
    )
    reserve = schedule.get("reserve_balance", D(0))
    out["pv_cfads"] = pv
    out["llcr"] = q(pv + reserve, schedule["debt_outstanding"], positive=True)
    for i, p in enumerate(schedule["periods"]):
        out[f"dscr.{i}"] = q(p["cfads"], p["debt_service"], positive=True)
    return out


def round_fraction(x: Fraction, places: int, mode: str) -> Fraction:
    """x rounded to `places` decimals, exactly: half_up = halves away from zero."""
    scaled = abs(x) * 10**places
    q, rem = divmod(scaled.numerator, scaled.denominator)
    twice = 2 * rem
    if twice > scaled.denominator or (
        twice == scaled.denominator and (mode == "half_up" or q % 2 == 1)
    ):
        q += 1
    return Fraction(q if x >= 0 else -q, 10**places)


@at_50_digits
def covenants(items: list[dict[str, Any]]) -> dict[str, Any]:
    """Covenant tests on contractual definitions (SPEC-0007 R5, decisions of 2026-11-08).

    Each item: numerator, denominator (amounts), operator (at_most, less_than, at_least,
    greater_than), threshold, if_denominator_not_positive (breached, passed, not_tested) and
    optional rounding (decimal_places, mode half_up or half_even). Exact arithmetic on
    fractions: the outcome compares the exact tested value, or the value rounded exactly from
    the exact quotient, with the threshold. Value, headroom and relative headroom are printed
    at 50 digits. A zero or negative denominator gives the outcome of the rule, value and
    headroom None, code denominator_not_positive when not tested. Headroom is positive on the
    compliant side; relative headroom = headroom / threshold, None for a threshold <= 0.
    """
    out: dict[str, Any] = {}
    counts = {"passed": 0, "breached": 0, "not_tested": 0}
    for i, c in enumerate(items):
        num, den, op, limit = c["numerator"], c["denominator"], c["operator"], c["threshold"]
        unrounded: Opt = None
        if den <= 0:
            outcome = c["if_denominator_not_positive"]
            value, headroom, relative = None, None, None
            code = "denominator_not_positive" if outcome == "not_tested" else None
        else:
            exact = Fraction(num) / Fraction(den)
            rounding = c.get("rounding")
            if rounding is not None:
                exact = round_fraction(exact, rounding["decimal_places"], rounding["mode"])
                unrounded = num / den
            lim = Fraction(limit)
            passed = {
                "at_most": exact <= lim,
                "less_than": exact < lim,
                "at_least": exact >= lim,
                "greater_than": exact > lim,
            }[op]
            outcome = "passed" if passed else "breached"
            is_max = op in ("at_most", "less_than")
            gap = lim - exact if is_max else exact - lim
            value = D(exact.numerator) / D(exact.denominator)
            headroom = D(gap.numerator) / D(gap.denominator)
            relative = headroom / limit if limit > 0 else None
            code = None
        counts[outcome] += 1
        out[f"covenants.{i}.outcome"] = outcome
        out[f"covenants.{i}.not_tested_code"] = code
        out[f"covenants.{i}.value"] = value
        out[f"covenants.{i}.value_unrounded"] = unrounded
        out[f"covenants.{i}.headroom"] = headroom
        out[f"covenants.{i}.headroom_relative.value"] = relative
    for k, v in counts.items():
        out[f"summary.{k}"] = v
    return out


@at_50_digits
def level_llcr(cfads: D, rate: D, periods: int, debt: D) -> D:
    """LLCR of a level CFADS in closed form: C * (1 - (1 + r)^-n) / r / debt."""
    return cfads * (1 - (1 + rate) ** -periods) / rate / debt


# ------------------------------------------------------------------ golden cases

PROFIT_INCOME = {"revenue": D(5000), "ebitda": D(800), "ebit": D(500), "net_income": D(260)}
PROFIT_CLOSING = {"equity": D(2000), "total_assets": D(6000), "invested_capital": D(4000)}
PROFIT_OPENING = {"equity": D(1800), "total_assets": D(5600), "invested_capital": D(3800)}

LEVERAGE = {"financial_charges": D(100), "taxes": D(140), "pre_tax_income": D(400)}

NFP_CLOSING = {"gross_debt": D(2200), "cash": D(300), "other_items": [D(150)]}
NFP_OPENING = {"gross_debt": D(2400), "cash": D(250), "other_items": []}
CS_CLOSING = {
    "equity": D(2000),
    "total_assets": D(6000),
    "fixed_assets": D(3500),
    "non_current_liabilities": D(1800),
    "net_financial_position": NFP_CLOSING,
}
CS_OPENING = {
    "equity": D(1800),
    "total_assets": D(5600),
    "fixed_assets": D(3300),
    "non_current_liabilities": D(1700),
    "net_financial_position": NFP_OPENING,
}

LIQ_CLOSING = {
    "current_assets": D(2500),
    "inventories": D(900),
    "cash": D(200),
    "current_liabilities": D(2000),
}

LIQ_OPENING = {
    "current_assets": D(2300),
    "inventories": D(850),
    "cash": D(150),
    "current_liabilities": D(1900),
}

WC_INCOME = {"revenue": D(5000), "cost_of_sales": D(3000), "purchases": D(2800)}
WC_CLOSING = {"trade_receivables": D(1200), "inventories": D(600), "trade_payables": D(700)}
WC_OPENING = {"trade_receivables": D(1000), "inventories": D(500), "trade_payables": D(650)}

DS_INCOME = {"ebitda": D(800), "ebit": D(500), "interest": D(100)}
DS_CLOSING = {"net_financial_position": {"gross_debt": D(2200), "cash": D(300), "other_items": []}}
DS_OPENING = {"net_financial_position": {"gross_debt": D(2000), "cash": D(200), "other_items": []}}
SCHEDULE = {
    "discount_rate": D("0.06"),
    "debt_outstanding": D(1000),
    "periods": [
        {"cfads": D(c), "debt_service": D(s)}
        for c, s in ((320, 250), (330, 250), (300, 240), (310, 240), (290, 230))
    ],
}
LEVEL_SCHEDULE = {
    "discount_rate": D("0.06"),
    "debt_outstanding": D(1000),
    "periods": [{"cfads": D(250), "debt_service": D(200)} for _ in range(5)],
}
ZERO_SERVICE_SCHEDULE = {
    "discount_rate": D("0.05"),
    "debt_outstanding": D(500),
    "periods": [
        {"cfads": D(120), "debt_service": D(0)},
        {"cfads": D(150), "debt_service": D(100)},
    ],
}


def cov(
    name: str,
    num: int | str,
    den: int | str,
    op: str,
    limit: str,
    rule: str = "not_tested",
    rounding: dict[str, Any] | None = None,
) -> dict[str, Any]:
    return {
        "name": name,
        "numerator": D(num),
        "denominator": D(den),
        "operator": op,
        "threshold": D(limit),
        "if_denominator_not_positive": rule,
        **({"rounding": rounding} if rounding else {}),
    }


# AC5: one breach (interest cover) among three covenants on the same test date.
COVENANTS_ONE_BREACH = [
    cov("net leverage", 1900, 600, "at_most", "3.5"),
    cov("interest cover", 600, 175, "at_least", "4"),
    cov("debt service cover", 700, 530, "at_least", "1.2"),
]
COVENANTS_ZERO_DENOMINATOR = [
    cov("net leverage", 1900, 0, "at_most", "3.5"),
    cov("interest cover", 600, 175, "at_least", "3"),
]
COVENANTS_NEGATIVE_DENOMINATOR = [
    cov("net leverage, breached by agreement", 1900, -600, "at_most", "3.5", "breached"),
    cov("net leverage, passed by agreement", 1900, -600, "at_most", "3.5", "passed"),
    cov("net leverage, not tested by agreement", 1900, -600, "at_most", "3.5", "not_tested"),
    cov("interest cover, zero charges breached", 600, 0, "at_least", "4", "breached"),
]
# Maintainer decision 4 (2026-11-08): 3.504 at most 3.50, with and without rounding.
COVENANTS_ROUNDING = [
    cov("net leverage, no rounding", 3504, 1000, "at_most", "3.50"),
    cov(
        "net leverage, rounded to 2 decimals half up",
        3504,
        1000,
        "at_most",
        "3.50",
        rounding={"decimal_places": 2, "mode": "half_up"},
    ),
]
COVENANTS_ROUNDING_MODES = [
    cov(
        "net leverage, 3.505 half up",
        3505,
        1000,
        "at_most",
        "3.50",
        rounding={"decimal_places": 2, "mode": "half_up"},
    ),
    cov(
        "net leverage, 3.505 half even",
        3505,
        1000,
        "at_most",
        "3.50",
        rounding={"decimal_places": 2, "mode": "half_even"},
    ),
    cov(
        "interest cover, 2 / 3 to 0 decimals half up",
        2,
        3,
        "at_least",
        "1",
        rounding={"decimal_places": 0, "mode": "half_up"},
    ),
]
COVENANTS_AT_THRESHOLD = [
    cov("leverage, inclusive maximum", 1750, 500, "at_most", "3.5"),
    cov("leverage, strict maximum", 1750, 500, "less_than", "3.5"),
    cov("cover, inclusive minimum", 1200, 300, "at_least", "4"),
    cov("cover, strict minimum", 1200, 300, "greater_than", "4"),
]
COVENANTS_NEGATIVE_NUMERATOR = [
    cov("net leverage, net cash", -300, 700, "at_most", "3"),
    cov("interest cover, negative EBITDA", -150, 90, "at_least", "2.5"),
]
COVENANTS_NON_POSITIVE_THRESHOLD = [
    cov("net financial position to EBITDA, net cash required", -200, 800, "at_most", "0"),
    cov("net financial position to EBITDA, minimum net cash", -400, 800, "at_most", "-0.25"),
]


def cases() -> dict[str, dict[str, Opt]]:
    neg = {**PROFIT_CLOSING, "equity": D(-100)}
    cs_neg = {**CS_CLOSING, "equity": D(-200)}
    return {
        "ratios.profitability/closing-balances": profitability(
            PROFIT_INCOME, PROFIT_CLOSING, None, average=False, base="invested_capital"
        ),
        "ratios.profitability/average-balances": profitability(
            PROFIT_INCOME, PROFIT_CLOSING, PROFIT_OPENING, average=True, base="total_assets"
        ),
        "ratios.profitability/negative-equity": profitability(
            PROFIT_INCOME, neg, None, average=False, base="total_assets"
        ),
        "ratios.profitability/zero-revenue": profitability(
            {**PROFIT_INCOME, "revenue": D(0)},
            PROFIT_CLOSING,
            None,
            average=False,
            base="invested_capital",
        ),
        "ratios.profitability/average-invested-capital": profitability(
            PROFIT_INCOME, PROFIT_CLOSING, PROFIT_OPENING, average=True, base="invested_capital"
        ),
        "ratios.profitability/negative-invested-capital": profitability(
            PROFIT_INCOME,
            {**PROFIT_CLOSING, "invested_capital": D(-500)},
            None,
            average=False,
            base="invested_capital",
        ),
        "ratios.profitability/leverage-net-financial-position": profitability(
            PROFIT_INCOME,
            {**PROFIT_CLOSING, "debt": D(2000)},
            None,
            average=False,
            base="invested_capital",
            leverage=LEVERAGE,
        ),
        "ratios.profitability/leverage-gross-debt-average": profitability(
            PROFIT_INCOME,
            {**PROFIT_CLOSING, "debt": D(2200)},
            {**PROFIT_OPENING, "debt": D(2000)},
            average=True,
            base="invested_capital",
            leverage=LEVERAGE,
        ),
        "ratios.profitability/leverage-zero-pre-tax-income": profitability(
            PROFIT_INCOME,
            {**PROFIT_CLOSING, "debt": D(2000)},
            None,
            average=False,
            base="invested_capital",
            leverage={**LEVERAGE, "pre_tax_income": D(0)},
        ),
        "ratios.capital_structure/closing-balances": capital_structure(
            CS_CLOSING, None, average=False
        ),
        "ratios.capital_structure/average-balances": capital_structure(
            CS_CLOSING, CS_OPENING, average=True
        ),
        "ratios.capital_structure/negative-equity": capital_structure(cs_neg, None, average=False),
        "ratios.liquidity/closing-balances": liquidity(LIQ_CLOSING, None, average=False),
        "ratios.liquidity/zero-current-liabilities": liquidity(
            {**LIQ_CLOSING, "current_liabilities": D(0)}, None, average=False
        ),
        "ratios.liquidity/average-balances": liquidity(LIQ_CLOSING, LIQ_OPENING, average=True),
        "ratios.working_capital/closing-360": working_capital(
            WC_INCOME, WC_CLOSING, None, average=False, days=360
        ),
        "ratios.working_capital/closing-365": working_capital(
            WC_INCOME, WC_CLOSING, None, average=False, days=365
        ),
        "ratios.working_capital/average-360": working_capital(
            WC_INCOME, WC_CLOSING, WC_OPENING, average=True, days=360
        ),
        "ratios.working_capital/zero-purchases": working_capital(
            {**WC_INCOME, "purchases": D(0)}, WC_CLOSING, None, average=False, days=365
        ),
        "ratios.debt_service/loan-schedule": debt_service(
            DS_INCOME, DS_CLOSING, None, SCHEDULE, average=False
        ),
        "ratios.debt_service/level-cfads": debt_service(
            DS_INCOME, DS_CLOSING, None, LEVEL_SCHEDULE, average=False
        ),
        "ratios.debt_service/zero-denominators": debt_service(
            {**DS_INCOME, "interest": D(0)},
            DS_CLOSING,
            DS_OPENING,
            ZERO_SERVICE_SCHEDULE,
            average=True,
        ),
        "ratios.debt_service/zero-debt-outstanding": debt_service(
            DS_INCOME,
            DS_CLOSING,
            None,
            {**LEVEL_SCHEDULE, "debt_outstanding": D(0)},
            average=False,
        ),
        "ratios.debt_service/non-positive-ebitda": debt_service(
            {"ebitda": D(-100), "ebit": D(-150), "interest": D(100)},
            DS_CLOSING,
            None,
            LEVEL_SCHEDULE,
            average=False,
        ),
        "ratios.debt_service/negative-interest": debt_service(
            {**DS_INCOME, "interest": D(-20)}, DS_CLOSING, None, LEVEL_SCHEDULE, average=False
        ),
        "ratios.debt_service/negative-debt-service": debt_service(
            DS_INCOME,
            DS_CLOSING,
            None,
            {
                **ZERO_SERVICE_SCHEDULE,
                "debt_outstanding": D(-100),
                "periods": [
                    {"cfads": D(120), "debt_service": D(-10)},
                    {"cfads": D(150), "debt_service": D(100)},
                ],
            },
            average=False,
        ),
        "ratios.debt_service/no-debt-schedule": debt_service(
            DS_INCOME, DS_CLOSING, None, None, average=False
        ),
        "ratios.debt_service/reserve-balance": debt_service(
            DS_INCOME,
            DS_CLOSING,
            None,
            {**LEVEL_SCHEDULE, "reserve_balance": D(100)},
            average=False,
        ),
        "ratios.covenants/one-breach": covenants(COVENANTS_ONE_BREACH),
        "ratios.covenants/zero-denominator": covenants(COVENANTS_ZERO_DENOMINATOR),
        "ratios.covenants/negative-denominator": covenants(COVENANTS_NEGATIVE_DENOMINATOR),
        "ratios.covenants/rounding": covenants(COVENANTS_ROUNDING),
        "ratios.covenants/rounding-modes": covenants(COVENANTS_ROUNDING_MODES),
        "ratios.covenants/at-the-threshold": covenants(COVENANTS_AT_THRESHOLD),
        "ratios.covenants/negative-numerator": covenants(COVENANTS_NEGATIVE_NUMERATOR),
        "ratios.covenants/non-positive-threshold": covenants(COVENANTS_NON_POSITIVE_THRESHOLD),
    }


def text(v: Any) -> str:
    if v is None:
        return "null"
    return v if isinstance(v, str | int) and not isinstance(v, D) else format(v, "f")


if __name__ == "__main__":
    for name, values in cases().items():
        print(name)
        for key, value in values.items():
            print(f"  /{key.replace('.', '/')}: {text(value)}")
    print(
        "level CFADS 250, 5 periods at 6%, debt 1000, LLCR in closed form:",
        text(level_llcr(D(250), D("0.06"), 5, D(1000))),
    )
