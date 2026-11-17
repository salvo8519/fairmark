"""scripts/check_golden_changes.py: an existing golden case cannot change without the label."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest
from conftest import git, load_script, write

check = load_script("check_golden_changes")
CASE = (
    "method: tvm.npv\nsource: reference script\n"
    "input: {rate: '0.1'}\nexpect: {npv: '1'}\ntolerance: '0'\n"
)


@pytest.fixture
def branch(repo: Path) -> Path:
    write(repo, {"tests/golden/tvm.npv/a.yaml": CASE})
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "base")
    git(repo, "switch", "-q", "-c", "work")
    return repo


def run(monkeypatch: pytest.MonkeyPatch, *extra: str) -> None:
    monkeypatch.setattr(sys, "argv", ["check_golden_changes.py", "--base", "main", *extra])
    check.main()


def commit(repo: Path) -> None:
    git(repo, "add", "-A")
    git(repo, "commit", "-q", "-m", "change")


def test_a_changed_expectation_fails(branch: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    write(branch, {"tests/golden/tvm.npv/a.yaml": CASE.replace("npv: '1'", "npv: '2'")})
    commit(branch)
    with pytest.raises(SystemExit, match="numerical-change"):
        run(monkeypatch)


def test_the_label_allows_it(
    branch: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    write(branch, {"tests/golden/tvm.npv/a.yaml": CASE.replace("npv: '1'", "npv: '2'")})
    commit(branch)
    run(monkeypatch, "--allowed")
    assert "changed 'expect': tests/golden/tvm.npv/a.yaml" in capsys.readouterr().out


def test_a_deleted_case_fails(branch: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    (branch / "tests/golden/tvm.npv/a.yaml").unlink()
    commit(branch)
    with pytest.raises(SystemExit):
        run(monkeypatch)


def test_a_new_case_needs_a_source(branch: Path, monkeypatch: pytest.MonkeyPatch) -> None:
    write(branch, {"tests/golden/tvm.npv/b.yaml": CASE.replace("source: reference script\n", "")})
    commit(branch)
    with pytest.raises(SystemExit, match="independent 'source'"):
        run(monkeypatch)


def test_a_new_sourced_case_and_a_description_change_pass(
    branch: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    write(
        branch,
        {
            "tests/golden/tvm.npv/b.yaml": CASE,
            "tests/golden/tvm.npv/a.yaml": "description: clearer\n" + CASE,
        },
    )
    commit(branch)
    run(monkeypatch)
    assert "No existing golden case changed." in capsys.readouterr().out
