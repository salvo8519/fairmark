# Command line

The `fairmark` binary reads JSON and writes JSON. Errors go to stderr as JSON with a
non-zero exit code.

Exit codes:

| Code | Meaning |
| --- | --- |
| `0` | Success: the result was written to stdout, or the reader closed the pipe early |
| `1` | Error: the JSON report on stderr gives the code and the message; `verify` also exits with `1` when the result does not match |
| `2` | Invalid command line (unknown subcommand or option); the message comes from the argument parser |

When the program reading the output stops early, as in `fairmark methods | head -1`,
`fairmark` stops writing and exits with `0` and nothing on stderr. The reader chose to stop,
so this is not an error, and `set -o pipefail` scripts are not failed by it. This differs from
C tools such as `cat`, which are ended by the `SIGPIPE` signal and report `141` in the shell.

```bash
fairmark methods                                   # list methods, with version and status
fairmark schema dcf.fcff                           # JSON Schema of the inputs
fairmark schema dcf.fcff --output                  # JSON Schema of the result value
fairmark schema dcf.fcff --envelope                # JSON Schema of the whole result
fairmark run dcf.fcff --input company.json         # full result: value, explain, meta
fairmark run dcf.fcff --input company.json --value-only
cat company.json | fairmark run dcf.fcff           # stdin works too
fairmark verify result.json --input company.json   # reproduce and compare
```

Example input:

```json
{
  "stages": [
    {"periods": 5, "projection": "given",
     "flows": ["1200000", "1350000", "1500000", "1600000", "1700000"]}
  ],
  "cash_flow_terms": "nominal",
  "discount_rate_terms": "nominal",
  "wacc": "0.095",
  "timing": "mid_period",
  "terminal_discount_time": "horizon",
  "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
  "net_financial_position": {"gross_debt": "3100000", "cash": "0"}
}
```

## Schemas of results

`fairmark schema <method> --output` prints the JSON Schema of the result `value` as the engine
writes it: every decimal is a string matching `^-?\d+(\.\d+)?$`, a field that may be absent is
not `required`, and a field that may be `null` says so. `--envelope` prints the schema of the
whole result, with `explain` and `meta`. The output schema of each method version is kept in
`schemas/<method>/v<N>.output.json`; a breaking change to it is a new method version (see
[method versions](../project/releases.md#method-versions)). A value added to an output
enumeration is not breaking: a client accepts enumeration values it does not know.

A field that holds the input or the output of another method, such as `solved_input` and
`inner_value` of `analysis.goal_seek`, carries `x-fairmark-input-of` or
`x-fairmark-output-of`: an RFC 6901 pointer to that method's id, relative to the object that
holds the field (`/inner_method/id` for goal seek, whose `inner_method` is in the same object).
Its schema is that method's input or output schema.
