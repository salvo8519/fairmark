# Error codes

Codes are stable: once released, a code never changes meaning. Every adapter (Python, CLI, MCP,
function calling) reports errors as `{"code": "...", "message": "..."}`.

| Code | Meaning | Typical fix |
| --- | --- | --- |
| `FM-E001` | Invalid input: wrong type, missing required field, unknown field | Check the input against `fairmark schema <method>` |
| `FM-E002` | Empty cash flow series | Provide at least one cash flow |
| `FM-E003` | Rate out of range: the rate per period, or per compounding period, is at or below −100% | Rates are decimal fractions: 10% is `0.10`; a nominal rate with compounding is checked per compounding period ($r/m$) |
| `FM-E004` | Growth ≥ discount rate (terminal value, perpetuity) | The Gordon model needs g < r |
| `FM-E005` | IRR or XIRR solver did not converge | Check the cash flows; try a different `guess` |
| `FM-E006` | No sign change in cash flows | IRR, XIRR and MIRR need both outflows and inflows |
| `FM-E007` | Unknown method or tool | See `fairmark methods` |
| `FM-E008` | Arithmetic overflow | Inputs out of a realistic range |
| `FM-E009` | Invalid date: malformed, impossible (`2023-02-29`) or outside 1900-01-01 to 2199-12-31 | Write dates as ISO 8601 `YYYY-MM-DD` within the supported range |
| `FM-E010` | A cash flow is dated before the valuation date | Move the valuation date to or before the earliest flow, or omit it to use the earliest flow date |
| `FM-E011` | More than one rate sets the present value to zero, or the sign changes of the flows show that another rate exists; the message lists the intervals | The IRR is ambiguous: use NPV at the cost of capital, or MIRR with explicit rates |
| `FM-E012` | Internal error: a defect in Fairmark, not in the input | Report it with the `input_hash`; the input is never needed |
