"""Tests of scripts/pr_queue.py: states, staleness, dry run and update of the queue (#320)."""

from __future__ import annotations

import datetime as dt
from typing import Any

import pytest
from conftest import load_script

pq = load_script("pr_queue")

NOW = dt.datetime(2026, 11, 17, 12, tzinfo=dt.UTC)
REPO = "o/r"


def pull(number: int, *, dirty: bool = False, fork: bool = False, updated: str = "2026-11-16"):
    base_repo = {"full_name": REPO}
    return {
        "number": number,
        "title": f"feat(x): pr {number}",
        "draft": False,
        "mergeable_state": "dirty" if dirty else "clean",
        "updated_at": f"{updated}T12:00:00+00:00",
        "head": {
            "ref": f"feat/{number}-x",
            "sha": f"sha{number}",
            "repo": {"full_name": "someone/r" if fork else REPO},
        },
        "base": {"ref": "main", "repo": base_repo},
    }


class FakeGitHub:
    def __init__(self, pulls, behind, failures, drafts=()):
        self.pulls = {p["number"]: p for p in pulls}
        self.behind, self.failures, self.drafts = behind, failures, set(drafts)
        self.calls: list[tuple[str, str, dict[str, str]]] = []

    def __call__(self, path: str, method: str = "GET", **fields: str) -> Any:
        self.calls.append((method, path, fields))
        if path.startswith(f"repos/{REPO}/pulls?"):
            return [{"number": n, "draft": n in self.drafts} for n in self.pulls]
        if path.endswith("/update-branch"):
            return {"message": "Updating pull request branch."}
        if path.startswith(f"repos/{REPO}/pulls/"):
            return self.pulls[int(path.rsplit("/", 1)[1])]
        if "/compare/" in path:
            sha = path.split("/compare/")[1].split("...")[0]
            return {"ahead_by": self.behind.get(sha, 0)}
        if "/check-runs" in path:
            sha = path.split("/commits/")[1].split("/")[0]
            names = self.failures.get(sha, [])
            runs = [{"name": n, "conclusion": "failure"} for n in names]
            return {"check_runs": [*runs, {"name": "ok", "conclusion": "success"}]}
        raise AssertionError(path)


@pytest.fixture
def github(monkeypatch: pytest.MonkeyPatch) -> FakeGitHub:
    fake = FakeGitHub(
        pulls=[
            pull(1),
            pull(2, dirty=True, updated="2026-11-01"),
            pull(3, updated="2026-11-10"),
            pull(4),
            pull(5, fork=True),
            pull(6),
        ],
        behind={"sha2": 4, "sha3": 2, "sha4": 1, "sha5": 3},
        failures={"sha3": ["Python 3.12", "Python 3.12"], "sha1": []},
        drafts={6},
    )
    monkeypatch.setattr(pq, "_gh", fake)
    return fake


def test_classify_order() -> None:
    assert pq.classify({"mergeable_state": "dirty"}, 3, ["x"]) == "conflict"
    assert pq.classify({"mergeable_state": "clean"}, 3, ["x"]) == "failing"
    assert pq.classify({"mergeable_state": "unstable"}, 3, []) == "behind"
    assert pq.classify({"mergeable_state": "clean"}, 0, []) == "ready"


def test_queue_states_staleness_and_drafts(github: FakeGitHub) -> None:
    rows = {r["number"]: r for r in pq.queue(REPO, NOW, stale_days=3)}
    assert sorted(rows) == [1, 2, 3, 4, 5]  # the draft is left out
    assert {n: r["state"] for n, r in rows.items()} == {
        1: "ready",
        2: "conflict",
        3: "failing",
        4: "behind",
        5: "behind",
    }
    assert rows[3]["failed_checks"] == ["Python 3.12"]  # each name once
    assert [n for n, r in rows.items() if r["stale"]] == [2, 3]
    assert rows[5]["fork"] and not rows[4]["fork"]
    assert [r["number"] for r in pq.blocked(list(rows.values()))] == [2, 3]


def test_render(github: FakeGitHub) -> None:
    text = pq.render(pq.queue(REPO, NOW, 3))
    assert "#2     conflict" in text and "(stale)" in text
    assert "failed: Python 3.12" in text
    assert "5 open: conflict 1, failing 1, behind 2, ready 1" in text
    assert pq.render([]) == "No open pull request."


def test_update_is_a_dry_run_without_apply(github: FakeGitHub) -> None:
    log = pq.update(REPO, pq.queue(REPO, NOW, 3), None, apply=False)
    assert "#4: would merge main (1 commits); --apply to do it" in log
    assert "#5: from a fork, not updated" in log
    assert "#2: conflict, not updated" in log
    assert not [c for c in github.calls if c[0] == "PUT"]


def test_update_applies_only_to_behind_with_the_head_it_read(github: FakeGitHub) -> None:
    rows = pq.queue(REPO, NOW, 3)
    log = pq.update(REPO, rows, None, apply=True)
    puts = [c for c in github.calls if c[0] == "PUT"]
    assert puts == [("PUT", f"repos/{REPO}/pulls/4/update-branch", {"expected_head_sha": "sha4"})]
    assert "#4: main merged into feat/4-x; CI runs again" in log


def test_update_one_pull_request(github: FakeGitHub) -> None:
    log = pq.update(REPO, pq.queue(REPO, NOW, 3), 2, apply=True)
    assert log == ["#2: conflict, not updated"]


def test_fail_on_stale_exit_code(
    github: FakeGitHub, capsys: pytest.CaptureFixture[str], monkeypatch: pytest.MonkeyPatch
) -> None:
    monkeypatch.setattr(pq, "now", lambda: NOW)
    assert pq.main(["list", "--repo", REPO]) == 0
    assert pq.main(["list", "--repo", REPO, "--fail-on-stale"]) == 1
    assert pq.main(["list", "--repo", REPO, "--stale-days", "60", "--fail-on-stale"]) == 0
    capsys.readouterr()


def test_origin_repo(monkeypatch: pytest.MonkeyPatch) -> None:
    class Done:
        stdout = "git@github.com:salvo8519/fairmark.git\n"

    monkeypatch.setattr(pq.subprocess, "run", lambda *a, **k: Done())
    assert pq.origin_repo() == "salvo8519/fairmark"
    Done.stdout = "https://github.com/salvo8519/fairmark\n"
    assert pq.origin_repo() == "salvo8519/fairmark"
