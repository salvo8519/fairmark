"""Every label the repository applies is declared in roadmap/roadmap.yaml (#17).

GitHub ignores a label that does not exist: an issue template that names one, a labeler rule
or a workflow that adds one then does nothing, with no error. The declared list is the one
`scripts/bootstrap_github.py` creates, so a label named anywhere must be declared there.
"""

from __future__ import annotations

import re

import yaml
from conftest import ROOT

GITHUB = ROOT / ".github"
DEPENDABOT_LABELS = {"dependencies", "github_actions"}  # added by Dependabot to its PRs


def declared() -> set[str]:
    data = yaml.safe_load((ROOT / "roadmap" / "roadmap.yaml").read_text(encoding="utf-8"))
    return {label["name"] for label in data["labels"]}


def referenced() -> dict[str, set[str]]:
    found: dict[str, set[str]] = {}

    def add(name: str, where: str) -> None:
        found.setdefault(name, set()).add(where)

    for path in sorted((GITHUB / "ISSUE_TEMPLATE").glob("*.yml")):
        for name in yaml.safe_load(path.read_text(encoding="utf-8")).get("labels") or []:
            add(name, path.name)
    for name in yaml.safe_load((GITHUB / "labeler.yml").read_text(encoding="utf-8")):
        add(name, "labeler.yml")
    pattern = re.compile(r"""(?:--label[ =]|labels\[\]=)["']?([A-Za-z0-9_:.-]+)""")
    for path in sorted((GITHUB / "workflows").glob("*.yml")):
        for name in pattern.findall(path.read_text(encoding="utf-8")):
            add(name, path.name)
    for name in DEPENDABOT_LABELS:
        add(name, "dependabot.yml")
    return found


def test_every_label_applied_somewhere_is_declared() -> None:
    known = declared()
    missing = {name: sorted(where) for name, where in referenced().items() if name not in known}
    assert missing == {}, f"declare these labels in roadmap/roadmap.yaml: {missing}"


def test_the_scan_finds_the_labels_of_templates_and_workflows() -> None:
    names = referenced()
    assert {"bug", "numerical", "method-request", "task"} <= names.keys()
    assert {"maintenance", "test-health", "no-release-note"} <= names.keys()
