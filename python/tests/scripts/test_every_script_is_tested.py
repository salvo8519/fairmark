"""Every script in `scripts/` has a test (issue #345).

On 16 November four defects of the process layer were found in a few hours, in scripts that no
test ran. A test names its script as `<name>.py` (a path) or loads it with `load_script` or
`load` by name; this test fails when a script is named by none.
"""

from __future__ import annotations

from conftest import ROOT, SCRIPTS


def named_by(text: str, name: str) -> bool:
    return any(form in text for form in (f"{name}.py", f'load_script("{name}")', f'load("{name}")'))


def test_every_script_has_a_test() -> None:
    tests = [p.read_text(encoding="utf-8") for p in (ROOT / "python" / "tests").rglob("test_*.py")]
    scripts = sorted(p.stem for p in SCRIPTS.glob("*.py"))
    untested = [s for s in scripts if not any(named_by(t, s) for t in tests if t)]
    assert untested == [], f"scripts without a test (python/tests/scripts/): {untested}"


def test_the_rule_needs_a_real_mention() -> None:
    assert named_by('ROOT / "scripts" / "sprint.py"', "sprint")
    assert named_by('load_script("check_yaml")', "check_yaml")
    assert not named_by("the sprint script", "sprint")
