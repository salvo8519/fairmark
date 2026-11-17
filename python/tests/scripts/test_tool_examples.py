"""scripts/tool_examples.py: the examples file is current and comes from golden cases."""

from __future__ import annotations

import json
from pathlib import Path

import pytest
import yaml
from conftest import ROOT, load_script

tool_examples = load_script("tool_examples")


def test_the_shipped_file_is_current() -> None:
    assert tool_examples.main(["--check"]) == 0


def test_every_example_is_the_input_of_its_golden_case() -> None:
    built = tool_examples.build()
    assert built, "no examples"
    for method, entry in built.items():
        assert "minimal" in entry, method
        for kind in ("minimal", "maximal"):
            if kind in entry:
                source = ROOT / entry[kind]["source"]
                case = yaml.safe_load(source.read_text(encoding="utf-8"))
                assert case["method"] == method and case["input"] == entry[kind]["input"]
    assert json.loads(tool_examples.render()) == built


def test_check_fails_on_a_stale_file(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    stale = tmp_path / "tool_examples.json"
    stale.write_text("{}\n", encoding="utf-8")
    monkeypatch.setattr(tool_examples, "OUTPUT", stale)
    monkeypatch.setattr(tool_examples, "ROOT", tmp_path)
    assert tool_examples.main(["--check"]) == 1
    assert tool_examples.main([]) == 0
    assert tool_examples.main(["--check"]) == 0
