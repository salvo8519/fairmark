"""Generate tool-specific agent folders from the canonical `.agents/` directory.

`.agents/` is the single source of truth:
  .agents/skills/<name>/SKILL.md   Agent Skills (open standard) — read natively by Codex,
                                   Gemini CLI, Copilot and others
  .agents/agents/<name>.md         role definitions (subagents)

Claude Code reads `.claude/skills/` and `.claude/agents/`; this script mirrors the canonical
files there with a "generated" notice. Never edit `.claude/skills` or `.claude/agents` by hand.

    python scripts/sync_agents.py          # regenerate
    python scripts/sync_agents.py --check  # fail if out of sync (CI)
"""

from __future__ import annotations

import argparse
import shutil
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / ".agents"
DST = ROOT / ".claude"
NOTICE = (
    "<!-- Generated from {src} by scripts/sync_agents.py. Edit the source, not this file. -->\n"
)


def render(src: Path) -> str:
    text = src.read_text(encoding="utf-8")
    notice = NOTICE.format(src=src.relative_to(ROOT).as_posix())
    if text.startswith("---\n"):
        end = text.index("\n---\n", 4) + len("\n---\n")
        return text[:end] + "\n" + notice + text[end:]
    return notice + text


def expected() -> dict[Path, str]:
    files: dict[Path, str] = {}
    for kind in ("skills", "agents"):
        for src in sorted((SRC / kind).rglob("*")):
            if src.is_file():
                rel = src.relative_to(SRC)
                files[DST / rel] = (
                    render(src) if src.suffix == ".md" else src.read_text(encoding="utf-8")
                )
    return files


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    files = expected()
    current = {p for kind in ("skills", "agents") for p in (DST / kind).rglob("*") if p.is_file()}

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

    for kind in ("skills", "agents"):
        shutil.rmtree(DST / kind, ignore_errors=True)
    for path, content in files.items():
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(content, encoding="utf-8")
    print(f"Synced {len(files)} files from .agents/ to .claude/.")


if __name__ == "__main__":
    main()
