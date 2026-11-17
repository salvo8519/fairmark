"""scripts/setup_github_project.py: the project board, with the GitHub CLI replaced by a fake."""

from __future__ import annotations

import json
import subprocess
from typing import Any

import pytest
from conftest import load_script

setup = load_script("setup_github_project")


def test_gh_parses_json_and_returns_text_otherwise(monkeypatch: pytest.MonkeyPatch) -> None:
    answers = iter(['{"number": 3}', "linked\n"])

    def fake_run(cmd: list[str], **_: Any) -> subprocess.CompletedProcess[str]:
        assert cmd[0] == "gh"
        return subprocess.CompletedProcess(cmd, 0, next(answers), "")

    monkeypatch.setattr(setup.subprocess, "run", fake_run)
    assert setup.gh("project", "view") == {"number": 3}
    assert setup.gh("project", "link") == "linked\n"


def test_areas_match_the_area_labels_of_the_roadmap() -> None:
    import yaml
    from conftest import ROOT

    plan = yaml.safe_load((ROOT / "roadmap/roadmap.yaml").read_text(encoding="utf-8"))
    labels = {lbl["name"].removeprefix("area:") for lbl in plan["labels"]}
    assert set(setup.AREAS) <= labels, sorted(set(setup.AREAS) - labels)
    assert json.dumps(setup.TITLE) == '"Fairmark roadmap"'
