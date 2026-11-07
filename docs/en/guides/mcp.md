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

## Logs

Logs go to stderr; `--log-format json` writes one JSON object per line with the tool, the
outcome (`ok` or an error code), the duration and the input hash. Logs never contain
financial values.

## What the client gets

- One tool per method (`fairmark_dcf_fcff`, `fairmark_tvm_npv`, ...), with the full input
  schema, marked read-only and idempotent.
- Server instructions telling the model to use the tools for every number and to cite
  `explain.steps`.
- Results as structured content (the full JSON envelope) and as text.
- Errors flagged with `isError` and a stable code, so the model can correct its call.
