"""Promotion rule of scripts/method_status.py (ADR 0011, docs/en/project/releases.md)."""

from __future__ import annotations

import importlib.util
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
_spec = importlib.util.spec_from_file_location("method_status", ROOT / "scripts/method_status.py")
assert _spec and _spec.loader
ms = importlib.util.module_from_spec(_spec)
_spec.loader.exec_module(ms)

SPEC_OK = """# SPEC-0042 — Example
| | |
| --- | --- |
| Status | Approved — 2026-11-08 |

## Interfaces
- Method id(s): `ex.one`, `ex.two`

## Open questions
- None.
"""
TASKS_OK = """# Tasks
- [x] T1 Implementation
- [x] T2 Independent numerical audit (numerical-auditor)
- [x] T3 Code review (code-reviewer)
- [ ] T4 Something unrelated
"""


def make_repo(tmp_path: Path, spec: str = SPEC_OK, tasks: str = TASKS_OK) -> Path:
    folder = tmp_path / "specs/0042-example"
    folder.mkdir(parents=True)
    (folder / "spec.md").write_text(spec)
    (folder / "tasks.md").write_text(tasks)
    for mid in ("ex.one", "ex.two"):
        golden = tmp_path / "tests/golden" / mid
        golden.mkdir(parents=True)
        (golden / "case.yaml").write_text(f"method: {mid}\nsource: textbook, p. 1\n")
    return tmp_path


EVIDENCE = {
    "specs": {
        "0042": {"methods": ["ex.one", "ex.two"], "numerical_audit": "T2", "code_review": "T3"}
    }
}


def methods(**status: str) -> list[dict[str, str]]:
    """The two example methods (preview unless given), plus any other given method."""
    merged = {"ex_one": "preview", "ex_two": "preview", **status}
    return [{"id": mid.replace("_", "."), "status": s} for mid, s in merged.items()]


def test_complete_evidence_allows_available(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    errors, warnings, _ = ms.evaluate(
        methods(ex_one="available", ex_two="preview"), EVIDENCE, root, legacy=frozenset()
    )
    assert errors == [] and warnings == []


def test_preview_methods_need_no_evidence(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    errors, _, _ = ms.evaluate(
        methods(ex_one="preview", ex_three="preview"), EVIDENCE, root, legacy=frozenset()
    )
    assert errors == []


@pytest.mark.parametrize(
    ("spec", "tasks", "expected"),
    [
        (SPEC_OK.replace("Approved", "Draft"), TASKS_OK, "spec 0042 is not approved"),
        (SPEC_OK + "- [ ] Open point\n", TASKS_OK, "unchecked item"),
        (SPEC_OK.replace("`ex.one`, ", ""), TASKS_OK, "does not name"),
        (SPEC_OK, TASKS_OK.replace("[x] T2", "[ ] T2"), "numerical audit"),
        (SPEC_OK, TASKS_OK.replace("[x] T3", "[ ] T3"), "code review"),
        (
            SPEC_OK,
            TASKS_OK.replace("T2 Independent numerical audit (numerical-auditor)", "T2 Docs"),
            "numerical",
        ),
    ],
)
def test_each_condition_blocks_promotion(
    tmp_path: Path, spec: str, tasks: str, expected: str
) -> None:
    root = make_repo(tmp_path, spec, tasks)
    errors, _, _ = ms.evaluate(methods(ex_one="available"), EVIDENCE, root, legacy=frozenset())
    assert len(errors) == 1 and expected in errors[0], errors


def test_golden_cases_need_a_source(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    (root / "tests/golden/ex.one/old.yaml").write_text("method: ex.one\n")
    errors, _, _ = ms.evaluate(methods(ex_one="available"), EVIDENCE, root, legacy=frozenset())
    assert len(errors) == 1 and "old.yaml" in errors[0]


def test_available_without_an_entry_fails(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    errors, _, _ = ms.evaluate(methods(ex_three="available"), EVIDENCE, root, legacy=frozenset())
    assert errors and "no entry" in errors[0]


def test_legacy_methods_warn_and_the_list_only_shrinks(tmp_path: Path) -> None:
    root = make_repo(tmp_path, tasks=TASKS_OK.replace("[x] T3", "[ ] T3"))
    errors, warnings, _ = ms.evaluate(
        methods(ex_one="available"), EVIDENCE, root, legacy=frozenset({"ex.one"})
    )
    assert errors == [] and len(warnings) == 1
    # Evidence complete: the legacy entry must go.
    root2 = make_repo(tmp_path / "b")
    errors, _, _ = ms.evaluate(
        methods(ex_one="available"), EVIDENCE, root2, legacy=frozenset({"ex.one"})
    )
    assert errors and "remove it" in errors[0]
    # Preview: the legacy entry must go too.
    errors, _, _ = ms.evaluate(
        methods(ex_one="preview"), EVIDENCE, root2, legacy=frozenset({"ex.one"})
    )
    assert errors and "remove it" in errors[0]


def test_evidence_names_registered_methods_once(tmp_path: Path) -> None:
    root = make_repo(tmp_path)
    twice = {"specs": {**EVIDENCE["specs"], "0043": {"methods": ["ex.one"]}}}
    errors, _, _ = ms.evaluate(methods(ex_one="preview"), twice, root, legacy=frozenset())
    assert any("more than one" in e for e in errors)
    errors, _, _ = ms.evaluate([], EVIDENCE, root, legacy=frozenset())
    assert any("not a registered method" in e for e in errors)


def test_repository_passes() -> None:
    """The real registry, evidence file and specs meet the rule (legacy methods aside)."""
    import fairmark as fm

    errors, _, _ = ms.evaluate(fm.methods(), ms.load_evidence(), ROOT, legacy=ms.LEGACY)
    assert errors == []
