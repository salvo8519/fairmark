# Scenario analysis (`analysis.scenarios`)

Reports one or more decimal outputs of any registered method under named cases, each a set of
input changes against the base case, with the delta and the relative delta of every output.
Typical question: the enterprise and equity value of a DCF in a downside and an upside case, and
how far each moves from the base. A case the inner method refuses is recorded with its error and
the other cases are still returned.

Scenarios is an analysis method: it evaluates another method through the registry (see
[Analysis methods](../concepts/analysis.md)). It cannot evaluate another analysis method.

## Formula

For each scenario $s$, with the base input changed by the overrides of $s$, and each output $y$:

$$
\Delta_s = y_s - y_{base},
\qquad
\delta_s = \frac{\Delta_s}{y_{base}} \quad (y_{base} \neq 0)
$$

$\Delta_s$ is in the output's units; $\delta_s$ is a decimal fraction (`-0.2` is a fall of 20%)
and is `null` when $y_{base}$ is zero. Inputs a scenario does not change keep their base value.

## Example

The five-year DCF of [`dcf.fcff`](dcf-fcff.md): free cash flows of 100, 110, 120, 130 and 140
currency units, a discount rate of 9%, terminal growth of 2% and net debt of 300. Three cases:
a downside (rate 10%, growth 1%, first-year flow 90), an upside (rate 8%, growth 2.5%) and a
case with growth of 12%, which the DCF refuses.

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
r = fm.analysis.scenarios(
    "dcf.fcff",
    base,
    outputs=["/enterprise_value", "/equity_value"],
    cases={
        "downside": {"/wacc": "0.10", "/terminal/tv.gordon/growth": "0.01",
                     "/stages/0/flows/0": "90"},
        "upside": {"/wacc": "0.08", "/terminal/tv.gordon/growth": "0.025"},
        "impossible": {"/terminal/tv.gordon/growth": "0.12"},
    },
)
print(r.base[1]["value"])        # 1485.9356357015730744749501235, base equity value
downside = r.scenarios[0]["value"][1]
print(downside["value"])         # 1114.1421730452534359371324060
print(downside["delta"])         # -371.7934626563196385378177175
print(downside["relative_delta"])  # -0.250208322435702416213594832, a fall of about 25%
print(r.scenarios[2]["error"]["code"])  # FM-E004: growth 12% is not below the rate 9%
print(r.failed_scenarios)        # 1

# Expected values from an independent 50-digit computation (scripts/reference_analysis.py).
assert abs(downside["value"] - Decimal("1114.1421730452534359371324059528")) < Decimal("1e-9")
upside = r.scenarios[1]["value"][0]
assert abs(upside["value"] - Decimal("2248.6986924572658454688633015108")) < Decimal("1e-9")
assert abs(upside["relative_delta"] - Decimal("0.259115192902181230325447937")) < Decimal("1e-9")
# The delta is exactly the scenario value minus the base value.
assert downside["delta"] == downside["value"] - r.base[1]["value"]

# The same request through the generic entry point, as an agent or the CLI sends it.
same = fm.run("analysis.scenarios", {
    "method": "dcf.fcff",
    "base_input": base,
    "outputs": ["/enterprise_value", "/equity_value"],
    "scenarios": [
        {"name": "downside", "overrides": [
            {"pointer": "/wacc", "value": "0.10"},
            {"pointer": "/terminal/tv.gordon/growth", "value": "0.01"},
            {"pointer": "/stages/0/flows/0", "value": "90"},
        ]},
        {"name": "upside", "overrides": [
            {"pointer": "/wacc", "value": "0.08"},
            {"pointer": "/terminal/tv.gordon/growth", "value": "0.025"},
        ]},
        {"name": "impossible", "overrides": [
            {"pointer": "/terminal/tv.gordon/growth", "value": "0.12"},
        ]},
    ],
})
assert same.scenarios == r.scenarios
```

The relative delta is not defined when the base output is zero. At 10% the NPV of −100 and 110
is exactly zero, so a case at 5% reports the delta and a `null` relative delta:

```python
import fairmark as fm

npv = fm.analysis.scenarios(
    "tvm.npv",
    {"rate": "0.10", "cash_flows": [-100, 110]},
    outputs=["/npv"],
    cases={"lower rate": {"/rate": "0.05"}},
)
lower = npv.scenarios[0]["value"][0]
print(lower["delta"])            # 4.76190476190476190476190476
assert npv.base[0]["value"] == 0
assert lower["relative_delta"] is None
```

## Inputs

| Field | Meaning | Default |
| --- | --- | --- |
| `method` | Id of the method to evaluate, for example `dcf.fcff`; analysis methods are not accepted | required |
| `base_input` | Input of that method, complete, exactly as the method accepts it: the base case | required |
| `outputs` | RFC 6901 JSON Pointers to the decimal outputs to report, in the `value` of the inner result, for example `["/enterprise_value", "/equity_value"]`; at least one, no repeats | required |
| `scenarios` | The cases, evaluated in the order given; at least one | required |
| `scenarios[].name` | Name of the case, unique and not empty, for example `downside` | required |
| `scenarios[].overrides` | Inputs changed in this case; pointers do not repeat within a scenario | required |
| `overrides[].pointer` | RFC 6901 JSON Pointer to a decimal input in `base_input`, for example `/terminal_growth` | required |
| `overrides[].value` | Value of that input in this case, in its units (a decimal fraction for a rate) | required |

In Python, `cases` maps each scenario name to `{pointer: value}`; the mapping keeps its order.

## Outputs

| Field | Meaning |
| --- | --- |
| `inner_method` | `id` and `version` of the method evaluated, at the version this engine ran |
| `base` | One `{output, value}` per output pointer: the outputs of `base_input` unchanged, in the output's units |
| `scenarios` | One entry per scenario, in the order given: `{name, value: [...]}`, or `{name, error: {code, message}}` when the inner method refused the case |
| `scenarios[].value[]` | Per output: `output`, `value` (output's units), `delta` (`value − base`, output's units), `relative_delta` (`delta / base`, decimal fraction, `null` when the base is zero) |
| `failed_scenarios` | Number of scenarios refused by the inner method or whose deltas overflow |

The steps list the base value of each output, then one step per output of each successful
scenario, with the overrides, the value, the delta and the relative delta substituted.

## Conventions (explicit, reported in `explain.conventions`)

| Convention | Value | Default |
| --- | --- | --- |
| Inner method | id and version, evaluated through the registry | required |
| Inner method status | when the inner method is preview, "Evaluated method <id> is preview"; the status is also in `meta.evaluated_method_status` | reported |
| Evaluation order | base case, then the scenarios in the order given | fixed |
| Inputs not overridden | keep their base value | fixed |
| Relative delta when the base output is zero | `null` | fixed |
| Failed scenarios | a scenario the inner method refuses, or whose deltas overflow the decimal range, holds its error code and message and the others go on; `FM-E012` (a defect in Fairmark) aborts the request | fixed |
| Override encoding | written into the inner input as decimal strings | fixed |

The last convention reported is the number of evaluations of the inner method and the number of
failed scenarios. A request needs one evaluation per scenario plus the base case.

## Errors

| Code | When |
| --- | --- |
| `FM-E001` | `outputs` or `scenarios` is empty, an output repeats, a scenario name is empty, has leading or trailing spaces, or repeats, a pointer repeats within one scenario, or a field is missing or unknown |
| `FM-E007` | `method` names no registered method |
| `FM-E015` | An override pointer does not resolve to a decimal in `base_input`, or an output pointer does not resolve to a decimal in the result of the base case; the message names the pointer |
| `FM-E016` | The number of scenarios plus one exceeds 10,000 evaluations or the evaluations × values in `base_input` exceed 1,000,000; returned before the first evaluation |
| `FM-E017` | `method` names an analysis method |

Override pointers, the syntax of the outputs and `base_input` are checked before any
evaluation, `base_input` by the validation alone of the inner method (every check of its input,
nothing computed, its input errors reported under `/base_input`); the outputs
are resolved on the base case. An error of the inner method on the base case is returned
unchanged, with its own code. An error of the inner method in a scenario is recorded in that
scenario, except `FM-E012`, which aborts the request.

## Accuracy

Values are as accurate as the inner method (see its page). Deltas are exact differences of the
reported values; relative deltas are rounded to the 28 significant digits of the decimal type.
The golden cases are checked to $10^{-9}$ in the output's units.

## Verified against

- Golden cases in `tests/golden/analysis.scenarios/` (three DCF cases, one refused, and a
  zero-base NPV case), whose expected values come from `scripts/reference_analysis.py`:
  `dcf.fcff` and `tvm.npv` restated from their documented formulas in Python's `decimal` at 50
  digits, with deltas and relative deltas computed from them. It shares no code with the
  engine.
- A test requires each delta to equal the scenario value minus the base value exactly, and a
  scenario with an invalid override to record its error while the others are returned.
