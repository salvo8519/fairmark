"""The pull request queue: which open pull requests are behind `main`, in conflict or failing.

uv run python scripts/pr_queue.py list [--json] [--stale-days 3] [--fail-on-stale]
uv run python scripts/pr_queue.py update [--pr 42] [--apply]

`list` classifies every open pull request that is not a draft:

| State | Meaning |
| --- | --- |
| conflict | GitHub cannot merge it into `main`: a rebase by hand is needed |
| failing | at least one check run on its head commit concluded `failure` |
| behind | `main` has commits it lacks, and it merges cleanly: `update` can bring it up to date |
| ready | none of the above |

A pull request is stale when it has not changed for `--stale-days` days. With `--fail-on-stale`
the command exits with 1 when a stale pull request is in conflict or failing: the queue is
blocked and someone must act (the scheduled report in test-health.yml uses it).

`update` merges `main` into the branch of every pull request in the state `behind` (or the one
named with `--pr`), through the GitHub API, only with `--apply`; without it, it prints what it
would do. It never touches a pull request in conflict, from a fork, into a branch other than
`main`, or whose head moved since it was read. A refused update (for example a GitHub App
without the Workflows permission, when `main` changed a workflow) is logged and the others go
on; the command then exits with 1. The update must run with a personal or App token: GitHub
does not start workflows for changes made with the `GITHUB_TOKEN` of a workflow, so an update
made that way would never be checked again. The scheduled workflow (test-health.yml) runs it
with a token of the project's GitHub App, whose key only `main` can use (#372).
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import subprocess
import sys
from typing import Any

STATES = ("conflict", "failing", "behind", "ready")


def _gh(path: str, method: str = "GET", **fields: str) -> Any:
    cmd = ["gh", "api", "-X", method, path]
    for key, value in fields.items():
        cmd += ["-f", f"{key}={value}"]
    out = subprocess.run(cmd, capture_output=True, text=True, check=True, encoding="utf-8")
    return json.loads(out.stdout) if out.stdout.strip() else None


def now() -> dt.datetime:
    return dt.datetime.now(dt.UTC)


def origin_repo() -> str:
    url = subprocess.run(
        ["git", "remote", "get-url", "origin"], capture_output=True, text=True, check=True
    ).stdout.strip()
    return url.removesuffix(".git").split("github.com")[-1].lstrip(":/")


def classify(pr: dict[str, Any], behind_by: int, failed: list[str]) -> str:
    """One state per pull request, in the order of STATES: the first that applies."""
    if pr.get("mergeable_state") == "dirty":
        return "conflict"
    if failed:
        return "failing"
    if behind_by > 0:
        return "behind"
    return "ready"


def queue(repo: str, now: dt.datetime, stale_days: int) -> list[dict[str, Any]]:
    """Every open pull request that is not a draft, oldest first, with its state."""
    rows = []
    for summary in _gh(f"repos/{repo}/pulls?state=open&per_page=100"):
        if summary.get("draft"):
            continue
        pr = _gh(f"repos/{repo}/pulls/{summary['number']}")
        head, base = pr["head"], pr["base"]
        compare = _gh(f"repos/{repo}/compare/{head['sha']}...{base['ref']}")
        runs = _gh(f"repos/{repo}/commits/{head['sha']}/check-runs?per_page=100")["check_runs"]
        failed = sorted({r["name"] for r in runs if r["conclusion"] == "failure"})
        updated = dt.datetime.fromisoformat(pr["updated_at"])
        idle = (now - updated).days
        rows.append(
            {
                "number": pr["number"],
                "title": pr["title"],
                "branch": head["ref"],
                "base": base["ref"],
                "head_sha": head["sha"],
                "fork": head["repo"]["full_name"] != base["repo"]["full_name"]
                if head.get("repo")
                else True,
                "behind_by": int(compare["ahead_by"]),
                "failed_checks": failed,
                "days_idle": idle,
                "stale": idle >= stale_days,
                "state": classify(pr, int(compare["ahead_by"]), failed),
            }
        )
    return sorted(rows, key=lambda r: r["number"])


def render(rows: list[dict[str, Any]]) -> str:
    if not rows:
        return "No open pull request."
    lines = ["  #     state     behind  idle  branch"]
    for r in rows:
        stale = "  (stale)" if r["stale"] else ""
        lines.append(
            f"  #{r['number']:<5} {r['state']:<9} {r['behind_by']:>6}  {r['days_idle']:>3}d  "
            f"{r['branch']}{stale}"
        )
        if r["failed_checks"]:
            lines.append(f"         failed: {', '.join(r['failed_checks'])}")
    counts = ", ".join(f"{s} {sum(r['state'] == s for r in rows)}" for s in STATES)
    return "\n".join([*lines, f"  {len(rows)} open: {counts}"])


def blocked(rows: list[dict[str, Any]]) -> list[dict[str, Any]]:
    """Stale pull requests that cannot move without someone: in conflict or failing."""
    return [r for r in rows if r["stale"] and r["state"] in ("conflict", "failing")]


def update(
    repo: str, rows: list[dict[str, Any]], only: int | None, apply: bool
) -> tuple[list[str], int]:
    """Bring the pull requests in the state `behind` up to date with `main`.

    Returns the log and the number of updates GitHub refused."""
    log, refused = [], 0
    for r in rows:
        if only is not None and r["number"] != only:
            continue
        if r["state"] != "behind":
            log.append(f"#{r['number']}: {r['state']}, not updated")
            continue
        if r["fork"]:
            log.append(f"#{r['number']}: from a fork, not updated")
            continue
        if r["base"] != "main":
            log.append(f"#{r['number']}: targets {r['base']}, not main; not updated")
            continue
        if not apply:
            behind = r["behind_by"]
            log.append(f"#{r['number']}: would merge main ({behind} commits); --apply to do it")
            continue
        try:
            _gh(
                f"repos/{repo}/pulls/{r['number']}/update-branch",
                "PUT",
                expected_head_sha=r["head_sha"],
            )
        except subprocess.CalledProcessError as error:
            reason = (error.stderr or "").strip().splitlines() or ["no reason given"]
            log.append(f"#{r['number']}: update refused: {reason[0]}")
            refused += 1
            continue
        log.append(f"#{r['number']}: main merged into {r['branch']}; CI runs again")
    return log, refused


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="The pull request queue.")
    sub = parser.add_subparsers(dest="command", required=True)
    ls = sub.add_parser("list", help="every open pull request with its state")
    ls.add_argument("--json", action="store_true")
    ls.add_argument("--stale-days", type=int, default=3)
    ls.add_argument("--fail-on-stale", action="store_true")
    up = sub.add_parser("update", help="merge main into the pull requests that are behind")
    up.add_argument("--pr", type=int)
    up.add_argument("--apply", action="store_true", help="really update; otherwise a dry run")
    for p in (ls, up):
        p.add_argument("--repo")
    args = parser.parse_args(argv)
    repo = args.repo or origin_repo()
    rows = queue(repo, now(), getattr(args, "stale_days", 3))
    if args.command == "list":
        print(json.dumps(rows, indent=2) if args.json else render(rows))
        return 1 if args.fail_on_stale and blocked(rows) else 0
    log, refused = update(repo, rows, args.pr, args.apply)
    print("\n".join(log) or "Nothing to update.")
    return 1 if refused else 0


if __name__ == "__main__":
    sys.exit(main())
