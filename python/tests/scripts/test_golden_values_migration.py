"""scripts/golden_values_migration.py (SPEC-0013 R13, AC13), on temporary trees only.

The engine is replaced by results written here (`ENGINE`), so the tests do not depend on the
engine version; `inventory` reads the committed schema snapshots of the repository.
"""

from __future__ import annotations

import json
from pathlib import Path
from typing import Any

import pytest
import yaml
from conftest import load_script

gvm = load_script("golden_values_migration")

RATIOS = """method: ratios.profitability
description: temporary ratios case
source: written for the test
input:
  revenue: '100'
expect:
  /roe/value: 'null'
expect_diagnostics:
  /not_computed/0/pointer: /value/roe
  /not_computed/0/reason_code: not_defined
  /not_computed/0/reason: equity is negative (-200)
  /not_computed/1/reason_code: missing_input
  /not_computed/1/missing_any_of/0/0: /revenue
expect_diagnostics_length:
  /not_computed: '2'
  /not_computed/1/missing_any_of: '1'
tolerance: '0'
"""
STATEMENTS = """method: statements.reorganise
description: temporary statements case
source: written for the test
input:
  years: []
expect:
  /years/0/form: statutory
expect_meta:
  /defaults_applied/0/pointer: /treatments/a
  /defaults_applied/0/value: operating
  /defaults_applied/1/value: investing
expect_diagnostics:
  /checks/1/status: not_checked
  /warnings/0/code: FM-W012
tolerance: '0'
"""
ERROR = """method: tvm.npv
description: temporary error case
source: written for the test
input:
  rat: '0.1'
expect_error: FM-E001
expect_error_report:
  /pointer: /cash_flows
  /errors/1/reason: unknown_field
expect_length:
  /errors: '2'
"""
UNTOUCHED = """method: tvm.npv
description: untouched
source: written for the test
input: {}
expect:
  npv: '1'
tolerance: '0'
"""
FILES = {
    "ratios.profitability/case.yaml": RATIOS,
    "statements.reorganise/case.yaml": STATEMENTS,
    "tvm.npv/error.yaml": ERROR,
    "tvm.npv/untouched.yaml": UNTOUCHED,
}
RESULTS: dict[str, dict[str, Any]] = {
    "ratios.profitability": {
        "value": {"roe": {"value": None}},
        "meta": {"defaults_applied": []},
        "diagnostics": {
            "not_computed": [
                {
                    "pointer": "/value/roe",
                    "reason_code": "not_defined",
                    "reason": "equity is negative (-200)",
                    "missing_any_of": [],
                    "permitted": [],
                },
                {
                    "pointer": "/value/ros",
                    "reason_code": "missing_input",
                    "reason": "r",
                    "missing_any_of": [["/revenue"]],
                    "permitted": [],
                },
            ],
            "warnings": [],
            "checks": [],
        },
    },
    "statements.reorganise": {
        "value": {"years": [{"form": "statutory"}]},
        "meta": {
            "defaults_applied": [
                {"pointer": "/treatments/a", "value": "operating", "reason": "r"},
                {"pointer": "/treatments/b", "value": "investing", "reason": "r"},
            ]
        },
        "diagnostics": {
            "not_computed": [],
            "warnings": [{"code": "FM-W012", "pointer": None, "message": "m"}],
            "checks": [
                {
                    "id": "a_total",
                    "status": "reconciled",
                    "pointer": "/value/years/0",
                    "pointers": [],
                },
                {
                    "id": "b_total",
                    "status": "not_checked",
                    "pointer": "/value/years/0",
                    "pointers": [],
                },
            ],
        },
    },
    "tvm.npv": {
        "error": {
            "code": "FM-E001",
            "pointer": "/cash_flows",
            "errors": [
                {"code": "FM-E001", "pointer": "/cash_flows", "reason": "missing"},
                {"code": "FM-E001", "pointer": "/rat", "reason": "unknown_field"},
            ],
        }
    },
}
ROOT_DIRS = ("ratios.profitability", "statements.reorganise", "tvm.npv")


def fake_engine(method: str, data: Any) -> dict[str, Any]:
    return RESULTS[method]


@pytest.fixture
def tree(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Path:
    monkeypatch.setattr(gvm, "ENGINE", fake_engine)
    for rel, text in FILES.items():
        path = tmp_path / "tests/golden" / rel
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8")
    return tmp_path


def run(root: Path, *args: str) -> int:
    plan = str(root / "plan.json")
    command, *rest = args
    extra = ["--base-root", str(root / "base")] if command == "verify" else []
    return int(gvm.main([command, *rest, "--root", str(root), "--plan", plan, *extra]))


def planned(root: Path) -> dict[str, Any]:
    assert run(root, "list", "--markdown", str(root / "list.md")) == 0
    plan: dict[str, Any] = json.loads((root / "plan.json").read_text(encoding="utf-8"))
    return plan


def migrate(root: Path) -> dict[str, Any]:
    plan = planned(root)
    base = root / "base/tests/golden"
    base.parent.mkdir(parents=True)
    import shutil

    shutil.copytree(root / "tests/golden", base)
    assert run(root, "apply", *plan["files"]) == 0
    return plan


def golden(root: Path, rel: str) -> Path:
    return root / "tests/golden" / rel


def test_list_names_every_entry_by_its_identity(tree: Path) -> None:
    plan = planned(tree)
    files = plan["files"]
    assert sorted(files) == [
        "tests/golden/ratios.profitability/case.yaml",
        "tests/golden/statements.reorganise/case.yaml",
        "tests/golden/tvm.npv/error.yaml",
    ]
    assert files["tests/golden/ratios.profitability/case.yaml"]["add"] == {
        "expect_counts": {"not_computed": "2"},
        "expect_not_computed": {
            "/value/roe": {"/reason": "equity is negative (-200)", "/reason_code": "not_defined"},
            "/value/ros": {
                "/missing_any_of#count": "1",
                "/missing_any_of/0/0": "/revenue",
                "/reason_code": "missing_input",
            },
        },
    }
    statements = files["tests/golden/statements.reorganise/case.yaml"]["add"]
    assert statements["expect_defaults"] == {
        "/treatments/a": {"/value": "operating"},
        "/treatments/b": {"/value": "investing"},
    }
    assert statements["expect_checks"] == {"/value/years/0 b_total": {"/status": "not_checked"}}
    assert statements["expect_warnings"] == {"FM-W012 -": {}}
    error = files["tests/golden/tvm.npv/error.yaml"]["add"]
    assert error == {
        "expect_counts": {"errors": "2"},
        "expect_errors": {"FM-E001 /rat": {"/reason": "unknown_field"}},
    }
    assert "Files that change: 3." in (tree / "list.md").read_text(encoding="utf-8")


def test_apply_moves_keys_and_keeps_every_other_line(tree: Path) -> None:
    migrate(tree)
    text = golden(tree, "tvm.npv/error.yaml").read_text(encoding="utf-8")
    # The top-level fields of the report stay where they are; `/errors/1/...` moves.
    assert "expect_error_report:\n  /pointer: /cash_flows\n" in text
    assert "/errors/1/reason" not in text and "expect_length" not in text
    for rel, old in FILES.items():
        new = golden(tree, rel).read_text(encoding="utf-8")
        for key, block in gvm.blocks(old):
            if key in ("", "method", "description", "source", "input", "expect", "tolerance"):
                assert block in new, (rel, key)
    assert golden(tree, "tvm.npv/untouched.yaml").read_text(encoding="utf-8") == UNTOUCHED
    case = yaml.safe_load(golden(tree, "ratios.profitability/case.yaml").read_text())
    assert "expect_diagnostics" not in case and "expect_diagnostics_length" not in case


def test_apply_refuses_a_file_outside_the_plan_or_changed_since(tree: Path) -> None:
    planned(tree)
    assert run(tree, "apply", "tests/golden/tvm.npv/untouched.yaml") == 1
    path = golden(tree, "tvm.npv/error.yaml")
    path.write_text(ERROR + "# edited\n", encoding="utf-8")
    assert run(tree, "apply", "tests/golden/tvm.npv/error.yaml") == 1
    assert path.read_text(encoding="utf-8") == ERROR + "# edited\n"


def test_verify_is_clean_after_apply(tree: Path, capsys: pytest.CaptureFixture[str]) -> None:
    migrate(tree)
    assert run(tree, "verify") == 0
    assert "3 checked, 0 problems" in capsys.readouterr().out


TAMPERING = [
    pytest.param(
        "ratios.profitability/case.yaml",
        "/reason_code: not_defined",
        "/reason_code: not_applicable",
        "expected 'not_defined'",
        id="value changed",
    ),
    pytest.param(
        "ratios.profitability/case.yaml",
        "  /value/ros:",
        "  /value/roi:",
        "do not resolve",
        id="key names no entry",
    ),
    pytest.param(
        "statements.reorganise/case.yaml",
        "/value: operating",
        "/value: TEMP",
        "expected 'operating'",
        id="values swapped between entries",
    ),
    pytest.param(
        "statements.reorganise/case.yaml",
        "tolerance: '0'",
        "tolerance: '1'",
        "tolerance: block changed",
        id="block changed",
    ),
    pytest.param(
        "tvm.npv/error.yaml",
        "    /reason: unknown_field\n",
        "    /reason: unknown_field\n    /code: FM-E001\n",
        "not the section of the plan",
        id="entry added",
    ),
]


@pytest.mark.parametrize(("rel", "old", "new", "message"), TAMPERING)
def test_verify_reports_a_tampered_file(
    tree: Path, capsys: pytest.CaptureFixture[str], rel: str, old: str, new: str, message: str
) -> None:
    migrate(tree)
    path = golden(tree, rel)
    text = path.read_text(encoding="utf-8")
    assert old in text
    text = text.replace(old, new, 1)
    if new == "/value: TEMP":  # finish the swap: the other entry gets the first value
        text = text.replace("/value: investing", "/value: operating").replace(
            "/value: TEMP", "/value: investing"
        )
    path.write_text(text, encoding="utf-8")
    assert run(tree, "verify", f"tests/golden/{rel}") == 1
    assert message in capsys.readouterr().out


def test_list_on_a_migrated_tree_finds_nothing(tree: Path) -> None:
    migrate(tree)
    (tree / "plan.json").unlink()
    assert planned(tree)["files"] == {}


def test_inventory_classifies_values_shape_and_wording(tree: Path) -> None:
    out = tree / "inventory.json"
    assert (
        gvm.main(
            [
                "inventory",
                "--root",
                str(tree),
                "--json",
                str(out),
                "--markdown",
                str(tree / "inventory.md"),
            ]
        )
        == 0
    )
    rows = {
        (r["file"].split("/", 2)[-1], r["section"], r["key"]): r
        for r in json.loads(out.read_text(encoding="utf-8"))
    }
    klass = {k: v["klass"] for k, v in rows.items()}
    assert klass[("ratios.profitability/case.yaml", "expect", "/roe/value")] == "VALUE"
    assert (
        klass[("ratios.profitability/case.yaml", "expect_diagnostics", "/not_computed/0/reason")]
        == "WORDING"
    )
    assert klass[("tvm.npv/error.yaml", "expect_error", "")] == "VALUE"
    assert klass[("tvm.npv/error.yaml", "expect_length", "/errors")] == "VALUE"
    assert (
        rows[("statements.reorganise/case.yaml", "expect_meta", "/defaults_applied/0/pointer")][
            "path_kind"
        ]
        == "keyed position"
    )
    assert "Golden files: 4." in (tree / "inventory.md").read_text(encoding="utf-8")
