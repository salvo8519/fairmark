"""Fail if an existing golden case changed, unless the PR is labelled `numerical-change`.

    python scripts/check_golden_changes.py --base origin/main [--allowed]

New golden files are always allowed. Modified or deleted ones change what Fairmark promises
to compute and need explicit maintainer review.
"""

from __future__ import annotations

import argparse
import subprocess
import sys
from pathlib import Path

import yaml


def git(*args: str) -> str:
    return subprocess.run(
        ["git", *args], capture_output=True, text=True, check=True, encoding="utf-8"
    ).stdout


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--base", required=True)
    parser.add_argument("--allowed", action="store_true", help="PR has the numerical-change label")
    args = parser.parse_args()

    changes = git("diff", "--name-status", f"{args.base}...HEAD", "--", "tests/golden")
    problems = []
    unsourced = []
    for line in changes.splitlines():
        status, *paths = line.split("\t")
        path = paths[-1]
        if status.startswith("A"):
            added = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
            if not str(added.get("source", "")).strip():
                unsourced.append(path)
            continue
        if status.startswith("D"):
            problems.append(f"deleted: {path}")
            continue
        before = yaml.safe_load(git("show", f"{args.base}:{paths[0]}"))
        after = yaml.safe_load(Path(path).read_text(encoding="utf-8"))
        for key in (
            "method",
            "input",
            "expect",
            "expect_meta",
            "expect_length",
            "expect_error",
            "expect_error_report",
            "tolerance",
        ):
            if before.get(key) != after.get(key):
                problems.append(f"changed '{key}': {path}")

    if unsourced:
        sys.exit("New golden cases must name an independent 'source':\n  " + "\n  ".join(unsourced))
    if not problems:
        print("No existing golden case changed.")
        return
    print("Existing golden cases changed:\n  " + "\n  ".join(problems))
    if args.allowed:
        print("Allowed: the PR is labelled numerical-change. Maintainer review required.")
        return
    sys.exit("Add the 'numerical-change' label (maintainers only) or revert these changes.")


if __name__ == "__main__":
    main()
