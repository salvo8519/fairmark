"""Generate the agent folders of every supported agent system from the canonical `.agents/`.

`.agents/` is the single source of truth (ADR 0015):
  .agents/skills/<name>/SKILL.md   Agent Skills (open standard, agentskills.io)
  .agents/agents/<name>.md         role definitions: front matter `name`, `description`,
                                   `tools` (Claude Code names), then the instructions

Skills need a mirror only where an agent system does not read `.agents/skills/`: Codex,
Cursor, Gemini CLI and GitHub Copilot read it natively; Claude Code reads `.claude/skills/`.
Roles have one format per agent system, so each gets a generated mirror:

| Agent system | Roles | Skills |
| --- | --- | --- |
| Claude Code | `.claude/agents/<name>.md` (copy) | `.claude/skills/` (copy) |
| Codex | `.codex/agents/<name>.toml` | native |
| Cursor | `.cursor/agents/<name>.md` | native |
| GitHub Copilot | `.github/agents/<name>.agent.md` | native |

A role without `Write` or `Edit` among its tools is read-only: Codex runs it in the read-only
sandbox, Cursor with `readonly: true`, Copilot without the `edit` tool. An unknown tool name,
a missing field or a name that differs from the file name fails the generation: nothing is
guessed. Every generated folder belongs to this script: a file there without a source fails
`--check` and is removed when regenerating. Never edit a mirror by hand.

    python scripts/sync_agents.py          # regenerate
    python scripts/sync_agents.py --check  # fail if out of sync (CI)
"""

from __future__ import annotations

import argparse
import json
import shutil
import sys
from collections.abc import Callable
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / ".agents"
DST = ROOT / ".claude"
NOTICE = (
    "<!-- Generated from {src} by scripts/sync_agents.py. Edit the source, not this file. -->\n"
)
TOML_NOTICE = "# Generated from {src} by scripts/sync_agents.py. Edit the source, not this file.\n"

# Claude Code tool names, as written in the roles, and what each allows.
WRITE_TOOLS = {"Write", "Edit"}
COPILOT_TOOLS = {
    "Read": "read",
    "Grep": "search",
    "Glob": "search",
    "Write": "edit",
    "Edit": "edit",
    "Bash": "execute",
}


class RoleError(ValueError):
    """A role definition that cannot be translated without guessing."""


def split_front_matter(text: str) -> tuple[str, str]:
    if not text.startswith("---\n"):
        return "", text
    end = text.index("\n---\n", 4)
    return text[4:end], text[end + len("\n---\n") :]


def render(src: Path) -> str:
    """A Markdown file with the generated notice after its front matter."""
    text = src.read_text(encoding="utf-8")
    notice = NOTICE.format(src=src.relative_to(ROOT).as_posix())
    if text.startswith("---\n"):
        end = text.index("\n---\n", 4) + len("\n---\n")
        return text[:end] + "\n" + notice + text[end:]
    return notice + text


def read_role(src: Path) -> dict[str, object]:
    head, body = split_front_matter(src.read_text(encoding="utf-8"))
    where = src.relative_to(ROOT).as_posix()
    meta = yaml.safe_load(head) if head else None
    if not isinstance(meta, dict):
        raise RoleError(f"{where}: front matter with name, description and tools is required")
    for field in ("name", "description", "tools"):
        if not isinstance(meta.get(field), str) or not meta[field].strip():
            raise RoleError(f"{where}: '{field}' is required")
    if meta["name"] != src.stem:
        raise RoleError(f"{where}: name '{meta['name']}' differs from the file name")
    tools = [t.strip() for t in str(meta["tools"]).split(",") if t.strip()]
    unknown = sorted(set(tools) - COPILOT_TOOLS.keys())
    if unknown:
        raise RoleError(f"{where}: unknown tools {unknown}; known: {sorted(COPILOT_TOOLS)}")
    return {
        "name": meta["name"],
        "description": " ".join(str(meta["description"]).split()),
        "tools": tools,
        "readonly": not WRITE_TOOLS & set(tools),
        "body": body.lstrip("\n"),
        "src": where,
    }


def toml_string(text: str) -> str:
    """A TOML string: multi-line literal when possible, otherwise a basic string."""
    if "'''" not in text:
        return "'''\n" + text + "'''"
    return json.dumps(text, ensure_ascii=False)


def codex_role(role: dict[str, object]) -> str:
    lines = [
        TOML_NOTICE.format(src=role["src"]),
        f"name = {json.dumps(role['name'])}",
        f"description = {json.dumps(role['description'], ensure_ascii=False)}",
    ]
    if role["readonly"]:
        lines.append('sandbox_mode = "read-only"')
    lines.append(f"developer_instructions = {toml_string(str(role['body']))}")
    return "\n".join(lines) + "\n"


def markdown_role(role: dict[str, object], extra: dict[str, object]) -> str:
    head = {"name": role["name"], "description": role["description"], **extra}
    front = "\n".join(
        f"{key}: {json.dumps(value, ensure_ascii=False)}" for key, value in head.items()
    )
    notice = NOTICE.format(src=role["src"])
    return f"---\n{front}\n---\n\n{notice}\n{role['body']}"


def copilot_role(role: dict[str, object]) -> str:
    tools = list(dict.fromkeys(COPILOT_TOOLS[t] for t in role["tools"]))  # type: ignore[attr-defined]
    return markdown_role(role, {"tools": tools})


def cursor_role(role: dict[str, object]) -> str:
    return markdown_role(role, {"readonly": role["readonly"]})


# One entry per generated folder: the folder, the file name of a role, and its content.
ROLE_MIRRORS: list[tuple[str, str, Callable[[dict[str, object]], str]]] = [
    (".codex/agents", "{name}.toml", codex_role),
    (".cursor/agents", "{name}.md", cursor_role),
    (".github/agents", "{name}.agent.md", copilot_role),
]


def managed_dirs() -> list[Path]:
    return [DST / "skills", DST / "agents", *(ROOT / d for d, _, _ in ROLE_MIRRORS)]


def expected() -> dict[Path, str]:
    files: dict[Path, str] = {}
    for kind in ("skills", "agents"):
        for src in sorted((SRC / kind).rglob("*")):
            if src.is_file():
                rel = src.relative_to(SRC)
                files[DST / rel] = (
                    render(src) if src.suffix == ".md" else src.read_text(encoding="utf-8")
                )
    for src in sorted((SRC / "agents").glob("*.md")):
        role = read_role(src)
        for folder, name, build in ROLE_MIRRORS:
            files[ROOT / folder / name.format(name=role["name"])] = build(role)
    return files


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    try:
        files = expected()
    except RoleError as error:
        sys.exit(f"Cannot generate the agent mirrors: {error}")
    current = {p for d in managed_dirs() for p in d.rglob("*") if p.is_file()}

    if args.check:
        stale = [
            p for p, c in files.items() if not p.exists() or p.read_text(encoding="utf-8") != c
        ]
        extra = sorted(current - files.keys())
        if stale or extra:
            names = [str(p.relative_to(ROOT)) for p in [*stale, *extra]]
            sys.exit(f"Agent folders out of sync: {names}. Run: python scripts/sync_agents.py")
        print("Agent folders in sync.")
        return

    for folder in managed_dirs():
        shutil.rmtree(folder, ignore_errors=True)
    for path, content in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    print(f"Synced {len(files)} files from .agents/ to {len(managed_dirs())} folders.")


if __name__ == "__main__":
    main()
