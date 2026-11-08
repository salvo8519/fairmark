"""PreToolUse hook: block edits to golden cases that already exist on the base branch.

A golden case is protected when its path, relative to the root of the git worktree that
contains it, exists on the base branch (``origin/main``, or ``FAIRMARK_GOLDEN_BASE``).
Cases added on the current branch stay editable until they merge. The check works in
every git worktree, not only in the main checkout. Outside a git repository, or when the
base branch is unknown, every existing file under ``tests/golden/`` is protected.

CI (``scripts/check_golden_changes.py``) is the second line of defence.
Exit status 2 blocks the edit; the message on stderr is shown to the agent.
"""

from __future__ import annotations

import json
import os
import subprocess
import sys

MARKER = "tests/golden/"


def _git(*args: str, cwd: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args], cwd=cwd, capture_output=True, text=True, encoding="utf-8", check=False
    )


def _existing_dir(path: str) -> str:
    directory = os.path.dirname(path)
    while directory and not os.path.isdir(directory):
        parent = os.path.dirname(directory)
        if parent == directory:
            break
        directory = parent
    return directory or "."


def reason_to_block(file_path: str, base: str) -> str | None:
    """Return why an edit to ``file_path`` must be blocked, or None to allow it."""
    normalised = file_path.replace("\\", "/")
    if not (normalised.startswith(MARKER) or f"/{MARKER}" in normalised):
        return None
    path = os.path.abspath(file_path)
    top = _git("rev-parse", "--show-toplevel", cwd=_existing_dir(path))
    if top.returncode != 0:
        return "is an existing golden case" if os.path.exists(path) else None
    root = top.stdout.strip()
    relative = os.path.relpath(os.path.realpath(path), os.path.realpath(root))
    relative = relative.replace(os.sep, "/")
    if _git("rev-parse", "--verify", "--quiet", f"{base}^{{commit}}", cwd=root).returncode != 0:
        if os.path.exists(path):
            return f"is an existing golden case ({base} is not available to compare against)"
        return None
    if _git("cat-file", "-e", f"{base}:{relative}", cwd=root).returncode == 0:
        return f"exists on {base}"
    return None


def main() -> int:
    data = json.load(sys.stdin)
    file_path = (data.get("tool_input") or {}).get("file_path") or ""
    reason = reason_to_block(file_path, os.environ.get("FAIRMARK_GOLDEN_BASE", "origin/main"))
    if reason is None:
        return 0
    sys.stderr.write(
        f"Blocked: {file_path} {reason}. Changing an existing golden case is a "
        "numerical-change and needs the maintainer's approval. Ask the user, or add a "
        "new case instead.\n"
    )
    return 2


if __name__ == "__main__":
    sys.exit(main())
