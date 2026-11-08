"""Naming conventions of Fairmark, as code: commit headers, pull request titles, branches and
issue links. One module, used by the commit-msg hook, by CI and by the agents' skills, so the
rule written in `docs/en/project/conventions.md` and the rule enforced are the same.

    uv run python scripts/conventions.py commit-msg .git/COMMIT_EDITMSG
    uv run python scripts/conventions.py pr           # in GitHub Actions (reads the event)
    uv run python scripts/conventions.py pr --title "feat(dcf): mid-year discounting" \
        --branch feat/42-mid-year --body "Closes #42"
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sys
from pathlib import Path

TYPES = ("feat", "fix", "perf", "refactor", "docs", "test", "build", "ci", "chore", "revert")
SCOPES = (
    # engine: one scope per theme module (ADR 0009), plus the shared parts
    "core", "dates", "rates", "tvm", "loans", "coc", "dcf", "tv", "ratios", "analysis",
    "registry", "explain", "errors", "schemas",
    # adapters
    "cli", "py", "tools", "mcp", "n8n", "logging",
    # documentation and project
    "docs", "i18n", "brand", "specs", "adr", "agents", "project",
    # build and delivery
    "ci", "deps", "release", "scripts",
)  # fmt: skip
MAX_HEADER = 72
BOTS = ("dependabot[bot]", "github-actions[bot]", "renovate[bot]")

HEADER = re.compile(
    r"^(?P<type>[a-z]+)(?:\((?P<scope>[a-z0-9-]+)\))?(?P<breaking>!)?: (?P<subject>.+)$"
)
SPEC_REF = re.compile(r"\[SPEC-\d{4}(?: R\d+(?:,\s*R\d+)*)?\]$")
BRANCH = re.compile(
    rf"^(?P<type>{'|'.join(TYPES)})/(?P<issue>\d+)-(?P<slug>[a-z0-9]+(?:-[a-z0-9]+)*)$"
)
BRANCH_EXEMPT = re.compile(r"^(dependabot/|release/v\d|revert-\d+-)")
ISSUE_LINK = re.compile(r"\b(?:close[sd]?|fix(?:e[sd])?|resolve[sd]?|refs?)\s+#(\d+)\b", re.I)


def check_header(header: str, *, bot: bool = False) -> list[str]:
    """Problems with a Conventional Commit header (commit subject or pull request title)."""
    m = HEADER.match(header)
    if not m:
        return [f"'{header}' is not '<type>(<scope>): <subject>'."]
    problems = []
    if m["type"] not in TYPES:
        problems.append(f"type '{m['type']}' is not one of: {', '.join(TYPES)}.")
    if m["type"] != "revert":
        if not m["scope"]:
            problems.append(f"a scope is required, one of: {', '.join(SCOPES)}.")
        elif m["scope"] not in SCOPES:
            problems.append(f"scope '{m['scope']}' is not one of: {', '.join(SCOPES)}.")
    subject = SPEC_REF.sub("", m["subject"]).strip()
    if not bot:
        if len(header) > MAX_HEADER:
            problems.append(f"header is {len(header)} characters, the limit is {MAX_HEADER}.")
        if subject[:1].isupper():
            problems.append("subject starts with a lowercase imperative verb ('add', not 'Added').")
        if subject.endswith("."):
            problems.append("subject does not end with a full stop.")
    return problems


def check_commit_message(text: str) -> list[str]:
    lines = [ln for ln in text.splitlines() if not ln.startswith("#")]
    if not lines or not lines[0].strip():
        return ["empty commit message."]
    header = lines[0].strip()
    if header.startswith(("Merge ", "fixup! ", "squash! ", "amend! ", 'Revert "')):
        return []
    problems = check_header(header)
    if len(lines) > 1 and lines[1].strip():
        problems.append("the second line must be blank (header, blank line, body).")
    if not any(ln.startswith("Signed-off-by: ") for ln in lines):
        problems.append("missing 'Signed-off-by:' trailer: commit with `git commit -s` (DCO).")
    return problems


def check_pull_request(
    title: str, branch: str, body: str, labels: list[str], author: str
) -> list[str]:
    bot = author in BOTS
    problems = [f"title: {p}" for p in check_header(title, bot=bot)]
    if bot:
        return problems
    linked = {int(n) for n in ISSUE_LINK.findall(body or "")}
    b = BRANCH.match(branch)
    if not b and not BRANCH_EXEMPT.match(branch):
        problems.append(
            f"branch: '{branch}' is not '<type>/<issue>-<slug>' (for example feat/42-mid-year)."
        )
    if "no-issue" not in labels:
        if not linked:
            problems.append(
                "body: link the issue with 'Closes #<n>' (or 'Refs #<n>'), "
                "or label the pull request 'no-issue' for trivial changes."
            )
        elif b and int(b["issue"]) not in linked:
            problems.append(f"branch refers to #{b['issue']} but the body links {sorted(linked)}.")
    return problems


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    c = sub.add_parser("commit-msg")
    c.add_argument("file")
    p = sub.add_parser("pr")
    p.add_argument("--title")
    p.add_argument("--branch")
    p.add_argument("--body", default="")
    p.add_argument("--label", action="append", default=[])
    p.add_argument("--author", default="")
    args = parser.parse_args()

    if args.cmd == "commit-msg":
        problems = check_commit_message(Path(args.file).read_text(encoding="utf-8"))
        where = "Commit message"
    else:
        title, branch, body, labels, author = (
            args.title, args.branch, args.body, args.label, args.author
        )  # fmt: skip
        event = os.environ.get("GITHUB_EVENT_PATH")
        if title is None and event:
            pr = json.loads(Path(event).read_text())["pull_request"]
            title, branch, body = pr["title"], pr["head"]["ref"], pr.get("body") or ""
            labels = [lbl["name"] for lbl in pr["labels"]]
            author = pr["user"]["login"]
        if title is None or branch is None:
            parser.error("pr needs --title and --branch outside GitHub Actions")
        problems = check_pull_request(title, branch, body, labels, author)
        where = "Pull request"
    if problems:
        print(f"{where} does not follow docs/en/project/conventions.md:")
        for problem in problems:
            print(f"  - {problem}")
        return 1
    print(f"{where} follows the conventions.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
