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
