# MCP server

`fairmark-mcp` exposes every method as an MCP tool. It runs locally over stdio, so there is
no server to host and no data leaves your machine.

## Configure your client

```json
{
  "mcpServers": {
    "fairmark": { "command": "uvx", "args": ["--from", "fairmark[mcp]", "fairmark-mcp"] }
  }
}
```

Or install it in an environment and use `"command": "fairmark-mcp"`.

## What the client gets

- One tool per method (`fairmark_dcf_fcff`, `fairmark_tvm_npv`, ...), with the full input
  schema, marked read-only and idempotent.
- Server instructions telling the model to use the tools for every number and to cite
  `explain.steps`.
- Results as structured content (the full JSON envelope) and as text.
- Errors flagged with `isError` and a stable code, so the model can correct its call.
