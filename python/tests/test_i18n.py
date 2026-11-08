"""Translations of the root documents are tracked like the documentation pages."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("i18n", ROOT / "scripts/i18n.py")
assert _spec and _spec.loader
i18n = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(i18n)


def test_root_documents_are_required_translations() -> None:
    names = {name: (dst.name, required) for name, _src, dst, required in i18n.pairs("it", {})}
    assert names["README.md"] == ("README.it.md", True)
    assert names["DISCLAIMER.md"] == ("DISCLAIMER.it.md", True)


def test_stamp_tracks_a_root_document(tmp_path, monkeypatch) -> None:  # type: ignore[no-untyped-def]
    (tmp_path / "DISCLAIMER.md").write_text("# Disclaimer\n", encoding="utf-8")
    translation = tmp_path / "DISCLAIMER.it.md"
    translation.write_text("# Avvertenze legali\n", encoding="utf-8")
    monkeypatch.setattr(i18n, "ROOT", tmp_path)
    i18n.stamp(translation)
    meta = i18n.meta(translation)
    assert meta["source"] == "DISCLAIMER.md"
    assert meta["source_hash"] == i18n.digest(tmp_path / "DISCLAIMER.md")


def test_stamp_refuses_an_english_source(tmp_path, monkeypatch) -> None:  # type: ignore[no-untyped-def]
    source = tmp_path / "DISCLAIMER.md"
    source.write_text("# Disclaimer\n", encoding="utf-8")
    monkeypatch.setattr(i18n, "ROOT", tmp_path)
    with pytest.raises(SystemExit):
        i18n.stamp(source)
    assert source.read_text(encoding="utf-8") == "# Disclaimer\n"
