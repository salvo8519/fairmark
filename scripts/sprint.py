"""Validate a sprint file and report its progress from GitHub.

uv run python scripts/sprint.py validate sprints/S01.yaml
uv run python scripts/sprint.py status sprints/S01.yaml [--repo owner/name]
uv run python scripts/sprint.py state sprints/S01.yaml 42 [--step tests --pr 33 ...]
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

import yaml

REQUIRED = {"id", "goal", "start", "end", "milestone", "status", "lanes"}
STATUSES = {"planned", "active", "done"}
# The per-issue state files are runtime state, never committed (sprints/README.md). They live
# in the git directory shared by every worktree of the repository (issue #333), so the
# orchestrator and every lane read and write the same files. Earlier versions wrote them next
# to the sprint file of the current checkout: that location stays git-ignored, and files found
# there are moved to the shared directory the first time they are read.
STATE_FILE_GITIGNORE = "/sprints/*/state/"
SHARED_STATE = Path("fairmark") / "sprints"


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


def git_common_dir(directory: Path) -> Path | None:
    """The git directory shared by every worktree of the repository, or None outside git."""
    out = subprocess.run(
        ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
        cwd=directory,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    return Path(out.stdout.strip()) if out.returncode == 0 and out.stdout.strip() else None


def legacy_state_dir(sprint_file: Path) -> Path:
    """sprints/S01.yaml -> sprints/S01/state/, the location of the checkout (before #333)."""
    return sprint_file.with_suffix("") / "state"


def state_dir(sprint_file: Path) -> Path:
    """The runtime state directory of a sprint, the same for every worktree.

    Inside a git repository: <git common dir>/fairmark/sprints/<sprint id>/state/, outside the
    working tree, so it is never committed. Outside git (for example a test directory) the
    directory next to the sprint file is used.
    """
    common = git_common_dir(sprint_file.resolve().parent)
    if common is None:
        return legacy_state_dir(sprint_file)
    return common / SHARED_STATE / sprint_file.stem / "state"


def _updated(path: Path) -> str:
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    return str(data.get("updated", "")) if isinstance(data, dict) else ""


def migrate_legacy_state(sprint_file: Path) -> list[Path]:
    """Move state files of the checkout location to the shared directory; returns them.

    When the shared directory already holds a file for the same issue, the more recent one
    (by its `updated` field) is kept. Nothing happens outside git, where both are the same.
    """
    legacy, shared = legacy_state_dir(sprint_file), state_dir(sprint_file)
    if legacy.resolve() == shared.resolve() or not legacy.is_dir():
        return []
    moved = []
    for old in sorted(legacy.glob("*.yaml")):
        new = shared / old.name
        if new.exists() and _updated(new) >= _updated(old):
            old.unlink()
            continue
        new.parent.mkdir(parents=True, exist_ok=True)
        shutil.move(old, new)  # a worktree may sit on another file system
        moved.append(new)
    return moved


def state_path(sprint_file: Path, issue: int) -> Path:
    """The state file of one issue: <state directory>/42.yaml."""
    return state_dir(sprint_file) / f"{issue}.yaml"


def state_files(sprint_file: Path) -> list[Path]:
    """Every per-issue state file of a sprint, after moving any legacy file."""
    migrate_legacy_state(sprint_file)
    directory = state_dir(sprint_file)
    return sorted(directory.glob("*.yaml"), key=lambda p: (len(p.stem), p.stem))


def read_state(sprint_file: Path, issue: int) -> dict[str, Any] | None:
    migrate_legacy_state(sprint_file)
    path = state_path(sprint_file, issue)
    if not path.exists():
        return None
    data = yaml.safe_load(path.read_text(encoding="utf-8"))
    return data if isinstance(data, dict) else None


def write_atomic(path: Path, text: str) -> None:
    """Write a file so that a reader never sees it half written: temporary file, then rename."""
    path.parent.mkdir(parents=True, exist_ok=True)
    tmp = path.with_name(f".{path.name}.{os.getpid()}.tmp")
    tmp.write_text(text, encoding="utf-8")
    os.replace(tmp, path)


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
    and the fields that no argument names are preserved. The file lives in the state
    directory shared by every worktree and is never committed: it survives context resets
    and crashes, not git history (sprints/README.md).
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
    write_atomic(state_path(sprint_file, issue), yaml.safe_dump(state, sort_keys=False))
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
        line += " (stale state file: remove it: it is runtime state, never committed)"
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
    print(f"  State: {state_dir(sprint_file)}/ (runtime, shared by every worktree)")
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
