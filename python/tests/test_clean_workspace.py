"""scripts/clean_workspace.py: every rule on temporary git repositories, never real worktrees."""

from __future__ import annotations

import importlib.util
import os
import shutil
import subprocess
import sys
import time
from collections.abc import Iterator
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/clean_workspace.py"
_spec = importlib.util.spec_from_file_location("clean_workspace", SCRIPT)
assert _spec and _spec.loader
cw = importlib.util.module_from_spec(_spec)
sys.modules["clean_workspace"] = cw  # dataclasses resolve types through sys.modules
_spec.loader.exec_module(cw)

pytestmark = pytest.mark.skipif(shutil.which("git") is None, reason="git is required")
needs_proc = pytest.mark.skipif(
    not Path("/proc/self").is_dir(), reason="real processes are read from /proc (Linux)"
)


def sh(cwd: Path, *args: str) -> str:
    return subprocess.run(
        args, cwd=cwd, check=True, capture_output=True, text=True, encoding="utf-8"
    ).stdout.strip()


class Repo:
    """An origin (bare), its main clone and helpers to add worktrees."""

    def __init__(self, base: Path) -> None:
        self.base = base
        self.origin = base / "origin.git"
        self.main = base / "main"
        self.tmp = base / "tmp"
        self.tmp.mkdir()
        sh(base, "git", "init", "-q", "--bare", "-b", "main", str(self.origin))
        sh(base, "git", "clone", "-q", str(self.origin), str(self.main))
        sh(self.main, "git", "switch", "-q", "-c", "main")
        for path, text in {
            "README.md": "readme\n",
            "src/app.py": "x = 1\n",
            "docs/page.md": "doc\n",
            "specs/s.md": "spec\n",
            ".agents/journal/j.md": "journal\n",
            ".gitignore": "target/\n__pycache__/\nmutants.out*/\n.pytest_cache/\n",
        }.items():
            (self.main / path).parent.mkdir(parents=True, exist_ok=True)
            (self.main / path).write_text(text, encoding="utf-8")
        sh(self.main, "git", "add", "-A")
        sh(self.main, "git", "commit", "-q", "-m", "init")
        sh(self.main, "git", "push", "-q", "-u", "origin", "main")

    def worktree(
        self, name: str, *, commit: bool = True, push: bool = True, merge: bool = False
    ) -> Path:
        path = self.base / name
        sh(self.main, "git", "worktree", "add", "-q", "-b", name, str(path), "origin/main")
        if commit:
            (path / f"{name}.txt").write_text(name, encoding="utf-8")
            sh(path, "git", "add", "-A")
            sh(path, "git", "commit", "-q", "-m", name)
        if push:
            sh(path, "git", "push", "-q", "origin", name)
        if merge:
            sh(path, "git", "push", "-q", "origin", "HEAD:main")
        sh(self.main, "git", "fetch", "-q", "origin")
        return path

    def squash_into_main(self, branch_path: Path) -> None:
        """Simulate a squash merge: main gets the content, not the commit."""
        other = self.base / "squasher"
        sh(self.main, "git", "worktree", "add", "-q", "--detach", str(other), "origin/main")
        sh(other, "git", "merge", "-q", "--squash", head(branch_path))
        sh(other, "git", "commit", "-q", "-m", "squash")
        sh(other, "git", "push", "-q", "origin", "HEAD:main")
        sh(self.main, "git", "worktree", "remove", str(other))
        sh(self.main, "git", "fetch", "-q", "origin")

    def cleaner(self, **kw: Any) -> Any:
        options: dict[str, Any] = {
            "tmp_root": self.tmp,
            "caches": False,
            "apply": False,
            "processes": [],
            "github": lambda _b: None,
            "idle_hours": 0,
            "out": lambda _s: None,
        }
        options.update(kw)
        return cw.Cleaner(self.main, **options)


def head(path: Path) -> str:
    return sh(path, "git", "rev-parse", "HEAD")


def row(rows: list[Any], path: Path) -> Any:
    matches = [r for r in rows if r.path == path]
    assert len(matches) == 1, (path, rows)
    return matches[0]


@pytest.fixture
def repo(tmp_path: Path, monkeypatch: pytest.MonkeyPatch) -> Repo:
    for key, value in {
        "GIT_AUTHOR_NAME": "t",
        "GIT_AUTHOR_EMAIL": "t@example.com",
        "GIT_COMMITTER_NAME": "t",
        "GIT_COMMITTER_EMAIL": "t@example.com",
        "GIT_CONFIG_GLOBAL": os.devnull,
        "GIT_CONFIG_NOSYSTEM": "1",
    }.items():
        monkeypatch.setenv(key, value)
    return Repo(tmp_path.resolve())


@pytest.fixture
def sleeper(tmp_path: Path) -> Iterator[Any]:
    """Start a long-running process with a given name and working directory."""
    started: list[subprocess.Popen[bytes]] = []

    def start(cwd: Path, name: str = "sleep") -> subprocess.Popen[bytes]:
        if name == "sleep":
            exe = shutil.which("sleep")
            assert exe
            command = [exe, "60"]
        else:
            # A shell script runs under its own name (the kernel names a process after the file
            # it executes). A copy of sleep would not do: multicall coreutils dispatch on argv[0].
            # The loop keeps the shell alive instead of letting it exec its last command.
            script = tmp_path / "bin" / name
            script.parent.mkdir(exist_ok=True)
            script.write_text("#!/bin/sh\nwhile :; do sleep 1; done\n", encoding="utf-8")
            script.chmod(0o755)
            command = [str(script)]
        proc = subprocess.Popen(command, cwd=cwd)
        started.append(proc)
        time.sleep(0.2)
        return proc

    yield start
    for proc in started:
        proc.kill()
        proc.wait()


# ------------------------------------------------------------------------------------------
# Worktrees


def test_merged_worktree_is_listed_in_dry_run_and_removed_with_apply(repo: Repo) -> None:
    wt = repo.worktree("done", merge=True)
    r = row(repo.cleaner().run(), wt)
    assert (r.action, r.reason) == ("remove", "merged into origin/main")
    assert r.size > 0 and wt.exists()  # dry run: nothing removed

    r = row(repo.cleaner(apply=True).run(), wt)
    assert r.action == "removed" and not wt.exists()
    assert sh(repo.main, "git", "branch", "--list", "done")  # the branch is kept


def test_main_checkout_is_always_kept(repo: Repo) -> None:
    assert row(repo.cleaner(apply=True).run(), repo.main).reason == "main checkout"
    assert (repo.main / "README.md").exists()


def test_locked_worktree_is_kept(repo: Repo) -> None:
    wt = repo.worktree("locked", merge=True)
    sh(repo.main, "git", "worktree", "lock", str(wt))
    r = row(repo.cleaner(apply=True).run(), wt)
    assert (r.action, r.reason) == ("keep", "locked") and wt.exists()


@pytest.mark.parametrize("change", ["modified", "untracked", "staged"])
def test_uncommitted_or_untracked_changes_keep_the_worktree(repo: Repo, change: str) -> None:
    wt = repo.worktree("dirty", merge=True)
    if change == "modified":
        (wt / "README.md").write_text("edited\n", encoding="utf-8")
    elif change == "untracked":
        (wt / "notes.txt").write_text("work in progress\n", encoding="utf-8")
    else:
        (wt / "new.py").write_text("y = 2\n", encoding="utf-8")
        sh(wt, "git", "add", "new.py")
    r = row(repo.cleaner(apply=True).run(), wt)
    assert r.action == "keep" and r.reason.startswith("uncommitted or untracked")
    assert wt.exists()


def test_unmerged_branch_is_kept(repo: Repo) -> None:
    wt = repo.worktree("open")
    r = row(repo.cleaner(apply=True).run(), wt)
    assert (r.action, r.reason) == ("keep", "branch not merged") and wt.exists()


def test_squash_merged_pull_request_with_the_same_head_is_removable(repo: Repo) -> None:
    wt = repo.worktree("squashed")
    repo.squash_into_main(wt)
    sha = head(wt)
    asked: list[str] = []

    def github(branch: str) -> tuple[int, str]:
        asked.append(branch)
        return 41, sha

    r = row(repo.cleaner(github=github, apply=True).run(), wt)
    assert (r.action, r.reason) == ("removed", "pull request #41 merged")
    assert asked == ["squashed"]  # one request per branch, even with the re-check of --apply
    assert not wt.exists()


def test_commits_after_the_merged_pull_request_keep_the_worktree(repo: Repo) -> None:
    wt = repo.worktree("later")
    merged_sha = head(wt)
    (wt / "more.txt").write_text("more\n", encoding="utf-8")
    sh(wt, "git", "add", "-A")
    sh(wt, "git", "commit", "-q", "-m", "after the merge")
    r = row(repo.cleaner(github=lambda _b: (5, merged_sha)).run(), wt)
    assert r.reason == "commits after the merged pull request #5"


def test_github_unreachable_keeps_the_worktree(repo: Repo) -> None:
    wt = repo.worktree("offline")
    r = row(repo.cleaner(apply=True, github=lambda _b: False).run(), wt)
    assert r.action == "keep" and "GitHub could not be reached" in r.reason
    assert wt.exists()


def test_unpushed_commits_keep_the_worktree(repo: Repo) -> None:
    # GitHub claims the local head was merged, but origin/<branch> does not contain it:
    # the remote check still refuses (defence in depth).
    wt = repo.worktree("unpushed")
    (wt / "local.txt").write_text("local only\n", encoding="utf-8")
    sh(wt, "git", "add", "-A")
    sh(wt, "git", "commit", "-q", "-m", "not pushed")
    r = row(repo.cleaner(apply=True, github=lambda _b: (9, head(wt))).run(), wt)
    assert (r.action, r.reason) == ("keep", "unpushed commits") and wt.exists()


def test_detached_worktree_on_origin_main_is_removable_elsewhere_kept(repo: Repo) -> None:
    on_main = repo.base / "detached-main"
    sh(repo.main, "git", "worktree", "add", "-q", "--detach", str(on_main), "origin/main")
    local = repo.base / "detached-local"
    sh(repo.main, "git", "worktree", "add", "-q", "--detach", str(local), "origin/main")
    (local / "x.txt").write_text("x\n", encoding="utf-8")
    sh(local, "git", "add", "-A")
    sh(local, "git", "commit", "-q", "-m", "local")
    rows = repo.cleaner().run()
    assert row(rows, on_main).action == "remove"
    assert row(rows, local).reason == "detached HEAD not in origin/main"


def test_process_working_directory_keeps_the_worktree(repo: Repo) -> None:
    wt = repo.worktree("busy", merge=True)
    shell = cw.Proc(pid=123, comm="bash", argv=["bash"], cwd=wt / "src")
    r = row(repo.cleaner(apply=True, processes=[shell]).run(), wt)
    assert (r.action, r.reason) == ("keep", "in use by process 123 (bash)") and wt.exists()


def test_open_file_keeps_the_worktree(repo: Repo) -> None:
    wt = repo.worktree("open-file", merge=True)
    editor = cw.Proc(pid=7, comm="vim", argv=["vim"], cwd=Path("/"), files=[wt / "README.md"])
    assert row(repo.cleaner(processes=[editor]).run(), wt).action == "keep"


def test_without_proc_every_worktree_is_kept(repo: Repo) -> None:
    wt = repo.worktree("no-proc", merge=True)
    r = row(repo.cleaner(apply=True, processes=None).run(), wt)
    assert r.reason == "running processes cannot be checked (no /proc)" and wt.exists()


def test_recently_active_worktree_is_kept_until_idle(repo: Repo) -> None:
    wt = repo.worktree("recent", merge=True)
    r = row(repo.cleaner(idle_hours=12).run(), wt)
    assert r.action == "keep" and r.reason.startswith("active")
    later = time.time() + 13 * 3600
    assert row(repo.cleaner(idle_hours=12, now=later).run(), wt).action == "remove"


@needs_proc
def test_real_process_in_worktree_is_detected(repo: Repo, sleeper: Any) -> None:
    wt = repo.worktree("live", merge=True)
    proc = sleeper(wt)
    processes = cw.scan_processes()
    assert processes is not None
    r = row(repo.cleaner(apply=True, processes=processes).run(), wt)
    assert r.action == "keep" and f"process {proc.pid}" in r.reason and wt.exists()


def test_scan_processes_without_proc_returns_none(tmp_path: Path) -> None:
    assert cw.scan_processes(tmp_path / "absent") is None


def test_missing_worktree_directory_is_pruned_only_with_apply(repo: Repo) -> None:
    wt = repo.worktree("gone", merge=True)
    shutil.rmtree(wt)
    lines: list[str] = []
    rows = repo.cleaner(out=lines.append).run()
    assert row(rows, wt).action == "keep" and any("gone" in line for line in lines)
    assert wt in listed(repo)
    repo.cleaner(apply=True).run()
    assert wt not in listed(repo)


def listed(repo: Repo) -> list[Path]:
    """Worktree paths known to git (git prints forward slashes, also on Windows)."""
    out = sh(repo.main, "git", "worktree", "list", "--porcelain")
    return [
        Path(line[len("worktree ") :]) for line in out.splitlines() if line.startswith("worktree ")
    ]


# ------------------------------------------------------------------------------------------
# Caches


def make_caches(tree: Path) -> list[Path]:
    paths = [
        tree / "target" / "debug" / "app",
        tree / "mutants.out" / "log",
        tree / "mutants.out.old" / "log",
        tree / ".pytest_cache" / "v",
        tree / "src" / "__pycache__" / "app.pyc",
    ]
    for p in paths:
        p.parent.mkdir(parents=True, exist_ok=True)
        p.write_bytes(b"0" * 4096)
    return [
        tree / "target",
        tree / "mutants.out",
        tree / "mutants.out.old",
        tree / ".pytest_cache",
        tree / "src" / "__pycache__",
    ]


def test_caches_need_the_flag_and_sources_survive(repo: Repo) -> None:
    wt = repo.worktree("kept", merge=False)  # kept: not merged
    (wt / "wip.txt").write_text("unsaved idea\n", encoding="utf-8")
    caches = make_caches(wt)
    rows = repo.cleaner(apply=True).run()
    for c in caches:
        assert row(rows, c).reason == "build cache: needs --caches" and c.exists()

    rows = repo.cleaner(apply=True, caches=True).run()
    for c in caches:
        assert row(rows, c).action == "removed" and not c.exists()
    assert (wt / "wip.txt").read_text(encoding="utf-8") == "unsaved idea\n"
    assert (wt / "src" / "app.py").exists() and (wt / ".git").exists()
    assert row(rows, wt).action == "keep"


def test_main_checkout_caches_are_removable_but_not_its_sources(repo: Repo) -> None:
    caches = make_caches(repo.main)
    repo.cleaner(apply=True, caches=True).run()
    assert not any(c.exists() for c in caches)
    assert (repo.main / "src" / "app.py").exists() and (repo.main / "README.md").exists()


def test_build_tool_in_the_tree_protects_its_caches(repo: Repo) -> None:
    wt = repo.worktree("building")
    caches = make_caches(wt)
    cargo = cw.Proc(pid=55, comm="cargo", argv=["cargo", "test"], cwd=wt)
    rows = repo.cleaner(apply=True, caches=True, processes=[cargo]).run()
    assert row(rows, caches[0]).reason == "in use by process 55 (cargo)"
    assert all(c.exists() for c in caches)


def test_cargo_target_dir_of_a_build_elsewhere_protects_the_cache(repo: Repo) -> None:
    wt = repo.worktree("shared-target")
    caches = make_caches(wt)
    cargo = cw.Proc(pid=8, comm="rustc", argv=["rustc"], cwd=Path("/"), target_dir=wt / "target")
    rows = repo.cleaner(apply=True, caches=True, processes=[cargo]).run()
    assert row(rows, caches[0]).action == "skip" and caches[0].exists()


def test_other_processes_do_not_block_caches(repo: Repo) -> None:
    wt = repo.worktree("idle-shell", merge=True)
    caches = make_caches(wt)
    shell = cw.Proc(pid=3, comm="bash", argv=["bash"], cwd=wt)
    rows = repo.cleaner(apply=True, caches=True, processes=[shell]).run()
    assert row(rows, caches[0]).action == "removed"
    assert row(rows, wt).reason.startswith("in use")  # the worktree itself stays


@needs_proc
def test_real_cargo_process_protects_caches(repo: Repo, sleeper: Any) -> None:
    wt = repo.worktree("real-cargo")
    caches = make_caches(wt)
    sleeper(wt, "cargo")
    processes = cw.scan_processes()
    assert processes is not None
    rows = repo.cleaner(apply=True, caches=True, processes=processes).run()
    assert row(rows, caches[0]).reason.startswith("in use by process")
    assert caches[0].exists()


def test_tracked_files_are_never_removed(repo: Repo) -> None:
    pyc = repo.main / "lib" / "__pycache__" / "keep.pyc"
    pyc.parent.mkdir(parents=True)
    pyc.write_bytes(b"tracked")
    sh(repo.main, "git", "add", "-f", str(pyc))
    sh(repo.main, "git", "commit", "-q", "-m", "tracked pycache")
    rows = repo.cleaner(apply=True, caches=True).run()
    assert row(rows, pyc.parent).reason == "refused: contains files tracked by git"
    assert pyc.exists()


@pytest.mark.parametrize("protected", ["docs", "specs", ".agents/journal"])
def test_protected_directories_are_never_touched(repo: Repo, protected: str) -> None:
    cache = repo.main / protected / "__pycache__"
    cache.mkdir(parents=True)
    (cache / "x.pyc").write_bytes(b"x")
    rows = repo.cleaner(apply=True, caches=True).run()
    assert row(rows, cache).reason == f"refused: under {protected}"
    assert (cache / "x.pyc").exists()
    assert (repo.main / ".agents/journal/j.md").exists()


def test_symlinked_cache_leaving_the_roots_is_refused(repo: Repo, tmp_path: Path) -> None:
    outside = tmp_path / "precious"
    outside.mkdir()
    (outside / "data.txt").write_text("keep me\n", encoding="utf-8")
    (repo.main / "target").symlink_to(outside, target_is_directory=True)
    (repo.main / "src" / "__pycache__").symlink_to(outside, target_is_directory=True)
    rows = repo.cleaner(apply=True, caches=True).run()
    assert row(rows, repo.main / "target").reason == "refused: symbolic link"
    assert not [r for r in rows if r.path == repo.main / "src" / "__pycache__"]
    assert (outside / "data.txt").read_text(encoding="utf-8") == "keep me\n"


def test_allow_list_refuses_anything_else(repo: Repo, tmp_path: Path) -> None:
    c = repo.cleaner()
    src = repo.main / "src"
    assert c.unsafe(src, repo.main) == "refused: not a cache directory name"
    assert c.unsafe(Path("target"), repo.main) == "refused: relative path"
    stray = tmp_path / "elsewhere" / "target"
    stray.mkdir(parents=True)
    assert c.unsafe(stray, repo.main) == "refused: outside the allowed roots"
    nested = repo.tmp / "deep" / "pytest-of-x"
    nested.mkdir(parents=True)
    assert c.unsafe(nested, None) == "refused: not a temporary cache directory"
    via_link = repo.base / "link"
    via_link.symlink_to(repo.main, target_is_directory=True)
    (repo.main / "target").mkdir()
    assert c.unsafe(via_link / "target", via_link) == (
        "refused: path passes through a symbolic link"
    )


def test_temporary_caches_follow_their_patterns_only(repo: Repo) -> None:
    mutants = repo.tmp / "cargo-mutants-wt-AbC123.tmp"
    pytest_tmp = repo.tmp / "pytest-of-someone"
    unrelated = [repo.tmp / "cargo-mutants-wt", repo.tmp / "important", repo.tmp / "target"]
    for d in [mutants, pytest_tmp, *unrelated]:
        d.mkdir()
        (d / "f").write_bytes(b"1" * 4096)
    rows = repo.cleaner(apply=True, caches=True).run()
    assert row(rows, mutants).action == "removed" and not mutants.exists()
    assert row(rows, pytest_tmp).action == "removed" and not pytest_tmp.exists()
    for d in unrelated:
        assert d.exists() and not [r for r in rows if r.path == d]


def test_running_tools_protect_temporary_caches(repo: Repo) -> None:
    mutants = repo.tmp / "cargo-mutants-wt-x.tmp"
    pytest_tmp = repo.tmp / "pytest-of-me"
    for d in (mutants, pytest_tmp):
        d.mkdir()
    procs = [
        cw.Proc(pid=1, comm="cargo-mutants", argv=["cargo-mutants", "mutants"], cwd=Path("/w")),
        cw.Proc(pid=2, comm="python3", argv=["/usr/bin/python3", "-m", "pytest"], cwd=Path("/w")),
    ]
    rows = repo.cleaner(apply=True, caches=True, processes=procs).run()
    assert row(rows, mutants).reason == "in use by process 1 (cargo-mutants)"
    assert row(rows, pytest_tmp).reason == "in use by process 2 (python3)"
    assert mutants.exists() and pytest_tmp.exists()


# ------------------------------------------------------------------------------------------
# GitHub lookup and command line


def fake_gh(tmp_path: Path, body: str, code: int = 0) -> str:
    script = tmp_path / "gh"
    script.write_text(f"#!/bin/sh\ncat <<'EOF'\n{body}\nEOF\nexit {code}\n", encoding="utf-8")
    script.chmod(0o755)
    return str(script)


@pytest.mark.skipif(os.name != "posix", reason="shell script stands in for gh")
def test_github_lookup(tmp_path: Path) -> None:
    merged = '[{"number": 12, "merged_at": "2026-11-09T10:00:00Z", "head": {"sha": "abc"}}]'
    closed = '[{"number": 13, "merged_at": null, "head": {"sha": "def"}}]'
    assert cw.github_lookup("o/r", fake_gh(tmp_path, merged))("b") == (12, "abc")
    assert cw.github_lookup("o/r", fake_gh(tmp_path, closed))("b") is None
    assert cw.github_lookup("o/r", fake_gh(tmp_path, "[]"))("b") is None
    assert cw.github_lookup("o/r", fake_gh(tmp_path, "boom", 1))("b") is False
    assert cw.github_lookup("o/r", fake_gh(tmp_path, "not json"))("b") is False
    assert cw.github_lookup("o/r", str(tmp_path / "missing-gh"))("b") is False
    assert cw.github_lookup(None)("b") is False


def test_origin_repo_parses_github_urls(repo: Repo) -> None:
    assert cw.origin_repo(repo.main) is None  # a local path is not GitHub
    sh(repo.main, "git", "remote", "set-url", "origin", "git@github.com:acme/tool.git")
    assert cw.origin_repo(repo.main) == "acme/tool"
    sh(repo.main, "git", "remote", "set-url", "origin", "https://github.com/acme/tool")
    assert cw.origin_repo(repo.main) == "acme/tool"


def test_command_line_dry_run_removes_nothing(
    repo: Repo, capsys: pytest.CaptureFixture[str]
) -> None:
    wt = repo.worktree("cli", merge=True)
    caches = make_caches(wt)
    argv = ["--repo", str(wt), "--offline", "--tmp-root", str(repo.tmp), "--idle-hours", "0"]
    assert cw.main([*argv, "--caches"]) == 0
    out = capsys.readouterr().out
    assert "Dry run: nothing was removed." in out and "Would free:" in out
    assert str(repo.main) in out and "main checkout" in out
    assert wt.exists() and all(c.exists() for c in caches)


def test_table_totals_only_what_is_removed() -> None:
    rows = [
        cw.Row(Path("/a"), "worktree", "remove", "merged into origin/main", 2 * 1024**3),
        cw.Row(Path("/b"), "worktree", "keep", "locked", 5 * 1024**3),
    ]
    text = cw.table(rows, apply=False)
    assert "Would free: 2.0 GiB." in text and "locked" in text
    assert "Freed: 2.0 GiB." in cw.table(rows, apply=True)


def test_unrelated_tools_do_not_protect_temporary_caches(repo: Repo) -> None:
    mutants = repo.tmp / "cargo-mutants-wt-y.tmp"
    pytest_tmp = repo.tmp / "pytest-of-me"
    for d in (mutants, pytest_tmp):
        d.mkdir()
    procs = [
        cw.Proc(pid=4, comm="cargo", argv=["cargo", "test"], cwd=Path("/w")),
        cw.Proc(pid=5, comm="rustdoc", argv=["rustdoc"], cwd=Path("/w")),
    ]
    rows = repo.cleaner(apply=True, caches=True, processes=procs).run()
    assert row(rows, mutants).action == "removed" and row(rows, pytest_tmp).action == "removed"


def test_process_kinds() -> None:
    uv = cw.Proc(pid=1, comm="uv", argv=["uv", "run", "pytest", "-q"], cwd=None)
    module = cw.Proc(pid=2, comm="python3", argv=["python3", "-m", "pytest"], cwd=None)
    mutants = cw.Proc(pid=3, comm="cargo", argv=["cargo", "mutants"], cwd=None)
    shell = cw.Proc(pid=4, comm="bash", argv=["bash", "-c", "ls"], cwd=None)
    assert uv.is_pytest() and module.is_pytest() and uv.is_build_tool()
    assert mutants.is_cargo_mutants() and mutants.is_build_tool() and not mutants.is_pytest()
    assert not shell.is_build_tool() and not shell.is_pytest()
