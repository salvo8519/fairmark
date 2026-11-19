"""One issue per failing workflow on main: opened, updated, and closed when green again.

Called by `.github/workflows/failure-issue.yml` when a workflow that runs on `main` or on a
schedule completes (CI, Docs, Security, Roadmap sync), and by the report jobs of the workflows
that keep their own issue (Weekly maintenance, Mutation testing, Test health). Uses only the
standard library and the GitHub CLI (`gh`, authenticated through `GH_TOKEN`).

- A failed run (`failure`, `timed_out`, `startup_failure`) opens an issue with the label, the
  failing jobs and the link to the log, or updates the open one: its body is replaced with the
  latest report and a comment links the new run.
- A successful run closes the open issue with a comment that links the run.
- Any other conclusion, and any run that is not a push to or a schedule on `main`, is ignored.

The issue of a workflow is found among the open issues with the label that the workflow itself
opened (author `app/github-actions`), by a marker in its body or by its exact title (issues
opened before the marker existed). Issues opened by people are never edited or closed. A run
already linked in the body is not reported twice.

    python3 scripts/failure_issue.py report --repo OWNER/NAME --workflow CI --run-id 1 \\
        --run-url URL --conclusion failure --event push --branch main [--label bug] \\
        [--title TITLE] [--jobs "a=failure, b=success"] [--details FILE]
"""

from __future__ import annotations

import argparse
import json
import subprocess
import sys
from collections.abc import Callable, Sequence
from pathlib import Path
from typing import Any

FAILED = {"failure", "timed_out", "startup_failure"}
EVENTS = {"push", "schedule"}
BRANCH = "main"
NOT_SUCCEEDED = '.jobs[] | select(.conclusion != "success" and .conclusion != "skipped") | .name'
MAX_BODY = 60_000  # GitHub refuses an issue body longer than 65536 characters

AUTHOR = "app/github-actions"

Gh = Callable[[list[str], str | None], str]


def gh_cli(args: list[str], stdin: str | None = None) -> str:
    """Run the GitHub CLI and return its output; a failure stops the script with its message."""
    done = subprocess.run(
        ["gh", *args], input=stdin, capture_output=True, text=True, encoding="utf-8", check=False
    )
    if done.returncode != 0:
        sys.exit(f"gh {' '.join(args[:2])} failed: {done.stderr.strip()}")
    return done.stdout


def marker(workflow: str) -> str:
    return f"<!-- failure-issue: {workflow} -->"


def find_issue(gh: Gh, repo: str, label: str, workflow: str, title: str) -> dict[str, Any] | None:
    """The open issue the workflow opened: by its marker, or by its exact title."""
    listed = gh(
        [
            "issue", "list", "--repo", repo, "--label", label, "--state", "open",
            "--author", AUTHOR, "--limit", "100", "--json", "number,title,body",
        ],
        None,
    )  # fmt: skip
    issues: list[dict[str, Any]] = json.loads(listed or "[]")
    for issue in issues:
        if marker(workflow) in (issue.get("body") or ""):
            return issue
    for issue in issues:
        if issue.get("title") == title:
            return issue
    return None


def failed_jobs(gh: Gh, repo: str, run_id: int) -> list[str]:
    """Names of the jobs of the run that did not succeed."""
    out = gh(
        [
            "api", f"repos/{repo}/actions/runs/{run_id}/jobs?per_page=100", "--paginate",
            "--jq", NOT_SUCCEEDED,
        ],
        None,
    )  # fmt: skip
    return [line for line in out.splitlines() if line.strip()]


def body(
    workflow: str, run_url: str, conclusion: str, event: str, jobs: list[str], details: str
) -> str:
    lines = [
        marker(workflow),
        f"The workflow **{workflow}** ended with `{conclusion}` on `{BRANCH}` ({event}).",
        "",
        "Jobs that did not succeed:",
        *([f"- {job}" for job in jobs] or ["- none reported"]),
        "",
        f"Log: {run_url}",
        "",
        "Every failure of this workflow updates this issue; the next successful run closes it "
        "(`scripts/failure_issue.py`). Fix the cause at once (a defect is never deferred, "
        "AGENTS.md) in a pull request that references this issue with `Refs #<this issue>`, "
        "not `Closes`: it closes itself when the workflow is green again "
        "(docs/en/project/conventions.md, Failures).",
    ]
    text = "\n".join(lines)
    if details:
        text += "\n\n" + details.strip()
    if len(text) > MAX_BODY:
        text = text[: MAX_BODY - 40] + "\n\n(cut to the size of an issue body)"
    return text


def report(
    gh: Gh,
    *,
    repo: str,
    workflow: str,
    run_id: int,
    run_url: str,
    conclusion: str,
    event: str,
    branch: str,
    label: str,
    title: str | None,
    jobs: list[str] | None,
    details: str,
) -> str:
    """Open, update or close the issue of the workflow; returns what was done."""
    if event not in EVENTS or branch != BRANCH:
        return f"ignored: {event} on {branch}"
    if conclusion != "success" and conclusion not in FAILED:
        return f"ignored: conclusion {conclusion}"
    title = title or f"{workflow} passes again on {BRANCH}"
    issue = find_issue(gh, repo, label, workflow, title)
    number = int(issue["number"]) if issue else None
    if conclusion == "success":
        if number is None:
            return "nothing to close"
        gh(
            [
                "issue", "close", str(number), "--repo", repo,
                "--comment", f"{workflow} is green again: {run_url}",
            ],
            None,
        )  # fmt: skip
        return f"closed #{number}"
    if issue and run_url in (issue.get("body") or ""):
        return f"already reported in #{number}"
    names = jobs if jobs is not None else failed_jobs(gh, repo, run_id)
    text = body(workflow, run_url, conclusion, event, names, details)
    if number is None:
        url = gh(
            [
                "issue", "create", "--repo", repo, "--title", title,
                "--label", label, "--body-file", "-",
            ],
            text,
        )  # fmt: skip
        return f"opened #{url.strip().rsplit('/', 1)[-1]}"
    gh(["issue", "edit", str(number), "--repo", repo, "--body-file", "-"], text)
    gh(
        ["issue", "comment", str(number), "--repo", repo, "--body-file", "-"],
        f"{workflow} failed again ({conclusion}): {run_url}",
    )
    return f"updated #{number}"


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="command", required=True)
    r = sub.add_parser("report", help="open, update or close the issue of a workflow run")
    r.add_argument("--repo", required=True)
    r.add_argument("--workflow", required=True)
    r.add_argument("--run-id", type=int, required=True)
    r.add_argument("--run-url", required=True)
    r.add_argument("--conclusion", required=True)
    r.add_argument("--event", required=True)
    r.add_argument("--branch", required=True)
    r.add_argument("--label", default="bug")
    r.add_argument("--title")
    r.add_argument("--jobs", help="comma-separated job results; default: read from the run")
    r.add_argument("--details", type=Path, help="Markdown file appended to the issue body")
    args = parser.parse_args(argv)
    jobs = None if args.jobs is None else [j.strip() for j in args.jobs.split(",") if j.strip()]
    details = args.details.read_text(encoding="utf-8") if args.details else ""
    print(
        report(
            gh_cli,
            repo=args.repo,
            workflow=args.workflow,
            run_id=args.run_id,
            run_url=args.run_url,
            conclusion=args.conclusion,
            event=args.event,
            branch=args.branch,
            label=args.label,
            title=args.title,
            jobs=jobs,
            details=details,
        )
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
