"""MCP server exposing every Fairmark method as a tool (local, stdio).

Run with ``uvx --from "fairmark[mcp]" fairmark-mcp`` or ``fairmark-mcp`` after
``pip install "fairmark[mcp]"``. No server to host: the agent starts it locally.
"""

from __future__ import annotations

import json
from typing import Any

from . import ENGINE_VERSION, tools

INSTRUCTIONS = (
    "Fairmark is a deterministic valuation engine. Use its tools for every number in a "
    "valuation (NPV, IRR, DCF). Never compute these values yourself. Rates are decimal "
    "fractions (0.09 = 9%). Pass amounts as strings or numbers. When you report results, "
    "cite explain.steps and keep the returned values unchanged."
)


def _build_server() -> Any:
    try:
        from mcp import types
        from mcp.server.lowlevel import Server
    except ImportError as err:  # pragma: no cover
        raise SystemExit('The MCP server needs the extra: pip install "fairmark[mcp]"') from err

    async def on_list_tools(ctx: Any, params: Any) -> Any:
        return types.ListToolsResult(
            tools=[
                types.Tool(
                    name=d["name"],
                    description=d["description"],
                    input_schema=d["parameters"],
                    annotations=types.ToolAnnotations(
                        read_only_hint=True, idempotent_hint=True, open_world_hint=False
                    ),
                )
                for d in tools.definitions()
            ]
        )

    async def on_call_tool(ctx: Any, params: Any) -> Any:
        out = tools.dispatch(params.name, params.arguments or {})
        return types.CallToolResult(
            content=[types.TextContent(text=json.dumps(out, ensure_ascii=False))],
            structured_content=out,
            is_error="error" in out,
        )

    return Server(
        "fairmark",
        version=ENGINE_VERSION,
        instructions=INSTRUCTIONS,
        on_list_tools=on_list_tools,
        on_call_tool=on_call_tool,
    )


async def serve() -> None:
    from mcp.server.stdio import stdio_server

    server = _build_server()
    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream, server.create_initialization_options())


def main() -> None:
    import anyio

    anyio.run(serve)


if __name__ == "__main__":
    main()
