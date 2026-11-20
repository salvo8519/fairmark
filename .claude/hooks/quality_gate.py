"""Stop and SubagentStop hook: the fast checks on the files changed, before the turn ends.

Changed files are those of the working tree and the index that differ from HEAD, plus untracked
files that git does not ignore. Only the relevant checks run:

- `.rs`: `rustfmt --check` on the files, then `cargo clippy -p <crate> --all-targets -- -D
  warnings` for every crate under `crates/` that a file belongs to;
- `.py`: `ruff check` and `ruff format --check` on the files, `mypy` on those inside the scope
  of `[tool.mypy]` in `pyproject.toml` (`files`, or `packages` under `mypy_path`).

The edition passed to rustfmt is the one of the workspace manifest, as `cargo fmt` uses it.

Findings block the end of the turn (exit status 2, findings on stderr), so the agent fixes
them first. A session is never blocked twice by the same findings, nor more than `MAX_BLOCKS`
times: the fingerprint of the last blocking findings and the number of blocks are kept per
session in the git directory. Findings that are not blocking, a check stopped by the time
budget (`FAIRMARK_QUALITY_BUDGET` seconds, default 60) and a missing tool are reported in a
`systemMessage` on stdout and never block.
"""

from __future__ import annotations

import contextlib
import hashlib
import io
import json
import os
import re
import shutil
import signal
import subprocess
import sys
import time
import tomllib
from pathlib import Path
from typing import Any

MAX_LINES = 40
MAX_BLOCKS = 3
DEFAULT_BUDGET = 60.0


def _git(root: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", *args], cwd=root, capture_output=True, text=True, encoding="utf-8", check=False
    )


def changed_files(root: Path) -> list[str] | None:
    """Existing files changed against HEAD, staged or not, and untracked ones; None outside git."""
    diff = _git(root, "diff", "-z", "--name-only", "HEAD")
    untracked = _git(root, "ls-files", "-z", "--others", "--exclude-standard")
    if diff.returncode != 0 or untracked.returncode != 0:
        return None
    names = {*diff.stdout.split("\0"), *untracked.stdout.split("\0")}
    return sorted(n for n in names if n and (root / n).is_file())


def crate_of(root: Path, path: str) -> str | None:
    """Package name of the crate under `crates/` that contains the path."""
    parts = Path(path).parts
    if len(parts) < 3 or parts[0] != "crates":
        return None
    name = _toml(root / "crates" / parts[1] / "Cargo.toml").get("package", {}).get("name")
    return name if isinstance(name, str) else None


def _toml(path: Path) -> dict[str, Any]:
    if not path.is_file():
        return {}
    try:
        with path.open("rb") as f:
            return tomllib.load(f)
    except (OSError, tomllib.TOMLDecodeError):
        return {}


def _strings(value: object) -> list[str]:
    if isinstance(value, str):
        return [value]
    return [str(v) for v in value] if isinstance(value, list) else []


def mypy_paths(root: Path) -> list[str]:
    """The paths that mypy checks: `[tool.mypy] files`, or each of `packages` under `mypy_path`."""
    config = _toml(root / "pyproject.toml").get("tool", {}).get("mypy", {})
    paths = [p.rstrip("/") for p in _strings(config.get("files"))]
    bases = _strings(config.get("mypy_path")) or ["."]
    for package in _strings(config.get("packages")):
        relative = package.replace(".", "/")
        paths += [str(Path(base) / relative).replace("\\", "/") for base in bases]
    return [p.removeprefix("./") for p in paths]


def rust_edition(root: Path) -> str:
    """The edition of the workspace manifest (`[workspace.package]` or `[package]`)."""
    manifest = _toml(root / "Cargo.toml")
    for table in (manifest.get("workspace", {}).get("package", {}), manifest.get("package", {})):
        edition = table.get("edition") if isinstance(table, dict) else None
        if isinstance(edition, str):
            return edition
    return "2021"


def python_tool(root: Path, name: str) -> list[str] | None:
    """The command of a Python tool: the project environment, then PATH, then uv."""
    for folder in ("bin", "Scripts"):  # POSIX and Windows layouts of a virtual environment
        found = shutil.which(name, path=str(root / ".venv" / folder))
        if found:
            return [found]
    found = shutil.which(name)
    if found:
        return [found]
    if shutil.which("uv"):
        return ["uv", "run", "--quiet", name]
    return None


def works(command: list[str], root: Path) -> bool:
    """True when the probe command runs and succeeds (a rustup component may be missing)."""
    try:
        done = subprocess.run(command, cwd=root, capture_output=True, timeout=20, check=False)
    except (OSError, subprocess.TimeoutExpired):
        return False
    return done.returncode == 0


def plan(root: Path, files: list[str]) -> list[tuple[str, list[str] | None]]:
    """The checks to run, cheapest first: a label and a command (None when the tool is missing)."""
    checks: list[tuple[str, list[str] | None]] = []
    rust = [f for f in files if f.endswith(".rs")]
    python = [f for f in files if f.endswith(".py")]
    if rust:
        rustfmt = shutil.which("rustfmt")
        ok = bool(rustfmt) and works([str(rustfmt), "--version"], root)
        edition = rust_edition(root)
        command = [str(rustfmt), "--check", "--color", "never", "--edition", edition, "--", *rust]
        checks.append(("rustfmt --check", command if ok else None))
    if python:
        ruff = python_tool(root, "ruff")
        lint = ["check", "--output-format", "concise", "--", *python]
        checks.append(("ruff check", [*ruff, *lint] if ruff else None))
        fmt = ["format", "--check", "--", *python]
        checks.append(("ruff format --check", [*ruff, *fmt] if ruff else None))
        paths = mypy_paths(root)
        typed = [f for f in python if any(f == p or f.startswith(p + "/") for p in paths)]
        if typed:
            mypy = python_tool(root, "mypy")
            checks.append(("mypy", [*mypy, "--", *typed] if mypy else None))
    crates = sorted({c for c in (crate_of(root, f) for f in rust) if c})
    if crates:
        cargo = shutil.which("cargo")
        ok = bool(cargo) and works([str(cargo), "clippy", "--version"], root)
        selected = [arg for c in crates for arg in ("-p", c)]
        label = "cargo clippy " + " ".join(selected)
        lint = ["clippy", "--quiet", *selected, "--all-targets", "--", "-D", "warnings"]
        checks.append((label, [str(cargo), *lint] if ok else None))
    return checks


def tail(text: str) -> str:
    lines = text.strip().splitlines()
    if len(lines) <= MAX_LINES:
        return "\n".join(lines)
    return "\n".join([*lines[:MAX_LINES], f"... {len(lines) - MAX_LINES} more lines"])


def run_one(command: list[str], root: Path, timeout: float) -> tuple[int, str] | None:
    """Exit status and output of the command; None when it ran out of time (group killed)."""
    with subprocess.Popen(
        command,
        cwd=root,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        text=True,
        encoding="utf-8",
        errors="replace",
        start_new_session=True,
    ) as proc:
        try:
            out, _ = proc.communicate(timeout=timeout)
        except subprocess.TimeoutExpired:
            with contextlib.suppress(ProcessLookupError):
                if os.name == "posix":
                    os.killpg(proc.pid, signal.SIGKILL)  # cargo and its rustc children too
                else:
                    proc.kill()
            proc.communicate()
            return None
        return proc.returncode, out


def run_checks(
    root: Path, checks: list[tuple[str, list[str] | None]], budget: float
) -> tuple[list[str], list[str]]:
    """Findings that block, and notes that do not (missing tools, time budget spent)."""
    findings: list[str] = []
    notes: list[str] = []
    deadline = time.monotonic() + budget
    for label, command in checks:
        if command is None:
            notes.append(f"{label}: tool not found, check skipped.")
            continue
        remaining = deadline - time.monotonic()
        if remaining <= 0:
            notes.append(f"{label}: not run, the time budget of {budget:g} s is spent.")
            continue
        result = run_one(command, root, remaining)
        if result is None:
            notes.append(f"{label}: stopped, the time budget of {budget:g} s is spent.")
        elif result[0] != 0:
            findings.append(f"$ {label}\n{tail(result[1])}")
    return findings, notes


def budget_from_env() -> float:
    """Seconds for all the checks; an invalid or negative value falls back to the default."""
    try:
        value = float(os.environ.get("FAIRMARK_QUALITY_BUDGET", DEFAULT_BUDGET))
    except ValueError:
        return DEFAULT_BUDGET
    return value if value >= 0 else DEFAULT_BUDGET


def state_file(root: Path, session: str) -> Path | None:
    """Per-session state in the git directory: never committed, one folder per worktree."""
    found = _git(root, "rev-parse", "--git-path", "fairmark-quality-gate")
    if found.returncode != 0:
        return None
    name = re.sub(r"[^A-Za-z0-9_-]", "_", session) or "default"
    return (root / found.stdout.strip() / f"{name}.json").resolve()


def load_state(path: Path | None) -> dict[str, object]:
    if path is None or not path.is_file():
        return {}
    try:
        data = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError):
        return {}
    return data if isinstance(data, dict) else {}


def save_state(path: Path | None, state: dict[str, object]) -> None:
    """Best effort: without a writable state the hook still checks, it only forgets."""
    if path is None:
        return
    with contextlib.suppress(OSError):
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(state), encoding="utf-8")


def report(notes: list[str]) -> int:
    """Non-blocking report: a systemMessage on stdout, shown to the user."""
    if notes:
        print(json.dumps({"systemMessage": "quality-gate hook: " + " ".join(notes)}))
    return 0


def main() -> int:
    # Claude Code exchanges UTF-8 with hooks. On Windows the standard streams default to the
    # locale code page, which would garble or reject a non-ASCII file name in the findings.
    for stream in (sys.stdin, sys.stdout, sys.stderr):
        if isinstance(stream, io.TextIOWrapper):
            stream.reconfigure(encoding="utf-8")
    try:
        payload = json.load(sys.stdin)
    except json.JSONDecodeError:
        payload = {}
    session = str(payload.get("session_id") or "default") if isinstance(payload, dict) else ""
    root = Path(os.environ.get("CLAUDE_PROJECT_DIR") or ".").resolve()
    if shutil.which("git") is None:
        return report(["git not found, check skipped."])
    files = changed_files(root)
    if not files:
        return 0
    checks = plan(root, files)
    if not checks:
        return 0
    findings, notes = run_checks(root, checks, budget_from_env())
    path = state_file(root, session)
    state = load_state(path)
    blocks = state.get("blocks", 0)
    blocks = blocks if isinstance(blocks, int) else 0
    if not findings:
        save_state(path, {"fingerprint": "", "blocks": blocks})
        return report(notes)
    text = "\n\n".join(findings)
    fingerprint = hashlib.sha256(text.encode("utf-8")).hexdigest()
    if fingerprint == state.get("fingerprint"):
        reason = "the same findings already blocked this turn once"
        return report([*notes, f"findings left, not blocking because {reason}:\n\n{text}"])
    if blocks >= MAX_BLOCKS:
        reason = f"this session was already blocked {blocks} times"
        return report([*notes, f"findings left, not blocking because {reason}:\n\n{text}"])
    save_state(path, {"fingerprint": fingerprint, "blocks": blocks + 1})
    print(
        "The changed files fail the fast checks. Fix every finding before you end the turn "
        "(`uv run just fmt` applies the formatting and the safe fixes):\n\n"
        + text
        + "".join(f"\n\nquality-gate hook: {note}" for note in notes),
        file=sys.stderr,
    )
    return 2


if __name__ == "__main__":
    sys.exit(main())
