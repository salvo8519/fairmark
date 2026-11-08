"""Check the status of every method against the promotion rule (ADR 0012).

The engine registry holds the status (`AVAILABLE` in crates/fairmark-core/src/registry.rs);
the evidence is in specs/method-evidence.yaml; the rule is in docs/en/project/releases.md,
"Method status". An available method must have:

  1. an approved spec (Status row starting with `Approved` or `Implementing`);
  2. no unchecked item (`- [ ]`, any bullet) in that spec, fenced code blocks aside;
  3. its id named in that spec;
  4. golden cases, each with a `source`;
  5. a ticked task naming `numerical-auditor` and 6. a ticked task naming `code-reviewer`
     in the spec's tasks.md, at the task ids the evidence file gives.

Methods listed under `legacy:` in the evidence file were available before the rule and do not
yet meet it: they are reported, not rejected. The list may only shrink: it must stay within
FROZEN_AT_149, and an entry must go once the method meets the rule or becomes preview.

    python scripts/method_status.py check    # CI and `just check`: fail on a violation
    python scripts/method_status.py report   # evidence of every method, eligible promotions
"""

from __future__ import annotations

import argparse
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "specs/method-evidence.yaml"

# The methods available when the promotion rule was introduced (issue #149) that did not meet
# it. `legacy:` in the evidence file must stay within this set. Never add to it.
FROZEN_AT_149 = frozenset(
    {
        "dates.year_fraction",
        "rates.convert",
        "rates.real_nominal",
        "tvm.npv",
        "tvm.present_value",
        "tvm.xnpv",
        "tvm.irr",
        "tvm.xirr",
        "tvm.mirr",
        "tvm.annuity",
        "tvm.perpetuity",
        "tvm.future_value",
        "loans.instalment",
        "loans.number_of_instalments",
        "loans.amortisation_schedule",
        "tv.gordon",
        "tv.value_driver",
        "tv.h_model",
        "tv.exit_multiple",
        "tv.limited_horizon",
        "tv.normalised_income",
        "tv.liquidation",
        "tv.compare",
        "analysis.goal_seek",
        "analysis.sensitivity",
        "analysis.scenarios",
        "analysis.tornado",
    }
)

STATUS_ROW = re.compile(r"^\|\s*Status\s*\|\s*([^|]*?)\s*\|", re.M)
BULLET = r"^[ \t]*(?:[-*+]|\d+[.)])[ \t]+"
UNCHECKED = re.compile(BULLET + r"\[ \][ \t]*(.*)$", re.M)
TASK = re.compile(BULLET + r"\[([ xX])\][ \t]+(T\d+)\b(.*)$")
ANY_BULLET = re.compile(BULLET)
FENCE = re.compile(r"^[ \t]*(```|~~~)")
APPROVED = ("Approved", "Implementing")
ROLE = {"numerical_audit": "numerical-auditor", "code_review": "code-reviewer"}
LABEL = {"numerical_audit": "numerical audit", "code_review": "code review"}
SPEC_KEY = re.compile(r"^\d{4}$")
TASK_ID = re.compile(r"^T\d+$")
ENTRY_KEYS = {"methods", "numerical_audit", "code_review"}
TOP_KEYS = {"specs", "legacy"}
WHERE = "specs/method-evidence.yaml"


@dataclass
class Task:
    ticked: bool
    text: str


def load_evidence(path: Path = EVIDENCE) -> Any:
    return yaml.safe_load(path.read_text(encoding="utf-8"))


def validate_evidence(data: Any) -> list[str]:
    """Shape errors of the evidence file, so that a typo cannot pass as missing evidence."""
    if not isinstance(data, dict):
        return [f"{WHERE}: the top level must be a mapping with `specs` and `legacy`"]
    errors = [f"{WHERE}: unknown top-level key {k!r}" for k in data if k not in TOP_KEYS]
    specs = data.get("specs")
    if not isinstance(specs, dict):
        errors.append(f"{WHERE}: `specs` must be a mapping of spec numbers to entries")
        specs = {}
    for key, entry in specs.items():
        if not isinstance(key, str) or not SPEC_KEY.match(key):
            errors.append(f'{WHERE}: spec key {key!r} must be a quoted four-digit string ("0002")')
            continue
        if not isinstance(entry, dict):
            errors.append(f"{WHERE}: spec {key}: the entry must be a mapping")
            continue
        errors += [f"{WHERE}: spec {key}: unknown key {k!r}" for k in entry if k not in ENTRY_KEYS]
        methods = entry.get("methods")
        if not isinstance(methods, list) or not all(isinstance(m, str) for m in methods):
            errors.append(f"{WHERE}: spec {key}: `methods` must be a list of method ids")
        for field in ("numerical_audit", "code_review"):
            value = entry.get(field)
            if value is not None and (not isinstance(value, str) or not TASK_ID.match(value)):
                errors.append(f"{WHERE}: spec {key}: `{field}` must be null or a task id like T6")
    legacy = data.get("legacy", [])
    if not isinstance(legacy, list) or not all(isinstance(m, str) for m in legacy):
        errors.append(f"{WHERE}: `legacy` must be a list of method ids")
    return errors


def without_fences(text: str) -> str:
    """`text` without fenced code blocks: examples there are not items of the document."""
    out: list[str] = []
    fence: str | None = None
    for line in text.splitlines():
        m = FENCE.match(line)
        if m and (fence is None or m[1] == fence):
            fence = None if fence else m[1]
            continue
        if fence is None:
            out.append(line)
    return "\n".join(out)


def tasks_of(text: str) -> dict[str, list[Task]]:
    """Every definition of every task id, with its indented continuation lines."""
    found: dict[str, list[Task]] = {}
    last: Task | None = None
    for line in without_fences(text).splitlines():
        m = TASK.match(line)
        if m:
            last = Task(m[1] != " ", m[3])
            found.setdefault(m[2], []).append(last)
        elif last and line.strip() and line[:1] in " \t" and not ANY_BULLET.match(line):
            last.text += " " + line.strip()
        else:
            last = None
    return found


def spec_folder(root: Path, number: str) -> Path | None:
    found = sorted((root / "specs").glob(f"{number}-*/spec.md"))
    return found[0].parent if len(found) == 1 else None


def missing_evidence(mid: str, number: str, entry: dict[str, Any], root: Path) -> list[str]:
    """Every condition of the promotion rule that `mid` does not meet, in rule order."""
    folder = spec_folder(root, number)
    if folder is None:
        return [f"spec {number} not found (one folder specs/{number}-*/ with spec.md)"]
    rel = folder.relative_to(root).as_posix()
    spec = (folder / "spec.md").read_text(encoding="utf-8")
    out = []
    status = STATUS_ROW.search(spec)
    if not status or not status[1].startswith(APPROVED):
        out.append(f"spec {number} is not approved (Status: {status[1] if status else 'none'})")
    for item in UNCHECKED.findall(without_fences(spec)):
        out.append(f"spec {number} has an unchecked item: {item.strip()[:60]}")
    if f"`{mid}`" not in spec:
        out.append(f"spec {number} does not name `{mid}`")
    cases = sorted((root / "tests/golden" / mid).glob("*.yaml"))
    if not cases:
        out.append(f"no golden cases in tests/golden/{mid}/")
    for case in cases:
        data = yaml.safe_load(case.read_text(encoding="utf-8"))
        if not isinstance(data, dict):
            out.append(f"golden case tests/golden/{mid}/{case.name} is not a YAML mapping")
        elif not str(data.get("source") or "").strip():
            out.append(f"golden case tests/golden/{mid}/{case.name} has no source")
    tasks_path = folder / "tasks.md"
    tasks = tasks_of(tasks_path.read_text(encoding="utf-8")) if tasks_path.exists() else {}
    for key in ("numerical_audit", "code_review"):
        task_id = entry.get(key)
        if not task_id:
            out.append(f"no {LABEL[key]} recorded for spec {number}")
            continue
        defs = tasks.get(task_id, [])
        if not defs:
            out.append(f"{LABEL[key]} task {task_id} is missing from {rel}/tasks.md")
        elif len(defs) > 1:
            out.append(
                f"{LABEL[key]} task {task_id} is defined {len(defs)} times in {rel}/tasks.md"
            )
        elif ROLE[key] not in defs[0].text:
            out.append(f"task {task_id} in {rel}/tasks.md does not name `{ROLE[key]}`")
        elif not defs[0].ticked:
            out.append(f"{LABEL[key]} task {task_id} is not ticked in {rel}/tasks.md")
    return out


def evaluate(
    methods: list[dict[str, str]],
    evidence: Any,
    root: Path,
    frozen: frozenset[str] = FROZEN_AT_149,
) -> tuple[list[str], list[str], list[tuple[str, str, str, list[str]]]]:
    """Errors, warnings and one report row (id, status, spec, missing) per method."""
    shape = validate_evidence(evidence)
    if shape:
        return shape, [], []
    errors: list[str] = []
    warnings: list[str] = []
    registered = {m["id"]: m["status"] for m in methods}
    legacy = set(evidence.get("legacy") or [])
    spec_of: dict[str, str] = {}
    for number, entry in evidence["specs"].items():
        for mid in entry["methods"]:
            if mid in spec_of:
                errors.append(f"{mid}: listed under more than one spec ({spec_of[mid]}, {number})")
            elif mid not in registered:
                errors.append(f"{mid}: in {WHERE} but not a registered method")
            spec_of.setdefault(mid, number)
    for mid in sorted(legacy - frozen):
        errors.append(f"{mid}: `legacy` may only shrink, and this method was never in it")
    for mid in sorted(legacy - set(registered)):
        errors.append(f"{mid}: in `legacy` but not a registered method")
    rows = []
    for mid, status in registered.items():
        number = spec_of.get(mid)
        if number is None:
            missing = [f"no entry in {WHERE}"]
        else:
            missing = missing_evidence(mid, number, evidence["specs"][number], root)
        rows.append((mid, status, number or "", missing))
        if mid in legacy:
            if status != "available":
                errors.append(f"{mid}: preview, so remove it from `legacy`")
            elif not missing:
                errors.append(f"{mid}: now meets the promotion rule, so remove it from `legacy`")
            else:
                warnings.append(f"{mid}: available before the rule, missing: {'; '.join(missing)}")
        elif status == "available":
            errors += [f"{mid}: {m}" for m in missing]
    return errors, warnings, rows


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("command", choices=["check", "report"])
    args = parser.parse_args()
    import fairmark as fm  # needs the project environment

    errors, warnings, rows = evaluate(fm.methods(), load_evidence(), ROOT)
    if args.command == "report":
        print("| Method | Status | Spec | Missing evidence |")
        print("| --- | --- | --- | --- |")
        for mid, status, number, missing in rows:
            print(f"| `{mid}` | {status} | {number} | {'; '.join(missing) or 'none'} |")
        eligible = [mid for mid, status, _, missing in rows if status == "preview" and not missing]
        print(f"\nPreview methods eligible for promotion: {', '.join(eligible) or 'none'}.")
    if warnings and args.command == "check":
        print(f"{len(warnings)} methods are available on legacy grounds (`legacy:` in {WHERE}):")
        print("\n".join(f"  - {w}" for w in warnings))
    if errors:
        print("Method status violates the promotion rule (docs/en/project/releases.md):")
        print("\n".join(f"  - {e}" for e in errors))
        sys.exit(1)
    if args.command == "check":
        available = sum(1 for _, s, _, _ in rows if s == "available")
        print(
            f"Method status check passed: {available} available, {len(rows) - available} preview."
        )


if __name__ == "__main__":
    main()
