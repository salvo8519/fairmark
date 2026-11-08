"""scripts/smoke_wheel.py, the check the Release workflow runs on every built package."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("smoke_wheel", ROOT / "scripts/smoke_wheel.py")
assert _spec and _spec.loader
smoke = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(smoke)

CASE = {
    "method": "tvm.npv",
    "input": {"rate": "0.09", "cash_flows": ["0", "120", "135", "150", "160", "170"]},
    "expect": {"npv": "563.3824336605614874662835655"},
    "tolerance": "0.00000000000000000001",
}


def test_matching_case_passes() -> None:
    assert smoke.check_case(CASE) == []


def test_different_value_is_reported() -> None:
    wrong = {**CASE, "expect": {"npv": "563.38"}}
    [problem] = smoke.check_case(wrong)
    assert problem.startswith("npv: 563.382433660561487466283565")
    assert problem.endswith(" != 563.38")


def test_missing_error_is_reported() -> None:
    case = {"method": "tvm.npv", "input": CASE["input"], "expect_error": "FM-E001"}
    assert smoke.check_case(case) == ["no error, expected FM-E001"]


def test_expected_version_is_the_python_form_of_cargo() -> None:
    out = subprocess.run(
        [sys.executable, "scripts/version.py", "show", "--python"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=True,
    )
    assert smoke.expected_version() == out.stdout.strip()


def test_source_tree_import_is_refused() -> None:
    """In the development environment fairmark comes from python/: the check must see it."""
    out = subprocess.run(
        [sys.executable, "-I", "scripts/smoke_wheel.py"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if out.returncode == 0:  # an installed, non-editable build: nothing to assert
        return
    assert "imported from the source tree" in out.stderr
