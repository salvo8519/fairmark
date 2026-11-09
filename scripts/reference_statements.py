"""Independent reference for SPEC-0005 (statements.reorganise), first and second pull requests:
validation, best effort, input checks, the reorganised balance sheet in both forms, the net
financial position, the bridge items, the current and non-current block, and the reorganised
income statement in both forms with its reconciliations and the `lease` double entry.

Written from Appendices A, B and C of `specs/0005-statements-reclassification/spec.md`, its
requirements R2 to R22 and its decisions (Decision 17 on write-downs, D1 to D6, M1, M2), and the
design choices recorded in its `plan.md`, with Python's `decimal` module at 50 significant
digits in a local context (importing this module never changes the caller's precision). It
shares no code with the Rust engine and is used to produce or confirm golden expectations:

    uv run python scripts/reference_statements.py [case.json]

Independence from the engine, by construction:
- best effort is modelled with Python sets of missing pointers attached to each amount, and
  every output is built by reading the statutory items directly, not through a shared pipeline;
- the net financial position is summed from its own list of signed components (the engine adds
  the SPEC-0007 object);
- current and non-current amounts are built from the statutory maturity splits item by item;
- the income statement keeps one counter per aggregate for what remains after the adjustments
  and builds each layer of Appendix B.2 from those counters; its reconciliations with A - B are
  built from the statutory items, and checked against the layers.

The reference returns the outputs as a nested dictionary with the same JSON pointers as the
engine, so a golden case can be written from `pointers(result)`.
"""

from __future__ import annotations

import json
import sys
from collections.abc import Callable
from dataclasses import dataclass
from decimal import Context, localcontext
from decimal import Decimal as D
from functools import wraps
from typing import Any, TypeVar

T = TypeVar("T", bound=Callable[..., Any])
CONTEXT = Context(prec=50)
ZERO = D(0)


def at_50_digits(func: T) -> T:
    @wraps(func)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        with localcontext(CONTEXT):
            return func(*args, **kwargs)

    return wrapper  # type: ignore[return-value]


class RefError(Exception):
    """A refusal, with its stable code (FM-E001, FM-E009, FM-E026)."""

    def __init__(self, code: str, detail: str) -> None:
        super().__init__(f"{code}: {detail}")
        self.code = code
        self.detail = detail


# ---------------------------------------------------------------- amounts with missing inputs
@dataclass(frozen=True)
class Amt:
    """A known amount, or the set of input pointers whose absence prevents it (R6)."""

    value: D | None
    missing: frozenset[str] = frozenset()

    @property
    def known(self) -> bool:
        return self.value is not None


def known(v: D) -> Amt:
    return Amt(v)


def lacking(*pointers: str) -> Amt:
    return Amt(None, frozenset(pointers))


def total(*terms: tuple[int, Amt]) -> Amt:
    """Signed sum of (sign, amount); missing if any term is missing (union of pointers)."""
    missing: set[str] = set()
    absent = False
    s = ZERO
    for sign, a in terms:
        if a.value is None:
            absent = True
            missing |= a.missing
        else:
            s += sign * a.value
    if absent:
        return Amt(None, frozenset(missing))
    return Amt(s)


def plus(*amounts: Amt) -> Amt:
    return total(*((1, a) for a in amounts))


# ---------------------------------------------------------------- signs (R3)
STATUTORY_SIGNED_BS = {
    ("equity", "other_reserves"),
    ("equity", "cash_flow_hedge_reserve"),
    ("equity", "retained_earnings"),
    ("equity", "profit_or_loss_for_the_year"),
    ("equity", "total"),
}
STATUTORY_NON_POSITIVE_BS = {("equity", "negative_reserve_for_own_shares")}
STATUTORY_SIGNED_IS = {
    "change_in_product_inventories",
    "change_in_raw_material_inventories",
    "exchange_gains_and_losses",
    "income_taxes",
    "profit_or_loss_for_the_year",
    "value_of_production",
    "costs_of_production",
    "difference_between_value_and_costs_of_production",
    "financial_income_and_charges",
    "value_adjustments_of_financial_assets",
    "profit_before_tax",
}
RECLASSIFIED_SIGNED_BS = {
    "invested_capital",
    "total_funds_invested",
    "equity",
    "total_sources",
    ("operating_working_capital", "total"),
    ("net_financial_position", "total"),
    ("net_financial_position", "current"),
    ("net_financial_position", "non_current"),
}
RECLASSIFIED_SIGNED_IS = {
    "external_costs",
    "value_of_production",
    "materials_consumed",
    "value_added",
    "ebitda",
    "operating_profit",
    "investing_result",
    "profit_before_financing_and_taxes",
    "non_recurring_result",
    "financing_result",
    "profit_before_tax",
    "income_taxes",
    "net_income",
}


def dec(x: Any) -> D:
    """An exact decimal from a string or an integer; a float is refused, because its binary
    value is not the decimal the user wrote (constitution I)."""
    if isinstance(x, float):
        raise TypeError(f"{x!r} is a float: give amounts as strings, for example '0.1'")
    return D(str(x))


def walk_signs(node: Any, pointer: str, path: tuple[str, ...], signed: Callable[..., str]) -> None:
    """Every decimal below `node`: `signed(path)` says `any`, `non_positive` or `non_negative`."""
    if isinstance(node, dict):
        for k, v in node.items():
            walk_signs(v, f"{pointer}/{k}", (*path, k), signed)
        return
    if isinstance(node, list):
        for i, v in enumerate(node):
            walk_signs(v, f"{pointer}/{i}", (*path, str(i)), signed)
        return
    if isinstance(node, float):
        raise TypeError(f"{pointer} is a float: give amounts as strings")
    if isinstance(node, (int, str)) and not isinstance(node, bool):
        try:
            v = dec(node)
        except Exception:
            return
        rule = signed(path)
        if rule == "non_negative" and v < 0:
            raise RefError("FM-E001", f"{pointer} negative")
        if rule == "non_positive" and v > 0:
            raise RefError("FM-E001", f"{pointer} positive")


def statutory_sign(path: tuple[str, ...]) -> str:
    if path[0] == "layout":
        return "any"
    if path[0] == "balance_sheet":
        key = tuple(path[1:3])
        if key in STATUTORY_NON_POSITIVE_BS:
            return "non_positive"
        if key in STATUTORY_SIGNED_BS:
            return "any"
        return "non_negative"
    if path[0] == "income_statement":
        return "any" if path[1] in STATUTORY_SIGNED_IS else "non_negative"
    return "non_negative"


def reclassified_sign(path: tuple[str, ...]) -> str:
    if path[0] == "balance_sheet":
        rest = path[1:]
        if rest[0] in RECLASSIFIED_SIGNED_BS or tuple(rest[:2]) in RECLASSIFIED_SIGNED_BS:
            return "any"
        if rest[0] == "net_financial_position" and rest[1] == "position":
            # SPEC-0007 other items are signed; gross debt and cash are not.
            return "any" if rest[2] == "other_items" else "non_negative"
        return "non_negative"
    if path[0] == "income_statement":
        return "any" if path[1] in RECLASSIFIED_SIGNED_IS else "non_negative"
    return "non_negative"


# ---------------------------------------------------------------- dates (R2)
def parse_date(s: str, pointer: str) -> tuple[int, int, int]:
    try:
        y, m, d = (int(p) for p in s.split("-"))
        assert len(s) == 10
    except Exception as e:
        raise RefError("FM-E009", pointer) from e
    if not (1900 <= y <= 2199) or not 1 <= m <= 12 or not 1 <= d <= month_days(y, m):
        raise RefError("FM-E009", pointer)
    return y, m, d


def month_days(y: int, m: int) -> int:
    if m == 2:
        return 29 if (y % 4 == 0 and y % 100 != 0) or y % 400 == 0 else 28
    return 30 if m in (4, 6, 9, 11) else 31


def consecutive(a: tuple[int, int, int], b: tuple[int, int, int]) -> bool:
    """`b` is one financial year after `a`: same month one calendar year later, and the same day,
    or 28 February after 29 February, or both the last day of their month (SPEC-0010)."""
    if b[0] != a[0] + 1 or b[1] != a[1]:
        return False
    if b[2] == a[2]:
        return True
    if a[1] == 2 and a[2] == 29 and b[2] == 28:
        return True
    return a[2] == month_days(*a[:2]) and b[2] == month_days(*b[:2])


# ---------------------------------------------------------------- input access
def get(d: dict[str, Any] | None, *keys: str) -> Any:
    for k in keys:
        if not isinstance(d, dict) or k not in d:
            return None
        d = d[k]
    return d


def leaf(bs: dict[str, Any] | None, base: str, key: str) -> Amt:
    v = get(bs, key)
    return known(dec(v)) if v is not None else lacking(f"{base}/{key}")


def node_total(bs: dict[str, Any] | None, base: str, key: str) -> Amt:
    """A node with a `total`: the node pointer when it is absent, its total when not given."""
    node = get(bs, key)
    if node is None:
        return lacking(f"{base}/{key}")
    if "total" in node:
        return known(dec(node["total"]))
    return lacking(f"{base}/{key}/total")


def part(bs: dict[str, Any] | None, base: str, key: str, sub: str) -> Amt:
    node = get(bs, key)
    if node is None:
        return lacking(f"{base}/{key}")
    if sub in node:
        return known(dec(node[sub]))
    return lacking(f"{base}/{key}/{sub}")


def net_node(bs: dict[str, Any] | None, base: str, key: str) -> Amt:
    """B.I, B.II: the net amount, or gross minus accumulated."""
    node = get(bs, key)
    if node is None:
        return lacking(f"{base}/{key}")
    if "total" in node:
        return known(dec(node["total"]))
    if "gross" in node and "accumulated" in node:
        return known(dec(node["gross"]) - dec(node["accumulated"]))
    return lacking(f"{base}/{key}/total")


EQUITY_PARTS = [
    "share_capital",
    "share_premium",
    "revaluation_reserves",
    "legal_reserve",
    "statutory_reserves",
    "other_reserves",
    "cash_flow_hedge_reserve",
    "retained_earnings",
    "profit_or_loss_for_the_year",
    "negative_reserve_for_own_shares",
]


def equity_amount(bs: dict[str, Any] | None, base: str) -> Amt:
    node = get(bs, "equity")
    if node is None:
        return lacking(f"{base}/equity")
    if "total" in node:
        return known(dec(node["total"]))
    if all(p in node for p in EQUITY_PARTS):
        return known(sum((dec(node[p]) for p in EQUITY_PARTS), ZERO))
    return lacking(f"{base}/equity/total")


def of_which(bs: dict[str, Any] | None, key: str, sub: str) -> D:
    """A destination "of which" part: none within its aggregate when not given."""
    v = get(bs, key, sub)
    return dec(v) if v is not None else ZERO


# ---------------------------------------------------------------- treatments
FFA_PERMITTED = ["operating", "non_operating", "financial_asset"]
SL_PERMITTED = ["financial_debt", "quasi_equity"]
DERIV_PERMITTED = ["financial_liability", "operating_provision"]


@dataclass
class Share:
    """One destination share of an aggregate: label, amount and, for a loan, the part due
    beyond one year (None: the maturity of the whole aggregate)."""

    dest: str
    label: str
    amount: Amt
    beyond: Amt | None = None


def shares_of(
    value: Amt,
    treatment: Any,
    treatment_pointer: str,
    permitted: list[str],
    label: str,
    on_split: Callable[[D, D], None],
    split_pointer: str,
) -> tuple[list[Share], dict[str, list[str]]]:
    """The destination shares of an aggregate with a required treatment (R8, R9), and the
    permitted destinations of a treatment that is needed and missing."""
    if treatment is None:
        if value.known and value.value == 0:
            return [], {}
        if not value.known:
            return [Share(d, label, value) for d in permitted], {}
        miss = lacking(treatment_pointer)
        return [Share(d, label, miss) for d in permitted], {treatment_pointer: permitted}
    if isinstance(treatment, str):
        return [Share(treatment, label, value)], {}
    if "destination" in treatment:
        return [Share(treatment["destination"], label, value)], {}
    parts = treatment["split"]
    if value.known:
        on_split(value.value, sum((dec(p["amount"]) for p in parts), ZERO))
    shares = []
    for j, p in enumerate(parts):
        beyond = (
            known(dec(p["beyond_one_year"]))
            if "beyond_one_year" in p
            else lacking(f"{split_pointer}/{j}/beyond_one_year")
        )
        amount = known(dec(p["amount"])) if value.known else value
        shares.append(Share(p["destination"], p["label"], amount, beyond))
    return shares, {}


def dest_sum(shares: list[Share], dest: str) -> Amt:
    return plus(*(s.amount for s in shares if s.dest == dest))


# ---------------------------------------------------------------- validation and defaults
# Appendix A.2: the aggregates an adjustment may move (Appendix C), with their references.
# Income aggregates add to profit; the others are charges.
IS_REFERENCES = {
    "revenue": "A1",
    "other_revenue_and_income": "A5",
    "purchases": "B6",
    "services": "B7",
    "use_of_third_party_assets": "B8",
    "personnel_costs": "B9",
    "depreciation_and_amortisation": "B10 a, b",
    "write_downs_of_fixed_assets": "B10 c",
    "write_downs_of_receivables": "B10 d",
    "provisions_for_risks": "B12",
    "other_provisions": "B13",
    "other_operating_expenses": "B14",
    "income_from_equity_investments": "C15",
    "other_financial_income": "C16",
    "interest_and_other_financial_charges": "C17",
    "exchange_gains_and_losses": "C17-bis",
}
NON_RECURRING_SOURCES = set(IS_REFERENCES)
INVESTING_SOURCES = {"other_revenue_and_income", "other_operating_expenses"}
IS_INCOME = {
    "revenue",
    "other_revenue_and_income",
    "income_from_equity_investments",
    "other_financial_income",
    "exchange_gains_and_losses",
}
# The aggregates of A and B (they enter A - B), for the reconciliations of R19.
IS_A_AND_B = {
    k
    for k in IS_REFERENCES
    if k
    not in (
        "income_from_equity_investments",
        "other_financial_income",
        "interest_and_other_financial_charges",
        "exchange_gains_and_losses",
    )
}
B10_KEYS = {
    "depreciation_and_amortisation",
    "write_downs_of_fixed_assets",
    "write_downs_of_receivables",
}

DEFAULT_ROWS_BS = [
    ("intangible_assets", "operating_fixed_assets"),
    ("tangible_assets", "operating_fixed_assets"),
    ("receivables", "operating_working_capital"),
    ("current_financial_assets", "net_financial_position_other_financial_asset"),
    ("cash", "net_financial_position_cash"),
    ("prepayments_and_accrued_income", "operating_working_capital"),
    ("provisions", "operating_provisions"),
    ("employee_severance_indemnity", "operating_provisions"),
    ("payables", "operating_working_capital"),
    ("accruals_and_deferred_income", "operating_working_capital"),
]


def aggregate_given(bs: dict[str, Any], key: str) -> bool:
    """The aggregate itself is given: a leaf, a node's total, or B.I and B.II gross and
    accumulated; a node holding only a part of the aggregate does not count."""
    v = bs.get(key)
    if v is None:
        return False
    if not isinstance(v, dict):
        return True
    return "total" in v or ("gross" in v and "accumulated" in v)


def defaults_applied(inp: dict[str, Any]) -> list[tuple[str, str]]:
    """(pointer, destination) of every default applied (R8 as clarified; R17), years first."""
    out: list[tuple[str, str]] = []
    wdr_given = get(inp, "treatments", "write_downs_of_receivables") is not None
    for i, y in enumerate(inp["years"]):
        st = y.get("statutory")
        if st is None:
            continue
        bs = st.get("balance_sheet") or {}
        base = f"/years/{i}/statutory/balance_sheet"
        for key, dest in DEFAULT_ROWS_BS:
            if aggregate_given(bs, key):
                out.append((f"{base}/{key}", dest))
        # Appendix A.2, choice D: A5 and B14 core (R18); B10 d in the investing result unless
        # the analyst states the treatment (R17).
        inc = st.get("income_statement") or {}
        ibase = f"/years/{i}/statutory/income_statement"
        if "other_revenue_and_income" in inc:
            out.append((f"{ibase}/other_revenue_and_income", "value_of_production"))
        if "other_operating_expenses" in inc:
            out.append((f"{ibase}/other_operating_expenses", "external_costs"))
        b10d = get(inc, "depreciation_and_write_downs", "write_downs_of_receivables")
        if not wdr_given and b10d is not None:
            out.append(
                (
                    f"{ibase}/depreciation_and_write_downs/write_downs_of_receivables",
                    "investing_result",
                )
            )
    if not wdr_given:
        out.append(("/treatments/write_downs_of_receivables", "investing"))
    for i, y in enumerate(inp["years"]):
        bs = get(y, "statutory", "balance_sheet")
        if bs is None:
            continue
        for key in taken_as_zero(bs):
            out.append((f"/years/{i}/statutory/balance_sheet/{key}", "0"))
    return out


SEPARATE_LINES = {
    "subscribed_capital_unpaid",
    "prepayments_and_accrued_income/total",
    "accruals_and_deferred_income/total",
}


def taken_as_zero(bs: dict[str, Any]) -> list[str]:
    """Items of a statutory balance sheet taken as zero because they are not given (D1, D2,
    D3), and separate lines D and E presented within C.II and D, in the order of art. 2424."""
    out = []
    if "subscribed_capital_unpaid" not in bs:
        out.append("subscribed_capital_unpaid")
    for parent, sub in (
        ("receivables", "subscribed_capital_unpaid"),
        ("receivables", "prepayments_and_accrued_income"),
    ):
        if get(bs, parent, sub) is None:
            out.append(f"{parent}/{sub}")
    if "prepayments_and_accrued_income" not in bs and (
        get(bs, "receivables", "prepayments_and_accrued_income") is not None
    ):
        out.append("prepayments_and_accrued_income/total")
    for parent, sub in (
        ("provisions", "of_which_derivative_liabilities"),
        ("payables", "accruals_and_deferred_income"),
    ):
        if get(bs, parent, sub) is None:
            out.append(f"{parent}/{sub}")
    if "accruals_and_deferred_income" not in bs and (
        get(bs, "payables", "accruals_and_deferred_income") is not None
    ):
        out.append("accruals_and_deferred_income/total")
    return out


def reclassified_sign_with_lines(path: tuple[str, ...]) -> str:
    if (
        len(path) >= 3
        and path[0] == "balance_sheet"
        and path[1]
        in (
            "non_operating_assets",
            "quasi_equity_items",
            "debt_like_items",
        )
    ):
        return "non_negative"
    return reclassified_sign(path)


def validate(inp: dict[str, Any]) -> None:
    years = inp.get("years") or []
    if not years:
        raise RefError("FM-E001", "/years empty")
    dates: list[tuple[int, int, int] | None] = []
    for i, y in enumerate(years):
        if "statutory" in y and "reclassified" in y:
            raise RefError("FM-E001", f"/years/{i} both forms")
        dates.append(parse_date(y["date"], f"/years/{i}/date") if "date" in y else None)
    for i in range(1, len(years)):
        a, b = dates[i - 1], dates[i]
        if a is not None and b is not None and not consecutive(a, b):
            raise RefError("FM-E001", f"/years/{i}/date not consecutive")
    for i, y in enumerate(years):
        if "statutory" in y:
            walk_signs(y["statutory"], f"/years/{i}/statutory", (), statutory_sign)
        if "reclassified" in y:
            r = y["reclassified"]
            walk_signs(r, f"/years/{i}/reclassified", (), reclassified_sign_with_lines)
            for key in ("non_operating_assets", "quasi_equity_items", "debt_like_items"):
                for j, line in enumerate(get(r, "balance_sheet", key) or []):
                    if not str(line["label"]).strip():
                        raise RefError("FM-E001", f"{key}/{j}/label")
                    other = key == "debt_like_items" and line["measurement_basis"] == "other"
                    blank_note = "basis_note" in line and not str(line["basis_note"]).strip()
                    if blank_note or (other and not str(line.get("basis_note", "")).strip()):
                        raise RefError("FM-E001", f"{key}/{j}/basis_note")
            if "treatment_overrides" in y:
                raise RefError("FM-E001", f"/years/{i}/treatment_overrides on reclassified")
        others = set(y.get("treatment_overrides") or {}) - {
            "financial_fixed_assets",
            "shareholder_loans",
        }
        if others:
            # R9: only B.III and shareholder loans accept a per-year override.
            raise RefError("FM-E001", f"/years/{i}/treatment_overrides/{sorted(others)[0]}")
        for key in ("financial_fixed_assets", "shareholder_loans"):
            ov = get(y, "treatment_overrides", key)
            if isinstance(ov, dict) and "split" in ov:
                for p in ov["split"]:
                    if dec(p["amount"]) < 0 or dec(p.get("beyond_one_year", 0)) < 0:
                        raise RefError("FM-E001", "split amount negative")
                    if not str(p["label"]).strip():
                        raise RefError("FM-E001", "split label blank")
                    if (
                        key == "shareholder_loans"
                        and p["destination"] == "quasi_equity"
                        and "beyond_one_year" in p
                    ):
                        # A quasi-equity part is outside the net financial position: no maturity.
                        raise RefError("FM-E001", "beyond_one_year on a quasi_equity part")
    for k, adj in enumerate(inp.get("adjustments") or []):
        kind = adj["kind"]
        # R17: D18 and D19 are fixed in the investing result; no adjustment moves them.
        if kind == "non_recurring" and adj["aggregate"] not in NON_RECURRING_SOURCES:
            raise RefError("FM-E001", f"/adjustments/{k}/aggregate")
        if kind == "investing" and adj["aggregate"] not in INVESTING_SOURCES:
            raise RefError("FM-E001", f"/adjustments/{k}/aggregate")
        if not str(adj["label"]).strip():
            raise RefError("FM-E001", f"/adjustments/{k}/label")
        if not str(adj["reason"]).strip():
            raise RefError("FM-E001", f"/adjustments/{k}/reason")
        related = adj.get("related_non_operating_asset")
        if kind == "investing" and related is not None and not str(related).strip():
            raise RefError("FM-E001", f"/adjustments/{k}/related_non_operating_asset")
        if len(adj["amounts"]) != len(years):
            raise RefError("FM-E001", f"/adjustments/{k}/amounts length")
        for i, a in enumerate(adj["amounts"]):
            if a is None:
                continue
            values = list(a.values()) if kind == "lease" else [a]
            if any(dec(v) < 0 for v in values):
                raise RefError("FM-E001", f"/adjustments/{k}/amounts/{i} negative")
            if "reclassified" in years[i]:
                raise RefError("FM-E001", f"/adjustments/{k}/amounts/{i} on reclassified")
            if kind == "lease" and dec(a["liability_current"]) > dec(a["liability"]):
                # R21: the current part of the lease liability is a part of it.
                raise e026("lease_liability_current")


# ---------------------------------------------------------------- input checks (R7)
def e026(relation: str) -> RefError:
    return RefError("FM-E026", relation)


def check(checks: dict[str, str], relation: str, left: Amt, right: Amt) -> None:
    if left.known and right.known:
        if left.value != right.value:
            raise e026(relation)
        checks[relation] = "reconciled"
    else:
        checks[relation] = "not_checked"


def check_at_most(checks: dict[str, str], relation: str, parts: Amt, whole: Amt) -> None:
    """A split within its aggregate: the parts do not exceed it."""
    if parts.known and whole.known:
        if parts.value > whole.value:
            raise e026(relation)
        checks[relation] = "reconciled"
    else:
        checks[relation] = "not_checked"


def opt(node: dict[str, Any] | None, *keys: str) -> Amt:
    v = get(node, *keys)
    return known(dec(v)) if v is not None else lacking("/".join(keys))


def given_parts(node: dict[str, Any], keys: list[str]) -> Amt:
    """Sum of the "of which" parts given; not checked when none is given."""
    vals = [
        dec(node[k]["total"] if isinstance(node[k], dict) else node[k])
        for k in keys
        if k in node and (not isinstance(node[k], dict) or "total" in node[k])
    ]
    return known(sum(vals, ZERO)) if vals else lacking("none")


def separate(
    bs: dict[str, Any], base: str, key: str, parent: str, sub: str, reader: Callable[..., Amt]
) -> Amt:
    """Item A, assets D or liabilities E as a separate item: zero when presented within C.II or
    D (its "of which" given there) and not given separately."""
    if key in bs:
        return reader(bs, base, key)
    if get(bs, parent, sub) is not None:
        return known(ZERO)
    return reader(bs, base, key)


def balance_sheet_checks(bs: dict[str, Any], inc: dict[str, Any], base: str) -> dict[str, str]:
    c: dict[str, str] = {}
    for key in ("intangible_assets", "tangible_assets"):
        n = get(bs, key) or {}
        check(
            c,
            f"{key}_net",
            opt(n, "total"),
            total((1, opt(n, "gross")), (-1, opt(n, "accumulated"))),
        )
    b1 = net_node(bs, base, "intangible_assets")
    b2 = net_node(bs, base, "tangible_assets")
    b3 = leaf(bs, base, "financial_fixed_assets")
    check(c, "fixed_assets_total", opt(bs, "fixed_assets"), plus(b1, b2, b3))
    c1 = leaf(bs, base, "inventories")
    c2 = node_total(bs, base, "receivables")
    c3 = leaf(bs, base, "current_financial_assets")
    c4 = leaf(bs, base, "cash")
    check(c, "current_assets_total", opt(bs, "current_assets"), plus(c1, c2, c3, c4))
    eq = get(bs, "equity") or {}
    check(c, "equity_total", opt(eq, "total"), plus(*(opt(eq, p) for p in EQUITY_PARTS)))
    # A relation compares items given only: item A as given, never as the zero of D1; the
    # separate lines D and E as given, or zero when their part is given within C.II or D (M2),
    # since that part is then inside C or D.
    a_sep = leaf(bs, base, "subscribed_capital_unpaid")
    d_sep = separate(
        bs,
        base,
        "prepayments_and_accrued_income",
        "receivables",
        "prepayments_and_accrued_income",
        node_total,
    )
    e_sep = separate(
        bs,
        base,
        "accruals_and_deferred_income",
        "payables",
        "accruals_and_deferred_income",
        node_total,
    )
    b_tot = opt(bs, "fixed_assets") if "fixed_assets" in bs else plus(b1, b2, b3)
    c_tot = opt(bs, "current_assets") if "current_assets" in bs else plus(c1, c2, c3, c4)
    assets = plus(a_sep, b_tot, c_tot, d_sep)
    check(c, "total_assets_total", opt(bs, "total_assets"), assets)
    liab = plus(
        equity_amount(bs, base),
        node_total(bs, base, "provisions"),
        node_total(bs, base, "employee_severance_indemnity"),
        node_total(bs, base, "payables"),
        e_sep,
    )
    check(c, "total_liabilities_and_equity_total", opt(bs, "total_liabilities_and_equity"), liab)
    ta = opt(bs, "total_assets") if "total_assets" in bs else assets
    tl = opt(bs, "total_liabilities_and_equity") if "total_liabilities_and_equity" in bs else liab
    check(c, "total_assets_equals_total_liabilities_and_equity", ta, tl)
    rec = get(bs, "receivables") or {}
    check_at_most(c, "receivables_beyond_one_year", opt(rec, "beyond_one_year"), opt(rec, "total"))
    check_at_most(
        c,
        "receivables_of_which",
        given_parts(rec, ["trade", "subscribed_capital_unpaid", "prepayments_and_accrued_income"]),
        opt(rec, "total"),
    )
    pay = get(bs, "payables") or {}
    check_at_most(c, "payables_beyond_one_year", opt(pay, "beyond_one_year"), opt(pay, "total"))
    check_at_most(
        c,
        "payables_of_which",
        given_parts(
            pay, ["financial_debt", "shareholder_loans", "trade", "accruals_and_deferred_income"]
        ),
        opt(pay, "total"),
    )
    for key in ("financial_debt", "shareholder_loans"):
        check_at_most(
            c, f"{key}_beyond_one_year", opt(pay, key, "beyond_one_year"), opt(pay, key, "total")
        )
    prov = get(bs, "provisions") or {}
    check_at_most(c, "provisions_beyond_one_year", opt(prov, "beyond_one_year"), opt(prov, "total"))
    check_at_most(
        c, "provisions_of_which", opt(prov, "of_which_derivative_liabilities"), opt(prov, "total")
    )
    for key in (
        "employee_severance_indemnity",
        "prepayments_and_accrued_income",
        "accruals_and_deferred_income",
    ):
        n = get(bs, key) or {}
        check_at_most(c, f"{key}_beyond_one_year", opt(n, "beyond_one_year"), opt(n, "total"))
    check(
        c,
        "profit_for_the_year",
        opt(eq, "profit_or_loss_for_the_year"),
        opt(inc, "profit_or_loss_for_the_year"),
    )
    return c


def income_statement_checks(inc: dict[str, Any], c: dict[str, str]) -> None:
    a_parts = [
        opt(inc, k)
        for k in (
            "revenue",
            "change_in_product_inventories",
            "own_work_capitalised",
            "other_revenue_and_income",
        )
    ]
    check(c, "value_of_production_total", opt(inc, "value_of_production"), plus(*a_parts))
    b10 = get(inc, "depreciation_and_write_downs") or {}
    ab = opt(b10, "depreciation_and_amortisation")
    cc = opt(b10, "write_downs_of_fixed_assets")
    dd = opt(b10, "write_downs_of_receivables")
    ac_given = opt(b10, "value_adjustments_of_fixed_assets")
    check(c, "value_adjustments_of_fixed_assets_total", ac_given, plus(ab, cc))
    # B10 c is a part of the group B10 a to c (review of PR 2, blocking 1).
    check_at_most(c, "value_adjustments_of_fixed_assets_of_which", cc, ac_given)
    ac = ac_given if ac_given.known else plus(ab, cc)
    check(c, "depreciation_and_write_downs_total", opt(b10, "total"), plus(ac, dd))
    b10_amt = opt(b10, "total") if "total" in b10 else plus(ac, dd)
    b_parts = [
        opt(inc, k)
        for k in ("purchases", "services", "use_of_third_party_assets", "personnel_costs")
    ]
    b_parts.append(b10_amt)
    b_parts += [
        opt(inc, k)
        for k in (
            "change_in_raw_material_inventories",
            "provisions_for_risks",
            "other_provisions",
            "other_operating_expenses",
        )
    ]
    check(c, "costs_of_production_total", opt(inc, "costs_of_production"), plus(*b_parts))
    a_amt = opt(inc, "value_of_production") if "value_of_production" in inc else plus(*a_parts)
    b_amt = opt(inc, "costs_of_production") if "costs_of_production" in inc else plus(*b_parts)
    diff = total((1, a_amt), (-1, b_amt))
    key = "difference_between_value_and_costs_of_production"
    check(c, f"{key}_total", opt(inc, key), diff)
    c_der = total(
        (1, opt(inc, "income_from_equity_investments")),
        (1, opt(inc, "other_financial_income")),
        (-1, opt(inc, "interest_and_other_financial_charges")),
        (1, opt(inc, "exchange_gains_and_losses")),
    )
    check(c, "financial_income_and_charges_total", opt(inc, "financial_income_and_charges"), c_der)
    d_der = total(
        (1, opt(inc, "revaluations_of_financial_assets")),
        (-1, opt(inc, "write_downs_of_financial_assets")),
    )
    check(
        c,
        "value_adjustments_of_financial_assets_total",
        opt(inc, "value_adjustments_of_financial_assets"),
        d_der,
    )
    diff_amt = opt(inc, key) if key in inc else diff
    c_amt = (
        opt(inc, "financial_income_and_charges") if "financial_income_and_charges" in inc else c_der
    )
    d_amt = (
        opt(inc, "value_adjustments_of_financial_assets")
        if "value_adjustments_of_financial_assets" in inc
        else d_der
    )
    pbt_der = plus(diff_amt, c_amt, d_amt)
    check(c, "profit_before_tax_total", opt(inc, "profit_before_tax"), pbt_der)
    pbt = opt(inc, "profit_before_tax") if "profit_before_tax" in inc else pbt_der
    check(
        c,
        "profit_or_loss_for_the_year_total",
        opt(inc, "profit_or_loss_for_the_year"),
        total((1, pbt), (-1, opt(inc, "income_taxes"))),
    )


# ---------------------------------------------------------------- statutory balance sheet
@dataclass
class Lst:
    """A list output: its items, or None with the amount that makes it missing."""

    items: list[dict[str, Any]] | None
    amount: Amt


@dataclass
class Moved:
    index: int
    kind: str
    label: str
    reason: str
    amount: Amt
    maturity: str | None
    aggregate: str | None


SOURCE_OF = {
    "financial_receivable": "receivables",
    "operating_cash": "cash",
    "accrued_interest": "accruals",
    "tax_instalments": "payables_remainder",
    "non_current_trade_payables": "payables_remainder",
    "dividends_declared": "equity",
}


@at_50_digits
def statutory_balance_sheet(inp: dict[str, Any], i: int) -> dict[str, Any]:
    y = inp["years"][i]
    st = y.get("statutory") or {}
    bs = st.get("balance_sheet") or {}
    inc = st.get("income_statement") or {}
    base = f"/years/{i}/statutory/balance_sheet"
    treatments = inp.get("treatments") or {}
    overrides = y.get("treatment_overrides") or {}
    checks = balance_sheet_checks(bs, inc, base)
    income_statement_checks(inc, checks)

    # Items of Appendix A.1.
    a_within = of_which(bs, "receivables", "subscribed_capital_unpaid")
    # D1: item A not given is zero, the same zero wherever it enters a number.
    a_total = plus(known(dec(bs.get("subscribed_capital_unpaid", 0))), known(a_within))
    b1 = net_node(bs, base, "intangible_assets")
    b2 = net_node(bs, base, "tangible_assets")
    b3 = leaf(bs, base, "financial_fixed_assets")
    c1 = leaf(bs, base, "inventories")
    c2 = node_total(bs, base, "receivables")
    c3 = leaf(bs, base, "current_financial_assets")
    c4 = leaf(bs, base, "cash")
    d_sep = separate(
        bs,
        base,
        "prepayments_and_accrued_income",
        "receivables",
        "prepayments_and_accrued_income",
        node_total,
    )
    eq = equity_amount(bs, base)
    prov = node_total(bs, base, "provisions")
    deriv = of_which(bs, "provisions", "of_which_derivative_liabilities")
    tfr = node_total(bs, base, "employee_severance_indemnity")
    pay = node_total(bs, base, "payables")
    pay_node = bs.get("payables")
    fd = node_total(pay_node, f"{base}/payables", "financial_debt")
    sl = node_total(pay_node, f"{base}/payables", "shareholder_loans")
    e_sep = separate(
        bs,
        base,
        "accruals_and_deferred_income",
        "payables",
        "accruals_and_deferred_income",
        node_total,
    )
    e_within = of_which(bs, "payables", "accruals_and_deferred_income")

    def treatment_of(name: str) -> tuple[Any, str]:
        if name in overrides:
            return overrides[name], f"/years/{i}/treatment_overrides/{name}"
        return treatments.get(name), f"/treatments/{name}"

    def on_split(name: str) -> Callable[[D, D], None]:
        def f(agg: D, s: D) -> None:
            if agg != s:
                raise e026(f"{name}_split")
            checks[f"{name}_split"] = "reconciled"

        return f

    permits: dict[str, list[str]] = {}
    for name in ("financial_fixed_assets", "shareholder_loans"):
        checks[f"{name}_split"] = "not_checked"
    loan_split_maturities(
        checks,
        overrides.get("shareholder_loans"),
        get(pay_node, "shareholder_loans", "beyond_one_year"),
    )
    t, tp = treatment_of("financial_fixed_assets")
    ffa, p = shares_of(
        b3,
        t,
        tp,
        FFA_PERMITTED,
        "Financial fixed assets (B.III)",
        on_split("financial_fixed_assets"),
        f"{tp}/split",
    )
    permits.update(p)
    t, tp = treatment_of("shareholder_loans")
    sls, p = shares_of(
        sl,
        t,
        tp,
        SL_PERMITTED,
        "Shareholder loans (D)",
        on_split("shareholder_loans"),
        f"{tp}/split",
    )
    permits.update(p)
    dv = treatments.get("derivative_liabilities")
    dshares, p = shares_of(
        known(deriv),
        dv,
        "/treatments/derivative_liabilities",
        DERIV_PERMITTED,
        "Derivative liabilities (B)",
        lambda a, b: None,
        "",
    )
    permits.update(p)

    # Adjustments of Appendix C, in the order given.
    remaining: dict[str, Amt] = {
        "intangible_assets": b1,
        "tangible_assets": b2,
        "receivables": total((1, c2), (-1, known(a_within))),
        "current_financial_assets": c3,
        "cash": c4,
        "accruals": plus(e_sep, known(e_within)),
        "payables_remainder": total((1, pay), (-1, fd), (-1, sl), (-1, known(e_within))),
        "provisions": total((1, prov), (-1, dest_sum(dshares, "financial_liability"))),
        "employee_severance_indemnity": tfr,
        "equity": total((1, eq), (-1, a_total)),
        # Assets D: the separate line (zero when presented within C.II), and the part within C.II.
        "prepayments": d_sep,
        "prepayments_within": known(of_which(bs, "receivables", "prepayments_and_accrued_income")),
    }

    def take(src: str, a: D) -> Amt:
        """Moves `a` out of what remains in `src` (R10): FM-E026 when it is larger."""
        rem = remaining[src]
        if not rem.known:
            return rem
        if a > rem.value and a != 0:
            raise e026("adjustment_within_source")
        remaining[src] = known(rem.value - a)
        return known(a)

    moved: list[Moved] = []
    leases: list[tuple[int, str, dict[str, D], Amt]] = []
    for k, adj in enumerate(inp.get("adjustments") or []):
        amount = adj["amounts"][i]
        if amount is None:
            continue
        kind = adj["kind"]
        if kind in ("investing", "non_recurring"):
            continue  # income statement kinds
        if kind == "lease":
            # R21, balance sheet side: the prepayment leaves assets D (the separate line when
            # given, otherwise the part presented within C.II, which also leaves C.II); the
            # asset joins operating fixed assets, the liability the net financial position, and
            # asset - liability - prepayment is the equity effect.
            la = {key: dec(v) for key, v in amount.items()}
            prepayment = known(ZERO)
            if la["prepayment"] != 0:
                if "prepayments_and_accrued_income" in bs or (
                    get(bs, "receivables", "prepayments_and_accrued_income") is None
                ):
                    prepayment = take("prepayments", la["prepayment"])
                else:
                    prepayment = take("prepayments_within", la["prepayment"])
                    if prepayment.known:
                        take("receivables", la["prepayment"])
            effect = total((1, known(la["asset"])), (-1, known(la["liability"])), (-1, prepayment))
            rem_eq = remaining["equity"]
            remaining["equity"] = plus(rem_eq, effect)
            leases.append((k, adj["label"], la, effect))
            continue
        src = (
            adj["aggregate"]
            if kind in ("non_operating_asset", "debt_like_provision")
            else SOURCE_OF[kind]
        )
        rem = remaining[src]
        a = dec(amount)
        if rem.known:
            if a > rem.value and a != 0:
                # R10: an amount larger than what remains in its source (zero moves nothing).
                raise e026("adjustment_within_source")
            remaining[src] = known(rem.value - a)
            took = known(a)
        else:
            took = rem
        moved.append(
            Moved(
                k,
                kind,
                adj["label"],
                adj["reason"],
                took,
                adj.get("maturity"),
                adj.get("aggregate"),
            )
        )

    def moved_of(kind: str) -> list[Moved]:
        return [m for m in moved if m.kind == kind]

    op_cash = plus(*(m.amount for m in moved_of("operating_cash")))

    # Operating block (B.1).
    ofa = plus(
        remaining["intangible_assets"],
        remaining["tangible_assets"],
        dest_sum(ffa, "operating"),
        *(known(la["asset"]) for _, _, la, _ in leases),
    )
    owc = total(
        (1, c1),
        (1, remaining["receivables"]),
        (1, remaining["prepayments"]),
        (1, op_cash),
        (-1, remaining["payables_remainder"]),
        (-1, remaining["accruals"]),
    )
    twc = total(
        (1, part(bs, base, "receivables", "trade")),
        (1, c1),
        (-1, part(bs, base, "payables", "trade")),
    )
    op_prov = plus(remaining["provisions"], remaining["employee_severance_indemnity"])
    ic = total((1, ofa), (1, owc), (-1, op_prov))

    noa_pairs = [(s.label, s.amount) for s in ffa if s.dest == "non_operating"]
    noa_pairs += [(m.label, m.amount) for m in moved_of("non_operating_asset")]
    noa_total = plus(*(a for _, a in noa_pairs))
    noa = Lst(
        [{"label": lb, "value": a.value} for lb, a in noa_pairs] if noa_total.known else None,
        noa_total,
    )
    tfi = plus(ic, noa_total)

    # Sources.
    equity = remaining["equity"]
    q_pairs = [(s.label, s.amount) for s in sls if s.dest == "quasi_equity"]
    q_total = plus(*(a for _, a in q_pairs))
    qe = Lst(
        [{"label": lb, "value": a.value} for lb, a in q_pairs] if q_total.known else None, q_total
    )
    dl_moves = [
        m
        for m in moved
        if m.kind in ("debt_like_provision", "tax_instalments", "dividends_declared")
    ]
    dl_total = plus(*(m.amount for m in dl_moves))
    dl = Lst(
        [
            {"label": m.label, "value": m.amount.value, "measurement_basis": "book_value"}
            for m in dl_moves
        ]
        if dl_total.known
        else None,
        dl_total,
    )

    # Net financial position: signed components with their current and non-current parts.
    comps: list[tuple[str, str, Amt, Amt, Amt]] = []
    fd_b = node_part(pay_node, f"{base}/payables", "financial_debt", fd)
    comps.append(
        (
            "Financial debt (D)",
            f"{base}/payables/financial_debt",
            fd,
            total((1, fd), (-1, fd_b)),
            fd_b,
        )
    )
    sl_b = node_part(pay_node, f"{base}/payables", "shareholder_loans", sl)
    for s in sls:
        if s.dest != "financial_debt":
            continue
        beyond = s.beyond if s.beyond is not None else sl_b
        # A maturity part never exists without its loan (R6): while the loan waits for its
        # treatment, the part beyond one year waits too.
        non_current = beyond if s.amount.known else plus(s.amount, beyond)
        comps.append(
            (
                s.label,
                f"{base}/payables/shareholder_loans",
                s.amount,
                total((1, s.amount), (-1, beyond)),
                non_current,
            )
        )
    cash_nfp = total((1, c4), (-1, op_cash))
    neg_cash = total((-1, cash_nfp))
    comps.append(("Cash (C.IV)", f"{base}/cash", neg_cash, neg_cash, known(ZERO)))
    other: list[tuple[str, Amt]] = []
    neg_c3 = total((-1, remaining["current_financial_assets"]))
    other.append(("Current financial assets (C.III)", neg_c3))
    comps.append(
        (
            "Current financial assets (C.III)",
            f"{base}/current_financial_assets",
            neg_c3,
            neg_c3,
            known(ZERO),
        )
    )
    for m in moved_of("financial_receivable"):
        neg = total((-1, m.amount))
        other.append((m.label, neg))
        within = m.maturity == "within_one_year"
        comps.append(
            (
                m.label,
                f"/adjustments/{m.index}",
                neg,
                neg if within else known(ZERO),
                known(ZERO) if within else neg,
            )
        )
    for s in ffa:
        if s.dest == "financial_asset":
            neg = total((-1, s.amount))
            other.append((s.label, neg))
            comps.append((s.label, f"{base}/financial_fixed_assets", neg, known(ZERO), neg))
    for s in dshares:
        if s.dest == "financial_liability":
            other.append((s.label, s.amount))
            comps.append(
                (
                    s.label,
                    f"{base}/provisions/of_which_derivative_liabilities",
                    s.amount,
                    known(ZERO),
                    s.amount,
                )
            )
    entries: list[tuple[int, Any]] = [(m.index, m) for m in moved] + [(lz[0], lz) for lz in leases]
    for index, e in sorted(entries, key=lambda x: x[0]):
        if isinstance(e, tuple):
            # R21: the lease liability, an other financial liability with its current part.
            _, label, la, _ = e
            liab = known(la["liability"])
            other.append((label, liab))
            comps.append(
                (
                    label,
                    f"/adjustments/{index}",
                    liab,
                    known(la["liability_current"]),
                    known(la["liability"] - la["liability_current"]),
                )
            )
        elif e.kind == "accrued_interest":
            other.append((e.label, e.amount))
            comps.append((e.label, f"/adjustments/{index}", e.amount, e.amount, known(ZERO)))
        elif e.kind == "non_current_trade_payables":
            other.append((e.label, e.amount))
            comps.append((e.label, f"/adjustments/{index}", e.amount, known(ZERO), e.amount))
    gross = plus(fd, dest_sum(sls, "financial_debt"))
    nfp_total = plus(*(c[2] for c in comps))
    nfp_cur = plus(*(c[3] for c in comps))
    nfp_ncur = plus(*(c[4] for c in comps))
    position = Lst(
        {
            "gross_debt": gross.value,
            "cash": cash_nfp.value,
            "other_items": [{"name": n, "amount": a.value} for n, a in other],
        }  # type: ignore[arg-type]
        if nfp_total.known
        else None,
        nfp_total,
    )
    composition = Lst(
        [
            {
                "label": lb,
                "source": src,
                "amount": a.value,
                "current": cu.value,
                "non_current": nc.value,
            }
            for lb, src, a, cu, nc in comps
        ]
        if nfp_total.known
        else None,
        nfp_total,
    )
    total_sources = plus(equity, q_total, nfp_total, dl_total)
    if tfi.known and total_sources.known:
        if tfi.value != total_sources.value:
            # With every item given the identity follows from R7 (a defect otherwise); with
            # items taken as zero, the inputs given do not balance.
            # Only a separate line taken as zero (item A, line D, line E) can unbalance the
            # sides; an "of which" part moves an amount within one side (M1).
            separate_lines = SEPARATE_LINES & set(taken_as_zero(bs))
            raise RefError("FM-E026" if separate_lines else "FM-E012", "identity")
        checks["funds_invested_equals_sources"] = "reconciled"
    else:
        checks["funds_invested_equals_sources"] = "not_checked"

    # Current and non-current (R15).
    rec_b = part(bs, base, "receivables", "beyond_one_year")
    pay_b = part(bs, base, "payables", "beyond_one_year")
    d_b = beyond_or(bs, "prepayments_and_accrued_income", d_sep, ZERO_IF_ABSENT)
    e_b = beyond_or(bs, "accruals_and_deferred_income", e_sep, ZERO_IF_ABSENT)
    prov_nc = beyond_or(bs, "provisions", prov, WHOLE_IF_ABSENT)
    tfr_nc = beyond_or(bs, "employee_severance_indemnity", tfr, WHOLE_IF_ABSENT)
    # Totals given (B, C, total assets) stand in for their components (review of PR 1).
    b_whole = opt(bs, "fixed_assets") if "fixed_assets" in bs else plus(b1, b2, b3)
    c_whole = opt(bs, "current_assets") if "current_assets" in bs else plus(c1, c2, c3, c4)
    fixed = plus(b_whole, rec_b, d_b)
    current = total(
        (1, c_whole),
        (-1, rec_b),
        (-1, known(a_within)),
        (1, d_sep),
        (-1, d_b),
    )
    cur_liab = total(
        (1, prov),
        (-1, prov_nc),
        (1, tfr),
        (-1, tfr_nc),
        (1, pay),
        (-1, pay_b),
        (1, e_sep),
        (-1, e_b),
    )
    nc_liab = plus(prov_nc, tfr_nc, pay_b, e_b)
    cnc = {
        "fixed_assets": fixed,
        "current_assets": current,
        "inventories": c1,
        "cash": c4,
        "total_assets": (
            total((1, opt(bs, "total_assets")), (-1, a_total))
            if "total_assets" in bs
            # B + C + D minus A within C.II: the parts beyond one year cancel out (audit F).
            else total((1, b_whole), (1, c_whole), (1, d_sep), (-1, known(a_within)))
        ),
        "current_liabilities": cur_liab,
        "non_current_liabilities": nc_liab,
    }
    balance_sheet = {
        "operating": {
            "operating_fixed_assets": ofa,
            "operating_working_capital": owc,
            "trade_working_capital": twc,
            "operating_provisions": op_prov,
            "invested_capital": ic,
        },
        "non_operating_assets": noa,
        "non_operating_assets_total": noa_total,
        "total_funds_invested": tfi,
        "sources": {
            "equity": equity,
            "quasi_equity_items": qe,
            "quasi_equity_items_total": q_total,
            "net_financial_position": {
                "position": position,
                "total": nfp_total,
                "current": nfp_cur,
                "non_current": nfp_ncur,
                "composition": composition,
            },
            "debt_like_items": dl,
            "debt_like_items_total": dl_total,
            "total_sources": total_sources,
        },
        "excess_cash": cash_nfp,
    }
    return finish(balance_sheet, cnc, checks, permits, i)


def loan_split_maturities(checks: dict[str, str], override: Any, node_beyond: Any) -> None:
    """A split of shareholder loans: each financial-debt part's amount beyond one year is at
    most the part; the parts' amounts beyond one year equal the loans' when every part is
    financial debt with its maturity given, and never exceed it."""
    parts_rel, sum_rel = (
        "shareholder_loans_split_parts_beyond_one_year",
        "shareholder_loans_split_beyond_one_year",
    )
    if not isinstance(override, dict) or "split" not in override:
        checks[parts_rel] = checks[sum_rel] = "not_checked"
        return
    total_beyond, n_given, complete = ZERO, 0, True
    for p in override["split"]:
        if p["destination"] != "financial_debt" or "beyond_one_year" not in p:
            complete = False
            continue
        if dec(p["beyond_one_year"]) > dec(p["amount"]):
            raise e026(parts_rel)
        total_beyond += dec(p["beyond_one_year"])
        n_given += 1
    every = complete and n_given > 0
    checks[parts_rel] = "reconciled" if every else "not_checked"
    if node_beyond is None:
        checks[sum_rel] = "not_checked"
    elif every:
        if total_beyond != dec(node_beyond):
            raise e026(sum_rel)
        checks[sum_rel] = "reconciled"
    elif total_beyond > dec(node_beyond):
        raise e026(sum_rel)
    else:
        checks[sum_rel] = "not_checked"


def node_part(node: dict[str, Any] | None, base: str, key: str, whole: Amt) -> Amt:
    """The beyond-one-year part of a loan node: missing with the loan, or with its own pointer."""
    if not whole.known:
        return whole
    return part(node, base, key, "beyond_one_year")


ZERO_IF_ABSENT = "zero"
WHOLE_IF_ABSENT = "whole"


def beyond_or(bs: dict[str, Any], key: str, whole: Amt, rule: str) -> Amt:
    """The non-current part of a node whose maturity has a default (R15)."""
    if not whole.known:
        return whole
    v = get(bs, key, "beyond_one_year")
    if v is not None:
        return known(dec(v))
    return known(ZERO) if rule == ZERO_IF_ABSENT else whole


def finish(
    balance_sheet: dict[str, Any],
    cnc: dict[str, Any],
    checks: dict[str, str],
    permits: dict[str, list[str]],
    i: int,
) -> dict[str, Any]:
    """Amounts to values; every null output listed in `not_computed`, per block (R6)."""
    nc: dict[str, list[dict[str, Any]]] = {"balance_sheet": [], "current_non_current": []}

    def note(pointer: str, a: Amt, block: str) -> None:
        entry: dict[str, Any] = {"output": pointer, "missing": sorted(a.missing)}
        perm = [
            {"treatment": p, "destinations": permits[p]} for p in sorted(a.missing) if p in permits
        ]
        if perm:
            entry["permitted"] = perm
        nc[block].append(entry)

    def visit(node: Any, pointer: str, block: str) -> Any:
        if isinstance(node, Amt):
            if not node.known:
                note(pointer, node, block)
            return node.value
        if isinstance(node, Lst):
            if node.items is None:
                note(pointer, node.amount, block)
            return node.items
        if isinstance(node, dict):
            return {k: visit(v, f"{pointer}/{k}", block) for k, v in node.items()}
        return node

    bsv = visit(balance_sheet, f"/years/{i}/balance_sheet", "balance_sheet")
    cncv = visit(cnc, f"/years/{i}/current_non_current", "current_non_current")
    for block in nc.values():
        block.sort(key=lambda e: e["output"])
    return {"balance_sheet": bsv, "current_non_current": cncv, "checks": checks, "not_computed": nc}


# ---------------------------------------------------------------- statutory income statement
EBITDA_DEFINITION = (
    "EBITDA (margine operativo lordo, MOL) = value added \u2212 personnel costs (B9): value of "
    "production (A1 to A5, core) less raw materials, services, use of third-party assets, change "
    "in raw material inventories and other operating expenses (B6, B7, B8, B11, B14, core) and "
    "personnel costs (B9); before depreciation, amortisation and write-downs (B10), provisions "
    "(B12, B13), the investing and financing results and the non-recurring items (SPEC-0005 "
    "Appendix B.2)"
)


def obj(items: dict[str, Any] | None, why: Amt) -> Lst:
    """An object output (a reconciliation): its value, or None with the amount that is missing."""
    return Lst(items, why)  # type: ignore[arg-type]


def b10_reading(inc: dict[str, Any], base: str) -> tuple[Amt, Amt, Amt, bool, list[str], list[str]]:
    """B10 a and b, B10 c and B10 d as read (Decision 8): B10 a to c given as one group without
    its B10 c part is depreciation and amortisation, B10 c none; with B10 c given, the group less
    B10 c is depreciation and amortisation."""
    node = inc.get("depreciation_and_write_downs")
    b = f"{base}/depreciation_and_write_downs"
    if node is None:
        m = lacking(b)
        return m, m, m, False, [b], [b]
    va, ab, c, d = (
        node.get(k)
        for k in (
            "value_adjustments_of_fixed_assets",
            "depreciation_and_amortisation",
            "write_downs_of_fixed_assets",
            "write_downs_of_receivables",
        )
    )
    grouped = ab is None and c is None and va is not None
    if ab is not None:
        ab_amt = known(dec(ab))
    elif va is not None:
        ab_amt = known(dec(va) - (dec(c) if c is not None else ZERO))
    else:
        ab_amt = lacking(f"{b}/depreciation_and_amortisation")
    if c is not None:
        c_amt = known(dec(c))
    elif grouped:
        c_amt = known(ZERO)
    else:
        c_amt = lacking(f"{b}/write_downs_of_fixed_assets")
    d_amt = known(dec(d)) if d is not None else lacking(f"{b}/write_downs_of_receivables")
    # The pointers actually read (review of PR 2, blocking 2).
    va_ptr = f"{b}/value_adjustments_of_fixed_assets"
    if ab is not None:
        ab_read = [f"{b}/depreciation_and_amortisation"]
    elif va is not None:
        ab_read = [va_ptr] + ([f"{b}/write_downs_of_fixed_assets"] if c is not None else [])
    else:
        ab_read = [f"{b}/depreciation_and_amortisation"]
    c_read = [va_ptr] if grouped else [f"{b}/write_downs_of_fixed_assets"]
    return ab_amt, c_amt, d_amt, grouped, ab_read, c_read


def statutory_a_minus_b(inc: dict[str, Any], base: str) -> Amt:
    """A - B of the statutory statement: given, or from the totals A and B given, or from their
    items given (the same reading as the checks of R7)."""
    key = "difference_between_value_and_costs_of_production"
    if key in inc:
        return known(dec(inc[key]))
    a = (
        leaf(inc, base, "value_of_production")
        if "value_of_production" in inc
        else plus(
            *(
                leaf(inc, base, k)
                for k in (
                    "revenue",
                    "change_in_product_inventories",
                    "own_work_capitalised",
                    "other_revenue_and_income",
                )
            )
        )
    )
    if "costs_of_production" in inc:
        b = leaf(inc, base, "costs_of_production")
    else:
        node = inc.get("depreciation_and_write_downs") or {}
        nb = f"{base}/depreciation_and_write_downs"
        if "total" in node:
            b10 = known(dec(node["total"]))
        else:
            ac = (
                leaf(node, nb, "value_adjustments_of_fixed_assets")
                if "value_adjustments_of_fixed_assets" in node
                else plus(
                    leaf(node, nb, "depreciation_and_amortisation"),
                    leaf(node, nb, "write_downs_of_fixed_assets"),
                )
            )
            b10 = plus(ac, leaf(node, nb, "write_downs_of_receivables"))
            if "depreciation_and_write_downs" not in inc:
                b10 = lacking(nb)
        b = plus(
            *(
                leaf(inc, base, k)
                for k in (
                    "purchases",
                    "services",
                    "use_of_third_party_assets",
                    "personnel_costs",
                )
            ),
            b10,
            *(
                leaf(inc, base, k)
                for k in (
                    "change_in_raw_material_inventories",
                    "provisions_for_risks",
                    "other_provisions",
                    "other_operating_expenses",
                )
            ),
        )
    return total((1, a), (-1, b))


@at_50_digits
def statutory_income_statement(inp: dict[str, Any], i: int) -> dict[str, Any]:
    """Appendix A.2 and B.2 for a statutory year (R16 to R21)."""
    y = inp["years"][i]
    inc = get(y, "statutory", "income_statement") or {}
    base = f"/years/{i}/statutory/income_statement"
    wdr = get(inp, "treatments", "write_downs_of_receivables") or "investing"
    ab, c10, d10, grouped, _ab_read, c_read = b10_reading(inc, base)
    has_income_statement = get(y, "statutory", "income_statement") is not None

    def item(key: str) -> Amt:
        if key == "depreciation_and_amortisation":
            return ab
        if key == "write_downs_of_fixed_assets":
            return c10
        if key == "write_downs_of_receivables":
            return d10
        return leaf(inc, base, key)

    def src_pointer(key: str) -> str:
        if key in B10_KEYS:
            return f"{base}/depreciation_and_write_downs/{key}"
        return f"{base}/{key}"

    # What remains of each aggregate after the adjustments, in the order given (R10).
    remaining = {k: item(k) for k in IS_REFERENCES}

    def take(key: str, a: D) -> Amt:
        rem = remaining[key]
        if not rem.known:
            return rem
        if a > rem.value and a != 0:
            raise e026("adjustment_within_source")
        remaining[key] = known(rem.value - a)
        return known(a)

    investing: list[tuple[int, str, str, Amt]] = []
    non_recurring: list[tuple[int, str, str, Amt]] = []
    leases: list[tuple[int, str, Amt, D, D]] = []
    for k, adj in enumerate(inp.get("adjustments") or []):
        amount = adj["amounts"][i]
        if amount is None:
            continue
        kind = adj["kind"]
        if kind == "investing":
            investing.append(
                (k, adj["label"], adj["aggregate"], take(adj["aggregate"], dec(amount)))
            )
        elif kind == "non_recurring":
            non_recurring.append(
                (k, adj["label"], adj["aggregate"], take(adj["aggregate"], dec(amount)))
            )
        elif kind == "lease":
            # R21: the payments leave B8; depreciation and interest are added.
            paid = take("use_of_third_party_assets", dec(amount["payments"]))
            leases.append(
                (k, adj["label"], paid, dec(amount["depreciation"]), dec(amount["interest"]))
            )

    def moved_from(entries: list[tuple[int, str, str, Amt]], *keys: str) -> Amt:
        return plus(*(a for _, _, key, a in entries if key in keys))

    core = remaining
    a_given = "value_of_production" in inc
    a_whole = (
        leaf(inc, base, "value_of_production")
        if a_given
        else plus(
            item("revenue"),
            item("change_in_product_inventories"),
            item("own_work_capitalised"),
            item("other_revenue_and_income"),
        )
    )
    # Value of production, core: A less the parts of A1 and A5 moved out (R16, R18).
    vop = total(
        (1, a_whole),
        (-1, moved_from(investing, "other_revenue_and_income")),
        (-1, moved_from(non_recurring, "revenue", "other_revenue_and_income")),
    )
    purchases = core["purchases"]
    materials = plus(purchases, item("change_in_raw_material_inventories"))
    external = plus(
        materials,
        core["services"],
        core["use_of_third_party_assets"],
        core["other_operating_expenses"],
    )
    value_added = total((1, vop), (-1, external))  # E1: external costs
    personnel = core["personnel_costs"]
    ebitda = total((1, value_added), (-1, personnel))
    dna = plus(core["depreciation_and_amortisation"], *(known(dep) for *_, dep, _ in leases))
    operating_wdr = wdr == "operating"
    # E2: provisions, and B10 d when operating, below EBITDA besides depreciation.
    other_terms = [core["provisions_for_risks"], core["other_provisions"]]
    if operating_wdr:
        other_terms.append(core["write_downs_of_receivables"])
    other_charges = plus(*other_terms)
    operating_profit = total((1, ebitda), (-1, dna), (-1, other_charges))
    if "value_adjustments_of_financial_assets" in inc:
        d_total = leaf(inc, base, "value_adjustments_of_financial_assets")
    else:
        d_total = total(
            (1, leaf(inc, base, "revaluations_of_financial_assets")),
            (-1, leaf(inc, base, "write_downs_of_financial_assets")),
        )
    inv_terms = [
        (1, moved_from(investing, "other_revenue_and_income")),
        (-1, moved_from(investing, "other_operating_expenses")),
        (1, core["income_from_equity_investments"]),
        (1, core["other_financial_income"]),
        (1, d_total),
        (-1, core["write_downs_of_fixed_assets"]),
    ]
    if not operating_wdr:
        inv_terms.append((-1, core["write_downs_of_receivables"]))
    investing_result = total(*inv_terms)
    pbft = plus(operating_profit, investing_result)
    nr_income = [a for _, _, key, a in non_recurring if key in IS_INCOME]
    nr_charges = [a for _, _, key, a in non_recurring if key not in IS_INCOME]
    nr_result = total(*((1, a) for a in nr_income), *((-1, a) for a in nr_charges))
    if not has_income_statement:
        # E6: null when the year gives no income statement.
        nr_result = plus(nr_result, lacking(f"/years/{i}/statutory/income_statement"))
    interest = plus(core["interest_and_other_financial_charges"], *(known(it) for *_, it in leases))
    financing = total((1, core["exchange_gains_and_losses"]), (-1, interest))
    pbt = plus(pbft, nr_result, financing)
    taxes = leaf(inc, base, "income_taxes")
    net_income = total((1, pbt), (-1, taxes))

    # Reconciliations with A - B (R19): every amount moved, from the statutory items.
    a_b = statutory_a_minus_b(inc, base)
    nr_lines = []
    for k, label, key, a in non_recurring:
        if key in IS_A_AND_B:
            sign = -1 if key in IS_INCOME else 1
            nr_lines.append(
                (
                    k,
                    f"{label} (non-recurring part of {IS_REFERENCES[key]})",
                    [f"/adjustments/{k}"],
                    total((sign, a)),
                )
            )
    lease_lines = []
    for k, label, paid, dep, _ in leases:
        lease_lines.append(
            (k, f"{label} (lease payments removed from B8)", [f"/adjustments/{k}"], paid)
        )
        lease_lines.append((k, f"{label} (lease depreciation)", [f"/adjustments/{k}"], known(-dep)))
    c_label = "B10 c, write-downs of fixed assets (investing result)"
    if grouped:
        c_label += ": read as none, give the B10 c part (Decision 8)"
    op_lines = [(-1, c_label, c_read, core["write_downs_of_fixed_assets"])]
    if not operating_wdr:
        op_lines.append(
            (
                -1,
                "B10 d, write-downs of receivables (investing result)",
                [src_pointer("write_downs_of_receivables")],
                core["write_downs_of_receivables"],
            )
        )
    inv_lines = []
    for k, label, key, a in investing:
        sign = -1 if key in IS_INCOME else 1
        inv_lines.append(
            (
                k,
                f"{label} (investing part of {IS_REFERENCES[key]})",
                [f"/adjustments/{k}"],
                total((sign, a)),
            )
        )
    op_lines += sorted(inv_lines + nr_lines + lease_lines, key=lambda x: x[0])
    d_read = (
        [f"{base}/value_adjustments_of_financial_assets"]
        if "value_adjustments_of_financial_assets" in inc
        else [f"{base}/revaluations_of_financial_assets", f"{base}/write_downs_of_financial_assets"]
    )
    pbft_lines = sorted(nr_lines + lease_lines, key=lambda x: x[0])
    pbft_lines += [
        (
            -1,
            "C15, income from equity investments (investing result)",
            [f"{base}/income_from_equity_investments"],
            core["income_from_equity_investments"],
        ),
        (
            -1,
            "C16, other financial income (investing result)",
            [f"{base}/other_financial_income"],
            core["other_financial_income"],
        ),
        (
            -1,
            "D18 \u2212 D19, value adjustments of financial assets (investing result)",
            d_read,
            d_total,
        ),
    ]

    def reconciliation(lines: list[tuple[int, str, list[str], Amt]], end: Amt) -> Lst:
        amounts = [a for *_, a in lines]
        summed = plus(a_b, *amounts)
        if not (summed.known and end.known):
            return obj(None, plus(a_b, end, *amounts))
        if summed.value != end.value:
            raise RefError("FM-E012", "reconciliation")
        return obj(
            {
                "start": a_b.value,
                "lines": [
                    {"label": label, "sources": sources, "amount": a.value}
                    for _, label, sources, a in lines
                ],
                "end": end.value,
            },
            end,
        )

    return {
        "revenue": core["revenue"],
        "value_of_production": vop,
        "purchases": purchases,
        "materials_consumed": materials,
        "external_costs": external,
        "value_added": value_added,
        "personnel_costs": personnel,
        "ebitda": ebitda,
        "ebitda_definition": EBITDA_DEFINITION,
        "depreciation_and_amortisation": dna,
        "other_charges_below_ebitda": other_charges,
        "operating_profit": operating_profit,
        "investing_result": investing_result,
        "profit_before_financing_and_taxes": pbft,
        "non_recurring_result": nr_result,
        "financing_result": financing,
        "interest_and_other_financial_charges": interest,
        "profit_before_tax": pbt,
        "income_taxes": taxes,
        "net_income": net_income,
        "reconciliations": {
            "operating_profit": reconciliation(op_lines, operating_profit),
            "profit_before_financing_and_taxes": reconciliation(pbft_lines, pbft),
        },
    }


# ---------------------------------------------------------------- reclassified income statement
@at_50_digits
def reclassified_income_statement(inp: dict[str, Any], i: int) -> tuple[dict[str, Any], dict]:
    """Appendix B.2 totals as given (R5): EBITDA, profit before financing and taxes, profit
    before tax and net income derived when not given, and checked when given."""
    inc = get(inp["years"][i], "reclassified", "income_statement") or {}
    base = f"/years/{i}/reclassified/income_statement"
    c: dict[str, str] = {}

    def f(key: str) -> Amt:
        return leaf(inc, base, key)

    def given_or_derived(relation: str, key: str, derived: Amt) -> Amt:
        given = f(key)
        check(c, relation, given, derived)
        return given if given.known or not derived.known else derived

    # E1, E2: value added and operating profit derived from the fields of 2026-11-09.
    value_added = given_or_derived(
        "value_added_total",
        "value_added",
        total((1, f("value_of_production")), (-1, f("external_costs"))),
    )
    ebitda = given_or_derived(
        "ebitda_total", "ebitda", total((1, value_added), (-1, f("personnel_costs")))
    )
    operating_profit = given_or_derived(
        "operating_profit_total",
        "operating_profit",
        total(
            (1, ebitda),
            (-1, f("depreciation_and_amortisation")),
            (-1, f("other_charges_below_ebitda")),
        ),
    )
    pbft = given_or_derived(
        "profit_before_financing_and_taxes_total",
        "profit_before_financing_and_taxes",
        plus(operating_profit, f("investing_result")),
    )
    pbt = given_or_derived(
        "profit_before_tax_total",
        "profit_before_tax",
        plus(pbft, f("non_recurring_result"), f("financing_result")),
    )
    net = given_or_derived(
        "net_income_total", "net_income", total((1, pbt), (-1, f("income_taxes")))
    )
    statutory = Amt(None, frozenset())
    return {
        "revenue": f("revenue"),
        "value_of_production": f("value_of_production"),
        "purchases": f("purchases"),
        "materials_consumed": f("materials_consumed"),
        "external_costs": f("external_costs"),
        "value_added": value_added,
        "personnel_costs": f("personnel_costs"),
        "ebitda": ebitda,
        "ebitda_definition": EBITDA_DEFINITION,
        "depreciation_and_amortisation": f("depreciation_and_amortisation"),
        "other_charges_below_ebitda": f("other_charges_below_ebitda"),
        "operating_profit": operating_profit,
        "investing_result": f("investing_result"),
        "profit_before_financing_and_taxes": pbft,
        "non_recurring_result": f("non_recurring_result"),
        "financing_result": f("financing_result"),
        "interest_and_other_financial_charges": f("interest_and_other_financial_charges"),
        "profit_before_tax": pbt,
        "income_taxes": f("income_taxes"),
        "net_income": net,
        "reconciliations": {
            "operating_profit": obj(None, statutory),
            "profit_before_financing_and_taxes": obj(None, statutory),
        },
    }, c


def finish_income_statement(node: dict[str, Any], i: int) -> tuple[dict[str, Any], list]:
    """Amounts to values; every null output of the income statement listed (R6)."""
    nc: list[dict[str, Any]] = []

    def visit(n: Any, pointer: str) -> Any:
        if isinstance(n, Amt):
            if not n.known:
                nc.append({"output": pointer, "missing": sorted(n.missing)})
            return n.value
        if isinstance(n, Lst):
            if n.items is None:
                nc.append({"output": pointer, "missing": sorted(n.amount.missing)})
            return n.items
        if isinstance(n, dict):
            return {k: visit(v, f"{pointer}/{k}") for k, v in n.items()}
        return n

    value = visit(node, f"/years/{i}/income_statement")
    nc.sort(key=lambda e: e["output"])
    return value, nc


# ---------------------------------------------------------------- reclassified balance sheet
OWC_PARTS = [
    ("trade_receivables", 1),
    ("inventories", 1),
    ("other_operating_assets", 1),
    ("trade_payables", -1),
    ("other_operating_liabilities", -1),
]


@at_50_digits
def reclassified_balance_sheet(inp: dict[str, Any], i: int) -> dict[str, Any]:
    r = inp["years"][i]["reclassified"]
    bs = r.get("balance_sheet") or {}
    base = f"/years/{i}/reclassified/balance_sheet"
    c: dict[str, str] = {}

    def given_or_derived(given: Amt, derived: Amt) -> Amt:
        return given if given.known or not derived.known else derived

    owc_node = bs.get("operating_working_capital")
    owc_ptr = f"{base}/operating_working_capital"
    if owc_node is None:
        owc_given = owc_der = twc = lacking(owc_ptr)
    else:
        owc_given = (
            known(dec(owc_node["total"])) if "total" in owc_node else lacking(f"{owc_ptr}/total")
        )
        owc_der = total(
            *((s, part(bs, base, "operating_working_capital", p)) for p, s in OWC_PARTS)
        )
        twc = total(
            (1, part(bs, base, "operating_working_capital", "trade_receivables")),
            (1, part(bs, base, "operating_working_capital", "inventories")),
            (-1, part(bs, base, "operating_working_capital", "trade_payables")),
        )
    check(c, "operating_working_capital_total", owc_given, owc_der)
    owc = given_or_derived(owc_given, owc_der)
    ofa = leaf(bs, base, "operating_fixed_assets")
    op_prov = leaf(bs, base, "operating_provisions")
    ic_given = leaf(bs, base, "invested_capital")
    ic_der = total((1, ofa), (1, owc), (-1, op_prov))
    check(c, "invested_capital_total", ic_given, ic_der)
    ic = given_or_derived(ic_given, ic_der)

    def lines(key: str) -> Lst:
        if key not in bs:
            return Lst(None, lacking(f"{base}/{key}"))
        return Lst(bs[key], known(sum((dec(x["value"]) for x in bs[key]), ZERO)))

    noa = lines("non_operating_assets")
    tfi_given = leaf(bs, base, "total_funds_invested")
    tfi_der = plus(ic, noa.amount)
    check(c, "total_funds_invested_total", tfi_given, tfi_der)
    tfi = given_or_derived(tfi_given, tfi_der)
    equity = leaf(bs, base, "equity")
    qe = lines("quasi_equity_items")
    dl = lines("debt_like_items")
    nfp = bs.get("net_financial_position")
    nptr = f"{base}/net_financial_position"
    if nfp is None:
        nfp_total = cur = ncur = lacking(nptr)
        position = Lst(None, lacking(nptr))
    else:
        t_given = known(dec(nfp["total"])) if "total" in nfp else lacking(f"{nptr}/total")
        pos = nfp.get("position")
        if pos is None:
            position = Lst(None, lacking(f"{nptr}/position"))
            t_pos = lacking(f"{nptr}/position")
        else:
            t_pos = known(
                dec(pos["gross_debt"])
                - dec(pos["cash"])
                + sum((dec(x["amount"]) for x in pos.get("other_items", [])), ZERO)
            )
            position = Lst(pos, t_pos)
        check(c, "net_financial_position_total", t_given, t_pos)
        independent = given_or_derived(t_given, t_pos)
        cur = known(dec(nfp["current"])) if "current" in nfp else lacking(f"{nptr}/current")
        ncur = (
            known(dec(nfp["non_current"]))
            if "non_current" in nfp
            else lacking(f"{nptr}/non_current")
        )
        if independent.known:
            check(c, "net_financial_position_maturity", independent, plus(cur, ncur))
            nfp_total = independent
        else:
            # R5: the total is derived from its parts, so the maturity relation is not checked.
            c["net_financial_position_maturity"] = "not_checked"
            parts = plus(cur, ncur)
            nfp_total = parts if parts.known else independent
    ts_given = leaf(bs, base, "total_sources")
    ts_der = plus(equity, qe.amount, nfp_total, dl.amount)
    check(c, "total_sources_total", ts_given, ts_der)
    ts = given_or_derived(ts_given, ts_der)
    if tfi.known and ts.known:
        if tfi.value != ts.value:
            raise e026("funds_invested_equals_sources")
        c["funds_invested_equals_sources"] = "reconciled"
    else:
        c["funds_invested_equals_sources"] = "not_checked"
    excess = known(dec(position.items["cash"])) if position.items is not None else position.amount  # type: ignore[call-overload]
    # Outputs only the statutory form gives: missing with no input to name (R13, R15).
    statutory = Amt(None, frozenset())
    if owc_node is None:
        inventories = lacking(f"{owc_ptr}/inventories")
    else:
        inventories = part(bs, base, "operating_working_capital", "inventories")
    balance_sheet = {
        "operating": {
            "operating_fixed_assets": ofa,
            "operating_working_capital": owc,
            "trade_working_capital": twc,
            "operating_provisions": op_prov,
            "invested_capital": ic,
        },
        "non_operating_assets": noa,
        "non_operating_assets_total": noa.amount,
        "total_funds_invested": tfi,
        "sources": {
            "equity": equity,
            "quasi_equity_items": qe,
            "quasi_equity_items_total": qe.amount,
            "net_financial_position": {
                "position": position,
                "total": nfp_total,
                "current": cur,
                "non_current": ncur,
                "composition": Lst(None, statutory),
            },
            "debt_like_items": dl,
            "debt_like_items_total": dl.amount,
            "total_sources": ts,
        },
        "excess_cash": excess,
    }
    cnc = {
        "fixed_assets": statutory,
        "current_assets": statutory,
        "inventories": inventories,
        "cash": statutory,
        "total_assets": statutory,
        "current_liabilities": statutory,
        "non_current_liabilities": statutory,
    }
    return finish(balance_sheet, cnc, c, {}, i)


@at_50_digits
def reorganise(inp: dict[str, Any]) -> dict[str, Any]:
    """The value of `statements.reorganise` for the parts of the first and second pull
    requests, with `defaults_applied` of `meta` as (pointer, destination) pairs."""
    validate(inp)
    years = []
    for i, y in enumerate(inp["years"]):
        if "reclassified" in y:
            body = reclassified_balance_sheet(inp, i)
            income, is_checks = reclassified_income_statement(inp, i)
            body["checks"].update(is_checks)
            form = "reclassified"
        else:
            body = statutory_balance_sheet(inp, i)
            income = statutory_income_statement(inp, i)
            form = "statutory" if "statutory" in y else None
        value, nc = finish_income_statement(income, i)
        body["income_statement"] = value
        body["not_computed"]["income_statement"] = nc
        years.append({"form": form, **body})
    return {"years": years, "defaults_applied": defaults_applied(inp)}


# ---------------------------------------------------------------- output helpers
def pointers(node: Any, prefix: str = "") -> dict[str, str]:
    """Every scalar of `node` by JSON pointer, decimals as plain strings, null as "null"."""
    out: dict[str, str] = {}
    if isinstance(node, dict):
        for k, v in node.items():
            out.update(pointers(v, f"{prefix}/{k.replace('~', '~0').replace('/', '~1')}"))
    elif isinstance(node, list):
        for j, v in enumerate(node):
            out.update(pointers(v, f"{prefix}/{j}"))
    elif node is None:
        out[prefix] = "null"
    elif isinstance(node, D):
        out[prefix] = plain(node)
    else:
        out[prefix] = str(node)
    return out


def plain(x: D) -> str:
    s = format(x, "f")
    if "." in s:
        s = s.rstrip("0").rstrip(".")
    return "0" if s in ("-0", "") else s


def _self_test() -> None:
    assert consecutive((2023, 2, 28), (2024, 2, 29))
    assert consecutive((2024, 2, 29), (2025, 2, 28))
    assert not consecutive((2023, 2, 27), (2024, 2, 29))
    assert consecutive((2023, 12, 31), (2024, 12, 31))
    assert not consecutive((2024, 12, 31), (2023, 12, 31))
    assert not consecutive((2023, 6, 30), (2024, 7, 31))
    assert consecutive((2023, 4, 30), (2024, 4, 30))
    assert not consecutive((2023, 4, 29), (2024, 4, 30))
    a = plus(known(D(1)), lacking("/x"), lacking("/y"))
    assert a.missing == {"/x", "/y"}
    assert plus(known(D("0.1")), known(D("0.2"))).value == D("0.3")
    try:
        dec(0.1)
    except TypeError:
        pass
    else:
        raise AssertionError("a float must be refused")


if __name__ == "__main__":
    _self_test()
    if len(sys.argv) > 1:
        with open(sys.argv[1], encoding="utf-8") as f:
            case = json.load(f)
        print(json.dumps(pointers(reorganise(case)), indent=1))
    else:
        print("reference_statements: self-tests passed")
