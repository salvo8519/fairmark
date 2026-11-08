"""Whether a version can be released, decided by checks rather than by memory.

Version policy (docs/en/project/releases.md): each milestone in roadmap/roadmap.yaml names the
release that ships it. A release X.Y.Z requires every milestone with release <= X.Y.Z to be
complete; a pre-release (X.Y.Z-alpha.N, -beta.N, -rc.N) can be cut at any time and reports the
open work instead of requiring it.

    python scripts/release.py ready 0.1.0-alpha.1 [--offline]

Checks: the version is valid and newer than the last tag; the required milestones are
complete; there are release notes to publish (fragments in changes/); the roadmap, the
translations and the generated files are current; every available method meets the promotion
rule and none is available on legacy grounds (ADR 0012); online: main is green, no open pull
request carries `numerical-change`, and the roadmap agrees with GitHub. Exit 1 when any check fails.
"""

from __future__ import annotations

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

import yaml

ROOT = Path(__file__).resolve().parents[1]
SEMVER = re.compile(r"^(\d+)\.(\d+)\.(\d+)(?:-((?:alpha|beta|rc)\.\d+))?\Z")
REPO = "salvo8519/fairmark"


def key(version: str) -> tuple[int, int, int, int, str]:
    """Order of versions: a pre-release comes before its release."""
    m = SEMVER.match(version)
    if not m:
        raise ValueError(version)
    major, minor, patch, pre = m.groups()
    return (int(major), int(minor), int(patch), 0 if pre else 1, pre or "")


def run(*cmd: str) -> tuple[bool, str]:
    out = subprocess.run(cmd, cwd=ROOT, capture_output=True, text=True)
    text = (out.stdout + out.stderr).strip().splitlines()
    return out.returncode == 0, text[-1] if text else ""


def gh(path: str) -> object:
    out = subprocess.run(["gh", "api", path], capture_output=True, text=True)
    if out.returncode != 0:
        raise RuntimeError(out.stderr.strip())
    return json.loads(out.stdout)


def legacy_methods(path: Path = ROOT / "specs/method-evidence.yaml") -> list[str]:
    """Methods available on legacy grounds (ADR 0012); a release requires none."""
    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    return list(data.get("legacy") or [])


def ready(version: str, offline: bool) -> int:
    results: list[tuple[bool, str]] = []

    def record(ok: bool, text: str) -> None:
        results.append((ok, text))

    if not SEMVER.match(version):
        print(f"FAIL  {version} is not X.Y.Z or X.Y.Z-(alpha|beta|rc).N")
        return 1
    pre = SEMVER.match(version).group(4)  # type: ignore[union-attr]

    tags = subprocess.run(["git", "tag", "-l", "v*"], cwd=ROOT, capture_output=True, text=True)
    previous = [t[1:] for t in tags.stdout.split() if SEMVER.match(t[1:])]
    last = max(previous, key=key) if previous else None
    record(
        last is None or key(version) > key(last),
        f"newer than the last tag ({last or 'none'})",
    )

    roadmap = yaml.safe_load((ROOT / "roadmap/roadmap.yaml").read_text(encoding="utf-8"))
    target = key(version)[:3]
    for m in roadmap["milestones"]:
        if key(m["release"])[:3] > target:
            continue
        open_tasks = [
            t for e in m["epics"] for t in e["tasks"] if t.get("status", "open") == "open"
        ]
        if not open_tasks:
            record(True, f"{m['title']} complete")
        elif pre:
            record(True, f"{m['title']}: {len(open_tasks)} open tasks (allowed in a pre-release)")
        else:
            numbers = ", ".join(f"#{t['issue']}" for t in open_tasks[:8])
            record(False, f"{m['title']}: {len(open_tasks)} open tasks ({numbers})")

    fragments = [
        p for p in (ROOT / "changes").iterdir() if p.suffix == ".md" and p.name != "README.md"
    ]
    record(bool(fragments), f"{len(fragments)} release note fragments in changes/")

    for label, cmd in [
        ("roadmap consistent", ["python", "scripts/roadmap.py", "check"]),
        ("translations current", ["python", "scripts/i18n.py", "check"]),
        ("schemas current", ["python", "scripts/schema_snapshots.py", "--check"]),
        ("version files consistent", ["python", "scripts/version.py", "check"]),
        ("method status meets the promotion rule", ["python", "scripts/method_status.py", "check"]),
    ]:
        ok, last_line = run(sys.executable, *cmd[1:])
        record(ok, f"{label}" + ("" if ok else f": {last_line}"))

    legacy = legacy_methods()
    record(
        not legacy,
        "no method available on legacy grounds"
        + (f": {', '.join(legacy)} (specs/method-evidence.yaml)" if legacy else ""),
    )

    if offline:
        record(True, "online checks skipped (--offline)")
    else:
        try:
            runs = gh(f"repos/{REPO}/actions/runs?branch=main&event=push&per_page=20")
            ci = [r for r in runs["workflow_runs"] if r["name"] == "CI"]  # type: ignore[index]
            latest = ci[0] if ci else None
            record(
                bool(latest) and latest["conclusion"] == "success",
                f"main is green (CI on {latest['head_sha'][:7] if latest else 'none'}: "
                f"{latest['conclusion'] if latest else 'no run'})",
            )
            pulls = gh(f"repos/{REPO}/pulls?state=open&per_page=100")
            numerical = [
                p["number"]
                for p in pulls  # type: ignore[union-attr]
                if any(lbl["name"] == "numerical-change" for lbl in p["labels"])
            ]
            record(not numerical, f"no open numerical-change pull request {numerical or ''}")
            ok, last_line = run(sys.executable, "scripts/roadmap.py", "sync")
            record(ok, "roadmap agrees with GitHub" + ("" if ok else f": {last_line}"))
        except RuntimeError as err:
            record(False, f"online checks failed: {err}")

    for ok, text in results:
        print(f"{'OK  ' if ok else 'FAIL'}  {text}")
    failed = sum(not ok for ok, _ in results)
    print(f"\n{version}: {'ready' if not failed else f'not ready ({failed} failing)'}")
    return 1 if failed else 0


def main() -> None:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    r = sub.add_parser("ready")
    r.add_argument("version")
    r.add_argument("--offline", action="store_true")
    args = parser.parse_args()
    sys.exit(ready(args.version, args.offline))


if __name__ == "__main__":
    main()
