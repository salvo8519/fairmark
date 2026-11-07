"""Every YAML file in the repository parses: a broken file must fail CI, not wait for the tool
that happens to read it (a translation glossary went unread for days).

    python scripts/check_yaml.py
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]


def tracked_yaml() -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "*.yml", "*.yaml"], cwd=ROOT, capture_output=True, text=True, check=True
    ).stdout
    return [ROOT / line for line in out.splitlines() if line]


def main() -> None:
    problems = []
    files = tracked_yaml()
    for path in files:
        try:
            list(yaml.safe_load_all(path.read_text(encoding="utf-8")))
        except yaml.YAMLError as err:
            problems.append(f"{path.relative_to(ROOT)}: {err}".replace("\n", " "))
    if problems:
        sys.exit("Invalid YAML:\n  " + "\n  ".join(problems))
    print(f"{len(files)} YAML files parse.")


if __name__ == "__main__":
    main()
