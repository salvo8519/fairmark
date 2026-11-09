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
    assert any("function" in p and "reason" in p and "reviewer" in p for p in problems)


def test_entry_outside_the_core_is_rejected(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    exclusions(
        tmp_path,
        monkeypatch,
        '[[equivalent]]\nfile = "crates/fairmark-cli/src/main.rs"\nfunction = "main"\n'
        'mutation = "replace main with ()"\nreason = "r"\nreviewer = "x"\n',
    )
    _, problems = mt.load_exclusions()
    assert any("under crates/fairmark-core/src/" in p for p in problems)


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
        'reason = "both branches return the same value at the boundary"\nreviewer = "x"\n',
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
