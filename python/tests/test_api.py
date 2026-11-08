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


def test_explain_markdown_is_report_ready() -> None:
    md = dcf([100, 110, 120], "0.10", "0.02").explain_markdown()
    assert "| Step | Calculation | Value |" in md
    assert "Terminal value" in md


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


def test_objects_under_text_keys_are_converted() -> None:
    # The `output` of a tv.compare row is a whole method output: its decimals become Decimal.
    from fairmark._core import _decimalize

    value = _decimalize({"output": {"terminal_value": "1250", "growth_cap": None}})
    assert value["output"]["terminal_value"] == Decimal("1250")
    assert isinstance(value["output"]["terminal_value"], Decimal)


def test_text_and_lists_under_text_keys_stay_text() -> None:
    from fairmark._core import _decimalize

    value = _decimalize(
        {"version": "2", "name": "2025", "pointer": ["1", "2.5"], "amount": ["1", "2.5"]}
    )
    assert value["version"] == "2" and value["name"] == "2025"
    assert value["pointer"] == ["1", "2.5"]
    assert value["amount"] == [Decimal("1"), Decimal("2.5")]


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
        fm.dcf.value_driver(150, "0.02", "0.12", growth_cap="0.03"),
        fm.dcf.exit_multiple(8, "ebitda", 180, next_flow=120, growth_cap="0.03"),
        fm.dcf.limited_horizon(110, 5, growth=0, residual_value=300, growth_cap="0.03"),
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
