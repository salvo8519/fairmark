# Getting started

Fairmark uses exact decimal arithmetic and returns an auditable explanation for each valuation result.

## Install

```bash
uv add fairmark
# or
pip install fairmark
```

## Minimal example

```python
import fairmark as fm

r = fm.dcf.fcff(
    [fm.dcf.given([100, 110, 120])],
    cash_flow_terms="nominal",
    discount_rate_terms="nominal",
    wacc="0.10",
    timing="end_of_period",
    terminal=fm.dcf.gordon("0.02", first_flow="last_flow_grown"),
    net_financial_position={"gross_debt": 200, "cash": 0},
)

print(r.equity_value)
print(r.steps[0].formula)
```

## More information

- [Project README](../README.md)
- [Documentation site](https://salvo8519.github.io/fairmark/)
- [Methods](https://salvo8519.github.io/fairmark/methods/)
- [Contributing guide](../CONTRIBUTING.md)
- [Security policy](../SECURITY.md)
