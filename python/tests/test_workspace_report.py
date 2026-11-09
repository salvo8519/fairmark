"""scripts/workspace_report.py: read-only report, tested on temporary git repositories only."""

from __future__ import annotations

import importlib.util
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/workspace_report.py"
_spec = importlib.util.spec_from_file_location("workspace_report", SCRIPT)
assert _spec and _spec.loader
wr = importlib.util.module_from_spec(_spec)
sys.modules["workspace_report"] = wr  # dataclasses resolve types through sys.modules
_spec.loader.exec_module(wr)

pytestmark = pytest.mark.skipif(shutil.which("git") is None, reason="git is required")


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
            ".gitignore": "target/\n.venv/\n__pycache__/\nmutants.out*/\n.pytest_cache/\n.env\n",
        }.items():
            (self.main / path).parent.mkdir(parents=True, exist_ok=True)
            (self.main / path).write_text(text, encoding="utf-8")
        sh(self.main, "git", "add", "-A")
        sh(self.main, "git", "commit", "-q", "-m", "init")
        sh(self.main, "git", "push", "-q", "-u", "origin", "main")

    def worktree(self, name: str, *, push: bool = True, merge: bool = False) -> Path:
        path = self.base / name
        sh(self.main, "git", "worktree", "add", "-q", "-b", name, str(path), "origin/main")
        (path / f"{name}.txt").write_text(name, encoding="utf-8")
        sh(path, "git", "add", "-A")
        sh(path, "git", "commit", "-q", "-m", name)
        if push:
            sh(path, "git", "push", "-q", "origin", name)
        if merge:
            sh(path, "git", "push", "-q", "origin", "HEAD:main")
        sh(self.main, "git", "fetch", "-q", "origin")
        return path

    def reporter(self, github: Any = None) -> Any:
        lookup = github if github is not None else (lambda _b, _h: wr.Merged("no", "stub"))
        return wr.Reporter(self.main, github=lookup, tmp_root=self.tmp)


def head(path: Path) -> str:
    return sh(path, "git", "rev-parse", "HEAD")


def report_of(reports: list[Any], path: Path) -> Any:
    matches = [r for r in reports if r.tree.path == path]
    assert len(matches) == 1, (path, reports)
    return matches[0]


def make_caches(tree: Path) -> None:
    for d in ("target/debug", ".venv/lib", "src/__pycache__", ".pytest_cache", "mutants.out"):
        (tree / d).mkdir(parents=True, exist_ok=True)
        (tree / d / "blob").write_bytes(b"x" * 8192)


def snapshot(root: Path) -> dict[str, tuple[int, int, int]]:
    """Every path below ``root`` with its size, modification time and mode."""
    out = {}
    for dirpath, dirnames, filenames in os.walk(root):
        for name in dirnames + filenames:
            p = Path(dirpath) / name
            st = p.lstat()
            out[str(p.relative_to(root))] = (st.st_size, st.st_mtime_ns, st.st_mode)
    return out


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


@pytest.fixture(autouse=True)
def fresh_git_env(monkeypatch: pytest.MonkeyPatch) -> None:
    """The sanitised git environment is built once per run: rebuild it for every test."""
    monkeypatch.setattr(wr, "_GIT_ENV", None, raising=False)


# ------------------------------------------------------------------------------------------
# Read-only by construction


FORBIDDEN = {
    "shutil": r"\bshutil\b",
    "rmtree": r"\brmtree\b",
    "os.remove": r"\bos\.remove\b",
    "unlink": r"\bunlink\b",
    "rmdir": r"\brmdir\b",
    "os.rename / os.replace": r"\bos\.(rename|replace|truncate)\b",
    "git worktree remove": r"worktree[\"',\s]+remove",
    "git worktree prune": r"\bprune\b",
    "--apply": r"--apply",
    "--caches": r"--caches",
}


def test_script_source_has_no_deletion_capability() -> None:
    source = SCRIPT.read_text(encoding="utf-8")
    found = [name for name, pattern in FORBIDDEN.items() if re.search(pattern, source)]
    assert found == [], f"workspace_report.py must stay read-only; found: {found}"


@pytest.mark.parametrize(
    "snippet",
    ["shutil.rmtree(p)", "os.remove(p)", "p.unlink()", 'git(m, "worktree", "remove", x)'],
)
def test_forbidden_patterns_catch_deletion_calls(snippet: str) -> None:
    assert any(re.search(pattern, snippet) for pattern in FORBIDDEN.values())


def test_report_changes_nothing_on_disk(repo: Repo) -> None:
    merged = repo.worktree("done", merge=True)
    dirty = repo.worktree("dirty")
    make_caches(merged)
    (dirty / "README.md").write_text("changed\n", encoding="utf-8")
    (repo.tmp / "pytest-of-me").mkdir()
    before = snapshot(repo.base)
    worktrees = sh(repo.main, "git", "worktree", "list", "--porcelain")
    assert wr.main(["--repo", str(repo.main), "--offline", "--tmp-root", str(repo.tmp)]) == 0
    assert snapshot(repo.base) == before
    assert sh(repo.main, "git", "worktree", "list", "--porcelain") == worktrees


def test_command_line_has_no_destructive_option() -> None:
    with pytest.raises(SystemExit):
        wr.main(["--apply"])


# ------------------------------------------------------------------------------------------
# Worktree list


def test_parse_worktree_list_handles_nul_records_and_odd_paths() -> None:
    raw = (
        "worktree /r/main\0HEAD aaa\0branch refs/heads/main\0\0"
        "worktree /r/with space\nand newline\0HEAD bbb\0detached\0\0"
        "worktree /r/locked\0HEAD ccc\0branch refs/heads/feat/x\0locked reason\0\0"
        "worktree /r/gone\0HEAD ddd\0branch refs/heads/old\0prunable gitdir file points\0\0"
    )
    trees = wr.parse_worktree_list(raw)
    assert [t.path for t in trees] == [
        Path("/r/main"), Path("/r/with space\nand newline"), Path("/r/locked"), Path("/r/gone")
    ]  # fmt: skip
    assert trees[0].main and not any(t.main for t in trees[1:])
    assert trees[1].branch is None and trees[1].detached
    assert trees[2].branch == "feat/x" and trees[2].locked
    assert trees[3].prunable


def test_list_worktrees_reads_a_real_repository(repo: Repo) -> None:
    wt = repo.worktree("feat")
    trees = wr.list_worktrees(wt)
    assert [t.path for t in trees] == [repo.main, wt]
    assert trees[1].branch == "feat" and trees[1].head == head(wt)


def test_directory_of_another_repository_is_not_queried(repo: Repo) -> None:
    wt = repo.worktree("swapped")
    (wt / ".git").unlink()  # the directory is replaced by an unrelated repository
    sh(wt, "git", "init", "-q")
    called: list[str] = []
    reporter = repo.reporter(lambda b, _h: called.append(b) or wr.Merged("yes", "x"))
    r = report_of(reporter.run(), wt)
    assert r.verified == "not a worktree of this repository"
    assert r.size is not None and r.status.uncommitted is None and called == []
    text = wr.render([r], [], [], 0.0)
    assert "not a worktree of this repository; other details unknown" in text


def test_missing_directory_is_reported_with_unknown_size(repo: Repo) -> None:
    wt = repo.worktree("gone")
    shutil.rmtree(wt)
    r = report_of(repo.reporter().run(), wt)
    assert r.verified == "directory missing" and r.size is None
    assert "Size                 unknown" in wr.render([r], [], [], 0.0)


# ------------------------------------------------------------------------------------------
# Merged into main


def test_commit_reachable_from_origin_main_is_merged(repo: Repo) -> None:
    wt = repo.worktree("done", merge=True)
    r = report_of(repo.reporter().run(), wt)
    assert r.merged == wr.Merged("yes", "reachable from origin/main")


def test_unmerged_branch_asks_github_with_branch_and_head(repo: Repo) -> None:
    wt = repo.worktree("open")
    calls: list[tuple[str, str]] = []

    def lookup(branch: str, sha: str) -> Any:
        calls.append((branch, sha))
        return wr.Merged("yes", "pull request #7 merged")

    r = report_of(repo.reporter(lookup).run(), wt)
    assert calls == [("open", head(wt))]
    assert r.merged.state == "yes"


def test_detached_head_not_in_main_is_unknown(repo: Repo) -> None:
    wt = repo.worktree("det")
    sh(wt, "git", "switch", "-q", "--detach")
    r = report_of(repo.reporter().run(), wt)
    assert r.merged == wr.Merged("unknown", "detached HEAD not in origin/main")


def test_offline_is_unknown_never_guessed(repo: Repo) -> None:
    wt = repo.worktree("open")
    r = report_of(repo.reporter(wr.offline_lookup).run(), wt)
    assert r.merged.state == "unknown" and "--offline" in r.merged.detail


def test_local_main_ahead_of_origin_is_not_merged(repo: Repo) -> None:
    (repo.main / "local.txt").write_text("x", encoding="utf-8")
    sh(repo.main, "git", "add", "-A")
    sh(repo.main, "git", "commit", "-q", "-m", "local")
    r = report_of(repo.reporter().run(), repo.main)
    assert r.merged == wr.Merged("no", "local commits not in origin/main")


def test_missing_origin_main_is_unknown(repo: Repo) -> None:
    wt = repo.worktree("open")
    sh(repo.main, "git", "update-ref", "-d", "refs/remotes/origin/main")
    r = report_of(repo.reporter().run(), wt)
    assert r.merged == wr.Merged("unknown", "origin/main not found")


def pull(**over: Any) -> dict[str, Any]:
    base: dict[str, Any] = {
        "number": 12,
        "merged_at": "2026-11-09T10:00:00Z",
        "base": {"ref": "main"},
        "head": {"ref": "feat/x", "sha": "abc", "repo": {"full_name": "Acme/Tool"}},
    }
    for key, value in over.items():
        if key in ("ref", "sha", "repo"):
            base["head"][key] = value
        elif key == "base_ref":
            base["base"]["ref"] = value
        else:
            base[key] = value
    return base


@pytest.mark.parametrize(
    ("pulls", "state", "detail"),
    [
        ([pull()], "yes", "pull request #12 merged"),
        ([pull(merged_at=None)], "no", "no merged pull request"),
        ([pull(merged_at="")], "no", "no merged pull request"),
        ([pull(base_ref="develop")], "no", "no merged pull request"),
        ([pull(ref="feat/other")], "no", "no merged pull request"),
        ([pull(repo={"full_name": "fork/tool"})], "no", "no merged pull request"),
        ([pull(repo=None)], "no", "no merged pull request"),
        ([pull(sha="older")], "no", "commits after merged pull request #12"),
        ([pull(sha="older", number=3), pull(number=4)], "yes", "pull request #4 merged"),
        ([], "no", "no merged pull request"),
        ({"message": "Not Found"}, "unknown", "unexpected GitHub response"),
        ([pull(), {"number": 1}], "unknown", "unexpected GitHub response"),
        ([pull(number="12")], "unknown", "unexpected GitHub response"),
        ([pull(number=True)], "unknown", "unexpected GitHub response"),
        ([pull(head=None)], "unknown", "unexpected GitHub response"),
        ([pull(merged_at=5)], "unknown", "unexpected GitHub response"),
        (["text"], "unknown", "unexpected GitHub response"),
    ],
)
def test_classify_pulls(pulls: Any, state: str, detail: str) -> None:
    merged = wr.classify_pulls(pulls, "acme/tool", "feat/x", "abc")
    assert merged.state == state and detail in merged.detail


def fake_gh(tmp_path: Path, body: str, code: int = 0) -> str:
    script = tmp_path / "gh"
    log = tmp_path / "gh.args"
    script.write_text(
        f"#!/bin/sh\nprintf '%s\\n' \"$@\" > '{log}'\ncat <<'EOF'\n{body}\nEOF\nexit {code}\n",
        encoding="utf-8",
    )
    script.chmod(0o755)
    return str(script)


@pytest.mark.skipif(os.name != "posix", reason="shell script stands in for gh")
def test_github_lookup(tmp_path: Path) -> None:
    body = json.dumps([pull(ref="feat/a&b#c", repo={"full_name": "o/r"})])
    lookup = wr.github_lookup("o/r", fake_gh(tmp_path, body))
    assert lookup("feat/a&b#c", "abc") == wr.Merged("yes", "pull request #12 merged")
    args = (tmp_path / "gh.args").read_text(encoding="utf-8").split("\n")
    assert args[0] == "api"
    assert args[1] == (
        "repos/o/r/pulls?head=o%3Afeat%2Fa%26b%23c&base=main&state=closed&per_page=100"
    )
    failed = wr.Merged("unknown", "GitHub lookup failed")
    assert wr.github_lookup("o/r", fake_gh(tmp_path, "boom", 1))("b", "x") == failed
    assert wr.github_lookup("o/r", fake_gh(tmp_path, "not json"))("b", "x") == failed
    assert wr.github_lookup("o/r", str(tmp_path / "missing-gh"))("b", "x") == failed
    assert wr.github_lookup(None)("b", "x").state == "unknown"


def test_origin_repo_parses_github_urls(repo: Repo) -> None:
    assert wr.origin_repo(repo.main) is None  # a local path is not GitHub
    sh(repo.main, "git", "remote", "set-url", "origin", "git@github.com:acme/tool.git")
    assert wr.origin_repo(repo.main) == "acme/tool"
    sh(repo.main, "git", "remote", "set-url", "origin", "https://github.com/acme/tool")
    assert wr.origin_repo(repo.main) == "acme/tool"


# ------------------------------------------------------------------------------------------
# Status, caches, activity


def test_status_counts_uncommitted_untracked_and_ignored_non_cache(repo: Repo) -> None:
    wt = repo.worktree("work")
    (wt / "README.md").write_text("changed\n", encoding="utf-8")
    (wt / "staged.txt").write_text("s", encoding="utf-8")
    sh(wt, "git", "add", "staged.txt")
    sh(wt, "git", "mv", "src/app.py", "src/moved.py")
    (wt / "new1.txt").write_text("n", encoding="utf-8")
    (wt / "dir").mkdir()
    (wt / "dir" / "new2.txt").write_text("n", encoding="utf-8")
    (wt / ".env").write_text("secret", encoding="utf-8")
    make_caches(wt)
    status = report_of(repo.reporter().run(), wt).status
    assert status.uncommitted == 3  # modified, added, renamed
    assert status.untracked == 2
    assert status.ignored == [".env"]  # caches are not counted


def test_ignored_directory_is_listed_once(repo: Repo) -> None:
    wt = repo.worktree("tooling")
    tool_cache = wt / ".cache"
    tool_cache.mkdir()
    (tool_cache / ".gitignore").write_text("*\n", encoding="utf-8")  # as ruff and mypy do
    for n in range(5):
        (tool_cache / f"entry{n}").write_text("x", encoding="utf-8")
    status = report_of(repo.reporter().run(), wt).status
    assert status.ignored == [".cache/"]


def test_disk_percentage_matches_df() -> None:
    disks = [(Path("/d"), 100 * 1024**3, 30 * 1024**3, 10 * 1024**3)]  # reserved space: 60 GiB
    text = wr.render([], [], disks, 0.0)
    assert "used 30.0 GiB (75 percent), available 10.0 GiB" in text


def test_clean_worktree_reports_zero(repo: Repo) -> None:
    wt = repo.worktree("clean")
    s = report_of(repo.reporter().run(), wt).status
    assert (s.uncommitted, s.untracked, s.ignored) == (0, 0, [])


def test_caches_are_measured_by_kind(repo: Repo) -> None:
    wt = repo.worktree("built")
    make_caches(wt)
    r = report_of(repo.reporter().run(), wt)
    assert set(r.caches) == {"target/", ".venv/", "__pycache__/", ".pytest_cache/", "mutants.out*/"}
    assert all(size >= 8192 for size in r.caches.values())
    assert r.size is not None and r.size >= sum(r.caches.values())
    text = wr.render([r], [], [], 0.0)
    assert "cache rebuildable" in text and "target/" in text


def test_nested_worktree_is_not_counted_twice(repo: Repo) -> None:
    nested = repo.main / "nested"
    sh(repo.main, "git", "worktree", "add", "-q", "-b", "nested", str(nested), "origin/main")
    make_caches(nested)
    reports = repo.reporter().run()
    outer = report_of(reports, repo.main)
    assert "target/" not in outer.caches and "__pycache__/" not in outer.caches


def test_last_activity_and_temporary_caches(repo: Repo) -> None:
    wt = repo.worktree("act")
    os.utime(wt / "act.txt", (1_000_000_000, 2_000_000_000))
    r = report_of(repo.reporter().run(), wt)
    assert r.last_activity is not None and r.last_activity >= 2_000_000_000
    (repo.tmp / "cargo-mutants-x.tmp").mkdir()
    (repo.tmp / "pytest-of-me").mkdir()
    (repo.tmp / "unrelated").mkdir()
    names = [p.name for p, _ in repo.reporter().tmp_caches()]
    assert names == ["cargo-mutants-x.tmp", "pytest-of-me"]


def test_render_prints_unknown_for_missing_values() -> None:
    tree = wr.Worktree(Path("/w"), None, None, False, False, False, False)
    r = wr.Report(tree, None, None)
    text = wr.render([r], [], [(Path("/w"), None, None, None)], 0.0)
    for line in ("Branch               unknown", "HEAD                 unknown",
                 "Merged into main     unknown", "Uncommitted          unknown",
                 "Last activity        unknown", "total size unknown", "/w: unknown"):  # fmt: skip
        assert line in text


def test_filesystem_usage_once_per_device(tmp_path: Path) -> None:
    rows = wr.filesystem_usage([tmp_path, tmp_path, tmp_path / "missing"])
    assert len(rows) == 1
    if hasattr(os, "statvfs"):
        _, size, used, available = rows[0]
        assert size and used is not None and available is not None and used <= size


# ------------------------------------------------------------------------------------------
# Command line


def test_command_line_report(repo: Repo, capsys: pytest.CaptureFixture[str]) -> None:
    wt = repo.worktree("cli")
    make_caches(wt)
    assert wr.main(["--repo", str(wt), "--offline", "--tmp-root", str(repo.tmp)]) == 0
    out = capsys.readouterr().out
    assert f"Worktree {repo.main}  (main checkout)" in out and f"Worktree {wt}" in out
    assert "Merged into main     unknown (not in origin/main; GitHub not asked" in out
    assert "Disk usage" in out and "Read-only report: nothing was changed." in out


def test_command_line_outside_a_repository(tmp_path: Path) -> None:
    assert wr.main(["--repo", str(tmp_path), "--offline", "--no-tmp"]) == 2


# ------------------------------------------------------------------------------------------
# Git environment (security review of PR #299)


def test_git_environment_cannot_redirect_the_script(
    repo: Repo, tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    (tmp_path / "victim-base").mkdir()
    victim = Repo((tmp_path / "victim-base").resolve())
    victim.worktree("victim-done", merge=True)
    monkeypatch.setenv("GIT_DIR", str(victim.main / ".git"))
    monkeypatch.setenv("GIT_WORK_TREE", str(victim.main))
    monkeypatch.setenv("GIT_CONFIG_PARAMETERS", "'core.bare'='true'")
    reports = repo.reporter().run()
    assert [r.tree.path for r in reports] == [repo.main]


def test_sanitised_environment_drops_redirecting_variables() -> None:
    names = [
        "GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_COMMON_DIR", "GIT_OBJECT_DIRECTORY",
        "GIT_ALTERNATE_OBJECT_DIRECTORIES", "GIT_NAMESPACE", "GIT_CEILING_DIRECTORIES",
        "GIT_CONFIG", "GIT_CONFIG_GLOBAL", "GIT_CONFIG_SYSTEM", "GIT_CONFIG_NOSYSTEM",
        "GIT_CONFIG_PARAMETERS", "GIT_CONFIG_COUNT", "GIT_CONFIG_KEY_0", "GIT_CONFIG_VALUE_0",
    ]  # fmt: skip
    env = wr.sanitized_env({**{n: "x" for n in names}, "PATH": "/bin", "HOME": "/h"})
    assert env == {"PATH": "/bin", "HOME": "/h"}
