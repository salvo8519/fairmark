"""The shared DCF engine (SPEC-0001, issue #110) against the independent reference
scripts/reference_dcf.py.

The golden cases pin single points; these tests check that the golden values are those of the
reference, that the engine and the 50-digit reference agree across a grid of stages, rates and
timings, and the properties AC5 and AC7.
"""

from __future__ import annotations

import importlib.util
from decimal import Context, localcontext
from decimal import Decimal as D
from itertools import product
from pathlib import Path
from typing import Any

import pytest
import yaml

import fairmark as fm

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("ref_dcf", ROOT / "scripts/reference_dcf.py")
assert _spec and _spec.loader
ref = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ref)

REFERENCE = "Independent 50-digit reference, scripts/reference_dcf.py"
GOLDEN = sorted((ROOT / "tests/golden/dcf.fcff").glob("*.yaml"))
# Relative agreement of engine and reference: about 1e-24 at end of period (exact powers and
# divisions in 28 digits), about 1e-18 at mid period (square roots in 28 digits).
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


def reference_cases() -> list[Path]:
    out = []
    for path in GOLDEN:
        case = yaml.safe_load(path.read_text(encoding="utf-8"))
        if "expect" in case and str(case.get("source", "")).startswith(REFERENCE):
            out.append(path)
    return out


@pytest.mark.parametrize("path", reference_cases(), ids=lambda p: p.stem)
def test_golden_values_are_the_reference_values(path: Path) -> None:
    """Each expected value of a reference-sourced golden case is the reference value, rounded
    to the digits written in the file."""
    case = yaml.safe_load(path.read_text(encoding="utf-8"))
    values = ref.dcf_fcff(case["input"])
    for key, expected in case["expect"].items():
        if not is_decimal(expected):
            continue  # names and nulls are compared exactly by the golden runner
        want = D(str(expected))
        got = values[key if key.startswith("/") else f"/{key}"]
        assert abs(got - want) <= D(1).scaleb(want.as_tuple().exponent), key  # type: ignore[arg-type]


def run(payload: dict[str, Any]) -> dict[str, Any]:
    return fm.run_json("dcf.fcff", payload)["value"]


def close(got: str | D, want: D, timing: str) -> bool:
    return abs(D(str(got)) - want) <= REL[timing] * max(D(1), abs(want))


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
RATES = ["0.065", "0.1", "0.14"]
TIMINGS = [("end_of_period", None), ("mid_period", "horizon"), ("mid_period", "mid_period")]


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
        "wacc": rate,
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": "0.015", "first_flow": "last_flow_grown"}},
        "net_financial_position": {"gross_debt": "120", "cash": "30"},
        "non_operating_assets": [{"label": "Land", "value": "15"}],
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got, want = run(payload), ref.dcf_fcff(payload)
    for key, value in want.items():
        assert close(pointer(got, key), value, timing[0]), key


@pytest.mark.parametrize("timing", TIMINGS, ids=str)
def test_engine_matches_reference_with_rates_per_period(timing: tuple[str, str | None]) -> None:
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
        "discount_rate_terms": "real",
        "wacc_per_period": ["0.12", "0.11", "-0.01", "0.095", "0.08"],
        "terminal_wacc": "0.075",
        "timing": timing[0],
        "terminal": {
            "tv.gordon": {
                "growth": "0.03",
                "first_flow": "normalised_next_flow",
                "normalised_next_flow": "61",
            }
        },
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got, want = run(payload), ref.dcf_fcff(payload)
    for key, value in want.items():
        assert close(pointer(got, key), value, timing[0]), key


@pytest.mark.parametrize("timing", TIMINGS, ids=str)
def test_ac5_generated_stages_equal_given_flows(timing: tuple[str, str | None]) -> None:
    """AC5: a constant growth stage equals a given stage with the same flows; a linear decline
    with equal growths equals constant growth at that rate."""
    common: dict[str, Any] = {
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": "0.085",
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
    }
    if timing[1]:
        common["terminal_discount_time"] = timing[1]
    constant = run(
        {
            **common,
            "base_flow": "100",
            "stages": [{"periods": 4, "projection": "constant_growth", "growth": "0.06"}],
        }
    )
    flows = [row["flow"] for row in constant["periods"]]
    as_given = run({**common, "stages": [{"periods": 4, "projection": "given", "flows": flows}]})
    decline = run(
        {
            **common,
            "base_flow": "100",
            "stages": [
                {
                    "periods": 4,
                    "projection": "linear_decline",
                    "from_growth": "0.06",
                    "to_growth": "0.06",
                }
            ],
        }
    )
    for other in (as_given, decline):
        assert other["value_of_operations"] == constant["value_of_operations"]
        assert other["terminal"]["terminal_value"] == constant["terminal"]["terminal_value"]
        assert [r["present_value"] for r in other["periods"]] == [
            r["present_value"] for r in constant["periods"]
        ]


@pytest.mark.parametrize("timing", ["end_of_period", "mid_period"])
def test_ac7_discount_factors_equal_those_of_tvm_present_value(timing: str) -> None:
    rates = ["0.12", "0.1", "-0.02", "0.07"]
    flows = ["10", "20", "30", "40"]
    payload: dict[str, Any] = {
        "stages": [{"periods": 4, "projection": "given", "flows": flows}],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc_per_period": rates,
        "terminal_wacc": "0.07",
        "timing": timing,
        "terminal": {"tv.gordon": {"growth": "0.01", "first_flow": "last_flow_grown"}},
    }
    if timing == "mid_period":
        payload["terminal_discount_time"] = "horizon"
    dcf = run(payload)
    pv = fm.run_json(
        "tvm.present_value", {"cash_flows": flows, "period_rates": rates, "timing": timing}
    )["value"]
    assert [r["discount_factor"] for r in dcf["periods"]] == pv["discount_factors"]
    # Present values divide by the compounding factor instead of multiplying by the discount
    # factor (precision at long horizons): equal to the last digits, not bit for bit.
    for ours, theirs in zip(dcf["periods"], pv["present_values"], strict=True):
        assert abs(D(ours["present_value"]) - D(theirs)) <= D("1e-26")


@pytest.mark.parametrize(
    ("periods", "rate", "growth", "terminal_growth", "timing"),
    [
        (400, "0.10", "0.095", "0.095", ("end_of_period", None)),
        (300, "0.20", "0.195", "0.19", ("mid_period", "mid_period")),
        (300, "0.20", "0.195", "0.19", ("mid_period", "horizon")),
        (1000, "0.05", "0.045", "0.04", ("end_of_period", None)),
        (250, "0.12", "0.11", "0.11", ("mid_period", "horizon")),
    ],
    ids=str,
)
def test_long_horizons_keep_full_precision(
    periods: int, rate: str, growth: str, terminal_growth: str, timing: tuple[str, str | None]
) -> None:
    """Audit finding 1: at long horizons the value of operations matches the 50-digit
    reference to 1e-24 relative, because present values divide by the compounding factor."""
    payload: dict[str, Any] = {
        "stages": [{"periods": periods, "projection": "constant_growth", "growth": growth}],
        "base_flow": "100",
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": rate,
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": terminal_growth, "first_flow": "last_flow_grown"}},
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got = D(run(payload)["value_of_operations"])
    want = ref.dcf_fcff(payload)["/value_of_operations"]
    assert abs(got - want) <= D("1e-24") * abs(want)


# Issue #43: every terminal technique inside the DCF (R19 to R22; AC11, AC12).
TECHNIQUES: dict[str, dict[str, Any]] = {
    "tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown", "growth_cap": "0.025"},
    "tv.h_model": {
        "short_term_growth": "0.07",
        "long_term_growth": "0.02",
        "half_life": "3.5",
        "first_flow": "last_flow_grown",
        "growth_cap": "0.015",
    },
    "tv.value_driver": {"nopat_next": "180", "growth": "0.03", "ronic": "0.14"},
    "tv.exit_multiple": {
        "multiple": "8.25",
        "metric": "ebitda",
        "metric_value": "230",
        "next_flow": "120",
        "growth_cap": "0.02",
    },
    "tv.limited_horizon": {
        "flow": "95",
        "periods": 12,
        "growth": "-0.015",
        "residual_value": "250",
        "growth_cap": "0.01",
    },
    "tv.normalised_income": {
        "income": "105",
        "basis": "enterprise",
        "growth": "0.015",
        "growth_cap": "0.01",
    },
    "tv.liquidation": {
        "assets": [{"label": "Plant", "value": "700"}, {"label": "Stock", "value": "150"}],
        "liabilities": [{"label": "Payables", "value": "120"}],
        "liquidation_costs": [{"label": "Closure", "value": "60"}],
        "tax_on_gains": "25",
        "basis": "enterprise",
    },
}
RATE_FORMS: list[dict[str, Any]] = [
    {"wacc": "0.095"},
    {"wacc_per_period": ["0.12", "0.11", "0.10", "0.09"], "terminal_wacc": "0.085"},
]


def terminal_payload(
    technique: str, rates: dict[str, Any], timing: tuple[str, str | None]
) -> dict[str, Any]:
    payload: dict[str, Any] = {
        "stages": [
            {"periods": 2, "projection": "given", "flows": ["70", "76"]},
            {"periods": 2, "projection": "constant_growth", "growth": "0.04"},
        ],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        **rates,
        "timing": timing[0],
        "terminal": {technique: TECHNIQUES[technique]},
        "net_financial_position": {"gross_debt": "300", "cash": "40"},
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    return payload


@pytest.mark.parametrize(
    ("technique", "rates", "timing"),
    list(product(TECHNIQUES, RATE_FORMS, TIMINGS)),
    ids=str,
)
def test_every_terminal_technique_matches_reference(
    technique: str, rates: dict[str, Any], timing: tuple[str, str | None]
) -> None:
    """R19, R21, R22: each technique, each rate form and each timing against the reference."""
    payload = terminal_payload(technique, rates, timing)
    got, want = run(payload), ref.dcf_fcff(payload)
    assert got["terminal"]["technique"] == technique
    for key, value in want.items():
        assert close(pointer(got, key), value, timing[0]), key


def standalone_input(technique: str, terms: dict[str, Any], rate: str, last_flow: str) -> Any:
    """The input of the `tv.*` method run alone with the same terms, at the terminal rate."""
    t = dict(terms)
    if technique == "tv.gordon":
        t.pop("first_flow")
        return {**t, "rate": rate, "last_flow": last_flow}
    if technique == "tv.h_model":
        if t.pop("first_flow") == "normalised_base_flow":
            last_flow = t.pop("normalised_base_flow")
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
    list(product(TECHNIQUES, RATE_FORMS, TIMINGS)),
    ids=str,
)
def test_ac11_terminal_value_equals_the_standalone_method(
    technique: str, rates: dict[str, Any], timing: tuple[str, str | None]
) -> None:
    """AC11: the terminal value inside the DCF, and the technique's whole output, equal the
    `tv.*` method run alone on the same terms and the terminal rate; its present value is that
    value discounted at the terminal discounting time."""
    payload = terminal_payload(technique, rates, timing)
    dcf = run(payload)
    rate = rates.get("wacc", rates.get("terminal_wacc"))
    last_flow = dcf["periods"][-1]["flow"]
    alone = fm.run_json(
        technique, standalone_input(technique, TECHNIQUES[technique], rate, last_flow)
    )
    assert alone["meta"]["method_version"] == dcf["terminal"]["technique_version"]
    assert dcf["terminal"]["terminal_value"] == alone["value"]["terminal_value"]
    assert dcf["terminal"]["output"] == alone["value"]

    period_rates = rates.get("wacc_per_period") or [rates["wacc"]] * len(dcf["periods"])
    compounding = D(1)
    for r in period_rates:
        compounding *= 1 + D(r)
    tv = D(alone["value"]["terminal_value"])
    if timing[1] == "mid_period":
        # Decision 16: the half-period shift uses the terminal rate.
        want = tv * (1 + D(rate)).sqrt() / compounding
        assert D(dcf["terminal"]["discount_time"]) == len(period_rates) - D("0.5")
    else:
        want = tv / compounding
        assert D(dcf["terminal"]["discount_time"]) == len(period_rates)
    assert close(dcf["terminal"]["present_value"], want, timing[0])


@pytest.mark.parametrize(("rates", "timing"), list(product(RATE_FORMS, TIMINGS)), ids=str)
def test_ac11_h_model_on_a_normalised_base_flow(
    rates: dict[str, Any], timing: tuple[str, str | None]
) -> None:
    """Decision 15: with a normalised base flow at N the H-model inside the DCF equals
    tv.h_model run alone on that flow, and the reference."""
    terms = {
        **TECHNIQUES["tv.h_model"],
        "first_flow": "normalised_base_flow",
        "normalised_base_flow": "88.5",
    }
    payload = terminal_payload("tv.h_model", rates, timing)
    payload["terminal"] = {"tv.h_model": terms}
    dcf, want = run(payload), ref.dcf_fcff(payload)
    for key, value in want.items():
        assert close(pointer(dcf, key), value, timing[0]), key
    rate = rates.get("wacc", rates.get("terminal_wacc"))
    alone = fm.run_json(
        "tv.h_model", standalone_input("tv.h_model", terms, rate, dcf["periods"][-1]["flow"])
    )
    assert dcf["terminal"]["output"] == alone["value"]


# ---------------------------------------------------------------- nominal and real terms (#166)

INFLATION_FORMS: list[dict[str, Any]] = [
    {"inflation": "0.025"},
    {"inflation": "-0.01"},
    {
        "inflation_per_period": ["0.04", "0.03", "0.035", "0.02", "0.015", "0.02", "0.02"],
        "terminal_inflation": "0.018",
    },
]
TERMS = [("real", "nominal"), ("nominal", "real"), ("nominal", "nominal"), ("real", "real")]


@pytest.mark.parametrize(
    ("terms", "inflation", "timing"), list(product(TERMS, INFLATION_FORMS, TIMINGS)), ids=str
)
def test_engine_matches_reference_with_inflation(
    terms: tuple[str, str], inflation: dict[str, Any], timing: tuple[str, str | None]
) -> None:
    """R16 to R18: rates converted to the terms of the flows when the terms differ, no
    conversion when they are equal; the perpetual growth restated in the other terms."""
    payload: dict[str, Any] = {
        "stages": STAGES[2],
        "cash_flow_terms": terms[0],
        "discount_rate_terms": terms[1],
        "wacc": "0.09",
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": "0.015", "first_flow": "last_flow_grown"}},
        **inflation,
    }
    if timing[1]:
        payload["terminal_discount_time"] = timing[1]
    got, want = run(payload), ref.dcf_fcff(payload)
    for key, value in want.items():
        assert close(pointer(got, key), value, timing[0]), key
    expected = "none" if terms[0] == terms[1] else f"to_{terms[0]}"
    assert got["terms"]["conversion"] == expected


@pytest.mark.parametrize("inflation", INFLATION_FORMS, ids=str)
@pytest.mark.parametrize("timing", TIMINGS, ids=str)
def test_ac9_conversion_equals_the_rates_of_rates_real_nominal(
    inflation: dict[str, Any], timing: tuple[str, str | None]
) -> None:
    """AC9: real flows at nominal rates with inflation give exactly the result of the same flows
    at the real rates computed by `rates.real_nominal`, given directly as real rates."""
    n = 7
    nominal = ["0.11", "0.1", "0.095", "0.09", "0.0875", "0.085", "0.085"]
    common: dict[str, Any] = {
        "stages": STAGES[2],
        "cash_flow_terms": "real",
        "timing": timing[0],
        "terminal": {"tv.gordon": {"growth": "0.01", "first_flow": "last_flow_grown"}},
    }
    if timing[1]:
        common["terminal_discount_time"] = timing[1]
    per_period = inflation.get("inflation_per_period", [inflation.get("inflation")] * n)
    terminal_inflation = inflation.get("terminal_inflation", inflation.get("inflation"))

    def real(rate: str, i: str) -> str:
        out = fm.run_json(
            "rates.real_nominal", {"rate": rate, "inflation": i, "direction": "to_real"}
        )["value"]
        return str(out["rate"])

    converted = run(
        {
            **common,
            "discount_rate_terms": "nominal",
            "wacc_per_period": nominal,
            "terminal_wacc": "0.08",
            **inflation,
        }
    )
    direct = run(
        {
            **common,
            "discount_rate_terms": "real",
            "wacc_per_period": [real(r, i) for r, i in zip(nominal, per_period, strict=True)],
            "terminal_wacc": real("0.08", terminal_inflation),
        }
    )
    assert converted["value_of_operations"] == direct["value_of_operations"]
    assert converted["terminal"] == direct["terminal"]
    for a, b in zip(converted["periods"], direct["periods"], strict=True):
        for field in ("flow", "rate_used", "discount_factor", "present_value"):
            assert a[field] == b[field], field


@pytest.mark.parametrize("inflation", INFLATION_FORMS, ids=str)
def test_ac9_real_flows_equal_the_inflated_nominal_flows(inflation: dict[str, Any]) -> None:
    """AC9: under end-of-period timing, real flows at nominal rates converted with inflation
    value like the independently inflated nominal flows at the nominal rates (1e-20 relative)."""
    payload: dict[str, Any] = {
        "stages": STAGES[2],
        "cash_flow_terms": "real",
        "discount_rate_terms": "nominal",
        "wacc": "0.095",
        "timing": "end_of_period",
        "terminal": {"tv.gordon": {"growth": "0.012", "first_flow": "last_flow_grown"}},
        **inflation,
    }
    nominal = ref.inflated_input(payload)
    want = ref.dcf_fcff(nominal)["/value_of_operations"]
    assert abs(ref.dcf_fcff(payload)["/value_of_operations"] - want) <= D("1e-40") * want
    for got in (run(payload), run(nominal)):
        assert abs(D(got["value_of_operations"]) - want) <= D("1e-20") * abs(want)


# Issue #44: from value of operations to equity value (R23, R25, AC13).


def random_bridge(rng: Any) -> dict[str, Any]:
    """A bridge with zero to three lines of each kind, amounts with up to four decimals."""

    def amount() -> str:
        return str(D(rng.randint(0, 50_000_000)).scaleb(-4))

    def claims(kind: str) -> list[dict[str, str]]:
        return [
            {"label": f"{kind} {i}", "value": amount(), "measurement_basis": "book value"}
            for i in range(rng.randint(0, 3))
        ]

    bridge: dict[str, Any] = {
        "non_operating_assets": [
            {"label": f"asset {i}", "value": amount()} for i in range(rng.randint(0, 3))
        ],
        "debt_like_items": claims("debt-like item"),
        "non_controlling_interests": claims("non-controlling interest"),
    }
    if rng.random() < 0.8:
        bridge["net_financial_position"] = {
            "gross_debt": amount(),
            "cash": amount(),
            "other_items": [{"name": "lease liabilities", "amount": amount()}],
        }
    return {k: v for k, v in bridge.items() if v}


@pytest.mark.parametrize("seed", range(20))
def test_bridge_matches_reference_on_random_lines(seed: int) -> None:
    """AC13 (R23, R25): value of operations, enterprise value, equity value and the totals of
    the bridge equal the 50-digit reference for random bridges with every kind of line."""
    import random

    rng = random.Random(seed)
    payload: dict[str, Any] = {
        "stages": STAGES[seed % len(STAGES)],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": RATES[seed % len(RATES)],
        "timing": "end_of_period",
        "terminal": {"tv.gordon": {"growth": "0.015", "first_flow": "last_flow_grown"}},
        **random_bridge(rng),
    }
    got, want = run(payload), ref.dcf_fcff(payload)
    for key, value in want.items():
        assert close(pointer(got, key), value, "end_of_period"), key


@pytest.mark.parametrize("seed", range(20))
def test_bridge_identity_holds_to_the_last_digit(seed: int) -> None:
    """AC13 (R23, R25): enterprise value - value of operations is the sum of the non-operating
    assets, and equity value = enterprise value - net financial position - Σ debt-like items -
    Σ non-controlling interests, within the rounding of the engine's 28 significant digits
    (one unit in the last place per addition or subtraction); the totals of the claims are
    exact; every label, amount and basis is echoed in the bridge."""
    import random

    rng = random.Random(1000 + seed)
    bridge = random_bridge(rng)
    payload: dict[str, Any] = {
        "stages": STAGES[1],
        "cash_flow_terms": "nominal",
        "discount_rate_terms": "nominal",
        "wacc": "0.09",
        "timing": "mid_period",
        "terminal_discount_time": "mid_period",
        "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
        **bridge,
    }
    got = run(payload)
    b = got["bridge"]
    # Exact arithmetic on the returned decimals: 60 digits, so that nothing here is rounded.
    with localcontext(Context(prec=60)):

        def total(items: list[dict[str, Any]]) -> D:
            return sum((D(i["value"]) for i in items), D(0))

        ev = D(got["enterprise_value"])
        ulp = D("1e-27") * max(D(1), abs(ev))
        nfp = D(b["net_financial_position"] or 0)
        assert abs(ev - D(got["value_of_operations"]) - total(b["non_operating_assets"])) <= ulp
        assert D(b["debt_like_items_total"]) == total(b["debt_like_items"])
        assert D(b["non_controlling_interests_total"]) == total(b["non_controlling_interests"])
        equity = ev - nfp - D(b["debt_like_items_total"]) - D(b["non_controlling_interests_total"])
        assert abs(D(got["equity_value"]) - equity) <= 3 * ulp
    for kind in ("debt_like_items", "non_controlling_interests"):
        given = bridge.get(kind, [])
        assert [(c["label"], D(c["value"]), c["measurement_basis"]) for c in b[kind]] == [
            (c["label"], D(c["value"]), c["measurement_basis"]) for c in given
        ]
