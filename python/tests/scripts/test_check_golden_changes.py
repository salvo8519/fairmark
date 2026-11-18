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


SECTIONS = {
    "expect_diagnostics": "{/warnings/0/code: FM-W001}",
    "expect_diagnostics_length": "{/warnings: '1'}",
    "expect_counts": "{warnings: '1'}",
    "expect_defaults": "{/timing: {/value: end_of_period}}",
    "expect_not_computed": "{/value/roe: {/reason_code: not_defined}}",
    "expect_warnings": "{'FM-W001 -': {}}",
    "expect_checks": "{'/value a': {/status: reconciled}}",
    "expect_errors": "{'FM-E001 /rate': {/reason: missing}}",
}


@pytest.mark.parametrize("section", sorted(SECTIONS))
def test_every_assertion_section_is_protected(
    repo: Path, monkeypatch: pytest.MonkeyPatch, section: str
) -> None:
    # SPEC-0013: the semantic sections, and the diagnostics sections of SPEC-0012, are
    # expectations like `expect`.
    write(repo, {"tests/golden/tvm.npv/a.yaml": CASE + f"{section}: {SECTIONS[section]}\n"})
    git(repo, "add", ".")
    git(repo, "commit", "-q", "-m", "base")
    git(repo, "switch", "-q", "-c", "work")
    write(repo, {"tests/golden/tvm.npv/a.yaml": CASE + f"{section}: {{}}\n"})
    commit(repo)
    with pytest.raises(SystemExit, match="numerical-change"):
        run(monkeypatch)


def test_a_file_that_is_not_yaml_is_not_a_case(
    branch: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    # Only YAML files under tests/golden are golden cases.
    write(branch, {"tests/golden/notes.md": "# Notes\n"})
    commit(branch)
    run(monkeypatch)
    assert "No existing golden case changed." in capsys.readouterr().out
