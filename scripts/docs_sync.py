"""Keep documentation in sync with the code, deterministically.

Two checks, used by `just`, pre-commit, CI and the Claude Code Stop hook:

  coverage   every method in the registry has a methodology page, a nav entry, a row in
             the methods index, golden cases and a Python helper
  impact     files changed in a way that needs documentation (core method, error codes, CLI,
             MCP, function calling) come with a matching documentation change

    python scripts/docs_sync.py coverage
    python scripts/docs_sync.py impact --base origin/main     # CI, pull requests
    python scripts/docs_sync.py impact --staged               # pre-commit
    python scripts/docs_sync.py impact --worktree --hook      # Claude Code Stop hook

`impact` uses only the standard library so that hooks run without the project environment.
Escape hatch for genuinely docs-neutral changes: the PR label `no-docs-needed`, or a commit
trailer `Docs-impact: none` on the latest commit.
"""

from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]

# (changed path pattern, documentation that must change with it, reason)
RULES: list[tuple[str, str, str]] = [
    (
        r"^crates/fairmark-core/src/error\.rs$",
        r"^docs/en/reference/errors\.md$",
        "error codes changed: update docs/en/reference/errors.md",
    ),
    (
        r"^crates/fairmark-core/src/(?P<area>[a-z_]+)\.rs$",
        r"^docs/en/methods/{area}-.*\.md$",
        "method area '{area}' changed: update its methodology page(s) in docs/en/methods/",
    ),
    (
        r"^crates/fairmark-core/src/(explain|registry|types)\.rs$",
        r"^docs/en/(concepts|reference)/",
        "engine contract changed: update docs/en/concepts/ or docs/en/reference/",
    ),
    (
        r"^crates/fairmark-cli/src/",
        r"^docs/en/guides/cli\.md$",
        "CLI changed: update docs/en/guides/cli.md",
    ),
    (
        r"^python/fairmark/mcp_server\.py$",
        r"^docs/en/guides/mcp\.md$",
        "MCP server changed: update docs/en/guides/mcp.md",
    ),
    (
        r"^python/fairmark/tools\.py$",
        r"^docs/en/guides/function-calling\.md$",
        "function calling changed: update docs/en/guides/function-calling.md",
    ),
]
NON_METHOD_FILES = {"error", "explain", "registry", "types", "lib", "num"}


def git(*args: str) -> list[str]:
    out = subprocess.run(["git", *args], cwd=ROOT, capture_output=True, text=True, check=True)
    return [line for line in out.stdout.splitlines() if line]


def changed_files(args: argparse.Namespace) -> list[str]:
    if args.staged:
        return git("diff", "--name-only", "--cached")
    if args.worktree:
        return sorted(
            set(git("diff", "--name-only", "HEAD"))
            | set(git("ls-files", "--others", "--exclude-standard"))
        )
    return sorted(set(git("diff", "--name-only", f"{args.base}...HEAD")))


def impact(args: argparse.Namespace) -> int:
    if not args.worktree and not args.staged:
        trailer = git("log", "-1", "--format=%(trailers:key=Docs-impact,valueonly)")
        if any(t.strip().lower() == "none" for t in trailer):
            print("Docs impact waived by 'Docs-impact: none' trailer.")
            return 0
    files = changed_files(args)
    missing: list[str] = []
    for pattern, docs, reason in RULES:
        for f in files:
            m = re.match(pattern, f)
            if not m:
                continue
            area = m.groupdict().get("area")
            if area in NON_METHOD_FILES:
                continue
            docs_re = docs.format(area=area) if area else docs
            if not any(re.match(docs_re, d) for d in files):
                msg = reason.format(area=area) if area else reason
                if msg not in missing:
                    missing.append(msg)
    if not missing:
        print("Docs impact check passed.")
        return 0
    stream = sys.stderr if args.hook else sys.stdout
    print("Documentation is out of sync with the code:", file=stream)
    for m in missing:
        print(f"  - {m}", file=stream)
    if args.hook:
        print("Delegate to the docs-writer agent, then run `uv run just generate`.", file=stream)
        return 2
    print("Fix it, or label the PR `no-docs-needed` if the change is genuinely docs-neutral.")
    return 1


def coverage() -> int:
    import fairmark as fm  # needs the project environment

    nav = (ROOT / "zensical.toml").read_text()
    index = (ROOT / "docs/en/methods/index.md").read_text()
    problems = []
    for m in fm.methods():
        mid = m["id"]
        slug = mid.replace(".", "-").replace("_", "-")
        area, name = mid.split(".", 1)
        page = f"methods/{slug}.md"
        if not (ROOT / "docs/en" / page).exists():
            problems.append(f"{mid}: missing methodology page docs/en/{page}")
        if f'"{page}"' not in nav:
            problems.append(f"{mid}: page not in zensical.toml nav")
        if f"{slug}.md" not in index:
            problems.append(f"{mid}: not listed in docs/en/methods/index.md")
        if not list((ROOT / "tests/golden" / mid).glob("*.yaml")):
            problems.append(f"{mid}: no golden cases in tests/golden/{mid}/")
        helper = ROOT / "python/fairmark" / f"{area}.py"
        if not helper.exists() or not re.search(rf"^def {name}\(", helper.read_text(), re.M):
            problems.append(f"{mid}: no Python helper `{name}` in python/fairmark/{area}.py")
    if problems:
        print("Documentation coverage gaps:")
        print("\n".join(f"  - {p}" for p in problems))
        return 1
    print(f"Docs coverage passed for {len(fm.methods())} methods.")
    return 0


def main() -> None:
    parser = argparse.ArgumentParser()
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("coverage")
    p = sub.add_parser("impact")
    group = p.add_mutually_exclusive_group(required=True)
    group.add_argument("--base")
    group.add_argument("--staged", action="store_true")
    group.add_argument("--worktree", action="store_true")
    p.add_argument("--hook", action="store_true", help="exit 2 with stderr (Claude Code hook)")
    args = parser.parse_args()
    sys.exit(coverage() if args.cmd == "coverage" else impact(args))


if __name__ == "__main__":
    main()
