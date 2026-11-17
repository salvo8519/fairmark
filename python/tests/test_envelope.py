"""The envelope in the Python SDK (SPEC-0012 R15, AC13): `Result.diagnostics` and the
properties `not_computed`, `warnings`, `checks` and `defaults_applied` equal the JSON of the core,
and take precedence over a field of `value` with the same name."""

from __future__ import annotations

from typing import Any

import fairmark as fm

DEBT_SERVICE = {
    "balances": "closing",
    "interest_definition": "gross interest expense",
    "income_statement": {"ebit": "500", "interest": "100"},
}
DCF_WITHOUT_NFP: dict[str, Any] = {
    "stages": [{"periods": 2, "projection": "given", "flows": [100, 110]}],
    "cash_flow_terms": "nominal",
    "discount_rate_terms": "nominal",
    "wacc": "0.09",
    "timing": "end_of_period",
    "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
}


def test_the_result_exposes_the_diagnostics_as_the_core_writes_them() -> None:
    for method, inputs in (
        ("ratios.debt_service", DEBT_SERVICE),
        ("dcf.fcff", DCF_WITHOUT_NFP),
        ("tvm.npv", {"rate": "0.1", "cash_flows": [-100, 60, 60]}),
    ):
        raw = fm.run_json(method, inputs)
        r = fm.run(method, inputs)
        assert r.diagnostics == raw["diagnostics"], method
        assert r.not_computed == raw["diagnostics"]["not_computed"], method
        assert r.warnings == raw["diagnostics"]["warnings"], method
        assert r.checks == raw["diagnostics"]["checks"], method
        assert r.defaults_applied == raw["meta"]["defaults_applied"], method
        for listed in (r.not_computed, r.warnings, r.checks, r.defaults_applied):
            assert isinstance(listed, list)


def test_the_lists_hold_text_not_numbers() -> None:
    r = fm.run("ratios.debt_service", DEBT_SERVICE)
    entry = next(e for e in r.not_computed if e["pointer"] == "/value/nfp_to_ebitda")
    assert entry["reason_code"] == "missing_input"
    assert all(isinstance(p, str) for p in entry["missing_any_of"][0])
    w = fm.run("dcf.fcff", DCF_WITHOUT_NFP).warnings
    assert [x["code"] for x in w] == ["FM-W012"]
    assert w[0]["pointer"] == "/net_financial_position"


def test_the_properties_take_precedence_over_a_value_field() -> None:
    # `analysis.tornado` has no such field; a result whose value had one would still read the
    # diagnostics: the properties are resolved before the value fields.
    r = fm.run("tvm.npv", {"rate": "0.1", "cash_flows": [-100, 60, 60]})
    object.__setattr__(r, "value", {**r.value, "warnings": "a value field"})
    assert r.warnings == []
    assert r.value["warnings"] == "a value field"
