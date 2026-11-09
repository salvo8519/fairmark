"""Versioned JSON Schema snapshots of every method input and output: Fairmark's migrations.

Layout (ADR 0009, ADR 0014 G8):

    schemas/index.json                 history: current version, files, hashes, release
    schemas/<method>/v<N>.json         the input schema of version N of <method>
    schemas/<method>/v<N>.output.json  the output schema (the result `value`) of version N

Rules, the same for inputs and outputs:
- the schemas of the current version are regenerated from the engine; files of earlier
  versions are never rewritten or deleted;
- a version present on the base branch may change only additively; a breaking change
  requires a new method version, hence new files;
- the current version of a method never goes back to an earlier number;
- a method is never withdrawn silently: removing one is a breaking change;
- a new version is recorded as `introduced_in: "unreleased"`; `scripts/version.py bump`
  stamps it with the release that publishes it.

    python scripts/schema_snapshots.py                       # write current snapshots and index
    python scripts/schema_snapshots.py --check               # fail if anything is stale
    python scripts/schema_snapshots.py --stamp-release X.Y.Z # used by scripts/version.py bump
    python scripts/schema_snapshots.py --diff-against <dir>  # compare with the base branch's
        schemas/ directory: exit 3 on a breaking change, 4 if the history was rewritten
        (an earlier version changed or disappeared, an index entry was altered, a current
        version went backwards), 0 otherwise

Breaking for an input: a field removed, a new required field, a changed constraint keyword
(type, const, default, format, reference, bounds, lengths, additionalProperties; see
CONSTRAINT_KEYS), a value removed from an enumeration, all of these also inside `$defs`,
`items`, `anyOf` and `oneOf`; a method removed (methods are deprecated, never removed).
Additive for an input: a new optional field, a value added to an enumeration (`enum`, or
`oneOf` of `const` as schemars generates documented enums), a new method or version,
documentation.

Breaking for an output: a field removed or renamed, a field retyped (any change of a
constraint keyword, which includes becoming nullable: `null` added to `type` or an `anyOf`
with `null`), a field that was always present and may now be absent (no longer `required`), a
value removed from an enumeration; also inside `$defs`, `items`, `anyOf` and `oneOf`.
Additive for an output: a new field (required or not), a value added to an enumeration
(maintainer decision of 2026-11-09, ADR 0014 G8: clients accept enumeration values they do
not know), documentation. The output snapshot of a version is recorded in the index as `output_file`
and `output_sha256`; versions published before output snapshots existed have none.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
SCHEMAS = ROOT / "schemas"
INDEX = SCHEMAS / "index.json"
UNRELEASED = "unreleased"
RELEASE = re.compile(r"^\d+\.\d+\.\d+(-[0-9A-Za-z.-]+)?$")

# Keywords whose change alters what a consumer may send. Documentation keywords (title,
# description, examples) are deliberately absent. Any change to one of these is treated as
# breaking, including some that only widen (a lower `minimum`, `additionalProperties` relaxed):
# conservative on purpose, the cost is a version bump, never a silent incompatibility.
CONSTRAINT_KEYS = (
    "type",
    "enum",
    "const",
    "default",
    "format",
    "$ref",
    "pattern",
    "additionalProperties",
    "minimum",
    "maximum",
    "exclusiveMinimum",
    "exclusiveMaximum",
    "multipleOf",
    "minItems",
    "maxItems",
    "minLength",
    "maxLength",
    # Fields holding another method's input or output (`analysis.goal_seek`): changing the
    # method they name changes how a client decodes them.
    "x-fairmark-input-of",
    "x-fairmark-output-of",
)
SUBSCHEMA_LISTS = ("anyOf", "oneOf", "allOf")


def snapshot(method: str, version: str, schema: dict[str, Any]) -> dict[str, Any]:
    """The schema as stored: a stable identifier and the method version, then the schema."""
    return {
        "$id": f"urn:fairmark:schema:{method}:v{version}",
        "x-fairmark-method": method,
        "x-fairmark-method-version": version,
        **schema,
    }


def output_snapshot(method: str, version: str, schema: dict[str, Any]) -> dict[str, Any]:
    """The output schema as stored, identified like the input schema with an `:output` suffix."""
    return {
        "$id": f"urn:fairmark:schema:{method}:v{version}:output",
        "x-fairmark-method": method,
        "x-fairmark-method-version": version,
        **schema,
    }


def output_file(method: str, version: str) -> str:
    return f"{method}/v{version}.output.json"


def dump(data: dict[str, Any]) -> str:
    return json.dumps(data, indent=2, ensure_ascii=False, sort_keys=True) + "\n"


def sha256(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def current() -> dict[str, tuple[str, str, str]]:
    """method id -> (version, input file text, output file text) for the engine as built."""
    import fairmark as fm  # imported here: --stamp-release must work without the built engine

    out = {}
    for m in fm.methods():
        version = str(m["version"])
        out[m["id"]] = (
            version,
            dump(snapshot(m["id"], version, fm.input_schema(m["id"]))),
            dump(output_snapshot(m["id"], version, fm.output_schema(m["id"]))),
        )
    return out


def load_index(path: Path) -> dict[str, Any]:
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else {"methods": {}}


def build_index(schemas: dict[str, tuple[str, str, str]], index_path: Path) -> dict[str, Any]:
    index = load_index(index_path)
    methods: dict[str, Any] = index.setdefault("methods", {})
    for method, (version, text, output_text) in schemas.items():
        entry = methods.setdefault(method, {"versions": {}})
        entry["current"] = version
        record = entry["versions"].get(version, {"introduced_in": UNRELEASED})
        record["file"] = f"{method}/v{version}.json"
        record["sha256"] = sha256(text)
        record["output_file"] = output_file(method, version)
        record["output_sha256"] = sha256(output_text)
        entry["versions"][version] = record
    index["format"] = 1
    index["methods"] = dict(sorted(methods.items()))
    return index


def dump_index(index: dict[str, Any]) -> str:
    """Index text, keys sorted, version keys in numeric order (v10 after v9, not after v1).

    `sort_keys` would put "10" before "9", so the ordering is built explicitly.
    """

    def ordered(value: Any, numeric: bool = False) -> Any:
        if not isinstance(value, dict):
            return value
        key = (lambda kv: int(kv[0])) if numeric else (lambda kv: kv[0])
        return {k: ordered(v, numeric=(k == "versions")) for k, v in sorted(value.items(), key=key)}

    return json.dumps(ordered(index), indent=2, ensure_ascii=False) + "\n"


# ---- classification of changes


def const_members(members: Any) -> dict[str, dict[str, Any]] | None:
    """`oneOf`/`anyOf` whose members are all `const` (a documented enum): value -> member."""
    if not isinstance(members, list) or not members:
        return None
    if not all(isinstance(m, dict) and "const" in m for m in members):
        return None
    return {json.dumps(m["const"]): m for m in members}


def classify(where: str, old: Any, new: Any, output: bool = False) -> list[str]:
    """Breaking differences between two (sub)schemas, recursively.

    For an input (`output=False`) a field that becomes required is breaking: a client that
    omitted it is refused. For an output (`output=True`) a field that stops being required is
    breaking instead: a client that read it may find it absent.
    """
    if not isinstance(old, dict) or not isinstance(new, dict):
        return [] if old == new else [f"{where}: changed"]
    breaking: list[str] = []
    o_props, n_props = old.get("properties", {}), new.get("properties", {})
    for field in sorted(o_props.keys() - n_props.keys()):
        breaking.append(f"{where}.{field}: field removed")
    o_req, n_req = set(old.get("required", [])), set(new.get("required", []))
    if output:
        for field in sorted((o_req - n_req) & n_props.keys()):
            breaking.append(f"{where}.{field}: may now be absent")
    else:
        for field in sorted(n_req - o_req):
            breaking.append(f"{where}.{field}: now required")
    for field in sorted(o_props.keys() & n_props.keys()):
        breaking += classify(f"{where}.{field}", o_props[field], n_props[field], output)
    for key in CONSTRAINT_KEYS:
        if old.get(key) != new.get(key):
            if key == "enum" and set(old.get("enum") or []) <= set(new.get("enum") or []):
                continue  # a widened enumeration accepts everything it accepted before
            breaking.append(f"{where}: '{key}' changed")
    if "items" in old or "items" in new:
        breaking += classify(f"{where}[]", old.get("items"), new.get("items"), output)
    for key in SUBSCHEMA_LISTS:
        a, b = old.get(key), new.get(key)
        if a is None and b is None:
            continue
        old_values, new_values = const_members(a), const_members(b)
        if old_values is not None and new_values is not None:
            for value in old_values.keys() - new_values.keys():
                breaking.append(f"{where}: value {value!r} removed")
            for value in old_values.keys() & new_values.keys():
                if old_values[value].get("type") != new_values[value].get("type"):
                    breaking.append(f"{where}: type of value {value!r} changed")
            continue
        if not isinstance(a, list) or not isinstance(b, list) or len(a) != len(b):
            breaking.append(f"{where}: '{key}' changed")
            continue
        for i, (x, y) in enumerate(zip(a, b, strict=True)):
            breaking += classify(f"{where}.{key}[{i}]", x, y, output)
    o_defs, n_defs = old.get("$defs", {}), new.get("$defs", {})
    for name in sorted(o_defs):
        if name not in n_defs:
            breaking.append(f"{where}.$defs.{name}: definition removed")
        else:
            breaking += classify(f"{where}.$defs.{name}", o_defs[name], n_defs[name], output)
    return breaking


def report(title: str, lines: list[str]) -> None:
    print(title + ":\n  " + "\n  ".join(lines))


def diff_against(base: Path) -> int:
    schemas = current()
    base_index_path = base / "index.json"
    if not base_index_path.exists():
        return diff_against_flat(base, schemas)

    base_methods = load_index(base_index_path).get("methods", {})
    head_methods = load_index(INDEX).get("methods", {})
    rewritten, breaking, removed = [], [], []
    for method, base_entry in base_methods.items():
        if method not in schemas:
            removed.append(method)
        head_entry = head_methods.get(method)
        if head_entry is None:
            rewritten.append(f"{method}: entry removed from the index")
            continue
        if int(head_entry["current"]) < int(base_entry["current"]):
            rewritten.append(
                f"{method}: current version went back from v{base_entry['current']} "
                f"to v{head_entry['current']}"
            )
        for version, record in base_entry.get("versions", {}).items():
            label = record["file"]
            head_record = head_entry.get("versions", {}).get(version)
            if head_record is None:
                rewritten.append(f"{label}: removed from the index")
                continue
            if head_record["file"] != record["file"]:
                rewritten.append(f"{label}: file name changed in the index")
            released, stamped = record["introduced_in"], head_record["introduced_in"]
            if stamped != released and (released != UNRELEASED or not RELEASE.match(stamped)):
                rewritten.append(f"{label}: introduced_in changed from {released} to {stamped}")
            is_current = method in schemas and schemas[method][0] == version
            files = [("file", "sha256", False)]
            if "output_file" in record:  # versions published before #265 have no output snapshot
                if head_record.get("output_file") != record["output_file"]:
                    # Reported; the input snapshot of the version is still checked below.
                    rewritten.append(f"{record['output_file']}: file name changed in the index")
                else:
                    files.append(("output_file", "output_sha256", True))
            for file_key, hash_key, is_output in files:
                name = record[file_key]
                old_file, new_file = base / name, SCHEMAS / name
                if not new_file.exists():
                    rewritten.append(f"{name}: removed")
                    continue
                if old_file.read_text(encoding="utf-8") == new_file.read_text(encoding="utf-8"):
                    if head_record.get(hash_key) != record[hash_key]:
                        rewritten.append(f"{name}: hash changed in the index")
                    continue
                if not is_current:
                    rewritten.append(f"{name}: an earlier version changed")
                    continue
                problems = classify(
                    f"{method} output" if is_output else method,
                    json.loads(old_file.read_text(encoding="utf-8")),
                    json.loads(new_file.read_text(encoding="utf-8")),
                    output=is_output,
                )
                breaking += [f"v{version} {p}" for p in problems]
    if rewritten:
        report("Schema history rewritten (published versions are immutable)", rewritten)
        return 4
    if removed:
        report(
            "Methods removed (an integration may depend on them: keep the method and mark it "
            "deprecated in its description instead)",
            removed,
        )
        return 3
    if breaking:
        report(
            "BREAKING changes to existing schema versions (bump the method version so that new "
            "schemas/<method>/v<N>.json and v<N>.output.json are created)",
            breaking,
        )
        return 3
    print("Schema changes are additive, new versions, or none.")
    return 0


def diff_against_flat(base: Path, schemas: dict[str, tuple[str, str, str]]) -> int:
    """Base branch still on the flat layout (schemas/<method>.json), before ADR 0009."""
    breaking = []
    for flat in sorted(base.glob("*.json")):
        method = flat.stem
        if method not in schemas:
            breaking.append(f"{method}: method removed")
            continue
        breaking += classify(
            method, json.loads(flat.read_text(encoding="utf-8")), json.loads(schemas[method][1])
        )
    if breaking:
        report("BREAKING schema changes", breaking)
        return 3
    print("Schema changes are additive or none (base used the flat layout).")
    return 0


# ---- check, write, stamp


def check() -> int:
    schemas = current()
    problems = []
    for method, (version, text, output_text) in schemas.items():
        path = SCHEMAS / method / f"v{version}.json"
        if not path.exists() or path.read_text(encoding="utf-8") != text:
            problems.append(f"{method} v{version}: snapshot missing or stale")
        path = SCHEMAS / output_file(method, version)
        if not path.exists() or path.read_text(encoding="utf-8") != output_text:
            problems.append(f"{method} v{version}: output snapshot missing or stale")
    if not INDEX.exists() or INDEX.read_text(encoding="utf-8") != dump_index(
        build_index(schemas, INDEX)
    ):
        problems.append("schemas/index.json is stale")
    indexed = set()
    for entry in load_index(INDEX).get("methods", {}).values():
        for record in entry.get("versions", {}).values():
            pairs = [("file", "sha256"), ("output_file", "output_sha256")]
            for file_key, hash_key in pairs:
                if file_key not in record:
                    continue
                indexed.add(record[file_key])
                path = SCHEMAS / record[file_key]
                if not path.exists():
                    problems.append(f"{record[file_key]}: listed in the index but missing")
                elif sha256(path.read_text(encoding="utf-8")) != record.get(hash_key):
                    problems.append(f"{record[file_key]}: content does not match its recorded hash")
    for path in sorted(SCHEMAS.glob("*/v*.json")):
        rel = path.relative_to(SCHEMAS).as_posix()
        if rel not in indexed:
            problems.append(f"{rel}: not recorded in schemas/index.json")
    stray = [p for p in SCHEMAS.glob("*.json") if p.name != "index.json"]
    problems += [f"{p.name}: stray flat snapshot (use schemas/<method>/v<N>.json)" for p in stray]
    if problems:
        report("Schema snapshots are out of date", problems)
        print("Run: uv run just generate")
        return 1
    print(f"Schema snapshots up to date ({len(schemas)} methods, versioned).")
    return 0


def write() -> int:
    schemas = current()
    old_methods = load_index(INDEX).get("methods", {})
    refused = []
    for method, (version, text, output_text) in schemas.items():
        entry = old_methods.get(method)
        record = (entry or {}).get("versions", {}).get(version)
        if entry and int(version) < int(entry["current"]):
            refused.append(f"{method}: current version would go back to v{version}")
        elif (
            entry
            and record
            and entry["current"] != version
            and (
                record["sha256"] != sha256(text)
                or record.get("output_sha256", sha256(output_text)) != sha256(output_text)
            )
        ):
            refused.append(f"{method} v{version}: an earlier version would be overwritten")
    if refused:
        report(
            "Refusing to rewrite schema history (bump the method version instead; to undo a "
            "bump not yet on main, restore schemas/ from main first)",
            refused,
        )
        return 4
    for method, (version, text, output_text) in schemas.items():
        path = SCHEMAS / method / f"v{version}.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(text, encoding="utf-8", newline="\n")
        out = SCHEMAS / output_file(method, version)
        out.write_text(output_text, encoding="utf-8", newline="\n")
    INDEX.write_text(dump_index(build_index(schemas, INDEX)), encoding="utf-8", newline="\n")
    return 0


def stamp_release(version: str) -> int:
    """Record the release that publishes every version still marked as unreleased."""
    index = load_index(INDEX)
    stamped = 0
    for entry in index.get("methods", {}).values():
        for record in entry.get("versions", {}).values():
            if record.get("introduced_in") == UNRELEASED:
                record["introduced_in"] = version
                stamped += 1
    INDEX.write_text(dump_index(index), encoding="utf-8", newline="\n")
    return stamped


def main() -> None:
    parser = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    parser.add_argument("--check", action="store_true")
    parser.add_argument("--diff-against", type=Path)
    parser.add_argument("--stamp-release", metavar="VERSION")
    args = parser.parse_args()
    if args.diff_against:
        sys.exit(diff_against(args.diff_against))
    if args.check:
        sys.exit(check())
    if args.stamp_release:
        print(f"{stamp_release(args.stamp_release)} schema versions stamped {args.stamp_release}")
        return
    sys.exit(write())


if __name__ == "__main__":
    main()
