"""scripts/envelope_inventory.py (SPEC-0012 T1): read-only, tested on temporary trees."""

from __future__ import annotations

import importlib.util
import json
import sys
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[2]
SCRIPT = ROOT / "scripts/envelope_inventory.py"
_spec = importlib.util.spec_from_file_location("envelope_inventory", SCRIPT)
assert _spec and _spec.loader
inv = importlib.util.module_from_spec(_spec)
sys.modules["envelope_inventory"] = inv  # dataclasses resolve types through sys.modules
_spec.loader.exec_module(inv)

RUST = r"""
use x::y;
// "warning: in a comment" is not a literal
fn emits(ex: &mut Explain) {
    let brace = '{';
    let quote = '"';
    let r = r#"raw { "string" }"#;
    ex.convention(format!(
        "warning: the base value {} is negative, so a positive growth makes the values more \
         negative; check the sign of the series",
        v
    ));
}

fn other<'a>(s: &'a str) -> &'a str {
    "warning: unknown condition"
}

#[cfg(test)]
mod tests {
    #[test]
    fn t() {
        let s = "warning: inside a test }";
    }
}
"""


def make_tree(tmp: Path, rust: str = RUST) -> Path:
    src = tmp / inv.CORE
    src.mkdir(parents=True)
    (src / "series.rs").write_text(rust, encoding="utf-8")
    schemas = tmp / "schemas"
    (schemas / "a.method").mkdir(parents=True)
    (schemas / "b.method").mkdir(parents=True)
    out_a = {
        "type": "object",
        "properties": {"total": {"type": "string"}, "not_computed": {"type": "array"}},
        "$defs": {
            "Line": {
                "properties": {
                    "source": {"type": "string", "description": "JSON pointer of the input."},
                    "source_pointer": {"type": "string", "description": "JSON pointer of it."},
                    "lines": {
                        "type": "array",
                        "items": {"$ref": "#/$defs/Line"},
                        "description": "Lines, each with the pointer of its input.",
                    },
                }
            }
        },
    }
    out_b = {"type": "object", "properties": {"total": {"type": "string"}}}
    in_b = {
        "type": "object",
        "properties": {
            "output": {"type": "string", "description": "RFC 6901 JSON Pointer to the output."},
            "output_pointer": {"type": "string", "description": "JSON Pointer to the output."},
        },
    }
    (schemas / "b.method/v3.json").write_text(json.dumps(in_b), encoding="utf-8")
    (schemas / "a.method/v1.output.json").write_text(json.dumps(out_a), encoding="utf-8")
    (schemas / "b.method/v3.output.json").write_text(json.dumps(out_b), encoding="utf-8")
    index = {
        "format": 1,
        "methods": {
            "a.method": {
                "current": "1",
                "versions": {"1": {"output_file": "a.method/v1.output.json"}},
            },
            "b.method": {
                "current": "3",
                "versions": {
                    "3": {"file": "b.method/v3.json", "output_file": "b.method/v3.output.json"}
                },
            },
        },
    }
    (schemas / "index.json").write_text(json.dumps(index), encoding="utf-8")
    return tmp


def test_lexer_reads_literals_and_skips_comments_chars_and_lifetimes() -> None:
    literals, code = inv.scan(RUST)
    texts = [lit.text for lit in literals]
    assert "warning: in a comment" not in " ".join(texts)
    assert 'raw { "string" }' in texts
    # The continuation is joined, the literal is reported on its first line.
    joined = next(lit for lit in literals if lit.text.startswith("warning: the base value"))
    assert "more negative; check" in joined.text
    assert RUST.splitlines()[joined.line - 1].strip().startswith('"warning: the base value')
    # Offsets are kept and no brace survives inside a blanked string or char literal.
    assert len(code) == len(RUST)
    assert code.count("{") == code.count("}")


def test_cfg_test_items_are_excluded() -> None:
    texts = [lit.text for lit in inv.production_literals(RUST)]
    assert "warning: inside a test }" not in texts
    assert "warning: unknown condition" in texts
    lines = [line for _, line in inv.production_lines(RUST)]
    assert not any("mod tests" in line for line in lines)


def test_sites_are_matched_to_the_catalogue(tmp_path: Path) -> None:
    root = make_tree(tmp_path)
    sites = inv.warning_sites(root)
    assert len(sites) == 2
    known = next(s for s in sites if s["code"])
    assert known["code"] == "FM-W002" and known["function"] == "emits"
    unknown = next(s for s in sites if s["text"].startswith("warning: unknown"))
    assert unknown["code"] is None and unknown["function"] == "other"


def test_differences_name_unmatched_sites_missing_codes_and_schema_moves(tmp_path: Path) -> None:
    root = make_tree(tmp_path)
    result = inv.inventory(root, engine=False)
    diffs = "\n".join(result["differences"])
    assert "warning literal not in the catalogue: series.rs:" in diffs
    assert "catalogue code without an emission site: FM-W001" in diffs
    assert "without an emission site: FM-W002" not in diffs
    assert "output moves a list but the spec does not say so: a.method" in diffs
    assert "the spec moves a list but no field moves: statements.reorganise" in diffs
    assert "the spec renames pointer fields but none is misnamed: analysis.tornado" in diffs
    assert "in a method not bumped: b.method input /output" in diffs
    a = result["schemas"]["a.method"]
    assert a["moves"] == ["/not_computed"]
    assert a["misnamed"] == ["$defs/Line/source"]
    b = result["schemas"]["b.method"]
    assert (b["moves"], b["misnamed"], b["misnamed_input"]) == ([], [], ["/output"])


def test_main_prints_markdown_and_check_fails_on_differences(
    tmp_path: Path, capsys: pytest.CaptureFixture[str]
) -> None:
    root = make_tree(tmp_path)
    assert inv.main(["--root", str(root), "--no-engine"]) == 0
    out = capsys.readouterr().out
    assert out.startswith("# SPEC-0012 envelope inventory")
    assert "not collected" in out
    assert inv.main(["--root", str(root), "--no-engine", "--check"]) == 1
    capsys.readouterr()
    assert inv.main(["--root", str(root), "--no-engine", "--json"]) == 0
    assert "differences" in json.loads(capsys.readouterr().out)
