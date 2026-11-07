# Command line

The `fairmark` binary reads JSON and writes JSON. Errors go to stderr as JSON with a
non-zero exit code.

```bash
fairmark methods                                   # list methods
fairmark schema dcf.fcff                           # JSON Schema of the inputs
fairmark run dcf.fcff --input company.json         # full result: value, explain, meta
fairmark run dcf.fcff --input company.json --value-only
cat company.json | fairmark run dcf.fcff           # stdin works too
fairmark verify result.json --input company.json   # reproduce and compare
```

Example input:

```json
{
  "free_cash_flows": ["1200000", "1350000", "1500000", "1600000", "1700000"],
  "discount_rate": "0.095",
  "terminal_growth": "0.02",
  "timing": "mid_period",
  "net_debt": "3100000"
}
```
