# Quickstart

This page takes five minutes. Every snippet on it runs as part of the test-suite.

## Your first valuation

```python
import fairmark as fm

r = fm.dcf.fcff(
    [fm.dcf.given([100, 110, 120])],   # one stage: FCFF for years 1, 2, 3
    cash_flow_terms="nominal",         # flows and rates in the same terms
    discount_rate_terms="nominal",
    wacc="0.10",                       # WACC as a decimal fraction
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
    net_financial_position={"gross_debt": 200, "cash": 0},
)

print(r.value_of_operations)          # 1421.4876033057851239669421487
print(r.equity_value)                 # 1221.4876033057851239669421487
print(r.terminal["terminal_value"])   # 1530
```

Values are Python `Decimal`s. Pass numbers as strings or `Decimal` to keep them exact;
floats are accepted and converted through their shortest representation (`0.1` → `"0.1"`).

## See how it was computed

```python
for step in r.steps:
    print(f"{step.label:<32} {step.formula:<40} {step.value}")

print(r.conventions)
```

`r.explain_markdown()` renders the same information as a table you can paste into a report.

## Reproduce a result later

```python
print(r.meta["input_hash"])       # SHA-256 of the canonical input
print(r.meta["engine_version"])
```

Same hash and same versions mean the same result. From the command line,
`fairmark verify result.json --input inputs.json` checks it for you.

## Handle errors

```python
try:
    fm.dcf.fcff(
        [fm.dcf.given([100])],
        cash_flow_terms="nominal",
        discount_rate_terms="nominal",
        wacc="0.05",
        timing="end_of_period",
        terminal=fm.dcf.gordon("0.06", first_flow="last_flow_grown"),
    )
except fm.FairmarkError as err:
    print(err.code)      # FM-E004
    print(err.message)   # `/terminal/tv.gordon/growth` (0.06) must be lower than ... `/wacc` (0.05) ...
```

Every error has a stable code: see the [error reference](../reference/errors.md).

## Next steps

- Let an AI agent do the extraction: [your first valuation with an agent](first-valuation-with-an-agent.md).
- Browse the [methods](../methods/index.md).
