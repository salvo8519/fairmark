import json
from datetime import date
from decimal import Decimal

import pytest

import fairmark as fm


def dcf(flows: list[int], wacc: str, growth: str, **kw: object) -> fm.Result:
    return fm.dcf.fcff(
        [fm.dcf.given(flows)],
        cash_flow_terms="nominal",
        discount_rate_terms="nominal",
        wacc=wacc,
        timing="end_of_period",
        terminal=fm.dcf.gordon(growth, first_flow="last_flow_grown"),
        **kw,  # type: ignore[arg-type]
    )


def test_dcf_returns_exact_decimals() -> None:
    r = dcf([100, 110, 120], "0.10", "0.02", net_financial_position={"gross_debt": 200, "cash": 0})
    assert r.terminal["terminal_value"] == Decimal("1530")
    assert isinstance(r.equity_value, Decimal)
    assert r.meta["input_hash"] and len(r.meta["input_hash"]) == 64


def test_same_input_same_result() -> None:
    a = fm.tvm.npv("0.07", [-5000, 1200, 1500])
    b = fm.tvm.npv(Decimal("0.07"), ["-5000", "1200", "1500"])
    assert a.npv == b.npv


def test_floats_go_through_shortest_repr() -> None:
    assert fm.tvm.npv(0, [0.1, 0.2]).npv == Decimal("0.3")


def test_errors_carry_stable_codes() -> None:
    with pytest.raises(fm.FairmarkError) as info:
        dcf([100], "0.05", "0.06")
    assert info.value.code == "FM-E004"


def test_unknown_method() -> None:
    with pytest.raises(fm.FairmarkError) as info:
        fm.run("dcf.nope", {})
    assert info.value.code == "FM-E007"


def test_json_is_the_only_output_format() -> None:
    r = dcf([100, 110, 120], "0.10", "0.02")
    assert not hasattr(type(r), "explain_markdown")
    assert any(s.label.startswith("Terminal value") for s in r.steps)
    assert json.loads(r.to_json())["method"] == r.method


def test_tool_definitions_are_provider_compatible() -> None:
    for tool in fm.tools.anthropic():
        assert tool["name"].startswith("fairmark_")
        assert "$ref" not in str(tool["input_schema"])
        assert tool["input_schema"]["type"] == "object"
    assert fm.tools.openai()[0]["type"] == "function"


def test_dispatch_never_raises() -> None:
    out = fm.tools.dispatch("fairmark_tvm_irr", '{"cash_flows": [1, 2]}')
    assert out["error"]["code"] == "FM-E006"
    ok = fm.tools.dispatch("fairmark_tvm_irr", {"cash_flows": [-100, 110]})
    assert Decimal(ok["value"]["irr"]) == Decimal("0.1")


def test_names_and_versions_stay_text() -> None:
    """Fields that name things are not turned into decimals, even when they look numeric."""
    r = fm.analysis.scenarios(
        "tvm.npv",
        {"rate": "0.1", "cash_flows": [-100, 60, 60]},
        outputs=["/npv"],
        cases={"2025": {"/rate": "0.05"}},
    )
    assert r.inner_method["version"] == "2"
    assert r.scenarios[0]["name"] == "2025"
    assert isinstance(r.scenarios[0]["value"][0]["value"], Decimal)


def test_repeated_names_in_the_list_form_reach_the_engine() -> None:
    with pytest.raises(fm.FairmarkError) as err:
        fm.analysis.scenarios(
            "tvm.npv",
            {"rate": "0.1", "cash_flows": [-100, 60, 60]},
            outputs=["/npv"],
            cases=[("a", {"/rate": "0.05"}), ("a", {"/rate": "0.2"})],
        )
    assert err.value.code == "FM-E001"


def test_existing_results_keep_their_shape() -> None:
    # A tv.compare row: the method id stays text, its output is converted.
    row = fm.tv.compare("0.1", "enterprise", [{"tv.gordon": {"growth": "0", "next_flow": 10}}])
    assert row.results[0]["method"] == "tv.gordon"
    assert row.results[0]["method_version"] == "1"
    assert row.results[0]["output"]["terminal_value"] == Decimal("100")
    # An analysis result: the inner method version stays text, the solution is a Decimal.
    seek = fm.analysis.goal_seek(
        "tvm.npv",
        {"rate": "0.05", "cash_flows": [-1000, 300, 400, 500]},
        variable="/rate",
        output="/npv",
        target="0",
        bracket=("0", "1"),
    )
    assert seek.value["inner_method"]["version"] == "2"
    assert isinstance(seek.value["solution"], Decimal)


def test_dcf_terminal_helpers_build_each_technique() -> None:
    """Issue #43: each helper builds the object of its technique; the engine accepts it."""
    lines = [{"label": "Plant", "value": 500}]
    for terminal in (
        fm.dcf.h_model("0.05", "0.02", 3, first_flow="last_flow_grown", growth_cap="0.03"),
        fm.dcf.h_model(
            "0.05", "0.02", 3, first_flow="normalised_base_flow", normalised_base_flow=95
        ),
        fm.dcf.value_driver(150, "0.02", "0.12", growth_cap="0.03"),
        fm.dcf.exit_multiple(8, "ebitda", 180, next_flow=120, growth_cap="0.03"),
        fm.dcf.limited_horizon(110, 5, growth=0, residual_value=300, growth_cap="0.03"),
        fm.dcf.normalised_income(140, basis="enterprise", growth="0.01", growth_cap="0.03"),
        fm.dcf.liquidation(
            lines, liabilities=[], liquidation_costs=[], tax_on_gains=0, basis="enterprise"
        ),
    ):
        r = fm.dcf.fcff(
            [fm.dcf.given([100, 110])],
            cash_flow_terms="nominal",
            discount_rate_terms="nominal",
            wacc="0.10",
            timing="end_of_period",
            terminal=terminal,
        )
        (technique,) = terminal
        assert r.terminal["technique"] == technique
        assert r.value_of_operations > 0


def test_dcf_converts_rates_with_inflation() -> None:
    """SPEC-0001 R16: real flows at a nominal WACC with expected inflation; the helper passes
    one inflation, or one per period with a terminal inflation."""
    one = fm.dcf.fcff(
        [fm.dcf.given([100, 110])],
        cash_flow_terms="real",
        discount_rate_terms="nominal",
        wacc="0.1016",
        inflation="0.02",
        timing="end_of_period",
        terminal=fm.dcf.gordon("0.01", first_flow="last_flow_grown"),
    )
    per = fm.dcf.fcff(
        [fm.dcf.given([100, 110])],
        cash_flow_terms="real",
        discount_rate_terms="nominal",
        wacc="0.1016",
        inflation_per_period=["0.02", "0.02"],
        terminal_inflation="0.02",
        timing="end_of_period",
        terminal=fm.dcf.gordon("0.01", first_flow="last_flow_grown"),
    )
    assert one.value["terms"]["conversion"] == "to_real"
    assert one.terminal["rate"] == Decimal("0.08")
    assert one.value_of_operations == per.value_of_operations


def test_dcf_bridge_with_debt_like_items_and_non_controlling_interests() -> None:
    """Issue #44: the helper passes both claim lists through; Damodaran Illustration 16.6
    (Segovia, consolidated approach) gives an equity value of 1,605."""
    r = dcf(
        [Decimal("110.25")],
        "0.10",
        "0.05",
        net_financial_position={"gross_debt": 500, "cash": 0},
        non_operating_assets=[{"label": "15% holding in LatinWorks", "value": "22.5"}],
        debt_like_items=[],
        non_controlling_interests=[
            {
                "label": "49% of Seville Television",
                "value": "122.5",
                "measurement_basis": "book_value",
            }
        ],
    )
    assert r.value_of_operations.quantize(Decimal("0.000001")) == Decimal("2205")
    assert r.equity_value.quantize(Decimal("0.000001")) == Decimal("1605")
    assert r.bridge["non_controlling_interests"][0]["measurement_basis"] == "book_value"
    assert any("debt-like items: none given" in c for c in r.conventions)


def test_bridge_claim_builds_a_claim() -> None:
    """Decision 21: the helper writes the basis and, when given, the note."""
    assert fm.dcf.bridge_claim("Pension", 10, measurement_basis="present_value") == {
        "label": "Pension",
        "value": 10,
        "measurement_basis": "present_value",
    }
    claim = fm.dcf.bridge_claim(
        "Provision", 5, measurement_basis="other", basis_note="undiscounted"
    )
    r = dcf([100, 110, 120], "0.10", "0.02", debt_like_items=[claim])
    assert r.bridge["debt_like_items"][0]["basis_note"] == "undiscounted"
    with pytest.raises(fm.FairmarkError) as info:
        dcf(
            [100, 110, 120],
            "0.10",
            "0.02",
            debt_like_items=[fm.dcf.bridge_claim("Provision", 5, measurement_basis="other")],
        )
    assert "/debt_like_items/0/basis_note" in str(info.value)


def test_input_errors_name_the_field_by_its_json_pointer() -> None:
    # Issue #174: the code is unchanged, the message names the field, not a line and column.
    flows = [{"date": "2024-01-01", "amount": -100}, {"date": "2025-01-01", "amount": 110}]
    with pytest.raises(fm.FairmarkError) as err:
        fm.run("tvm.xirr", {"cash_flows": flows, "day_count": "act_365"})
    assert err.value.code == "FM-E001"
    assert "`/day_count`" in str(err.value)
    assert "expected one of `act_365_fixed`" in str(err.value)
    assert " at line " not in str(err.value)

    with pytest.raises(fm.FairmarkError) as err:
        fm.run("tvm.xirr", {"cash_flows": [flows[0], {"date": "2025-01-01"}]})
    assert err.value.code == "FM-E001"
    assert "`/cash_flows/1/amount`" in str(err.value)
    assert "missing required field `amount`" in str(err.value)

    with pytest.raises(fm.FairmarkError) as err:
        fm.tvm.irr([-100, 60, 60], guess={"value": "0.1"})  # type: ignore[arg-type]
    assert err.value.code == "FM-E001"
    assert "`/guess`" in str(err.value)


def test_dates_schedule_helper() -> None:
    """SPEC-0010: `fm.dates.schedule` builds [start, end) periods; the two period-length forms
    give the same dates."""
    named = fm.dates.schedule("2024-01-31", 4, frequency="monthly", day_count="act_360")
    assert named.end_dates == ["2024-02-29", "2024-03-31", "2024-04-30", "2024-05-31"]
    assert named.start_dates[1:] == named.end_dates[:-1]
    assert round(named.year_fractions[0], 25) == round(Decimal(29) / 360, 25)
    assert (named.months_per_period, named.periods_per_year) == (1, 12)
    months = fm.dates.schedule(date(2024, 1, 31), 4, period_months=1, day_count="act_360")
    assert months.value == named.value
    eighteen = fm.dates.schedule("2025-03-15", 2, period_months=18)
    assert eighteen.end_dates == ["2026-09-15", "2028-03-15"]
    assert "periods_per_year" not in eighteen.value
    assert "year_fractions" not in eighteen.value
    with pytest.raises(fm.FairmarkError) as err:
        fm.dates.schedule("2025-01-31", 4, frequency="quarterly", period_months=3)
    assert err.value.code == "FM-E001"


def test_series_project_helpers() -> None:
    """SPEC-0010: `fm.series.project` with each rule builder and both timelines; the dated values
    feed `fm.tvm.xnpv` unchanged."""
    s = fm.series
    r = s.project(100, base_period="period_0", growth=s.constant("0.05"), timeline=s.periods(3))
    assert r.values == [Decimal("105"), Decimal("110.25"), Decimal("115.7625")]
    one = s.project(100, base_period="period_1", growth=s.constant("0.05"), timeline=s.periods(3))
    assert one.values == [Decimal("100"), Decimal("105"), Decimal("110.25")]
    lin = s.project(
        1000, base_period="period_0", growth=s.linear("0.1308", "0.05"), timeline=s.periods(5)
    )
    assert [p["stated_growth"] for p in lin.periods] == [
        Decimal("0.11464"),
        Decimal("0.09848"),
        Decimal("0.08232"),
        Decimal("0.06616"),
        Decimal("0.05"),
    ]
    rates = s.per_period(["0.10", "0.08", "-0.02"])
    per = s.project(500, base_period="period_1", growth=rates, timeline=s.periods(4))
    assert per.values[-1] == Decimal("582.12")
    simple = s.project(
        1000,
        base_period="period_0",
        growth=s.constant("0.12"),
        growth_basis="annual_simple",
        timeline=s.periods(12, frequency="monthly"),
    )
    assert simple.values[-1] == Decimal("1000") * Decimal("1.01") ** 12
    dated = s.project(
        300,
        base_period="period_0",
        growth=s.constant("0.02"),
        growth_basis="per_period",
        timeline=s.schedule(fm.dates.schedule_input("2026-01-01", 4, frequency="quarterly")),
        timing="end_of_period",
    )
    assert [f["date"] for f in dated.dated_values] == [
        "2026-04-01",
        "2026-07-01",
        "2026-10-01",
        "2027-01-01",
    ]
    flows = [("2026-01-01", -1000)] + [(f["date"], f["amount"]) for f in dated.dated_values]
    by_hand = [
        ("2026-01-01", "-1000"),
        ("2026-04-01", "306"),
        ("2026-07-01", "312.12"),
        ("2026-10-01", "318.3624"),
        ("2027-01-01", "324.729648"),
    ]
    assert fm.tvm.xnpv("0.10", flows).xnpv == fm.tvm.xnpv("0.10", by_hand).xnpv
    with pytest.raises(fm.FairmarkError) as err:
        s.project(
            1000,
            base_period="period_0",
            growth=s.constant("0.12"),
            timeline=s.periods(12, frequency="monthly"),
        )
    assert err.value.code == "FM-E001"
    assert "`/growth_basis`" in str(err.value)
