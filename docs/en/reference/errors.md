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
| `FM-E013` | Goal seek: the output minus the target has the same sign at both ends of the bracket | Widen the bracket so that the target lies between the outputs at its two ends |
| `FM-E014` | Goal seek did not reach the tolerance within `max_iterations`; also returned when the output jumps across the target (discontinuity) | Raise `max_iterations` or the tolerance; check that the output is continuous in the variable |
| `FM-E015` | An analysis pointer (`variable`, override `pointer`, `output`) does not resolve to a decimal | Use an RFC 6901 pointer to an existing decimal field, for example `/discount_rate` |
| `FM-E016` | The analysis needs more than 10,000 evaluations of the inner method, or more than 1,000,000 evaluations × values in `base_input` | Split the request into smaller tables, or shorten the inner input |
| `FM-E017` | An analysis method was asked to evaluate another analysis method | Run the inner analysis directly; analyses do not nest |
| `FM-E018` | A terminal value input is outside its economic domain: a return on new invested capital or a capitalisation rate of zero or below, a multiple or metric of zero or below, a finite horizon outside 1 to 10,000 periods, a negative half-life, or a negative asset value, liability, liquidation cost or tax on gains | Check the sign and the unit of the field named in the message; obligations belong in `liabilities`, not as negative assets |
| `FM-E019` | Enterprise and equity measures are mixed: a comparison of terminal values declared on one basis contains an exit multiple, a normalised income or a liquidation of the other basis (P/E and P/BV are equity; EV/EBITDA, EV/EBIT, EV/Sales are enterprise) | Compare techniques on one basis: use an enterprise multiple with an enterprise comparison, P/E or P/BV with an equity comparison, and declare a normalised income or a liquidation on the basis of the comparison |
| `FM-E020` | Balloon amount out of range: negative, not lower than the principal, or given with a `bullet` schedule | Give a balloon amount of at least 0 and lower than the principal; omit it for `bullet` |
| `FM-E021` | The instalment does not exceed the interest of the first period: the outstanding balance never decreases and the loan is never repaid | Raise the instalment above periodic rate × principal (for payments in advance, periodic rate × principal / (1 + periodic rate)) |
