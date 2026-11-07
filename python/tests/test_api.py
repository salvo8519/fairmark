from decimal import Decimal

import pytest

import fairmark as fm


def test_dcf_returns_exact_decimals() -> None:
    r = fm.dcf.fcff([100, 110, 120], "0.10", "0.02", net_debt=200)
    assert r.terminal_value == Decimal("1530")
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
        fm.dcf.fcff([100], "0.05", "0.06")
    assert info.value.code == "FM-E004"


def test_unknown_method() -> None:
    with pytest.raises(fm.FairmarkError) as info:
        fm.run("dcf.nope", {})
    assert info.value.code == "FM-E007"


def test_explain_markdown_is_report_ready() -> None:
    md = fm.dcf.fcff([100, 110, 120], "0.10", "0.02").explain_markdown()
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
