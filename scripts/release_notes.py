"""Extract the text of one release for the GitHub Release page.

    python scripts/release_notes.py 0.2.0 > body.md

Combines the functional notes for that version (RELEASE_NOTES.md, built by towncrier) with a
link to the technical changelog (CHANGELOG.md, built by git-cliff).
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def section(text: str, version: str) -> str:
    pattern = re.compile(rf"^## Fairmark {re.escape(version)}\b.*?(?=^## |\Z)", re.M | re.S)
    m = pattern.search(text)
    return m.group(0).split("\n", 1)[1].strip() if m else ""


def main() -> None:
    if len(sys.argv) != 2:
        sys.exit("usage: release_notes.py <version>")
    version = sys.argv[1].removeprefix("v")
    notes = section((ROOT / "RELEASE_NOTES.md").read_text(encoding="utf-8"), version)
    if not notes:
        sys.exit(f"No release notes for {version} in RELEASE_NOTES.md: run towncrier build.")
    print(notes)
    print("\n---\n")
    print(
        "Technical changes: "
        f"[CHANGELOG.md](https://github.com/salvo8519/fairmark/blob/v{version}/CHANGELOG.md). "
        "Documentation: https://salvo8519.github.io/fairmark/"
    )


if __name__ == "__main__":
    main()
