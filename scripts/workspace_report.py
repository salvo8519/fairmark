"""Report disk use of the git worktrees of this repository. Read-only: it changes nothing.

    python scripts/workspace_report.py              # every worktree, then the disk summary
    python scripts/workspace_report.py --offline    # do not ask GitHub

For each worktree the report shows its path, its branch (or detached HEAD), its size, the size
of the rebuildable caches inside it (``target/``, ``.venv/``, ``__pycache__/``,
``.pytest_cache/``, ``mutants.out*``), whether its branch is merged into ``main``, the number
of uncommitted, untracked and ignored non-cache paths, and its last activity. It then lists the
cargo-mutants and pytest directories of the temporary directory and the usage of every file
system involved.

"Merged" is ``yes`` when the worktree commit is reachable from ``origin/main``, or when GitHub
reports a pull request of this repository, from this branch, into ``main``, merged, whose head
is exactly the worktree commit (squash merges). Anything that cannot be established is printed
as ``unknown``, never guessed.

The script never deletes, moves or rewrites a file and never changes a git reference: git runs
with ``--no-optional-locks`` in an environment without the variables that redirect it. Deleting
anything is a manual decision of the maintainer (docs/en/project/workspace-report.md).
"""

from __future__ import annotations

import argparse
import contextlib
import fnmatch
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from collections.abc import Callable, Mapping
from dataclasses import dataclass, field
from pathlib import Path
from urllib.parse import quote

# Rebuildable caches at the top level of a worktree.
TOP_CACHE_GLOBS = ("target", ".venv", ".pytest_cache", "mutants.out*")
PYCACHE = "__pycache__"
CACHE_GLOBS = (*TOP_CACHE_GLOBS, PYCACHE)
# Rebuildable caches in the temporary directory.
TMP_CACHE_GLOBS = ("cargo-mutants-*.tmp", "pytest-of-*")
# Never descended into when looking for __pycache__ directories.
WALK_SKIP = {".git", ".venv", "target", "node_modules"}
# Variables that make git operate on another repository, work tree, index or configuration.
# They are removed from the environment of every git call (GIT_CONFIG* by prefix).
UNSAFE_GIT_ENV = frozenset(
    {
        "GIT_DIR",
        "GIT_WORK_TREE",
        "GIT_INDEX_FILE",
        "GIT_COMMON_DIR",
        "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES",
        "GIT_NAMESPACE",
        "GIT_CEILING_DIRECTORIES",
    }
)
BASE_BRANCH = "main"
BASE_REF = "origin/main"
GH_TIMEOUT_S = 20
UNKNOWN = "unknown"


# ---------------------------------------------------------------------------------------------
# Data


@dataclass
class Worktree:
    path: Path
    head: str | None
    branch: str | None  # short name, None when detached
    detached: bool
    locked: bool
    prunable: bool
    main: bool


@dataclass(frozen=True)
class Merged:
    """Whether a branch is merged into main: ``yes``, ``no`` or ``unknown``, with the reason."""

    state: str
    detail: str

    def __str__(self) -> str:
        return f"{self.state} ({self.detail})"


@dataclass
class Status:
    """Paths reported by git; None for a value that could not be read."""

    uncommitted: int | None = None
    untracked: int | None = None
    ignored: list[str] | None = None  # ignored paths that are not rebuildable caches


@dataclass
class Report:
    tree: Worktree
    verified: str | None  # None when the worktree belongs to this repository, else the reason
    size: int | None
    caches: dict[str, int] = field(default_factory=dict)
    merged: Merged = field(default_factory=lambda: Merged(UNKNOWN, "not checked"))
    status: Status = field(default_factory=Status)
    last_activity: float | None = None


GithubLookup = Callable[[str, str], Merged]
"""(branch, head sha) -> merged state according to GitHub."""


# ---------------------------------------------------------------------------------------------
# Helpers


def sanitized_env(environ: Mapping[str, str] | None = None) -> dict[str, str]:
    """The environment without the variables that redirect git (``UNSAFE_GIT_ENV``)."""
    source = os.environ if environ is None else environ
    return {
        k: v
        for k, v in source.items()
        if k not in UNSAFE_GIT_ENV and not k.startswith("GIT_CONFIG")
    }


_GIT_ENV: dict[str, str] | None = None


def git_env() -> dict[str, str]:
    """Built once per run, then passed to every git call."""
    global _GIT_ENV
    if _GIT_ENV is None:
        _GIT_ENV = sanitized_env()
    return _GIT_ENV


def git(cwd: Path, *args: str) -> subprocess.CompletedProcess[str]:
    """Run a read-only git command; never raises on a non-zero exit status."""
    return subprocess.run(
        ["git", "--no-optional-locks", *args],
        cwd=cwd,
        capture_output=True,
        text=True,
        encoding="utf-8",
        errors="replace",
        check=False,
        env=git_env(),
    )


def common_dir(path: Path) -> Path | None:
    """The real path of the git common directory of ``path``, or None."""
    try:
        out = git(path, "rev-parse", "--path-format=absolute", "--git-common-dir")
    except OSError:
        return None
    if out.returncode != 0 or not out.stdout.strip():
        return None
    return Path(os.path.realpath(out.stdout.strip()))


def disk_usage(path: Path, skip: frozenset[Path] = frozenset()) -> int:
    """Bytes allocated below ``path``; symbolic links are not followed, ``skip`` not entered."""
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
                stack.extend(c for c in current.iterdir() if c not in skip)
            except OSError:
                continue
    return total


def human(size: int | None) -> str:
    if size is None:
        return UNKNOWN
    value = float(size)
    for unit in ("B", "KiB", "MiB", "GiB"):
        if value < 1024 or unit == "GiB":
            return f"{value:.0f} {unit}" if unit == "B" else f"{value:.1f} {unit}"
        value /= 1024
    return f"{value:.1f} GiB"  # pragma: no cover


def latest_mtime(path: Path, gitdir: Path | None) -> float | None:
    """Most recent change in the worktree: its git metadata, top level and build output."""
    candidates = [path, path / "target", path / "target" / "debug"]
    candidates += [path / "target" / "debug" / ".cargo-lock", path / ".venv"]
    if gitdir is not None:
        candidates += [gitdir, gitdir / "HEAD", gitdir / "index", gitdir / "logs" / "HEAD"]
    with contextlib.suppress(OSError):
        candidates += list(path.iterdir())
    latest: float | None = None
    for c in candidates:
        try:
            mtime = c.lstat().st_mtime
        except OSError:
            continue
        latest = mtime if latest is None else max(latest, mtime)
    return latest


def is_cache_path(rel: str) -> bool:
    """True when any component of a relative path names a rebuildable cache."""
    parts = [p for p in rel.strip("/").split("/") if p]
    return any(fnmatch.fnmatch(p, g) for p in parts for g in CACHE_GLOBS)


# ---------------------------------------------------------------------------------------------
# Worktrees


def parse_worktree_list(raw: str) -> list[Worktree]:
    """Parse ``git worktree list --porcelain -z``: NUL-terminated lines, empty line per record."""
    trees: list[Worktree] = []
    fields: dict[str, str] = {}
    for line in [*raw.split("\0"), ""]:
        if line:
            key, _, value = line.partition(" ")
            fields[key] = value
            continue
        if "worktree" in fields:
            branch = fields.get("branch")
            trees.append(
                Worktree(
                    path=Path(fields["worktree"]),
                    head=fields.get("HEAD"),
                    branch=branch.removeprefix("refs/heads/") if branch else None,
                    detached="detached" in fields,
                    locked="locked" in fields,
                    prunable="prunable" in fields,
                    main=not trees,
                )
            )
        fields = {}
    return trees


def list_worktrees(repo: Path) -> list[Worktree]:
    out = git(repo, "worktree", "list", "--porcelain", "-z")
    if out.returncode != 0:
        raise RuntimeError(out.stderr.strip() or "git worktree list failed")
    trees = parse_worktree_list(out.stdout)
    if not trees:
        raise RuntimeError("git worktree list returned no worktree")
    return trees


def ref_exists(repo: Path, ref: str) -> bool:
    return git(repo, "rev-parse", "--verify", "--quiet", ref).returncode == 0


def is_ancestor(repo: Path, commit: str, ref: str) -> bool | None:
    """True or False, None when git cannot tell (for example an unknown commit)."""
    code = git(repo, "merge-base", "--is-ancestor", commit, ref).returncode
    return {0: True, 1: False}.get(code)


def read_status(path: Path) -> Status:
    status = Status()
    out = git(path, "status", "--porcelain=v1", "-z", "--untracked-files=all")
    if out.returncode == 0:
        entries = out.stdout.split("\0")
        uncommitted = untracked = 0
        i = 0
        while i < len(entries):
            entry = entries[i]
            i += 1
            if not entry:
                continue
            if entry.startswith("??"):
                untracked += 1
            else:
                uncommitted += 1
                if entry[0] in "RC":
                    i += 1  # the original path of a rename or copy follows
        status.uncommitted, status.untracked = uncommitted, untracked
    ignored = git(
        path, "ls-files", "-z", "--others", "--ignored", "--exclude-standard", "--directory"
    )
    if ignored.returncode == 0:
        paths = sorted(p for p in ignored.stdout.split("\0") if p and not is_cache_path(p))
        # An ignored directory is listed once: entries below a listed directory are dropped.
        dirs = [p for p in paths if p.endswith("/")]
        status.ignored = [p for p in paths if not any(p != d and p.startswith(d) for d in dirs)]
    return status


def worktree_caches(tree: Path, other_roots: set[Path]) -> dict[str, int]:
    """Size of each kind of rebuildable cache inside a worktree."""
    sizes: dict[str, int] = {}
    try:
        children = sorted(tree.iterdir())
    except OSError:
        return sizes
    for child in children:
        for pattern in TOP_CACHE_GLOBS:
            if fnmatch.fnmatch(child.name, pattern) and not child.is_symlink():
                key = f"{pattern}/"
                sizes[key] = sizes.get(key, 0) + disk_usage(child)
    pycache = 0
    for dirpath, dirnames, _ in os.walk(tree):
        current = Path(dirpath)
        keep = []
        for d in sorted(dirnames):
            sub = current / d
            if sub.is_symlink() or sub in other_roots:
                continue
            if d == PYCACHE:
                pycache += disk_usage(sub)
            elif d not in WALK_SKIP:
                keep.append(d)
        dirnames[:] = keep
    if pycache:
        sizes[f"{PYCACHE}/"] = pycache
    return sizes


# ---------------------------------------------------------------------------------------------
# GitHub


def origin_repo(main: Path) -> str | None:
    url = git(main, "remote", "get-url", "origin").stdout.strip()
    m = re.search(r"github\.com[:/]([^/]+/[^/]+?)(?:\.git)?/?$", url)
    return m.group(1) if m else None


def classify_pulls(pulls: object, owner_repo: str, branch: str, head: str) -> Merged:
    """Decide from the pull request list of the GitHub REST API. Malformed data is unknown.

    A pull request counts as merged only when ``merged_at`` is set, its base is ``main``, its
    head branch is ``branch`` in this repository (not a fork) and its head is exactly ``head``.
    """
    malformed = Merged(UNKNOWN, "unexpected GitHub response")
    if not isinstance(pulls, list):
        return malformed
    # The whole response is validated before any decision: one malformed entry makes it unknown.
    parsed: list[tuple[int, str | None, str, str, str, str | None]] = []
    for pull in pulls:
        try:
            number = pull["number"]
            merged_at = pull["merged_at"]
            base_ref = pull["base"]["ref"]
            head_ref = pull["head"]["ref"]
            head_sha = pull["head"]["sha"]
            head_repo = pull["head"]["repo"]  # null when the fork was deleted
            head_repo_name = None if head_repo is None else head_repo["full_name"]
        except (KeyError, TypeError, IndexError):
            return malformed
        if (
            isinstance(number, bool)
            or not isinstance(number, int)
            or not all(isinstance(v, str) for v in (base_ref, head_ref, head_sha))
            or not (merged_at is None or isinstance(merged_at, str))
            or not (head_repo_name is None or isinstance(head_repo_name, str))
        ):
            return malformed
        parsed.append((number, merged_at, base_ref, head_ref, head_sha, head_repo_name))
    merged_other: list[int] = []
    for number, merged_at, base_ref, head_ref, head_sha, head_repo_name in parsed:
        if not merged_at:
            continue  # closed without merge
        if base_ref != BASE_BRANCH or head_ref != branch:
            continue
        if head_repo_name is None or head_repo_name.lower() != owner_repo.lower():
            continue
        if head_sha == head:
            return Merged("yes", f"pull request #{number} merged")
        merged_other.append(number)
    if merged_other:
        return Merged("no", f"commits after merged pull request #{merged_other[0]}")
    return Merged("no", "no merged pull request from this branch into main")


def pulls_url(owner_repo: str, branch: str) -> str:
    owner = owner_repo.split("/")[0]
    head = quote(f"{owner}:{branch}", safe="")
    return f"repos/{owner_repo}/pulls?head={head}&base={BASE_BRANCH}&state=closed&per_page=100"


def github_lookup(owner_repo: str | None, gh: str = "gh") -> GithubLookup:
    """Return a function that asks the GitHub REST API (``gh api``) about a branch."""

    def lookup(branch: str, head: str) -> Merged:
        if owner_repo is None:
            return Merged(UNKNOWN, "origin is not a GitHub repository")
        try:
            out = subprocess.run(
                [gh, "api", pulls_url(owner_repo, branch)],
                capture_output=True,
                text=True,
                encoding="utf-8",
                timeout=GH_TIMEOUT_S,
                check=True,
            ).stdout
            pulls = json.loads(out)
        except (OSError, subprocess.SubprocessError, ValueError):
            return Merged(UNKNOWN, "GitHub lookup failed")
        return classify_pulls(pulls, owner_repo, branch, head)

    return lookup


def offline_lookup(_branch: str, _head: str) -> Merged:
    return Merged(UNKNOWN, f"not in {BASE_REF}; GitHub not asked (--offline)")


# ---------------------------------------------------------------------------------------------
# Report


class Reporter:
    def __init__(self, repo: Path, *, github: GithubLookup, tmp_root: Path | None) -> None:
        common = common_dir(repo)
        if common is None:
            raise RuntimeError(f"not a git repository: {repo}")
        self.common = common
        self.trees = list_worktrees(repo)
        self.main = self.trees[0].path
        self.github = github
        self.tmp_root = tmp_root
        self.has_base = ref_exists(repo, BASE_REF)
        self.repo = repo

    def merged(self, tree: Worktree) -> Merged:
        if not tree.head:
            return Merged(UNKNOWN, "no commit")
        if not self.has_base:
            return Merged(UNKNOWN, f"{BASE_REF} not found")
        reachable = is_ancestor(self.repo, tree.head, BASE_REF)
        if reachable is None:
            return Merged(UNKNOWN, "git could not compare the commit")
        if reachable:
            return Merged("yes", f"reachable from {BASE_REF}")
        if tree.branch is None:
            return Merged(UNKNOWN, f"detached HEAD not in {BASE_REF}")
        if tree.branch == BASE_BRANCH:
            return Merged("no", f"local commits not in {BASE_REF}")
        return self.github(tree.branch, tree.head)

    def report(self, tree: Worktree) -> Report:
        if not tree.path.is_dir():
            return Report(tree, "directory missing", None)
        roots = {t.path for t in self.trees} - {tree.path}
        size = disk_usage(tree.path, frozenset(roots))
        if common_dir(tree.path) != self.common:
            # Not (or no longer) a worktree of this repository: no git query runs inside it.
            return Report(tree, "not a worktree of this repository", size)
        gitdir = git(tree.path, "rev-parse", "--absolute-git-dir").stdout.strip()
        return Report(
            tree,
            None,
            size,
            caches=worktree_caches(tree.path, roots),
            merged=self.merged(tree),
            status=read_status(tree.path),
            last_activity=latest_mtime(tree.path, Path(gitdir) if gitdir else None),
        )

    def tmp_caches(self) -> list[tuple[Path, int]]:
        if self.tmp_root is None or not self.tmp_root.is_dir():
            return []
        try:
            entries = sorted(self.tmp_root.iterdir())
        except OSError:
            return []
        uid = os.getuid() if hasattr(os, "getuid") else None
        out = []
        for p in entries:
            if not any(fnmatch.fnmatch(p.name, g) for g in TMP_CACHE_GLOBS):
                continue
            with contextlib.suppress(OSError):
                if uid is not None and p.lstat().st_uid != uid:
                    continue
                out.append((p, disk_usage(p)))
        return out

    def run(self) -> list[Report]:
        return [self.report(t) for t in self.trees]


Disk = tuple[Path, int | None, int | None, int | None]


def filesystem_usage(paths: list[Path]) -> list[Disk]:
    """(path, size, used, available) for each file system once; None where unknown."""
    seen: set[int] = set()
    rows: list[Disk] = []
    statvfs = getattr(os, "statvfs", None)
    for path in paths:
        try:
            dev = path.stat().st_dev
        except OSError:
            continue
        if dev in seen:
            continue
        seen.add(dev)
        try:
            st = statvfs(path) if statvfs else None
        except OSError:
            st = None
        if st is None:
            rows.append((path, None, None, None))
            continue
        size = st.f_blocks * st.f_frsize
        used = (st.f_blocks - st.f_bfree) * st.f_frsize
        rows.append((path, size, used, st.f_bavail * st.f_frsize))
    return rows


def fmt_count(n: int | None) -> str:
    return UNKNOWN if n is None else str(n)


def fmt_time(ts: float | None, now: float) -> str:
    if ts is None:
        return UNKNOWN
    stamp = time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime(ts))
    hours = max(0.0, now - ts) / 3600
    ago = f"{hours:.1f} h ago" if hours < 48 else f"{hours / 24:.1f} days ago"
    return f"{stamp} ({ago})"


def render(
    reports: list[Report], tmp: list[tuple[Path, int]], disks: list[Disk], now: float
) -> str:
    lines: list[str] = []
    for r in reports:
        t = r.tree
        lines.append(f"Worktree {t.path}" + ("  (main checkout)" if t.main else ""))
        branch = t.branch if t.branch else ("detached HEAD" if t.detached else UNKNOWN)
        flags = [f for f, on in (("locked", t.locked), ("prunable", t.prunable)) if on]
        suffix = f" [{', '.join(flags)}]" if flags else ""
        lines.append(f"  Branch               {branch}{suffix}")
        lines.append(f"  HEAD                 {t.head[:12] if t.head else UNKNOWN}")
        lines.append(f"  Size                 {human(r.size)}")
        if r.verified is not None:
            lines.append(f"  Note                 {r.verified}; other details {UNKNOWN}")
            lines.append("")
            continue
        if r.caches:
            parts = ", ".join(f"{k} {human(v)}" for k, v in sorted(r.caches.items()))
            total = human(sum(r.caches.values()))
            lines.append(f"  Caches               {total}, cache rebuildable: {parts}")
        else:
            lines.append("  Caches               none")
        lines.append(f"  Merged into main     {r.merged}")
        s = r.status
        ignored = fmt_count(None if s.ignored is None else len(s.ignored))
        if s.ignored:
            ignored += " (" + ", ".join(s.ignored[:3]) + (", ..." if len(s.ignored) > 3 else "")
            ignored += ")"
        lines.append(f"  Uncommitted          {fmt_count(s.uncommitted)}")
        lines.append(f"  Untracked            {fmt_count(s.untracked)}")
        lines.append(f"  Ignored, not cache   {ignored}")
        lines.append(f"  Last activity        {fmt_time(r.last_activity, now)}")
        lines.append("")
    if tmp:
        lines.append("Temporary directory caches (cache rebuildable)")
        lines += [f"  {human(size):>10}  {path}" for path, size in tmp]
        lines.append("")
    sizes = [r.size for r in reports]
    total = UNKNOWN if None in sizes else human(sum(s for s in sizes if s is not None))
    caches = sum(sum(r.caches.values()) for r in reports) + sum(s for _, s in tmp)
    lines.append(f"Worktrees: {len(reports)}, total size {total}")
    lines.append(f"Rebuildable caches: {human(caches)}")
    lines.append("")
    lines.append("Disk usage")
    for path, size, used, available in disks:
        if size is None or used is None:
            lines.append(f"  {path}: {UNKNOWN}")
            continue
        # As df: the share of the space usable by unprivileged users that is in use.
        usable = used + (available or 0)
        pct = f"{100 * used / usable:.0f} percent" if usable else UNKNOWN
        lines.append(
            f"  {path}: size {human(size)}, used {human(used)} ({pct}), "
            f"available {human(available)}"
        )
    lines.append("")
    lines.append("Read-only report: nothing was changed.")
    return "\n".join(lines)


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description=__doc__.splitlines()[0],
        epilog="See docs/en/project/workspace-report.md.",
    )
    parser.add_argument("--repo", type=Path, default=Path.cwd(), help="any checkout of the repo")
    parser.add_argument(
        "--tmp-root",
        type=Path,
        default=Path(tempfile.gettempdir()),
        help="directory holding cargo-mutants-*.tmp and pytest-of-* (default: system temp)",
    )
    parser.add_argument("--no-tmp", action="store_true", help="ignore the temporary directory")
    parser.add_argument("--offline", action="store_true", help="do not ask GitHub")
    parser.add_argument("--gh", default="gh", help="GitHub CLI executable")
    args = parser.parse_args(argv)

    global _GIT_ENV
    _GIT_ENV = sanitized_env()
    tmp_root = None if args.no_tmp else Path(os.path.realpath(args.tmp_root))
    try:
        main_checkout = list_worktrees(args.repo)[0].path
        github = (
            offline_lookup if args.offline else github_lookup(origin_repo(main_checkout), args.gh)
        )
        reporter = Reporter(main_checkout, github=github, tmp_root=tmp_root)
    except (RuntimeError, OSError) as exc:
        print(f"Cannot read the worktrees of {args.repo}: {exc}", file=sys.stderr)
        return 2
    reports = reporter.run()
    tmp = reporter.tmp_caches()
    paths = [r.tree.path for r in reports if r.tree.path.is_dir()]
    if tmp_root is not None:
        paths.append(tmp_root)
    print(render(reports, tmp, filesystem_usage(paths), time.time()))
    return 0


if __name__ == "__main__":
    sys.exit(main())
