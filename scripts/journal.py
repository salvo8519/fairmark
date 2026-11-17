"""Index and search the session journal (`.agents/journal/`), the handover tier of ADR 0015.

uv run python scripts/journal.py index [--json]          # every entry, one line each
uv run python scripts/journal.py search 327              # entries about issue #327
uv run python scripts/journal.py search area:agents      # entries about an area
uv run python scripts/journal.py search 2026-11-16       # entries of a day (or 2026-11)
uv run python scripts/journal.py search "merge order"    # lines that contain the text

The index is built from the entries each time it is read and is never committed: every session
writes a new entry, and a committed index rewritten by every pull request would make parallel
lanes collide on one file (the problem #335 removes for the schema index). Reading 100 entries
takes a few milliseconds; the text is the only store (no database, ADR 0015).

For each entry the index records the date and the branch (from the file and its title), the
issue of the entry (the `- Issue:` line), every other issue it mentions, the areas of the
entry's issue, or of the issues it mentions when it has none, and a one-line summary (the
first item under `## Done`). Areas are the `area:` labels of the epics in
`roadmap/roadmap.yaml`: the labels of a single issue live only on GitHub and are not read, so
that the index needs no network.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import asdict, dataclass, field
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
JOURNAL = ROOT / ".agents" / "journal"
ROADMAP = ROOT / "roadmap" / "roadmap.yaml"
ENTRY = re.compile(r"^(?P<date>\d{4}-\d{2}-\d{2})-.+\.md$")
ISSUE_LINE = re.compile(r"^- Issue:\s*(?:#(?P<issue>\d+))?", re.M)
MENTION = re.compile(r"(?<![\w/&])#(\d+)\b")
DATE_QUERY = re.compile(r"^\d{4}-\d{2}(?:-\d{2})?$")
SUMMARY_LENGTH = 110


@dataclass
class Entry:
    file: str
    date: str
    branch: str
    issue: int | None
    mentions: list[int] = field(default_factory=list)
    areas: list[str] = field(default_factory=list)
    summary: str = ""


def issue_areas(roadmap: Path) -> dict[int, list[str]]:
    """Issue number -> areas, from the `area:` labels of the epic that holds the task."""
    plan = yaml.safe_load(roadmap.read_text(encoding="utf-8")) if roadmap.exists() else {}
    out: dict[int, list[str]] = {}
    for milestone in (plan or {}).get("milestones", []):
        for epic in milestone.get("epics", []):
            areas = sorted(
                label.removeprefix("area:")
                for label in epic.get("labels", [])
                if label.startswith("area:")
            )
            for number in [epic.get("issue"), *(t.get("issue") for t in epic.get("tasks", []))]:
                if isinstance(number, int):
                    out[number] = areas
    return out


METADATA = re.compile(r"^[-*] (?:Issue|Pull request|Spec):")


def summary(text: str) -> str:
    """The first item under `## Done`, else the first item of the entry that is not one of
    the metadata lines (issue, pull request, spec), on one line."""
    done = re.search(r"^## Done\s*\n(?P<body>.*?)(?=^## |\Z)", text, re.M | re.S)
    blocks = [done.group("body")] if done else []
    blocks.append(text)
    for block in blocks:
        items = re.finditer(r"^[-*] (?P<item>.+(?:\n  +\S.*)*)", block, re.M)
        item = next((i for i in items if not METADATA.match(i.group(0))), None)
        if item:
            line = " ".join(part.strip() for part in item.group("item").splitlines())
            return line if len(line) <= SUMMARY_LENGTH else line[: SUMMARY_LENGTH - 3] + "..."
    return ""


def parse(path: Path, areas: dict[int, list[str]]) -> Entry | None:
    match = ENTRY.match(path.name)
    if match is None:
        return None
    text = path.read_text(encoding="utf-8")
    first = text.splitlines()[0] if text else ""
    branch = first.split("—", 1)[1].strip() if "—" in first else path.stem[11:]
    line = ISSUE_LINE.search(text)
    issue = int(line.group("issue")) if line and line.group("issue") else None
    mentions = sorted({int(n) for n in MENTION.findall(text)} - ({issue} if issue else set()))
    # The areas of the entry's own issue; mentions only count when the entry has none, since
    # an entry that cites another issue is not about that issue's area.
    source = [issue] if issue else mentions
    entry_areas = sorted({a for n in source for a in areas.get(n, [])})
    return Entry(path.name, match["date"], branch, issue, mentions, entry_areas, summary(text))


def build(journal: Path | None = None, roadmap: Path | None = None) -> list[Entry]:
    """Every journal entry, oldest first (by date, then file name)."""
    journal, roadmap = journal or JOURNAL, roadmap or ROADMAP
    areas = issue_areas(roadmap)
    entries = (parse(p, areas) for p in sorted(journal.glob("*.md")))
    return sorted((e for e in entries if e), key=lambda e: (e.date, e.file))


def search(
    entries: list[Entry], query: str, journal: Path | None = None
) -> list[tuple[Entry, str]]:
    """Entries that match, with the reason; the entries of an issue come before its mentions.

    `327` or `#327`: the issue of the entry, then entries that mention it. `area:<name>` or a
    known area: entries about that area. `2026-11-16` or `2026-11`: entries of that day or month.
    Anything else: entries whose text contains it (case-insensitive), with the first matching line.
    """
    journal = journal or JOURNAL
    q = query.strip()
    number = q.removeprefix("#")
    if number.isdigit():
        n = int(number)
        own = [(e, "issue") for e in entries if e.issue == n]
        mentioned = [(e, "mentions") for e in entries if n in e.mentions]
        return own + mentioned
    known = {a for e in entries for a in e.areas}
    area = q.removeprefix("area:")
    if q.startswith("area:") or area in known:
        return [(e, f"area {area}") for e in entries if area in e.areas]
    if DATE_QUERY.match(q):
        return [(e, "date") for e in entries if e.date.startswith(q)]
    found = []
    for e in entries:
        for line in (journal / e.file).read_text(encoding="utf-8").splitlines():
            if q.lower() in line.lower():
                found.append((e, line.strip()))
                break
    return found


def render(entry: Entry, reason: str = "") -> str:
    issue = f"#{entry.issue}" if entry.issue else "-"
    areas = ",".join(entry.areas) or "-"
    line = f"{entry.date}  {issue:<6} {areas:<16} {entry.file}"
    structured = reason in {"", "issue", "mentions", "date"} or reason.startswith("area ")
    detail = entry.summary if structured else reason  # a text search shows the matching line
    tag = " (mentions)" if reason == "mentions" else ""
    return f"{line}{tag}\n            {detail}"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description="Index and search the session journal.")
    sub = parser.add_subparsers(dest="command", required=True)
    ix = sub.add_parser("index", help="every entry, oldest first")
    ix.add_argument("--json", action="store_true", help="machine-readable, for agents")
    se = sub.add_parser("search", help="entries about an issue, an area, a date or a text")
    se.add_argument("query")
    se.add_argument("--json", action="store_true", help="machine-readable, for agents")
    args = parser.parse_args(argv)
    entries = build()
    if args.command == "index":
        rows: list[tuple[Entry, str]] = [(e, "") for e in entries]
    else:
        rows = search(entries, args.query)
    if args.json:
        payload: list[dict[str, Any]] = [{**asdict(e), "match": r} for e, r in rows]
        print(json.dumps(payload, indent=2, ensure_ascii=False))
        return 0
    if not rows:
        if args.command == "index":
            print("The journal is empty.")
            return 0
        print(f"No journal entry matches {args.query!r}.")
        if args.query.startswith("area:"):
            known = sorted({a for e in entries for a in e.areas})
            print(f"Areas in the journal: {', '.join(known) or 'none'}.")
        return 1
    print("\n".join(render(e, r) for e, r in rows))
    return 0


if __name__ == "__main__":
    sys.exit(main())
