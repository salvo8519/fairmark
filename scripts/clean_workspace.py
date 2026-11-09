"""Free disk space taken by merged agent worktrees and build caches, without losing work.

Dry run by default: the script prints what it would remove, why every other path is kept,
and the space it would free. Nothing is removed without ``--apply``.

    python scripts/clean_workspace.py                    # dry run: merged worktrees
    python scripts/clean_workspace.py --caches           # dry run: also build caches
    python scripts/clean_workspace.py --apply            # remove merged worktrees
    python scripts/clean_workspace.py --apply --caches   # and build caches

A git worktree is removed only when every condition holds (otherwise it is kept, with the
reason in the summary):

1. it is not the main checkout and it is not locked;
2. it has no uncommitted or untracked changes (``git status --porcelain`` is empty);
3. its branch is merged: its commit is reachable from ``origin/main``, or GitHub reports a
   merged pull request whose head is exactly that commit (squash merges). When GitHub cannot
   be reached the worktree is kept;
4. it has no unpushed commits: the commit is on ``origin/<branch>``, on ``origin/main`` or is
   the head of the merged pull request;
5. no running process has its working directory or an open file inside it (``/proc``; where
   ``/proc`` is absent the worktree is kept);
6. nothing in it changed for ``--idle-hours`` hours (default 12): agent shells do not keep a
   working directory between commands, so a process check alone cannot see an agent that is
   between two commands.

The worktree is removed with ``git worktree remove`` (never ``--force``), so git refuses it
again if anything changed in the meantime. Branches are never deleted: their history stays.

Build caches are removed only with ``--caches``, also inside kept worktrees and in the main
checkout, and only when no cargo, rustc, cargo-mutants or pytest process uses them. They are
rebuildable by definition:

- in each worktree: ``target/``, ``mutants.out*``, ``.pytest_cache/`` at the top level and
  every ``__pycache__/`` directory (outside ``.git``, ``.venv`` and ``target``);
- in the temporary directory (``--tmp-root``): ``cargo-mutants-*.tmp`` and ``pytest-of-*``
  owned by the current user.

Safety: every path removed must match one of the patterns above, sit inside an allowed root,
contain no file tracked by git, not lie under ``.git``, ``.agents/journal``, ``specs`` or
``docs``, and must not be or pass through a symbolic link. ``git worktree prune`` runs at the
end (``--dry-run`` without ``--apply``).
"""

from __future__ import annotations

import argparse
import contextlib
import fnmatch
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable, Iterable
from dataclasses import dataclass, field
from pathlib import Path

WORKTREE_CACHE_GLOBS = ("target", "mutants.out*", ".pytest_cache")
PYCACHE = "__pycache__"
TMP_CACHE_GLOBS = ("cargo-mutants-*.tmp", "pytest-of-*")
CACHE_NAMES = (*WORKTREE_CACHE_GLOBS, PYCACHE, *TMP_CACHE_GLOBS)
# Never descended into when looking for __pycache__ directories.
WALK_SKIP = {".git", ".venv", "target", "node_modules"}
# Never removed, nor anything below them (relative to a worktree root).
PROTECTED = (".git", ".agents/journal", "specs", "docs")
BUILD_TOOLS = {"cargo", "rustc", "cargo-mutants", "pytest", "py.test", "rustdoc", "clippy-driver"}
DEFAULT_IDLE_HOURS = 12.0
GH_TIMEOUT_S = 20


# ---------------------------------------------------------------------------------------------
# Data


@dataclass
class Proc:
    """What the script knows about one running process."""

    pid: int
    comm: str
    argv: list[str]
    cwd: Path | None
    files: list[Path] = field(default_factory=list)
    target_dir: Path | None = None

    def is_pytest(self) -> bool:
        # pytest, python -m pytest, uv run pytest
        return any(Path(a).name in ("pytest", "py.test") for a in [self.comm, *self.argv])

    def is_cargo_mutants(self) -> bool:
        return self.comm == "cargo-mutants" or self.argv[1:2] == ["mutants"]

    def is_build_tool(self) -> bool:
        names = {self.comm, *(Path(a).name for a in self.argv[:1])}
        return bool(names & BUILD_TOOLS) or self.is_pytest()

    def paths(self) -> list[Path]:
        out = [p for p in (self.cwd, self.target_dir) if p is not None]
        out += self.files
        out += [Path(a) for a in self.argv if a.startswith("/")]
        return out

    def touches(self, root: Path) -> bool:
        return any(is_within(p, root) for p in self.paths())


@dataclass
class Row:
    path: Path
    kind: str  # worktree | cache
    action: str  # remove | removed | keep | skip | failed
    reason: str
    size: int = 0


@dataclass
class Worktree:
    path: Path
    head: str | None
    branch: str | None  # short name, None when detached
    locked: bool
    prunable: bool
    main: bool


GithubLookup = Callable[[str], "tuple[int, str] | bool | None"]
"""Branch -> (pull request number, head sha) if merged, None if not merged, False on error."""


# ---------------------------------------------------------------------------------------------
# Helpers


def is_within(path: Path, root: Path) -> bool:
    try:
        Path(os.path.abspath(path)).relative_to(os.path.abspath(root))
    except ValueError:
        return False
    return True


def git(cwd: Path, *args: str, check: bool = True) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        ["git", "--no-optional-locks", *args],
        cwd=cwd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        check=check,
    )


def disk_usage(path: Path) -> int:
    """Bytes allocated on disk below ``path``; symbolic links are not followed."""
    total = 0
    stack = [path]
    while stack:
        current = stack.pop()
        try:
            st = current.lstat()
        except OSError:
            continue
        total += getattr(st, "st_blocks", 0) * 512 or st.st_size
        if current.is_dir() and not current.is_symlink():
            try:
                stack.extend(current.iterdir())
            except OSError:
                continue
    return total


def human(size: int) -> str:
    value = float(size)
    for unit in ("B", "KiB", "MiB", "GiB"):
        if value < 1024 or unit == "GiB":
            return f"{value:.0f} {unit}" if unit == "B" else f"{value:.1f} {unit}"
        value /= 1024
    return f"{value:.1f} GiB"  # pragma: no cover


def latest_mtime(path: Path, gitdir: Path | None) -> float:
    """Most recent change in the worktree: its git metadata, top level and build output."""
    candidates = [path, path / "target", path / "target" / "debug"]
    candidates += [path / "target" / "debug" / ".cargo-lock", path / ".venv"]
    if gitdir is not None:
        candidates += [gitdir, gitdir / "HEAD", gitdir / "index", gitdir / "logs" / "HEAD"]
    with contextlib.suppress(OSError):
        candidates += list(path.iterdir())
    latest = 0.0
    for c in candidates:
        try:
            latest = max(latest, c.lstat().st_mtime)
        except OSError:
            continue
    return latest


# ---------------------------------------------------------------------------------------------
# Processes


def scan_processes(proc_root: Path = Path("/proc")) -> list[Proc] | None:
    """Running processes of this machine, or None when ``proc_root`` does not exist."""
    if not proc_root.is_dir():
        return None
    me = os.getpid()
    uid = os.getuid() if hasattr(os, "getuid") else None
    procs: list[Proc] = []
    for entry in proc_root.iterdir():
        if not entry.name.isdigit() or int(entry.name) == me:
            continue
        try:
            comm = (entry / "comm").read_text(encoding="utf-8", errors="replace").strip()
            raw = (entry / "cmdline").read_bytes()
        except OSError:
            continue  # the process ended
        argv = [a.decode("utf-8", "replace") for a in raw.split(b"\0") if a]
        try:
            cwd: Path | None = Path(os.readlink(entry / "cwd"))
        except PermissionError:
            # Processes of other users are not readable for an unprivileged user; ours are.
            try:
                owner = entry.stat().st_uid
            except OSError:
                continue
            if uid is not None and owner == uid:
                cwd = Path("/")  # unreadable process of ours: treat as touching everything
            else:
                continue
        except OSError:
            continue
        files: list[Path] = []
        try:
            for fd in (entry / "fd").iterdir():
                try:
                    target = os.readlink(fd)
                except OSError:
                    continue
                if target.startswith("/"):
                    files.append(Path(target.removesuffix(" (deleted)")))
        except OSError:
            pass
        target_dir = None
        try:
            for var in (entry / "environ").read_bytes().split(b"\0"):
                if var.startswith(b"CARGO_TARGET_DIR="):
                    target_dir = Path(var.split(b"=", 1)[1].decode("utf-8", "replace"))
        except OSError:
            pass
        procs.append(Proc(int(entry.name), comm, argv, cwd, files, target_dir))
    return procs


# ---------------------------------------------------------------------------------------------
# GitHub


def github_lookup(owner_repo: str | None, gh: str = "gh") -> GithubLookup:
    """Return a function that finds the merged pull request of a branch with ``gh api``."""

    def lookup(branch: str) -> tuple[int, str] | bool | None:
        if owner_repo is None:
            return False
        owner = owner_repo.split("/")[0]
        url = f"repos/{owner_repo}/pulls?head={owner}:{branch}&state=closed&per_page=100"
        try:
            out = subprocess.run(
                [gh, "api", url],
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=GH_TIMEOUT_S,
                check=True,
            ).stdout
            pulls = json.loads(out)
        except (OSError, subprocess.SubprocessError, ValueError):
            return False
        if not isinstance(pulls, list):
            return False
        for pull in pulls:
            if pull.get("merged_at"):
                return int(pull["number"]), str(pull["head"]["sha"])
        return None

    return lookup


def origin_repo(main: Path) -> str | None:
    url = git(main, "remote", "get-url", "origin", check=False).stdout.strip()
    m = re.search(r"github\.com[:/]([^/]+/[^/]+?)(?:\.git)?/?$", url)
    return m.group(1) if m else None


# ---------------------------------------------------------------------------------------------
# Worktrees


def list_worktrees(repo: Path) -> list[Worktree]:
    out = git(repo, "worktree", "list", "--porcelain").stdout
    trees: list[Worktree] = []
    for n, block in enumerate(b for b in out.strip().split("\n\n") if b.strip()):
        fields: dict[str, str] = {}
        for line in block.splitlines():
            key, _, value = line.partition(" ")
            fields[key] = value
        branch = fields.get("branch")
        trees.append(
            Worktree(
                path=Path(fields["worktree"]),
                head=fields.get("HEAD"),
                branch=branch.removeprefix("refs/heads/") if branch else None,
                locked="locked" in fields,
                prunable="prunable" in fields,
                main=n == 0,
            )
        )
    return trees


def ref_exists(repo: Path, ref: str) -> bool:
    return git(repo, "rev-parse", "--verify", "--quiet", ref, check=False).returncode == 0


def is_ancestor(repo: Path, commit: str, ref: str) -> bool:
    return git(repo, "merge-base", "--is-ancestor", commit, ref, check=False).returncode == 0


class Cleaner:
    def __init__(
        self,
        repo: Path,
        *,
        tmp_root: Path | None,
        caches: bool,
        apply: bool,
        processes: list[Proc] | None,
        github: GithubLookup,
        idle_hours: float = DEFAULT_IDLE_HOURS,
        base: str = "origin/main",
        now: float | None = None,
        out: Callable[[str], None] = print,
    ) -> None:
        self.repo = repo
        self.tmp_root = tmp_root
        self.caches = caches
        self.apply = apply
        self.processes = processes
        self._github = github
        self._merged: dict[str, tuple[int, str] | bool | None] = {}
        self.idle_s = idle_hours * 3600
        self.base = base
        self.now = time.time() if now is None else now
        self.out = out
        self.rows: list[Row] = []
        self.trees = list_worktrees(repo)
        self.main = next(t.path for t in self.trees if t.main)

    def github(self, branch: str) -> tuple[int, str] | bool | None:
        """GitHub is asked at most once per branch and run."""
        if branch not in self._merged:
            self._merged[branch] = self._github(branch)
        return self._merged[branch]

    # -- decisions --------------------------------------------------------------------------

    def worktree_verdict(self, tree: Worktree) -> str | None:
        """None when the worktree can be removed, otherwise the reason to keep it."""
        if tree.main:
            return "main checkout"
        if tree.locked:
            return "locked"
        if tree.prunable or not tree.path.is_dir():
            return "directory missing (git worktree prune handles it)"
        if not tree.head:
            return "no commit"
        status = git(tree.path, "status", "--porcelain", "--untracked-files=all", check=False)
        if status.returncode != 0:
            return "git status failed"
        if status.stdout.strip():
            n = len(status.stdout.strip().splitlines())
            return f"uncommitted or untracked changes ({n} paths)"
        if not ref_exists(self.repo, self.base):
            return f"{self.base} not found"
        if not is_ancestor(self.repo, tree.head, self.base):
            # Not reachable from origin/main: only a squash-merged pull request whose head is
            # exactly this commit proves that nothing would be lost.
            if tree.branch is None:
                return f"detached HEAD not in {self.base}"
            found = self.github(tree.branch)
            if found is False:
                return f"not in {self.base} and GitHub could not be reached"
            if not isinstance(found, tuple):
                return "branch not merged"
            if found[1] != tree.head:
                return f"commits after the merged pull request #{found[0]}"
            # Defence in depth: the commit must also be on the remote branch when it exists
            # (it may have been deleted after the merge).
            remote = f"refs/remotes/origin/{tree.branch}"
            if ref_exists(self.repo, remote) and not is_ancestor(self.repo, tree.head, remote):
                return "unpushed commits"
        if self.processes is None:
            return "running processes cannot be checked (no /proc)"
        users = [p for p in self.processes if p.touches(tree.path)]
        if users:
            return f"in use by process {users[0].pid} ({users[0].comm})"
        gitdir = git(tree.path, "rev-parse", "--absolute-git-dir", check=False).stdout.strip()
        idle = self.now - latest_mtime(tree.path, Path(gitdir) if gitdir else None)
        if idle < self.idle_s:
            return f"active {idle / 3600:.1f} h ago (idle threshold {self.idle_s / 3600:g} h)"
        return None

    def merged_reason(self, tree: Worktree) -> str:
        assert tree.head
        if is_ancestor(self.repo, tree.head, self.base):
            return f"merged into {self.base}"
        assert tree.branch
        found = self.github(tree.branch)
        assert isinstance(found, tuple)
        return f"pull request #{found[0]} merged"

    def cache_verdict(self, path: Path, owner: Path | None) -> str | None:
        """None when a cache directory can be removed, otherwise the reason to keep it."""
        problem = self.unsafe(path, owner)
        if problem:
            return problem
        if self.processes is None:
            return "running processes cannot be checked (no /proc)"
        tools = [p for p in self.processes if p.is_build_tool()]
        users = [p for p in self.processes if p.touches(path)]
        if owner is not None:
            users += [p for p in tools if p.touches(owner)]
        elif fnmatch.fnmatch(path.name, "cargo-mutants-*.tmp"):
            # cargo-mutants works in a copy of the tree: any running instance may own it.
            users += [p for p in self.processes if p.is_cargo_mutants()]
        elif fnmatch.fnmatch(path.name, "pytest-of-*"):
            # Shared by every pytest run of the user: any running pytest may be using it.
            users += [p for p in self.processes if p.is_pytest()]
        if users:
            return f"in use by process {users[0].pid} ({users[0].comm})"
        if not self.caches:
            return "build cache: needs --caches"
        return None

    def unsafe(self, path: Path, owner: Path | None) -> str | None:
        """The allow-list: a reason to refuse ``path``, or None when it may be removed."""
        if not path.is_absolute():
            return "refused: relative path"
        if path.is_symlink():
            return "refused: symbolic link"
        if os.path.realpath(path) != str(path):
            return "refused: path passes through a symbolic link"
        if not path.is_dir():
            return "refused: not a directory"
        if not any(fnmatch.fnmatch(path.name, g) for g in CACHE_NAMES):
            return "refused: not a cache directory name"
        root = owner if owner is not None else self.tmp_root
        if root is None or path == root or not is_within(path, root):
            return "refused: outside the allowed roots"
        if owner is None:
            if path.parent != self.tmp_root or not any(
                fnmatch.fnmatch(path.name, g) for g in TMP_CACHE_GLOBS
            ):
                return "refused: not a temporary cache directory"
            if hasattr(os, "getuid") and path.lstat().st_uid != os.getuid():
                return "refused: owned by another user"
            for tree in self.trees:
                if is_within(tree.path, path):
                    return "refused: contains a worktree"
            return None
        rel = path.relative_to(owner).as_posix()
        for protected in PROTECTED:
            if rel == protected or rel.startswith(protected + "/"):
                return f"refused: under {protected}"
        if any(part == ".git" for part in path.relative_to(owner).parts):
            return "refused: under .git"
        for tree in self.trees:
            if tree.path != owner and is_within(tree.path, path):
                return "refused: contains another worktree"
        tracked = git(owner, "ls-files", "-z", "--", rel, check=False)
        if tracked.returncode != 0 or tracked.stdout:
            return "refused: contains files tracked by git"
        return None

    # -- discovery --------------------------------------------------------------------------

    def worktree_caches(self, tree: Path) -> list[Path]:
        found: list[Path] = []
        try:
            for child in sorted(tree.iterdir()):
                if any(fnmatch.fnmatch(child.name, g) for g in WORKTREE_CACHE_GLOBS):
                    found.append(child)
        except OSError:
            return found
        other_roots = {t.path for t in self.trees} - {tree}
        for dirpath, dirnames, _ in os.walk(tree):
            current = Path(dirpath)
            keep = []
            for d in sorted(dirnames):
                sub = current / d
                if d == PYCACHE and not sub.is_symlink():
                    found.append(sub)
                elif d not in WALK_SKIP and sub not in other_roots and not sub.is_symlink():
                    keep.append(d)
            dirnames[:] = keep
        return found

    def tmp_caches(self) -> list[Path]:
        if self.tmp_root is None or not self.tmp_root.is_dir():
            return []
        return sorted(
            p
            for p in self.tmp_root.iterdir()
            if any(fnmatch.fnmatch(p.name, g) for g in TMP_CACHE_GLOBS)
        )

    # -- actions ----------------------------------------------------------------------------

    def remove_worktree(self, tree: Worktree, size: int, reason: str) -> None:
        if not self.apply:
            self.rows.append(Row(tree.path, "worktree", "remove", reason, size))
            return
        again = self.worktree_verdict(tree)  # conditions may have changed since the scan
        if again is not None:
            self.rows.append(Row(tree.path, "worktree", "keep", again, size))
            return
        result = git(self.main, "worktree", "remove", str(tree.path), check=False)
        if result.returncode == 0:
            self.rows.append(Row(tree.path, "worktree", "removed", reason, size))
        else:
            msg = result.stderr.strip().splitlines()[-1] if result.stderr.strip() else "error"
            self.rows.append(Row(tree.path, "worktree", "failed", msg, size))

    def remove_cache(self, path: Path, owner: Path | None, size: int) -> None:
        if not self.apply:
            self.rows.append(Row(path, "cache", "remove", "rebuildable build cache", size))
            return
        again = self.cache_verdict(path, owner)
        if again is not None:
            self.rows.append(Row(path, "cache", "keep", again, size))
            return
        try:
            shutil.rmtree(path)  # refuses a symbolic link; never follows links inside
        except OSError as exc:
            self.rows.append(Row(path, "cache", "failed", str(exc), size))
            return
        self.rows.append(Row(path, "cache", "removed", "rebuildable build cache", size))

    def run(self) -> list[Row]:
        kept: list[Path] = []
        for tree in self.trees:
            reason = self.worktree_verdict(tree)
            size = disk_usage(tree.path) if tree.path.is_dir() else 0
            if reason is None:
                self.remove_worktree(tree, size, self.merged_reason(tree))
                if self.rows[-1].action in ("keep", "failed"):
                    kept.append(tree.path)
            else:
                self.rows.append(Row(tree.path, "worktree", "keep", reason, size))
                if tree.path.is_dir():
                    kept.append(tree.path)
        candidates: list[tuple[Path, Path | None]] = []
        for tree_path in kept:
            candidates += [(c, tree_path) for c in self.worktree_caches(tree_path)]
        candidates += [(c, None) for c in self.tmp_caches()]
        for path, owner in candidates:
            size = disk_usage(path)
            reason = self.cache_verdict(path, owner)
            if reason is None:
                self.remove_cache(path, owner, size)
            elif size or not reason.startswith("build cache"):
                self.rows.append(Row(path, "cache", "skip", reason, size))
        prune = ["worktree", "prune", "--verbose"] + ([] if self.apply else ["--dry-run"])
        pruned = git(self.main, *prune, check=False)
        for line in (pruned.stdout + pruned.stderr).strip().splitlines():
            self.out(f"git worktree prune: {line}")
        return self.rows


# ---------------------------------------------------------------------------------------------
# Output


def table(rows: Iterable[Row], apply: bool) -> str:
    rows = list(rows)
    header = ("Path", "Kind", "Action", "Reason", "Size")
    body = [(str(r.path), r.kind, r.action, r.reason, human(r.size)) for r in rows]
    widths = [max(len(x) for x in col) for col in zip(header, *body, strict=True)]

    def fmt(cells: tuple[str, ...]) -> str:
        parts = [c.ljust(w) for c, w in zip(cells[:-1], widths[:-1], strict=True)]
        return "  ".join([*parts, cells[-1].rjust(widths[-1])])

    lines = [fmt(header), "  ".join("-" * w for w in widths)]
    lines += [fmt(b) for b in body]
    freed = sum(r.size for r in rows if r.action in ("remove", "removed"))
    failed = sum(1 for r in rows if r.action == "failed")
    lines.append("")
    if apply:
        lines.append(f"Freed: {human(freed)}." + (f" Failures: {failed}." if failed else ""))
    else:
        lines.append(f"Would free: {human(freed)}. Dry run: nothing was removed.")
        lines.append("Read the kept list, then re-run with --apply (and --caches for caches).")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__.splitlines()[0],
        epilog="Dry run unless --apply is given. See docs/en/project/contributing.md.",
    )
    parser.add_argument("--apply", action="store_true", help="remove; without it: dry run")
    parser.add_argument("--caches", action="store_true", help="also remove build caches")
    parser.add_argument("--repo", type=Path, default=Path.cwd(), help="any checkout of the repo")
    parser.add_argument(
        "--tmp-root",
        type=Path,
        default=Path(tempfile.gettempdir()),
        help="directory holding cargo-mutants-*.tmp and pytest-of-* (default: system temp)",
    )
    parser.add_argument("--no-tmp", action="store_true", help="ignore the temporary directory")
    parser.add_argument(
        "--idle-hours",
        type=float,
        default=DEFAULT_IDLE_HOURS,
        help=f"keep worktrees changed more recently (default {DEFAULT_IDLE_HOURS:g})",
    )
    parser.add_argument("--offline", action="store_true", help="do not ask GitHub")
    parser.add_argument("--gh", default="gh", help="GitHub CLI executable")
    args = parser.parse_args(argv)

    top = subprocess.run(
        ["git", "rev-parse", "--path-format=absolute", "--git-common-dir"],
        cwd=args.repo,
        capture_output=True,
        text=True,
        encoding="utf-8",
    )
    if top.returncode != 0:
        print(f"Not a git repository: {args.repo}", file=sys.stderr)
        return 2
    trees = list_worktrees(args.repo)
    main_checkout = trees[0].path
    github: GithubLookup = (
        (lambda _branch: False)
        if args.offline
        else github_lookup(origin_repo(main_checkout), args.gh)
    )
    tmp_root = None if args.no_tmp else Path(os.path.realpath(args.tmp_root))
    cleaner = Cleaner(
        main_checkout,
        tmp_root=tmp_root,
        caches=args.caches,
        apply=args.apply,
        processes=scan_processes(),
        github=github,
        idle_hours=args.idle_hours,
    )
    rows = cleaner.run()
    print(table(rows, args.apply))
    return 1 if any(r.action == "failed" for r in rows) else 0


if __name__ == "__main__":
    sys.exit(main())
