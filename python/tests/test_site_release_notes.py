"""Release notes and changelog as the documentation site renders them between releases."""

from __future__ import annotations

import importlib.util
import shutil
import subprocess
from pathlib import Path

import pytest

# scripts/docs.py runs with the documentation toolchain (Python 3.11 or later, tomllib);
# the package itself supports 3.10.
pytest.importorskip("tomllib")

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("docs_site", ROOT / "scripts/docs.py")
assert _spec and _spec.loader
docs_site = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(docs_site)


def test_unreleased_title_carries_no_date() -> None:
    draft = "## Fairmark Unreleased — 2026-11-08\n\n### New\n\n- Something new."
    assert docs_site.undated_draft(draft) == "## Fairmark Unreleased\n\n### New\n\n- Something new."


def test_released_sections_keep_their_date() -> None:
    released = "## Fairmark 0.1.0 — 2026-12-01\n\n### New\n\n- Something new."
    assert docs_site.undated_draft(released) == released


def test_placeholder_without_changes_is_unchanged() -> None:
    empty = "## Fairmark Unreleased\n\nNo user-facing changes since the last release."
    assert docs_site.undated_draft(empty) == empty


def _git(repo: Path, *args: str) -> None:
    subprocess.run(["git", *args], cwd=repo, check=True, capture_output=True)


@pytest.mark.skipif(shutil.which("git-cliff") is None, reason="git-cliff not installed")
def test_numerical_changes_name_the_method_versions(tmp_path: Path) -> None:
    _git(tmp_path, "init", "-q", "-b", "main")
    _git(tmp_path, "config", "user.name", "Test")
    _git(tmp_path, "config", "user.email", "test@example.org")
    _git(
        tmp_path,
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "fix(tvm): exact discounting at negative rates\n\n"
        "Numerical-change: tvm.npv v1 -> v2\n"
        "Numerical-change: dcf.fcff v1 -> v2\n"
        "Signed-off-by: Test <test@example.org>",
    )
    _git(
        tmp_path,
        "commit",
        "-q",
        "--allow-empty",
        "-m",
        "feat(coc): weighted average cost of capital\n\nSigned-off-by: Test <test@example.org>",
    )
    out = subprocess.run(
        ["git-cliff", "--config", str(ROOT / "cliff.toml"), "--repository", str(tmp_path)],
        check=True,
        capture_output=True,
        text=True,
        encoding="utf-8",
    ).stdout
    assert (
        "- **tvm**: Exact discounting at negative rates. "
        "Method versions: tvm.npv v1 -> v2, dcf.fcff v1 -> v2"
    ) in out
    assert "- **coc**: Weighted average cost of capital\n" in out
