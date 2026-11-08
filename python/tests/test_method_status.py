"""Promotion rule of scripts/method_status.py (ADR 0012, docs/en/project/releases.md)."""

from __future__ import annotations

import copy
import importlib.util
import sys
from pathlib import Path
from typing import Any

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("method_status", ROOT / "scripts/method_status.py")
assert _spec and _spec.loader
ms = importlib.util.module_from_spec(_spec)
sys.modules["method_status"] = ms  # dataclasses resolve their module by name
_spec.loader.exec_module(ms)

SPEC_OK = """# SPEC-0042 — Example
| | |
| --- | --- |
| Status | Approved — 2026-11-08 |

## Interfaces
- Method id(s): `ex.one`, `ex.two`

## Open questions
- None.

```markdown
- [ ] an example inside a code block is not an open item
```
"""
TASKS_OK = """# Tasks
- [x] T1 Implementation
- [x] T2 Independent numerical audit
      (`numerical-auditor`), findings fixed
- [x] T3 Code review (`code-reviewer`)
- [ ] T4 Something unrelated
"""
EVIDENCE: dict[str, Any] = {
    "specs": {
        "0042": {"methods": ["ex.one", "ex.two"], "numerical_audit": "T2", "code_review": "T3"}
    },
    "legacy": [],
}
NONE: frozenset[str] = frozenset()


def make_repo(tmp_path: Path, spec: str = SPEC_OK, tasks: str = TASKS_OK) -> Path:
    folder = tmp_path / "specs/0042-example"
    folder.mkdir(parents=True)
    (folder / "spec.md").write_text(spec, encoding="utf-8")
    (folder / "tasks.md").write_text(tasks, encoding="utf-8")
    for mid in ("ex.one", "ex.two"):
        golden = tmp_path / "tests/golden" / mid
        golden.mkdir(parents=True)
        (golden / "case.yaml").write_text(
            f"method: {mid}\nsource: textbook, p. 1\n", encoding="utf-8"
        )
    return tmp_path


def methods(**status: str) -> list[dict[str, str]]:
    """The two example methods (preview unless given), plus any other given method."""
    merged = {"ex_one": "preview", "ex_two": "preview", **status}
    return [{"id": mid.replace("_", "."), "status": s} for mid, s in merged.items()]


def with_legacy(*ids: str) -> dict[str, Any]:
    return {**EVIDENCE, "legacy": list(ids)}


def test_complete_evidence_allows_available(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    errors, warnings, _ = ms.evaluate(methods(ex_one="available"), EVIDENCE, root, NONE)
    assert errors == [] and warnings == []


def test_preview_methods_need_no_evidence(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    errors, _, _ = ms.evaluate(methods(ex_three="preview"), EVIDENCE, root, NONE)
    assert errors == []


@pytest.mark.parametrize(
    ("spec", "tasks", "expected"),
    [
        (SPEC_OK.replace("Approved", "Draft"), TASKS_OK, "spec 0042 is not approved"),
        (SPEC_OK + "- [ ] Open point\n", TASKS_OK, "unchecked item: Open point"),
        (SPEC_OK + "* [ ] Star bullet\n", TASKS_OK, "unchecked item: Star bullet"),
        (SPEC_OK + "+ [ ] Plus bullet\n", TASKS_OK, "unchecked item: Plus bullet"),
        (SPEC_OK + "1. [ ] Numbered\n", TASKS_OK, "unchecked item: Numbered"),
        (SPEC_OK.replace("`ex.one`, ", ""), TASKS_OK, "does not name"),
        (SPEC_OK, TASKS_OK.replace("[x] T2", "[ ] T2"), "numerical audit task T2 is not ticked"),
        (SPEC_OK, TASKS_OK.replace("[x] T3", "[ ] T3"), "code review task T3 is not ticked"),
        (SPEC_OK, TASKS_OK.replace("(`numerical-auditor`)", "(audit)"), "numerical-auditor"),
        (SPEC_OK, TASKS_OK.replace("(`code-reviewer`)", "(review)"), "code-reviewer"),
        (SPEC_OK, TASKS_OK.replace("T3 Code", "T5 Code"), "T3 is missing"),
        (SPEC_OK, TASKS_OK + "- [x] T3 Again (`code-reviewer`)\n", "defined 2 times"),
    ],
)
def test_each_condition_blocks_promotion(
    tmp_path: Path, spec: str, tasks: str, expected: str
) -> None:
    root = make_repo(tmp_path, spec, tasks)
    errors, _, _ = ms.evaluate(methods(ex_one="available"), EVIDENCE, root, NONE)
    assert len(errors) == 1 and expected in errors[0], errors


def test_task_ids_inside_code_blocks_are_ignored(tmp_path: Path) -> None:
    tasks = TASKS_OK + "```\n- [ ] T3 Code review (`code-reviewer`)\n```\n"
    root = make_repo(tmp_path, tasks=tasks)
    errors, _, _ = ms.evaluate(methods(ex_one="available"), EVIDENCE, root, NONE)
    assert errors == []


def test_golden_cases_need_a_source_and_a_mapping(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    (root / "tests/golden/ex.one/old.yaml").write_text("method: ex.one\n", encoding="utf-8")
    (root / "tests/golden/ex.one/list.yaml").write_text("- not a mapping\n", encoding="utf-8")
    errors, _, _ = ms.evaluate(methods(ex_one="available"), EVIDENCE, root, NONE)
    assert any("old.yaml has no source" in e for e in errors)
    assert any("list.yaml is not a YAML mapping" in e for e in errors)


def test_available_without_an_entry_fails(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    errors, _, _ = ms.evaluate(methods(ex_three="available"), EVIDENCE, root, NONE)
    assert errors and "no entry" in errors[0]


def test_legacy_methods_warn_and_the_list_only_shrinks(tmp_path: Path) -> None:
    frozen = frozenset({"ex.one"})
    root = make_repo(tmp_path, tasks=TASKS_OK.replace("[x] T3", "[ ] T3"))
    errors, warnings, _ = ms.evaluate(
        methods(ex_one="available"), with_legacy("ex.one"), root, frozen
    )
    assert errors == [] and len(warnings) == 1
    # A method outside the frozen set cannot be added to `legacy`.
    errors, _, _ = ms.evaluate(methods(ex_two="available"), with_legacy("ex.two"), root, frozen)
    assert any("may only shrink" in e for e in errors)
    # Evidence complete, or preview: the legacy entry must go.
    root2 = make_repo(tmp_path / "b")
    for status in ("available", "preview"):
        errors, _, _ = ms.evaluate(methods(ex_one=status), with_legacy("ex.one"), root2, frozen)
        assert errors and "remove it" in errors[0]


def _rekey(evidence: dict[str, Any], key: Any) -> None:
    evidence["specs"][key] = evidence["specs"].pop("0042")


@pytest.mark.parametrize(
    ("mutate", "expected"),
    [
        (lambda e: _rekey(e, 42), "quoted four-digit"),
        (lambda e: _rekey(e, "42"), "quoted four-digit"),
        (lambda e: e["specs"]["0042"].update({"audit": "T2"}), "unknown key 'audit'"),
        (lambda e: e["specs"]["0042"].update({"code_review": "3"}), "task id like T6"),
        (lambda e: e["specs"]["0042"].update({"methods": "ex.one"}), "list of method ids"),
        (lambda e: e.update({"extra": 1}), "unknown top-level key"),
        (lambda e: e.update({"legacy": "ex.one"}), "`legacy` must be a list"),
    ],
)
def test_evidence_shape_is_validated(tmp_path: Path, mutate: Any, expected: str) -> None:
    root = make_repo(tmp_path)
    evidence = copy.deepcopy(EVIDENCE)
    mutate(evidence)
    errors, _, _ = ms.evaluate(methods(), evidence, root, NONE)
    assert any(expected in e for e in errors), errors


def test_evidence_names_registered_methods_once(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    twice = copy.deepcopy(EVIDENCE)
    twice["specs"]["0043"] = {"methods": ["ex.one"]}
    errors, _, _ = ms.evaluate(methods(ex_one="preview"), twice, root, NONE)
    assert any("more than one" in e for e in errors)
    errors, _, _ = ms.evaluate([], EVIDENCE, root, NONE)
    assert any("not a registered method" in e for e in errors)


def test_legacy_stays_within_the_set_frozen_at_issue_149() -> None:
    """`legacy:` may only shrink: it never names a method outside FROZEN_AT_149."""
    assert set(ms.load_evidence()["legacy"]) <= ms.FROZEN_AT_149


def test_repository_passes() -> None:
    """The real registry, evidence file and specs meet the rule (legacy methods aside)."""
    import fairmark as fm

    errors, _, _ = ms.evaluate(fm.methods(), ms.load_evidence(), ROOT)
    assert errors == []
