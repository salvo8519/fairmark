"""Keep the status checks required on `main` in step with the workflows and the ruleset.

The "Protect main" ruleset requires status checks by job name. A renamed job leaves the
ruleset waiting for a check that never comes, and every pull request is blocked. The list of
required checks is committed in `.github/required-checks.txt` and checked two ways:

    python scripts/required_checks.py check    # offline: every name is produced on pull requests
    python scripts/required_checks.py remote   # online: the list equals the ruleset (needs gh)
    python scripts/required_checks.py list     # the check names produced on pull requests
"""

from __future__ import annotations

import argparse
import itertools
import json
import re
import subprocess
import sys
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
WORKFLOWS = ROOT / ".github" / "workflows"
REQUIRED = ROOT / ".github" / "required-checks.txt"
PR_EVENTS = ("pull_request", "pull_request_target")
MATRIX_REF = re.compile(r"\$\{\{\s*matrix\.([A-Za-z0-9_-]+)\s*\}\}")
# A job condition that excludes pull request events never reports on a pull request.
NOT_ON_PR = re.compile(
    r"github\.event_name\s*!=\s*'pull_request'|github\.event_name\s*==\s*'(?!pull_request)\w+'"
    r"|startsWith\(github\.ref,\s*'refs/tags/"
)


def required() -> list[str]:
    lines = REQUIRED.read_text(encoding="utf-8").splitlines()
    return [line.strip() for line in lines if line.strip() and not line.startswith("#")]


def triggers(workflow: dict[Any, Any]) -> dict[str, Any]:
    # PyYAML reads the key `on` as the boolean True.
    on = workflow.get("on", workflow.get(True, {}))
    if isinstance(on, str):
        return {on: None}
    if isinstance(on, list):
        return {event: None for event in on}
    return dict(on or {})


def combinations(matrix: dict[str, Any]) -> list[dict[str, Any]]:
    """Expand a strategy matrix as GitHub does for names: product, then include, then exclude."""
    axes = {
        k: v for k, v in matrix.items() if k not in ("include", "exclude") and isinstance(v, list)
    }
    combos = [dict(zip(axes, values, strict=True)) for values in itertools.product(*axes.values())]
    if not axes:
        combos = []
    for extra in matrix.get("include", []) or []:
        matched = False
        for combo in combos:
            if all(combo.get(k, v) == v for k, v in extra.items() if k in axes):
                combo.update({k: v for k, v in extra.items() if k not in axes})
                matched = True
        if not matched:
            combos.append(dict(extra))
    for drop in matrix.get("exclude", []) or []:
        combos = [c for c in combos if not all(c.get(k) == v for k, v in drop.items())]
    return combos or [{}]


def job_names(job_id: str, job: dict[str, Any]) -> list[str]:
    name = str(job.get("name", job_id))
    if not MATRIX_REF.search(name):
        return [name]
    matrix = (job.get("strategy") or {}).get("matrix") or {}
    names = []
    for combo in combinations(matrix):
        names.append(MATRIX_REF.sub(lambda m, c=combo: str(c.get(m.group(1), "")), name))
    return names


def produced_on_pull_requests(workflow_dir: Path = WORKFLOWS) -> dict[str, str]:
    """Check names reported on every pull request, with the workflow that produces each."""
    out: dict[str, str] = {}
    for path in sorted(workflow_dir.glob("*.yml")):
        workflow = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
        on = triggers(workflow)
        events = [e for e in PR_EVENTS if e in on]
        if not events:
            continue
        # A path filter skips the workflow on some pull requests: such a check cannot be required.
        if any(
            isinstance(on[e], dict) and ({"paths", "paths-ignore"} & set(on[e])) for e in events
        ):
            continue
        for job_id, job in (workflow.get("jobs") or {}).items():
            if NOT_ON_PR.search(str(job.get("if", ""))):
                continue
            for name in job_names(job_id, job):
                out[name] = path.name
    return out


def check(names: list[str], produced: dict[str, str]) -> list[str]:
    problems = [
        f"'{n}' is required but no job reports it on pull requests "
        "(renamed job? update .github/required-checks.txt and the ruleset together)"
        for n in names
        if n not in produced
    ]
    duplicates = {n for n in names if names.count(n) > 1}
    problems += [f"'{n}' is listed more than once" for n in sorted(duplicates)]
    return problems


def ruleset_checks(repo: str) -> set[str]:
    """Required checks of the active rulesets that apply to the default branch."""

    def gh(path: str) -> Any:
        res = subprocess.run(
            ["gh", "api", path], capture_output=True, text=True, check=True, encoding="utf-8"
        )
        return json.loads(res.stdout)

    found: set[str] = set()
    for summary in gh(f"repos/{repo}/rulesets?includes_parents=true"):
        ruleset = gh(f"repos/{repo}/rulesets/{summary['id']}")
        if ruleset.get("enforcement") != "active" or ruleset.get("target") != "branch":
            continue
        include = ((ruleset.get("conditions") or {}).get("ref_name") or {}).get("include", [])
        if not {"~DEFAULT_BRANCH", "refs/heads/main", "~ALL"} & set(include):
            continue
        for rule in ruleset.get("rules", []):
            if rule.get("type") == "required_status_checks":
                params = rule.get("parameters") or {}
                found |= {c["context"] for c in params.get("required_status_checks", [])}
    return found


def compare(listed: list[str], in_ruleset: set[str]) -> list[str]:
    problems = [
        f"'{n}' is listed but the ruleset does not require it"
        for n in listed
        if n not in in_ruleset
    ]
    problems += [
        f"'{n}' is required by the ruleset but missing from .github/required-checks.txt"
        for n in sorted(in_ruleset - set(listed))
    ]
    return problems


def main() -> None:
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    sub = parser.add_subparsers(dest="cmd", required=True)
    sub.add_parser("check")
    sub.add_parser("list")
    remote = sub.add_parser("remote")
    remote.add_argument("--repo", default="salvo8519/fairmark")
    args = parser.parse_args()

    if args.cmd == "list":
        for name, workflow in sorted(produced_on_pull_requests().items()):
            print(f"{name}    ({workflow})")
        return
    if args.cmd == "check":
        problems = check(required(), produced_on_pull_requests())
        ok = f"All {len(required())} required checks are reported on pull requests."
    else:
        problems = compare(required(), ruleset_checks(args.repo))
        ok = f"The ruleset requires the {len(required())} checks in .github/required-checks.txt."
    if problems:
        print("Required status checks:\n  - " + "\n  - ".join(problems))
        sys.exit(1)
    print(ok)


if __name__ == "__main__":
    main()
