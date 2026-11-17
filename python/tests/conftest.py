"""Quarantine of intermittent tests (issue #320).

A test listed in `quarantine.yaml` still runs. It is marked as an expected failure that is not
strict: if it fails, pytest reports it as `xfailed` and the suite stays green; if it passes, it
is reported as `xpassed`. Either way the outcome is written in the test report and in the JUnit
file, so `scripts/test_failures.py` keeps counting it. The list is validated by
`scripts/test_failures.py quarantine-check`: every entry has an issue, a reason and an expiry
date, so a quarantine never becomes permanent by neglect.
"""

from __future__ import annotations

from pathlib import Path

import pytest
import yaml

QUARANTINE = Path(__file__).with_name("quarantine.yaml")


def _quarantined() -> dict[str, str]:
    data = yaml.safe_load(QUARANTINE.read_text(encoding="utf-8")) or {}
    out = {}
    for entry in data.get("quarantine") or []:
        if isinstance(entry, dict) and isinstance(entry.get("test"), str):
            out[entry["test"]] = f"quarantined, #{entry.get('issue')}: {entry.get('reason')}"
    return out


def pytest_collection_modifyitems(config: pytest.Config, items: list[pytest.Item]) -> None:
    quarantined = _quarantined()
    if not quarantined:
        return
    root = Path(config.rootpath)
    for item in items:
        path = Path(item.path).resolve().relative_to(root.resolve()).as_posix()
        node = "::".join([path, *item.nodeid.split("::")[1:]])
        reason = quarantined.get(node)
        if reason:
            item.add_marker(pytest.mark.xfail(reason=reason, strict=False))
