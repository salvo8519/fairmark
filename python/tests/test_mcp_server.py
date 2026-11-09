"""MCP server options (issue #264): tool subsets by area and the compact profile."""

from __future__ import annotations

import json
from typing import Any

import anyio
import pytest

import fairmark as fm
from fairmark import mcp_server

METHODS = sorted(m["id"] for m in fm.methods())


def test_mcp_lists_all_tools_by_default() -> None:
    listed = anyio.run(mcp_server.list_tools)
    assert len(listed) == len(METHODS)
    assert all("examples" in t.input_schema for t in listed)


def test_mcp_areas_and_compact() -> None:
    async def listed() -> list[Any]:
        return await mcp_server.list_tools(areas=["statements"], compact=True)

    tools_ = anyio.run(listed)
    assert [t.name for t in tools_] == ["fairmark_statements_reorganise"]
    assert '"default": null' not in json.dumps(tools_[0].input_schema)


def test_mcp_command_line_options() -> None:
    args = mcp_server.parse_args(["--areas", "dcf, tvm", "--compact"])
    assert args.areas == ["dcf", "tvm"] and args.compact is True
    args = mcp_server.parse_args([])
    assert args.areas is None and args.compact is False
    with pytest.raises(SystemExit):
        mcp_server.parse_args(["--areas", "nope"])
