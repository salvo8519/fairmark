# The audit trail

Every result is an envelope with four parts:

```json
{
  "method": "dcf.fcff",
  "value":   { "value_of_operations": "1421.48…", "equity_value": "1221.48…", "...": "..." },
  "explain": {
    "formula": "Value of operations = Σ FCFF_k × DF_k + TV_N × DF_TV; ...",
    "conventions": ["timing: end of period (flow k discounted at t = k)", "..."],
    "steps": [ { "label": "Period 1: discount factor", "formula": "1 / (1 + 0.10)", "value": "0.909…" } ]
  },
  "meta": { "engine_version": "…", "method_version": "3", "input_hash": "b963…", "arithmetic": "decimal…", "method_status": "preview" }
}
```

- **`value`** holds the results, exact, as decimal strings.
- **`explain.steps`** lists each intermediate calculation with the actual numbers substituted.
  An analyst can recompute any step by hand; an agent cites them instead of recomputing.
- **`explain.conventions`** states every choice that changes the number (timing, terminal
  value discounting). Conventions are never implicit.
- **`meta`** makes the result reproducible: the same `input_hash` with the same versions
  always yields the same value. `meta.method_status` states whether the method was
  `available` or `preview` when it ran (see
  [method status](../project/releases.md#method-status)); report a preview result as such.
- **`meta.defaults_applied`** lists the documented defaults the engine filled in because their
  fields were not given, in input order: for each, `field` (its JSON pointer, for example
  `/timing`), `value` (the value filled in, as it would be written in the input) and `reason`
  (one line). Each is also the first entries of `explain.conventions`, starting with
  "default:". The key is omitted when no default applies, as for every method without
  documented defaults; a field given explicitly always prevails and is never listed. The DCF
  methods use it for their minimal input (SPEC-0001 Decision 24, see
  [`dcf.fcff`](../methods/dcf-fcff.md#minimal-input)).

In Python, `result.explain_markdown()` renders a methodology note for reports.
