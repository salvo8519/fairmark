"""Check the status of every method against the promotion rule (ADR 0011).

The engine registry holds the status (`AVAILABLE` in crates/fairmark-core/src/registry.rs);
the evidence is in specs/method-evidence.yaml; the rule is in docs/en/project/releases.md,
"Method status". An available method must have:

  1. an approved spec (Status row starting with `Approved` or `Implementing`);
  2. no unchecked item (`- [ ]`) in that spec;
  3. its id named in that spec;
  4. golden cases, each with a `source`;
  5. a ticked numerical-audit task and 6. a ticked code-review task in the spec's tasks.md.

    python scripts/method_status.py check    # CI and `just check`: fail on a violation
    python scripts/method_status.py report   # evidence of every method, eligible promotions
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
EVIDENCE = ROOT / "specs/method-evidence.yaml"

# Methods available before the promotion rule was introduced (issue #149) that do not yet
# meet it. Reported on every run. This set may only shrink: the check fails when one of them
# meets the rule or becomes preview. Never add a method to it.
LEGACY = frozenset(
    {
        # SPEC-0002: no code review recorded in tasks.md; tvm.npv and tvm.irr also have
        # golden cases without a source.
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
        # SPEC-0006 T17, SPEC-0008 T25, SPEC-0009 T23: audit and review not yet ticked.
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
        "tvm.future_value",
        "loans.instalment",
        "loans.number_of_instalments",
        "loans.amortisation_schedule",
    }
)

STATUS_ROW = re.compile(r"^\|\s*Status\s*\|\s*([^|]*?)\s*\|", re.M)
UNCHECKED = re.compile(r"^\s*- \[ \] (.*)$", re.M)
TASK = re.compile(r"^- \[([ xX])\] (T\d+)\b(.*)$", re.M)
APPROVED = ("Approved", "Implementing")
KEYWORDS = {"numerical_audit": ("numerical", "audit"), "code_review": ("review",)}
LABEL = {"numerical_audit": "numerical audit", "code_review": "code review"}


def load_evidence(path: Path = EVIDENCE) -> dict[str, Any]:
    data: dict[str, Any] = yaml.safe_load(path.read_text(encoding="utf-8"))
    return data


def spec_folder(root: Path, number: str) -> Path | None:
    found = sorted((root / "specs").glob(f"{number}-*/spec.md"))
    return found[0].parent if len(found) == 1 else None


def task_line(tasks: str, task_id: str) -> tuple[bool, str] | None:
    """(ticked, text) of a task, or None when the id is missing or ambiguous."""
    lines = [(m[1] != " ", m[3]) for m in TASK.finditer(tasks) if m[2] == task_id]
    return lines[0] if len(lines) == 1 else None


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
    for item in UNCHECKED.findall(spec):
        out.append(f"spec {number} has an unchecked item: {item.strip()[:60]}")
    if f"`{mid}`" not in spec:
        out.append(f"spec {number} does not name `{mid}`")
    cases = sorted((root / "tests/golden" / mid).glob("*.yaml"))
    if not cases:
        out.append(f"no golden cases in tests/golden/{mid}/")
    for case in cases:
        data = yaml.safe_load(case.read_text(encoding="utf-8")) or {}
        if not str(data.get("source") or "").strip():
            out.append(f"golden case tests/golden/{mid}/{case.name} has no source")
    tasks_path = folder / "tasks.md"
    tasks = tasks_path.read_text(encoding="utf-8") if tasks_path.exists() else ""
    for key in ("numerical_audit", "code_review"):
        task_id = entry.get(key)
        if not task_id:
            out.append(f"no {LABEL[key]} recorded for spec {number}")
            continue
        line = task_line(tasks, str(task_id))
        if line is None:
            out.append(f"{LABEL[key]} task {task_id} not found once in {rel}/tasks.md")
        elif not any(k in line[1].lower() for k in KEYWORDS[key]):
            out.append(f"task {task_id} in {rel}/tasks.md does not mention a {LABEL[key]}")
        elif not line[0]:
            out.append(f"{LABEL[key]} task {task_id} is not ticked in {rel}/tasks.md")
    return out


def evaluate(
    methods: list[dict[str, str]],
    evidence: dict[str, Any],
    root: Path,
    legacy: frozenset[str] = LEGACY,
) -> tuple[list[str], list[str], list[tuple[str, str, str, list[str]]]]:
    """Errors, warnings and one report row (id, status, spec, missing) per method."""
    errors: list[str] = []
    warnings: list[str] = []
    registered = {m["id"]: m["status"] for m in methods}
    spec_of: dict[str, str] = {}
    for number, entry in (evidence.get("specs") or {}).items():
        for mid in entry.get("methods") or []:
            if mid in spec_of:
                errors.append(f"{mid}: listed under more than one spec ({spec_of[mid]}, {number})")
            elif mid not in registered:
                errors.append(f"{mid}: in specs/method-evidence.yaml but not a registered method")
            spec_of.setdefault(mid, str(number))
    for mid in sorted(legacy - set(registered)):
        errors.append(f"{mid}: in LEGACY but not a registered method")
    rows = []
    for mid, status in registered.items():
        number = spec_of.get(mid)
        if number is None:
            missing = ["no entry in specs/method-evidence.yaml"]
        else:
            missing = missing_evidence(mid, number, evidence["specs"][number], root)
        rows.append((mid, status, number or "", missing))
        if mid in legacy:
            if status != "available":
                errors.append(f"{mid}: preview, so remove it from LEGACY")
            elif not missing:
                errors.append(f"{mid}: now meets the promotion rule, so remove it from LEGACY")
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
        print(f"{len(warnings)} methods are available on legacy grounds (see LEGACY):")
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
