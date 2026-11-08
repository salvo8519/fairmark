"""SPEC-0001 Decision 24 (issue #197): the DCF from a vector of cash flows and one rate, through
the Python helpers. The engine fills in the documented defaults; the helpers only omit the
fields that were not given."""

from __future__ import annotations

from decimal import Decimal as D

import pytest

import fairmark as fm

FLOWS = ["120", "135", "150", "160", "170"]


def test_minimal_call_works_and_reports_its_defaults() -> None:
    r = fm.dcf.fcff(flows=FLOWS, wacc="0.09")
    # tests/golden/dcf.fcff/flows-only-single-wacc.yaml (50-digit reference).
    assert r.value_of_operations == D("563.38243366056148746628356553")
    fields = [d["field"] for d in r.meta["defaults_applied"]]
    assert fields == [
        "/stages/0/projection",
        "/stages/0/periods",
        "/cash_flow_terms",
        "/discount_rate_terms",
        "/timing",
        "/terminal",
    ]
    assert any(c.startswith("warning: no terminal value is included") for c in r.conventions)


def test_minimal_call_equals_tvm_npv_of_the_same_flows() -> None:
    dcf = fm.dcf.fcff(flows=FLOWS, wacc="0.09")
    npv = fm.run("tvm.npv", {"rate": "0.09", "cash_flows": ["0", *FLOWS]})
    assert dcf.value_of_operations == npv.npv


def test_explicit_choices_prevail_and_hash_like_the_minimal_call() -> None:
    minimal = fm.dcf.fcff(flows=FLOWS, wacc="0.09")
    explicit = fm.dcf.fcff(
        [fm.dcf.given(FLOWS)],
        cash_flow_terms="nominal",
        discount_rate_terms="nominal",
        timing="end_of_period",
        terminal=fm.dcf.no_terminal(),
        wacc="0.09",
    )
    assert "defaults_applied" not in explicit.meta
    assert explicit.meta["input_hash"] == minimal.meta["input_hash"]
    assert explicit.value == minimal.value


def test_fcfe_minimal_call_and_given_terminal_value() -> None:
    r = fm.dcf.fcfe(
        flows=["80", "85", "92", "96", "101"],
        cost_of_equity="0.11",
        terminal=fm.dcf.given_terminal("1500"),
    )
    # tests/golden/dcf.fcfe/terminal-given-equity-basis.yaml (50-digit reference).
    assert r.terminal["technique"] == "given"
    assert r.terminal["terminal_value"] == D("1500")
    assert "/terminal" not in [d["field"] for d in r.meta["defaults_applied"]]


def test_stages_and_flows_together_are_refused() -> None:
    with pytest.raises(TypeError, match="not both"):
        fm.dcf.fcff([fm.dcf.given(FLOWS)], flows=FLOWS, wacc="0.09")
