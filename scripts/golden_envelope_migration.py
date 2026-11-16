"""Golden key migration of SPEC-0012 (one envelope with diagnostics), task T3.

    python scripts/golden_envelope_migration.py list [--markdown FILE] [--plan FILE]
    python scripts/golden_envelope_migration.py apply FILE... [--plan FILE]
    python scripts/golden_envelope_migration.py verify [FILE...] [--plan FILE] [--base REF]

SPEC-0012 moves `not_computed`, `checks` and `not_defined` out of `value` into `diagnostics`,
renames the pointer fields of R12 and the key `field` of `meta.defaults_applied` to `pointer`.
No computed number changes, but the keys of the golden cases that assert them move.

- `list` runs the engine as it is before the change on every golden input concerned, computes
  where each expectation moves (the index of an entry in `diagnostics.not_computed` and
  `diagnostics.checks` follows the order of R7 and R13), writes the plan as JSON (`--plan`,
  the machine-readable list `apply` and `verify` read) and prints it as Markdown (or writes it
  to `--markdown`), for the maintainer's named approval. It refuses to run on an engine that
  already writes `diagnostics`: the committed plan is then the reference.
- `apply` rewrites only the files named on the command line, and only when they are still the
  files the plan was computed from. Lines that do not move are kept byte for byte.
- `verify` checks that each file of the plan is either unchanged from `--base` or exactly the
  plan applied to it: every value read at its new key equals the value at its old key on the
  base, except the changes the plan flags (a result pointer gains the root `/value`; a list
  length counts the entries that join it). Nothing else in the file may change (AC16).

Golden files are protected (AGENTS.md): `apply` is run only on the files the maintainer has
approved by name.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import subprocess
import sys
from collections.abc import Callable, Iterable
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
GOLDEN = Path("tests/golden")
SPEC_DIR = Path("specs/0012-envelope-diagnostics")
PLAN = SPEC_DIR / "golden-migration.json"
MARKDOWN = SPEC_DIR / "golden-migration.md"

SECTIONS = ("expect", "expect_length", "expect_meta")
NEW_EXPECT = "expect_diagnostics"
NEW_LENGTH = "expect_diagnostics_length"
RATIOS = {
    "ratios.profitability",
    "ratios.capital_structure",
    "ratios.liquidity",
    "ratios.working_capital",
    "ratios.debt_service",
    "ratios.covenants",
}
STATEMENTS = "statements.reorganise"
# R12: pointer fields of statements.reorganise v1 renamed in v2, by the type that holds them.
RENAMED_FIELDS = {
    ("Assignment", "aggregate"): "aggregate_pointer",
    ("AppliedAdjustment", "source"): "source_pointer",
    ("NfpComponent", "source"): "source_pointer",
    ("ReconciliationLine", "sources"): "source_pointers",
}
DEFAULTS_FIELD = re.compile(r"^/defaults_applied/(\d+)/field$")

Runner = Callable[[str, Any], dict[str, Any]]


@dataclass(frozen=True)
class Change:
    """One expectation of a golden file and where it goes.

    `kind`: `rename` (same section, new key), `move` (to a diagnostics section, same value),
    `move_value_changes` (moved, and the value changes as `note` says), `add` (an expectation
    added because the old key carried it, for example the relation id of a check),
    `drop` (removed: the new contract has no key for it, and `note` names the expectation of
    the same case that still asserts it), `unmapped` (no rule: the migration stops).
    """

    kind: str
    section: str
    key: str
    value: Any
    new_section: str
    new_key: str
    new_value: Any
    note: str = ""


# ---------------------------------------------------------------------------- pointers


def tokens(pointer: str) -> list[str]:
    if pointer == "":
        return []
    return [t.replace("~1", "/").replace("~0", "~") for t in pointer[1:].split("/")]


def pointer_of(parts: Iterable[str | int]) -> str:
    return "".join("/" + str(p).replace("~", "~0").replace("/", "~1") for p in parts)


def pointer_key(pointer: str) -> list[tuple[int, int, str]]:
    """Sort key of `ratios::pointer_cmp`: array indices as numbers, other tokens as text."""
    return [
        (0, int(t), "") if t and t.isascii() and t.isdigit() else (1, 0, t)
        for t in pointer.split("/")
    ]


# ---------------------------------------------------------------------------- schema walk


def def_at(schema: dict[str, Any], parts: list[str]) -> str | None:
    """Name of the `$defs` type of the object at `parts` of a value of `schema`, if any."""
    defs = schema.get("$defs", {})

    def step(node: Any, name: str | None, rest: list[str]) -> str | None:
        if not isinstance(node, dict):
            return None
        if "$ref" in node:
            ref = node["$ref"].rsplit("/", 1)[-1]
            return step(defs.get(ref), ref, rest)
        for key in ("anyOf", "oneOf", "allOf"):
            for branch in node.get(key, []):
                found = step(branch, name, rest)
                if found is not None:
                    return found
        if not rest:
            return name
        head, tail = rest[0], rest[1:]
        if head.isdigit() and "items" in node:
            return step(node["items"], None, tail)
        props = node.get("properties", {})
        if head in props:
            return step(props[head], None, tail)
        extra = node.get("additionalProperties")
        if isinstance(extra, dict):
            return step(extra, None, tail)
        return None

    return step(schema, None, parts)


def renamed_key(schema: dict[str, Any], key: str) -> str | None:
    """`key` with the pointer fields of R12 renamed, or None when nothing is renamed."""
    parts = tokens(key)
    changed = False
    for i in range(1, len(parts)):
        holder = def_at(schema, parts[:i])
        new = RENAMED_FIELDS.get((holder or "", parts[i]))
        if new:
            parts[i] = new
            changed = True
    return pointer_of(parts) if changed else None


# ---------------------------------------------------------------------------- the old results


def ratio_entries(value: dict[str, Any]) -> list[dict[str, Any]]:
    """The future `diagnostics.not_computed` of a ratios result of today, in the order of R13."""
    entries = [
        {"pointer": "/value" + e["output"], "old": ("list", i)}
        for i, e in enumerate(value.get("not_computed") or [])
    ]

    listed = {e["pointer"] for e in entries}

    def walk(node: Any, at: list[str | int]) -> None:
        if isinstance(node, dict):
            # An output listed as not computed is listed once (R5), even when its ratio also
            # carries a `not_defined` text today.
            if node.get("not_defined") is not None and "/value" + pointer_of(at) not in listed:
                entries.append({"pointer": "/value" + pointer_of(at), "old": ("not_defined", at)})
            for k, v in node.items():
                if k != "not_computed":
                    walk(v, [*at, k])
        elif isinstance(node, list):
            for i, v in enumerate(node):
                walk(v, [*at, i])

    walk(value, [])
    entries.sort(key=lambda e: pointer_key(e["pointer"]))
    return entries


def statement_entries(value: dict[str, Any]) -> tuple[list[dict[str, Any]], list[dict[str, Any]]]:
    """The future `diagnostics.not_computed` and `diagnostics.checks` of a statements result of
    today: not computed in the order of R13, checks by year then relation id (R7)."""
    not_computed = []
    checks = []
    for y, year in enumerate(value.get("years", [])):
        for group, items in (year.get("not_computed") or {}).items():
            for k, e in enumerate(items or []):
                not_computed.append({"pointer": "/value" + e["output"], "old": (y, group, k)})
        for relation in sorted(year.get("checks") or {}):
            checks.append({"id": relation, "pointer": f"/value/years/{y}", "old": (y, relation)})
    not_computed.sort(key=lambda e: pointer_key(e["pointer"]))
    return not_computed, checks


def old_engine_result(run: Runner, method: str, data: Any) -> dict[str, Any]:
    result = run(method, data)
    if "diagnostics" in result:
        raise SystemExit(
            "the engine already writes `diagnostics`: the plan must be computed with the engine "
            f"before SPEC-0012; use the committed {PLAN}"
        )
    return result


# ---------------------------------------------------------------------------- the plan


NOT_COMPUTED_FIELD = [
    (re.compile(r"^output$"), lambda m: "pointer"),
    (re.compile(r"^missing/(\d+)$"), lambda m: f"missing_any_of/0/{m.group(1)}"),
    (re.compile(r"^missing$"), lambda m: "missing_any_of/0"),
    (re.compile(r"^permitted/(\d+)/treatment$"), lambda m: f"permitted/{m.group(1)}/pointer"),
    (
        re.compile(r"^permitted/(\d+)/destinations(/\d+)?$"),
        lambda m: f"permitted/{m.group(1)}/values{m.group(2) or ''}",
    ),
    (re.compile(r"^reason$"), lambda m: "reason"),
]


def entry_field(rest: str) -> str | None:
    for pattern, target in NOT_COMPUTED_FIELD:
        m = pattern.match(rest)
        if m:
            return target(m)
    return None


def plan_case(case: dict[str, Any], run: Runner, statements_schema: dict[str, Any]) -> list[Change]:
    """The changes of one golden case (empty when nothing moves)."""
    method = case["method"]
    changes: list[Change] = []
    for key, value in (case.get("expect_meta") or {}).items():
        m = DEFAULTS_FIELD.match(key)
        if m:
            new = f"/defaults_applied/{m.group(1)}/pointer"
            changes.append(Change("rename", "expect_meta", key, value, "expect_meta", new, value))
    if "expect_error" in case:
        return changes
    if method.startswith("analysis."):
        for section in SECTIONS:
            for key in case.get(section) or {}:
                if re.search(r"/(not_computed|not_defined|checks)(/|$)", key):
                    changes.append(Change("unmapped", section, key, None, "", "", None))
        return changes
    if method not in RATIOS and method != STATEMENTS:
        return changes
    concerned = any(
        re.search(r"/(not_computed|not_defined|checks)(/|$)", k)
        or (method == STATEMENTS and renamed_key(statements_schema, k))
        for section in ("expect", "expect_length")
        for k in case.get(section) or {}
    )
    if not concerned:
        return changes
    value = old_engine_result(run, method, case["input"])["value"]
    if method in RATIOS:
        changes += plan_ratios(case, value)
    else:
        changes += plan_statements(case, value, statements_schema)
    return changes


def identity(
    changes: list[Change], index: int, fields: dict[str, Any], why: str, base: str
) -> None:
    """Adds the identity fields of entry `index` unless a change already asserts them."""
    for name, val in fields.items():
        key = f"{base}/{index}/{name}"
        if not any(c.new_section == NEW_EXPECT and c.new_key == key for c in changes):
            changes.append(Change("add", "", "", None, NEW_EXPECT, key, val, why))


def plan_ratios(case: dict[str, Any], value: dict[str, Any]) -> list[Change]:
    entries = ratio_entries(value)
    by_list = {e["old"][1]: n for n, e in enumerate(entries) if e["old"][0] == "list"}
    by_path = {pointer_of(e["old"][1]): n for n, e in enumerate(entries) if e["old"][0] != "list"}
    listed = {e["pointer"] for e in entries if e["old"][0] == "list"}
    changes: list[Change] = []
    touched: set[int] = set()
    for key, val in (case.get("expect") or {}).items():
        m = re.match(r"^/not_computed/(\d+)/(.+)$", key)
        if m and int(m.group(1)) in by_list:
            n = by_list[int(m.group(1))]
            field = entry_field(m.group(2))
            if field is None:
                changes.append(Change("unmapped", "expect", key, val, "", "", None))
                continue
            new_key = f"/not_computed/{n}/{field}"
            touched.add(n)
            if field == "pointer":
                changes.append(
                    Change(
                        "move_value_changes", "expect", key, val, NEW_EXPECT, new_key,
                        "/value" + str(val), "a result pointer is written from the envelope root",
                    )
                )  # fmt: skip
            else:
                changes.append(Change("move", "expect", key, val, NEW_EXPECT, new_key, val))
            continue
        m = re.match(r"^(.*)/not_defined$", key)
        if m and val == "null":
            # The ratio is defined: no entry in the diagnostics says so; the assertion is kept
            # only when the same case asserts the ratio's value as a number (R6: a ratio with a
            # value is computed and defined).
            number = (case.get("expect") or {}).get(m.group(1) + "/value")
            if number is not None and number != "null" and m.group(1) not in by_path:
                changes.append(
                    Change(
                        "drop", "expect", key, val, "", "", None,
                        f"the ratio is defined: `{m.group(1)}/value` is asserted as {number!r}",
                    )
                )  # fmt: skip
            else:
                changes.append(Change("unmapped", "expect", key, val, "", "", None))
            continue
        if m:
            if "/value" + m.group(1) in listed:
                changes.append(
                    Change(
                        "unmapped", "expect", key, val, "", "", None,
                        "the output is also listed as not computed for a missing input: one "
                        "entry with reason_code missing_input and the reason of that list "
                        "(R5); the spec does not say where this text goes",
                    )
                )  # fmt: skip
                continue
            if m.group(1) not in by_path:
                changes.append(Change("unmapped", "expect", key, val, "", "", None))
                continue
            n = by_path[m.group(1)]
            touched.add(n)
            changes.append(
                Change("move", "expect", key, val, NEW_EXPECT, f"/not_computed/{n}/reason", val)
            )
            identity(
                changes, n,
                {"pointer": "/value" + m.group(1), "reason_code": "not_defined"},
                "carried by the old key: the ratio and that it is not defined",
                "/not_computed",
            )  # fmt: skip
            continue
        if re.search(r"/(not_computed|checks)(/|$)", key):
            changes.append(Change("unmapped", "expect", key, val, "", "", None))
    for n in sorted(touched):
        identity(
            changes, n, {"pointer": entries[n]["pointer"]},
            "carried by the old index: which output the entry is about", "/not_computed",
        )  # fmt: skip
    for key, val in (case.get("expect_length") or {}).items():
        if key == "/not_computed":
            count = str(len(entries))
            if count == str(val):
                changes.append(
                    Change("move", "expect_length", key, val, NEW_LENGTH, "/not_computed", val)
                )
            else:
                changes.append(
                    Change(
                        "move_value_changes", "expect_length", key, val, NEW_LENGTH,
                        "/not_computed", count,
                        "the ratios not defined join the list of the outputs not computed",
                    )
                )  # fmt: skip
        elif re.search(r"/(not_computed|not_defined|checks)(/|$)", key):
            changes.append(Change("unmapped", "expect_length", key, val, "", "", None))
    return changes


def plan_statements(
    case: dict[str, Any], value: dict[str, Any], schema: dict[str, Any]
) -> list[Change]:
    not_computed, checks = statement_entries(value)
    nc_index = {e["old"]: n for n, e in enumerate(not_computed)}
    check_index = {e["old"]: n for n, e in enumerate(checks)}
    changes: list[Change] = []
    nc_touched: set[int] = set()
    for key, val in (case.get("expect") or {}).items():
        m = re.match(r"^/years/(\d+)/not_computed/([a-z_]+)/(\d+)/(.+)$", key)
        if m:
            old = (int(m.group(1)), m.group(2), int(m.group(3)))
            field = entry_field(m.group(4))
            if old not in nc_index or field is None:
                changes.append(Change("unmapped", "expect", key, val, "", "", None))
                continue
            n = nc_index[old]
            nc_touched.add(n)
            new_key = f"/not_computed/{n}/{field}"
            if field == "pointer":
                changes.append(
                    Change(
                        "move_value_changes", "expect", key, val, NEW_EXPECT, new_key,
                        "/value" + str(val), "a result pointer is written from the envelope root",
                    )
                )  # fmt: skip
            else:
                changes.append(Change("move", "expect", key, val, NEW_EXPECT, new_key, val))
            continue
        m = re.match(r"^/years/(\d+)/checks/([^/]+)/(.+)$", key)
        if m:
            old_check = (int(m.group(1)), tokens("/" + m.group(2))[0])
            if old_check not in check_index:
                changes.append(Change("unmapped", "expect", key, val, "", "", None))
                continue
            n = check_index[old_check]
            changes.append(
                Change("move", "expect", key, val, NEW_EXPECT, f"/checks/{n}/{m.group(3)}", val)
            )
            identity(
                changes, n, {"id": old_check[1], "pointer": f"/value/years/{old_check[0]}"},
                "carried by the old key: the relation id and the year", "/checks",
            )  # fmt: skip
            continue
        if re.search(r"/(not_computed|checks)(/|$)", key):
            changes.append(Change("unmapped", "expect", key, val, "", "", None))
            continue
        new = renamed_key(schema, key)
        if new:
            changes.append(Change("rename", "expect", key, val, "expect", new, val))
    for n in sorted(nc_touched):
        identity(
            changes, n, {"pointer": not_computed[n]["pointer"]},
            "carried by the old index: which output the entry is about", "/not_computed",
        )  # fmt: skip
    for key, val in (case.get("expect_length") or {}).items():
        if re.search(r"/(not_computed|checks)(/|$)", key):
            changes.append(Change("unmapped", "expect_length", key, val, "", "", None))
        else:
            new = renamed_key(schema, key)
            if new:
                changes.append(
                    Change("rename", "expect_length", key, val, "expect_length", new, val)
                )
    return changes


def sha(text: str) -> str:
    return hashlib.sha256(text.encode("utf-8")).hexdigest()


def build_plan(root: Path, run: Runner) -> dict[str, Any]:
    schema_file = root / f"schemas/{STATEMENTS}/v1.output.json"
    statements_schema = json.loads(schema_file.read_text(encoding="utf-8"))
    files = {}
    for path in sorted((root / GOLDEN).rglob("*.yaml")):
        text = path.read_text(encoding="utf-8")
        case = yaml.safe_load(text)
        changes = plan_case(case, run, statements_schema)
        if changes:
            rel = path.relative_to(root).as_posix()
            files[rel] = {"sha256": sha(text), "changes": [asdict(c) for c in changes]}
    return {"format": 1, "spec": "SPEC-0012", "files": files}


# ---------------------------------------------------------------------------- applying


def transform(case: dict[str, Any], changes: list[dict[str, Any]]) -> dict[str, Any]:
    """The parsed case with `changes` applied, sections in the order of the file."""
    out = {k: (dict(v) if k in SECTIONS else v) for k, v in case.items()}
    added: dict[str, dict[str, Any]] = {NEW_LENGTH: {}, NEW_EXPECT: {}}
    for c in changes:
        if c["kind"] == "unmapped":
            raise ValueError(f"unmapped key {c['section']} {c['key']}")
        if c["section"]:
            del out[c["section"]][c["key"]]
        if c["kind"] == "drop":
            continue
        if c["new_section"] in added:
            added[c["new_section"]][c["new_key"]] = c["new_value"]
        else:
            out[c["new_section"]][c["new_key"]] = c["new_value"]
    for section in SECTIONS:
        if section in out and not out[section] and section != "expect":
            del out[section]
    for section, items in added.items():
        if items:
            out[section] = items
    return out


def _blocks(text: str) -> list[tuple[str, list[str]]]:
    """Top-level blocks of a golden file: (key, lines), the header line first."""
    blocks: list[tuple[str, list[str]]] = []
    for line in text.splitlines(keepends=True):
        m = re.match(r"^([a-z_]+):", line)
        if m:
            blocks.append((m.group(1), [line]))
        else:
            blocks[-1][1].append(line)
    return blocks


def _entries(lines: list[str]) -> list[list[str]]:
    """The entries of a flat mapping block (its lines after the header)."""
    entries: list[list[str]] = []
    for line in lines:
        if line.startswith("  ") and not line.startswith("   "):
            entries.append([line])
        elif entries:
            entries[-1].append(line)
    return entries


def _entry_key(entry: list[str]) -> str:
    parsed = yaml.safe_load("".join(entry))
    assert isinstance(parsed, dict) and len(parsed) == 1, entry
    return str(next(iter(parsed)))


def _with_key(entry: list[str], key: str) -> list[str]:
    """`entry` with its key replaced, the value text kept as written."""
    old = _entry_key(entry)
    first = entry[0]
    prefix = "  " + _dump_key(old) + ":"
    assert first.startswith(prefix), (first, prefix)
    return ["  " + _dump_key(key) + ":" + first[len(prefix) :], *entry[1:]]


def _dump_key(key: str) -> str:
    return yaml.safe_dump({key: 0}, width=10_000).split(":")[0] if key else "''"


def _new_entry(key: str, value: Any) -> list[str]:
    text = yaml.safe_dump({key: value}, width=10_000, allow_unicode=True, sort_keys=False)
    return ["  " + text]


def apply_text(text: str, changes: list[dict[str, Any]]) -> str:
    """`text` with `changes` applied; lines that do not move are kept byte for byte."""
    by_old = {(c["section"], c["key"]): c for c in changes if c["section"]}
    added: dict[str, list[list[str]]] = {NEW_LENGTH: [], NEW_EXPECT: []}
    out: list[tuple[str, list[str]]] = []
    for name, lines in _blocks(text):
        if name not in SECTIONS:
            out.append((name, lines))
            continue
        kept: list[list[str]] = []
        for entry in _entries(lines[1:]):
            c = by_old.get((name, _entry_key(entry)))
            if c is None:
                kept.append(entry)
                continue
            if c["kind"] == "unmapped":
                raise ValueError(f"unmapped key {name} {c['key']}")
            if c["kind"] == "drop":
                continue
            if c["new_value"] == c["value"]:
                moved = _with_key(entry, c["new_key"])
            else:
                moved = _new_entry(c["new_key"], c["new_value"])
            if c["new_section"] in added:
                added[c["new_section"]].append(moved)
            else:
                kept.append(moved)
        if kept or name == "expect":
            header = lines[0] if kept else f"{name}: {{}}\n"
            out.append((name, [header, *[line for e in kept for line in e]]))
    for c in changes:
        if c["kind"] == "add":
            added[c["new_section"]].append(_new_entry(c["new_key"], c["new_value"]))
    # The new sections go after the last expectation section, before `tolerance`.
    last = max((i for i, (name, _) in enumerate(out) if name in SECTIONS), default=len(out) - 1)
    result: list[str] = []
    for i, (_, lines) in enumerate(out):
        result += lines
        if i == last:
            for section in (NEW_LENGTH, NEW_EXPECT):
                if added[section]:
                    result.append(f"{section}:\n")
                    result += [line for e in added[section] for line in e]
    return "".join(result)


# ---------------------------------------------------------------------------- commands


def load_plan(path: Path) -> dict[str, Any]:
    plan: dict[str, Any] = json.loads(path.read_text(encoding="utf-8"))
    return plan


def engine_runner() -> Runner:
    import fairmark

    def run(method: str, data: Any) -> dict[str, Any]:
        result: dict[str, Any] = fairmark.run_json(method, data)
        return result

    return run


def render(plan: dict[str, Any]) -> str:
    files = plan["files"]
    by_folder: dict[str, int] = {}
    kinds: dict[str, int] = {}
    flagged: list[str] = []
    dropped: list[str] = []
    unmapped: list[str] = []
    for rel, entry in files.items():
        folder = Path(rel).parent.name
        by_folder[folder] = by_folder.get(folder, 0) + 1
        for c in entry["changes"]:
            label = _kind_label(c)
            kinds[label] = kinds.get(label, 0) + 1
            if c["kind"] == "move_value_changes":
                flagged.append(
                    f"- `{rel}`: `{c['section']}` `{c['key']}`: {c['value']!r} -> "
                    f"{c['new_value']!r} ({c['note']})"
                )
            if c["kind"] == "drop":
                dropped.append(
                    f"- `{rel}`: `{c['section']}` `{c['key']}`: {c['value']!r} ({c['note']})"
                )
            if c["kind"] == "unmapped":
                unmapped.append(
                    f"- `{rel}`: `{c['section']}` `{c['key']}`: {c['value']!r}"
                    + (f" ({c['note']})" if c["note"] else "")
                )
    out = [
        "# SPEC-0012 golden migration list",
        "",
        "Generated by `python scripts/golden_envelope_migration.py list` with the engine before",
        "SPEC-0012 (task T3). Machine-readable plan: `golden-migration.json` in this folder;",
        "`apply` reads it, `verify` checks every migrated file against it (AC16). No golden file",
        "is changed before the maintainer approves this list by name.",
        "",
        f"Files that change: {len(files)}.",
        "",
        "| Folder | Files |",
        "| --- | --- |",
        *[f"| `{f}` | {n} |" for f, n in sorted(by_folder.items())],
        "",
        "| Kind of change | Keys |",
        "| --- | --- |",
        *[f"| {k} | {n} |" for k, n in sorted(kinds.items())],
        "",
        "Renamed keys stay in their section. Moved keys go to `expect_diagnostics` or",
        "`expect_diagnostics_length`, at the index of R7 (checks by year, then relation id) or",
        "R13 (not computed by pointer). Added keys carry what the old key or index said: the",
        "relation id and year of a check, the output an entry is about and, for a ratio not",
        "defined, `reason_code` `not_defined`.",
        "",
        "## Expected values that change",
        "",
        "No number changes. These expected values change form, as the spec requires:",
        "",
        *(flagged or ["- none"]),
        "",
        "## Expectations removed",
        "",
        "A ratio asserted as defined (`not_defined: null`) has no entry in the diagnostics, so",
        "no key holds the assertion. Each is removed only where the same case asserts the",
        "ratio's value as a number, which implies it (R6).",
        "",
        *(dropped or ["- none"]),
        "",
        "## Keys without a rule: maintainer decision needed",
        "",
        "`apply` refuses a file with one of these keys until the plan gives it a rule.",
        "",
        *(unmapped or ["- none"]),
        "",
        "## Files",
    ]
    for rel, entry in files.items():
        out += ["", f"### `{rel}`", ""]
        for c in entry["changes"]:
            out.append(_line(c))
    return "\n".join(out) + "\n"


def _kind_label(c: dict[str, Any]) -> str:
    if c["kind"] == "rename":
        pairs = zip(c["key"].split("/"), c["new_key"].split("/"), strict=True)
        old, new = next((a, b) for a, b in pairs if a != b)
        return f"rename `{old}` -> `{new}` in `{c['section']}`"
    if c["kind"] == "drop":
        return f"removed from `{c['section']}` (implied by an expectation kept)"
    if c["kind"] == "add":
        return f"added in `{c['new_section']}` (identity carried by the old key or index)"
    if c["kind"] == "unmapped":
        return "unmapped"
    return f"moved `{c['section']}` -> `{c['new_section']}`" + (
        " (value changes form)" if c["kind"] == "move_value_changes" else ""
    )


def _line(c: dict[str, Any]) -> str:
    if c["kind"] == "add":
        return f"- added `{c['new_section']}` `{c['new_key']}`: {c['new_value']!r} ({c['note']})"
    if c["kind"] == "drop":
        return f"- removed `{c['section']}` `{c['key']}`: {c['value']!r} ({c['note']})"
    if c["kind"] == "unmapped":
        return f"- UNMAPPED `{c['section']}` `{c['key']}`: {c['value']!r}"
    text = f"- `{c['section']}` `{c['key']}` -> `{c['new_section']}` `{c['new_key']}`"
    if c["kind"] == "move_value_changes":
        text += f"; value {c['value']!r} -> {c['new_value']!r} ({c['note']})"
    return text


def dump_plan(plan: dict[str, Any]) -> str:
    """The plan as JSON with one change per line, so that a review reads it line by line."""
    lines = ["{", f' "format": {plan["format"]},', f' "spec": {json.dumps(plan["spec"])},']
    lines.append(' "files": {')
    for i, (rel, entry) in enumerate(plan["files"].items()):
        lines.append(f"  {json.dumps(rel)}: {{")
        lines.append(f'   "sha256": {json.dumps(entry["sha256"])},')
        lines.append('   "changes": [')
        changes = entry["changes"]
        for j, c in enumerate(changes):
            comma = "," if j < len(changes) - 1 else ""
            lines.append("    " + json.dumps(c, ensure_ascii=False) + comma)
        lines.append("   ]")
        lines.append("  }" + ("," if i < len(plan["files"]) - 1 else ""))
    lines += [" }", "}"]
    return "\n".join(lines) + "\n"


def cmd_list(args: argparse.Namespace, run: Runner | None = None) -> int:
    root: Path = args.root
    plan = build_plan(root, run or engine_runner())
    plan_path = root / args.plan
    plan_path.parent.mkdir(parents=True, exist_ok=True)
    plan_path.write_text(dump_plan(plan), encoding="utf-8")
    text = render(plan)
    if args.markdown:
        (root / args.markdown).write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)
    unmapped = any(c["kind"] == "unmapped" for e in plan["files"].values() for c in e["changes"])
    return 1 if unmapped else 0


def cmd_apply(args: argparse.Namespace) -> int:
    root: Path = args.root
    plan = load_plan(root / args.plan)
    problems = []
    for name in args.files:
        rel = Path(name).as_posix()
        entry = plan["files"].get(rel)
        if entry is None:
            problems.append(f"{rel}: not in the plan")
            continue
        if any(c["kind"] == "unmapped" for c in entry["changes"]):
            problems.append(f"{rel}: has keys without a rule (see the list)")
            continue
        path = root / rel
        text = path.read_text(encoding="utf-8")
        if sha(text) != entry["sha256"]:
            problems.append(f"{rel}: not the file the plan was computed from (applied already?)")
            continue
        path.write_text(apply_text(text, entry["changes"]), encoding="utf-8")
        print(f"migrated {rel}")
    for p in problems:
        print(p, file=sys.stderr)
    return 1 if problems else 0


def base_text(root: Path, base: str, rel: str, base_dir: Path | None) -> str:
    if base_dir is not None:
        return (base_dir / rel).read_text(encoding="utf-8")
    return subprocess.run(
        ["git", "show", f"{base}:{rel}"],
        cwd=root, capture_output=True, text=True, check=True, encoding="utf-8",
    ).stdout  # fmt: skip


def verify_file(old_text: str, new_text: str, entry: dict[str, Any]) -> list[str]:
    """Why `new_text` is not `old_text` unchanged or migrated by `entry`; empty when it is."""
    if new_text == old_text:
        return []
    problems = []
    if sha(old_text) != entry["sha256"]:
        problems.append("the base file is not the file the plan was computed from")
    old, new = yaml.safe_load(old_text), yaml.safe_load(new_text)
    for c in entry["changes"]:
        if c["section"]:
            got = (old.get(c["section"]) or {}).get(c["key"], "<absent>")
            if got != c["value"]:
                problems.append(f"{c['section']} {c['key']}: base has {got!r}, plan {c['value']!r}")
        if c["kind"] in ("rename", "move") and c["new_value"] != c["value"]:
            problems.append(f"{c['new_key']}: a {c['kind']} must keep its value")
        if c["kind"] == "drop":
            if c["key"] in (new.get(c["section"]) or {}):
                problems.append(f"{c['section']} {c['key']}: dropped by the plan, still present")
            continue
        got = (new.get(c["new_section"]) or {}).get(c["new_key"], "<absent>")
        if got != c["new_value"]:
            problems.append(f"{c['new_section']} {c['new_key']}: {got!r}, want {c['new_value']!r}")
    try:
        want = transform(old, entry["changes"])
    except ValueError as err:
        return [*problems, str(err)]
    if new != want:
        problems.append("the file differs from the plan applied to the base")
    if new_text != apply_text(old_text, entry["changes"]):
        problems.append("the text differs from `apply` (lines that do not move must be kept)")
    return problems


def cmd_verify(args: argparse.Namespace) -> int:
    root: Path = args.root
    plan = load_plan(root / args.plan)
    names = [Path(f).as_posix() for f in args.files] or sorted(plan["files"])
    failures = 0
    migrated = 0
    for rel in names:
        entry = plan["files"].get(rel)
        if entry is None:
            print(f"{rel}: not in the plan", file=sys.stderr)
            failures += 1
            continue
        old = base_text(root, args.base, rel, args.base_dir)
        new = (root / rel).read_text(encoding="utf-8")
        problems = verify_file(old, new, entry)
        migrated += new != old
        for p in problems:
            print(f"{rel}: {p}", file=sys.stderr)
        failures += bool(problems)
    print(f"{len(names)} files checked, {migrated} migrated, {failures} with problems.")
    return 1 if failures else 0


def main(argv: list[str] | None = None, run: Runner | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--plan", type=Path, default=PLAN, help="plan, relative to --root")
    sub = parser.add_subparsers(dest="cmd", required=True)
    p_list = sub.add_parser("list")
    p_list.add_argument("--markdown", type=Path, help="write the Markdown list to this file")
    p_apply = sub.add_parser("apply")
    p_apply.add_argument("files", nargs="+", help="approved golden files, relative to --root")
    p_verify = sub.add_parser("verify")
    p_verify.add_argument("files", nargs="*")
    p_verify.add_argument("--base", default="origin/main")
    p_verify.add_argument("--base-dir", type=Path, help="originals in a directory, not git")
    args = parser.parse_args(argv)
    if args.cmd == "list":
        return cmd_list(args, run)
    if args.cmd == "apply":
        return cmd_apply(args)
    return cmd_verify(args)


if __name__ == "__main__":
    sys.exit(main())
