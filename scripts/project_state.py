"""Where the project stands, for the start of a working session (human or agent).

Reads git and GitHub (REST, through the GitHub CLI) and prints a Markdown briefing:
current branch and its issue, uncommitted work, recent session journal, open pull requests
with CI and review state, the active sprint, issues in progress and issues ready to start.

    uv run python scripts/project_state.py            # briefing in Markdown
    uv run python scripts/project_state.py --json     # the same data, for agents
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
JOURNAL = ROOT / ".agents" / "journal"


def sh(*args: str) -> str:
    out = subprocess.run(args, cwd=ROOT, capture_output=True, text=True, encoding="utf-8")
    return out.stdout.strip() if out.returncode == 0 else ""


def gh(path: str) -> Any:
    text = sh("gh", "api", path)
    return json.loads(text) if text else None


def repo() -> str:
    url = sh("git", "remote", "get-url", "origin")
    return re.sub(r"(\.git)?$", "", re.split(r"github\.com[:/]", url)[-1])


def checks_summary(r: str, sha: str) -> str:
    data = gh(f"repos/{r}/commits/{sha}/check-runs?per_page=100") or {}
    runs = data.get("check_runs", [])
    if not runs:
        return "no checks"
    pending = sum(c["status"] != "completed" for c in runs)
    failed = [c["name"] for c in runs if c.get("conclusion") in ("failure", "timed_out")]
    if failed:
        return "failing: " + ", ".join(sorted(set(failed)))
    return f"running ({pending})" if pending else "green"


def collect() -> dict[str, Any]:
    r = repo()
    branch = sh("git", "rev-parse", "--abbrev-ref", "HEAD")
    sh("git", "fetch", "--quiet", "origin", "main")
    issue = re.search(r"/(\d+)-", "/" + branch)
    state: dict[str, Any] = {
        "repository": r,
        "branch": branch,
        "branch_issue": int(issue.group(1)) if issue else None,
        "uncommitted_files": [ln for ln in sh("git", "status", "--porcelain").splitlines() if ln],
        "ahead_of_main": sh("git", "rev-list", "--count", "origin/main..HEAD") or "0",
        "behind_main": sh("git", "rev-list", "--count", "HEAD..origin/main") or "0",
        "recent_commits": sh("git", "log", "-8", "--format=%h %s").splitlines(),
        "journal": [
            p.name for p in sorted(JOURNAL.glob("*.md"), reverse=True) if p.name != "README.md"
        ][:5],
    }
    prs = gh(f"repos/{r}/pulls?state=open&per_page=50") or []
    state["pull_requests"] = [
        {
            "number": p["number"],
            "title": p["title"],
            "branch": p["head"]["ref"],
            "draft": p["draft"],
            "labels": [lbl["name"] for lbl in p["labels"]],
            "checks": checks_summary(r, p["head"]["sha"]),
            "reviews": sorted(
                {rv["state"] for rv in (gh(f"repos/{r}/pulls/{p['number']}/reviews") or [])}
            ),
        }
        for p in prs
    ]
    issues = [
        i
        for i in (gh(f"repos/{r}/issues?state=open&per_page=100") or [])
        if "pull_request" not in i
    ]

    def labels(i: dict[str, Any]) -> set[str]:
        return {lbl["name"] for lbl in i["labels"]}

    state["in_progress"] = [
        f"#{i['number']} {i['title']}" for i in issues if "in-progress" in labels(i)
    ]
    state["blocked"] = [f"#{i['number']} {i['title']}" for i in issues if "blocked" in labels(i)]
    sprint = None
    for f in sorted((ROOT / "sprints").glob("S*.yaml")):
        data = yaml.safe_load(f.read_text(encoding="utf-8"))
        if data.get("status") == "active":
            sprint = {
                "file": f.name,
                "goal": data["goal"],
                "end": str(data["end"]),
                "issues": [n for lane in data["lanes"] for n in lane.get("issues", [])],
            }
    state["active_sprint"] = sprint
    open_numbers = {i["number"] for i in issues}
    if sprint:
        pool = [i for i in issues if i["number"] in sprint["issues"]]
        state["sprint_done"] = [n for n in sprint["issues"] if n not in open_numbers]
    else:
        milestones = sorted({i["milestone"]["title"] for i in issues if i.get("milestone")})
        current = milestones[0] if milestones else None
        pool = [i for i in issues if i.get("milestone") and i["milestone"]["title"] == current]
        state["current_milestone"] = current
    busy = {pr["branch"] for pr in state["pull_requests"]}
    state["ready"] = [
        {
            "number": i["number"],
            "title": i["title"],
            "parallel": "parallel" in labels(i),
            "areas": sorted(lbl for lbl in labels(i) if lbl.startswith("area:")),
        }
        for i in pool
        if not labels(i) & {"epic", "in-progress", "blocked"}
        and not any(f"/{i['number']}-" in "/" + b for b in busy)
    ][:12]
    return state


def render(s: dict[str, Any]) -> str:
    out = [
        f"# Session briefing — {s['repository']}",
        "",
        f"Branch `{s['branch']}`"
        + (f", issue #{s['branch_issue']}" if s["branch_issue"] else "")
        + f"; {s['ahead_of_main']} commit(s) ahead of main, {s['behind_main']} behind; "
        f"{len(s['uncommitted_files'])} uncommitted file(s).",
        "",
        "## Recent commits",
        *[f"- {c}" for c in s["recent_commits"]],
        "",
        "## Latest journal entries",
        *([f"- .agents/journal/{j}" for j in s["journal"]] or ["- none"]),
        "",
        "## Open pull requests",
    ]
    if s["pull_requests"]:
        out += ["| PR | Title | Checks | Reviews | Labels |", "| --- | --- | --- | --- | --- |"]
        for p in s["pull_requests"]:
            draft = " (draft)" if p["draft"] else ""
            out.append(
                f"| #{p['number']}{draft} | {p['title']} | {p['checks']} | "
                f"{', '.join(p['reviews']) or 'none'} | {', '.join(p['labels'])} |"
            )
    else:
        out.append("None.")
    sp = s["active_sprint"]
    out += ["", "## Sprint"]
    if sp:
        out.append(
            f"{sp['file']}: {sp['goal']} (ends {sp['end']}); "
            f"{len(s['sprint_done'])}/{len(sp['issues'])} issues closed."
        )
    else:
        out.append(f"No active sprint. Current milestone: {s.get('current_milestone')}.")
    out += [
        "",
        "## In progress",
        *([f"- {x}" for x in s["in_progress"]] or ["- none"]),
        "",
        "## Blocked",
        *([f"- {x}" for x in s["blocked"]] or ["- none"]),
        "",
        "## Ready to start",
        "| Issue | Title | Parallel | Areas |",
        "| --- | --- | --- | --- |",
    ]
    out += [
        f"| #{i['number']} | {i['title']} | {'yes' if i['parallel'] else 'no'} | "
        f"{', '.join(i['areas'])} |"
        for i in s["ready"]
    ]
    return "\n".join(out)


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--json", action="store_true")
    args = parser.parse_args()
    state = collect()
    print(json.dumps(state, indent=2) if args.json else render(state))


if __name__ == "__main__":
    main()
