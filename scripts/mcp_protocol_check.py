"""The MCP server spoken to over stdio by the reference client of the protocol (issue #331).

`python/tests/test_mcp_server.py` calls the server's Python functions in process. Agent
harnesses do something else: they start `fairmark-mcp` as a process and speak the protocol to
it. This script does the same with the MCP Inspector in CLI mode (the reference client of the
Model Context Protocol, pinned below) and checks that:

1. `tools/list` lists one tool per method of the registry, each with an input schema with
   examples, and the Inspector finds no portability problem in the schemas (`--strict`):
   neither an error nor a warning;
2. the `--areas` and `--compact` options of the server reach the protocol;
3. `tools/call` on golden cases returns, over the protocol, exactly the structured content of
   the in-process call (canonical JSON), and that content meets the golden expectations;
4. an invalid input returns the structured error report as a tool result (`isError`), with the
   JSON pointer of every input error, not a protocol error.

    uv run python scripts/mcp_protocol_check.py            # needs Node.js and npx
    uv run python scripts/mcp_protocol_check.py --server /path/to/fairmark-mcp

Run by `uv run just mcp-check` and by the CI job "MCP · protocol check with the Inspector".
"""

from __future__ import annotations

import argparse
import json
import os
import re
import shutil
import subprocess
import sys
import tempfile
from decimal import Decimal, InvalidOperation
from pathlib import Path
from typing import Any

import yaml

ROOT = Path(__file__).resolve().parents[1]
GOLDEN = ROOT / "tests" / "golden"
# The reference client, pinned: a new version is a reviewed change of this line.
INSPECTOR = "@modelcontextprotocol/inspector@2.15.0"
# One golden case per area, the first case of each method that expects a result.
CASE_METHODS = ("dcf.fcff", "tvm.npv", "ratios.liquidity", "statements.reorganise")
# An input with a non-numeric string and a missing required field.
INVALID_TOOL = "fairmark_tvm_npv"
INVALID_INPUT: dict[str, Any] = {"cash_flows": ["abc"]}
INVALID_POINTERS = ("/cash_flows/0", "/rate")
STRICT_SUMMARY = re.compile(r"(\d+) errors?, (\d+) warnings?")


class Failure(Exception):
    """A check failed; the message says which and why."""


def first_json(text: str) -> Any:
    """The first JSON value printed on stdout (the Inspector may print more after it)."""
    start = min((i for i in (text.find("{"), text.find("[")) if i >= 0), default=-1)
    if start < 0:
        raise Failure(f"no JSON in the output of the Inspector: {text[:200]!r}")
    value, _ = json.JSONDecoder().raw_decode(text[start:])
    return value


def canonical(value: Any) -> str:
    return json.dumps(value, sort_keys=True, ensure_ascii=False, separators=(",", ":"))


def resolve(value: Any, field: str) -> Any:
    """A golden expectation key: a field name, or an RFC 6901 pointer starting with '/'."""
    if not field.startswith("/"):
        return value[field]
    for token in field[1:].split("/"):
        token = token.replace("~1", "/").replace("~0", "~")
        value = value[int(token)] if isinstance(value, list) else value[token]
    return value


def unmet_expectations(case: dict[str, Any], envelope: dict[str, Any]) -> list[str]:
    """The golden expectations a result envelope does not meet, as in python/tests/test_golden."""
    tolerance = Decimal(str(case["tolerance"]))
    checks = [(envelope["value"], f, e) for f, e in case["expect"].items()]
    problems = []
    # Defaults applied, named by their input pointer (SPEC-0013 R1).
    defaults = envelope.get("meta", {}).get("defaults_applied", [])
    for pointer, fields in (case.get("expect_defaults") or {}).items():
        named = [d for d in defaults if d.get("pointer") == pointer]
        if len(named) != 1:
            problems.append(f"expect_defaults {pointer}: {len(named)} entries")
            continue
        checks += [(named[0], f, e) for f, e in (fields or {}).items()]
    for part, field, expected in checks:
        try:
            raw = resolve(part, field)
        except (KeyError, IndexError):
            # `null` means not computed: the last token may be absent under an existing
            # object (SPEC-0013 R4).
            parent_field = field.rsplit("/", 1)[0] if field.startswith("/") else ""
            try:
                parent = resolve(part, parent_field) if parent_field else part
            except (KeyError, IndexError):
                parent = None
            if expected != "null" or not isinstance(parent, dict):
                problems.append(f"{field}: not in the result")
            continue
        try:
            want = Decimal(str(expected))
            if not want.is_finite():
                raise InvalidOperation
        except InvalidOperation:
            if (raw is None) != (expected == "null") or (
                raw is not None and str(raw) != str(expected)
            ):
                problems.append(f"{field}: {raw!r} != {expected!r}")
            continue
        if abs(Decimal(str(raw)) - want) > tolerance:
            problems.append(f"{field}: {raw} != {expected} (tolerance {tolerance})")
    return problems


def golden_case(method: str) -> tuple[Path, dict[str, Any]]:
    for path in sorted((GOLDEN / method).glob("*.yaml")):
        case = yaml.safe_load(path.read_text(encoding="utf-8"))
        if "expect_error" not in case and "expect" in case:
            return path, case
    raise Failure(f"no golden case with an expected result for {method}")


class Inspector:
    """Runs one Inspector CLI command against the server, over stdio."""

    def __init__(self, server: list[str]) -> None:
        self.server = server
        self.npx = shutil.which("npx")
        if self.npx is None:
            raise Failure("npx not found: install Node.js 22.23 or later")
        # The Inspector writes a catalog of servers to ~/.mcp-inspector by default.
        self.home = tempfile.mkdtemp(prefix="fairmark-inspector-")

    def run(self, *args: str, server_args: tuple[str, ...] = ()) -> tuple[int, str, str]:
        env = {**os.environ, "MCP_CATALOG_PATH": str(Path(self.home) / "catalog.json")}
        cmd = [self.npx or "npx", "-y", INSPECTOR, "--cli", *args, *self.server, *server_args]
        out = subprocess.run(
            cmd, capture_output=True, text=True, encoding="utf-8", env=env, timeout=600
        )
        return out.returncode, out.stdout, out.stderr


def check_list(inspector: Inspector, report: list[str]) -> None:
    import fairmark as fm

    code, stdout, stderr = inspector.run("--strict", "--method", "tools/list")
    summary = STRICT_SUMMARY.search(stderr)
    if code == 6:
        raise Failure(f"tools/list --strict: error-severity schema problems\n{stderr[-3000:]}")
    if code != 0:
        raise Failure(f"tools/list failed with exit code {code}\n{stderr[-2000:]}")
    problems = [line for line in stderr.splitlines() if line.startswith(("Warning", "Error"))]
    if (summary and int(summary.group(1)) + int(summary.group(2)) > 0) or problems:
        found = summary.group(0) if summary else f"{len(problems)} problems"
        raise Failure(
            f"tools/list --strict: {found}: the tool schemas must be portable to every client "
            f"(python/fairmark/tools.py)\n{stderr[-3000:]}"
        )
    tools = first_json(stdout)["tools"]
    expected = sorted(fm.tools.tool_name(m["id"]) for m in fm.methods())
    names = sorted(t["name"] for t in tools)
    if names != expected:
        missing, extra = sorted(set(expected) - set(names)), sorted(set(names) - set(expected))
        raise Failure(f"tools/list: missing {missing}, unexpected {extra}")
    no_examples = [t["name"] for t in tools if "examples" not in t.get("inputSchema", {})]
    if no_examples:
        raise Failure(f"tools/list: input schema without examples: {no_examples}")
    report.append(f"tools/list: {len(tools)} tools; schema portability: no problem found")


def check_options(inspector: Inspector, report: list[str]) -> None:
    code, stdout, stderr = inspector.run(
        "--method", "tools/list", server_args=("--areas", "statements", "--compact")
    )
    if code != 0:
        raise Failure(f"tools/list with --areas and --compact failed ({code})\n{stderr[-2000:]}")
    tools = first_json(stdout)["tools"]
    names = [t["name"] for t in tools]
    if names != ["fairmark_statements_reorganise"]:
        raise Failure(f"--areas statements --compact listed {names}")
    if '"default": null' in json.dumps(tools[0]["inputSchema"]):
        raise Failure("--compact: the schema still lists null defaults")
    report.append("server options --areas and --compact: reach the protocol")


def check_calls(inspector: Inspector, report: list[str]) -> None:
    from fairmark import mcp_server, tools

    for method in CASE_METHODS:
        path, case = golden_case(method)
        name = tools.tool_name(method)
        arguments = case["input"]
        code, stdout, stderr = inspector.run(
            "-q",
            "--method",
            "tools/call",
            "--tool-name",
            name,
            "--tool-args-json",
            json.dumps(arguments),
        )
        if code != 0:
            raise Failure(f"{name} on {path.name}: exit code {code}\n{stderr[-2000:]}")
        result = first_json(stdout)
        if result.get("isError"):
            raise Failure(f"{name} on {path.name}: isError\n{canonical(result)[:2000]}")
        expected = json.loads(json.dumps(mcp_server.call_tool(name, arguments).structured_content))
        if canonical(result.get("structuredContent")) != canonical(expected):
            raise Failure(
                f"{name} on {path.name}: protocol result differs from the in-process call"
            )
        text = result["content"][0]["text"]
        if canonical(json.loads(text)) != canonical(expected):
            raise Failure(f"{name} on {path.name}: text content differs from structured content")
        problems = unmet_expectations(case, result["structuredContent"])
        if problems:
            raise Failure(f"{name} on {path.name}: golden expectations not met: {problems}")
        report.append(f"tools/call {name}: {path.relative_to(ROOT)} identical and as expected")


def check_error(inspector: Inspector, report: list[str]) -> None:
    code, stdout, stderr = inspector.run(
        "-q",
        "--method",
        "tools/call",
        "--tool-name",
        INVALID_TOOL,
        "--tool-args-json",
        json.dumps(INVALID_INPUT),
    )
    try:
        result = first_json(stdout)
    except Failure as err:
        raise Failure(
            f"invalid input: no tool result (exit code {code})\n{stderr[-2000:]}"
        ) from err
    if "error" in result and "content" not in result:
        raise Failure(f"invalid input: protocol error instead of a tool result: {result}")
    if result.get("isError") is not True:
        raise Failure("invalid input: the tool result is not flagged isError")
    error = result["structuredContent"]["error"]
    pointers = {e.get("pointer") for e in error.get("errors", [])}
    missing = [p for p in INVALID_POINTERS if p not in pointers]
    if error.get("code") != "FM-E001" or missing:
        raise Failure(f"invalid input: code {error.get('code')}, pointers missing {missing}")
    report.append(f"invalid input: {error['code']} with pointers {sorted(pointers)}")


def default_server() -> list[str]:
    beside = Path(sys.executable).with_name("fairmark-mcp")
    found = str(beside) if beside.exists() else shutil.which("fairmark-mcp")
    if found is None:
        raise Failure('fairmark-mcp not found: uv sync --all-extras (extra "mcp")')
    return [found]


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--server", nargs="+", help="command that starts the stdio server")
    args = parser.parse_args(argv)
    report: list[str] = []
    try:
        inspector = Inspector(args.server or default_server())
        for check in (check_list, check_options, check_calls, check_error):
            check(inspector, report)
    except Failure as err:
        print("\n".join(report))
        print(f"MCP protocol check failed: {err}", file=sys.stderr)
        return 1
    print("\n".join(report))
    print(f"MCP protocol check passed ({INSPECTOR}).")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
