"""House style gate: no emoji, pictographic icons or icon shortcodes in the repository.

Fairmark's documents follow a professional, corporate identity. Typographic symbols used
in finance and maths (multiplication, minus, sigma, arrows, euro) are fine; emoji, dingbats,
check marks, geometric
bullets and theme icon shortcodes (``:material-…:``) are not.

    python scripts/check_style.py           # exit 1 and list offending lines
"""

from __future__ import annotations

import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SUFFIXES = {".md", ".py", ".rs", ".toml", ".yml", ".yaml", ".json", ".html", ".css", ".js"}
EXCLUDE = ("docs/_shared/vendor/", "uv.lock", "Cargo.lock", "schemas/")

FORBIDDEN = re.compile(
    "["
    "\U0001f000-\U0001faff"  # emoji, pictographs, symbols & pictographs extended
    "☀-➿"  # miscellaneous symbols and dingbats (check marks, stars, ...)
    "■-◿"  # geometric shapes used as icons
    "⬀-⯿"  # miscellaneous symbols and arrows (stars, heavy shapes)
    "️"  # emoji presentation selector
    "]"
    r"|:(?:material|octicons|fontawesome|simple|lucide)-[a-z0-9-]+:"
)


def tracked_files() -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "--cached", "--others", "--exclude-standard"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
    ).stdout
    return [
        ROOT / f
        for f in out.splitlines()
        if Path(f).suffix in SUFFIXES and not f.startswith(EXCLUDE) and (ROOT / f).is_file()
    ]


def main() -> None:
    problems = []
    for path in tracked_files():
        if path.resolve() == Path(__file__).resolve():
            continue
        for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            match = FORBIDDEN.search(line)
            if match:
                problems.append(f"{path.relative_to(ROOT)}:{n}: {match.group()!r}")
    if problems:
        print("Icons or emoji found (house style forbids them):")
        print("\n".join(f"  {p}" for p in problems))
        sys.exit(1)
    print("Style check passed: no icons or emoji.")


if __name__ == "__main__":
    main()
