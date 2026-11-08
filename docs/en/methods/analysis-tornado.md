# Tornado (`analysis.tornado`)

Ranks the inputs of any registered method by how much they move one decimal output. Each input
is varied alone between an explicit low and high value, the others stay at base, and the inputs
are ranked by the absolute swing of the output: the data behind a tornado chart. Typical
question: which DCF assumption matters most for the equity value.

Tornado is an analysis method: it evaluates another method through the registry (see
[Analysis methods](../concepts/analysis.md)). It cannot evaluate another analysis method.

## Formula

For each variable $x_k$ with values $\text{low}_k < \text{high}_k$, the other inputs at base:

$$
\text{swing}_k = y(x_k = \text{high}_k) - y(x_k = \text{low}_k),
\qquad
\text{bars ranked by } \lvert \text{swing}_k \rvert \text{, largest first}
$$

The swing is in the output's units and keeps its sign: negative when the output falls as the
input rises. Equal absolute swings keep the order in which the variables were given.

## Example

The five-year DCF of [`dcf.fcff`](dcf-fcff.md): free cash flows of 100, 110, 120, 130 and 140
currency units, a discount rate of 9%, terminal growth of 2% and net debt of 300. Four drivers
of the equity value: net debt 250 to 350, first-year flow 90 to 110, terminal growth 1% to 3%
and discount rate 8% to 10%.

```python
from decimal import Decimal

import fairmark as fm

base = {
    "stages": [{"periods": 5, "projection": "given", "flows": [100, 110, 120, 130, 140]}],
    "cash_flow_terms": "nominal",
    "discount_rate_terms": "nominal",
    "wacc": "0.09",
    "timing": "end_of_period",
    "terminal": {"tv.gordon": {"growth": "0.02", "first_flow": "last_flow_grown"}},
    "net_financial_position": {"gross_debt": 300, "cash": 0},
}
t = fm.analysis.tornado(
    "dcf.fcff",
    base,
    output="/equity_value",
    variables={
        "/net_financial_position/gross_debt": ("250", "350"),
        "/stages/0/flows/0": ("90", "110"),
        "/terminal/tv.gordon/growth": ("0.01", "0.03"),
        "/wacc": ("0.08", "0.10"),
    },
)
print(t.base)  # 1485.9356357015730744749501235
for bar in t.ranking:
    print(bar["variable"], bar["swing"])
# /wacc -536.7420151241019796250302455
# /terminal/tv.gordon/growth 413.2480397880313369436240625
# /net_financial_position/gross_debt -100.0000000000000000000000000
# /stages/0/flows/0 18.3486238532110091743119266

assert [bar["variable"] for bar in t.ranking] == [
    "/wacc", "/terminal/tv.gordon/growth", "/net_financial_position/gross_debt", "/stages/0/flows/0",
]
# Expected swings from an independent 50-digit computation (scripts/reference_analysis.py).
expected = ["-536.74201512410197962503024533", "413.24803978803133694362406247",
            "-100", "18.348623853211009174311926606"]
for bar, swing in zip(t.ranking, expected):
    assert abs(bar["swing"] - Decimal(swing)) < Decimal("0.000000001")
    assert bar["abs_swing"] == abs(bar["swing"])

# The same request through the generic entry point, as an agent or the CLI sends it.
same = fm.run("analysis.tornado", {
    "method": "dcf.fcff",
    "base_input": base,
    "output": "/equity_value",
    "variables": [
        {"variable": "/net_financial_position/gross_debt", "low": "250", "high": "350"},
        {"variable": "/stages/0/flows/0", "low": "90", "high": "110"},
        {"variable": "/terminal/tv.gordon/growth", "low": "0.01", "high": "0.03"},
        {"variable": "/wacc", "low": "0.08", "high": "0.10"},
    ],
})
assert same.ranking == t.ranking
```

The discount rate dominates: two percentage points move the equity value by about 537 currency
units, more than five times the effect of 100 of net debt. The steps of the result list the base
case, then for each variable in the order given its output at low, its output at high and its
swing.

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `method` | Id of the method to evaluate, for example `dcf.fcff`; analysis methods are not accepted | required |
| `base_input` | Input of that method, complete, exactly as the method accepts it: the base case; each variable changes alone, the others keep their base values | required |
| `output` | RFC 6901 JSON Pointer to the decimal output to rank, in the `value` of the inner result, for example `/equity_value` | required |
| `variables` | The inputs to vary; at least one; pointers do not repeat | required |
| `variables[].variable` | RFC 6901 JSON Pointer to a decimal input in `base_input`, for example `/discount_rate` | required |
| `variables[].low` | Low value of the input, in its units, lower than `high` | required |
| `variables[].high` | High value of the input, in its units | required |

Low and high are explicit values, not percentage shocks: "±10% on a 9% rate" could mean 8.1% to
9.9% or 8% to 10%, so the caller writes the values. In Python, `variables` maps each pointer to
its `(low, high)` pair; the mapping keeps its order.

## Outputs

| Field | Meaning |
| --- | --- |
| `inner_method` | `id` and `version` of the method evaluated, at the version this engine ran |
| `output` | The output pointer ranked |
| `base` | The output for `base_input` unchanged, in the output's units |
| `ranking` | The bars by decreasing `abs_swing`; equal swings keep the order of `variables` |
| `ranking[].variable`, `low`, `high` | The input varied and its two values, as given |
| `ranking[].output_low`, `output_high` | The output with the input at `low` and at `high`, in the output's units |
| `ranking[].swing` | `output_high − output_low`, in the output's units; negative when the output falls as the input rises |
| `ranking[].abs_swing` | $\lvert$`swing`$\rvert$: the length of the bar and the ranking key |

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Inner method | id and version, evaluated through the registry | required |
| Inner method status | when the inner method is preview, "Evaluated method <id> is preview"; the status is also in `meta.evaluated_method_status` | reported |
| Output | the pointer the bars measure | required |
| Variation | each variable alone; the other inputs keep their base values | fixed |
| Shocks | explicit low and high values; no percentage shocks | required |
| Evaluation order | base case, then each variable in the order given, low before high | fixed |
| Ranking | by absolute swing, largest first; equal swings keep the order given | fixed |
| Failed evaluations | any evaluation the inner method refuses ends the request with its error | fixed |
| Variable encoding | written into the inner input as decimal strings | fixed |

The last convention reported is the number of evaluations of the inner method: two per variable
plus the base case.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `variables` is empty, a variable repeats, low ≥ high, or a field is missing or unknown |
| `FM-E007` | `method` names no registered method |
| `FM-E015` | A variable does not resolve to a decimal in `base_input`, or `output` does not resolve to a decimal in the result of the base case; the message names the pointer |
| `FM-E016` | Two evaluations per variable plus the base case exceed 10,000 or the evaluations × values in `base_input` exceed 1,000,000; returned before the first evaluation |
| `FM-E017` | `method` names an analysis method |

Variable pointers, low and high, and the syntax of `output` are checked before any evaluation;
`output` is resolved on the base case. Unlike tables and scenarios, an error of the inner method
at any evaluation, base case included, ends the request and is returned unchanged with its own
code: a ranking with a missing bar is not defined. For example, a terminal growth high value at
or above the discount rate of `dcf.fcff` returns `FM-E004`; choose low and high values at which
the inner method is valid.

## Accuracy

Outputs are as accurate as the inner method (see its page); swings are exact differences of the
reported outputs. The golden cases are checked to $10^{-9}$ in the output's units; ties are
checked exactly.

## Verified against

- Golden cases in `tests/golden/analysis.tornado/`: four DCF drivers, whose expected ranking and
  swings come from `scripts/reference_analysis.py` (`dcf.fcff` restated from its documented
  formula in Python's `decimal` at 50 digits; it shares no code with the engine), and a
  constructed tie on `tvm.npv` at rate 0, where shocking either of two cash flows by ±1 moves
  the NPV by 2 and the declaration order is kept.
