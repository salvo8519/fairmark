"""The status checks required on main stay in step with the workflows and the ruleset."""

from __future__ import annotations

import importlib.util
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location(
    "required_checks", ROOT / "scripts/required_checks.py"
)
assert _spec and _spec.loader
rc = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(rc)

WORKFLOW = """
name: CI
on:
  push:
    branches: [main]
  pull_request:
jobs:
  rust:
    name: Rust · MSRV 1.88
    runs-on: ubuntu-latest
    steps: [{run: "true"}]
  python:
    name: Python ${{ matrix.python }} · ${{ matrix.os }}
    runs-on: ${{ matrix.os }}
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest]
        python: ["3.10", "3.13"]
        exclude:
          - {os: windows-latest, python: "3.10"}
    steps: [{run: "true"}]
  deploy:
    name: Deploy
    if: github.event_name != 'pull_request'
    runs-on: ubuntu-latest
    steps: [{run: "true"}]
"""

FILTERED = """
name: Docs
on:
  pull_request:
    paths: ["docs/**"]
jobs:
  build:
    name: Build docs
    runs-on: ubuntu-latest
    steps: [{run: "true"}]
"""


def workflows(tmp_path: Path) -> Path:
    (tmp_path / "ci.yml").write_text(WORKFLOW, encoding="utf-8")
    (tmp_path / "docs.yml").write_text(FILTERED, encoding="utf-8")
    return tmp_path


def test_matrix_names_are_expanded(tmp_path: Path) -> None:
    produced = rc.produced_on_pull_requests(workflows(tmp_path))
    assert {"Python 3.10 · ubuntu-latest", "Python 3.13 · ubuntu-latest"} <= set(produced)
    assert "Python 3.13 · windows-latest" in produced
    assert "Python 3.10 · windows-latest" not in produced  # excluded


def test_jobs_that_never_run_on_pull_requests_do_not_count(tmp_path: Path) -> None:
    produced = rc.produced_on_pull_requests(workflows(tmp_path))
    assert "Deploy" not in produced
    assert "Build docs" not in produced  # path filter: skipped on some pull requests


def test_a_renamed_job_is_reported(tmp_path: Path) -> None:
    produced = rc.produced_on_pull_requests(workflows(tmp_path))
    problems = rc.check(["Rust · MSRV 1.86", "Python 3.13 · ubuntu-latest"], produced)
    assert len(problems) == 1 and "Rust · MSRV 1.86" in problems[0]


def test_duplicates_are_reported() -> None:
    problems = rc.check(["Area labels", "Area labels"], {"Area labels": "pr.yml"})
    assert problems == ["'Area labels' is listed more than once"]


def test_comparison_with_the_ruleset() -> None:
    assert rc.compare(["A", "B"], {"A", "B"}) == []
    problems = rc.compare(["A", "B"], {"A", "C"})
    assert any("'B' is listed but the ruleset does not require it" in p for p in problems)
    assert any("'C' is required by the ruleset but missing" in p for p in problems)


def test_the_committed_list_matches_the_workflows() -> None:
    assert rc.check(rc.required(), rc.produced_on_pull_requests()) == []
