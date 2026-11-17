"""Per-test failure statistics across CI runs, and the quarantine of intermittent tests.

uv run python scripts/test_failures.py collect DIR [--out stats.json]
uv run python scripts/test_failures.py report stats.json [--json]
uv run python scripts/test_failures.py quarantine-check

`collect` reads the JUnit XML files that pytest writes in CI (`--junitxml`), found anywhere
below DIR: one file per run and platform. For each test it counts the runs and the failures (a
failure or an error), and keeps the last failure message.

`report` classifies every test that failed at least once:

| Kind | Meaning |
| --- | --- |
| intermittent | failed in some runs and passed in others: a candidate for the quarantine |
| always failing | failed in every run where it ran: a defect to fix, never to quarantine |

`quarantine-check` validates `python/tests/quarantine.yaml`. Each entry names a test (its pytest
node id), the issue that tracks the fix, the reason, and the date until which it holds. A
quarantined test still runs; its failure does not fail the suite, and is reported
(`python/tests/conftest.py`). An entry past its date, without an issue, or naming a test file
that does not exist fails the check, so a quarantine can never become permanent by neglect.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import sys
import xml.etree.ElementTree as ET
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
QUARANTINE = ROOT / "python" / "tests" / "quarantine.yaml"
MESSAGE_LENGTH = 300


def node_id(case: ET.Element) -> str:
    """The pytest node id of a JUnit test case: `python/tests/test_x.py::test_y`."""
    classname, name = case.get("classname", ""), case.get("name", "")
    parts = classname.split(".")
    # pytest writes the module path with dots and an optional class at the end.
    for cut in range(len(parts), 0, -1):
        module = "/".join(parts[:cut]) + ".py"
        if (ROOT / module).exists():
            return "::".join([module, *parts[cut:], name])
    return f"{classname}::{name}"


def collect(directory: Path) -> dict[str, dict[str, Any]]:
    stats: dict[str, dict[str, Any]] = {}
    for path in sorted(directory.rglob("*.xml")):
        try:
            tree = ET.parse(path)
        except ET.ParseError:
            continue
        # The directory of the file names the run: `<run id>/junit-<os>-<python>`.
        run = path.parent.relative_to(directory).as_posix()
        for case in tree.iter("testcase"):
            if case.find("skipped") is not None:
                continue
            entry = stats.setdefault(node_id(case), {"runs": 0, "failures": 0, "failed_in": []})
            entry["runs"] += 1
            problem = case.find("failure")
            problem = problem if problem is not None else case.find("error")
            if problem is not None:
                entry["failures"] += 1
                entry["failed_in"].append(run)
                entry["message"] = (problem.get("message") or "")[:MESSAGE_LENGTH]
    return stats


def classify(stats: dict[str, dict[str, Any]]) -> dict[str, list[dict[str, Any]]]:
    out: dict[str, list[dict[str, Any]]] = {"intermittent": [], "always_failing": []}
    for test, s in sorted(stats.items()):
        if not s["failures"]:
            continue
        kind = "always_failing" if s["failures"] == s["runs"] else "intermittent"
        out[kind].append({"test": test, **s, "rate": round(s["failures"] / s["runs"], 3)})
    return out


def load_quarantine(path: Path = QUARANTINE) -> list[dict[str, Any]]:
    if not path.exists():
        return []
    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    entries = data.get("quarantine") or []
    return entries if isinstance(entries, list) else []


def quarantine_problems(
    entries: list[dict[str, Any]], today: dt.date, root: Path = ROOT
) -> list[str]:
    problems = []
    for i, e in enumerate(entries):
        test = e.get("test") if isinstance(e, dict) else None
        where = test or f"entry {i + 1}"
        if not isinstance(e, dict) or not isinstance(test, str) or "::" not in test:
            problems.append(f"{where}: 'test' must be a pytest node id (file.py::test_name)")
            continue
        if not isinstance(e.get("issue"), int):
            problems.append(f"{where}: 'issue' must be the number of the issue that tracks the fix")
        if not str(e.get("reason") or "").strip():
            problems.append(f"{where}: 'reason' is required")
        until = e.get("until")
        if not isinstance(until, dt.date):
            problems.append(f"{where}: 'until' must be a date (YYYY-MM-DD)")
        elif until < today:
            problems.append(f"{where}: quarantine expired on {until}: fix the test or renew it")
        if not (root / test.split("::", 1)[0]).exists():
            problems.append(f"{where}: no such test file")
    return problems


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Per-test failure statistics and quarantine.")
    sub = parser.add_subparsers(dest="command", required=True)
    c = sub.add_parser("collect")
    c.add_argument("directory", type=Path)
    c.add_argument("--out", type=Path)
    r = sub.add_parser("report")
    r.add_argument("stats", type=Path)
    r.add_argument("--json", action="store_true")
    sub.add_parser("quarantine-check")
    args = parser.parse_args(argv)
    if args.command == "collect":
        text = json.dumps(collect(args.directory), indent=2, sort_keys=True)
        if args.out:
            args.out.write_text(text + "\n", encoding="utf-8")
        else:
            print(text)
        return 0
    if args.command == "report":
        stats = json.loads(args.stats.read_text(encoding="utf-8"))
        kinds = classify(stats)
        quarantined = {e.get("test") for e in load_quarantine()}
        if args.json:
            print(json.dumps(kinds, indent=2))
        else:
            runs = max((s["runs"] for s in stats.values()), default=0)
            print(f"{len(stats)} tests, up to {runs} runs each.")
            for kind, rows in kinds.items():
                print(f"{kind.replace('_', ' ').capitalize()}: {len(rows)}")
                for row in rows:
                    mark = " (quarantined)" if row["test"] in quarantined else ""
                    print(f"  {row['test']}: {row['failures']}/{row['runs']}{mark}")
        unquarantined = [r for r in kinds["intermittent"] if r["test"] not in quarantined]
        return 1 if unquarantined or kinds["always_failing"] else 0
    problems = quarantine_problems(load_quarantine(), dt.datetime.now(dt.UTC).date())
    if problems:
        print("Quarantine file is invalid:\n  " + "\n  ".join(problems))
        return 1
    print(f"Quarantine valid: {len(load_quarantine())} tests.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
