"""The equity DCF `dcf.fcfe` (SPEC-0001, issue #165) against the independent reference
scripts/reference_dcf.py, and its consistency with the enterprise DCF `dcf.fcff`.

The golden cases pin single points; these tests check that the golden values are those of the
reference, that the engine and the 50-digit reference agree across stages, rate forms, timings
and bridges, and the consistency property of the issue: with no debt and no net borrowing, the
FCFE equals the FCFF and `dcf.fcfe` gives the `dcf.fcff` equity value at the same rate.
"""

from __future__ import annotations

import importlib.util
import random
import sys
from decimal import Context, localcontext
from decimal import Decimal as D
from itertools import product
from pathlib import Path
from typing import Any

import pytest
import yaml

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]


def _load(name: str, file: str) -> Any:
    spec = importlib.util.spec_from_file_location(name, ROOT / "scripts" / file)
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    sys.modules[name] = module  # dataclasses resolve their module by name
    spec.loader.exec_module(module)
    return module


ref = _load("ref_dcf_fcfe", "reference_dcf.py")
ref_fcf = _load("ref_fcf_fcfe", "reference_free_cash_flows.py")

REFERENCE = "Independent 50-digit reference, scripts/reference_dcf.py"
COMPOSED = "Independent 50-digit references composed"
GOLDEN = sorted((ROOT / "tests/golden/dcf.fcfe").glob("*.yaml"))
# Relative agreement of engine and reference, as for dcf.fcff (test_reference_dcf.py).
REL = {"end_of_period": D("1e-22"), "mid_period": D("1e-17")}


def pointer(value: Any, path: str) -> Any:
    for part in path.strip("/").split("/"):
        value = value[int(part)] if isinstance(value, list) else value[part]
    return value


def is_decimal(value: Any) -> bool:
    try:
        D(str(value))
    except ArithmeticError:
        return False
    return True


def run(method: str, payload: dict[str, Any]) -> dict[str, Any]:
    return fm.run_json(method, payload)["value"]


def close(got: str | D, want: D, timing: str) -> bool:
    return abs(D(str(got)) - want) <= REL[timing] * max(D(1), abs(want))


def reference_values(case: dict[str, Any]) -> dict[str, D]:
    """The reference values of a golden case: built-up lines first when the source composes
    the two references."""
    source = str(case.get("source", ""))
    inp = case["input"]
    if source.startswith(COMPOSED):
        inp = ref_fcf.equity_stages_with_flows(inp)
    return ref.dcf_fcfe(inp)


def sourced_cases() -> list[Path]:
    out = []
    for path in GOLDEN:
        case = yaml.safe_load(path.read_text(encoding="utf-8"))
        source = str(case.get("source", ""))
        if "expect" in case and (source.startswith(REFERENCE) or source.startswith(COMPOSED)):
            out.append(path)
    return out


def test_there_are_reference_golden_cases() -> None:
    assert len(sourced_cases()) >= 5


@pytest.mark.parametrize("path", sourced_cases(), ids=lambda p: p.stem)
def test_golden_values_are_the_reference_values(path: Path) -> None:
    """Each expected value of a reference-sourced golden case is the reference value, rounded
    to the digits written in the file."""
    case = yaml.safe_load(path.read_text(encoding="utf-8"))
    values = reference_values(case)
    for key, expected in case["expect"].items():
        if not is_decimal(expected):
            continue  # names and nulls are compared exactly by the golden runner
        want = D(str(expected))
        got = values[key]
        assert abs(got - want) <= D(1).scaleb(want.as_tuple().exponent), key  # type: ignore[arg-type]


STAGES = [
    [{"periods": 2, "projection": "given", "flows": ["100", "-20"]}],
    [
        {"periods": 2, "projection": "given", "flows": ["50", "60"]},
        {"periods": 3, "projection": "constant_growth", "growth": "0.07"},
    ],
    [
        {"periods": 1, "projection": "given", "flows": ["75.5"]},
        {"periods": 4, "projection": "linear_decline", "from_growth": "0.15", "to_growth": "0.02"},
        {"periods": 2, "projection": "constant_growth", "growth": "0.02"},
    ],
]
RATES = ["0.085", "0.115", "0.16"]
TIMINGS = [("end_of_period", None), ("mid_period", "horizon"), ("mid_period", "mid_period")]
BRIDGE = {
    "non_operating_assets": [{"label": "Excess cash", "value": "35"}],
    "debt_like_items": [
        {"label": "Pension deficit", "value": "12.5", "measurement_basis": "present_value"}
    ],
    "non_controlling_interests": [
        {"label": "Minority", "value": "20", "measurement_basis": "book_value"}
    ],
}


@pytest.mark.parametrize(
    ("stages", "rate", "timing"), list(product(STAGES, RATES, TIMINGS)), ids=str
)
def test_engine_matches_reference(
    stages: list[dict[str, Any]], rate: str, timing: tuple[str, str | None]
) -> None:
    payload: dict[str, Any] = {
        "stages": stages,
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "cost_of_equity": rate,
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": "0.015", "first_flow": "last_flow_grown"}},
        **BRIDGE,
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got, want = run("dcf.fcfe", payload), ref.dcf_fcfe(payload)
    for key, value in want.items():
        assert close(pointer(got, key), value, timing[0]), key


@pytest.mark.parametrize("timing", TIMINGS, ids=str)
def test_engine_matches_reference_with_cost_of_equity_per_period(
    timing: tuple[str, str | None],
) -> None:
    payload: dict[str, Any] = {
        "stages": [
            {"periods": 3, "projection": "constant_growth", "growth": "0.09"},
            {
                "periods": 2,
                "projection": "linear_decline",
                "from_growth": "0.09",
                "to_growth": "0.03",
            },
        ],
        "base_flow": "40",
        "cash_flow_terms": "real",
        "discount_rate_terms": "nominal",
        "cost_of_equity_per_period": ["0.14", "0.135", "0.13", "0.12", "0.11"],
        "terminal_cost_of_equity": "0.105",
        "inflation_per_period": ["0.03", "0.028", "0.025", "0.022", "0.02"],
        "terminal_inflation": "0.02",
        "timing": timing[0],
        "terminal": {
            "tv.h_model": {
                "short_term_growth": "0.05",
                "long_term_growth": "0.01",
                "half_life": "3",
                "first_flow": "last_flow_grown",
            }
        },
        **BRIDGE,
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got, want = run("dcf.fcfe", payload), ref.dcf_fcfe(payload)
    for key, value in want.items():
        assert close(pointer(got, key), value, timing[0]), key


def random_claims(rng: random.Random) -> dict[str, Any]:
    """Zero to three lines of each kind of the equity bridge, amounts with four decimals."""

    def amount() -> str:
        return str(D(rng.randint(0, 50_000_000)).scaleb(-4))

    def claims(kind: str) -> list[dict[str, str]]:
        return [
            {"label": f"{kind} {i}", "value": amount(), "measurement_basis": "book_value"}
            for i in range(rng.randint(0, 3))
        ]

    bridge = {
        "non_operating_assets": [
            {"label": f"asset {i}", "value": amount()} for i in range(rng.randint(0, 3))
        ],
        "debt_like_items": claims("debt-like item"),
        "non_controlling_interests": claims("non-controlling interest"),
    }
    return {k: v for k, v in bridge.items() if v}


@pytest.mark.parametrize("seed", range(12))
def test_consistency_with_fcff_without_debt(seed: int) -> None:
    """Issue #165: with no debt, the FCFE are the FCFF; `dcf.fcfe` at the cost of equity then
    equals the `dcf.fcff` equity value at a WACC equal to that rate, with the same bridge items
    and no net financial position: every period, the terminal value and the equity value are
    identical, to the last digit (R1, R22 to R24)."""
    rng = random.Random(seed)
    timing = TIMINGS[seed % len(TIMINGS)]
    rate = RATES[seed % len(RATES)]
    common: dict[str, Any] = {
        "stages": STAGES[seed % len(STAGES)],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": "0.015", "first_flow": "last_flow_grown"}},
        **random_claims(rng),
    }
    if timing[1]:
        common["terminal_discount_time"] = timing[1]
    equity = run("dcf.fcfe", {**common, "cost_of_equity": rate})
    firm = run("dcf.fcff", {**common, "wacc": rate})
    assert D(equity["equity_value"]) == D(firm["equity_value"])
    assert D(equity["bridge"]["pv_explicit_and_terminal"]) == D(firm["value_of_operations"])
    assert equity["periods"] == firm["periods"]
    assert equity["terminal"] == firm["terminal"]
    assert D(equity["terminal_value_weight"]) == D(firm["terminal_value_weight"])
    assert equity["basis"] == "equity" and firm["basis"] == "enterprise"
    assert "value_of_operations" not in equity and "enterprise_value" not in equity


LINES = [
    {
        "ebit": "640",
        "depreciation_and_amortisation": "120",
        "capital_expenditure": "150",
        "change_in_net_working_capital": "18",
    },
    {
        "ebit": "-35",
        "depreciation_and_amortisation": "125",
        "capital_expenditure": "160",
        "change_in_net_working_capital": "-6",
    },
    {
        "ebit": "702.25",
        "depreciation_and_amortisation": "131",
        "capital_expenditure": "149.5",
        "change_in_net_working_capital": "21.4",
        "other_adjustments": [{"label": "Disposal", "value": "12"}],
    },
]


@pytest.mark.parametrize("negative_ebit", ["credit", "zero"])
def test_consistency_with_fcff_on_build_up_lines_without_debt(negative_ebit: str) -> None:
    """Issue #165 with AC2: lines with zero interest and zero net borrowing give FCFE = FCFF,
    so `dcf.fcfe` on those lines equals `dcf.fcff` on the same lines without the financing
    lines, to the last digit."""
    unlevered = [{**line, "interest_expense": "0", "net_borrowing": "0"} for line in LINES]
    common: dict[str, Any] = {
        "notional_tax_rate": "0.24",
        "negative_ebit": negative_ebit,
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "timing": "end_of_period",
        "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
    }
    equity = run(
        "dcf.fcfe",
        {
            **common,
            "stages": [{"periods": 3, "projection": "given", "lines": unlevered}],
            "cost_of_equity": "0.095",
        },
    )
    firm = run(
        "dcf.fcff",
        {
            **common,
            "stages": [{"periods": 3, "projection": "given", "lines": LINES}],
            "wacc": "0.095",
        },
    )
    assert D(equity["equity_value"]) == D(firm["equity_value"])
    assert [D(r["flow"]) for r in equity["periods"]] == [D(r["flow"]) for r in firm["periods"]]


@pytest.mark.parametrize("seed", range(12))
def test_equity_value_identity(seed: int) -> None:
    """R24: equity value = present value of the explicit FCFE + present value of the terminal
    value + Σ non-operating assets - Σ debt-like items - Σ non-controlling interests, within
    the rounding of the engine's 28 significant digits; the totals are exact."""
    rng = random.Random(500 + seed)
    bridge = random_claims(rng)
    got = run(
        "dcf.fcfe",
        {
            "stages": STAGES[1],
            "cash_flow_terms": "nominal",
            "discount_rate_terms": "nominal",
            "cost_of_equity": "0.12",
            "timing": "mid_period",
            "terminal_discount_time": "mid_period",
            "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
            **bridge,
        },
    )
    b = got["bridge"]
    with localcontext(Context(prec=60)):

        def total(items: list[dict[str, Any]]) -> D:
            return sum((D(i["value"]) for i in items), D(0))

        start = D(b["pv_explicit_and_terminal"])
        ulp = D("1e-27") * max(D(1), abs(start))
        assert (
            abs(start - D(got["pv_explicit_period"]) - D(got["terminal"]["present_value"])) <= ulp
        )
        assert D(b["non_operating_assets_total"]) == total(b["non_operating_assets"])
        assert D(b["debt_like_items_total"]) == total(b["debt_like_items"])
        assert D(b["non_controlling_interests_total"]) == total(b["non_controlling_interests"])
        equity = (
            start
            + D(b["non_operating_assets_total"])
            - D(b["debt_like_items_total"])
            - D(b["non_controlling_interests_total"])
        )
        assert abs(D(got["equity_value"]) - equity) <= 3 * ulp
        assert D(b["equity_value"]) == D(got["equity_value"])


def test_net_financial_position_is_refused_with_the_fix() -> None:
    """R24, AC13: a net financial position on the equity basis is FM-E019, naming the field and
    the fix."""
    with pytest.raises(fm.FairmarkError) as err:
        fm.dcf.fcfe(
            [fm.dcf.given([100, 110])],
            cash_flow_terms="nominal",
            discount_rate_terms="nominal",
            cost_of_equity="0.11",
            timing="end_of_period",
            terminal=fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
            net_financial_position={"gross_debt": 500, "cash": 80},
        )
    assert err.value.code == "FM-E019"
    assert "/net_financial_position" in str(err.value)


def test_python_helper_matches_the_registry() -> None:
    """The helper only builds the input: its result equals the registry run."""
    payload: dict[str, Any] = {
        "stages": [{"periods": 2, "projection": "given", "flows": ["100", "110"]}],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "cost_of_equity": "0.11",
        "timing": "end_of_period",
        "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
        "non_operating_assets": [{"label": "Excess cash", "value": "30"}],
    }
    helper = fm.dcf.fcfe(
        [fm.dcf.given(["100", "110"])],
        cash_flow_terms="nominal",
        discount_rate_terms="nominal",
        cost_of_equity="0.11",
        timing="end_of_period",
        terminal=fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
        non_operating_assets=[{"label": "Excess cash", "value": "30"}],
    )
    assert helper.equity_value == D(run("dcf.fcfe", payload)["equity_value"])


# Every technique accepted on the equity basis, with its terms inside the DCF.
EQUITY_TECHNIQUES: dict[str, dict[str, Any]] = {
    "tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"},
    "tv.h_model": {
        "short_term_growth": "0.06",
        "long_term_growth": "0.02",
        "half_life": "2.5",
        "first_flow": "last_flow_grown",
    },
    "tv.limited_horizon": {
        "flow": "95",
        "periods": 12,
        "growth": "0.01",
        "residual_value": "400",
    },
    "tv.liquidation": {
        "assets": [{"label": "Plant", "value": "900"}, {"label": "Receivables", "value": "210"}],
        "liabilities": [{"label": "Bank loans", "value": "350"}],
        "liquidation_costs": [{"label": "Closure", "value": "45"}],
        "tax_on_gains": "30",
        "basis": "equity",
    },
    "tv.exit_multiple": {
        "multiple": "11.5",
        "metric": "earnings",
        "metric_value": "120",
        "next_flow": "98",
    },
    "tv.normalised_income": {"income": "105", "basis": "equity", "growth": "0.015"},
}
EQUITY_RATE_FORMS: list[dict[str, Any]] = [
    {"cost_of_equity": "0.115"},
    {
        "cost_of_equity_per_period": ["0.13", "0.125", "0.12", "0.115"],
        "terminal_cost_of_equity": "0.11",
    },
]


def standalone_input(technique: str, terms: dict[str, Any], rate: str, last_flow: str) -> Any:
    """The input of the `tv.*` method run alone with the same terms, at the terminal cost of
    equity."""
    t = dict(terms)
    if technique in ("tv.gordon", "tv.h_model"):
        t.pop("first_flow")
        return {**t, "rate": rate, "last_flow": last_flow}
    if technique == "tv.exit_multiple":
        return {**t, "rate": rate} if "next_flow" in t else t
    if technique == "tv.liquidation":
        return t
    if technique == "tv.normalised_income":
        return {**t, "capitalisation_rate": rate}
    return {**t, "rate": rate}


@pytest.mark.parametrize(
    ("technique", "rates", "timing"),
    list(product(EQUITY_TECHNIQUES, EQUITY_RATE_FORMS, TIMINGS)),
    ids=str,
)
def test_ac11_terminal_value_on_the_equity_basis_equals_the_standalone_method(
    technique: str, rates: dict[str, Any], timing: tuple[str, str | None]
) -> None:
    """AC11 on the equity basis (R19): each technique accepted by dcf.fcfe gives, inside the
    DCF, the terminal value and the whole output of the `tv.*` method run alone on the same
    terms at the terminal cost of equity."""
    payload: dict[str, Any] = {
        "stages": [
            {"periods": 2, "projection": "given", "flows": ["80", "86"]},
            {"periods": 2, "projection": "constant_growth", "growth": "0.03"},
        ],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        **rates,
        "timing": timing[0],
        "terminal": {technique: EQUITY_TECHNIQUES[technique]},
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    dcf = run("dcf.fcfe", payload)
    rate = rates.get("cost_of_equity", rates.get("terminal_cost_of_equity"))
    alone = fm.run_json(
        technique,
        standalone_input(technique, EQUITY_TECHNIQUES[technique], rate, dcf["periods"][-1]["flow"]),
    )
    assert dcf["terminal"]["technique"] == technique
    assert alone["meta"]["method_version"] == dcf["terminal"]["technique_version"]
    assert dcf["terminal"]["terminal_value"] == alone["value"]["terminal_value"]
    assert dcf["terminal"]["output"] == alone["value"]
