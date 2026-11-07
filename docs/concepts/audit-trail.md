# The audit trail

Every result is an envelope with four parts:

```json
{
  "method": "dcf.fcff",
  "value":   { "enterprise_value": "1421.48…", "equity_value": "1221.48…", "...": "..." },
  "explain": {
    "formula": "EV = Σ FCFF_t × DF_t + TV × DF_n; ...",
    "conventions": ["cash flows discounted at end of period", "..."],
    "steps": [ { "label": "Discount factor, year 1", "formula": "1 / (1 + 0.10)^1", "value": "0.909…" } ]
  },
  "meta": { "engine_version": "…", "method_version": "1", "input_hash": "b963…", "arithmetic": "decimal…" }
}
```

- **`value`** holds the results, exact, as decimal strings.
- **`explain.steps`** lists each intermediate calculation with the actual numbers substituted.
  An analyst can recompute any step by hand; an agent cites them instead of recomputing.
- **`explain.conventions`** states every choice that changes the number (timing, terminal
  value discounting). Conventions are never implicit.
- **`meta`** makes the result reproducible: the same `input_hash` with the same versions
  always yields the same value.

In Python, `result.explain_markdown()` renders a methodology note for reports.
