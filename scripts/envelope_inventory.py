"""Read-only inventory of what SPEC-0012 (one envelope with diagnostics) changes, task T1.

    python scripts/envelope_inventory.py [--root PATH] [--no-engine] [--check]

Reports, from the sources and the schema snapshots of the repository:

1. every `warning` string literal in the production code of `crates/fairmark-core/src`
   (test items under `#[cfg(test)]` excluded), matched to the codes FM-W001 to FM-W016 of the
   spec catalogue, plus the preview convention of the analyses (FM-W016);
2. the callers of `ratios::warn_defaults` (FM-W013);
3. every `DefaultApplied::new` and `DefaultApplied::destination` call in production code;
4. in the current output schema of every method: the fields named `not_computed`, `checks` and
   `not_defined` (they move to the diagnostics, so their method changes version), and every
   field whose description mentions a pointer but whose name does not follow ADR 0014 G4 as
   amended (`pointer`, `pointers`, `*_pointer`, `*_pointers`, `missing_any_of`);
5. with the engine importable (and without `--no-engine`), the relation ids of the checks of
   `statements.reorganise` seen in its golden results, by form.

It ends with the comparison against the spec: the sixteen codes and the seven methods whose
output changes. `--check` exits with status 1 when they differ. Nothing is written.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from dataclasses import dataclass
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
CORE = Path("crates/fairmark-core/src")

# The catalogue of SPEC-0012: code -> (source file under CORE, start of the literal after the
# `warning: ` prefix). FM-W016 is the preview convention, which has no `warning: ` prefix.
CATALOGUE: dict[str, tuple[str, str]] = {
    "FM-W001": ("reading.rs", "`{field}` has more digits than a decimal holds"),
    "FM-W002": ("series.rs", "the base value {} is negative"),
    "FM-W003": ("dcf/engine.rs", "period {k}: growth {g} applied to a negative flow"),
    "FM-W004": ("dcf/engine.rs", "the perpetual growth {g} (nominal) is below"),
    "FM-W005": ("dcf/terminal.rs", "no terminal value is included"),
    "FM-W006": ("dcf/terminal.rs", "the given terminal value {v} is negative"),
    "FM-W007": ("dcf/terminal.rs", "the terminal value is negative, so the terminal value weight"),
    "FM-W008": ("dcf/terminal.rs", "the growth of the last explicit period"),
    "FM-W009": ("dcf/terminal.rs", "{what}; discounting it at N"),
    "FM-W010": ("dcf/terminal.rs", "under mid-period timing a perpetuity-growth terminal value"),
    "FM-W011": ("dcf/bridge.rs", "the equity value is negative"),
    "FM-W012": ("dcf/fcff.rs", "no net financial position given"),
    "FM-W013": ("ratios/mod.rs", "no other items of the net financial position given"),
    "FM-W014": ("ratios/debt_service.rs", "no reserve balance given"),
    "FM-W015": ("statements/income_statement.rs", "year {name}: {item}"),
    "FM-W016": ("analysis/mod.rs", "Evaluated method {} is preview"),
}
# Literals that start with `warning` but emit nothing: (file, start of the literal, why).
NOT_EMISSIONS: list[tuple[str, str, str]] = [
    (
        "registry.rs",
        "warning: `/base_input{v}` ",
        "goal seek: filters the rounding warnings of its own variable out of the inner result",
    ),
]
# Methods whose output schema changes shape (R18 of the spec).
SPEC_BUMPED = {
    "statements.reorganise",
    "ratios.profitability",
    "ratios.capital_structure",
    "ratios.liquidity",
    "ratios.working_capital",
    "ratios.debt_service",
    "ratios.covenants",
}
FORBIDDEN_FIELDS = ("not_computed", "checks", "not_defined")
POINTER_NAME = re.compile(r"^(pointer|pointers|[a-z0-9_]+_pointers?|missing_any_of)$")


# ------------------------------------------------------------------------------ Rust sources


@dataclass(frozen=True)
class Literal:
    """A string literal of a Rust source: its first line (1-based) and its text, unescaped
    only for line continuations."""

    line: int
    text: str


def _skip_string(src: str, i: int) -> int:
    """Index after the string literal starting at `i` (a `"`, or `r`/`br` raw string)."""
    if src[i] == "b":
        i += 1
    if src[i] == "r":
        j = i + 1
        hashes = 0
        while src[j] == "#":
            hashes += 1
            j += 1
        end = src.index('"' + "#" * hashes, j + 1)
        return end + 1 + hashes
    j = i + 1
    while src[j] != '"':
        j += 2 if src[j] == "\\" else 1
    return j + 1


def _is_raw_start(src: str, i: int) -> bool:
    if i > 0 and (src[i - 1].isalnum() or src[i - 1] == "_"):
        return False
    m = re.match(r'b?r#*"', src[i:])
    return m is not None


def scan(src: str) -> tuple[list[Literal], str]:
    """String literals of `src`, and `src` with every string, char literal and comment blanked.

    A small lexer: enough to tell code from strings, comments and char literals, so that
    braces inside them are not counted. Offsets are preserved in the blanked text.
    """
    literals: list[Literal] = []
    code = list(src)
    i, n = 0, len(src)

    def blank(a: int, b: int) -> None:
        code[a:b] = [ch if ch == "\n" else " " for ch in src[a:b]]

    while i < n:
        c = src[i]
        if src.startswith("//", i):
            end = src.find("\n", i)
            end = n if end < 0 else end
            blank(i, end)
            i = end
        elif src.startswith("/*", i):
            depth, j = 1, i + 2
            while depth and j < n:
                if src.startswith("/*", j):
                    depth, j = depth + 1, j + 2
                elif src.startswith("*/", j):
                    depth, j = depth - 1, j + 2
                else:
                    j += 1
            blank(i, j)
            i = j
        elif c == '"' or (c in "br" and _is_raw_start(src, i)):
            end = _skip_string(src, i)
            body = src[i:end]
            text = re.sub(r"\\\n\s*", "", body[body.index('"') + 1 : body.rindex('"')])
            literals.append(Literal(src.count("\n", 0, i) + 1, text))
            blank(i, end)
            i = end
        elif c == "'":
            # A char literal ('a', an escape, a unicode escape, '{') or a lifetime ('a).
            m = re.match(r"'(\\u\{[0-9a-fA-F]+\}|\\.|[^\\'])'", src[i:])
            if m:
                blank(i, i + len(m.group(0)))
                i += len(m.group(0))
            else:
                i += 1
        else:
            i += 1
    return literals, "".join(code)


def test_line_spans(src: str) -> list[tuple[int, int]]:
    """First and last line (1-based) of every item annotated `#[cfg(test)]`."""
    _, code = scan(src)
    spans = []
    for m in re.finditer(r"#\[cfg\(test\)\]", code):
        j = m.end()
        while j < len(code) and code[j] not in "{;":
            j += 1
        if j < len(code) and code[j] == "{":
            depth = 0
            while j < len(code):
                depth += {"{": 1, "}": -1}.get(code[j], 0)
                if depth == 0:
                    break
                j += 1
        spans.append((src.count("\n", 0, m.start()) + 1, src.count("\n", 0, j) + 1))
    return spans


def production_literals(src: str) -> list[Literal]:
    """The string literals of `src` outside `#[cfg(test)]` items."""
    literals, _ = scan(src)
    spans = test_line_spans(src)
    return [lit for lit in literals if not any(a <= lit.line <= b for a, b in spans)]


def production_lines(src: str) -> list[tuple[int, str]]:
    """The lines of `src` outside `#[cfg(test)]` items, numbered from 1."""
    spans = test_line_spans(src)
    return [
        (n, line)
        for n, line in enumerate(src.splitlines(), start=1)
        if not any(a <= n <= b for a, b in spans)
    ]


def enclosing_fn(src: str, line: int) -> str:
    """The name of the last `fn` declared at or before `line`."""
    names = re.findall(r"\bfn\s+([a-z_][a-z0-9_]*)", "\n".join(src.splitlines()[:line]))
    return names[-1] if names else "-"


def rust_sources(root: Path) -> list[tuple[str, str]]:
    base = root / CORE
    return [
        (p.relative_to(base).as_posix(), p.read_text(encoding="utf-8"))
        for p in sorted(base.rglob("*.rs"))
    ]


def warning_sites(root: Path) -> list[dict[str, Any]]:
    """Every production literal starting with `warning`, and the preview convention."""
    sites = []
    for rel, src in rust_sources(root):
        for lit in production_literals(src):
            text = lit.text.lstrip()
            preview = text.startswith("Evaluated method") and "is preview" in text
            if not (text.lower().startswith("warning") or preview):
                continue
            body = text.removeprefix("warning: ")
            code = next(
                (c for c, (f, start) in CATALOGUE.items() if f == rel and body.startswith(start)),
                None,
            )
            why = next(
                (w for f, start, w in NOT_EMISSIONS if f == rel and text.startswith(start)), None
            )
            sites.append(
                {
                    "file": rel,
                    "line": lit.line,
                    "function": enclosing_fn(src, lit.line),
                    "code": code,
                    "not_an_emission": why,
                    "text": text[:110],
                }
            )
    return sites


def calls(root: Path, pattern: str) -> list[dict[str, Any]]:
    """Production lines matching `pattern`, with the next lines up to the first argument."""
    out = []
    regex = re.compile(pattern)
    for rel, src in rust_sources(root):
        lines = src.splitlines()
        for n, line in production_lines(src):
            if regex.search(line) and not line.lstrip().startswith(("//", "use ", "pub(crate) fn")):
                tail = " ".join(s.strip() for s in lines[n - 1 : n + 2])
                out.append(
                    {
                        "file": rel,
                        "line": n,
                        "function": enclosing_fn(src, n),
                        "text": tail[:140],
                    }
                )
    return out


# ------------------------------------------------------------------------------ schemas


def output_schemas(root: Path) -> dict[str, tuple[str, dict[str, Any]]]:
    """Current version and output schema of every method in `schemas/index.json`."""
    index = json.loads((root / "schemas/index.json").read_text(encoding="utf-8"))
    out = {}
    for method, entry in sorted(index["methods"].items()):
        version = entry["current"]
        file = entry["versions"][version].get("output_file")
        if file:
            schema = json.loads((root / "schemas" / file).read_text(encoding="utf-8"))
            out[method] = (version, schema)
    return out


def schema_fields(schema: Any, at: str = "") -> list[tuple[str, str, str]]:
    """(location, property name, description) of every property of `schema`, `$defs` included."""
    found: list[tuple[str, str, str]] = []
    if isinstance(schema, dict):
        for name, sub in (schema.get("properties") or {}).items():
            desc = sub.get("description", "") if isinstance(sub, dict) else ""
            found.append((f"{at}/{name}", name, desc))
        for key, sub in schema.items():
            if key == "properties" and isinstance(sub, dict):
                for name, prop in sub.items():
                    found += schema_fields(prop, f"{at}/{name}")
            elif key == "$defs" and isinstance(sub, dict):
                for name, prop in sub.items():
                    found += schema_fields(prop, f"$defs/{name}")
            elif isinstance(sub, (dict, list)) and key not in ("properties", "$defs"):
                found += schema_fields(sub, at)
    elif isinstance(schema, list):
        for item in schema:
            found += schema_fields(item, at)
    return found


def schema_findings(root: Path) -> dict[str, dict[str, Any]]:
    findings: dict[str, dict[str, Any]] = {}
    for method, (version, schema) in output_schemas(root).items():
        fields = sorted(set(schema_fields(schema)))
        forbidden = sorted({loc for loc, name, _ in fields if name in FORBIDDEN_FIELDS})
        misnamed = sorted(
            {
                loc
                for loc, name, desc in fields
                if "pointer" in desc.lower() and not POINTER_NAME.match(name)
            }
        )
        if forbidden or misnamed:
            findings[method] = {"version": version, "moves": forbidden, "misnamed": misnamed}
    return findings


# ------------------------------------------------------------------------------ engine


def check_ids(root: Path) -> dict[str, list[str]] | None:
    """Relation ids of `statements.reorganise` seen in its golden results, by form."""
    try:
        import yaml

        import fairmark
    except ImportError:
        return None
    seen: dict[str, set[str]] = {}
    for path in sorted((root / "tests/golden/statements.reorganise").glob("*.yaml")):
        case = yaml.safe_load(path.read_text(encoding="utf-8"))
        if "expect_error" in case:
            continue
        result = fairmark.run_json(case["method"], case["input"])
        for year in result["value"].get("years", []):
            for relation in year.get("checks") or {}:
                seen.setdefault(relation, set()).add(str(year.get("form")))
    return {relation: sorted(forms) for relation, forms in sorted(seen.items())}


# ------------------------------------------------------------------------------ report


def inventory(root: Path, engine: bool) -> dict[str, Any]:
    sites = warning_sites(root)
    schemas = schema_findings(root)
    emitted = {s["code"] for s in sites if s["code"]}
    bumped = {m for m, f in schemas.items() if f["moves"]}
    differences = []
    for site in sites:
        if not site["code"] and not site["not_an_emission"]:
            differences.append(
                f"warning literal not in the catalogue: {site['file']}:{site['line']}"
            )
    for code in sorted(set(CATALOGUE) - emitted):
        differences.append(f"catalogue code without an emission site: {code}")
    for code in sorted(emitted):
        count = sum(1 for s in sites if s["code"] == code)
        if count > 1:
            differences.append(f"{code} has {count} emission sites")
    for method in sorted(bumped - SPEC_BUMPED):
        differences.append(f"output changes but no version bump in the spec: {method}")
    for method in sorted(SPEC_BUMPED - bumped):
        differences.append(f"version bump in the spec but no field moves: {method}")
    for method, f in sorted(schemas.items()):
        if f["misnamed"] and method not in SPEC_BUMPED:
            differences.append(
                f"pointer fields not named per G4 in a method not bumped: {method} "
                + ", ".join(f["misnamed"])
            )
    return {
        "warnings": sites,
        "warn_defaults_callers": calls(root, r"\bwarn_defaults\("),
        "defaults_applied": calls(root, r"DefaultApplied::(new|destination)\("),
        "schemas": schemas,
        "check_ids": check_ids(root) if engine else None,
        "differences": differences,
    }


def render(inv: dict[str, Any]) -> str:
    out = ["# SPEC-0012 envelope inventory", "", "## Warnings and the preview convention"]
    for s in inv["warnings"]:
        label = s["code"] or ("not an emission" if s["not_an_emission"] else "UNMATCHED")
        out.append(f"- {label}: {s['file']}:{s['line']} `{s['function']}`: {s['text']}")
        if s["not_an_emission"]:
            out.append(f"  ({s['not_an_emission']})")
    out += ["", "## Callers of warn_defaults (FM-W013)"]
    out += [f"- {c['file']}:{c['line']} `{c['function']}`" for c in inv["warn_defaults_callers"]]
    out += ["", "## Defaults applied (production calls)"]
    out += [
        f"- {c['file']}:{c['line']} `{c['function']}`: {c['text']}" for c in inv["defaults_applied"]
    ]
    out += ["", "## Output schemas (current versions)"]
    for method, f in inv["schemas"].items():
        out.append(f"- {method} v{f['version']}")
        if f["moves"]:
            out.append(f"  - moves to the diagnostics: {', '.join(f['moves'])}")
        if f["misnamed"]:
            out.append(f"  - pointer fields not named per G4: {', '.join(f['misnamed'])}")
    out += ["", "## Check relation ids of statements.reorganise seen in the golden results"]
    if inv["check_ids"] is None:
        out.append("- not collected (engine not importable or --no-engine)")
    else:
        out += [f"- {r} ({', '.join(forms)})" for r, forms in inv["check_ids"].items()]
    out += ["", "## Differences from the spec"]
    out += [f"- {d}" for d in inv["differences"]] or ["- none"]
    return "\n".join(out) + "\n"


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--no-engine", action="store_true", help="skip the golden runs")
    parser.add_argument("--json", action="store_true", help="print JSON instead of Markdown")
    parser.add_argument("--check", action="store_true", help="exit 1 on a difference")
    args = parser.parse_args(argv)
    inv = inventory(args.root, engine=not args.no_engine)
    sys.stdout.write(json.dumps(inv, indent=2) + "\n" if args.json else render(inv))
    return 1 if args.check and inv["differences"] else 0


if __name__ == "__main__":
    sys.exit(main())
