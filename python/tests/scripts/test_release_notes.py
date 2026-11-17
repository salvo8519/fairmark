"""scripts/release_notes.py: the text of one release, with the changelog link."""

from __future__ import annotations

import re
import sys
from pathlib import Path

import pytest
from conftest import load_script

release_notes = load_script("release_notes")
NOTES = """# Release notes

## Fairmark 0.2.0 (2026-12-01)

### Features

- Second.

## Fairmark 0.1.0 (2026-11-20)

- First.
"""


def test_section_returns_one_release_only() -> None:
    assert release_notes.section(NOTES, "0.2.0") == "### Features\n\n- Second."
    assert release_notes.section(NOTES, "0.1.0") == "- First."
    assert release_notes.section(NOTES, "0.3.0") == ""


def test_main_prints_the_notes_and_the_changelog_of_the_tag(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    (tmp_path / "RELEASE_NOTES.md").write_text(NOTES, encoding="utf-8")
    monkeypatch.setattr(release_notes, "ROOT", tmp_path)
    monkeypatch.setattr(sys, "argv", ["release_notes.py", "v0.1.0"])
    release_notes.main()
    out = capsys.readouterr().out
    assert out.startswith("- First.") and "/blob/v0.1.0/CHANGELOG.md" in out


def test_main_fails_without_notes(monkeypatch: pytest.MonkeyPatch, tmp_path: Path) -> None:
    (tmp_path / "RELEASE_NOTES.md").write_text(NOTES, encoding="utf-8")
    monkeypatch.setattr(release_notes, "ROOT", tmp_path)
    monkeypatch.setattr(sys, "argv", ["release_notes.py", "9.9.9"])
    with pytest.raises(SystemExit, match=re.escape("No release notes for 9.9.9")):
        release_notes.main()
