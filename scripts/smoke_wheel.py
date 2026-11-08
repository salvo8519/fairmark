"""Smoke test of an installed Fairmark package: a built wheel or sdist, before it is published.

The Release workflow installs each wheel in a new virtual environment that contains only the
wheel and PyYAML, then runs, from a checkout of the tagged commit:

    python -I scripts/smoke_wheel.py

`-I` keeps the checkout out of `sys.path`, so the package under test is the installed one. The
script fails when:

- `fairmark` is imported from the source tree instead of the environment;
- the installed version differs from `[workspace.package] version` in `Cargo.toml` (converted
  to its Python form by `scripts/version.py`);
- a method that has golden cases is not registered;
- any golden case in `tests/golden/` gives a different result than its expected values.

`--source-tree` skips the first check, for the test of this script in the development
environment.
"""

from __future__ import annotations

import argparse
import importlib.util
import re
import sys
from decimal import Decimal
from pathlib import Path
from typing import Any

import yaml

import fairmark as fm

ROOT = Path(__file__).resolve().parents[1]
GOLDEN = ROOT / "tests" / "golden"


def expected_version() -> str:
    """The Python version of the package, from Cargo.toml through scripts/version.py."""
    spec = importlib.util.spec_from_file_location("version", ROOT / "scripts" / "version.py")
    assert spec and spec.loader
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return str(module.pep440(module.current()))


def resolve(value: Any, field: str) -> Any:
    """A key starting with '/' is an RFC 6901 pointer into the value; other keys are fields."""
    if not field.startswith("/"):
        return value[field]
    for token in field[1:].split("/"):
        token = token.replace("~1", "/").replace("~0", "~")
        if isinstance(value, list):
            if not re.fullmatch(r"0|[1-9][0-9]*", token):
                raise KeyError(f"bad array index {token!r}")
            value = value[int(token)]
        else:
            value = value[token]
    return value


def check_case(case: dict[str, Any]) -> list[str]:
    """Differences between the engine result and the expected values of one golden case."""
    if "expect_error" in case:
        try:
            fm.run(case["method"], case["input"])
        except fm.FairmarkError as err:
            if err.code != case["expect_error"]:
                return [f"error {err.code}, expected {case['expect_error']}"]
            return []
        return [f"no error, expected {case['expect_error']}"]
    result = fm.run(case["method"], case["input"])
    tolerance = Decimal(case["tolerance"])
    problems = []
    for field, expected in case["expect"].items():
        raw = resolve(result.value, field)
        try:
            want = Decimal(str(expected))
            if not want.is_finite():
                raise ArithmeticError
        except ArithmeticError:
            # Not a decimal: compared exactly (null, error codes, names).
            if expected == "null" and raw is not None:
                problems.append(f"{field}: {raw!r} is not null")
            elif expected != "null" and (raw is None or str(raw) != str(expected)):
                problems.append(f"{field}: {raw!r} != {expected!r}")
            continue
        got = Decimal(str(raw))
        if abs(got - want) > tolerance:
            problems.append(f"{field}: {got} != {expected}")
    return problems


def check_golden(golden: Path = GOLDEN) -> tuple[int, list[str]]:
    """Run every golden case; return the number of cases and the failures."""
    cases = sorted(golden.rglob("*.yaml"))
    registered = {m["id"] for m in fm.methods()}
    failures = [
        f"{d.name}: has golden cases but is not registered"
        for d in sorted(p for p in golden.iterdir() if p.is_dir())
        if d.name not in registered
    ]
    for path in cases:
        name = f"{path.parent.name}/{path.stem}"
        try:
            case = yaml.safe_load(path.read_text(encoding="utf-8"))
            failures += [f"{name}: {p}" for p in check_case(case)]
        except Exception as err:  # report every case, not only the first failure
            failures.append(f"{name}: {type(err).__name__}: {err}")
    return len(cases), failures


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n", 1)[0])
    parser.add_argument(
        "--source-tree",
        action="store_true",
        help="allow fairmark imported from the source tree (development test only)",
    )
    args = parser.parse_args()

    location = Path(fm.__file__).resolve()
    print(f"fairmark {fm.__version__} (engine {fm.ENGINE_VERSION}) from {location.parent}")
    print(f"Python {sys.version.split()[0]} on {sys.platform}")
    problems = []
    if not args.source_tree and ROOT in location.parents:
        problems.append(f"fairmark is imported from the source tree ({location}), not installed")
    want = expected_version()
    if fm.__version__ != want:
        problems.append(f"installed version {fm.__version__}, expected {want} (Cargo.toml)")
    count, failures = check_golden()
    problems += failures
    if count == 0:
        problems.append(f"no golden cases found in {GOLDEN}")
    if problems:
        sys.exit("Smoke test failed:\n  " + "\n  ".join(problems))
    print(f"Smoke test passed: {count} golden cases.")


if __name__ == "__main__":
    main()
