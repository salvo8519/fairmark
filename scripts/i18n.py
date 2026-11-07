"""Track translations of the documentation and README against their English source.

Every translated file starts with front matter naming its source and the hash of the source
it was translated from:

    ---
    source: getting-started/quickstart.md
    source_hash: 3f9a1c0b7d2e
    ---

    uv run python scripts/i18n.py status            # table: translated, stale, missing
    uv run python scripts/i18n.py check             # fail on stale/missing required pages
    uv run python scripts/i18n.py stamp docs/it/getting-started/quickstart.md
                                                    # after (re)translating a page
    uv run python scripts/i18n.py todo              # stale and missing pages, one per line

README translations (README.<lang>.md) carry the same data in an HTML comment on the first
line, because GitHub would render front matter:

    <!-- i18n source: README.md source_hash: 3f9a1c0b7d2e -->
"""

from __future__ import annotations

import argparse
import fnmatch
import hashlib
import re
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
EN = ROOT / "docs" / "en"
FRONT = re.compile(r"\A---\n(.*?)\n---\n", re.DOTALL)
COMMENT = re.compile(r"\A<!-- i18n source: (\S+) source_hash: (\S+) -->\n")


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()[:12]


def meta(path: Path) -> dict[str, str]:
    text = path.read_text(encoding="utf-8")
    c = COMMENT.match(text)
    if c:
        return {"source": c.group(1), "source_hash": c.group(2)}
    m = FRONT.match(text)
    data = yaml.safe_load(m.group(1)) if m else {}
    return data if isinstance(data, dict) else {}


def languages() -> dict[str, dict[str, list[str]]]:
    return {
        p.stem: yaml.safe_load(p.read_text())
        for p in sorted((ROOT / "i18n").glob("*.yaml"))
        if p.stem != "glossary"
    }


def pairs(lang: str, cfg: dict[str, list[str]]) -> list[tuple[str, Path, Path, bool]]:
    """(name, source, translation, required) for every page that should be translated."""
    out = []
    for src in sorted(EN.rglob("*.md")):
        rel = src.relative_to(EN).as_posix()
        if any(
            fnmatch.fnmatch(rel, p) for p in cfg.get("english_only", []) + cfg.get("generated", [])
        ):
            continue
        required = any(fnmatch.fnmatch(rel, p) for p in cfg.get("required", []))
        out.append((rel, src, ROOT / "docs" / lang / rel, required))
    out.append(("README.md", ROOT / "README.md", ROOT / f"README.{lang}.md", True))
    return out


def evaluate() -> list[tuple[str, str, str, bool]]:
    rows = []
    for lang, cfg in languages().items():
        for rel, src, dst, required in pairs(lang, cfg):
            if not dst.exists():
                state = "missing"
            elif meta(dst).get("source_hash") != digest(src):
                state = "stale"
            else:
                state = "translated"
            rows.append((lang, rel, state, required))
    return rows


def stamp(path: Path) -> None:
    path = path.resolve()
    text = path.read_text(encoding="utf-8")
    if path.name.startswith("README."):
        body = COMMENT.sub("", text, count=1)
        line = f"<!-- i18n source: README.md source_hash: {digest(ROOT / 'README.md')} -->\n"
        path.write_text(line + body, encoding="utf-8")
        print(f"Stamped {path.relative_to(ROOT)} against README.md.")
        return
    else:
        lang = path.relative_to(ROOT / "docs").parts[0]
        rel = path.relative_to(ROOT / "docs" / lang).as_posix()
        src = EN / rel
    data = meta(path)
    body = FRONT.sub("", text, count=1)
    data.update({"source": rel, "source_hash": digest(src)})
    path.write_text(
        "---\n"
        + yaml.safe_dump(data, sort_keys=False, allow_unicode=True)
        + "---\n"
        + body.lstrip("\n"),
        encoding="utf-8",
    )
    print(f"Stamped {path.relative_to(ROOT)} against {rel} ({data['source_hash']}).")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["status", "check", "stamp", "todo"])
    parser.add_argument("files", nargs="*", type=Path)
    args = parser.parse_args()

    if args.command == "stamp":
        for f in args.files:
            stamp(f)
        return
    rows = evaluate()
    if args.command == "todo":
        for lang, rel, state, _ in rows:
            if state != "translated":
                print(f"{lang}\t{state}\t{rel}")
        return
    if args.command == "status":
        for lang, rel, state, required in rows:
            print(f"{lang}  {state:<10} {'required' if required else 'optional':<8}  {rel}")
        done = sum(r[2] == "translated" for r in rows)
        print(f"\n{done}/{len(rows)} pages translated and current.")
        return
    problems = [r for r in rows if r[3] and r[2] != "translated"]
    if problems:
        print("Translations out of date or missing (English is the source):")
        for lang, rel, state, _ in problems:
            print(f"  {lang}: {state:<8} {rel}")
        print("Run the translate skill, then: uv run python scripts/i18n.py stamp <file>")
        sys.exit(1)
    print("All required translations are current.")


if __name__ == "__main__":
    main()
