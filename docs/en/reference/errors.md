# Error codes

Codes are stable: once released, a code never changes meaning. Every adapter (Python, CLI, MCP,
function calling) reports errors as `{"code": "...", "message": "..."}`.

| Code | Meaning | Typical fix |
| --- | --- | --- |
| `FM-E001` | Invalid input: wrong type, missing required field, unknown field | Check the input against `fairmark schema <method>` |
| `FM-E002` | Empty cash flow series | Provide at least one cash flow |
| `FM-E003` | Rate ≤ −100% | Rates are decimal fractions: 10% is `0.10` |
| `FM-E004` | Terminal growth ≥ discount rate | The Gordon model needs g < WACC |
| `FM-E005` | IRR solver did not converge | Check the cash flows; try a different `guess` |
| `FM-E006` | No sign change in cash flows | An IRR needs both outflows and inflows |
| `FM-E007` | Unknown method or tool | See `fairmark methods` |
| `FM-E008` | Arithmetic overflow | Inputs out of a realistic range |
