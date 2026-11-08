"""Independent reference for SPEC-0001 (shared DCF engine, `dcf.fcff` version 3 and
`dcf.fcfe` version 1).

Written from the formulas of the spec, in Python's `decimal` module at 50 digits in a local
context (importing this module never changes the caller's precision). It shares no code with
the Rust engine and computes differently where it can:

- discount factors are reciprocals of the cumulated growth factor Π (1 + r_i), not a chain of
  divisions; mid-period factors divide by the square root of (1 + r_k) taken at 50 digits;
- the linear decline of growth uses g_from + (g_to - g_from) x k / m directly (Decision 4);
- nominal and real terms (R16, R17): when the terms of flows and rates differ, every rate is
  restated as (1 + r) / (1 + i) - 1 (to real) or (1 + r) x (1 + i) - 1 (to nominal), written
  here, not through `rates.real_nominal`; `inflated_input` restates real flows in nominal terms
  instead, the independent route of AC9;
- every terminal technique is restated here from SPEC-0006 (Gordon, H-model, value driver,
  exit multiple, normalised income at the terminal rate, liquidation); the limited horizon is
  summed flow by flow, not computed with the annuity closed form that the engine reuses from
  `tvm.annuity`;
- the documented defaults of Decision 24 (projection `given` and periods from the flows of a
  stage, nominal terms, end-of-period timing, no terminal value) are filled in by
  `with_defaults`, written from the spec, not from the engine; the terminal options `given`
  (an amount at N) and `none` are restated in `terminal_value`.

It takes the version 3 input as a JSON-like dictionary (amounts and rates as strings) and
returns the outputs that the golden cases check, keyed by their JSON Pointer in the result.

Usage:

    uv run python scripts/reference_dcf.py

prints the expected values of the reference-sourced golden cases in `tests/golden/dcf.fcff/`
and `tests/golden/dcf.fcfe/`.
"""

from __future__ import annotations

import sys
from collections.abc import Callable, Mapping
from decimal import Context, localcontext
from decimal import Decimal as D
from functools import wraps
from pathlib import Path
from typing import Any, TypeVar

T = TypeVar("T", bound=Callable[..., Any])

CONTEXT = Context(prec=50)
# Golden cases whose expected values come from this script start their `source` with this.
REFERENCE = "Independent 50-digit reference, scripts/reference_dcf.py"


def at_50_digits(func: T) -> T:
    @wraps(func)
    def wrapper(*args: Any, **kwargs: Any) -> Any:
        with localcontext(CONTEXT):
            return func(*args, **kwargs)

    return wrapper  # type: ignore[return-value]


def _d(v: Any) -> D:
    return D(str(v))


def with_defaults(inp: Mapping[str, Any]) -> dict[str, Any]:
    """SPEC-0001 Decision 24: the input with every documented default filled in. A field given
    is kept as given."""
    out = dict(inp)
    stages = []
    for stage in inp["stages"]:
        stage = dict(stage)
        amounts = stage.get("flows", stage.get("lines"))
        if "projection" not in stage and amounts is not None:
            stage["projection"] = "given"
        if "periods" not in stage and stage.get("projection") == "given":
            stage["periods"] = len(amounts)
        stages.append(stage)
    out["stages"] = stages
    out.setdefault("cash_flow_terms", "nominal")
    out.setdefault("discount_rate_terms", "nominal")
    out.setdefault("timing", "end_of_period")
    out.setdefault("terminal", {"none": {}})
    return out


@at_50_digits
def project(stages: list[Mapping[str, Any]], base_flow: Any = None) -> list[dict[str, Any]]:
    """R6 to R9: one row per period with the stage (1-based), the growth applied and the flow."""
    rows: list[dict[str, Any]] = []
    previous = None if base_flow is None else _d(base_flow)
    for s, stage in enumerate(stages, start=1):
        m = int(stage["periods"])
        kind = stage["projection"]
        for k in range(1, m + 1):
            if kind == "given":
                growth = None
                flow = _d(stage["flows"][k - 1])
            else:
                if kind == "constant_growth":
                    growth = _d(stage["growth"])
                else:  # linear_decline
                    g_from, g_to = _d(stage["from_growth"]), _d(stage["to_growth"])
                    growth = g_from + (g_to - g_from) * D(k) / D(m)
                assert previous is not None
                flow = previous * (1 + growth)
            rows.append({"stage": s, "projection": kind, "growth": growth, "flow": flow})
            previous = flow
    return rows


@at_50_digits
def terminal_value(
    technique: str, terms: Mapping[str, Any], last_flow: D, r: D
) -> tuple[D, dict[str, D]]:
    """R19, R20: the terminal value at N of each technique (SPEC-0006 R1 to R5, R7) at the
    terminal rate r, with the outputs of the technique that the golden cases check."""
    if technique == "tv.gordon":
        g = _d(terms["growth"])
        assert g < r
        if terms["first_flow"] == "last_flow_grown":
            next_flow = last_flow * (1 + g)
        else:
            next_flow = _d(terms["normalised_next_flow"])
        return next_flow / (r - g), {"next_flow": next_flow}
    if technique == "tv.h_model":
        # Decision 15: a normalised base flow at N replaces the last explicit flow.
        if terms["first_flow"] == "normalised_base_flow":
            last_flow = _d(terms["normalised_base_flow"])
        gs, gl = _d(terms["short_term_growth"]), _d(terms["long_term_growth"])
        h = _d(terms["half_life"])
        stable = last_flow * (1 + gl) / (r - gl)
        extra = last_flow * h * (gs - gl) / (r - gl)
        return stable + extra, {"stable_growth_value": stable, "extra_growth_value": extra}
    if technique == "tv.value_driver":
        nopat, g, ronic = _d(terms["nopat_next"]), _d(terms["growth"]), _d(terms["ronic"])
        reinvestment = g / ronic
        fcf = nopat * (1 - reinvestment)
        tv = nopat / r if ronic == r else fcf / (r - g)
        return tv, {"reinvestment_rate": reinvestment, "next_free_cash_flow": fcf}
    if technique == "tv.exit_multiple":
        tv = _d(terms["multiple"]) * _d(terms["metric_value"])
        implied: dict[str, D] = {}
        if "next_flow" in terms:
            implied["implied_growth"] = r - _d(terms["next_flow"]) / tv
        return tv, implied
    if technique == "tv.limited_horizon":
        flow, n = _d(terms["flow"]), int(terms["periods"])
        g, rv = _d(terms["growth"]), _d(terms["residual_value"])
        # Summed flow by flow: CF_{N+j} = flow x (1 + g)^(j-1), discounted to N at (1 + r)^j.
        flows_value = sum((flow * (1 + g) ** (j - 1) / (1 + r) ** j for j in range(1, n + 1)), D(0))
        rv_at_n = rv / (1 + r) ** n
        return flows_value + rv_at_n, {
            "flows_value": flows_value,
            "residual_value_discounted": rv_at_n,
        }
    if technique == "tv.normalised_income":
        # Capitalised at the terminal rate (R19, Decision 13): R / (r - g).
        income, g = _d(terms["income"]), _d(terms["growth"])
        assert r > 0 and g < r
        return income / (r - g), {}
    if technique == "tv.liquidation":

        def total(group: str) -> D:
            return sum((_d(line["value"]) for line in terms[group]), D(0))

        tv = (
            total("assets")
            - total("liabilities")
            - total("liquidation_costs")
            - _d(terms["tax_on_gains"])
        )
        return tv, {"total_assets": total("assets")}
    if technique == "none":  # Decision 24: no terminal value, the given flows only
        return D(0), {}
    if technique == "given":  # Decision 24: the analyst's amount at N, any sign
        return _d(terms["value"]), {}
    raise ValueError(f"technique not restated in the reference: {technique}")


def perpetual_growth(technique: str, terms: Mapping[str, Any]) -> D | None:
    """The perpetual growth stated by a terminal technique (R11, R17), or None."""
    if technique in ("tv.gordon", "tv.value_driver", "tv.normalised_income"):
        return _d(terms["growth"])
    if technique == "tv.h_model":
        return _d(terms["long_term_growth"])
    return None


@at_50_digits
def inflation_schedule(inp: Mapping[str, Any], n: int) -> tuple[list[D], D | None] | None:
    """Expected inflation of each explicit period and of the terminal phase, or None."""
    if "inflation" in inp:
        i = _d(inp["inflation"])
        return [i] * n, i
    if "inflation_per_period" in inp:
        per = [_d(i) for i in inp["inflation_per_period"]]
        assert len(per) == n
        # Decision 24: no terminal inflation when no terminal rate is used.
        return per, _d(inp["terminal_inflation"]) if "terminal_inflation" in inp else None
    return None


@at_50_digits
def restate(rate: D, inflation: D, to: str) -> D:
    """Exact Fisher relation (1 + nominal) = (1 + real) x (1 + inflation), solved for `to`."""
    if to == "real":
        return (1 + rate) / (1 + inflation) - 1
    return (1 + rate) * (1 + inflation) - 1


@at_50_digits
def discounted(
    inp: Mapping[str, Any], single: str, per_period: str, terminal: str
) -> tuple[dict[str, D], D, D]:
    """The part shared by both bases: the explicit periods and the terminal value, discounted
    at the rates named by the method's fields (`wacc`, or `cost_of_equity`). Returns the
    outputs keyed by JSON Pointer, the present value of the explicit flows and the present
    value of the terminal value. Outputs that are `null` (Decision 24: no terminal rate when
    none is used) are left out; the golden cases state them."""
    inp = with_defaults(inp)
    rows = project(inp["stages"], inp.get("base_flow"))
    n = len(rows)
    mid = inp["timing"] == "mid_period"
    technique, terms = next(iter(inp["terminal"].items()))
    at_mid = mid and inp.get("terminal_discount_time") == "mid_period"
    # Decision 24: the terminal rate is used by a technique, or by the half-period shift of a
    # given terminal value; never with no terminal value.
    uses_terminal_rate = technique.startswith("tv.") or (technique == "given" and at_mid)
    if single in inp:
        given = [_d(inp[single])] * n
        terminal_given = _d(inp[single]) if uses_terminal_rate else None
    else:
        given = [_d(r) for r in inp[per_period]]
        terminal_given = _d(inp[terminal]) if uses_terminal_rate else None
    assert len(given) == n

    out: dict[str, D] = {}
    # R16: rates restated in the terms of the flows when the terms differ; flows never.
    flow_terms = inp["cash_flow_terms"]
    schedule = inflation_schedule(inp, n)
    convert = flow_terms != inp["discount_rate_terms"]
    if convert:
        assert schedule is not None, "different terms need inflation (R15)"
        per_period, terminal_inflation = schedule
        rates = [restate(r, i, flow_terms) for r, i in zip(given, per_period, strict=True)]
        terminal_rate = None
        if terminal_given is not None:
            assert terminal_inflation is not None
            terminal_rate = restate(terminal_given, terminal_inflation, flow_terms)
        for k, i in enumerate(per_period):
            out[f"/periods/{k}/inflation"] = i
    else:
        rates, terminal_rate = given, terminal_given
    for k, r in enumerate(rates):
        out[f"/periods/{k}/discount_rate"] = given[k]
        out[f"/periods/{k}/rate_used"] = r
    if terminal_given is not None and terminal_rate is not None:
        out["/terms/terminal_rate_given"] = terminal_given
        out["/terms/terminal_rate_used"] = terminal_rate
        out["/terminal/rate"] = terminal_rate
    # R17: the perpetual growth restated in the other terms with the terminal inflation.
    g = perpetual_growth(technique, terms)
    if g is not None:
        out["/terms/perpetual_growth"] = g
    if schedule is not None and schedule[1] is not None and uses_terminal_rate:
        out["/terms/terminal_inflation"] = schedule[1]
        if g is not None:
            other = "real" if flow_terms == "nominal" else "nominal"
            out["/terms/perpetual_growth_other_terms"] = restate(g, schedule[1], other)
    growth_factor = D(1)  # Π (1 + r_i) up to the end of the previous period
    pv_explicit = D(0)
    for k, (row, r) in enumerate(zip(rows, rates, strict=True)):
        # mid period: half a period at the rate of the period, (1 + r_k)^0.5
        df = 1 / (growth_factor * ((1 + r).sqrt() if mid else 1 + r))
        growth_factor *= 1 + r
        pv = row["flow"] * df
        pv_explicit += pv
        out[f"/periods/{k}/flow"] = row["flow"]
        if row["growth"] is not None:
            out[f"/periods/{k}/growth"] = row["growth"]
        out[f"/periods/{k}/discount_factor"] = df
        out[f"/periods/{k}/present_value"] = pv

    tv, technique_out = terminal_value(
        technique, terms, rows[-1]["flow"], terminal_rate if terminal_rate is not None else D(0)
    )
    out.update({f"/terminal/output/{k}": v for k, v in technique_out.items()})
    df_n = 1 / growth_factor
    # Decision 16: the half-period shift at N - 0.5 uses the terminal rate.
    if at_mid:
        assert terminal_rate is not None
        df_tv = df_n * (1 + terminal_rate).sqrt()
    else:
        df_tv = df_n
    pv_tv = tv * df_tv
    out.update(
        {
            "/pv_explicit_period": pv_explicit,
            "/terminal/terminal_value": tv,
            "/terminal/discount_time": D(n) - (D("0.5") if at_mid else 0),
            "/terminal/discount_factor": df_tv,
            "/terminal/present_value": pv_tv,
        }
    )
    return out, pv_explicit, pv_tv


@at_50_digits
def dcf_fcff(inp: Mapping[str, Any]) -> dict[str, D]:
    """Expected outputs of `dcf.fcff` version 3, keyed by JSON Pointer into `value`."""
    out, pv_explicit, pv_tv = discounted(inp, "wacc", "wacc_per_period", "terminal_wacc")
    value_of_operations = pv_explicit + pv_tv
    non_operating = sum((_d(a["value"]) for a in inp.get("non_operating_assets", [])), D(0))
    enterprise_value = value_of_operations + non_operating
    nfp = D(0)
    if "net_financial_position" in inp:
        p = inp["net_financial_position"]
        nfp = _d(p["gross_debt"]) - _d(p["cash"])
        nfp += sum((_d(i["amount"]) for i in p.get("other_items", [])), D(0))
    # R23, R25 (#44): debt-like items and non-controlling interests, each zero or more,
    # subtracted from enterprise value after the net financial position.
    debt_like = sum((_d(c["value"]) for c in inp.get("debt_like_items", [])), D(0))
    non_controlling = sum((_d(c["value"]) for c in inp.get("non_controlling_interests", [])), D(0))
    if "net_financial_position" in inp:
        out["/bridge/net_financial_position"] = nfp
    for kind in ("non_operating_assets", "debt_like_items", "non_controlling_interests"):
        for i, item in enumerate(inp.get(kind, [])):
            out[f"/bridge/{kind}/{i}/value"] = _d(item["value"])
    out.update(
        {
            "/value_of_operations": value_of_operations,
            "/enterprise_value": enterprise_value,
            "/equity_value": enterprise_value - nfp - debt_like - non_controlling,
            "/bridge/non_operating_assets_total": non_operating,
            "/bridge/debt_like_items_total": debt_like,
            "/bridge/non_controlling_interests_total": non_controlling,
            "/terminal_value_weight": pv_tv / value_of_operations,
        }
    )
    return out


@at_50_digits
def dcf_fcfe(inp: Mapping[str, Any]) -> dict[str, D]:
    """Expected outputs of `dcf.fcfe` version 1, keyed by JSON Pointer into `value` (SPEC-0001
    R22, R24, R25): FCFE discounted at the cost of equity; equity value = present value of the
    explicit FCFE + present value of the terminal value + non-operating assets - debt-like
    items - non-controlling interests. No net financial position: debt is in the flows."""
    assert "net_financial_position" not in inp, "refused on the equity basis (R24)"
    out, pv_explicit, pv_tv = discounted(
        inp, "cost_of_equity", "cost_of_equity_per_period", "terminal_cost_of_equity"
    )
    start = pv_explicit + pv_tv

    def total(kind: str) -> D:
        return sum((_d(item["value"]) for item in inp.get(kind, [])), D(0))

    non_operating = total("non_operating_assets")
    debt_like = total("debt_like_items")
    non_controlling = total("non_controlling_interests")
    for kind in ("non_operating_assets", "debt_like_items", "non_controlling_interests"):
        for i, item in enumerate(inp.get(kind, [])):
            out[f"/bridge/{kind}/{i}/value"] = _d(item["value"])
    equity_value = start + non_operating - debt_like - non_controlling
    out.update(
        {
            "/bridge/pv_explicit_and_terminal": start,
            "/bridge/non_operating_assets_total": non_operating,
            "/bridge/debt_like_items_total": debt_like,
            "/bridge/non_controlling_interests_total": non_controlling,
            "/bridge/equity_value": equity_value,
            "/equity_value": equity_value,
        }
    )
    if start != 0:
        # R22: the weight is taken on the present value of the flows and the terminal value.
        out["/terminal_value_weight"] = pv_tv / start
    return out


@at_50_digits
def inflated_input(inp: Mapping[str, Any]) -> dict[str, Any]:
    """AC9, the independent route: real flows restated in nominal terms instead of the rates.

    Takes a `dcf.fcff` input with real flows in `given` stages, nominal rates, inflation and a
    `tv.gordon` terminal value on the last flow grown, and returns the same valuation in
    nominal terms on both sides: CF_k x Π_(i <= k) (1 + i_i), and the perpetual growth
    (1 + g)(1 + i_T) - 1. Under end-of-period timing both inputs have the same value.
    """
    assert inp["cash_flow_terms"] == "real" and inp["discount_rate_terms"] == "nominal"
    rows = project(inp["stages"], inp.get("base_flow"))
    schedule = inflation_schedule(inp, len(rows))
    assert schedule is not None
    per_period, terminal_inflation = schedule
    index = D(1)
    flows = []
    for row, i in zip(rows, per_period, strict=True):
        index *= 1 + i
        flows.append(str(row["flow"] * index))
    gordon = dict(inp["terminal"]["tv.gordon"])
    assert gordon["first_flow"] == "last_flow_grown"
    gordon["growth"] = str(restate(_d(gordon["growth"]), terminal_inflation, "nominal"))
    out = {
        k: v
        for k, v in inp.items()
        if k
        not in ("stages", "base_flow", "inflation", "inflation_per_period", "terminal_inflation")
    }
    out.update(
        {
            "stages": [{"periods": len(flows), "projection": "given", "flows": flows}],
            "cash_flow_terms": "nominal",
            "terminal": {"tv.gordon": gordon},
        }
    )
    return out


def main() -> None:
    import yaml

    golden = Path(__file__).resolve().parents[1] / "tests" / "golden"
    for method, valuation in (("dcf.fcff", dcf_fcff), ("dcf.fcfe", dcf_fcfe)):
        for path in sorted((golden / method).glob("*.yaml")):
            case = yaml.safe_load(path.read_text(encoding="utf-8"))
            if "expect" not in case or not str(case.get("source", "")).startswith(REFERENCE):
                continue
            print(f"{method}/{path.name}")
            for pointer, value in valuation(case["input"]).items():
                if pointer in case["expect"]:
                    print(f"  {pointer}: {value}")


if __name__ == "__main__":
    sys.exit(main())
