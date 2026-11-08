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
