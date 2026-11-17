"""Validate a sprint file and report its progress from GitHub.

uv run python scripts/sprint.py validate sprints/S01.yaml
uv run python scripts/sprint.py status sprints/S01.yaml [--repo owner/name]
uv run python scripts/sprint.py state sprints/S01.yaml 42 [--step tests --pr 33 ...]
uv run python scripts/sprint.py burndown sprints/S01.yaml [--json]
uv run python scripts/sprint.py check-diff --branch sprint/S01/42-slug --base origin/main
"""

from __future__ import annotations

import argparse
import datetime as dt
import fnmatch
import functools
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

# Files written by `uv run just generate` (sprints/README.md, Generated artifacts and the merge
# order), grouped by the generator that writes them (#347). They belong to no single lane: only
# the lane that names the group in `generates` may change them, one lane per group.
# README.md, README.it.md and the methods indexes are listed whole because their generated
# block cannot be separated from the rest of the file in a diff.
GENERATED_GROUPS: dict[str, tuple[str, ...]] = {
    # scripts/schema_snapshots.py, gen_reference.py, tool_examples.py: from the method registry
    "registry": (
        "schemas/**",
        "docs/*/reference/methods.md",
        "docs/*/methods/index.md",
        "README.md",
        "README.it.md",
        "python/fairmark/tool_examples.json",
    ),
    # scripts/sync_agents.py: the mirrors of .agents/, one per agent system (#328)
    "agents": (".claude/**", ".codex/agents/**", ".cursor/agents/**", ".github/agents/**"),
}
GENERATED = tuple(p for group in GENERATED_GROUPS.values() for p in group)
# Files every pull request of a sprint may change, whatever its lane: the roadmap line of the
# issue it closes, the session journal and the release note fragment.
ALWAYS_ALLOWED = ("roadmap/roadmap.yaml", ".agents/journal/**", "changes/**")
SPRINT_BRANCH = re.compile(r"^sprint/(?P<sprint>S\d+)/(?P<issue>\d+)-")


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
    errors += validate_paths(sprint["lanes"])
    return errors


# ------------------------------------------------------------------------------------------
# Lane paths (issue #334): each lane declares the files it may change, as glob patterns.
#
# Pattern grammar, relative to the repository root, `/` as separator: `*` matches within one
# path segment, `**` matches any number of segments, `?` one character; a pattern that ends
# with `/` stands for the directory and everything below it.


def normalise_pattern(pattern: str) -> str:
    pattern = pattern.strip().removeprefix("./")
    return pattern + "**" if pattern.endswith("/") else pattern


def pattern_regex(pattern: str) -> re.Pattern[str]:
    out, i, p = [], 0, normalise_pattern(pattern)
    while i < len(p):
        if p.startswith("**/", i):
            out.append("(?:.*/)?")
            i += 3
        elif p.startswith("**", i):
            out.append(".*")
            i += 2
        elif p[i] == "*":
            out.append("[^/]*")
            i += 1
        elif p[i] == "?":
            out.append("[^/]")
            i += 1
        else:
            out.append(re.escape(p[i]))
            i += 1
    return re.compile("".join(out) + r"\Z")


def matches(path: str, pattern: str) -> bool:
    return pattern_regex(pattern).match(path) is not None


def _has_wildcard(segment: str) -> bool:
    return "*" in segment or "?" in segment


def _segment_witness(a: str, b: str) -> str | None:
    """A file or directory name matched by both segment patterns, or None."""
    if not _has_wildcard(a):
        return a if fnmatch.fnmatchcase(a, b) else None
    if not _has_wildcard(b):
        return b if fnmatch.fnmatchcase(b, a) else None
    head_a, tail_a = re.split(r"[*?]", a)[0], re.split(r"[*?]", a)[-1]
    head_b, tail_b = re.split(r"[*?]", b)[0], re.split(r"[*?]", b)[-1]
    if not (head_a.startswith(head_b) or head_b.startswith(head_a)):
        return None
    if not (tail_a.endswith(tail_b) or tail_b.endswith(tail_a)):
        return None
    head = max(head_a, head_b, key=len)
    tail = max(tail_a, tail_b, key=len)
    return f"{head}x{tail}"


def overlap_example(a: str, b: str) -> str | None:
    """A path that both patterns match, or None when no path can match both.

    The patterns are compared segment by segment; `**` stands for any number of segments.
    Between two wildcards in the same segment the check keeps only their fixed beginning and
    end, which is exact for the patterns lanes use (`*.md`, `test_*.py`).
    """
    sa = normalise_pattern(a).split("/")
    sb = normalise_pattern(b).split("/")

    @functools.cache
    def walk(i: int, j: int) -> tuple[str, ...] | None:
        if i == len(sa) and j == len(sb):
            return ()
        if i < len(sa) and sa[i] == "**":
            rest = walk(i + 1, j)
            if rest is not None:
                return rest
            if j < len(sb):
                tail = walk(i, j + 1)
                if tail is not None:
                    return ("x" if sb[j] == "**" else _segment_witness(sb[j], "*") or "x", *tail)
            return None
        if j < len(sb) and sb[j] == "**":
            rest = walk(i, j + 1)
            if rest is not None:
                return rest
            if i < len(sa):
                tail = walk(i + 1, j)
                if tail is not None:
                    return (_segment_witness(sa[i], "*") or "x", *tail)
            return None
        if i == len(sa) or j == len(sb):
            return None
        segment = _segment_witness(sa[i], sb[j])
        if segment is None:
            return None
        tail = walk(i + 1, j + 1)
        return None if tail is None else (segment, *tail)

    found = walk(0, 0)
    if found is None:
        return None
    witness = "/".join(found)
    # `dir/**` needs at least one name below the directory to match a file.
    for candidate in (witness, f"{witness}/x" if witness else "x"):
        if matches(candidate, a) and matches(candidate, b):
            return candidate
    return witness


def generated_groups(lane: dict[str, Any]) -> list[str]:
    """The generator groups a lane regenerates; invalid values are reported by validation."""
    groups = lane.get("generates", [])
    return [g for g in groups if g in GENERATED_GROUPS] if isinstance(groups, list) else []


def validate_paths(lanes: list[dict[str, Any]]) -> list[str]:
    errors: list[str] = []
    known = ", ".join(GENERATED_GROUPS)
    owners: dict[str, list[str]] = {g: [] for g in GENERATED_GROUPS}
    for lane in lanes:
        groups = lane.get("generates", [])
        if not isinstance(groups, list) or any(g not in GENERATED_GROUPS for g in groups):
            errors.append(
                f"lane {lane.get('name')}: 'generates' must be a list of groups ({known}), "
                f"got {groups!r}"
            )
            continue
        for group in groups:
            owners[group].append(str(lane.get("name")))
    for group, names in owners.items():
        if len(names) > 1:
            errors.append(f"only one lane may generate the {group} artifacts; lanes: {names}")
    for lane in lanes:
        name, paths = lane.get("name"), lane.get("paths")
        if not isinstance(paths, list) or not paths or not all(isinstance(p, str) for p in paths):
            errors.append(f"lane {name}: 'paths' must be a non-empty list of glob patterns")
            continue
        own = generated_groups(lane)
        for path in paths:
            for group, patterns in GENERATED_GROUPS.items():
                if group in own:
                    continue
                for generated in patterns:
                    example = overlap_example(path, generated)
                    if example:
                        errors.append(
                            f"lane {name}: '{path}' covers the generated artifact '{example}' "
                            f"of the {group} group; only the lane with '{group}' in "
                            "'generates' may change it"
                        )
    valid = [lane for lane in lanes if isinstance(lane.get("paths"), list)]
    for i, first in enumerate(valid):
        for second in valid[i + 1 :]:
            for a in first["paths"]:
                for b in second["paths"]:
                    example = overlap_example(str(a), str(b))
                    if example:
                        errors.append(
                            f"lanes {first.get('name')} and {second.get('name')} overlap: "
                            f"'{a}' and '{b}' both cover '{example}'"
                        )
    return errors


def allowed_patterns(lane: dict[str, Any]) -> list[str]:
    generated = [p for g in generated_groups(lane) for p in GENERATED_GROUPS[g]]
    return [*lane["paths"], *ALWAYS_ALLOWED, *generated]


def files_outside_lane(lane: dict[str, Any], changed: list[str]) -> list[str]:
    patterns = allowed_patterns(lane)
    return [f for f in changed if not any(matches(f, p) for p in patterns)]


def changed_files(base: str) -> list[str]:
    out = subprocess.run(
        ["git", "diff", "--name-only", "--no-renames", f"{base}...HEAD"],
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=False,
    )
    if out.returncode != 0:
        out = subprocess.run(
            ["git", "diff", "--name-only", "--no-renames", base, "HEAD"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            check=True,
        )
    return [line for line in out.stdout.splitlines() if line]


def check_diff_command(args: argparse.Namespace) -> int:
    m = SPRINT_BRANCH.match(args.branch)
    if m is None:
        print(f"{args.branch} is not a sprint branch (sprint/SNN/<issue>-<slug>): nothing to check")
        return 0
    sprint_file = Path(args.sprints) / f"{m['sprint']}.yaml"
    if not sprint_file.exists():
        sys.exit(f"{args.branch}: {sprint_file} does not exist")
    sprint = load(sprint_file)
    errors = validate(sprint)
    if errors:
        sys.exit(f"Invalid sprint file {sprint_file}:\n  " + "\n  ".join(errors))
    issue = int(m["issue"])
    lane = next((x for x in sprint["lanes"] if issue in x.get("issues", [])), None)
    if lane is None:
        sys.exit(f"#{issue} is not in the lanes of {sprint['id']}")
    outside = files_outside_lane(lane, changed_files(args.base))
    if outside:
        sys.exit(
            f"#{issue} belongs to lane {lane['name']} of {sprint['id']}, which may change "
            f"{allowed_patterns(lane)}.\nFiles outside the lane:\n  " + "\n  ".join(outside)
        )
    print(f"#{issue}: every changed file is inside lane {lane['name']} of {sprint['id']}.")
    return 0


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


def ledger_path(sprint_file: Path) -> Path:
    """The event ledger of a sprint, beside its state directory: one JSON line per checkpoint."""
    return state_dir(sprint_file).parent / "ledger.jsonl"


def append_ledger(sprint_file: Path, event: dict[str, Any]) -> None:
    """Append one event as one line, in a single write of a file opened for appending, so
    that checkpoints written at the same time by parallel lanes never interleave."""
    path = ledger_path(sprint_file)
    path.parent.mkdir(parents=True, exist_ok=True)
    line = (json.dumps(event, sort_keys=True, ensure_ascii=False) + "\n").encode("utf-8")
    fd = os.open(path, os.O_WRONLY | os.O_APPEND | os.O_CREAT, 0o644)
    try:
        os.write(fd, line)
    finally:
        os.close(fd)


def read_ledger(sprint_file: Path) -> list[dict[str, Any]]:
    """Every event of the sprint ledger, oldest first; a line that is not JSON is skipped."""
    path = ledger_path(sprint_file)
    if not path.exists():
        return []
    events = []
    for line in path.read_text(encoding="utf-8").splitlines():
        try:
            event = json.loads(line)
        except json.JSONDecodeError:
            continue
        if isinstance(event, dict):
            events.append(event)
    return sorted(events, key=lambda e: str(e.get("at", "")))


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

    Every call is one orchestrator step on the issue: `agent_steps` counts them. It is the
    agent cost of the issue, measured in orchestrator steps, not in tokens: the scripts cannot
    see token usage. `updated` moves with each call, `started` is kept from the first one,
    `blocked_since` records when the issue became blocked (null when it is not), and the
    fields that no argument names are preserved. Each call also appends one event to the
    sprint ledger (`ledger_path`), from which the burn-down is computed. The file lives in the state
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
    # When the issue became blocked; cleared when its blockers are.
    if state["blockers"] and not state.get("blocked_since"):
        state["blocked_since"] = state["updated"]
    elif not state["blockers"]:
        state["blocked_since"] = None
    write_atomic(state_path(sprint_file, issue), yaml.safe_dump(state, sort_keys=False))
    append_ledger(
        sprint_file,
        {
            "at": state["updated"],
            "issue": issue,
            "lane": state.get("lane"),
            "step": state.get("step"),
            "pull_request": state.get("pull_request"),
            "blockers": len(state["blockers"]),
            "agent_steps": state["agent_steps"],
        },
    )
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


# ------------------------------------------------------------------------------------------
# Burn-down and loop metrics (issue #320), from the ledger and from GitHub.
#
# What each figure means (constitution IX, one meaning per name):
# - per day, every issue of the sprint is in exactly one state: `done` (closed on GitHub that
#   day or before), `in_review` (one of its pull requests open at the end of that day),
#   `in_progress` (a ledger checkpoint that day or before), `open` (none of these);
# - cycle time: from the first ledger checkpoint of the issue to its closing on GitHub, in
#   hours; an issue closed without any checkpoint is counted as not measured, never guessed;
# - failed gates: check runs that concluded `failure` on any commit of the pull requests of the
#   issue (the pull requests opened from a branch of the issue), counted per lane;
# - agent steps: orchestrator steps recorded in the ledger, not tokens.


def _day(timestamp: str | None) -> dt.date | None:
    return dt.datetime.fromisoformat(timestamp).date() if timestamp else None


def issue_state_on(day: dt.date, first_checkpoint: str | None, facts: dict[str, Any]) -> str:
    closed = _day(facts.get("closed_at"))
    if closed and closed <= day:
        return "done"
    for pr in facts.get("pull_requests", []):
        opened, shut = _day(pr.get("created_at")), _day(pr.get("closed_at"))
        if opened and opened <= day and (shut is None or shut > day):
            return "in_review"
    started = _day(first_checkpoint)
    if started and started <= day:
        return "in_progress"
    return "open"


def burndown(
    sprint: dict[str, Any],
    events: list[dict[str, Any]],
    facts: dict[int, dict[str, Any]],
    today: dt.date,
) -> dict[str, Any]:
    """The burn-down of a sprint: computed from the ledger events and the GitHub facts of each
    issue (`closed_at`, `pull_requests` with `created_at` and `closed_at`, `failed_checks`).
    No I/O."""
    lanes = {i: lane["name"] for lane in sprint["lanes"] for i in lane.get("issues", [])}
    first: dict[int, str] = {}
    last: dict[int, dict[str, Any]] = {}
    steps: dict[int, int] = {}
    for event in events:
        issue = event.get("issue")
        if not isinstance(issue, int) or issue not in lanes:
            continue
        first.setdefault(issue, str(event["at"]))
        last[issue] = event
        steps[issue] = max(steps.get(issue, 0), int(event.get("agent_steps") or 0))
    start = (
        sprint["start"]
        if isinstance(sprint["start"], dt.date)
        else dt.date.fromisoformat(str(sprint["start"]))
    )
    end = (
        sprint["end"]
        if isinstance(sprint["end"], dt.date)
        else dt.date.fromisoformat(str(sprint["end"]))
    )
    days = []
    day = start
    while day <= min(end, today):
        counts = {"open": 0, "in_progress": 0, "in_review": 0, "done": 0}
        for issue in lanes:
            counts[issue_state_on(day, first.get(issue), facts.get(issue, {}))] += 1
        days.append({"day": day.isoformat(), **counts})
        day += dt.timedelta(days=1)
    cycle: dict[int, float] = {}
    not_measured = []
    for issue in lanes:
        closed = facts.get(issue, {}).get("closed_at")
        if not closed:
            continue
        if issue not in first:
            not_measured.append(issue)
            continue
        hours = (
            dt.datetime.fromisoformat(closed) - dt.datetime.fromisoformat(first[issue])
        ).total_seconds() / 3600
        cycle[issue] = round(hours, 1)
    ordered = sorted(cycle.values())
    median = None
    if ordered:
        mid = len(ordered) // 2
        median = (
            ordered[mid] if len(ordered) % 2 else round((ordered[mid - 1] + ordered[mid]) / 2, 1)
        )
    failed: dict[str, int] = {lane["name"]: 0 for lane in sprint["lanes"]}
    for issue, lane in lanes.items():
        failed[lane] += int(facts.get(issue, {}).get("failed_checks", 0))
    blocked = {
        issue: event["at"]
        for issue, event in last.items()
        if event.get("blockers") and not facts.get(issue, {}).get("closed_at")
    }
    return {
        "sprint": sprint["id"],
        "days": days,
        "cycle_time_hours": {str(k): v for k, v in sorted(cycle.items())},
        "cycle_time_median_hours": median,
        "closed_without_checkpoint": sorted(not_measured),
        "failed_gates_per_lane": failed,
        "agent_steps": {str(k): v for k, v in sorted(steps.items())},
        "blocked": {str(k): v for k, v in sorted(blocked.items())},
    }


def _gh(path: str) -> Any:
    out = subprocess.run(
        ["gh", "api", path], capture_output=True, text=True, check=True, encoding="utf-8"
    )
    return json.loads(out.stdout)


def issue_branch(issue: int) -> re.Pattern[str]:
    """The branches of an issue: `<type>/<issue>-<slug>` or `sprint/SNN/<issue>-<slug>`."""
    return re.compile(rf"^(?:[a-z]+|sprint/S\d+)/{issue}-")


def pull_requests_of(repo: str, issue: int, ledger_pr: int | None) -> list[dict[str, Any]]:
    """The pull requests of an issue: the one recorded in the ledger, and every pull request
    that cross-references the issue from a branch of that issue. A pull request that only
    mentions the issue (a roadmap update, for example) is not one of its pull requests."""
    numbers = {ledger_pr} if ledger_pr else set()
    timeline = _gh(f"repos/{repo}/issues/{issue}/timeline?per_page=100")
    numbers |= {
        e["source"]["issue"]["number"]
        for e in timeline
        if e.get("event") == "cross-referenced"
        and (e.get("source") or {}).get("issue", {}).get("pull_request")
    }
    branch = issue_branch(issue)
    pulls = [_gh(f"repos/{repo}/pulls/{n}") for n in sorted(numbers)]
    return [pr for pr in pulls if pr["number"] == ledger_pr or branch.match(pr["head"]["ref"])]


def github_facts(
    repo: str, sprint: dict[str, Any], events: list[dict[str, Any]]
) -> dict[int, dict[str, Any]]:
    """For each issue of the sprint: `closed_at` (not for issues closed as not planned) and its
    pull requests: the first opening date, and the check runs that failed on their commits,
    summed over every pull request of the issue (a closed attempt counts too)."""
    recorded = {e["issue"]: e.get("pull_request") for e in events if e.get("pull_request")}
    facts: dict[int, dict[str, Any]] = {}
    for lane in sprint["lanes"]:
        for issue in lane.get("issues", []):
            info = _gh(f"repos/{repo}/issues/{issue}")
            closed = info.get("closed_at") if info.get("state_reason") != "not_planned" else None
            entry: dict[str, Any] = {"title": info["title"], "closed_at": closed}
            pulls = pull_requests_of(repo, issue, recorded.get(issue))
            if pulls:
                # Two pull requests of one issue may share commits: each commit counts once.
                shas = {
                    c["sha"]
                    for pr in pulls
                    for c in _gh(f"repos/{repo}/pulls/{pr['number']}/commits?per_page=100")
                }
                failed = 0
                for sha in sorted(shas):
                    runs = _gh(f"repos/{repo}/commits/{sha}/check-runs?per_page=100")
                    failed += sum(r["conclusion"] == "failure" for r in runs["check_runs"])
                entry["pull_requests"] = [
                    {
                        "number": pr["number"],
                        "created_at": pr["created_at"],
                        "closed_at": pr["closed_at"],
                    }
                    for pr in pulls
                ]
                entry["failed_checks"] = failed
            facts[issue] = entry
    return facts


def render_burndown(report: dict[str, Any]) -> str:
    lines = [f"{report['sprint']} burn-down", "  day          open  progress  review  done"]
    for d in report["days"]:
        lines.append(
            f"  {d['day']}  {d['open']:>4}  {d['in_progress']:>8}  {d['in_review']:>6}  "
            f"{d['done']:>4}"
        )
    cycle = report["cycle_time_hours"]
    if cycle:
        per_issue = ", ".join(f"#{k} {v} h" for k, v in cycle.items())
        lines.append(
            f"  Cycle time (first checkpoint to close): {per_issue}; "
            f"median {report['cycle_time_median_hours']} h over {len(cycle)} issues"
        )
    else:
        lines.append("  Cycle time: no issue closed after a ledger checkpoint yet")
    if report["closed_without_checkpoint"]:
        listed = ", ".join(f"#{i}" for i in report["closed_without_checkpoint"])
        lines.append(f"  Closed without a ledger checkpoint (cycle time not measured): {listed}")
    gates = ", ".join(f"{lane} {n}" for lane, n in report["failed_gates_per_lane"].items())
    lines.append(f"  Failed check runs per lane: {gates}")
    steps = ", ".join(f"#{k} {v}" for k, v in report["agent_steps"].items()) or "none recorded"
    lines.append(f"  Agent steps (orchestrator steps, not tokens): {steps}")
    blocked = ", ".join(f"#{k} since {v}" for k, v in report["blocked"].items()) or "none"
    lines.append(f"  Blocked now: {blocked}")
    return "\n".join(lines)


def burndown_command(sprint: dict[str, Any], args: argparse.Namespace) -> None:
    events = read_ledger(args.file)
    facts = github_facts(args.repo or origin_repo(), sprint, events)
    report = burndown(sprint, events, facts, _now().date())
    print(json.dumps(report, indent=2) if args.json else render_burndown(report))


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="command", required=True)
    v = sub.add_parser("validate")
    v.add_argument("file", type=Path)
    s = sub.add_parser("status")
    s.add_argument("file", type=Path)
    s.add_argument("--repo")
    bd = sub.add_parser("burndown", help="burn-down and loop metrics of a sprint")
    bd.add_argument("file", type=Path)
    bd.add_argument("--repo")
    bd.add_argument("--json", action="store_true", help="machine-readable, for agents")
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
    cd = sub.add_parser(
        "check-diff", help="fail when a sprint branch changes files outside its lane"
    )
    cd.add_argument("--branch", required=True, help="head branch, for example sprint/S01/42-slug")
    cd.add_argument("--base", required=True, help="base revision, for example origin/main")
    cd.add_argument("--sprints", default="sprints", help="directory of the sprint files")
    args = parser.parse_args(argv)
    if args.command == "check-diff":
        return check_diff_command(args)
    sprint = load(args.file)
    errors = validate(sprint)
    if errors:
        sys.exit("Invalid sprint file:\n  " + "\n  ".join(errors))
    if args.command == "validate":
        print(f"{args.file} is valid.")
    elif args.command == "status":
        status(sprint, args.repo, args.file)
    elif args.command == "burndown":
        burndown_command(sprint, args)
    else:
        state_command(args)
    return 0


if __name__ == "__main__":
    main()
