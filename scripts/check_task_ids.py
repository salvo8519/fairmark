"""Every task id in a tasks.md file is defined once.

A task is defined by a checklist item (`- [ ] T12 ...`); mentions of a task id elsewhere in
the text are references and are not counted.

    python scripts/check_task_ids.py
"""

from __future__ import annotations

import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

TASK_DEFINITION = re.compile(r"^\s*[-*] \[[ xX]\] (T\d+)\b")


def tracked_tasks_files() -> list[Path]:
    out = subprocess.run(
        ["git", "ls-files", "specs/*/tasks.md"],
        cwd=ROOT,
        capture_output=True,
        text=True,
        check=True,
        encoding="utf-8",
    ).stdout
    return [ROOT / line for line in out.splitlines() if line]


def main() -> None:
    problems = []
    files = tracked_tasks_files()
    for path in files:
        definitions: dict[str, list[int]] = defaultdict(list)
        for number, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            match = TASK_DEFINITION.match(line)
            if match:
                definitions[match.group(1)].append(number)
        for task_id, lines in definitions.items():
            if len(lines) > 1:
                where = ", ".join(str(n) for n in lines)
                problems.append(f"{path.relative_to(ROOT)}: {task_id} is defined on lines {where}")
    if problems:
        sys.exit("Task ids defined more than once:\n  " + "\n  ".join(problems))
    print(f"{len(files)} tasks.md files: every task id is defined once.")


if __name__ == "__main__":
    main()
