# Warning codes

A warning tells the reader of a result something they could miss: a value the engine used as
given although it looks wrong, a default that decided an amount, a convention that moves a
value. A warning never stops the calculation and never changes a number: the result is complete
and exact. Conditions that make a result impossible are errors, with their own codes (see
[error codes](errors.md)).

Every warning is an entry of `diagnostics.warnings` in the result envelope:

```json
{"code": "FM-W012", "pointer": "/net_financial_position",
 "message": "no net financial position given (`/net_financial_position`): zero is subtracted, ..."}
```

- **`code`** is stable: `FM-W` and three digits. A code is never reused or renumbered; a new
  condition takes the next free code, which is an additive change of the envelope. A client
  accepts codes it does not know.
- **`pointer`** is an RFC 6901 JSON pointer to the field the warning concerns, from the root
  the table below states: the **input** the client sent (inside an analysis, under
  `/base_input`), or the **envelope** of the result (it then starts with `/value`). It is
  `null` only where the table says so.
- **`message`** is one sentence in English. It is guidance, not contract: its wording may
  change between releases; `code` and `pointer` do not.

Each warning is also echoed once in `explain.conventions`, as `warning FM-Wnnn: <message>`, in
the same order as `diagnostics.warnings`. No convention starts with `warning` without its
entry. Read `diagnostics.warnings` rather than parsing the conventions.

## Catalogue

| Code | Condition | Methods | Pointer root | Message pattern | Added in |
| --- | --- | --- | --- | --- | --- |
| `FM-W001` | A decimal of the input has more digits than a decimal holds and is rounded as it is read (ADR 0014 G7) | every method | input: the field | `` `{pointer}` has more digits than a decimal holds: given {text}, used {used}, rounded half away from zero ... `` | 0.1.0 |
| `FM-W002` | The base value is negative, so a positive growth makes the values more negative | `series.project` | input: `/base_value` | `the base value {v} is negative, ...` | 0.1.0 |
| `FM-W003` | A growth is applied to a negative flow, once per period | `dcf.fcff`, `dcf.fcfe` | input: `/stages/{i}`, the generated stage | `period {k}: growth {g} applied to a negative flow ({prev}); ...` | 0.1.0 |
| `FM-W004` | The nominal perpetual growth is below the expected inflation of the terminal phase | `dcf.fcff`, `dcf.fcfe` | input: the growth of the technique, `/terminal/{id}/growth` or `/terminal/tv.h_model/long_term_growth` | `the perpetual growth {g} (nominal) is below the expected inflation of the terminal phase {pi}: ...` | 0.1.0 |
| `FM-W005` | No terminal value is included, given or by default | `dcf.fcff`, `dcf.fcfe` | input: `/terminal` | `no terminal value is included: the value covers the {n} explicit flows only; ...` | 0.1.0 |
| `FM-W006` | A given terminal value is negative | `dcf.fcff`, `dcf.fcfe` | input: `/terminal/given/value` | `the given terminal value {v} is negative (for example a decommissioning cost); ...` | 0.1.0 |
| `FM-W007` | The terminal value is negative, so its weight is not meaningful | `dcf.fcff`, `dcf.fcfe` | envelope: `/value/terminal_value_weight`; inside an analysis other than goal seek, `null` (the inner result is not returned) | `the terminal value is negative, so the terminal value weight is not meaningful ...` | 0.1.0 |
| `FM-W008` | The growth of the last explicit period differs from the perpetual growth | `dcf.fcff`, `dcf.fcfe` | input: the growth of the technique, as for `FM-W004` | `the growth of the last explicit period ({a}) differs from the perpetual growth of the terminal value ({g})` | 0.1.0 |
| `FM-W009` | A price at a date (exit multiple, liquidation, residual value, given value) is discounted at N − 0.5 | `dcf.fcff`, `dcf.fcfe` | input: `/terminal_discount_time` | `{what}; discounting it at N − 0.5 brings it forward half a period; ...` | 0.1.0 |
| `FM-W010` | Under mid-period timing a perpetuity-growth terminal value is discounted at N | `dcf.fcff`, `dcf.fcfe` | input: `/terminal_discount_time` | `under mid-period timing a perpetuity-growth terminal value is commonly discounted at N − 0.5, ...` | 0.1.0 |
| `FM-W011` | The equity value is negative | `dcf.fcff`, `dcf.fcfe` | envelope: `/value/equity_value`; inside an analysis other than goal seek, `null` | `the equity value is negative ({v}): {reason}; it is reported as computed, not floored at zero` | 0.1.0 |
| `FM-W012` | No net financial position given: zero is subtracted | `dcf.fcff` | input: `/net_financial_position` | `no net financial position given (...): zero is subtracted, so the equity value equals the enterprise value...` | 0.1.0 |
| `FM-W013` | No other items of a net financial position given: the default `[]` decides it | `ratios.capital_structure`, `ratios.debt_service` | input: `/{closing,opening}/net_financial_position/other_items` | `no other items of the net financial position given (...): the net financial position is gross debt less cash; ...` | 0.1.0 |
| `FM-W014` | No reserve balance given: the LLCR assumes no reserve account | `ratios.debt_service` | input: `/debt_schedule/reserve_balance` | `no reserve balance given (...): nothing is added to the present value of CFADS, ...` | 0.1.0 |
| `FM-W015` | A non-zero B10 c or B10 d is placed below EBITDA by the default treatment `operating` | `statements.reorganise` | input: `/treatments/write_downs_of_fixed_assets` or `/treatments/write_downs_of_receivables` | `year {name}: {item} ({v}) is in {line}, below EBITDA, by the default ...` | 0.1.0 |
| `FM-W016` | The method an analysis evaluates is `preview` (ADR 0012) | `analysis.*` | input: `/method` | `Evaluated method {id} is preview: its results have not been independently verified ...` | 0.1.0 |
| `FM-W017` | A value of the input arrived at the adapter as a binary floating-point number: a client library read a JSON number as a float, so digits beyond about 17 may be lost | every method, through function calling with a dict, the MCP server and LangChain | input: the field | `` `{pointer}` arrived as a binary floating-point number ({value}): a JSON number may have lost digits before Fairmark read it; ... `` | 0.1.0 |

## Warnings inside an analysis

An analysis (`analysis.*`) reports the warnings of the evaluation of `base_input` once, whatever
the number of cells, scenarios or bars, with their input pointers under `/base_input`, after
its own `FM-W016`. `analysis.goal_seek` reports the warnings of the solved evaluation instead,
with the pointers into its result under `/value/inner_value`. A cell, scenario or bar that
fails keeps its compact `{code, message}` error and adds no warning.

## How the catalogue is kept

The codes are declared once in the core (`crates/fairmark-core/src/diagnostics.rs`) and written
only through its warning API. A test fails when a code the engine declares is missing from this
page, when a code of this page is not declared or has no emission site, or when production code
writes a convention starting with `warning` outside the API.
