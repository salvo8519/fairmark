"""Mutation testing tooling: exclusions file, per-module summary and the pull-request gate."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[3]
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


# ---------------------------------------------------------------- budget, rotation, coverage
# Issue #368: runs fit a time budget, the weekly slices cover the core, and an incomplete run
# fails explicitly.


def name(n: int) -> str:
    return f"{RATES}:{n}:5: replace a"


def test_committed_budget_file_is_valid() -> None:
    assert mt.budget_problems() == []
    assert mt.main(["check-slices"]) == 0


def budget_file(tmp_path: Path, slices: str) -> Path:
    path = tmp_path / "budget.toml"
    path.write_text(
        "[estimate]\nseconds_per_mutant = 30\nbaseline_minutes = 3\nsetup_minutes = 10\n"
        "[pr]\nrun_minutes = 30\nmax_shards = 8\n[weekly]\nrun_minutes = 45\nmax_shards = 4\n"
        + slices,
        encoding="utf-8",
    )
    return path


def test_every_source_file_is_in_exactly_one_slice(tmp_path: Path) -> None:
    src = tmp_path / "src"
    (src / "tv").mkdir(parents=True)
    for f in ("lib.rs", "tvm.rs", "tv/gordon.rs"):
        (src / f).write_text("", encoding="utf-8")
    ok = budget_file(
        tmp_path,
        '[[slice]]\nname = "a"\nfiles = ["lib.rs", "tvm.rs"]\n'
        '[[slice]]\nname = "b"\nfiles = ["tv/*.rs"]\nparts = 2\n',
    )
    assert mt.budget_problems(ok, src) == []
    assert [s.label for s in mt.load_slices(ok)] == ["a", "b (part 1 of 2)", "b (part 2 of 2)"]
    gap = budget_file(tmp_path, '[[slice]]\nname = "a"\nfiles = ["lib.rs", "tv/*.rs"]\n')
    assert any("tvm.rs is in no slice" in p for p in mt.budget_problems(gap, src))
    twice = budget_file(
        tmp_path,
        '[[slice]]\nname = "a"\nfiles = ["*.rs", "tv/*.rs"]\n'
        '[[slice]]\nname = "b"\nfiles = ["tvm.rs"]\n',
    )
    assert any("tvm.rs is in more than one slice" in p for p in mt.budget_problems(twice, src))
    nothing = budget_file(tmp_path, '[[slice]]\nname = "a"\nfiles = ["*.rs", "tv/*.rs", "x.rs"]\n')
    assert any("x.rs matches no source file" in p for p in mt.budget_problems(nothing, src))


def test_mutants_per_shard_and_job_limit_follow_the_budget() -> None:
    b = mt.Budget(
        seconds_per_mutant=30, baseline_minutes=3, setup_minutes=10, run_minutes=45, max_shards=4
    )
    assert b.per_shard == 84  # (45 - 3) * 60 / 30
    assert b.timeout_minutes == 55


@pytest.mark.parametrize(("m", "shards"), [(1, 1), (1, 4), (3, 1), (3, 2), (4, 3)])
def test_shard_of_a_part_is_round_robin_within_the_part(m: int, shards: int) -> None:
    """Shard j of S of part k of m selects exactly round-robin shard j of S of part k."""
    items = list(range(47))
    for k in range(m):
        part_items = [i for i in items if i % m == k]
        for j in range(shards):
            a, n = (
                int(x) for x in mt.shard_arg(j, shards, f"{k}/{m}" if m > 1 else None).split("/")
            )
            got = [i for i in items if i % n == a]
            want = [x for t, x in enumerate(part_items) if t % shards == j]
            assert got == want


def test_plan_sizes_the_shards_and_refuses_more_than_the_budget(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    listed = tmp_path / "all.txt"
    listed.write_text("".join(name(i) + "\n" for i in range(100)), encoding="utf-8")
    out = tmp_path / "out"
    assert (
        mt.main(
            [
                "plan",
                "--kind",
                "weekly",
                "--list",
                str(listed),
                "--part",
                "1/3",
                "--github-output",
                str(out),
                "--shards-file",
                str(tmp_path / "s.txt"),
            ]
        )
        == 0
    )
    plan = json.loads(capsys.readouterr().out)
    assert (plan["mutants"], plan["shards"], plan["per_shard"]) == (100, 2, 84)
    assert [m["shard"] for m in plan["matrix"]] == ["1/6", "4/6"]
    assert "shards=2" in out.read_text(encoding="utf-8")
    assert (tmp_path / "s.txt").read_text(encoding="utf-8") == "0 1/6\n1 4/6\n"
    listed.write_text("".join(name(i) + "\n" for i in range(337)), encoding="utf-8")
    assert mt.main(["plan", "--kind", "weekly", "--list", str(listed)]) == 1
    assert "Split this slice" in capsys.readouterr().err
    empty = tmp_path / "none.txt"
    empty.write_text("", encoding="utf-8")
    assert mt.main(["plan", "--kind", "pr", "--list", str(empty)]) == 0
    assert json.loads(capsys.readouterr().out)["shards"] == 0


def test_the_slice_of_a_week_rotates_through_the_cycle() -> None:
    n = len(mt.load_slices())
    monday = mt.dt.date(2026, 11, 23)
    first = mt.week_number(monday)
    assert mt.week_number(monday + mt.dt.timedelta(days=6)) == first  # Sunday, same week
    assert mt.week_number(monday + mt.dt.timedelta(days=7)) == first + 1
    assert mt.week_number(mt.EPOCH_MONDAY) == 0
    seen = {(mt.week_number(monday + mt.dt.timedelta(weeks=w))) % n for w in range(n)}
    assert seen == set(range(n))


def test_a_shard_that_stops_early_lists_its_untested_mutants(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    expected = tmp_path / "shard-0.txt"
    expected.write_text("".join(name(i) + "\n" for i in (1, 2, 3)), encoding="utf-8")
    out = write_run(tmp_path / "run", [outcome(RATES, 1, "f", "replace a", "CaughtMutant")])
    args = ["coverage", str(expected), str(out), "--run-minutes", "45"]
    assert mt.main([*args, "--exit-status", "124"]) == 1
    text = capsys.readouterr().out
    assert "Mutation coverage incomplete" in text
    assert "the time budget of 45 minutes was reached" in text
    assert f"- `{name(2)}`" in text and f"- `{name(3)}`" in text and name(1) + "`" not in text
    status = json.loads((out / mt.STATUS_FILE).read_text(encoding="utf-8"))
    assert (status["expected"], status["tested"]) == (3, 1)
    expected.write_text(name(1) + "\n", encoding="utf-8")
    assert mt.main([*args, "--exit-status", "0"]) == 0


def run_jobs(tmp_path: Path, jobs: list[tuple[int, str, str]]) -> Path:
    path = tmp_path / "jobs.json"
    data = {
        "jobs": [
            {"id": i, "name": f"Mutation testing · weekly slice / {n}", "conclusion": c}
            for i, n, c in jobs
        ]
    }
    path.write_text(json.dumps(data), encoding="utf-8")
    return path


def test_completeness_tells_a_lost_runner_from_a_timeout(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    plan = tmp_path / "plan"
    plan.mkdir()
    (plan / "plan.json").write_text(json.dumps({"mutants": 4, "shards": 4}), encoding="utf-8")
    for shard in range(4):
        (plan / f"shard-{shard}.txt").write_text(name(shard) + "\n", encoding="utf-8")
    shards = tmp_path / "shards"
    # Shard 0 complete; shard 1 lost its runner; shard 2 hit the job limit; shard 3 stopped at
    # its time budget and wrote its status.
    done = write_run(
        shards / "mutants-shard-0", [outcome(RATES, 0, "f", "replace a", "CaughtMutant")]
    )
    (done / mt.STATUS_FILE).write_text(json.dumps({"reason": "complete"}), encoding="utf-8")
    stopped = shards / "mutants-shard-3" / "mutants.out"
    stopped.mkdir(parents=True)
    reason = "timeout: the time budget of 45 minutes was reached"
    (stopped / mt.STATUS_FILE).write_text(json.dumps({"reason": reason}), encoding="utf-8")
    jobs = run_jobs(
        tmp_path,
        [
            (10, "Shard 0 of 4", "success"),
            (11, "Shard 1 of 4", "failure"),
            (12, "Shard 2 of 4", "cancelled"),
            (13, "Shard 3 of 4", "failure"),
        ],
    )
    logs = tmp_path / "logs"
    logs.mkdir()
    (logs / "11.log").write_text(
        "##[error]The runner has received a shutdown signal. This can happen when ...",
        encoding="utf-8",
    )
    (logs / "12.log").write_text(
        "##[error]The job has exceeded the maximum execution time of 55m0s", encoding="utf-8"
    )
    assert mt.main(["failed-jobs", str(jobs)]) == 0
    assert capsys.readouterr().out.split() == ["11", "12", "13"]
    argv = [
        "completeness",
        "--plan",
        str(plan),
        "--shards",
        str(shards),
        "--jobs",
        str(jobs),
        "--logs",
        str(logs),
    ]
    assert mt.main(argv) == 1
    text = capsys.readouterr().out
    assert "| 0 | 1 | 1 | complete |" in text
    assert "| 1 | 1 | 0 | infrastructure: the runner was lost" in text
    assert "| 2 | 1 | 0 | timeout: the job time limit was reached |" in text
    assert f"| 3 | 1 | 0 | {reason} |" in text
    assert f"- `{name(1)}`" in text and f"- `{name(0)}`" not in text
    assert "Mutation coverage incomplete" in text


def test_summary_reports_the_mean_time_per_mutant(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    caught = outcome(RATES, 1, "f", "replace a", "CaughtMutant")
    caught["phase_results"] = [
        {"phase": "Build", "duration": 12.0},
        {"phase": "Test", "duration": 20.0},
    ]
    out = write_run(tmp_path / "run", [caught])
    assert mt.main(["summary", str(out)]) == 0
    assert (
        "Mean time per mutant: 32 s (build 12 s, test 20 s) over 1 mutants"
        in capsys.readouterr().out
    )


def test_stale_entries_are_checked_only_in_the_files_given(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    exclusions(
        tmp_path,
        monkeypatch,
        f'[[equivalent]]\nfile = "{RATES}"\nfunction = "f"\nmutation = "replace a in f"\n'
        'line = 1\nreason = "r"\nreviewer = "salvo8519"\n',
    )
    out = write_run(tmp_path / "run", [outcome(RATES, 1, "f", "replace a in f", "CaughtMutant")])
    assert mt.main(["summary", str(out), "--stale-in", "crates/fairmark-core/src/tvm.rs"]) == 0
    assert "match no missed mutant" not in capsys.readouterr().out
    assert mt.main(["summary", str(out), "--stale-in", RATES]) == 0
    assert "match no missed mutant" in capsys.readouterr().out
