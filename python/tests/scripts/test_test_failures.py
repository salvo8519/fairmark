"""scripts/test_failures.py and the quarantine in python/tests/conftest.py (issue #320)."""

from __future__ import annotations

import datetime as dt
import importlib.util
import json
from pathlib import Path
from types import SimpleNamespace

import pytest
from conftest import ROOT, load_script, write

tf = load_script("test_failures")

MODULE = "python.tests.scripts.test_pr_queue"
FILE = "python/tests/scripts/test_pr_queue.py"


def junit(cases: list[tuple[str, str, str]]) -> str:
    """cases: (classname, name, outcome) with outcome pass, fail, error or skip."""
    body = []
    for classname, name, outcome in cases:
        inner = {
            "pass": "",
            "fail": '<failure message="assert 1 == 2">trace</failure>',
            "error": '<error message="fixture broke">trace</error>',
            "skip": '<skipped message="no oracle"/>',
        }[outcome]
        body.append(f'<testcase classname="{classname}" name="{name}">{inner}</testcase>')
    return f'<?xml version="1.0"?><testsuites><testsuite>{"".join(body)}</testsuite></testsuites>'


def test_node_id_maps_dotted_module_and_class_to_the_file() -> None:
    plain = tf.ET.fromstring(f'<testcase classname="{MODULE}" name="test_render"/>')
    nested = tf.ET.fromstring(f'<testcase classname="{MODULE}.TestX" name="test_y[a-b]"/>')
    unknown = tf.ET.fromstring('<testcase classname="nowhere.mod" name="t"/>')
    assert tf.node_id(plain) == f"{FILE}::test_render"
    assert tf.node_id(nested) == f"{FILE}::TestX::test_y[a-b]"
    assert tf.node_id(unknown) == "nowhere.mod::t"


def test_collect_and_classify(tmp_path: Path) -> None:
    runs = {
        "101/junit-ubuntu-latest-3.12/pytest.xml": [
            (MODULE, "flaky", "fail"),
            (MODULE, "broken", "error"),
            (MODULE, "fine", "pass"),
            (MODULE, "skipped", "skip"),
        ],
        "102/junit-macos-latest-3.15/pytest.xml": [
            (MODULE, "flaky", "pass"),
            (MODULE, "broken", "fail"),
            (MODULE, "fine", "pass"),
        ],
    }
    write(tmp_path, {path: junit(cases) for path, cases in runs.items()})
    write(tmp_path, {"103/junit-x/pytest.xml": "<not xml"})  # a truncated file is ignored
    stats = tf.collect(tmp_path)
    assert f"{FILE}::skipped" not in stats
    assert stats[f"{FILE}::fine"] == {"runs": 2, "failures": 0, "failed_in": []}
    flaky = stats[f"{FILE}::flaky"]
    assert flaky["failed_in"] == ["101/junit-ubuntu-latest-3.12"]
    assert flaky["message"] == "assert 1 == 2"
    kinds = tf.classify(stats)
    assert [r["test"] for r in kinds["intermittent"]] == [f"{FILE}::flaky"]
    assert kinds["intermittent"][0]["rate"] == 0.5
    assert [r["test"] for r in kinds["always_failing"]] == [f"{FILE}::broken"]


def entry(**over: object) -> dict[str, object]:
    base = {
        "test": f"{FILE}::test_render",
        "issue": 320,
        "reason": "timing on macOS",
        "until": dt.date(2026, 12, 31),
    }
    return {**base, **over}


def test_quarantine_problems() -> None:
    today = dt.date(2026, 11, 17)
    assert tf.quarantine_problems([entry()], today) == []
    assert tf.quarantine_problems([entry(until=today)], today) == []
    problems = tf.quarantine_problems(
        [
            entry(until=dt.date(2026, 11, 16)),
            entry(issue="320"),
            entry(reason=" "),
            entry(until="soon"),
            entry(test="python/tests/missing.py::t"),
            entry(test="no-node-id"),
            "not a mapping",
        ],
        today,
    )
    assert problems == [
        f"{FILE}::test_render: quarantine expired on 2026-11-16: fix the test or renew it",
        f"{FILE}::test_render: 'issue' must be the number of the issue that tracks the fix",
        f"{FILE}::test_render: 'reason' is required",
        f"{FILE}::test_render: 'until' must be a date (YYYY-MM-DD)",
        "python/tests/missing.py::t: no such test file",
        "no-node-id: 'test' must be a pytest node id (file.py::test_name)",
        "entry 7: 'test' must be a pytest node id (file.py::test_name)",
    ]


def test_the_committed_quarantine_is_valid() -> None:
    assert tf.main(["quarantine-check"]) == 0


def test_report_exit_code_honours_the_quarantine(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch, capsys: pytest.CaptureFixture[str]
) -> None:
    flaky = {"runs": 2, "failures": 1, "failed_in": ["1/a"], "message": "m"}
    fine = {"runs": 2, "failures": 0, "failed_in": []}
    stats = tmp_path / "stats.json"
    stats.write_text(json.dumps({f"{FILE}::flaky": flaky, f"{FILE}::fine": fine}))
    monkeypatch.setattr(tf, "load_quarantine", lambda: [])
    assert tf.main(["report", str(stats)]) == 1
    monkeypatch.setattr(tf, "load_quarantine", lambda: [{"test": f"{FILE}::flaky"}])
    assert tf.main(["report", str(stats)]) == 0
    out = capsys.readouterr().out
    assert f"{FILE}::flaky: 1/2 (quarantined)" in out
    assert "2 tests, up to 2 runs each." in out


def test_collect_writes_the_statistics_file(tmp_path: Path) -> None:
    write(tmp_path, {"r/1/pytest.xml": junit([(MODULE, "a", "pass")])})
    out = tmp_path / "stats.json"
    assert tf.main(["collect", str(tmp_path / "r"), "--out", str(out)]) == 0
    assert json.loads(out.read_text())[f"{FILE}::a"]["runs"] == 1


def load_quarantine_conftest():
    spec = importlib.util.spec_from_file_location(
        "quarantine_conftest", ROOT / "python" / "tests" / "conftest.py"
    )
    module = importlib.util.module_from_spec(spec)  # type: ignore[arg-type]
    spec.loader.exec_module(module)  # type: ignore[union-attr]
    return module


class Item:
    def __init__(self, nodeid: str) -> None:
        self.nodeid = nodeid
        self.path = ROOT / nodeid.split("::")[0]
        self.markers: list[pytest.MarkDecorator] = []

    def add_marker(self, marker: pytest.MarkDecorator) -> None:
        self.markers.append(marker)


def test_conftest_marks_only_quarantined_tests_as_non_strict_xfail(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    conf = load_quarantine_conftest()
    listed = tmp_path / "quarantine.yaml"
    listed.write_text(
        f"quarantine:\n  - test: {FILE}::test_render\n    issue: 320\n    reason: timing\n"
        "    until: 2026-12-31\n",
        encoding="utf-8",
    )
    monkeypatch.setattr(conf, "QUARANTINE", listed)
    items = [Item(f"{FILE}::test_render"), Item(f"{FILE}::test_classify_order")]
    conf.pytest_collection_modifyitems(SimpleNamespace(rootpath=ROOT), items)  # type: ignore[arg-type]
    marked, untouched = items
    assert untouched.markers == []
    (mark,) = marked.markers
    assert mark.name == "xfail"
    assert mark.kwargs == {"reason": "quarantined, #320: timing", "strict": False}


def test_conftest_with_an_empty_quarantine_marks_nothing(
    tmp_path: Path, monkeypatch: pytest.MonkeyPatch
) -> None:
    conf = load_quarantine_conftest()
    listed = tmp_path / "quarantine.yaml"
    listed.write_text("quarantine: []\n", encoding="utf-8")
    monkeypatch.setattr(conf, "QUARANTINE", listed)
    items = [Item(f"{FILE}::test_render")]
    conf.pytest_collection_modifyitems(SimpleNamespace(rootpath=ROOT), items)  # type: ignore[arg-type]
    assert items[0].markers == []
