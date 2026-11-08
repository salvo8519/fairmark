# Terminal value, liquidation (`tv.liquidation`)

Returns the net liquidation value at the horizon N: the realisable values of the assets, less
the liabilities settled, the costs of the liquidation and the tax on the gains realised. Use
it for ventures without continuity after N or with separable assets. Every line appears in
`explain` with its label, so the value can be audited line by line.

## Formula

$$
TV_N = \sum \text{realisable values} - \sum \text{liabilities}
- \sum \text{liquidation costs} - \text{tax on gains}
$$

The result is negative when the liabilities and costs exceed the realisable values; it is
returned as is, with no floor at zero, and the convention says so.

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.tv.liquidation(
    assets={"Land": 800, "Plant": 300, "Receivables": 200},
    liabilities={"Bank loan": 600, "Suppliers": 250},
    liquidation_costs={"Legal fees": 40, "Closure": 60},
    tax_on_gains=30,
)
assert r.terminal_value == Decimal("320")
assert r.total_liabilities == Decimal("850")
for step in r.steps[:3]:
    print(step.label, step.value)  # Assets: Land 800, Assets: Plant 300, Assets: Receivables 200

# Lists of (label, amount) pairs are accepted too
pairs = fm.tv.liquidation([("Lot", 500), ("Lot", 300)], [("Suppliers", 100)], [], 0)
assert pairs.terminal_value == Decimal("700")
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `assets` | List of `{label, value}`: realisable value at N of each asset, zero or positive | required |
| `liabilities` | List of `{label, value}`: amount settled at N, zero or positive | required |
| `liquidation_costs` | List of `{label, value}`: legal fees, closure, severance, zero or positive | required |
| `tax_on_gains` | Tax on the gains realised, as an amount, zero or positive | required |

`assets` needs at least one line; the other lists may be empty. Labels must not be empty.

## Outputs

| Field | Meaning |
| --- | --- |
| `terminal_value` | Net liquidation value at N, not discounted |
| `total_assets` | Sum of the realisable values |
| `total_liabilities` | Sum of the liabilities |
| `total_liquidation_costs` | Sum of the liquidation costs |
| `tax_on_gains` | Tax on gains, as given |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Timing | assets realised and liabilities settled at N | fixed |
| Tax on gains | given as an amount | fixed: not computed from a rate |
| Negative net value | returned as is | fixed: no floor at zero |
| Basis | the liabilities listed decide it: with financial debt the value is an equity value | analyst's choice |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `assets` is empty, a label is empty, or a field is missing or unknown |
| `FM-E018` | An asset value, a liability, a liquidation cost or the tax on gains is negative |
| `FM-E008` | Arithmetic overflow (inputs far outside a realistic range) |

## Accuracy

Additions and subtractions of the given amounts: exact.

## Verified against

- Golden cases in `tests/golden/tv.liquidation/`, with the sum written out in each file and
  recomputed by an independent script (`scripts/reference_tv.py`).
