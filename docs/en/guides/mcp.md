# MCP server

`fairmark-mcp` exposes every method as an MCP tool. By default it runs locally over stdio, so
there is no server to host and no data leaves your machine. With `--http` it serves the
Streamable HTTP transport, for n8n and other clients that connect over HTTP.

## Configure your client

```json
{
  "mcpServers": {
    "fairmark": { "command": "uvx", "args": ["--from", "fairmark[mcp]", "fairmark-mcp"] }
  }
}
```

Or install it in an environment and use `"command": "fairmark-mcp"`.

## HTTP mode (n8n and remote clients)

```bash
fairmark-mcp --http                                # http://127.0.0.1:8765/mcp
fairmark-mcp --http --host 0.0.0.0 --port 8765 --log-format json
```

In n8n: add an **MCP Client Tool** to the AI Agent node, choose the HTTP Streamable transport
and set the URL to `http://<host>:8765/mcp`. The agent then sees every Fairmark method as a tool.

The server binds to `127.0.0.1` by default. Expose it on a network only behind your own
authentication (reverse proxy, VPN): the HTTP mode has no built-in access control.

## Fewer or smaller tools

By default the server lists every tool with its full schema. Two options reduce what the
model sees, for clients with a limited context or a narrow task:

```bash
fairmark-mcp --areas dcf,tvm          # only the tools of these areas
fairmark-mcp --compact                # compact schemas
fairmark-mcp --http --areas statements,ratios --compact
```

In a client configuration, add them to `args`, for example `["--from", "fairmark[mcp]",
"fairmark-mcp", "--areas", "dcf,tvm"]`. The areas are the part of the method id before the dot
(`analysis`, `coc`, `dates`, `dcf`, `loans`, `rates`, `ratios`, `series`, `statements`, `tv`,
`tvm`); an empty selection or an unknown area stops the server with the list of known ones. The
compact profile is the one of [function calling](function-calling.md#compact-profile): exactly
the same inputs are accepted as without it, and every enum value keeps its meaning. A tool
outside the selected areas is not listed, but a call to it is still executed.

## Logs

Logs go to stderr; `--log-format json` writes one JSON object per line with the tool, the
outcome (`ok` or an error code), the duration and the input hash. Logs never contain
financial values.

## What the client gets

- One tool per method (`fairmark_dcf_fcff`, `fairmark_tvm_npv`, ...), with the full input
  schema, marked read-only and idempotent.
- The meaning of every enum value in the field description, and input examples taken from
  golden cases in the schema `examples` keyword (minimal first, then maximal where useful;
  see [function calling](function-calling.md#examples)).
- The status of each method in the tool's `_meta`, `{"fairmark/status": "available"}` or
  `"preview"`. The description of a preview tool has "(preview)" after the title and a
  sentence starting "Status: preview", so a model sees it before calling. The
  [method status](../project/releases.md#method-status) page defines both statuses.
- Server instructions telling the model to use the tools for every number, to cite
  `explain.steps`, to tell the user when a result comes from a preview method, and to present
  every result as a calculation under the user's responsibility, never as advice (see the
  [disclaimer](../disclaimer.md)).
- Results as structured content (the full JSON envelope) and as text.
- Errors flagged with `isError` and a stable code, so the model can correct its call.
