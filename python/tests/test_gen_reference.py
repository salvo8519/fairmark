"""Italian method titles of scripts/gen_reference.py (README methods table, issue #185)."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

import fairmark as fm

# scripts/gen_reference.py is tooling run with the development Python (3.11+, tomllib).
pytest.importorskip("tomllib")

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("gen_reference", ROOT / "scripts/gen_reference.py")
assert _spec and _spec.loader
gr = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(gr)


def test_every_registered_method_has_an_italian_title() -> None:
    ids = [m["id"] for m in fm.methods()]
    assert gr.title_problems(ids, gr.load_titles("it"), "it") == []


def test_missing_title_is_reported() -> None:
    problems = gr.title_problems(["ex.one", "ex.two"], {"ex.one": "Uno"}, "it")
    assert problems == ["ex.two: no title in i18n/method_titles/it.yaml"]


def test_unknown_id_is_reported() -> None:
    problems = gr.title_problems(["ex.one"], {"ex.one": "Uno", "ex.gone": "Via"}, "it")
    assert problems == ["ex.gone: listed in i18n/method_titles/it.yaml but not a registered method"]


def test_empty_title_is_reported() -> None:
    problems = gr.title_problems(["ex.one"], {"ex.one": " "}, "it")
    assert problems == ["ex.one: empty or non-text title in i18n/method_titles/it.yaml"]


def test_readme_tables_carry_the_legend_and_italian_titles() -> None:
    it = gr.readme_block("it")
    en = gr.readme_block("en")
    assert "Disponibile: " in it and "Anteprima: " in it
    assert "Available: verified independently" in en
    assert "#method-status" in it and "#method-status" in en
    assert "| `coc.wacc` | " in it and "Costo medio ponderato del capitale (WACC)" in it
    assert "Weighted average cost of capital (WACC)" not in it
