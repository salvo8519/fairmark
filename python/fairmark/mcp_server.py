"""MCP server exposing every Fairmark method as a tool.

Two transports:

    fairmark-mcp                      # stdio: started by a local client (Claude, Cursor, ...)
    fairmark-mcp --http               # Streamable HTTP on 127.0.0.1:8765/mcp, for n8n's
                                      # MCP Client Tool and other remote-capable clients

Options for clients that want fewer or smaller tools (default: every tool, full schemas):

    fairmark-mcp --areas dcf,tvm      # only the tools of these areas
    fairmark-mcp --compact            # compact schemas (see fairmark.tools.definitions)

Install with ``pip install "fairmark[mcp]"``. Logs go to stderr; ``--log-format json`` emits
one JSON object per line. Logs never contain financial values (see docs: logging policy).
"""

from __future__ import annotations

import argparse
import json
import logging
import time
from collections.abc import Iterable
from typing import Any

from . import ENGINE_VERSION, tools
from ._core import methods
from ._logging import configure, log_event

INSTRUCTIONS = (
    "Fairmark is a deterministic valuation engine. Use its tools for every number in a "
    "valuation (NPV, IRR, DCF). Never compute these values yourself. Rates are decimal "
    "fractions (0.09 = 9%). Pass amounts as strings or numbers. When you report results, "
    "cite explain.steps and keep the returned values unchanged. Present every result as a "
    "calculation from the method, conventions and inputs you used, never as advice or a "
    "recommendation: decisions belong to the user, who verifies the results before relying "
    "on them (https://salvo8519.github.io/fairmark/disclaimer/). A Fairmark tool whose "
    "description says 'Status: preview' has not completed independent verification: tell "
    "the user when a result comes from it. A refused call returns error.pointer (the "
    "argument to correct), error.reason, error.expected and error.fix: correct every entry of "
    "error.errors before calling again."
)

logger = logging.getLogger("fairmark.mcp")


async def list_tools(*, areas: Iterable[str] | None = None, compact: bool = False) -> list[Any]:
    """One MCP tool per method, with its status and version in `_meta`: `fairmark/status`
    (ADR 0012) and `fairmark/method_version` (ADR 0014 G8). A client compares the version with
    `meta.method_version` of each result to notice that the method changed.

    ``areas`` and ``compact`` are passed to ``fairmark.tools.definitions``.
    """
    from mcp import types

    catalogue = {tools.tool_name(m["id"]): m for m in methods()}
    return [
        types.Tool(
            name=d["name"],
            description=d["description"],
            input_schema=d["parameters"],
            annotations=types.ToolAnnotations(
                read_only_hint=True, idempotent_hint=True, open_world_hint=False
            ),
            _meta={
                "fairmark/status": catalogue[d["name"]]["status"],
                "fairmark/method_version": catalogue[d["name"]]["version"],
            },
        )
        for d in tools.definitions(compact=compact, areas=areas)
    ]


def call_tool(name: str, arguments: Any) -> Any:
    """One tool call: the result envelope, or ``{"error": <report>}`` with the whole error
    report (ADR 0014 G5: pointer, pointers, reason, expected, fix, missing_any_of, errors), as
    text and as structured content; ``isError`` is set for an error."""
    from mcp import types

    started = time.perf_counter()
    out = tools.dispatch(name, arguments)
    # A JSON number reaches this server already read by the MCP library as a binary float:
    # its digits beyond about 17 are lost before Fairmark sees it. Said, never silent.
    if "error" not in out and isinstance(arguments, dict):
        conventions = out.setdefault("explain", {}).setdefault("conventions", [])
        for pointer, value in _floats(arguments, ""):
            conventions.append(
                f"warning: `{pointer}` arrived as a binary floating-point number ({value!r}): "
                "a JSON number may have lost digits before Fairmark read it; send decimals as "
                'strings, for example "0.095", to keep every digit'
            )
    log_event(
        logger,
        "tool_call",
        tool=name,
        outcome=out["error"]["code"] if "error" in out else "ok",
        duration_ms=round((time.perf_counter() - started) * 1000, 3),
        input_hash=out.get("meta", {}).get("input_hash"),
    )
    return types.CallToolResult(
        content=[types.TextContent(text=json.dumps(out, ensure_ascii=False))],
        structured_content=out,
        is_error="error" in out,
    )


def _floats(value: Any, at: str) -> list[tuple[str, float]]:
    """The JSON pointer and value of every Python float in tool arguments."""
    if isinstance(value, float):
        return [(at, value)]
    if isinstance(value, dict):
        return [
            found
            for k, v in value.items()
            for found in _floats(v, f"{at}/{str(k).replace('~', '~0').replace('/', '~1')}")
        ]
    if isinstance(value, list):
        return [found for i, v in enumerate(value) for found in _floats(v, f"{at}/{i}")]
    return []


def _build_server(areas: Iterable[str] | None = None, compact: bool = False) -> Any:
    try:
        from mcp import types
        from mcp.server.lowlevel import Server
    except ImportError as err:  # pragma: no cover
        raise SystemExit('The MCP server needs the extra: pip install "fairmark[mcp]"') from err

    async def on_list_tools(ctx: Any, params: Any) -> Any:
        return types.ListToolsResult(tools=await list_tools(areas=areas, compact=compact))

    async def on_call_tool(ctx: Any, params: Any) -> Any:
        return call_tool(params.name, params.arguments or {})

    return Server(
        "fairmark",
        version=ENGINE_VERSION,
        instructions=INSTRUCTIONS,
        on_list_tools=on_list_tools,
        on_call_tool=on_call_tool,
    )


async def serve_stdio(areas: Iterable[str] | None = None, compact: bool = False) -> None:
    from mcp.server.stdio import stdio_server

    server = _build_server(areas, compact)
    async with stdio_server() as (read_stream, write_stream):
        await server.run(read_stream, write_stream, server.create_initialization_options())


def serve_http(
    host: str, port: int, areas: Iterable[str] | None = None, compact: bool = False
) -> None:
    import uvicorn

    app = _build_server(areas, compact).streamable_http_app(streamable_http_path="/mcp", host=host)
    log_event(logger, "server_start", transport="http", url=f"http://{host}:{port}/mcp")
    uvicorn.run(app, host=host, port=port, log_level="warning")


def _areas(value: str) -> list[str]:
    """`--areas dcf,tvm` -> ["dcf", "tvm"], each a known area."""
    wanted = [a.strip() for a in value.split(",") if a.strip()]
    try:  # an empty selection ("" or ",") is refused like an unknown area
        tools.definitions(areas=wanted)
    except ValueError as err:
        raise argparse.ArgumentTypeError(str(err)) from None
    return wanted


def parse_args(argv: list[str] | None = None) -> argparse.Namespace:
    """Command-line options of `fairmark-mcp`."""
    parser = argparse.ArgumentParser(prog="fairmark-mcp", description=__doc__.split("\n")[0])
    parser.add_argument("--http", action="store_true", help="serve Streamable HTTP")
    parser.add_argument("--host", default="127.0.0.1", help="HTTP bind address")
    parser.add_argument("--port", type=int, default=8765, help="HTTP port")
    parser.add_argument("--log-format", choices=["text", "json"], default="text")
    parser.add_argument("--log-level", default="INFO")
    parser.add_argument(
        "--areas",
        type=_areas,
        default=None,
        help="comma-separated areas whose tools are listed, for example dcf,tvm (default: all)",
    )
    parser.add_argument(
        "--compact",
        action="store_true",
        help="list compact tool schemas (repeated descriptions given once, minimal example only)",
    )
    return parser.parse_args(argv)


def main(argv: list[str] | None = None) -> None:
    args = parse_args(argv)
    configure(args.log_format, args.log_level)

    if args.http:
        serve_http(args.host, args.port, args.areas, args.compact)
    else:
        import anyio

        anyio.run(serve_stdio, args.areas, args.compact)


if __name__ == "__main__":
    main()
