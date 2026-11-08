"""Version ordering of scripts/release.py."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("release", ROOT / "scripts/release.py")
assert _spec and _spec.loader
release = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(release)


def test_pre_releases_come_before_their_release() -> None:
    order = ["0.1.0-alpha.1", "0.1.0-alpha.2", "0.1.0-beta.1", "0.1.0-rc.1", "0.1.0", "0.1.1"]
    assert sorted(order, key=release.key) == order
    assert release.key("0.10.0") > release.key("0.9.0")


@pytest.mark.parametrize("bad", ["1.0", "v1.0.0", "1.0.0-preview", "1.0.0-alpha"])
def test_invalid_versions_are_refused(bad: str) -> None:
    with pytest.raises(ValueError):
        release.key(bad)


_vspec = importlib.util.spec_from_file_location("version", ROOT / "scripts/version.py")
assert _vspec and _vspec.loader
version_script = importlib.util.module_from_spec(_vspec)
_vspec.loader.exec_module(version_script)


@pytest.mark.parametrize("bad", ["1.2.3\n", "0.1.0-alpha.1\n"])
def test_trailing_newline_is_refused(bad: str) -> None:
    """`$` matches before a final newline; the patterns must anchor at the end (#154)."""
    with pytest.raises(ValueError):
        release.key(bad)
    assert release.ready(bad, offline=True) == 1
    with pytest.raises(SystemExit):
        version_script.pep440(bad)
    assert not release.SEMVER.match(bad)


def test_valid_versions_still_pass() -> None:
    assert release.key("1.2.3") == (1, 2, 3, 1, "")
    assert version_script.pep440("0.1.0-alpha.1") == "0.1.0a1"


def test_a_release_needs_an_empty_legacy_list(tmp_path: Path) -> None:
    """ADR 0012: no release ships a method available on legacy grounds."""
    evidence = tmp_path / "method-evidence.yaml"
    evidence.write_text("specs: {}\nlegacy: [tvm.npv]\n")
    assert release.legacy_methods(evidence) == ["tvm.npv"]
    evidence.write_text("specs: {}\nlegacy: []\n")
    assert release.legacy_methods(evidence) == []
    assert release.legacy_methods() == [], "the repository's legacy list must be empty"
