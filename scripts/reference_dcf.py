"""Independent reference for SPEC-0001 (shared DCF engine, `dcf.fcff` version 3).

Written from the formulas of the spec, in Python's `decimal` module at 50 digits in a local
context (importing this module never changes the caller's precision). It shares no code with
the Rust engine and computes differently where it can:

- discount factors are reciprocals of the cumulated growth factor Π (1 + r_i), not a chain of
  divisions; mid-period factors divide by the square root of (1 + r_k) taken at 50 digits;
- the linear decline of growth uses g_from + (g_to - g_from) x k / m directly (Decision 4);
- every terminal technique is restated here from SPEC-0006 (Gordon, H-model, value driver,
  exit multiple, normalised income at the terminal rate, liquidation); the limited horizon is
  summed flow by flow, not computed with the annuity closed form that the engine reuses from
  `tvm.annuity`.

It takes the version 3 input as a JSON-like dictionary (amounts and rates as strings) and
returns the outputs that the golden cases check, keyed by their JSON Pointer in the result.

Usage:

    uv run python scripts/reference_dcf.py

prints the expected values of the reference-sourced golden cases in `tests/golden/dcf.fcff/`.
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
        assert terms["first_flow"] == "last_flow_grown"
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
    raise ValueError(f"technique not restated in the reference: {technique}")


@at_50_digits
def dcf_fcff(inp: Mapping[str, Any]) -> dict[str, D]:
    """Expected outputs of `dcf.fcff` version 3, keyed by JSON Pointer into `value`."""
    rows = project(inp["stages"], inp.get("base_flow"))
    n = len(rows)
    if "wacc" in inp:
        rates = [_d(inp["wacc"])] * n
        terminal_rate = _d(inp["wacc"])
    else:
        rates = [_d(r) for r in inp["wacc_per_period"]]
        terminal_rate = _d(inp["terminal_wacc"])
    assert len(rates) == n
    mid = inp["timing"] == "mid_period"

    out: dict[str, D] = {}
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

    technique, terms = next(iter(inp["terminal"].items()))
    tv, technique_out = terminal_value(technique, terms, rows[-1]["flow"], terminal_rate)
    out.update({f"/terminal/output/{k}": v for k, v in technique_out.items()})
    df_n = 1 / growth_factor
    at_mid = mid and inp["terminal_discount_time"] == "mid_period"
    df_tv = df_n * (1 + rates[-1]).sqrt() if at_mid else df_n
    pv_tv = tv * df_tv

    value_of_operations = pv_explicit + pv_tv
    non_operating = sum((_d(a["value"]) for a in inp.get("non_operating_assets", [])), D(0))
    enterprise_value = value_of_operations + non_operating
    nfp = D(0)
    if "net_financial_position" in inp:
        p = inp["net_financial_position"]
        nfp = _d(p["gross_debt"]) - _d(p["cash"])
        nfp += sum((_d(i["amount"]) for i in p.get("other_items", [])), D(0))
    out.update(
        {
            "/pv_explicit_period": pv_explicit,
            "/terminal/terminal_value": tv,
            "/terminal/discount_time": D(n) - (D("0.5") if at_mid else 0),
            "/terminal/discount_factor": df_tv,
            "/terminal/present_value": pv_tv,
            "/value_of_operations": value_of_operations,
            "/enterprise_value": enterprise_value,
            "/equity_value": enterprise_value - nfp,
            "/terminal_value_weight": pv_tv / value_of_operations,
        }
    )
    return out


def main() -> None:
    import yaml

    root = Path(__file__).resolve().parents[1] / "tests" / "golden" / "dcf.fcff"
    for path in sorted(root.glob("*.yaml")):
        case = yaml.safe_load(path.read_text(encoding="utf-8"))
        if "expect" not in case or not str(case.get("source", "")).startswith(REFERENCE):
            continue
        print(path.name)
        for pointer, value in dcf_fcff(case["input"]).items():
            if pointer in case["expect"]:
                print(f"  {pointer}: {value}")


if __name__ == "__main__":
    sys.exit(main())
