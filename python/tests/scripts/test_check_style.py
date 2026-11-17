"""scripts/check_style.py: icons and emoji fail, the symbols of finance and maths pass."""

from __future__ import annotations

from pathlib import Path

import pytest
from conftest import load_script

check_style = load_script("check_style")


def run(monkeypatch: pytest.MonkeyPatch, tmp_path: Path, text: str) -> None:
    page = tmp_path / "page.md"
    page.write_text(text, encoding="utf-8")
    monkeypatch.setattr(check_style, "ROOT", tmp_path)
    monkeypatch.setattr(check_style, "tracked_files", lambda: [page])
    check_style.main()


@pytest.mark.parametrize(
    "text", ["Done \N{WHITE HEAVY CHECK MARK}", "Star \N{BLACK STAR}", "Tip :" + "material-check:"]
)
def test_icons_fail(monkeypatch: pytest.MonkeyPatch, tmp_path: Path, text: str) -> None:
    with pytest.raises(SystemExit):
        run(monkeypatch, tmp_path, text)


def test_finance_and_maths_symbols_pass(
    monkeypatch: pytest.MonkeyPatch, tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    run(
        monkeypatch,
        tmp_path,
        "NPV = \N{N-ARY SUMMATION} CF \N{MULTIPLICATION SIGN} d \N{RIGHTWARDS ARROW} 10 "
        "\N{EURO SIGN}, a \N{MINUS SIGN} b \N{LESS-THAN OR EQUAL TO} c",
    )
    assert "Style check passed" in capsys.readouterr().out


def test_the_repository_is_clean() -> None:
    check_style.main()
