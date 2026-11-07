"""Pin every GitHub Action to the commit of its tag, keeping the tag as a comment.

A tag such as `v7` can be moved to other code by whoever controls the action; a commit hash
cannot. Dependabot keeps pinned actions current and updates the comment with the version.
zizmor fails CI on any unpinned action, so run this after adding or changing one:

    python scripts/pin_actions.py            # rewrite .github/workflows/*.yml
    python scripts/pin_actions.py --check    # exit 1 if any action is not pinned

`dtolnay/rust-toolchain` and `taiki-e/install-action` select what they install from the ref
(`@stable`, `@1.88`, `@cargo-llvm-cov`). Pinned to a hash they read it from an input instead,
which this script adds (`toolchain:` and `tool:`).
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = sorted((ROOT / ".github" / "workflows").glob("*.yml"))
USES = re.compile(
    r"^(?P<indent>\s*)(?P<dash>-\s+)?uses:\s+(?P<action>[^@\s]+)@(?P<ref>[^\s#]+)\s*(?:#.*)?$"
)
PINNED = re.compile(r"^[0-9a-f]{40}$")
# Actions whose ref selects what they install: the input that replaces it.
REF_AS_INPUT = {"dtolnay/rust-toolchain": "toolchain", "taiki-e/install-action": "tool"}


def resolve(action: str, ref: str, cache: dict[tuple[str, str], str]) -> str:
    """Commit of a tag (peeled when annotated) or of a branch, read with `git ls-remote`."""
    repo = "/".join(action.split("/")[:2])
    if (repo, ref) not in cache:
        out = subprocess.run(
            [
                "git",
                "ls-remote",
                f"https://github.com/{repo}",
                f"refs/tags/{ref}*",
                f"refs/heads/{ref}",
            ],
            capture_output=True,
            text=True,
        )
        refs = dict(reversed(line.split("\t")) for line in out.stdout.split("\n") if line)
        sha = (
            refs.get(f"refs/tags/{ref}^{{}}")
            or refs.get(f"refs/tags/{ref}")
            or refs.get(f"refs/heads/{ref}")
        )
        if out.returncode != 0 or not sha:
            sys.exit(
                f"cannot resolve {repo}@{ref}: {out.stderr.strip() or 'no such tag or branch'}"
            )
        cache[(repo, ref)] = sha
    return cache[(repo, ref)]


def pin_file(path: Path, cache: dict[tuple[str, str], str]) -> None:
    lines = path.read_text(encoding="utf-8").splitlines()
    out: list[str] = []
    i = 0
    while i < len(lines):
        line = lines[i]
        m = USES.match(line)
        if not m or PINNED.match(m["ref"]):
            out.append(line)
            i += 1
            continue
        action, ref = m["action"], m["ref"]
        sha = resolve(action, ref, cache)
        out.append(f"{m['indent']}{m['dash'] or ''}uses: {action}@{sha} # {ref}")
        i += 1
        key = REF_AS_INPUT.get(action)
        if key is None or ref.startswith("v"):
            continue
        # Ref names the toolchain or tool: move it to the input, merging an existing `with:`.
        body = m["indent"] + ("  " if m["dash"] else "")
        if i < len(lines) and lines[i].strip().startswith("with:"):
            with_line = lines[i]
            if with_line.strip() == "with:":
                out.append(with_line)
                out.append(f"{body}  {key}: {ref}")
            else:  # inline mapping, for example `with: { tool: cargo-mutants }`
                out.append(with_line)
            i += 1
        else:
            out.append(f"{body}with:")
            out.append(f"{body}  {key}: {ref}")
    path.write_text("\n".join(out) + "\n", encoding="utf-8", newline="\n")


def unpinned() -> list[str]:
    found = []
    for path in WORKFLOWS:
        for n, line in enumerate(path.read_text(encoding="utf-8").splitlines(), 1):
            m = USES.match(line)
            if m and not PINNED.match(m["ref"]) and not m["action"].startswith("./"):
                found.append(f"{path.relative_to(ROOT)}:{n}: {m['action']}@{m['ref']}")
    return found


def main() -> None:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    if args.check:
        found = unpinned()
        if found:
            print("Actions not pinned to a commit:\n  " + "\n  ".join(found))
            print("Run: python scripts/pin_actions.py")
            sys.exit(1)
        print("Every action is pinned to a commit.")
        return
    cache: dict[tuple[str, str], str] = {}
    for path in WORKFLOWS:
        pin_file(path, cache)
    print(json.dumps({f"{r}@{t}": s[:12] for (r, t), s in sorted(cache.items())}, indent=2))


if __name__ == "__main__":
    main()
