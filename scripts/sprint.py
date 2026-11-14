"""Validate a sprint file and report its progress from GitHub.

uv run python scripts/sprint.py validate sprints/S01.yaml
uv run python scripts/sprint.py status sprints/S01.yaml [--repo owner/name]
uv run python scripts/sprint.py state sprints/S01.yaml 42 [--step tests --pr 33 ...]
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
# The per-issue state files (sprints/SNN/state/<issue>.yaml) are runtime state: they are
# git-ignored, owned by the checkout of the orchestrator, and never committed (sprints/README).
STATE_FILE_GITIGNORE = "/sprints/*/state/"


def _now() -> dt.datetime:
    return dt.datetime.now(dt.UTC)


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


# ------------------------------------------------------------------------------------------
# Per-issue state (issue #318): sprints/SNN/state/<issue>.yaml


def lane_of(sprint: dict[str, Any], issue: int) -> str:
    for lane in sprint["lanes"]:
        if issue in lane.get("issues", []):
            return lane["name"]
    sys.exit(
        f"#{issue} is not in the lanes of {sprint.get('id', 'the sprint')}: "
        "the state is only written for the issues of the sprint file"
    )


def state_path(sprint_file: Path, issue: int) -> Path:
    """sprints/S01.yaml -> sprints/S01/state/42.yaml, next to the plan it belongs to."""
    return sprint_file.with_suffix("") / "state" / f"{issue}.yaml"


def read_state(sprint_file: Path, issue: int) -> dict[str, Any] | None:
    path = state_path(sprint_file, issue)
    if not path.exists():
        return None
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    return data if isinstance(data, dict) else None


def write_state(
    sprint_file: Path,
    issue: int,
    *,
    branch: str | None = None,
    pull_request: int | None = None,
    step: str | None = None,
    blocker: list[str] | None = None,
    clear_blockers: bool = False,
) -> dict[str, Any]:
    """Create or update the state file of one issue; returns what it wrote.

    Every call is one orchestrator step on the issue: `agent_steps` counts them (the agent
    cost of the issue), `updated` moves with each call, `started` is kept from the first one,
    and the fields that no argument names are preserved. The file lives in the checkout of
    the orchestrator and is git-ignored: it survives context resets and crashes, not git
    history (sprints/README.md).
    """
    sprint = load(sprint_file)
    if validate(sprint):
        sys.exit("Invalid sprint file: run 'sprint.py validate' first")
    if pull_request is not None and not isinstance(pull_request, int):
        sys.exit("pull request must be an integer, for example: --pr 33")
    state = read_state(sprint_file, issue) or {
        "issue": issue,
        "lane": lane_of(sprint, issue),
        "branch": None,
        "pull_request": None,
        "step": None,
        "blockers": [],
        "started": _now().isoformat(timespec="seconds"),
        "agent_steps": 0,
    }
    state["agent_steps"] += 1
    state["updated"] = _now().isoformat(timespec="seconds")
    if branch is not None:
        state["branch"] = branch
    if pull_request is not None:
        state["pull_request"] = pull_request
    if step is not None:
        state["step"] = step
    if blocker:
        state["blockers"] = list(state.get("blockers") or []) + blocker
    if clear_blockers:
        state["blockers"] = []
    path = state_path(sprint_file, issue)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(yaml.safe_dump(state, sort_keys=False), encoding="utf-8")
    return state


def state_summary(state: dict[str, Any], *, closed: bool) -> str:
    """One line for `sprint.py status`: step, PR, blockers, agent cost; stale hint."""
    parts = []
    if state.get("step"):
        parts.append(f"step {state['step']}")
    if state.get("pull_request"):
        parts.append(f"PR #{state['pull_request']}")
    blockers = state.get("blockers") or []
    parts.append(f"{len(blockers)} blocker" if len(blockers) == 1 else f"{len(blockers)} blockers")
    parts.append(f"{state.get('agent_steps', 0)} agent steps")
    line = " — " + ", ".join(parts)
    if closed:
        line += " (stale state file: remove it: it is git-ignored runtime state)"
    return line


def state_command(args: argparse.Namespace) -> int:
    state = write_state(
        args.file,
        args.issue,
        branch=args.branch,
        pull_request=args.pr,
        step=args.step,
        blocker=args.blocker,
        clear_blockers=args.clear_blockers,
    )
    print(state_path(args.file, args.issue))
    print(yaml.safe_dump(state, sort_keys=False).rstrip())
    return 0


def status(sprint: dict[str, Any], repo: str | None, sprint_file: Path) -> None:
    repo = repo or origin_repo()
    print(f"{sprint['id']} — {sprint['goal']} ({sprint['start']} to {sprint['end']})")
    print(f"  State: {state_path(sprint_file, 0).parent}/ (runtime, git-ignored)")
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
            closed = info["state"] == "closed"
            state = "done" if closed else "open"
            total += 1
            done += closed
            st = read_state(sprint_file, issue)
            summary = state_summary(st, closed=closed) if st else ""
            print(f"    #{issue:<5} {state:<5} {info['title']}{summary}")
    print(f"\n  Progress: {done}/{total} issues closed")


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    v = sub.add_parser("validate")
    v.add_argument("file", type=Path)
    s = sub.add_parser("status")
    s.add_argument("file", type=Path)
    s.add_argument("--repo")
    st = sub.add_parser("state")
    st.add_argument("file", type=Path)
    st.add_argument("issue", type=int)
    st.add_argument("--branch", help="branch of the issue, for example feat/42-mid-year")
    st.add_argument("--pr", type=int, help="number of the pull request of the issue")
    st.add_argument(
        "--step",
        help="current step of the lane loop: spec, tests, implementation, review, pr, done",
    )
    st.add_argument("--blocker", action="append", help="a blocker, repeatable")
    st.add_argument("--clear-blockers", action="store_true", help="empty the blocker list")
    args = parser.parse_args(argv)
    sprint = load(args.file)
    errors = validate(sprint)
    if errors:
        sys.exit("Invalid sprint file:\n  " + "\n  ".join(errors))
    if args.command == "validate":
        print(f"{args.file} is valid.")
    elif args.command == "status":
        status(sprint, args.repo, args.file)
    else:
        state_command(args)
    return 0


if __name__ == "__main__":
    main()
