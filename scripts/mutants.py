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
        of the whole core)

Equivalent mutants are applied after the run, not passed to cargo-mutants as exclusions: they
are still built and tested, so an entry that stops being equivalent (because a test now
catches the mutant, or the code changed) is visible in the next full run.

Policy and triage rules: docs/en/project/mutation-testing.md.
"""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import tomllib

ROOT = Path(__file__).resolve().parents[1]
EXCLUSIONS = ROOT / "tests" / "mutants" / "equivalent.toml"
CORE_SRC = "crates/fairmark-core/src/"
OUTCOMES = ("caught", "missed", "equivalent", "unviable", "timeout", "failure")
SUMMARY_TO_OUTCOME = {
    "CaughtMutant": "caught",
    "MissedMutant": "missed",
    "Unviable": "unviable",
    "Timeout": "timeout",
    "Failure": "failure",
}
REQUIRED_KEYS = ("file", "function", "mutation", "line", "reason", "reviewer")
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


def read_outcomes(directory: Path, allow_empty: bool) -> tuple[list[Mutant], bool]:
    """Mutants tested in one `mutants.out` (or its parent) and whether its baseline passed."""
    path = directory / "outcomes.json"
    if not path.is_file():
        path = directory / "mutants.out" / "outcomes.json"
    if not path.is_file():
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
    baseline_failed = []
    for directory in args.dirs:
        found, ok = read_outcomes(Path(directory), args.allow_empty)
        mutants.extend(found)
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
    if args.full:
        # Stale: no mutant it matches is still missed (caught now, unviable, or gone).
        stale = [
            e
            for e in entries
            if not any(m.outcome == "equivalent" and e.matches(m) for m in mutants)
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
    s.add_argument("--title", default="Mutation testing of fairmark-core")
    s.set_defaults(func=summary)
    args = parser.parse_args(argv)
    result: int = args.func(args)
    return result


if __name__ == "__main__":
    sys.exit(main())
