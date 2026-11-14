"""Task IDs must be unique within each tasks.md file.

Task IDs follow the pattern T1, T2, T3, etc. and must appear only once per tasks.md.
The check fails if any duplicate is found.

    python scripts/check_task_ids.py
"""

from __future__ import annotations

import re
import subprocess
import sys
from collections import defaultdict
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

TASK_ID_PATTERN = re.compile(r"\bT\d+\b")


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
        task_ids = defaultdict(list)
        
        for line_num, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            for match in TASK_ID_PATTERN.finditer(line):
                task_id = match.group()
                task_ids[task_id].append(line_num)
        
        for task_id, lines in sorted(task_ids.items()):
            if len(lines) > 1:
                rel_path = path.relative_to(ROOT)
                lines_str = ", ".join(str(l) for l in lines)
                problems.append(f"{rel_path}: {task_id} appears on lines {lines_str}")
    
    if problems:
        print("Duplicate task IDs found:")
        print("\n".join(f"  {p}" for p in problems))
        sys.exit(1)
    
    print(f"Task ID check passed: {len(files)} files, all IDs unique.")


if __name__ == "__main__":
    main()
