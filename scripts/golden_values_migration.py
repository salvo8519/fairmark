"""Golden cases assert values, schema snapshots assert shape (issue #351, SPEC-0013 design).

    python scripts/golden_values_migration.py inventory [--markdown FILE] [--root DIR]
    python scripts/golden_values_migration.py list [--plan FILE] [--markdown FILE] [--root DIR]
    python scripts/golden_values_migration.py apply FILE... [--plan FILE] [--root DIR]
    python scripts/golden_values_migration.py verify [FILE...] [--plan FILE] [--root DIR]
        [--base-root DIR]

- `inventory` reads every golden case, runs the engine on its input and classifies every
  assertion as VALUE, SHAPE or WORDING by the rule of the design document
  (`specs/0013-golden-values-schema-shape/design.md`, section 1). It changes nothing.
- `list` computes, for every golden case that asserts the envelope by position, the semantic
  sections that replace those assertions (`expect_defaults`, `expect_not_computed`,
  `expect_warnings`, `expect_checks`, `expect_errors`, `expect_counts`), writes the plan as
  JSON and prints the file list as Markdown for the maintainer's named approval. It runs the
  engine as it is on the base branch: the engine only names which entry a positional key
  pointed at; no expected value is taken from it.
- `apply` rewrites only the files named on the command line, and only when they are still the
  files the plan was computed from. Top-level blocks that do not move are kept byte for byte.
- `verify` checks every file of the plan against its base copy: each old assertion and its new
  form resolve to the same node of the same engine result, with the same expected text; every
  dropped assertion is one the plan lists; every other block is unchanged.

Golden files are protected (AGENTS.md): `apply` runs on the repository only on the files the
maintainer has approved by name. Before that, it runs on a temporary copy (`--root`).
"""

from __future__ import annotations

import argparse
import collections
import hashlib
import json
import re
import sys
from collections.abc import Iterator
from dataclasses import dataclass, field
from pathlib import Path
from typing import Any

import yaml

REPO = Path(__file__).resolve().parents[1]
GOLDEN = Path("tests/golden")
SPEC_DIR = Path("specs/0013-golden-values-schema-shape")
PLAN = SPEC_DIR / "golden-migration.json"
MARKDOWN = SPEC_DIR / "golden-migration.md"

# Assertion sections of today's golden format and the part of the result each one reads.
PARTS = {
    "expect": "value",
    "expect_length": "value",
    "expect_meta": "meta",
    "expect_diagnostics": "diagnostics",
    "expect_diagnostics_length": "diagnostics",
    "expect_error_report": "error",
}
LENGTH_SECTIONS = {"expect_length", "expect_diagnostics_length"}
# In an error case `expect_length` counts arrays of the error report.
DECIMAL = re.compile(r"^-?\d+(\.\d+)?$")
DATE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
CODE = re.compile(r"^FM-[EW]\d{3}$")
# Fields the schemas themselves declare as guidance, not contract ("its wording may change",
# "guidance, not contract"), and free-text notes written by the engine.
WORDING_FIELDS = {"fix", "message", "reason", "basis_note", "not_tested_reason"}
# Fields of a list item that can name it (the key of a semantic path), in order of preference.
NATURAL_KEYS = (
    "id",
    "pointer",
    "source_pointer",
    "variable_pointer",
    "aggregate_pointer",
    "code",
    "name",
    "label",
    "date",
    "method",
)
# Fields that name an entry of a list or an echoed input: asserting them at a position asserts
# where the entry sits, not what was computed.
IDENTITY_FIELDS = {"label", "name", "id"}
# Lists whose order is computed by the method: the position of an entry is itself a result.
RANKED_LISTS = {"ranking"}


# ---------------------------------------------------------------- pointers and schemas


def tokens(pointer: str) -> list[str]:
    """RFC 6901 tokens of `pointer`; a key without a leading '/' is one top-level field."""
    if not pointer.startswith("/"):
        return [pointer]
    return [t.replace("~1", "/").replace("~0", "~") for t in pointer[1:].split("/")]


def pointer_of(parts: list[str | int]) -> str:
    return "".join("/" + str(p).replace("~", "~0").replace("/", "~1") for p in parts)


def resolve(data: Any, parts: list[str]) -> Any:
    """The node of `data` at `parts`; KeyError when it does not exist."""
    for t in parts:
        if isinstance(data, list):
            if not re.fullmatch(r"0|[1-9][0-9]*", t) or int(t) >= len(data):
                raise KeyError(t)
            data = data[int(t)]
        elif isinstance(data, dict):
            if t not in data:
                raise KeyError(t)
            data = data[t]
        else:
            raise KeyError(t)
    return data


def _deref(root: dict[str, Any], node: Any) -> Any:
    seen = 0
    while isinstance(node, dict) and isinstance(node.get("$ref"), str) and seen < 20:
        name = node["$ref"].removeprefix("#/$defs/")
        node = root.get("$defs", {}).get(name, {})
        seen += 1
    return node


def _branches(root: dict[str, Any], node: Any) -> list[Any]:
    node = _deref(root, node)
    if not isinstance(node, dict):
        return []
    out = [node]
    for k in ("anyOf", "oneOf", "allOf"):
        for b in node.get(k, []):
            out.extend(_branches(root, b))
    return out


def schema_at(root: dict[str, Any], parts: list[str]) -> list[dict[str, Any]]:
    """Every schema node that can describe the data at `parts` (all branches kept)."""
    nodes = _branches(root, root)
    for t in parts:
        nxt: list[dict[str, Any]] = []
        for n in nodes:
            if t.isdigit() and "items" in n:
                nxt.extend(_branches(root, n["items"]))
            props = n.get("properties", {})
            if t in props:
                nxt.extend(_branches(root, props[t]))
            ap = n.get("additionalProperties")
            if isinstance(ap, dict) and t not in props and not t.isdigit():
                nxt.extend(_branches(root, ap))
        nodes = nxt
    return nodes


def is_enum(nodes: list[dict[str, Any]]) -> bool:
    return any("enum" in n or "const" in n for n in nodes)


def is_decimal_node(nodes: list[dict[str, Any]]) -> bool:
    for n in nodes:
        t = n.get("type")
        types = t if isinstance(t, list) else [t]
        if "integer" in types or "number" in types or "pattern" in n:
            return True
    return False


def item_props(nodes: list[dict[str, Any]], root: dict[str, Any]) -> set[str]:
    props: set[str] = set()
    for n in nodes:
        if "items" in n:
            for b in _branches(root, n["items"]):
                props |= set(b.get("properties", {}))
    return props


class Schemas:
    """The current output snapshot of each method and the envelope snapshot, read from disk."""

    def __init__(self, repo: Path) -> None:
        index = json.loads((repo / "schemas/index.json").read_text(encoding="utf-8"))
        self.output: dict[str, dict[str, Any]] = {}
        for method, entry in index["methods"].items():
            version = entry["versions"][entry["current"]]
            if "output_file" in version:
                self.output[method] = json.loads(
                    (repo / "schemas" / version["output_file"]).read_text(encoding="utf-8")
                )
        env = index["envelope"]["versions"][index["envelope"]["current"]]
        self.envelope = json.loads((repo / "schemas" / env["file"]).read_text(encoding="utf-8"))

    def part(self, method: str, part: str) -> dict[str, Any]:
        """A schema rooted at `part` of the result, with the `$defs` of its document."""
        if part == "value":
            return self.output.get(method, {})
        defs = self.envelope["$defs"]
        name = {"meta": "Meta", "diagnostics": "Diagnostics", "error": "FairmarkErrorReport"}[part]
        return {"$defs": defs, "$ref": f"#/$defs/{name}"}


# ---------------------------------------------------------------- the engine


def engine_result(method: str, data: Any) -> dict[str, Any]:
    """{"value", "meta", "diagnostics"} of a success, or {"error": report} of a failure."""
    import fairmark as fm

    try:
        return dict(fm.run_json(method, data))
    except fm.FairmarkError as e:
        return {"error": e.report}


# ---------------------------------------------------------------- inventory


@dataclass
class Assertion:
    file: str
    method: str
    section: str
    key: str
    expected: str
    klass: str = ""
    kind: str = ""
    path_kind: str = ""
    lists: list[str] = field(default_factory=list)


def assertions(rel: str, case: dict[str, Any]) -> Iterator[Assertion]:
    if "expect_error" in case:
        yield Assertion(rel, case["method"], "expect_error", "", str(case["expect_error"]))
    for section in PARTS:
        for key, expected in (case.get(section) or {}).items():
            yield Assertion(rel, case["method"], section, key, str(expected))


def input_strings(data: Any) -> set[str]:
    if isinstance(data, dict):
        return set().union(*(input_strings(v) for v in data.values())) if data else set()
    if isinstance(data, list):
        return set().union(*(input_strings(v) for v in data)) if data else set()
    return {data} if isinstance(data, str) else set()


def classify(
    a: Assertion,
    case: dict[str, Any],
    schemas: Schemas,
    result: dict[str, Any],
    always_empty: set[tuple[str, str]],
) -> None:
    """Set `klass` (VALUE, SHAPE, WORDING), `kind` and `path_kind` of an assertion (design 1.2)."""
    if a.section == "expect_error":
        a.klass, a.kind, a.path_kind = "VALUE", "error code", "none"
        return
    part = PARTS[a.section]
    error_case = "expect_error" in case
    if a.section == "expect_length" and error_case:
        part = "error"
    parts = tokens(a.key)
    schema = schemas.part(a.method, part)
    nodes = schema_at(schema, parts)
    leaf = parts[-1]
    # Path kind: no index; indices into lists whose items carry a natural key; ordinal indices.
    ordinal, keyed = False, False
    for i, t in enumerate(parts):
        if t.isdigit() and i > 0:
            owner = parts[i - 1]
            if owner.isdigit() and i > 1:
                owner = parts[i - 2] + "/N"  # a list of lists, such as `missing_any_of`
            props = item_props(schema_at(schema, parts[:i]), schema)
            if owner in RANKED_LISTS:
                ordinal = True
            elif props & set(NATURAL_KEYS):
                keyed = True
            else:
                ordinal = True
            a.lists.append(f"{part}:{owner}")
    a.path_kind = (
        "keyed and ordinal"
        if keyed and ordinal
        else "keyed position"
        if keyed
        else "ordinal position"
        if ordinal
        else "plain"
    )
    if a.section in LENGTH_SECTIONS:
        if (a.method, a.key) in always_empty and a.expected == "0":
            a.klass, a.kind = "SHAPE", "count of a list the method never fills"
        else:
            a.klass, a.kind = "VALUE", "count (no entry beyond those asserted)"
        return
    if a.expected == "null":
        a.klass, a.kind = "VALUE", "null (not computed or undefined)"
        return
    if leaf in WORDING_FIELDS and not (part == "error" and leaf == "reason"):
        a.klass, a.kind = "WORDING", f"{leaf} text"
        return
    if DECIMAL.match(a.expected) and (is_decimal_node(nodes) or not nodes):
        a.klass, a.kind = "VALUE", "number"
        return
    if CODE.match(a.expected):
        a.klass, a.kind = "VALUE", "error or warning code"
        return
    if is_enum(nodes) or (part == "error" and leaf == "reason"):
        a.klass, a.kind = "VALUE", "enumerated code or status"
        return
    if DATE.match(a.expected):
        a.klass, a.kind = "VALUE", "date"
        return
    owner_ranked = any(lst.endswith(":" + r) for lst in a.lists for r in RANKED_LISTS)
    if owner_ranked:
        a.klass, a.kind = "VALUE", "input echo whose position is a computed rank"
        return
    if a.expected.startswith("/"):
        a.klass, a.kind = "VALUE", "pointer the user acts on"
        return
    if leaf in IDENTITY_FIELDS and part == "value":
        echo = a.expected in input_strings(case["input"])
        a.klass = "SHAPE"
        a.kind = "identity of an entry: " + ("input text echoed" if echo else "engine label")
        return
    a.klass, a.kind = "VALUE", "identifier or code not enumerated in the schema"


def golden_files(root: Path) -> list[Path]:
    return sorted((root / GOLDEN).rglob("*.yaml"))


def load(path: Path) -> dict[str, Any]:
    data: dict[str, Any] = yaml.safe_load(path.read_text(encoding="utf-8"))
    return data


def lists_never_filled(cases: dict[str, tuple[dict[str, Any], dict[str, Any]]]) -> set[str]:
    """(method, pointer) of the diagnostics lists that no golden result of the method fills."""
    filled: dict[tuple[str, str], bool] = {}
    for case, result in cases.values():
        if "diagnostics" not in result:
            continue
        for name, items in result["diagnostics"].items():
            key = (case["method"], "/" + name)
            filled[key] = filled.get(key, False) or bool(items)
    return {k for k, v in filled.items() if not v}  # type: ignore[misc]


def cmd_inventory(args: argparse.Namespace) -> int:
    root = Path(args.root)
    schemas = Schemas(REPO)
    cases = {}
    for path in golden_files(root):
        case = load(path)
        cases[str(path.relative_to(root))] = (case, engine_result(case["method"], case["input"]))
    empty = lists_never_filled(cases)
    rows: list[Assertion] = []
    for rel, (case, result) in cases.items():
        for a in assertions(rel, case):
            classify(a, case, schemas, result, empty)  # type: ignore[arg-type]
            rows.append(a)
    text = render_inventory(rows, len(cases))
    if args.markdown:
        Path(args.markdown).write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)
    if args.json:
        Path(args.json).write_text(
            json.dumps([a.__dict__ for a in rows], indent=1) + "\n", encoding="utf-8"
        )
    return 0


def _table(head: tuple[str, ...], rows: list[tuple[Any, ...]]) -> list[str]:
    out = ["| " + " | ".join(head) + " |", "| " + " | ".join("---" for _ in head) + " |"]
    out += ["| " + " | ".join(str(c) for c in r) + " |" for r in rows]
    return out


def render_inventory(rows: list[Assertion], n_files: int) -> str:
    out = [f"Golden files: {n_files}. Assertions: {len(rows)}.", ""]
    by_section: dict[str, collections.Counter[str]] = collections.defaultdict(collections.Counter)
    files_by_section: dict[str, set[str]] = collections.defaultdict(set)
    for a in rows:
        by_section[a.section][a.klass] += 1
        files_by_section[a.section].add(a.file)
    sections = ["expect_error", *PARTS]
    out += _table(
        ("Section", "Files", "Assertions", "VALUE", "SHAPE", "WORDING"),
        [
            (
                f"`{s}`",
                len(files_by_section[s]),
                sum(by_section[s].values()),
                by_section[s]["VALUE"],
                by_section[s]["SHAPE"],
                by_section[s]["WORDING"],
            )
            for s in sections
        ],
    )
    out.append("")
    kinds = collections.Counter((a.klass, a.kind) for a in rows)
    kind_files: dict[tuple[str, str], set[str]] = collections.defaultdict(set)
    for a in rows:
        kind_files[(a.klass, a.kind)].add(a.file)
    out += _table(
        ("Class", "Kind", "Assertions", "Files"),
        [(k[0], k[1], n, len(kind_files[k])) for k, n in sorted(kinds.items())],
    )
    out.append("")
    paths = collections.Counter(
        (a.section, a.path_kind) for a in rows if a.section != "expect_error"
    )
    out += _table(
        ("Section", "Path kind", "Assertions"),
        [(f"`{s}`", p, n) for (s, p), n in sorted(paths.items())],
    )
    out.append("")
    lists = collections.Counter(lst for a in rows for lst in dict.fromkeys(a.lists))
    out += _table(
        ("List indexed by position (part:list)", "Assertions"),
        [(f"`{k}`", n) for k, n in sorted(lists.items(), key=lambda x: (-x[1], x[0]))],
    )
    out.append("")
    return "\n".join(out)


# ---------------------------------------------------------------- the migration plan

# Envelope lists that become semantic sections, with the fields that name one entry.
ENVELOPE_LISTS = {
    ("meta", "defaults_applied"): ("expect_defaults", ("pointer",)),
    ("diagnostics", "not_computed"): ("expect_not_computed", ("pointer",)),
    ("diagnostics", "warnings"): ("expect_warnings", ("code", "pointer")),
    ("diagnostics", "checks"): ("expect_checks", ("pointer", "id")),
    ("error", "errors"): ("expect_errors", ("code", "pointer")),
}
# Fields of the error report that repeat its first entry (ADR 0014 G5).
REPORT_FIRST = {"pointer", "reason", "expected", "pointers", "missing_any_of", "fix", "message"}


def entry_key(entry: dict[str, Any], names: tuple[str, ...]) -> str:
    """The key of an entry in a semantic section: its naming fields joined by one space, a
    null pointer written `-`."""
    parts = []
    for n in names:
        v = entry.get(n)
        parts.append("-" if v is None else str(v))
    return " ".join(parts)


@dataclass
class Change:
    kind: str  # "keyed", "count", "dropped"
    section: str
    key: str
    expected: str
    new_section: str = ""
    entry: str = ""
    field: str = ""
    note: str = ""


def plan_case(case: dict[str, Any], result: dict[str, Any]) -> list[Change]:
    """The changes of one case; empty when it asserts no envelope list by position."""
    changes: list[Change] = []
    error_case = "expect_error" in case
    for section, part in PARTS.items():
        if section == "expect":
            continue  # `value` keeps its pointers (design, section 2).
        for key, expected in (case.get(section) or {}).items():
            expected = str(expected)
            the_part = "error" if (section == "expect_length" and error_case) else part
            parts = tokens(key)
            if section == "expect_length" and not error_case:
                continue  # value list lengths stay as they are
            data = result.get("error") if the_part == "error" else result.get(the_part)
            if the_part == "error" and parts[0] in REPORT_FIRST:
                continue  # the first error, as the typed API returns it: a plain path (design 2)
            target = ENVELOPE_LISTS.get((the_part, parts[0]))
            if target is None:
                raise SystemExit(f"{section} {key}: no semantic section for it")
            new_section, names = target
            if section in LENGTH_SECTIONS or (section == "expect_length"):
                if len(parts) == 1:
                    changes.append(
                        Change("count", section, key, expected, "expect_counts", parts[0])
                    )
                    continue
                entry = resolve(data, parts[:2])
                changes.append(
                    Change(
                        "count",
                        section,
                        key,
                        expected,
                        new_section,
                        entry_key(entry, names),
                        pointer_of(parts[2:]) + "#count",  # type: ignore[arg-type]
                    )
                )
                continue
            entry = resolve(data, parts[:2])
            rest = parts[2:]
            note = "named by the key" if len(rest) == 1 and rest[0] in names else ""
            # Wording assertions move like the others: the migration drops nothing (design 4).
            changes.append(
                Change(
                    "keyed",
                    section,
                    key,
                    expected,
                    new_section,
                    entry_key(entry, names),
                    pointer_of(rest),  # type: ignore[arg-type]
                    note,
                )
            )
    return changes


def new_sections(changes: list[Change]) -> dict[str, Any]:
    """The semantic sections a case gains, in a fixed order."""
    out: dict[str, dict[str, Any]] = {}
    order = [
        "expect_counts",
        "expect_defaults",
        "expect_not_computed",
        "expect_warnings",
        "expect_checks",
        "expect_errors",
    ]
    for c in changes:
        if c.kind == "dropped":
            continue
        if c.new_section == "expect_counts":
            out.setdefault("expect_counts", {})[c.entry] = c.expected
            continue
        entry = out.setdefault(c.new_section, {}).setdefault(c.entry, {})
        if c.note == "named by the key":
            continue  # the key states it; asserting it again would say it twice
        entry[c.field] = c.expected
    return {k: out[k] for k in order if k in out}


def sha(text: str) -> str:
    return hashlib.sha256(text.encode()).hexdigest()


def build_plan(root: Path) -> dict[str, Any]:
    files: dict[str, Any] = {}
    for path in golden_files(root):
        case = load(path)
        if not set(case) & (set(PARTS) - {"expect"}):
            continue
        result = engine_result(case["method"], case["input"])
        changes = plan_case(case, result)
        if not changes:
            continue
        text = path.read_text(encoding="utf-8")
        files[str(path.relative_to(root))] = {
            "sha256": sha(text),
            "sections": sorted({c.section for c in changes}),
            "add": new_sections(changes),
            "changes": [c.__dict__ for c in changes],
        }
    return {"format": 1, "spec": "SPEC-0013 (design)", "files": files}


def render_plan(plan: dict[str, Any]) -> str:
    files = plan["files"]
    out = [
        "# SPEC-0013 golden migration list (dry run)",
        "",
        "Generated by `python scripts/golden_values_migration.py list` on `main` at the commit",
        "this file was written from. Machine-readable plan: `golden-migration.json` in this",
        "folder; `apply` reads it, `verify` checks every migrated file against it. No golden",
        "file is changed before the maintainer approves this list by name.",
        "",
        f"Files that change: {len(files)}.",
        "",
    ]
    folders = collections.Counter(Path(f).parent.name for f in files)
    out += _table(("Folder", "Files"), [(f"`{k}`", v) for k, v in sorted(folders.items())])
    out.append("")
    kinds: collections.Counter[str] = collections.Counter()
    for entry in files.values():
        for c in entry["changes"]:
            label = {
                "keyed": f"`{c['section']}` -> `{c['new_section']}`",
                "count": f"`{c['section']}` -> `{c['new_section']}` (count)",
                "dropped": f"`{c['section']}` dropped ({c['note']})",
            }[c["kind"]]
            kinds[label] += 1
    out += _table(("Change", "Keys"), [(k, v) for k, v in sorted(kinds.items())])
    out += ["", "## Assertions dropped", ""]
    dropped = [(f, c) for f, e in files.items() for c in e["changes"] if c["kind"] == "dropped"]
    if not dropped:
        out.append("None.")
    for f, c in dropped:
        out.append(f"- `{f}`: `{c['section']}` `{c['key']}` ({c['note']})")
    out += ["", "## Files", ""]
    for f in sorted(files):
        e = files[f]
        out.append(
            f"- `{f}`: moves keys out of {', '.join(f'`{s}`' for s in e['sections'])}; adds "
            f"{', '.join(f'`{s}`' for s in e['add'])}"
        )
    out.append("")
    return "\n".join(out)


def cmd_list(args: argparse.Namespace) -> int:
    plan = build_plan(Path(args.root))
    Path(args.plan).write_text(json.dumps(plan, indent=1, sort_keys=True) + "\n", encoding="utf-8")
    text = render_plan(plan)
    if args.markdown:
        Path(args.markdown).write_text(text, encoding="utf-8")
    else:
        sys.stdout.write(text)
    return 0


# ---------------------------------------------------------------- apply


def blocks(text: str) -> list[tuple[str, str]]:
    """(top-level key, text of its block) in file order; comments before the first key are a
    block named ''."""
    out: list[tuple[str, list[str]]] = [("", [])]
    for line in text.splitlines(keepends=True):
        m = re.match(r"^([A-Za-z_][A-Za-z0-9_]*):", line)
        if m:
            out.append((m.group(1), [line]))
        else:
            out[-1][1].append(line)
    return [(k, "".join(v)) for k, v in out]


def dump_sections(sections: dict[str, Any]) -> str:
    return yaml.safe_dump(sections, sort_keys=False, allow_unicode=True, width=100)


def block_entries(block: str) -> list[str]:
    """The entries of a top-level mapping block: its first line, then one text per key."""
    lines = block.splitlines(keepends=True)
    out = [lines[0]]
    for line in lines[1:]:
        if re.match(r"^  [^ ]", line):
            out.append(line)
        else:
            out[-1] += line
    return out


def apply_text(text: str, entry: dict[str, Any]) -> str:
    moved: dict[str, set[str]] = collections.defaultdict(set)
    for c in entry["changes"]:
        moved[c["section"]].add(c["key"])
    kept = []
    for k, b in blocks(text):
        if k not in moved:
            kept.append(b)
            continue
        head, *entries = block_entries(b)
        rest = [e for e in entries if str(next(iter(yaml.safe_load(e)))) not in moved[k]]
        if rest:
            kept.append(head + "".join(rest))
    body = "".join(kept)
    if not body.endswith("\n"):
        body += "\n"
    return body + dump_sections(entry["add"])


def cmd_apply(args: argparse.Namespace) -> int:
    plan = json.loads(Path(args.plan).read_text(encoding="utf-8"))
    root = Path(args.root)
    failed = 0
    for name in args.files:
        rel = str(Path(name))
        entry = plan["files"].get(rel)
        if entry is None:
            print(f"{rel}: not in the plan", file=sys.stderr)
            failed += 1
            continue
        path = root / rel
        text = path.read_text(encoding="utf-8")
        if sha(text) != entry["sha256"]:
            print(f"{rel}: changed since the plan was computed", file=sys.stderr)
            failed += 1
            continue
        path.write_text(apply_text(text, entry), encoding="utf-8")
    return 1 if failed else 0


# ---------------------------------------------------------------- the semantic resolver


def find_entry(items: list[dict[str, Any]], key: str, names: tuple[str, ...]) -> int:
    """The index of the one entry of `items` named `key`; KeyError when none or several are."""
    found = [i for i, e in enumerate(items) if entry_key(e, names) == key]
    if len(found) != 1:
        raise KeyError(f"{len(found)} entries named {key!r}")
    return found[0]


SECTION_LIST = {v[0]: (k, v[1]) for k, v in ENVELOPE_LISTS.items()}
Path_ = tuple[str, tuple[str, ...]]


def semantic_paths(
    case: dict[str, Any], result: dict[str, Any]
) -> dict[tuple[str, str, str], Path_]:
    """(section, entry, field) -> (part, path) of the node a semantic key reads in `result`:
    the rule the runners will follow. A field ending in `#count` reads the length of a list."""
    out: dict[tuple[str, str, str], Path_] = {}
    for name in case.get("expect_counts") or {}:
        if name.startswith("/"):
            out[("expect_counts", name, "")] = ("value", tuple(tokens(name)))
            continue
        part = "error" if "expect_error" in case else "diagnostics"
        resolve(result[part], [name])
        out[("expect_counts", name, "")] = (part, (name,))
    for section, ((part, lst), names) in SECTION_LIST.items():
        for ekey, fields in (case.get(section) or {}).items():
            items = result[part][lst]
            at = (lst, str(find_entry(items, ekey, names)))
            out[(section, ekey, "")] = (part, at)
            for f in fields:
                inner = tokens(f.removesuffix("#count"))
                resolve(resolve(result[part], list(at)), inner)
                out[(section, ekey, f)] = (part, at + tuple(inner))
    return out


# ---------------------------------------------------------------- verify


def verify_file(old_text: str, new_text: str, entry: dict[str, Any]) -> list[str]:
    problems: list[str] = []
    old_blocks = dict(blocks(old_text))
    new_blocks = dict(blocks(new_text))
    moved: dict[str, set[str]] = collections.defaultdict(set)
    for c in entry["changes"]:
        moved[c["section"]].add(c["key"])
    for k, b in old_blocks.items():
        if k in moved:
            head, *entries = block_entries(b)
            rest = [e for e in entries if str(next(iter(yaml.safe_load(e)))) not in moved[k]]
            want = head + "".join(rest) if rest else None
            if new_blocks.get(k) != want:
                problems.append(f"{k}: kept entries changed")
            continue
        if new_blocks.get(k) != b:
            problems.append(f"{k}: block changed")
    extra = set(new_blocks) - set(old_blocks) - set(entry["add"])
    if extra:
        problems.append(f"unexpected blocks: {sorted(extra)}")
    old, new = yaml.safe_load(old_text), yaml.safe_load(new_text)
    for section, content in entry["add"].items():
        if new.get(section) != content:
            problems.append(f"{section}: not the section of the plan")
    result = engine_result(old["method"], old["input"])
    try:
        paths = semantic_paths(new, result)
    except (KeyError, IndexError, TypeError) as e:
        return [*problems, f"new sections do not resolve: {e}"]
    for c in entry["changes"]:
        part = PARTS[c["section"]]
        if c["section"] == "expect_length" and "expect_error" in old:
            part = "error"
        old_path = (part, tuple(tokens(c["key"])))
        section = new.get(c["new_section"]) or {}
        if c["new_section"] == "expect_counts":
            new_value = section.get(c["entry"])
            new_path = paths.get(("expect_counts", c["entry"], ""))
        elif c["note"] == "named by the key":
            # The old assertion named the entry: the entry the key finds holds that value there.
            base = paths.get((c["new_section"], c["entry"], ""))
            new_path = (base[0], base[1] + tuple(tokens(c["field"]))) if base else None
            got = resolve(result[part], list(new_path[1])) if new_path else KeyError
            new_value = "null" if got is None else got
        else:
            new_value = (section.get(c["entry"]) or {}).get(c["field"])
            new_path = paths.get((c["new_section"], c["entry"], c["field"]))
        if str(new_value) != c["expected"]:
            problems.append(f"{c['key']}: expected {c['expected']!r}, now {new_value!r}")
        if new_path != old_path:
            problems.append(f"{c['key']}: the new key reads {new_path}, the old one {old_path}")
    return problems


def cmd_verify(args: argparse.Namespace) -> int:
    plan = json.loads(Path(args.plan).read_text(encoding="utf-8"))
    root, base = Path(args.root), Path(args.base_root)
    names = args.files or sorted(plan["files"])
    total = 0
    for rel in names:
        entry = plan["files"][rel]
        old_text = (base / rel).read_text(encoding="utf-8")
        new_text = (root / rel).read_text(encoding="utf-8")
        if old_text == new_text:
            continue  # not migrated (yet): nothing to check
        for p in verify_file(old_text, new_text, entry):
            print(f"{rel}: {p}")
            total += 1
    print(f"{len(names)} checked, {total} problems")
    return 1 if total else 0


# ---------------------------------------------------------------- main


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    sub = parser.add_subparsers(dest="cmd", required=True)
    p = sub.add_parser("inventory")
    p.add_argument("--root", default=str(REPO))
    p.add_argument("--markdown")
    p.add_argument("--json")
    p = sub.add_parser("list")
    p.add_argument("--root", default=str(REPO))
    p.add_argument("--plan", default=str(REPO / PLAN))
    p.add_argument("--markdown")
    p = sub.add_parser("apply")
    p.add_argument("files", nargs="+")
    p.add_argument("--root", default=str(REPO))
    p.add_argument("--plan", default=str(REPO / PLAN))
    p = sub.add_parser("verify")
    p.add_argument("files", nargs="*")
    p.add_argument("--root", default=str(REPO))
    p.add_argument("--base-root", default=str(REPO))
    p.add_argument("--plan", default=str(REPO / PLAN))
    args = parser.parse_args(argv)
    return {"inventory": cmd_inventory, "list": cmd_list, "apply": cmd_apply, "verify": cmd_verify}[
        args.cmd
    ](args)


if __name__ == "__main__":
    sys.exit(main())
