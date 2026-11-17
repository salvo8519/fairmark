"""Translations of the root documents are tracked like the documentation pages."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest
from conftest import load_script

ROOT = Path(__file__).resolve().parents[3]
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


# ------------------------------------------------------------------------------------------
# A required page is translated even in an English-only section (issue #346)


def test_a_required_page_overrides_english_only(tmp_path, monkeypatch) -> None:  # type: ignore[no-untyped-def]
    en = tmp_path / "docs" / "en"
    for page in ("project/vision.md", "project/agentic-development.md", "index.md"):
        (en / page).parent.mkdir(parents=True, exist_ok=True)
        (en / page).write_text("# Page\n", encoding="utf-8")
    monkeypatch.setattr(i18n, "EN", en)
    monkeypatch.setattr(i18n, "ROOT", tmp_path)
    monkeypatch.setattr(i18n, "ROOT_DOCUMENTS", [])
    cfg = {
        "required": ["index.md", "project/agentic-development.md"],
        "english_only": ["project/*.md"],
        "generated": [],
    }
    pages = {rel: required for rel, _, _, required in i18n.pairs("it", cfg)}
    assert pages == {"index.md": True, "project/agentic-development.md": True}


def test_the_italian_nav_links_a_required_project_page_in_italian(tmp_path, monkeypatch) -> None:  # type: ignore[no-untyped-def]
    docs = load_script("docs")
    for lang in ("en", "it"):
        page = tmp_path / lang / "project" / "agentic-development.md"
        page.parent.mkdir(parents=True)
        page.write_text("# Agentic development\n", encoding="utf-8")
    (tmp_path / "en" / "project" / "vision.md").write_text("# Vision\n", encoding="utf-8")
    monkeypatch.setattr(docs, "DOCS", tmp_path)
    cfg = {
        "sections": {"Project": "Progetto"},
        "required": ["project/agentic-development.md"],
        "english_only": ["project/*.md"],
    }
    nav = [{"Project": [{"x": "project/vision.md"}, {"y": "project/agentic-development.md"}]}]
    out = docs.translate_nav(nav, cfg, "https://example.org/")
    assert out == [
        {
            "Progetto": [
                {"Vision (English)": "https://example.org/project/vision/"},
                {"y": "project/agentic-development.md"},
            ]
        }
    ]
