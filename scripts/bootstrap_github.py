"""Create labels, milestones, epics and tasks on GitHub from roadmap/roadmap.yaml.

Requires the GitHub CLI (`gh auth login`) and PyYAML. Idempotent: existing labels,
milestones and issues (matched by title) are left untouched.

    python scripts/bootstrap_github.py --repo OWNER/fairmark --dry-run
    python scripts/bootstrap_github.py --repo OWNER/fairmark
"""

from __future__ import annotations

import argparse
import json
import subprocess
from pathlib import Path
from typing import Any

import yaml

ROADMAP = Path(__file__).resolve().parents[1] / "roadmap" / "roadmap.yaml"
TAG_LABELS = {
    "parallel": "parallel",
    "gfi": "good first issue",
    "maintainer": "maintainer",
    "needs-methodology": "needs-methodology",
}


class GitHub:
    def __init__(self, repo: str, dry_run: bool) -> None:
        self.repo, self.dry_run = repo, dry_run

    def api(self, method: str, path: str, **fields: Any) -> Any:
        if self.dry_run and method != "GET":
            print(f"[dry-run] {method} {path} {json.dumps(fields)[:120]}")
            return {"number": 0, "id": 0}
        cmd = ["gh", "api", "-X", method, f"repos/{self.repo}/{path}", "--input", "-"]
        if method == "GET":
            cmd = ["gh", "api", "--paginate", f"repos/{self.repo}/{path}"]
        out = subprocess.run(
            cmd, input=json.dumps(fields), capture_output=True, text=True, check=True
        )
        return json.loads(out.stdout) if out.stdout.strip() else None


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--repo", required=True)
    parser.add_argument("--dry-run", action="store_true")
    args = parser.parse_args()

    gh = GitHub(args.repo, args.dry_run)
    roadmap = yaml.safe_load(ROADMAP.read_text())

    existing_labels = {lbl["name"] for lbl in gh.api("GET", "labels?per_page=100")}
    for lbl in roadmap["labels"]:
        if lbl["name"] not in existing_labels:
            gh.api("POST", "labels", name=lbl["name"], color=lbl["color"])

    milestones = {
        m["title"]: m["number"] for m in gh.api("GET", "milestones?state=all&per_page=100")
    }
    issues = {i["title"]: i for i in gh.api("GET", "issues?state=all&per_page=100")}

    for ms in roadmap["milestones"]:
        if ms["title"] not in milestones:
            created = gh.api(
                "POST", "milestones", title=ms["title"], description=f"Gate: {ms['gate']}"
            )
            milestones[ms["title"]] = created["number"]
        number = milestones[ms["title"]]

        for epic in ms["epics"]:
            epic_title = f"[Epic] {epic['title']}"
            if epic_title in issues:
                continue
            body = f"Milestone gate: {ms['gate']}\n\nTasks are linked as sub-issues."
            parent = gh.api(
                "POST",
                "issues",
                title=epic_title,
                body=body,
                milestone=number,
                labels=["epic", *epic.get("labels", [])],
            )
            for task in epic["tasks"]:
                labels = [*epic.get("labels", []), *(TAG_LABELS[t] for t in task.get("tags", []))]
                child = gh.api(
                    "POST",
                    "issues",
                    title=task["title"],
                    milestone=number,
                    labels=labels,
                    body=f"Part of #{parent['number']}.",
                )
                gh.api("POST", f"issues/{parent['number']}/sub_issues", sub_issue_id=child["id"])
                if task.get("done"):
                    gh.api("PATCH", f"issues/{child['number']}", state="closed")


if __name__ == "__main__":
    main()
