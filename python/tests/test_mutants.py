"""Mutation testing tooling: exclusions file, per-module summary and the pull-request gate."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

import pytest

pytest.importorskip("tomllib")  # the script runs on Python 3.11 or later (CI uses 3.13)

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("mutants", ROOT / "scripts/mutants.py")
assert _spec and _spec.loader
mt = importlib.util.module_from_spec(_spec)
sys.modules["mutants"] = mt  # dataclasses resolve types through sys.modules
_spec.loader.exec_module(mt)

RATES = "crates/fairmark-core/src/rates.rs"


def outcome(file: str, line: int, function: str, mutation: str, summary: str) -> dict[str, Any]:
    return {
        "scenario": {
            "Mutant": {
                "name": f"{file}:{line}:5: {mutation}",
                "package": "fairmark-core",
                "file": file,
                "function": {"function_name": function, "return_type": "", "span": {}},
                "span": {"start": {"line": line, "column": 5}, "end": {"line": line, "column": 9}},
                "replacement": "",
                "genre": "BinaryOperator",
            }
        },
        "summary": summary,
    }


def write_run(directory: Path, outcomes: list[dict[str, Any]], baseline: str = "Success") -> Path:
    out = directory / "mutants.out"
    out.mkdir(parents=True)
    data = {"outcomes": [{"scenario": "Baseline", "summary": baseline}, *outcomes]}
    (out / "outcomes.json").write_text(json.dumps(data), encoding="utf-8")
    return out


def exclusions(tmp_path: Path, monkeypatch: pytest.MonkeyPatch, body: str) -> None:
    path = tmp_path / "equivalent.toml"
    path.write_text(body, encoding="utf-8")
    monkeypatch.setattr(mt, "EXCLUSIONS", path)


def test_module_names() -> None:
    assert mt.module_of("crates/fairmark-core/src/dcf/engine.rs") == "dcf/engine"
    assert mt.module_of("crates/fairmark-core/src/tv/mod.rs") == "tv"
    assert mt.module_of("crates/fairmark-core/src/tvm.rs") == "tvm"


def test_committed_exclusions_file_is_valid() -> None:
    entries, problems = mt.load_exclusions()
    assert problems == []
    assert all(e.reason and e.reviewer for e in entries)


def test_entry_needs_reason_reviewer_and_one_function(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    exclusions(
        tmp_path,
        monkeypatch,
        f'[[equivalent]]\nfile = "{RATES}"\nmutation = "replace + with - in f"\nreason = ""\n',
    )
    _, problems = mt.load_exclusions()
    assert any(
        all(key in p for key in ("function", "line", "reason", "reviewer")) for p in problems
    )


def test_entry_outside_the_core_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    exclusions(
        tmp_path,
        monkeypatch,
        '[[equivalent]]\nfile = "crates/fairmark-cli/src/main.rs"\nfunction = "main"\n'
        'mutation = "replace main with ()"\nline = 1\nreason = "r"\nreviewer = "x"\n',
    )
    _, problems = mt.load_exclusions()
    assert any("under crates/fairmark-core/src/" in p for p in problems)


@pytest.mark.parametrize(
    ("reviewer", "valid"),
    [
        ("salvo8519", True),
        ("a-b-1", True),
        ("coordinator review pending", False),
        ("pending: maintainer", False),
        ("@salvo8519", False),
    ],
)
def test_reviewer_must_be_a_github_login(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, reviewer: str, valid: bool
) -> None:
    exclusions(
        tmp_path,
        monkeypatch,
        f'[[equivalent]]\nfile = "{RATES}"\nfunction = "f"\n'
        f'mutation = "replace + with - in f"\nline = 1\nreason = "r"\nreviewer = "{reviewer}"\n',
    )
    _, problems = mt.load_exclusions()
    assert any("reviewer must be the GitHub login" in p for p in problems) is not valid


def test_gate_fails_on_missed_and_passes_when_recorded_as_equivalent(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    missed = "replace <= with < in f"
    run = write_run(
        tmp_path / "run",
        [
            outcome(RATES, 10, "f", missed, "MissedMutant"),
            outcome(RATES, 12, "f", "replace + with - in f", "CaughtMutant"),
            outcome(RATES, 14, "g", "replace g -> u32 with 0", "Unviable"),
        ],
    )
    exclusions(tmp_path, monkeypatch, "equivalent = []\n")
    assert mt.main(["summary", str(run), "--gate"]) == 1
    text = capsys.readouterr().out
    assert "| rates | 3 | 1 | 1 | 0 | 1 | 0 | 0 |" in text
    assert f"{RATES}:10: {missed}" in text

    exclusions(
        tmp_path,
        monkeypatch,
        f'[[equivalent]]\nfile = "{RATES}"\nfunction = "f"\nmutation = "{missed}"\n'
        'line = 10\nreason = "both branches return the same value at the boundary"\n'
        'reviewer = "x"\n',
    )
    assert mt.main(["summary", str(run), "--gate"]) == 0
    assert "| rates | 3 | 1 | 0 | 1 | 1 | 0 | 0 |" in capsys.readouterr().out


def test_entry_pinned_to_a_line_matches_only_that_line(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    mutation = "replace + with - in f"
    run = write_run(
        tmp_path / "run",
        [
            outcome(RATES, 10, "f", mutation, "MissedMutant"),
            outcome(RATES, 20, "f", mutation, "MissedMutant"),
        ],
    )
    exclusions(
        tmp_path,
        monkeypatch,
        f'[[equivalent]]\nfile = "{RATES}"\nfunction = "f"\nmutation = "{mutation}"\n'
        'line = 10\nreason = "r"\nreviewer = "x"\n',
    )
    assert mt.main(["summary", str(run), "--gate"]) == 1


def entry(function: str, mutation: str, line: int = 10) -> str:
    return (
        f'[[equivalent]]\nfile = "{RATES}"\nfunction = "{function}"\nmutation = "{mutation}"\n'
        f'line = {line}\nreason = "r"\nreviewer = "x"\n'
    )


def test_stale_entry_is_reported_when_its_mutant_is_caught(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    mutation = "replace <= with < in f"
    run = write_run(tmp_path / "run", [outcome(RATES, 10, "f", mutation, "CaughtMutant")])
    exclusions(tmp_path, monkeypatch, entry("f", mutation))
    assert mt.main(["summary", str(run), "--full"]) == 0
    text = capsys.readouterr().out
    assert "match no missed mutant" in text
    assert mutation in text.split("match no missed mutant", 1)[1]


def test_entry_still_equivalent_is_not_stale(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    mutation = "replace <= with < in f"
    run = write_run(tmp_path / "run", [outcome(RATES, 10, "f", mutation, "MissedMutant")])
    exclusions(tmp_path, monkeypatch, entry("f", mutation))
    assert mt.main(["summary", str(run), "--full", "--gate"]) == 0
    assert "match no missed mutant" not in capsys.readouterr().out


@pytest.mark.parametrize(
    ("function", "mutation", "accepted"),
    [
        ("f", "replace <= with < in f", True),
        ("Outputs::note", "delete match arm Basis::Gross in Outputs::note", True),
        ("f", "replace f -> Result<Decimal> with Ok(Default::default())", True),
        ("f", "replace f with ()", True),
        ("f", "replace <= with < in g", False),
        ("f", "replace <= with < in ff", False),
        ("f", "replace g -> u32 with 0", False),
    ],
)
def test_mutation_must_belong_to_its_function(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, function: str, mutation: str, accepted: bool
) -> None:
    exclusions(tmp_path, monkeypatch, entry(function, mutation))
    _, problems = mt.load_exclusions()
    assert (problems == []) is accepted, problems


def test_line_is_required_and_duplicates_are_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    no_line = f'[[equivalent]]\nfile = "{RATES}"\nfunction = "f"\n'
    no_line += 'mutation = "replace + with - in f"\nreason = "r"\nreviewer = "x"\n'
    exclusions(tmp_path, monkeypatch, no_line)
    _, problems = mt.load_exclusions()
    assert any("line" in p for p in problems)

    twice = entry("f", "replace + with - in f") + entry("f", "replace + with - in f")
    exclusions(tmp_path, monkeypatch, twice)
    _, problems = mt.load_exclusions()
    assert any("duplicate" in p for p in problems)


def test_shards_are_merged_and_a_failing_baseline_fails(tmp_path: Path) -> None:
    a = write_run(tmp_path / "a", [outcome(RATES, 1, "f", "replace a", "CaughtMutant")])
    b = write_run(tmp_path / "b", [outcome(RATES, 2, "f", "replace b", "Timeout")])
    assert mt.main(["summary", str(a), str(b)]) == 0
    broken = write_run(tmp_path / "c", [], baseline="Failure")
    assert mt.main(["summary", str(a), str(broken)]) == 1


def test_missing_outcomes(tmp_path: Path) -> None:
    assert mt.main(["summary", str(tmp_path), "--gate", "--allow-empty"]) == 0
    with pytest.raises(SystemExit, match="did not complete"):
        mt.main(["summary", str(tmp_path)])
