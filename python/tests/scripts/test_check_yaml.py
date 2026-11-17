"""scripts/check_yaml.py: a broken YAML file fails, valid ones pass."""

from __future__ import annotations

import re
from pathlib import Path

import pytest
from conftest import load_script

check_yaml = load_script("check_yaml")


def test_broken_yaml_fails(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    good, bad = tmp_path / "good.yaml", tmp_path / "bad.yml"
    good.write_text("a: 1\n---\nb: [1, 2]\n", encoding="utf-8")
    bad.write_text("a: [1, 2\n", encoding="utf-8")
    monkeypatch.setattr(check_yaml, "ROOT", tmp_path)
    monkeypatch.setattr(check_yaml, "tracked_yaml", lambda: [good, bad])
    with pytest.raises(SystemExit, match=re.escape("bad.yml")):
        check_yaml.main()


def test_valid_yaml_passes(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    good = tmp_path / "good.yaml"
    good.write_text("a: 1\n", encoding="utf-8")
    monkeypatch.setattr(check_yaml, "tracked_yaml", lambda: [good])
    check_yaml.main()
    assert capsys.readouterr().out == "1 YAML files parse.\n"
