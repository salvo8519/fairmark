"""Single source of truth for Fairmark's version.

The version lives in one place: `[workspace.package] version` in the root Cargo.toml. The
Rust crates inherit it, the Python package derives it at build time (maturin converts
`0.1.0-alpha.1` to `0.1.0a1`), and the engine stamps it into every result.

    python scripts/version.py show
    python scripts/version.py check            # CI: every place agrees (and the tag, if any)
    python scripts/version.py bump minor       # or major | patch | 1.2.0 | 1.2.0-rc.1
"""

from __future__ import annotations

import argparse
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
CARGO = ROOT / "Cargo.toml"
SEMVER = re.compile(r"^(\d+)\.(\d+)\.(\d+)(?:-([0-9A-Za-z.]+))?$")


def current() -> str:
    m = re.search(r'\[workspace\.package\][^\[]*?\nversion = "([^"]+)"', CARGO.read_text())
    if not m:
        sys.exit("version not found in Cargo.toml [workspace.package]")
    return m.group(1)


def pep440(version: str) -> str:
    m = SEMVER.match(version)
    if not m:
        sys.exit(f"not a semantic version: {version}")
    major, minor, patch, pre = m.groups()
    base = f"{major}.{minor}.{patch}"
    if not pre:
        return base
    kind, _, num = pre.partition(".")
    short = {"alpha": "a", "beta": "b", "rc": "rc"}.get(kind)
    if short is None:
        sys.exit(f"unsupported pre-release '{pre}' (use alpha.N, beta.N or rc.N)")
    return f"{base}{short}{num or 0}"


def check() -> None:
    version = current()
    text = CARGO.read_text()
    problems = []
    dep = re.search(r'fairmark-core = \{[^}]*version = "([^"]+)"', text)
    if not dep or dep.group(1) != version:
        problems.append("workspace dependency fairmark-core version differs from the workspace")
    lock = (ROOT / "uv.lock").read_text()
    m = re.search(r'\[\[package\]\]\nname = "fairmark"\nversion = "([^"]+)"', lock)
    if m and m.group(1) != pep440(version):
        problems.append(
            f"uv.lock has fairmark {m.group(1)}, expected {pep440(version)}: run uv lock"
        )
    ref = os.environ.get("GITHUB_REF", "")
    if ref.startswith("refs/tags/v") and ref.removeprefix("refs/tags/v") != version:
        problems.append(f"tag {ref} does not match version {version}")
    if problems:
        sys.exit("Version inconsistencies:\n  " + "\n  ".join(problems))
    print(f"Version {version} (Python {pep440(version)}) is consistent.")


def bump(spec: str) -> None:
    old = current()
    m = SEMVER.match(old)
    assert m
    major, minor, patch = (int(x) for x in m.groups()[:3])
    new = {
        "major": f"{major + 1}.0.0",
        "minor": f"{major}.{minor + 1}.0",
        "patch": f"{major}.{minor}.{patch + 1}",
    }.get(spec, spec)
    pep440(new)  # validates
    text = CARGO.read_text().replace(f'version = "{old}"', f'version = "{new}"')
    CARGO.write_text(text)
    subprocess.run(["uv", "lock", "--quiet"], cwd=ROOT, check=True)
    subprocess.run(["cargo", "update", "--workspace", "--quiet"], cwd=ROOT, check=True)
    print(f"{old} -> {new} (Python {pep440(new)})")


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("command", choices=["show", "check", "bump"])
    parser.add_argument("spec", nargs="?")
    args = parser.parse_args()
    if args.command == "show":
        print(current())
    elif args.command == "check":
        check()
    else:
        if not args.spec:
            sys.exit("bump needs major | minor | patch | X.Y.Z[-pre.N]")
        bump(args.spec)


if __name__ == "__main__":
    main()
