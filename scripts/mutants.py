"""Mutation testing of fairmark-core: exclusions, per-module summary and the pull-request gate.

cargo-mutants (configured in `.cargo/mutants.toml`) writes one `mutants.out` directory per
run or shard. This script reads their `outcomes.json` files and:

    python scripts/mutants.py check-exclusions
        validate tests/mutants/equivalent.toml (every entry names one mutant, a reason and a
        reviewer; no blanket exclusions)
    python scripts/mutants.py summary DIR [DIR ...] [--gate] [--full]
        print a Markdown table per module (caught, missed, equivalent, unviable, timeout) and
        the missed mutants; with --gate exit 1 when a missed mutant is not recorded as
        equivalent; with --full also report equivalent entries that no longer match a missed
        mutant (stale entries: the mutant is now caught, unviable or gone; only meaningful on a run
        of the whole core); with --stale-in FILE ... the same, limited to entries of those files
        (a run that tested every mutant of them)
    python scripts/mutants.py check-slices
        validate tests/mutants/budget.toml: the budgets, and the weekly slices cover every source
        file of the core exactly once
    python scripts/mutants.py slice [--date YYYY-MM-DD | --index N]
        the slice of the weekly rotation for a date (default today, UTC) or by index
    python scripts/mutants.py plan --kind pr|weekly --list FILE [--part K/M]
        the shards of a run: their number from the mutants listed in FILE (`cargo mutants
        --list`) and the time budget, and the `--shard` argument of each
    python scripts/mutants.py coverage EXPECTED DIR --exit-status N --run-minutes M
        one shard: exit 1 with "mutation coverage incomplete" and the untested mutants when DIR
        has no outcome for a mutant listed in EXPECTED
    python scripts/mutants.py completeness --plan DIR --shards DIR [--jobs FILE --logs DIR]
        the whole run: every shard of the plan reported every mutant; otherwise exit 1 and say,
        per shard, whether it stopped at its time budget, hit the job time limit, lost its runner
        (infrastructure) or failed, with the untested mutants

Equivalent mutants are applied after the run, not passed to cargo-mutants as exclusions: they
are still built and tested, so an entry that stops being equivalent (because a test now
catches the mutant, or the code changed) is visible in the next full run.

Policy and triage rules: docs/en/project/mutation-testing.md.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import math
import re
import sys
import tomllib
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
EXCLUSIONS = ROOT / "tests" / "mutants" / "equivalent.toml"
BUDGET = ROOT / "tests" / "mutants" / "budget.toml"
CORE_SRC = "crates/fairmark-core/src/"
# The weeks of the rotation are counted from this Monday (ISO weeks, Monday to Sunday, UTC).
EPOCH_MONDAY = dt.date(1970, 1, 5)
# Exit status of `timeout` when it stopped cargo-mutants at the end of the time budget.
TIMEOUT_EXIT = 124
# The status file a shard writes beside outcomes.json.
STATUS_FILE = "shard-status.json"
# How a lost runner and a job time limit appear in the log of a GitHub Actions job.
RUNNER_LOST = (
    "The runner has received a shutdown signal",
    "lost communication with the server",
)
JOB_TIME_LIMIT = "has exceeded the maximum execution time"
OUTCOMES = ("caught", "missed", "equivalent", "unviable", "timeout", "failure")
SUMMARY_TO_OUTCOME = {
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Unviable": "unviable",
    "Timeout": "timeout",
    "Failure": "failure",
}
REQUIRED_KEYS = ("file", "function", "mutation", "line", "reason", "reviewer")
# The reviewer is the GitHub login of the maintainer who approved the entry, nothing else: a
# placeholder such as "pending review" is rejected, so an entry cannot pass unapproved.
GITHUB_LOGIN = re.compile(r"^[A-Za-z0-9-]+$")
ALLOWED_KEYS = set(REQUIRED_KEYS)


@dataclass(frozen=True)
class Mutant:
    """One tested mutant, as reported by cargo-mutants."""

    file: str
    line: int
    function: str
    mutation: str
    outcome: str

    @property
    def module(self) -> str:
        return module_of(self.file)

    def describe(self) -> str:
        return f"{self.file}:{self.line}: {self.mutation}"


@dataclass(frozen=True)
class Equivalent:
    """A reviewed equivalent mutant: one mutation in one function at one line.

    Matching is by exact text and line, never by pattern, so an entry cannot exclude more than
    the one mutation it names.
    """

    file: str
    function: str
    mutation: str
    reason: str
    reviewer: str
    line: int

    def matches(self, m: Mutant) -> bool:
        return (
            m.file == self.file
            and m.function == self.function
            and m.mutation == self.mutation
            and m.line == self.line
        )


def module_of(path: str) -> str:
    """`crates/fairmark-core/src/dcf/engine.rs` -> `dcf/engine`; `tv/mod.rs` -> `tv`."""
    rel = path.removeprefix(CORE_SRC).removesuffix(".rs")
    return rel.removesuffix("/mod") if rel.endswith("/mod") else rel


def belongs_to(mutation: str, function: str) -> bool:
    """The mutation text names its function: "... in <function>" for a change inside the body,
    "replace <function> -> <type> with ..." or "replace <function> with ..." for the whole body.
    """
    return (
        mutation.endswith(f" in {function}")
        or mutation.startswith(f"replace {function} -> ")
        or mutation.startswith(f"replace {function} with ")
    )


def load_exclusions(path: Path | None = None) -> tuple[list[Equivalent], list[str]]:
    """Read and validate the exclusions file; return the entries and the problems found."""
    path = path or EXCLUSIONS
    data = tomllib.loads(path.read_text(encoding="utf-8"))
    problems: list[str] = []
    unknown = set(data) - {"equivalent"}
    if unknown:
        problems.append(f"unknown top-level keys: {sorted(unknown)}")
    entries: list[Equivalent] = []
    seen: set[tuple[str, str, str, int]] = set()
    for i, raw in enumerate(data.get("equivalent", [])):
        where = f"equivalent[{i}]"
        if not isinstance(raw, dict):
            problems.append(f"{where}: must be a table")
            continue
        missing = [k for k in REQUIRED_KEYS if not str(raw.get(k, "")).strip()]
        extra = sorted(set(raw) - ALLOWED_KEYS)
        if missing:
            problems.append(f"{where}: missing or empty {missing}")
        if extra:
            problems.append(f"{where}: unknown keys {extra}")
        if missing or extra:
            continue
        file, function, mutation = str(raw["file"]), str(raw["function"]), str(raw["mutation"])
        if not file.startswith(CORE_SRC) or not file.endswith(".rs"):
            problems.append(f"{where}: file must be a source file under {CORE_SRC}")
        if not (ROOT / file).is_file():
            problems.append(f"{where}: file {file} does not exist")
        if not mutation.startswith(("replace ", "delete ")):
            problems.append(f"{where}: mutation must be the text cargo-mutants prints")
        if not belongs_to(mutation, function):
            problems.append(f"{where}: mutation does not belong to function {function}")
        if not GITHUB_LOGIN.match(str(raw["reviewer"]).strip()):
            problems.append(
                f"{where}: reviewer must be the GitHub login of the maintainer who approved the "
                "entry (letters, digits and hyphens)"
            )
        line = raw["line"]
        if not isinstance(line, int) or isinstance(line, bool) or line < 1:
            problems.append(f"{where}: line must be a positive integer")
            continue
        key = (file, function, mutation, line)
        if key in seen:
            problems.append(f"{where}: duplicate of an earlier entry")
        seen.add(key)
        entries.append(
            Equivalent(
                file=file,
                function=function,
                mutation=mutation,
                reason=str(raw["reason"]).strip(),
                reviewer=str(raw["reviewer"]).strip(),
                line=line,
            )
        )
    return entries, problems


def outcomes_path(directory: Path) -> Path | None:
    """The `outcomes.json` of a `mutants.out` directory, or of its parent; None when absent."""
    for path in (directory / "outcomes.json", directory / "mutants.out" / "outcomes.json"):
        if path.is_file():
            return path
    return None


def read_outcomes(directory: Path, allow_empty: bool) -> tuple[list[Mutant], bool]:
    """Mutants tested in one `mutants.out` (or its parent) and whether its baseline passed."""
    path = outcomes_path(directory)
    if path is None:
        if allow_empty:  # --in-diff with no mutable line: cargo-mutants tests nothing
            return [], True
        raise SystemExit(f"No outcomes.json in {directory}: the run did not complete.")
    data: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    mutants: list[Mutant] = []
    baseline_ok = True
    for item in data.get("outcomes", []):
        scenario = item["scenario"]
        summary = item["summary"]
        if scenario == "Baseline":
            baseline_ok = summary == "Success"
            continue
        m = scenario["Mutant"]
        function = (m.get("function") or {}).get("function_name", "")
        # The name is "<file>:<line>:<col>: <mutation>"; the mutation is the stable part.
        mutation = m["name"].split(": ", 1)[1] if ": " in m["name"] else m["name"]
        mutants.append(
            Mutant(
                file=m["file"],
                line=int(m["span"]["start"]["line"]),
                function=function,
                mutation=mutation,
                outcome=SUMMARY_TO_OUTCOME.get(summary, "failure"),
            )
        )
    return mutants, baseline_ok


def mutant_seconds(directory: Path) -> list[tuple[float, float]]:
    """(build, test) seconds of each mutant tested in a `mutants.out`, baseline excluded."""
    path = outcomes_path(directory)
    if path is None:
        return []
    data: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    times = []
    for item in data.get("outcomes", []):
        if item["scenario"] == "Baseline":
            continue
        phases = {p["phase"]: float(p.get("duration", 0)) for p in item.get("phase_results", [])}
        times.append((phases.get("Build", 0.0), phases.get("Test", 0.0)))
    return times


def timing_line(times: list[tuple[float, float]]) -> str:
    """One line with the mean time per mutant, to keep `seconds_per_mutant` measured."""
    n = len(times)
    build = sum(b for b, _ in times) / n
    test = sum(t for _, t in times) / n
    return (
        f"Mean time per mutant: {build + test:.0f} s (build {build:.0f} s, test {test:.0f} s) "
        f"over {n} mutants; compare with `seconds_per_mutant` in tests/mutants/budget.toml."
    )


def classify(mutants: list[Mutant], entries: list[Equivalent]) -> list[Mutant]:
    """Missed mutants that a reviewed entry records as equivalent become `equivalent`."""
    out = []
    for m in mutants:
        if m.outcome == "missed" and any(e.matches(m) for e in entries):
            m = Mutant(m.file, m.line, m.function, m.mutation, "equivalent")
        out.append(m)
    return out


def table(mutants: list[Mutant]) -> str:
    """Markdown table: one row per module, then the total."""
    per_module: dict[str, Counter[str]] = {}
    for m in mutants:
        per_module.setdefault(m.module, Counter())[m.outcome] += 1
    head = "| Module | Mutants | " + " | ".join(o.capitalize() for o in OUTCOMES) + " |"
    rows = [head, "| --- " * (len(OUTCOMES) + 2) + "|"]
    total: Counter[str] = Counter()
    for module in sorted(per_module):
        c = per_module[module]
        total.update(c)
        cells = " | ".join(str(c[o]) for o in OUTCOMES)
        rows.append(f"| {module} | {sum(c.values())} | {cells} |")
    cells = " | ".join(str(total[o]) for o in OUTCOMES)
    rows.append(f"| Total | {sum(total.values())} | {cells} |")
    return "\n".join(rows)


def summary(args: argparse.Namespace) -> int:
    entries, problems = load_exclusions()
    if problems:
        print("\n".join(f"Exclusions file: {p}" for p in problems), file=sys.stderr)
        return 1
    mutants: list[Mutant] = []
    times: list[tuple[float, float]] = []
    baseline_failed = []
    for directory in args.dirs:
        found, ok = read_outcomes(Path(directory), args.allow_empty)
        mutants.extend(found)
        times.extend(mutant_seconds(Path(directory)))
        if not ok:
            baseline_failed.append(directory)
    mutants = classify(mutants, entries)
    lines = [f"## {args.title}", ""]
    if not mutants:
        lines.append("No mutant was tested (no mutable change in fairmark-core).")
    else:
        lines += [table(mutants), ""]
        missed = sorted((m for m in mutants if m.outcome == "missed"), key=Mutant.describe)
        timeouts = sorted((m for m in mutants if m.outcome == "timeout"), key=Mutant.describe)
        failures = sorted((m for m in mutants if m.outcome == "failure"), key=Mutant.describe)
        for label, items in (
            ("Missed mutants (triage: kill with a test or record as equivalent)", missed),
            ("Timed out (detected; check that the timeout is not too short)", timeouts),
            ("Failures of the tool (inspect the logs in the artifact)", failures),
        ):
            if items:
                lines += [f"### {label}", ""]
                lines += [f"- `{m.describe()}` in `{m.function}`" for m in items]
                lines.append("")
    if times:
        lines += [timing_line(times), ""]
    if args.full or args.stale_in:
        # Stale: no mutant it matches is still missed (caught now, unviable, or gone).
        scope = set(args.stale_in or [])
        stale = [
            e
            for e in entries
            if (args.full or e.file in scope)
            and not any(m.outcome == "equivalent" and e.matches(m) for m in mutants)
        ]
        if stale:
            lines += ["### Equivalent entries that match no missed mutant (review them)", ""]
            lines += [f"- `{e.file}`: `{e.mutation}` in `{e.function}`" for e in stale]
            lines.append("")
    for directory in baseline_failed:
        lines.append(f"Baseline tests failed in {directory}: no mutant there is meaningful.")
    print("\n".join(lines))
    if baseline_failed:
        return 1
    if args.gate:
        unexplained = [m for m in mutants if m.outcome in ("missed", "failure")]
        if unexplained:
            print(
                f"{len(unexplained)} mutant(s) missed or failed on changed lines of "
                "fairmark-core. Kill each with a test, or record it as equivalent in "
                "tests/mutants/equivalent.toml with a reason and a reviewer.",
                file=sys.stderr,
            )
            return 1
    return 0


# ---------------------------------------------------------------- budget and weekly rotation


@dataclass(frozen=True)
class Budget:
    """The time budget of one kind of run (tests/mutants/budget.toml)."""

    seconds_per_mutant: int
    baseline_minutes: int
    setup_minutes: int
    run_minutes: int
    max_shards: int

    @property
    def per_shard(self) -> int:
        """Mutants one shard tests within `run_minutes`, after the baseline."""
        return max(1, (self.run_minutes - self.baseline_minutes) * 60 // self.seconds_per_mutant)

    @property
    def timeout_minutes(self) -> int:
        """The limit of the shard job: the run and its setup."""
        return self.run_minutes + self.setup_minutes


@dataclass(frozen=True)
class Slice:
    """One slice of the weekly rotation: files of the core, or part k of m of their mutants."""

    name: str
    files: tuple[str, ...]
    part: int = 1
    parts: int = 1

    @property
    def label(self) -> str:
        return self.name if self.parts == 1 else f"{self.name} (part {self.part} of {self.parts})"

    @property
    def part_arg(self) -> str | None:
        """The `--shard` argument that selects the part from the mutants of the files."""
        return None if self.parts == 1 else f"{self.part - 1}/{self.parts}"

    def paths(self, src: Path | None = None) -> list[str]:
        """The source files of the slice, relative to the repository root, sorted."""
        src = src or ROOT / CORE_SRC
        found = {p for glob in self.files for p in src.glob(glob) if p.suffix == ".rs"}
        return sorted(CORE_SRC + p.relative_to(src).as_posix() for p in found)

    def file_args(self) -> list[str]:
        """The cargo-mutants arguments that select the files of the slice."""
        out = []
        for path in self.paths():
            out += ["--file", path]
        return out


def shard_arg(index: int, shards: int, part: str | None) -> str:
    """The `--shard` argument of shard `index` of `shards`, round-robin, within `part` (k/m).

    Round-robin shard i of N takes the mutants whose position p in the list has p mod N = i.
    Shard j of S of part k of m (both zero-based) is therefore shard k + m * j of m * S: the
    positions p with p mod m = k and (p div m) mod S = j.
    """
    if part is None:
        return f"{index}/{shards}"
    k, m = (int(x) for x in part.split("/"))
    return f"{k + m * index}/{m * shards}"


def read_budget_file(path: Path | None = None) -> dict[str, Any]:
    return tomllib.loads((path or BUDGET).read_text(encoding="utf-8"))


def load_budget(kind: str, path: Path | None = None) -> Budget:
    data = read_budget_file(path)
    estimate, run = data["estimate"], data[kind]
    return Budget(
        seconds_per_mutant=int(estimate["seconds_per_mutant"]),
        baseline_minutes=int(estimate["baseline_minutes"]),
        setup_minutes=int(estimate["setup_minutes"]),
        run_minutes=int(run["run_minutes"]),
        max_shards=int(run["max_shards"]),
    )


def load_slices(path: Path | None = None) -> list[Slice]:
    """The slices of the rotation, in order, with their parts expanded."""
    out = []
    for raw in read_budget_file(path).get("slice", []):
        parts = int(raw.get("parts", 1))
        for part in range(1, parts + 1):
            out.append(Slice(str(raw["name"]), tuple(raw["files"]), part, parts))
    return out


def budget_problems(path: Path | None = None, src: Path | None = None) -> list[str]:
    """Problems of the budget file: missing or invalid values, and files of the core that are
    in no slice or in more than one."""
    src = src or ROOT / CORE_SRC
    data = read_budget_file(path)
    problems: list[str] = []
    unknown = set(data) - {"estimate", "pr", "weekly", "slice"}
    if unknown:
        problems.append(f"unknown top-level keys: {sorted(unknown)}")
    for table, keys in (
        ("estimate", ("seconds_per_mutant", "baseline_minutes", "setup_minutes")),
        ("pr", ("run_minutes", "max_shards")),
        ("weekly", ("run_minutes", "max_shards")),
    ):
        values = data.get(table, {})
        extra = sorted(set(values) - set(keys))
        if extra:
            problems.append(f"[{table}]: unknown keys {extra}")
        for key in keys:
            value = values.get(key)
            if not isinstance(value, int) or isinstance(value, bool) or value < 1:
                problems.append(f"[{table}] {key} must be a positive integer")
    if problems:
        return problems
    for kind in ("pr", "weekly"):
        b = load_budget(kind, path)
        if b.run_minutes <= b.baseline_minutes:
            problems.append(f"[{kind}] run_minutes must exceed baseline_minutes")
    raw_slices = data.get("slice", [])
    if not raw_slices:
        problems.append("no [[slice]]: the weekly rotation is empty")
    owner: dict[str, list[str]] = {}
    names = set()
    for i, raw in enumerate(raw_slices):
        where = f"slice[{i}]"
        extra = sorted(set(raw) - {"name", "files", "parts"})
        if extra:
            problems.append(f"{where}: unknown keys {extra}")
        name = str(raw.get("name", "")).strip()
        if not name:
            problems.append(f"{where}: name is required")
        if name in names:
            problems.append(f"{where}: duplicate name {name!r}")
        names.add(name)
        parts = raw.get("parts", 1)
        if not isinstance(parts, int) or isinstance(parts, bool) or parts < 1:
            problems.append(f"{where}: parts must be a positive integer")
        files = raw.get("files")
        if not isinstance(files, list) or not files:
            problems.append(f"{where}: files must be a non-empty list of globs")
            continue
        for glob in files:
            matched = sorted(p for p in src.glob(str(glob)) if p.suffix == ".rs")
            if not matched:
                problems.append(f"{where} ({name}): {glob} matches no source file")
            for f in matched:
                owner.setdefault(f.relative_to(src).as_posix(), []).append(name)
    for rel in sorted(p.relative_to(src).as_posix() for p in src.rglob("*.rs")):
        slices = sorted(set(owner.get(rel, [])))
        if not slices:
            problems.append(f"{CORE_SRC}{rel} is in no slice of the weekly rotation")
        elif len(owner[rel]) > 1:
            problems.append(f"{CORE_SRC}{rel} is in more than one slice: {slices}")
    return problems


def week_number(day: dt.date) -> int:
    """ISO weeks (Monday to Sunday) since Monday 5 January 1970."""
    return (day - EPOCH_MONDAY).days // 7


def github_output(path: str | None, values: dict[str, Any]) -> None:
    """Append `values` to a GitHub Actions output file, one `key=value` line each."""
    if not path:
        return
    with Path(path).open("a", encoding="utf-8") as f:
        for key, value in values.items():
            text = value if isinstance(value, str) else json.dumps(value)
            f.write(f"{key}={text}\n")


def check_slices(_: argparse.Namespace) -> int:
    problems = budget_problems()
    if problems:
        print("\n".join(f"Budget file: {p}" for p in problems), file=sys.stderr)
        return 1
    slices = load_slices()
    print(f"Budget file valid: weekly rotation of {len(slices)} slices ({len(slices)} weeks).")
    return 0


def slice_command(args: argparse.Namespace) -> int:
    slices = load_slices()
    if args.index is not None:
        index = args.index
    else:
        day = dt.date.fromisoformat(args.date) if args.date else dt.datetime.now(dt.UTC).date()
        index = week_number(day) % len(slices)
    if not 0 <= index < len(slices):
        print(
            f"No slice {index}: the rotation has {len(slices)} (0 to {len(slices) - 1}).",
            file=sys.stderr,
        )
        return 1
    s = slices[index]
    values = {
        "index": index,
        "cycle": len(slices),
        "label": s.label,
        "title": f"Mutation testing of the weekly slice {index + 1} of {len(slices)}: {s.label}",
        "files": " ".join(s.paths()),
        "part": s.part_arg or "",
        # Stale equivalent entries are checked only when the run covers whole files.
        "stale_in": " ".join(s.paths()) if s.parts == 1 else "",
    }
    print(json.dumps(values, indent=2))
    github_output(args.github_output, values)
    if args.args_file:
        Path(args.args_file).write_text("\n".join(s.file_args()) + "\n", encoding="utf-8")
    if args.part_file:
        Path(args.part_file).write_text(s.part_arg or "", encoding="utf-8")
    return 0


def plan(args: argparse.Namespace) -> int:
    budget = load_budget(args.kind)
    listed = [line for line in Path(args.list).read_text(encoding="utf-8").splitlines() if line]
    n = len(listed)
    per = budget.per_shard
    shards = math.ceil(n / per)
    values: dict[str, Any] = {
        "mutants": n,
        "shards": shards,
        "per_shard": per,
        "run_minutes": budget.run_minutes,
        "timeout_minutes": budget.timeout_minutes,
        "matrix": [{"index": j, "shard": shard_arg(j, shards, args.part)} for j in range(shards)],
    }
    print(json.dumps(values, indent=2))
    if shards > budget.max_shards:
        print(
            f"{n} mutants need {shards} shards of {per} mutants ({budget.run_minutes} minutes "
            f"each), more than the {budget.max_shards} of the {args.kind} budget "
            "(tests/mutants/budget.toml). "
            + (
                "Split the pull request, or ask the maintainer to raise the budget."
                if args.kind == "pr"
                else "Split this slice of the weekly rotation in tests/mutants/budget.toml."
            ),
            file=sys.stderr,
        )
        return 1
    github_output(args.github_output, values)
    if args.shards_file:
        lines = [f"{m['index']} {m['shard']}\n" for m in values["matrix"]]
        Path(args.shards_file).write_text("".join(lines), encoding="utf-8")
    return 0


SHARD_JOB = re.compile(r"shard (\d+) of (\d+)$", re.IGNORECASE)


def shard_jobs(path: Path, shards: int | None = None) -> dict[int, dict[str, Any]]:
    """The shard jobs of a run (GitHub REST API, jobs of a run attempt), by shard index."""
    jobs: dict[int, dict[str, Any]] = {}
    for job in json.loads(path.read_text(encoding="utf-8")).get("jobs", []):
        m = SHARD_JOB.search(job.get("name", ""))
        if m and (shards is None or int(m.group(2)) == shards):
            jobs[int(m.group(1))] = job
    return jobs


def failed_jobs(args: argparse.Namespace) -> int:
    for job in shard_jobs(Path(args.jobs)).values():
        if job.get("conclusion") != "success":
            print(job["id"])
    return 0


def tested_names(directory: Path) -> set[str]:
    """The names (as `cargo mutants --list` prints them) of the mutants with an outcome."""
    path = outcomes_path(directory)
    if path is None:
        return set()
    data: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    return {
        item["scenario"]["Mutant"]["name"]
        for item in data.get("outcomes", [])
        if item["scenario"] != "Baseline"
    }


def expected_names(path: Path) -> list[str]:
    return [line for line in path.read_text(encoding="utf-8").splitlines() if line]


def stop_reason(status: int, run_minutes: int) -> str:
    if status == TIMEOUT_EXIT:
        return f"timeout: the time budget of {run_minutes} minutes was reached"
    if status in (0, 2, 3):
        return "cargo-mutants finished without an outcome for every mutant"
    return f"tool failure: cargo-mutants exited with status {status}"


def incomplete_lines(title: str, reason: str, untested: list[str], total: int) -> list[str]:
    lines = [
        f"### Mutation coverage incomplete: {title}",
        "",
        f"{reason}. {len(untested)} of {total} mutants were not tested:",
        "",
    ]
    return lines + [f"- `{name}`" for name in untested] + [""]


def coverage(args: argparse.Namespace) -> int:
    expected = expected_names(Path(args.expected))
    directory = Path(args.dir)
    tested = tested_names(directory)
    untested = [name for name in expected if name not in tested]
    reason = stop_reason(args.exit_status, args.run_minutes)
    status = {
        "exit_status": args.exit_status,
        "expected": len(expected),
        "tested": len(expected) - len(untested),
        "reason": reason if untested else "complete",
    }
    directory.mkdir(parents=True, exist_ok=True)
    (directory / STATUS_FILE).write_text(json.dumps(status, indent=2), encoding="utf-8")
    if not untested:
        print(f"Mutation coverage complete: {len(expected)} of {len(expected)} mutants tested.")
        return 0
    print("\n".join(incomplete_lines(args.title, reason, untested, len(expected))))
    return 1


def job_reason(job: dict[str, Any] | None, log: str) -> str:
    """Why a shard job left no result, from its conclusion and its log."""
    if any(marker in log for marker in RUNNER_LOST):
        return "infrastructure: the runner was lost (shutdown signal or lost communication)"
    if JOB_TIME_LIMIT in log:
        return "timeout: the job time limit was reached"
    if job is None:
        return "no result: the shard job was not found"
    conclusion = job.get("conclusion") or job.get("status") or "unknown"
    if conclusion == "cancelled":
        return "cancelled: the run was cancelled"
    return f"no result: the shard job ended with conclusion {conclusion}"


def completeness(args: argparse.Namespace) -> int:
    plan_dir, shards_dir = Path(args.plan), Path(args.shards)
    planned = json.loads((plan_dir / "plan.json").read_text(encoding="utf-8"))
    jobs: dict[int, dict[str, Any]] = {}
    if args.jobs and Path(args.jobs).is_file():
        jobs = shard_jobs(Path(args.jobs), planned["shards"])
    lines = [f"## Completeness of {args.title}", ""]
    rows = ["| Shard | Mutants | Tested | Result |", "| --- | --- | --- | --- |"]
    sections: list[str] = []
    for shard in range(planned["shards"]):
        expected = expected_names(plan_dir / f"shard-{shard}.txt")
        directory = shards_dir / f"mutants-shard-{shard}"
        status_path = next(iter(sorted(directory.rglob(STATUS_FILE))), None)
        outcomes = next(iter(sorted(directory.rglob("outcomes.json"))), None)
        tested = tested_names(outcomes.parent) if outcomes else set()
        untested = [name for name in expected if name not in tested]
        if status_path is not None:
            reason = json.loads(status_path.read_text(encoding="utf-8"))["reason"]
        else:
            job = jobs.get(shard)
            log = ""
            if job is not None and args.logs:
                log_path = Path(args.logs) / f"{job['id']}.log"
                if log_path.is_file():
                    log = log_path.read_text(encoding="utf-8", errors="replace")
            reason = job_reason(job, log)
        if not untested:
            reason = "complete"
        rows.append(f"| {shard} | {len(expected)} | {len(expected) - len(untested)} | {reason} |")
        if untested:
            sections += incomplete_lines(f"shard {shard}", reason, untested, len(expected))
    lines += [*rows, "", *sections]
    if sections:
        lines.append(
            "Mutation coverage incomplete: the run did not test every mutant of its plan. An "
            "infrastructure failure is retried by re-running the failed jobs; a timeout means "
            "the budget in tests/mutants/budget.toml no longer fits "
            "(docs/en/project/mutation-testing.md)."
        )
    else:
        lines.append(f"Mutation coverage complete: {planned['mutants']} mutants tested.")
    print("\n".join(lines))
    return 1 if sections else 0


def check_exclusions(_: argparse.Namespace) -> int:
    entries, problems = load_exclusions()
    if problems:
        print("\n".join(f"Exclusions file: {p}" for p in problems), file=sys.stderr)
        return 1
    print(f"Exclusions file valid: {len(entries)} equivalent mutant(s) recorded.")
    return 0


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    sub = parser.add_subparsers(dest="command", required=True)
    sub.add_parser("check-exclusions", help="validate the exclusions file").set_defaults(
        func=check_exclusions
    )
    s = sub.add_parser("summary", help="per-module Markdown summary of one or more runs")
    s.add_argument("dirs", nargs="+", help="mutants.out directories (or their parents)")
    s.add_argument("--gate", action="store_true", help="exit 1 on unexplained missed mutants")
    s.add_argument("--full", action="store_true", help="the run covers the whole core")
    s.add_argument(
        "--allow-empty", action="store_true", help="a missing outcomes.json means no mutant"
    )
    s.add_argument(
        "--stale-in",
        nargs="+",
        metavar="FILE",
        help="report stale equivalent entries of these files (the run tested all their mutants)",
    )
    s.add_argument("--title", default="Mutation testing of fairmark-core")
    s.set_defaults(func=summary)
    sub.add_parser("check-slices", help="validate the budget and the weekly slices").set_defaults(
        func=check_slices
    )
    sl = sub.add_parser("slice", help="the slice of the weekly rotation for a date or an index")
    sl.add_argument("--date", help="YYYY-MM-DD (default: today, UTC)")
    sl.add_argument("--index", type=int, help="the slice by its position in the rotation")
    sl.add_argument("--github-output", help="also append the values to this file")
    sl.add_argument("--args-file", help="write the cargo-mutants arguments, one per line")
    sl.add_argument("--part-file", help="write the --shard argument of the part (or nothing)")
    sl.set_defaults(func=slice_command)
    pl = sub.add_parser("plan", help="the shards of a run within its time budget")
    pl.add_argument("--kind", choices=("pr", "weekly"), required=True)
    pl.add_argument("--list", required=True, help="output of `cargo mutants --list`")
    pl.add_argument("--part", help="k/m: the part of the list a weekly slice tests")
    pl.add_argument("--github-output", help="also append the values to this file")
    pl.add_argument("--shards-file", help="write one line per shard: index and --shard argument")
    pl.set_defaults(func=plan)
    fj = sub.add_parser("failed-jobs", help="ids of the shard jobs of a run that did not succeed")
    fj.add_argument("jobs", help="the jobs of the run (GitHub REST API)")
    fj.set_defaults(func=failed_jobs)
    cv = sub.add_parser("coverage", help="one shard tested every mutant it was given")
    cv.add_argument("expected", help="the mutants of the shard, one name per line")
    cv.add_argument("dir", help="the mutants.out directory of the shard")
    cv.add_argument("--exit-status", type=int, required=True, help="of timeout cargo mutants")
    cv.add_argument("--run-minutes", type=int, required=True)
    cv.add_argument("--title", default="shard")
    cv.set_defaults(func=coverage)
    cp = sub.add_parser("completeness", help="every shard of a run tested every mutant")
    cp.add_argument("--plan", required=True, help="plan.json and shard-<n>.txt")
    cp.add_argument("--shards", required=True, help="mutants-shard-<n> artifacts")
    cp.add_argument("--jobs", help="the jobs of the run (GitHub REST API)")
    cp.add_argument("--logs", help="directory of <job id>.log for the shard jobs that failed")
    cp.add_argument("--title", default="the run")
    cp.set_defaults(func=completeness)
    args = parser.parse_args(argv)
    result: int = args.func(args)
    return result


if __name__ == "__main__":
    sys.exit(main())
