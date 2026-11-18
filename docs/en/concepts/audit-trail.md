# The audit trail

Every result is an envelope with five parts, in this order (ADR 0014 G4; envelope version 1):

```json
{
  "method": "dcf.fcff",
  "value":   { "value_of_operations": "1421.48…", "equity_value": "1221.48…", "...": "..." },
  "explain": {
    "formula": "Value of operations = Σ FCFF_k × DF_k + TV_N × DF_TV; ...",
    "conventions": ["timing: end of period (flow k discounted at t = k)", "..."],
    "steps": [ { "label": "Period 1: discount factor", "formula": "1 / (1 + 0.10)", "value": "0.909…" } ]
  },
  "diagnostics": { "not_computed": [], "warnings": [], "checks": [] },
  "meta": { "engine_version": "…", "envelope_version": 1, "method_version": "3", "method_status": "preview",
            "input_hash": "b963…", "arithmetic": "decimal…", "defaults_applied": [] }
}
```

- **`value`** holds the results, exact, as decimal strings. Its JSON Schema documents every
  field with its unit (`fairmark schema <method> --output`, `fm.output_schema(method)`); in
  Python, `result.value` is decoded by it: each decimal becomes a `Decimal`, and labels, ids,
  versions, pointers and messages stay text even when they look like numbers.
- **`explain.steps`** lists each intermediate calculation with the actual numbers substituted.
  An analyst can recompute any step by hand; an agent cites them instead of recomputing.
- **`explain.conventions`** states every choice that changes the number (timing, terminal
  value discounting). Conventions are never implicit.
- **`diagnostics`** says what the result does not contain, what the reader could miss and what
  was verified, in three lists that are always present, empty when there is nothing to report:
  - `not_computed`: one entry per output not computed, `{pointer, reason_code, reason,
    missing_any_of, permitted}`. `pointer` is the output from the envelope root
    (`/value/...`); `reason_code` is `missing_input` (the inputs that would make it computable
    are in `missing_any_of`, alternative sets of input pointers), `choice_required` (a
    treatment to give, with its permitted values in `permitted`), `not_defined` (a given term
    makes a ratio not defined, for example a negative equity) or `not_applicable` (no input
    could supply it in the form given). Ordered by pointer.
  - `warnings`: `{code, pointer, message}`, with a stable code `FM-Wnnn` documented in the
    [warning codes](../reference/warnings.md); each is echoed in `explain.conventions` as
    `warning FM-Wnnn: <message>`.
  - `checks`: every relation the method verified, `{id, status, pointer, pointers}`, `status`
    `reconciled` or `not_checked` (a relation that fails is an error, `FM-E026`, never a
    check).
- **`meta`** makes the result reproducible: the same `input_hash` with the same versions
  always yields the same value. `meta.method_status` states whether the method was
  `available` or `preview` when it ran (see
  [method status](../project/releases.md#method-status)); report a preview result as such.
- **`meta.envelope_version`** is the version of this envelope: a change of `explain`,
  `diagnostics` or `meta` follows the versioning rules against it, not against the method
  version (snapshot `schemas/envelope/v1.json`).
- **`meta.defaults_applied`** lists the documented defaults the engine filled in because their
  fields were not given, in input order (a depth-first walk of the input, objects in the
  property order of the method's input schema, arrays by index, the same for every method):
  for each, `pointer` (its JSON pointer, for example
  `/timing`), `value` (the value filled in, as it would be written in the input) and `reason`
  (one line). Each is also the first entries of `explain.conventions`, starting with
  "default:". The list is empty when no default applies, as for every method without
  documented defaults; a field given explicitly always prevails and is never listed. The DCF
  methods use it for their minimal input (SPEC-0001 Decision 24, see
  [`dcf.fcff`](../methods/dcf-fcff.md#minimal-input)); `dcf.fcff` also lists a net financial
  position not given, subtracted as zero with a warning (Decision 25): for this entry `value`
  (`"0"`) is the amount subtracted, not a value to write in the field, which is a structure,
  and the default is not part of `meta.input_hash`. A method that maps input items to
  destinations also lists there each item given whose destination is a documented default:
  `pointer` is the pointer of the item, `value` the destination, and the convention says
  "assigned to" instead of "not given, set to" (SPEC-0005 R8, see
  [`statements.reorganise`](../methods/statements-reorganise.md#defaults)).

In Python, `result.steps`, `result.conventions`, `result.diagnostics` and `result.meta` hold the
same trail as data, with `result.not_computed`, `result.warnings`, `result.checks` and
`result.defaults_applied` as lists.
