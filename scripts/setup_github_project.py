"""Create the Fairmark GitHub Project (Kanban) and fill it with every roadmap issue.

GitHub Projects belong to a user or an organization and are managed through the GraphQL API,
so this runs on the maintainer's machine with the GitHub CLI:

    gh auth refresh -s project          # once: grant the 'project' scope
    uv run python scripts/setup_github_project.py --owner salvo8519 --repo salvo8519/fairmark

What it does (idempotent: re-running only adds what is missing):
  1. creates the project "Fairmark roadmap" (or reuses it) and links it to the repository;
  2. adds the fields "Area" and "Type" (Epic / Task);
  3. adds every issue; Status = Done for closed issues, Todo otherwise; Area from the
     `area:*` label; Type from the `epic` label.

Afterwards, in the project page: open the view menu, choose the "Board" layout grouped by
Status, and optionally a "Roadmap" layout grouped by Milestone. For sprints, add once an
"Iteration" field (two-week cycle) in the project settings: the GitHub CLI cannot create it.
Sprint plans for agents live in sprints/ (see sprints/README.md).
"""

from __future__ import annotations

import argparse
import json
import subprocess
from typing import Any

TITLE = "Fairmark roadmap"
AREAS = ["core", "python", "cli", "mcp", "n8n", "docs", "infra", "telemetry"]


def gh(*args: str) -> Any:
    out = subprocess.run(
        ["gh", *args], capture_output=True, text=True, check=True, encoding="utf-8"
    ).stdout
    return json.loads(out) if out.strip().startswith(("{", "[")) else out


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--owner", required=True, help="user or organization owning the project")
    parser.add_argument("--repo", required=True, help="owner/name of the repository")
    args = parser.parse_args()
    owner = args.owner

    projects = gh("project", "list", "--owner", owner, "--format", "json")["projects"]
    project = next((p for p in projects if p["title"] == TITLE), None)
    if project is None:
        project = gh("project", "create", "--owner", owner, "--title", TITLE, "--format", "json")
        print(f"Created project {project['url']}")
    number, project_id = str(project["number"]), project["id"]
    subprocess.run(["gh", "project", "link", number, "--owner", owner, "--repo", args.repo])

    def fields() -> dict[str, Any]:
        data = gh("project", "field-list", number, "--owner", owner, "--format", "json")
        return {f["name"]: f for f in data["fields"]}

    existing = fields()
    if "Area" not in existing:
        gh(
            "project",
            "field-create",
            number,
            "--owner",
            owner,
            "--name",
            "Area",
            "--data-type",
            "SINGLE_SELECT",
            "--single-select-options",
            ",".join(AREAS),
        )
    if "Type" not in existing:
        gh(
            "project",
            "field-create",
            number,
            "--owner",
            owner,
            "--name",
            "Type",
            "--data-type",
            "SINGLE_SELECT",
            "--single-select-options",
            "Epic,Task",
        )
    f = fields()
    option = {
        name: {o["name"]: o["id"] for o in f[name].get("options", [])}
        for name in ("Status", "Area", "Type")
    }

    items = gh(
        "project", "item-list", number, "--owner", owner, "--limit", "1000", "--format", "json"
    )["items"]
    present = {i.get("content", {}).get("url") for i in items}

    issues = gh(
        "issue",
        "list",
        "--repo",
        args.repo,
        "--state",
        "all",
        "--limit",
        "500",
        "--json",
        "url,state,labels,title",
    )
    added = 0
    for issue in sorted(issues, key=lambda i: i["url"]):
        if issue["url"] in present:
            continue
        item = gh(
            "project",
            "item-add",
            number,
            "--owner",
            owner,
            "--url",
            issue["url"],
            "--format",
            "json",
        )
        labels = {lbl["name"] for lbl in issue["labels"]}
        status = "Done" if issue["state"] == "CLOSED" else "Todo"
        edits = [("Status", status), ("Type", "Epic" if "epic" in labels else "Task")]
        area = next(
            (lbl.split(":", 1)[1] for lbl in sorted(labels) if lbl.startswith("area:")), None
        )
        if area in option["Area"]:
            edits.append(("Area", area))
        for field, value in edits:
            if value in option[field]:
                gh(
                    "project",
                    "item-edit",
                    "--id",
                    item["id"],
                    "--project-id",
                    project_id,
                    "--field-id",
                    f[field]["id"],
                    "--single-select-option-id",
                    option[field][value],
                )
        added += 1
    print(f"Added {added} issues. Open: {project['url']}")


if __name__ == "__main__":
    main()
