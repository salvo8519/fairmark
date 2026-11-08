"""Validate a sprint file and report its progress from GitHub.

uv run python scripts/sprint.py validate sprints/S01.yaml
uv run python scripts/sprint.py status sprints/S01.yaml [--repo owner/name]
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

import yaml

REQUIRED = {"id", "goal", "start", "end", "milestone", "status", "lanes"}
STATUSES = {"planned", "active", "done"}


def load(path: Path) -> dict[str, Any]:
    data: dict[str, Any] = yaml.safe_load(path.read_text(encoding="utf-8"))
    return data


def validate(sprint: dict[str, Any]) -> list[str]:
    errors = [f"missing field '{f}'" for f in sorted(REQUIRED - sprint.keys())]
    if errors:
        return errors
    if sprint["status"] not in STATUSES:
        errors.append(f"status must be one of {sorted(STATUSES)}")
    start, end = sprint["start"], sprint["end"]
    if not (isinstance(start, dt.date) and isinstance(end, dt.date)) or end <= start:
        errors.append("start and end must be dates with end after start")
    seen: dict[int, str] = {}
    for lane in sprint["lanes"]:
        for issue in lane.get("issues", []):
            if not isinstance(issue, int):
                errors.append(f"lane {lane.get('name')}: issue ids must be integers")
            elif issue in seen:
                errors.append(f"issue #{issue} is in lanes {seen[issue]} and {lane['name']}")
            else:
                seen[issue] = lane["name"]
    return errors


def origin_repo() -> str:
    url = subprocess.run(
        ["git", "remote", "get-url", "origin"],
        capture_output=True,
        text=True,
        check=True,
        encoding="utf-8",
    ).stdout.strip()
    return re.sub(r"(\.git)?$", "", re.split(r"github\.com[:/]", url)[-1])


def status(sprint: dict[str, Any], repo: str | None) -> None:
    repo = repo or origin_repo()
    print(f"{sprint['id']} — {sprint['goal']} ({sprint['start']} to {sprint['end']})")
    total = done = 0
    for lane in sprint["lanes"]:
        print(f"\n  Lane {lane['name']}")
        for issue in lane.get("issues", []):
            cmd = ["gh", "api", f"repos/{repo}/issues/{issue}"]
            info = json.loads(
                subprocess.run(
                    cmd, capture_output=True, text=True, check=True, encoding="utf-8"
                ).stdout
            )
            state = "done" if info["state"] == "closed" else "open"
            total += 1
            done += state == "done"
            print(f"    #{issue:<5} {state:<5} {info['title']}")
    print(f"\n  Progress: {done}/{total} issues closed")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["validate", "status"])
    parser.add_argument("file", type=Path)
    parser.add_argument("--repo")
    args = parser.parse_args()
    sprint = load(args.file)
    errors = validate(sprint)
    if errors:
        sys.exit("Invalid sprint file:\n  " + "\n  ".join(errors))
    if args.command == "validate":
        print(f"{args.file} is valid.")
    else:
        status(sprint, args.repo)


if __name__ == "__main__":
    main()
