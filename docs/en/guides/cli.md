# Command line

The `fairmark` binary reads JSON and writes JSON. Errors go to stderr as JSON with a
non-zero exit code.

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
