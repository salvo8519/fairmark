"""JSON Schema snapshots of every method input — Fairmark's equivalent of migrations.

    python scripts/schema_snapshots.py            # write schemas/<method>.json
    python scripts/schema_snapshots.py --check    # fail if snapshots are stale
    python scripts/schema_snapshots.py --diff-against <dir>
        # classify changes vs another snapshot dir (e.g. the base branch):
        # exit 3 on BREAKING changes, 0 otherwise

Breaking: a field removed, a new required field, a type or enum narrowed, a default changed.
Additive: a new optional field, a new method, documentation-only changes.
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path
from typing import Any

import fairmark as fm

ROOT = Path(__file__).resolve().parents[1]
SCHEMAS = ROOT / "schemas"


def current() -> dict[str, dict[str, Any]]:
    return {m["id"]: fm.input_schema(m["id"]) for m in fm.methods()}


def dump(schema: dict[str, Any]) -> str:
    return json.dumps(schema, indent=2, ensure_ascii=False, sort_keys=True) + "\n"


def load(directory: Path) -> dict[str, dict[str, Any]]:
    return {p.stem: json.loads(p.read_text()) for p in sorted(directory.glob("*.json"))}


def classify(old: dict[str, dict[str, Any]], new: dict[str, dict[str, Any]]) -> list[str]:
    breaking: list[str] = []
    for method, old_schema in old.items():
        if method not in new:
            breaking.append(f"{method}: method removed")
            continue
        o_props, n_props = old_schema.get("properties", {}), new[method].get("properties", {})
        o_req, n_req = set(old_schema.get("required", [])), set(new[method].get("required", []))
        for field in o_props.keys() - n_props.keys():
            breaking.append(f"{method}.{field}: field removed")
        for field in n_req - o_req:
            breaking.append(f"{method}.{field}: now required")
        for field in o_props.keys() & n_props.keys():
            a, b = o_props[field], n_props[field]
            for key in ("type", "enum", "$ref", "default", "items"):
                if a.get(key) != b.get(key):
                    breaking.append(f"{method}.{field}: '{key}' changed")
    return breaking


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--diff-against", type=Path)
    args = parser.parse_args()
    schemas = current()

    if args.diff_against:
        breaking = classify(load(args.diff_against), schemas)
        if breaking:
            print("BREAKING schema changes:\n  " + "\n  ".join(breaking))
            sys.exit(3)
        print("Schema changes are additive or none.")
        return

    if args.check:
        stale = [m for m, s in schemas.items() if (SCHEMAS / f"{m}.json").read_text() != dump(s)]
        stale += [p.stem for p in SCHEMAS.glob("*.json") if p.stem not in schemas]
        if stale:
            sys.exit(f"Stale schema snapshots: {stale}. Run: uv run just generate")
        print("Schema snapshots up to date.")
        return

    SCHEMAS.mkdir(exist_ok=True)
    for old in SCHEMAS.glob("*.json"):
        if old.stem not in schemas:
            old.unlink()
    for method, schema in schemas.items():
        (SCHEMAS / f"{method}.json").write_text(dump(schema))


if __name__ == "__main__":
    main()
