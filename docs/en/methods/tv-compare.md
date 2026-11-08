# Terminal value, comparison of techniques (`tv.compare`)

Runs several terminal value techniques on one consistent set of inputs and returns each
terminal value at the horizon N with its implied assumptions, side by side. The conclusion of
a valuation often depends more on the technique than on any single input; this method shows by
how much. Consistency is by construction: the rate of the terminal phase, the growth cap and
the basis (enterprise or equity) are given once and apply to every technique.

## How it works

Each technique is written as `{"<method id>": {terms}}`, where the terms are the inputs of
that method without `rate` and `growth_cap`:

| Technique | Terms | Shared inputs used |
| --- | --- | --- |
| [`tv.gordon`](tv-gordon.md) | `growth`, `next_flow` or `last_flow` | rate, growth cap |
| [`tv.value_driver`](tv-value-driver.md) | `nopat_next`, `growth`, `ronic` | rate, growth cap |
| [`tv.h_model`](tv-h-model.md) | `last_flow`, `short_term_growth`, `long_term_growth`, `half_life` | rate, growth cap |
| [`tv.exit_multiple`](tv-exit-multiple.md) | `multiple`, `metric`, `metric_value`, optional `next_flow` | rate and growth cap when `next_flow` is given; the metric must be on the declared basis |
| [`tv.limited_horizon`](tv-limited-horizon.md) | `flow`, `periods`, optional `growth`, `residual_value` | rate, growth cap |
| [`tv.normalised_income`](tv-normalised-income.md) | `income`, `capitalisation_rate`, `basis`, optional `growth` | growth cap; the capitalisation rate stays its own; `basis` must match the declared basis |
| [`tv.liquidation`](tv-liquidation.md) | `assets`, `liabilities`, `liquidation_costs`, `tax_on_gains` | none; the liabilities listed must match the declared basis |

Each technique is computed by its own method, so a row of the comparison equals the result of
that method run alone. Steps and conventions of each technique appear in `explain`, prefixed
with its position and id, for example `[1] tv.gordon: Terminal value at N`. The first technique
that fails returns its error; there are no partial results. A technique entry must have
exactly one key, the method id; an entry with several keys is refused with a message that
lists them.

Each row records `method_version`, the version of the technique that produced it. A numerical
change in any `tv.*` method therefore requires a new version of `tv.compare` as well; a test
pins the technique versions together with the version of `tv.compare`.

`tv.normalised_income` declares its own `basis`, checked against the basis of the comparison
like the exit multiples (`FM-E019` when they differ). An exit multiple without `next_flow` has
no implied growth, so it is not checked against the growth cap; `explain.conventions` states
it when it applies.

## Example

```python
from decimal import Decimal

import fairmark as fm

r = fm.tv.compare(
    "0.10",
    "enterprise",
    [
        {"tv.gordon": {"growth": "0.02", "last_flow": 100}},
        {"tv.value_driver": {"nopat_next": 150, "growth": "0.02", "ronic": "0.12"}},
        {"tv.exit_multiple": {"multiple": "7.5", "metric": "ebitda", "metric_value": 180,
                              "next_flow": 95}},
    ],
    growth_cap="0.03",
)
values = [row["terminal_value"] for row in r.results]
assert r.results[0]["method_version"] == "1"
assert values[0] == Decimal("1275")
assert values[2] == Decimal("1350")
assert round(r.results[2]["output"]["implied_growth"], 6) == Decimal("0.029630")

try:  # P/E is an equity measure: never mixed into an enterprise comparison
    fm.tv.compare("0.10", "enterprise", [
        {"tv.exit_multiple": {"multiple": 12, "metric": "earnings", "metric_value": 50}},
    ])
except fm.FairmarkError as err:
    assert err.code == "FM-E019"
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `rate` | Discount rate of the terminal phase, decimal fraction per period, shared | required |
| `basis` | `enterprise` or `equity`, shared by every terminal value compared | required |
| `growth_cap` | Upper bound for the perpetual growth, decimal fraction per period, shared | none, reported as absent |
| `techniques` | List of `{"<method id>": {terms}}`, at least one; a technique may appear more than once | required |

## Outputs

| Field | Meaning |
| --- | --- |
| `basis` | The declared basis |
| `results` | One row per technique, in the order requested: `method`, `method_version`, `terminal_value` and `output`, the technique's own output with its implied assumptions |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Options | Default |
| --- | --- | --- |
| Basis | enterprise or equity, declared once | none: must be given |
| Rate | one rate for every technique that discounts; the capitalisation rate of `tv.normalised_income` is its own | none: must be given |
| Growth cap | one value for every growth-based technique, or none | none, reported as absent |

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | The list of techniques is empty, an entry has more than one key, a technique id is unknown, or a field is missing or unknown |
| `FM-E019` | An exit multiple or a normalised income is on the other basis than the one declared |
| Any code of a technique | The first technique that fails returns its own error (for example `FM-E004`, `FM-E018`) |

## Accuracy

Each row has the accuracy of its method; the comparison adds no arithmetic.

## Verified against

- `tests/golden/tv.compare/`: seven techniques on one rate, basis and cap, each expected value
  computed on its own by an independent 50-digit script (`scripts/reference_tv.py`).
- A Python test checks that each row equals the method run alone.
