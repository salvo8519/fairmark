"""scripts/golden_envelope_migration.py (SPEC-0012 T3), tested on temporary copies only.

The engine is replaced by results written here in the shape of today's results, so the tests do
not depend on the engine version.
"""

from __future__ import annotations

import importlib.util
import json
import shutil
import sys
from pathlib import Path
from typing import Any

import pytest
import yaml

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/golden_envelope_migration.py"
_spec = importlib.util.spec_from_file_location("golden_envelope_migration", SCRIPT)
assert _spec and _spec.loader
gm = importlib.util.module_from_spec(_spec)
sys.modules["golden_envelope_migration"] = gm  # dataclasses resolve types through sys.modules
_spec.loader.exec_module(gm)

STATEMENTS_SCHEMA = {
    "type": "object",
    "properties": {"years": {"type": "array", "items": {"$ref": "#/$defs/YearOutput"}}},
    "$defs": {
        "YearOutput": {
            "type": "object",
            "properties": {
                "assignments": {"type": "array", "items": {"$ref": "#/$defs/Assignment"}},
                "total": {"type": "string"},
            },
        },
        "Assignment": {
            "type": "object",
            "properties": {"aggregate": {"type": "string"}, "amount": {"type": "string"}},
        },
    },
}

RATIOS_CASE = """method: ratios.profitability
description: temporary ratios case
source: written for the test
input:
  revenue: '100'
expect:
  /roe/value: 'null'
  /roe/not_defined: equity is negative (-200)
  /roi/value: '0.1'
  /roi/not_defined: 'null'
  /not_computed/0/output: /ros
  /not_computed/0/missing/0: /income_statement/net_income
  /not_computed/1/missing/0: /revenue
expect_length:
  /not_computed: '2'
tolerance: '0'
"""
RATIOS_RESULT = {
    "value": {
        "roe": {"value": None, "not_defined": "equity is negative (-200)"},
        "roi": {"value": "0.1", "not_defined": None},
        "ros": {"value": None, "not_defined": None},
        "turnover": {"value": None, "not_defined": None},
        "not_computed": [
            {"output": "/ros", "missing": ["/income_statement/net_income"], "reason": "r"},
            {"output": "/turnover", "missing": ["/revenue"], "reason": "r"},
        ],
    }
}
STATEMENTS_CASE = """method: statements.reorganise
description: temporary statements case
source: written for the test
input:
  years: []
expect:
  /years/0/total: '10'
  /years/0/assignments/0/aggregate: /years/0/statutory/balance_sheet/assets
  /years/1/checks/b_total/status: reconciled
  /years/0/not_computed/income_statement/0/output: /years/0/income_statement/ebitda
  /years/0/not_computed/income_statement/0/permitted/0/treatment: /treatments/x
  /years/0/not_computed/income_statement/0/permitted/0/destinations/1: financial
expect_meta:
  /defaults_applied/0/field: /treatments/write_downs_of_receivables
  /defaults_applied/0/value: operating
tolerance: '0'
"""
STATEMENTS_RESULT = {
    "value": {
        "years": [
            {
                "checks": {"z_total": {}, "a_total": {}},
                "not_computed": {
                    "balance_sheet": [{"output": "/years/0/balance_sheet/total"}],
                    "income_statement": [{"output": "/years/0/income_statement/ebitda"}],
                },
            },
            {"checks": {"b_total": {}, "a_total": {}}, "not_computed": {}},
        ]
    }
}
UNTOUCHED_CASE = """method: tvm.npv
description: untouched
input: {}
expect:
  npv: '1'
tolerance: '0'
"""


def fake_run(method: str, data: Any) -> dict[str, Any]:
    return RATIOS_RESULT if method.startswith("ratios.") else STATEMENTS_RESULT


def make_root(tmp: Path) -> Path:
    golden = tmp / "tests/golden"
    for folder, name, text in [
        ("ratios.profitability", "case.yaml", RATIOS_CASE),
        ("statements.reorganise", "case.yaml", STATEMENTS_CASE),
        ("tvm.npv", "case.yaml", UNTOUCHED_CASE),
    ]:
        (golden / folder).mkdir(parents=True, exist_ok=True)
        (golden / folder / name).write_text(text, encoding="utf-8")
    schema = tmp / "schemas/statements.reorganise/v1.output.json"
    schema.parent.mkdir(parents=True)
    schema.write_text(json.dumps(STATEMENTS_SCHEMA), encoding="utf-8")
    return tmp


def list_plan(root: Path) -> dict[str, Any]:
    md = root / "list.md"
    assert gm.main(["--root", str(root), "list", "--markdown", str(md)], run=fake_run) == 0
    plan: dict[str, Any] = json.loads((root / gm.PLAN).read_text(encoding="utf-8"))
    return plan


def changes_of(plan: dict[str, Any], rel: str) -> dict[str, dict[str, Any]]:
    return {c["key"] or c["new_key"]: c for c in plan["files"][rel]["changes"]}


RATIOS = "tests/golden/ratios.profitability/case.yaml"
STATEMENTS = "tests/golden/statements.reorganise/case.yaml"


def test_list_maps_ratio_keys_in_the_order_of_r13(tmp_path: Path) -> None:
    plan = list_plan(make_root(tmp_path))
    assert set(plan["files"]) == {RATIOS, STATEMENTS}
    c = changes_of(plan, RATIOS)
    # New list by pointer: /value/roe (not defined), /value/ros, /value/turnover.
    assert c["/roe/not_defined"]["new_key"] == "/not_computed/0/reason"
    assert c["/roe/not_defined"]["new_value"] == "equity is negative (-200)"
    added = {k: v["new_value"] for k, v in c.items() if v["kind"] == "add"}
    assert added["/not_computed/0/pointer"] == "/value/roe"
    assert added["/not_computed/0/reason_code"] == "not_defined"
    assert added["/not_computed/2/pointer"] == "/value/turnover"
    moved = c["/not_computed/0/output"]
    assert (moved["kind"], moved["new_key"], moved["new_value"]) == (
        "move_value_changes",
        "/not_computed/1/pointer",
        "/value/ros",
    )
    assert c["/not_computed/1/missing/0"]["new_key"] == "/not_computed/2/missing_any_of/0/0"
    length = c["/not_computed"]
    assert (length["new_section"], length["value"], length["new_value"]) == (
        "expect_diagnostics_length",
        "2",
        "3",
    )
    assert c["/roi/not_defined"]["kind"] == "drop"
    assert "/roi/value" in c["/roi/not_defined"]["note"]
    assert "/roe/value" not in c and "/roi/value" not in c


def test_list_maps_statement_keys_checks_by_year_then_id(tmp_path: Path) -> None:
    plan = list_plan(make_root(tmp_path))
    c = changes_of(plan, STATEMENTS)
    # Checks: year 0 a_total, z_total; year 1 a_total, b_total -> index 3.
    check = c["/years/1/checks/b_total/status"]
    assert (check["new_section"], check["new_key"]) == ("expect_diagnostics", "/checks/3/status")
    added = {k: v["new_value"] for k, v in c.items() if v["kind"] == "add"}
    assert added["/checks/3/id"] == "b_total"
    assert added["/checks/3/pointer"] == "/value/years/1"
    # Not computed: balance_sheet before income_statement by pointer.
    nc = c["/years/0/not_computed/income_statement/0/output"]
    assert nc["new_key"] == "/not_computed/1/pointer"
    assert nc["new_value"] == "/value/years/0/income_statement/ebitda"
    treatment = c["/years/0/not_computed/income_statement/0/permitted/0/treatment"]
    assert treatment["new_key"] == "/not_computed/1/permitted/0/pointer"
    values = c["/years/0/not_computed/income_statement/0/permitted/0/destinations/1"]
    assert values["new_key"] == "/not_computed/1/permitted/0/values/1"
    rename = c["/years/0/assignments/0/aggregate"]
    assert (rename["kind"], rename["new_key"]) == (
        "rename",
        "/years/0/assignments/0/aggregate_pointer",
    )
    field = c["/defaults_applied/0/field"]
    assert (field["section"], field["new_key"]) == ("expect_meta", "/defaults_applied/0/pointer")
    assert "/years/0/total" not in c
    text = (tmp_path / "list.md").read_text(encoding="utf-8")
    assert "| `statements.reorganise` | 1 |" in text
    assert "rename `aggregate` -> `aggregate_pointer` in `expect`" in text


def test_apply_moves_keys_and_keeps_other_lines(tmp_path: Path) -> None:
    root = make_root(tmp_path)
    list_plan(root)
    before = (root / STATEMENTS).read_text(encoding="utf-8")
    assert gm.main(["--root", str(root), "apply", STATEMENTS]) == 0
    after = (root / STATEMENTS).read_text(encoding="utf-8")
    case = yaml.safe_load(after)
    assert case["expect"] == {
        "/years/0/total": "10",
        "/years/0/assignments/0/aggregate_pointer": "/years/0/statutory/balance_sheet/assets",
    }
    assert case["expect_meta"]["/defaults_applied/0/pointer"] == (
        "/treatments/write_downs_of_receivables"
    )
    assert case["expect_diagnostics"]["/checks/3/id"] == "b_total"
    assert list(case)[-1] == "tolerance"
    # The input block and the lines that stay are kept byte for byte.
    assert before.split("expect:")[0] == after.split("expect:")[0]
    assert "  /years/0/total: '10'\n" in after
    # The untouched case and the ratios case (not named) are not changed.
    assert (root / RATIOS).read_text(encoding="utf-8") == RATIOS_CASE


def test_apply_refuses_files_outside_the_plan_or_already_changed(tmp_path: Path) -> None:
    root = make_root(tmp_path)
    list_plan(root)
    untouched = "tests/golden/tvm.npv/case.yaml"
    assert gm.main(["--root", str(root), "apply", untouched]) == 1
    assert gm.main(["--root", str(root), "apply", RATIOS]) == 0
    assert gm.main(["--root", str(root), "apply", RATIOS]) == 1  # applied already


def test_verify_accepts_the_plan_and_rejects_a_changed_value(tmp_path: Path) -> None:
    root = make_root(tmp_path)
    base = tmp_path / "base"
    shutil.copytree(root / "tests", base / "tests")
    list_plan(root)
    verify = ["--root", str(root), "verify", "--base-dir", str(base)]
    assert gm.main(verify) == 0  # nothing migrated yet: unchanged files pass
    assert gm.main(["--root", str(root), "apply", RATIOS, STATEMENTS]) == 0
    assert gm.main(verify) == 0
    path = root / RATIOS
    path.write_text(
        path.read_text(encoding="utf-8").replace("equity is negative (-200)", "other"),
        encoding="utf-8",
    )
    assert gm.main(verify) == 1
    path.write_text(RATIOS_CASE.replace("tolerance: '0'", "tolerance: '1'"), encoding="utf-8")
    assert gm.main(verify) == 1


def test_list_stops_on_keys_without_a_rule_and_apply_refuses_them(tmp_path: Path) -> None:
    root = make_root(tmp_path)
    # A ratio also listed as missing an input, with a not_defined text: no rule in the spec.
    case = RATIOS_CASE.replace(
        "  /roi/not_defined: 'null'\n", "  /ros/not_defined: needs net income\n"
    )
    (root / RATIOS).write_text(case, encoding="utf-8")
    result = json.loads(json.dumps(RATIOS_RESULT))
    result["value"]["ros"]["not_defined"] = "needs net income"

    def run(method: str, data: Any) -> dict[str, Any]:
        return result if method.startswith("ratios.") else STATEMENTS_RESULT

    md = root / "list.md"
    assert gm.main(["--root", str(root), "list", "--markdown", str(md)], run=run) == 1
    assert "/ros/not_defined" in md.read_text(encoding="utf-8").split("decision needed")[1]
    assert gm.main(["--root", str(root), "apply", RATIOS]) == 1


def test_list_refuses_an_engine_that_writes_diagnostics(tmp_path: Path) -> None:
    root = make_root(tmp_path)

    def run(method: str, data: Any) -> dict[str, Any]:
        return {"value": {}, "diagnostics": {}}

    with pytest.raises(SystemExit, match="already writes"):
        gm.main(["--root", str(root), "list"], run=run)


def test_pointer_order_reads_indices_as_numbers() -> None:
    pointers = ["/value/dscr/10", "/value/dscr/2", "/value/b", "/value/a/0"]
    assert sorted(pointers, key=gm.pointer_key) == [
        "/value/a/0",
        "/value/b",
        "/value/dscr/2",
        "/value/dscr/10",
    ]
